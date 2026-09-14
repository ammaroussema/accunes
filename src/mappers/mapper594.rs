use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, mmc3_chr_bank};

const DIFF_LOOKUP: [i16; 49 * 16] = [
    2, 6, 10, 14, 18, 22, 26, 30, -2, -6, -10, -14, -18, -22, -26, -30,
    2, 6, 10, 14, 19, 23, 27, 31, -2, -6, -10, -14, -19, -23, -27, -31,
    2, 6, 11, 15, 21, 25, 30, 34, -2, -6, -11, -15, -21, -25, -30, -34,
    2, 7, 12, 17, 23, 28, 33, 38, -2, -7, -12, -17, -23, -28, -33, -38,
    2, 7, 13, 18, 25, 30, 36, 41, -2, -7, -13, -18, -25, -30, -36, -41,
    3, 9, 15, 21, 28, 34, 40, 46, -3, -9, -15, -21, -28, -34, -40, -46,
    3, 10, 17, 24, 31, 38, 45, 52, -3, -10, -17, -24, -31, -38, -45, -52,
    3, 10, 18, 25, 34, 41, 49, 56, -3, -10, -18, -25, -34, -41, -49, -56,
    4, 12, 21, 29, 38, 46, 55, 63, -4, -12, -21, -29, -38, -46, -55, -63,
    4, 13, 22, 31, 41, 50, 59, 68, -4, -13, -22, -31, -41, -50, -59, -68,
    5, 15, 25, 35, 46, 56, 66, 76, -5, -15, -25, -35, -46, -56, -66, -76,
    5, 16, 27, 38, 50, 61, 72, 83, -5, -16, -27, -38, -50, -61, -72, -83,
    6, 18, 31, 43, 56, 68, 81, 93, -6, -18, -31, -43, -56, -68, -81, -93,
    6, 19, 33, 46, 61, 74, 88, 101, -6, -19, -33, -46, -61, -74, -88, -101,
    7, 22, 37, 52, 67, 82, 97, 112, -7, -22, -37, -52, -67, -82, -97, -112,
    8, 24, 41, 57, 74, 90, 107, 123, -8, -24, -41, -57, -74, -90, -107, -123,
    9, 27, 45, 63, 82, 100, 118, 136, -9, -27, -45, -63, -82, -100, -118, -136,
    10, 30, 50, 70, 90, 110, 130, 150, -10, -30, -50, -70, -90, -110, -130, -150,
    11, 33, 55, 77, 99, 121, 143, 165, -11, -33, -55, -77, -99, -121, -143, -165,
    12, 36, 60, 84, 109, 133, 157, 181, -12, -36, -60, -84, -109, -133, -157, -181,
    13, 39, 66, 92, 120, 146, 173, 199, -13, -39, -66, -92, -120, -146, -173, -199,
    14, 43, 73, 102, 132, 161, 191, 220, -14, -43, -73, -102, -132, -161, -191, -220,
    16, 48, 81, 113, 146, 178, 211, 243, -16, -48, -81, -113, -146, -178, -211, -243,
    17, 52, 88, 123, 160, 195, 231, 266, -17, -52, -88, -123, -160, -195, -231, -266,
    19, 58, 97, 136, 176, 215, 254, 293, -19, -58, -97, -136, -176, -215, -254, -293,
    21, 64, 107, 150, 194, 237, 280, 323, -21, -64, -107, -150, -194, -237, -280, -323,
    23, 70, 118, 165, 213, 260, 308, 355, -23, -70, -118, -165, -213, -260, -308, -355,
    26, 78, 130, 182, 235, 287, 339, 391, -26, -78, -130, -182, -235, -287, -339, -391,
    28, 85, 143, 200, 258, 315, 373, 430, -28, -85, -143, -200, -258, -315, -373, -430,
    31, 94, 157, 220, 284, 347, 410, 473, -31, -94, -157, -220, -284, -347, -410, -473,
    34, 103, 173, 242, 313, 382, 452, 521, -34, -103, -173, -242, -313, -382, -452, -521,
    38, 114, 191, 267, 345, 421, 498, 574, -38, -114, -191, -267, -345, -421, -498, -574,
    42, 126, 210, 294, 379, 463, 547, 631, -42, -126, -210, -294, -379, -463, -547, -631,
    46, 138, 231, 323, 417, 509, 602, 694, -46, -138, -231, -323, -417, -509, -602, -694,
    51, 153, 255, 357, 459, 561, 663, 765, -51, -153, -255, -357, -459, -561, -663, -765,
    56, 168, 280, 392, 505, 617, 729, 841, -56, -168, -280, -392, -505, -617, -729, -841,
    61, 184, 308, 431, 555, 678, 802, 925, -61, -184, -308, -431, -555, -678, -802, -925,
    68, 204, 340, 476, 612, 748, 884, 1020, -68, -204, -340, -476, -612, -748, -884, -1020,
    74, 223, 373, 522, 672, 821, 971, 1120, -74, -223, -373, -522, -672, -821, -971, -1120,
    82, 246, 411, 575, 740, 904, 1069, 1233, -82, -246, -411, -575, -740, -904, -1069, -1233,
    90, 271, 452, 633, 814, 995, 1176, 1357, -90, -271, -452, -633, -814, -995, -1176, -1357,
    99, 298, 497, 696, 895, 1094, 1293, 1492, -99, -298, -497, -696, -895, -1094, -1293, -1492,
    109, 328, 547, 766, 985, 1204, 1423, 1642, -109, -328, -547, -766, -985, -1204, -1423, -1642,
    120, 360, 601, 841, 1083, 1323, 1564, 1804, -120, -360, -601, -841, -1083, -1323, -1564, -1804,
    132, 397, 662, 927, 1192, 1457, 1722, 1987, -132, -397, -662, -927, -1192, -1457, -1722, -1987,
    145, 436, 728, 1019, 1311, 1602, 1894, 2185, -145, -436, -728, -1019, -1311, -1602, -1894, -2185,
    160, 480, 801, 1121, 1442, 1762, 2083, 2403, -160, -480, -801, -1121, -1442, -1762, -2083, -2403,
    176, 528, 881, 1233, 1587, 1939, 2292, 2644, -176, -528, -881, -1233, -1587, -1939, -2292, -2644,
    194, 582, 970, 1358, 1746, 2134, 2522, 2910, -194, -582, -970, -1358, -1746, -2134, -2522, -2910,
];
const INDEX_SHIFT: [i8; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];

