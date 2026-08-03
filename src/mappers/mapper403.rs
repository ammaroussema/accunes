// Mapper 403 - 89433 (multicart multiple regs)
//
// Reference: NintendulatorNRS-DBG multicart multiple regs/mapper403.cpp
//
// Four write-only registers at $x100-$x103 within $4000-$7FFF, gated on
// address bit 8. PRG is a single 16KB bank (reg[2] bit 0) or one 32KB bank
// (reg[0] >> 1 or >> 2). CHR is 32KB of CHR RAM banked in 8KB units by
// reg[1]; CPU writes to $8000-$FFFF also set reg[1] when reg[2] bit 2 is set.
// reg[2] bit 4 selects horizontal mirroring and write-protects the CHR RAM.

use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper403 {
    reg: [u8; 4],
}

impl Mapper403 {
    pub fn new() -> Self {
        Self { reg: [0; 4] }
    }

    fn mirror_address(&self, address: u16) -> u16 {
        if self.reg[2] & 0x10 != 0 {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn chr_offset(&self, address: u16) -> usize {
        (self.reg[1] as usize & 3) * 0x2000 + (address as usize & 0x1FFF)
    }
}

impl Mapper for Mapper403 {
    fn reset(&mut self) {
        self.reg = [0; 4];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: false,
                };
            }
            let data = if self.reg[2] & 0x01 != 0 {
                let bank = (self.reg[0] >> 1) as usize;
                cart.prg_rom[(bank * 0x4000 + (address as usize & 0x3FFF)) % len]
            } else {
                let bank = (self.reg[0] >> 2) as usize;
                cart.prg_rom[(bank * 0x8000 + (address as usize & 0x7FFF)) % len]
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
        if address >= 0x8000 {
            if self.reg[2] & 0x04 != 0 {
                self.reg[1] = data;
            }
        } else if address & 0x100 != 0 {
            self.reg[address as usize & 3] = data;
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
            let offset = self.chr_offset(address);
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
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if self.reg[2] & 0x10 == 0 {
                let offset = self.chr_offset(address);
                let len = cart.chr_ram.len();
                if len > 0 {
                    cart.chr_ram[offset % len] = data;
                }
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_address(address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        self.reg.to_vec()
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..4 {
            if p < state.len() {
                self.reg[i] = state[p];
                p += 1;
            }
        }
        p
    }
}
