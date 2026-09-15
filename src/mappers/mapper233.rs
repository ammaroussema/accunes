use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper233 {
    latche: [u8; 2],
    reset_flag: u8,
}

impl Mapper233 {
    pub fn new() -> Self {
        Self { latche: [0; 2], reset_flag: 0 }
    }

    fn mirror_addr(&self, address: u16) -> u16 {
        match self.latche[0] >> 6 {
            0 => address & 0x3FFF,
            1 => address & 0x37FF,
            2 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            _ => (address & 0x3BFF) | ((address & 0x0400) << 1),
        }
    }
}

impl Mapper for Mapper233 {
    fn reset(&mut self) {
        self.reset_flag ^= 0x20;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = (self.latche[0] & 0x1f) as usize
                | self.reset_flag as usize
                | ((self.latche[1] as usize & 1) << 6);
            let offset = if self.latche[0] & 0x20 == 0 {
                (bank >> 1) * 0x8000 + (address as usize - 0x8000)
            } else {
                bank * 0x4000 + (address as usize & 0x3FFF)
            };
            return FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.latche[address as usize & 1] = data;
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
            let len = chr_ram.len();
            if len == 0 {
                return (0, new_addr_bus);
            }
            new_addr_bus |= chr_ram[(address as usize & 0x1FFF) % len] as u16;
        } else {
            new_addr_bus |= vram[(self.mirror_addr(address) & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            vram[(self.mirror_addr(address) & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.latche[0], self.latche[1], self.reset_flag]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.latche[0] = state[p]; p += 1; }
        if p < state.len() { self.latche[1] = state[p]; p += 1; }
        if p < state.len() { self.reset_flag = state[p]; p += 1; }
        p
    }
}
