use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper202 {
    addr: u16,
}

impl Mapper202 {
    pub fn new() -> Self {
        Self { addr: 0 }
    }
}

impl Mapper for Mapper202 {
    fn reset(&mut self) {
        self.addr = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            if address >= 0x6000 {
                let len = cart.prg_ram.len();
                if len > 0 {
                    return FetchResult { data: cart.prg_ram[(address as usize) & 0x1FFF], driven: true };
                }
            }
            return FetchResult { data: 0, driven: false };
        }
        let prg_len = cart.prg_rom.len();
        if prg_len == 0 {
            return FetchResult { data: 0, driven: false };
        }
        let mirror = (self.addr & 1) != 0;
        let bank = ((self.addr >> 1) & 0x7) as usize;
        let select = mirror && bank >= 4;
        let num_16k = (prg_len / 0x4000).max(1);
        let bank_idx = if address < 0xC000 {
            if select { bank & 0x6 } else { bank }
        } else {
            if select { (bank & 0x6) | 1 } else { bank }
        };
        let offset = (bank_idx % num_16k) * 0x4000 + (address as usize & 0x3FFF);
        FetchResult { data: cart.prg_rom[offset % prg_len], driven: true }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, _data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                cart.prg_ram[(address as usize) & 0x1FFF] = _data;
            }
        } else if address >= 0x8000 {
            self.addr = address;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let h = (self.addr & 1) == 0;
        if h {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
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
            let bank = ((self.addr >> 1) & 0x7) as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let h = (self.addr & 1) == 0;
            let mirrored = if h {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let bank = ((self.addr >> 1) & 0x7) as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        self.addr.to_le_bytes().to_vec()
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start + 2 <= state.len() {
            self.addr = u16::from_le_bytes([state[start], state[start + 1]]);
            start + 2
        } else { start }
    }
}
