use crate::mappers::adpcm_vt::{vt1682_decode_adpcm, vt369_decode_adpcm};
use crate::mappers::one_bus_gpio::GpioPort;

#[derive(Clone, Copy, Default)]
pub struct OneBusChrCtx {
    pub eva: u16,
    pub is_bg: bool,
    pub is_sprite: bool,
    pub active: bool,
}

impl OneBusChrCtx {
    pub fn map_chr_address(&self, raw_address: u16) -> u16 {
        if !self.active {
            return raw_address;
        }
        let pat = raw_address & 0x1FFF;
        let high_4bpp = raw_address >= 0x4000 && raw_address < 0x6000;
        if self.is_sprite {
            if high_4bpp {
                0xE000 | pat
            } else {
                0xA000 | pat
            }
        } else if self.is_bg {
            if high_4bpp {
                0xC000 | pat
            } else {
                0x8000 | pat
            }
        } else {
            raw_address
        }
    }
}

pub const VB0S_TABLE: [u8; 8] = [0, 1, 2, 0, 3, 4, 5, 0];
pub fn is_onebus_mapper(mapper: u16) -> bool {
    matches!(
        mapper,
        256 | 270 | 296 | 407 | 408 | 419 | 423 | 424 | 425 | 426 | 427 | 436 | 496
    )
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
#[allow(dead_code)]
pub fn prg_or_from_296_regs(reg2c: u8, reg2e: u8) -> u16 {
    (if reg2c & 1 != 0 { 0x1000 } else { 0 })
        | (if reg2c & 4 != 0 { 0x2000 } else { 0 })
        | (if reg2e & 1 != 0 { 0x4000 } else { 0 })
}
#[allow(dead_code)]
pub fn chr_or_from_296_regs(reg2c: u8, reg2e: u8) -> usize {
    (if reg2c & 2 != 0 { 0x8000 } else { 0 })
        | (if reg2c & 8 != 0 { 0x10000 } else { 0 })
        | (if reg2e & 1 != 0 { 0x20000 } else { 0 })
}
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
    pub relative_8k: usize,
    pub vt369_relative: usize,
    pub vt369_bg_data: usize,
    pub vt369_spr_data: usize,
    chr_low: Vec<u8>,
    chr_high: Vec<u8>,
    chr_low16: Vec<u8>,
    chr_high16: Vec<u8>,
    chr_mask: usize,
    chr_source_len: usize,
    pub banking: OneBusBanking,
    pub irq_counter: u8,
    pub irq_reload: u8,
    pub irq_enabled: bool,
    pub pa12_filter: u8,
    pub irq_delay: u8,
    pub prg_ram_protect: u8,
    pub pending_irq: bool,
    pub irq_ack: bool,
    pub gpio: [GpioPort; 4],
    pub console_type_vt369: bool,
    pub console_type_vt09: bool,
    pub console_type_vt03: bool,
    pub submapper: u8,
    pub opcode_encryption: bool,
    pub dma_middle_addr: u8,
    pub dma_length: u16,
    pub dma_target: u16,
    pub alu_operand14: u32,
    pub alu_operand56: u16,
    pub alu_operand67: u16,
    pub alu_busy: u8,
    pub sound_ram: [u8; 8192],
    pub sound_dac: i16,
    pub sound_prescaler: u8,
    pub sound_prescaler_hle: u8,
    pub sound_timer_period: i16,
    pub sound_last_period: i16,
    pub sound_timer_count: i16,
    pub sound_timer_control: u8,
    pub sound_adpcm_frame: [u64; 3],
    pub sound_adpcm_frame_count: [u8; 3],
    pub prg_rom: Vec<u8>,
}
impl OneBus {
    pub fn new(prg_rom: &[u8], chr_rom: &[u8], banking: OneBusBanking) -> Self {
        let raw_chr = if chr_rom.is_empty() {
            prg_rom
        } else {
            chr_rom
        };
        let chr_size = next_pow2(raw_chr.len().max(1));
        let mut chr_low = vec![0u8; chr_size >> 1];
        let mut chr_high = vec![0u8; chr_size >> 1];
        let mut chr_low16 = vec![0u8; chr_size >> 1];
        let mut chr_high16 = vec![0u8; chr_size >> 1];
        for i in 0..chr_size.min(raw_chr.len()) {
            let shifted = (i & 0xF) | ((i >> 1) & !0xF);
            if i & 0x10 != 0 {
                if shifted < chr_high.len() {
                    chr_high[shifted] = raw_chr[i];
                }
            } else if shifted < chr_low.len() {
                chr_low[shifted] = raw_chr[i];
            }
            if i & 1 != 0 {
                if (i >> 1) < chr_high16.len() {
                    chr_high16[i >> 1] = raw_chr[i];
                }
            } else if (i >> 1) < chr_low16.len() {
                chr_low16[i >> 1] = raw_chr[i];
            }
        }
        let mut ob = OneBus {
            reg2000: [0; 0x100],
            reg4100: [0; 0x100],
            relative_8k: 0,
            vt369_relative: 0,
            vt369_bg_data: 0,
            vt369_spr_data: 0,
            chr_low,
            chr_high,
            chr_low16,
            chr_high16,
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
            irq_ack: false,
            gpio: [GpioPort::new(), GpioPort::new(), GpioPort::new(), GpioPort::new()],
            console_type_vt369: false,
            console_type_vt09: false,
            console_type_vt03: false,
            submapper: 0,
            opcode_encryption: false,
            dma_middle_addr: 0,
            dma_length: 0x100,
            dma_target: 0x2004,
            alu_operand14: 0,
            alu_operand56: 0,
            alu_operand67: 0,
            alu_busy: 0,
            sound_ram: [0; 8192],
            sound_dac: 0,
            sound_prescaler: 6,
            sound_prescaler_hle: 6,
            sound_timer_period: 0,
            sound_last_period: 0,
            sound_timer_count: 0,
            sound_timer_control: 0,
            sound_adpcm_frame: [0; 3],
            sound_adpcm_frame_count: [0; 3],
            prg_rom: prg_rom.to_vec(),
        };
        ob.reset();
        ob
    }

