use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper520 {
    prg: [u8; 2],
    chr: [u16; 8],
    mirroring: u8,
    prg_flip: bool,
    wram_enable: bool,
    current_chr_bank: usize,

    irq_latch: u8,
    irq_counter: u8,
    irq_prescaler: i16,
    irq_enabled: bool,
    irq_mode: bool,
    irq_enable_on_ack: bool,
}

impl Mapper520 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self {
            prg: [0, 1],
            chr: [0; 8],
            mirroring: 0,
            prg_flip: false,
            wram_enable: true,
            current_chr_bank: 0,

            irq_latch: 0,
            irq_counter: 0,
            irq_prescaler: 0,
            irq_enabled: false,
            irq_mode: false,
            irq_enable_on_ack: false,
        }
    }
}

impl Mapper for Mapper520 {
    fn reset(&mut self) {
        self.prg = [0, 1];
        self.chr = [0; 8];
        self.mirroring = 0;
        self.prg_flip = false;
        self.wram_enable = true;
        self.current_chr_bank = 0;

        self.irq_latch = 0;
        self.irq_counter = 0;
        self.irq_prescaler = 0;
        self.irq_enabled = false;
        self.irq_mode = false;
        self.irq_enable_on_ack = false;
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x6000..0x8000).contains(&address) {
            if self.wram_enable && !cart.prg_ram.is_empty() {
                let offset = (address - 0x6000) as usize;
                FetchResult {
                    data: cart.prg_ram[offset % cart.prg_ram.len()],
                    driven: true,
                }
            } else {
                FetchResult {
                    data: 0,
                    driven: false,
                }
            }
        } else if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }

            let prg_or = ((self.chr[self.current_chr_bank & 7] as usize) & 0x08) << 2;
            let page = (address as usize - 0x8000) / 0x2000;
            let bank = match (page, self.prg_flip) {
                (0, false) => (self.prg[0] as usize & 0x1F) | prg_or,
                (0, true) => 0x1E | prg_or,
                (1, _) => (self.prg[1] as usize & 0x1F) | prg_or,
                (2, false) => 0x1E | prg_or,
                (2, true) => (self.prg[0] as usize & 0x1F) | prg_or,
                (3, _) => 0x1F | prg_or,
                _ => 0,
            };

            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) {
            if self.wram_enable && !cart.prg_ram.is_empty() {
                let offset = (address - 0x6000) as usize;
                let len = cart.prg_ram.len();
                cart.prg_ram[offset % len] = data;
            }
        } else if address >= 0x8000 {
            let bit0 = if (address & 0x04) != 0 { 1 } else { 0 };
            let bit1 = if (address & 0x08) != 0 { 2 } else { 0 };
            let decoded_reg = bit1 | bit0;

            match address & 0xF000 {
                0x8000 => {
                    self.prg[0] = data & 0x1F;
                }
                0x9000 => match decoded_reg {
                    0 | 1 => {
                        self.mirroring = data & 3;
                    }
                    2 => {
                        self.wram_enable = (data & 1) != 0;
                        self.prg_flip = (data & 2) != 0;
                    }
                    _ => {}
                },
                0xA000 => {
                    self.prg[1] = data & 0x1F;
                }
                0xB000..=0xE000 => {
                    let bank_idx = (((address >> 12) & 0xF) - 0xB) as usize;
                    let slot = (bank_idx << 1) | if (decoded_reg & 2) != 0 { 1 } else { 0 };
                    if (decoded_reg & 1) != 0 {
                        self.chr[slot] = (self.chr[slot] & 0x0F) | (((data as u16) & 0x1F) << 4);
                    } else {
                        self.chr[slot] = (self.chr[slot] & 0x1F0) | ((data as u16) & 0x0F);
                    }
                }
                0xF000 => match decoded_reg {
                    0 => {
                        self.irq_latch = (self.irq_latch & 0xF0) | (data & 0x0F);
                    }
                    1 => {
                        self.irq_latch = (self.irq_latch & 0x0F) | ((data & 0x0F) << 4);
                    }
                    2 => {
                        self.irq_mode = (data & 4) != 0;
                        self.irq_enabled = (data & 2) != 0;
                        self.irq_enable_on_ack = (data & 1) != 0;
                        if self.irq_enabled {
                            self.irq_counter = self.irq_latch;
                            self.irq_prescaler = 341;
                        }
                    }
                    3 => {
                        self.irq_enabled = self.irq_enable_on_ack;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else {
            match self.mirroring & 3 {
                0 => address & 0x37FF,                              
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1), 
                2 => address & 0x33FF,                                
                3 => (address & 0x33FF) | 0x0400,                   
                _ => address,
            }
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        _using_chr_ram: bool,
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
            let bank = (address >> 10) as usize & 7;
            self.current_chr_bank = bank;
            let page = (self.chr[bank] & 0x07) as usize;
            let offset = page * 0x0400 + (address as usize & 0x03FF);
            let byte = if !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else {
                match self.mirroring & 3 {
                    0 => address & 0x37FF,
                    1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                    2 => address & 0x33FF,
                    3 => (address & 0x33FF) | 0x0400,
                    _ => address,
                }
            };

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
            let bank = (address >> 10) as usize & 7;
            let page = (self.chr[bank] & 0x07) as usize;
            let offset = page * 0x0400 + (address as usize & 0x03FF);
            let len = cart.chr_ram.len();
            if len > 0 {
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
        _ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        _scanline: u16,
        _dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        if self.irq_enabled && !self.irq_mode {
            self.irq_prescaler += 3;
            if self.irq_prescaler >= 341 {
                while self.irq_prescaler >= 341 {
                    self.irq_prescaler -= 341;
                    if self.irq_counter == 0xFF {
                        self.irq_counter = self.irq_latch;
                        return true;
                    } else {
                        self.irq_counter += 1;
                    }
                }
            }
        }
        false
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.irq_enabled && self.irq_mode {
            for _ in 0..cycles {
                if self.irq_counter == 0xFF {
                    self.irq_counter = self.irq_latch;
                    return true;
                } else {
                    self.irq_counter += 1;
                }
            }
        }
        false
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg);
        for &c in &self.chr {
            state.extend_from_slice(&c.to_le_bytes());
        }
        state.push(self.mirroring);
        state.push(self.prg_flip as u8);
        state.push(self.wram_enable as u8);
        state.push(self.current_chr_bank as u8);
        state.push(self.irq_latch);
        state.push(self.irq_counter);
        state.extend_from_slice(&self.irq_prescaler.to_le_bytes());
        state.push(self.irq_enabled as u8);
        state.push(self.irq_mode as u8);
        state.push(self.irq_enable_on_ack as u8);
        state.extend_from_slice(&cart.prg_ram);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.prg[0] = state[p];
            self.prg[1] = state[p + 1];
            p += 2;
        }
        if p + 16 <= state.len() {
            for i in 0..8 {
                self.chr[i] = u16::from_le_bytes([state[p], state[p + 1]]);
                p += 2;
            }
        }
        if p < state.len() {
            self.mirroring = state[p];
            p += 1;
        }
        if p < state.len() {
            self.prg_flip = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.wram_enable = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.current_chr_bank = state[p] as usize;
            p += 1;
        }
        if p < state.len() {
            self.irq_latch = state[p];
            p += 1;
        }
        if p < state.len() {
            self.irq_counter = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.irq_prescaler = i16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.irq_enabled = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.irq_mode = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.irq_enable_on_ack = state[p] != 0;
            p += 1;
        }
        if p < state.len() && !cart.prg_ram.is_empty() {
            let copy_len = cart.prg_ram.len().min(state.len() - p);
            cart.prg_ram[..copy_len].copy_from_slice(&state[p..p + copy_len]);
            p += copy_len;
        }
        p
    }
}
