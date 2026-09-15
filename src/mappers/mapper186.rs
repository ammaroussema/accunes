use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const WRAM_SIZE: usize = 1024;

pub struct Mapper186 {
    reg: [u8; 4],
    wram: [u8; WRAM_SIZE],
}

impl Mapper186 {
    pub fn new() -> Self {
        Self { reg: [0; 4], wram: [0; WRAM_SIZE] }
    }
}

impl Mapper for Mapper186 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if address < 0xC000 {
                let bank = self.reg[1] as usize;
                let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
            } else {
                let offset = address as usize & 0x3FFF;
                FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
            }
        } else if address >= 0x6000 {
            let off = (address - 0x6000) as usize;
            if off < cart.prg_ram.len() {
                FetchResult { data: cart.prg_ram[off], driven: true }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else if address >= 0x4000 {
            let addr_in_page = (address & 0xFFF) as usize;
            if addr_in_page == 0x202 {
                FetchResult { data: 0x40, driven: true }
            } else if (0x200..=0x203).contains(&addr_in_page) {
                FetchResult { data: 0x00, driven: true }
            } else if addr_in_page >= 0x400 && addr_in_page < 0x800 {
                FetchResult { data: self.wram[addr_in_page - 0x400], driven: true }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            if address < 0xC000 {
                let bank = self.reg[1] as usize;
                let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                if offset % cart.prg_rom.len() < cart.prg_rom.len() {
                }
            }
        } else if address >= 0x6000 {
            let off = (address - 0x6000) as usize;
            if off < cart.prg_ram.len() {
                cart.prg_ram[off] = data;
            }
        } else if address >= 0x4000 {
            let addr_in_page = (address & 0xFFF) as usize;
            if (0x200..=0x203).contains(&addr_in_page) {
                self.reg[addr_in_page & 3] = data;
            } else if addr_in_page >= 0x400 && addr_in_page < 0x800 {
                self.wram[addr_in_page - 0x400] = data;
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
            let data = if using_chr_ram && !chr_ram.is_empty() { chr_ram[address as usize % chr_ram.len()] } else { chr_rom[address as usize % chr_rom.len()] };
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
        let mut state = Vec::with_capacity(4 + WRAM_SIZE);
        state.extend_from_slice(&self.reg);
        state.extend_from_slice(&self.wram);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..4 {
            if p < state.len() { self.reg[i] = state[p]; p += 1; }
        }
        for i in 0..WRAM_SIZE {
            if p < state.len() { self.wram[i] = state[p]; p += 1; }
        }
        p
    }

    fn reset(&mut self) {
        self.reg = [0; 4];
    }
}