    pub fn ensure_prg_rom(&mut self, prg: &[u8]) {
        if self.prg_rom.is_empty() && !prg.is_empty() {
            self.prg_rom = prg.to_vec();
        }
    }
    pub fn reset(&mut self) {
        self.reg2000 = [0; 0x100];
        self.reg4100 = [0; 0x100];
        self.alu_operand14 = 0;
        self.alu_operand56 = 0;
        self.alu_operand67 = 0;
        self.alu_busy = 0;
        self.sound_ram = [0; 8192];
        self.sound_dac = 0;
        self.sound_prescaler = 6;
        self.sound_prescaler_hle = 6;
        self.sound_timer_period = 0;
        self.sound_last_period = 0;
        self.sound_timer_count = 0;
        self.sound_timer_control = 0;
        self.sound_adpcm_frame = [0; 3];
        self.sound_adpcm_frame_count = [0; 3];
        self.relative_8k = 0;
        self.vt369_relative = 0;
        self.vt369_bg_data = 0;
        self.vt369_spr_data = 0;
        self.irq_counter = 0;
        self.irq_reload = 0;
        self.irq_enabled = false;
        self.pa12_filter = 0;
        self.irq_delay = 0;
        self.prg_ram_protect = 0;
        self.pending_irq = false;
        self.irq_ack = false;
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
        if self.console_type_vt369 {
            self.reg4100[0x62] = 0x00;
        }
        for g in &mut self.gpio {
            g.reset();
        }
        self.opcode_encryption = self.submapper >= 12;
        self.dma_middle_addr = 0;
        self.dma_length = 0x100;
        self.dma_target = 0x2004;
        self.update_vt369_offsets();
        self.update_sound_prescaler();
    }

    pub fn update_sound_prescaler(&mut self) {
        if (self.reg4100[0x1F] & 1) != 0 {
            self.sound_prescaler = if (self.reg4100[0x1C] & 0x80) != 0 { 1 } else { 3 };
        } else if (self.reg4100[0x1C] & 0x80) != 0 {
            self.sound_prescaler = 2;
        } else {
            self.sound_prescaler = 6;
        }
        self.sound_prescaler_hle = 6;
    }

