use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const UPD7756_STEP: [[i16; 16]; 16] = [
    [ 0,  0,  1,  2,  3,   5,   7,  10,  0,   0,  -1,  -2,  -3,   -5,   -7,  -10 ],
    [ 0,  1,  2,  3,  4,   6,   8,  13,  0,  -1,  -2,  -3,  -4,   -6,   -8,  -13 ],
    [ 0,  1,  2,  4,  5,   7,  10,  15,  0,  -1,  -2,  -4,  -5,   -7,  -10,  -15 ],
    [ 0,  1,  3,  4,  6,   9,  13,  19,  0,  -1,  -3,  -4,  -6,   -9,  -13,  -19 ],
    [ 0,  2,  3,  5,  8,  11,  15,  23,  0,  -2,  -3,  -5,  -8,  -11,  -15,  -23 ],
    [ 0,  2,  4,  7, 10,  14,  19,  29,  0,  -2,  -4,  -7, -10,  -14,  -19,  -29 ],
    [ 0,  3,  5,  8, 12,  16,  22,  33,  0,  -3,  -5,  -8, -12,  -16,  -22,  -33 ],
    [ 1,  4,  7, 10, 15,  20,  29,  43, -1,  -4,  -7, -10, -15,  -20,  -29,  -43 ],
    [ 1,  4,  8, 13, 18,  25,  35,  53, -1,  -4,  -8, -13, -18,  -25,  -35,  -53 ],
    [ 1,  6, 10, 16, 22,  31,  43,  64, -1,  -6, -10, -16, -22,  -31,  -43,  -64 ],
    [ 2,  7, 12, 19, 27,  37,  51,  76, -2,  -7, -12, -19, -27,  -37,  -51,  -76 ],
    [ 2,  9, 16, 24, 34,  46,  64,  96, -2,  -9, -16, -24, -34,  -46,  -64,  -96 ],
    [ 3, 11, 19, 29, 41,  57,  79, 117, -3, -11, -19, -29, -41,  -57,  -79, -117 ],
    [ 4, 13, 24, 36, 50,  69,  96, 143, -4, -13, -24, -36, -50,  -69,  -96, -143 ],
    [ 4, 16, 29, 44, 62,  85, 118, 175, -4, -16, -29, -44, -62,  -85, -118, -175 ],
    [ 6, 20, 36, 54, 76, 104, 144, 214, -6, -20, -36, -54, -76, -104, -144, -214 ],
];
const UPD7756_STATE_TABLE: [i16; 16] = [ -1, -1, 0, 0, 1, 2, 2, 3, -1, -1, 0, 0, 1, 2, 2, 3 ];
struct SpeechSample {
    pcm: Vec<i16>,
    sample_rate: u32,
}

pub struct Mapper86 {
    prg_bank: u8,
    chr_bank: u8,
    speech: u8,
    playing: bool,
    samples: Vec<SpeechSample>,
    current_track: usize,
    current_idx: usize,
    audio_cycles: f64,
    current_audio_sample: f32,
    cpu_clock: f64,
}

impl Mapper86 {
    pub fn new(misc_rom: Vec<u8>) -> Self {
        let mut samples = Vec::new();
        if misc_rom.len() > 5 && misc_rom[1] == 0x5A && misc_rom[2] == 0xA5 && misc_rom[3] == 0x69 && misc_rom[4] == 0x55 {
            let samples_count = (misc_rom[0] as usize) + 1;
            for i in 0..samples_count {
                let start_idx = i * 2 + 5;
                if start_idx + 1 < misc_rom.len() {
                    let offset = ((misc_rom[start_idx] as usize) << 9) | ((misc_rom[start_idx + 1] as usize) << 1);
                    let sample_start = offset + 1;
                    if sample_start < misc_rom.len() {
                        let sample = Self::decode_sample(&misc_rom[sample_start..]);
                        samples.push(sample);
                    }
                }
            }
        }
        Self {
            prg_bank: 0,
            chr_bank: 0,
            speech: 0,
            playing: false,
            samples,
            current_track: 0,
            current_idx: 0,
            audio_cycles: 0.0,
            current_audio_sample: 0.0,
            cpu_clock: 1_789_773.0,
        }
    }

