use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc1::{Mmc1Config, Mmc1Core, Mmc1Variant, mmc1_mirror_for_ppu};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, mmc3_chr_bank};
use crate::mappers::vrc2_4::{Vrc2And4, VrcVariant};

pub struct Mapper351 {
    reg: [u8; 4],
    mmc3: MapperMMC3,
    mmc1_core: Mmc1Core,
    vrc4: Vrc2And4,
    combined_prg: Vec<u8>,
    combined_8k: usize,
    irq_enabled: bool,
    dip_switches: u8,
}

impl Mapper351 {
    fn mapper(&self) -> u8 { self.reg[0] & 3 }
    fn nrom(&self) -> bool { (self.reg[2] & 0x10) != 0 }
    fn prg128(&self) -> bool { (self.reg[2] & 0x04) != 0 }
    fn chrram(&self) -> bool { (self.reg[2] & 0x01) != 0 }
    fn chr8(&self) -> bool { (self.reg[2] & 0x40) != 0 }
    fn chr128(&self) -> bool { (self.reg[2] & 0x20) != 0 }
    fn chr32(&self) -> bool { (self.reg[2] & 0x10) != 0 && !self.chr128() }

    fn prg_and(&self) -> u8 { if self.prg128() { 0x0F } else { 0x1F } }
    fn prg_or(&self) -> u8 { (self.reg[1] >> 1) & !self.prg_and() }
    fn chr_and(&self) -> u8 {
        if self.chr32() { 0x1F } else if self.chr128() { 0x7F } else { 0xFF }
    }
    fn chr_or(&self) -> u8 { (self.reg[0] << 1) & !self.chr_and() }

    fn get_last_bank(&self) -> usize { self.combined_8k.saturating_sub(1) }
    fn get_second_last_bank(&self) -> usize { self.combined_8k.saturating_sub(2) }

    fn prg_offset(&self, address: u16) -> usize {
        let page = (address as usize - 0x8000) / 0x2000;
        let and = self.prg_and() as usize;
        let or = self.prg_or() as usize;
        if self.nrom() {
            if (self.reg[2] & 0x08) != 0 {
                let bank = (self.reg[1] >> 1) as usize;
                bank.min(self.combined_8k.saturating_sub(1)) * 0x2000 + (address as usize & 0x1FFF)
            } else {
                let bank16 = (self.reg[1] >> 2) as usize;
                let (b0, b1) = if self.prg128() {
                    (bank16 & !1, bank16)
                } else {
                    (bank16, bank16 | 1)
                };
                if page < 2 {
                    b0 * 0x2000 + (address as usize & 0x1FFF)
                } else {
                    b1 * 0x2000 + (address as usize & 0x1FFF)
                }
            }
        } else {
            match self.mapper() {
                2 => {
                    let mode = (self.mmc1_core.control >> 2) & 3;
                    let prg_reg = (self.mmc1_core.prg & 0x0F) as usize;
                    let num_banks_16k = self.combined_8k / 2;
                    let outer = (self.mmc1_core.chr0 as usize >> 4) & 1;
                    let mmc1_and = and >> 1;
                    let mmc1_or = or >> 1;
                    let reg_masked = (prg_reg & mmc1_and) | mmc1_or;
                    match mode {
                        0 | 1 => {
                            let bank32 = ((reg_masked & 0x0E) + outer * 16).min(num_banks_16k.saturating_sub(2));
                            bank32 * 0x8000 + (address as usize & 0x7FFF)
                        }
                        2 => {
                            if address >= 0xC000 {
                                (reg_masked + outer * 16).min(num_banks_16k.saturating_sub(1)) * 0x4000 + (address as usize & 0x3FFF)
                            } else {
                                outer.min(num_banks_16k.saturating_sub(1)) * 0x4000 + (address as usize & 0x3FFF)
                            }
                        }
                        3 => {
                            if address >= 0xC000 {
                                (0x0F + outer * 16).min(num_banks_16k.saturating_sub(1)) * 0x4000 + (address as usize & 0x3FFF)
                            } else {
                                (reg_masked + outer * 16).min(num_banks_16k.saturating_sub(1)) * 0x4000 + (address as usize & 0x3FFF)
                            }
                        }
                        _ => 0,
                    }
                }
                _ => {
                    let bank_8c = (self.mmc3.bank_8c as usize & and) | or;
                    let bank_a = (self.mmc3.bank_a as usize & and) | or;
                    let mode = self.mmc3.r8000 & 0x40;
                    let last = self.get_last_bank();
                    let second_last = self.get_second_last_bank();
                    let offset = address as usize & 0x1FFF;
                    match page {
                        0 => {
                            if mode == 0 { bank_8c * 0x2000 + offset } else { second_last * 0x2000 + offset }
                        }
                        1 => { bank_a * 0x2000 + offset }
                        2 => {
                            if mode == 0 { second_last * 0x2000 + offset } else { bank_8c * 0x2000 + offset }
                        }
                        3 => { last * 0x2000 + offset }
                        _ => 0,
                    }
                }
            }
        }
    }