    pub fn run_sound_hle(&mut self) {
        if !self.console_type_vt369 || self.reg4100[0x62] != 0x0D {
            return;
        }
        self.sound_ram[0x1FF5] = 0x01;

        let reset_vector = (self.sound_ram[0x1FF8] as u16) | ((self.sound_ram[0x1FF9] as u16) << 8);
        if reset_vector == 0x0293 {
            match self.sound_ram[0x1FA2] {
                0x01 => {
                    let ch = (self.sound_ram[0x1FA3] as usize).min(2);
                    self.sound_ram[0x1CB0 + ch] = 0xFF;
                    self.sound_ram[0x1CB7 + ch * 2] = 0x00;
                    self.sound_ram[0x1980 + ch * 2] = 0x00;
                    self.sound_ram[0x1981 + ch * 2] = 0x00;
                    self.sound_ram[0x1FA2] = 0x00;
                    self.sound_adpcm_frame_count[ch] = 0;
                }
                0x02 => {
                    let period = (self.sound_ram[0x1FA0] as i16) | ((self.sound_ram[0x1FA1] as i16) << 8);
                    self.sound_timer_period = period.wrapping_mul(3);
                    self.sound_ram[0x1FA2] = 0x00;
                }
                0x03 => {
                    let ch = (self.sound_ram[0x1FA3] as usize).min(2);
                    self.sound_ram[0x1CB0 + ch] = 0x00;
                    self.sound_ram[0x1FA2] = 0x00;
                }
                _ => {}
            }
            self.sound_timer_count = self.sound_timer_count.wrapping_sub(self.sound_prescaler_hle as i16);
            let mut guard = 0;
            while self.sound_timer_period != 0 && self.sound_timer_count <= self.sound_timer_period && guard < 100 {
                guard += 1;
                self.sound_timer_count = self.sound_timer_count.wrapping_sub(self.sound_timer_period);
                let mut adpcm_output: i32 = 0;
                for ch in 0..3 {
                    if self.sound_ram[0x1CB0 + ch] == 0 {
                        continue;
                    }
                    if self.sound_adpcm_frame_count[ch] == 0 {
                        self.sound_adpcm_frame[ch] = 0;
                        let offset = self.sound_ram[0x1CB7 + ch * 2] as usize;
                        let read_addr = 0x1800 + ch * 0x80 + offset;
                        for i in 0..8 {
                            if read_addr + i < self.sound_ram.len() {
                                self.sound_adpcm_frame[ch] |= (self.sound_ram[read_addr + i] as u64) << (i * 8);
                            }
                        }
                        if (self.sound_adpcm_frame[ch] & 0x8000_0000_0000_0000) != 0 {
                            self.sound_adpcm_frame[ch] = 0;
                        }
                        self.sound_adpcm_frame_count[ch] = 21;
                        self.sound_ram[0x1CB7 + ch * 2] = (self.sound_ram[0x1CB7 + ch * 2] + 8) & 0x7F;
                    }
                    let mut out_byte = self.sound_ram[0x1980 + ch * 2];
                    let mut idx_byte = self.sound_ram[0x1981 + ch * 2];
                    vt1682_decode_adpcm(
                        (self.sound_adpcm_frame[ch] & 7) as u8,
                        &mut out_byte,
                        &mut idx_byte,
                    );
                    self.sound_ram[0x1980 + ch * 2] = out_byte;
                    self.sound_ram[0x1981 + ch * 2] = idx_byte;
                    let predictor = out_byte as i8;
                    adpcm_output += (predictor as i32) << 7;

                    self.sound_adpcm_frame[ch] >>= 3;
                    self.sound_adpcm_frame_count[ch] = self.sound_adpcm_frame_count[ch].saturating_sub(1);
                }
                self.sound_dac = adpcm_output.clamp(-32768, 32767) as i16;
            }
        } else {
            let (adr_masks, adr_period, adr_loop, adr_loop_inc, stream) = match reset_vector {
                0x0203 | 0x02A0 => (0x184Cusize, 0x183Busize, 0x18FCusize, 1usize, false),
                0x02E0 => (0x184C, 0x183B, 0x1873, 4, false),
                0x0250 => (0x184C, 0x183B, 0x18FC, 1, true),
                0x1C4C | 0x40AE => (0x18A4, 0x186B, 0, 1, false),
                _ => (0x184C, 0x183B, 0x18FC, 1, false),
            };

            let mut timer_period = (self.sound_ram[adr_period] as i16) | ((self.sound_ram[adr_period + 4] as i16) << 8);
            if timer_period == (!0xE7i16) {
                timer_period = timer_period * 5 / 2;
            }
            self.sound_timer_period = timer_period;
            self.sound_timer_count = self.sound_timer_count.wrapping_sub(self.sound_prescaler_hle as i16);

            let mut guard = 0;
            while self.sound_timer_period != 0 && self.sound_timer_count <= self.sound_timer_period && guard < 100 {
                guard += 1;
                self.sound_timer_count = self.sound_timer_count.wrapping_sub(self.sound_timer_period);
                self.sound_ram[0x18F6] = self.sound_ram[0x18F6].wrapping_add(1);
                self.sound_ram[adr_masks + 1] &= !self.sound_ram[adr_masks + 2];

                let mut adpcm_output: i32 = 0;
                for ch in 0..4 {
                    if (self.sound_ram[adr_masks] & (1 << ch)) != 0 {
                        self.sound_ram[0x1830 + ch * 4] = self.sound_ram[0x1860 + ch * 4];
                        self.sound_ram[0x1831 + ch * 4] = self.sound_ram[0x1861 + ch * 4];
                        self.sound_ram[0x1832 + ch * 4] = self.sound_ram[0x1862 + ch * 4];
                    }
                    if (self.sound_ram[adr_masks + 1] & (1 << ch)) != 0 {
                        let mut wave_addr: u32 = if stream {
                            self.sound_ram[0x1D21 + ch * 2] as u32
                        } else {
                            (self.sound_ram[0x1830 + ch * 4] as u32)
                                | ((self.sound_ram[0x1831 + ch * 4] as u32) << 8)
                                | ((self.sound_ram[0x1832 + ch * 4] as u32) << 16)
                        };
                        let prg_len = self.prg_rom.len().max(1);
                        if self.sound_ram[0x1805 + ch * 8] == 48 {
                            if stream {
                                self.sound_ram[0x1800 + ch * 8] = self.sound_ram[0x1900 + ch * 0x100 + ((wave_addr & 0xFF) as usize)];
                                wave_addr += 1;
                                self.sound_ram[0x1801 + ch * 8] = self.sound_ram[0x1900 + ch * 0x100 + ((wave_addr & 0xFF) as usize)];
                                wave_addr += 1;
                            } else {
                                self.sound_ram[0x1800 + ch * 8] = self.prg_rom.get(((wave_addr as usize) + self.vt369_relative) % prg_len).copied().unwrap_or(0);
                                wave_addr += 1;
                                self.sound_ram[0x1801 + ch * 8] = self.prg_rom.get(((wave_addr as usize) + self.vt369_relative) % prg_len).copied().unwrap_or(0);
                                wave_addr += 1;
                            }
                            if self.sound_ram[0x1800 + ch * 8] == 0xFF {
                                if adr_loop != 0 && self.sound_ram[adr_loop + ch * adr_loop_inc] != 0 {
                                    self.sound_ram[0x1830 + ch * 4] = self.sound_ram[0x1860 + ch * 4];
                                    self.sound_ram[0x1831 + ch * 4] = self.sound_ram[0x1861 + ch * 4];
                                    self.sound_ram[0x1832 + ch * 4] = self.sound_ram[0x1862 + ch * 4];
                                    self.sound_ram[0x1803 + ch * 8] = 0;
                                    self.sound_ram[0x1804 + ch * 8] = 0;
                                    self.sound_ram[0x1805 + ch * 8] = 48;
                                } else {
                                    self.sound_ram[adr_masks + 1] &= !(1 << ch);
                                }
                                continue;
                            }
                        } else if (self.sound_ram[0x1805 + ch * 8] & 1) == 0 {
                            if stream {
                                self.sound_ram[0x1801 + ch * 8] = self.sound_ram[0x1900 + ch * 0x100 + ((wave_addr & 0xFF) as usize)];
                                wave_addr += 1;
                            } else {
                                self.sound_ram[0x1801 + ch * 8] = self.prg_rom.get(((wave_addr as usize) + self.vt369_relative) % prg_len).copied().unwrap_or(0);
                                wave_addr += 1;
                            }
                        }
                        if stream {
                            self.sound_ram[0x1D21 + ch * 2] = (wave_addr & 0xFF) as u8;
                        } else {
                            self.sound_ram[0x1830 + ch * 4] = (wave_addr & 0xFF) as u8;
                            self.sound_ram[0x1831 + ch * 4] = ((wave_addr >> 8) & 0xFF) as u8;
                            self.sound_ram[0x1832 + ch * 4] = ((wave_addr >> 16) & 0xFF) as u8;
                        }
                        adpcm_output += vt369_decode_adpcm(&mut self.sound_ram[0x1800 + ch * 8..0x1800 + ch * 8 + 6]);
                    }
                }
                self.sound_ram[adr_masks + 1] |= self.sound_ram[adr_masks];
                self.sound_dac = adpcm_output.clamp(-32768, 32767) as i16;
            }
        }
    }

