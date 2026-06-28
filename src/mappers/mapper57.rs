use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper57 {
    prg_reg: u8,
    chr_reg: u8,
    hrd_flag: u8,
}

impl Mapper57 {
    pub fn new() -> Self {
        Mapper57 {
            prg_reg: 0,
            chr_reg: 0,
            hrd_flag: 0,
        }
    }
}

impl Mapper for Mapper57 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = if self.prg_reg & 0x80 != 0 {
                (self.prg_reg >> 6) as usize
            } else {
                ((self.prg_reg >> 5) & 3) as usize
            };
            let offset = if self.prg_reg & 0x80 != 0 {
                (bank * 0x8000) + (address as usize & 0x7FFF)
            } else {
                (bank * 0x4000) + (address as usize & 0x3FFF)
            };
            let final_offset = offset % cart.prg_rom.len();
            FetchResult { data: cart.prg_rom[final_offset], driven: true }
        } else if address == 0x6000 {
            FetchResult { data: self.hrd_flag, driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            if (address & 0x8800) == 0x8800 {
                self.prg_reg = data;
            } else {
                self.chr_reg = data;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let mirror_mode = ((self.prg_reg & 8) >> 3) ^ 1;
        if mirror_mode == 0 {
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
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let bank = ((self.chr_reg & 3) | (self.prg_reg & 7) | ((self.prg_reg & 0x10) >> 1)) as usize;
            let offset = (bank * 0x2000) + (address as usize & 0x1FFF);
            if using_chr_ram {
                new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
            } else {
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else {
            let mirror_mode = ((self.prg_reg & 8) >> 3) ^ 1;
            let mirrored = if mirror_mode == 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let idx = (mirrored & 0x7FF) as usize;
            new_addr_bus |= vram[idx] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.hrd_flag);
        state.push(self.prg_reg);
        state.push(self.chr_reg);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 3 <= state.len() {
            self.hrd_flag = state[start];
            self.prg_reg = state[start + 1];
            self.chr_reg = state[start + 2];
            start += 3;
        }
        start
    }

    fn reset(&mut self) {
        self.prg_reg = 0;
        self.chr_reg = 0;
        self.hrd_flag = 0;
    }
}
