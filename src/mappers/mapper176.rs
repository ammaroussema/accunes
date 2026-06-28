use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper176 {
    prg: [u8; 4],
    chr: u8,
    sbw: bool,
    prg_ram: [u8; 0x2000],
}

impl Mapper176 {
    pub fn new() -> Self {
        Self { prg: [0, 1, 0xFE, 0xFF], chr: 0, sbw: false, prg_ram: [0; 0x2000] }
    }
}

impl Mapper for Mapper176 {
    fn reset(&mut self) {
        self.prg = [0, 1, 0xFE, 0xFF];
        self.chr = 0;
        self.sbw = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            FetchResult { data: self.prg_ram[(address & 0x1FFF) as usize], driven: true }
        } else if address >= 0x8000 {
            let len = cart.prg_rom.len();
            let slot = ((address - 0x8000) / 0x2000) as usize;
            let bank = (self.prg[slot] as usize) * 0x2000;
            let offset = bank + (address as usize & 0x1FFF);
            FetchResult { data: if len > 0 { cart.prg_rom[offset % len] } else { 0 }, driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            self.prg_ram[(address & 0x1FFF) as usize] = data;
        } else if address >= 0x5000 && address < 0x6000 {
            let addr = address & 0xFFF;
            match addr {
                0x001 => {
                    if self.sbw {
                        let v = data;
                        self.prg[0] = v * 4;
                        self.prg[1] = v * 4 + 1;
                        self.prg[2] = v * 4 + 2;
                        self.prg[3] = v * 4 + 3;
                    }
                }
                0x010 => {
                    if data == 0x24 { self.sbw = true; }
                }
                0x011 => {
                    if self.sbw {
                        let v = data >> 1;
                        self.prg[0] = v * 4;
                        self.prg[1] = v * 4 + 1;
                        self.prg[2] = v * 4 + 2;
                        self.prg[3] = v * 4 + 3;
                    }
                }
                0xFF1 => {
                    let v = data >> 1;
                    self.prg[0] = v * 4;
                    self.prg[1] = v * 4 + 1;
                    self.prg[2] = v * 4 + 2;
                    self.prg[3] = v * 4 + 3;
                }
                0xFF2 => {
                    self.chr = data;
                }
                _ => {}
            }
        } else if address == 0xA001 {
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        address & 0x37FF
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
            let bank = (self.chr as usize) * 0x2000 + (address as usize & 0x1FFF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[bank % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[bank % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            new_addr_bus |= vram[(address & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, _cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address >= 0x2000 && address < 0x3F00 {
            vram[(address & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut s = Vec::new();
        s.extend_from_slice(&self.prg);
        s.push(self.chr);
        s.push(if self.sbw { 1 } else { 0 });
        s.extend_from_slice(&self.prg_ram);
        s
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for b in self.prg.iter_mut() { if p < state.len() { *b = state[p]; p += 1; } }
        if p < state.len() { self.chr = state[p]; p += 1; }
        if p < state.len() { self.sbw = state[p] != 0; p += 1; }
        for b in self.prg_ram.iter_mut() { if p < state.len() { *b = state[p]; p += 1; } }
        p
    }
}
