use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const VRC7_PATCHES: [[u8; 8]; 16] = [
    [0; 8],
    [0x03, 0x21, 0x05, 0x06, 0xE8, 0x81, 0x42, 0x27],
    [0x13, 0x41, 0x14, 0x0D, 0xD8, 0xF6, 0x23, 0x12],
    [0x11, 0x11, 0x08, 0x08, 0xFA, 0xB2, 0x20, 0x12],
    [0x31, 0x61, 0x0C, 0x07, 0xA8, 0x64, 0x61, 0x27],
    [0x32, 0x21, 0x1E, 0x06, 0xE1, 0x76, 0x01, 0x28],
    [0x02, 0x01, 0x06, 0x00, 0xA3, 0xE2, 0xF4, 0xF4],
    [0x21, 0x61, 0x1D, 0x07, 0x82, 0x81, 0x11, 0x07],
    [0x23, 0x21, 0x22, 0x17, 0xA2, 0x72, 0x01, 0x17],
    [0x35, 0x11, 0x25, 0x00, 0x40, 0x73, 0x72, 0x01],
    [0xB5, 0x01, 0x0F, 0x0F, 0xA8, 0xA5, 0x51, 0x02],
    [0x17, 0xC1, 0x24, 0x07, 0xF8, 0xF8, 0x22, 0x12],
    [0x71, 0x23, 0x11, 0x06, 0x65, 0x74, 0x18, 0x16],
    [0x01, 0x02, 0xD3, 0x05, 0xC9, 0x95, 0x03, 0x02],
    [0x61, 0x63, 0x0C, 0x00, 0x94, 0xC0, 0x33, 0xF6],
    [0x21, 0x72, 0x0D, 0x00, 0xC1, 0xD5, 0x56, 0x06],
];
const MULT_LUT: [f32; 16] = [
    0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 10.0, 12.0, 12.0, 15.0, 15.0,
];

#[derive(Clone, Copy, PartialEq, Debug)]
enum EnvState {
    Attack,
    Decay,
    SustainHold,
    SustainDecay,
    Release,
    Idle,
}

#[derive(Clone, Debug)]
struct Operator {
    phase: f32,
    env_state: EnvState,
    env_level: f32, 
    prev_out1: f32,
    prev_out2: f32,
}

impl Operator {
    fn new() -> Self {
        Self {
            phase: 0.0,
            env_state: EnvState::Idle,
            env_level: 255.0,
            prev_out1: 0.0,
            prev_out2: 0.0,
        }
    }

    fn key_on(&mut self) {
        self.env_state = EnvState::Attack;
        self.phase = 0.0;
    }

    fn key_off(&mut self) {
        if self.env_state != EnvState::Idle {
            self.env_state = EnvState::Release;
        }
    }
}

#[derive(Clone, Debug)]
struct Channel {
    fnum: u16,
    block: u8,
    volume: u8,
    sustain: bool,
    patch_idx: u8,
    key_on: bool,
    mod_op: Operator,
    car_op: Operator,
}

impl Channel {
    fn new() -> Self {
        Self {
            fnum: 0,
            block: 0,
            volume: 0,
            sustain: false,
            patch_idx: 0,
            key_on: false,
            mod_op: Operator::new(),
            car_op: Operator::new(),
        }
    }
}

pub struct Vrc7 {
    a0_mask: u16,
    a1_mask: u16,
    prg: [u8; 3],
    chr: [u8; 8],
    misc: u8,
    latch: u8,
    irq: u8,
    counter: u8,
    irq_latch: u8,
    irq_cycles: i32,
    reg_addr: u8,
    custom_patch: [u8; 8],
    channels: Vec<Channel>,
    audio_cycles: u32,
    cycles_per_audio_step: u32,
    lfo_am_phase: f32,
    lfo_pm_phase: f32,
    current_sample: f32,
    irq_ack: bool,
}

