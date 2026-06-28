use std::collections::VecDeque;
struct LpfRc {
    a0: f64,
    b1: f64,
    z1: f64,
}

impl LpfRc {
    fn new() -> Self {
        LpfRc { a0: 1.0, b1: 0.0, z1: 0.0 }
    }

    fn set_fc(&mut self, fc: f64) {
        self.b1 = (-2.0 * std::f64::consts::PI * fc).exp();
        self.a0 = 1.0 - self.b1;
    }

    fn process(&mut self, input: f64) -> f64 {
        self.z1 = input * self.a0 + self.z1 * self.b1;
        self.z1
    }
}
const INDEX_STEP: [u8; 4] = [0, 0, 3, 5];
const INDEX_TABLE: [u8; 26] = [0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 20, 20, 20, 20];
const STEP_TABLE: [[i8; 21]; 4] = [
    [0, 1, 1, 1, 1, 1, 2, 2, 2, 3, 3, 4, 5, 5, 6, 7, 8, 10, 11, 13, 15],
    [1, 3, 3, 3, 4, 4, 6, 6, 7, 9, 10, 12, 15, 16, 19, 22, 25, 30, 34, 40, 46],
    [3, 5, 5, 6, 7, 8, 10, 11, 13, 16, 18, 21, 25, 28, 32, 38, 43, 51, 58, 68, 78],
    [4, 7, 7, 8, 10, 11, 14, 15, 18, 22, 25, 29, 35, 39, 45, 53, 60, 71, 81, 95, 109],
];

pub struct Adpcm3Bit {
    count: u32,
    period: u32,
    inhibit: u32,
    command: u8,
    latch: u8,
    data: u8,
    bytes_left: u8,
    clock: bool,
    ready: bool,
    playing: bool,
    index: u8,
    output: i8,
    input: Vec<u8>,
    frames: VecDeque<u64>,
    chip_count: i32,
    chip_clock: i32,
    host_clock: i32,
    low_pass: LpfRc,
}

impl Adpcm3Bit {
    pub fn new(chip_clock: i32, host_clock: i32) -> Self {
        let mut s = Adpcm3Bit {
            count: 0,
            period: 512,
            inhibit: 0,
            command: 0xFF,
            latch: 0x00,
            data: 0x0,
            bytes_left: 0,
            clock: false,
            ready: true,
            playing: false,
            index: 0,
            output: 0,
            input: Vec::new(),
            frames: VecDeque::new(),
            chip_count: 0,
            chip_clock,
            host_clock,
            low_pass: LpfRc::new(),
        };
        s.low_pass.set_fc(chip_clock as f64 / 512.0 * 0.425 / host_clock as f64);
        s
    }

    fn decode_sample(&mut self, code: u8) {
        let step = STEP_TABLE[(code & 3) as usize][self.index as usize];
        let predictor = self.output as i16 + step as i16 * if (code & 4) != 0 { -1 } else { 1 };
        self.output = if predictor < -128 { -128 } else if predictor > 127 { 127 } else { predictor as i8 };
        self.index = INDEX_TABLE[(self.index as usize + INDEX_STEP[(code & 3) as usize] as usize) % 26];
    }

    pub fn reset(&mut self) {
        self.chip_count = 0;
        self.count = 0;
        self.period = 512;
        self.inhibit = 0;
        self.command = 0xFF;
        self.latch = 0x00;
        self.data = 0x0;
        self.bytes_left = 0;
        self.clock = false;
        self.ready = true;
        self.index = 0;
        self.output = 0;
        self.playing = false;
        self.input.clear();
        self.frames.clear();
    }

    pub fn run(&mut self) {
        self.chip_count += self.chip_clock;
        while self.chip_count >= self.host_clock {
            self.chip_count -= self.host_clock;
            if self.inhibit > 0 { self.inhibit -= 1; }
            if self.playing {
                self.count = self.count.wrapping_add(1);
                if self.count % self.period == 0 {
                    if !self.frames.is_empty() {
                        let frame = self.frames[0];
                        self.decode_sample((frame & 7) as u8);
                        self.frames[0] = frame >> 3;
                    }
                }
                if self.count >= self.period * 21 {
                    self.count = 0;
                    if !self.frames.is_empty() {
                        self.frames.pop_front();
                    }
                }
            }
        }
    }

    pub fn get_ack(&self) -> bool {
        if self.clock && self.inhibit > 0 { true } else { !self.clock }
    }

    pub fn get_audio(&mut self) -> i32 {
        let v = self.output as f64 * 256.0 + 1e-15;
        self.low_pass.process(v) as i32
    }

