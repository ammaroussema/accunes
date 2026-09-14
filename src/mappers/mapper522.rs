use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

struct FdsSound {
    fout: i32,
    master_io: bool,
    master_vol: u8,
    wave: [[i32; 64]; 2],
    freq: [u32; 2],
    phase: [u32; 2],
    wav_write: bool,
    wav_halt: bool,
    env_halt: bool,
    mod_halt: bool,
    mod_pos: u32,
    mod_write_pos: u32,
    env_mode: [bool; 2],
    env_disable: [bool; 2],
    env_timer: [u32; 2],
    env_speed: [u32; 2],
    env_out: [u32; 2],
    master_env_speed: u8,
    current_audio_sample: f32,
}

impl FdsSound {
    fn new() -> Self {
        let mut s = Self {
            fout: 0,
            master_io: true,
            master_vol: 0,
            wave: [[0; 64]; 2],
            freq: [0; 2],
            phase: [0; 2],
            wav_write: false,
            wav_halt: true,
            env_halt: true,
            mod_halt: true,
            mod_pos: 0,
            mod_write_pos: 0,
            env_mode: [false; 2],
            env_disable: [true; 2],
            env_timer: [0; 2],
            env_speed: [0; 2],
            env_out: [0; 2],
            master_env_speed: 0xFF,
            current_audio_sample: 0.0,
        };
        s.reset();
        s
    }

    fn reset(&mut self) {
        self.master_io = true;
        self.master_vol = 0;
        for i in 0..2 {
            self.wave[i] = [0; 64];
            self.freq[i] = 0;
            self.phase[i] = 0;
        }
        self.wav_write = false;
        self.wav_halt = true;
        self.env_halt = true;
        self.mod_halt = true;
        self.mod_pos = 0;
        self.mod_write_pos = 0;
        for i in 0..2 {
            self.env_mode[i] = false;
            self.env_disable[i] = true;
            self.env_timer[i] = 0;
            self.env_speed[i] = 0;
            self.env_out[i] = 0;
        }
        self.master_env_speed = 0xFF;
        self.fout = 0;
        self.current_audio_sample = 0.0;
        self.write_reg(0x4023, 0x00);
        self.write_reg(0x4023, 0x83);
        self.write_reg(0x4080, 0x80);
        self.write_reg(0x408A, 0xE8);
        self.write_reg(0x4082, 0x00);
        self.write_reg(0x4083, 0x80);
        self.write_reg(0x4084, 0x80);
        self.write_reg(0x4085, 0x00);
        self.write_reg(0x4086, 0x00);
        self.write_reg(0x4087, 0x80);
        self.write_reg(0x4089, 0x00);
    }

    fn run(&mut self, clocks: u32) {
        if clocks == 0 {
            return;
        }
        if !self.env_halt && !self.wav_halt && self.master_env_speed != 0 {
            for i in 0..2 {
                if !self.env_disable[i] {
                    self.env_timer[i] += clocks;
                    let period = ((self.env_speed[i] + 1) * (self.master_env_speed as u32 + 1)) << 3;
                    while self.env_timer[i] >= period {
                        if self.env_mode[i] {
                            if self.env_out[i] < 32 {
                                self.env_out[i] += 1;
                            }
                        } else if self.env_out[i] > 0 {
                            self.env_out[i] -= 1;
                        }
                        self.env_timer[i] -= period;
                    }
                }
            }
        }

        if !self.mod_halt {
            let start_pos = (self.phase[0] >> 16) as u32;
            self.phase[0] = self.phase[0].wrapping_add(clocks * self.freq[0]) & 0x3FFFFF;
            let end_pos = (self.phase[0] >> 16) as u32;
            let mut p = start_pos;
            while p < end_pos {
                let wv = self.wave[0][(p & 0x3F) as usize];
                if wv == 4 {
                    self.mod_pos = 0;
                } else {
                    const BIAS: [i32; 8] = [0, 1, 2, 4, 0, -4, -2, -1];
                    let bias_idx = (wv & 7) as usize;
                    self.mod_pos = (self.mod_pos as i32 + BIAS[bias_idx]) as u32 & 0x7F;
                }
                p += 1;
            }
        }

        if !self.wav_halt {
            let mut modulation = 0i32;
            if self.env_out[0] != 0 {
                let pos = if self.mod_pos < 64 {
                    self.mod_pos as i32
                } else {
                    (self.mod_pos as i32) - 128
                };
                let mut temp = pos * self.env_out[0] as i32;
                let rem = temp & 0x0F;
                temp >>= 4;
                if rem > 0 && (temp & 0x80) == 0 {
                    if pos < 0 {
                        temp -= 1;
                    } else {
                        temp += 2;
                    }
                }
                while temp >= 192 {
                    temp -= 256;
                }
                while temp < -64 {
                    temp += 256;
                }
                temp = (self.freq[1] as i32) * temp;
                let rem2 = temp & 0x3F;
                temp >>= 6;
                if rem2 >= 32 {
                    temp += 1;
                }
                modulation = temp;
            }
            let f = (self.freq[1] as i32) + modulation;
            self.phase[1] = self.phase[1].wrapping_add(clocks * f as u32) & 0x3FFFFF;
        }

        let vol_out = self.env_out[1].min(32);
        if !self.wav_write {
            let idx = ((self.phase[1] >> 16) & 0x3F) as usize;
            let sample = self.wave[1][idx];
            self.fout = sample * vol_out as i32 - vol_out as i32 * 31;
        }

        self.compute_audio_sample();
    }

