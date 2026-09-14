use crate::cartridge::Cartridge;
use crate::mapper::{ExpansionAudioType, FetchResult, Mapper};

const SUBMAPPER_VOLUME: [u8; 16] = [
    34, 34, 0, 34, 59, 68, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34,
];

pub struct Mapper19 {
    prg: [u8; 4],
    chr: [u8; 12],
    wram_protect: u8,
    counter: i16,
    index: u8,
    iram: [u8; 128],
    sound_enabled: bool,
    volume: u8,
    #[allow(dead_code)]
    submapper_id: u8,

    irq_pending: bool,
    irq_ack: bool,

    channel_output: [i16; 8],
    update_counter: u8,
    current_channel: usize,
    current_audio_sample: f32,
}

impl Mapper19 {
    pub fn new() -> Self {
        Self::with_submapper(0)
    }

    pub fn with_submapper(submapper_id: u8) -> Self {
        let volume = SUBMAPPER_VOLUME[(submapper_id & 0x0F) as usize];
        let mut m = Self {
            prg: [0xFC, 0xFD, 0xFE, 0xFF],
            chr: [0, 1, 2, 3, 4, 5, 6, 7, 0xE0, 0xE1, 0xE0, 0xE1],
            wram_protect: 0xFF,
            counter: 0,
            index: 0,
            iram: [0; 128],
            sound_enabled: true,
            volume,
            submapper_id,
            irq_pending: false,
            irq_ack: false,
            channel_output: [0; 8],
            update_counter: 0,
            current_channel: 7,
            current_audio_sample: 0.0,
        };
        m.reset_internal();
        m
    }

    fn reset_internal(&mut self) {
        for i in 0..4 {
            self.prg[i] = (i as u8) | 0xFC;
        }
        for i in 0..4 {
            self.chr[i] = i as u8;
            self.chr[i + 4] = (i as u8) | 0x04;
            self.chr[i + 8] = (i as u8 & 0x01) | 0xE0;
        }
        self.counter = 0;
        self.wram_protect = 0xFF;
        self.sound_enabled = true;
        self.irq_pending = false;
        self.irq_ack = false;
        self.current_audio_sample = 0.0;
    }

    fn get_freq(&self, ch: usize) -> u32 {
        let base = 0x40 + ch * 8;
        ((self.iram[base + 4] as u32 & 0x03) << 16)
            | ((self.iram[base + 2] as u32) << 8)
            | (self.iram[base] as u32)
    }

    fn get_phase(&self, ch: usize) -> u32 {
        let base = 0x40 + ch * 8;
        ((self.iram[base + 5] as u32) << 16)
            | ((self.iram[base + 3] as u32) << 8)
            | (self.iram[base + 1] as u32)
    }

    fn set_phase(&mut self, ch: usize, phase: u32) {
        let base = 0x40 + ch * 8;
        self.iram[base + 5] = (phase >> 16) as u8;
        self.iram[base + 3] = (phase >> 8) as u8;
        self.iram[base + 1] = phase as u8;
    }

    fn clock_audio_channel(&mut self, ch: usize) {
        let base = 0x40 + ch * 8;
        let mut phase = self.get_phase(ch);
        let freq = self.get_freq(ch);
        let length = 256usize.saturating_sub((self.iram[base + 4] & 0xFC) as usize);
        let offset = self.iram[base + 6];
        let volume = self.iram[base + 7] & 0x0F;

        if length > 0 {
            phase = (phase.wrapping_add(freq)) % ((length as u32) << 16);
        }

        let sample_pos = (((phase >> 16) as u8).wrapping_add(offset)) as usize;
        let sample_byte = self.iram[(sample_pos / 2) & 0x7F];
        let sample = if (sample_pos & 1) != 0 {
            sample_byte >> 4
        } else {
            sample_byte & 0x0F
        };

        self.channel_output[ch] = (sample as i16 - 8) * (volume as i16);
        self.set_phase(ch, phase);

        let num_channels = ((self.iram[0x7F] >> 4) & 0x07) as usize;
        let min_ch = 7usize.saturating_sub(num_channels);
        let mut sum = 0i32;
        for i in min_ch..=7 {
            sum += self.channel_output[i] as i32;
        }
        let raw = (sum / (num_channels as i32 + 1)) as f32;
        self.current_audio_sample = raw * (self.volume as f32 / 40.0);
    }

    fn clock_sound(&mut self, cycles: u8) {
        if !self.sound_enabled || self.volume == 0 {
            self.current_audio_sample = 0.0;
            return;
        }
        for _ in 0..cycles {
            self.update_counter += 1;
            if self.update_counter >= 15 {
                self.update_counter = 0;
                self.clock_audio_channel(self.current_channel);
                let num_channels = ((self.iram[0x7F] >> 4) & 0x07) as usize;
                let min_ch = 7usize.saturating_sub(num_channels);
                if self.current_channel <= min_ch {
                    self.current_channel = 7;
                } else {
                    self.current_channel -= 1;
                }
            }
        }
    }
}

