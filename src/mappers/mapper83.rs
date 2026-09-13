use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper83 {
    mapper_num: u16,
    submapper: u8,
    reg: [u8; 16],
    scratch: [u8; 4],
    flags: u8,
    previous_pa: u16,
    dip_switches: u8,
    irq_pending: bool,
}

impl Mapper83 {
    pub fn new(mapper_num: u16, submapper: u8) -> Self {
        let mut m = Mapper83 {
            mapper_num,
            submapper,
            reg: [0; 16],
            scratch: [0; 4],
            flags: 0,
            previous_pa: 0,
            dip_switches: 0,
            irq_pending: false,
        };
        m.reset();
        m
    }

    fn prg_and(&self) -> u8 {
        if self.mapper_num == 264 || self.submapper == 3 {
            0x0F
        } else {
            0x1F
        }
    }

    fn decode_address(&self, address: u16) -> u16 {
        let mut addr = address & 0x0FFF;
        if self.mapper_num == 264 {
            addr = ((addr >> 2) & !0x3F) | (addr & 0x3F);
        }
        addr
    }

    fn clock_counter(&mut self) {
        let mut counter = (self.reg[2] as u16) | ((self.reg[3] as u16) << 8);
        if (self.flags & 0x80) != 0 && counter != 0 {
            if (self.reg[1] & 0x40) != 0 {
                counter = counter.wrapping_sub(1);
            } else {
                counter = counter.wrapping_add(1);
            }
            if counter == 0 {
                self.irq_pending = true;
                self.flags &= !0x80;
            }
        }
        self.reg[2] = (counter & 0xFF) as u8;
        self.reg[3] = (counter >> 8) as u8;
    }
}

