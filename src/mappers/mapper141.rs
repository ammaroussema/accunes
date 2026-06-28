use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper141 {
    index: u8,
    reg: [u8; 8],
}

impl Mapper141 {
    pub fn new() -> Self {
        Self { index: 0, reg: [0; 8] }
    }
}

impl Mapper for Mapper141 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = self.reg[5] as usize;
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
        } else if address >= 0x6000 {
            let off = (address - 0x6000) as usize;
            if off < cart.prg_ram.len() { FetchResult { data: cart.prg_ram[off], driven: true } }
            else { FetchResult { data: 0, driven: false } }
        } else { FetchResult { data: 0, driven: false } }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x4000 && address <= 0x5FFF {
            if (address & 0x100) != 0 {
                if (address & 1) != 0 {
                    self.reg[(self.index & 7) as usize] = data;
                } else {
                    self.index = data;
                }
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        match self.reg[7] & 7 {
            0 => (address & 0x33FF) | ((address & 0x0400) >> 1),
            2 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            6 => address & 0x37FF,
            _ => address & 0x37FF,
        }
    }

    fn fetch_ppu(&mut self, _prg_rom: &[u8], chr_rom: &[u8], _prg_ram: &[u8], chr_ram: &[u8], _prg_vram: &[u8], using_chr_ram: bool, _nametable_horizontal_mirroring: bool, _alternative_nametable_arrangement: bool, ppu_address_bus: u16, ppu_octal_latch: u8, vram: &[u8]) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let page = (address >> 11) as usize;
            let chrmode = (self.reg[7] & 1) != 0;
            let reg_idx = if chrmode { 0 } else { page >> 1 };
            let base_bank = ((self.reg[reg_idx] as usize) & 0x07) | ((self.reg[4] as usize) << 3);
            let bank = (base_bank << 1) | (page & 1);
            let offset = bank * 0x800 + (address as usize & 0x7FF);
            let data = if using_chr_ram && !chr_ram.is_empty() { chr_ram[offset % chr_ram.len()] } else { chr_rom[offset % chr_rom.len()] };
            new_addr_bus |= data as u16;
        } else {
            let mirrored = match self.reg[7] & 7 {
                0 => (address & 0x33FF) | ((address & 0x0400) >> 1),
                2 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                6 => address & 0x37FF,
                _ => address & 0x37FF,
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 { if cart.using_chr_ram && !cart.chr_ram.is_empty() { let len = cart.chr_ram.len(); cart.chr_ram[address as usize % len] = data; } }
        else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = match self.reg[7] & 7 {
                0 => (address & 0x33FF) | ((address & 0x0400) >> 1),
                2 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                6 => address & 0x37FF,
                _ => address & 0x37FF,
            };
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 { let idx = (mirrored & 0x7FF) as usize; if idx < cart.prg_vram.len() { cart.prg_vram[idx] = data; } }
            else { vram[mirrored as usize & 0x7FF] = data; }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut s = vec![self.index];
        s.extend_from_slice(&self.reg);
        s
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.index = state[p]; p += 1; }
        for i in 0..8 { if p < state.len() { self.reg[i] = state[p]; p += 1; } }
        p
    }

    fn reset(&mut self) {
        self.index = 0;
        self.reg = [0; 8];
    }
}
