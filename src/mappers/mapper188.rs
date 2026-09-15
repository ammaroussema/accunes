use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper188 {
    reg: u8,
}

impl Mapper188 {
    pub fn new() -> Self {
        Self { reg: 0 }
    }
}

impl Mapper for Mapper188 {
    fn reset(&mut self) {
        self.reg = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            FetchResult { data: 0x07, driven: true }
        } else if address >= 0x8000 {
            let len = cart.prg_rom.len();
            let bank = if address < 0xC000 {
                if self.reg & 0x10 != 0 { (self.reg & 0x07) as usize }
                else { (self.reg | 0x08) as usize }
            } else {
                7usize
            };
            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            FetchResult { data: if len > 0 { cart.prg_rom[offset % len] } else { 0 }, driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x6000 {
            self.reg = data;
        } else if address >= 0x8000 {
            self.reg = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.nametable_horizontal_mirroring {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[address as usize % chr_ram.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement { address }
                else if nametable_horizontal_mirroring { (address & 0x33FF) | ((address & 0x0800) >> 1) }
                else { address & 0x37FF };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.reg]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        self.reg = state[start];
        start + 1
    }
}
