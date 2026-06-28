use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

#[derive(Clone, Copy)]
pub enum Vrc6Variant {
    Mapper24,
    Mapper26,
}

pub struct Vrc6 {
    variant: Vrc6Variant,
    prg: [u8; 2],
    chr: [u8; 8],
    mirr: u8,
    irq_latch: u8,
    irq_enabled: bool,
    irq_reload: bool,
    irq_mode: bool,
    irq_count: i32,
    cycle_count: i32,
    has_wram: bool,
}

impl Vrc6 {
    pub fn new(variant: Vrc6Variant) -> Self {
        let has_wram = match variant {
            Vrc6Variant::Mapper24 => false,
            Vrc6Variant::Mapper26 => true,
        };
        Vrc6 {
            variant,
            prg: [0; 2],
            chr: [0; 8],
            mirr: 0,
            irq_latch: 0,
            irq_enabled: false,
            irq_reload: false,
            irq_mode: false,
            irq_count: 0,
            cycle_count: 0,
            has_wram,
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
            if decoded >= 0x9000 && decoded <= 0xB002 {
                return;
            }
            match decoded & 0xF003 {
                0x8000 => {
                    self.prg[0] = data;
                }
                0xB003 => {
                    self.mirr = (data >> 2) & 3;
                }
                0xC000 => {
                    self.prg[1] = data;
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
                    if data & 2 != 0 {
                        self.irq_enabled = true;
                        self.irq_count = self.irq_latch as i32;
                    } else {
                        self.irq_enabled = false;
                    }
                    self.cycle_count = 0;
                }
                0xF002 => {
                    self.irq_enabled = self.irq_reload;
                }
                _ => {}
            }
        } else if address >= 0x6000 && address < 0x8000 && self.has_wram {
            let idx = (address as usize - 0x6000) & (cart.prg_ram.len() - 1);
            cart.prg_ram[idx] = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        match self.mirr & 3 {
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
            let chr_bank = self.chr[bank] as usize;
            let offset = (chr_bank * 0x400) + (address as usize & 0x3FF);
            if using_chr_ram {
                new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
            } else if chr_rom.is_empty() {
                new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
            } else {
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else {
            let mirrored = match self.mirr & 3 {
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
        const LCYCS: i32 = 341;
        if self.irq_enabled && !self.irq_mode {
            self.cycle_count += 1;
            while self.cycle_count >= LCYCS {
                self.cycle_count -= LCYCS;
                self.irq_count += 1;
                if self.irq_count == 0x100 {
                    self.irq_count = self.irq_latch as i32;
                    return true; 
                }
            }
        }
        false
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.irq_enabled && self.irq_mode {
            self.cycle_count += _cycles as i32;
            while self.cycle_count > 0 {
                self.cycle_count -= 1;
                self.irq_count += 1;
                if self.irq_count & 0x100 != 0 {
                    self.irq_count = self.irq_latch as i32;
                    return true; 
                }
            }
        }
        false
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.irq_enabled {
            self.irq_enabled = self.irq_reload;
            return true;
        }
        false
    }

    fn audio_sample(&self) -> f32 {
        0.0
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg);
        state.extend_from_slice(&self.chr);
        state.push(self.mirr);
        state.push(self.irq_enabled as u8);
        state.push(self.irq_reload as u8);
        state.push(self.irq_latch);
        state.extend_from_slice(&self.irq_count.to_le_bytes());
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
            self.mirr = state[start];
            start += 1;
            self.irq_enabled = state[start] != 0;
            start += 1;
            self.irq_reload = state[start] != 0;
            start += 1;
            self.irq_latch = state[start];
            start += 1;
            self.irq_count = i32::from_le_bytes([state[start], state[start + 1], state[start + 2], state[start + 3]]);
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
        self.mirr = 0;
        self.irq_latch = 0;
        self.irq_enabled = false;
        self.irq_reload = false;
        self.irq_mode = false;
        self.irq_count = 0;
        self.cycle_count = 0;
    }
}
