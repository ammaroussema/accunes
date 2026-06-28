use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mapper90Variant {
    Mapper35,
    Mapper90,
    Mapper209,
    Mapper211,
    Mapper281,
    Mapper282,
    Mapper295,
    Mapper358,
    Mapper386,
    Mapper387,
    Mapper388,
    Mapper397,
}

pub struct Mapper90 {
    variant: Mapper90Variant,
    mode: u8,
    ciram_config: u8,
    vram_config: u8,
    outer_bank: u8,
    irq_control: u8,
    irq_enabled: bool,
    irq_prescaler: u8,
    irq_counter: u8,
    irq_xor: u8,
    last_a12: bool,
    irq_pending: bool,
    prg: [u8; 4],
    chr: [u16; 8],
    nt: [u16; 4],
    latch: [u8; 2],
    mul1: u8,
    mul2: u8,
    adder: u8,
    test: u8,
    chr_rom_len: usize,
    chr_ram_len: usize,
    dip_switches: u8,
}

impl Mapper90 {
    pub fn new(variant: Mapper90Variant) -> Self {
        Self {
            variant,
            mode: 0,
            ciram_config: 0,
            vram_config: 0,
            outer_bank: 0,
            irq_control: 0,
            irq_enabled: false,
            irq_prescaler: 0,
            irq_counter: 0,
            irq_xor: 0,
            last_a12: false,
            irq_pending: false,
            prg: [0; 4],
            chr: [0; 8],
            nt: [0; 4],
            latch: [0, 4],
            mul1: 0,
            mul2: 0,
            adder: 0,
            test: 0,
            chr_rom_len: 0,
            chr_ram_len: 0,
            dip_switches: 0,
        }
    }

    fn reverse_bits(val: u8) -> u8 {
        (val << 6) & 0x40
            | (val << 4) & 0x20
            | (val << 2) & 0x10
            | (val) & 0x08
            | (val >> 2) & 0x04
            | (val >> 4) & 0x02
            | (val >> 6) & 0x01
    }

    fn get_prg_mask_and_offset(&self) -> (usize, usize) {
        let ob = self.outer_bank as usize;
        match self.variant {
            Mapper90Variant::Mapper35
            | Mapper90Variant::Mapper90
            | Mapper90Variant::Mapper209
            | Mapper90Variant::Mapper211 => (0x3F, (ob << 5) & !0x3F),
            Mapper90Variant::Mapper281 => (0x1F, ob << 5),
            Mapper90Variant::Mapper282
            | Mapper90Variant::Mapper358
            | Mapper90Variant::Mapper397 => (0x1F, (ob << 4) & !0x1F),
            Mapper90Variant::Mapper295 => (0x0F, ob << 4),
            Mapper90Variant::Mapper386 => (0x1F, (ob << 4) & 0x20 | (ob << 3) & 0x40),
            Mapper90Variant::Mapper387 => (0x0F, (ob << 3) & 0x10 | (ob << 2) & 0x20),
            Mapper90Variant::Mapper388 => (0x1F, (ob << 3) & 0x60),
        }
    }

    fn get_chr_mask_and_offset(&self) -> (usize, usize) {
        let ob = self.outer_bank as usize;
        match self.variant {
            Mapper90Variant::Mapper35
            | Mapper90Variant::Mapper90
            | Mapper90Variant::Mapper209
            | Mapper90Variant::Mapper211 => {
                if (self.outer_bank & 0x20) != 0 {
                    (0x1FF, (ob << 6) & 0x600)
                } else {
                    (0x0FF, ((ob << 8) & 0x100) | ((ob << 6) & 0x600))
                }
            }
            Mapper90Variant::Mapper281 => (0xFF, ob << 8),
            Mapper90Variant::Mapper282 => {
                if (self.outer_bank & 0x20) != 0 {
                    (0x1FF, (ob << 6) & 0x600)
                } else {
                    (0x0FF, ((ob << 8) & 0x100) | ((ob << 6) & 0x600))
                }
            }
            Mapper90Variant::Mapper295
            | Mapper90Variant::Mapper397 => (0x7F, ob << 7),
            Mapper90Variant::Mapper358
            | Mapper90Variant::Mapper386
            | Mapper90Variant::Mapper387 => {
                if (self.outer_bank & 0x20) != 0 {
                    (0x1FF, (ob << 7) & 0x600)
                } else {
                    (0x0FF, ((ob << 8) & 0x100) | ((ob << 7) & 0x600))
                }
            }
            Mapper90Variant::Mapper388 => {
                if (self.outer_bank & 0x20) != 0 {
                    (0x1FF, (ob << 8) & 0x200)
                } else {
                    (0x0FF, ((ob << 8) & 0x100) | ((ob << 8) & 0x200))
                }
            }
        }
    }

