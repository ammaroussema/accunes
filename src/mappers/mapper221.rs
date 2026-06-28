use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper221 {
    mode: u16,
    prg_reg: u8,
}

impl Mapper221 {
    pub fn new() -> Self {
        Mapper221 {
            mode: 0,
            prg_reg: 0,
        }
    }

    fn update_state(&self) -> (usize, usize, bool) {
        let outer_bank = ((self.mode & 0xFC) >> 2) as usize;
        let (prg_bank_0, prg_bank_1) = if self.mode & 0x02 != 0 {
            if self.mode & 0x0100 != 0 {
                (outer_bank | (self.prg_reg as usize), outer_bank | 0x07)
            } else {
                let bank = outer_bank | ((self.prg_reg & 0x06) as usize);
                (bank, bank | 1)
            }
        } else {
            let bank = outer_bank | (self.prg_reg as usize);
            (bank, bank)
        };
        let horizontal_mirroring = self.mode & 0x01 != 0;
        (prg_bank_0, prg_bank_1, horizontal_mirroring)
    }
}

impl Mapper for Mapper221 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let (prg_bank_0, prg_bank_1, _) = self.update_state();
            let bank = if address < 0xC000 { prg_bank_0 } else { prg_bank_1 };
            let offset = (bank * 0x4000) + (address as usize & 0x3FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if address >= 0x6000 {
            let offset = address as usize & 0x1FFF;
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
            match address & 0xC000 {
                0x8000 => {
                    self.mode = address as u16;
                }
                0xC000 => {
                    self.prg_reg = (address & 0x07) as u8;
                }
                _ => {}
            }
        } else if address >= 0x6000 {
            let offset = address as usize & 0x1FFF;
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        let (_, _, horizontal_mirroring) = self.update_state();
        if cart.alternative_nametable_arrangement {
            address
        } else if horizontal_mirroring {
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
            let chr_bank = 0;
            let offset = (chr_bank * 0x2000) + (address as usize & 0x1FFF);
            if using_chr_ram && !chr_ram.is_empty() {
                let mask = chr_ram.len() - 1;
                new_addr_bus |= chr_ram[offset & mask] as u16;
            } else if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let idx = (address & 0x7FF) as usize;
            new_addr_bus |= vram[idx] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.extend_from_slice(&self.mode.to_le_bytes());
        state.push(self.prg_reg);
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
        if start + 2 <= state.len() {
            self.mode = u16::from_le_bytes([state[start], state[start + 1]]);
            start += 2;
        }
        if start + 1 <= state.len() {
            self.prg_reg = state[start];
            start += 1;
        }
        start
    }
}
