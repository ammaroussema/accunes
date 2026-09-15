use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

pub struct Mapper599 {
    latch_addr: u16,
    latch_data: u8,
}

impl Mapper599 {
    pub fn new() -> Self {
        Self {
            latch_addr: 0,
            latch_data: 0,
        }
    }

    fn prg_and_or(&self, prg_rom_len: usize) -> (usize, usize) {
        if prg_rom_len >= 512 * 1024 {
            let prg_and = if (self.latch_addr & 0x400) != 0 { 0x07 } else { 0x1F };
            let prg_or = if (self.latch_addr & 0x400) != 0 { 0x20 } else { 0x00 };
            (prg_and, prg_or)
        } else {
            let prg_and = if (self.latch_addr & 0x400) != 0 { 0x07 } else { 0x0F };
            let prg_or = if (self.latch_addr & 0x400) != 0 { 0x10 } else { 0x00 };
            (prg_and, prg_or)
        }
    }

    fn prg_16k_banks(&self, prg_rom_len: usize) -> (usize, usize) {
        let (prg_and, prg_or) = self.prg_and_or(prg_rom_len);
        let bank16 = prg_or | (((self.latch_addr as usize) >> 2) & prg_and);

        if (self.latch_addr & 0x080) != 0 {
            if (self.latch_addr & 0x001) != 0 {
                let bank32 = bank16 >> 1;
                (bank32 * 2, bank32 * 2 + 1)
            } else {
                (bank16, bank16)
            }
        } else {
            (bank16, prg_or)
        }
    }
}

impl Mapper for Mapper599 {
    fn reset(&mut self) {
        self.latch_addr = 0;
        self.latch_data = 0;
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 || cart.prg_rom.is_empty() {
            return FetchResult { data: 0, driven: false };
        }
        let (bank0, bank1) = self.prg_16k_banks(cart.prg_rom.len());
        let bank = if address < 0xC000 { bank0 } else { bank1 };
        let offset = bank * 0x4000 + (address as usize & 0x3FFF);
        FetchResult {
            data: cart.prg_rom[offset % cart.prg_rom.len()],
            driven: true,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            if (self.latch_addr & 0x4000) == 0 {
                self.latch_addr = address;
            }
            self.latch_data = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v((self.latch_addr & 0x02) != 0, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;

        if address < 0x2000 {
            let byte = if (self.latch_addr & 0x400) != 0 && !chr_rom.is_empty() && !using_chr_ram {
                let bank = (((self.latch_addr as usize) >> 6) & !0x03) | ((self.latch_data as usize) & 0x03);
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                chr_rom[offset % chr_rom.len()]
            } else if !chr_ram.is_empty() {
                let offset = address as usize & 0x1FFF;
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                let offset = address as usize & 0x1FFF;
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mirror_h_or_v((self.latch_addr & 0x02) != 0, address);
            let idx = (mirrored & 0x7FF) as usize;
            new_addr_bus |= vram[idx] as u16;
        }

        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let offset = address as usize & 0x1FFF;
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = self.latch_addr.to_le_bytes().to_vec();
        state.push(self.latch_data);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 2 <= state.len() {
            self.latch_addr = u16::from_le_bytes([state[start], state[start + 1]]);
            start += 2;
        }
        if start < state.len() {
            self.latch_data = state[start];
            start += 1;
        }
        start
    }
}
