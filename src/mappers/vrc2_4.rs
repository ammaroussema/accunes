use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

#[derive(Clone, Copy)]
pub enum VrcVariant {
    Mapper21,
    Mapper22,
    Mapper23,
    Mapper25,
}

pub struct Vrc2And4 {
    variant: VrcVariant,
    prg_reg: [u8; 2],
    chr_reg: [u8; 8],
    chr_hi: [u16; 8],
    reg_cmd: u8,
    irq_cmd: u8,
    mirr: u8,
    big_bank: u8,
    irq_count: u16,
    irq_latch: u8,
    irq_enabled: bool,
    irq_mode: bool,
    acount: u16,
    reg1mask: u8,
    reg2mask: u8,
    has_irq: bool,
    has_wram: bool,
    is22: bool,
}

impl Vrc2And4 {
    pub fn new(variant: VrcVariant) -> Self {
        let (reg1mask, reg2mask, has_irq, has_wram, is22) = match variant {
            VrcVariant::Mapper21 => (0x42, 0x84, true, true, false),
            VrcVariant::Mapper22 => (0x02, 0x01, false, false, true),
            VrcVariant::Mapper23 => (0x15, 0x2a, true, true, false),
            VrcVariant::Mapper25 => (0x0a, 0x05, true, true, false),
        };
        Vrc2And4 {
            variant,
            prg_reg: [0; 2],
            chr_reg: [0; 8],
            chr_hi: [0; 8],
            reg_cmd: 0,
            irq_cmd: 0,
            mirr: 0,
            big_bank: 0x20,
            irq_count: 0,
            irq_latch: 0,
            irq_enabled: false,
            irq_mode: false,
            acount: 0,
            reg1mask,
            reg2mask,
            has_irq,
            has_wram,
            is22,
        }
    }

    fn decode_address(&self, address: u16) -> u16 {
        let base = address & 0xF000;
        let bit1 = if address & self.reg2mask as u16 != 0 { 1 << 1 } else { 0 };
        let bit0 = if address & self.reg1mask as u16 != 0 { 1 } else { 0 };
        base | bit1 | bit0
    }
}