const HOST_CLOCK: i32 = 1_789_773;

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

struct MSM6585 {
    which_nibble: bool,
    input: u8,
    signal: i16,
    count: i32,
    rate: i32,
    step: i16,
    low_pass: LpfRc,
}

impl MSM6585 {
    fn new() -> Self {
        let mut m = MSM6585 {
            which_nibble: false,
            input: 0,
            signal: -2,
            count: 0,
            rate: 4000,
            step: 0,
            low_pass: LpfRc::new(),
        };
        m.low_pass.set_fc(4000.0 * 0.4 / HOST_CLOCK as f64);
        m
    }

    fn reset(&mut self) {
        self.count = 0;
        self.step = 0;
        self.signal = -2;
    }

    fn set_rate(&mut self, rate_byte: u8) {
        self.rate = 4000 << (rate_byte & 3);
        self.low_pass.set_fc(self.rate as f64 * 0.4 / HOST_CLOCK as f64);
        self.count = 0;
    }

    fn run(&mut self, fifo: &mut Fifo) {
        self.count += self.rate;
        while self.count >= HOST_CLOCK {
            self.count -= HOST_CLOCK;
            let nibble;
            if self.which_nibble {
                nibble = self.input & 0x0F;
            } else {
                self.input = fifo.retrieve();
                nibble = self.input >> 4;
            }
            self.which_nibble = !self.which_nibble;
            self.signal = self.signal
                .saturating_add(DIFF_LOOKUP[(self.step as usize) * 16 + nibble as usize]);
            if self.signal > 2047 {
                self.signal = 2047;
            }
            if self.signal < -2048 {
                self.signal = -2048;
            }
            self.step += INDEX_SHIFT[(nibble & 7) as usize] as i16;
            if self.step > 48 {
                self.step = 48;
            }
            if self.step < 0 {
                self.step = 0;
            }
        }
    }