impl Default for Mapper19 {
    fn default() -> Self {
        Self::new()
    }
}

impl Mapper for Mapper19 {
    fn reset(&mut self) {
        self.reset_internal();
    }

    fn reset_power_cycle(&mut self) {
        self.reset_internal();
        self.iram = [0; 128];
        self.index = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x4800..=0x4FFF).contains(&address) {
            let data = self.iram[(self.index & 0x7F) as usize];
            if (self.index & 0x80) != 0 {
                self.index = (self.index & 0x80) | ((self.index.wrapping_add(1)) & 0x7F);
            }
            return FetchResult { data, driven: true };
        }
        if (0x5000..=0x57FF).contains(&address) {
            return FetchResult {
                data: (self.counter & 0xFF) as u8,
                driven: true,
            };
        }
        if (0x5800..=0x5FFF).contains(&address) {
            return FetchResult {
                data: ((self.counter >> 8) & 0xFF) as u8,
                driven: true,
            };
        }
        if (0x6000..=0x7FFF).contains(&address) {
            if cart.prg_ram.is_empty() {
                return FetchResult { data: 0, driven: false };
            }
            let off = (address - 0x6000) as usize % cart.prg_ram.len();
            return FetchResult {
                data: cart.prg_ram[off],
                driven: true,
            };
        }
        if address >= 0x8000 {
            let bank_idx = ((address - 0x8000) >> 13) as usize;
            let bank = (self.prg[bank_idx] & 0x3F) as usize;
            let banks_8k = (cart.prg_rom.len() / 0x2000).max(1);
            let offset = (bank % banks_8k) * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x4800..=0x4FFF).contains(&address) {
            self.iram[(self.index & 0x7F) as usize] = data;
            if (self.index & 0x80) != 0 {
                self.index = (self.index & 0x80) | ((self.index.wrapping_add(1)) & 0x7F);
            }
        }
        else if (0x5000..=0x57FF).contains(&address) {
            self.irq_ack = true;
            self.irq_pending = false;
            self.counter = (self.counter & !0x00FF) | (data as i16);
        }
        else if (0x5800..=0x5FFF).contains(&address) {
            self.irq_ack = true;
            self.irq_pending = false;
            self.counter = (self.counter & 0x00FF) | ((data as i16) << 8);
        }
        else if (0x6000..=0x7FFF).contains(&address) {
            let bank_2k = ((address - 0x6000) >> 11) as usize; 
            let global_write_enable = (self.wram_protect & 0xF0) == 0x40;
            let bank_write_enable = (self.wram_protect & (1 << bank_2k)) == 0;
            if global_write_enable && bank_write_enable && !cart.prg_ram.is_empty() {
                let off = (address - 0x6000) as usize % cart.prg_ram.len();
                cart.prg_ram[off] = data;
            }
        }
        else if (0x8000..=0xDFFF).contains(&address) {
            let reg = ((address - 0x8000) >> 11) as usize; 
            self.chr[reg] = data;
        }
        else if (0xE000..=0xE7FF).contains(&address) {
            self.prg[0] = data;
            self.sound_enabled = (data & 0x40) == 0;
        }
        else if (0xE800..=0xEFFF).contains(&address) {
            self.prg[1] = data;
        }
        else if (0xF000..=0xF7FF).contains(&address) {
            self.prg[2] = data;
        }
        else if address >= 0xF800 {
            self.wram_protect = data;
            self.index = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        address
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

        if address < 0x2000 {
            let reg_idx = (address >> 10) as usize;
            let force_vrom = if reg_idx < 4 {
                (self.prg[1] & 0x40) != 0
            } else {
                (self.prg[1] & 0x80) != 0
            };
            let bank = self.chr[reg_idx];
            let byte = if bank < 0xE0 || force_vrom {
                let offset = (bank as usize) * 0x400 + (address as usize & 0x3FF);
                if using_chr_ram || chr_rom.is_empty() {
                    if !chr_ram.is_empty() {
                        chr_ram[offset % chr_ram.len()]
                    } else {
                        0
                    }
                } else {
                    chr_rom[offset % chr_rom.len()]
                }
            } else if using_chr_ram && !chr_ram.is_empty() {
                let offset = (bank as usize) * 0x400 + (address as usize & 0x3FF);
                chr_ram[offset % chr_ram.len()]
            } else {
                let nt_page = (bank & 1) as usize;
                let vram_offset = (nt_page << 10) | (address as usize & 0x3FF);
                if !vram.is_empty() {
                    vram[vram_offset % vram.len()]
                } else {
                    0
                }
            };
            new_addr_bus |= byte as u16;
        } else if address >= 0x2000 && address < 0x3F00 {
            let reg_idx = 8 + (((address - 0x2000) >> 10) & 3) as usize;
            let bank = self.chr[reg_idx];
            let byte = if bank < 0xE0 {
                let offset = (bank as usize) * 0x400 + (address as usize & 0x3FF);
                if using_chr_ram || chr_rom.is_empty() {
                    if !chr_ram.is_empty() {
                        chr_ram[offset % chr_ram.len()]
                    } else {
                        0
                    }
                } else {
                    chr_rom[offset % chr_rom.len()]
                }
            } else if using_chr_ram && !chr_ram.is_empty() {
                let offset = (bank as usize) * 0x400 + (address as usize & 0x3FF);
                chr_ram[offset % chr_ram.len()]
            } else {
                let nt_page = (bank & 1) as usize;
                let vram_offset = (nt_page << 10) | (address as usize & 0x3FF);
                if !vram.is_empty() {
                    vram[vram_offset % vram.len()]
                } else {
                    0
                }
            };
            new_addr_bus |= byte as u16;
        }

        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            let reg_idx = (address >> 10) as usize;
            let force_vrom = if reg_idx < 4 {
                (self.prg[1] & 0x40) != 0
            } else {
                (self.prg[1] & 0x80) != 0
            };
            let bank = self.chr[reg_idx];
            if bank >= 0xE0 && !force_vrom {
                if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                    let offset = (bank as usize) * 0x400 + (address as usize & 0x3FF);
                    let len = cart.chr_ram.len();
                    cart.chr_ram[offset % len] = data;
                } else {
                    let nt_page = (bank & 1) as usize;
                    let vram_offset = (nt_page << 10) | (address as usize & 0x3FF);
                    vram[vram_offset & 0x7FF] = data;
                }
            } else if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let offset = (bank as usize) * 0x400 + (address as usize & 0x3FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let reg_idx = 8 + (((address - 0x2000) >> 10) & 3) as usize;
            let bank = self.chr[reg_idx];
            if bank >= 0xE0 {
                if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                    let offset = (bank as usize) * 0x400 + (address as usize & 0x3FF);
                    let len = cart.chr_ram.len();
                    cart.chr_ram[offset % len] = data;
                } else {
                    let nt_page = (bank & 1) as usize;
                    let vram_offset = (nt_page << 10) | (address as usize & 0x3FF);
                    vram[vram_offset & 0x7FF] = data;
                }
            } else if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let offset = (bank as usize) * 0x400 + (address as usize & 0x3FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        for _ in 0..cycles {
            if self.counter < -1 {
                self.counter = self.counter.wrapping_add(1);
                if self.counter == -1 {
                    self.irq_pending = true;
                }
            }
        }
        self.clock_sound(cycles);
        self.irq_pending
    }

