use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};
use crate::mappers::vrc7::Vrc7;

const LUT509: [u16; 512] = [
    7, 8, 9, 10, 11,
    12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
    48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 0, 1, 73, 74, 75, 76, 77, 78, 79, 80, 81,
    82, 83, 84, 85, 86, 87, 88, 89, 90, 4, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116,
    117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 2, 3, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 5, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149,
    150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185,
    186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221,
    222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 256, 6, 257, 258,
    259, 260, 261, 262, 263, 264, 265, 266, 267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 277, 278, 279, 280, 281, 282, 283, 284, 285, 286, 287, 288, 289, 290, 291, 292, 293, 294,
    295, 296, 297, 298, 299, 300, 301, 302, 303, 304, 305, 306, 307, 308, 309, 310, 311, 312, 313, 314, 315, 316, 317, 318, 319, 320, 321, 322, 323, 324, 325, 326, 327, 328, 329, 330,
    331, 332, 333, 334, 335, 336, 337, 338, 339, 340, 341, 342, 343, 344, 345, 346, 347, 349, 350, 351, 352, 353, 354, 355, 356, 357, 358, 359, 360, 361, 362, 363, 364, 365, 366, 367,
    368, 369, 370, 371, 372, 373, 374, 375, 376, 377, 378, 379, 380, 381, 382, 383, 384, 385, 386, 387, 388, 389, 390, 391, 392, 393, 394, 395, 396, 397, 398, 399, 400, 401, 402, 403,
    404, 405, 406, 407, 408, 409, 410, 411, 412, 413, 414, 415, 416, 417, 418, 419, 420, 421, 422, 423, 424, 425, 426, 427, 428, 429, 430, 431, 432, 433, 434, 435, 436, 437, 438, 439,
    440, 441, 442, 443, 444, 445, 446, 447, 448, 449, 450, 451, 452, 453, 454, 455, 456, 457, 458, 459, 460, 461, 462, 463, 464, 465, 466, 467, 468, 469, 470, 471, 472, 473, 474, 475,
    476, 477, 478, 479, 480, 481, 482, 483, 484, 485, 486, 487, 488, 489, 490, 491, 492, 493, 494, 495, 496, 497, 498, 499, 500, 501, 502, 503, 504, 505, 506, 507, 508, 512, 513, 514,
    515, 516, 517,
];

#[derive(Clone)]
struct SerialLatch {
    scratch_data: [u8; 16],
    command: u8,
    state: u8,
    clock: bool,
    data: bool,
    output: bool,
}

impl SerialLatch {
    fn new() -> Self {
        Self {
            scratch_data: [0; 16],
            command: 0,
            state: 0,
            clock: true,
            data: true,
            output: true,
        }
    }

    fn reset(&mut self) {
        self.state = 0;
        self.clock = true;
        self.data = true;
        self.output = true;
    }

    fn get_data(&self) -> bool {
        self.output
    }

    fn set_pins(&mut self, select: bool, new_clock: bool, new_data: bool) {
        if select {
            self.state = 0;
        } else if !self.clock && new_clock {
            if self.state < 8 {
                self.command = (self.command << 1) | (if new_data { 1 } else { 0 });
                self.state += 1;
                if self.state == 8 && (self.command & 0xF0) != 0x50 && (self.command & 0xF0) != 0xA0 {
                    self.state = 0;
                }
            } else {
                let mask = 1 << (15 - self.state);
                let address = (self.command & 0x0F) as usize;
                let lut_address = (self.scratch_data[0] as usize)
                    | (self.scratch_data[1] as usize)
                    | ((self.scratch_data[2] as usize) << 8);
                if (self.command & 0xF0) == 0xA0 {
                    self.scratch_data[address] =
                        (self.scratch_data[address] & !mask) | (if new_data { mask } else { 0 });
                    let shift = if (address & 1) != 0 { 0 } else { 8 };
                    self.output = ((LUT509[lut_address & 0x1FF] >> shift) & (mask as u16)) != 0;
                } else if (self.command & 0xF0) == 0x50 {
                    self.output = (self.scratch_data[address] & mask) != 0;
                }

                self.state += 1;
                if self.state == 16 {
                    self.state = 0;
                }
            }
        }
        self.clock = new_clock;
        self.data = new_data;
    }
}

#[derive(Clone, Copy)]
struct Mmc1State {
    reg: [u8; 4],
    shift: u8,
    bits: u8,
    filter: u8,
}

impl Mmc1State {
    fn new() -> Self {
        Self {
            reg: [0x0C, 0, 0, 0],
            shift: 0,
            bits: 0,
            filter: 0,
        }
    }

    fn reset(&mut self) {
        self.reg = [0x0C, 0, 0, 0];
        self.shift = 0;
        self.bits = 0;
        self.filter = 0;
    }

    fn write(&mut self, bank: usize, data: u8) {
        if data & 0x80 != 0 {
            self.reg[0] |= 0x0C;
            self.shift = 0;
            self.bits = 0;
        } else if self.filter == 0 {
            self.shift |= (data & 1) << self.bits;
            self.bits += 1;
            if self.bits == 5 {
                let idx = ((bank >> 1) & 3) as usize;
                self.reg[idx] = self.shift;
                self.shift = 0;
                self.bits = 0;
            }
        }
        self.filter = 2;
    }

    fn prg16_bank(&self, bank: u8, andh: usize, orh: usize) -> usize {
        let prg = self.reg[3] as usize;
        let control = self.reg[0];
        let result = match (control >> 2) & 3 {
            0 | 1 => (prg & !1) | (bank as usize),
            2 => if bank == 0 { 0 } else { prg },
            3 => if bank == 0 { prg } else { 0x0F },
            _ => 0,
        };
        ((result & 0x0F) & andh) | orh
    }

    fn chr_bank(&self, bank: u8) -> u8 {
        if (self.reg[0] & 0x10) != 0 {
            self.reg[1 + (bank as usize)]
        } else {
            (self.reg[1] & !1) | bank
        }
    }
}

#[derive(Clone, Copy)]
struct Mmc2State {
    prg: u8,
    chr: [u8; 4],
    state: [u8; 2],
    mirroring: u8,
}

impl Mmc2State {
    fn new() -> Self {
        Self {
            prg: 0,
            chr: [0; 4],
            state: [0; 2],
            mirroring: 0,
        }
    }

    fn reset(&mut self) {
        self.prg = 0;
        self.chr = [0; 4];
        self.state = [0; 2];
        self.mirroring = 0;
    }
}

#[derive(Clone, Copy)]
struct Mmc3State {
    index: u8,
    reg: [u8; 8],
    mirroring: u8,
    wram_control: u8,
    enable_irq: bool,
    reload: bool,
    counter: u8,
    reload_value: u8,
    pa12_filter: u8,
}

impl Mmc3State {
    fn new() -> Self {
        Self {
            index: 0,
            reg: [0, 2, 4, 5, 6, 7, 0, 1],
            mirroring: 0,
            wram_control: 0,
            enable_irq: false,
            reload: false,
            counter: 0,
            reload_value: 0,
            pa12_filter: 0,
        }
    }

    fn reset(&mut self) {
        self.index = 0;
        self.reg = [0, 2, 4, 5, 6, 7, 0, 1];
        self.mirroring = 0;
        self.wram_control = 0;
        self.enable_irq = false;
        self.reload = false;
        self.counter = 0;
        self.reload_value = 0;
        self.pa12_filter = 0;
    }

    fn get_prg_bank(&self, bank: usize) -> usize {
        let mut b = bank as u8;
        if (self.index & 0x40) != 0 && (b & 1) == 0 {
            b ^= 2;
        }
        if (b & 2) != 0 {
            (0xFE | (b & 1)) as usize
        } else {
            self.reg[(6 | (b & 1)) as usize] as usize
        }
    }

    fn get_chr_bank(&self, bank: usize) -> usize {
        let mut b = bank as u8;
        if (self.index & 0x80) != 0 {
            b ^= 4;
        }
        if (b & 4) != 0 {
            self.reg[(b - 2) as usize] as usize
        } else {
            ((self.reg[(b >> 1) as usize] & !1) | (b & 1)) as usize
        }
    }
}

#[derive(Clone, Copy)]
struct Mmc4State {
    prg: u8,
    chr: [u8; 4],
    state: [u8; 2],
    mirroring: u8,
}

impl Mmc4State {
    fn new() -> Self {
        Self {
            prg: 0,
            chr: [0; 4],
            state: [0; 2],
            mirroring: 0,
        }
    }

    fn reset(&mut self) {
        self.prg = 0;
        self.chr = [0; 4];
        self.state = [0; 2];
        self.mirroring = 0;
    }
}

#[derive(Clone, Copy)]
struct Vrc1State {
    prg: [u8; 3],
    chr: [u8; 2],
    misc: u8,
}

impl Vrc1State {
    fn new() -> Self {
        Self {
            prg: [0; 3],
            chr: [0; 2],
            misc: 0,
        }
    }

    fn reset(&mut self) {
        self.prg = [0; 3];
        self.chr = [0; 2];
        self.misc = 0;
    }
}

#[derive(Clone, Copy)]
struct Vrc24State {
    prg: [u8; 2],
    chr: [u16; 8],
    mirroring: u8,
    latch: u8,
    mode: u8,
    counter: u8,
    cycles: i16,
    misc: u8,
    is_vrc4: bool,
    a0: u8,
    a1: u8,
}

