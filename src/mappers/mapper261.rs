use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper261 {
    pub addr: u16,
}

impl Mapper261 {
    pub fn new() -> Self {
        Self { addr: 0 }
    }
}

impl Mapper for Mapper261 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let prg_bank = (((self.addr >> 6) & 0x0E) | ((self.addr >> 5) & 0x01)) as usize;
            let nrom256 = (self.addr & 0x40) != 0;

            let bank = if nrom256 {
                if address < 0xC000 {
                    prg_bank & 0xFE
                } else {
                    prg_bank | 0x01
                }
            } else {
                prg_bank
            };

            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, _data: u8) {
        if address >= 0x8000 {
            self.addr = address;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let mirror_h = (self.addr & 0x10) != 0;
        if mirror_h {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
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
            let bank = (self.addr & 0x0F) as usize;
            if using_chr_ram {
                new_addr_bus |= chr_ram[(bank * 0x2000 + (address as usize & 0x1FFF)) % chr_ram.len()] as u16;
            } else {
                new_addr_bus |= chr_rom[(bank * 0x2000 + (address as usize & 0x1FFF)) % chr_rom.len()] as u16;
            }
        } else {
            let mirror_h = (self.addr & 0x10) != 0;
            let mirrored = if mirror_h {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![
            (self.addr & 0xFF) as u8,
            ((self.addr >> 8) & 0xFF) as u8,
        ]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        self.addr = state[start] as u16 | ((state[start + 1] as u16) << 8);
        start + 2
    }
}