    fn get_output(&mut self) -> f32 {
        self.low_pass.process(self.signal as f64 * 16.0) as f32
    }
}

struct Fifo {
    data: [u8; 1024],
    front: u16,
    back: u16,
}

impl Fifo {
    fn new() -> Self {
        Fifo {
            data: [0; 1024],
            front: 0,
            back: 0,
        }
    }

    fn size(&self) -> u16 {
        (self.back.wrapping_sub(self.front) + 1024) % 1024
    }

    fn half_full(&self) -> bool {
        self.size() >= 512
    }

    fn retrieve(&mut self) -> u8 {
        let f = self.front as usize;
        self.front = (self.front + 1) % 1024;
        self.data[f]
    }

    fn add(&mut self, value: u8) {
        if (self.size() as usize) < 1024 {
            let b = self.back as usize;
            self.data[b] = value;
            self.back = (self.back + 1) % 1024;
        }
    }

    fn reset(&mut self) {
        self.front = 0;
        self.back = 0;
    }
}

pub struct Mapper594 {
    mmc3: MapperMMC3,
    reg: [u8; 4],
    fifo: Fifo,
    adpcm: MSM6585,
    audio_out: f32,
}

impl Mapper594 {
    pub fn new() -> Self {
        let mut config = Mmc3Config::embedded();
        config.ax5202p = true;
        Self {
            mmc3: MapperMMC3::new(config),
            reg: [0; 4],
            fifo: Fifo::new(),
            adpcm: MSM6585::new(),
            audio_out: 0.0,
        }
    }

    fn second_last_prg_bank(&self, cart: &Cartridge) -> usize {
        let num = cart.prg_rom.len() / 0x2000;
        if num < 2 { 0 } else { num - 2 }
    }

    fn last_prg_bank(&self, cart: &Cartridge) -> usize {
        let num = cart.prg_rom.len() / 0x2000;
        if num == 0 { 0 } else { num - 1 }
    }

    fn get_prg_bank(&self, cart: &Cartridge, slot: usize) -> usize {
        let swap = (self.mmc3.r8000 & 0x40) != 0;
        match slot {
            0 => {
                if swap {
                    self.second_last_prg_bank(cart)
                } else {
                    self.mmc3.bank_8c as usize
                }
            }
            1 => self.mmc3.bank_a as usize,
            2 => {
                if swap {
                    self.mmc3.bank_8c as usize
                } else {
                    self.second_last_prg_bank(cart)
                }
            }
            3 => self.last_prg_bank(cart),
            _ => 0,
        }
    }

    fn get_chr_bank(&self, bank: usize) -> usize {
        let base = mmc3_chr_bank(
            self.mmc3.r8000,
            self.mmc3.chr_2k0,
            self.mmc3.chr_2k8,
            self.mmc3.chr_1k0,
            self.mmc3.chr_1k4,
            self.mmc3.chr_1k8,
            self.mmc3.chr_1kc,
            (bank * 0x400) as u16,
        ) as usize;
        base | ((bank << 6) & 0x100)
    }

    fn chr_and(&self) -> usize {
        if (self.reg[2] & 0xC0) != 0 { 0xFF } else { 0x1FF }
    }

    fn chr_or(&self) -> usize {
        let mut v = 0usize;
        if (self.reg[2] & 0x40) != 0 { v |= 0x200; }
        if (self.reg[2] & 0x80) != 0 { v |= 0x300; }
        v
    }