    pub fn audio_sample(&self) -> f32 {
        if self.console_type_vt369 {
            (self.sound_dac as f32) / 32767.0
        } else {
            0.0
        }
    }

    pub fn update_vt369_offsets(&mut self) {
        if !self.console_type_vt369 {
            return;
        }
        let rel = self.relative_8k & 0xFFF;
        let prg_and = self.banking.prg_and as usize;
        let prg_or = self.banking.prg_or as usize;
        self.vt369_relative = (rel & prg_and | prg_or) << 13;

        let chr_and3 = self.banking.chr_and >> 3;
        let chr_or3 = self.banking.chr_or >> 3;
        let bg_reg =
            (self.reg2000[0x20] as usize) | ((self.reg2000[0x21] as usize) << 8);
        self.vt369_bg_data = (((bg_reg + self.relative_8k) & 0xFFF & chr_and3) | chr_or3) << 13;
        let spr_reg =
            (self.reg2000[0x22] as usize) | ((self.reg2000[0x23] as usize) << 8);
        self.vt369_spr_data = (((spr_reg + self.relative_8k) & 0xFFF & chr_and3) | chr_or3) << 13;
    }

    pub fn ppu_reg_mask_bit6(&self) -> bool {
        self.console_type_vt03 || self.console_type_vt09
    }

    pub fn unscramble_opcode(&self, opcode: u8) -> u8 {
        if !self.opcode_encryption {
            return opcode;
        }
        match self.submapper {
            12 => {
                let mut r = opcode & !0xC6;
                r |= if opcode & 0x40 != 0 { 0x80 } else { 0 };
                r |= if opcode & 0x80 != 0 { 0x40 } else { 0 };
                r |= if opcode & 0x02 != 0 { 0x04 } else { 0 };
                r |= if opcode & 0x04 != 0 { 0x02 } else { 0 };
                r
            }
            13 => {
                let mut r = opcode & !0x12;
                r |= if opcode & 0x10 != 0 { 0x02 } else { 0 };
                r |= if opcode & 0x02 != 0 { 0x10 } else { 0 };
                r
            }
            14 => {
                let mut r = opcode & !0xC0;
                r |= if opcode & 0x80 != 0 { 0x40 } else { 0 };
                r |= if opcode & 0x40 != 0 { 0x80 } else { 0 };
                r
            }
            _ => {
                let mut r = opcode & !0x60;
                r |= if opcode & 0x40 != 0 { 0x20 } else { 0 };
                r |= if opcode & 0x20 != 0 { 0x40 } else { 0 };
                r
            }
        }
    }

