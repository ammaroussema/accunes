use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

const EXRAM_SIZE: usize = 0x200000;
const PRG_RAM_SIZE: usize = 0x8000;

const MIRRORING: [[u8; 4]; 8] = [
    [0, 1, 0, 1], 
    [0, 0, 1, 1], 
    [0, 0, 0, 0], 
    [1, 1, 1, 1], 
    [2, 2, 2, 2],
    [3, 3, 3, 3],
    [0, 1, 2, 3],
    [0, 1, 0, 1], 
];

pub struct Mapper800 {
    reg: [u8; 0x60],
    exram: Vec<u8>,
    prg_ram: Vec<u8>,

    counter: u8,
    enable_irq: u8,
    cycles: i16,
    irq_line: bool,
    deassert_pending: bool,

    dma_mode: u8,
    dma_pending: bool,
}

impl Mapper800 {
    pub fn new() -> Self {
        let mut mapper = Self {
            reg: [0; 0x60],
            exram: vec![0; EXRAM_SIZE],
            prg_ram: vec![0; PRG_RAM_SIZE],
            counter: 0,
            enable_irq: 0,
            cycles: 0,
            irq_line: false,
            deassert_pending: false,
            dma_mode: 0,
            dma_pending: false,
        };
        mapper.set_default_regs();
        mapper
    }

    fn set_default_regs(&mut self) {
        self.reg = [0; 0x60];
        self.reg[0x04] = 0xFE;
        self.reg[0x05] = 0x7F;
        self.reg[0x06] = 0xFF;
        self.reg[0x07] = 0x7F;
        self.reg[0x10] = 0x00;
        self.reg[0x12] = 0x01;
        self.reg[0x14] = 0x02;
        self.reg[0x16] = 0x03;
        self.reg[0x18] = 0x04;
        self.reg[0x1A] = 0x05;
        self.reg[0x1C] = 0x06;
        self.reg[0x1E] = 0x07;
        self.reg[0x52] = 0xFF;
        self.reg[0x53] = 0xFF;
        self.reg[0x56] = 0xFF;
        self.reg[0x57] = 0xFF;
        self.counter = 0;
        self.enable_irq = 0;
        self.cycles = 0;
        self.irq_line = false;
        self.deassert_pending = false;
        self.dma_pending = false;
    }


    fn prg_bank_value(&self, base_reg: usize) -> usize {
        (self.reg[base_reg] as usize) | ((self.reg[base_reg | 1] as usize) << 8)
    }

    fn prg_bank_index(&self, base_reg: usize) -> usize {
        let and = (self.reg[0x53] as usize) << 8 | self.reg[0x52] as usize;
        let or = (self.reg[0x51] as usize) << 8 | self.reg[0x50] as usize;
        (self.prg_bank_value(base_reg) & and) + or
    }

    fn prg_is_ram(&self, base_reg: usize) -> bool {
        self.prg_bank_value(base_reg) & 0x8000 != 0
    }


    fn chr_bank_value(&self, bank: usize) -> usize {
        let r = 0x10 + bank * 2;
        (self.reg[r] as usize) | ((self.reg[r | 1] as usize) << 8)
    }

    fn chr_bank_index(&self, bank: usize) -> usize {
        let and = (self.reg[0x57] as usize) << 8 | self.reg[0x56] as usize;
        let or = (self.reg[0x55] as usize) << 8 | self.reg[0x54] as usize;
        (self.chr_bank_value(bank) & and) + or
    }

    fn chr_is_ram(&self, bank: usize) -> bool {
        self.chr_bank_value(bank) & 0x8000 != 0
    }


    fn ex4080(&self) -> usize {
        ((self.reg[0x38] as usize) << 7) | 0x1F0000
    }
    fn ex4100(&self) -> usize {
        ((self.reg[0x39] as usize) << 8) | 0x1E0000
    }
    fn ex4200(&self) -> usize {
        ((self.reg[0x3A] as usize) << 9) | 0x1C0000
    }
    fn ex4400(&self) -> usize {
        ((self.reg[0x3B] as usize) << 10) | 0x180000
    }
    fn ex4800(&self) -> usize {
        ((self.reg[0x3C] as usize) << 11) | 0x100000
    }
    fn ex5000(&self) -> usize {
        (self.reg[0x3D] as usize) << 12
    }

