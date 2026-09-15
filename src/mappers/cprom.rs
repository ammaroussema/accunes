use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::uxrom::mirror_address;

pub struct MapperCpROM {
    latch: u8,
}

impl MapperCpROM {
    pub fn new() -> Self {
        Self { latch: 0 }
    }

    fn chr_read(&self, address: u16, chr_rom: &[u8], chr_ram: &[u8]) -> u8 {
        let use_ram = !chr_ram.is_empty();
        let len = if use_ram {
            chr_ram.len()
        } else {
            chr_rom.len()
        };
        if len == 0 {
            return 0;
        }
        let offset = if address < 0x1000 {
            address as usize & 0x0FFF
        } else {
            let bank = (self.latch & 3) as usize;
            bank * 0x1000 + (address as usize & 0x0FFF)
        };
        if use_ram {
            chr_ram[offset % len]
        } else {
            chr_rom[offset % len]
        }
    }

    fn chr_write_offset(&self, address: u16, len: usize) -> usize {
        if address < 0x1000 {
            address as usize & 0x0FFF
        } else {
            let bank = (self.latch & 3) as usize;
            (bank * 0x1000 + (address as usize & 0x0FFF)) % len
        }
    }
}

impl Mapper for MapperCpROM {
    fn reset(&mut self) {
        self.latch = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            let data = if len == 0 {
                0
            } else {
                cart.prg_rom[(address as usize & 0x7FFF) % len]
            };
            FetchResult { data, driven: true }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.latch = data;
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
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = self.chr_read(address, chr_rom, chr_ram);
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mirror_address(
                alternative_nametable_arrangement,
                nametable_horizontal_mirroring,
                address,
            );
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && !cart.chr_ram.is_empty() {
            let len = cart.chr_ram.len();
            cart.chr_ram[self.chr_write_offset(address, len)] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = mirror_address(
                cart.alternative_nametable_arrangement,
                cart.nametable_horizontal_mirroring,
                address,
            );
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.latch]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start < state.len() {
            self.latch = state[start];
            start + 1
        } else {
            start
        }
    }
}
