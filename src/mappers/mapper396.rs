use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper396 {
    latch: u8,
    outer_bank: u8,
    submapper: u8,
}

impl Mapper396 {
    pub fn new(submapper: u8) -> Self {
        Self { latch: 0, outer_bank: 0, submapper }
    }

    fn prg_bank_8000(&self) -> usize {
        ((self.latch & 7) | (self.outer_bank << 3)) as usize
    }

    fn prg_bank_c000(&self) -> usize {
        (7 | (self.outer_bank << 3)) as usize
    }

    fn is_horizontal_mirroring(&self) -> bool {
        (self.outer_bank & 0x60) == 0
    }
}

impl Mapper for Mapper396 {
    fn reset(&mut self) {
        self.latch = 0;
        self.outer_bank = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }
        let len = cart.prg_rom.len();
        if len == 0 {
            return FetchResult { data: 0, driven: true };
        }
        let bank = if address >= 0xC000 {
            self.prg_bank_c000()
        } else {
            self.prg_bank_8000()
        };
        let offset = bank * 0x4000 + (address as usize & 0x3FFF);
        FetchResult { data: cart.prg_rom[offset % len], driven: true }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            return;
        }
        if self.submapper == 1 {
            if address < 0xC000 {
                self.outer_bank = data;
            } else {
                self.latch = data;
            }
        } else {
            if address >= 0xA000 && address < 0xC000 {
                self.outer_bank = data;
            } else {
                self.latch = data;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v(self.is_horizontal_mirroring(), address)
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
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[(address & 0x1FFF) as usize]
            } else if !chr_rom.is_empty() {
                chr_rom[(address & 0x1FFF) as usize % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else if address < 0x3F00 {
            let mir = mirror_h_or_v(self.is_horizontal_mirroring(), address);
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.latch, self.outer_bank, self.submapper]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.latch = state[p]; p += 1; }
        if p < state.len() { self.outer_bank = state[p]; p += 1; }
        if p < state.len() { self.submapper = state[p]; p += 1; }
        p
    }
}