    fn compute_audio_sample(&mut self) {
        const MASTER_SCALE: [f32; 4] = [1.0, 2.0 / 3.0, 0.5, 0.4];
        let scale = MASTER_SCALE[(self.master_vol & 3) as usize];
        self.current_audio_sample = (self.fout as f32) / (32.0 * 32.0) * scale;
    }

    fn read_reg(&mut self, adr: u16) -> u8 {
        match adr {
            0x4040..=0x407F => {
                if self.wav_write {
                    self.wave[1][(adr - 0x4040) as usize] as u8 & 0x3F
                } else {
                    self.wave[1][((self.phase[1] >> 16) & 0x3F) as usize] as u8 & 0x3F
                }
            }
            0x4090 => (self.env_out[1] as u8) | 0x40,
            0x4092 => (self.env_out[0] as u8) | 0x40,
            _ => 0,
        }
    }

    fn write_reg(&mut self, adr: u16, val: u8) {
        if adr == 0x4023 {
            self.master_io = (val & 2) != 0;
            return;
        }
        if !self.master_io {
            return;
        }
        if adr < 0x4040 || adr > 0x408A {
            return;
        }
        if adr < 0x4080 {
            if self.wav_write {
                let idx = (adr - 0x4040) as usize;
                self.wave[1][idx] = (val & 0x3F) as i32;
            }
            return;
        }
        match adr & 0xFF {
            0x80 => {
                self.env_disable[1] = (val & 0x80) != 0;
                self.env_mode[1] = (val & 0x40) != 0;
                self.env_timer[1] = 0;
                self.env_speed[1] = (val & 0x3F) as u32;
                if self.env_disable[1] {
                    self.env_out[1] = self.env_speed[1];
                }
            }
            0x82 => {
                self.freq[1] = (self.freq[1] & 0xF00) | val as u32;
            }
            0x83 => {
                self.freq[1] = (self.freq[1] & 0x0FF) | ((val as u32 & 0x0F) << 8);
                self.wav_halt = (val & 0x80) != 0;
                self.env_halt = (val & 0x40) != 0;
                if self.wav_halt {
                    self.phase[1] = 0;
                }
                if self.env_halt {
                    self.env_timer[0] = 0;
                    self.env_timer[1] = 0;
                }
            }
            0x84 => {
                self.env_disable[0] = (val & 0x80) != 0;
                self.env_mode[0] = (val & 0x40) != 0;
                self.env_timer[0] = 0;
                self.env_speed[0] = (val & 0x3F) as u32;
                if self.env_disable[0] {
                    self.env_out[0] = self.env_speed[0];
                }
            }
            0x85 => {
                self.mod_pos = (val & 0x7F) as u32;
                self.phase[0] = self.mod_write_pos << 16;
            }
            0x86 => {
                self.freq[0] = (self.freq[0] & 0xF00) | val as u32;
            }
            0x87 => {
                self.freq[0] = (self.freq[0] & 0x0FF) | ((val as u32 & 0x0F) << 8);
                self.mod_halt = (val & 0x80) != 0;
                if self.mod_halt {
                    self.phase[0] &= 0x3F0000;
                }
            }
            0x88 => {
                if self.mod_halt {
                    let idx = ((self.phase[0] >> 16) & 0x3F) as usize;
                    self.wave[0][idx] = (val & 0x07) as i32;
                    self.phase[0] = self.phase[0].wrapping_add(0x010000) & 0x3FFFFF;
                    let idx2 = ((self.phase[0] >> 16) & 0x3F) as usize;
                    self.wave[0][idx2] = (val & 0x07) as i32;
                    self.phase[0] = self.phase[0].wrapping_add(0x010000) & 0x3FFFFF;
                    self.mod_write_pos = self.phase[0] >> 16;
                    self.phase[1] = self.phase[1].wrapping_add(1) & 0x3FFFFF;
                }
            }
            0x89 => {
                self.wav_write = (val & 0x80) != 0;
                self.master_vol = val & 0x03;
            }
            0x8A => {
                self.master_env_speed = val;
                self.env_timer[0] = 0;
                self.env_timer[1] = 0;
            }
            _ => {}
        }
    }

}

