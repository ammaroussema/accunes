use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper290 {
    prg_bank_0: u8,
    prg_bank_1: u8,
    prg_mode_32k: bool,
    chr_bank: u8,
    horizontal_mirroring: bool,
}

impl Mapper290 {
    pub fn new() -> Self {
        Mapper290 {
            prg_bank_0: 0,
            prg_bank_1: 0,
            prg_mode_32k: false,
            chr_bank: 0,
            horizontal_mirroring: false,
        }
    }

    fn write_register(&mut self, address: u16) {
        let prg = ((address >> 10) & 0x1E) as u8;
        let chr = (((address & 0x0300) >> 5) | (address & 0x07)) as u8;
        if address & 0x80 != 0 {
            self.prg_mode_32k = false;
            let bank = prg | ((address >> 6) & 1) as u8;
            self.prg_bank_0 = bank;
            self.prg_bank_1 = bank;
        } else {
            self.prg_mode_32k = true;
            let bank = prg & 0xFE;
            self.prg_bank_0 = bank;
            self.prg_bank_1 = bank | 1;
        }
        self.chr_bank = chr;
        self.horizontal_mirroring = address & 0x400 != 0;
    }
}

impl Mapper for Mapper290 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = if address < 0xC000 {
                self.prg_bank_0 as usize
            } else {
                self.prg_bank_1 as usize
            };
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
            self.write_register(address);
        } else if address >= 0x6000 {
            let offset = address as usize & 0x1FFF;
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else if self.horizontal_mirroring {
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
            let chr_bank = self.chr_bank as usize;
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

    fn reset(&mut self) {
        self.write_register(0x8000);
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.push(self.prg_bank_0);
        state.push(self.prg_bank_1);
        state.push(self.prg_mode_32k as u8);
        state.push(self.chr_bank);
        state.push(self.horizontal_mirroring as u8);
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
        if start + 1 <= state.len() {
            self.prg_bank_0 = state[start];
            start += 1;
        }
        if start + 1 <= state.len() {
            self.prg_bank_1 = state[start];
            start += 1;
        }
        if start + 1 <= state.len() {
            self.prg_mode_32k = state[start] != 0;
            start += 1;
        }
        if start + 1 <= state.len() {
            self.chr_bank = state[start];
            start += 1;
        }
        if start + 1 <= state.len() {
            self.horizontal_mirroring = state[start] != 0;
            start += 1;
        }
        start
    }
}
