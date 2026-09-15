use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper122 {
    reg: [u8; 2],
}

impl Mapper122 {
    pub fn new() -> Self {
        Self { reg: [0; 2] }
    }
}

impl Mapper for Mapper122 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let offset = address as usize & 0x7FFF;
            FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.reg[(address as usize) & 1] = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        crate::mappers::uxrom::mirror_address(
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
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        let byte = if address < 0x2000 {
            let bank_idx = (address >> 12) as usize; 
            let bank = self.reg[bank_idx] as usize;
            let offset = bank * 0x1000 + (address as usize & 0xFFF);
            if using_chr_ram {
                chr_ram[offset % chr_ram.len().max(1)]
            } else {
                chr_rom[offset % chr_rom.len().max(1)]
            }
        } else if address < 0x3F00 {
            let mirrored = crate::mappers::uxrom::mirror_address(
                alternative_nametable_arrangement,
                nametable_horizontal_mirroring,
                address,
            );
            vram[(mirrored & 0x7FF) as usize]
        } else {
            0
        };
        new_addr_bus |= byte as u16;
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram {
                let bank_idx = (address >> 12) as usize;
                let bank = self.reg[bank_idx] as usize;
                let offset = bank * 0x1000 + (address as usize & 0xFFF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len.max(1)] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = crate::mappers::uxrom::mirror_address(
                cart.alternative_nametable_arrangement,
                cart.nametable_horizontal_mirroring,
                address,
            );
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        self.reg.to_vec()
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let count = self.reg.len().min(state.len().saturating_sub(start));
        self.reg.copy_from_slice(&state[start..start + count]);
        start + count
    }

    fn reset(&mut self) {
        self.reg = [0; 2];
    }
}
