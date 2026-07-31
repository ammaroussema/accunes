use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper550 {
    outer_bank: u8,
    latch: u8,
    reg: [u8; 4],
    shift: u8,
    bits: u8,
    filter: u8,
}

impl Mapper550 {
    pub fn new() -> Self {
        Self {
            outer_bank: 0,
            latch: 0,
            reg: [0x0C, 0, 0, 0],
            shift: 0,
            bits: 0,
            filter: 0,
        }
    }

    fn get_prg_bank(&self, bank: usize) -> u8 {
        let prg = self.reg[3];
        let result = if (self.reg[0] & 0x08) != 0 {
            if (self.reg[0] & 0x04) != 0 {
                prg | (bank as u8 * 0x0F)
            } else {
                prg & (bank as u8 * 0x0F)
            }
        } else {
            prg & !1 | bank as u8
        };
        if (self.reg[3] & 0x10) != 0 {
            (result & 0x07) | (prg & 0x08)
        } else {
            result & 0x0F
        }
    }

    fn get_chr_bank(&self, bank: usize) -> u8 {
        if (self.reg[0] & 0x10) != 0 {
            self.reg[1 + bank]
        } else {
            self.reg[1] & !1 | bank as u8
        }
    }

    fn prg_offset(&self, cart: &Cartridge, address: u16) -> usize {
        let prg_len = cart.prg_rom.len();
        if prg_len == 0 {
            return 0;
        }
        if (self.outer_bank & 6) == 6 {
            let and = 0x07usize;
            let or = (self.outer_bank as usize) << 2;
            let bank16 = if address < 0xC000 {
                (self.get_prg_bank(0) as usize & and) | or
            } else {
                (self.get_prg_bank(1) as usize & and) | or
            };
            (bank16 * 0x4000 + (address as usize & 0x3FFF)) % prg_len
        } else {
            let bank32 = ((self.latch >> 4) as usize) | ((self.outer_bank as usize) << 1);
            (bank32 * 0x8000 + (address as usize & 0x7FFF)) % prg_len
        }
    }

    fn chr_offset(&self, address: u16, chr_len: usize) -> usize {
        if chr_len == 0 {
            return 0;
        }
        if (self.outer_bank & 6) == 6 {
            let and = 0x07usize;
            let or = ((self.outer_bank as usize) << 2) & 0x18;
            let bank4 = if address < 0x1000 {
                (self.get_chr_bank(0) as usize & and) | or
            } else {
                (self.get_chr_bank(1) as usize & and) | or
            };
            (bank4 * 0x1000 + (address as usize & 0x0FFF)) % chr_len
        } else {
            let bank8 = ((self.latch & 3) as usize) | (((self.outer_bank as usize) << 1) & 0x0C);
            (bank8 * 0x2000 + (address as usize & 0x1FFF)) % chr_len
        }
    }

    fn mmc1_write(&mut self, address: u16, data: u8) {
        if (data & 0x80) != 0 {
            self.reg[0] |= 0x0C;
            self.shift = 0;
            self.bits = 0;
        } else if self.filter == 0 {
            self.shift |= (data & 1) << self.bits;
            self.bits += 1;
            if self.bits == 5 {
                let reg = ((address >> 13) & 3) as usize;
                self.reg[reg] = self.shift;
                self.shift = 0;
                self.bits = 0;
            }
        }
        self.filter = 2;
    }

    fn mirror_for_ppu(&self, address: u16) -> u16 {
        match self.reg[0] & 0x03 {
            0 => address & 0x23FF,
            1 => (address & 0x23FF) | 0x0400,
            2 => address & 0x37FF,
            _ => (address & 0x33FF) | ((address & 0x0800) >> 1),
        }
    }
}

impl Mapper for Mapper550 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            FetchResult {
                data: cart.prg_rom[self.prg_offset(cart, address)],
                driven: true,
            }
        } else if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let mask = cart.prg_ram.len() - 1;
                FetchResult {
                    data: cart.prg_ram[(address as usize - 0x6000) & mask],
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
        if address >= 0x6000 && address < 0x7000 {
            if !cart.prg_ram.is_empty() {
                let mask = cart.prg_ram.len() - 1;
                cart.prg_ram[(address as usize - 0x6000) & mask] = data;
            }
        } else if address >= 0x7000 && address < 0x8000 {
            if (self.outer_bank & 8) == 0 {
                self.outer_bank = (address & 0x0F) as u8;
            }
            if !cart.prg_ram.is_empty() {
                let mask = cart.prg_ram.len() - 1;
                cart.prg_ram[(address as usize - 0x6000) & mask] = data;
            }
        } else if address >= 0x8000 {
            self.latch = data;
            if (self.outer_bank & 6) == 6 {
                self.mmc1_write(address, data);
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror_for_ppu(address)
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
            let len = if using_chr_ram { chr_ram.len() } else { chr_rom.len() };
            let offset = self.chr_offset(address, len);
            let data = if using_chr_ram { chr_ram[offset] } else { chr_rom[offset] };
            new_addr_bus |= data as u16;
        } else if address < 0x3F00 {
            let mirrored = self.mirror_for_ppu(address);
            let data = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= data as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let chr_len = cart.chr_ram.len();
            if chr_len > 0 {
                let offset = self.chr_offset(address, chr_len);
                cart.chr_ram[offset] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_for_ppu(address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.filter > 0 {
            self.filter = self.filter.saturating_sub(cycles);
        }
        false
    }

    fn reset(&mut self) {
        self.latch = 0;
        self.outer_bank = 0;
        self.shift = 0;
        self.bits = 0;
        self.filter = 0;
        self.reg = [0x0C, 0, 0, 0];
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        for &r in &self.reg {
            state.push(r);
        }
        state.push(self.shift);
        state.push(self.bits);
        state.push(self.filter);
        state.push(self.outer_bank);
        state.push(self.latch);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        let expected = 4 + 5;
        if start + expected <= state.len() {
            for i in 0..4 {
                self.reg[i] = state[start + i];
            }
            start += 4;
            self.shift = state[start]; start += 1;
            self.bits = state[start]; start += 1;
            self.filter = state[start]; start += 1;
            self.outer_bank = state[start]; start += 1;
            self.latch = state[start]; start += 1;
        }
        start
    }
}
