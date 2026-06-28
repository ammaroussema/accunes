use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper27 {
    latch: u16,
}

impl Mapper27 {
    pub fn new() -> Self {
        Mapper27 { latch: 0 }
    }
}

impl Mapper for Mapper27 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let offset = (address as usize - 0x8000) % cart.prg_rom.len();
            FetchResult { data: cart.prg_rom[offset], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            if address == 0x8000 {
                self.latch = data as u16;
            } else {
                self.latch = address;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.latch & 1 == 0 {
            address & 0x3FFF
        } else {
            (address & 0x3FFF) | 0x0400
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
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let bank = (self.latch & 1) as usize;
            let chr_size = if using_chr_ram {
                chr_ram.len()
            } else if !chr_rom.is_empty() {
                chr_rom.len()
            } else {
                chr_ram.len()
            };
            if chr_size == 8192 {
                let offset = (bank * 0x1000) + (address as usize & 0x0FFF);
                if using_chr_ram {
                    new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
                } else if chr_rom.is_empty() {
                    new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
                } else {
                    new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
                }
            } else {
                let offset = (bank * 0x2000) + (address as usize & 0x1FFF);
                if using_chr_ram {
                    new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
                } else if chr_rom.is_empty() {
                    new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
                } else {
                    new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
                }
            }
        } else {
            let mirrored = if self.latch & 1 == 0 {
                address & 0x3FFF
            } else {
                (address & 0x3FFF) | 0x0400
            };
            let idx = (mirrored & 0x7FF) as usize;
            new_addr_bus |= vram[idx] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn cpu_clock_rise(&mut self, _ppu_address_bus: u16) -> bool {
        false
    }

    fn ppu_clock(
        &mut self,
        _ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        _scanline: u16,
        _dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        false
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        false
    }

    fn take_irq_ack(&mut self) -> bool {
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.latch.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start + 2 <= state.len() {
            self.latch = u16::from_le_bytes([state[start], state[start + 1]]);
            start + 2
        } else {
            start
        }
    }

    fn reset(&mut self) {
        self.latch = 0;
    }
}
