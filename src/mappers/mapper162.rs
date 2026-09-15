use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper162 {
    reg: [u8; 8],
    prg_ram: [u8; 0x2000],
    pa09: bool,
    pa13: bool,
}

impl Mapper162 {
    pub fn new() -> Self {
        Mapper162 { reg: [3, 0, 0, 7, 0, 0, 0, 0], prg_ram: [0; 0x2000], pa09: false, pa13: false }
    }

    fn prg_bank(&self) -> u8 {
        match self.reg[3] & 7 {
            0 | 2 => (self.reg[0] & 0x0C) | (self.reg[1] & 2) | ((self.reg[2] & 0x0F) << 4),
            1 | 3 => (self.reg[0] & 0x0C) | ((self.reg[2] & 0x0F) << 4),
            4 | 6 => (self.reg[0] & 0x0E) | (((self.reg[1] >> 1) & 1)) | ((self.reg[2] & 0x0F) << 4),
            _ => (self.reg[0] & 0x0F) | ((self.reg[2] & 0x0F) << 4),
        }
    }

    fn chr_split(&self) -> bool {
        (self.reg[0] & 0x80) != 0
    }
}

impl Mapper for Mapper162 {
    fn reset(&mut self) {
        self.reg = [3, 0, 0, 7, 0, 0, 0, 0];
        self.prg_ram = [0; 0x2000];
        self.pa09 = false;
        self.pa13 = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x6000 {
            return FetchResult { data: 0, driven: true };
        }
        if address >= 0x6000 && address < 0x8000 {
            return FetchResult { data: self.prg_ram[(address & 0x1FFF) as usize], driven: true };
        }
        let bank = self.prg_bank() as usize;
        let num_32k = cart.prg_rom.len() / 0x8000;
        if num_32k == 0 {
            return FetchResult { data: 0, driven: true };
        }
        let bank_idx = bank % num_32k;
        let offset = bank_idx * 0x8000 + (address as usize & 0x7FFF);
        FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x5400 {
            let index = ((address >> 8) & 3) as usize;
            self.reg[index] = data;
        } else if address >= 0x6000 && address < 0x8000 {
            self.prg_ram[(address & 0x1FFF) as usize] = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.nametable_horizontal_mirroring {
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
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        let pa13_new = (address & 0x2000) != 0;
        if !self.pa13 && pa13_new {
            self.pa09 = (address & 0x0200) != 0;
        }
        self.pa13 = pa13_new;
        if address < 0x2000 {
            let offset = if self.chr_split() {
                let group = if self.pa09 { 0x1000 } else { 0 };
                let bank_low = (address as usize >> 10) & 3;
                group + bank_low * 0x400 + (address as usize & 0x3FF)
            } else {
                address as usize & 0x1FFF
            };
            let byte = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[offset % chr_ram.len()] }
            } else if chr_rom.is_empty() {
                0
            } else {
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if nametable_horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[address as usize % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.reg[..4]);
        state.extend_from_slice(&self.prg_ram);
        state.push(if self.pa09 { 1 } else { 0 });
        state.push(if self.pa13 { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for r in &mut self.reg[..4] { *r = state[p]; p += 1; }
        for b in self.prg_ram.iter_mut() { *b = state[p]; p += 1; }
        self.pa09 = state[p] != 0; p += 1;
        self.pa13 = state[p] != 0; p += 1;
        p - start
    }
}