    fn cpu_clock_irq_level(&self) -> bool {
        self.irq_pending
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack;
        self.irq_ack = false;
        ack
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.extend_from_slice(&self.prg);
        state.extend_from_slice(&self.chr);
        state.push(self.wram_protect);
        state.extend_from_slice(&self.counter.to_le_bytes());
        state.push(self.index);
        state.push(if self.sound_enabled { 1 } else { 0 });
        state.push(if self.irq_pending { 1 } else { 0 });
        state.extend_from_slice(&self.iram);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        let prg_len = cart.prg_ram.len();
        if start + prg_len <= state.len() {
            cart.prg_ram.copy_from_slice(&state[start..start + prg_len]);
            start += prg_len;
        }
        let chr_len = cart.chr_ram.len();
        if start + chr_len <= state.len() {
            cart.chr_ram.copy_from_slice(&state[start..start + chr_len]);
            start += chr_len;
        }
        if start + 4 <= state.len() {
            self.prg.copy_from_slice(&state[start..start + 4]);
            start += 4;
        }
        if start + 12 <= state.len() {
            self.chr.copy_from_slice(&state[start..start + 12]);
            start += 12;
        }
        if start + 6 <= state.len() {
            self.wram_protect = state[start];
            self.counter = i16::from_le_bytes([state[start + 1], state[start + 2]]);
            self.index = state[start + 3];
            self.sound_enabled = state[start + 4] != 0;
            self.irq_pending = state[start + 5] != 0;
            start += 6;
        }
        if start + 128 <= state.len() {
            self.iram.copy_from_slice(&state[start..start + 128]);
            start += 128;
        }
        start
    }

    fn audio_sample(&self) -> f32 {
        self.current_audio_sample
    }

    fn expansion_audio_type(&self) -> ExpansionAudioType {
        ExpansionAudioType::Namco163
    }
}
