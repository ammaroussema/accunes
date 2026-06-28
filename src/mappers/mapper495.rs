use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper495 {
    prg: [u8; 3],
    chr: [u8; 4],
    state: [u8; 2],
}

impl Mapper495 {
    pub fn new() -> Self {
        Self { prg: [0; 3], chr: [0; 4], state: [0; 2] }
    }
}

impl Mapper for Mapper495 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank_idx = ((address - 0x8000) >> 13) as usize;
            let bank = if bank_idx < 3 { self.prg[bank_idx] } else { 0xFF };
            let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
            FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            if address < 0xE000 {
                let idx = ((address >> 13) & 3) as usize;
                if idx < 3 { self.prg[idx] = data; }
            } else {
                let idx = ((address >> 10) & 3) as usize;
                if idx < 4 { self.chr[idx] = data; }
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let ch = self.chr[self.state[0] as usize] >> 6;
        match ch {
            0 => address & 0x33FF,
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            2 => address & 0x37FF,
            _ if (address & 0x0800) == 0 => 0x2000 | (address & 0x3FF),
            _ => 0x2400 | (address & 0x3FF),
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
            let bank = (address >> 12) as usize;
            let ch = if bank == 0 {
                self.chr[self.state[0] as usize]
            } else {
                self.chr[(self.state[1] as usize) | 2]
            };
            let offset = (ch as usize) * 0x1000 + (address as usize & 0xFFF);
            let data = if using_chr_ram { chr_ram[offset % chr_ram.len()] } else { chr_rom[offset % chr_rom.len()] };
            new_addr_bus |= data as u16;
            let bank4 = if bank == 0 { 3 } else { 7 };
            let addr_in_page = if bank4 == 7 { address as usize & 0x3F8 } else { address as usize & 0x3FF };
            if addr_in_page == 0x3D8 {
                self.state[bank] = 1;
            } else if addr_in_page == 0x3E8 {
                self.state[bank] = 0;
            }
        } else {
            let ch = self.chr[self.state[0] as usize] >> 6;
            let mirrored = match ch {
                0 => address & 0x33FF,
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                2 => address & 0x37FF,
                _ if (address & 0x0800) == 0 => 0x2000 | (address & 0x3FF),
                _ => 0x2400 | (address & 0x3FF),
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let bank = (address >> 12) as usize;
                let bank4 = if bank == 0 { 3 } else { 7 };
                let addr_in_page = if bank4 == 7 { address as usize & 0x3F8 } else { address as usize & 0x3FF };
                if addr_in_page == 0x3D8 {
                    self.state[bank] = 1;
                } else if addr_in_page == 0x3E8 {
                    self.state[bank] = 0;
                }
                let len = cart.chr_ram.len();
                cart.chr_ram[address as usize % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() { cart.prg_vram[idx] = data; }
            } else {
                vram[mirrored as usize & 0x7FF] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::with_capacity(9);
        state.extend_from_slice(&self.prg);
        state.extend_from_slice(&self.chr);
        state.extend_from_slice(&self.state);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..3 { if p < state.len() { self.prg[i] = state[p]; p += 1; } }
        for i in 0..4 { if p < state.len() { self.chr[i] = state[p]; p += 1; } }
        for i in 0..2 { if p < state.len() { self.state[i] = state[p]; p += 1; } }
        p
    }

    fn reset(&mut self) {
        self.prg = [0; 3];
        self.chr = [0; 4];
        self.state = [0; 2];
    }
}
