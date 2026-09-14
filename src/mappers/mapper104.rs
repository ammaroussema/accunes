use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper104 {
    inner: u8,
    outer: u8,
    cycles: u32,
}

impl Mapper104 {
    pub fn new() -> Self {
        Self {
            inner: 0,
            outer: 0,
            cycles: 0,
        }
    }

    fn locked(&self) -> bool {
        (self.outer & 0x08) != 0
    }

    fn sync(&self, _cart: &Cartridge) -> (usize, usize) {
        let prg_bank_8000 = ((self.outer as usize) << 4) | ((self.inner & 0x0F) as usize);
        let prg_bank_c000 = ((self.outer as usize) << 4) | 0x0F;
        (prg_bank_8000, prg_bank_c000)
    }
}

impl Mapper for Mapper104 {
    fn reset(&mut self) {
        self.inner = 0;
        self.outer = 0;
        self.cycles = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let (bank_8000, bank_c000) = self.sync(cart);
            let prg_len = cart.prg_rom.len();
            if prg_len == 0 {
                return FetchResult { data: 0, driven: false };
            }
            let bank = if address < 0xC000 {
                bank_8000
            } else {
                bank_c000
            };
            let offset = (bank * 0x4000 + (address as usize & 0x3FFF)) % prg_len;
            FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 && address <= 0xBFFF {
            if !self.locked() && self.cycles >= 120_000 {
                self.outer = data;
            }
        } else if address >= 0xC000 {
            self.inner = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        let norm = address & 0x2FFF;
        if cart.nametable_horizontal_mirroring {
            (norm & 0x33FF) | ((norm & 0x0800) >> 1)
        } else {
            norm & 0x37FF
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let len = chr_ram.len();
            if len == 0 {
                return (0, new_addr_bus);
            }
            let offset = (address as usize & 0x1FFF) % len;
            new_addr_bus |= chr_ram[offset] as u16;
        } else {
            let mirrored = if nametable_horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.cycles < 120_000 {
            self.cycles += 1;
        }
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![
            self.inner,
            self.outer,
            (self.cycles & 0xFF) as u8,
            ((self.cycles >> 8) & 0xFF) as u8,
            ((self.cycles >> 16) & 0xFF) as u8,
            ((self.cycles >> 24) & 0xFF) as u8,
        ]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.inner = state[p];
            p += 1;
        }
        if p < state.len() {
            self.outer = state[p];
            p += 1;
        }
        if p + 3 < state.len() {
            self.cycles = u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
            p += 4;
        }
        p
    }
}
