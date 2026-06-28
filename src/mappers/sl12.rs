use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct MapperSL12 {
    mode: u8,
    vrc2_chr: [u8; 8],
    vrc2_prg: [u8; 2],
    vrc2_mirr: u8,
    mmc3_regs: [u8; 10],
    mmc3_ctrl: u8,
    mmc3_mirr: u8,
    irq_reload: bool,
    irq_count: u8,
    irq_latch: u8,
    irq_enabled: bool,
    mmc1_regs: [u8; 4],
    mmc1_buffer: u8,
    mmc1_shift: u8,
}

impl MapperSL12 {
    pub fn new() -> Self {
        Self {
            mode: 0,
            vrc2_chr: [0xFF, 0xFF, 0xFF, 0xFF, 4, 5, 6, 7],
            vrc2_prg: [0, 1],
            vrc2_mirr: 0,
            mmc3_regs: [0, 2, 4, 5, 6, 7, 0xFC, 0xFD, 0xFE, 0xFF],
            mmc3_ctrl: 0,
            mmc3_mirr: 0,
            irq_reload: false,
            irq_count: 0,
            irq_latch: 0,
            irq_enabled: false,
            mmc1_regs: [0x0C, 0, 0, 0],
            mmc1_buffer: 0,
            mmc1_shift: 0,
        }
    }

    fn get_prg_bank(&self, reg_val: u8, num_8k_banks: usize) -> usize {
        if reg_val >= 0x80 {
            (num_8k_banks as i32 + (reg_val as i8) as i32) as usize % num_8k_banks
        } else {
            reg_val as usize % num_8k_banks
        }
    }

    fn mirror_address(&self, address: u16) -> u16 {
        match self.mode & 3 {
            0 => {
                let horizontal = (self.vrc2_mirr & 1) != 0;
                if horizontal {
                    (address & 0x33FF) | ((address & 0x0800) >> 1)
                } else {
                    address & 0x37FF
                }
            }
            1 => {
                let horizontal = (self.mmc3_mirr & 1) != 0;
                if horizontal {
                    (address & 0x33FF) | ((address & 0x0800) >> 1)
                } else {
                    address & 0x37FF
                }
            }
            _ => match self.mmc1_regs[0] & 3 {
                0 => address & 0x33FF,
                1 => (address & 0x33FF) | 0x0400,
                2 => address & 0x37FF,
                3 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                _ => unreachable!(),
            },
        }
    }

    fn chr_bank(&self, address: u16) -> usize {
        let base = ((self.mode & 4) as usize) << 6;
        match self.mode & 3 {
            0 => {
                let slot = (address >> 10) as usize & 7;
                base | self.vrc2_chr[slot] as usize
            }
            1 => {
                let invert = (self.mmc3_ctrl & 0x80) != 0;
                if !invert {
                    if address < 0x0400 {
                        base | (self.mmc3_regs[0] as usize & !1)
                    } else if address < 0x0800 {
                        base | (self.mmc3_regs[0] as usize | 1)
                    } else if address < 0x0C00 {
                        base | (self.mmc3_regs[1] as usize & !1)
                    } else if address < 0x1000 {
                        base | (self.mmc3_regs[1] as usize | 1)
                    } else if address < 0x1400 {
                        base | self.mmc3_regs[2] as usize
                    } else if address < 0x1800 {
                        base | self.mmc3_regs[3] as usize
                    } else if address < 0x1C00 {
                        base | self.mmc3_regs[4] as usize
                    } else {
                        base | self.mmc3_regs[5] as usize
                    }
                } else if address < 0x0400 {
                    base | self.mmc3_regs[2] as usize
                } else if address < 0x0800 {
                    base | self.mmc3_regs[3] as usize
                } else if address < 0x0C00 {
                    base | self.mmc3_regs[4] as usize
                } else if address < 0x1000 {
                    base | self.mmc3_regs[5] as usize
                } else if address < 0x1400 {
                    base | (self.mmc3_regs[0] as usize & !1)
                } else if address < 0x1800 {
                    base | (self.mmc3_regs[0] as usize | 1)
                } else if address < 0x1C00 {
                    base | (self.mmc3_regs[1] as usize & !1)
                } else {
                    base | (self.mmc3_regs[1] as usize | 1)
                }
            }
            _ => {
                if (self.mmc1_regs[0] & 0x10) != 0 {
                    if address < 0x1000 {
                        (self.mmc1_regs[1] as usize) * 4 + ((address as usize & 0x0FFF) / 1024)
                    } else {
                        (self.mmc1_regs[2] as usize) * 4 + ((address as usize & 0x0FFF) / 1024)
                    }
                } else {
                    ((self.mmc1_regs[1] >> 1) as usize) * 8 + (address as usize / 1024)
                }
            }
        }
    }

