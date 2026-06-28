use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

fn is_nes20(header: &[u8]) -> bool {
    header.len() >= 16 && (header[7] & 0x0C) == 0x08
}

fn nes20_ram_kb(shift: u8) -> usize {
    if shift == 0 {
        0
    } else {
        (64usize << shift) / 1024
    }
}

pub fn wram_size(header: &[u8]) -> usize {
    if is_nes20(header) {
        let volatile_kb = nes20_ram_kb(header[10] & 0x0F);
        let battery_kb = nes20_ram_kb((header[10] >> 4) & 0x0F);
        (volatile_kb + battery_kb) * 1024
    } else {
        0x2000
    }
}

pub struct MapperFME7 {
    cmd: u8,
    chr_1k: [u8; 8],
    bank_6: u8,
    bank_6_is_ram: bool,
    bank_6_is_ram_enabled: bool,
    bank_8: u8,
    bank_a: u8,
    bank_c: u8,
    mirr: u8,
    irqa: u8,
    irq_count: i32,
    sndcmd: u8,
    sreg: [u8; 14],
    vcount: [i32; 3],
    dcount: [u8; 3],
    current_audio_sample: f32,
    irq_ack_pending: bool,
}

impl MapperFME7 {
    pub fn new() -> Self {
        Self {
            cmd: 0,
            chr_1k: [0; 8],
            bank_6: 0,
            bank_6_is_ram: false,
            bank_6_is_ram_enabled: false,
            bank_8: 0,
            bank_a: 0,
            bank_c: 0,
            mirr: 0,
            irqa: 0,
            irq_count: 0xFFFF,
            sndcmd: 0,
            sreg: [0; 14],
            vcount: [0; 3],
            dcount: [0; 3],
            current_audio_sample: 0.0,
            irq_ack_pending: false,
        }
    }

    fn mirror_address(&self, address: u16) -> u16 {
        match self.mirr & 3 {
            0 => address & 0x37FF,
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            2 => address & 0x33FF,
            3 => (address & 0x33FF) | 0x0400,
            _ => address & 0x37FF,
        }
    }

    fn chr_bank_index(&self, address: u16) -> usize {
        let slot = (address >> 10) as usize & 7;
        self.chr_1k[slot] as usize
    }

    fn read_chr(
        &self,
        address: u16,
        chr_rom: &[u8],
        chr_ram: &[u8],
    ) -> u8 {
        let len = if !chr_ram.is_empty() {
            chr_ram.len()
        } else {
            chr_rom.len()
        };
        if len == 0 {
            return 0;
        }
        let bank = self.chr_bank_index(address);
        let offset = bank * 0x400 + (address as usize & 0x3FF);
        if !chr_ram.is_empty() {
            chr_ram[offset % len]
        } else {
            chr_rom[offset % len]
        }
    }

