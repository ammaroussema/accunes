use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::cnrom::{mirror_address, read_prg_fixed};

#[derive(Clone, Debug)]
pub struct Mapper185Config {
    pub cs_enable: Option<u8>,
}

impl Mapper185Config {
    pub fn for_ines(header: &[u8], sub_mapper: u8) -> Self {
        let nes2 = header.len() >= 16 && (header[7] & 0x0C) == 0x08;
        let cs_enable = if nes2 && (4..=7).contains(&sub_mapper) {
            Some(sub_mapper - 4)
        } else {
            None
        };
        Self { cs_enable }
    }
}

pub struct Mapper185 {
    config: Mapper185Config,
    datareg: u8,
}

impl Mapper185 {
    pub fn new(config: Mapper185Config) -> Self {
        Self { config, datareg: 0 }
    }

    fn chr_enabled(&self) -> bool {
        match self.config.cs_enable {
            Some(bank) => (self.datareg & 3) == bank,
            None => (self.datareg & 3) != 0 && self.datareg != 0x13,
        }
    }
}

impl Mapper for Mapper185 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            FetchResult {
                data: read_prg_fixed(cart, address),
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
            self.datareg = data & read_prg_fixed(cart, address);
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        mirror_address(
            cart.alternative_nametable_arrangement,
            cart.nametable_horizontal_mirroring,
            address,
        )
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        _chr_ram: &[u8],
        prg_vram: &[u8],
        _using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            if self.chr_enabled() && !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[address as usize & 0x1FFF] as u16;
            } else {
                new_addr_bus |= 0xFF;
            }
        } else {
            let mirrored = mirror_address(
                alternative_nametable_arrangement,
                nametable_horizontal_mirroring,
                address,
            );
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() {
                    prg_vram[idx]
                } else {
                    0
                }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.datareg]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        self.datareg = state[start];
        start + 1
    }

    fn reset(&mut self) {
        self.datareg = 0;
    }
}
