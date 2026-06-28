use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper486 {
    pointer: u8,
    reg: [u8; 8],
}

impl Mapper486 {
    pub fn new() -> Self {
        Self { pointer: 0, reg: [0, 2, 4, 5, 6, 7, 0, 1] }
    }
}

impl Mapper for Mapper486 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = match address >> 13 {
                0 => self.reg[6] & 0x0F,
                1 => self.reg[7] & 0x0F,
                2 => 0xFE & 0x0F,
                3 | _ => 0xFF & 0x0F,
            };
            let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
            FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 && address <= 0x9FFF {
            let idx = (address & 7) as usize;
            self.reg[idx] = data;
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
            let bank = match (address >> 10) as usize {
                0 | 1 => self.reg[0] & 0x3F,
                2 | 3 => self.reg[1] & 0x3F,
                4 => self.reg[2] & 0x3F,
                5 => self.reg[3] & 0x3F,
                6 => self.reg[4] & 0x3F,
                7 | _ => self.reg[5] & 0x3F,
            };
            let offset = (bank as usize) * 0x400 + (address as usize & 0x3FF);
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
        let mut state = vec![self.pointer];
        state.extend_from_slice(&self.reg);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.pointer = state[p]; p += 1; }
        for i in 0..8 { if p < state.len() { self.reg[i] = state[p]; p += 1; } }
        p
    }

    fn reset(&mut self) {
        self.pointer = 0;
        self.reg = [0, 2, 4, 5, 6, 7, 0, 1];
    }
}