    fn prg_bank_offset(&self, cart: &Cartridge, address: u16) -> usize {
        let prg_len = cart.prg_rom.len();
        if prg_len == 0 {
            return 0;
        }
        let (and_mask, or_offset) = self.get_prg_mask_and_offset();
        let prg_mode = self.mode & 0x03;
        let switchable_last = (self.mode & 0x04) != 0;
        let prg3 = if switchable_last { self.prg[3] } else { 0xFF };
        let bank = match prg_mode {
            0 => {
                let base = (prg3 as usize & (and_mask >> 2)) | (or_offset >> 2);
                base * 4 + ((address as usize - 0x8000) / 0x2000)
            }
            1 => {
                if address < 0xC000 {
                    let base = (self.prg[1] as usize & (and_mask >> 1)) | (or_offset >> 1);
                    base * 2 + ((address as usize - 0x8000) / 0x2000)
                } else {
                    let base = (prg3 as usize & (and_mask >> 1)) | (or_offset >> 1);
                    base * 2 + ((address as usize - 0xC000) / 0x2000)
                }
            }
            2 => {
                let sub_bank = match address & 0xE000 {
                    0x8000 => self.prg[0],
                    0xA000 => self.prg[1],
                    0xC000 => self.prg[2],
                    0xE000 => prg3,
                    _ => 0,
                };
                (sub_bank as usize & and_mask) | or_offset
            }
            3 => {
                let sub_bank = match address & 0xE000 {
                    0x8000 => Self::reverse_bits(self.prg[0]),
                    0xA000 => Self::reverse_bits(self.prg[1]),
                    0xC000 => Self::reverse_bits(self.prg[2]),
                    0xE000 => Self::reverse_bits(prg3),
                    _ => 0,
                };
                (sub_bank as usize & and_mask) | or_offset
            }
            _ => 0,
        };
        (bank * 0x2000 + (address as usize & 0x1FFF)) % prg_len
    }

    fn prg_6000_offset(&self, cart: &Cartridge) -> usize {
        let prg_len = cart.prg_rom.len();
        if prg_len == 0 {
            return 0;
        }
        let (and_mask, or_offset) = self.get_prg_mask_and_offset();
        let prg_mode = self.mode & 0x03;
        let prg3 = if (self.mode & 0x04) != 0 { self.prg[3] } else { 0xFF };
        let raw_bank = match prg_mode {
            0 => (prg3 << 2) | 3,
            1 => (prg3 << 1) | 1,
            2 => prg3,
            3 => Self::reverse_bits(prg3),
            _ => 0,
        };
        let bank = (raw_bank as usize & and_mask) | or_offset;
        (bank * 0x2000) % prg_len
    }

    fn chr_bank_offset(&self, address: u16, chr_len: usize) -> usize {
        if chr_len == 0 {
            return 0;
        }
        let (and_mask, or_offset) = self.get_chr_mask_and_offset();
        let chr_mode = (self.mode >> 3) & 0x03;
        let page = address / 0x400;
        let bank = match chr_mode {
            0 => {
                let base = (self.chr[0] as usize & (and_mask >> 3)) | (or_offset >> 3);
                base * 8 + page as usize
            }
            1 => {
                let latch_idx = (page / 4) as usize;
                let base = (self.chr[self.latch[latch_idx] as usize] as usize & (and_mask >> 2)) | (or_offset >> 2);
                base * 4 + (page as usize & 3)
            }
            2 => {
                let base = (self.chr[((page / 2) * 2) as usize] as usize & (and_mask >> 1)) | (or_offset >> 1);
                base * 2 + (page as usize & 1)
            }
            3 => {
                (self.chr[page as usize] as usize & and_mask) | or_offset
            }
            _ => 0,
        };
        (bank * 0x400 + (address as usize & 0x3FF)) % chr_len
    }