impl Vrc7 {
    pub fn new(submapper: u8) -> Self {
        let (a0, a1) = match submapper {
            1 => (0x08, 0x20), 
            2 => (0x10, 0x20), 
            _ => (0x18, 0x20), 
        };
        Self {
            a0_mask: a0,
            a1_mask: a1,
            prg: [0, 1, 0xFE],
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            misc: 0,
            latch: 0,
            irq: 0,
            counter: 0,
            irq_latch: 0,
            irq_cycles: 0,
            reg_addr: 0,
            custom_patch: [0; 8],
            channels: vec![Channel::new(); 6],
            audio_cycles: 0,
            cycles_per_audio_step: 36,
            lfo_am_phase: 0.0,
            lfo_pm_phase: 0.0,
            current_sample: 0.0,
            irq_ack: false,
        }
    }

    fn step_audio(&mut self) {
        self.lfo_am_phase = (self.lfo_am_phase + (2.0 * std::f32::consts::PI * 3.7 / 49716.0)) % (2.0 * std::f32::consts::PI);
        self.lfo_pm_phase = (self.lfo_pm_phase + (2.0 * std::f32::consts::PI * 6.4 / 49716.0)) % (2.0 * std::f32::consts::PI);
        let am_lfo = (1.0 + self.lfo_am_phase.sin()) * 0.5; 
        let pm_lfo = self.lfo_pm_phase.sin(); 
        let mut output_mix = 0.0;
        for chan in &mut self.channels {
            let patch = if chan.patch_idx == 0 {
                self.custom_patch
            } else {
                VRC7_PATCHES[chan.patch_idx as usize]
            };
            let am0 = (patch[0] & 0x80) != 0;
            let pm0 = (patch[0] & 0x40) != 0;
            let eg0 = (patch[0] & 0x20) != 0;
            let kr0 = (patch[0] & 0x10) != 0;
            let ml0 = MULT_LUT[(patch[0] & 0x0F) as usize];
            let kl0 = (patch[2] >> 6) & 3;
            let tl0 = patch[2] & 0x3F;
            let fb = patch[3] & 7;
            let wf0 = (patch[3] & 0x08) != 0;
            let am1 = (patch[1] & 0x80) != 0;
            let pm1 = (patch[1] & 0x40) != 0;
            let eg1 = (patch[1] & 0x20) != 0;
            let kr1 = (patch[1] & 0x10) != 0;
            let ml1 = MULT_LUT[(patch[1] & 0x0F) as usize];
            let kl1 = (patch[3] >> 6) & 3;
            let wf1 = (patch[3] & 0x10) != 0;
            let ar0 = patch[4] >> 4;
            let dr0 = patch[4] & 0x0F;
            let ar1 = patch[5] >> 4;
            let dr1 = patch[5] & 0x0F;
            let sl0 = patch[6] >> 4;
            let rr0 = patch[6] & 0x0F;
            let sl1 = patch[7] >> 4;
            let rr1 = patch[7] & 0x0F;
            let rks0 = if kr0 {
                chan.block * 2 + (chan.fnum >> 8) as u8
            } else {
                chan.block / 2
            };
            let rks1 = if kr1 {
                chan.block * 2 + (chan.fnum >> 8) as u8
            } else {
                chan.block / 2
            };
            let mut base_dphase = (chan.fnum as f32) * ((1 << chan.block) as f32) / 524288.0;
            if pm0 || pm1 {
                let vibrato = (2.0f32).powf(pm_lfo * 13.75 / 1200.0);
                base_dphase *= vibrato;
            }
            Self::step_eg(&mut chan.mod_op, ar0, dr0, sl0, rr0, rks0, eg0, chan.sustain);
            Self::step_eg(&mut chan.car_op, ar1, dr1, sl1, rr1, rks1, eg1, chan.sustain);
            chan.mod_op.phase = (chan.mod_op.phase + base_dphase * ml0) % 1.0;
            let fb_scale = if fb == 0 {
                0.0
            } else {
                (1 << (fb - 1)) as f32 / 32.0
            };
            let feedback = ((chan.mod_op.prev_out1 + chan.mod_op.prev_out2) / 2.0) * fb_scale;
            let mut mod_att = chan.mod_op.env_level;
            if am0 {
                mod_att += (1.0 - am_lfo) * (4.8 / 0.375);
            }
            if kl0 > 0 {
                mod_att += Self::get_ksl_attenuation(chan.fnum, chan.block, kl0);
            }
            mod_att += (tl0 as f32) * 2.0; 
            let mod_linear = if mod_att >= 960.0 {
                0.0
            } else {
                (10.0f32).powf(-mod_att * 0.375 / 20.0)
            };
            let mod_out = Self::sine(chan.mod_op.phase + feedback, wf0) * mod_linear;
            chan.mod_op.prev_out2 = chan.mod_op.prev_out1;
            chan.mod_op.prev_out1 = mod_out;
            chan.car_op.phase = (chan.car_op.phase + base_dphase * ml1) % 1.0;
            let mut car_att = chan.car_op.env_level;
            if am1 {
                car_att += (1.0 - am_lfo) * (4.8 / 0.375);
            }
            if kl1 > 0 {
                car_att += Self::get_ksl_attenuation(chan.fnum, chan.block, kl1);
            }
            car_att += (chan.volume as f32) * 8.0; 
            let car_linear = if car_att >= 960.0 {
                0.0
            } else {
                (10.0f32).powf(-car_att * 0.375 / 20.0)
            };
            let car_out = Self::sine(chan.car_op.phase + mod_out * 4.0, wf1) * car_linear;
            output_mix += car_out;
        }
        if (self.misc & 0x40) != 0 {
            self.current_sample = 0.0;
        } else {
            self.current_sample = output_mix * 3.0;
        }
    }