impl Vrc24State {
    fn new() -> Self {
        Self {
            prg: [0, 1],
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            mirroring: 0,
            latch: 0,
            mode: 0,
            counter: 0,
            cycles: 0,
            misc: 0,
            is_vrc4: false,
            a0: 1,
            a1: 2,
        }
    }

    fn reset(&mut self, is_vrc4: bool, a0: u8, a1: u8) {
        self.prg = [0, 1];
        for i in 0..8 {
            self.chr[i] = i as u16;
        }
        self.mirroring = 0;
        self.latch = 0;
        self.mode = 0;
        self.counter = 0;
        self.cycles = 0;
        self.misc = 0;
        self.is_vrc4 = is_vrc4;
        self.a0 = a0;
        self.a1 = a1;
    }

    fn get_prg_bank(&self, bank: usize) -> usize {
        let mut b = bank as u8;
        if (b & 1) == 0 && (self.misc & 2) != 0 {
            b ^= 2;
        }
        if (b & 2) != 0 {
            (0xFE | (b & 1)) as usize
        } else {
            self.prg[(b & 1) as usize] as usize
        }
    }
}

#[derive(Clone, Copy)]
struct Vrc3State {
    irq: u8,
    counter: u16,
    latch: u16,
}

impl Vrc3State {
    fn new() -> Self {
        Self {
            irq: 0,
            counter: 0,
            latch: 0,
        }
    }

    fn reset(&mut self) {
        self.irq = 0;
        self.counter = 0;
        self.latch = 0;
    }
}

#[derive(Clone, Copy)]
struct Vrc6State {
    prg: [u8; 2],
    chr: [u8; 8],
    mode: u8,
    irq_control: u8,
    irq_counter: u8,
    irq_latch: u8,
    irq_cycles: i16,
    a0: u8,
    a1: u8,
}

impl Vrc6State {
    fn new(a0: u8, a1: u8) -> Self {
        Self {
            prg: [0, 0xFE],
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            mode: 0,
            irq_control: 0,
            irq_counter: 0,
            irq_latch: 0,
            irq_cycles: 0,
            a0,
            a1,
        }
    }

    fn reset(&mut self) {
        self.prg = [0, 0xFE];
        for i in 0..8 {
            self.chr[i] = i as u8;
        }
        self.mode = 0;
        self.irq_control = 0;
        self.irq_counter = 0;
        self.irq_latch = 0;
        self.irq_cycles = 0;
    }
}

struct Vrc7State {
    prg: [u8; 3],
    chr: [u8; 8],
    misc: u8,
    irq: u8,
    counter: u8,
    latch: u8,
    cycles: i16,
    a0: u8,
    a1: u8,
    core: Vrc7,
}

impl Vrc7State {
    fn new(a0: u8, a1: u8) -> Self {
        Self {
            prg: [0, 1, 0xFE],
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            misc: 0,
            irq: 0,
            counter: 0,
            latch: 0,
            cycles: 0,
            a0,
            a1,
            core: Vrc7::new(0),
        }
    }

    fn reset(&mut self) {
        self.prg = [0, 1, 0xFE];
        for i in 0..8 {
            self.chr[i] = i as u8;
        }
        self.misc = 0;
        self.irq = 0;
        self.counter = 0;
        self.latch = 0;
        self.cycles = 0;
        self.core.reset();
    }
}

#[derive(Clone)]
struct Fme7State {
    cmd: u8,
    chr_1k: [u8; 8],
    bank_6: u8,
    bank_6_is_ram: bool,
    bank_6_is_ram_enabled: bool,
    bank_8: u8,
    bank_a: u8,
    bank_c: u8,
    mirr: u8,
    irq_control: u8,
    counter: u16,
    sndcmd: u8,
    sreg: [u8; 14],
    vcount: [i32; 3],
    dcount: [u8; 3],
    current_audio_sample: f32,
}

impl Fme7State {
    fn new() -> Self {
        Self {
            cmd: 0,
            chr_1k: [0, 1, 2, 3, 4, 5, 6, 7],
            bank_6: 0,
            bank_6_is_ram: false,
            bank_6_is_ram_enabled: false,
            bank_8: 0,
            bank_a: 1,
            bank_c: 0xFE,
            mirr: 0,
            irq_control: 0,
            counter: 0,
            sndcmd: 0,
            sreg: [0; 14],
            vcount: [0; 3],
            dcount: [0; 3],
            current_audio_sample: 0.0,
        }
    }

    fn reset(&mut self) {
        self.cmd = 0;
        for i in 0..8 {
            self.chr_1k[i] = i as u8;
        }
        self.bank_6 = 0;
        self.bank_6_is_ram = false;
        self.bank_6_is_ram_enabled = false;
        self.bank_8 = 0;
        self.bank_a = 1;
        self.bank_c = 0xFE;
        self.mirr = 0;
        self.irq_control = 0;
        self.counter = 0;
    }

    fn channel_period(&self, ch: usize) -> i32 {
        let lo = self.sreg[ch * 2] as i32;
        let hi = (self.sreg[ch * 2 + 1] & 0x0F) as i32;
        ((lo | (hi << 8)) + 1) << 4
    }

    fn channel_amp(&self, ch: usize) -> f32 {
        let raw = (self.sreg[0x8 + ch] & 0x0F) as f32;
        (raw + raw * 0.5) / 15.0
    }

    fn channel_enabled(&self, ch: usize) -> bool {
        (self.sreg[0x7] & (1 << ch)) == 0
    }

    fn presync_channel(&mut self, ch: usize) {
        self.vcount[ch] = self.channel_period(ch);
    }

    fn sound_data_write(&mut self, data: u8) {
        match self.sndcmd {
            0 | 1 | 8 => self.presync_channel(0),
            2 | 3 | 9 => self.presync_channel(1),
            4 | 5 | 10 => self.presync_channel(2),
            7 => {
                self.presync_channel(0);
                self.presync_channel(1);
            }
            _ => {}
        }
        if (self.sndcmd as usize) < 14 {
            self.sreg[self.sndcmd as usize] = data;
        }
    }

    fn tick_audio(&mut self) {
        let mut mix = 0.0f32;
        for ch in 0..3 {
            if !self.channel_enabled(ch) {
                continue;
            }
            let amp = self.channel_amp(ch);
            if amp == 0.0 {
                continue;
            }
            if self.dcount[ch] != 0 {
                mix += amp;
            }
            self.vcount[ch] -= 1;
            if self.vcount[ch] <= 0 {
                self.dcount[ch] ^= 1;
                self.vcount[ch] += self.channel_period(ch);
            }
        }
        self.current_audio_sample = mix * 0.12;
    }
}

#[derive(Clone, Copy)]
struct LatchState {
    addr: u16,
    data: u8,
}

impl LatchState {
    fn new() -> Self {
        Self { addr: 0, data: 0 }
    }

    fn reset(&mut self) {
        self.addr = 0;
        self.data = 0;
    }
}

pub struct Mapper468 {
    submapper_id: u8,
    reg: [u16; 8],
    extra: u16,
    scratch_rom: SerialLatch,
    pa09: bool,
    pa13: bool,
    irq_ack_pending: bool,

    mmc1: Mmc1State,
    mmc2: Mmc2State,
    mmc3: Mmc3State,
    mmc4: Mmc4State,
    vrc1: Vrc1State,
    vrc24: Vrc24State,
    vrc3: Vrc3State,
    vrc6: Vrc6State,
    vrc7: Vrc7State,
    fme7: Fme7State,
    latch: LatchState,
}

impl Mapper468 {
    pub fn new(
        submapper_id: u8,
        _header: &[u8],
        _rom: &[u8],
        _rom_name: &str,
        _using_chr_ram: bool,
        _has_battery: bool,
    ) -> Self {
        let mut m = Self {
            submapper_id,
            reg: [0; 8],
            extra: 0x10,
            scratch_rom: SerialLatch::new(),
            pa09: false,
            pa13: false,
            irq_ack_pending: false,

            mmc1: Mmc1State::new(),
            mmc2: Mmc2State::new(),
            mmc3: Mmc3State::new(),
            mmc4: Mmc4State::new(),
            vrc1: Vrc1State::new(),
            vrc24: Vrc24State::new(),
            vrc3: Vrc3State::new(),
            vrc6: Vrc6State::new(1, 2),
            vrc7: Vrc7State::new(0x10, 0x20),
            fme7: Fme7State::new(),
            latch: LatchState::new(),
        };
        m.reset_state();
        m
    }

    fn reset_state(&mut self) {
        self.reg[6] = 0x0000;
        self.reg[7] = 0xFF0F;
        self.extra = 0x10;
        self.pa09 = false;
        self.pa13 = false;
        self.irq_ack_pending = false;
        self.scratch_rom.reset();
        self.mmc1.reset();
        self.mmc2.reset();
        self.mmc3.reset();
        self.mmc4.reset();
        self.vrc1.reset();
        self.vrc3.reset();
        self.vrc6.reset();
        self.vrc7.reset();
        self.fme7.reset();
        self.latch.reset();
        self.apply_mapper();
    }

    fn fusemap(&self) -> u16 {
        ((self.submapper_id as u16) << 4) | ((self.reg[7] >> 4) & 0x0F)
    }

    fn flags(&self) -> u8 {
        (self.reg[7] & 0x0F) as u8
    }