    fn decode_sample(data: &[u8]) -> SpeechSample {
        let mut pcm = Vec::new();
        let mut divider = 0;
        let mut sample_rate = 8000;
        let mut sample: i16 = 0;
        let mut state: usize = 0;
        let mut repeat_count = 0;
        let mut initial_silence = 0;
        let mut ptr = 0;
        let mut repeat_offset = 0;
        while ptr < data.len() && data[ptr] != 0x00 {
            let cmd_byte = data[ptr];
            ptr += 1;
            let command = cmd_byte >> 6;
            let parameter = cmd_byte & 0x3F;
            let mut silence = 0;
            let mut nibbles = 0;
            match command {
                0 => {
                    silence = 256 * (parameter as i32 + 1);
                    sample = 0;
                    state = 0;
                }
                1 => {
                    divider = parameter as i32 + 1;
                    nibbles = 256;
                }
                2 => {
                    divider = parameter as i32 + 1;
                    if ptr < data.len() {
                        nibbles = data[ptr] as i32 + 1;
                        ptr += 1;
                    }
                }
                3 => {
                    repeat_count = (parameter & 7) + 1;
                    repeat_offset = ptr;
                }
                _ => {}
            }
            for j in 0..nibbles {
                if ptr >= data.len() {
                    break;
                }
                let nibble = if (j & 1) != 0 {
                    let val = data[ptr] & 0x0F;
                    ptr += 1;
                    val
                } else {
                    data[ptr] >> 4
                } as usize;
                sample += UPD7756_STEP[state][nibble];
                let next_state = state as i16 + UPD7756_STATE_TABLE[nibble];
                state = next_state.clamp(0, 15) as usize;
                pcm.push((sample << 7) | (sample & 0x7F));
            }
            if (nibbles & 1) != 0 {
                ptr += 1;
            }
            if silence > 0 {
                if divider > 0 {
                    let count = silence / divider;
                    for _ in 0..count {
                        pcm.push(0);
                    }
                } else {
                    initial_silence += silence;
                }
            }
            if repeat_count > 0 {
                repeat_count -= 1;
                ptr = repeat_offset;
            }
            if divider > 0 {
                sample_rate = 160000 / divider as u32;
            }
        }
        if initial_silence > 0 && divider > 0 {
            let count = (initial_silence / divider) as usize;
            for _ in 0..count {
                pcm.insert(0, 0);
            }
        }
        SpeechSample { pcm, sample_rate }
    }
}

impl Mapper for Mapper86 {
    fn set_cpu_clock(&mut self, clock: f64) {
        self.cpu_clock = clock;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let offset = (self.prg_bank as usize * 0x8000) + (address as usize & 0x7FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        let is_reg_write = (address >= 0x6000 && address < 0x8000) || (address >= 0xE000);
        if is_reg_write {
            match address & 0x7000 {
                0x6000 => {
                    self.prg_bank = (data & 0x30) >> 4;
                    self.chr_bank = (data & 0x03) | ((data >> 4) & 0x04);
                }
                0x7000 => {
                    self.speech = data;
                    let speech_start = (self.speech & 0x10) == 0;
                    let speech_stop = (self.speech & 0x20) == 0;
                    let track = (self.speech & 0x0F) as usize;
                    if speech_start {
                        if track < self.samples.len() && !self.samples[track].pcm.is_empty() {
                            self.current_track = track;
                            self.current_idx = 0;
                            self.playing = true;
                            self.audio_cycles = 0.0;
                        }
                    }
                    if speech_stop {
                        self.playing = false;
                    }
                }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.nametable_horizontal_mirroring {
            (address & 0x3FFF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
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
            let offset = (self.chr_bank as usize * 0x2000) + (address as usize & 0x1FFF);
            if using_chr_ram || chr_rom.is_empty() {
                new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
            } else {
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else {
            let mirrored = if _nametable_horizontal_mirroring {
                (address & 0x3FFF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let idx = (mirrored & 0x7FF) as usize;
            new_addr_bus |= vram[idx] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.playing {
            let sample = &self.samples[self.current_track];
            let cycles_per_sample = self.cpu_clock / (sample.sample_rate as f64);
            self.audio_cycles += cycles as f64;
            while self.audio_cycles >= cycles_per_sample {
                self.audio_cycles -= cycles_per_sample;
                self.current_idx += 1;
                if self.current_idx >= sample.pcm.len() {
                    self.playing = false;
                    self.current_audio_sample = 0.0;
                    break;
                }
            }
            if self.playing {
                self.current_audio_sample = (sample.pcm[self.current_idx] as f32) / 32768.0;
            }
        } else {
            self.current_audio_sample = 0.0;
        }
        false
    }

    fn audio_sample(&self) -> f32 {
        self.current_audio_sample * 1.5
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = vec![self.prg_bank, self.chr_bank, self.speech, self.playing as u8];
        state.extend_from_slice(&(self.current_track as u64).to_le_bytes());
        state.extend_from_slice(&(self.current_idx as u64).to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 4 + 8 + 8 <= state.len() {
            self.prg_bank = state[start];
            self.chr_bank = state[start + 1];
            self.speech = state[start + 2];
            self.playing = state[start + 3] != 0;
            start += 4;
            let mut track_bytes = [0u8; 8];
            track_bytes.copy_from_slice(&state[start..start + 8]);
            self.current_track = u64::from_le_bytes(track_bytes) as usize;
            start += 8;
            let mut idx_bytes = [0u8; 8];
            idx_bytes.copy_from_slice(&state[start..start + 8]);
            self.current_idx = u64::from_le_bytes(idx_bytes) as usize;
            start += 8;
        }
        start
    }

    fn reset(&mut self) {
        self.prg_bank = 0;
        self.chr_bank = 0;
        self.speech = 0;
        self.playing = false;
        self.current_track = 0;
        self.current_idx = 0;
        self.audio_cycles = 0.0;
        self.current_audio_sample = 0.0;
    }
}
