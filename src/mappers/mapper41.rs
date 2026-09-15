use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper41 {
    mainreg: u8,
    chrreg: u8,
    mirror: u8,
}

impl Mapper41 {
    pub fn new() -> Self {
        Mapper41 {
            mainreg: 0,
            chrreg: 0,
            mirror: 1,
        }
    }
}

impl Mapper for Mapper41 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = (self.mainreg & 7) as usize;
            let offset = (bank * 0x8000) + (address as usize & 0x7FFF);
            let final_offset = offset % cart.prg_rom.len();
            FetchResult { data: cart.prg_rom[final_offset], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x6800 {
            let addr = address as u8;
            self.mainreg = addr & 0xFF;
            self.mirror = ((addr >> 5) & 1) ^ 1;
            self.chrreg = (self.chrreg & 3) | ((addr >> 1) & 0xC);
        }
        else if address >= 0x8000 {
            if self.mainreg & 0x4 != 0 {
                self.chrreg = (self.chrreg & 0xC) | (data & 3);
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mirror == 0 {
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
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let bank = self.chrreg as usize;
            let offset = (bank * 0x2000) + (address as usize & 0x1FFF);
            if using_chr_ram {
                new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
            } else {
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else {
            let mirrored = if nametable_horizontal_mirroring { address & 0x37FF } else { (address & 0x33FF) | ((address & 0x0800) >> 1) };
            let idx = (mirrored & 0x7FF) as usize;
            new_addr_bus |= vram[idx] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.mainreg);
        state.push(self.chrreg);
        state.push(self.mirror);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 3 <= state.len() {
            self.mainreg = state[start];
            self.chrreg = state[start + 1];
            self.mirror = state[start + 2];
            start += 3;
        }
        start
    }

    fn reset(&mut self) {
        self.mainreg = 0;
        self.chrreg = 0;
        self.mirror = 1;
    }
}
