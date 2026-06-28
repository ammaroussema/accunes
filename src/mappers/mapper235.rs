use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper235 {
    addr_latch: u16,
    open_bus: bool,
    unrom: bool,
    prg_page_count: usize,
}

impl Mapper235 {
    pub fn new(prg_size_16k: usize) -> Self {
        let prg_page_count = match prg_size_16k {
            64 => 64,
            128 => 128,
            256 => 256,
            _ => 512,
        };
        Self { addr_latch: 0, open_bus: false, unrom: false, prg_page_count }
    }

    fn config_table(mode: usize) -> [(u8, bool); 4] {
        match mode {
            0 => [(0x00, false), (0x00, true), (0x00, true), (0x00, true)],
            1 => [(0x00, false), (0x00, true), (0x20, false), (0x00, true)],
            2 => [(0x00, false), (0x00, true), (0x20, false), (0x40, false)],
            _ => [(0x00, false), (0x20, false), (0x40, false), (0x60, false)],
        }
    }

    fn mode(&self) -> usize {
        match self.prg_page_count {
            64 => 0,
            128 => 1,
            256 => 2,
            _ => 3,
        }
    }
}

impl Mapper235 {
    fn mirrored_addr(&self, address: u16) -> u16 {
        if self.addr_latch & 0x0400 != 0 {
            address & 0x3FFF
        } else if self.addr_latch & 0x2000 != 0 {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }
}

impl Mapper for Mapper235 {
    fn reset(&mut self) {
        self.addr_latch = 0;
        self.open_bus = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if self.open_bus {
                return FetchResult { data: (address >> 8) as u8, driven: true };
            }
            let config = Self::config_table(self.mode());
            let entry = config[(self.addr_latch as usize >> 8) & 3];
            let mut bank = entry.0 as usize | (self.addr_latch as usize & 0x1F);
            let offset = if self.addr_latch & 0x800 != 0 {
                bank = (bank << 1) | ((self.addr_latch >> 12) as usize & 1);
                let bank_offset = bank * 0x4000;
                (bank_offset + (address as usize & 0x3FFF)) % cart.prg_rom.len()
            } else {
                let bank_offset = bank * 0x8000;
                (bank_offset + (address as usize - 0x8000)) % cart.prg_rom.len()
            };
            FetchResult { data: cart.prg_rom[offset], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, _data: u8) {
        if address >= 0x8000 {
            self.addr_latch = address;
            let config = Self::config_table(self.mode());
            let entry = config[(address as usize >> 8) & 3];
            self.open_bus = entry.1;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirrored_addr(address)
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
            let len = chr_ram.len();
            if len == 0 {
                return (0, new_addr_bus);
            }
            new_addr_bus |= chr_ram[(address as usize & 0x1FFF) % len] as u16;
        } else {
            new_addr_bus |= vram[(self.mirrored_addr(address) & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.addr_latch as u8, (self.addr_latch >> 8) as u8, if self.open_bus { 1 } else { 0 }, if self.unrom { 1 } else { 0 }]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.addr_latch = state[p] as u16; p += 1; }
        if p < state.len() { self.addr_latch |= (state[p] as u16) << 8; p += 1; }
        if p < state.len() { self.open_bus = state[p] != 0; p += 1; }
        if p < state.len() { self.unrom = state[p] != 0; p += 1; }
        p
    }
}
