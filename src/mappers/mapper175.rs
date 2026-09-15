use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper175 {
    reg: u8,
    new_reg: u8,
    mirroring: u8,
    prg_ram: [u8; 0x2000],
}

impl Mapper175 {
    pub fn new() -> Self {
        Self { reg: 0, new_reg: 0, mirroring: 0, prg_ram: [0; 0x2000] }
    }
}

impl Mapper for Mapper175 {
    fn reset(&mut self) {
        self.reg = 0;
        self.new_reg = 0;
        self.mirroring = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            FetchResult { data: self.prg_ram[(address & 0x1FFF) as usize], driven: true }
        } else if address >= 0x8000 {
            if address >= 0xFFF0 {
                if self.reg != self.new_reg {
                    self.reg = self.new_reg;
                }
            }
            let len = cart.prg_rom.len();
            let bank = (self.reg as usize) * 0x4000;
            let offset = bank + (address as usize & 0x3FFF);
            FetchResult { data: if len > 0 { cart.prg_rom[offset % len] } else { 0 }, driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            self.prg_ram[(address & 0x1FFF) as usize] = data;
        } else if address >= 0x8000 {
            if address <= 0x8FFF {
                self.mirroring = data;
            } else if address >= 0xA000 && address <= 0xAFFF {
                self.new_reg = data & 0x0F;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mirroring & 4 != 0 {
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
            let bank = (self.reg as usize) * 0x2000 + (address as usize & 0x1FFF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[bank % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[bank % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if self.mirroring & 4 != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, _cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address >= 0x2000 && address < 0x3F00 {
            let mirrored = if self.mirroring & 4 != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut s = Vec::new();
        s.push(self.reg); s.push(self.new_reg); s.push(self.mirroring);
        s.extend_from_slice(&self.prg_ram);
        s
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.reg = state[p]; p += 1;
        self.new_reg = state[p]; p += 1;
        self.mirroring = state[p]; p += 1;
        for b in self.prg_ram.iter_mut() { if p < state.len() { *b = state[p]; p += 1; } }
        p
    }
}
