use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::rainbow_audio::{Vrc6Pulse, Vrc6Saw};

#[derive(Clone, Copy)]
pub enum Vrc6Variant {
    Mapper24,
    Mapper26,
}

pub struct Vrc6 {
    variant: Vrc6Variant,
    prg: [u8; 2],
    chr: [u8; 8],
    mirr_ctrl: u8,
    irq_latch: u8,
    irq_enabled: bool,
    irq_reload: bool,
    irq_mode: bool,
    irq_count: u8,
    cycle_count: i32,
    has_wram: bool,
    irq_ack: bool,
    pub pulse1: Vrc6Pulse,
    pub pulse2: Vrc6Pulse,
    pub saw: Vrc6Saw,
    pub halt_audio: bool,
}

impl Vrc6 {
    pub fn new(variant: Vrc6Variant) -> Self {
        Vrc6 {
            variant,
            prg: [0; 2],
            chr: [0; 8],
            mirr_ctrl: 0,
            irq_latch: 0,
            irq_enabled: false,
            irq_reload: false,
            irq_mode: false,
            irq_count: 0,
            cycle_count: 0,
            has_wram: true,
            irq_ack: false,
            pulse1: Vrc6Pulse::new(),
            pulse2: Vrc6Pulse::new(),
            saw: Vrc6Saw::new(),
            halt_audio: false,
        }
    }

    fn decode_address(&self, address: u16) -> u16 {
        match self.variant {
            Vrc6Variant::Mapper24 => address,
            Vrc6Variant::Mapper26 => {
                (address & 0xFFFC) | ((address >> 1) & 1) | ((address << 1) & 2)
            }
        }
    }

    fn get_chr_bank(&self, slot: usize) -> usize {
        let mask = if (self.mirr_ctrl & 0x20) != 0 { 0xFE } else { 0xFF };
        let or_mask = if (self.mirr_ctrl & 0x20) != 0 { (slot & 1) as u8 } else { 0 };

        match self.mirr_ctrl & 0x03 {
            0 => self.chr[slot] as usize,
            1 => {
                let reg = self.chr[slot / 2];
                ((reg & mask) | or_mask) as usize
            }
            _ => {
                if slot < 4 {
                    self.chr[slot] as usize
                } else {
                    let reg = self.chr[4 + (slot - 4) / 2];
                    ((reg & mask) | or_mask) as usize
                }
            }
        }
    }

    fn get_ciram_page(&self, slot: usize) -> usize {
        match self.mirr_ctrl & 0x2F {
            0x20 | 0x27 => {
                // Vertical
                slot & 1
            }
            0x23 | 0x24 => {
                // Horizontal
                (slot >> 1) & 1
            }
            0x28 | 0x2F => {
                // Screen A
                0
            }
            0x2B | 0x2C => {
                // Screen B
                1
            }
            _ => match self.mirr_ctrl & 0x07 {
                0 | 6 | 7 => match slot {
                    0 | 1 => (self.chr[6] & 1) as usize,
                    _ => (self.chr[7] & 1) as usize,
                },
                1 | 5 => (self.chr[4 + slot] & 1) as usize,
                _ => match slot {
                    0 | 2 => (self.chr[6] & 1) as usize,
                    _ => (self.chr[7] & 1) as usize,
                },
            },
        }
    }

    fn get_rom_nt_bank(&self, slot: usize) -> usize {
        match self.mirr_ctrl & 0x2F {
            0x20 | 0x27 => match slot {
                0 => (self.chr[6] & 0xFE) as usize,
                1 => ((self.chr[6] & 0xFE) | 1) as usize,
                2 => (self.chr[7] & 0xFE) as usize,
                _ => ((self.chr[7] & 0xFE) | 1) as usize,
            },
            0x23 | 0x24 => match slot {
                0 => (self.chr[6] & 0xFE) as usize,
                1 => (self.chr[7] & 0xFE) as usize,
                2 => ((self.chr[6] & 0xFE) | 1) as usize,
                _ => ((self.chr[7] & 0xFE) | 1) as usize,
            },
            0x28 | 0x2F => match slot {
                0 | 1 => (self.chr[6] & 0xFE) as usize,
                _ => (self.chr[7] & 0xFE) as usize,
            },
            0x2B | 0x2C => match slot {
                0 => ((self.chr[6] & 0xFE) | 1) as usize,
                1 => ((self.chr[7] & 0xFE) | 1) as usize,
                2 => ((self.chr[6] & 0xFE) | 1) as usize,
                _ => ((self.chr[7] & 0xFE) | 1) as usize,
            },
            _ => match self.mirr_ctrl & 0x07 {
                0 | 6 | 7 => match slot {
                    0 | 1 => self.chr[6] as usize,
                    _ => self.chr[7] as usize,
                },
                1 | 5 => self.chr[4 + slot] as usize,
                _ => match slot {
                    0 | 2 => self.chr[6] as usize,
                    _ => self.chr[7] as usize,
                },
            },
        }
    }
}

