use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper326 {
    prg: [u8; 4],
    chr: [u8; 16],
}

impl Mapper326 {
    pub fn new() -> Self {
        Self { prg: [0xFC, 0xFD, 0xFE, 0xFF], chr: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15] }
    }
}

impl Mapper for Mapper326 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank_idx = ((address - 0x8000) >> 13) as usize;
            let bank = self.prg[bank_idx] as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let full_addr = address as usize | 0x8000;
            match full_addr & 0xE010 {
                0x8000 => self.prg[0] = data,
                0xA000 => self.prg[1] = data,
                0xC000 => self.prg[2] = data,
                _ => {}
            }
            if (full_addr & 0x8010) == 0x8010 {
                self.chr[full_addr & 0xF] = data;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if address & 0x0800 == 0 {
            let nt_bank = ((address >> 11) & 1) as usize;
            let screen = (self.chr[8 + nt_bank] & 1) as u16;
            if screen == 0 {
                address & 0x33FF
            } else {
                0x2400 | (address & 0x33FF)
            }
        } else {
            let nt_bank = ((address >> 11) & 1) as usize;
            let screen = (self.chr[12 + nt_bank] & 1) as u16;
            if screen == 0 {
                0x2000 | (address & 0x33FF)
            } else {
                0x2400 | (address & 0x33FF)
            }
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
            let bank = self.chr[(address >> 10) as usize] as usize;
            let offset = bank * 0x400 + (address as usize & 0x3FF);
            let data = if using_chr_ram { chr_ram[offset % chr_ram.len()] } else { chr_rom[offset % chr_rom.len()] };
            new_addr_bus |= data as u16;
        } else {
            let nt_idx = ((address - 0x2000) >> 10) as usize;
            let screen = (self.chr[8 + nt_idx] & 1) as u16;
            let nt_base = if screen == 0 { 0x2000 } else { 0x2400 };
            let offset = (address as usize) & 0x3FF;
            new_addr_bus |= vram[(nt_base as usize - 0x2000 + offset) & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[address as usize % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() { cart.prg_vram[idx] = data; }
            } else {
                vram[mirrored as usize & 0x7FF] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::with_capacity(20);
        state.extend_from_slice(&self.prg);
        state.extend_from_slice(&self.chr);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..4 { if p < state.len() { self.prg[i] = state[p]; p += 1; } }
        for i in 0..16 { if p < state.len() { self.chr[i] = state[p]; p += 1; } }
        p
    }

    fn reset(&mut self) {
        self.prg = [0xFC, 0xFD, 0xFE, 0xFF];
        self.chr = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    }
}
