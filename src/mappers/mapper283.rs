use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::nrom::mirror_address;

pub struct Mapper283 {
    latch: u8,
    prg_68k_bank: usize,
    prg_32k_bank: usize,
}

impl Mapper283 {
    pub fn new() -> Self {
        Self { latch: 0, prg_68k_bank: 0, prg_32k_bank: 0 }
    }

    fn sync(&mut self, cart: &Cartridge) {
        if cart.prg_rom.len() & 0x6000 != 0 {
            self.prg_68k_bank = 0x20;
        } else {
            self.prg_68k_bank = 0x1F;
        }
        self.prg_32k_bank = self.latch as usize;
    }
}

impl Mapper for Mapper283 {
    fn reset(&mut self) {
        self.latch = 0;
        self.prg_68k_bank = 0;
        self.prg_32k_bank = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            let num_8k = cart.prg_rom.len() / 0x2000;
            if num_8k == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let bank = self.prg_68k_bank % num_8k;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0x8000 {
            let num_32k = cart.prg_rom.len() / 0x8000;
            if num_32k == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let bank = self.prg_32k_bank % num_32k;
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.latch = data;
            self.sync(cart);
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        mirror_address(
            cart.alternative_nametable_arrangement,
            cart.nametable_horizontal_mirroring,
            address,
        )
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = if !chr_ram.is_empty() {
                chr_ram[address as usize & 0x1FFF]
            } else if !chr_rom.is_empty() {
                chr_rom[address as usize & 0x1FFF]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mirror_address(
                _alternative_nametable_arrangement,
                _nametable_horizontal_mirroring,
                address,
            );
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                cart.chr_ram[address as usize & 0x1FFF] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = mirror_address(
                cart.alternative_nametable_arrangement,
                cart.nametable_horizontal_mirroring,
                address,
            );
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.latch]
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start < state.len() {
            self.latch = state[start];
            self.sync(cart);
            start + 1
        } else {
            start
        }
    }
}