    fn read_chr(
        &self,
        address: u16,
        chr_rom: &[u8],
        chr_ram: &[u8],
        bank: usize,
    ) -> u8 {
        let len = if !chr_ram.is_empty() {
            chr_ram.len()
        } else {
            chr_rom.len()
        };
        if len == 0 {
            return 0;
        }
        let offset = bank * 0x400 + (address as usize & 0x3FF);
        if !chr_ram.is_empty() {
            chr_ram[offset % len]
        } else {
            chr_rom[offset % len]
        }
    }

    fn chr_write_offset(&self, address: u16, len: usize) -> usize {
        let bank = self.chr_bank(address);
        (bank * 0x400 + (address as usize & 0x3FF)) % len
    }

    fn hbirq_tick(&mut self) -> bool {
        if (self.mode & 3) != 1 {
            return false;
        }
        if self.irq_reload || self.irq_count == 0 {
            self.irq_count = self.irq_latch;
            self.irq_reload = false;
        } else {
            self.irq_count = self.irq_count.saturating_sub(1);
        }
        self.irq_count == 0 && self.irq_enabled
    }
}

impl Mapper for MapperSL12 {
    fn reset(&mut self) {
        *self = Self::new();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let num_8k_banks = cart.prg_rom.len() / 8192;
            let bank = match self.mode & 3 {
                0 => {
                    if address >= 0xE000 {
                        num_8k_banks.saturating_sub(1)
                    } else if address >= 0xC000 {
                        num_8k_banks.saturating_sub(2)
                    } else if address >= 0xA000 {
                        self.vrc2_prg[1] as usize
                    } else {
                        self.vrc2_prg[0] as usize
                    }
                }
                1 => {
                    let swap = ((self.mmc3_ctrl >> 5) & 2) as usize;
                    if address >= 0xE000 {
                        self.get_prg_bank(self.mmc3_regs[9], num_8k_banks)
                    } else if address >= 0xC000 {
                        self.get_prg_bank(self.mmc3_regs[6 + (swap ^ 2)], num_8k_banks)
                    } else if address >= 0xA000 {
                        self.get_prg_bank(self.mmc3_regs[7], num_8k_banks)
                    } else {
                        self.get_prg_bank(self.mmc3_regs[6 + swap], num_8k_banks)
                    }
                }
                _ => {
                    let bank_16k = (self.mmc1_regs[3] & 0x0F) as usize;
                    if (self.mmc1_regs[0] & 8) != 0 {
                        if (self.mmc1_regs[0] & 4) != 0 {
                            if address < 0xC000 {
                                bank_16k * 2 + ((address as usize & 0x3FFF) / 8192)
                            } else {
                                0x0F * 2 + ((address as usize & 0x3FFF) / 8192)
                            }
                        } else if address < 0xC000 {
                            (address as usize & 0x3FFF) / 8192
                        } else {
                            bank_16k * 2 + ((address as usize & 0x3FFF) / 8192)
                        }
                    } else {
                        let bank_32k = bank_16k >> 1;
                        bank_32k * 4 + ((address as usize & 0x7FFF) / 8192)
                    }
                }
            };
            let offset = (bank * 8192 + (address as usize & 0x1FFF)) % cart.prg_rom.len();
            FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            }
        } else if address >= 0x4100 && address < 0x8000 {
            if (address & 0x4100) == 0x4100 {
                FetchResult {
                    data: self.mode,
                    driven: true,
                }
            } else {
                FetchResult {
                    data: 0,
                    driven: false,
                }
            }
        } else if address >= 0x6000 {
            if !cart.prg_ram.is_empty() {
                let offset = (address as usize & 0x1FFF) % cart.prg_ram.len();
                FetchResult {
                    data: cart.prg_ram[offset],
                    driven: true,
                }
            } else {
                FetchResult {
                    data: 0,
                    driven: false,
                }
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            match self.mode & 3 {
                0 => {
                    if address >= 0xB000 && address <= 0xE003 {
                        let ind = (((((address & 2) as usize) | ((address >> 10) as usize)) >> 1) + 2) & 7;
                        let sar = ((address & 1) << 2) as usize;
                        self.vrc2_chr[ind] =
                            (self.vrc2_chr[ind] & (0xF0 >> sar)) | ((data & 0x0F) << sar);
                    } else {
                        match address & 0xF000 {
                            0x8000 => self.vrc2_prg[0] = data,
                            0xA000 => self.vrc2_prg[1] = data,
                            0x9000 => self.vrc2_mirr = data,
                            _ => {}
                        }
                    }
                }
                1 => match address & 0xE001 {
                    0x8000 => self.mmc3_ctrl = data,
                    0x8001 => {
                        let reg_idx = (self.mmc3_ctrl & 7) as usize;
                        self.mmc3_regs[reg_idx] = data;
                    }
                    0xA000 => self.mmc3_mirr = data,
                    0xC000 => self.irq_latch = data,
                    0xC001 => self.irq_reload = true,
                    0xE000 => self.irq_enabled = false,
                    0xE001 => self.irq_enabled = true,
                    _ => {}
                },
                _ => {
                    if (data & 0x80) != 0 {
                        self.mmc1_regs[0] |= 0x0C;
                        self.mmc1_buffer = 0;
                        self.mmc1_shift = 0;
                    } else {
                        let reg_idx = ((address >> 13) - 4) as usize;
                        self.mmc1_buffer |= (data & 1) << self.mmc1_shift;
                        self.mmc1_shift += 1;
                        if self.mmc1_shift == 5 {
                            self.mmc1_regs[reg_idx] = self.mmc1_buffer;
                            self.mmc1_buffer = 0;
                            self.mmc1_shift = 0;
                        }
                    }
                }
            }
        } else if address >= 0x4100 && address < 0x8000 {
            if (address & 0x4100) == 0x4100 {
                self.mode = data;
                if (address & 1) != 0 {
                    self.mmc1_regs[0] = 0x0C;
                    self.mmc1_regs[3] = 0;
                    self.mmc1_buffer = 0;
                    self.mmc1_shift = 0;
                }
            }
        } else if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let offset = (address as usize & 0x1FFF) % cart.prg_ram.len();
                cart.prg_ram[offset] = data;
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
        prg_vram: &[u8],
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
            let bank = self.chr_bank(address);
            let byte = self.read_chr(address, chr_rom, chr_ram, bank);
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else {
                self.mirror_address(address)
            };
            if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                new_addr_bus |= prg_vram[mirrored as usize & 0x07FF] as u16;
            } else {
                new_addr_bus |= vram[mirrored as usize & 0x07FF] as u16;
            }
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

    fn ppu_clock(
        &mut self,
        _ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        _ppu_sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        if rendering_on
            && dot == 257
            && (scanline < 240 || scanline == 261)
        {
            return self.hbirq_tick();
        }
        false
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.push(self.mode);
        state.extend_from_slice(&self.vrc2_chr);
        state.extend_from_slice(&self.vrc2_prg);
        state.push(self.vrc2_mirr);
        state.extend_from_slice(&self.mmc3_regs);
        state.push(self.mmc3_ctrl);
        state.push(self.mmc3_mirr);
        state.push(u8::from(self.irq_reload));
        state.push(self.irq_count);
        state.push(self.irq_latch);
        state.push(u8::from(self.irq_enabled));
        state.extend_from_slice(&self.mmc1_regs);
        state.push(self.mmc1_buffer);
        state.push(self.mmc1_shift);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..cart.prg_ram.len() {
            cart.prg_ram[i] = state[p];
            p += 1;
        }
        for i in 0..cart.chr_ram.len() {
            cart.chr_ram[i] = state[p];
            p += 1;
        }
        if p + 34 <= state.len() {
            self.mode = state[p];
            p += 1;
            self.vrc2_chr.copy_from_slice(&state[p..p + 8]);
            p += 8;
            self.vrc2_prg.copy_from_slice(&state[p..p + 2]);
            p += 2;
            self.vrc2_mirr = state[p];
            p += 1;
            self.mmc3_regs.copy_from_slice(&state[p..p + 10]);
            p += 10;
            self.mmc3_ctrl = state[p];
            p += 1;
            self.mmc3_mirr = state[p];
            p += 1;
            self.irq_reload = state[p] != 0;
            p += 1;
            self.irq_count = state[p];
            p += 1;
            self.irq_latch = state[p];
            p += 1;
            self.irq_enabled = state[p] != 0;
            p += 1;
            self.mmc1_regs.copy_from_slice(&state[p..p + 4]);
            p += 4;
            self.mmc1_buffer = state[p];
            p += 1;
            self.mmc1_shift = state[p];
            p += 1;
        }
        p
    }
}
