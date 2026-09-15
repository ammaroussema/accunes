use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper476 {
    reg: [u16; 4],
}

impl Mapper476 {
    pub fn new() -> Self {
        Self { reg: [0; 4] }
    }
}

impl Mapper for Mapper476 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = self.reg[0] as usize;
            if (self.reg[2] & 4) != 0 {
                let offset = bank * 0x8000 + (address as usize & 0x7FFF);
                FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
            } else {
                let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address <= 0x5FFF {
            let idx = ((address >> 8) & 3) as usize;
            self.reg[idx] = data as u16;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if (self.reg[2] & 1) != 0 {
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
            let data = if using_chr_ram && !chr_ram.is_empty() { chr_ram[address as usize % chr_ram.len()] } else { chr_rom[address as usize % chr_rom.len()] };
            new_addr_bus |= data as u16;
        } else {
            let h = (self.reg[2] & 1) != 0;
            let mirrored = if h { (address & 0x33FF) | ((address & 0x0800) >> 1) } else { address & 0x37FF };
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
        let mut state = Vec::with_capacity(8);
        for &r in &self.reg {
            state.extend_from_slice(&r.to_le_bytes());
        }
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..4 {
            if p + 2 <= state.len() {
                self.reg[i] = u16::from_le_bytes([state[p], state[p + 1]]);
                p += 2;
            }
        }
        p
    }

    fn reset(&mut self) {
        self.reg = [0; 4];
    }
}
