use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper352 {
    game: u8,
}

impl Mapper352 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self { game: 0 }
    }
}

impl Mapper for Mapper352 {
    fn reset(&mut self) {
        self.game = self.game.wrapping_add(1);
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = self.game as usize;
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            return FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len().max(1)],
                driven: true,
            };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, _address: u16, _data: u8) {}

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.sub_mapper == 1 {
            mirror_h_or_v(cart.nametable_horizontal_mirroring, address)
        } else if (self.game & 1) != 0 {
            mirror_h_or_v(false, address)
        } else {
            mirror_h_or_v(true, address)
        }
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
            let bank = self.game as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            new_addr_bus |= (if !chr_rom.is_empty() { chr_rom[offset % chr_rom.len()] } else { 0 }) as u16;
        } else {
            let mir = if (self.game & 1) != 0 {
                mirror_h_or_v(false, address)
            } else {
                mirror_h_or_v(true, address)
            };
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, _cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address >= 0x2000 && address < 0x3F00 {
            let mir = if (self.game & 1) != 0 {
                mirror_h_or_v(false, address)
            } else {
                mirror_h_or_v(true, address)
            };
            vram[(mir & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.game]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start < state.len() { self.game = state[start]; start + 1 } else { start }
    }
}
