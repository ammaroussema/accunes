// Mapper 379 - 35BH-1 multicart (data latch)
//
// Reference: NintendulatorNRS-DBG multicart data latch/mapper379.cpp

use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::uxrom::mirror_address;

pub struct Mapper379 {
    data: u8,
}

impl Mapper379 {
    pub fn new() -> Self {
        Self { data: 0 }
    }

    fn prg_read(&self, cart: &Cartridge, address: u16) -> u8 {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }
        let banks_32k = (len / 0x8000).max(1);
        let bank = (self.data as usize & 0x03) % banks_32k;
        let offset = bank * 0x8000 + (address as usize & 0x7FFF);
        cart.prg_rom[offset % len]
    }

    fn chr_read(
        &self,
        address: u16,
        chr_rom: &[u8],
        chr_ram: &[u8],
        using_chr_ram: bool,
    ) -> u8 {
        let len = if using_chr_ram {
            chr_ram.len()
        } else {
            chr_rom.len()
        };
        if len == 0 {
            return 0;
        }
        let banks_8k = (len / 0x2000).max(1);
        let bank = (self.data as usize >> 2) % banks_8k;
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        if using_chr_ram {
            chr_ram[offset % len]
        } else {
            chr_rom[offset % len]
        }
    }
}

impl Mapper for Mapper379 {
    fn reset(&mut self) {
        self.data = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            FetchResult {
                data: self.prg_read(cart, address),
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
            self.data = data;
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
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = self.chr_read(address, chr_rom, chr_ram, using_chr_ram);
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
        if address < 0x2000 && cart.using_chr_ram && !cart.chr_ram.is_empty() {
            let len = cart.chr_ram.len();
            let banks_8k = (len / 0x2000).max(1);
            let bank = (self.data as usize >> 2) % banks_8k;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            cart.chr_ram[offset % len] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
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
