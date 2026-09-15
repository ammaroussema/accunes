use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper365 {
    prg: [u8; 4], 
    chr: [u8; 8], 
    keyboard_row: u8,
}

impl Mapper365 {
    pub fn new() -> Self {
        let mut m = Mapper365 {
            prg: [0; 4],
            chr: [0; 8],
            keyboard_row: 0,
        };
        m.reset();
        m
    }

    fn prg_bank_fixed(&self, slot: usize) -> u8 {
        if slot == 3 {
            self.prg[3] | 0x01
        } else {
            self.prg[slot]
        }
    }

    fn chr_bank_count(chr_len: usize) -> usize {
        (chr_len / 0x400).max(1)
    }
}

impl Mapper for Mapper365 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address == 0x4906 {
            return FetchResult { data: 0, driven: false };
        }
        if address >= 0x8000 {
            let slot = ((address - 0x8000) / 0x2000) as usize; 
            let bank = self.prg_bank_fixed(slot) as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if address >= 0x6000 {
            let offset = (address - 0x6000) as usize;
            if offset < cart.prg_ram.len() {
                FetchResult { data: cart.prg_ram[offset], driven: true }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address == 0x4904 {
            self.keyboard_row = data;
            return;
        }
        if address == 0x4905 {
            return;
        }
        if address >= 0x8000 && address <= 0x9FFF {
            let idx = (address as usize) & 0x03;
            self.prg[idx] = data;
        } else if address >= 0xA000 && address <= 0xBFFF {
            let idx = (address as usize) & 0x07;
            self.chr[idx] = data;
        } else if address >= 0x6000 {
            let offset = (address - 0x6000) as usize;
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else if cart.nametable_horizontal_mirroring {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let slot = ((address >> 10) & 0x07) as usize; 
            let bank_count = Self::chr_bank_count(chr_ram.len());
            let bank = (self.chr[slot] as usize) % bank_count;
            let offset = bank * 0x400 + (address as usize & 0x03FF);
            let byte = if chr_ram.is_empty() { 0 } else { chr_ram[offset % chr_ram.len()] };
            new_addr_bus |= byte as u16;
        } else if address < 0x3F00 {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if nametable_horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            let slot = ((address >> 10) & 0x07) as usize;
            let len = cart.chr_ram.len();
            if len > 0 {
                let bank_count = Self::chr_bank_count(len);
                let bank = (self.chr[slot] as usize) % bank_count;
                let offset = (bank * 0x400 + (address as usize & 0x03FF)) % len;
                cart.chr_ram[offset] = data;
            }
        } else if address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn reset(&mut self) {
        for i in 0..4 {
            self.prg[i] = (i as u8) | 0xFC;
        }
        self.chr = [0; 8];
        self.keyboard_row = 0;
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.extend_from_slice(&self.prg);
        state.extend_from_slice(&self.chr);
        state.push(self.keyboard_row);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        let prg_len = cart.prg_ram.len();
        if start + prg_len <= state.len() {
            cart.prg_ram.copy_from_slice(&state[start..start + prg_len]);
            start += prg_len;
        }
        let chr_len = cart.chr_ram.len();
        if start + chr_len <= state.len() {
            cart.chr_ram.copy_from_slice(&state[start..start + chr_len]);
            start += chr_len;
        }
        if start + 4 <= state.len() {
            self.prg.copy_from_slice(&state[start..start + 4]);
            start += 4;
        }
        if start + 8 <= state.len() {
            self.chr.copy_from_slice(&state[start..start + 8]);
            start += 8;
        }
        if start < state.len() {
            self.keyboard_row = state[start];
            start += 1;
        }
        start
    }
}
