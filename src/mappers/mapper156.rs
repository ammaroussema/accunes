use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper156 {
    prg: u8,
    chr: [u16; 8],
    mirroring: u8,
}

impl Mapper156 {
    pub fn new() -> Self {
        Self { prg: 0, chr: [0; 8], mirroring: 2 }
    }
}

impl Mapper for Mapper156 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if address < 0xC000 {
                let bank = self.prg as usize;
                let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
            } else {
                let offset = 0xFF * 0x4000 + (address as usize & 0x3FFF);
                FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
            }
        } else if address >= 0x6000 {
            let off = (address - 0x6000) as usize;
            if off < cart.prg_ram.len() {
                FetchResult { data: cart.prg_ram[off], driven: true }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0xC000 {
            let reg = ((address >> 2) & 7) as usize;
            match reg {
                0 => self.chr[0 | (address as usize & 3)] = (self.chr[0 | (address as usize & 3)] & 0xFF00) | data as u16,
                1 => self.chr[0 | (address as usize & 3)] = (self.chr[0 | (address as usize & 3)] & 0x00FF) | (data as u16) << 8,
                2 => self.chr[4 | (address as usize & 3)] = (self.chr[4 | (address as usize & 3)] & 0xFF00) | data as u16,
                3 => self.chr[4 | (address as usize & 3)] = (self.chr[4 | (address as usize & 3)] & 0x00FF) | (data as u16) << 8,
                4 => self.prg = data,
                5 => self.mirroring = data & 1,
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        match self.mirroring {
            0 => address & 0x27FF,  
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1),  
            _ => address & 0x33FF,  
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
            let bank = (address >> 10) as usize;
            let base = self.chr[bank & 7] as usize;
            let offset = base * 0x400 + (address as usize & 0x3FF);
            let data = if using_chr_ram { chr_ram[offset % chr_ram.len()] } else { chr_rom[offset % chr_rom.len()] };
            new_addr_bus |= data as u16;
        } else {
            let mirrored = match self.mirroring {
                0 => address & 0x37FF,
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                _ => address & 0x33FF,
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
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
                vram[mirrored as usize & 0x7FF] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = vec![self.prg, self.mirroring];
        for &c in &self.chr {
            state.extend_from_slice(&c.to_le_bytes());
        }
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.prg = state[p]; p += 1; }
        if p < state.len() { self.mirroring = state[p]; p += 1; }
        for i in 0..8 {
            if p + 2 <= state.len() {
                self.chr[i] = u16::from_le_bytes([state[p], state[p + 1]]);
                p += 2;
            }
        }
        p
    }

    fn reset(&mut self) {
        self.prg = 0;
        self.chr = [0; 8];
        self.mirroring = 2;
    }
}
