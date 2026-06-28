use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper62 {
    bank: u8,
    mode: u16,
}

impl Mapper62 {
    pub fn new() -> Self {
        Mapper62 {
            bank: 0,
            mode: 0,
        }
    }
}

impl Mapper for Mapper62 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let prg_bank = ((self.mode & 0x40) as usize) | (((self.mode >> 8) & 0x3F) as usize);
            let bank = if self.mode & 0x20 != 0 {
                prg_bank
            } else {
                prg_bank >> 1
            };
            let offset = if self.mode & 0x20 != 0 {
                (bank * 0x4000) + (address as usize & 0x3FFF)
            } else {
                (bank * 0x8000) + (address as usize & 0x7FFF)
            };
            let final_offset = offset % cart.prg_rom.len();
            FetchResult { data: cart.prg_rom[final_offset], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        self.mode = (address & 0x3FFF) as u16;
        self.bank = data & 3;
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let mirror_mode = ((self.mode >> 7) & 1) ^ 1;
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
            let chr_bank = (((self.mode & 0x1F) as usize) << 2) | ((self.bank & 0x03) as usize);
            let offset = (chr_bank * 0x2000) + (address as usize & 0x1FFF);
            if using_chr_ram {
                new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
            } else {
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else {
            let mirror_mode = ((self.mode >> 7) & 1) ^ 1;
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
        state.push(self.bank);
        state.extend_from_slice(&self.mode.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 3 <= state.len() {
            self.bank = state[start];
            self.mode = u16::from_le_bytes([state[start + 1], state[start + 2]]);
            start += 3;
        }
        start
    }

    fn reset(&mut self) {
        self.bank = 0;
        self.mode = 0;
    }
}
