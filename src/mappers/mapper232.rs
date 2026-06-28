use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper232 {
    outer_bank: u8,
    inner_bank: u8,
}

impl Mapper232 {
    pub fn new() -> Self {
        Self {
            outer_bank: 0,
            inner_bank: 0,
        }
    }

    fn sync(&self, _cart: &Cartridge, submapper: u8) -> (usize, usize) {
        let prg_bank_8000 = {
            let mut outer = self.outer_bank;
            if submapper == 1 {
                outer = ((outer & 0x8) >> 1) | ((outer & 0x4) << 1);
            }
            (outer as usize) | (self.inner_bank as usize)
        };
        let prg_bank_c000 = {
            let mut outer = self.outer_bank;
            if submapper == 1 {
                outer = ((outer & 0x8) >> 1) | ((outer & 0x4) << 1);
            }
            (outer as usize) | 0x3
        };
        (prg_bank_8000, prg_bank_c000)
    }
}

impl Mapper for Mapper232 {
    fn reset(&mut self) {
        self.outer_bank = 0;
        self.inner_bank = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let (bank_8000, bank_c000) = self.sync(cart, cart.sub_mapper);
            let prg_len = cart.prg_rom.len();
            if prg_len == 0 {
                return FetchResult { data: 0, driven: false };
            }
            let bank = if address < 0xC000 {
                bank_8000
            } else {
                bank_c000
            };
            let offset = (bank * 0x4000 + (address as usize & 0x3FFF)) % prg_len;
            FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 && address <= 0xBFFF {
            self.outer_bank = (data >> 1) & 0x0C;
        } else if address >= 0xC000 {
            self.inner_bank = data & 0x03;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        let norm = address & 0x2FFF;
        if cart.nametable_horizontal_mirroring {
            (norm & 0x33FF) | ((norm & 0x0800) >> 1)
        } else {
            norm & 0x37FF
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
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
            let len = chr_ram.len();
            if len == 0 {
                return (0, new_addr_bus);
            }
            let offset = (address as usize & 0x1FFF) % len;
            new_addr_bus |= chr_ram[offset] as u16;
        } else {
            let mirrored = if nametable_horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.outer_bank, self.inner_bank]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.outer_bank = state[p];
            p += 1;
        }
        if p < state.len() {
            self.inner_bank = state[p];
            p += 1;
        }
        p
    }
}
