use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::uxrom::mirror_address;

pub struct Mapper271 {
    latch: u8,
}

impl Mapper271 {
    pub fn new() -> Self {
        Self { latch: 0 }
    }
}

impl Mapper for Mapper271 {
    fn reset(&mut self) {
        self.latch = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let banks_32k = (len / 0x8000).max(1);
            let bank = ((self.latch >> 4) as usize) % banks_32k;
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.latch = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if (self.latch & 0x20) != 0 {
            address & 0x37FF
        } else {
            mirror_address(
                cart.alternative_nametable_arrangement,
                cart.nametable_horizontal_mirroring,
                address,
            )
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
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let len = if using_chr_ram { chr_ram.len() } else { chr_rom.len() };
            if len == 0 {
                new_addr_bus |= 0;
            } else {
                let banks_8k = (len / 0x2000).max(1);
                let bank = ((self.latch & 0x0F) as usize) % banks_8k;
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                let byte = if using_chr_ram { chr_ram[offset] } else { chr_rom[offset % chr_rom.len()] };
                new_addr_bus |= byte as u16;
            }
        } else {
            let mirrored = if self.latch & 0x20 != 0 {
                address & 0x37FF
            } else {
                mirror_address(
                    alternative_nametable_arrangement,
                    _nametable_horizontal_mirroring,
                    address,
                )
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                let banks_8k = (len / 0x2000).max(1);
                let bank = ((self.latch & 0x0F) as usize) % banks_8k;
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = if self.latch & 0x20 != 0 {
                address & 0x37FF
            } else {
                mirror_address(
                    cart.alternative_nametable_arrangement,
                    cart.nametable_horizontal_mirroring,
                    address,
                )
            };
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
