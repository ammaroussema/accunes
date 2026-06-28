use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper174 {
    prg_mode_32k: bool,
    prg_bank: u8,
    chr_bank: u8,
    horizontal_mirroring: bool,
}

impl Mapper174 {
    pub fn new() -> Self {
        Self {
            prg_mode_32k: false,
            prg_bank: 0,
            chr_bank: 0,
            horizontal_mirroring: false,
        }
    }
}

impl Mapper for Mapper174 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = if self.prg_mode_32k {
                (self.prg_bank & 0xFE) as usize
            } else {
                let bank = self.prg_bank as usize;
                if address >= 0xC000 {
                    bank
                } else {
                    bank
                }
            };
            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, _data: u8) {
        if address >= 0x8000 {
            self.prg_bank = ((address >> 4) & 0x07) as u8;
            self.prg_mode_32k = (address & 0x80) != 0;
            self.chr_bank = ((address >> 1) & 0x07) as u8;
            self.horizontal_mirroring = (address & 0x01) != 0;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.horizontal_mirroring {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
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
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = self.chr_bank as usize;
            let chr_offset = bank * 0x2000 + (address as usize & 0x1FFF);
            if using_chr_ram && !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[chr_offset % chr_ram.len()] as u16;
            } else if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[chr_offset % chr_rom.len()] as u16;
            }
        } else {
            let mirrored = if nametable_horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let bank = self.chr_bank as usize;
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[mirrored as usize & 0x7FF] = data;
            }
        }
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.prg_bank);
        state.push(self.chr_bank);
        state.push(if self.prg_mode_32k { 1 } else { 0 });
        state.push(if self.horizontal_mirroring { 1 } else { 0 });
        if cart.using_chr_ram {
            state.extend_from_slice(&cart.chr_ram);
        }
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.prg_bank = state[p];
            p += 1;
        }
        if p < state.len() {
            self.chr_bank = state[p];
            p += 1;
        }
        if p < state.len() {
            self.prg_mode_32k = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.horizontal_mirroring = state[p] != 0;
            p += 1;
        }
        if cart.using_chr_ram {
            for i in 0..cart.chr_ram.len() {
                if p < state.len() {
                    cart.chr_ram[i] = state[p];
                    p += 1;
                }
            }
        }
        p
    }

    fn reset(&mut self) {
        self.prg_mode_32k = false;
        self.prg_bank = 0;
        self.chr_bank = 0;
        self.horizontal_mirroring = false;
    }
}