impl Mapper for Vrc2And4 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = if self.reg_cmd & 2 != 0 {
                match address {
                    0x8000..=0x9FFF => (!1 & 0x1F) | self.big_bank as u16,
                    0xA000..=0xBFFF => self.prg_reg[1] as u16 | self.big_bank as u16,
                    0xC000..=0xDFFF => self.prg_reg[0] as u16 | self.big_bank as u16,
                    0xE000..=0xFFFF => (!0 & 0x1F) | self.big_bank as u16,
                    _ => 0,
                }
            } else {
                match address {
                    0x8000..=0x9FFF => self.prg_reg[0] as u16 | self.big_bank as u16,
                    0xA000..=0xBFFF => self.prg_reg[1] as u16 | self.big_bank as u16,
                    0xC000..=0xDFFF => (!1 & 0x1F) | self.big_bank as u16,
                    0xE000..=0xFFFF => (!0 & 0x1F) | self.big_bank as u16,
                    _ => 0,
                }
            };
            let offset = (bank as usize * 0x2000) + (address as usize & 0x1FFF);
            let final_offset = offset % cart.prg_rom.len();
            FetchResult { data: cart.prg_rom[final_offset], driven: true }
        } else if address >= 0x6000 && address < 0x8000 && self.has_wram {
            let idx = (address as usize - 0x6000) & (cart.prg_ram.len() - 1);
            FetchResult { data: cart.prg_ram[idx], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let decoded = self.decode_address(address);
            if decoded >= 0xB000 && decoded <= 0xE003 {
                if cart.using_chr_ram {
                    self.big_bank = (data & 8) << 2;
                } else {
                    let i = (((decoded >> 1) & 1) | ((decoded - 0xB000) >> 11)) as usize;
                    let nibble = (decoded & 1) << 2;
                    self.chr_reg[i] = (self.chr_reg[i] & (0xF0 >> nibble)) | ((data & 0xF) << nibble);
                    if nibble != 0 {
                        self.chr_hi[i] = ((data & 0x10) << 4) as u16;
                    }
                }
            } else {
                match decoded & 0xF003 {
                    0x8000 | 0x8001 | 0x8002 | 0x8003 => {
                        self.prg_reg[0] = data & 0x1F;
                    }
                    0xA000 | 0xA001 | 0xA002 | 0xA003 => {
                        self.prg_reg[1] = data & 0x1F;
                    }
                    0x9000 | 0x9001 => {
                        if data != 0xFF {
                            self.mirr = data;
                        }
                    }
                    0x9002 | 0x9003 => {
                        self.reg_cmd = data;
                    }
                    0xF000 => {
                        if self.has_irq {
                            self.irq_latch &= 0xF0;
                            self.irq_latch |= data & 0xF;
                        }
                    }
                    0xF001 => {
                        if self.has_irq {
                            self.irq_latch &= 0x0F;
                            self.irq_latch |= data << 4;
                        }
                    }
                    0xF002 => {
                        if self.has_irq {
                            self.acount = 0;
                            self.irq_count = self.irq_latch as u16;
                            self.irq_mode = (data & 4) != 0;
                            self.irq_enabled = (data & 2) != 0;
                            self.irq_cmd = if data & 1 != 0 { 1 } else { 0 };
                        }
                    }
                    0xF003 => {
                        if self.has_irq {
                            self.irq_enabled = self.irq_cmd != 0;
                        }
                    }
                    _ => {}
                }
            }
        } else if address >= 0x6000 && address < 0x8000 && self.has_wram {
            let idx = (address as usize - 0x6000) & (cart.prg_ram.len() - 1);
            cart.prg_ram[idx] = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        match self.mirr & 0x3 {
            0 => {
                address & 0x37FF
            }
            1 => {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            }
            2 => {
                address & 0x3FFF
            }
            3 => {
                (address & 0x3FFF) | 0x0400
            }
            _ => address,
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
            let bank = (address >> 10) as usize & 0x07;
            let shift = if self.is22 { 1 } else { 0 };
            let chr_bank = ((self.chr_hi[bank] | self.chr_reg[bank] as u16) >> shift) as usize;
            let offset = (chr_bank * 0x400) + (address as usize & 0x3FF);
            if using_chr_ram {
                new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
            } else if chr_rom.is_empty() {
                new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
            } else {
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else {
            let mirrored = match self.mirr & 0x3 {
                0 => {
                    address & 0x37FF
                }
                1 => {
                    (address & 0x33FF) | ((address & 0x0800) >> 1)
                }
                2 => {
                    address & 0x3FFF
                }
                3 => {
                    (address & 0x3FFF) | 0x0400
                }
                _ => address,
            };
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
        if !self.has_irq {
            return false;
        }
        const LCYCS: u16 = 341;
        if self.irq_enabled {
            if self.irq_mode {
                return false;
            } else {
                self.acount += 3;
                if self.acount >= LCYCS {
                    while self.acount >= LCYCS {
                        self.acount -= LCYCS;
                        self.irq_count += 1;
                        if self.irq_count & 0x100 != 0 {
                            self.irq_count = self.irq_latch as u16;
                            return true; 
                        }
                    }
                }
                return false;
            }
        }
        false
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if !self.has_irq {
            return false;
        }
        if self.irq_enabled && self.irq_mode {
            self.acount += _cycles as u16;
            while self.acount > 0 {
                self.acount -= 1;
                self.irq_count += 1;
                if self.irq_count & 0x100 != 0 {
                    self.irq_count = self.irq_latch as u16;
                    return true; 
                }
            }
        }
        false
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
        state.push(self.big_bank);
        state.push(self.variant as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 2 + 8 + 16 + 1 + 1 + 1 + 1 <= state.len() {
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
            self.big_bank = state[start];
            start += 1;
            self.variant = match state[start] {
                0 => VrcVariant::Mapper21,
                1 => VrcVariant::Mapper22,
                2 => VrcVariant::Mapper23,
                3 => VrcVariant::Mapper25,
                _ => VrcVariant::Mapper21,
            };
            start += 1;
        }
        start
    }

    fn reset(&mut self) {
        self.prg_reg = [0; 2];
        self.chr_reg = [0; 8];
        self.chr_hi = [0; 8];
        self.reg_cmd = 0;
        self.irq_cmd = 0;
        self.mirr = 0;
        self.big_bank = 0x20;
        if self.has_irq {
            self.irq_count = 0;
            self.irq_latch = 0;
            self.irq_enabled = false;
            self.irq_mode = false;
            self.acount = 0;
        }
    }
}
