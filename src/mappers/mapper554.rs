use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
pub struct Mapper554 {
    reg: u8,
}
impl Mapper554 {
    pub fn new() -> Self { Self { reg: 0 } }
    fn prg_read_bank(&self, bank: usize, address: u16, prg_rom: &[u8]) -> u8 {
        if prg_rom.is_empty() { return 0xFF; }
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        prg_rom[offset % prg_rom.len()]
    }
}
impl Mapper for Mapper554 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        match address {
            0xCAB6..=0xCAD7 => { self.reg = (address as u8 >> 2) & 0x0F; }
            0xEBE2 | 0xEBE3 | 0xEE32 | 0xEE33 => { self.reg = (address as u8 >> 2) & 0x0F; }
            0xFFFC | 0xFFFD => { self.reg = (address as u8 >> 2) & 0x0F; }
            _ => {}
        }
        match address {
            0x6000..=0x7FFF => FetchResult { data: self.prg_read_bank(self.reg as usize, address, &cart.prg_rom), driven: true },
            0x8000..=0x9FFF => FetchResult { data: self.prg_read_bank(0xA, address, &cart.prg_rom), driven: true },
            0xA000..=0xBFFF => FetchResult { data: self.prg_read_bank(0xB, address, &cart.prg_rom), driven: true },
            0xC000..=0xDFFF => FetchResult { data: self.prg_read_bank(0x6, address, &cart.prg_rom), driven: true },
            0xE000..=0xFFFF => FetchResult { data: self.prg_read_bank(0x7, address, &cart.prg_rom), driven: true },
            _ => FetchResult { data: 0, driven: false },
        }
    }
    fn store_prg(&mut self, _cart: &mut Cartridge, _address: u16, _data: u8) {}
    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.nametable_horizontal_mirroring { (address & 0x3FFF) | ((address & 0x0800) >> 1) } else { address & 0x37FF }
    }
    fn fetch_ppu(&mut self, _prg_rom: &[u8], chr_rom: &[u8], _prg_ram: &[u8], chr_ram: &[u8], _prg_vram: &[u8], using_chr_ram: bool, nh: bool, _alt: bool, ppu_addr: u16, ppu_latch: u8, vram: &[u8]) -> (u8, u16) {
        let address = (ppu_addr & 0x3F00) | ppu_latch as u16;
        let mut nab = ppu_addr & 0xFF00;
        if address < 0x2000 {
            let off = (address as usize) & 0x1FFF;
            if using_chr_ram {
                nab |= chr_ram.get(off).copied().unwrap_or(0) as u16;
            } else {
                let bank = self.reg as usize;
                let chr_off = bank * 0x2000 + off;
                nab |= chr_rom.get(chr_off % chr_rom.len().max(1)).copied().unwrap_or(0) as u16;
            }
        } else if address < 0x3F00 {
            let mir = if nh { (address & 0x3FFF) | ((address & 0x0800) >> 1) } else { address & 0x37FF };
            nab |= vram.get((mir & 0x7FF) as usize).copied().unwrap_or(0) as u16;
        }
        (nab as u8, nab)
    }
    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.reg]
    }
    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start < state.len() { self.reg = state[start]; start += 1; }
        start
    }
    fn reset(&mut self) { self.reg = 0; }
}