    fn prg_and(&self) -> usize {
        let fusemap = self.fusemap();
        let flags = self.flags();
        match fusemap {
            0x50 => {
                if (flags & 2) != 0 {
                    if (self.extra & 2) != 0 { 0x07 } else { 0x0F }
                } else {
                    0x1F
                }
            }
            0x00 | 0x01 | 0x32 => {
                if (flags & 2) != 0 {
                    if (flags & 8) != 0 { 0x03 } else { 0x07 }
                } else {
                    0x0F
                }
            }
            0x0A => 0x0F,
            0x08 => {
                if (flags & 2) != 0 { 0x07 } else { 0x0F }
            }
            0x09 | 0x0B | 0x17 | 0x37 => {
                if fusemap == 0x0B || (fusemap == 0x17 && (self.reg[7] & 2) == 0) {
                    0x1F
                } else if (self.reg[7] & 2) != 0 {
                    0x07
                } else {
                    0x0F
                }
            }
            0x04 | 0x06 | 0x14 | 0x16 => {
                if fusemap == 0x06 || fusemap == 0x16 {
                    0x0F
                } else if (flags & 2) != 0 {
                    0x03
                } else {
                    0x07
                }
            }
            0x0C | 0x0D | 0x1C | 0x1D => {
                if (flags & 8) != 0 { 1 } else { 3 }
            }
            0x20 | 0x21 | 0x22 | 0x23 => {
                if (flags & 2) != 0 { 0x0F } else { 0x1F }
            }
            0x30 | 0x31 => {
                if (flags & 2) != 0 { 0x0F } else { 0x1F }
            }
            0x40 => {
                if (flags & 8) != 0 {
                    if (flags & 4) != 0 {
                        if (flags & 2) != 0 { 0x0F } else { 0x1F }
                    } else {
                        0x3F
                    }
                } else {
                    0x7F
                }
            }
            0x41 => {
                if (flags & 8) != 0 {
                    if (flags & 4) != 0 {
                        if (flags & 2) != 0 { 0x0F } else { 0x1F }
                    } else {
                        0x3F
                    }
                } else {
                    0x7F
                }
            }
            0x44 => 0x07,
            0x10 | 0x11 | 0x12 | _ => {
                if (flags & 8) != 0 {
                    if (flags & 4) != 0 {
                        if (flags & 2) != 0 {
                            if (self.extra & 2) != 0 { 0x07 } else { 0x0F }
                        } else {
                            0x1F
                        }
                    } else {
                        0x3F
                    }
                } else {
                    0x7F
                }
            }
        }
    }

    fn prg_or(&self) -> usize {
        if self.submapper_id == 1 {
            (((self.extra as usize) << 9) & 0x2000)
                | (((self.reg[7] as usize) >> 3) & 0x1FE0)
                | (((self.reg[7] as usize) << 4) & 0x0010)
        } else {
            (((self.reg[6] as usize) << 1) & 0x2000)
                | (((self.reg[7] as usize) >> 3) & 0x1FE0)
                | (((self.reg[7] as usize) << 4) & 0x0010)
        }
    }

    fn apply_mapper(&mut self) {
        let fusemap = self.fusemap();
        match fusemap {
            0x50 => {
                self.fme7.reset();
            }
            0x00 | 0x01 | 0x32 => {
                self.mmc1.reset();
            }
            0x0A => {
                self.mmc2.reset();
            }
            0x08 => {
                self.mmc4.reset();
            }
            0x09 | 0x0B | 0x17 | 0x37 => {
                if (self.flags() & 8) != 0 {
                    self.latch.reset();
                }
            }
            0x04 | 0x06 | 0x14 | 0x16 => {
                self.latch.reset();
            }
            0x05 | 0x15 => {
                self.reg[0] = 0;
                self.reg[1] = 0;
                self.reg[2] = 0;
            }
            0x0C | 0x0D | 0x1C | 0x1D => {
                self.latch.reset();
            }
            0x20 | 0x21 | 0x22 | 0x23 => {
                let a0 = if (fusemap & 2) != 0 { 2 } else { 1 };
                let a1 = if (fusemap & 2) != 0 { 1 } else { 2 };
                let is_vrc4 = (fusemap & 1) != 0;
                self.vrc24.reset(is_vrc4, a0, a1);
            }
            0x30 | 0x31 => {
                self.vrc6.reset();
            }
            0x40 => {
                self.vrc1.reset();
            }
            0x41 => {
                self.vrc7.reset();
            }
            0x44 => {
                self.vrc3.reset();
            }
            0x0E | 0x1E => {
                self.reg[0] = 0;
                self.reg[1] = 0;
                self.reg[2] = 0;
                self.reg[3] = 0;
            }
            0x07 => {
                self.reg[0] = 0;
                self.reg[1] = 0;
                self.reg[2] = 0;
            }
            0x10 | 0x11 | 0x12 | _ => {
                self.mmc3.enable_irq = false;
            }
        }
    }

    fn write_reg(&mut self, addr: u16, val: u8) {
        if (self.reg[6] & 0x8000) != 0 && addr >= 0x600 {
            return;
        }
        if addr >= 0x700 && (addr & 2) != 0 {
            self.extra = val as u16;
            self.apply_mapper();
            return;
        }
        let index = ((addr >> 8) & 7) as usize;
        if (addr & 1) != 0 {
            self.reg[index] = (self.reg[index] & 0x00FF) | ((val as u16) << 8);
        } else {
            self.reg[index] = (self.reg[index] & 0xFF00) | (val as u16);
        }

        if index == 7 && (addr & 1) == 0 {
            self.apply_mapper();
        }
        if index == 3 && self.submapper_id == 0 {
            self.scratch_rom.set_pins(
                (self.reg[3] & 0x0400) != 0,
                (self.reg[3] & 0x0200) != 0,
                (self.reg[3] & 0x0100) != 0,
            );
        }
        if index == 6 && self.submapper_id == 1 {
            self.scratch_rom.set_pins(
                (self.reg[6] & 0x1000) != 0,
                (self.reg[6] & 0x0200) != 0,
                (self.reg[6] & 0x0100) != 0,
            );
        }
    }

