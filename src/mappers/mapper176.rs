use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper176Config {
    pub submapper: u8,
    pub prg_ram_size: usize,
    pub chr_ram_size: usize,
    pub has_battery: bool,
    pub dip_value: u16,
    pub is_523: bool,
    pub header_horizontal_mirroring: bool,
}

impl Mapper176Config {
    pub fn for_ines(
        header: &[u8],
        submapper_id: u8,
        _rom: &[u8],
        prg_size: u8,
        using_chr_ram: bool,
        has_battery: bool,
    ) -> Self {
        let is_nes2 = (header[7] & 0x0C) == 0x08;
        let prg_rom_len = prg_size as usize * 0x4000;
        let chr_rom_len = if using_chr_ram { 0 } else { header[5] as usize * 0x2000 };

        let mut submapper = submapper_id;
        let mut prg_ram_size = if has_battery { 32768 } else { 8192 };

        if !is_nes2 {
            if has_battery {
                submapper = 2;
                prg_ram_size = 32768;
            } else if prg_rom_len == 1024 * 1024 && chr_rom_len == 1024 * 1024 {
                submapper = 1;
            } else if prg_rom_len == 256 * 1024 && chr_rom_len == 128 * 1024 {
                submapper = 1;
            } else if prg_rom_len >= 8192 * 1024 && chr_rom_len == 0 {
                submapper = 2;
            } else if prg_rom_len == 4096 * 1024 && chr_rom_len == 0 {
                submapper = 3;
            }
        }

        let chr_ram_size = if !is_nes2 {
            if chr_rom_len > 0 {
                if prg_rom_len == 2048 * 1024 && chr_rom_len == 512 * 1024 {
                    8192
                } else {
                    0
                }
            } else {
                if prg_rom_len >= 2_097_152 {
                    128 * 1024
                } else {
                    8192
                }
            }
        } else {
            let volatile_shift = header[11] & 0x0F;
            let nv_shift = header[11] >> 4;
            let volatile_size = if volatile_shift > 0 { 64usize << volatile_shift } else { 0 };
            let nv_size = if nv_shift > 0 { 64usize << nv_shift } else { 0 };
            let nes2_ram = volatile_size + nv_size;
            if nes2_ram > 0 {
                nes2_ram
            } else if using_chr_ram || chr_rom_len == 0 {
                if prg_rom_len >= 2_097_152 {
                    128 * 1024
                } else {
                    8192
                }
            } else {
                0
            }
        };

        let header_horizontal_mirroring = if header.len() > 6 {
            (header[6] & 1) == 0
        } else {
            false
        };

        Self {
            submapper,
            prg_ram_size,
            chr_ram_size,
            has_battery,
            dip_value: 0x010,
            is_523: false,
            header_horizontal_mirroring,
        }
    }
}

pub struct Mapper176 {
    pointer: u8,
    mmc3_reg: [u8; 16],
    fk23_reg: [u8; 8],
    mirroring: u8,
    wram: u8,
    latch: u8,
    reg4800: u8,

    counter: u8,
    reload_value: u8,
    pa12_filter: u8,
    irq_enabled: bool,
    irq_pending: bool,
    irq_ack: bool,

    submapper: u8,
    has_battery: bool,
    dip_switches: u8,
    dip_value: u16,
    is_523: bool,
    header_horizontal_mirroring: bool,

    prg_ram: Vec<u8>,
    chr_ram: Vec<u8>,
}

impl Mapper176 {
    pub fn new(config: Mapper176Config) -> Self {
        let mut fk23_reg = [0u8; 8];
        if config.submapper == 1 {
            fk23_reg[1] = 0xFF;
        }
        let wram = if config.submapper == 2 { 0xC0 } else { 0x80 };
        let initial_mmc3 = [
            0x00, 0x02, 0x04, 0x05, 0x06, 0x07, 0x00, 0x01, 0xFE, 0xFF, 0x01, 0x03, 0, 0, 0, 0,
        ];

        let mut m = Self {
            pointer: 0,
            mmc3_reg: initial_mmc3,
            fk23_reg,
            mirroring: 0,
            wram,
            latch: 0,
            reg4800: 0,
            counter: 0,
            reload_value: 0,
            pa12_filter: 0,
            irq_enabled: false,
            irq_pending: false,
            irq_ack: false,
            submapper: config.submapper,
            has_battery: config.has_battery,
            dip_switches: 0,
            dip_value: config.dip_value,
            is_523: config.is_523,
            header_horizontal_mirroring: config.header_horizontal_mirroring,
            prg_ram: vec![0; config.prg_ram_size],
            chr_ram: vec![0; config.chr_ram_size],
        };
        if config.is_523 {
            m.mirroring = if config.header_horizontal_mirroring { 1 } else { 0 };
        }
        m
    }

