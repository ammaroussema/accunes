//! Shared OneBus ASIC core (NintendulatorNRS `h_OneBus.cpp`).
//! Used by mappers 256, 270, and 296 (MMC3 mode).

pub const VB0S_TABLE: [u8; 8] = [0, 1, 2, 0, 3, 4, 5, 1];

pub fn is_onebus_mapper(mapper: u16) -> bool {
    matches!(mapper, 256 | 270 | 296)
}

pub fn descramble_chr_byte(val: u8) -> u8 {
    (val << 4 & 0x90) | (val >> 4 & 0x09) | (val << 1 & 0x44) | (val >> 1 & 0x22)
}

fn next_pow2(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    let mut p = 1;
    while p < n {
        p <<= 1;
    }
    p
}

fn rom_read(rom: &[u8], offset: usize) -> u8 {
    if rom.is_empty() {
        0
    } else {
        rom[offset % rom.len()]
    }
}

/// PRG/CHR window masks applied on top of the base OneBus banking formulas.
#[derive(Clone, Copy, Debug, Default)]
pub struct OneBusBanking {
    pub prg_and: u16,
    pub prg_or: u16,
    pub chr_and: usize,
    pub chr_or: usize,
}

impl OneBusBanking {
    pub const MAPPER256: Self = Self {
        prg_and: 0x0FFF,
        prg_or: 0,
        chr_and: 0x7FFF,
        chr_or: 0,
    };

    pub fn mapper270(submapper: u8, reg2c: u8) -> Self {
        let prg_or = prg_or_from_412c(submapper, reg2c);
        Self {
            prg_and: 0x07FF,
            prg_or,
            chr_and: 0x3FFF,
            chr_or: (prg_or as usize) << 3,
        }
    }
}

pub fn prg_or_from_412c(submapper: u8, reg2c: u8) -> u16 {
    match submapper {
        1 => {
            if reg2c & 0x02 != 0 {
                0x0800
            } else {
                0
            }
        }
        2 => {
            let mut or_val = 0u16;
            if reg2c & 0x02 != 0 {
                or_val |= 0x0800;
            }
            if reg2c & 0x01 != 0 {
                or_val |= 0x1000;
            }
            or_val
        }
        3 => {
            if reg2c & 0x04 != 0 {
                0x0800
            } else {
                0
            }
        }
        _ => {
            let mut or_val = 0u16;
            if reg2c & 0x06 != 0 {
                or_val |= 0x0800;
            }
            if reg2c & 0x01 != 0 {
                or_val |= 0x1000;
            }
            or_val
        }
    }
}

pub fn prg_or_from_296_regs(reg2c: u8, reg2e: u8) -> u16 {
    (if reg2c & 1 != 0 { 0x1000 } else { 0 })
        | (if reg2c & 4 != 0 { 0x2000 } else { 0 })
        | (if reg2e & 1 != 0 { 0x4000 } else { 0 })
}

pub fn chr_or_from_296_regs(reg2c: u8, reg2e: u8) -> usize {
    (if reg2c & 2 != 0 { 0x8000 } else { 0 })
        | (if reg2c & 8 != 0 { 0x10000 } else { 0 })
        | (if reg2e & 1 != 0 { 0x20000 } else { 0 })
}

/// Optional submapper register remapping (mapper 256).
#[derive(Clone, Copy, Default)]
pub struct OneBusMangle {
    pub ppu: [u8; 6],
    pub cpu: [u8; 4],
    pub mmc3: [u8; 8],
}

impl OneBusMangle {
    pub const IDENTITY: Self = Self {
        ppu: [0, 1, 2, 3, 4, 5],
        cpu: [0, 1, 2, 3],
        mmc3: [0, 1, 2, 3, 4, 5, 6, 7],
    };
}

pub struct OneBus {
    pub reg2000: [u8; 0x100],
    pub reg4100: [u8; 0x100],
    chr_low: Vec<u8>,
    chr_high: Vec<u8>,
    chr_mask: usize,
    chr_source_len: usize,
    pub banking: OneBusBanking,
    pub irq_counter: u8,
    pub irq_reload: u8,
    pub irq_enabled: bool,
    pub pa12_filter: u8,
    pub irq_delay: u8,
    pub prg_ram_protect: u8,
    pending_irq: bool,
}

