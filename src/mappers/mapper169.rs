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
    prg_ram: Vec<u8>,
    chr_ram: Vec<u8>,
    pa00: bool,
    pa09: bool,
    pa13: bool,
    pa0809: u16,
    split_bank0: u8,
    split_bank1: u8,
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
            prg_ram: vec![0u8; 0x80000],
            chr_ram: vec![0u8; 0x40000],
            pa00: false,
            pa09: false,
            pa13: false,
            pa0809: 0,
            split_bank0: 0,
            split_bank1: 1,
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
        self.split_bank0 = 0;
        self.split_bank1 = 1;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if self.reg5501 & 0x80 != 0 {
                let bank = match address {
                    0x8000..=0x9FFF => {
                        if (self.mmc3.r8000 & 0x40) != 0 {
                            0x3E
                        } else {
                            self.mmc3.bank_8c & 0x3F
                        }
                    }
                    0xA000..=0xBFFF => self.mmc3.bank_a & 0x3F,
                    0xC000..=0xDFFF => {
                        if (self.mmc3.r8000 & 0x40) != 0 {
                            self.mmc3.bank_8c & 0x3F
                        } else {
                            0x3E
                        }
                    }
                    _ => 0x3F,
                };
                let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
                FetchResult {
                    data: self.prg_ram[offset % self.prg_ram.len()],
                    driven: true,
                }
            } else if self.reg5500 & 0x04 != 0 {
                let offset = if self.reg5500 & 0x40 != 0 {
                    (self.ram_latch as usize >> 1) * 0x8000 + (address as usize & 0x7FFF)
                } else {
                    let bank = if address < 0xC000 {
                        self.ram_latch as usize
                    } else {
                        0xFF
                    };
                    bank * 0x4000 + (address as usize & 0x3FFF)
                };
                FetchResult {
                    data: self.prg_ram[offset % self.prg_ram.len()],
                    driven: true,
                }
            } else {
                let len = cart.prg_rom.len();
                if len > 0 {
                    let bank_idx = (self.reg4800 as usize & 0x1F) * 0x8000;
                    let offset = bank_idx + (address as usize & 0x7FFF);
                    FetchResult {
                        data: cart.prg_rom[offset % len],
                        driven: true,
                    }
                } else {
                    FetchResult { data: 0, driven: true }
                }
            }
        } else if address >= 0x6000 && address < 0x8000 {
            let bank = if self.reg5500 & 0x03 == 0 { 0x3C } else { self.reg5500 & 0x03 };
            let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: self.prg_ram[offset % self.prg_ram.len()],
                driven: true,
            }
        } else if address >= 0x4000 {
            match address {
                0x4207 => {
                    let mut result = 0;
                    for row in 0..14u16 {
                        if self.keyboard_row & (1 << row) != 0 {
                            result = result >> 1 | 0x80;
                        }
                    }
                    FetchResult { data: result as u8, driven: true }
                }
                0x4204 | 0x4205 | 0x4304 | 0x4305 => {
                    FetchResult { data: 0, driven: true }
                }
                0x5002 => {
                    FetchResult { data: 0x02, driven: true }
                }
                _ => FetchResult { data: 0, driven: false },
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            if self.reg5501 & 0x80 != 0 {
                self.mmc3.store_prg(cart, address, data);
            } else if self.reg4800 & 0x20 != 0 {
                self.ram_latch = data;
            } else if self.reg5500 & 0x04 != 0 {
                let offset = if self.reg5500 & 0x40 != 0 {
                    (self.ram_latch as usize >> 1) * 0x8000 + (address as usize & 0x7FFF)
                } else {
                    let bank = if address < 0xC000 {
                        self.ram_latch as usize
                    } else {
                        0xFF
                    };
                    bank * 0x4000 + (address as usize & 0x3FFF)
                };
                let ram_len = self.prg_ram.len();
                self.prg_ram[offset % ram_len] = data;
            }
        } else if address >= 0x6000 && address < 0x8000 {
            let bank = if self.reg5500 & 0x03 == 0 { 0x3C } else { self.reg5500 & 0x03 };
            let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
            let ram_len = self.prg_ram.len();
            self.prg_ram[offset % ram_len] = data;
        } else if address >= 0x4000 && address < 0x6000 {
            match address {
                0x4200 | 0x4300 => { self.fdc_write(7, data); }
                0x4201 | 0x4301 => { self.fdc_write(2, data); }
                0x4202 | 0x4302 | 0x5004 => {
                    self.keyboard_row = (self.keyboard_row & 0xFF00) | data as u16;
                }
                0x4203 | 0x4303 | 0x5005 => {
                    self.keyboard_row = (self.keyboard_row & 0x00FF) | ((data as u16) << 8);
                }
                0x4205 | 0x4305 => { self.fdc_write(5, data); }
                0x4800 => { self.reg4800 = data; }
                0x5500 => { self.reg5500 = data; }
                0x5501 => { self.reg5501 = data; }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if self.reg5501 & 0x80 != 0 {
            self.mmc3.mirror_nametable(cart, address)
        } else if self.reg5500 & 0x08 != 0 {
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
        _chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;

        let pa13_new = address >= 0x2000;
        if !self.pa13 && pa13_new {
            self.pa00 = (address & 0x001) != 0;
            self.pa09 = (address & 0x0200) != 0;
        }
        self.pa13 = pa13_new;

        if address < 0x2000 {
            if self.reg5501 & 0x80 != 0 {
                let bank = self.mmc3.chr_bank(address);
                let offset = (bank as usize) * 0x400 + (address as usize & 0x3FF);
                new_addr_bus |= self.chr_ram[offset % self.chr_ram.len()] as u16;
            } else if self.reg4800 & 0x80 != 0 {
                if self.reg5500 & 0x80 != 0 {
                    let offset = if address < 0x1000 {
                        (self.split_bank0 as usize) * 0x1000 + (address as usize & 0x0FFF)
                    } else {
                        (self.split_bank1 as usize) * 0x1000 + (address as usize & 0x0FFF)
                    };
                    new_addr_bus |= self.chr_ram[offset % self.chr_ram.len()] as u16;
                } else {
                    let base_8k = (self.reg5501 & 0x10) | ((self.reg5501 << 2) & 0x0C) | ((self.reg5501 >> 2) & 0x03);
                    let bank = ((address >> 10) as usize & 3) | if self.pa09 { 4 } else { 0 };
                    let addr2 = (address as usize & 0x3FF & !8) | if self.pa00 { 8 } else { 0 };
                    let offset = (base_8k as usize) * 0x2000 + bank * 0x400 + addr2;
                    new_addr_bus |= self.chr_ram[offset % self.chr_ram.len()] as u16;
                }
            } else {
                let chr_bank = (self.reg5501 & 0x10) | ((self.reg5501 << 2) & 0x0C) | ((self.reg5501 >> 2) & 0x03);
                let offset = (chr_bank as usize) * 0x2000 + (address as usize & 0x1FFF);
                new_addr_bus |= self.chr_ram[offset % self.chr_ram.len()] as u16;
            }
        } else {
            if self.reg5501 & 0x80 == 0 && self.reg4800 & 0x80 != 0 && self.reg5500 & 0x80 != 0 {
                let addr = address & 0x3FF;
                if (addr & 0xFF) == 0 {
                    self.pa0809 = addr;
                    self.split_bank0 = ((self.reg5501 >> 1) & 0x06) | ((self.reg5501 << 3) & 0x18);
                    self.split_bank1 = (((addr >> 7) & 0x06) as u8) | ((self.reg5501 << 3) & 0x18) | 0x01;
                }
            }
            let mirrored = if self.reg5501 & 0x80 != 0 {
                if self.mmc3.nametable_mirroring() {
                    (address & 0x33FF) | ((address & 0x0800) >> 1)
                } else {
                    address & 0x37FF
                }
            } else if self.reg5500 & 0x08 != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        let pa13_new = address >= 0x2000;
        if !self.pa13 && pa13_new {
            self.pa00 = (address & 0x001) != 0;
            self.pa09 = (address & 0x0200) != 0;
        }
        self.pa13 = pa13_new;

        if address < 0x2000 {
            let chr_len = self.chr_ram.len();
            if self.reg5501 & 0x80 != 0 {
                let bank = self.mmc3.chr_bank(address);
                let offset = (bank as usize) * 0x400 + (address as usize & 0x3FF);
                self.chr_ram[offset % chr_len] = data;
            } else {
                let chr_bank = (self.reg5501 & 0x10) | ((self.reg5501 << 2) & 0x0C) | ((self.reg5501 >> 2) & 0x03);
                let offset = (chr_bank as usize) * 0x2000 + (address as usize & 0x1FFF);
                self.chr_ram[offset % chr_len] = data;
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
        state.push(self.split_bank0);
        state.push(self.split_bank1);
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
        if p < state.len() { self.split_bank0 = state[p]; p += 1; }
        if p < state.len() { self.split_bank1 = state[p]; p += 1; }
        p
    }
}

impl Mapper169 {
    fn fdc_write(&mut self, _reg: u8, _data: u8) {
    }
}