    fn clock_irq(&mut self) {
        let direction = self.irq_control >> 6;
        let small_prescaler = (self.irq_control & 0x04) != 0;
        let not_counting = (self.irq_control & 0x08) != 0;
        let mask = if small_prescaler { 0x07 } else { 0xFF };
        if self.irq_enabled {
            match direction {
                1 => {
                    self.irq_prescaler = (self.irq_prescaler & !mask) | ((self.irq_prescaler.wrapping_add(1)) & mask);
                    if (self.irq_prescaler & mask) == 0x00 {
                        if !not_counting {
                            self.irq_counter = self.irq_counter.wrapping_add(1);
                        }
                        if self.irq_counter == 0x00 {
                            self.irq_pending = true;
                        }
                    }
                }
                2 => {
                    self.irq_prescaler = (self.irq_prescaler & !mask) | ((self.irq_prescaler.wrapping_sub(1)) & mask);
                    if (self.irq_prescaler & mask) == mask {
                        if !not_counting {
                            self.irq_counter = self.irq_counter.wrapping_sub(1);
                        }
                        if self.irq_counter == 0xFF {
                            self.irq_pending = true;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

impl Mapper for Mapper90 {
    fn reset(&mut self) {
        self.mode = 0x00;
        self.ciram_config = 0x00;
        self.vram_config = 0x00;
        self.outer_bank = 0x00;
        self.irq_enabled = false;
        self.irq_control = 0x00;
        self.irq_prescaler = 0x00;
        self.irq_counter = 0x00;
        self.irq_xor = 0x00;
        self.last_a12 = false;
        self.mul1 = 0x00;
        self.mul2 = 0x00;
        self.adder = 0x00;
        self.test = 0x00;
        self.prg = [0; 4];
        self.chr = [0; 8];
        self.nt = [0; 4];
        self.latch = [0, 4];
        self.irq_pending = false;
        self.dip_switches = 0;
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            FetchResult {
                data: cart.prg_rom[self.prg_bank_offset(cart, address)],
                driven: true,
            }
        } else if address >= 0x6000 && address < 0x8000 {
            let rom_at_6000 = (self.mode & 0x80) != 0;
            if rom_at_6000 {
                FetchResult {
                    data: cart.prg_rom[self.prg_6000_offset(cart) + (address as usize & 0x1FFF)],
                    driven: true,
                }
            } else if !cart.prg_ram.is_empty() {
                let mask = cart.prg_ram.len() - 1;
                FetchResult {
                    data: cart.prg_ram[(address as usize - 0x6000) & mask],
                    driven: true,
                }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else if address >= 0x5000 && address < 0x6000 {
            let res = if (address & 0x800) != 0 {
                match address & 3 {
                    0 => (self.mul1 as u16 * self.mul2 as u16) as u8,
                    1 => ((self.mul1 as u16 * self.mul2 as u16) >> 8) as u8,
                    2 => self.adder,
                    3 => self.test,
                    _ => 0,
                }
            } else if (address & 0x3FF) == 0 {
                self.dip_switches & 0xC0
            } else {
                0
            };
            FetchResult { data: res, driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x6000 {
            match address & 3 {
                0 => self.mul1 = data,
                1 => self.mul2 = data,
                2 => self.adder = self.adder.wrapping_add(data),
                3 => {
                    self.test = data;
                    self.adder = 0;
                }
                _ => {}
            }
        } else if address >= 0x6000 && address < 0x8000 {
            let rom_at_6000 = (self.mode & 0x80) != 0;
            if !rom_at_6000 && !cart.prg_ram.is_empty() {
                let mask = cart.prg_ram.len() - 1;
                cart.prg_ram[(address as usize - 0x6000) & mask] = data;
            }
        } else if address >= 0x8000 && address < 0x9000 {
            if (address & 0x800) == 0 {
                self.prg[(address & 3) as usize] = data;
            }
        } else if address >= 0x9000 && address < 0xA000 {
            if (address & 0x800) == 0 {
                let reg = (address & 7) as usize;
                self.chr[reg] = (self.chr[reg] & 0xFF00) | (data as u16);
            }
        } else if address >= 0xA000 && address < 0xB000 {
            if (address & 0x800) == 0 {
                let reg = (address & 7) as usize;
                self.chr[reg] = (self.chr[reg] & 0x00FF) | ((data as u16) << 8);
            }
        } else if address >= 0xB000 && address < 0xC000 {
            if (address & 0x800) == 0 {
                let reg = (address & 3) as usize;
                if (address & 4) == 0 {
                    self.nt[reg] = (self.nt[reg] & 0xFF00) | (data as u16);
                } else {
                    self.nt[reg] = (self.nt[reg] & 0x00FF) | ((data as u16) << 8);
                }
            }
        } else if address >= 0xC000 && address < 0xD000 {
            match address & 7 {
                0 => {
                    self.irq_enabled = (data & 1) != 0;
                    if !self.irq_enabled {
                        self.irq_prescaler = 0;
                        self.irq_pending = false;
                    }
                }
                1 => self.irq_control = data,
                2 => {
                    self.irq_enabled = false;
                    self.irq_prescaler = 0;
                    self.irq_pending = false;
                }
                3 => self.irq_enabled = true,
                4 => self.irq_prescaler = data ^ self.irq_xor,
                5 => self.irq_counter = data ^ self.irq_xor,
                6 => self.irq_xor = data,
                _ => {}
            }
        } else if address >= 0xD000 && address < 0xE000 {
            if (address & 0x800) == 0 {
                let allow_extended_mirroring = !matches!(self.variant,
                    Mapper90Variant::Mapper90 | Mapper90Variant::Mapper388
                );
                match address & 3 {
                    0 => {
                        self.mode = data;
                        if !allow_extended_mirroring {
                            self.mode &= !0x20;
                        }
                    }
                    1 => {
                        self.ciram_config = data;
                        if !allow_extended_mirroring {
                            self.ciram_config &= !0x08;
                        }
                    }
                    2 => self.vram_config = data,
                    3 => self.outer_bank = data,
                    _ => {}
                }
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let vrom_enabled = (self.mode & 0x20) != 0;
        let extended_mirroring = (self.ciram_config & 0x08) != 0;
        let mirroring = self.ciram_config & 0x03;
        if vrom_enabled || extended_mirroring {
            let page = ((address - 0x2000) / 0x400) as usize & 3;
            if (self.nt[page] & 1) != 0 {
                0x2000 | 0x400 | (address & 0x3FF)
            } else {
                0x2000 | (address & 0x3FF)
            }
        } else {
            match mirroring {
                0 => address & 0x37FF,                                        
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1),          
                2 => 0x2000 | (address & 0x3FF),                              
                3 => 0x2000 | 0x400 | (address & 0x3FF),                     
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
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        let irq_src = self.irq_control & 0x03;
        if irq_src == 2 {
            self.clock_irq();
        }
        self.chr_rom_len = chr_rom.len();
        self.chr_ram_len = chr_ram.len();
        let mmc4_mode = (self.outer_bank & 0x80) != 0;
        if mmc4_mode && !ciram {
            let bank = (address / 0x400) as usize;
            if (bank & 3) == 3 {
                let latch_number = (bank & 4) >> 2;
                match address & 0x3F8 {
                    0x3D8 => {
                        self.latch[latch_number] = (bank as u8 & 4) | 0;
                    }
                    0x3E8 => {
                        self.latch[latch_number] = (bank as u8 & 4) | 2;
                    }
                    _ => {}
                }
            }
        }
        if !ciram {
            let len = if using_chr_ram { chr_ram.len() } else { chr_rom.len() };
            let offset = self.chr_bank_offset(address, len);
            let data = if using_chr_ram { chr_ram[offset] } else { chr_rom[offset] };
            new_addr_bus |= data as u16;
        } else {
            let vrom_enabled = (self.mode & 0x20) != 0;
            let vrom_bit = (self.vram_config & 0x80) != 0;
            let vrom_everywhere = (self.mode & 0x40) != 0;
            if vrom_enabled {
                let page = ((address - 0x2000) / 0x400) as usize & 3;
                let vrom_here = ((self.nt[page] & 0x80) != 0) ^ vrom_bit || vrom_everywhere;
                if vrom_here {
                    let (and_mask, or_offset) = self.get_chr_mask_and_offset();
                    let bank = (self.nt[page] as usize & and_mask) | or_offset;
                    let chr_len = if using_chr_ram { chr_ram.len() } else { chr_rom.len() };
                    if chr_len > 0 {
                        let offset = (bank * 0x400 + (address as usize & 0x3FF)) % chr_len;
                        let data = if using_chr_ram { chr_ram[offset] } else { chr_rom[offset] };
                        new_addr_bus |= data as u16;
                    }
                } else {
                    let ciram_page = (self.nt[page] & 1) as u16;
                    let ciram_addr = ciram_page * 0x400 + (address & 0x3FF);
                    let data = vram[(ciram_addr & 0x7FF) as usize];
                    new_addr_bus |= data as u16;
                }
            } else {
                let extended_mirroring = (self.ciram_config & 0x08) != 0;
                let mirrored = if extended_mirroring {
                    let page = ((address - 0x2000) / 0x400) as usize & 3;
                    let ciram_page = (self.nt[page] & 1) as u16;
                    ciram_page * 0x400 + (address & 0x3FF)
                } else {
                    let mirroring = self.ciram_config & 0x03;
                    match mirroring {
                        0 => (address & 0x37FF) - 0x2000,
                        1 => ((address & 0x33FF) | ((address & 0x0800) >> 1)) - 0x2000,
                        2 => address & 0x3FF,
                        3 => 0x400 | (address & 0x3FF),
                        _ => (address - 0x2000) & 0x7FF,
                    }
                };
                let data = vram[(mirrored & 0x7FF) as usize];
                new_addr_bus |= data as u16;
            }
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        let chr_writable = (self.vram_config & 0x40) != 0;
        if address < 0x2000 {
            if chr_writable && cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                let offset = self.chr_bank_offset(address, len);
                cart.chr_ram[offset] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        let irq_src = self.irq_control & 0x03;
        if irq_src == 0 {
            for _ in 0..cycles {
                self.clock_irq();
            }
        }
        self.irq_pending
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        let irq_src = self.irq_control & 0x03;
        let is_a12 = (ppu_address_bus & 0x1000) != 0;
        if is_a12 && !self.last_a12 && irq_src == 1 {
            self.clock_irq();
        }
        self.last_a12 = is_a12;
        self.irq_pending
    }

    fn take_irq_ack(&mut self) -> bool {
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = vec![
            self.mode, self.ciram_config, self.vram_config, self.outer_bank,
            self.mul1, self.mul2, self.adder, self.test,
            self.latch[0], self.latch[1],
        ];
        state.extend_from_slice(&self.prg);
        for &c in &self.chr {
            state.extend_from_slice(&c.to_le_bytes());
        }
        for &n in &self.nt {
            state.extend_from_slice(&n.to_le_bytes());
        }
        state.push(self.irq_control);
        state.push(self.irq_prescaler);
        state.push(self.irq_counter);
        state.push(self.irq_xor);
        state.push(self.last_a12 as u8);
        state.push(self.irq_enabled as u8);
        state.push(self.dip_switches);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        let expected_size = 10 + 4 + 16 + 8 + 6;
        if start + expected_size <= state.len() {
            self.mode = state[start];
            self.ciram_config = state[start + 1];
            self.vram_config = state[start + 2];
            self.outer_bank = state[start + 3];
            self.mul1 = state[start + 4];
            self.mul2 = state[start + 5];
            self.adder = state[start + 6];
            self.test = state[start + 7];
            self.latch[0] = state[start + 8];
            self.latch[1] = state[start + 9];
            start += 10;
            self.prg.copy_from_slice(&state[start..start + 4]);
            start += 4;
            for i in 0..8 {
                let mut bytes = [0u8; 2];
                bytes.copy_from_slice(&state[start..start + 2]);
                self.chr[i] = u16::from_le_bytes(bytes);
                start += 2;
            }
            for i in 0..4 {
                let mut bytes = [0u8; 2];
                bytes.copy_from_slice(&state[start..start + 2]);
                self.nt[i] = u16::from_le_bytes(bytes);
                start += 2;
            }
            self.irq_control = state[start];
            self.irq_prescaler = state[start + 1];
            self.irq_counter = state[start + 2];
            self.irq_xor = state[start + 3];
            self.last_a12 = state[start + 4] != 0;
            self.irq_enabled = state[start + 5] != 0;
            start += 6;
            self.dip_switches = if start < state.len() { state[start] } else { 0 };
            if start < state.len() { start += 1; }
        }
        start
    }
}