pub struct Mapper522 {
    reg_index: u8,
    regs: [u8; 8],
    fds: FdsSound,
}

impl Mapper522 {
    pub fn new() -> Self {
        Self {
            reg_index: 0,
            regs: [0; 8],
            fds: FdsSound::new(),
        }
    }

    fn prg_read_bank(&self, bank: u8, address: u16, prg_rom: &[u8]) -> u8 {
        let offset = (bank as usize * 0x2000) + (address as usize & 0x1FFF);
        if prg_rom.is_empty() {
            return 0xFF;
        }
        prg_rom[offset % prg_rom.len()]
    }
}

impl Mapper for Mapper522 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        match address {
            0x4020..=0x409F => FetchResult {
                data: self.fds.read_reg(address),
                driven: true,
            },
            0x6000..=0x7FFF => FetchResult {
                data: self.prg_read_bank(0xFE, address, &cart.prg_rom),
                driven: true,
            },
            0x8000..=0x9FFF => FetchResult {
                data: self.prg_read_bank(self.regs[6], address, &cart.prg_rom),
                driven: true,
            },
            0xA000..=0xBFFF => FetchResult {
                data: self.prg_read_bank(self.regs[7], address, &cart.prg_rom),
                driven: true,
            },
            0xC000..=0xDFFF => {
                let idx = (address as usize & 0x1FFF) % cart.prg_ram.len().max(1);
                FetchResult {
                    data: cart.prg_ram.get(idx).copied().unwrap_or(0),
                    driven: true,
                }
            }
            0xE000..=0xFFFF => FetchResult {
                data: self.prg_read_bank(0xFF, address, &cart.prg_rom),
                driven: true,
            },
            _ => FetchResult {
                data: 0,
                driven: false,
            },
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        match address {
            0x4020..=0x409F => {
                self.fds.write_reg(address, data);
            }
            0x8000..=0x9FFF => {
                if address & 1 == 0 {
                    self.reg_index = data & 7;
                } else {
                    self.regs[self.reg_index as usize] = data;
                }
            }
            0xC000..=0xDFFF => {
                let idx = (address as usize & 0x1FFF) % cart.prg_ram.len().max(1);
                if idx < cart.prg_ram.len() {
                    cart.prg_ram[idx] = data;
                }
            }
            _ => {}
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
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let offset = (address as usize) & 0x1FFF;
            if using_chr_ram {
                new_addr_bus |= chr_ram.get(offset).copied().unwrap_or(0) as u16;
            } else {
                new_addr_bus |= chr_rom.get(offset % chr_rom.len().max(1)).copied().unwrap_or(0) as u16;
            }
        } else {
            let mirrored = if nametable_horizontal_mirroring {
                (address & 0x3FFF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let idx = (mirrored & 0x7FF) as usize;
            new_addr_bus |= vram.get(idx).copied().unwrap_or(0) as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if cycles > 0 {
            self.fds.run(cycles as u32);
        }
        false
    }

    fn audio_sample(&self) -> f32 {
        self.fds.current_audio_sample
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = vec![self.reg_index];
        state.extend_from_slice(&self.regs);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start < state.len() {
            self.reg_index = state[start] & 7;
            start += 1;
            let copy_len = (state.len() - start).min(8);
            self.regs[..copy_len].copy_from_slice(&state[start..start + copy_len]);
            start += copy_len;
        }
        start
    }

    fn reset(&mut self) {
        self.reg_index = 0;
        self.regs = [0; 8];
        self.fds.reset();
    }
}
