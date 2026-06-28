use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper169 {
    mmc3: MapperMMC3,
    reg4800: u8,
    reg5500: u8,
    reg5501: u8,
    ram_latch: u8,
    keyboard_row: u16,
    prg_ram: [u8; 0x10000],
    chr_ram: Vec<u8>,
    pa00: bool,
    pa09: bool,
    pa13: bool,
    pa0809: u16,
}

impl Mapper169 {
    pub fn new() -> Self {
        let config = Mmc3Config::embedded();
        Self {
            mmc3: MapperMMC3::new(config),
            reg4800: 0,
            reg5500: 0,
            reg5501: 0,
            ram_latch: 0,
            keyboard_row: 0,
            prg_ram: [0; 0x10000],
            chr_ram: vec![0; 0x20000],
            pa00: false,
            pa09: false,
            pa13: false,
            pa0809: 0,
        }
    }
}

impl Mapper for Mapper169 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.reg4800 = 0;
        self.reg5500 = 0;
        self.reg5501 = 0;
        self.ram_latch = 0;
        self.keyboard_row = 0;
        self.pa00 = false;
        self.pa09 = false;
        self.pa13 = false;
        self.pa0809 = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if self.reg5501 & 0x80 != 0 {
                self.mmc3.fetch_prg(cart, address)
            } else {
                let len = cart.prg_rom.len();
                if self.reg5500 & 0x04 != 0 {
                    let bank_idx = if self.reg5500 & 0x40 != 0 {
                        (self.ram_latch as usize >> 1) * 0x8000
                    } else if address < 0xC000 {
                        (self.ram_latch as usize) * 0x4000
                    } else {
                        (len / 0x4000).saturating_sub(1) * 0x4000
                    };
                    let offset = bank_idx + (address as usize & 0x3FFF);
                    FetchResult { data: self.prg_ram[offset % self.prg_ram.len()], driven: true }
                } else {
                    let bank_idx = (self.reg4800 as usize & 0x1F) * 0x8000;
                    let offset = bank_idx + (address as usize & 0x7FFF);
                    FetchResult { data: if len > 0 { cart.prg_rom[offset % len] } else { 0 }, driven: true }
                }
            }
        } else if address >= 0x6000 && address < 0x8000 {
            let bank = if self.reg5500 & 0x03 == 0 { 0x3C } else { self.reg5500 & 0x03 };
            let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
            FetchResult { data: self.prg_ram[offset % self.prg_ram.len()], driven: true }
        } else if address >= 0x4000 {
            let addr = address & 0xFFF;
            match addr {
                0x207 => {
                    let mut result = 0;
                    for row in 0..14u16 {
                        if self.keyboard_row & (1 << row) != 0 {
                            result = result >> 1 | 0x80;
                        }
                    }
                    FetchResult { data: result as u8, driven: true }
                }
                0x204 | 0x205 | 0x304 | 0x305 => {
                    FetchResult { data: 0, driven: true }
                }
                0x002 => {
                    FetchResult { data: 0x02, driven: true }
                }
                _ => {
                    FetchResult { data: 0, driven: false }
                }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            if self.reg5501 & 0x80 != 0 {
                self.mmc3.store_prg(cart, address, data);
            } else if self.reg5500 & 0x04 != 0 && self.reg4800 & 0x20 != 0 {
                self.ram_latch = data;
            }
        } else if address >= 0x6000 && address < 0x8000 {
            let bank = if self.reg5500 & 0x03 == 0 { 0x3C } else { self.reg5500 & 0x03 };
            let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
            self.prg_ram[offset % self.prg_ram.len()] = data;
        } else if address >= 0x4000 && address < 0x6000 {
            let addr = address & 0xFFF;
            match addr {
                0x200 | 0x300 => { self.fdc_write(7, data); }
                0x201 | 0x301 => { self.fdc_write(2, data); }
                0x202 | 0x302 | 0x004 => {
                    self.keyboard_row = (self.keyboard_row & 0xFF00) | data as u16;
                }
                0x203 | 0x303 | 0x005 => {
                    self.keyboard_row = (self.keyboard_row & 0x00FF) | ((data as u16) << 8);
                }
                0x205 | 0x305 => { self.fdc_write(5, data); }
                0x800 => { self.reg4800 = data; }
                0x500 => { self.reg5500 = data; }
                0x501 => { self.reg5501 = data; }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if self.reg5501 & 0x80 != 0 {
            self.mmc3.mirror_nametable(cart, address)
        } else {
            if self.reg5500 & 0x08 != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            }
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if self.reg5501 & 0x80 != 0 {
            return self.mmc3.fetch_ppu(
                _prg_rom, chr_rom, _prg_ram, chr_ram, prg_vram,
                true, _nametable_horizontal_mirroring,
                alternative_nametable_arrangement,
                ppu_address_bus, ppu_octal_latch, vram,
            );
        }
        if address < 0x2000 {
            if self.reg4800 & 0x80 != 0 {
                if self.reg5500 & 0x80 != 0 {
                    let bank_reg = (self.reg5501 >> 1 & 0x06) | (self.reg5501 << 3 & 0x18);
                    if address as u8 == 0 {
                        self.pa0809 = address;
                        let page0 = bank_reg;
                        let page4 = (address >> 7 & 0x06) | (bank_reg as u16) | 0x01;
                        let offset0 = (page0 as usize) * 0x1000 + (address as usize & 0x0FFF);
                        let bank4_addr = if address >= 0x1000 {
                            0x1000 + (address as usize & 0x0FFF)
                        } else {
                            address as usize
                        };
                        let offset4 = (page4 as usize) * 0x1000 + (bank4_addr & 0x0FFF);
                        let byte = if address < 0x1000 {
                            self.chr_ram[offset0 % self.chr_ram.len()]
                        } else {
                            self.chr_ram[offset4 % self.chr_ram.len()]
                        };
                        new_addr_bus |= byte as u16;
                    } else {
                        let offset = if address < 0x1000 {
                            (bank_reg as usize) * 0x1000 + (address as usize & 0x0FFF)
                        } else {
                            (bank_reg as usize | 0x01) * 0x1000 + (address as usize & 0x0FFF)
                        };
                        new_addr_bus |= self.chr_ram[offset % self.chr_ram.len()] as u16;
                    }
                } else {
                    let ppu_bank = address >> 13;
                    let was_pa13 = self.pa13;
                    self.pa13 = ppu_bank != 0;
                    if !was_pa13 && self.pa13 {
                        self.pa00 = (address & 0x001) != 0;
                        self.pa09 = (address & 0x200) != 0;
                    }
                    if !self.pa13 {
                        let bank = (address >> 10) as usize & 3 | if self.pa09 { 4 } else { 0 };
                        let addr2 = (address as usize & 0x3FF & !8) | if self.pa00 { 8 } else { 0 };
                        let offset = bank * 0x400 + addr2;
                        new_addr_bus |= self.chr_ram[offset % self.chr_ram.len()] as u16;
                    } else {
                        let offset = address as usize;
                        new_addr_bus |= self.chr_ram[offset % self.chr_ram.len()] as u16;
                    }
                }
            } else {
                let chr_bank = (self.reg5501 & 0x10) | (self.reg5501 << 2 & 0x0C) | (self.reg5501 >> 2 & 0x03);
                let offset = (chr_bank as usize) * 0x2000 + (address as usize & 0x1FFF);
                new_addr_bus |= self.chr_ram[offset % self.chr_ram.len()] as u16;
            }
        } else {
            let mirrored = self.mirror_nametable_raw(address, alternative_nametable_arrangement);
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if self.reg5501 & 0x80 != 0 {
            return self.mmc3.store_ppu(cart, address, data, vram);
        }
        if address < 0x2000 {
            let chr_len = self.chr_ram.len();
            if self.reg4800 & 0x80 != 0 && self.reg5500 & 0x80 != 0 {
                let bank_reg = (self.reg5501 >> 1 & 0x06) | (self.reg5501 << 3 & 0x18);
                self.chr_ram[(bank_reg as usize) * 0x2000 + (address as usize & 0x1FFF) % chr_len] = data;
            } else {
                self.chr_ram[address as usize % chr_len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.reg5501 & 0x80 != 0 {
            self.mmc3.cpu_clock(cycles)
        } else {
            false
        }
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        if self.reg5501 & 0x80 != 0 {
            self.mmc3.cpu_clock_rise(ppu_address_bus)
        } else {
            false
        }
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        if self.reg5501 & 0x80 != 0 {
            self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, sprite_x16, rendering_on)
        } else {
            false
        }
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.mmc3.save_mapper_registers(cart));
        state.push(self.reg4800);
        state.push(self.reg5500);
        state.push(self.reg5501);
        state.push(self.ram_latch);
        state.extend_from_slice(&self.keyboard_row.to_le_bytes());
        state.extend_from_slice(&self.prg_ram);
        state.extend_from_slice(&self.chr_ram);
        state.push(if self.pa00 { 1 } else { 0 });
        state.push(if self.pa09 { 1 } else { 0 });
        state.push(if self.pa13 { 1 } else { 0 });
        state.extend_from_slice(&self.pa0809.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        self.reg4800 = state[p]; p += 1;
        self.reg5500 = state[p]; p += 1;
        self.reg5501 = state[p]; p += 1;
        self.ram_latch = state[p]; p += 1;
        self.keyboard_row = u16::from_le_bytes([state[p], state[p + 1]]); p += 2;
        for b in self.prg_ram.iter_mut() { if p < state.len() { *b = state[p]; p += 1; } }
        for b in self.chr_ram.iter_mut() { if p < state.len() { *b = state[p]; p += 1; } }
        if p < state.len() { self.pa00 = state[p] != 0; p += 1; }
        if p < state.len() { self.pa09 = state[p] != 0; p += 1; }
        if p < state.len() { self.pa13 = state[p] != 0; p += 1; }
        if p + 1 < state.len() { self.pa0809 = u16::from_le_bytes([state[p], state[p + 1]]); p += 2; }
        p
    }
}

impl Mapper169 {
    fn fdc_write(&mut self, _reg: u8, _data: u8) {
    }

    fn mirror_nametable_raw(&self, address: u16, alternative: bool) -> u16 {
        if alternative { return address; }
        if self.reg5500 & 0x08 != 0 {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }
}
