// Mapper 378 - 8-in-1 AOROM+UNROM multicart (data latch, bus conflict AND)
//
// Reference: NintendulatorNRS-DBG multicart data latch/mapper378.cpp

use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper378 {
    data: u8,
}

impl Mapper378 {
    pub fn new() -> Self {
        Self { data: 0 }
    }

    fn aorom_mode(&self) -> bool {
        (self.data & 0x20) != 0
    }

    fn prg_offset(&self, cart: &Cartridge, address: u16) -> usize {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }

        if self.aorom_mode() {
            let num_16k = (len / 0x4000).max(1);
            let bank = if address < 0xC000 {
                0x10usize | ((self.data as usize) << 1 & 0x0E) | ((self.data as usize) >> 3 & 0x01)
            } else {
                0x10usize | ((self.data as usize) << 1 & 0x08) | 0x07
            };
            (bank % num_16k) * 0x4000 + (address as usize & 0x3FFF)
        } else {
            let num_32k = (len / 0x8000).max(1);
            let bank = (self.data as usize) & 0x07;
            (bank % num_32k) * 0x8000 + (address as usize & 0x7FFF)
        }
    }

    fn single_screen_high(&self) -> bool {
        !self.aorom_mode() && (self.data & 0x10) != 0
    }

    fn vram_index(&self, address: u16) -> usize {
        let off = (address & 0x03FF) as usize;
        if self.aorom_mode() {
            let horizontal = (self.data & 0x04) != 0;
            let page = if horizontal {
                if (address & 0x0800) != 0 {
                    1
                } else {
                    0
                }
            } else if (address & 0x0400) != 0 {
                1
            } else {
                0
            };
            (page << 10) | off
        } else if self.single_screen_high() {
            0x400 | off
        } else {
            off
        }
    }
}

impl Mapper for Mapper378 {
    fn reset(&mut self) {
        self.data = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            let offset = self.prg_offset(cart, address);
            FetchResult {
                data: if len > 0 {
                    cart.prg_rom[offset % len]
                } else {
                    0
                },
                driven: true,
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            let rom_data = if len > 0 {
                cart.prg_rom[self.prg_offset(cart, address) % len]
            } else {
                0xFF
            };
            self.data = data & rom_data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.aorom_mode() {
            mirror_h_or_v((self.data & 0x04) != 0, address)
        } else if self.single_screen_high() {
            0x2400 | (address & 0x03FF)
        } else {
            0x2000 | (address & 0x03FF)
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
            let idx = self.vram_index(address);
            new_addr_bus |= vram[idx % vram.len().max(1)] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize) % len] = data;
            }
        } else if address < 0x3F00 {
            let idx = self.vram_index(address);
            let len = vram.len();
            if len > 0 {
                vram[idx % len] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.data]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start < state.len() {
            self.data = state[start];
            start + 1
        } else {
            start
        }
    }
}
