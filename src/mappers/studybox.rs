use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::studybox::{find_page_index, StudyBoxTape};

pub struct MapperStudyBox {
    tape: StudyBoxTape,

    ready_for_bit: bool,
    process_bit_delay: u16,
    reg4202: u8,
    reg4200: u8,
    bg_prg_bank: u16,

    command_counter: u8,
    command: u8,

    current_page: u8,
    seek_page: i16,
    seek_page_delay: u32,

    enable_decoder: bool,
    audio_enabled: bool,
    motor_disabled: bool,
    byte_read_delay: u16,
    irq_enabled: bool,
    irq_pending: bool,

    page_found: bool,
    page_index: usize,
    page_position: i32,

    in_data_delay: u32,
    in_data_region: bool,

    wave_started: bool,
    wave_position: f64,
    audio_sample_rate: u32,

    cpu_clock_freq: f64,
}

impl MapperStudyBox {
    pub fn new(tape: StudyBoxTape) -> Self {
        let audio_sample_rate = tape
            .audio
            .as_ref()
            .map(|w| w.sample_rate)
            .unwrap_or(44100);
        Self {
            tape,
            ready_for_bit: false,
            process_bit_delay: 0,
            reg4202: 0,
            reg4200: 0,
            bg_prg_bank: 0,
            command_counter: 0,
            command: 0,
            current_page: 0,
            seek_page: 0,
            seek_page_delay: 0,
            enable_decoder: false,
            audio_enabled: false,
            motor_disabled: true,
            byte_read_delay: 0,
            irq_enabled: false,
            irq_pending: false,
            page_found: false,
            page_index: 0,
            page_position: -1,
            in_data_delay: 0,
            in_data_region: false,
            wave_started: false,
            wave_position: 0.0,
            audio_sample_rate,
            cpu_clock_freq: 1_789_773.0,
        }
    }

    fn read_lead_in_track(&mut self) {
        if self.page_index >= self.tape.pages.len() {
            return;
        }
        let page = &self.tape.pages[self.page_index];
        let wave_rate = self
            .tape
            .audio
            .as_ref()
            .map(|w| w.sample_rate as u32)
            .unwrap_or(self.audio_sample_rate) as f64;
        if wave_rate <= 0.0 {
            self.in_data_delay = 0;
        } else {
            let diff = page.audio_offset.saturating_sub(page.lead_in_offset) as f64;
            self.in_data_delay = (diff * self.cpu_clock_freq / wave_rate).round() as u32;
        }
        self.page_position = -1;
        self.byte_read_delay = 0;
        self.motor_disabled = false;
        self.page_found = true;
    }

    fn process_cpu_clock(&mut self) {
        if self.process_bit_delay > 0 {
            self.process_bit_delay -= 1;
            if self.process_bit_delay == 0 {
                self.ready_for_bit = true;
            }
        }

        if self.motor_disabled {
            return;
        }

        if self.seek_page != self.current_page as i16 {
            self.seek_page_delay = self.seek_page_delay.saturating_sub(1);
            if self.seek_page_delay == 0 {
                self.seek_page_delay = 3_000_000;
                if self.seek_page > self.current_page as i16 {
                    self.current_page = self.current_page.wrapping_add(1);
                } else {
                    self.current_page = self.current_page.wrapping_sub(1);
                }
                self.page_index = find_page_index(&self.tape.pages, self.current_page.wrapping_sub(1));
                self.read_lead_in_track();
            }
        } else if self.in_data_delay > 0 {
            self.in_data_region = true;
            self.in_data_delay -= 1;
            if self.in_data_delay == 0 {
                self.byte_read_delay = 7820;
                if self.tape.audio.is_some() {
                    let audio_off = self
                        .tape
                        .pages
                        .get(self.page_index)
                        .map(|p| p.audio_offset as f64)
                        .unwrap_or(0.0);
                    self.wave_started = true;
                    self.wave_position = audio_off;
                }
                self.irq_pending = true;
            }
        } else if self.byte_read_delay > 0 {
            self.byte_read_delay -= 1;
            if self.byte_read_delay == 0 {
                self.byte_read_delay = 3355;
                self.page_position += 1;

                if self.page_position >= self.tape.pages[self.page_index].data.len() as i32 {
                    self.page_found = false;
                    self.in_data_region = false;
                    self.motor_disabled = true;
                    self.wave_started = false;
                }

                if self.irq_enabled {
                    self.irq_pending = true;
                }
            }
        }
    }
}

