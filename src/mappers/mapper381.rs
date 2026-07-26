// Mapper 381 - KN-42 (multicart latch)
//
// Reference: NintendulatorNRS-DBG multicart multiple regs/mapper381.cpp

use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper381 {
    latch: u8,
    game: u8,
    first_reset: bool,
}

impl Mapper381 {
    pub fn new() -> Self {
        Self { latch: 0, game: 0, first_reset: true }
    }

    fn prg_bank_lo(&self) -> usize {
        let bank = (((self.latch as u16) << 1) | ((self.latch as u16) >> 4)) as u8;
        ((bank & 0x0F) | (self.game << 4)) as usize
    }

    fn prg_bank_hi(&self) -> usize {
        0x0F | ((self.game as usize) << 4)
    }

    fn prg_read(&self, cart: &Cartridge, address: u16) -> u8 {
        let prg_len = cart.prg_rom.len();
        if prg_len == 0 {
            return 0;
        }
        let bank = if address < 0xC000 {
            self.prg_bank_lo()
        } else {
            self.prg_bank_hi()
        };
        let offset = (bank * 0x4000 + (address as usize & 0x3FFF)) % prg_len;
        cart.prg_rom[offset]
    }
}

impl Mapper for Mapper381 {
    fn reset(&mut self) {
        self.latch = 0;
        if self.first_reset {
            self.first_reset = false;
            self.game = 0;
        } else {
            self.game = self.game.wrapping_add(1);
        }
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            return FetchResult {
                data: self.prg_read(cart, address),
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.latch = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v(cart.nametable_horizontal_mirroring, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = if !chr_ram.is_empty() {
                chr_ram[(address as usize) % chr_ram.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let idx = address as usize & 0x7FF;
            new_addr_bus |= vram[idx % vram.len().max(1)] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize) % len] = data;
            }
        } else if address < 0x3F00 {
            let idx = address as usize & 0x7FF;
            let len = vram.len();
            if len > 0 {
                vram[idx % len] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.latch, self.game, if self.first_reset { 1 } else { 0 }]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start < state.len() {
            self.latch = state[start];
        }
        if start + 1 < state.len() {
            self.game = state[start + 1];
        }
        if start + 2 < state.len() {
            self.first_reset = state[start + 2] != 0;
        }
        start + 3
    }
}
