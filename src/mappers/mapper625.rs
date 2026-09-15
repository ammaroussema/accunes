use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper625 {
    latch: u16,
    data: u8,
}

impl Mapper625 {
    pub fn new() -> Self {
        Self { latch: 0, data: 0 }
    }

    fn prg(&self) -> u8 {
        let mut prg = ((self.data >> 1) & 0x40) | ((self.latch << 5) & 0x20) as u8 | (self.data & 0x1F);
        if prg & 0x40 != 0 {
            prg ^= 0x20;
        }
        prg
    }

    fn prg_byte(&self, cart: &Cartridge, address: u16) -> u8 {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }
        let offset = if (self.latch & 0x08) != 0 {
            let bank = if address < 0xC000 { self.prg() } else { self.prg() | 7 };
            (bank as usize) * 0x4000 + (address as usize & 0x3FFF)
        } else if (self.data & 0x20) != 0 {
            let bank = self.prg();
            (bank as usize) * 0x4000 + (address as usize & 0x3FFF)
        } else {
            ((self.prg() >> 1) as usize) * 0x8000 + (address as usize & 0x7FFF)
        };
        cart.prg_rom[offset % len]
    }

    fn mirror_horizontal(&self) -> bool {
        (self.data & 0x40) == 0
    }
}

impl Mapper for Mapper625 {
    fn reset(&mut self) {
        self.latch = 0;
        self.data = 0;
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 || cart.prg_rom.is_empty() {
            return FetchResult { data: 0, driven: false };
        }
        FetchResult {
            data: self.prg_byte(cart, address),
            driven: true,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            return;
        }
        let mut new_addr = address;
        let mut new_data = data;
        let rom_value = self.prg_byte(cart, address);
        if self.latch & 0x08 != 0 {
            new_addr = self.latch;
            new_data = (data & !0x07) | (rom_value & 0x07);
        }
        self.latch = new_addr;
        self.data = new_data;
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        crate::mapper::mirror_h_or_v(self.mirror_horizontal(), address)
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
            let offset = address as usize & 0x1FFF;
            let byte = if !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = crate::mapper::mirror_h_or_v(self.mirror_horizontal(), address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }

        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let offset = address as usize & 0x1FFF;
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = self.latch.to_le_bytes().to_vec();
        state.push(self.data);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 2 <= state.len() {
            self.latch = u16::from_le_bytes([state[start], state[start + 1]]);
            start += 2;
        }
        if start < state.len() {
            self.data = state[start];
            start += 1;
        }
        start
    }
}