impl Mapper for Mapper83 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let prg_len = cart.prg_rom.len();
            if prg_len == 0 {
                return FetchResult { data: 0, driven: false };
            }
            let prg_and = self.prg_and() as usize;
            let reg0 = self.reg[0] as usize;

            let offset = match self.reg[1] & 0x18 {
                0x00 => {
                    let bank_16k = if address < 0xC000 {
                        reg0
                    } else {
                        reg0 | (prg_and >> 1)
                    };
                    (bank_16k * 0x4000 + (address as usize & 0x3FFF)) % prg_len
                }
                0x08 => {
                    let bank_16k = reg0;
                    (bank_16k * 0x4000 + (address as usize & 0x3FFF)) % prg_len
                }
                0x10 | 0x18 => {
                    let slot = match address {
                        0x8000..=0x9FFF => 0,
                        0xA000..=0xBFFF => 1,
                        0xC000..=0xDFFF => 2,
                        _ => 3,
                    };
                    let reg_val = if slot == 3 { 0xFF } else { self.reg[4 + slot] as usize };
                    let bank_8k = ((reg0 << 1) & !prg_and) | (reg_val & prg_and);
                    (bank_8k * 0x2000 + (address as usize & 0x1FFF)) % prg_len
                }
                _ => 0,
            };
            FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            }
        } else if address >= 0x6000 {
            if self.submapper == 2 {
                let ram_len = cart.prg_ram.len();
                if ram_len > 0 {
                    let bank = (self.reg[0] >> 6) as usize;
                    let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % ram_len;
                    FetchResult {
                        data: cart.prg_ram[offset],
                        driven: true,
                    }
                } else {
                    FetchResult { data: 0, driven: false }
                }
            } else if (self.reg[1] & 0x20) != 0 {
                let prg_len = cart.prg_rom.len();
                if prg_len > 0 {
                    let bank = self.reg[7] as usize;
                    let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % prg_len;
                    FetchResult {
                        data: cart.prg_rom[offset],
                        driven: true,
                    }
                } else {
                    FetchResult { data: 0, driven: false }
                }
            } else {
                let ram_len = cart.prg_ram.len();
                let offset = (address - 0x6000) as usize;
                if ram_len > 0 && offset < ram_len {
                    FetchResult {
                        data: cart.prg_ram[offset],
                        driven: true,
                    }
                } else {
                    FetchResult { data: 0, driven: false }
                }
            }
        } else if address >= 0x5000 {
            let dip_mask = if self.mapper_num == 264 { 0x400 } else { 0x100 };
            if (address & dip_mask) != 0 {
                FetchResult {
                    data: self.scratch[address as usize & 3],
                    driven: true,
                }
            } else {
                FetchResult {
                    data: self.dip_switches,
                    driven: true,
                }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let addr = self.decode_address(address);
            match addr & 0x318 {
                0x000 | 0x008 | 0x010 | 0x018 | 0x100 | 0x108 | 0x110 | 0x118 => {
                    self.reg[((addr >> 8) & 1) as usize] = data;
                }
                0x200 | 0x208 | 0x210 | 0x218 => {
                    self.reg[2 | (addr as usize & 1)] = data;
                    if (addr & 1) != 0 {
                        self.flags = (self.flags & !0x80) | (self.reg[1] & 0x80);
                    } else {
                        self.irq_pending = false;
                    }
                }
                0x300 | 0x308 => {
                    self.reg[4 | (addr as usize & 3)] = data;
                }
                0x310 => {
                    self.reg[8 | (addr as usize & 7)] = data;
                }
                0x318 => {
                    self.flags = (self.flags & !0x40) | (data & 0x40);
                }
                _ => {}
            }
        } else if address >= 0x6000 {
            if self.submapper == 2 {
                let ram_len = cart.prg_ram.len();
                if ram_len > 0 {
                    let bank = (self.reg[0] >> 6) as usize;
                    let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % ram_len;
                    cart.prg_ram[offset] = data;
                }
            } else {
                let ram_len = cart.prg_ram.len();
                let offset = (address - 0x6000) as usize;
                if ram_len > 0 && offset < ram_len {
                    cart.prg_ram[offset] = data;
                }
            }
        } else if address >= 0x5000 {
            self.scratch[address as usize & 3] = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else {
            match self.reg[1] & 0x03 {
                0 => address & 0x37FF,
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                2 => address & 0x23FF,
                3 => (address & 0x23FF) | 0x400,
                _ => address,
            }
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
            let chr_len = if using_chr_ram { chr_ram.len() } else { chr_rom.len() };
            let chr_data = if chr_len == 0 {
                0
            } else {
                let sub = if self.mapper_num == 264 { 1 } else { self.submapper };
                let offset = match sub {
                    0 => {
                        let bank = self.reg[8 | ((address >> 10) as usize & 7)] as usize;
                        (bank * 0x400 + (address as usize & 0x3FF)) % chr_len
                    }
                    1 => {
                        let bank = match address {
                            0x0000..=0x07FF => self.reg[8],
                            0x0800..=0x0FFF => self.reg[9],
                            0x1000..=0x17FF => self.reg[14],
                            _ => self.reg[15],
                        } as usize;
                        (bank * 0x800 + (address as usize & 0x7FF)) % chr_len
                    }
                    2 => {
                        let base = ((self.reg[0] as usize) << 4) & !0xFF;
                        let bank = base | (self.reg[8 | ((address >> 10) as usize & 7)] as usize);
                        (bank * 0x400 + (address as usize & 0x3FF)) % chr_len
                    }
                    3 => {
                        let base = ((self.reg[0] as usize) << 2) & !0xFF;
                        let bank = base | (self.reg[8 | ((address >> 10) as usize & 7)] as usize);
                        (bank * 0x400 + (address as usize & 0x3FF)) % chr_len
                    }
                    _ => 0,
                };
                if using_chr_ram {
                    chr_ram[offset % chr_ram.len()]
                } else {
                    chr_rom[offset % chr_rom.len()]
                }
            };
            new_addr_bus |= chr_data as u16;
        } else if address < 0x3F00 {
            let dummy = Cartridge {
                name: String::new(),
                prg_rom: vec![],
                prg_ram: vec![],
                chr_rom: vec![],
                chr_ram: vec![],
                prg_vram: vec![],
                memory_mapper: 83,
                sub_mapper: self.submapper,
                prg_size: 0,
                chr_size: 0,
                prg_size_minus_1: 0,
                using_chr_ram: false,
                has_battery: false,
                alternative_nametable_arrangement: false,
                nametable_horizontal_mirroring: false,
                fds_disks: vec![],
                trainer: vec![],
                misc_rom: vec![],
                mapper_cpu_cycle: 0,
                prg_rom_crc32: 0,
                chr_rom_crc32: 0,
                overall_crc32: 0,
                prg_chr_crc32: 0,
                is_vs_system: false,
                mapper_chip: Box::new(crate::mapper::MapperNROM::new(crate::mapper::NromConfig::default())),
                tv_system: crate::region::TvSystem::Unknown,
            };
            let mirrored = self.mirror_nametable(&dummy, address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if (self.flags & 0x40) == 0 {
            self.clock_counter();
        }
        self.irq_pending
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        _scanline: u16,
        _dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        if (self.flags & 0x40) != 0 && (ppu_address_bus & 0x1000) != 0 && (self.previous_pa & 0x1000) != 0 {
            self.clock_counter();
        }
        self.previous_pa = ppu_address_bus;
        self.irq_pending
    }

    fn reset(&mut self) {
        self.reg = [0; 16];
        self.scratch = [0; 4];
        self.flags = 0;
        self.previous_pa = 0;
        self.dip_switches = 0;
        self.irq_pending = false;
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.extend_from_slice(&self.reg);
        state.extend_from_slice(&self.scratch);
        state.push(self.flags);
        state.extend_from_slice(&self.previous_pa.to_le_bytes());
        state.push(self.dip_switches);
        state.push(self.irq_pending as u8);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        let prg_len = cart.prg_ram.len();
        if start + prg_len <= state.len() {
            cart.prg_ram.copy_from_slice(&state[start..start + prg_len]);
            start += prg_len;
        }
        let chr_len = cart.chr_ram.len();
        if start + chr_len <= state.len() {
            cart.chr_ram.copy_from_slice(&state[start..start + chr_len]);
            start += chr_len;
        }
        if start + 16 <= state.len() {
            self.reg.copy_from_slice(&state[start..start + 16]);
            start += 16;
        }
        if start + 4 <= state.len() {
            self.scratch.copy_from_slice(&state[start..start + 4]);
            start += 4;
        }
        if start < state.len() {
            self.flags = state[start];
            start += 1;
        }
        if start + 2 <= state.len() {
            self.previous_pa = u16::from_le_bytes([state[start], state[start + 1]]);
            start += 2;
        }
        if start < state.len() {
            self.dip_switches = state[start];
            start += 1;
        }
        if start < state.len() {
            self.irq_pending = state[start] != 0;
            start += 1;
        }
        start
    }
}