impl OneBus {
    pub fn new(prg_rom: &[u8], chr_rom: &[u8], banking: OneBusBanking) -> Self {
        let raw_chr = if chr_rom.is_empty() {
            prg_rom
        } else {
            chr_rom
        };
        let chr_size = next_pow2(raw_chr.len().max(1));
        let mut chr_low = vec![0u8; chr_size];
        let mut chr_high = vec![0u8; chr_size];
        for i in 0..chr_size.min(raw_chr.len()) {
            let shifted = (i & 0xF) | ((i >> 1) & !0xF);
            if i & 0x10 != 0 {
                chr_high[shifted] = raw_chr[i];
            } else {
                chr_low[shifted] = raw_chr[i];
            }
        }
        let mut ob = OneBus {
            reg2000: [0; 0x100],
            reg4100: [0; 0x100],
            chr_low,
            chr_high,
            chr_mask: chr_size.saturating_sub(1),
            chr_source_len: raw_chr.len(),
            banking,
            irq_counter: 0,
            irq_reload: 0,
            irq_enabled: false,
            pa12_filter: 0,
            irq_delay: 0,
            prg_ram_protect: 0,
            pending_irq: false,
        };
        ob.reset();
        ob
    }

    pub fn reset(&mut self) {
        self.reg2000 = [0; 0x100];
        self.reg4100 = [0; 0x100];
        self.irq_counter = 0;
        self.irq_reload = 0;
        self.irq_enabled = false;
        self.pa12_filter = 0;
        self.irq_delay = 0;
        self.prg_ram_protect = 0;
        self.pending_irq = false;
        self.reg2000[0x10] = 0x00;
        self.reg2000[0x12] = 0x04;
        self.reg2000[0x13] = 0x05;
        self.reg2000[0x14] = 0x06;
        self.reg2000[0x15] = 0x07;
        self.reg2000[0x16] = 0x00;
        self.reg2000[0x17] = 0x02;
        self.reg2000[0x18] = 0x00;
        self.reg2000[0x1A] = 0x00;
        self.reg4100[0x00] = 0x00;
        self.reg4100[0x05] = 0x00;
        self.reg4100[0x07] = 0x00;
        self.reg4100[0x08] = 0x01;
        self.reg4100[0x09] = 0xFE;
        self.reg4100[0x0A] = 0x00;
        self.reg4100[0x0B] = 0x00;
        self.reg4100[0x0F] = 0xFF;
        self.reg4100[0x60] = 0x00;
        self.reg4100[0x61] = 0x00;
    }

    pub fn relative_8k(&self) -> usize {
        (self.reg4100[0x60] as usize) | ((self.reg4100[0x61] as usize) << 8 & 0xF00)
    }

    fn ps(&self) -> u8 {
        self.reg4100[0x0B] & 7
    }

    pub fn comr6(&self) -> bool {
        (self.reg4100[0x05] & 0x40) != 0
    }

    pub fn comr7(&self) -> bool {
        (self.reg4100[0x05] & 0x80) != 0
    }

    fn pq2en(&self) -> bool {
        (self.reg4100[0x0B] & 0x40) != 0
    }

    fn tsynen(&self) -> bool {
        (self.reg4100[0x0B] & 0x80) != 0
    }

    fn bk16en(&self) -> bool {
        (self.reg2000[0x10] & 0x02) != 0
    }

    fn use_4bpp_chr(&self) -> bool {
        let flags = self.reg2000[0x10];
        (flags & 0x86) != 0
    }

    pub fn hv(&self) -> u8 {
        self.reg4100[0x06] & 1
    }

    pub fn get_prg_bank(&self, slot: usize) -> usize {
        let ps = self.ps();
        let prg_and = if ps == 7 { 0xFFu16 } else { 0x3Fu16 >> ps };
        let prg_or =
            (self.reg4100[0x0A] as u16 | ((self.reg4100[0x00] >> 4) as u16) << 8) & !prg_and;
        let flip = if self.comr6() { 2 } else { 0 };
        let effective_slot = if slot & 1 == 0 { slot ^ flip as usize } else { slot };
        let pq = match effective_slot {
            0 => self.reg4100[0x07] as u16,
            1 => self.reg4100[0x08] as u16,
            2 => {
                if self.pq2en() {
                    self.reg4100[0x09] as u16
                } else {
                    0xFE
                }
            }
            3 => 0xFF,
            _ => 0,
        };
        let bank = (pq & prg_and | prg_or) as usize + self.relative_8k();
        (bank as u16 & self.banking.prg_and | self.banking.prg_or) as usize
    }

