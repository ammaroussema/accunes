use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, mmc3_chr_bank};

fn rev(val: u8) -> u8 {
    (val << 6 & 0x40) | (val << 4 & 0x20) | (val << 2 & 0x10) | (val & 0x08) | (val >> 2 & 0x04) | (val >> 4 & 0x02) | (val >> 6 & 0x01)
}

fn prg_rom_read(cart: &Cartridge, offset: usize) -> u8 {
    let len = cart.prg_rom.len();
    if len == 0 { 0 } else { cart.prg_rom[offset % len] }
}

fn chr_read(chr_rom: &[u8], chr_ram: &[u8], offset: usize) -> u8 {
    if !chr_rom.is_empty() {
        chr_rom[offset % chr_rom.len()]
    } else if !chr_ram.is_empty() {
        chr_ram[offset % chr_ram.len()]
    } else { 0 }
}

pub struct Mapper394 {
    mmc3: MapperMMC3,
    reg: [u8; 4],
    sub_mapper: u8,
    jy_mode: u8,
    jy_ciram: u8,
    jy_vram: u8,
    jy_outer: u8,
    jy_prg: [u8; 4],
    jy_chr: [u16; 8],
    jy_nt: [u16; 4],
    jy_latch: [u8; 2],
    jy_irq_control: u8,
    jy_irq_enabled: bool,
    jy_irq_pending: bool,
    jy_irq_prescaler: u8,
    jy_irq_counter: u8,
    jy_irq_xor: u8,
    jy_last_a12: bool,
    jy_mul1: u8,
    jy_mul2: u8,
    jy_adder: u8,
    jy_test: u8,
}

