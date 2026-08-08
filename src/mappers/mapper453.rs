use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

pub struct Mapper453 {
    latch_data: u8,
}

impl Mapper453 {
    pub fn new() -> Self {
        Self { latch_data: 0 }
    }

    fn mirror_addr(&self, address: u16) -> u16 {
        let d = self.latch_data;
        if d & 0x40 != 0 {
            if d & 0x10 != 0 {
                address & 0x23FF
            } else {
                (address & 0x23FF) | 0x0400
            }
        } else {
            mirror_h_or_v(d & 0x10 != 0, address)
        }
    }
}

impl Mapper for Mapper453 {
    fn reset(&mut self) {
        self.latch_data = 0;
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
            let d = self.latch_data;
            let (bank, mask) = if d & 0x40 != 0 {
                (((d & 7) | ((d >> 3) & 0x18)) as usize, 0x7FFF)
            } else {
                let bank = if address < 0xC000 {
                    (d & 7) | ((d >> 2) & 0x38)
                } else {
                    7 | ((d >> 2) & 0x38)
                };
                (bank as usize, 0x3FFF)
            };
            let offset = bank * (mask + 1) + (address as usize & mask);
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
        if address >= 0x8000 {
            if self.latch_data & 0xE0 != 0 {
                self.latch_data = (self.latch_data & 0xE0) | (data & 0x1F);
            } else {
                self.latch_data = data;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror_addr(address)
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
                chr_ram[(address as usize & 0x1FFF) % chr_ram.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = self.mirror_addr(address);
            let byte = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.latch_data]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start < state.len() {
            self.latch_data = state[start];
            start + 1
        } else {
            start
        }
    }
}