    pub fn chr_bank_1k(&self, slot: usize) -> usize {
        let vb0s = self.reg2000[0x1A] & 7;
        let shift = VB0S_TABLE[vb0s as usize] as u16;
        let chr_and = 0xFFu16 >> shift;
        let chr_or = (self.reg2000[0x1A] as u16 & 0xF8) & !chr_and;
        let va18 = ((self.reg2000[0x18] >> 4) & 7) as u16;
        let chr_or_va = chr_or | (va18 << 8);
        let va21 = (self.reg4100[0x00] & 0x0F) as u16;
        let bank_reg = match slot {
            0 => self.reg2000[0x16] & !1,
            1 => self.reg2000[0x16] | 1,
            2 => self.reg2000[0x17] & !1,
            3 => self.reg2000[0x17] | 1,
            4 => self.reg2000[0x12],
            5 => self.reg2000[0x13],
            6 => self.reg2000[0x14],
            7 => self.reg2000[0x15],
            _ => 0,
        };
        let bank = ((bank_reg as u16 & chr_and) | chr_or_va | (va21 << 11)) as usize
            + self.relative_8k() * 8;
        (bank & self.banking.chr_and | self.banking.chr_or) as usize
    }

    pub fn fetch_prg_byte(&self, prg_rom: &[u8], address: u16) -> u8 {
        if address < 0x8000 {
            return 0;
        }
        let slot = ((address - 0x8000) >> 13) as usize;
        let bank = self.get_prg_bank(slot);
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        rom_read(prg_rom, offset)
    }

    fn ensure_chr_planes(&mut self, prg_rom: &[u8], chr_rom: &[u8]) {
        let raw_chr = if chr_rom.is_empty() { prg_rom } else { chr_rom };
        if raw_chr.len() == self.chr_source_len {
            return;
        }
        let chr_size = next_pow2(raw_chr.len().max(1));
        self.chr_low = vec![0u8; chr_size];
        self.chr_high = vec![0u8; chr_size];
        self.chr_mask = chr_size.saturating_sub(1);
        for i in 0..chr_size.min(raw_chr.len()) {
            let shifted = (i & 0xF) | ((i >> 1) & !0xF);
            if i & 0x10 != 0 {
                self.chr_high[shifted] = raw_chr[i];
            } else {
                self.chr_low[shifted] = raw_chr[i];
            }
        }
        self.chr_source_len = raw_chr.len();
    }

    pub fn fetch_chr_byte(
        &mut self,
        prg_rom: &[u8],
        chr_rom: &[u8],
        chr_ram: &[u8],
        address: u16,
        chr_ram_flat: bool,
    ) -> u8 {
        self.ensure_chr_planes(prg_rom, chr_rom);
        if address >= 0x2000 {
            return 0;
        }
        if chr_ram_flat && !chr_ram.is_empty() {
            return chr_ram[address as usize & 0x1FFF];
        }
        let flip = if self.comr7() { 4 } else { 0 };
        let slot = ((address as usize >> 10) & 7) ^ flip;
        let bank = self.chr_bank_1k(slot);
        let offset = bank * 0x400 + (address as usize & 0x3FF);
        // OneBus carts with no CHR ROM fetch graphics from PRG (see h_OneBus.cpp `chrData`).
        // iNES still allocates CHR RAM for CHR size 0; ignore it unless flat CHR RAM mode is active.
        let use_planes = self.use_4bpp_chr();
        let result = if use_planes {
            let plane = if address & 0x4000 != 0 {
                &self.chr_high
            } else {
                &self.chr_low
            };
            if !plane.is_empty() {
                rom_read(plane, offset & self.chr_mask)
            } else {
                0
            }
        } else if !chr_rom.is_empty() {
            rom_read(chr_rom, offset)
        } else {
            rom_read(prg_rom, offset)
        };

        result
    }

    pub fn read_apu(&self, address: u16) -> Option<u8> {
        let idx = (address & 0xFF) as usize;
        if address >= 0x4100 && address < 0x4200 {
            if (0x100..=0x10D).contains(&idx) || (0x160..0x200).contains(&idx) {
                return Some(self.reg4100[idx]);
            }
        }
        None
    }

    pub fn write_ppu(&mut self, addr: u16, val: u8, mangle: &OneBusMangle) {
        let mut a = (addr & 0xFF) as u8;
        if a >= 0x12 && a <= 0x17 {
            a = 0x12 + mangle.ppu[(a - 0x12) as usize];
        }
        if a >= 8 {
            self.reg2000[a as usize] = val;
        }
    }

    pub fn write_apu(&mut self, addr: u16, val: u8, mangle: &OneBusMangle) {
        let mut idx = (addr & 0xFF) as usize;
        if idx >= 0x07 && idx <= 0x0A {
            idx = 0x07 + mangle.cpu[(idx - 0x07) as usize] as usize;
        }
        match idx {
            0x01 => self.irq_reload = val,
            0x02 => self.irq_counter = 0,
            0x03 => {
                self.irq_enabled = false;
                self.pending_irq = false;
            }
            0x04 => self.irq_enabled = true,
            _ => {}
        }
        self.reg4100[idx] = val;
    }

