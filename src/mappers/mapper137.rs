use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper137 {
    reg: [u8; 8],
    index: u8,
}

impl Mapper137 {
    pub fn new() -> Self {
        Self {
            reg: [0; 8],
            index: 0,
        }
    }
}

impl Mapper for Mapper137 {
    fn reset(&mut self) {
        self.index = 0;
        self.reg = [0; 8];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let num_32k_banks = cart.prg_rom.len() / 0x8000;
            if num_32k_banks == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let bank = (self.reg[5] as usize) % num_32k_banks;
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x4000 && (address & 0x100) != 0 {
            if (address & 1) != 0 {
                self.reg[(self.index & 7) as usize] = data;
            } else {
                self.index = data;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        match self.reg[7] & 7 {
            2 => {
                address & 0x37FF
            }
            4 => {
                let table = (address >> 10) & 3;
                let mirrored_table = if table == 3 { 1 } else { 0 };
                (address & 0x3FF) | (mirrored_table << 10)
            }
            6 => {
                address & 0x3FF
            }
            _ => {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            }
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
            let bank_1k = (address >> 10) & 7; 
            let bank = if bank_1k < 4 {
                let reg_index = if (self.reg[7] & 1) != 0 { 0 } else { bank_1k as u8 };
                let base_bank = (self.reg[reg_index as usize] & 0x07) as usize;
                match bank_1k {
                    0 => base_bank,
                    1 => base_bank | (((self.reg[4] << 4) & 0x10) as usize),
                    2 => base_bank | (((self.reg[4] << 3) & 0x10) as usize),
                    3 => base_bank | (((self.reg[6] << 3) & 0x08) as usize) | (((self.reg[4] << 2) & 0x10) as usize),
                    _ => base_bank,
                }
            } else {
                (0xFF << 2) + ((bank_1k - 4) as usize)
            };
            let offset = (bank * 0x400) + (address as usize & 0x3FF);
            if using_chr_ram {
                let len = chr_ram.len();
                if len > 0 {
                    new_addr_bus |= chr_ram[offset % len] as u16;
                }
            } else {
                let len = chr_rom.len();
                if len > 0 {
                    new_addr_bus |= chr_rom[offset % len] as u16;
                }
            }
        } else {
            let mirrored = match self.reg[7] & 7 {
                2 => {
                    address & 0x37FF
                }
                4 => {
                    let table = (address >> 10) & 3;
                    let mirrored_table = if table == 3 { 1 } else { 0 };
                    (address & 0x3FF) | (mirrored_table << 10)
                }
                6 => {
                    address & 0x3FF
                }
                _ => {
                    (address & 0x33FF) | ((address & 0x0800) >> 1)
                }
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            let bank_1k = (address >> 10) & 7; 
            let bank = if bank_1k < 4 {
                let reg_index = if (self.reg[7] & 1) != 0 { 0 } else { bank_1k as u8 };
                let base_bank = (self.reg[reg_index as usize] & 0x07) as usize;
                match bank_1k {
                    0 => base_bank,
                    1 => base_bank | (((self.reg[4] << 4) & 0x10) as usize),
                    2 => base_bank | (((self.reg[4] << 3) & 0x10) as usize),
                    3 => base_bank | (((self.reg[6] << 3) & 0x08) as usize) | (((self.reg[4] << 2) & 0x10) as usize),
                    _ => base_bank,
                }
            } else {
                (0xFF << 2) + ((bank_1k - 4) as usize)
            };
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let offset = (bank * 0x400) + (address as usize & 0x3FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = match self.reg[7] & 7 {
                2 => {
                    address & 0x37FF
                }
                4 => {
                    let table = (address >> 10) & 3;
                    let mirrored_table = if table == 3 { 1 } else { 0 };
                    (address & 0x3FF) | (mirrored_table << 10)
                }
                6 => {
                    address & 0x3FF
                }
                _ => {
                    (address & 0x33FF) | ((address & 0x0800) >> 1)
                }
            };
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = vec![self.index];
        state.extend_from_slice(&self.reg);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 9 <= state.len() {
            self.index = state[start]; start += 1;
            for i in 0..8 {
                self.reg[i] = state[start]; start += 1;
            }
        }
        start
    }
}