impl Mapper394 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let mut config = Mmc3Config::for_ines(header, 0, chr_size, rom, rom_name);
        config.ax5202p = true;
        config.prg_ram_size = 0;
        let sm = if header.len() > 9 { header[8] >> 4 } else { 0 };
        Self {
            mmc3: MapperMMC3::new(config),
            reg: [0; 4],
            sub_mapper: sm,
            jy_mode: 0, jy_ciram: 0, jy_vram: 0, jy_outer: 0,
            jy_prg: [0; 4], jy_chr: [0; 8], jy_nt: [0; 4],
            jy_latch: [0, 4],
            jy_irq_control: 0, jy_irq_enabled: false, jy_irq_pending: false,
            jy_irq_prescaler: 0, jy_irq_counter: 0, jy_irq_xor: 0, jy_last_a12: false,
            jy_mul1: 0, jy_mul2: 0, jy_adder: 0, jy_test: 0,
        }
    }

    fn prg_and(&self) -> u8 {
        if (self.reg[3] & 0x10) != 0 { 0x1F } else { 0x0F }
    }

    fn chr_and(&self) -> u8 {
        if (self.reg[3] & 0x80) != 0 { 0xFF } else { 0x7F }
    }

    fn prg_or(&self) -> u8 {
        ((self.reg[3] << 1) & 0x10) | ((self.reg[1] << 5) & 0x60)
    }

    fn chr_or(&self) -> u16 {
        if self.sub_mapper == 1 {
            ((self.reg[3] as u16) << 1 & 0x080) | ((self.reg[1] as u16) << 8 & 0x200) | ((self.reg[1] as u16) << 6 & 0x100)
        } else {
            ((self.reg[3] as u16) << 1 & 0x080) | ((self.reg[1] as u16) << 8 & 0x300)
        }
    }

    fn prg_raw_bank_val(&self, cart: &Cartridge, cpu_bank: u8) -> u16 {
        let num_banks = (cart.prg_rom.len() / 0x2000) as u16;
        match cpu_bank {
            0 => {
                if (self.mmc3.r8000 & 0x40) != 0 {
                    num_banks.saturating_sub(2)
                } else {
                    self.mmc3.bank_8c as u16
                }
            }
            1 => self.mmc3.bank_a as u16,
            2 => {
                if (self.mmc3.r8000 & 0x40) != 0 {
                    self.mmc3.bank_8c as u16
                } else {
                    num_banks.saturating_sub(2)
                }
            }
            _ => num_banks.saturating_sub(1),
        }
    }

    fn jy_switchable_last(&self) -> bool {
        (self.jy_mode & 0x04) != 0
    }

    fn jy_extended_mirroring(&self) -> bool {
        (self.jy_ciram & 0x08) != 0
    }

    fn jy_rom_at_6000(&self) -> bool {
        (self.jy_mode & 0x80) != 0
    }

    fn jy_mirroring(&self) -> u8 {
        self.jy_ciram & 0x03
    }

    fn jy_vrom_enabled(&self) -> bool {
        (self.jy_mode & 0x20) != 0
    }

    fn jy_vrom_everywhere(&self) -> bool {
        (self.jy_mode & 0x40) != 0
    }

    fn jy_chr_writable(&self) -> bool {
        (self.jy_vram & 0x40) != 0
    }

    fn jy_vrom_bit(&self) -> bool {
        (self.jy_vram & 0x80) != 0
    }

    fn jy_irq_source(&self) -> u8 {
        self.jy_irq_control & 0x03
    }

    fn jy_small_prescaler(&self) -> bool {
        (self.jy_irq_control & 0x04) != 0
    }

    fn jy_not_counting(&self) -> bool {
        (self.jy_irq_control & 0x08) != 0
    }

    fn jy_irq_direction(&self) -> u8 {
        self.jy_irq_control >> 6
    }

    fn jy_clock_irq(&mut self) {
        let mask = if self.jy_small_prescaler() { 0x07 } else { 0xFF };
        if !self.jy_irq_enabled { return; }
        match self.jy_irq_direction() {
            1 => {
                let prescaler = (self.jy_irq_prescaler & !mask) | ((self.jy_irq_prescaler + 1) & mask);
                self.jy_irq_prescaler = prescaler;
                if (prescaler & mask) == 0 {
                    if !self.jy_not_counting() { self.jy_irq_counter = self.jy_irq_counter.wrapping_add(1); }
                    if self.jy_irq_counter == 0 { self.jy_irq_pending = true; }
                }
            }
            2 => {
                let prescaler = (self.jy_irq_prescaler & !mask) | ((self.jy_irq_prescaler.wrapping_sub(1)) & mask);
                self.jy_irq_prescaler = prescaler;
                if (prescaler & mask) == mask {
                    if !self.jy_not_counting() { self.jy_irq_counter = self.jy_irq_counter.wrapping_sub(1); }
                    if self.jy_irq_counter == 0xFF { self.jy_irq_pending = true; }
                }
            }
            _ => {}
        }
    }

    fn mmc3_fetch_chr(&self, address: u16, chr_rom: &[u8], chr_ram: &[u8]) -> u8 {
        let raw_bank = mmc3_chr_bank(
            self.mmc3.r8000, self.mmc3.chr_2k0, self.mmc3.chr_2k8,
            self.mmc3.chr_1k0, self.mmc3.chr_1k4, self.mmc3.chr_1k8, self.mmc3.chr_1kc, address,
        );
        let bank = ((raw_bank & self.chr_and()) as u16) | self.chr_or();
        let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
        chr_read(chr_rom, chr_ram, offset)
    }

    fn jy_fetch_chr(&self, address: u16, chr_rom: &[u8], chr_ram: &[u8]) -> u8 {
        let and = 0xFFu8;
        let or = self.chr_or();
        let (bank, sub_offset) = match (self.jy_mode >> 3) & 0x03 {
            0 => {
                let b = (self.jy_chr[0] as usize) & (and as usize >> 3) | (or as usize >> 3);
                (b, (address & 0x1FFF) as usize)
            }
            1 => {
                let half = ((address >> 12) & 1) as usize;
                let latch = self.jy_latch[half] as usize;
                let b = (self.jy_chr[latch] as usize) & (and as usize >> 2) | (or as usize >> 2);
                (b, (address & 0x0FFF) as usize)
            }
            2 => {
                let idx = ((address >> 11) & 3) as usize;
                let b = (self.jy_chr[idx * 2] as usize) & (and as usize >> 1) | (or as usize >> 1);
                (b, (address & 0x07FF) as usize)
            }
            3 => {
                let idx = ((address >> 10) & 7) as usize;
                let b = (self.jy_chr[idx] as usize) & (and as usize) | (or as usize);
                (b, (address & 0x03FF) as usize)
            }
            _ => (0, 0)
        };
        let offset = bank * 0x400 + sub_offset;
        chr_read(chr_rom, chr_ram, offset)
    }

    fn jy_store_chr(&self, cart: &mut Cartridge, address: u16, data: u8) {
        if cart.chr_ram.is_empty() || !self.jy_chr_writable() { return; }
        let and = 0xFFu8;
        let or = self.chr_or();
        let (bank, sub_offset) = match (self.jy_mode >> 3) & 0x03 {
            0 => {
                let b = (self.jy_chr[0] as usize) & (and as usize >> 3) | (or as usize >> 3);
                (b, (address & 0x1FFF) as usize)
            }
            1 => {
                let half = ((address >> 12) & 1) as usize;
                let latch = self.jy_latch[half] as usize;
                let b = (self.jy_chr[latch] as usize) & (and as usize >> 2) | (or as usize >> 2);
                (b, (address & 0x0FFF) as usize)
            }
            2 => {
                let idx = ((address >> 11) & 3) as usize;
                let b = (self.jy_chr[idx * 2] as usize) & (and as usize >> 1) | (or as usize >> 1);
                (b, (address & 0x07FF) as usize)
            }
            3 => {
                let idx = ((address >> 10) & 7) as usize;
                let b = (self.jy_chr[idx] as usize) & (and as usize) | (or as usize);
                (b, (address & 0x03FF) as usize)
            }
            _ => (0, 0)
        };
        let offset = bank * 0x400 + sub_offset;
        let len = cart.chr_ram.len();
        if len > 0 { cart.chr_ram[offset % len] = data; }
    }

    fn jy_compute_prg6000_bank(&self) -> usize {
        let and = 0x1Fu8;
        let or = self.prg_or();
        let prg3 = if self.jy_switchable_last() { self.jy_prg[3] } else { 0xFF };
        let bank = match self.jy_mode & 0x03 {
            0 => (prg3 as usize) << 2 | 3,
            1 => (prg3 as usize) << 1 | 1,
            2 => prg3 as usize,
            3 => rev(prg3) as usize,
            _ => prg3 as usize,
        };
        bank & (and as usize) | (or as usize)
    }

    fn jy_nt_slot(&self, address: u16) -> usize {
        ((address >> 10) & 3) as usize
    }

    fn jy_is_vrom_here(&self, slot: usize) -> bool {
        ((self.jy_nt[slot] & 0x80) != 0) ^ self.jy_vrom_bit() || self.jy_vrom_everywhere()
    }

    fn jy_mirror_nametable(&self, address: u16) -> u16 {
        let slot = self.jy_nt_slot(address);
        if self.jy_extended_mirroring() || (self.jy_vrom_enabled() && !self.jy_is_vrom_here(slot)) {
            (address & 0x37FF & !0x0400) | ((self.jy_nt[slot] & 1) as u16) << 10
        } else {
            match self.jy_mirroring() {
                0 => address & 0x37FF,
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                2 => (address & 0x37FF) & !0x0400,
                3 => (address & 0x37FF) | 0x0400,
                _ => address & 0x37FF,
            }
        }
    }
}

