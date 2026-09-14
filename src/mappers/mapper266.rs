use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

fn butterworth_coeffs(n: usize, sample_rate: f64, cutoff: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let half = n / 2;
    let mut a = vec![0.0; half];
    let mut d1 = vec![0.0; half];
    let mut d2 = vec![0.0; half];
    let a_val = (std::f64::consts::PI * cutoff / sample_rate).tan();
    let a2 = a_val * a_val;
    for i in 0..half {
        let r = (std::f64::consts::PI * (2.0 * i as f64 + 1.0) / (4.0 * half as f64)).sin();
        let s = a2 + 2.0 * a_val * r + 1.0;
        a[i] = a2 / s;
        d1[i] = 2.0 * (1.0 - a2) / s;
        d2[i] = -(a2 - 2.0 * a_val * r + 1.0) / s;
    }
    (a, d1, d2)
}

pub struct Mapper266 {
    prg: u8,
    pcm: u8,
    chr_reg: [u8; 8],
    chr_hi: [u16; 8],
    prg_flip: u8,
    mirr: u8,
    wram: [u8; 0x2000],
    cpu_clock_hz: f64,
    prev_pcm: u8,
    accumulated_cycles: u64,
    filter_a: Vec<f64>,
    filter_d1: Vec<f64>,
    filter_d2: Vec<f64>,
    filter_w0: Vec<f64>,
    filter_w1: Vec<f64>,
    filter_w2: Vec<f64>,
}

impl Mapper266 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        let half = 10;
        let (a, d1, d2) = butterworth_coeffs(20, 1_789_772.727_272, 2000.0);
        Self {
            prg: 0,
            pcm: 7,
            chr_reg: [0, 1, 2, 3, 4, 5, 6, 7],
            chr_hi: [0; 8],
            prg_flip: 0,
            mirr: 0,
            wram: [0; 0x2000],
            cpu_clock_hz: 1_789_772.727_272,
            prev_pcm: 7,
            accumulated_cycles: 0,
            filter_a: a,
            filter_d1: d1,
            filter_d2: d2,
            filter_w0: vec![0.0; half],
            filter_w1: vec![0.0; half],
            filter_w2: vec![0.0; half],
        }
    }

    fn run_filter(&mut self, input: f64) {
        let mut x = input;
        for i in 0..10 {
            self.filter_w0[i] = self.filter_d1[i] * self.filter_w1[i]
                + self.filter_d2[i] * self.filter_w2[i]
                + x;
            x = self.filter_a[i] * (self.filter_w0[i] + 2.0 * self.filter_w1[i] + self.filter_w2[i]);
            self.filter_w2[i] = self.filter_w1[i];
            self.filter_w1[i] = self.filter_w0[i];
        }
    }
}