impl Mapper for MapperStudyBox {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        match address {
            0x4200 => {
                self.irq_pending = false;
                if self.page_position >= 0
                    && self.page_index < self.tape.pages.len()
                    && (self.page_position as usize) < self.tape.pages[self.page_index].data.len()
                {
                    return FetchResult {
                        data: self.tape.pages[self.page_index].data[self.page_position as usize],
                        driven: true,
                    };
                }
                FetchResult { data: 0xAA, driven: true }
            }
            0x4201 => {
                let mut value = 0u8;
                if self.in_data_region {
                    value |= 0x20;
                }
                if self.page_found {
                    value |= 0x40;
                }
                if self.enable_decoder {
                    value |= 0x80;
                }
                self.page_found = false;
                FetchResult { data: value, driven: true }
            }
            0x4202 => {
                let value = if self.ready_for_bit { 0x40 } else { 0 };
                FetchResult { data: value, driven: true }
            }
            0x4203 => FetchResult { data: 0x00, driven: true },
            0x4400..=0x4FFF => {
                let offset = 0x8000 + (address as usize - 0x4000);
                FetchResult {
                    data: cart.prg_ram.get(offset).copied().unwrap_or(0),
                    driven: cart.prg_ram.get(offset).is_some(),
                }
            }
            0x5000..=0x5FFF => {
                let bank = ((self.reg4200 & 0x07) as usize) + 8;
                let offset = bank * 0x1000 + (address as usize - 0x5000);
                FetchResult {
                    data: cart.prg_ram.get(offset).copied().unwrap_or(0),
                    driven: cart.prg_ram.get(offset).is_some(),
                }
            }
            0x6000..=0x6FFF => {
                let bank = ((self.reg4200 & 0xC0) >> 5) as usize;
                let offset = bank * 0x1000 + (address as usize - 0x6000);
                FetchResult {
                    data: cart.prg_ram.get(offset).copied().unwrap_or(0),
                    driven: cart.prg_ram.get(offset).is_some(),
                }
            }
            0x7000..=0x7FFF => {
                let bank = (((self.reg4200 & 0xC0) >> 5) as usize) + 1;
                let offset = bank * 0x1000 + (address as usize - 0x7000);
                FetchResult {
                    data: cart.prg_ram.get(offset).copied().unwrap_or(0),
                    driven: cart.prg_ram.get(offset).is_some(),
                }
            }
            0x8000..=0xBFFF => {
                let page_count = (cart.prg_rom.len() / 0x4000).max(1);
                let bank = (self.bg_prg_bank as usize) % page_count;
                let offset = bank * 0x4000 + (address as usize - 0x8000);
                FetchResult {
                    data: cart.prg_rom.get(offset).copied().unwrap_or(0),
                    driven: cart.prg_rom.get(offset).is_some(),
                }
            }
            0xC000..=0xFFFF => {
                let offset = address as usize - 0xC000;
                FetchResult {
                    data: cart.prg_rom.get(offset).copied().unwrap_or(0),
                    driven: cart.prg_rom.get(offset).is_some(),
                }
            }
            _ => FetchResult { data: 0, driven: false },
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        match address {
            0x4200 => {
                self.reg4200 = data;
            }
            0x4201 => {
                self.bg_prg_bank = data as u16;
            }
            0x4202 => {
                self.store_tape_control(data);
            }
            0x4203 => {}
            0x4400..=0x4FFF => {
                let offset = 0x8000 + (address as usize - 0x4000);
                if let Some(slot) = cart.prg_ram.get_mut(offset) {
                    *slot = data;
                }
            }
            0x5000..=0x5FFF => {
                let bank = ((self.reg4200 & 0x07) as usize) + 8;
                let offset = bank * 0x1000 + (address as usize - 0x5000);
                if let Some(slot) = cart.prg_ram.get_mut(offset) {
                    *slot = data;
                }
            }
            0x6000..=0x6FFF => {
                let bank = ((self.reg4200 & 0xC0) >> 5) as usize;
                let offset = bank * 0x1000 + (address as usize - 0x6000);
                if let Some(slot) = cart.prg_ram.get_mut(offset) {
                    *slot = data;
                }
            }
            0x7000..=0x7FFF => {
                let bank = (((self.reg4200 & 0xC0) >> 5) as usize) + 1;
                let offset = bank * 0x1000 + (address as usize - 0x7000);
                if let Some(slot) = cart.prg_ram.get_mut(offset) {
                    *slot = data;
                }
            }
            _ => {}
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        crate::mappers::nrom::mirror_address(
            cart.alternative_nametable_arrangement,
            cart.nametable_horizontal_mirroring,
            address,
        )
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            if using_chr_ram && !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[(address as usize) & (chr_ram.len() - 1)] as u16;
            } else if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[address as usize % chr_rom.len()] as u16;
            }
        } else {
            let mirrored = crate::mappers::nrom::mirror_address(
                alternative_nametable_arrangement,
                nametable_horizontal_mirroring,
                address,
            );
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() {
                    prg_vram[idx]
                } else {
                    0
                }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let mask = cart.chr_ram.len() - 1;
                cart.chr_ram[(address as usize) & mask] = data;
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

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.process_cpu_clock();
        self.irq_pending
    }

    fn cpu_clock_irq_level(&self) -> bool {
        true
    }

    fn is_study_box(&self) -> bool {
        true
    }

    fn reset(&mut self) {
        self.ready_for_bit = false;
        self.process_bit_delay = 0;
        self.reg4202 = 0;
        self.reg4200 = 0;
        self.bg_prg_bank = 0;
        self.command_counter = 0;
        self.command = 0;
        self.current_page = 0;
        self.seek_page = 0;
        self.seek_page_delay = 0;
        self.enable_decoder = false;
        self.audio_enabled = false;
        self.motor_disabled = true;
        self.byte_read_delay = 0;
        self.irq_enabled = false;
        self.irq_pending = false;
        self.page_found = false;
        self.page_index = 0;
        self.page_position = -1;
        self.in_data_delay = 0;
        self.in_data_region = false;
        self.wave_started = false;
        self.wave_position = 0.0;
    }

    fn set_cpu_clock(&mut self, clock: f64) {
        self.cpu_clock_freq = clock;
    }

    fn audio_sample(&self) -> f32 {
        0.0
    }

    fn extra_audio(&mut self, out: &mut Vec<f32>, count: usize, host_sample_rate: u32) {
        if self.motor_disabled
            || !self.wave_started
            || self.audio_sample_rate == 0
            || host_sample_rate == 0
        {
            out.resize(out.len() + count, 0.0);
            return;
        }

        let wave = match self.tape.audio.as_ref() {
            Some(w) => w,
            None => {
                out.resize(out.len() + count, 0.0);
                return;
            }
        };
        let wave_rate = wave.sample_rate.max(1) as f64;
        let wave_len = wave.samples.len();
        let step = wave_rate / host_sample_rate as f64;

        if wave_len == 0 {
            out.resize(out.len() + count, 0.0);
            self.wave_started = false;
            return;
        }

        let mut produced = 0usize;
        while produced < count {
            if self.wave_position >= wave_len as f64 {
                break;
            }
            let i = self.wave_position.floor() as usize;
            let frac = (self.wave_position - i as f64) as f32;
            let s0 = wave.samples[i];
            let s1 = if i + 1 < wave_len { wave.samples[i + 1] } else { s0 };
            if frac > 0.0 {
                out.push(s0 + (s1 - s0) * frac);
            } else {
                out.push(s0);
            }
            produced += 1;
            self.wave_position += step;
        }

        if produced < count {
            self.wave_started = false;
            out.resize(out.len() + (count - produced), 0.0);
        }
    }

    fn set_audio_sample_rate(&mut self, sample_rate: u32) {
        self.audio_sample_rate = sample_rate;
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut v = Vec::new();
        v.push(self.ready_for_bit as u8);
        v.extend_from_slice(&self.process_bit_delay.to_le_bytes());
        v.push(self.reg4202);
        v.push(self.reg4200);
        v.extend_from_slice(&self.bg_prg_bank.to_le_bytes());
        v.push(self.command_counter);
        v.push(self.command);
        v.push(self.current_page);
        v.extend_from_slice(&self.seek_page.to_le_bytes());
        v.extend_from_slice(&self.seek_page_delay.to_le_bytes());
        v.push(self.enable_decoder as u8);
        v.push(self.audio_enabled as u8);
        v.push(self.motor_disabled as u8);
        v.extend_from_slice(&self.byte_read_delay.to_le_bytes());
        v.push(self.irq_enabled as u8);
        v.push(self.page_found as u8);
        v.extend_from_slice(&(self.page_index as u32).to_le_bytes());
        v.extend_from_slice(&self.page_position.to_le_bytes());
        v.extend_from_slice(&self.in_data_delay.to_le_bytes());
        v.push(self.in_data_region as u8);
        v.push(self.irq_pending as u8);
        v.push(self.wave_started as u8);
        v.extend_from_slice(&self.wave_position.to_le_bytes());
        v
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start + 31 > state.len() {
            return start;
        }
        let mut p = start;
        self.ready_for_bit = state[p] != 0;
        p += 1;
        self.process_bit_delay = u16::from_le_bytes([state[p], state[p + 1]]);
        p += 2;
        self.reg4202 = state[p];
        p += 1;
        self.reg4200 = state[p];
        p += 1;
        self.bg_prg_bank = u16::from_le_bytes([state[p], state[p + 1]]);
        p += 2;
        self.command_counter = state[p];
        p += 1;
        self.command = state[p];
        p += 1;
        self.current_page = state[p];
        p += 1;
        self.seek_page = i16::from_le_bytes([state[p], state[p + 1]]);
        p += 2;
        self.seek_page_delay = u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
        p += 4;
        self.enable_decoder = state[p] != 0;
        p += 1;
        self.audio_enabled = state[p] != 0;
        p += 1;
        self.motor_disabled = state[p] != 0;
        p += 1;
        self.byte_read_delay = u16::from_le_bytes([state[p], state[p + 1]]);
        p += 2;
        self.irq_enabled = state[p] != 0;
        p += 1;
        self.page_found = state[p] != 0;
        p += 1;
        self.page_index = u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]) as usize;
        p += 4;
        self.page_position = i32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
        p += 4;
        self.in_data_delay = u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
        p += 4;
        self.in_data_region = state[p] != 0;
        p += 1;
        if p < state.len() {
            self.irq_pending = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.wave_started = state[p] != 0;
            p += 1;
        }
        if p + 8 <= state.len() {
            self.wave_position = f64::from_le_bytes(state[p..p + 8].try_into().unwrap());
            p += 8;
        }
        p
    }
}