impl Mapper for Mapper394 {
    fn reset(&mut self) {
        self.reg = [0; 4];
        self.reg[1] = 0x0F;
        self.reg[3] = 0x90;
        self.mmc3.reset();
        self.jy_mode = 0;
        self.jy_ciram = 0;
        self.jy_vram = 0;
        self.jy_outer = 0;
        self.jy_irq_enabled = false;
        self.jy_irq_pending = false;
        self.jy_irq_control = 0;
        self.jy_irq_prescaler = 0;
        self.jy_irq_counter = 0;
        self.jy_irq_xor = 0;
        self.jy_last_a12 = false;
        self.jy_mul1 = 0;
        self.jy_mul2 = 0;
        self.jy_adder = 0;
        self.jy_test = 0;
        for c in &mut self.jy_prg { *c = 0; }
        for c in &mut self.jy_chr { *c = 0; }
        for c in &mut self.jy_nt { *c = 0; }
        self.jy_latch = [0, 4];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x5000 && address < 0x6000 {
            let page_addr = address as usize & 0xFFF;
            let data = if (page_addr & 0x3FF) == 0 && page_addr != 0x800 {
                0
            } else if page_addr & 0x800 != 0 {
                match page_addr & 3 {
                    0 => (self.jy_mul1 as u16 * self.jy_mul2 as u16) as u8,
                    1 => ((self.jy_mul1 as u16 * self.jy_mul2 as u16) >> 8) as u8,
                    2 => self.jy_adder,
                    3 => self.jy_test,
                    _ => 0,
                }
            } else { 0 };
            return FetchResult { data, driven: true };
        }
        if address >= 0x6000 && (self.reg[1] & 0x10) != 0 && self.jy_rom_at_6000() {
            let bank = self.jy_compute_prg6000_bank();
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let data = prg_rom_read(cart, offset);
            return FetchResult { data, driven: true };
        }
        if address >= 0x8000 {
            if (self.reg[1] & 0x10) != 0 {
                let prg_mode = self.jy_mode & 0x03;
                let and = 0x1Fu8;
                let or = self.prg_or();
                let prg3 = if self.jy_switchable_last() { self.jy_prg[3] } else { 0xFF };
                let bank = match prg_mode {
                    0 => (prg3 as usize) & (and as usize >> 2) | (or as usize >> 2),
                    1 => {
                        if ((address - 0x8000) / 0x2000) < 2 {
                            (self.jy_prg[1] as usize) & (and as usize >> 1) | (or as usize >> 1)
                        } else {
                            (prg3 as usize) & (and as usize >> 1) | (or as usize >> 1)
                        }
                    }
                    _ => {
                        let idx = ((address - 0x8000) / 0x2000) as usize;
                        let raw = if prg_mode == 3 { rev(self.jy_prg[idx.clamp(0, 3)]) } else { self.jy_prg[idx.clamp(0, 3)] };
                        (raw as usize) & (and as usize) | (or as usize)
                    }
                };
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                let data = prg_rom_read(cart, offset);
                FetchResult { data, driven: true }
            } else {
                if (self.reg[1] & 0x08) != 0 {
                    let cpu_bank = ((address - 0x8000) / 0x2000) as u8;
                    let raw_bank = self.prg_raw_bank_val(cart, cpu_bank);
                    let bank = ((raw_bank as u8) & self.prg_and()) | self.prg_or();
                    let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
                    let data = prg_rom_read(cart, offset);
                    FetchResult { data, driven: true }
                } else {
                    let prg_or_ext = self.prg_or() | ((self.reg[3] << 1) & 0x0F);
                    let bank_32k = (prg_or_ext as usize) >> 2;
                    let offset = bank_32k * 0x8000 + (address as usize & 0x7FFF);
                    let data = prg_rom_read(cart, offset);
                    FetchResult { data, driven: true }
                }
            }
        } else {
            self.mmc3.fetch_prg(cart, address)
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (self.reg[1] & 0x10) != 0 && self.jy_irq_source() == 3 {
            self.jy_clock_irq();
        }
        if address >= 0x5000 && address < 0x6000 {
            self.reg[(address as usize) & 3] = data;
        } else if address >= 0x8000 {
            if (self.reg[1] & 0x10) != 0 {
                let page = (address >> 12) as usize;
                let page_addr = address as usize & 0xFFF;
                match page {
                    0x8 => {
                        if page_addr & 0x800 != 0 { return; }
                        self.jy_prg[page_addr & 3] = data;
                    }
                    0x9 => {
                        if page_addr & 0x800 != 0 { return; }
                        let idx = page_addr & 7;
                        self.jy_chr[idx] = (self.jy_chr[idx] & 0xFF00) | data as u16;
                    }
                    0xA => {
                        if page_addr & 0x800 != 0 { return; }
                        let idx = page_addr & 7;
                        self.jy_chr[idx] = (self.jy_chr[idx] & 0x00FF) | (data as u16) << 8;
                    }
                    0xB => {
                        if page_addr & 0x800 != 0 { return; }
                        if page_addr & 4 == 0 {
                            self.jy_nt[page_addr & 3] = (self.jy_nt[page_addr & 3] & 0xFF00) | data as u16;
                        } else {
                            self.jy_nt[page_addr & 3] = (self.jy_nt[page_addr & 3] & 0x00FF) | (data as u16) << 8;
                        }
                    }
                    0xC => {
                        match page_addr & 7 {
                            0 => {
                                self.jy_irq_enabled = (data & 1) != 0;
                                if !self.jy_irq_enabled {
                                    self.jy_irq_prescaler = 0;
                                    self.jy_irq_pending = false;
                                }
                            }
                            1 => self.jy_irq_control = data,
                            2 => { self.jy_irq_enabled = false; self.jy_irq_prescaler = 0; self.jy_irq_pending = false; }
                            3 => self.jy_irq_enabled = true,
                            4 => self.jy_irq_prescaler = data ^ self.jy_irq_xor,
                            5 => self.jy_irq_counter = data ^ self.jy_irq_xor,
                            6 => self.jy_irq_xor = data,
                            _ => {}
                        }
                    }
                    0xD => {
                        if page_addr & 0x800 != 0 { return; }
                        match page_addr & 3 {
                            0 => self.jy_mode = data,
                            1 => self.jy_ciram = data,
                            2 => self.jy_vram = data,
                            3 => self.jy_outer = data,
                            _ => {}
                        }
                    }
                    _ => {}
                }
            } else {
                self.mmc3.store_prg(cart, address, data);
            }
        } else {
            self.mmc3.store_prg(cart, address, data);
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if (self.reg[1] & 0x10) != 0 {
            self.jy_mirror_nametable(address)
        } else {
            self.mmc3.mirror_nametable(cart, address)
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if (self.reg[1] & 0x10) != 0 && self.jy_irq_source() == 2 {
            self.jy_clock_irq();
        }
        let byte = if address >= 0x2000 && (self.reg[1] & 0x10) != 0 {
            let slot = self.jy_nt_slot(address);
            if self.jy_vrom_enabled() && self.jy_is_vrom_here(slot) {
                let and = 0xFFu8;
                let or = self.chr_or();
                let bank = ((self.jy_nt[slot] as usize) & (and as usize)) | (or as usize);
                let offset = bank * 0x400 + (address as usize & 0x3FF);
                chr_read(chr_rom, chr_ram, offset)
            } else {
                let mirrored = self.jy_mirror_nametable(address);
                if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                    let idx = (mirrored & 0x7FF) as usize;
                    if idx < prg_vram.len() { prg_vram[idx] } else { 0 }
                } else {
                    vram[(mirrored & 0x7FF) as usize]
                }
            }
        } else if address >= 0x2000 {
            let (b, a) = self.mmc3.fetch_ppu(
                _prg_rom, chr_rom, _prg_ram, chr_ram, prg_vram,
                using_chr_ram, _nametable_horizontal_mirroring,
                alternative_nametable_arrangement, ppu_address_bus, ppu_octal_latch, vram,
            );
            return (b, a);
        } else {
            if (self.reg[1] & 0x10) != 0 {
                self.jy_fetch_chr(address, chr_rom, chr_ram)
            } else {
                self.mmc3_fetch_chr(address, chr_rom, chr_ram)
            }
        };
        if (self.reg[1] & 0x10) != 0 && (self.jy_outer & 0x80) != 0 && (ppu_address_bus & 0x3000) == 0x3000 {
            let latch_idx = ((ppu_address_bus >> 14) & 1) as usize;
            let bank4 = ((ppu_address_bus >> 12) as u8) & 0x04;
            match ppu_address_bus & 0x3F8 {
                0x3D8 => self.jy_latch[latch_idx] = bank4 | 0,
                0x3E8 => self.jy_latch[latch_idx] = bank4 | 2,
                _ => {}
            }
        }
        new_addr_bus |= byte as u16;
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address >= 0x2000 && (self.reg[1] & 0x10) != 0 {
            let slot = self.jy_nt_slot(address);
            if self.jy_vrom_enabled() && self.jy_is_vrom_here(slot) {
                return;
            }
            let mirrored = self.jy_mirror_nametable(address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
            }
        } else if address >= 0x2000 {
            self.mmc3.store_ppu(cart, address, data, vram);
        } else if cart.chr_ram.is_empty() {
            self.mmc3.store_ppu(cart, address, data, vram);
        } else if (self.reg[1] & 0x10) != 0 {
            self.jy_store_chr(cart, address, data);
        } else {
            let raw_bank = mmc3_chr_bank(
                self.mmc3.r8000, self.mmc3.chr_2k0, self.mmc3.chr_2k8,
                self.mmc3.chr_1k0, self.mmc3.chr_1k4, self.mmc3.chr_1k8, self.mmc3.chr_1kc, address,
            );
            let bank = ((raw_bank & self.chr_and()) as u16) | self.chr_or();
            let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
            let len = cart.chr_ram.len();
            if len > 0 { cart.chr_ram[offset % len] = data; }
        }
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        ppu_sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        if (self.reg[1] & 0x10) != 0 {
            let a12 = (ppu_address_bus & 0x1000) != 0;
            if a12 && !self.jy_last_a12 && self.jy_irq_source() == 1 {
                self.jy_clock_irq();
            }
            self.jy_last_a12 = a12;
            self.jy_irq_pending
        } else {
            self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
        }
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        if (self.reg[1] & 0x10) != 0 {
            if self.jy_irq_source() == 0 { self.jy_clock_irq(); }
            self.jy_irq_pending
        } else {
            self.mmc3.cpu_clock_rise(ppu_address_bus)
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        if (self.reg[1] & 0x10) != 0 {
            let pending = self.jy_irq_pending;
            self.jy_irq_pending = false;
            pending
        } else {
            self.mmc3.take_irq_ack()
        }
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        for r in &self.reg { state.push(*r); }
        state.push(self.sub_mapper);
        state.push(self.jy_mode);
        state.push(self.jy_ciram);
        state.push(self.jy_vram);
        state.push(self.jy_outer);
        for p in &self.jy_prg { state.push(*p); }
        for c in &self.jy_chr { state.extend_from_slice(&c.to_le_bytes()); }
        for n in &self.jy_nt { state.extend_from_slice(&n.to_le_bytes()); }
        for l in &self.jy_latch { state.push(*l); }
        state.push(self.jy_irq_control);
        state.push(self.jy_irq_prescaler);
        state.push(self.jy_irq_counter);
        state.push(self.jy_irq_xor);
        state.push(if self.jy_irq_enabled { 1 } else { 0 });
        state.push(if self.jy_irq_pending { 1 } else { 0 });
        state.push(if self.jy_last_a12 { 1 } else { 0 });
        state.push(self.jy_mul1);
        state.push(self.jy_mul2);
        state.push(self.jy_adder);
        state.push(self.jy_test);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        for r in &mut self.reg {
            if p < state.len() { *r = state[p]; p += 1; }
        }
        if p < state.len() { self.sub_mapper = state[p]; p += 1; }
        if p < state.len() { self.jy_mode = state[p]; p += 1; }
        if p < state.len() { self.jy_ciram = state[p]; p += 1; }
        if p < state.len() { self.jy_vram = state[p]; p += 1; }
        if p < state.len() { self.jy_outer = state[p]; p += 1; }
        for c in &mut self.jy_prg { if p < state.len() { *c = state[p]; p += 1; } }
        for c in &mut self.jy_chr {
            if p + 1 < state.len() { *c = u16::from_le_bytes([state[p], state[p+1]]); p += 2; }
        }
        for n in &mut self.jy_nt {
            if p + 1 < state.len() { *n = u16::from_le_bytes([state[p], state[p+1]]); p += 2; }
        }
        for l in &mut self.jy_latch { if p < state.len() { *l = state[p]; p += 1; } }
        if p < state.len() { self.jy_irq_control = state[p]; p += 1; }
        if p < state.len() { self.jy_irq_prescaler = state[p]; p += 1; }
        if p < state.len() { self.jy_irq_counter = state[p]; p += 1; }
        if p < state.len() { self.jy_irq_xor = state[p]; p += 1; }
        if p < state.len() { self.jy_irq_enabled = state[p] != 0; p += 1; }
        if p < state.len() { self.jy_irq_pending = state[p] != 0; p += 1; }
        if p < state.len() { self.jy_last_a12 = state[p] != 0; p += 1; }
        if p < state.len() { self.jy_mul1 = state[p]; p += 1; }
        if p < state.len() { self.jy_mul2 = state[p]; p += 1; }
        if p < state.len() { self.jy_adder = state[p]; p += 1; }
        if p < state.len() { self.jy_test = state[p]; p += 1; }
        p
    }
}
