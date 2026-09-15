use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper335 {
    reg8: u8,
    regc: u8,
}

impl Mapper335 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self { reg8: 0, regc: 0 }
    }
}

impl Mapper for Mapper335 {
    fn reset(&mut self) {
        self.reg8 = 0;
        self.regc = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if (self.regc & 0x10) != 0 {
                let bank = ((self.regc & 0x07) as usize) << 1 | ((self.regc >> 3) as usize & 1);
                let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                return FetchResult {
                    data: cart.prg_rom[offset % cart.prg_rom.len().max(1)],
                    driven: true,
                };
            } else {
                let bank = (self.regc & 0x07) as usize;
                let offset = bank * 0x8000 + (address as usize & 0x7FFF);
                return FetchResult {
                    data: cart.prg_rom[offset % cart.prg_rom.len().max(1)],
                    driven: true,
                };
            }
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 && address < 0xC000 {
            self.reg8 = data;
        } else if address >= 0xC000 {
            self.regc = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v((self.regc & 0x20) != 0, address)
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
            let bank = self.reg8 as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            new_addr_bus |= vram[(mirror_h_or_v((self.regc & 0x20) != 0, address) & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let bank = self.reg8 as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.reg8, self.regc]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.reg8 = state[p];
            p += 1;
        }
        if p < state.len() {
            self.regc = state[p];
            p + 1
        } else { p }
    }
}
