use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper580 {
    addr: u8,
}

impl Mapper580 {
    pub fn new() -> Self {
        Self { addr: 0 }
    }
}

impl Mapper for Mapper580 {
    fn reset(&mut self) {
        self.addr = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if cart.prg_rom.is_empty() {
            return FetchResult { data: 0, driven: true };
        }
        let data = if self.addr & 0x01 != 0 {
            let page = (self.addr >> 5) as usize;
            let offset = page * 0x8000 + (address as usize & 0x7FFF);
            cart.prg_rom[offset % cart.prg_rom.len()]
        } else {
            let page = (self.addr >> 4) as usize;
            let offset = page * 0x4000 + (address as usize & 0x3FFF);
            cart.prg_rom[offset % cart.prg_rom.len()]
        };
        FetchResult { data, driven: true }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.addr = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.addr & 0x02 != 0 {
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
        _chr_ram: &[u8],
        prg_vram: &[u8],
        _using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = if self.addr & 0x01 != 0 {
                ((self.addr >> 3) as u16) & !3 | ((self.addr >> 2) as u16) & 3
            } else {
                ((self.addr >> 3) as u16) & !1 | ((self.addr >> 2) as u16) & 1
            };
            let offset = (bank as usize) * 0x0800 + (address as usize & 0x07FF);
            let byte = if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if nametable_horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() { prg_vram[idx] } else { 0 }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.addr]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.addr = state[p];
            p += 1;
        }
        p
    }
}
