use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper179 {
    reg: u8,
}

impl Mapper179 {
    pub fn new() -> Self {
        Self { reg: 0 }
    }
}

impl Mapper for Mapper179 {
    fn reset(&mut self) {
        self.reg = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            let bank = (self.reg as usize >> 1) * 0x8000;
            let offset = bank + (address as usize & 0x7FFF);
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

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.reg & 0x01 != 0 {
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
            let byte = if !chr_rom.is_empty() {
                chr_rom[address as usize % chr_rom.len()]
            } else if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[address as usize % chr_ram.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = self.mirror_nametable_raw(address);
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let chr_len = cart.chr_ram.len();
            if chr_len > 0 {
                cart.chr_ram[address as usize % chr_len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable_raw(address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.reg]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        self.reg = state[start];
        start + 1
    }
}

impl Mapper179 {
    fn mirror_nametable_raw(&self, address: u16) -> u16 {
        if self.reg & 0x01 != 0 {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }
}
