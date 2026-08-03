// Mapper 400 - 8BIT-XMAS (multicart address latch + 4 write-only registers)
//
// Reference: NintendulatorNRS-DBG multicart multiple regs/mapper400.cpp
//
// Writes to $C000-$FFFF latch the data byte (Latch). Writes to
// $6000/$6800/$7000/$7800 (only when addr & 0x7FF == 0) set four registers.
// Writes to $8000-$BFFF only drive the cart's cosmetic LED label display,
// which has no effect on banking. PRG is two 16KB banks; CHR is 8KB of
// CHR RAM selected by bits 5+ of the latch. Mirroring comes from reg[3] bit 5.

use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper400 {
    reg: [u8; 4],
    latch_data: u8,
}

impl Mapper400 {
    pub fn new() -> Self {
        Self {
            reg: [0, 0, 0, 0],
            latch_data: 0,
        }
    }

    fn mirror_address(&self, address: u16) -> u16 {
        if self.reg[3] & 0x20 != 0 {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }
}

impl Mapper for Mapper400 {
    fn reset(&mut self) {
        self.reg[2] = 0;
        self.reg[3] = 0x80;
        self.latch_data = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank16 = if address < 0xC000 {
                (self.latch_data & 7) | (self.reg[2] & 0xF8)
            } else {
                7 | (self.reg[3] & 0xF8)
            };
            let len = cart.prg_rom.len();
            let data = if len == 0 {
                0
            } else {
                let offset = (bank16 as usize * 0x4000 + (address as usize & 0x3FFF)) % len;
                cart.prg_rom[offset]
            };
            FetchResult {
                data,
                driven: true,
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0xC000 {
            self.latch_data = data;
        } else if (0x6000..0x8000).contains(&address) && (address & 0x7FF) == 0 {
            let bank = address >> 12;
            let addr = address & 0xFFF;
            let index = (((bank << 1) & 2) | ((addr >> 11) & 1)) as usize;
            self.reg[index] = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror_address(address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
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
            let bank = (self.latch_data >> 5) as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let byte = if !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = self.mirror_address(address);
            let byte = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            let bank = (self.latch_data >> 5) as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_address(address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = vec![self.latch_data];
        state.extend_from_slice(&self.reg);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.latch_data = state[p];
            p += 1;
        }
        for i in 0..4 {
            if p < state.len() {
                self.reg[i] = state[p];
                p += 1;
            }
        }
        p
    }
}
