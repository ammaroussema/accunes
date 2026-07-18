use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper347 {
    reg0: u8,
    reg1: u8,
    mirroring: bool,
}

impl Mapper347 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self { reg0: 0xFF, reg1: 0xFF, mirroring: false }
    }
}

impl Mapper for Mapper347 {
    fn reset(&mut self) {
        self.reg0 = 0xFF;
        self.reg1 = 0xFF;
        self.mirroring = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        let len = cart.prg_rom.len().max(1);
        let data = match address {
            0x6000..=0x6BFF => {
                let offset = address as usize - 0x6000;
                cart.prg_ram[offset % cart.prg_ram.len().max(1)]
            }
            0x6C00..=0x6FFF => {
                let offset = (address as usize - 0x6000) + (self.reg1 as usize) * 0x1000;
                cart.prg_rom[offset % len]
            }
            0x7000..=0x7FFF => {
                let offset = (address as usize - 0x7000) + (self.reg0 as usize) * 0x1000 + 0x10000;
                cart.prg_rom[offset % len]
            }
            0x8000..=0xB7FF => {
                let offset = address as usize - 0x8000 + 0x18000;
                cart.prg_rom[offset % len]
            }
            0xB800..=0xBFFF => {
                let offset = address as usize - 0xB800 + 0x0C00;
                cart.prg_ram[offset % cart.prg_ram.len().max(1)]
            }
            0xC000..=0xCBFF => {
                let offset = (address as usize - 0xC000) + (self.reg1 as usize) * 0x1000;
                cart.prg_rom[offset % len]
            }
            0xCC00..=0xD7FF => {
                let offset = address as usize - 0xCC00 + 0x1400;
                cart.prg_ram[offset % cart.prg_ram.len().max(1)]
            }
            0x8000..=0xFFFF => {
                let offset = address as usize - 0x8000 + 0x18000;
                cart.prg_rom[offset % len]
            }
            _ => 0,
        };
        let driven = address >= 0x6000;
        FetchResult { data, driven }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, val: u8) {
        match address {
            0x6000..=0x6BFF => {
                let i = address as usize - 0x6000;
                if i < cart.prg_ram.len() { cart.prg_ram[i] = val; }
            }
            0xB800..=0xBFFF => {
                let i = address as usize - 0xB800 + 0x0C00;
                if i < cart.prg_ram.len() { cart.prg_ram[i] = val; }
            }
            0xCC00..=0xD7FF => {
                let i = address as usize - 0xCC00 + 0x1400;
                if i < cart.prg_ram.len() { cart.prg_ram[i] = val; }
            }
            0x8000..=0x8FFF => {
                self.reg0 = (address & 7) as u8;
                self.mirroring = (address & 8) != 0;
            }
            0x9000..=0x9FFF => {
                self.reg1 = (address & 0xF) as u8;
            }
            _ => {}
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v(self.mirroring, address)
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
                chr_ram[(address as usize) % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[(address as usize) % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mir = mirror_h_or_v(self.mirroring, address);
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[(address as usize) % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mir = mirror_h_or_v(self.mirroring, address);
            vram[(mir & 0x7FF) as usize] = data;
        }
    }

    fn audio_sample(&self) -> f32 {
        0.0
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.reg0, self.reg1, if self.mirroring { 1 } else { 0 }]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.reg0 = state[p]; p += 1; }
        if p < state.len() { self.reg1 = state[p]; p += 1; }
        if p < state.len() { self.mirroring = state[p] != 0; p += 1; }
        p
    }
}
