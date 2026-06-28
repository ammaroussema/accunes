use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper132 {
    increase: bool,
    output: u8,
    invert: u8,
    staging: u8,
    accumulator: u8,
    inverter: u8,
    a: bool,
    b: bool,
    x: bool,
    y: bool,
}

impl Mapper132 {
    pub fn new() -> Self {
        Self { increase: false, output: 0, invert: 0, staging: 0, accumulator: 0, inverter: 0, a: false, b: true, x: false, y: false }
    }
}

impl Mapper for Mapper132 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = ((self.output >> 2) & 1) as usize;
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
        } else if address >= 0x4020 && address <= 0x5FFF {
            if (address & 0x103) == 0x100 {
                let result = (self.accumulator & 7) | ((self.inverter ^ self.invert) & 0xF8);
                self.y = self.x || ((result & 0x10) != 0);
                FetchResult { data: result & 0x0F, driven: true }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else if address >= 0x6000 {
            let off = (address - 0x6000) as usize;
            if off < cart.prg_ram.len() { FetchResult { data: cart.prg_ram[off], driven: true } }
            else { FetchResult { data: 0, driven: false } }
        } else { FetchResult { data: 0, driven: false } }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        let masked = data & 0x0F;
        if address >= 0x8000 {
            self.output = (self.accumulator & 0xF) | ((self.inverter << 1) & 0x10);
        } else if address >= 0x4020 {
            match address & 0x103 {
                0x100 => {
                    if self.increase { self.accumulator = self.accumulator.wrapping_add(1); }
                    else { self.accumulator = (self.accumulator & 0xF8) | ((self.staging ^ self.invert) & 7); }
                }
                0x101 => self.invert = if (masked & 1) != 0 { 0xFF } else { 0x00 },
                0x102 => { self.staging = masked & 7; self.inverter = masked & 0xF8; }
                0x103 => self.increase = (masked & 1) != 0,
                _ => {}
            }
        }
        if self.invert != 0 { self.x = self.a; } else { self.x = self.b; }
        self.y = self.x || ((masked & 0x10) != 0);
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.nametable_horizontal_mirroring { (address & 0x33FF) | ((address & 0x0800) >> 1) } else { address & 0x37FF }
    }

    fn fetch_ppu(&mut self, _prg_rom: &[u8], chr_rom: &[u8], _prg_ram: &[u8], chr_ram: &[u8], _prg_vram: &[u8], using_chr_ram: bool, nametable_horizontal_mirroring: bool, _alternative_nametable_arrangement: bool, ppu_address_bus: u16, ppu_octal_latch: u8, vram: &[u8]) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = (self.output & 3) as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let data = if using_chr_ram && !chr_ram.is_empty() { chr_ram[offset % chr_ram.len()] } else { chr_rom[offset % chr_rom.len()] };
            new_addr_bus |= data as u16;
        } else {
            let mirrored = if nametable_horizontal_mirroring { (address & 0x33FF) | ((address & 0x0800) >> 1) } else { address & 0x37FF };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 { if cart.using_chr_ram && !cart.chr_ram.is_empty() { let len = cart.chr_ram.len(); cart.chr_ram[address as usize % len] = data; } }
        else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 { let idx = (mirrored & 0x7FF) as usize; if idx < cart.prg_vram.len() { cart.prg_vram[idx] = data; } }
            else { vram[mirrored as usize & 0x7FF] = data; }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut s = vec![if self.increase { 1 } else { 0 }, self.output, self.invert, self.staging, self.accumulator, self.inverter];
        s.push(if self.a { 1 } else { 0 }); s.push(if self.b { 1 } else { 0 }); s.push(if self.x { 1 } else { 0 }); s.push(if self.y { 1 } else { 0 });
        s
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.increase = state[p] != 0; p += 1; }
        if p < state.len() { self.output = state[p]; p += 1; }
        if p < state.len() { self.invert = state[p]; p += 1; }
        if p < state.len() { self.staging = state[p]; p += 1; }
        if p < state.len() { self.accumulator = state[p]; p += 1; }
        if p < state.len() { self.inverter = state[p]; p += 1; }
        if p < state.len() { self.a = state[p] != 0; p += 1; }
        if p < state.len() { self.b = state[p] != 0; p += 1; }
        if p < state.len() { self.x = state[p] != 0; p += 1; }
        if p < state.len() { self.y = state[p] != 0; p += 1; }
        p
    }

    fn reset(&mut self) {
        self.output = 0; self.invert = 0; self.staging = 0; self.accumulator = 0; self.inverter = 0;
        self.a = false; self.b = true; self.x = false; self.y = false;
    }
}
