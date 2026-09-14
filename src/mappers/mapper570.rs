use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper570 {
    reg: u8,
    prg: [u8; 2],
    chr: [u16; 8],
    mirroring: u8,
    prg_flip: u8,
    wram_enable: bool,
    irq: u8,
    counter: u8,
    latch: u8,
    cycles: i16,
    irq_raise_count: u8,
    irq_active: bool,
}

impl Mapper570 {
    pub fn new() -> Self {
        Self {
            reg: 0,
            prg: [0, 1],
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            mirroring: 0,
            prg_flip: 0,
            wram_enable: true,
            irq: 0,
            counter: 0,
            latch: 0,
            cycles: 0,
            irq_raise_count: 0,
            irq_active: false,
        }
    }

    fn prg_window(&self, address: u16) -> (u16, u16) {
        let raw = if self.prg_flip != 0 {
            match address {
                0x8000..=0x9FFF => 0x0E,
                0xA000..=0xBFFF => self.prg[1],
                0xC000..=0xDFFF => self.prg[0],
                _ => 0x0F,
            }
        } else {
            match address {
                0x8000..=0x9FFF => self.prg[0],
                0xA000..=0xBFFF => self.prg[1],
                0xC000..=0xDFFF => 0x0E,
                _ => 0x0F,
            }
        };
        let page = ((raw & 0x0F) as u16) | ((self.reg as u16) << 4);
        (page, (address as u16) & 0x1FFF)
    }

    fn chr_and_or(&self) -> (u16, u16) {
        let and = if self.reg & 3 != 0 { 0x0FF } else { 0x1FF };
        let or = if self.reg & 3 != 0 { 0x200 } else { 0 } | if self.reg & 2 != 0 { 0x100 } else { 0 };
        (and, or)
    }
}