    fn nt_source(&self, addr: usize, vram: &[u8]) -> u8 {
        let quad = (addr >> 10) & 3;
        let off = addr & 0x3FF;
        let m = MIRRORING[(self.reg[0x2E] & 7) as usize][quad] as usize;
        match self.reg[0x2F] & 3 {
            0 => vram[((m & 1) * 0x400 + off) & (vram.len() - 1)],
            1 => self.exram[self.ex4800() + (m & 1) * 0x400 + off],
            3 => self.exram[self.ex5000() + (m & 3) * 0x400 + off],
            _ => self.exram[self.ex5000() + (m & 1) * 0x400 + off],
        }
    }

    fn nt_store(&mut self, addr: usize, data: u8, vram: &mut [u8]) {
        let quad = (addr >> 10) & 3;
        let off = addr & 0x3FF;
        let m = MIRRORING[(self.reg[0x2E] & 7) as usize][quad] as usize;
        match self.reg[0x2F] & 3 {
            0 => vram[((m & 1) * 0x400 + off) & (vram.len() - 1)] = data,
            1 => {
                let base = self.ex4800() + (m & 1) * 0x400 + off;
                self.exram[base] = data;
            }
            3 => {
                let base = self.ex5000() + (m & 3) * 0x400 + off;
                self.exram[base] = data;
            }
            _ => {
                let base = self.ex5000() + (m & 1) * 0x400 + off;
                self.exram[base] = data;
            }
        }
    }


    fn set_irq(&mut self, deassert: bool) {
        self.irq_line = !deassert;
    }


    fn read_chr(&self, bank: usize, offset: usize, chr_rom: &[u8], chr_ram: &[u8]) -> u8 {
        let idx = self.chr_bank_index(bank);
        if self.chr_is_ram(bank) {
            if !chr_ram.is_empty() {
                chr_ram[(idx * 0x400 + offset) % chr_ram.len()]
            } else {
                0
            }
        } else if !chr_rom.is_empty() {
            chr_rom[(idx * 0x400 + offset) % chr_rom.len()]
        } else {
            0
        }
    }

    fn write_chr(&mut self, bank: usize, offset: usize, data: u8, chr_ram: &mut [u8]) {
        if !self.chr_is_ram(bank) || chr_ram.is_empty() {
            return;
        }
        let idx = self.chr_bank_index(bank);
        let len = chr_ram.len();
        chr_ram[(idx * 0x400 + offset) % len] = data;
    }

    fn write_reg(&mut self, r: usize, val: u8) {
        self.reg[r] = val;
        self.deassert_pending = false;
        match r {
            0x0A | 0x0B => {
                self.apply_coarse_reg(1, 0x0A, 0x00);
            }
            0x0D => {
                self.counter = val;
            }
            0x0E | 0x0F => {
                self.enable_irq = (r & 1) as u8;
                self.set_irq(true);
                self.deassert_pending = true;
            }
            0x20 | 0x21 => self.apply_coarse_reg(1, 0x20, 0x10),
            0x22 | 0x23 => self.apply_coarse_reg(1, 0x22, 0x14),
            0x24 | 0x25 => self.apply_coarse_reg(1, 0x24, 0x18), 
            0x26 | 0x27 => self.apply_coarse_reg(1, 0x26, 0x1C), 
            0x28 | 0x29 => self.apply_coarse_reg(2, 0x28, 0x10),
            0x2A | 0x2B => self.apply_coarse_reg(2, 0x2A, 0x18),
            0x2C | 0x2D => self.apply_coarse_reg(3, 0x2C, 0x10),
            0x36 => {
                self.dma_mode = val;
                self.dma_pending = true;
            }
            0x40..=0x43 => {
                let a = (self.reg[0x41] as u32) << 8 | self.reg[0x40] as u32;
                let b = (self.reg[0x43] as u32) << 8 | self.reg[0x42] as u32;
                let product = a.wrapping_mul(b);
                self.reg[0x44] = (product & 0xFF) as u8;
                self.reg[0x45] = ((product >> 8) & 0xFF) as u8;
                self.reg[0x46] = ((product >> 16) & 0xFF) as u8;
                self.reg[0x47] = ((product >> 24) & 0xFF) as u8;
            }
            0x48 | 0x49 => {
                let number = (self.reg[0x48] as u32) | ((self.reg[0x49] as u32) << 8);
                self.reg[0x4A] = (number % 10) as u8;
                self.reg[0x4B] = ((number / 10) % 10) as u8;
                self.reg[0x4C] = ((number / 100) % 10) as u8;
                self.reg[0x4D] = ((number / 1000) % 10) as u8;
                self.reg[0x4E] = ((number / 10000) % 10) as u8;
            }
            _ => {}
        }
    }