    fn sine(phase: f32, half_sine: bool) -> f32 {
        let val = (phase * 2.0 * std::f32::consts::PI).sin();
        if half_sine {
            if val > 0.0 { val } else { 0.0 }
        } else {
            val
        }
    }

    fn get_ksl_attenuation(fnum: u16, block: u8, kl: u8) -> f32 {
        let note = (block * 12) as f32 + ((fnum as f32) / 32.0);
        let att = (note - 60.0).max(0.0) * (kl as f32) * 1.5;
        att
    }

    fn step_eg(
        op: &mut Operator,
        ar: u8,
        dr: u8,
        sl: u8,
        rr: u8,
        rks: u8,
        eg: bool,
        sustain_held: bool,
    ) {
        match op.env_state {
            EnvState::Attack => {
                let inc = Self::get_env_increment(ar, rks);
                if inc >= 127.0 {
                    op.env_level = 0.0;
                    op.env_state = EnvState::Decay;
                } else {
                    op.env_level -= inc * 4.0; 
                    if op.env_level <= 0.0 {
                        op.env_level = 0.0;
                        op.env_state = EnvState::Decay;
                    }
                }
            }
            EnvState::Decay => {
                let inc = Self::get_env_increment(dr, rks);
                let target = (sl as f32) * 8.0;
                op.env_level += inc;
                if op.env_level >= target {
                    op.env_level = target;
                    if eg {
                        op.env_state = EnvState::SustainHold;
                    } else {
                        op.env_state = EnvState::SustainDecay;
                    }
                }
            }
            EnvState::SustainHold => {
            }
            EnvState::SustainDecay => {
                let inc = Self::get_env_increment(rr, rks);
                op.env_level += inc;
                if op.env_level >= 255.0 {
                    op.env_level = 255.0;
                    op.env_state = EnvState::Idle;
                }
            }
            EnvState::Release => {
                let r_rate = if sustain_held { 5 } else if eg { rr } else { 7 };
                let inc = Self::get_env_increment(r_rate, rks);
                op.env_level += inc;
                if op.env_level >= 255.0 {
                    op.env_level = 255.0;
                    op.env_state = EnvState::Idle;
                }
            }
            EnvState::Idle => {
                op.env_level = 255.0;
            }
        }
    }

