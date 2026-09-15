use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper228 {
    mram: [u8; 4],
    areg: u16,
    vreg: u8,
}

impl Mapper228 {
    pub fn new() -> Self {
        Self { mram: [0; 4], areg: 0x8000, vreg: 0 }
    }
}

impl Mapper for Mapper228 {
    fn reset(&mut self) {
        self.areg = 0x8000;
        self.vreg = 0;
        self.mram = [0; 4];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x5000 && address <= 0x5FFF {
            return FetchResult { data: self.mram[address as usize & 3], driven: true };
        }
        if address >= 0x8000 {
            let page = ((self.areg >> 7) & 0x3F) as usize;
            let page_adj = if (page & 0x30) == 0x30 { page - 0x10 } else { page };
            let prgl = (page_adj << 1) + (((self.areg >> 6) & 1) & ((self.areg >> 5) & 1)) as usize;
            let prgh = prgl + (((self.areg >> 5) & 1) ^ 1) as usize;
            let slot = if address < 0xC000 { 0 } else { 1 };
            let bank = if slot == 0 { prgl } else { prgh };
            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            return FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address <= 0x5FFF {
            self.mram[address as usize & 3] = data & 0x0F;
            return;
        }
        if address >= 0x8000 {
            self.areg = address;
            self.vreg = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if ((self.areg >> 13) ^ 1) & 1 != 0 {
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
            let bank = ((self.vreg & 0x03) as usize) | (((self.areg as u8 & 0x0F) as usize) << 2);
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if ((self.areg >> 13) ^ 1) & 1 != 0 {
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
            let bank = ((self.vreg & 0x03) as usize) | (((self.areg as u8 & 0x0F) as usize) << 2);
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let len = cart.chr_ram.len();
            if len > 0 { cart.chr_ram[offset % len] = data; }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut v = self.mram.to_vec();
        v.push(self.areg as u8);
        v.push((self.areg >> 8) as u8);
        v.push(self.vreg);
        v
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..4 { if p < state.len() { self.mram[i] = state[p]; p += 1; } }
        if p < state.len() { self.areg = state[p] as u16; p += 1; }
        if p < state.len() { self.areg |= (state[p] as u16) << 8; p += 1; }
        if p < state.len() { self.vreg = state[p]; p += 1; }
        p
    }
}