impl Mapper for Mapper266 {
    fn reset(&mut self) {
        self.prg = 0;
        self.pcm = 7;
        self.chr_reg = [0, 1, 2, 3, 4, 5, 6, 7];
        self.chr_hi = [0; 8];
        self.prg_flip = 0;
        self.mirr = 0;
        self.prev_pcm = 7;
        self.accumulated_cycles = 0;
        for w in &mut self.filter_w0 { *w = 0.0; }
        for w in &mut self.filter_w1 { *w = 0.0; }
        for w in &mut self.filter_w2 { *w = 0.0; }
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if cart.prg_rom.is_empty() {
                return FetchResult { data: 0, driven: false };
            }
            let offset = ((self.prg as usize) * 0x8000) + (address as usize & 0x7FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if address >= 0x6000 {
            let idx = (address as usize - 0x6000) & 0x1FFF;
            FetchResult { data: self.wram[idx], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            let idx = (address as usize - 0x6000) & 0x1FFF;
            self.wram[idx] = data;
            return;
        }
        if address < 0x8000 {
            return;
        }
        let bank = (address >> 12) as u8;
        let addr = address & 0xFFF;
        let sw_bank = (bank & 0x9) | ((bank & 0x2) << 1) | ((bank & 0x4) >> 1);
        match sw_bank {
            0x9 => {
                let reg = (((addr & 0x08) != 0) as u8) << 1 | ((addr & 0x04) != 0) as u8;
                match reg & 3 {
                    0 | 1 => {
                        if data != 0xFF {
                            self.mirr = data & 3;
                        }
                    }
                    3 => {
                        if addr & 0x800 != 0 {
                            self.pcm = data & 0xF;
                        } else {
                            self.prg = data >> 2;
                        }
                    }
                    _ => {}
                }
            }
            0xB..=0xE => {
                let vrc4_reg = (((sw_bank - 0xB) as usize) << 1) | if addr & 0x08 != 0 { 1 } else { 0 };
                if vrc4_reg < 8 {
                    if addr & 0x04 != 0 {
                        self.chr_reg[vrc4_reg] = (self.chr_reg[vrc4_reg] & 0x0F) | (data << 4);
                        self.chr_hi[vrc4_reg] = ((data & 0x10) << 4) as u16;
                    } else {
                        self.chr_reg[vrc4_reg] = (self.chr_reg[vrc4_reg] & 0xF0) | (data & 0x0F);
                    }
                }
            }
            _ => {}
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        match self.mirr & 0x3 {
            0 => address & 0x37FF,
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            2 => address & 0x3FFF,
            3 => (address & 0x3FFF) | 0x0400,
            _ => address,
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
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let bank = (address >> 10) as usize & 0x07;
            let chr_bank = (self.chr_hi[bank] | self.chr_reg[bank] as u16) as usize;
            let offset = chr_bank * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = match self.mirr & 0x3 {
                0 => address & 0x37FF,
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                2 => address & 0x3FFF,
                3 => (address & 0x3FFF) | 0x0400,
                _ => address,
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() { prg_vram[idx] } else { 0 }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram && !cart.chr_ram.is_empty() {
            let bank = (address >> 10) as usize & 0x07;
            let chr_bank = (self.chr_hi[bank] | self.chr_reg[bank] as u16) as usize;
            let offset = chr_bank * 0x0400 + (address as usize & 0x03FF);
            let len = cart.chr_ram.len();
            cart.chr_ram[offset % len] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.pcm != self.prev_pcm {
            let diff = (self.pcm as f64 - 7.0) * 4096.0 + 1e-15;
            self.run_filter(diff);
            self.prev_pcm = self.pcm;
        } else {
            let diff = (self.pcm as f64 - 7.0) * 4096.0 + 1e-15;
            self.accumulated_cycles += cycles as u64;
            let cycles_per_sample = (self.cpu_clock_hz / 2000.0) as u64;
            while self.accumulated_cycles >= cycles_per_sample {
                self.accumulated_cycles -= cycles_per_sample;
                self.run_filter(diff);
            }
        }
        false
    }

    fn audio_sample(&self) -> f32 {
        let raw = if !self.filter_w1.is_empty() {
            self.filter_w1[0]
        } else {
            0.0
        };
        (raw / 32768.0) as f32
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.prg);
        state.push(self.pcm);
        state.push(self.prg_flip);
        state.extend_from_slice(&self.chr_reg);
        for &hi in &self.chr_hi {
            state.extend_from_slice(&hi.to_le_bytes());
        }
        state.push(self.mirr);
        state.extend_from_slice(&self.wram);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.prg = state[p]; p += 1; }
        if p < state.len() { self.pcm = state[p]; p += 1; }
        if p < state.len() { self.prg_flip = state[p]; p += 1; }
        if p + 8 <= state.len() { self.chr_reg.copy_from_slice(&state[p..p + 8]); p += 8; }
        for i in 0..8 {
            if p + 2 <= state.len() {
                self.chr_hi[i] = u16::from_le_bytes([state[p], state[p + 1]]);
                p += 2;
            }
        }
        if p < state.len() { self.mirr = state[p]; p += 1; }
        let wram_len = (state.len() - p).min(0x2000);
        if wram_len > 0 {
            self.wram[..wram_len].copy_from_slice(&state[p..p + wram_len]);
            p += wram_len;
        }
        p
    }
}