    pub fn set_clock(&mut self, new_clock: bool) {
        if !self.clock && new_clock {
            self.latch = self.data << 4;
        } else if self.clock && !new_clock && self.inhibit == 0 {
            self.latch |= self.data & 0x0F;
            if self.command == 0x55 && self.latch == 0xAA {
                self.reset();
            } else if self.bytes_left > 0 {
                self.input.push(self.latch);
                self.bytes_left -= 1;
            } else {
                match self.latch {
                    0x03 => {
                        self.bytes_left = 2;
                        self.index = 0;
                        self.output = 0;
                    }
                    0x04 => {
                        self.frames.clear();
                        self.bytes_left = 96;
                    }
                    0x06 => {
                        if self.frames.len() < 12 {
                            self.bytes_left = 8;
                            self.ready = true;
                        }
                    }
                    0x07 => {
                        self.index = 0;
                        self.output = 0;
                        self.frames.clear();
                        self.playing = false;
                    }
                    0x55 => {}
                    _ => {}
                }
                self.command = self.latch;
            }
            if self.bytes_left == 0 {
                match self.command {
                    0x03 => {
                        if self.input.len() >= 2 {
                            let p = self.input[0] as u32 | (self.input[1] as u32) << 8;
                            self.period = if p == 0 { 1 } else { p };
                            self.low_pass.set_fc(self.chip_clock as f64 / self.period as f64 * 0.425 / self.host_clock as f64);
                        }
                    }
                    0x06 => {
                        if self.input.len() >= 8 {
                            let mut frame: u64 = 0;
                            for i in 0..8 {
                                frame |= (self.input[i] as u64) << (i * 8);
                            }
                            if frame & 0x8000_0000_0000_0000 == 0 {
                                self.frames.push_back(frame);
                            }
                            if self.frames.len() >= 12 {
                                self.playing = true;
                                self.ready = false;
                                self.inhibit = 16384;
                            } else {
                                self.bytes_left = 8;
                            }
                        }
                    }
                    _ => {}
                }
                self.input.clear();
            }
        }
        self.clock = new_clock;
    }

    pub fn set_data(&mut self, new_data: u8) {
        self.data = new_data;
    }

    pub fn save(&self) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.count.to_le_bytes());
        state.extend_from_slice(&self.period.to_le_bytes());
        state.extend_from_slice(&self.inhibit.to_le_bytes());
        state.push(self.command);
        state.push(self.latch);
        state.push(self.data);
        state.push(self.bytes_left);
        state.push(if self.clock { 1 } else { 0 });
        state.push(if self.ready { 1 } else { 0 });
        state.push(if self.playing { 1 } else { 0 });
        state.push(self.index);
        state.push(self.output as u8);
        let input_len = self.input.len() as u32;
        state.extend_from_slice(&input_len.to_le_bytes());
        for &b in &self.input {
            state.push(b);
        }
        let frames_len = self.frames.len() as u32;
        state.extend_from_slice(&frames_len.to_le_bytes());
        for &f in &self.frames {
            state.extend_from_slice(&f.to_le_bytes());
        }
        state
    }

    pub fn load(&mut self, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 4 <= state.len() { self.count = u32::from_le_bytes([state[p], state[p+1], state[p+2], state[p+3]]); p += 4; }
        if p + 4 <= state.len() { self.period = u32::from_le_bytes([state[p], state[p+1], state[p+2], state[p+3]]); p += 4; }
        if p + 4 <= state.len() { self.inhibit = u32::from_le_bytes([state[p], state[p+1], state[p+2], state[p+3]]); p += 4; }
        if p < state.len() { self.command = state[p]; p += 1; }
        if p < state.len() { self.latch = state[p]; p += 1; }
        if p < state.len() { self.data = state[p]; p += 1; }
        if p < state.len() { self.bytes_left = state[p]; p += 1; }
        if p < state.len() { self.clock = state[p] != 0; p += 1; }
        if p < state.len() { self.ready = state[p] != 0; p += 1; }
        if p < state.len() { self.playing = state[p] != 0; p += 1; }
        if p < state.len() { self.index = state[p]; p += 1; }
        if p < state.len() { self.output = state[p] as i8; p += 1; }
        if p + 4 <= state.len() {
            let input_len = u32::from_le_bytes([state[p], state[p+1], state[p+2], state[p+3]]) as usize;
            p += 4;
            self.input.clear();
            for _ in 0..input_len.min(96) {
                if p < state.len() { self.input.push(state[p]); p += 1; }
            }
        }
        if p + 4 <= state.len() {
            let frames_len = u32::from_le_bytes([state[p], state[p+1], state[p+2], state[p+3]]) as usize;
            p += 4;
            self.frames.clear();
            for _ in 0..frames_len.min(12) {
                if p + 8 <= state.len() {
                    self.frames.push_back(u64::from_le_bytes([state[p], state[p+1], state[p+2], state[p+3], state[p+4], state[p+5], state[p+6], state[p+7]]));
                    p += 8;
                }
            }
        }
        p
    }
}
