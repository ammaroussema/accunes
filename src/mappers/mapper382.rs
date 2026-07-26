// Mapper 382 - 830928C (multicart mixed latch)
//
// Reference: NintendulatorNRS-DBG multicart mixed latch/mapper382.cpp

use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper382 {
    latch_addr: u16,
    latch_data: u8,
    addr_locked: u16,
    data_locked: u8,
}

impl Mapper382 {
    pub fn new() -> Self {
        Self {
            latch_addr: 0,
            latch_data: 0,
            addr_locked: 0,
            data_locked: 0,
        }
    }

    fn is_32k_mode(&self) -> bool {
        (self.latch_addr & 0x08) != 0
    }

    fn prg_read(&self, cart: &Cartridge, address: u16) -> u8 {
        let prg_len = cart.prg_rom.len();
        if prg_len == 0 {
            return 0;
        }
        if self.is_32k_mode() {
            let bank = ((self.latch_addr as usize) << 2) | (self.latch_data as usize & 3);
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            cart.prg_rom[offset % prg_len]
        } else {
            let bank_lo = ((self.latch_addr as usize) << 3) | (self.latch_data as usize & 7);
            let bank_hi = ((self.latch_addr as usize) << 3) | 7;
            let bank = if address < 0xC000 { bank_lo } else { bank_hi };
            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            cart.prg_rom[offset % prg_len]
        }
    }
}

impl Mapper for Mapper382 {
    fn reset(&mut self) {
        self.latch_addr = 0;
        self.latch_data = 0;
        self.addr_locked = 0;
        self.data_locked = 0;
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

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let rom_data = self.prg_read(cart, address);
            let new_data = data & rom_data;
            let mut new_addr = address;
            new_addr = (self.latch_addr & self.addr_locked) | (new_addr & !self.addr_locked);
            let new_data = (self.latch_data & self.data_locked) | (new_data & !self.data_locked);
            self.latch_addr = new_addr;
            self.latch_data = new_data;
            if (self.latch_addr & 0x20) != 0 {
                self.addr_locked = 0xFFFF;
                self.data_locked = 0x00;
            } else {
                self.addr_locked = 0x0000;
                self.data_locked = 0x00;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v((self.latch_addr & 0x10) != 0, address)
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
                chr_ram[(address as usize) % chr_ram.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mir = mirror_h_or_v((self.latch_addr & 0x10) != 0, address);
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize) % len] = data;
            }
        } else if address < 0x3F00 {
            let mir = mirror_h_or_v((self.latch_addr & 0x10) != 0, address);
            vram[(mir & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = self.latch_addr.to_le_bytes().to_vec();
        state.push(self.latch_data);
        let locked: u16 = (self.addr_locked != 0) as u16;
        state.extend_from_slice(&locked.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.latch_addr = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.latch_data = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            let locked = u16::from_le_bytes([state[p], state[p + 1]]);
            self.addr_locked = if locked != 0 { 0xFFFF } else { 0x0000 };
            p += 2;
        }
        p
    }
}