    fn apply_coarse_reg(&mut self, shift: usize, coarse_reg: usize, mut fine_reg: usize) {
        let banks = 1usize << shift;
        for bank in 0..banks {
            let lo = ((self.reg[coarse_reg] as u16) << shift | bank as u16) as u8;
            let hi = (self.reg[coarse_reg | 1] & 0x80)
                | ((((self.reg[coarse_reg | 1] as u16) << shift) & 0x7F) as u8)
                | (self.reg[coarse_reg] >> (8 - shift));
            self.reg[fine_reg] = lo;
            self.reg[fine_reg | 1] = hi;
            fine_reg += 2;
        }
    }

    fn read_cpu_bus(&self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x5000 && address < 0x6000 {
            let idx = self.ex5000() + (address as usize & 0xFFF);
            return FetchResult { data: self.exram[idx], driven: true };
        }
        if address >= 0x4000 && address < 0x5000 {
            let al = (address & 0xFFF) as usize;
            if (0x20..=0x7F).contains(&al) {
                return FetchResult { data: self.reg[al - 0x20], driven: true };
            }
            if al & 0x800 != 0 {
                return FetchResult { data: self.exram[self.ex4800() + (al & 0x7FF)], driven: true };
            }
            if al & 0x400 != 0 {
                return FetchResult { data: self.exram[self.ex4400() + (al & 0x3FF)], driven: true };
            }
            if al & 0x200 != 0 {
                return FetchResult { data: self.exram[self.ex4200() + (al & 0x1FF)], driven: true };
            }
            if al & 0x100 != 0 {
                return FetchResult { data: self.exram[self.ex4100() + (al & 0xFF)], driven: true };
            }
            if al & 0x80 != 0 {
                return FetchResult { data: self.exram[self.ex4080() + (al & 0x7F)], driven: true };
            }
            return FetchResult { data: 0, driven: false };
        }
        if address >= 0x6000 && address < 0x8000 {
            let data = if self.prg_is_ram(0x08) {
                let bank = self.prg_bank_index(0x08);
                let len = self.prg_ram.len();
                self.prg_ram[(bank * 0x2000 + address as usize - 0x6000) % len]
            } else {
                let bank = self.prg_bank_index(0x08);
                if !cart.prg_rom.is_empty() {
                    cart.prg_rom[(bank * 0x2000 + address as usize - 0x6000) % cart.prg_rom.len()]
                } else {
                    0
                }
            };
            return FetchResult { data, driven: true };
        }
        if address >= 0x8000 {
            let (base_reg, offset) = match address {
                0x8000..=0x9FFF => (0x00, address as usize - 0x8000),
                0xA000..=0xBFFF => (0x02, address as usize - 0xA000),
                0xC000..=0xDFFF => (0x04, address as usize - 0xC000),
                _ => (0x06, address as usize - 0xE000),
            };
            let data = if self.prg_is_ram(base_reg) {
                let bank = self.prg_bank_index(base_reg);
                let len = self.prg_ram.len();
                self.prg_ram[(bank * 0x2000 + offset) % len]
            } else {
                let bank = self.prg_bank_index(base_reg);
                if !cart.prg_rom.is_empty() {
                    cart.prg_rom[(bank * 0x2000 + offset) % cart.prg_rom.len()]
                } else {
                    0
                }
            };
            return FetchResult { data, driven: true };
        }
        FetchResult { data: 0, driven: false }
    }

    fn write_cpu_bus(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x6000 {
            let idx = self.ex5000() + (address as usize & 0xFFF);
            self.exram[idx] = data;
            return;
        }
        if address >= 0x4000 && address < 0x5000 {
            let al = (address & 0xFFF) as usize;
            if (0x20..=0x7F).contains(&al) {
                self.write_reg(al - 0x20, data);
                return;
            }
            if al & 0x800 != 0 {
                let idx = self.ex4800() + (al & 0x7FF);
                self.exram[idx] = data;
                return;
            }
            if al & 0x400 != 0 {
                let idx = self.ex4400() + (al & 0x3FF);
                self.exram[idx] = data;
                return;
            }
            if al & 0x200 != 0 {
                let idx = self.ex4200() + (al & 0x1FF);
                self.exram[idx] = data;
                return;
            }
            if al & 0x100 != 0 {
                let idx = self.ex4100() + (al & 0xFF);
                self.exram[idx] = data;
                return;
            }
            if al & 0x80 != 0 {
                let idx = self.ex4080() + (al & 0x7F);
                self.exram[idx] = data;
                return;
            }
            return;
        }
        if address >= 0x6000 && address < 0x8000 {
            if self.prg_is_ram(0x08) {
                let bank = self.prg_bank_index(0x08);
                let len = self.prg_ram.len();
                self.prg_ram[(bank * 0x2000 + address as usize - 0x6000) % len] = data;
            }
            return;
        }
        if address >= 0x8000 {
            let (base_reg, offset) = match address {
                0x8000..=0x9FFF => (0x00, address as usize - 0x8000),
                0xA000..=0xBFFF => (0x02, address as usize - 0xA000),
                0xC000..=0xDFFF => (0x04, address as usize - 0xC000),
                _ => (0x06, address as usize - 0xE000),
            };
            if self.prg_is_ram(base_reg) {
                let bank = self.prg_bank_index(base_reg);
                let len = self.prg_ram.len();
                self.prg_ram[(bank * 0x2000 + offset) % len] = data;
            }
        }
    }

