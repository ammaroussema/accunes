// Mapper 402 - J-2282 (multicart address latch)
//
// Reference: NintendulatorNRS-DBG multicart address latch/mapper402.cpp
//
// Any CPU write to $8000-$FFFF latches the full write address. The latched
// address bits drive PRG banking (bit 0x40 selects 16KB/32KB, bits 0-4 the
// bank), an FDS-slot ROM window at $6000-$6FFF (bit 0x800), CHR-RAM write
// protection (bit 0x400) and mirroring (bit 0x80).

use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper402 {
    latch_addr: u16,
}

impl Mapper402 {
    pub fn new() -> Self {
        Self { latch_addr: 0 }
    }

    fn prg(&self) -> u16 {
        self.latch_addr & 0x1F
    }

    fn mirror_address(&self, address: u16) -> u16 {
        if self.latch_addr & 0x80 != 0 {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }
}

impl Mapper for Mapper402 {
    fn reset(&mut self) {
        self.latch_addr = 0;
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
            let data = if self.latch_addr & 0x40 != 0 {
                let bank = self.prg() as usize;
                cart.prg_rom[(bank * 0x4000 + (address as usize & 0x3FFF)) % len]
            } else {
                let bank = (self.prg() >> 1) as usize;
                cart.prg_rom[(bank * 0x8000 + (address as usize & 0x7FFF)) % len]
            };
            FetchResult {
                data,
                driven: true,
            }
        } else if address >= 0x6000 {
            if self.latch_addr & 0x800 != 0 && address < 0x7000 {
                let len = cart.prg_rom.len();
                if len == 0 {
                    return FetchResult {
                        data: 0,
                        driven: false,
                    };
                }
                let bank = ((self.prg() << 1) | 3) as usize;
                FetchResult {
                    data: cart.prg_rom[(bank * 0x2000 + (address as usize & 0x1FFF)) % len],
                    driven: true,
                }
            } else {
                FetchResult {
                    data: 0,
                    driven: false,
                }
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, _data: u8) {
        if address >= 0x8000 {
            self.latch_addr = address;
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
            let offset = address as usize & 0x1FFF;
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
            if self.latch_addr & 0x400 != 0 {
                let offset = address as usize & 0x1FFF;
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
        self.latch_addr.to_le_bytes().to_vec()
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start + 1 < state.len() {
            self.latch_addr = u16::from_le_bytes([state[start], state[start + 1]]);
        }
        start + 2
    }
}