    fn prg_or(&self) -> usize {
        let mut v = 0usize;
        if (self.reg[2] & 0x40) != 0 { v |= 0x0C0; }
        if (self.reg[2] & 0x80) != 0 { v |= 0x100; }
        v
    }

    fn final_chr_bank(&self, bank: usize) -> usize {
        (self.get_chr_bank(bank) & self.chr_and()) | self.chr_or()
    }
}

impl Mapper for Mapper594 {
    fn reset(&mut self) {
        self.reg = [0; 4];
        self.fifo.reset();
        self.adpcm.reset();
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let slot = ((address as usize >> 13) & 3) as usize;
            let bank = (self.get_prg_bank(cart, slot) & 0x3F) | self.prg_or();
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            if cart.prg_rom.is_empty() {
                return FetchResult { data: 0, driven: true };
            }
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if address >= 0x6000 {
            let bank = (self.reg[0] as usize) | self.prg_or();
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            if cart.prg_rom.is_empty() {
                return FetchResult { data: 0, driven: true };
            }
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if address >= 0x5000 {
            let data = if self.fifo.half_full() { 0x00 } else { 0x40 };
            FetchResult { data, driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x9000 && address <= 0x9FFF {
            self.reg[(address & 1) as usize] = data;
        } else if address >= 0xB000 && address <= 0xBFFF {
            self.reg[2 + (address & 1) as usize] = data;
        } else if address >= 0x5000 && address <= 0x5FFF {
            if (address & 1) != 0 {
                self.adpcm.set_rate(data >> 6);
                self.fifo.reset();
            } else {
                self.fifo.add(data);
            }
        } else if address >= 0x6000 {
            self.mmc3.store_prg(cart, address, data);
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mmc3.mirror_nametable(cart, address)
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
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = self.final_chr_bank((address as usize) >> 10);
            let offset = bank * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if self.mmc3.nametable_mirroring() {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
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
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let bank = self.final_chr_bank((address as usize) >> 10);
                let offset = bank * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
            }
        }
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        ppu_sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        self.mmc3.ppu_clock(
            ppu_address_bus,
            ppu_a12_prev,
            scanline,
            dot,
            ppu_sprite_x16,
            rendering_on,
        )
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.adpcm.run(&mut self.fifo);
        self.audio_out = self.adpcm.get_output() / 32768.0;
        false
    }

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
    }

    fn audio_sample(&self) -> f32 {
        self.audio_out
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.extend_from_slice(&self.reg);
        state.push(if self.adpcm.which_nibble { 1 } else { 0 });
        state.push(self.adpcm.input);
        state.extend_from_slice(&self.adpcm.signal.to_le_bytes());
        state.extend_from_slice(&self.adpcm.count.to_le_bytes());
        state.extend_from_slice(&self.adpcm.rate.to_le_bytes());
        state.extend_from_slice(&self.adpcm.step.to_le_bytes());
        for &b in &self.fifo.data {
            state.push(b);
        }
        state.extend_from_slice(&self.fifo.front.to_le_bytes());
        state.extend_from_slice(&self.fifo.back.to_le_bytes());
        state
    }

    fn load_mapper_registers(
        &mut self,
        cart: &mut Cartridge,
        state: &[u8],
        start: usize,
    ) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        for r in &mut self.reg {
            if p < state.len() {
                *r = state[p];
                p += 1;
            }
        }
        if p < state.len() {
            self.adpcm.which_nibble = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.adpcm.input = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.adpcm.signal = i16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p + 4 <= state.len() {
            self.adpcm.count =
                i32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
            p += 4;
        }
        if p + 4 <= state.len() {
            self.adpcm.rate =
                i32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
            p += 4;
        }
        if p + 2 <= state.len() {
            self.adpcm.step = i16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p + 1024 <= state.len() {
            self.fifo.data.copy_from_slice(&state[p..p + 1024]);
            p += 1024;
        }
        if p + 2 <= state.len() {
            self.fifo.front = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p + 2 <= state.len() {
            self.fifo.back = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        p
    }
}
