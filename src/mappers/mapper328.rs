use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper328 {
    rng_seed: u32,
}

impl Mapper328 {
    pub fn new() -> Self {
        Self { rng_seed: 1 }
    }

    fn rand_bits(&mut self) -> u8 {
        self.rng_seed = self.rng_seed.wrapping_mul(1103515245).wrapping_add(12345);
        ((self.rng_seed >> 16) & 0x0D) as u8
    }
}

impl Mapper for Mapper328 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let offset = address as usize & 0x7FFF;
            let data = cart.prg_rom[offset % cart.prg_rom.len()];
            if (address >= 0xCE80 && address < 0xCF00) || (address >= 0xFE80 && address < 0xFF00) {
                FetchResult { data: 0xF2 | self.rand_bits(), driven: true }
            } else {
                FetchResult { data, driven: true }
            }
        } else {
            FetchResult { data: 0, driven: false }
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
            let data = if using_chr_ram { chr_ram[address as usize % chr_ram.len()] } else { chr_rom[address as usize % chr_rom.len()] };
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

    fn store_prg(&mut self, _cart: &mut Cartridge, _address: u16, _data: u8) {}

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        self.rng_seed.to_le_bytes().to_vec()
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start + 4 <= state.len() {
            self.rng_seed = u32::from_le_bytes([state[start], state[start + 1], state[start + 2], state[start + 3]]);
            start + 4
        } else {
            start
        }
    }

    fn reset(&mut self) {
        self.rng_seed = 1;
    }
}
