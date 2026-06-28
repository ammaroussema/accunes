use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper331 {
    mode: u8,
    prg: [u8; 2],
    current_chr_bank: u8,
}

impl Mapper331 {
    pub fn new() -> Self {
        Self { mode: 0, prg: [0; 2], current_chr_bank: 0 }
    }

    fn prg_bank(&self) -> usize {
        (self.mode as usize) << 3 | (self.prg[self.current_chr_bank as usize] as usize & 7)
    }
}

impl Mapper for Mapper331 {
    fn reset(&mut self) {
        self.mode = 0;
        self.prg = [0; 2];
        self.current_chr_bank = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }
        let num_banks = cart.prg_rom.len();
        if num_banks == 0 {
            return FetchResult { data: 0, driven: false };
        }
        if (self.mode & 8) != 0 {
            let bank = (self.prg_bank() >> 1) % (num_banks / 0x8000);
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            FetchResult { data: cart.prg_rom[offset % num_banks], driven: true }
        } else {
            let banks_16k = num_banks / 0x4000;
            let bank = if address < 0xC000 {
                self.prg_bank() % banks_16k
            } else {
                ((self.mode as usize) << 3 | 7) % banks_16k
            };
            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            FetchResult { data: cart.prg_rom[offset % num_banks], driven: true }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            match address >> 12 {
                0xA | 0xB => {
                    self.prg[0] = data;
                }
                0xC | 0xD => {
                    self.prg[1] = data;
                }
                0xE | 0xF => {
                    self.mode = data & 0x0F;
                }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if (self.mode & 4) != 0 {
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
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = address >> 10;
            let chr_bank = if (bank & 4) != 0 { 1 } else { 0 };
            self.current_chr_bank = chr_bank;
            let chr_page = ((self.mode as usize) << 5) | (self.prg[chr_bank as usize] as usize >> 3);
            let offset = chr_page * 0x1000 + (address as usize & 0x0FFF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if (self.mode & 4) != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[address as usize % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() { cart.prg_vram[idx] = data; }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut s = Vec::with_capacity(4);
        s.push(self.mode);
        s.push(self.prg[0]);
        s.push(self.prg[1]);
        s.push(self.current_chr_bank);
        s
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.mode = state[p]; p += 1; }
        if p < state.len() { self.prg[0] = state[p]; p += 1; }
        if p < state.len() { self.prg[1] = state[p]; p += 1; }
        if p < state.len() { self.current_chr_bank = state[p]; p += 1; }
        p
    }
}
