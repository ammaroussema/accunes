use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper127 {
    prg: [u8; 4],
    chr: [u8; 8],
    irq_enabled: bool,
    irq_counter: u8,
    mirror: [u8; 4],
    irq_ack: bool,
}

impl Mapper127 {
    pub fn new() -> Self {
        Self {
            prg: [0x0F; 4],
            chr: [0; 8],
            irq_enabled: false,
            irq_counter: 0,
            mirror: [0; 4],
            irq_ack: false,
        }
    }
}

impl Mapper for Mapper127 {
    fn reset(&mut self) {
        self.prg = [0x0F; 4];
        self.chr = [0; 8];
        self.irq_enabled = false;
        self.irq_counter = 0;
        self.mirror = [0; 4];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            if address >= 0x6000 {
                let offset = (address - 0x6000) as usize;
                if offset < cart.prg_ram.len() {
                    return FetchResult { data: cart.prg_ram[offset], driven: true };
                }
            }
            return FetchResult { data: 0, driven: false };
        }
        let slot = ((address - 0x8000) / 0x2000) as usize;
        let bank = if slot < 4 { self.prg[slot] as usize } else { 0 };
        let num_banks = cart.prg_rom.len() / 0x2000;
        let bank = if num_banks > 0 { bank % num_banks } else { 0 };
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        let data = if cart.prg_rom.is_empty() { 0 } else { cart.prg_rom[offset % cart.prg_rom.len()] };
        FetchResult { data, driven: true }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            return;
        }
        match address & 0x73 {
            0x00 => self.prg[0] = data & 0x0F,
            0x01 => self.prg[1] = data & 0x0F,
            0x02 => self.prg[2] = data & 0x0F,
            0x03 => self.prg[3] = (data & 0x03) | 0x0C,
            0x10 => self.chr[0] = data & 0x7F,
            0x11 => self.chr[1] = data & 0x7F,
            0x12 => self.chr[2] = data & 0x7F,
            0x13 => self.chr[3] = data & 0x7F,
            0x20 => self.chr[4] = data & 0x7F,
            0x21 => self.chr[5] = data & 0x7F,
            0x22 => self.chr[6] = data & 0x7F,
            0x23 => self.chr[7] = data & 0x7F,
            0x30 | 0x31 | 0x32 | 0x33 => self.irq_enabled = true,
            0x40 | 0x41 | 0x42 | 0x43 => {
                self.irq_enabled = false;
                self.irq_counter = 0;
                self.irq_ack = true;
            }
            0x50 => self.mirror[0] = data & 1,
            0x51 => self.mirror[1] = data & 1,
            0x52 => self.mirror[2] = data & 1,
            0x53 => self.mirror[3] = data & 1,
            _ => {}
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let quad = ((address >> 10) & 3) as usize;
        let page = self.mirror[quad];
        0x2000 + (page as u16) * 0x400 + (address & 0x3FF)
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
            let chr_bank = if (bank as usize) < 8 { self.chr[bank as usize] } else { 0 };
            let offset = (chr_bank as usize) * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let quad = ((address >> 10) & 3) as usize;
            let page = self.mirror[quad];
            let mirrored = 0x2000 + (page as u16) * 0x400 + (address & 0x3FF);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram && !cart.chr_ram.is_empty() {
            let len = cart.chr_ram.len();
            cart.chr_ram[address as usize % len] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.irq_enabled {
            self.irq_counter = self.irq_counter.wrapping_sub(1);
            if self.irq_counter == 0 {
                return true;
            }
        }
        false
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.irq_ack {
            self.irq_ack = false;
            true
        } else {
            false
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::with_capacity(15);
        state.extend_from_slice(&self.prg);
        state.extend_from_slice(&self.chr);
        state.push(if self.irq_enabled { 1 } else { 0 });
        state.push(self.irq_counter);
        state.extend_from_slice(&self.mirror);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..4 { if p < state.len() { self.prg[i] = state[p]; p += 1; } }
        for i in 0..8 { if p < state.len() { self.chr[i] = state[p]; p += 1; } }
        if p < state.len() { self.irq_enabled = state[p] != 0; p += 1; }
        if p < state.len() { self.irq_counter = state[p]; p += 1; }
        for i in 0..4 { if p < state.len() { self.mirror[i] = state[p]; p += 1; } }
        p
    }
}
