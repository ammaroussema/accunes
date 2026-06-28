use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper91 {
    submapper: u8,
    prg: [u8; 2],
    chr: [u8; 4],
    outer_bank: u8,             
    horizontal_mirroring: bool, 
    irq_enabled: bool,
    irq_pending: bool,
    pa12_counter: u8,
    previous_pa12: bool,
    m2_prescaler: u8,           
    m2_counter: i16,
}

impl Mapper91 {
    pub fn new(submapper: u8) -> Self {
        Self {
            submapper,
            prg: [0, 1],
            chr: [0, 1, 2, 3],
            outer_bank: 0,
            horizontal_mirroring: false,
            irq_enabled: false,
            irq_pending: false,
            pa12_counter: 0,
            previous_pa12: false,
            m2_prescaler: 3,
            m2_counter: 0,
        }
    }

    fn prg_bank_offset(&self, prg_len: usize, slot: usize) -> usize {
        if prg_len == 0 { return 0; }
        let num_banks_8k = prg_len / 0x2000;
        let outer_prg = ((self.outer_bank as usize >> 1) & 0x3) << 4;
        let bank = match slot {
            0 => (self.prg[0] as usize & 0xF) | outer_prg,
            1 => (self.prg[1] as usize & 0xF) | outer_prg,
            2 => (num_banks_8k.saturating_sub(2) & 0xF) | outer_prg,
            _ => (num_banks_8k.saturating_sub(1) & 0xF) | outer_prg,
        };
        (bank % num_banks_8k) * 0x2000
    }

    fn chr_bank_offset(&self, chr_len: usize, address: u16) -> usize {
        if chr_len == 0 { return 0; }
        let slot = (address / 0x800) as usize; 
        let outer_chr = (self.outer_bank as usize & 1) << 8;
        let bank = (self.chr[slot] as usize & 0xFF) | outer_chr;
        let num_2k = chr_len / 0x800;
        (bank % num_2k) * 0x800 + (address as usize & 0x7FF)
    }
}