    fn chr_write_offset(&self, address: u16, len: usize) -> usize {
        let bank = self.chr_bank_index(address);
        (bank * 0x400 + (address as usize & 0x3FF)) % len
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

impl Mapper for MapperFME7 {
    fn reset(&mut self) {
        *self = Self::new();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x6000 {
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        let tempo = (address as usize) & 0x1FFF;
        if address < 0x8000 {
            if self.bank_6_is_ram {
                if self.bank_6_is_ram_enabled && !cart.prg_ram.is_empty() {
                    FetchResult {
                        data: cart.prg_ram[tempo % cart.prg_ram.len()],
                        driven: true,
                    }
                } else {
                    FetchResult {
                        data: 0,
                        driven: false,
                    }
                }
            } else {
                let offset = (self.bank_6 as usize) * 0x2000 + tempo;
                FetchResult {
                    data: cart.prg_rom[offset % cart.prg_rom.len()],
                    driven: true,
                }
            }
        } else if address < 0xA000 {
            let offset = (self.bank_8 as usize) * 0x2000 + tempo;
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if address < 0xC000 {
            let offset = (self.bank_a as usize) * 0x2000 + tempo;
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if address < 0xE000 {
            let offset = (self.bank_c as usize) * 0x2000 + tempo;
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else {
            let offset = cart.prg_rom.len().saturating_sub(0x2000) + tempo;
            FetchResult {
                data: cart.prg_rom[offset.min(cart.prg_rom.len().saturating_sub(1))],
                driven: true,
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0xC000 {
            if address < 0xE000 {
                self.sndcmd = data % 14;
            } else {
                self.sound_data_write(data);
            }
            return;
        }
        if address < 0x6000 {
            return;
        }
        if address < 0x8000 {
            if self.bank_6_is_ram
                && self.bank_6_is_ram_enabled
                && !cart.prg_ram.is_empty()
            {
                let offset = (address as usize & 0x1FFF) % cart.prg_ram.len();
                cart.prg_ram[offset] = data;
            }
        } else if address < 0xA000 {
            self.cmd = data & 0x0F;
        } else if address < 0xC000 {
            match self.cmd {
                0..=7 => self.chr_1k[self.cmd as usize] = data,
                8 => {
                    self.bank_6 = data & 0x3F;
                    self.bank_6_is_ram = (data & 0x40) != 0;
                    self.bank_6_is_ram_enabled = (data & 0x80) != 0;
                }
                9 => self.bank_8 = data & 0x3F,
                10 => self.bank_a = data & 0x3F,
                11 => self.bank_c = data & 0x3F,
                12 => self.mirr = data & 0x03,
                13 => {
                    self.irqa = data;
                    self.irq_ack_pending = true;
                }
                14 => self.irq_count = (self.irq_count & 0xFF00) | data as i32,
                15 => self.irq_count = (self.irq_count & 0x00FF) | ((data as i32) << 8),
                _ => {}
            }
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
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = self.read_chr(address, chr_rom, chr_ram);
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else {
                self.mirror_address(address)
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                let offset = self.chr_write_offset(address, len);
                cart.chr_ram[offset] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = if cart.alternative_nametable_arrangement {
                address
            } else {
                self.mirror_address(address)
            };
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.tick_audio();
        if self.irqa != 0 {
            self.irq_count -= 1;
            if self.irq_count <= 0 {
                self.irqa = 0;
                self.irq_count = 0xFFFF;
                return true;
            }
        }
        false
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack_pending;
        self.irq_ack_pending = false;
        ack
    }

    fn audio_sample(&self) -> f32 {
        self.current_audio_sample
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.push(self.cmd);
        state.extend_from_slice(&self.chr_1k);
        state.push(self.bank_6);
        state.push(u8::from(self.bank_6_is_ram));
        state.push(u8::from(self.bank_6_is_ram_enabled));
        state.push(self.bank_8);
        state.push(self.bank_a);
        state.push(self.bank_c);
        state.push(self.mirr);
        state.push(self.irqa);
        state.extend_from_slice(&self.irq_count.to_le_bytes());
        state.push(self.sndcmd);
        state.extend_from_slice(&self.sreg);
        for i in 0..3 {
            state.extend_from_slice(&self.vcount[i].to_le_bytes());
            state.push(self.dcount[i]);
        }
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..cart.prg_ram.len() {
            if p >= state.len() {
                return p;
            }
            cart.prg_ram[i] = state[p];
            p += 1;
        }
        for i in 0..cart.chr_ram.len() {
            if p >= state.len() {
                return p;
            }
            cart.chr_ram[i] = state[p];
            p += 1;
        }
        if p + 20 > state.len() {
            return p;
        }
        self.cmd = state[p];
        p += 1;
        self.chr_1k.copy_from_slice(&state[p..p + 8]);
        p += 8;
        self.bank_6 = state[p];
        p += 1;
        self.bank_6_is_ram = state[p] != 0;
        p += 1;
        self.bank_6_is_ram_enabled = state[p] != 0;
        p += 1;
        self.bank_8 = state[p];
        p += 1;
        self.bank_a = state[p];
        p += 1;
        self.bank_c = state[p];
        p += 1;
        self.mirr = state[p];
        p += 1;
        self.irqa = state[p];
        p += 1;
        if p + 4 <= state.len() {
            self.irq_count = i32::from_le_bytes([
                state[p],
                state[p + 1],
                state[p + 2],
                state[p + 3],
            ]);
            p += 4;
        }
        if p + 15 <= state.len() {
            self.sndcmd = state[p];
            p += 1;
            self.sreg.copy_from_slice(&state[p..p + 14]);
            p += 14;
            for i in 0..3 {
                if p + 5 > state.len() {
                    break;
                }
                self.vcount[i] = i32::from_le_bytes([
                    state[p],
                    state[p + 1],
                    state[p + 2],
                    state[p + 3],
                ]);
                p += 4;
                self.dcount[i] = state[p];
                p += 1;
            }
        }
        p
    }
}
