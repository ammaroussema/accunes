use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper390 {
    prg: u8,
    chr: u8,
    dip_switches: u8,
}

impl Mapper390 {
    pub fn new() -> Self {
        Self { prg: 0, chr: 0, dip_switches: 0 }
    }

    fn prg_mode(&self) -> u8 {
        (self.prg >> 4) & 3
    }

    fn prg_read(&self, cart: &Cartridge, address: u16) -> u8 {
        let prg_len = cart.prg_rom.len();
        if prg_len == 0 {
            return 0;
        }
        let addr = if self.prg_mode() == 1 {
            address | self.dip_switches as u16
        } else {
            address
        };
        match self.prg_mode() {
            0 | 1 => {
                let bank_lo = self.prg as usize;
                let bank_hi = (self.prg | 7) as usize;
                let offset = if addr < 0xC000 {
                    bank_lo * 0x4000 + (addr as usize & 0x3FFF)
                } else {
                    bank_hi * 0x4000 + (addr as usize & 0x3FFF)
                };
                cart.prg_rom[offset % prg_len]
            }
            2 => {
                let bank = (self.prg >> 1) as usize;
                let offset = bank * 0x8000 + (addr as usize & 0x7FFF);
                cart.prg_rom[offset % prg_len]
            }
            3 => {
                let bank = self.prg as usize;
                let offset = bank * 0x4000 + (addr as usize & 0x3FFF);
                cart.prg_rom[offset % prg_len]
            }
            _ => 0,
        }
    }
}

impl Mapper for Mapper390 {
    fn reset(&mut self) {
        self.prg = 0;
        self.chr = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            FetchResult { data: self.prg_read(cart, address), driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, _data: u8) {
        if address >= 0x8000 && address < 0xC000 {
            self.chr = (address & 0xFF) as u8;
        } else if address >= 0xC000 {
            self.prg = (address & 0xFF) as u8;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v((self.chr & 0x20) != 0, address)
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
            let bank = self.chr as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[address as usize & 0x1FFF]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else if address < 0x3F00 {
            let mir = mirror_h_or_v((self.chr & 0x20) != 0, address);
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
            let mir = mirror_h_or_v((self.chr & 0x20) != 0, address);
            vram[(mir & 0x7FF) as usize] = data;
        }
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.prg, self.chr, self.dip_switches]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.prg = state[p]; p += 1; }
        if p < state.len() { self.chr = state[p]; p += 1; }
        if p < state.len() { self.dip_switches = state[p]; p += 1; }
        p
    }
}