    fn get_env_increment(rate: u8, rks: u8) -> f32 {
        if rate == 0 {
            return 0.0;
        }
        let eff_rate = (rate * 4 + rks).min(60);
        if eff_rate >= 60 {
            return 255.0;
        }
        0.375 * (2.0f32).powf((eff_rate as f32 - 48.0) / 4.0)
    }

    fn write_audio_reg(&mut self, reg: u8, data: u8) {
        if reg <= 0x07 {
            self.custom_patch[reg as usize] = data;
        } else if reg >= 0x10 && reg <= 0x15 {
            let c = (reg - 0x10) as usize;
            self.channels[c].fnum = (self.channels[c].fnum & 0xFF00) | (data as u16);
        } else if reg >= 0x20 && reg <= 0x25 {
            let c = (reg - 0x20) as usize;
            self.channels[c].fnum = (self.channels[c].fnum & 0x00FF) | (((data & 1) as u16) << 8);
            self.channels[c].block = (data >> 1) & 7;
            self.channels[c].sustain = (data & 0x20) != 0;
            let key_on = (data & 0x10) != 0;
            if key_on && !self.channels[c].key_on {
                self.channels[c].mod_op.key_on();
                self.channels[c].car_op.key_on();
            } else if !key_on && self.channels[c].key_on {
                self.channels[c].mod_op.key_off();
                self.channels[c].car_op.key_off();
            }
            self.channels[c].key_on = key_on;
        } else if reg >= 0x30 && reg <= 0x35 {
            let c = (reg - 0x30) as usize;
            self.channels[c].volume = data & 0x0F;
            self.channels[c].patch_idx = data >> 4;
        }
    }

    fn mirror_address(&self, address: u16) -> u16 {
        match self.misc & 3 {
            0 => address & 0x37FF, 
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1), 
            2 => address & 0x3FFF, 
            3 => (address & 0x3FFF) | 0x0400, 
            _ => address,
        }
    }
}

