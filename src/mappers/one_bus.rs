fn next_pow2(n: usize) -> usize {
    if n == 0 { return 1; }
    let mut p = 1;
    while p < n { p <<= 1; }
    p
}

pub struct OneBus {
    pub reg2000: [u8; 0x100],
    pub reg4100: [u8; 0x100],
    #[allow(dead_code)]
    pub chr_low: Vec<u8>,
    #[allow(dead_code)]
    pub chr_high: Vec<u8>,
    #[allow(dead_code)]
    pub chr_rom_size: usize,
    pub irq_counter: u8,
    pub irq_reload: u8,
    pub irq_enabled: bool,
    pub pa12_filter: u8,
    pub irq_delay: u8,
    irq_ack: bool,
    pub relative_8k: usize,
}

impl OneBus {
    pub fn new(_prg_rom: &[u8], chr_rom: &[u8]) -> Self {
        let raw_chr = if chr_rom.is_empty() {
            vec![0u8; 0x2000]
        } else {
            chr_rom.to_vec()
        };
        let chr_rom_size = next_pow2(raw_chr.len().max(1));
        let mut chr_low = vec![0u8; chr_rom_size];
        let mut chr_high = vec![0u8; chr_rom_size];
        for i in 0..chr_rom_size.min(raw_chr.len()) {
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
            chr_rom_size,
            irq_counter: 0,
            irq_reload: 0,
            irq_enabled: false,
            pa12_filter: 0,
            irq_delay: 0,
            irq_ack: false,
            relative_8k: 0,
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
        self.irq_ack = false;
        self.relative_8k = 0;
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

    fn ps(&self) -> u8 { self.reg4100[0x0B] & 7 }
    pub fn comr6(&self) -> bool { (self.reg4100[0x05] & 0x40) != 0 }
    pub fn comr7(&self) -> bool { (self.reg4100[0x05] & 0x80) != 0 }
    fn pq0(&self) -> u8 { self.reg4100[0x07] }
    fn pq1(&self) -> u8 { self.reg4100[0x08] }
    fn pq2(&self) -> u8 { self.reg4100[0x09] }
    fn pq3(&self) -> u8 { self.reg4100[0x0A] }
    fn pq2en(&self) -> bool { (self.reg4100[0x0B] & 0x40) != 0 }
    fn pa21(&self) -> u16 { (self.reg4100[0x00] >> 4) as u16 }
    fn va21(&self) -> u16 { (self.reg4100[0x00] & 0x0F) as u16 }
    fn va18(&self) -> u16 { ((self.reg2000[0x18] >> 4) & 7) as u16 }
    #[allow(dead_code)]
    fn vrwb(&self) -> u8 { self.reg2000[0x18] & 7 }
    fn vb0s(&self) -> u8 { self.reg2000[0x1A] & 7 }
    fn rv6(&self) -> u16 { (self.reg2000[0x1A] as u16) & 0xF8 }
    pub fn hv(&self) -> u8 { self.reg4100[0x06] & 1 }
    fn tsynen(&self) -> bool { (self.reg4100[0x0B] & 0x80) != 0 }

    pub fn get_prg_bank(&self, slot: usize) -> usize {
        let ps = self.ps();
        let prg_and = if ps == 7 { 0xFFu16 } else { 0x3Fu16 >> ps };
        let prg_or = (self.pq3() as u16 | (self.pa21() << 8)) & !prg_and;
        let flip = if self.comr6() { 2 } else { 0 };
        let effective_slot = if slot & 1 == 0 { slot ^ flip } else { slot };
        let pq = match effective_slot {
            0 => self.pq0(),
            1 => self.pq1(),
            2 => if self.pq2en() { self.pq2() } else { 0xFE },
            3 => 0xFF,
            _ => 0,
        };
        ((pq as u16 & prg_and) | prg_or).wrapping_add(self.relative_8k as u16) as usize
    }

    pub fn fetch_prg(&self, prg_rom: &[u8], address: u16, and_mask: u16, or_mask: u16) -> u8 {
        if address < 0x8000 { return 0; }
        let slot = ((address - 0x8000) >> 13) as usize;
        let bank = (self.get_prg_bank(slot) as u16 & and_mask) | or_mask;
        let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
        if prg_rom.is_empty() { 0 } else { prg_rom[offset % prg_rom.len()] }
    }

    pub fn fetch_chr(&self, chr_data: &[u8], address: u16, and_mask: usize, or_mask: usize) -> u8 {
        if address >= 0x2000 { return 0; }
        let vb0s_table: [u8; 8] = [0, 1, 2, 0, 3, 4, 5, 1];
        let vb0s = self.vb0s() as usize;
        let shift = vb0s_table[vb0s.min(7)] as u16;
        let chr_and = 0xFFu16 >> shift;
        let chr_or = (self.rv6() as u16 & !chr_and) | (self.va18() << 8);
        let flip = if self.comr7() { 4 } else { 0 };
        let slot = ((address as usize >> 10) & 7) ^ flip;
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
        let bank = (((bank_reg as u16 & chr_and) | chr_or | (self.va21() << 11)) as usize & and_mask | or_mask).wrapping_add(self.relative_8k * 8);
        let offset = bank * 0x400 + (address as usize & 0x3FF);
        if chr_data.is_empty() { 0 } else { chr_data[offset % chr_data.len()] }
    }

    pub fn mirror_nametable(&self, address: u16) -> u16 {
        if self.hv() == 0 {
            address & 0x37FF
        } else {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        }
    }

    pub fn write_ppu(&mut self, addr: u16, val: u8) {
        if (addr & 0xFF) >= 8 {
            self.reg2000[(addr & 0xFF) as usize] = val;
        }
    }

    pub fn write_apu(&mut self, addr: u16, val: u8) {
        let idx = (addr & 0xFF) as usize;
        match idx {
            0x01 => self.irq_reload = val,
            0x02 => self.irq_counter = 0,
            0x03 => { self.irq_enabled = false; self.irq_ack = true; }
            0x04 => self.irq_enabled = true,
            _ => {}
        }
        self.reg4100[idx] = val;
        if idx == 0x60 || idx == 0x61 {
            self.relative_8k = (self.reg4100[0x60] as usize) | ((self.reg4100[0x61] as usize) << 8 & 0xF00);
        }
    }

    pub fn write_mmc3(&mut self, address: u16, val: u8) {
        let bank_bits = ((address >> 12) & 6) as u8;
        let addr_bit_0 = (address & 1) as u8;
        let mmc3_addr = bank_bits | addr_bit_0;
        match mmc3_addr {
            0 => self.write_apu(0x4105, val & !0x20),
            1 => {
                let pointer = self.reg4100[0x05] & 7;
                if pointer < 2 {
                    self.write_ppu(0x2016 + pointer as u16, val);
                } else if pointer < 6 {
                    self.write_ppu(0x2010 + pointer as u16, val);
                } else {
                    self.write_apu(0x4101 + pointer as u16, val);
                }
            }
            2 => self.write_apu(0x4106, val & 1),
            4 => self.write_apu(0x4101, val),
            5 => self.write_apu(0x4102, val),
            6 => self.write_apu(0x4103, val),
            7 => self.write_apu(0x4104, val),
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
            self.irq_ack = true;
        }
    }

    pub fn cpu_cycle(&mut self) -> bool {
        if self.pa12_filter > 0 {
            self.pa12_filter = self.pa12_filter.wrapping_sub(1);
        }
        if self.irq_delay > 0 {
            self.irq_delay = self.irq_delay.wrapping_sub(1);
            if self.irq_delay == 0 {
                self.irq_ack = false;
                return true;
            }
        }
        false
    }

    pub fn ppu_cycle(&mut self, addr: u16, _a12_prev: bool, scanline: u16, dot: u16, rendering: bool) -> bool {
        if self.tsynen() || (self.reg2000[0x10] & 0x02) != 0 {
            if scanline < 242 && dot == 256 {
                self.clock_scanline_counter(rendering);
            }
        } else {
            if (addr & 0x1000) != 0 && self.pa12_filter == 0 {
                self.clock_scanline_counter(rendering);
                self.pa12_filter = 3;
            }
        }
        if self.irq_ack {
            self.irq_ack = false;
            return true;
        }
        false
    }

    pub fn take_irq_ack(&mut self) -> bool {
        if self.irq_ack {
            self.irq_ack = false;
            return true;
        }
        false
    }

    pub fn save(&self) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.reg2000);
        state.extend_from_slice(&self.reg4100);
        state.push(self.irq_counter);
        state.push(self.irq_reload);
        state.push(if self.irq_enabled { 1 } else { 0 });
        state.push(self.pa12_filter);
        state.push(self.irq_delay);
        state
    }

    pub fn load_state(&mut self, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 0x100 <= state.len() {
            self.reg2000.copy_from_slice(&state[p..p + 0x100]);
            p += 0x100;
        }
        if p + 0x100 <= state.len() {
            self.reg4100.copy_from_slice(&state[p..p + 0x100]);
            p += 0x100;
        }
        if p < state.len() { self.irq_counter = state[p]; p += 1; }
        if p < state.len() { self.irq_reload = state[p]; p += 1; }
        if p < state.len() { self.irq_enabled = state[p] != 0; p += 1; }
        if p < state.len() { self.pa12_filter = state[p]; p += 1; }
        if p < state.len() { self.irq_delay = state[p]; p += 1; }
        self.relative_8k = (self.reg4100[0x60] as usize) | ((self.reg4100[0x61] as usize) << 8 & 0xF00);
        p
    }
}
