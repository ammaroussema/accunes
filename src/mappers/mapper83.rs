use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper83 {
    mapper_num: u16,
    outer_bank: u8,
    misc: u8,
    prg_mask: u8,
    prg: [u8; 4],
    chr_mode: u8,
    chr: [u8; 8],
    dip_mask: u16,
    dip_switches: u8,
    scratch: [u8; 4],
    counter: u16,
    counting: bool,
}

impl Mapper83 {
    pub fn new(mapper_num: u16, submapper: u8) -> Self {
        let (prg_mask, chr_mode, dip_mask) = if mapper_num == 264 {
            (0x0F, 1, 0x400)
        } else {
            let chr_mode = submapper;
            (0x1F, chr_mode, 0x100)
        };
        let mut m = Mapper83 {
            mapper_num,
            outer_bank: 0,
            misc: 2 << 3,
            prg_mask,
            prg: [0xFC, 0xFD, 0xFE, 0xFF],
            chr_mode,
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            dip_mask,
            dip_switches: 0,
            scratch: [0; 4],
            counter: 0,
            counting: false,
        };
        m.reset();
        m
    }

    fn mirroring(&self) -> u8 {
        self.misc & 0x03
    }

    fn prg_mode(&self) -> u8 {
        (self.misc >> 3) & 3
    }

    fn decreasing(&self) -> bool {
        (self.misc & 0x40) != 0
    }

    fn counter_enabled(&self) -> bool {
        (self.misc & 0x80) != 0
    }

    fn decode_address(&self, address: u16) -> u16 {
        if self.mapper_num == 264 {
            (address >> 2 & 0x3FC0) | (address & 0x003F)
        } else {
            address
        }
    }
}

