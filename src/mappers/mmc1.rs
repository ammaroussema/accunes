use crate::cartridge::Cartridge;
use crate::crc::crc32;
use crate::mapper::{FetchResult, Mapper};
const VS_FRAME_CYCLES: u64 = 29780;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mmc1Variant {
    Standard,
    Mmc1A,
    Nwc,
    Kaiser171,
}

#[derive(Clone, Debug)]
pub struct Mmc1Config {
    pub variant: Mmc1Variant,
    pub serom: bool,
    pub wram_size: usize,
    #[allow(dead_code)]
    pub battery_wram_size: usize,
    pub snrom: bool,
}

impl Mmc1Config {
    pub fn for_ines(
        header: &[u8],
        rom: &[u8],
        mapper_id: u16,
        sub_mapper: u8,
        prg_size: u8,
        using_chr_ram: bool,
        has_battery: bool,
    ) -> Self {
        let variant = match mapper_id {
            105 => Mmc1Variant::Nwc,
            155 => Mmc1Variant::Mmc1A,
            171 => Mmc1Variant::Kaiser171,
            1 if sub_mapper == 7 => Mmc1Variant::Kaiser171,
            _ => Mmc1Variant::Standard,
        };
        let serom = sub_mapper == 5 || (mapper_id == 1 && prg_size == 2);
        let (wram_kb, battery_kb) = if matches!(variant, Mmc1Variant::Kaiser171) {
            (0, 0)
        } else {
            detect_wram_sizes(header, rom, has_battery, using_chr_ram)
        };
        let wram_size = wram_kb * 1024;
        let battery_wram_size = battery_kb * 1024;
        let snrom =
            using_chr_ram && wram_size > 0 && mapper_id != 105 && mapper_id != 171;
        Self {
            variant,
            serom,
            wram_size,
            battery_wram_size,
            snrom,
        }
    }
}

fn is_nes20(header: &[u8]) -> bool {
    header.len() >= 16 && (header[7] & 0x0C) == 0x08
}

fn detect_wram_sizes(
    header: &[u8],
    rom: &[u8],
    has_battery: bool,
    using_chr_ram: bool,
) -> (usize, usize) {
    if is_nes20(header) && header.len() >= 16 {
        let prg_ram_shift = header[9] & 0x0F;
        let mut total_kb = if prg_ram_shift == 0 {
            0
        } else {
            (64usize << prg_ram_shift) / 1024
        };
        if total_kb > 0 && total_kb < 8 {
            total_kb = 8;
        }
        if total_kb > 32 {
            total_kb = 32;
        }
        let mut battery_kb = if has_battery { total_kb } else { 0 };
        if battery_kb > total_kb {
            battery_kb = total_kb;
        }
        if total_kb > 0 {
            return (total_kb, battery_kb);
        }
    }
    let crc = crc32(rom);
    match crc {
        0xc6182024 | 0xabbf7217 | 0xccf35c02 | 0x2225c20f | 0xfb69743a | 0x4642dda6
        | 0x3f7ad415 | 0x2b11e0b0 => (16, 8),
        0xb8747abf | 0xc3de7c69 | 0xc9556b36 => (32, 32),
        _ => {
            if using_chr_ram && !has_battery {
                (0, 0)
            } else {
                (8, if has_battery { 8 } else { 0 })
            }
        }
    }
}

fn mirror_from_ines_header(cart: &Cartridge, address: u16) -> u16 {
    if cart.nametable_horizontal_mirroring {
        (address & 0x33FF) | ((address & 0x0800) >> 1)
    } else {
        address & 0x37FF
    }
}

pub struct Mmc1Core {
    pub shift_register: u8,
    pub control: u8,
    pub chr0: u8,
    pub chr1: u8,
    pub prg: u8,
    pub config: Mmc1Config,
    last_write_cycle: i64,
    last_reset_cycle: i64,
    pub nwc_rec: u8,
    nwc_irq_count: u32,
}

impl Mmc1Core {
    pub fn new(config: Mmc1Config) -> Self {
        let mut core = Self {
            shift_register: 0x10,
            control: 0x1F,
            chr0: 0,
            chr1: 0,
            prg: 0,
            config,
            last_write_cycle: -2,
            last_reset_cycle: -2,
            nwc_rec: 0,
            nwc_irq_count: 0,
        };
        if core.config.variant == Mmc1Variant::Nwc {
            core.nwc_rec = 0;
        }
        core
    }