    fn read_ppu_slices(&self, chr_rom: &[u8], chr_ram: &[u8], address: u16, vram: &[u8]) -> u8 {
        let addr = address & 0x3FFF;
        if addr < 0x2000 {
            let bank = ((addr >> 10) as usize) & 7;
            self.read_chr(bank, (addr as usize) & 0x3FF, chr_rom, chr_ram)
        } else if addr < 0x3F00 {
            self.nt_source(addr as usize, vram)
        } else {
            0
        }
    }

    fn read_ppu_bus(&self, cart: &Cartridge, address: u16, vram: &[u8]) -> u8 {
        self.read_ppu_slices(&cart.chr_rom, &cart.chr_ram, address, vram)
    }

    fn write_ppu_bus(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        let addr = address & 0x3FFF;
        if addr < 0x2000 {
            let bank = ((addr >> 10) as usize) & 7;
            self.write_chr(bank, (addr as usize) & 0x3FF, data, &mut cart.chr_ram);
        } else if addr < 0x3F00 {
            self.nt_store(addr as usize, data, vram);
        }
    }


    fn dma_read_cpu(&self, cart: &Cartridge, ram: &[u8], addr: u16) -> u8 {
        if addr < 0x2000 {
            ram[addr as usize & (ram.len() - 1)]
        } else if addr >= 0x4020 {
            self.read_cpu_bus(cart, addr).data
        } else {
            0
        }
    }

    fn dma_write_cpu(&mut self, cart: &mut Cartridge, ram: &mut [u8], addr: u16, data: u8) {
        if addr < 0x2000 {
            ram[addr as usize & (ram.len() - 1)] = data;
        } else if addr >= 0x4020 {
            self.write_cpu_bus(cart, addr, data);
        }
    }

    fn dma_read_ppu(&self, cart: &Cartridge, vram: &[u8], addr: u16) -> u8 {
        self.read_ppu_bus(cart, addr, vram)
    }

    fn dma_write_ppu(&mut self, cart: &mut Cartridge, vram: &mut [u8], addr: u16, data: u8) {
        self.write_ppu_bus(cart, addr, data, vram);
    }

    fn execute_dma_inner(&mut self, cart: &mut Cartridge, ram: &mut [u8], vram: &mut [u8]) -> bool {
        if !self.dma_pending {
            return false;
        }
        self.dma_pending = false;
        let mode = self.dma_mode;
        if mode > 5 {
            return true;
        }
        let mut source = (self.reg[0x30] as usize) | ((self.reg[0x31] as usize) << 8);
        let mut target = (self.reg[0x32] as usize) | ((self.reg[0x33] as usize) << 8);
        let mut length = (self.reg[0x34] as usize) | ((self.reg[0x35] as usize) << 8);
        let mut byte = (source & 0xFF) as u8;
        while length != 0 {
            if mode & 2 != 0 {
                byte = self.dma_read_ppu(cart, vram, source as u16);
            } else if mode < 2 {
                byte = self.dma_read_cpu(cart, ram, source as u16);
            }
            if mode & 1 != 0 {
                self.dma_write_ppu(cart, vram, target as u16, byte);
            } else {
                self.dma_write_cpu(cart, ram, target as u16, byte);
            }
            source = source.wrapping_add(1);
            target = target.wrapping_add(1);
            length -= 1;
        }
        true
    }
}

