use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper32 {
    preg: [u8; 2],
    creg: [u8; 8],
    mirr: u8,
}

impl Mapper32 {
    pub fn new() -> Self {
        Self {
            preg: [0, 0],
            creg: [0; 8],
            mirr: 0,
        }
    }
}

impl Mapper for Mapper32 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let swap = ((self.mirr & 2) as u16) << 13;
            let effective_addr = address ^ swap;
            let num_banks = cart.prg_rom.len() / 0x2000;
            let last_bank = num_banks - 1;
            let actual_bank = match effective_addr {
                0x8000..=0x9FFF => self.preg[0] as usize,
                0xA000..=0xBFFF => self.preg[1] as usize,
                0xC000..=0xDFFF => last_bank.saturating_sub(1),
                0xE000..=0xFFFF => last_bank,
                _ => 0,
            };
            let offset = actual_bank * 0x2000 + (effective_addr as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if address >= 0x6000 {
            let offset = (address - 0x6000) as usize;
            if offset < cart.prg_ram.len() {
                FetchResult {
                    data: cart.prg_ram[offset],
                    driven: true,
                }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            match address {
                0x8000..=0x8FFF => self.preg[0] = data,
                0x9000..=0x9FFF => self.mirr = data,
                0xA000..=0xAFFF => self.preg[1] = data,
                0xB000..=0xBFFF => self.creg[(address & 7) as usize] = data,
                _ => {}
            }
        } else if address >= 0x6000 {
            let offset = (address - 0x6000) as usize;
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if (self.mirr & 1) == 0 {
            address & 0x37FF 
        } else {
            (address & 0x33FF) | ((address & 0x0800) >> 1) 
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
            let bank = self.creg[(address >> 10) as usize] as usize;
            let offset = bank * 0x0400 + (address as usize & 0x03FF);
            if using_chr_ram {
                if !chr_ram.is_empty() {
                    let mask = chr_ram.len() - 1;
                    new_addr_bus |= chr_ram[offset & mask] as u16;
                }
            } else if !chr_rom.is_empty() {
                let mask = chr_rom.len() - 1;
                new_addr_bus |= chr_rom[offset & mask] as u16;
            }
        } else {
            let mirrored = if (self.mirr & 1) == 0 {
                address & 0x37FF 
            } else {
                (address & 0x33FF) | ((address & 0x0800) >> 1) 
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&self.preg);
        state.extend_from_slice(&self.creg);
        state.push(self.mirr);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..cart.prg_ram.len() {
            if p < state.len() {
                cart.prg_ram[i] = state[p];
                p += 1;
            }
        }
        for i in 0..2 {
            if p < state.len() {
                self.preg[i] = state[p];
                p += 1;
            }
        }
        for i in 0..8 {
            if p < state.len() {
                self.creg[i] = state[p];
                p += 1;
            }
        }
        if p < state.len() {
            self.mirr = state[p];
            p += 1;
        }
        p
    }

    fn reset(&mut self) {
        self.preg = [0, 0];
        self.creg = [0; 8];
        self.mirr = 0;
    }
}