impl Mapper for Mapper91 {
    fn reset(&mut self) {
        self.prg = [0, 1];
        self.chr = [0, 1, 2, 3];
        self.outer_bank = 0;
        self.horizontal_mirroring = false;
        self.irq_enabled = false;
        self.irq_pending = false;
        self.pa12_counter = 0;
        self.previous_pa12 = false;
        self.m2_prescaler = 3;
        self.m2_counter = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }
        let slot = match address {
            0x8000..=0x9FFF => 0,
            0xA000..=0xBFFF => 1,
            0xC000..=0xDFFF => 2,
            _               => 3,
        };
        let base = self.prg_bank_offset(cart.prg_rom.len(), slot);
        let idx = (base + (address as usize & 0x1FFF)) % cart.prg_rom.len().max(1);
        FetchResult { data: cart.prg_rom[idx], driven: true }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x7000 {
            let reg = if self.submapper == 1 { address & 7 } else { address & 3 };
            match reg {
                0 | 1 | 2 | 3 => self.chr[reg as usize] = data,
                4 if self.submapper == 1 => self.horizontal_mirroring = true,
                5 if self.submapper == 1 => self.horizontal_mirroring = false,
                6 if self.submapper == 1 => {
                    self.m2_counter = (self.m2_counter & -256_i16) | data as i16;
                }
                7 if self.submapper == 1 => {
                    self.m2_counter = (self.m2_counter & 0x00FF) | ((data as i16) << 8);
                }
                _ => {}
            }
        } else if address >= 0x7000 && address < 0x8000 {
            let reg = if self.submapper == 1 { address & 7 } else { address & 3 };
            match reg {
                0 | 1 => self.prg[reg as usize] = data,
                2 => {
                    self.irq_enabled = false;
                    self.pa12_counter = 0;
                    self.irq_pending = false;
                }
                3 => {
                    self.irq_enabled = true;
                    self.m2_prescaler = 3;
                }
                _ => {}
            }
        } else if address >= 0x8000 && address < 0xA000 && self.submapper == 0 {
            self.outer_bank = (address & 0xFF) as u8;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        let horizontal = if self.submapper == 1 {
            self.horizontal_mirroring
        } else {
            cart.nametable_horizontal_mirroring
        };
        if horizontal {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
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
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let data = if using_chr_ram && !chr_ram.is_empty() {
                let offset = self.chr_bank_offset(chr_ram.len(), address);
                chr_ram[offset]
            } else if !chr_rom.is_empty() {
                let offset = self.chr_bank_offset(chr_rom.len(), address);
                chr_rom[offset]
            } else {
                0
            };
            new_addr_bus |= data as u16;
        } else {
            let dummy = Cartridge {
                name: String::new(),
                prg_rom: Vec::new(),
                chr_rom: Vec::new(),
                memory_mapper: 91,
                sub_mapper: self.submapper,
                prg_size: 0,
                chr_size: 0,
                prg_size_minus_1: 0,
                chr_ram: Vec::new(),
                using_chr_ram: false,
                prg_ram: Vec::new(),
                has_battery: false,
                alternative_nametable_arrangement: false,
                prg_vram: Vec::new(),
                nametable_horizontal_mirroring,
                fds_disks: Vec::new(),
                trainer: Vec::new(),
                misc_rom: Vec::new(),
                mapper_chip: Box::new(crate::mapper::MapperNROM::new(
                    crate::mapper::NromConfig::default(),
                )),
                                mapper_cpu_cycle: 0,
                prg_rom_crc32: 0,
                chr_rom_crc32: 0,
                overall_crc32: 0,
                is_vs_system: false,
                tv_system: crate::region::TvSystem::Unknown,
            };
            let mirrored = self.mirror_nametable(&dummy, address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let offset = self.chr_bank_offset(cart.chr_ram.len(), address);
                cart.chr_ram[offset] = data;
            }
        } else if address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        if self.submapper == 0 {
            let a12 = (ppu_address_bus & 0x1000) != 0;
            if a12 && !self.previous_pa12 && self.irq_enabled {
                self.pa12_counter = self.pa12_counter.wrapping_add(1);
                if self.pa12_counter >= 64 {
                    self.irq_pending = true;
                }
            }
            self.previous_pa12 = a12;
        }
        self.irq_pending
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.submapper == 1 {
            for _ in 0..cycles {
                self.m2_prescaler = self.m2_prescaler.wrapping_add(1);
                if (self.m2_prescaler & 3) == 0 {
                    self.m2_counter -= 5;
                    if self.m2_counter <= 0 && self.irq_enabled {
                        self.irq_pending = true;
                    }
                }
            }
        }
        self.irq_pending
    }

    fn take_irq_ack(&mut self) -> bool {
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.prg);
        out.extend_from_slice(&self.chr);
        out.push(self.outer_bank);
        out.push(self.horizontal_mirroring as u8);
        out.push(self.irq_enabled as u8);
        out.push(self.pa12_counter);
        out.push(self.previous_pa12 as u8);
        out.push(self.m2_prescaler);
        out.extend_from_slice(&self.m2_counter.to_le_bytes());
        out
    }

    fn load_mapper_registers(
        &mut self,
        _cart: &mut Cartridge,
        state: &[u8],
        mut i: usize,
    ) -> usize {
        macro_rules! load_byte {
            ($field:expr) => {
                if i < state.len() { $field = state[i]; i += 1; }
            };
        }
        macro_rules! load_bool {
            ($field:expr) => {
                if i < state.len() { $field = state[i] != 0; i += 1; }
            };
        }
        if i + 1 < state.len() { self.prg.copy_from_slice(&state[i..i+2]); i += 2; }
        if i + 3 < state.len() { self.chr.copy_from_slice(&state[i..i+4]); i += 4; }
        load_byte!(self.outer_bank);
        load_bool!(self.horizontal_mirroring);
        load_bool!(self.irq_enabled);
        load_byte!(self.pa12_counter);
        load_bool!(self.previous_pa12);
        load_byte!(self.m2_prescaler);
        if i + 1 < state.len() {
            self.m2_counter = i16::from_le_bytes([state[i], state[i+1]]);
            i += 2;
        }
        i
    }
}
