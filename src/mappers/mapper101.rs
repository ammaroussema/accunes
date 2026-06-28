use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper101 {
    reg: u8,
}

impl Mapper101 {
    pub fn new() -> Self {
        Self { reg: 0 }
    }

    fn chr_read(
        &self,
        address: u16,
        chr_rom: &[u8],
        chr_ram: &[u8],
        using_chr_ram: bool,
    ) -> u8 {
        let len = if using_chr_ram { chr_ram.len() } else { chr_rom.len() };
        if len == 0 {
            return 0;
        }
        let bank = self.reg as usize;
        let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % len;
        if using_chr_ram {
            chr_ram[offset]
        } else {
            chr_rom[offset]
        }
    }

    fn mirror_addr(horizontal: bool, address: u16) -> u16 {
        let norm = address & 0x2FFF;
        if horizontal {
            (norm & 0x33FF) | ((norm & 0x0800) >> 1)
        } else {
            norm & 0x37FF
        }
    }
}

impl Mapper for Mapper101 {
    fn reset(&mut self) {
        self.reg = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let prg_len = cart.prg_rom.len();
            if prg_len == 0 {
                return FetchResult { data: 0, driven: false };
            }
            let offset = (address as usize & 0x7FFF) % prg_len;
            return FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            self.reg = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        Self::mirror_addr(cart.nametable_horizontal_mirroring, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let data = self.chr_read(address, chr_rom, chr_ram, using_chr_ram);
            new_addr_bus |= data as u16;
        } else {
            let mirrored = Self::mirror_addr(nametable_horizontal_mirroring, address);
            let data = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= data as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.reg]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start < state.len() {
            self.reg = state[start];
            start + 1
        } else {
            start
        }
    }
}