    pub fn write_mmc3(&mut self, address: u16, val: u8, mangle: &OneBusMangle) {
        if (self.reg4100[0x0B] & 0x08) != 0 {
            return;
        }
        let bank_bits = ((address >> 12) & 6) as u8;
        let addr_bit_0 = (address & 1) as u8;
        let mmc3_addr = bank_bits | addr_bit_0;
        let mangled = if mmc3_addr == 1 {
            val & 0xF8 | mangle.mmc3[(val & 0x07) as usize]
        } else {
            val
        };
        match mmc3_addr {
            0 => self.write_apu(0x4105, mangled & !0x20, mangle),
            1 => {
                let pointer = self.reg4100[0x05] & 7;
                if pointer < 2 {
                    self.write_ppu(0x2016 + pointer as u16, mangled, mangle);
                } else if pointer < 6 {
                    self.write_ppu(0x2010 + pointer as u16, mangled, mangle);
                } else {
                    self.write_apu(0x4101 + pointer as u16, mangled, mangle);
                }
            }
            2 => self.write_apu(0x4106, mangled & 1, mangle),
            4 => self.write_apu(0x4101, mangled, mangle),
            5 => self.write_apu(0x4102, mangled, mangle),
            6 => self.write_apu(0x4103, mangled, mangle),
            7 => self.write_apu(0x4104, mangled, mangle),
            _ => {}
        }
    }

    pub fn store_prg_mmc3(&mut self, address: u16, data: u8, mangle: &OneBusMangle) {
        match address & 0xE001 {
            0x8000 => {
                self.reg4100[0x05] = data & 0xF8 | mangle.mmc3[(data & 0x07) as usize];
                self.reg4100[0x05] &= !0x20;
            }
            0x8001 => self.write_mmc3(address, data, mangle),
            0xA000 => self.reg4100[0x06] = data & 1,
            0xA001 => self.prg_ram_protect = data,
            0xC000 => self.irq_reload = data,
            0xC001 => self.irq_counter = 0,
            0xE000 => {
                self.irq_enabled = false;
                self.pending_irq = false;
            }
            0xE001 => self.irq_enabled = true,
            _ => {}
        }
    }

    fn clock_scanline_counter(&mut self, rendering: bool) {
        if self.irq_counter == 0 {
            self.irq_counter = self.irq_reload;
        } else {
            self.irq_counter = self.irq_counter.wrapping_sub(1);
        }
        if self.irq_counter == 0 && self.irq_enabled && rendering {
            self.pending_irq = true;
        }
    }

    pub fn ppu_cycle(
        &mut self,
        addr: u16,
        scanline: u16,
        dot: u16,
        rendering: bool,
    ) -> bool {
        if self.tsynen() || self.bk16en() {
            if scanline < 242 && dot == 256 {
                self.clock_scanline_counter(rendering);
            }
        } else if (addr & 0x1000) != 0 {
            if self.pa12_filter == 0 {
                self.clock_scanline_counter(rendering);
            }
            self.pa12_filter = 3;
        }
        if self.pending_irq {
            self.pending_irq = false;
            return true;
        }
        false
    }

    pub fn cpu_cycle(&mut self) -> bool {
        if self.pa12_filter > 0 {
            self.pa12_filter = self.pa12_filter.wrapping_sub(1);
        }
        if self.irq_delay > 0 {
            self.irq_delay = self.irq_delay.wrapping_sub(1);
            if self.irq_delay == 0 {
                return true;
            }
        }
        false
    }

    pub fn save_core(&self) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.reg2000);
        state.extend_from_slice(&self.reg4100);
        state.push(self.irq_counter);
        state.push(self.irq_reload);
        state.push(if self.irq_enabled { 1 } else { 0 });
        state.push(self.pa12_filter);
        state.push(self.irq_delay);
        state.push(self.prg_ram_protect);
        state.push(if self.pending_irq { 1 } else { 0 });
        state
    }

    pub fn load_core(&mut self, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 0x100 <= state.len() {
            self.reg2000.copy_from_slice(&state[p..p + 0x100]);
            p += 0x100;
        }
        if p + 0x100 <= state.len() {
            self.reg4100.copy_from_slice(&state[p..p + 0x100]);
            p += 0x100;
        }
        if p < state.len() {
            self.irq_counter = state[p];
            p += 1;
        }
        if p < state.len() {
            self.irq_reload = state[p];
            p += 1;
        }
        if p < state.len() {
            self.irq_enabled = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.pa12_filter = state[p];
            p += 1;
        }
        if p < state.len() {
            self.irq_delay = state[p];
            p += 1;
        }
        if p < state.len() {
            self.prg_ram_protect = state[p];
            p += 1;
        }
        if p < state.len() {
            self.pending_irq = state[p] != 0;
            p += 1;
        }
        p
    }
}