impl Mapper for Mapper570 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            if self.wram_enable || cart.prg_rom_crc32 == 0xC24B972C {
                let len = cart.prg_ram.len();
                if len > 0 {
                    return FetchResult {
                        data: cart.prg_ram[(address as usize & 0x1FFF) % len],
                        driven: true,
                    };
                }
            }
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        if address >= 0x8000 {
            let (page, offset_in) = self.prg_window(address);
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            let offset = (page as usize * 0x2000 + offset_in as usize) % len;
            return FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x6000 {
            self.reg = (address & 0xFF) as u8;
            return;
        }
        if address >= 0x6000 && address < 0x8000 {
            if self.wram_enable || cart.prg_rom_crc32 == 0xC24B972C {
                let len = cart.prg_ram.len();
                if len > 0 {
                    cart.prg_ram[(address as usize & 0x1FFF) % len] = data;
                }
            }
            return;
        }
        match address {
            0x8000..=0x8FFF => {
                self.prg[0] = data;
            }
            0x9000..=0x9FFF => match address & 3 {
                0 | 1 => {
                    self.mirroring = data & 3;
                }
                2 => {
                    self.wram_enable = data & 1 != 0;
                    self.prg_flip = if data & 2 != 0 { 4 } else { 0 };
                }
                _ => {}
            },
            0xA000..=0xAFFF => {
                self.prg[1] = data;
            }
            0xB000..=0xEFFF => {
                let bank = (address >> 12) as usize;
                let reg = ((bank - 0xB) << 1) | if address & 2 != 0 { 1 } else { 0 };
                if address & 1 != 0 {
                    self.chr[reg] = (self.chr[reg] & 0x00F) | ((data as u16) << 4);
                } else {
                    self.chr[reg] = (self.chr[reg] & 0xFF0) | ((data & 0x0F) as u16);
                }
            }
            0xF000..=0xFFFF => match address & 3 {
                0 => {
                    self.latch = (self.latch & 0xF0) | (data & 0x0F);
                }
                1 => {
                    self.latch = (self.latch & 0x0F) | (data << 4);
                }
                2 => {
                    self.irq = data;
                    if self.irq & 2 != 0 {
                        self.counter = self.latch;
                        self.cycles = 341;
                    }
                    self.irq_active = false;
                }
                _ => {
                    self.irq = (self.irq & !2) | (self.irq << 1 & 2);
                    self.irq_active = false;
                }
            },
            _ => {}
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        match self.mirroring & 3 {
            0 => address & 0x37FF,
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            2 => address & 0x27FF,
            _ => (address & 0x27FF) | 0x0400,
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
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
        let byte;
        if address < 0x2000 {
            if !chr_ram.is_empty() {
                byte = chr_ram[(address & 0x1FFF) as usize % chr_ram.len()];
            } else if !chr_rom.is_empty() {
                let (chr_and, chr_or) = self.chr_and_or();
                let bank = (address >> 10) as usize & 7;
                let page = (self.chr[bank] & chr_and) | chr_or;
                let offset = page as usize * 0x400 + (address as usize & 0x3FF);
                byte = chr_rom[offset % chr_rom.len()];
            } else {
                byte = 0;
            }
        } else if address < 0x3F00 {
            let mirrored = match self.mirroring & 3 {
                0 => address & 0x37FF,
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                2 => address & 0x27FF,
                _ => (address & 0x27FF) | 0x0400,
            };
            byte = vram[(mirrored & 0x7FF) as usize];
        } else {
            return (ppu_address_bus as u8, new_addr_bus);
        }
        new_addr_bus |= byte as u16;
        (new_addr_bus as u8, new_addr_bus)
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.irq_raise_count != 0 {
            self.irq_raise_count -= 1;
            if self.irq_raise_count == 0 {
                self.irq_active = true;
            }
        }
        if self.irq & 2 != 0 && (self.irq & 4 != 0 || {
            self.cycles -= 3;
            self.cycles <= 0
        }) {
            if self.irq & 4 == 0 {
                self.cycles += 341;
            }
            self.counter = self.counter.wrapping_add(1);
            if self.counter == 0 {
                self.counter = self.latch;
                self.irq_raise_count = 0;
                self.irq_active = true;
            }
        }
        self.irq_active
    }

    fn cpu_clock_irq_level(&self) -> bool {
        true
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut s = Vec::new();
        s.push(self.reg);
        s.extend_from_slice(&self.prg);
        for c in &self.chr {
            s.extend_from_slice(&c.to_le_bytes());
        }
        s.push(self.mirroring);
        s.push(self.prg_flip);
        s.push(self.wram_enable as u8);
        s.push(self.irq);
        s.push(self.counter);
        s.push(self.latch);
        s.extend_from_slice(&self.cycles.to_le_bytes());
        s.push(self.irq_raise_count);
        s
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.reg = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.prg.copy_from_slice(&state[p..p + 2]);
            p += 2;
        }
        if p + 16 <= state.len() {
            for i in 0..8 {
                self.chr[i] = u16::from_le_bytes([state[p + i * 2], state[p + i * 2 + 1]]);
            }
            p += 16;
        }
        if p < state.len() {
            self.mirroring = state[p];
            p += 1;
        }
        if p < state.len() {
            self.prg_flip = state[p];
            p += 1;
        }
        if p < state.len() {
            self.wram_enable = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.irq = state[p];
            p += 1;
        }
        if p < state.len() {
            self.counter = state[p];
            p += 1;
        }
        if p < state.len() {
            self.latch = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.cycles = i16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.irq_raise_count = state[p];
            p += 1;
        }
        p - start
    }

    fn reset(&mut self) {
        self.reg = 0;
        self.irq = 0;
        self.counter = 0;
        self.latch = 0;
        self.cycles = 0;
        self.prg_flip = 0;
        self.prg = [0, 1];
        self.chr = [0, 1, 2, 3, 4, 5, 6, 7];
        self.wram_enable = true;
        self.mirroring = 0;
        self.irq_raise_count = 0;
        self.irq_active = false;
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }
}