    fn chr_bank_masked(&self, address: u16) -> usize {
        let and = self.chr_and() as usize;
        let or = self.chr_or() as usize;
        let b = mmc3_chr_bank(
            self.mmc3.r8000, self.mmc3.chr_2k0, self.mmc3.chr_2k8,
            self.mmc3.chr_1k0, self.mmc3.chr_1k4, self.mmc3.chr_1k8, self.mmc3.chr_1kc,
            address,
        ) as usize;
        (b & and) | or
    }

    fn vrc4_chr_offset(&self, address: u16) -> usize {
        let and = self.chr_and() as usize;
        let or = self.chr_or() as usize;
        let bank_idx = (address >> 10) as usize & 7;
        let raw = self.vrc4.chr_bank_raw(bank_idx) as usize;
        let bank = (raw & and) | or;
        bank * 0x400 + (address as usize & 0x3FF)
    }

    fn mmc1_chr_offset(&self, address: u16) -> usize {
        let chr_and = self.chr_and() as usize;
        let chr_or = self.chr_or() as usize;
        let chr_mode = (self.mmc1_core.control >> 4) & 1;
        if chr_mode != 0 {
            let bank = if address < 0x1000 {
                ((self.mmc1_core.chr0 as usize) & chr_and) | chr_or
            } else {
                ((self.mmc1_core.chr1 as usize) & chr_and) | chr_or
            };
            bank * 0x1000 + (address as usize & 0xFFF)
        } else {
            let bank = ((self.mmc1_core.chr0 as usize) & 0xFE) & chr_and | chr_or;
            bank * 0x1000 + (address as usize & 0x1FFF)
        }
    }

    fn apply_mode(&mut self, _cart: &mut Cartridge) {
        self.irq_enabled = self.mapper() != 2;
    }

    fn sync(&mut self) {
    }
}

impl Mapper351 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let mmc1_config = Mmc1Config {
            variant: Mmc1Variant::Mmc1A,
            serom: false,
            wram_size: 0,
            battery_wram_size: 0,
            snrom: false,
        };
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let mmc3_config = Mmc3Config::for_ines(header, 0, chr_size, rom, rom_name);
        let mut mmc3 = MapperMMC3::new(mmc3_config);
        mmc3.reset();
        let mut mmc1_core = Mmc1Core::new(mmc1_config);
        mmc1_core.reset();
        let mut vrc4 = Vrc2And4::new(VrcVariant::Mapper21);
        vrc4.reset();
        let prg_size = if header.len() > 4 { header[4] } else { 0 };
        let prg_start = 16 + ((header.get(6).copied().unwrap_or(0) as usize >> 2) & 1) * 512;
        let prg_len = (prg_size as usize) * 0x4000;
        let prg_rom = if prg_start + prg_len <= rom.len() { rom[prg_start..prg_start+prg_len].to_vec() } else { rom.to_vec() };
        let chr_rom = if chr_size > 0 {
            let start = 16 + ((header.get(6).copied().unwrap_or(0) as usize >> 2) & 1) * 512;
            let len = (chr_size as usize) * 0x4000;
            if start + len <= rom.len() { rom[start..start+len].to_vec() } else { Vec::new() }
        } else { Vec::new() };
        let mut combined = prg_rom.clone();
        combined.extend_from_slice(&chr_rom);
        let combined_8k = combined.len() / 0x2000;
        Self { reg: [0; 4], mmc3, mmc1_core, vrc4, combined_prg: combined, combined_8k, irq_enabled: false, dip_switches: 0 }
    }
}