    pub fn ps(&self) -> u8 {
        self.reg4100[0x0B] & 7
    }
    pub fn fwen(&self) -> bool {
        (self.reg4100[0x0B] & 0x08) != 0
    }
    pub fn comr6(&self) -> bool {
        (self.reg4100[0x05] & 0x40) != 0
    }
    pub fn comr7(&self) -> bool {
        (self.reg4100[0x05] & 0x80) != 0
    }
    pub fn pq2en(&self) -> bool {
        (self.reg4100[0x0B] & 0x40) != 0
    }
    pub fn tsynen(&self) -> bool {
        (self.reg4100[0x0B] & 0x80) != 0
    }
    pub fn bk16en(&self) -> bool {
        (self.reg2000[0x10] & 0x02) != 0
    }
    pub fn use_4bpp_chr(&self) -> bool {
        let flags = self.reg2000[0x10];
        (flags & 0x86) != 0
    }
    pub fn hv(&self) -> u8 {
        self.reg4100[0x06] & 1
    }
    pub fn mirror_nametable_address(&self, address: u16) -> u16 {
        if (self.reg4100[0x06] & 2) != 0 {
            address & 0x23FF
        } else if (self.reg4100[0x06] & 1) != 0 {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }
    pub fn get_prg_bank(&self, mut slot: usize) -> usize {
        let ps = self.ps();
        let prg_and = if ps == 7 { 0xFFu16 } else { 0x3Fu16 >> ps };
        let pa21 = (self.reg4100[0x00] >> 4) as u16;
        let prg_or = ((self.reg4100[0x0A] as u16) | (pa21 << 8)) & !prg_and;
        let flip = if self.comr6() { 2 } else { 0 };
        if slot & 1 == 0 {
            slot ^= flip;
        }
        let pq = match slot & 3 {
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
        let bank = ((pq & prg_and | prg_or) as usize) + self.relative_8k;
        ((bank as u16 & self.banking.prg_and) | self.banking.prg_or) as usize
    }
    #[allow(dead_code)]
    pub fn get_prg16_bank(&self, bank0: usize, slot: usize) -> usize {
        let ps = self.ps();
        let prg_and = if ps == 7 { 0xFFu16 } else { 0x3Fu16 >> ps };
        let pa21 = (self.reg4100[0x00] >> 4) as u16;
        let prg_or = ((self.reg4100[0x0A] as u16) | (pa21 << 8)) & !prg_and;
        let sub = (bank0 << 1 | (slot & 1)) as u16;
        let bank = (sub & prg_and | prg_or) as usize;
        ((bank as u16 & self.banking.prg_and) | self.banking.prg_or) as usize
    }
    pub fn vrwb(&self) -> u8 {
        self.reg2000[0x18] & 0x07
    }
    pub fn bkpage(&self) -> bool {
        (self.reg2000[0x18] & 0x08) != 0
    }
    pub fn bkexten(&self) -> bool {
        (self.reg2000[0x10] & 0x10) != 0
    }
    pub fn spexten(&self) -> bool {
        (self.reg2000[0x10] & 0x08) != 0
    }
    pub fn chr_bank_1k_ext(&self, slot: usize, is_bg: bool, is_sprite: bool) -> usize {
        let vb0s = (self.reg2000[0x1A] & 7) as usize;
        let shift = VB0S_TABLE[vb0s] as u16;
        let chr_and = 0xFFu16 >> shift;
        let rv6 = (self.reg2000[0x1A] & 0xF8) as u16;
        let chr_or = rv6 & !chr_and;
        let extended = if is_bg {
            self.bkexten()
        } else if is_sprite {
            self.spexten()
        } else {
            self.bkexten() || self.spexten()
        };
        let is_4bpp = if is_bg {
            self.use_4bpp_chr() && self.bk16en()
        } else if is_sprite {
            (self.reg2000[0x10] & 0x04) != 0
        } else {
            self.use_4bpp_chr()
        };
        let bank_reg = match slot & 7 {
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
        let va21 = (self.reg4100[0x00] & 0x0F) as u16;
        let mut map_and = self.banking.chr_and;
        let mut map_or = self.banking.chr_or;
        let mut rel = self.relative_8k << 3;

        if is_4bpp {
            map_and >>= 1;
            map_or >>= 1;
            rel >>= 1;
        }

        let bank_val = if extended {
            let eva = if is_bg {
                if self.bkpage() { 4 } else { 0 }
            } else if is_sprite {
                0
            } else {
                self.vrwb() as usize
            };
            let raw_bank = ((((bank_reg as u16 & chr_and) | chr_or) as usize) << 3) | eva | ((va21 as usize) << 11);
            ((raw_bank & map_and) | map_or) + rel
        } else {
            let va18 = ((self.reg2000[0x18] >> 4) & 7) as u16;
            let raw_or = chr_or | (va18 << 8);
            let raw_bank = ((bank_reg as u16 & chr_and) | raw_or | (va21 << 11)) as usize;
            ((raw_bank & map_and) | map_or) + rel
        };
        bank_val
    }
    pub fn chr_bank_1k(&self, slot: usize) -> usize {
        self.chr_bank_1k_ext(slot, false, false)
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
        self.chr_low = vec![0u8; chr_size >> 1];
        self.chr_high = vec![0u8; chr_size >> 1];
        self.chr_low16 = vec![0u8; chr_size >> 1];
        self.chr_high16 = vec![0u8; chr_size >> 1];
        self.chr_mask = chr_size.saturating_sub(1);
        for i in 0..chr_size.min(raw_chr.len()) {
            let shifted = (i & 0xF) | ((i >> 1) & !0xF);
            if i & 0x10 != 0 {
                if shifted < self.chr_high.len() {
                    self.chr_high[shifted] = raw_chr[i];
                }
            } else if shifted < self.chr_low.len() {
                self.chr_low[shifted] = raw_chr[i];
            }
            if i & 1 != 0 {
                if (i >> 1) < self.chr_high16.len() {
                    self.chr_high16[i >> 1] = raw_chr[i];
                }
            } else if (i >> 1) < self.chr_low16.len() {
                self.chr_low16[i >> 1] = raw_chr[i];
            }
        }
        self.chr_source_len = raw_chr.len();
    }
    pub fn update_chr_plane_byte(&mut self, rom_addr: usize, val: u8) {
        let shifted = (rom_addr & 0xF) | ((rom_addr >> 1) & !0xF);
        if rom_addr & 0x10 != 0 {
            if shifted < self.chr_high.len() {
                self.chr_high[shifted] = val;
            }
        } else if shifted < self.chr_low.len() {
            self.chr_low[shifted] = val;
        }
        let half = rom_addr >> 1;
        if rom_addr & 1 != 0 {
            if half < self.chr_high16.len() {
                self.chr_high16[half] = val;
            }
        } else if half < self.chr_low16.len() {
            self.chr_low16[half] = val;
        }
    }

    pub fn fetch_chr_byte_ext(
        &mut self,
        prg_rom: &[u8],
        chr_rom: &[u8],
        chr_ram: &[u8],
        address: u16,
        chr_ram_flat: bool,
        is_bg: bool,
        is_sprite: bool,
        chr_eva: u16,
    ) -> u8 {
        self.ensure_chr_planes(prg_rom, chr_rom);
        let in_low_plane  = address < 0x2000;
        let in_high_plane = address >= 0x4000 && address < 0x6000;
        let in_bg_low     = address >= 0x8000 && address < 0xA000;
        let in_spr_low    = address >= 0xA000 && address < 0xC000;
        let in_bg_high    = address >= 0xC000 && address < 0xE000;
        let in_spr_high   = address >= 0xE000;
        if !in_low_plane && !in_high_plane && !in_bg_low && !in_spr_low && !in_bg_high && !in_spr_high {
            return 0;
        }
        if chr_ram_flat && !chr_ram.is_empty() {
            return chr_ram[(address as usize) & 0x1FFF];
        }
        let use_4bpp = self.use_4bpp_chr();
        let flip = if self.comr7() { 4 } else { 0 };
        let slot_base_addr = if in_low_plane || in_high_plane {
            address as usize & 0x1FFF
        } else if in_bg_low || in_spr_low {
            address as usize & 0x1FFF
        } else {
            address as usize & 0x1FFF
        };
        let slot = ((slot_base_addr >> 10) & 7) ^ flip;
        let (eff_is_bg, eff_is_sprite) = if in_low_plane || in_high_plane {
            (is_bg, is_sprite)
        } else if in_bg_low || in_bg_high {
            (true, false)
        } else {
            (false, true)
        };
        let fetch_high_plane = in_high_plane || in_bg_high || in_spr_high;
        let is_4bpp_window = if in_bg_low || in_bg_high {
            use_4bpp && self.bk16en()
        } else if in_spr_low || in_spr_high {
            (self.reg2000[0x10] & 0x04) != 0
        } else {
            use_4bpp
        };
        let use_16bit_planes = self.console_type_vt369
            || (self.reg2000[0x10] & 0x40) != 0
            || self.reg4100[0x2B] == 0x61;
        if is_4bpp_window {
            let bank_full = self.chr_bank_1k_ext(slot, eff_is_bg, eff_is_sprite);
            let within_1k = (address as usize & 0x3FF) | (chr_eva as usize);
            let plane = if fetch_high_plane {
                if use_16bit_planes {
                    &self.chr_high16
                } else {
                    &self.chr_high
                }
            } else if use_16bit_planes {
                &self.chr_low16
            } else {
                &self.chr_low
            };
            if !plane.is_empty() {
                let offset = bank_full * 0x400 + within_1k;
                rom_read(plane, offset)
            } else {
                0
            }
        } else {
            let bank = self.chr_bank_1k_ext(slot, eff_is_bg, eff_is_sprite);
            let offset = bank * 0x400 + ((address as usize & 0x3FF) | (chr_eva as usize));
            let raw = if !chr_rom.is_empty() { chr_rom } else { prg_rom };
            rom_read(raw, offset)
        }
    }
    pub fn fetch_chr_byte(
        &mut self,
        prg_rom: &[u8],
        chr_rom: &[u8],
        chr_ram: &[u8],
        address: u16,
        chr_ram_flat: bool,
    ) -> u8 {
        self.fetch_chr_byte_ext(prg_rom, chr_rom, chr_ram, address, chr_ram_flat, false, false, 0)
    }
    pub fn read_apu(&mut self, address: u16) -> Option<u8> {
        if address >= 0x4800 && address <= 0x4FFF {
            if self.console_type_vt369 {
                return Some(self.sound_ram[((address & 0x7FF) | 0x1800) as usize]);
            }
        }
        let idx = (address & 0xFF) as usize;
        if address >= 0x4020 && address < 0x4040 && idx == 0x35 {
            return Some(0);
        }
        if address >= 0x4100 && address < 0x4200 {
            if self.console_type_vt369 {
                match idx {
                    0x30 | 0x38 => return Some((self.alu_operand14 & 0xFF) as u8),
                    0x31 | 0x39 => return Some(((self.alu_operand14 >> 8) & 0xFF) as u8),
                    0x32 | 0x3A => return Some(((self.alu_operand14 >> 16) & 0xFF) as u8),
                    0x33 | 0x3B => return Some(((self.alu_operand14 >> 24) & 0xFF) as u8),
                    0x34 | 0x3C => return Some((self.alu_operand56 & 0xFF) as u8),
                    0x35 | 0x3D => return Some(((self.alu_operand56 >> 8) & 0xFF) as u8),
                    0x36 | 0x3E => return Some(self.alu_busy),
                    _ => {}
                }
            }
            match idx {
                0x40..=0x5B | 0x5D..=0x5F => {
                    if self.console_type_vt369 {
                        let port_idx = (idx >> 3) & 3;
                        let sub_addr = (idx & 7) as u8;
                        return Some(self.gpio[port_idx].read(sub_addr));
                    }
                    return Some(0xFF);
                }
                0x5C => return Some(0x10),
                0x8A => return Some(0x04),
                0x99 => return Some(0x02),
                0xB7 => return Some(0x04),
                0xB9 => return Some(0x80),
                _ => {}
            }
            if (0x00..=0x0D).contains(&idx) || (0x60..=0xFF).contains(&idx) {
                return Some(self.reg4100[idx]);
            }
            if address >= 0x4200 && address < 0x4800 {
                return Some(self.reg4100[idx]);
            }
        } else if address == 0x4326 {
            return Some(0x01);
        }
        None
    }
    pub fn write_ppu(&mut self, addr: u16, val: u8, mangle: &OneBusMangle) {
        self.reg2000[(addr & 0xFF) as usize] = val;

        let mut a = (addr & 0xFF) as u8;
        if (0x12..=0x17).contains(&a) {
            a = 0x12 + mangle.ppu[(a - 0x12) as usize];
        }
        if self.ppu_reg_mask_bit6() && a >= 8 {
            a &= !0x40;
        }
        if a >= 8 {
            self.reg2000[a as usize] = val;
        }
        if self.console_type_vt369 && (0x20..=0x23).contains(&(addr & 0xFF)) {
            self.update_vt369_offsets();
        }
    }
    pub fn write_apu(&mut self, addr: u16, val: u8, mangle: &OneBusMangle) {
        if addr >= 0x4800 && addr <= 0x4FFF {
            if self.console_type_vt369 {
                self.sound_ram[((addr & 0x7FF) | 0x1800) as usize] = val;
                return;
            }
        }
        let mut idx = (addr & 0xFF) as usize;
        if (0x07..=0x0A).contains(&idx) {
            idx = 0x07 + mangle.cpu[(idx - 0x07) as usize] as usize;
        }
        if idx == 0x1C && (self.submapper == 12 || self.submapper == 14) {
            self.opcode_encryption = (val & 0x40) != 0;
        }
        if idx == 0x1C || idx == 0x1F {
            self.update_sound_prescaler();
        }
        if idx == 0x2D {
            self.dma_middle_addr = 0;
        }
        if idx == 0x69 && (self.submapper == 13 || self.submapper == 15) {
            self.opcode_encryption = (val & 1) == 0;
        }
        if idx == 0x34 {
            let mut shift = (val >> 1) & 7;
            if shift == 0 {
                shift = 8;
            }
            self.dma_middle_addr = val & 0xF0;
            self.dma_length = 1u16 << shift;
            self.dma_target = if val & 1 != 0 { 0x2007 } else { 0x2004 };
        }
        if (0x40..=0x5F).contains(&idx) && self.console_type_vt369 {
            let port_idx = (idx >> 3) & 3;
            let sub_addr = (idx & 7) as u8;
            self.gpio[port_idx].write(sub_addr, val);
        }
        if self.console_type_vt369 && (0x30..=0x37).contains(&idx) {
            match idx {
                0x30 => self.alu_operand14 = (self.alu_operand14 & 0xFFFFFF00) | (val as u32),
                0x31 => self.alu_operand14 = (self.alu_operand14 & 0xFFFF00FF) | ((val as u32) << 8),
                0x32 => self.alu_operand14 = (self.alu_operand14 & 0xFF00FFFF) | ((val as u32) << 16),
                0x33 => self.alu_operand14 = (self.alu_operand14 & 0x00FFFFFF) | ((val as u32) << 24),
                0x34 => self.alu_operand56 = (self.alu_operand56 & 0xFF00) | (val as u16),
                0x35 => {
                    self.alu_operand56 = (self.alu_operand56 & 0x00FF) | ((val as u16) << 8);
                    let op1 = (self.alu_operand14 & 0xFFFF) as u64;
                    let op2 = self.alu_operand56 as u64;
                    self.alu_operand14 = (op1 * op2) as u32;
                    self.alu_busy = 16;
                }
                0x36 => self.alu_operand67 = (self.alu_operand67 & 0xFF00) | (val as u16),
                0x37 => {
                    self.alu_operand67 = (self.alu_operand67 & 0x00FF) | ((val as u16) << 8);
                    if self.alu_operand67 != 0 {
                        let num = self.alu_operand14;
                        let den = self.alu_operand67 as u32;
                        self.alu_operand56 = (num % den) as u16;
                        self.alu_operand14 = num / den;
                        self.alu_busy = 32;
                    }
                }
                _ => {}
            }
        }
        match idx {
            0x01 => self.irq_reload = val,
            0x02 => self.irq_counter = 0,
            0x03 => {
                self.irq_enabled = false;
                self.pending_irq = false;
                self.irq_ack = true;
            }
            0x04 => self.irq_enabled = true,
            0x60 | 0x61 => {
                if self.console_type_vt369 {
                    self.reg4100[idx] = val;
                    self.relative_8k = (self.reg4100[0x60] as usize)
                        | (((self.reg4100[0x61] as usize) << 8) & 0xF00);
                    self.update_vt369_offsets();
                    return;
                }
            }
            0x62 => {
                if self.console_type_vt369 && val == 0x0D {
                    self.sound_ram[0x1FF5] = 0x01;
                }
            }
            _ => {}
        }
        self.reg4100[idx] = val;
    }
    pub fn write_mmc3(&mut self, address: u16, val: u8, _mangle: &OneBusMangle) {
        if self.fwen() {
            return;
        }
        let bank_bits = ((address >> 12) & 6) as u8;
        let addr_bit_0 = (address & 1) as u8;
        let mmc3_addr = bank_bits | addr_bit_0;
        let identity = OneBusMangle::IDENTITY;
        match mmc3_addr {
            0 => self.write_apu(0x4105, val & !0x20, &identity),
            1 => {
                let pointer = self.reg4100[0x05] & 7;
                if pointer < 2 {
                    self.write_ppu(0x2016 + pointer as u16, val, &identity);
                } else if pointer < 6 {
                    self.write_ppu(0x2010 + pointer as u16, val, &identity);
                } else {
                    self.write_apu(0x4101 + pointer as u16, val, &identity);
                }
            }
            2 => self.write_apu(0x4106, val & 1, &identity),
            4 => self.write_apu(0x4101, val, &identity),
            5 => self.write_apu(0x4102, val, &identity),
            6 => self.write_apu(0x4103, val, &identity),
            7 => self.write_apu(0x4104, val, &identity),
            _ => {}
        }
    }
    pub fn store_prg_mmc3(&mut self, address: u16, data: u8, mangle: &OneBusMangle) {
        let val = if (address <= 0x9FFF) && ((address & 1) == 0) {
            data & 0xF8 | mangle.mmc3[(data & 0x07) as usize]
        } else {
            data
        };
        self.write_mmc3(address, val, mangle);
    }
    fn clock_scanline_counter(&mut self, rendering: bool) {
        if self.irq_counter == 0 {
            self.irq_counter = self.irq_reload;
        } else {
            self.irq_counter = self.irq_counter.wrapping_sub(1);
        }
        if self.irq_counter == 0 && self.irq_enabled && rendering {
            if self.console_type_vt369 && (self.reg4100[0x1C] & 0x20) != 0 {
                self.irq_delay = 24;
            } else {
                self.pending_irq = true;
            }
        }
    }
    pub fn ppu_cycle(
        &mut self,
        addr: u16,
        scanline: u16,
        dot: u16,
        rendering: bool,
    ) -> bool {
        let is_prerender_or_zero = scanline == 0 || scanline == 261;
        if self.console_type_vt369 && (self.reg4100[0x1C] & 0x80) != 0 && is_prerender_or_zero {
            return false;
        }
        if self.console_type_vt369 && (self.reg4100[0x1C] & 0x20) != 0 && scanline == 0 {
            return false;
        }
        let target_cycle = if self.console_type_vt369 { 240 } else { 256 };
        if self.tsynen() || self.bk16en() {
            if scanline < 242 && dot == target_cycle {
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
        if self.alu_busy > 0 {
            self.alu_busy = self.alu_busy.wrapping_sub(1);
        }
        if self.pa12_filter > 0 {
            self.pa12_filter = self.pa12_filter.wrapping_sub(1);
        }
        if self.console_type_vt369 && self.reg4100[0x62] == 0x0D {
            self.run_sound_hle();
        }
        if self.irq_delay > 0 {
            self.irq_delay = self.irq_delay.wrapping_sub(1);
            if self.irq_delay == 0 {
                return true;
            }
        }
        false
    }
    pub fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack;
        self.irq_ack = false;
        ack
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
        state.push(if self.opcode_encryption { 1 } else { 0 });
        state.push(self.dma_middle_addr);
        state.extend_from_slice(&self.dma_length.to_le_bytes());
        state.extend_from_slice(&self.dma_target.to_le_bytes());
        if self.console_type_vt369 {
            state.extend_from_slice(&self.sound_ram);
            state.extend_from_slice(&self.sound_dac.to_le_bytes());
            state.extend_from_slice(&self.sound_timer_period.to_le_bytes());
            state.extend_from_slice(&self.sound_timer_count.to_le_bytes());
        }
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
        if p < state.len() {
            self.opcode_encryption = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.dma_middle_addr = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.dma_length = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p + 2 <= state.len() {
            self.dma_target = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if self.console_type_vt369 {
            if p + 8192 <= state.len() {
                self.sound_ram.copy_from_slice(&state[p..p + 8192]);
                p += 8192;
            }
            if p + 2 <= state.len() {
                self.sound_dac = i16::from_le_bytes([state[p], state[p + 1]]);
                p += 2;
            }
            if p + 2 <= state.len() {
                self.sound_timer_period = i16::from_le_bytes([state[p], state[p + 1]]);
                p += 2;
            }
            if p + 2 <= state.len() {
                self.sound_timer_count = i16::from_le_bytes([state[p], state[p + 1]]);
                p += 2;
            }
            self.relative_8k = (self.reg4100[0x60] as usize)
                | (((self.reg4100[0x61] as usize) << 8) & 0xF00);
            self.update_vt369_offsets();
            self.update_sound_prescaler();
        }
        p
    }
}