impl Mapper for Vrc6 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let (bank, bank_size) = match address {
                0x8000..=0xBFFF => (self.prg[0] as usize, 0x4000),
                0xC000..=0xDFFF => (self.prg[1] as usize, 0x2000),
                0xE000..=0xFFFF => ((cart.prg_rom.len() / 0x2000 - 1) as usize, 0x2000),
                _ => (0, 0x2000),
            };
            let offset = (bank * bank_size) + (address as usize & (bank_size - 1));
            let final_offset = offset % cart.prg_rom.len();
            FetchResult { data: cart.prg_rom[final_offset], driven: true }
        } else if address >= 0x6000 && address < 0x8000 && ((self.mirr_ctrl & 0x80) != 0 || self.has_wram) {
            if !cart.prg_ram.is_empty() {
                let idx = (address as usize - 0x6000) & (cart.prg_ram.len() - 1);
                FetchResult { data: cart.prg_ram[idx], driven: true }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let decoded = self.decode_address(address);
            match decoded & 0xF003 {
                0x8000 | 0x8001 | 0x8002 | 0x8003 => {
                    self.prg[0] = data & 0x0F;
                }
                0x9000 | 0x9001 | 0x9002 => {
                    self.pulse1.write_reg(decoded, data);
                }
                0x9003 => {
                    self.halt_audio = (data & 0x01) != 0;
                    let frequency_shift = if (data & 0x04) != 0 { 8 } else if (data & 0x02) != 0 { 4 } else { 0 };
                    self.pulse1.set_frequency_shift(frequency_shift);
                    self.pulse2.set_frequency_shift(frequency_shift);
                    self.saw.set_frequency_shift(frequency_shift);
                }
                0xA000 | 0xA001 | 0xA002 => {
                    self.pulse2.write_reg(decoded, data);
                }
                0xB000 | 0xB001 | 0xB002 => {
                    self.saw.write_reg(decoded, data);
                }
                0xB003 => {
                    self.mirr_ctrl = data;
                }
                0xC000 | 0xC001 | 0xC002 | 0xC003 => {
                    self.prg[1] = data & 0x1F;
                }
                0xD000 => {
                    self.chr[0] = data;
                }
                0xD001 => {
                    self.chr[1] = data;
                }
                0xD002 => {
                    self.chr[2] = data;
                }
                0xD003 => {
                    self.chr[3] = data;
                }
                0xE000 => {
                    self.chr[4] = data;
                }
                0xE001 => {
                    self.chr[5] = data;
                }
                0xE002 => {
                    self.chr[6] = data;
                }
                0xE003 => {
                    self.chr[7] = data;
                }
                0xF000 => {
                    self.irq_latch = data;
                }
                0xF001 => {
                    self.irq_mode = (data & 4) != 0;
                    self.irq_reload = (data & 1) != 0;
                    self.irq_enabled = (data & 2) != 0;
                    if self.irq_enabled {
                        self.irq_count = self.irq_latch;
                        self.cycle_count = 341;
                    }
                    self.irq_ack = true;
                }
                0xF002 | 0xF003 => {
                    self.irq_enabled = self.irq_reload;
                    self.irq_ack = true;
                }
                _ => {}
            }
        } else if address >= 0x6000 && address < 0x8000 && ((self.mirr_ctrl & 0x80) != 0 || self.has_wram) {
            if !cart.prg_ram.is_empty() {
                let idx = (address as usize - 0x6000) & (cart.prg_ram.len() - 1);
                cart.prg_ram[idx] = data;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let slot = ((address >> 10) & 3) as usize;
        let page = self.get_ciram_page(slot);
        0x2000 | ((page as u16) * 0x400) | (address & 0x3FF)
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
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let bank = (address >> 10) as usize & 0x07;
            let chr_bank = self.get_chr_bank(bank);
            let offset = (chr_bank * 0x400) + (address as usize & 0x3FF);
            if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            } else if !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
            }
        } else {
            let slot = ((address >> 10) & 3) as usize;
            let rom_nt = (self.mirr_ctrl & 0x10) != 0;
            if rom_nt {
                let bank = self.get_rom_nt_bank(slot);
                let offset = (bank * 0x400) + (address as usize & 0x3FF);
                let byte = if !chr_rom.is_empty() {
                    chr_rom[offset % chr_rom.len()]
                } else if !chr_ram.is_empty() {
                    chr_ram[offset & (chr_ram.len() - 1)]
                } else {
                    0
                };
                new_addr_bus |= byte as u16;
            } else {
                let page = self.get_ciram_page(slot);
                let idx = (page * 0x400) | (address as usize & 0x3FF);
                new_addr_bus |= vram[idx & 0x7FF] as u16;
            }
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        let addr = address & 0x3FFF;
        if addr < 0x2000 {
            if cart.chr_rom.is_empty() && !cart.chr_ram.is_empty() {
                let bank = (addr >> 10) as usize & 0x07;
                let chr_bank = self.get_chr_bank(bank);
                let offset = (chr_bank * 0x400) + (addr as usize & 0x3FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset & (len - 1)] = data;
            }
        } else if addr >= 0x2000 && addr < 0x3F00 {
            let rom_nt = (self.mirr_ctrl & 0x10) != 0;
            let slot = ((addr >> 10) & 3) as usize;
            if rom_nt {
                if cart.chr_rom.is_empty() && !cart.chr_ram.len() > 0 {
                    let bank = self.get_rom_nt_bank(slot);
                    let offset = (bank * 0x400) + (addr as usize & 0x3FF);
                    let len = cart.chr_ram.len();
                    cart.chr_ram[offset & (len - 1)] = data;
                }
                return;
            }
            let page = self.get_ciram_page(slot);
            let idx = (page * 0x400) | (addr as usize & 0x3FF);
            vram[idx & 0x7FF] = data;
        }
    }

    fn cpu_clock_rise(&mut self, _ppu_address_bus: u16) -> bool {
        false
    }

    fn ppu_clock(
        &mut self,
        _ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        _scanline: u16,
        _dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        false
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        for _ in 0.._cycles {
            if !self.halt_audio {
                self.pulse1.clock();
                self.pulse2.clock();
                self.saw.clock();
            }
        }
        let mut irq = false;
        if self.irq_enabled {
            for _ in 0.._cycles {
                if self.irq_mode {
                    if self.irq_count == 0xFF {
                        self.irq_count = self.irq_latch;
                        irq = true;
                    } else {
                        self.irq_count += 1;
                    }
                } else {
                    self.cycle_count -= 3;
                    if self.cycle_count <= 0 {
                        self.cycle_count += 341;
                        if self.irq_count == 0xFF {
                            self.irq_count = self.irq_latch;
                            irq = true;
                        } else {
                            self.irq_count += 1;
                        }
                    }
                }
            }
        }
        irq
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack;
        self.irq_ack = false;
        ack
    }

    fn audio_sample(&self) -> f32 {
        ((self.pulse1.get_volume() as f32)
            + (self.pulse2.get_volume() as f32)
            + (self.saw.get_volume() as f32))
            * 15.0
    }

    fn expansion_audio_type(&self) -> crate::mapper::ExpansionAudioType {
        crate::mapper::ExpansionAudioType::Vrc6
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg);
        state.extend_from_slice(&self.chr);
        state.push(self.mirr_ctrl);
        state.push(self.irq_enabled as u8);
        state.push(self.irq_reload as u8);
        state.push(self.irq_latch);
        state.extend_from_slice(&(self.irq_count as i32).to_le_bytes());
        state.extend_from_slice(&self.cycle_count.to_le_bytes());
        state.push(self.irq_mode as u8);
        state.push(self.variant as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 2 + 8 + 1 + 1 + 1 + 1 + 4 + 4 + 1 + 1 <= state.len() {
            for i in 0..2 {
                self.prg[i] = state[start];
                start += 1;
            }
            for i in 0..8 {
                self.chr[i] = state[start];
                start += 1;
            }
            self.mirr_ctrl = state[start];
            start += 1;
            self.irq_enabled = state[start] != 0;
            start += 1;
            self.irq_reload = state[start] != 0;
            start += 1;
            self.irq_latch = state[start];
            start += 1;
            self.irq_count = i32::from_le_bytes([state[start], state[start + 1], state[start + 2], state[start + 3]]) as u8;
            start += 4;
            self.cycle_count = i32::from_le_bytes([state[start], state[start + 1], state[start + 2], state[start + 3]]);
            start += 4;
            self.irq_mode = state[start] != 0;
            start += 1;
            self.variant = match state[start] {
                0 => Vrc6Variant::Mapper24,
                1 => Vrc6Variant::Mapper26,
                _ => Vrc6Variant::Mapper24,
            };
            start += 1;
        }
        start
    }

    fn reset(&mut self) {
        self.prg = [0; 2];
        self.chr = [0; 8];
        self.mirr_ctrl = 0;
        self.irq_latch = 0;
        self.irq_enabled = false;
        self.irq_reload = false;
        self.irq_mode = false;
        self.irq_count = 0;
        self.cycle_count = 0;
        self.irq_ack = false;
        self.halt_audio = false;
    }
}
