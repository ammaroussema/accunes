use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

#[derive(Clone, Copy)]
pub enum VrcVariant {
    Mapper21,
    Mapper22,
    Mapper23,
    Mapper25,
    Mapper617,
}

pub struct Vrc2And4 {
    variant: VrcVariant,
    prg_reg: [u8; 2],
    chr_reg: [u8; 8],
    chr_hi: [u16; 8],
    reg_cmd: u8,
    irq_cmd: u8,
    mirr: u8,
    irq_count: u16,
    irq_latch: u8,
    irq_enabled: bool,
    irq_mode: bool,
    acount: u16,
    a0mask: u8,
    a1mask: u8,
    has_irq: bool,
    is_vrc2: bool,
    chr_shift: u8,
    use_repeat_bit: bool,
}

impl Vrc2And4 {
    pub fn new(variant: VrcVariant, submapper: u8) -> Self {
        let (a0mask, a1mask, has_irq, is_vrc2, chr_shift) = match variant {
            VrcVariant::Mapper21 => match submapper {
                1 => (0x02, 0x04, true, false, 0),
                2 => (0x40, 0x80, true, false, 0),
                _ => (0x42, 0x84, true, false, 0),
            },
            VrcVariant::Mapper22 => (0x02, 0x01, false, true, 1),
            VrcVariant::Mapper23 => match submapper {
                1 => (0x01, 0x02, true, false, 0),
                2 => (0x04, 0x08, true, false, 0),
                3 => (0x01, 0x02, false, true, 0),
                _ => (0x05, 0x0A, true, false, 0),
            },
            VrcVariant::Mapper25 => match submapper {
                1 => (0x02, 0x01, true, false, 0),
                2 => (0x08, 0x04, true, false, 0),
                3 => (0x02, 0x01, false, true, 0),
                _ => (0x0A, 0x05, true, false, 0),
            },
            VrcVariant::Mapper617 => (0x05, 0x0a, true, false, 0),
        };
        let use_repeat_bit = !is_vrc2;
        Vrc2And4 {
            variant,
            prg_reg: [0, 1],
            chr_reg: [0, 1, 2, 3, 4, 5, 6, 7],
            chr_hi: [0; 8],
            reg_cmd: 0,
            irq_cmd: 0,
            mirr: 0,
            irq_count: 0,
            irq_latch: 0,
            irq_enabled: false,
            irq_mode: false,
            acount: 0,
            a0mask,
            a1mask,
            has_irq,
            is_vrc2,
            chr_shift,
            use_repeat_bit,
        }
    }

    pub fn nametable_mirroring_value(&self) -> u8 {
        self.mirr
    }

    pub fn chr_bank_raw(&self, index: usize) -> u16 {
        if index < 8 { self.chr_reg[index] as u16 | self.chr_hi[index] } else { 0 }
    }

    pub fn prg_slot_bank(&self, slot: usize) -> u16 {
        let value: u8 = if self.reg_cmd & 2 != 0 {
            match slot {
                0 => 0xFE,
                1 => self.prg_reg[1],
                2 => self.prg_reg[0],
                _ => 0xFF,
            }
        } else {
            match slot {
                0 => self.prg_reg[0],
                1 => self.prg_reg[1],
                2 => 0xFE,
                _ => 0xFF,
            }
        };
        (value & 0x3F) as u16
    }

    fn mirror_mask(&self) -> u8 {
        if self.is_vrc2 { self.mirr & 1 } else { self.mirr & 3 }
    }

    fn mirror_address(&self, address: u16) -> u16 {
        match self.mirror_mask() {
            0 => address & 0x37FF,
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            2 => address & 0x33FF,
            3 => (address & 0x33FF) | 0x0400,
            _ => address,
        }
    }

    fn tick_irq_counter(&mut self) -> bool {
        self.irq_count += 1;
        if self.irq_count & 0x100 != 0 {
            self.irq_count = self.irq_latch as u16;
            true
        } else {
            false
        }
    }
}

