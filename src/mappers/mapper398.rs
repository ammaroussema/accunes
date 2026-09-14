use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper398 {
    prg: [u8; 2],
    chr: [u16; 8],
    mirroring: u8,
    prg_flip: bool,
    wram_enable: bool,
    irq: u8,
    irq_counter: u8,
    irq_latch: u8,
    irq_cycles: i16,
    irq_ack: bool,
    latch: u8,
    current_chr_bank: u8,
}

impl Mapper398 {
    pub fn new() -> Self {
        Self {
            prg: [0, 1],
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            mirroring: 0,
            prg_flip: false,
            wram_enable: true,
            irq: 0,
            irq_counter: 0,
            irq_latch: 0,
            irq_cycles: 341,
            irq_ack: false,
            latch: 0xC0,
            current_chr_bank: 0,
        }
    }
}

impl Mapper for Mapper398 {
    fn reset(&mut self) {
        self.prg = [0, 1];
        self.chr = [0, 1, 2, 3, 4, 5, 6, 7];
        self.mirroring = 0;
        self.prg_flip = false;
        self.wram_enable = true;
        self.irq = 0;
        self.irq_counter = 0;
        self.irq_latch = 0;
        self.irq_cycles = 341;
        self.irq_ack = false;
        self.latch = 0xC0;
        self.current_chr_bank = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            if self.wram_enable {
                let len = cart.prg_ram.len().max(1);
                let offset = (address as usize - 0x6000) % len;
                return FetchResult {
                    data: cart.prg_ram.get(offset).copied().unwrap_or(0),
                    driven: true,
                };
            }
            return FetchResult { data: 0, driven: false };
        }