impl Mapper for Mapper800 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        self.read_cpu_bus(cart, address)
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        self.write_cpu_bus(cart, address, data);
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let quad = ((address >> 10) & 3) as usize;
        let off = (address & 0x3FF) as usize;
        let m = MIRRORING[(self.reg[0x2E] & 7) as usize][quad] as usize;
        ((m & 1) * 0x400 + off) as u16
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
        let addr = (ppu_address_bus & 0x3F00) | (ppu_octal_latch as u16);
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        let data = self.read_ppu_slices(chr_rom, chr_ram, addr, vram);
        new_addr_bus |= data as u16;
        (data, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        self.write_ppu_bus(cart, address, data, vram);
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.enable_irq & 1 != 0 && self.reg[0x0C] & 0x02 != 0 {
            let bit0 = self.reg[0x0C] & 1;
            let fire = if bit0 != 0 {
                self.cycles = self.cycles.wrapping_sub(3);
                self.cycles <= 0
            } else {
                true
            };
            if fire {
                if bit0 != 0 {
                    self.cycles = self.cycles.wrapping_add(341);
                }
                self.counter = self.counter.wrapping_sub(1);
                if self.counter == 0 {
                    self.counter = self.reg[0x0D];
                    self.set_irq(false);
                }
            }
        }
        self.irq_line
    }

    fn cpu_clock_irq_level(&self) -> bool {
        self.irq_line
    }

    fn ppu_clock(
        &mut self,
        _ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        _ppu_sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        if self.enable_irq & 1 != 0 && self.reg[0x0C] & 0x02 == 0 {
            if self.reg[0x0C] & 1 != 0 {
                if scanline >= 240 || !rendering_on {
                    self.enable_irq = 0;
                    self.reg[0x0D] = 0;
                    self.set_irq(true);
                } else if dot == 256 && scanline == self.reg[0x0D] as u16 {
                    self.set_irq(false);
                }
            } else if dot == 260 && rendering_on && scanline < 240 {
                self.counter = if self.counter == 0 {
                    self.reg[0x0D]
                } else {
                    self.counter.wrapping_sub(1)
                };
                if self.counter == 0 {
                    self.set_irq(false);
                }
            }
        }
        self.irq_line
    }

    fn take_irq_ack(&mut self) -> bool {
        let pending = self.deassert_pending;
        self.deassert_pending = false;
        pending
    }

    fn execute_dma(&mut self, cart: &mut Cartridge, ram: &mut [u8], vram: &mut [u8]) -> bool {
        self.execute_dma_inner(cart, ram, vram)
    }

    fn reset(&mut self) {
        self.set_default_regs();
    }
    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut s = Vec::new();
        s.extend_from_slice(&self.reg);
        s.push(self.counter);
        s.push(self.enable_irq);
        s.extend_from_slice(&self.cycles.to_le_bytes());
        s.push(if self.irq_line { 1 } else { 0 });
        s.push(if self.deassert_pending { 1 } else { 0 });
        s.push(self.dma_mode);
        s.push(if self.dma_pending { 1 } else { 0 });
        s
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut offset = start;
        if offset + 0x60 > state.len() {
            return offset;
        }
        self.reg.copy_from_slice(&state[offset..offset + 0x60]);
        offset += 0x60;
        if offset + 6 > state.len() {
            return offset;
        }
        self.counter = state[offset];
        offset += 1;
        self.enable_irq = state[offset];
        offset += 1;
        self.cycles = i16::from_le_bytes([state[offset], state[offset + 1]]);
        offset += 2;
        self.irq_line = state[offset] != 0;
        offset += 1;
        self.deassert_pending = state[offset] != 0;
        offset += 1;
        self.dma_mode = state[offset];
        offset += 1;
        self.dma_pending = state[offset] != 0;
        offset += 1;
        offset
    }

    fn battery_save_data(&self, _cart: &Cartridge) -> Option<Vec<u8>> {
        let mut save = Vec::new();
        save.extend_from_slice(&(self.exram.len() as u32).to_le_bytes());
        save.extend_from_slice(&self.exram);
        save.extend_from_slice(&(self.prg_ram.len() as u32).to_le_bytes());
        save.extend_from_slice(&self.prg_ram);
        Some(save)
    }

    fn load_battery_save(&mut self, _cart: &mut Cartridge, data: &[u8]) {
        let mut offset = 0;
        if offset + 4 > data.len() {
            return;
        }
        let exram_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        offset += 4;
        if exram_len == self.exram.len() && offset + exram_len <= data.len() {
            self.exram.copy_from_slice(&data[offset..offset + exram_len]);
        }
        offset = offset.saturating_add(exram_len);
        if offset + 4 > data.len() {
            return;
        }
        let prg_ram_len =
            u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
                as usize;
        offset += 4;
        if prg_ram_len == self.prg_ram.len() && offset + prg_ram_len <= data.len() {
            self.prg_ram.copy_from_slice(&data[offset..offset + prg_ram_len]);
        }
    }
}