    fn prg_8k_slots(&self) -> [usize; 4] {
        let fusemap = self.fusemap();
        let flags = self.flags();
        let and = self.prg_and();
        let or = self.prg_or();

        match fusemap {
            0x50 => [
                (self.fme7.bank_8 as usize & and) | (or & !and),
                (self.fme7.bank_a as usize & and) | (or & !and),
                (self.fme7.bank_c as usize & and) | (or & !and),
                (0xFF & and) | (or & !and),
            ],
            0x00 | 0x01 | 0x32 => {
                if fusemap == 0x01 {
                    let orh = or >> 1;
                    let chr_high = (self.mmc1.chr_bank(0) & 0x10) as usize;
                    let surom_or = chr_high | (orh & !0x1F);
                    let b0 = self.mmc1.prg16_bank(0, 0x0F, surom_or);
                    let b1 = self.mmc1.prg16_bank(1, 0x0F, surom_or);
                    [b0 * 2, b0 * 2 + 1, b1 * 2, b1 * 2 + 1]
                } else {
                    let orh = (or >> 1) | ((self.reg[7] as usize) & 0x0006);
                    let b0 = self.mmc1.prg16_bank(0, and, orh & !and);
                    let b1 = self.mmc1.prg16_bank(1, and, orh & !and);
                    [b0 * 2, b0 * 2 + 1, b1 * 2, b1 * 2 + 1]
                }
            }
            0x0A => [
                (self.mmc2.prg as usize & and) | (or & !and),
                (0x0D & and) | (or & !and),
                (0x0E & and) | (or & !and),
                (0x0F & and) | (or & !and),
            ],
            0x08 => {
                let orh = or >> 1;
                let b0 = (self.mmc4.prg as usize & and) | (orh & !and);
                let b1 = (0xFF & and) | (orh & !and);
                [b0 * 2, b0 * 2 + 1, b1 * 2, b1 * 2 + 1]
            }
            0x09 | 0x0B | 0x17 | 0x37 => {
                let orh = or >> 1;
                if (flags & 8) != 0 {
                    let mut latch_d = self.latch.data as usize;
                    if self.latch.addr == 0xA000
                        && self.latch.data == 0x00
                        && (self.reg[7] & 0xFF) == 0x9E
                    {
                        latch_d = 0x06;
                    }
                    let b0 = (latch_d & and) | (orh & !and);
                    let b1 = (and) | (orh & !and);
                    [b0 * 2, b0 * 2 + 1, b1 * 2, b1 * 2 + 1]
                } else {
                    let b0 = (self.reg[2] as usize & and) | (orh & !and);
                    let b1 = (and) | (orh & !and);
                    [b0 * 2, b0 * 2 + 1, b1 * 2, b1 * 2 + 1]
                }
            }
            0x04 | 0x06 | 0x14 | 0x16 => {
                let or32 = or >> 2;
                let b = (self.latch.data as usize & and) | (or32 & !and);
                [b * 4, b * 4 + 1, b * 4 + 2, b * 4 + 3]
            }
            0x05 | 0x15 => {
                if (self.reg[7] & 0x02) != 0 {
                    let or_masked = or & !0x0F;
                    let b = (((self.reg[2] as usize) << 1) & 0x0E)
                        | ((self.reg[7] as usize) & 0x01)
                        | ((or_masked >> 1) & !0x0F);
                    [b * 2, b * 2 + 1, b * 2, b * 2 + 1]
                } else {
                    let b = ((self.reg[2] as usize) & 0x07) | (or >> 2);
                    [b * 4, b * 4 + 1, b * 4 + 2, b * 4 + 3]
                }
            }
            0x0C | 0x0D | 0x1C | 0x1D => {
                let or32 = (or >> 2) | (if (self.reg[7] & 4) != 0 { 2 } else { 0 });
                let b = if (self.reg[6] & 0x1000) != 0 && (fusemap & !0x10) != 0 {
                    (((self.latch.data as usize) >> 4) & and) | (or32 & !and)
                } else {
                    ((self.latch.data as usize) & and) | (or32 & !and)
                };
                [b * 4, b * 4 + 1, b * 4 + 2, b * 4 + 3]
            }
            0x20 | 0x21 | 0x22 | 0x23 => [
                (self.vrc24.get_prg_bank(0) & and) | (or & !and),
                (self.vrc24.get_prg_bank(1) & and) | (or & !and),
                (self.vrc24.get_prg_bank(2) & and) | (or & !and),
                (self.vrc24.get_prg_bank(3) & and) | (or & !and),
            ],
            0x30 | 0x31 => {
                let or16 = or >> 1;
                let b0 = ((self.vrc6.prg[0] as usize) & (and >> 1)) | (or16 & !(and >> 1));
                let b2 = ((self.vrc6.prg[1] as usize) & and) | (or & !and);
                let b3 = (0xFF & and) | (or & !and);
                [b0 * 2, b0 * 2 + 1, b2, b3]
            }
            0x40 => [
                (self.vrc1.prg[0] as usize & and) | (or & !and),
                (self.vrc1.prg[1] as usize & and) | (or & !and),
                (self.vrc1.prg[2] as usize & and) | (or & !and),
                (0xFF & and) | (or & !and),
            ],
            0x41 => [
                (self.vrc7.prg[0] as usize & and) | (or & !and),
                (self.vrc7.prg[1] as usize & and) | (or & !and),
                (self.vrc7.prg[2] as usize & and) | (or & !and),
                (0xFF & and) | (or & !and),
            ],
            0x44 => {
                let orh = or >> 1;
                let b0 = ((self.reg[3] as usize) & and) | (orh & !and);
                let b1 = and | (orh & !and);
                [b0 * 2, b0 * 2 + 1, b1 * 2, b1 * 2 + 1]
            }
            0x0E | 0x1E => {
                let b = (((self.reg[2] as usize) << 4) & 0x30)
                    | ((self.reg[0] as usize) & 0x0F)
                    | (if (self.reg[3] & 4) != 0 { 0x00 } else { 0x03 })
                    | (or >> 2);
                [b * 4, b * 4 + 1, b * 4 + 2, b * 4 + 3]
            }
            0x07 => {
                let or_smb = or | ((self.reg[7] as usize) & 8);
                [
                    4 | or_smb,
                    5 | or_smb,
                    ((self.reg[2] as usize) & 7) | or_smb,
                    7 | or_smb,
                ]
            }
            0x10 | 0x11 | 0x12 | _ => {
                let mmc3_or = or | (if (self.extra & 1) != 0 { 12 } else { 0 });
                [
                    (self.mmc3.get_prg_bank(0) & and) | (mmc3_or & !and),
                    (self.mmc3.get_prg_bank(1) & and) | (mmc3_or & !and),
                    (self.mmc3.get_prg_bank(2) & and) | (mmc3_or & !and),
                    (self.mmc3.get_prg_bank(3) & and) | (mmc3_or & !and),
                ]
            }
        }
    }

    fn chr_1k_slots(&self, chr_rom_len: usize) -> [usize; 8] {
        let fusemap = self.fusemap();
        let flags = self.flags();

        match fusemap {
            0x50 => {
                let chr_and = if (self.reg[7] & 0x08) != 0 { 0xFF } else { 0x7F };
                let mut out = [0usize; 8];
                for i in 0..8 {
                    out[i] = (self.fme7.chr_1k[i] as usize) & chr_and;
                }
                out
            }
            0x00 | 0x01 | 0x32 => {
                let (b0, b1) = if fusemap == 0x01 {
                    (
                        (self.mmc1.chr_bank(0) & 0x01) as usize,
                        (self.mmc1.chr_bank(1) & 0x01) as usize,
                    )
                } else {
                    (
                        (self.mmc1.chr_bank(0) & 0x1F) as usize,
                        (self.mmc1.chr_bank(1) & 0x1F) as usize,
                    )
                };
                [
                    b0 * 4,
                    b0 * 4 + 1,
                    b0 * 4 + 2,
                    b0 * 4 + 3,
                    b1 * 4,
                    b1 * 4 + 1,
                    b1 * 4 + 2,
                    b1 * 4 + 3,
                ]
            }
            0x0A => {
                let b0 = (self.mmc2.chr[self.mmc2.state[0] as usize] as usize) & 0x1F;
                let b1 = (self.mmc2.chr[(self.mmc2.state[1] | 2) as usize] as usize) & 0x1F;
                [
                    b0 * 4,
                    b0 * 4 + 1,
                    b0 * 4 + 2,
                    b0 * 4 + 3,
                    b1 * 4,
                    b1 * 4 + 1,
                    b1 * 4 + 2,
                    b1 * 4 + 3,
                ]
            }
            0x08 => {
                let b0 = (self.mmc4.chr[self.mmc4.state[0] as usize] as usize) & 0x1F;
                let b1 = (self.mmc4.chr[(self.mmc4.state[1] | 2) as usize] as usize) & 0x1F;
                [
                    b0 * 4,
                    b0 * 4 + 1,
                    b0 * 4 + 2,
                    b0 * 4 + 3,
                    b1 * 4,
                    b1 * 4 + 1,
                    b1 * 4 + 2,
                    b1 * 4 + 3,
                ]
            }
            0x09 | 0x0B | 0x17 | 0x37 => {
                if (flags & 8) != 0 {
                    [0, 1, 2, 3, 4, 5, 6, 7]
                } else {
                    let b = ((self.reg[0] as usize) >> 1) & 0x0F;
                    [
                        b * 8,
                        b * 8 + 1,
                        b * 8 + 2,
                        b * 8 + 3,
                        b * 8 + 4,
                        b * 8 + 5,
                        b * 8 + 6,
                        b * 8 + 7,
                    ]
                }
            }
            0x04 | 0x06 | 0x14 | 0x16 => [0, 1, 2, 3, 4, 5, 6, 7],
            0x05 | 0x15 => {
                let b = if (self.reg[7] & 0x02) != 0 {
                    (self.reg[0] as usize) & 3
                } else {
                    (self.reg[0] as usize) & 15
                };
                [
                    b * 8,
                    b * 8 + 1,
                    b * 8 + 2,
                    b * 8 + 3,
                    b * 8 + 4,
                    b * 8 + 5,
                    b * 8 + 6,
                    b * 8 + 7,
                ]
            }
            0x0C | 0x0D | 0x1C | 0x1D => {
                let b = if (self.reg[6] & 0x1000) != 0 && (fusemap & !0x10) != 0 {
                    (self.latch.data as usize) & 15
                } else {
                    ((self.latch.data as usize) >> 4) & 15
                };
                [
                    b * 8,
                    b * 8 + 1,
                    b * 8 + 2,
                    b * 8 + 3,
                    b * 8 + 4,
                    b * 8 + 5,
                    b * 8 + 6,
                    b * 8 + 7,
                ]
            }
            0x20 | 0x21 | 0x22 | 0x23 => {
                let mut out = [0usize; 8];
                for i in 0..8 {
                    out[i] = (self.vrc24.chr[i] as usize) & 0xFF;
                }
                out
            }
            0x30 | 0x31 => {
                let mut out = [0usize; 8];
                let is_4mbit = chr_rom_len >= 512 * 1024;
                let c = self.vrc6.mode & 3;
                let change_a10 = (self.vrc6.mode & 0x20) != 0;

                for bank in 0..8 {
                    let val = if c == 0 {
                        self.vrc6.chr[bank] as usize
                    } else if bank < 4 {
                        self.vrc6.chr[bank] as usize
                    } else {
                        let reg = if bank < 6 { 4 } else { 5 };
                        let rval = self.vrc6.chr[reg] as usize;
                        if change_a10 {
                            if (bank & 1) != 0 { rval | 1 } else { rval & !1 }
                        } else {
                            rval
                        }
                    };
                    out[bank] = if is_4mbit {
                        (val << 1) | (bank & 1)
                    } else {
                        val
                    };
                }
                out
            }
            0x40 => {
                let b0 = (((self.vrc1.chr[0] & 0x0F) | ((self.vrc1.misc << 3) & 0x10)) as usize) & 0x1F;
                let b1 = (((self.vrc1.chr[1] & 0x0F) | ((self.vrc1.misc << 2) & 0x10)) as usize) & 0x1F;
                [
                    b0 * 4,
                    b0 * 4 + 1,
                    b0 * 4 + 2,
                    b0 * 4 + 3,
                    b1 * 4,
                    b1 * 4 + 1,
                    b1 * 4 + 2,
                    b1 * 4 + 3,
                ]
            }
            0x41 => {
                let mut out = [0usize; 8];
                for i in 0..8 {
                    out[i] = (self.vrc7.chr[i] as usize) & 0xFF;
                }
                out
            }
            0x44 => [0, 1, 2, 3, 4, 5, 6, 7],
            0x0E | 0x1E => [0, 1, 2, 3, 4, 5, 6, 7],
            0x07 => [0, 1, 2, 3, 4, 5, 6, 7],
            0x10 | 0x11 | 0x12 | _ => {
                let chr_and = if (self.reg[7] & 0x10) != 0 { 0xFF } else { 0x7F };
                let mut out = [0usize; 8];
                for i in 0..8 {
                    out[i] = (self.mmc3.get_chr_bank(i) as usize) & chr_and;
                }
                out
            }
        }
    }