    pub fn reset(&mut self) {
        self.shift_register = 0x10;
        self.control = 0x1F;
        self.chr0 = 0;
        self.chr1 = 0;
        self.prg = 0;
        self.last_write_cycle = -2;
        self.last_reset_cycle = -2;
        self.nwc_irq_count = 0;
        if self.config.variant == Mmc1Variant::Nwc {
            self.nwc_rec = 0;
        }
    }

    fn prg_outer_bank(&self) -> usize {
        if (self.chr0 & 0x10) != 0 {
            16
        } else {
            0
        }
    }

    fn wram_enabled(&self) -> bool {
        match self.config.variant {
            Mmc1Variant::Mmc1A => true,
            Mmc1Variant::Kaiser171 => false,
            Mmc1Variant::Standard | Mmc1Variant::Nwc => {
                if (self.prg & 0x10) != 0 {
                    return false;
                }
                if self.config.snrom && (self.chr0 & 0x10) != 0 {
                    return false;
                }
                true
            }
        }
    }

    fn wram_offset(&self, address: u16) -> Option<usize> {
        if self.config.wram_size == 0 {
            return None;
        }
        let addr_lo = (address - 0x6000) as usize;
        if addr_lo >= self.config.wram_size {
            return None;
        }
        if self.config.wram_size <= 0x2000 {
            return Some(addr_lo);
        }
        let bank = if self.config.wram_size > 0x4000 {
            (self.chr0 >> 2) & 3
        } else {
            (self.chr0 >> 3) & 1
        };
        Some((bank as usize) * 0x2000 + (addr_lo & 0x1FFF))
    }

