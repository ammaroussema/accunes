use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper589 {
    addr: u16,
    data: u8,
}

impl Mapper589 {
    pub fn new() -> Self {
        Self { addr: 0, data: 0 }
    }
}

impl Mapper for Mapper589 {
    fn reset(&mut self) {
        self.addr = 0;
        self.data = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        if cart.prg_rom.is_empty() {
            return FetchResult {
                data: 0,
                driven: true,
            };
        }
        let page = if self.addr & 0x20 != 0 {
            if address >= 0xC000 {
                ((self.addr >> 1) & 0x08 | 0x07) as usize
            } else {
                ((self.addr >> 1) & 0x08 | (self.data & 0x07) as u16) as usize
            }
        } else {
            (0x10 | (self.addr >> 1) & 0x07) as usize
        };
        let offset = page * 0x4000 + (address as usize & 0x3FFF);
        FetchResult {
            data: cart.prg_rom[offset % cart.prg_rom.len()],
            driven: true,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            if self.data & 0x08 != 0 && data & 0x08 == 0 {
                self.addr = address & 0x0FFF;
            }
            self.data = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.addr & 1 != 0 {
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
        prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[(address as usize) % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[(address as usize) % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if nametable_horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() {
                    prg_vram[idx]
                } else {
                    0
                }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, _vram: &mut [u8]) {
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let offset = address as usize % cart.chr_ram.len();
                cart.chr_ram[offset] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.addr.to_le_bytes());
        state.push(self.data);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.addr = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.data = state[p];
            p += 1;
        }
        p
    }
}