impl Mapper for Mapper351 {
    fn reset(&mut self) {
        self.reg = [0; 4];
        self.mmc3.reset();
        self.mmc1_core.reset();
        self.vrc4.reset();
        self.irq_enabled = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x4020 && address < 0x6000 {
            if address >= 0x5000 {
                return FetchResult { data: self.dip_switches & 7, driven: true };
            }
            return FetchResult { data: 0, driven: false };
        }
        if address >= 0x6000 && address < 0x8000 {
            let idx = address as usize - 0x6000;
            if idx < cart.prg_ram.len() {
                return FetchResult { data: cart.prg_ram[idx], driven: true };
            }
            return FetchResult { data: 0, driven: false };
        }
        if address >= 0x8000 {
            let off = self.prg_offset(address) % self.combined_prg.len().max(1);
            return FetchResult { data: self.combined_prg[off], driven: true };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, val: u8) {
        if address >= 0x4020 && address < 0x6000 {
            if address >= 0x5000 {
                let idx = address as usize & 3;
                self.reg[idx] = val;
                if idx == 0 { self.apply_mode(cart); }
                self.sync();
                return;
            }
            if address == 0x4025 {
                self.mmc3.set_nametable_horizontal((val >> 3) & 1 != 0);
                self.sync();
                return;
            }
            return;
        }
        if address >= 0x6000 && address < 0x8000 {
            let idx = address as usize - 0x6000;
            if idx < cart.prg_ram.len() { cart.prg_ram[idx] = val; }
            return;
        }
        if address >= 0x8000 {
            match self.mapper() {
                0 | 1 => {
                    self.mmc3.store_prg(cart, address, val);
                }
                2 => {
                    self.mmc1_core.write_register(cart, address, val, cart.mapper_cpu_cycle);
                }
                3 => {
                    let a = if (self.reg[2] & 4) == 0 { address << 1 } else { address };
                    let decoded = if (a & 0x800) != 0 {
                        ((if (a & 4) != 0 { 8 } else { 0 }) | (if (a & 8) != 0 { 4 } else { 0 }) | (a & !0xC)) as u16
                    } else { a as u16 };
                    self.vrc4.store_prg(cart, decoded, val);
                }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        match self.mapper() {
            2 => self.mmc1_core.mirror_nametable(cart, address),
            3 => self.vrc4.mirror_nametable(cart, address),
            _ => {
                if self.mmc3.nametable_mirroring() {
                    (address & 0x33FF) | ((address & 0x0800) >> 1)
                } else {
                    address & 0x37FF
                }
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
        _using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            if self.chrram() {
                let byte = if !chr_ram.is_empty() { chr_ram[(address as usize) % chr_ram.len()] } else { 0 };
                new_addr_bus |= byte as u16;
            } else if self.chr8() {
                let bank = (self.reg[0] >> 2) as usize;
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                let src = if !chr_rom.is_empty() { chr_rom } else { chr_ram };
                let byte = if !src.is_empty() { src[offset % src.len()] } else { 0 };
                new_addr_bus |= byte as u16;
            } else {
                let offset = match self.mapper() {
                    2 => self.mmc1_chr_offset(address),
                    3 => self.vrc4_chr_offset(address),
                    0 | 1 => {
                        let bank = self.chr_bank_masked(address);
                        bank * 0x400 + (address as usize & 0x3FF)
                    }
                    _ => address as usize & 0x3FF,
                };
                let src = if !chr_rom.is_empty() { chr_rom } else { chr_ram };
                let byte = if !src.is_empty() { src[offset % src.len()] } else { 0 };
                new_addr_bus |= byte as u16;
            }
        } else {
            let mir = match self.mapper() {
                2 => mmc1_mirror_for_ppu(&self.mmc1_core, nametable_horizontal_mirroring, address),
                3 => {
                    let mirr = self.vrc4.nametable_mirroring_value();
                    match mirr & 3 {
                        0 => address & 0x37FF,
                        1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                        2 => address & 0x3FFF,
                        3 => (address & 0x3FFF) | 0x0400,
                        _ => address,
                    }
                }
                _ => {
                    if self.mmc3.nametable_mirroring() {
                        (address & 0x33FF) | ((address & 0x0800) >> 1)
                    } else {
                        address & 0x37FF
                    }
                }
            };
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let len = cart.chr_ram.len();
            if len > 0 { cart.chr_ram[(address as usize) % len] = data; }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mir = self.mirror_nametable(cart, address);
            vram[(mir & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        match self.mapper() {
            2 => self.mmc1_core.cpu_clock_irq(),
            3 => self.vrc4.cpu_clock(_cycles),
            _ => {
                self.mmc3.cpu_clock_rise(0);
                false
            }
        }
    }

    fn ppu_clock(&mut self, ppu_address_bus: u16, ppu_a12_prev: bool, scanline: u16, dot: u16, ppu_sprite_x16: bool, rendering_on: bool) -> bool {
        if self.mapper() == 0 || self.mapper() == 1 {
            self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
        } else {
            false
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.mapper() == 0 || self.mapper() == 1 {
            self.mmc3.take_irq_ack()
        } else {
            false
        }
    }

    fn audio_sample(&self) -> f32 { 0.0 }

    fn get_dip_switches(&self) -> u8 { self.dip_switches }
    fn set_dip_switches(&mut self, value: u8) { self.dip_switches = value; }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.reg);
        state.extend_from_slice(&self.mmc3.save_mapper_registers(cart));
        let mut mmc1_state = Vec::new();
        self.mmc1_core.append_save_state(&mut mmc1_state);
        state.extend_from_slice(&mmc1_state);
        state.extend_from_slice(&self.vrc4.save_mapper_registers(cart));
        state.push(if self.irq_enabled { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..4 {
            if p < state.len() { self.reg[i] = state[p]; p += 1; }
        }
        p = self.mmc3.load_mapper_registers(cart, state, p);
        p = self.mmc1_core.load_save_state(state, p);
        p = self.vrc4.load_mapper_registers(cart, state, p);
        if p < state.len() { self.irq_enabled = state[p] != 0; p += 1; }
        p
    }
}