    fn mmc3_extended(&self) -> bool {
        (self.fk23_reg[3] & 0x02) != 0 && (self.submapper == 1 || self.submapper == 2 || self.is_523)
    }

    fn get_prg_bank(&self, bank: usize) -> usize {
        let prg_mode = (self.fk23_reg[0] & 7) as usize;
        let prg_mode_and = [0x3F, 0x1F, 0x0F, 0x00, 0x00, 0x00, 0x7F, 0xFF];
        let mut prg_and = prg_mode_and[prg_mode];
        let mut prg_or = (self.fk23_reg[1] & 0x7F) as usize;

        let extended = self.mmc3_extended();

        match self.submapper {
            1 => {
                if prg_mode == 0 && extended {
                    prg_and = 0xFF;
                }
            }
            2 => {
                prg_or |= (((self.fk23_reg[0] as usize) << 4) & 0x080)
                    | (((self.fk23_reg[0] as usize) << 1) & 0x100)
                    | (((self.fk23_reg[2] as usize) << 3) & 0x600)
                    | (((self.fk23_reg[2] as usize) << 6) & 0x800);
            }
            3 => {
                prg_or |= (self.fk23_reg[5] as usize) << 7;
                if prg_mode == 0 {
                    prg_and = 0xFF;
                }
            }
            4 => {
                prg_or |= (self.fk23_reg[2] as usize) & 0x080;
            }
            5 => {
                prg_or = (prg_or & 0x1F) | ((self.reg4800 as usize) << 5);
            }
            _ => {}
        }

        match prg_mode {
            3 => (prg_or << 1) | (bank & 1),
            4 => ((prg_or << 1) & !3) | bank,
            5 => {
                ((prg_or << 1) & !15)
                    | (((self.latch as usize) << 1) & 14)
                    | (bank & 1)
                    | if (bank & 2) != 0 { 14 } else { 0 }
            }
            6 | 7 => 0,
            _ => {
                let prg_invert = (self.pointer & 0x40) != 0;
                let mut b = bank;
                if (b & 1) == 0 && prg_invert {
                    b ^= 2;
                }
                let reg_val = if !extended && (b & 2) != 0 {
                    0xFE | (b & 1)
                } else {
                    self.mmc3_reg[6 + b] as usize
                };
                let shifted_or = prg_or << 1;
                (reg_val & prg_and) | (shifted_or & !prg_and)
            }
        }
    }

    fn get_mmc3_chr_bank(&self, bank: usize) -> usize {
        let bank2ext = [0, 10, 1, 11, 2, 3, 4, 5];
        let bank2reg = [0, 0, 1, 1, 2, 3, 4, 5];
        let bank_and = [!1usize, !1, !1, !1, !0, !0, !0, !0];
        let bank_or = [0usize, 1, 0, 1, 0, 0, 0, 0];

        let chr_invert = (self.pointer & 0x80) != 0;
        let mut b = bank;
        if chr_invert {
            b ^= 4;
        }

        if self.mmc3_extended() {
            self.mmc3_reg[bank2ext[b]] as usize
        } else {
            (self.mmc3_reg[bank2reg[b]] as usize & bank_and[b]) | bank_or[b]
        }
    }

    fn get_chr_bank(&self, bank: usize, chr_rom_len: usize) -> usize {
        let chr_nrom = (self.fk23_reg[0] & 0x60) != 0
            || (chr_rom_len == 0 && self.chr_ram.len() == 8192);
        let chr_cnrom =
            (self.fk23_reg[0] & 0x20) == 0 && (self.submapper == 1 || self.submapper == 5);
        let chr_small = (self.fk23_reg[0] & 0x10) != 0;

        let mut chr_or = (self.fk23_reg[2] as usize) << 3;
        let chr_and = if chr_small { 0x7F } else { 0xFF };

        if self.submapper == 3 {
            chr_or |= (self.fk23_reg[6] as usize) << 11;
        }

        if chr_nrom {
            let mask = if chr_cnrom {
                if chr_small { 0x08 } else { 0x18 }
            } else {
                0x00
            };
            let base_or = if self.submapper == 5 {
                chr_or
            } else {
                chr_or & !mask
            };
            base_or | (((self.latch as usize) << 3) & mask) | bank
        } else {
            let mmc3_bank = self.get_mmc3_chr_bank(bank);
            (mmc3_bank & chr_and) | (chr_or & !chr_and)
        }
    }

