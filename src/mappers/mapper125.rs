use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const RC_BITS: i32 = 12;

pub struct Mapper125 {
    prg_bank: u8,
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
    master_env_speed: u32,
    rc_accum: i32,
    rc_k: i32,
    rc_l: i32,
    fout: i32,
}

impl Mapper125 {
    pub fn new() -> Self {
        let rc_bits_f = 1 << RC_BITS;
        let cutoff = 2000.0;
        let rate = 19687500.0 / 11.0;
        let leak = if cutoff > 0.0 {
            (-2.0 * std::f64::consts::PI * cutoff / rate).exp()
        } else {
            0.0
        };
        let rc_k = (leak * rc_bits_f as f64) as i32;
        let rc_l = rc_bits_f - rc_k;
        Self {
            prg_bank: 0,
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
            rc_accum: 0,
            rc_k,
            rc_l,
            fout: 0,
        }
    }

    fn fds_write(&mut self, adr: u16, val: u8) {
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
                self.wave[1][(adr - 0x4040) as usize] = (val & 0x3F) as i32;
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
                self.freq[1] = (self.freq[1] & 0x0FF) | (((val & 0x0F) as u32) << 8);
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
                self.freq[0] = (self.freq[0] & 0x0FF) | (((val & 0x0F) as u32) << 8);
                self.mod_halt = (val & 0x80) != 0;
                if self.mod_halt {
                    self.phase[0] = self.phase[0] & 0x3F0000;
                }
            }
            0x88 => {
                if self.mod_halt {
                    let idx = ((self.phase[0] >> 16) & 0x3F) as usize;
                    self.wave[0][idx] = (val & 0x07) as i32;
                    self.phase[0] = (self.phase[0] + 0x010000) & 0x3FFFFF;
                    let idx2 = ((self.phase[0] >> 16) & 0x3F) as usize;
                    self.wave[0][idx2] = (val & 0x07) as i32;
                    self.phase[0] = (self.phase[0] + 0x010000) & 0x3FFFFF;
                    self.mod_write_pos = self.phase[0] >> 16;
                    self.phase[1] = (self.phase[1] + 1) & 0x3FFFFF;
                }
            }
            0x89 => {
                self.wav_write = (val & 0x80) != 0;
                self.master_vol = val & 0x03;
            }
            0x8A => {
                self.master_env_speed = val as u32;
                self.env_timer[0] = 0;
                self.env_timer[1] = 0;
            }
            _ => {}
        }
    }

    fn fds_read(&self, adr: u16) -> u8 {
        if adr >= 0x4040 && adr <= 0x407F {
            let idx = (adr - 0x4040) as usize;
            if self.wav_write {
                return self.wave[1][idx] as u8;
            } else {
                return self.wave[1][((self.phase[1] >> 16) & 0x3F) as usize] as u8;
            }
        }
        if adr == 0x4090 {
            return (self.env_out[1] | 0x40) as u8;
        }
        if adr == 0x4092 {
            return (self.env_out[0] | 0x40) as u8;
        }
        0
    }

    fn fds_run(&mut self, clocks: u32) {
        if !self.env_halt && !self.wav_halt && self.master_env_speed != 0 {
            for i in 0..2 {
                if !self.env_disable[i] {
                    self.env_timer[i] = self.env_timer[i].wrapping_add(clocks);
                    let period = ((self.env_speed[i] + 1) * (self.master_env_speed + 1)) << 3;
                    while self.env_timer[i] >= period {
                        if self.env_mode[i] {
                            if self.env_out[i] < 32 {
                                self.env_out[i] += 1;
                            }
                        } else {
                            if self.env_out[i] > 0 {
                                self.env_out[i] -= 1;
                            }
                        }
                        self.env_timer[i] -= period;
                    }
                }
            }
        }
        if !self.mod_halt {
            let start_pos = self.phase[0] >> 16;
            self.phase[0] = self.phase[0].wrapping_add(clocks * self.freq[0]);
            let end_pos = self.phase[0] >> 16;
            self.phase[0] &= 0x3FFFFF;
            let bias: [i32; 8] = [0, 1, 2, 4, 0, -4, -2, -1];
            for p in start_pos..end_pos {
                let wv = self.wave[0][(p & 0x3F) as usize];
                if wv == 4 {
                    self.mod_pos = 0;
                } else {
                    let new_pos = self.mod_pos as i32 + bias[wv as usize];
                    self.mod_pos = (new_pos as u32) & 0x7F;
                }
            }
        }
        if !self.wav_halt {
            let mut mod_val = 0i32;
            if self.env_out[0] != 0 {
                let pos = if self.mod_pos < 64 {
                    self.mod_pos as i32
                } else {
                    self.mod_pos as i32 - 128
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
                mod_val = temp;
            }
            let f = self.freq[1] as i32 + mod_val;
            self.phase[1] = self.phase[1].wrapping_add((clocks as i32 * f) as u32);
            self.phase[1] &= 0x3FFFFF;
        }
        let vol_out = if self.env_out[1] > 32 { 32 } else { self.env_out[1] };
        if !self.wav_write {
            self.fout = self.wave[1][((self.phase[1] >> 16) & 0x3F) as usize] * vol_out as i32 - vol_out as i32 * 0x3F / 2;
        }
    }

    fn fds_get(&mut self, cycles: u32) -> i32 {
        self.fds_run(cycles);
        let master_vol_scale: [i32; 4] = [
            (2.4 * 1223.0 * 256.0 * 2.0 / 2.0 / (32.0 * 63.0)) as i32,
            (2.4 * 1223.0 * 256.0 * 2.0 / 3.0 / (32.0 * 63.0)) as i32,
            (2.4 * 1223.0 * 256.0 * 2.0 / 4.0 / (32.0 * 63.0)) as i32,
            (2.4 * 1223.0 * 256.0 * 2.0 / 5.0 / (32.0 * 63.0)) as i32,
        ];
        let v = self.fout * master_vol_scale[self.master_vol as usize];
        let rc_out = ((self.rc_accum * self.rc_k) + (v * self.rc_l)) >> RC_BITS;
        self.rc_accum = rc_out;
        rc_out
    }

    fn prg_addr(&self, cart: &Cartridge, bank: u8, address: u16) -> usize {
        let num_8k = cart.prg_rom.len() / 0x2000;
        if num_8k == 0 {
            return 0;
        }
        let bank_idx = (bank as usize) % num_8k;
        bank_idx * 0x2000 + (address as usize & 0x1FFF)
    }
}

