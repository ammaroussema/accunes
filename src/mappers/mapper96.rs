use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper96 {
    prg_bank: u8,
    outer_chr_bank: u8,
    chr: u8,
    prev_a12_a13: u8,
    bus_conflict: bool,
}

impl Mapper96 {
    pub fn new(header: &[u8]) -> Self {
        let nes2 = header.len() >= 16 && (header[7] & 0x0C) == 0x08;
        let sub_mapper = if nes2 { (header[8] >> 4) & 0x0F } else { 0 };
        let bus_conflict = nes2 && sub_mapper == 2;
        Self {
            prg_bank: 0,
            outer_chr_bank: 0,
            chr: 0,
            prev_a12_a13: 0,
            bus_conflict,
        }
    }

    fn update_latch(&mut self, addr: u16) {
        let a12_a13 = ((addr >> 12) & 3) as u8;
        if self.prev_a12_a13 != 2 && a12_a13 == 2 {
            self.chr = ((addr >> 8) & 3) as u8;
        }
        self.prev_a12_a13 = a12_a13;
    }
}

impl Mapper for Mapper96 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = self.prg_bank & 3;
            let offset = (bank as usize * 0x8000) + (address as usize & 0x7FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
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
            let val = if self.bus_conflict {
                let bank = self.prg_bank & 3;
                let offset = (bank as usize * 0x8000) + (address as usize & 0x7FFF);
                data & cart.prg_rom[offset % cart.prg_rom.len()]
            } else {
                data
            };
            self.prg_bank = val & 3;
            self.outer_chr_bank = (val >> 2) & 1;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        address & 0x37FF
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
        self.update_latch(ppu_address_bus);
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            if !chr_ram.is_empty() {
                let bank = if address < 0x1000 {
                    (self.outer_chr_bank << 2) | (self.chr & 3)
                } else {
                    (self.outer_chr_bank << 2) | 3
                };
                let offset = (bank as usize * 0x1000) + (address as usize & 0x0FFF);
                new_addr_bus |= chr_ram[offset % chr_ram.len()] as u16;
            }
        } else {
            let mirrored = address & 0x37FF; 
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        self.update_latch(address);
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let bank = if address < 0x1000 {
                    (self.outer_chr_bank << 2) | (self.chr & 3)
                } else {
                    (self.outer_chr_bank << 2) | 3
                };
                let offset = (bank as usize * 0x1000) + (address as usize & 0x0FFF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = address & 0x37FF; 
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        _scanline: u16,
        _dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        self.update_latch(ppu_address_bus);
        false
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.chr_ram);
        state.push(self.prg_bank);
        state.push(self.outer_chr_bank);
        state.push(self.chr);
        state.push(self.prev_a12_a13);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..cart.chr_ram.len() {
            cart.chr_ram[i] = state[p];
            p += 1;
        }
        self.prg_bank = state[p];
        self.outer_chr_bank = state[p + 1];
        self.chr = state[p + 2];
        self.prev_a12_a13 = state[p + 3];
        p + 4
    }

    fn reset(&mut self) {
        self.prg_bank = 0;
        self.outer_chr_bank = 0;
        self.chr = 0;
        self.prev_a12_a13 = 0;
    }
}