    fn mirror_nametable_addr(&self, address: u16) -> u16 {
        if self.is_523 {
            return if self.header_horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
        }
        let mode = self.mirroring & if self.submapper == 2 { 3 } else { 1 };
        match mode {
            // Vertical: A,B side-by-side; $2800/$2C00 mirror $2000/$2400
            0 => address & 0x37FF,
            // Horizontal: A,B stacked; $2400/$2C00 mirror $2000/$2800
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            // Single-screen lower bank
            2 => address & 0x23FF,
            // Single-screen upper bank
            3 => (address & 0x23FF) | 0x0400,
            _ => address & 0x37FF,
        }
    }
}

impl Mapper for Mapper176 {
    fn reset(&mut self) {
        self.pointer = 0;
        self.mmc3_reg = [
            0x00, 0x02, 0x04, 0x05, 0x06, 0x07, 0x00, 0x01, 0xFE, 0xFF, 0x01, 0x03, 0, 0, 0, 0,
        ];
        self.fk23_reg = [0; 8];
        if self.submapper == 1 {
            self.fk23_reg[1] = 0xFF;
        }
        self.mirroring = if self.is_523 {
            if self.header_horizontal_mirroring { 1 } else { 0 }
        } else {
            0
        };
        self.wram = if self.submapper == 2 { 0xC0 } else { 0x80 };
        self.latch = 0;
        self.reg4800 = 0;
        self.counter = 0;
        self.reload_value = 0;
        self.pa12_filter = 0;
        self.irq_enabled = false;
        self.irq_pending = false;
        self.irq_ack = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            if (self.wram & 0x80) != 0 && !self.prg_ram.is_empty() {
                let ram_offset = if self.submapper == 2 {
                    match (self.wram >> 5) & 3 {
                        0 => 0x2000 + (address as usize & 0x1FFF),
                        1 => ((self.wram & 1) as usize) * 0x4000 + 0x2000 + (address as usize & 0x1FFF),
                        2 => address as usize & 0x1FFF,
                        3 => ((self.wram & 3) as usize) * 0x2000 + (address as usize & 0x1FFF),
                        _ => address as usize & 0x1FFF,
                    }
                } else {
                    address as usize & 0x1FFF
                };
                let idx = ram_offset % self.prg_ram.len();
                FetchResult {
                    data: self.prg_ram[idx],
                    driven: true,
                }
            } else {
                FetchResult {
                    data: 0,
                    driven: false,
                }
            }
        } else if address >= 0x5000 && address < 0x6000 {
            let mut data = 0u8;
            let mut driven = false;
            if self.submapper == 2 && (self.wram & 0x80) != 0 && !self.prg_ram.is_empty() {
                let ram_offset = match (self.wram >> 5) & 3 {
                    0 => Some(0x1000 + (address as usize & 0xFFF)),
                    1 => Some(((self.wram & 1) as usize) * 0x4000 + 0x1000 + (address as usize & 0xFFF)),
                    _ => None,
                };
                if let Some(off) = ram_offset {
                    data = self.prg_ram[off % self.prg_ram.len()];
                    driven = true;
                }
            }
            FetchResult { data, driven }
        } else if address >= 0x8000 {
            let slot = ((address - 0x8000) / 0x2000) as usize;
            let bank = self.get_prg_bank(slot);
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let len = cart.prg_rom.len();
            FetchResult {
                data: if len > 0 {
                    cart.prg_rom[offset % len]
                } else {
                    0
                },
                driven: true,
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if (self.wram & 0x80) != 0 && !self.prg_ram.is_empty() {
                let ram_offset = if self.submapper == 2 {
                    match (self.wram >> 5) & 3 {
                        0 => 0x2000 + (address as usize & 0x1FFF),
                        1 => ((self.wram & 1) as usize) * 0x4000 + 0x2000 + (address as usize & 0x1FFF),
                        2 => address as usize & 0x1FFF,
                        3 => ((self.wram & 3) as usize) * 0x2000 + (address as usize & 0x1FFF),
                        _ => address as usize & 0x1FFF,
                    }
                } else {
                    address as usize & 0x1FFF
                };
                let idx = ram_offset % self.prg_ram.len();
                self.prg_ram[idx] = data;
            }
        } else if address >= 0x5000 && address < 0x6000 {
            if self.submapper == 2 && (self.wram & 0x80) != 0 && !self.prg_ram.is_empty() {
                let ram_offset = match (self.wram >> 5) & 3 {
                    0 => Some(0x1000 + (address as usize & 0xFFF)),
                    1 => Some(((self.wram & 1) as usize) * 0x4000 + 0x1000 + (address as usize & 0xFFF)),
                    _ => None,
                };
                if let Some(off) = ram_offset {
                    let idx = off % self.prg_ram.len();
                    self.prg_ram[idx] = data;
                }
            }
            let enable_regs = self.submapper != 2 || (self.wram & 0x40) != 0;
            if enable_regs && (address & self.dip_value) != 0 {
                let mask = if self.submapper == 3 { 7 } else { 3 };
                let index = (address & mask) as usize;
                self.fk23_reg[index] = data;
            }
        } else if address >= 0x4000 && address < 0x5000 {
            if self.submapper == 5 && (address & 0x800) != 0 {
                self.reg4800 = data;
            }
        } else if address >= 0x8000 {
            match (address >> 13) & 3 {
                0 => {
                    match address & 3 {
                        0 => {
                            self.pointer = data;
                            if self.submapper == 2
                                && (self.pointer & 0x40) != 0
                                && (data & 7) >= 6
                            {
                                self.pointer ^= 1;
                            }
                        }
                        1 => {
                            let extended = self.mmc3_extended();
                            let idx = (self.pointer & if extended { 0x0F } else { 0x07 }) as usize;
                            self.mmc3_reg[idx] = data;
                        }
                        _ => {}
                    }
                    self.latch = data;
                }
                1 => {
                    if (address & 1) != 0 {
                        self.wram = data;
                    } else {
                        self.mirroring = data;
                    }
                    self.latch = data;
                }
                2 => {
                    if (address & 1) != 0 {
                        self.counter = 0;
                    } else {
                        self.reload_value = data;
                    }
                    self.latch = data;
                }
                3 => {
                    if (address & 1) != 0 {
                        self.irq_enabled = true;
                    } else {
                        self.irq_enabled = false;
                        self.irq_pending = false;
                        self.irq_ack = true;
                    }
                    self.latch = data;
                }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror_nametable_addr(address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        _chr_ram_cart: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank_1k_idx = (address / 0x0400) as usize;
            let bank = if self.is_523 {
                let base = self.get_chr_bank(bank_1k_idx & !1, chr_rom.len());
                base * 2 + (bank_1k_idx & 1)
            } else {
                self.get_chr_bank(bank_1k_idx, chr_rom.len())
            };

            let chr_ram_mode = (self.fk23_reg[0] & 0x20) != 0
                && (self.fk23_reg[0] & 0x40) == 0
                && !self.chr_ram.is_empty();
            let chr_mixed = self.submapper == 2 && (self.wram & 0x04) != 0;

            let chr_ram_active =
                chr_ram_mode || chr_rom.is_empty() || (chr_mixed && bank < 8);

            let offset = bank * 0x0400 + (address as usize & 0x03FF);

            let byte = if chr_ram_active && !self.chr_ram.is_empty() {
                self.chr_ram[offset % self.chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let nt_addr = self.mirror_nametable_addr(address);
            new_addr_bus |= vram[(nt_addr & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            let bank_1k_idx = (address / 0x0400) as usize;
            let bank = if self.is_523 {
                let base = self.get_chr_bank(bank_1k_idx & !1, cart.chr_rom.len());
                base * 2 + (bank_1k_idx & 1)
            } else {
                self.get_chr_bank(bank_1k_idx, cart.chr_rom.len())
            };

            let chr_ram_mode = (self.fk23_reg[0] & 0x20) != 0
                && (self.fk23_reg[0] & 0x40) == 0
                && !self.chr_ram.is_empty();
            let chr_mixed = self.submapper == 2 && (self.wram & 0x04) != 0;

            let chr_ram_active =
                chr_ram_mode || cart.chr_rom.is_empty() || (chr_mixed && bank < 8);

            if chr_ram_active && !self.chr_ram.is_empty() {
                let offset = bank * 0x0400 + (address as usize & 0x03FF);
                let idx = offset % self.chr_ram.len();
                self.chr_ram[idx] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let nt_addr = self.mirror_nametable_addr(address);
            vram[(nt_addr & 0x7FF) as usize] = data;
        }
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        ppu_a12_prev: bool,
        _scanline: u16,
        _dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        let a12 = (ppu_address_bus & 0x1000) != 0;
        let mut irq = false;
        if !ppu_a12_prev && a12 {
            if self.pa12_filter == 0 {
                if self.counter == 0 {
                    self.counter = self.reload_value;
                } else {
                    self.counter -= 1;
                }
                if self.counter == 0 && self.irq_enabled {
                    self.irq_pending = true;
                    irq = true;
                }
            }
            self.pa12_filter = 5;
        }
        irq
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.pa12_filter > 0 {
            self.pa12_filter = self.pa12_filter.saturating_sub(cycles);
        }
        false
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.irq_ack {
            self.irq_ack = false;
            true
        } else {
            false
        }
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
        self.dip_value = if (value & 1) != 0 {
            0x010
        } else if (value & 2) != 0 {
            0x020
        } else if (value & 4) != 0 {
            0x040
        } else if (value & 8) != 0 {
            0x080
        } else if (value & 16) != 0 {
            0x100
        } else {
            0x010
        };
    }

    fn battery_save_data(&self, _cart: &Cartridge) -> Option<Vec<u8>> {
        if self.has_battery && !self.prg_ram.is_empty() {
            Some(self.prg_ram.clone())
        } else {
            None
        }
    }

    fn load_battery_save(&mut self, _cart: &mut Cartridge, data: &[u8]) {
        if self.has_battery && !self.prg_ram.is_empty() {
            let len = data.len().min(self.prg_ram.len());
            self.prg_ram[..len].copy_from_slice(&data[..len]);
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut s = Vec::new();
        s.push(self.pointer);
        s.extend_from_slice(&self.mmc3_reg);
        s.extend_from_slice(&self.fk23_reg);
        s.push(self.mirroring);
        s.push(self.wram);
        s.push(self.latch);
        s.push(self.counter);
        s.push(self.reload_value);
        s.push(self.pa12_filter);
        s.push(if self.irq_enabled { 1 } else { 0 });
        s.push(if self.irq_pending { 1 } else { 0 });
        s.push(if self.irq_ack { 1 } else { 0 });
        s.push(self.reg4800);
        s.push(self.dip_switches);
        s.extend_from_slice(&self.prg_ram);
        s.extend_from_slice(&self.chr_ram);
        s
    }

    fn load_mapper_registers(
        &mut self,
        _cart: &mut Cartridge,
        state: &[u8],
        start: usize,
    ) -> usize {
        let mut p = start;
        if p < state.len() {
            self.pointer = state[p];
            p += 1;
        }
        for b in self.mmc3_reg.iter_mut() {
            if p < state.len() {
                *b = state[p];
                p += 1;
            }
        }
        for b in self.fk23_reg.iter_mut() {
            if p < state.len() {
                *b = state[p];
                p += 1;
            }
        }
        if p < state.len() {
            self.mirroring = state[p];
            p += 1;
        }
        if p < state.len() {
            self.wram = state[p];
            p += 1;
        }
        if p < state.len() {
            self.latch = state[p];
            p += 1;
        }
        if p < state.len() {
            self.counter = state[p];
            p += 1;
        }
        if p < state.len() {
            self.reload_value = state[p];
            p += 1;
        }
        if p < state.len() {
            self.pa12_filter = state[p];
            p += 1;
        }
        if p < state.len() {
            self.irq_enabled = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.irq_pending = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.irq_ack = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.reg4800 = state[p];
            p += 1;
        }
        if p < state.len() {
            self.set_dip_switches(state[p]);
            p += 1;
        }
        for b in self.prg_ram.iter_mut() {
            if p < state.len() {
                *b = state[p];
                p += 1;
            }
        }
        for b in self.chr_ram.iter_mut() {
            if p < state.len() {
                *b = state[p];
                p += 1;
            }
        }
        p
    }
}