        if address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }

        let prg_len = cart.prg_rom.len().max(1);

        if self.latch & 0x80 != 0 {
            let bank = ((self.latch >> 5) & 6)
                | ((self.chr[self.current_chr_bank as usize] >> 2) as u8 & 1);
            let offset = (bank as usize) * 0x8000 + (address as usize & 0x7FFF);
            FetchResult {
                data: cart.prg_rom[offset % prg_len],
                driven: true,
            }
        } else {
            let mask = 0x0F;
            let bank = match address & 0xE000 {
                0x8000 => {
                    if self.prg_flip {
                        0xFE & mask
                    } else {
                        self.prg[0] & mask
                    }
                }
                0xA000 => self.prg[1] & mask,
                0xC000 => {
                    if self.prg_flip {
                        self.prg[0] & mask
                    } else {
                        0xFE & mask
                    }
                }
                _ => 0xFF & mask,
            };
            let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % prg_len],
                driven: true,
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if self.wram_enable {
                let len = cart.prg_ram.len().max(1);
                let offset = (address as usize - 0x6000) % len;
                if let Some(byte) = cart.prg_ram.get_mut(offset) {
                    *byte = data;
                }
            }
            return;
        }

        if address < 0x8000 {
            return;
        }

        self.latch = (address & 0xFF) as u8;

        let bank = address >> 12;
        let a0 = (address & 0x01) != 0;
        let a1 = (address & 0x02) != 0;

        match bank {
            0x8 | 0xA => {
                let idx = (bank >> 1) as usize & 1;
                self.prg[idx] = data;
            }
            0x9 => {
                let reg = ((a1 as u8) << 1) | (a0 as u8);
                match reg {
                    0 | 1 => self.mirroring = data & 3,
                    2 => {
                        self.wram_enable = (data & 1) != 0;
                        self.prg_flip = (data & 2) != 0;
                    }
                    _ => {}
                }
            }
            0xB..=0xE => {
                let idx = ((bank - 0xB) << 1) as usize | (a1 as usize);
                if idx < 8 {
                    if a0 {
                        self.chr[idx] = (self.chr[idx] & 0x000F) | ((data as u16) << 4);
                    } else {
                        self.chr[idx] = (self.chr[idx] & 0x0FF0) | (data as u16 & 0x000F);
                    }
                }
            }
            0xF => {
                let reg = ((a1 as u8) << 1) | (a0 as u8);
                match reg {
                    0 => self.irq_latch = (self.irq_latch & 0xF0) | (data & 0x0F),
                    1 => self.irq_latch = (self.irq_latch & 0x0F) | (data << 4),
                    2 => {
                        self.irq = data;
                        if self.irq & 0x02 != 0 {
                            self.irq_counter = self.irq_latch;
                            self.irq_cycles = 341;
                        }
                        self.irq_ack = true;
                    }
                    3 => {
                        self.irq = self.irq & !0x02 | ((self.irq << 1) & 0x02);
                        self.irq_ack = true;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        match self.mirroring & 3 {
            0 => address & 0x37FF,
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            2 => 0x2000 | (address & 0x3FF),
            3 => 0x2400 | (address & 0x3FF),
            _ => address & 0x37FF,
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
        let mut new_addr_bus = ppu_address_bus & 0xFF00;

        if address < 0x2000 {
            let bank = ((address >> 10) & 7) as u8;
            self.current_chr_bank = bank;

            let chr_len = chr_rom.len().max(1);
            let data = if self.latch & 0x80 != 0 {
                let chr_bank = 0x40
                    | ((self.latch >> 3) & 8)
                    | (self.chr[self.current_chr_bank as usize] as u8 & 7);
                let offset = (chr_bank as usize) * 0x2000 + (address as usize & 0x1FFF);
                if using_chr_ram || chr_rom.is_empty() {
                    chr_ram[offset % chr_ram.len().max(1)]
                } else {
                    chr_rom[offset % chr_len]
                }
            } else {
                let chr_bank = (self.chr[bank as usize] as usize) & 0x1FF;
                let offset = chr_bank * 0x400 + (address as usize & 0x3FF);
                if using_chr_ram || chr_rom.is_empty() {
                    chr_ram[offset % chr_ram.len().max(1)]
                } else {
                    chr_rom[offset % chr_len]
                }
            };
            new_addr_bus |= data as u16;
        } else {
            let mirrored = match self.mirroring & 3 {
                0 => address & 0x37FF,
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                2 => address & 0x3FFF,
                3 => (address & 0x3FFF) | 0x0400,
                _ => address,
            };
            let idx = (mirrored & 0x7FF) as usize;
            new_addr_bus |= vram[idx] as u16;
        }

        (new_addr_bus as u8, new_addr_bus)
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.irq & 0x02 == 0 {
            return false;
        }

        let active = self.irq & 0x04 != 0 || {
            self.irq_cycles -= 3;
            self.irq_cycles <= 0
        };

        if active {
            if self.irq & 0x04 == 0 {
                self.irq_cycles += 341;
            }
            self.irq_counter = self.irq_counter.wrapping_add(1);
            if self.irq_counter == 0 {
                self.irq_counter = self.irq_latch;
                return true;
            }
        }

        false
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack;
        self.irq_ack = false;
        ack
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.prg[0]);
        state.push(self.prg[1]);
        for c in &self.chr {
            state.extend_from_slice(&c.to_le_bytes());
        }
        state.push(self.mirroring);
        state.push(if self.prg_flip { 1 } else { 0 });
        state.push(if self.wram_enable { 1 } else { 0 });
        state.push(self.irq);
        state.push(self.irq_counter);
        state.push(self.irq_latch);
        state.extend_from_slice(&self.irq_cycles.to_le_bytes());
        state.push(self.latch);
        state.push(self.current_chr_bank);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.prg[0] = state.get(p).copied().unwrap_or(0);
        p += 1;
        self.prg[1] = state.get(p).copied().unwrap_or(0);
        p += 1;
        for c in self.chr.iter_mut() {
            if p + 1 < state.len() {
                *c = u16::from_le_bytes([state[p], state[p + 1]]);
                p += 2;
            }
        }
        self.mirroring = state.get(p).copied().unwrap_or(0);
        p += 1;
        self.prg_flip = state.get(p).copied().unwrap_or(0) != 0;
        p += 1;
        self.wram_enable = state.get(p).copied().unwrap_or(0) != 0;
        p += 1;
        self.irq = state.get(p).copied().unwrap_or(0);
        p += 1;
        self.irq_counter = state.get(p).copied().unwrap_or(0);
        p += 1;
        self.irq_latch = state.get(p).copied().unwrap_or(0);
        p += 1;
        if p + 1 < state.len() {
            self.irq_cycles = i16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        self.latch = state.get(p).copied().unwrap_or(0xC0);
        p += 1;
        self.current_chr_bank = state.get(p).copied().unwrap_or(0);
        p += 1;
        p
    }
}