impl Mapper for Mapper83 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let prg_mode = self.prg_mode();
            let bank = match prg_mode {
                0 => {
                    let bank_idx = if address < 0xC000 {
                        self.outer_bank as usize
                    } else {
                        (self.outer_bank | (self.prg_mask >> 1)) as usize
                    };
                    let bank_count = (cart.prg_rom.len() / 0x4000).max(1);
                    let bank = bank_idx % bank_count;
                    let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                    FetchResult {
                        data: cart.prg_rom[offset % cart.prg_rom.len()],
                        driven: true,
                    }
                }
                1 => {
                    let bank_idx = (self.outer_bank >> 1) as usize;
                    let bank_count = (cart.prg_rom.len() / 0x8000).max(1);
                    let bank = bank_idx % bank_count;
                    let offset = bank * 0x8000 + (address as usize & 0x7FFF);
                    FetchResult {
                        data: cart.prg_rom[offset % cart.prg_rom.len()],
                        driven: true,
                    }
                }
                2 | 3 => {
                    let bank_idx = match address {
                        0x8000..=0x9FFF => 0,
                        0xA000..=0xBFFF => 1,
                        0xC000..=0xDFFF => 2,
                        0xE000..=0xFFFF => 3,
                        _ => return FetchResult { data: 0, driven: false },
                    };
                    let base = (self.outer_bank << 1) & !self.prg_mask;
                    let val = if bank_idx == 3 {
                        0x1F
                    } else {
                        self.prg[bank_idx]
                    };
                    let bank_idx = (base | (val & self.prg_mask)) as usize;
                    let bank_count = (cart.prg_rom.len() / 0x2000).max(1);
                    let bank = bank_idx % bank_count;
                    let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                    FetchResult {
                        data: cart.prg_rom[offset % cart.prg_rom.len()],
                        driven: true,
                    }
                }
                _ => FetchResult { data: 0, driven: false },
            };
            bank
        } else if address >= 0x6000 {
            let offset = (address - 0x6000) as usize;
            if offset < cart.prg_ram.len() {
                FetchResult {
                    data: cart.prg_ram[offset],
                    driven: true,
                }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else if address >= 0x5000 {
            if address & self.dip_mask != 0 {
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
            let reg = ((addr >> 8) & 3) as usize;
            let index = (addr & 0x1F) as usize;
            match reg {
                0 => {
                    self.outer_bank = data;
                }
                1 => {
                    self.misc = data;
                }
                2 => {
                    if index & 1 != 0 {
                        self.counter = (self.counter & 0x00FF) | ((data as u16) << 8);
                        self.counting = self.counter_enabled();
                    } else {
                        self.counter = (self.counter & 0xFF00) | (data as u16);
                    }
                }
                3 => {
                    if index < 0x10 {
                        self.prg[index & 3] = data;
                    } else if index < 0x18 {
                        self.chr[index & 7] = data;
                    }
                }
                _ => {}
            }
        } else if address >= 0x6000 {
            let offset = (address - 0x6000) as usize;
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
        } else if address >= 0x5000 {
            self.scratch[address as usize & 3] = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else {
            match self.mirroring() {
                0 => address & 0x2FFF, 
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1), 
                2 => address & 0x3FFF, 
                3 => address | 0x400, 
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
            let chr_data = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[address as usize & 0x1FFF] }
            } else {
                let bank = match self.chr_mode {
                    0 => {
                        let bank_idx = (address >> 10) as usize;
                        let bank = self.chr[bank_idx & 7] as usize;
                        let bank_count = (chr_rom.len() / 0x400).max(1);
                        let bank = bank % bank_count;
                        let offset = bank * 0x400 + (address as usize & 0x3FF);
                        if chr_rom.is_empty() { 0 } else { chr_rom[offset % chr_rom.len()] }
                    }
                    1 => {
                        let bank_idx = match address {
                            0x0000..=0x07FF => 0,
                            0x0800..=0x0FFF => 1,
                            0x1000..=0x17FF => 6,
                            0x1800..=0x1FFF => 7,
                            _ => 0,
                        };
                        let bank = self.chr[bank_idx] as usize;
                        let bank_count = (chr_rom.len() / 0x800).max(1);
                        let bank = bank % bank_count;
                        let offset = bank * 0x800 + (address as usize & 0x7FF);
                        if chr_rom.is_empty() { 0 } else { chr_rom[offset % chr_rom.len()] }
                    }
                    2 => {
                        let bank_idx = (address >> 10) as usize;
                        let bank = (self.chr[bank_idx & 7] | ((self.outer_bank << 4) & 0x30)) as usize;
                        let bank_count = (chr_rom.len() / 0x400).max(1);
                        let bank = bank % bank_count;
                        let offset = bank * 0x400 + (address as usize & 0x3FF);
                        if chr_rom.is_empty() { 0 } else { chr_rom[offset % chr_rom.len()] }
                    }
                    _ => 0,
                };
                bank
            };
            new_addr_bus |= chr_data as u16;
        } else if address < 0x3F00 {
            let mirrored = self.mirror_nametable(&crate::cartridge::Cartridge {
                name: String::new(),
                prg_rom: vec![],
                prg_ram: vec![],
                chr_rom: vec![],
                chr_ram: vec![],
                prg_vram: vec![],
                memory_mapper: 0,
                sub_mapper: 0,
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
                is_vs_system: false,
                mapper_chip: Box::new(crate::mapper::MapperNROM::new(crate::mapper::NromConfig::default())),
                tv_system: crate::region::TvSystem::Unknown,
            }, address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.counting && self.counter != 0 {
            if self.decreasing() {
                self.counter -= 1;
            } else {
                self.counter += 1;
            }
            if self.counter == 0 {
                self.counting = false;
                true 
            } else {
                false
            }
        } else {
            false
        }
    }

    fn reset(&mut self) {
        self.outer_bank = 0;
        self.misc = 2 << 3;
        for i in 0..4 {
            self.prg[i] = 0xFC | i as u8;
        }
        for i in 0..8 {
            self.chr[i] = i as u8;
        }
        for i in 0..4 {
            self.scratch[i] = 0;
        }
        self.counter = 0;
        self.counting = false;
        self.dip_switches = 0;
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
        state.push(self.outer_bank);
        state.push(self.misc);
        state.extend_from_slice(&self.prg);
        state.extend_from_slice(&self.chr);
        state.extend_from_slice(&self.scratch);
        state.extend_from_slice(&self.counter.to_le_bytes());
        state.push(self.counting as u8);
        state.push(self.dip_switches);
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
        if start < state.len() {
            self.outer_bank = state[start];
            start += 1;
        }
        if start < state.len() {
            self.misc = state[start];
            start += 1;
        }
        if start + 4 <= state.len() {
            self.prg.copy_from_slice(&state[start..start + 4]);
            start += 4;
        }
        if start + 8 <= state.len() {
            self.chr.copy_from_slice(&state[start..start + 8]);
            start += 8;
        }
        if start + 4 <= state.len() {
            self.scratch.copy_from_slice(&state[start..start + 4]);
            start += 4;
        }
        if start + 2 <= state.len() {
            self.counter = u16::from_le_bytes([state[start], state[start + 1]]);
            start += 2;
        }
        if start < state.len() {
            self.counting = state[start] != 0;
            start += 1;
        }
        if start < state.len() {
            self.dip_switches = state[start];
            start += 1;
        }
        start
    }
}
