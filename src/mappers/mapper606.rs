use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

pub struct Mapper606 {
    reg: u8,
    latch: u8,
}

impl Mapper606 {
    pub fn new() -> Self {
        Self { reg: 0, latch: 0 }
    }

    fn prg_16k_banks(&self) -> (usize, usize) {
        if (self.reg & 0x80) != 0 {
            let bank32 = (self.latch as usize & 0x03)
                | (((self.reg as usize) >> 4) & 0x04)
                | (((self.reg as usize) >> 2) & 0x08);
            (bank32 * 2, bank32 * 2 + 1)
        } else {
            let high = (((self.reg as usize) >> 3) & 0x08) | (((self.reg as usize) >> 1) & 0x10);
            let bank0 = (self.latch as usize & 0x07) | high;
            let bank1 = 0x07 | high;
            (bank0, bank1)
        }
    }

    fn mirror_address(&self, address: u16) -> u16 {
        if (self.reg & 0x80) != 0 {
            if (self.latch & 0x10) != 0 {
                (address & 0x33FF) | 0x0400
            } else {
                address & 0x33FF
            }
        } else if (self.latch & 0x10) != 0 {
            mirror_h_or_v(true, address)
        } else {
            mirror_h_or_v(false, address)
        }
    }
}

impl Mapper for Mapper606 {
    fn reset(&mut self) {
        self.reg = 0;
        self.latch = 0;
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 || cart.prg_rom.is_empty() {
            return FetchResult { data: 0, driven: false };
        }
        let (bank0, bank1) = self.prg_16k_banks();
        let bank = if address < 0xC000 { bank0 } else { bank1 };
        let offset = bank * 0x4000 + (address as usize & 0x3FFF);
        FetchResult {
            data: cart.prg_rom[offset % cart.prg_rom.len()],
            driven: true,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) {
            if (self.reg & 0x10) == 0 {
                self.reg = data;
            }
        } else if address >= 0x8000 {
            self.latch = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror_address(address)
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
            let offset = address as usize & 0x1FFF;
            let byte = if using_chr_ram || chr_rom.is_empty() {
                if !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else {
                    0
                }
            } else {
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = self.mirror_address(address);
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
        vec![self.latch, self.reg]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start < state.len() {
            self.latch = state[start];
            start += 1;
        }
        if start < state.len() {
            self.reg = state[start];
            start += 1;
        }
        start
    }
}