impl MapperStudyBox {
    fn store_tape_control(&mut self, value: u8) {
        if (self.reg4202 & 0x10) != 0 && (value & 0x10) == 0 {
            if !self.ready_for_bit {
            }
            self.command <<= 1;
            self.command |= (value & 0x80) >> 7;
            self.command_counter += 1;

            if self.command_counter == 8 {
                self.command_counter = 0;

                if self.command >= 1 && self.command < 0x40 {
                    self.seek_page = self.command as i16 + self.current_page as i16;
                    self.seek_page_delay = 3_000_000;
                    self.motor_disabled = false;
                } else if self.command > 0x40 && self.command < 0x80 {
                    self.seek_page = -(self.command as i16 - 0x40) + self.current_page as i16;
                    self.seek_page_delay = 3_000_000;
                    self.motor_disabled = false;
                } else if self.command == 0 {
                    self.seek_page = self.current_page as i16;
                    self.current_page = self.current_page.wrapping_sub(1);
                    self.seek_page_delay = 3_000_000;
                    self.motor_disabled = false;
                } else if self.command == 0x86 {
                    if self.page_index < self.tape.pages.len() - 1 {
                        self.page_index += 1;
                    } else {
                        self.page_index = 0;
                    }
                    self.read_lead_in_track();
                }
            }
        }

        if value & 0x10 != 0 {
            self.ready_for_bit = false;
            self.process_bit_delay = 100;
        }

        if (self.reg4202 & 0x20) != 0 && (value & 0x20) == 0 {
            self.command = 0;
            self.command_counter = 0;
            self.ready_for_bit = true;
        }

        if (value & 0x04) != (self.reg4202 & 0x04) {
            self.audio_enabled = (value & 0x04) == 0;
        }

        self.reg4202 = value;
        self.enable_decoder = value & 0x01 != 0;
        self.irq_enabled = value & 0x02 != 0;
        self.irq_pending = false;
    }
}
