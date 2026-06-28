use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper36 {
    latch: u8,
    mirror_vertical: bool,
}

impl Mapper36 {
    pub fn new() -> Self {
        Self {
            latch: 0,
            mirror_vertical: true, 
        }
    }
}

impl Mapper for Mapper36 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = (self.latch >> 4) as usize;
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if address == 0x4100 {
            FetchResult {
                data: self.latch,
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 && address <= 0xFFFE {
            match (address >> 12) & 7 {
                0 => self.mirror_vertical = true,  
                4 => self.mirror_vertical = false, 
                _ => {}
            }
            self.latch = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mirror_vertical {
            address & 0x37FF 
        } else {
            (address & 0x33FF) | ((address & 0x0800) >> 1) 
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
            let bank = (self.latch & 0xF) as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            if using_chr_ram {
                if !chr_ram.is_empty() {
                    new_addr_bus |= chr_ram[offset % chr_ram.len()] as u16;
                }
            } else if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else {
            let mirrored = if self.mirror_vertical {
                address & 0x37FF 
            } else {
                (address & 0x33FF) | ((address & 0x0800) >> 1) 
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.latch, if self.mirror_vertical { 1 } else { 0 }]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        self.latch = state[start];
        self.mirror_vertical = state[start + 1] != 0;
        start + 2
    }

    fn reset(&mut self) {
        self.latch = 0;
        self.mirror_vertical = true;
    }
}