impl Mapper for Mapper125 {
    fn reset(&mut self) {
        self.prg_bank = 0;
        self.fds_write(0x4023, 0x00);
        self.fds_write(0x4023, 0x83);
        self.fds_write(0x4080, 0x80);
        self.fds_write(0x408A, 0xE8);
        self.fds_write(0x4082, 0x00);
        self.fds_write(0x4083, 0x80);
        self.fds_write(0x4084, 0x80);
        self.fds_write(0x4085, 0x00);
        self.fds_write(0x4086, 0x00);
        self.fds_write(0x4087, 0x80);
        self.fds_write(0x4089, 0x00);
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x4020 && address < 0x4040 {
            return FetchResult {
                data: self.fds_read(address),
                driven: true,
            };
        }
        if address >= 0x4040 && address <= 0x4092 {
            return FetchResult {
                data: self.fds_read(address),
                driven: true,
            };
        }
        if address < 0x6000 {
            return FetchResult { data: 0, driven: false };
        }
        let offset = if address >= 0xE000 {
            self.prg_addr(cart, 0xFF, address)
        } else if address >= 0xC000 {
            let ram_offset = (address - 0xC000) as usize;
            if ram_offset < cart.prg_ram.len() {
                return FetchResult {
                    data: cart.prg_ram[ram_offset],
                    driven: true,
                };
            }
            return FetchResult { data: 0, driven: false };
        } else if address >= 0xA000 {
            self.prg_addr(cart, 0xFD, address)
        } else if address >= 0x8000 {
            self.prg_addr(cart, 0xFC, address)
        } else {
            self.prg_addr(cart, self.prg_bank, address)
        };
        let data = if offset < cart.prg_rom.len() {
            cart.prg_rom[offset]
        } else {
            0
        };
        FetchResult { data, driven: true }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            self.prg_bank = data;
            return;
        }
        if address >= 0x4040 && address <= 0x408A {
            self.fds_write(address, data);
            return;
        }
        if address >= 0x4020 && address <= 0x403F {
            self.fds_write(address, data);
            return;
        }
        if address >= 0xC000 && address < 0xE000 {
            let ram_offset = (address - 0xC000) as usize;
            if ram_offset < cart.prg_ram.len() {
                cart.prg_ram[ram_offset] = data;
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.nametable_horizontal_mirroring {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
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
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let offset = address as usize & 0x1FFF;
            let byte = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[offset % chr_ram.len()] }
            } else if chr_rom.is_empty() {
                0
            } else {
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if nametable_horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[address as usize % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.fds_get(cycles as u32);
        false
    }

    fn audio_sample(&self) -> f32 {
        let v = self.rc_accum as f32 / (1 << RC_BITS) as f32;
        v / 32768.0
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.prg_bank);
        state.push(if self.master_io { 1 } else { 0 });
        state.push(self.master_vol);
        for t in 0..2 {
            for w in &self.wave[t] {
                state.extend_from_slice(&w.to_le_bytes());
            }
        }
        for f in &self.freq {
            state.extend_from_slice(&f.to_le_bytes());
        }
        for p in &self.phase {
            state.extend_from_slice(&p.to_le_bytes());
        }
        state.push(if self.wav_write { 1 } else { 0 });
        state.push(if self.wav_halt { 1 } else { 0 });
        state.push(if self.env_halt { 1 } else { 0 });
        state.push(if self.mod_halt { 1 } else { 0 });
        state.extend_from_slice(&self.mod_pos.to_le_bytes());
        state.extend_from_slice(&self.mod_write_pos.to_le_bytes());
        for e in &self.env_mode {
            state.push(if *e { 1 } else { 0 });
        }
        for e in &self.env_disable {
            state.push(if *e { 1 } else { 0 });
        }
        for e in &self.env_timer {
            state.extend_from_slice(&e.to_le_bytes());
        }
        for e in &self.env_speed {
            state.extend_from_slice(&e.to_le_bytes());
        }
        for e in &self.env_out {
            state.extend_from_slice(&e.to_le_bytes());
        }
        state.extend_from_slice(&self.master_env_speed.to_le_bytes());
        state.extend_from_slice(&self.rc_accum.to_le_bytes());
        state.extend_from_slice(&self.fout.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p >= state.len() {
            return start;
        }
        self.prg_bank = state[p];
        p += 1;
        if p >= state.len() {
            return start;
        }
        self.master_io = state[p] != 0;
        p += 1;
        if p >= state.len() {
            return start;
        }
        self.master_vol = state[p];
        p += 1;
        for t in 0..2 {
            for w in &mut self.wave[t] {
                if p + 4 <= state.len() {
                    *w = i32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
                    p += 4;
                } else {
                    return start;
                }
            }
        }
        for f in &mut self.freq {
            if p + 4 <= state.len() {
                *f = u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
                p += 4;
            } else {
                return start;
            }
        }
        for ph in &mut self.phase {
            if p + 4 <= state.len() {
                *ph = u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
                p += 4;
            } else {
                return start;
            }
        }
        if p >= state.len() {
            return start;
        }
        self.wav_write = state[p] != 0;
        p += 1;
        if p >= state.len() {
            return start;
        }
        self.wav_halt = state[p] != 0;
        p += 1;
        if p >= state.len() {
            return start;
        }
        self.env_halt = state[p] != 0;
        p += 1;
        if p >= state.len() {
            return start;
        }
        self.mod_halt = state[p] != 0;
        p += 1;
        if p + 4 <= state.len() {
            self.mod_pos = u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
            p += 4;
        } else {
            return start;
        }
        if p + 4 <= state.len() {
            self.mod_write_pos = u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
            p += 4;
        } else {
            return start;
        }
        for e in &mut self.env_mode {
            if p >= state.len() {
                return start;
            }
            *e = state[p] != 0;
            p += 1;
        }
        for e in &mut self.env_disable {
            if p >= state.len() {
                return start;
            }
            *e = state[p] != 0;
            p += 1;
        }
        for et in &mut self.env_timer {
            if p + 4 <= state.len() {
                *et = u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
                p += 4;
            } else {
                return start;
            }
        }
        for es in &mut self.env_speed {
            if p + 4 <= state.len() {
                *es = u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
                p += 4;
            } else {
                return start;
            }
        }
        for eo in &mut self.env_out {
            if p + 4 <= state.len() {
                *eo = u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
                p += 4;
            } else {
                return start;
            }
        }
        if p + 4 <= state.len() {
            self.master_env_speed = u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
            p += 4;
        } else {
            return start;
        }
        if p + 4 <= state.len() {
            self.rc_accum = i32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
            p += 4;
        } else {
            return start;
        }
        if p + 4 <= state.len() {
            self.fout = i32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
            p += 4;
        }
        p
    }
}
