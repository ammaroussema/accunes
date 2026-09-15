use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper557 {
    pointer: u8,
    regs: [u8; 8],
}

impl Mapper557 {
    pub fn new() -> Self { Self { pointer: 0, regs: [0; 8] } }

    fn prg_read_bank(&self, bank: usize, address: u16, prg_rom: &[u8]) -> u8 {
        if prg_rom.is_empty() { return 0xFF; }
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        prg_rom[offset % prg_rom.len()]
    }
}

impl Mapper for Mapper557 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        match address {
            0x6000..=0x7FFF => {
                if cart.prg_ram.is_empty() {
                    FetchResult { data: 0xFF, driven: true }
                } else {
                    FetchResult { data: cart.prg_ram[(address as usize & 0x1FFF) % cart.prg_ram.len()], driven: true }
                }
            }
            0x8000..=0x9FFF => FetchResult { data: self.prg_read_bank((self.regs[6] & 0x0F) as usize, address, &cart.prg_rom), driven: true },
            0xA000..=0xBFFF => FetchResult { data: self.prg_read_bank((self.regs[7] & 0x0F) as usize, address, &cart.prg_rom), driven: true },
            0xC000..=0xDFFF => FetchResult { data: self.prg_read_bank(0x0E, address, &cart.prg_rom), driven: true },
            0xE000..=0xFFFF => FetchResult { data: self.prg_read_bank(0x0F, address, &cart.prg_rom), driven: true },
            _ => FetchResult { data: 0, driven: false },
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let len = cart.prg_ram.len();
                cart.prg_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if address >= 0x8000 && address < 0xA000 {
            if address & 1 == 0 {
                self.pointer = data & 7;
            } else {
                self.regs[self.pointer as usize] = data;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.regs[5] & 0x20 != 0 { (address & 0x3FFF) | ((address & 0x0800) >> 1) } else { address & 0x37FF }
    }

    fn fetch_ppu(&mut self, _prg_rom: &[u8], chr_rom: &[u8], _prg_ram: &[u8], chr_ram: &[u8], _prg_vram: &[u8], using_chr_ram: bool, _nh: bool, _alt: bool, ppu_addr: u16, ppu_latch: u8, vram: &[u8]) -> (u8, u16) {
        let address = (ppu_addr & 0x3F00) | ppu_latch as u16;
        let mut nab = ppu_addr & 0xFF00;
        if address < 0x2000 {
            let off = (address as usize) & 0x1FFF;
            if using_chr_ram { nab |= chr_ram.get(off).copied().unwrap_or(0) as u16; }
            else { nab |= chr_rom.get(off % chr_rom.len().max(1)).copied().unwrap_or(0) as u16; }
        } else if address < 0x3F00 {
            let mir = if self.regs[5] & 0x20 != 0 { (address & 0x3FFF) | ((address & 0x0800) >> 1) } else { address & 0x37FF };
            nab |= vram.get((mir & 0x7FF) as usize).copied().unwrap_or(0) as u16;
        }
        (nab as u8, nab)
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = vec![self.pointer];
        state.extend_from_slice(&self.regs);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start < state.len() { self.pointer = state[start] & 7; start += 1; }
        let copy_len = (state.len() - start).min(8);
        self.regs[..copy_len].copy_from_slice(&state[start..start + copy_len]);
        start += copy_len;
        start
    }

    fn reset(&mut self) { self.pointer = 0; self.regs = [0; 8]; }
}