    fn mirror_address(&self, address: u16) -> u16 {
        let fusemap = self.fusemap();
        let flags = self.flags();

        match fusemap {
            0x50 => match self.fme7.mirr & 3 {
                0 => address & 0x37FF,
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                2 => address & 0x33FF,
                3 => (address & 0x33FF) | 0x0400,
                _ => address,
            },
            0x00 | 0x01 | 0x32 => match self.mmc1.reg[0] & 3 {
                0 => address & 0x33FF,
                1 => (address & 0x33FF) | 0x0400,
                2 => address & 0x37FF,
                3 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                _ => address,
            },
            0x0A => mirror_h_or_v((self.mmc2.mirroring & 1) != 0, address),
            0x08 => mirror_h_or_v((self.mmc4.mirroring & 1) != 0, address),
            0x09 | 0x0B | 0x17 | 0x37 => {
                if (flags & 8) != 0 {
                    mirror_h_or_v((self.reg[7] & 4) != 0, address)
                } else {
                    mirror_h_or_v((self.reg[0] & 1) != 0, address)
                }
            }
            0x04 | 0x06 | 0x14 | 0x16 => {
                if (flags & 8) != 0 {
                    mirror_h_or_v((self.reg[7] & 4) != 0, address)
                } else if (self.latch.data & 0x10) != 0 {
                    (address & 0x33FF) | 0x0400
                } else {
                    address & 0x33FF
                }
            }
            0x05 | 0x15 => {
                if (self.reg[7] & 8) != 0 {
                    mirror_h_or_v((self.reg[7] & 4) != 0, address)
                } else if (self.reg[1] & 0x10) != 0 {
                    (address & 0x33FF) | 0x0400
                } else {
                    address & 0x33FF
                }
            }
            0x0C | 0x0D | 0x1C | 0x1D => mirror_h_or_v((self.reg[7] & 0x10) != 0, address),
            0x20 | 0x21 | 0x22 | 0x23 => {
                let mask = if self.vrc24.is_vrc4 { 3 } else { 1 };
                match self.vrc24.mirroring & mask {
                    0 => address & 0x37FF,
                    1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                    2 => address & 0x33FF,
                    3 => (address & 0x33FF) | 0x0400,
                    _ => address,
                }
            }
            0x30 | 0x31 => {
                let screen = ((address >> 10) & 3) as usize;
                let c = self.vrc6.mode & 3;
                let m = (self.vrc6.mode >> 2) & 3;
                let reg = if c == 1 {
                    4 | screen
                } else if ((c >> 1) ^ (m & 1)) != 0 {
                    6 | (screen & 1)
                } else {
                    6 | (screen >> 1)
                };
                let mut val = self.vrc6.chr[reg] as usize;
                if (self.vrc6.mode & 0x20) != 0 && (c == 0 || c == 3) {
                    val &= !1;
                    match m ^ (c & 1) {
                        0 => val |= if (screen & 1) != 0 { 1 } else { 0 },
                        1 => val |= if (screen & 2) != 0 { 1 } else { 0 },
                        2 => {}
                        3 => val |= 1,
                        _ => {}
                    }
                }
                let page = (val & 1) as u16;
                (page * 0x400) | (address & 0x3FF)
            }
            0x40 => mirror_h_or_v((self.vrc1.misc & 1) != 0, address),
            0x41 => match self.vrc7.misc & 3 {
                0 => address & 0x37FF,
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                2 => address & 0x33FF,
                3 => (address & 0x33FF) | 0x0400,
                _ => address,
            },
            0x44 => mirror_h_or_v((self.reg[7] & 4) != 0, address),
            0x0E | 0x1E => mirror_h_or_v((self.reg[7] & 4) != 0, address),
            0x07 => mirror_h_or_v((self.reg[7] & 4) != 0, address),
            0x10 | 0x11 | 0x12 | _ => {
                if fusemap == 0x12 {
                    match self.mmc3.mirroring & 3 {
                        0 => address & 0x37FF,
                        1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                        2 => {
                            let screen = ((address >> 10) & 3) as usize;
                            let chr_bank = self.mmc3.get_chr_bank(screen);
                            let page = ((chr_bank >> 7) & 1) as u16;
                            (page * 0x400) | (address & 0x3FF)
                        }
                        3 => (address & 0x33FF) | 0x0400,
                        _ => address,
                    }
                } else {
                    mirror_h_or_v((self.mmc3.mirroring & 1) != 0, address)
                }
            }
        }
    }

    fn check_chr_split(&mut self, address: u16) {
        let pa13new = (address & 0x1000) != 0;
        if !self.pa13 && pa13new {
            self.pa09 = (address & 0x0200) != 0;
        }
        self.pa13 = pa13new;
    }

    fn mmc3_clock_counter(&mut self) -> bool {
        let prev = self.mmc3.counter;
        self.mmc3.counter = if self.mmc3.counter == 0 {
            self.mmc3.reload_value
        } else {
            self.mmc3.counter.wrapping_sub(1)
        };
        let fire = (prev != 0 || self.mmc3.reload) && self.mmc3.counter == 0 && self.mmc3.enable_irq;
        self.mmc3.reload = false;
        fire
    }
}