impl Mapper for Vrc2And4 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let slot = (address as usize - 0x8000) / 0x2000;
            let bank = self.prg_slot_bank(slot);
            let offset = (bank as usize * 0x2000) + (address as usize & 0x1FFF);
            let final_offset = offset % cart.prg_rom.len();
            FetchResult { data: cart.prg_rom[final_offset], driven: true }
        } else if (0x6000..0x8000).contains(&address) {
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
            let bank = (address >> 12) as u8 & 0x0F;
            let index = {
                let bit0 = if address & self.a0mask as u16 != 0 { 1 } else { 0 };
                let bit1 = if address & self.a1mask as u16 != 0 { 2 } else { 0 };
                bit0 | bit1
            };
            match bank {
                0x8 | 0xA => {
                    self.prg_reg[(bank >> 1 & 1) as usize] = data;
                }
                0x9 => {
                    if self.is_vrc2 || index == 0 {
                        self.mirr = data;
                    } else if index == 2 {
                        self.reg_cmd = data;
                    }
                }
                0xF => {
                    if self.has_irq {
                        match index & 3 {
                            0 => {
                                self.irq_latch &= 0xF0;
                                self.irq_latch |= data & 0xF;
                            }
                            1 => {
                                self.irq_latch &= 0x0F;
                                self.irq_latch |= data << 4;
                            }
                            2 => {
                                self.irq_mode = (data & 4) != 0;
                                self.irq_enabled = (data & 2) != 0;
                                self.irq_cmd = if data & 1 != 0 { 1 } else { 0 };
                                if self.irq_enabled {
                                    self.irq_count = self.irq_latch as u16;
                                    self.acount = 341;
                                }
                            }
                            3 => {
                                if self.use_repeat_bit {
                                    self.irq_enabled = self.irq_cmd != 0;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {
                    if (0xB..=0xE).contains(&bank) {
                        let reg = (((bank as u16 - 0xB) << 1) | ((index >> 1) & 1)) as usize;
                        if reg < 8 {
                            let nibble_shift = (index & 1) << 2;
                            self.chr_reg[reg] =
                                (self.chr_reg[reg] & (0xF0 >> nibble_shift)) | ((data & 0xF) << nibble_shift);
                            if nibble_shift != 0 {
                                self.chr_hi[reg] = ((data & 0x10) << 4) as u16;
                            }
                        }
                    }
                }
            }
        } else if (0x6000..0x8000).contains(&address) {
            if !cart.prg_ram.is_empty() {
                let idx = (address as usize - 0x6000) & (cart.prg_ram.len() - 1);
                cart.prg_ram[idx] = data;
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
            let bank = (address >> 10) as usize & 0x07;
            let chr_bank = ((self.chr_hi[bank] | self.chr_reg[bank] as u16) >> self.chr_shift) as usize;
            let offset = (chr_bank * 0x400) + (address as usize & 0x3FF);
            if using_chr_ram {
                new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
            } else if chr_rom.is_empty() {
                new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
            } else {
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else {
            let mirrored = self.mirror_address(address);
            let idx = (mirrored & 0x7FF) as usize;
            new_addr_bus |= vram[idx] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
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
        if !self.has_irq || !self.irq_enabled {
            return false;
        }
        let mut triggered = false;
        for _ in 0.._cycles {
            if self.irq_mode {
                if self.tick_irq_counter() {
                    triggered = true;
                }
            } else {
                if self.acount <= 3 {
                    self.acount = self.acount + 341 - 3;
                    if self.tick_irq_counter() {
                        triggered = true;
                    }
                } else {
                    self.acount -= 3;
                }
            }
        }
        triggered
    }

    fn take_irq_ack(&mut self) -> bool {
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg_reg);
        state.extend_from_slice(&self.chr_reg);
        for &hi in &self.chr_hi {
            state.extend_from_slice(&hi.to_le_bytes());
        }
        if self.has_irq {
            state.extend_from_slice(&self.acount.to_le_bytes());
            state.push(self.irq_cmd);
            state.extend_from_slice(&self.irq_count.to_le_bytes());
            state.push(self.irq_latch);
            state.push(self.irq_enabled as u8);
            state.push(self.irq_mode as u8);
        }
        state.push(self.reg_cmd);
        state.push(self.mirr);
        state.push(self.variant as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 2 + 8 + 16 + 1 + 1 + 1 <= state.len() {
            for i in 0..2 {
                self.prg_reg[i] = state[start];
                start += 1;
            }
            for i in 0..8 {
                self.chr_reg[i] = state[start];
                start += 1;
            }
            for i in 0..8 {
                self.chr_hi[i] = u16::from_le_bytes([state[start], state[start + 1]]);
                start += 2;
            }
            if self.has_irq {
                self.acount = u16::from_le_bytes([state[start], state[start + 1]]);
                start += 2;
                self.irq_cmd = state[start];
                start += 1;
                self.irq_count = u16::from_le_bytes([state[start], state[start + 1]]);
                start += 2;
                self.irq_latch = state[start];
                start += 1;
                self.irq_enabled = state[start] != 0;
                start += 1;
                self.irq_mode = state[start] != 0;
                start += 1;
            }
            self.reg_cmd = state[start];
            start += 1;
            self.mirr = state[start];
            start += 1;
            self.variant = match state[start] {
                0 => VrcVariant::Mapper21,
                1 => VrcVariant::Mapper22,
                2 => VrcVariant::Mapper23,
                3 => VrcVariant::Mapper25,
                4 => VrcVariant::Mapper617,
                _ => VrcVariant::Mapper21,
            };
            start += 1;
        }
        start
    }

    fn reset(&mut self) {
        self.prg_reg = [0, 1];
        for i in 0..8 {
            self.chr_reg[i] = i as u8;
        }
        self.chr_hi = [0; 8];
        self.reg_cmd = 0;
        self.irq_cmd = 0;
        self.mirr = 0;
        if self.has_irq {
            self.irq_count = 0;
            self.irq_latch = 0;
            self.irq_enabled = false;
            self.irq_mode = false;
            self.acount = 0;
        }
    }
}