use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const LUT_PRG: [[u8; 4]; 4] = [
    [0, 1, 2, 3],
    [3, 2, 1, 0],
    [0, 2, 1, 3],
    [3, 1, 2, 0],
];
const LUT_CHR: [[u8; 8]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 2, 1, 3, 4, 6, 5, 7],
    [0, 1, 4, 5, 2, 3, 6, 7],
    [0, 4, 1, 5, 2, 6, 3, 7],
    [0, 4, 2, 6, 1, 5, 3, 7],
    [0, 2, 4, 6, 1, 3, 5, 7],
    [7, 6, 5, 4, 3, 2, 1, 0],
    [7, 6, 5, 4, 3, 2, 1, 0],
];

pub struct Mapper244 {
    prg: u8,
    chr: u8,
}

impl Mapper244 {
    pub fn new() -> Self {
        Self { prg: 0, chr: 0 }
    }
}

impl Mapper for Mapper244 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = self.prg as usize;
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            if (data & 8) != 0 {
                self.chr = LUT_CHR[((data >> 4) & 7) as usize][(data & 7) as usize];
            } else {
                self.prg = LUT_PRG[((data >> 4) & 3) as usize][(data & 3) as usize];
            }
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
        if address < 0x2000 {
            let bank = self.chr as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let data = if using_chr_ram { chr_ram[offset % chr_ram.len()] } else { chr_rom[offset % chr_rom.len()] };
            new_addr_bus |= data as u16;
        } else {
            let mirrored = if !nametable_horizontal_mirroring { address & 0x37FF } else { (address & 0x33FF) | ((address & 0x0800) >> 1) };
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
        vec![self.prg, self.chr]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.prg = state[p]; p += 1; }
        if p < state.len() { self.chr = state[p]; p += 1; }
        p
    }

    fn reset(&mut self) {
        self.prg = 0;
        self.chr = 0;
    }
}
