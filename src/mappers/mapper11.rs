use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::uxrom::mirror_address;

pub struct Mapper11 {
    latch: u8,
}

impl Mapper11 {
    pub fn new() -> Self {
        Self { latch: 0 }
    }

    fn prg_read(&self, cart: &Cartridge, address: u16) -> u8 {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }
        let banks_32k = (len / 0x8000).max(1);
        let bank = (self.latch & 0x0F) as usize % banks_32k;
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
        let bank = ((self.latch >> 4) & 0x0F) as usize % banks_8k;
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        if using_chr_ram {
            chr_ram[offset]
        } else {
            chr_rom[offset]
        }
    }
}

impl Mapper for Mapper11 {
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
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                let banks_8k = (len / 0x2000).max(1);
                let bank = ((self.latch >> 4) & 0x0F) as usize % banks_8k;
                let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % len;
                cart.chr_ram[offset] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = mirror_address(
                cart.alternative_nametable_arrangement,
                cart.nametable_horizontal_mirroring,
                address,
            );
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn reset(&mut self) {
        self.latch = 0;
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
