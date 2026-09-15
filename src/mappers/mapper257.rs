use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const BANK_LUT: [u8; 128] = [
    0x03, 0x13, 0x23, 0x33, 0x03, 0x13, 0x23, 0x33, 0x03, 0x13, 0x23, 0x33, 0x03, 0x13, 0x23, 0x33,
    0x45, 0x67, 0x45, 0x67, 0x45, 0x67, 0x45, 0x67, 0x45, 0x67, 0x45, 0x67, 0x45, 0x67, 0x45, 0x67,
    0x03, 0x13, 0x23, 0x33, 0x03, 0x13, 0x23, 0x33, 0x03, 0x13, 0x23, 0x33, 0x03, 0x13, 0x23, 0x33,
    0x47, 0x67, 0x47, 0x67, 0x47, 0x67, 0x47, 0x67, 0x47, 0x67, 0x47, 0x67, 0x47, 0x67, 0x47, 0x67,
    0x02, 0x12, 0x22, 0x32, 0x02, 0x12, 0x22, 0x32, 0x02, 0x12, 0x22, 0x32, 0x02, 0x12, 0x22, 0x32,
    0x45, 0x67, 0x45, 0x67, 0x45, 0x67, 0x45, 0x67, 0x45, 0x67, 0x45, 0x67, 0x45, 0x67, 0x45, 0x67,
    0x02, 0x12, 0x22, 0x32, 0x02, 0x12, 0x22, 0x32, 0x02, 0x12, 0x22, 0x32, 0x00, 0x10, 0x20, 0x30,
    0x47, 0x67, 0x47, 0x67, 0x47, 0x67, 0x47, 0x67, 0x47, 0x67, 0x47, 0x67, 0x47, 0x67, 0x47, 0x67,
];

pub struct Mapper257 {
    is_small: bool,
    mode: u8,
    index: u8,
    prg_sw: u8,
    last_nt_addr: u16,
}

impl Mapper257 {
    pub fn new_small() -> Self {
        Self { is_small: true, mode: 0x0E, index: 0, prg_sw: 0, last_nt_addr: 0 }
    }

    pub fn new_large() -> Self {
        Self { is_small: false, mode: 0, index: 0, prg_sw: 0, last_nt_addr: 0 }
    }
}

impl Mapper for Mapper257 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if self.is_small {
                let lut = BANK_LUT[(self.mode & 0x7F) as usize];
                let bank_lo = (lut >> 4) as usize;
                let bank_hi = (lut & 0xF) as usize;
                let bank = if address < 0xC000 { bank_lo } else { bank_hi };
                let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
            } else {
                if !self.is_small {
                    if (self.mode & 0x10) == 0 && (self.mode & 0x40) != 0 && address < 0xA000 {
                        let bank = 0x20 | (self.mode as usize & 0xF) | if (self.mode & 0x20) != 0 { 0x10 } else { 0 };
                        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                        FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
                    } else {
                        let bank = (self.mode & 7) as usize;
                        let offset = bank * 0x8000 + (address as usize & 0x7FFF);
                        FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
                    }
                } else {
                    FetchResult { data: 0, driven: false }
                }
            }
        } else if address >= 0x5000 && address < 0x6000 {
            if self.is_small {
                if (address & 0x700) == 0x500 {
                    FetchResult { data: 0, driven: false }
                } else {
                    FetchResult { data: 0, driven: false }
                }
            } else {
                if (address >> 8 & 7) == 3 {
                    FetchResult { data: 0, driven: false }
                } else {
                    FetchResult { data: 0, driven: false }
                }
            }
        } else if address >= 0x6000 {
            let off = (address - 0x6000) as usize;
            if off < cart.prg_ram.len() { FetchResult { data: cart.prg_ram[off], driven: true } }
            else { FetchResult { data: 0, driven: false } }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x6000 {
            match address & 0x700 {
                0x000 => { self.mode = data; }
                0x100 => {
                    if self.is_small {
                        if self.prg_sw == data { self.prg_sw = data; }
                    } else {
                    }
                }
                0x400 => { self.index = data; }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.is_small {
            if (self.prg_sw & 2) != 0 { (address & 0x33FF) | ((address & 0x0800) >> 1) } else { address & 0x37FF }
        } else {
            if (self.mode & 0x18) == 0x18 { (address & 0x33FF) | ((address & 0x0800) >> 1) } else { address & 0x37FF }
        }
    }

    fn fetch_ppu(&mut self, _prg_rom: &[u8], _chr_rom: &[u8], _prg_ram: &[u8], chr_ram: &[u8], _prg_vram: &[u8], using_chr_ram: bool, _nametable_horizontal_mirroring: bool, _alternative_nametable_arrangement: bool, ppu_address_bus: u16, ppu_octal_latch: u8, vram: &[u8]) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let mut idx = address as usize;
            if self.mode & 0x80 != 0 {
                idx = (idx & !0x0008) | ((self.last_nt_addr as usize & 0x0001) << 3);
                idx = (idx & !0x1000) | ((self.last_nt_addr as usize & 0x0200) << 3);
            }
            let len = if using_chr_ram && !chr_ram.is_empty() { chr_ram.len() } else { _chr_rom.len() };
            let data = if len > 0 {
                if using_chr_ram && !chr_ram.is_empty() { chr_ram[idx % len] } else { _chr_rom[idx % len] }
            } else { 0 };
            new_addr_bus |= data as u16;
        } else if address < 0x3F00 {
            if address & 0x3FF < 0x3C0 {
                let a = address & 0x3FF;
                if (a & 0x3C0) == 0 {
                    self.last_nt_addr = address & 0x3FF;
                }
            }
            let mirrored = if self.is_small {
                if (self.prg_sw & 2) != 0 { (address & 0x33FF) | ((address & 0x0800) >> 1) } else { address & 0x37FF }
            } else {
                if (self.mode & 0x18) == 0x18 { (address & 0x33FF) | ((address & 0x0800) >> 1) } else { address & 0x37FF }
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 { if cart.using_chr_ram && !cart.chr_ram.is_empty() { let len = cart.chr_ram.len(); cart.chr_ram[address as usize % len] = data; } }
        else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 { let idx = (mirrored & 0x7FF) as usize; if idx < cart.prg_vram.len() { cart.prg_vram[idx] = data; } }
            else { vram[mirrored as usize & 0x7FF] = data; }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![if self.is_small { 1 } else { 0 }, self.mode, self.index]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { p += 1; }
        if p < state.len() { self.mode = state[p]; p += 1; }
        if p < state.len() { self.index = state[p]; p += 1; }
        p
    }

    fn reset(&mut self) {
        if self.is_small { self.mode = 0x0E; } else { self.mode = 0; }
        self.index = 0;
        self.prg_sw = 0;
        self.last_nt_addr = 0;
    }
}
