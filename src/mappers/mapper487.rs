use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

pub struct Mapper487 {
    reg: [u8; 2],
}

impl Mapper487 {
    pub fn new() -> Self {
        Self { reg: [0; 2] }
    }

    fn prg_bank(&self) -> u8 {
        let mut prg = if (self.reg[1] & 0x40) != 0 {
            (self.reg[0] >> 3) & 1
        } else {
            self.reg[1] & 0x01
        };
        prg |= self.reg[1] & 0x3E;
        if prg & 0x30 != 0 {
            prg -= 0x10;
        }
        prg
    }

    fn chr_bank(&self) -> u16 {
        let mut chr = (self.reg[0] & 0x03) as u16;
        if (self.reg[1] & 0x40) != 0 {
            chr |= (self.reg[0] & 0x04) as u16;
        } else {
            chr |= ((self.reg[1] as u16) << 2) & 4;
        }
        chr |= ((self.reg[1] as u16) << 2) & 0xF8;
        if chr & 0xC0 != 0 {
            chr -= 0x40;
        }
        chr
    }
}

impl Mapper for Mapper487 {
    fn reset(&mut self) {
        *self = Self::new();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            let offset = (self.prg_bank() as usize) * 0x8000 + (address as usize & 0x7FFF);
            return FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if (0x4000..0x6000).contains(&address) {
            if (address & 0x100) != 0 {
                if (address & 0x80) != 0 {
                    self.reg[1] = data;
                } else if (self.reg[1] & 0x20) == 0 {
                    self.reg[0] = data;
                }
            }
        } else if address >= 0x8000 {
            if (self.reg[1] & 0x20) != 0 {
                self.reg[0] = ((data << 3) & 8) | ((data >> 4) & 7);
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v((self.reg[1] & 0x80) != 0, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        _chr_ram: &[u8],
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
            let offset = (self.chr_bank() as usize) * 0x2000 + (address as usize & 0x1FFF);
            let byte = if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mirror_h_or_v((self.reg[1] & 0x80) != 0, address);
            let byte = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, _cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if (0x2000..0x3F00).contains(&address) {
            let mirrored = mirror_h_or_v((self.reg[1] & 0x80) != 0, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.reg[0], self.reg[1]]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.reg[0] = state.get(p).copied().unwrap_or(0);
        p += 1;
        self.reg[1] = state.get(p).copied().unwrap_or(0);
        p += 1;
        p
    }
}