impl Mapper for Mapper468 {
    fn reset(&mut self) {
        self.reset_state();
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn set_cpu_clock(&mut self, clock: f64) {
        self.vrc7.core.set_audio_clock(clock);
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x5000 {
            return FetchResult { data: 0, driven: false };
        }
        if address < 0x6000 {
            let addr = address & 0x0FFF;
            return match addr {
                0x030 => FetchResult { data: 0x02, driven: true },
                0x301 | 0x601 => FetchResult {
                    data: if self.scratch_rom.get_data() { 0x80 } else { 0x00 },
                    driven: true,
                },
                _ => FetchResult { data: 0, driven: false },
            };
        }
        if address < 0x8000 {
            let fusemap = self.fusemap();
            if fusemap == 0x50 {
                let tempo = (address as usize) & 0x1FFF;
                if self.fme7.bank_6_is_ram {
                    if self.fme7.bank_6_is_ram_enabled && !cart.prg_ram.is_empty() {
                        return FetchResult {
                            data: cart.prg_ram[tempo % cart.prg_ram.len()],
                            driven: true,
                        };
                    } else {
                        return FetchResult { data: 0, driven: false };
                    }
                } else if !cart.prg_rom.is_empty() {
                    let and = self.prg_and();
                    let or = self.prg_or();
                    let bank = (self.fme7.bank_6 as usize & and) | (or & !and);
                    let offset = bank * 0x2000 + tempo;
                    return FetchResult {
                        data: cart.prg_rom[offset % cart.prg_rom.len()],
                        driven: true,
                    };
                }
            }
            if !cart.prg_ram.is_empty() {
                let offset = (address as usize) & 0x1FFF;
                return FetchResult {
                    data: cart.prg_ram[offset % cart.prg_ram.len()],
                    driven: true,
                };
            }
            return FetchResult { data: 0, driven: false };
        }

        if cart.prg_rom.is_empty() {
            return FetchResult { data: 0, driven: false };
        }
        let slots = self.prg_8k_slots();
        let slot_idx = ((address >> 13) & 3) as usize;
        let bank = slots[slot_idx];
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        FetchResult {
            data: cart.prg_rom[offset % cart.prg_rom.len()],
            driven: true,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x5000 {
            return;
        }
        if address < 0x6000 {
            self.write_reg(address & 0x0FFF, data);
            return;
        }
        if address < 0x8000 {
            let fusemap = self.fusemap();
            if fusemap == 0x50 {
                if self.fme7.bank_6_is_ram && self.fme7.bank_6_is_ram_enabled && !cart.prg_ram.is_empty() {
                    let offset = (address as usize & 0x1FFF) % cart.prg_ram.len();
                    cart.prg_ram[offset] = data;
                }
                return;
            }
            if !cart.prg_ram.is_empty() {
                let offset = (address as usize & 0x1FFF) % cart.prg_ram.len();
                cart.prg_ram[offset] = data;
            }
            return;
        }

        let fusemap = self.fusemap();
        let bank = ((address >> 12) & 0xF) as usize;
        match fusemap {
            0x50 => match bank {
                0x8 | 0x9 => {
                    self.fme7.cmd = data & 0x0F;
                }
                0xA | 0xB => match self.fme7.cmd {
                    0x0..=0x7 => self.fme7.chr_1k[self.fme7.cmd as usize] = data,
                    0x8 => {
                        self.fme7.bank_6 = data & 0x3F;
                        self.fme7.bank_6_is_ram = (data & 0x40) != 0;
                        self.fme7.bank_6_is_ram_enabled = (data & 0x80) != 0;
                    }
                    0x9 => self.fme7.bank_8 = data & 0x3F,
                    0xA => self.fme7.bank_a = data & 0x3F,
                    0xB => self.fme7.bank_c = data & 0x3F,
                    0xC => self.fme7.mirr = data,
                    0xD => {
                        self.fme7.irq_control = data;
                        self.irq_ack_pending = true;
                    }
                    0xE => self.fme7.counter = (self.fme7.counter & 0xFF00) | (data as u16),
                    0xF => self.fme7.counter = (self.fme7.counter & 0x00FF) | ((data as u16) << 8),
                    _ => {}
                },
                0xC | 0xD => {
                    self.fme7.sndcmd = data % 14;
                }
                0xE | 0xF => {
                    self.fme7.sound_data_write(data);
                }
                _ => {}
            },
            0x00 | 0x01 | 0x32 => {
                self.mmc1.write(bank, data);
            }
            0x0A => match bank {
                0xA => self.mmc2.prg = data,
                0xB..=0xE => self.mmc2.chr[bank - 0xB] = data,
                0xF => self.mmc2.mirroring = data,
                _ => {}
            },
            0x08 => match bank {
                0xA => self.mmc4.prg = data,
                0xB..=0xE => self.mmc4.chr[bank - 0xB] = data,
                0xF => self.mmc4.mirroring = data,
                _ => {}
            },
            0x09 | 0x0B | 0x17 | 0x37 => {
                if (self.flags() & 8) != 0 {
                    self.latch.addr = address;
                    self.latch.data = data;
                } else if bank <= 0xB {
                    self.reg[0] = data as u16;
                } else {
                    self.reg[2] = data as u16;
                }
            }
            0x04 | 0x06 | 0x14 | 0x16 => {
                self.latch.data = data;
            }
            0x05 | 0x15 => {
                if bank <= 0xB {
                    self.reg[0] = data as u16;
                } else if bank == 0x9 && self.flags() == 0 {
                    self.reg[1] = data as u16;
                } else if bank >= 0xE {
                    self.reg[2] = data as u16;
                }
            }
            0x0C | 0x0D | 0x1C | 0x1D => {
                self.latch.data = data;
            }
            0x20 | 0x21 | 0x22 | 0x23 => {
                let a0 = self.vrc24.a0 as u16;
                let a1 = self.vrc24.a1 as u16;
                let index = if address & a0 != 0 { 1 } else { 0 } | if address & a1 != 0 { 2 } else { 0 };
                match bank {
                    0x8 | 0xA => {
                        self.vrc24.prg[((bank >> 1) & 1) as usize] = data;
                    }
                    0x9 => {
                        if !self.vrc24.is_vrc4 || index == 0 {
                            self.vrc24.mirroring = data;
                        } else if self.vrc24.is_vrc4 && index == 2 {
                            self.vrc24.misc = data;
                        }
                    }
                    0xF => {
                        if self.vrc24.is_vrc4 {
                            match index & 3 {
                                0 => self.vrc24.latch = (self.vrc24.latch & 0xF0) | (data & 0x0F),
                                1 => self.vrc24.latch = (self.vrc24.latch & 0x0F) | (data << 4),
                                2 => {
                                    self.vrc24.mode = data;
                                    if self.vrc24.mode & 0x02 != 0 {
                                        self.vrc24.counter = self.vrc24.latch;
                                        self.vrc24.cycles = 341;
                                    }
                                    self.irq_ack_pending = true;
                                }
                                3 => {
                                    self.vrc24.mode = (self.vrc24.mode & !0x02) | ((self.vrc24.mode << 1) & 0x02);
                                    self.irq_ack_pending = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {
                        let reg = (((bank as u16) - 0xB) << 1) | if address & a1 != 0 { 1 } else { 0 };
                        let reg = reg as usize;
                        if address & a0 != 0 {
                            self.vrc24.chr[reg] = (self.vrc24.chr[reg] & 0x00F) | ((data as u16) << 4);
                        } else {
                            self.vrc24.chr[reg] = (self.vrc24.chr[reg] & 0xFF0) | ((data & 0x0F) as u16);
                        }
                    }
                }
            }
            0x30 | 0x31 => {
                let a0 = self.vrc6.a0 as u16;
                let a1 = self.vrc6.a1 as u16;
                let reg = if address & a1 != 0 { 2 } else { 0 } | if address & a0 != 0 { 1 } else { 0 };
                match bank {
                    0x8 => self.vrc6.prg[0] = data,
                    0xB => {
                        if reg == 3 {
                            self.vrc6.mode = data;
                        }
                    }
                    0xC => self.vrc6.prg[1] = data,
                    0xD | 0xE => {
                        let idx = (((bank - 0xD) << 2) | if address & a1 != 0 { 2 } else { 0 } | if address & a0 != 0 { 1 } else { 0 }) as usize;
                        if idx < 8 {
                            self.vrc6.chr[idx] = data;
                        }
                    }
                    0xF => match reg {
                        0 => self.vrc6.irq_latch = data,
                        1 => {
                            self.vrc6.irq_control = data;
                            if (self.vrc6.irq_control & 2) != 0 {
                                self.vrc6.irq_counter = self.vrc6.irq_latch;
                                self.vrc6.irq_cycles = 341;
                            }
                            self.irq_ack_pending = true;
                        }
                        2 => {
                            if (self.vrc6.irq_control & 1) != 0 {
                                self.vrc6.irq_control |= 2;
                            } else {
                                self.vrc6.irq_control &= !2;
                            }
                            self.irq_ack_pending = true;
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            0x40 => match bank {
                0x8 | 0xA | 0xB | 0xC | 0xD => {
                    self.vrc1.prg[((bank >> 1) & 3) as usize] = data;
                }
                0x9 => self.vrc1.misc = data,
                0xE | 0xF => self.vrc1.chr[bank & 1] = data,
                _ => {}
            },
            0x41 => {
                let a0 = self.vrc7.a0 as u16;
                let a1 = self.vrc7.a1 as u16;
                match bank {
                    0x8 | 0x9 => {
                        let reg = ((bank << 1) & 2) | if address & a0 != 0 { 1 } else { 0 };
                        if reg < 3 {
                            self.vrc7.prg[reg as usize] = data;
                        } else {
                            self.vrc7.core.write_sound(address & a1 != 0, data);
                        }
                    }
                    0xA..=0xD => {
                        let reg = (((bank - 0xA) << 1) | if address & a0 != 0 { 1 } else { 0 }) as usize;
                        if reg < 8 {
                            self.vrc7.chr[reg] = data;
                        }
                    }
                    0xE => {
                        if address & a0 != 0 {
                            self.vrc7.latch = data;
                        } else {
                            self.vrc7.misc = data;
                        }
                    }
                    0xF => {
                        if address & a0 != 0 {
                            self.vrc7.irq = (self.vrc7.irq & !2) | ((self.vrc7.irq << 1) & 2);
                        } else {
                            self.vrc7.irq = data;
                            if (self.vrc7.irq & 2) != 0 {
                                self.vrc7.counter = self.vrc7.latch;
                                self.vrc7.cycles = 341;
                            }
                        }
                        self.irq_ack_pending = true;
                    }
                    _ => {}
                }
            }
            0x44 => match bank {
                0x8..=0xB => {
                    let val = data & 0x0F;
                    let shift = (bank << 2) & 0xC;
                    self.vrc3.latch = (self.vrc3.latch & !(0xF << shift)) | ((val as u16) << shift);
                }
                0xC => {
                    self.vrc3.irq = data;
                    if self.vrc3.irq & 2 != 0 {
                        self.vrc3.counter = self.vrc3.latch;
                    }
                    self.irq_ack_pending = true;
                }
                0xD => {
                    self.vrc3.irq = (self.vrc3.irq & !0x02) | ((self.vrc3.irq << 1) & 0x01);
                    self.irq_ack_pending = true;
                }
                0xF => self.reg[3] = data as u16,
                _ => {}
            },
            0x0E | 0x1E => {}
            0x07 => {
                if bank <= 0xB {
                    self.reg[0] = if (bank & 2) != 0 { 1 } else { 0 };
                } else if bank >= 0xE {
                    self.reg[2] = data as u16;
                }
            }
            0x10 | 0x11 | 0x12 | _ => match bank & !1 {
                0x8 => {
                    if (address & 1) != 0 {
                        self.mmc3.reg[self.mmc3.index as usize & 7] = data;
                    } else {
                        self.mmc3.index = data;
                    }
                }
                0xA => {
                    if (address & 1) != 0 {
                        self.mmc3.wram_control = data;
                    } else {
                        self.mmc3.mirroring = data;
                    }
                }
                0xC => {
                    if (address & 1) != 0 {
                        self.mmc3.counter = 0;
                        self.mmc3.reload = true;
                    } else {
                        self.mmc3.reload_value = data;
                    }
                }
                0xE => {
                    self.mmc3.enable_irq = (address & 1) != 0;
                    if !self.mmc3.enable_irq {
                        self.irq_ack_pending = true;
                    }
                }
                _ => {}
            },
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
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        let fusemap = self.fusemap();

        if address < 0x2000 {
            if fusemap == 0x0E || fusemap == 0x1E {
                self.check_chr_split(address);
            }
            let bank_1k = ((address >> 10) & 7) as usize;
            let effective_bank = if (fusemap == 0x0E || fusemap == 0x1E)
                && (self.reg[0] & 0x80) != 0
                && !self.pa13
            {
                (bank_1k & 3) | (if self.pa09 { 4 } else { 0 })
            } else {
                bank_1k
            };

            let slots = self.chr_1k_slots(chr_rom.len());
            let mapped_bank = slots[effective_bank];
            let offset = mapped_bank * 0x400 + (address as usize & 0x3FF);

            let byte = if using_chr_ram || chr_rom.is_empty() {
                if !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else {
                    0
                }
            } else {
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;

            if fusemap == 0x0A {
                let cmp = address & if (address & 0x1000) != 0 { 0x3F8 } else { 0x3FF };
                let idx = ((address >> 12) & 1) as usize;
                if cmp == 0x3D8 {
                    self.mmc2.state[idx] = 0;
                } else if cmp == 0x3E8 {
                    self.mmc2.state[idx] = 1;
                }
            } else if fusemap == 0x08 {
                let cmp = address & 0x3F8;
                let idx = ((address >> 12) & 1) as usize;
                if cmp == 0x3D8 {
                    self.mmc4.state[idx] = 0;
                } else if cmp == 0x3E8 {
                    self.mmc4.state[idx] = 1;
                }
            }
        } else {
            if fusemap == 0x0E || fusemap == 0x1E {
                self.check_chr_split(address);
            }
            let mirrored = self.mirror_address(address);
            let idx = (mirrored & 0x7FF) as usize;
            new_addr_bus |= vram[idx] as u16;
        }

        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        let fusemap = self.fusemap();
        if address < 0x2000 {
            if fusemap == 0x0E || fusemap == 0x1E {
                self.check_chr_split(address);
            }
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let bank_1k = ((address >> 10) & 7) as usize;
                let slots = self.chr_1k_slots(cart.chr_rom.len());
                let mapped_bank = slots[bank_1k];
                let offset = mapped_bank * 0x400 + (address as usize & 0x3FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
            if fusemap == 0x0A {
                let cmp = address & if (address & 0x1000) != 0 { 0x3F8 } else { 0x3FF };
                let idx = ((address >> 12) & 1) as usize;
                if cmp == 0x3D8 {
                    self.mmc2.state[idx] = 0;
                } else if cmp == 0x3E8 {
                    self.mmc2.state[idx] = 1;
                }
            } else if fusemap == 0x08 {
                let cmp = address & 0x3F8;
                let idx = ((address >> 12) & 1) as usize;
                if cmp == 0x3D8 {
                    self.mmc4.state[idx] = 0;
                } else if cmp == 0x3E8 {
                    self.mmc4.state[idx] = 1;
                }
            }
        } else if (0x2000..0x3F00).contains(&address) {
            if fusemap == 0x0E || fusemap == 0x1E {
                self.check_chr_split(address);
            }
            let mirrored = self.mirror_address(address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        let fusemap = self.fusemap();
        match fusemap {
            0x50 => {
                self.fme7.tick_audio();
                if self.fme7.irq_control & 0x80 != 0 {
                    for _ in 0..cycles {
                        self.fme7.counter = self.fme7.counter.wrapping_sub(1);
                        if self.fme7.counter == 0 && (self.fme7.irq_control & 0x01) != 0 {
                            return true;
                        }
                    }
                }
                false
            }
            0x20..=0x23 => {
                if self.vrc24.mode & 2 != 0 && (self.vrc24.mode & 4 != 0 || {
                    self.vrc24.cycles = self.vrc24.cycles.wrapping_sub(3 * (cycles as i16));
                    self.vrc24.cycles <= 0
                }) {
                    if self.vrc24.mode & 4 == 0 {
                        self.vrc24.cycles += 341;
                    }
                    self.vrc24.counter = self.vrc24.counter.wrapping_add(1);
                    if self.vrc24.counter == 0 {
                        self.vrc24.counter = self.vrc24.latch;
                        return true;
                    }
                }
                false
            }
            0x30 | 0x31 => {
                if self.vrc6.irq_control & 0x02 != 0 {
                    let step = 3 * (cycles as i16);
                    if self.vrc6.irq_control & 0x04 != 0 || {
                        self.vrc6.irq_cycles = self.vrc6.irq_cycles.wrapping_sub(step);
                        self.vrc6.irq_cycles <= 0
                    } {
                        if self.vrc6.irq_control & 0x04 == 0 {
                            self.vrc6.irq_cycles += 341;
                        }
                        if self.vrc6.irq_counter == 0xFF {
                            self.vrc6.irq_counter = self.vrc6.irq_latch;
                            return true;
                        } else {
                            self.vrc6.irq_counter = self.vrc6.irq_counter.wrapping_add(1);
                        }
                    }
                }
                false
            }
            0x41 => {
                self.vrc7.core.clock_audio(cycles);
                if self.vrc7.irq & 2 != 0 {
                    if self.vrc7.irq & 4 != 0 {
                        for _ in 0..cycles {
                            if self.vrc7.counter == 0xFF {
                                self.vrc7.counter = self.vrc7.latch;
                                return true;
                            }
                            self.vrc7.counter = self.vrc7.counter.wrapping_add(1);
                        }
                    } else {
                        for _ in 0..cycles {
                            self.vrc7.cycles -= 3;
                            if self.vrc7.cycles <= 0 {
                                self.vrc7.cycles += 341;
                                if self.vrc7.counter == 0xFF {
                                    self.vrc7.counter = self.vrc7.latch;
                                    return true;
                                }
                                self.vrc7.counter = self.vrc7.counter.wrapping_add(1);
                            }
                        }
                    }
                }
                false
            }
            0x44 => {
                if self.vrc3.irq & 2 != 0 {
                    let mask = if self.vrc3.irq & 4 != 0 { 0xFF } else { 0xFFFF };
                    for _ in 0..cycles {
                        if (self.vrc3.counter & mask) == mask {
                            self.vrc3.counter = self.vrc3.latch;
                            return true;
                        } else {
                            self.vrc3.counter = self.vrc3.counter.wrapping_add(1);
                        }
                    }
                }
                false
            }
            0x07 => {
                if self.reg[0] != 0 {
                    for _ in 0..cycles {
                        self.reg[1] = self.reg[1].wrapping_add(1);
                        if (self.reg[1] & 0x1000) != 0 {
                            return true;
                        }
                    }
                } else {
                    self.reg[1] = 0;
                }
                false
            }
            _ => {
                if self.mmc3.pa12_filter > 0 {
                    self.mmc3.pa12_filter = self.mmc3.pa12_filter.saturating_sub(cycles);
                }
                false
            }
        }
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        _scanline: u16,
        _dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        let fusemap = self.fusemap();
        match fusemap {
            0x10 | 0x11 | 0x12 | _ if fusemap >= 0x10 && fusemap <= 0x12 => {
                if (ppu_address_bus & 0x1000) != 0 {
                    if self.mmc3.pa12_filter == 0 {
                        self.mmc3.pa12_filter = 3;
                        return self.mmc3_clock_counter();
                    }
                    self.mmc3.pa12_filter = 3;
                }
                false
            }
            _ => false,
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack_pending;
        self.irq_ack_pending = false;
        ack
    }

    fn audio_sample(&self) -> f32 {
        let fusemap = self.fusemap();
        match fusemap {
            0x50 => self.fme7.current_audio_sample,
            0x41 => self.vrc7.core.get_audio_sample() * (if (self.vrc7.misc & 0x40) != 0 { 0.0 } else { 1.0 }),
            _ => 0.0,
        }
    }

    fn expansion_audio_type(&self) -> crate::mapper::ExpansionAudioType {
        match self.fusemap() {
            0x50 => crate::mapper::ExpansionAudioType::Sunsoft5b,
            0x41 => crate::mapper::ExpansionAudioType::Vrc7,
            _ => crate::mapper::ExpansionAudioType::None,
        }
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        for r in &self.reg {
            state.extend_from_slice(&r.to_le_bytes());
        }
        state.extend_from_slice(&self.extra.to_le_bytes());
        state.extend_from_slice(&self.scratch_rom.scratch_data);
        state.push(self.scratch_rom.command);
        state.push(self.scratch_rom.state);
        state.push(self.scratch_rom.clock as u8);
        state.push(self.scratch_rom.data as u8);
        state.push(self.scratch_rom.output as u8);
        state.push(self.pa09 as u8);
        state.push(self.pa13 as u8);
        state.push(self.irq_ack_pending as u8);

        state.extend_from_slice(&self.mmc1.reg);
        state.push(self.mmc1.shift);
        state.push(self.mmc1.bits);
        state.push(self.mmc1.filter);

        state.push(self.mmc2.prg);
        state.extend_from_slice(&self.mmc2.chr);
        state.extend_from_slice(&self.mmc2.state);
        state.push(self.mmc2.mirroring);

        state.push(self.mmc3.index);
        state.extend_from_slice(&self.mmc3.reg);
        state.push(self.mmc3.mirroring);
        state.push(self.mmc3.wram_control);
        state.push(self.mmc3.enable_irq as u8);
        state.push(self.mmc3.reload as u8);
        state.push(self.mmc3.counter);
        state.push(self.mmc3.reload_value);
        state.push(self.mmc3.pa12_filter);

        state.push(self.mmc4.prg);
        state.extend_from_slice(&self.mmc4.chr);
        state.extend_from_slice(&self.mmc4.state);
        state.push(self.mmc4.mirroring);

        state.extend_from_slice(&self.vrc1.prg);
        state.extend_from_slice(&self.vrc1.chr);
        state.push(self.vrc1.misc);

        state.extend_from_slice(&self.vrc24.prg);
        for c in &self.vrc24.chr {
            state.extend_from_slice(&c.to_le_bytes());
        }
        state.push(self.vrc24.mirroring);
        state.push(self.vrc24.latch);
        state.push(self.vrc24.mode);
        state.push(self.vrc24.counter);
        state.extend_from_slice(&self.vrc24.cycles.to_le_bytes());
        state.push(self.vrc24.misc);
        state.push(self.vrc24.is_vrc4 as u8);
        state.push(self.vrc24.a0);
        state.push(self.vrc24.a1);

        state.extend_from_slice(&self.vrc6.prg);
        state.extend_from_slice(&self.vrc6.chr);
        state.push(self.vrc6.mode);
        state.push(self.vrc6.irq_control);
        state.push(self.vrc6.irq_counter);
        state.push(self.vrc6.irq_latch);
        state.extend_from_slice(&self.vrc6.irq_cycles.to_le_bytes());
        state.push(self.vrc6.a0);
        state.push(self.vrc6.a1);

        state.extend_from_slice(&self.vrc7.prg);
        state.extend_from_slice(&self.vrc7.chr);
        state.push(self.vrc7.misc);
        state.push(self.vrc7.irq);
        state.push(self.vrc7.counter);
        state.push(self.vrc7.latch);
        state.extend_from_slice(&self.vrc7.cycles.to_le_bytes());
        state.push(self.vrc7.a0);
        state.push(self.vrc7.a1);
        state.extend_from_slice(&self.vrc7.core.save_mapper_registers(cart));

        state.push(self.fme7.cmd);
        state.extend_from_slice(&self.fme7.chr_1k);
        state.push(self.fme7.bank_6);
        state.push(self.fme7.bank_6_is_ram as u8);
        state.push(self.fme7.bank_6_is_ram_enabled as u8);
        state.push(self.fme7.bank_8);
        state.push(self.fme7.bank_a);
        state.push(self.fme7.bank_c);
        state.push(self.fme7.mirr);
        state.push(self.fme7.irq_control);
        state.extend_from_slice(&self.fme7.counter.to_le_bytes());
        state.push(self.fme7.sndcmd);
        state.extend_from_slice(&self.fme7.sreg);
        for i in 0..3 {
            state.extend_from_slice(&self.fme7.vcount[i].to_le_bytes());
            state.push(self.fme7.dcount[i]);
        }

        state.push(self.vrc3.irq);
        state.extend_from_slice(&self.vrc3.counter.to_le_bytes());
        state.extend_from_slice(&self.vrc3.latch.to_le_bytes());

        state.extend_from_slice(&self.latch.addr.to_le_bytes());
        state.push(self.latch.data);

        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 16 + 2 + 16 + 5 + 3 <= state.len() {
            for i in 0..8 {
                self.reg[i] = u16::from_le_bytes([state[start], state[start + 1]]);
                start += 2;
            }
            self.extra = u16::from_le_bytes([state[start], state[start + 1]]);
            start += 2;
            self.scratch_rom.scratch_data.copy_from_slice(&state[start..start + 16]);
            start += 16;
            self.scratch_rom.command = state[start];
            start += 1;
            self.scratch_rom.state = state[start];
            start += 1;
            self.scratch_rom.clock = state[start] != 0;
            start += 1;
            self.scratch_rom.data = state[start] != 0;
            start += 1;
            self.scratch_rom.output = state[start] != 0;
            start += 1;
            self.pa09 = state[start] != 0;
            start += 1;
            self.pa13 = state[start] != 0;
            start += 1;
            self.irq_ack_pending = state[start] != 0;
            start += 1;

            if start + 7 + 8 + 17 + 8 + 6 + 29 + 17 + 17 <= state.len() {
                self.mmc1.reg.copy_from_slice(&state[start..start + 4]);
                start += 4;
                self.mmc1.shift = state[start];
                start += 1;
                self.mmc1.bits = state[start];
                start += 1;
                self.mmc1.filter = state[start];
                start += 1;

                self.mmc2.prg = state[start];
                start += 1;
                self.mmc2.chr.copy_from_slice(&state[start..start + 4]);
                start += 4;
                self.mmc2.state.copy_from_slice(&state[start..start + 2]);
                start += 2;
                self.mmc2.mirroring = state[start];
                start += 1;

                self.mmc3.index = state[start];
                start += 1;
                self.mmc3.reg.copy_from_slice(&state[start..start + 8]);
                start += 8;
                self.mmc3.mirroring = state[start];
                start += 1;
                self.mmc3.wram_control = state[start];
                start += 1;
                self.mmc3.enable_irq = state[start] != 0;
                start += 1;
                self.mmc3.reload = state[start] != 0;
                start += 1;
                self.mmc3.counter = state[start];
                start += 1;
                self.mmc3.reload_value = state[start];
                start += 1;
                self.mmc3.pa12_filter = state[start];
                start += 1;

                self.mmc4.prg = state[start];
                start += 1;
                self.mmc4.chr.copy_from_slice(&state[start..start + 4]);
                start += 4;
                self.mmc4.state.copy_from_slice(&state[start..start + 2]);
                start += 2;
                self.mmc4.mirroring = state[start];
                start += 1;

                self.vrc1.prg.copy_from_slice(&state[start..start + 3]);
                start += 3;
                self.vrc1.chr.copy_from_slice(&state[start..start + 2]);
                start += 2;
                self.vrc1.misc = state[start];
                start += 1;

                self.vrc24.prg.copy_from_slice(&state[start..start + 2]);
                start += 2;
                for i in 0..8 {
                    self.vrc24.chr[i] = u16::from_le_bytes([state[start], state[start + 1]]);
                    start += 2;
                }
                self.vrc24.mirroring = state[start];
                start += 1;
                self.vrc24.latch = state[start];
                start += 1;
                self.vrc24.mode = state[start];
                start += 1;
                self.vrc24.counter = state[start];
                start += 1;
                self.vrc24.cycles = i16::from_le_bytes([state[start], state[start + 1]]);
                start += 2;
                self.vrc24.misc = state[start];
                start += 1;
                self.vrc24.is_vrc4 = state[start] != 0;
                start += 1;
                self.vrc24.a0 = state[start];
                start += 1;
                self.vrc24.a1 = state[start];
                start += 1;

                self.vrc6.prg.copy_from_slice(&state[start..start + 2]);
                start += 2;
                self.vrc6.chr.copy_from_slice(&state[start..start + 8]);
                start += 8;
                self.vrc6.mode = state[start];
                start += 1;
                self.vrc6.irq_control = state[start];
                start += 1;
                self.vrc6.irq_counter = state[start];
                start += 1;
                self.vrc6.irq_latch = state[start];
                start += 1;
                self.vrc6.irq_cycles = i16::from_le_bytes([state[start], state[start + 1]]);
                start += 2;
                self.vrc6.a0 = state[start];
                start += 1;
                self.vrc6.a1 = state[start];
                start += 1;

                self.vrc7.prg.copy_from_slice(&state[start..start + 3]);
                start += 3;
                self.vrc7.chr.copy_from_slice(&state[start..start + 8]);
                start += 8;
                self.vrc7.misc = state[start];
                start += 1;
                self.vrc7.irq = state[start];
                start += 1;
                self.vrc7.counter = state[start];
                start += 1;
                self.vrc7.latch = state[start];
                start += 1;
                self.vrc7.cycles = i16::from_le_bytes([state[start], state[start + 1]]);
                start += 2;
                self.vrc7.a0 = state[start];
                start += 1;
                self.vrc7.a1 = state[start];
                start += 1;
                start = self.vrc7.core.load_mapper_registers(cart, state, start);

                if start + 1 + 8 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 2 + 1 + 14 + 15 + 5 + 3 <= state.len() {
                    self.fme7.cmd = state[start];
                    start += 1;
                    self.fme7.chr_1k.copy_from_slice(&state[start..start + 8]);
                    start += 8;
                    self.fme7.bank_6 = state[start];
                    start += 1;
                    self.fme7.bank_6_is_ram = state[start] != 0;
                    start += 1;
                    self.fme7.bank_6_is_ram_enabled = state[start] != 0;
                    start += 1;
                    self.fme7.bank_8 = state[start];
                    start += 1;
                    self.fme7.bank_a = state[start];
                    start += 1;
                    self.fme7.bank_c = state[start];
                    start += 1;
                    self.fme7.mirr = state[start];
                    start += 1;
                    self.fme7.irq_control = state[start];
                    start += 1;
                    self.fme7.counter = u16::from_le_bytes([state[start], state[start + 1]]);
                    start += 2;
                    self.fme7.sndcmd = state[start];
                    start += 1;
                    self.fme7.sreg.copy_from_slice(&state[start..start + 14]);
                    start += 14;
                    for i in 0..3 {
                        self.fme7.vcount[i] = i32::from_le_bytes([state[start], state[start + 1], state[start + 2], state[start + 3]]);
                        start += 4;
                        self.fme7.dcount[i] = state[start];
                        start += 1;
                    }

                    self.vrc3.irq = state[start];
                    start += 1;
                    self.vrc3.counter = u16::from_le_bytes([state[start], state[start + 1]]);
                    start += 2;
                    self.vrc3.latch = u16::from_le_bytes([state[start], state[start + 1]]);
                    start += 2;

                    self.latch.addr = u16::from_le_bytes([state[start], state[start + 1]]);
                    start += 2;
                    self.latch.data = state[start];
                    start += 1;
                }
            }
        }
        start
    }
}