impl Mapper for Vrc7 {
    fn set_cpu_clock(&mut self, clock: f64) {
        self.cycles_per_audio_step = (clock / 49716.0).round() as u32;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let (bank, bank_size) = match address {
                0x8000..=0x9FFF => (self.prg[0] as usize, 0x2000), 
                0xA000..=0xBFFF => (self.prg[1] as usize, 0x2000), 
                0xC000..=0xDFFF => (self.prg[2] as usize, 0x2000), 
                0xE000..=0xFFFF => ((cart.prg_rom.len() / 0x2000 - 1) as usize, 0x2000), 
                _ => (0, 0x2000),
            };
            let offset = (bank * bank_size) + (address as usize & (bank_size - 1));
            let final_offset = offset % cart.prg_rom.len();
            FetchResult { data: cart.prg_rom[final_offset], driven: true }
        } else if address >= 0x6000 && address < 0x8000 && (self.misc & 0x80) != 0 {
            let idx = (address as usize - 0x6000) & (cart.prg_ram.len() - 1);
            FetchResult { data: cart.prg_ram[idx], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let page = (address >> 12) & 7; 
            let addr = address & 0x0FFF;
            match page {
                0 | 1 => {
                    let reg = ((page & 1) << 1) | (if (addr & self.a0_mask) != 0 { 1 } else { 0 });
                    if reg < 3 {
                        self.prg[reg as usize] = data;
                    } else {
                        let is_data = (addr & self.a1_mask) != 0;
                        if is_data {
                            self.write_audio_reg(self.reg_addr, data);
                        } else {
                            self.reg_addr = data;
                        }
                    }
                }
                2 | 3 | 4 | 5 => {
                    let reg = (((page - 2) as u8) << 1) | (if (addr & self.a0_mask) != 0 { 1 } else { 0 });
                    if (reg as usize) < self.chr.len() {
                        self.chr[reg as usize] = data;
                    }
                }
                6 => {
                    if (addr & self.a0_mask) != 0 {
                        self.latch = data;
                    } else {
                        self.misc = data;
                    }
                }
                7 => {
                    self.irq_ack = true;
                    if (addr & self.a0_mask) != 0 {
                        self.irq = (self.irq & !2) | ((self.irq << 1) & 2);
                    } else {
                        self.irq = data;
                        if (self.irq & 2) != 0 {
                            self.counter = self.latch;
                            self.irq_cycles = 341;
                        }
                    }
                }
                _ => {}
            }
        } else if address >= 0x6000 && address < 0x8000 && (self.misc & 0x80) != 0 {
            let idx = (address as usize - 0x6000) & (cart.prg_ram.len() - 1);
            cart.prg_ram[idx] = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror_address(address)
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
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let bank = (address >> 10) as usize & 0x07;
            let chr_bank = self.chr[bank] as usize;
            let offset = (chr_bank * 0x400) + (address as usize & 0x3FF);
            if using_chr_ram || chr_rom.is_empty() {
                new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
            } else {
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else {
            let mirrored = self.mirror_address(address);
            let idx = (mirrored & 0x7FF) as usize;
            new_addr_bus |= vram[idx] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.audio_cycles += cycles as u32;
        while self.audio_cycles >= self.cycles_per_audio_step {
            self.audio_cycles -= self.cycles_per_audio_step;
            self.step_audio();
        }
        let irq_enabled = (self.irq & 2) != 0;
        let irq_mode = (self.irq & 4) != 0;
        if irq_enabled {
            if irq_mode {
                for _ in 0..cycles {
                    if self.counter == 0xFF {
                        self.counter = self.latch;
                        return true;
                    }
                    self.counter += 1;
                }
            } else {
                for _ in 0..cycles {
                    self.irq_cycles -= 3;
                    if self.irq_cycles <= 0 {
                        self.irq_cycles += 341;
                        if self.counter == 0xFF {
                            self.counter = self.latch;
                            return true;
                        }
                        self.counter += 1;
                    }
                }
            }
        }
        false
    }

    fn audio_sample(&self) -> f32 {
        self.current_sample
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack;
        self.irq_ack = false;
        ack
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg);
        state.extend_from_slice(&self.chr);
        state.push(self.misc);
        state.push(self.latch);
        state.push(self.irq);
        state.push(self.counter);
        state.push(self.irq_latch);
        state.extend_from_slice(&self.irq_cycles.to_le_bytes());
        state.push(self.reg_addr);
        state.extend_from_slice(&self.custom_patch);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 3 + 8 + 1 + 1 + 1 + 1 + 1 + 4 + 1 + 8 <= state.len() {
            self.prg.copy_from_slice(&state[start..start + 3]);
            start += 3;
            self.chr.copy_from_slice(&state[start..start + 8]);
            start += 8;
            self.misc = state[start];
            start += 1;
            self.latch = state[start];
            start += 1;
            self.irq = state[start];
            start += 1;
            self.counter = state[start];
            start += 1;
            self.irq_latch = state[start];
            start += 1;
            self.irq_cycles = i32::from_le_bytes([state[start], state[start + 1], state[start + 2], state[start + 3]]);
            start += 4;
            self.reg_addr = state[start];
            start += 1;
            self.custom_patch.copy_from_slice(&state[start..start + 8]);
            start += 8;
        }
        start
    }

    fn reset(&mut self) {
        self.prg = [0, 1, 0xFE];
        for i in 0..8 {
            self.chr[i] = i as u8;
        }
        self.misc = 0;
        self.latch = 0;
        self.irq = 0;
        self.counter = 0;
        self.irq_latch = 0;
        self.irq_cycles = 0;
        self.reg_addr = 0;
        self.custom_patch = [0; 8];
        self.channels = vec![Channel::new(); 6];
        self.audio_cycles = 0;
        self.cycles_per_audio_step = 36;
        self.lfo_am_phase = 0.0;
        self.lfo_pm_phase = 0.0;
        self.current_sample = 0.0;
        self.irq_ack = false;
    }
}