    pub fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if self.config.variant == Mmc1Variant::Kaiser171 {
            return mirror_from_ines_header(cart, address);
        }
        match self.control & 0x03 {
            0 => address & 0x23FF,
            1 => (address & 0x23FF) | 0x0400,
            2 => address & 0x37FF,
            3 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            _ => unreachable!(),
        }
    }

    fn chr_bank_and_mask(&self, address: u16) -> (usize, usize) {
        let chr_mode = (self.control >> 4) & 0x01;
        if chr_mode != 0 {
            let bank = if address < 0x1000 {
                self.chr0 as usize
            } else {
                self.chr1 as usize
            };
            (bank, 0x0FFF)
        } else {
            ((self.chr0 & 0x1E) as usize, 0x1FFF)
        }
    }

    pub fn chr_offset(&self, address: u16, chr_len: usize) -> usize {
        let (mut bank, mask) = self.chr_bank_and_mask(address);
        if chr_len > 0 && chr_len <= 0x2000 {
            bank &= 1;
        }
        let offset = bank * 0x1000 + (address as usize & mask);
        if chr_len == 0 {
            offset
        } else {
            offset % chr_len
        }
    }

    fn num_prg_banks(cart: &Cartridge) -> usize {
        (cart.prg_rom.len() / 0x4000).max(1)
    }

    fn prg_rom_offset_nwc(&self, _cart: &Cartridge, address: u16) -> usize {
        if (self.nwc_rec & 0x08) != 0 {
            let bank = if address >= 0xC000 {
                8 | 0x07
            } else {
                8 | (self.prg & 0x07) as usize
            };
            bank * 0x4000 + (address as usize & 0x3FFF)
        } else {
            let bank32 = ((self.nwc_rec >> 1) & 3) as usize;
            bank32 * 0x8000 + (address as usize & 0x7FFF)
        }
    }

    fn prg_rom_offset_standard(&self, cart: &Cartridge, address: u16) -> usize {
        if self.config.serom {
            return (address as usize - 0x8000) & (cart.prg_rom.len() - 1);
        }
        let offs = self.prg_outer_bank();
        let prg_reg = (self.prg & 0x0F) as usize;
        let num_banks = Self::num_prg_banks(cart);
        let mode = (self.control >> 2) & 0x03;
        match mode {
            0 | 1 => {
                let bank32 = ((prg_reg & 0x0E) + offs).min(num_banks.saturating_sub(2));
                bank32 * 0x8000 + (address as usize & 0x7FFF)
            }
            2 => {
                if address >= 0xC000 {
                    let bank = (prg_reg + offs).min(num_banks - 1);
                    bank * 0x4000 + (address as usize & 0x3FFF)
                } else {
                    let bank = offs.min(num_banks - 1);
                    bank * 0x4000 + (address as usize & 0x3FFF)
                }
            }
            3 => {
                if address >= 0xC000 {
                    let fixed = (0x0F + offs).min(num_banks - 1);
                    fixed * 0x4000 + (address as usize & 0x3FFF)
                } else {
                    let bank = (prg_reg + offs).min(num_banks - 1);
                    bank * 0x4000 + (address as usize & 0x3FFF)
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn prg_rom_offset(&self, cart: &Cartridge, address: u16) -> usize {
        let offset = if self.config.variant == Mmc1Variant::Nwc {
            self.prg_rom_offset_nwc(cart, address)
        } else {
            self.prg_rom_offset_standard(cart, address)
        };
        offset & (cart.prg_rom.len() - 1)
    }

    pub fn fetch_prg_byte(&self, cart: &Cartridge, address: u16) -> Option<u8> {
        if address >= 0x8000 {
            return Some(cart.prg_rom[self.prg_rom_offset(cart, address)]);
        }
        if address >= 0x6000 {
            if !self.wram_enabled() {
                return None;
            }
            let off = self.wram_offset(address)?;
            return Some(cart.prg_ram[off]);
        }
        None
    }

    pub fn store_prg_ram(&self, cart: &mut Cartridge, address: u16, data: u8) -> bool {
        if address < 0x6000 || address >= 0x8000 {
            return false;
        }
        if !self.wram_enabled() {
            return false;
        }
        if let Some(off) = self.wram_offset(address) {
            cart.prg_ram[off] = data;
            return true;
        }
        false
    }

    fn on_reg_write(&mut self, reg: u8, value: u8) {
        match reg {
            0 => self.control = value,
            1 => {
                self.chr0 = value;
                if self.config.variant == Mmc1Variant::Nwc {
                    if (value & 0x10) != 0 {
                        self.nwc_irq_count = 0;
                    }
                    self.nwc_rec = value;
                }
            }
            2 => self.chr1 = value,
            3 => self.prg = value,
            _ => {}
        }
    }

    pub fn write_register(
        &mut self,
        cart: &mut Cartridge,
        address: u16,
        data: u8,
        cpu_cycle: i64,
    ) {
        if address < 0x8000 {
            let _ = self.store_prg_ram(cart, address, data);
            return;
        }
        if (data & 0x80) != 0 {
            self.control |= 0x0C;
            self.shift_register = 0x10;
            self.last_reset_cycle = cpu_cycle;
            self.last_write_cycle = cpu_cycle;
            return;
        }
        if cpu_cycle >= 0 && self.last_reset_cycle >= 0 && cpu_cycle < self.last_reset_cycle + 2 {
            return;
        }
        if cpu_cycle >= 0 && self.last_write_cycle >= 0 && cpu_cycle == self.last_write_cycle + 1 {
            return;
        }
        let done = (self.shift_register & 1) != 0;
        self.shift_register >>= 1;
        self.shift_register |= (data & 1) << 4;
        self.last_write_cycle = cpu_cycle;
        if done {
            let reg = ((address >> 13) as u8).wrapping_sub(4);
            let value = self.shift_register;
            self.on_reg_write(reg, value);
            self.shift_register = 0x10;
        }
    }

    pub fn cpu_clock_irq(&mut self) -> bool {
        if self.config.variant != Mmc1Variant::Nwc {
            return false;
        }
        if (self.nwc_rec & 0x10) != 0 {
            return false;
        }
        self.nwc_irq_count = self.nwc_irq_count.wrapping_add(1);
        if (self.nwc_irq_count | (0x0E << 25)) >= 0x3e000000 {
            self.nwc_irq_count = 0;
            return true;
        }
        false
    }

    pub fn append_save_state(&self, state: &mut Vec<u8>) {
        state.push(self.shift_register);
        state.push(self.control);
        state.push(self.chr0);
        state.push(self.chr1);
        state.push(self.prg);
        state.extend_from_slice(&self.last_write_cycle.to_le_bytes());
        state.extend_from_slice(&self.last_reset_cycle.to_le_bytes());
        state.push(self.nwc_rec);
        state.extend_from_slice(&self.nwc_irq_count.to_le_bytes());
    }

    pub fn load_save_state(&mut self, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.shift_register = state[p];
        p += 1;
        self.control = state[p];
        p += 1;
        self.chr0 = state[p];
        p += 1;
        self.chr1 = state[p];
        p += 1;
        self.prg = state[p];
        p += 1;
        if p + 16 <= state.len() {
            self.last_write_cycle = i64::from_le_bytes(state[p..p + 8].try_into().unwrap());
            p += 8;
            self.last_reset_cycle = i64::from_le_bytes(state[p..p + 8].try_into().unwrap());
            p += 8;
            self.nwc_rec = state[p];
            p += 1;
            self.nwc_irq_count = u32::from_le_bytes(state[p..p + 4].try_into().unwrap());
            p += 4;
        }
        p
    }
}

pub struct MapperMMC1 {
    core: Mmc1Core,
    vsdip: u8,
    coinon: u8,
    coinon2: u8,
    service: u8,
    cycle_accum: u64,
}

impl MapperMMC1 {
    pub fn new(config: Mmc1Config) -> Self {
        Self {
            core: Mmc1Core::new(config),
            vsdip: 0,
            coinon: 0,
            coinon2: 0,
            service: 0,
            cycle_accum: 0,
        }
    }
}

fn mmc1_mirror_for_ppu(core: &Mmc1Core, ines_horizontal: bool, address: u16) -> u16 {
    if core.config.variant == Mmc1Variant::Kaiser171 {
        if ines_horizontal {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    } else {
        match core.control & 0x03 {
            0 => address & 0x23FF,
            1 => (address & 0x23FF) | 0x0400,
            2 => address & 0x37FF,
            3 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            _ => unreachable!(),
        }
    }
}

fn mmc1_fetch_ppu(
    core: &Mmc1Core,
    ines_horizontal: bool,
    chr_rom: &[u8],
    chr_ram: &[u8],
    using_chr_ram: bool,
    ppu_address_bus: u16,
    ppu_octal_latch: u8,
    vram: &[u8],
) -> (u8, u16) {
    let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
    let ciram = address >= 0x2000;
    let mut new_addr_bus = ppu_address_bus & 0xFF00;
    if !ciram {
        if using_chr_ram {
            let offset = core.chr_offset(address, chr_ram.len());
            new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
        } else {
            let offset = core.chr_offset(address, chr_rom.len());
            new_addr_bus |= chr_rom[offset] as u16;
        }
    } else {
        let mirrored = mmc1_mirror_for_ppu(core, ines_horizontal, address);
        let idx = (mirrored & 0x7FF) as usize;
        new_addr_bus |= vram[idx] as u16;
    }
    (new_addr_bus as u8, new_addr_bus)
}

impl Mapper for MapperMMC1 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if let Some(data) = self.core.fetch_prg_byte(cart, address) {
            FetchResult { data, driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        self.core
            .write_register(cart, address, data, cart.mapper_cpu_cycle);
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.core.mirror_nametable(cart, address)
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
        mmc1_fetch_ppu(
            &self.core,
            nametable_horizontal_mirroring,
            chr_rom,
            chr_ram,
            using_chr_ram,
            ppu_address_bus,
            ppu_octal_latch,
            vram,
        )
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let chr_len = cart.chr_ram.len();
            let offset = self.core.chr_offset(address, chr_len);
            cart.chr_ram[offset & (chr_len - 1)] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn adjust_controller_read(&self, address: u16, value: u8) -> u8 {
        if address & 0x1F == 0x16 {
            let mut vs = value & 0x01;
            if self.service > 0 { vs |= 0x04; }
            vs |= (self.vsdip & 0x03) << 3;
            if self.coinon > 0 { vs |= 0x20; }
            if self.coinon2 > 0 { vs |= 0x40; }
            vs
        } else if address & 0x1F == 0x17 {
            (value & 0x01) | (self.vsdip & 0xFC)
        } else {
            value
        }
    }

    fn insert_coin(&mut self, coin: u8) {
        match coin {
            0 => self.coinon = 6,
            1 => self.coinon2 = 6,
            _ => {}
        }
    }

    fn service_button(&mut self) {
        self.service = 6;
    }

    fn get_dip_switches(&self) -> u8 {
        self.vsdip
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.vsdip = value;
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.cycle_accum += _cycles as u64;
        if self.cycle_accum >= VS_FRAME_CYCLES {
            self.cycle_accum = 0;
            if self.coinon > 0 { self.coinon -= 1; }
            if self.coinon2 > 0 { self.coinon2 -= 1; }
            if self.service > 0 { self.service -= 1; }
        }
        self.core.cpu_clock_irq()
    }

    fn reset(&mut self) {
        self.core.reset();
        self.vsdip = 0;
        self.coinon = 0;
        self.coinon2 = 0;
        self.service = 0;
        self.cycle_accum = 0;
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        self.core.append_save_state(&mut state);
        state.push(self.vsdip);
        state.push(self.coinon);
        state.push(self.coinon2);
        state.push(self.service);
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
        p = self.core.load_save_state(state, p);
        self.vsdip = state.get(p).copied().unwrap_or(0); p += 1;
        self.coinon = state.get(p).copied().unwrap_or(0); p += 1;
        self.coinon2 = state.get(p).copied().unwrap_or(0); p += 1;
        self.service = state.get(p).copied().unwrap_or(0); p += 1;
        p
    }
}
#[allow(dead_code)]
pub type Mapper171 = MapperMMC1;
