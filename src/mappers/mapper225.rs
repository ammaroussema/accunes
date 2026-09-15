use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper225 {
    extra_ram: [u8; 4],
    prg: u8,
    mode: u8,
    chr: u8,
    mirr: u8,
}

impl Mapper225 {
    pub fn new() -> Self {
        Self { extra_ram: [0; 4], prg: 0, mode: 0, chr: 0, mirr: 0 }
    }
}

impl Mapper for Mapper225 {
    fn reset(&mut self) {
        self.prg = 0;
        self.mode = 0;
        self.extra_ram = [0; 4];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x5000 && address <= 0x5FFF {
            if address & 0x800 != 0 {
                return FetchResult { data: self.extra_ram[address as usize & 3], driven: true };
            }
            return FetchResult { data: 0, driven: false };
        }
        if address >= 0x8000 {
            let offset = if self.mode != 0 {
                (self.prg as usize) * 0x4000 + (address as usize & 0x3FFF)
            } else {
                ((self.prg >> 1) as usize) * 0x8000 + (address as usize - 0x8000)
            };
            return FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address <= 0x5FFF {
            if address & 0x800 != 0 {
                self.extra_ram[address as usize & 3] = data & 0x0F;
            }
            return;
        }
        if address >= 0x8000 {
            let bank = ((address >> 14) & 1) as u8;
            self.mirr = ((address >> 13) & 1) as u8;
            self.mode = ((address >> 12) & 1) as u8;
            self.chr = (address as u8 & 0x3F) | (bank << 6);
            self.prg = (((address >> 6) as u8) & 0x3F) | (bank << 6);
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mirr != 0 {
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
            let bank = self.chr as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if self.mirr != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let bank = self.chr as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let len = cart.chr_ram.len();
            if len > 0 { cart.chr_ram[offset % len] = data; }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut v = self.extra_ram.to_vec();
        v.push(self.prg); v.push(self.mode); v.push(self.chr); v.push(self.mirr);
        v
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..4 { if p < state.len() { self.extra_ram[i] = state[p]; p += 1; } }
        if p < state.len() { self.prg = state[p]; p += 1; }
        if p < state.len() { self.mode = state[p]; p += 1; }
        if p < state.len() { self.chr = state[p]; p += 1; }
        if p < state.len() { self.mirr = state[p]; p += 1; }
        p
    }
}
