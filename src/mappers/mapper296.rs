use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};
use crate::mappers::one_bus::{descramble_chr_byte, OneBus, OneBusBanking, OneBusMangle};

const MODE_MMC3: u8 = 0;
const MODE_MMC1: u8 = 1;
const MODE_UNROM: u8 = 2;
const MODE_CNROM: u8 = 3;

pub struct Mapper296 {
    core: OneBus,
    mode: u8,
    chrram: bool,
    latch_data: u8,
    dip_value: u8,
    mmc1_shift: u8,
    mmc1_shift_count: u8,
    mmc1_control: u8,
    mmc1_chr0: u8,
    mmc1_chr1: u8,
    mmc1_prg: u8,
    mmc1_last_write_cycle: i64,
}

impl Default for Mapper296 {
    fn default() -> Self {
        Self::new()
    }
}

impl Mapper296 {
    pub fn new() -> Self {
        Self {
            core: OneBus::new(&[], &[], OneBusBanking::MAPPER256),
            mode: MODE_MMC3,
            chrram: false,
            latch_data: 0,
            dip_value: 3,
            mmc1_shift: 0x10,
            mmc1_shift_count: 0,
            mmc1_control: 0x1F,
            mmc1_chr0: 0,
            mmc1_chr1: 0,
            mmc1_prg: 0,
            mmc1_last_write_cycle: -2,
        }
    }

    fn prg_or(&self) -> u16 {
        let reg2c = self.core.reg4100[0x2C] as u16;
        let reg2e = self.core.reg4100[0x2E] as u16;
        ((reg2c << 12) & 0x1000) | ((reg2c << 11) & 0x2000) | ((reg2e << 14) & 0x4000)
    }

    fn chr_or(&self) -> usize {
        let reg2c = self.core.reg4100[0x2C] as usize;
        let reg2e = self.core.reg4100[0x2E] as usize;
        ((reg2c << 14) & 0x8000) | ((reg2c << 13) & 0x10000) | ((reg2e << 17) & 0x20000)
    }

    fn update_banking_and_mode(&mut self) {
        let reg1d = self.core.reg4100[0x1D];
        self.mode = reg1d & 3;
        self.chrram = (reg1d & 4) != 0;

        let prg_or = self.prg_or();
        let chr_or = self.chr_or();
        self.core.banking = OneBusBanking {
            prg_and: 0x0FFF,
            prg_or,
            chr_and: 0x7FFF,
            chr_or,
        };

        match self.mode {
            MODE_MMC1 => {
                let chr0 = (self.mmc1_chr0 as usize) << 2;
                let chr1 = (self.mmc1_chr1 as usize) << 2;
                self.core.reg2000[0x16] = chr0 as u8;
                self.core.reg2000[0x17] = (chr0 | 2) as u8;
                self.core.reg2000[0x12] = chr1 as u8;
                self.core.reg2000[0x13] = (chr1 | 1) as u8;
                self.core.reg2000[0x14] = (chr1 | 2) as u8;
                self.core.reg2000[0x15] = (chr1 | 3) as u8;
            }
            MODE_CNROM => {
                let l = self.latch_data << 3;
                self.core.reg2000[0x16] = l;
                self.core.reg2000[0x17] = l | 2;
                self.core.reg2000[0x12] = l | 4;
                self.core.reg2000[0x13] = l | 5;
                self.core.reg2000[0x14] = l | 6;
                self.core.reg2000[0x15] = l | 7;
            }
            _ => {}
        }
    }

    fn mmc1_prg_bank(&self, slot: usize) -> usize {
        let mode = (self.mmc1_control >> 2) & 3;
        let prg = (self.mmc1_prg & 0x0F) as usize;
        match mode {
            0 | 1 => (prg & 0x0E) | (slot & 1),
            2 => {
                if slot == 0 {
                    0
                } else {
                    prg
                }
            }
            3 => {
                if slot == 0 {
                    prg
                } else {
                    0x0F
                }
            }
            _ => 0,
        }
    }

    fn write_mmc1(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (data & 0x80) != 0 {
            self.mmc1_control |= 0x0C;
            self.mmc1_shift = 0x10;
            self.mmc1_shift_count = 0;
            self.mmc1_last_write_cycle = cart.mapper_cpu_cycle;
            self.update_banking_and_mode();
            return;
        }
        if cart.mapper_cpu_cycle >= 0
            && self.mmc1_last_write_cycle >= 0
            && cart.mapper_cpu_cycle == self.mmc1_last_write_cycle + 1
        {
            return;
        }
        self.mmc1_shift_count += 1;
        let done = self.mmc1_shift_count >= 5;
        self.mmc1_shift = (self.mmc1_shift >> 1) | ((data & 1) << 4);
        self.mmc1_last_write_cycle = cart.mapper_cpu_cycle;
        if done {
            let val = self.mmc1_shift;
            self.mmc1_shift = 0x10;
            self.mmc1_shift_count = 0;
            match (address >> 13) & 3 {
                0 => self.mmc1_control = val,
                1 => self.mmc1_chr0 = val,
                2 => self.mmc1_chr1 = val,
                3 => self.mmc1_prg = val,
                _ => {}
            }
            self.update_banking_and_mode();
        }
    }
}

impl Mapper for Mapper296 {
    fn reset(&mut self) {
        self.core.reset();
        self.core.reg4100[0x1D] = 0;
        self.core.reg4100[0x2C] = 0;
        self.core.reg4100[0x2E] = 0;
        self.mode = MODE_MMC3;
        self.chrram = false;
        self.latch_data = 0;
        self.dip_value = 3;
        self.mmc1_shift = 0x10;
        self.mmc1_shift_count = 0;
        self.mmc1_control = 0x1F;
        self.mmc1_chr0 = 0;
        self.mmc1_chr1 = 0;
        self.mmc1_prg = 0;
        self.mmc1_last_write_cycle = -2;
    }

    fn handle_cpu_write(&mut self, address: u16, data: u8) {
        let mangle = OneBusMangle::IDENTITY;
        if (0x2000..0x2100).contains(&address) {
            self.core.write_ppu(address, data, &mangle);
        } else if (0x4100..0x4200).contains(&address) {
            self.core.write_apu(address, data, &mangle);
            if address == 0x411D || address == 0x412C || address == 0x412E {
                self.update_banking_and_mode();
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            return;
        }
        match self.mode {
            MODE_MMC3 => {
                self.core.store_prg_mmc3(address, data, &OneBusMangle::IDENTITY);
            }
            MODE_MMC1 => {
                self.write_mmc1(cart, address, data);
            }
            MODE_UNROM | MODE_CNROM => {
                self.latch_data = data;
                self.update_banking_and_mode();
            }
            _ => {}
        }
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address == 0x412D {
            return FetchResult {
                data: self.dip_value,
                driven: true,
            };
        }
        if address >= 0x4100 && address < 0x4200 {
            if let Some(data) = self.core.read_apu(address) {
                return FetchResult { data, driven: true };
            }
        }
        if address >= 0x8000 {
            let prg_or = self.prg_or();
            let bank = match self.mode {
                MODE_MMC3 => self.core.get_prg_bank(((address - 0x8000) >> 13) as usize),
                MODE_MMC1 => {
                    let slot = ((address - 0x8000) >> 14) as usize;
                    let bank16 = self.mmc1_prg_bank(slot);
                    self.core.get_prg16_bank(bank16, ((address - 0x8000) >> 13) as usize & 1)
                }
                MODE_UNROM => {
                    let slot = ((address - 0x8000) >> 14) as usize;
                    let bank16 = if slot == 0 { self.latch_data as usize } else { 0xFF };
                    self.core.get_prg16_bank(bank16, ((address - 0x8000) >> 13) as usize & 1)
                }
                MODE_CNROM => self.core.get_prg_bank(((address - 0x8000) >> 13) as usize),
                _ => 0,
            };
            let offset = ((bank & 0x0FFF) | (prg_or as usize)) * 0x2000 + (address as usize & 0x1FFF);
            let data = if !cart.prg_rom.is_empty() {
                cart.prg_rom[offset % cart.prg_rom.len()]
            } else {
                0
            };
            return FetchResult { data, driven: true };
        }
        FetchResult { data: 0, driven: false }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mode == MODE_MMC1 {
            match self.mmc1_control & 3 {
                0 => 0x2000 + (address & 0x3FF),
                1 => 0x2400 + (address & 0x3FF),
                2 => mirror_h_or_v(false, address),
                3 => mirror_h_or_v(true, address),
                _ => mirror_h_or_v(false, address),
            }
        } else {
            mirror_h_or_v(self.core.hv() != 0, address)
        }
    }

    fn fetch_ppu(
        &mut self,
        prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let raw_address = (ppu_address_bus & 0x7FFF) | (ppu_octal_latch as u16);
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        let is_chr_fetch = raw_address < 0x2000 || (raw_address >= 0x4000 && raw_address < 0x6000);
        if is_chr_fetch {
            let high_plane = raw_address >= 0x4000 && raw_address < 0x6000;
            let chr_addr = raw_address & 0x1FFF;
            let ext_address = if high_plane { 0x4000 | chr_addr } else { chr_addr };
            let descramble = (self.core.reg4100[0x1E] & 0xC0) != 0;
            let mut byte = self.core.fetch_chr_byte_ext(
                prg_rom,
                chr_rom,
                chr_ram,
                ext_address,
                self.chrram,
                false,
                false,
                0,
            );
            if descramble {
                byte = descramble_chr_byte(byte);
            }
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mirror_h_or_v(self.core.hv() != 0, raw_address);
            let byte = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 || (address >= 0x4000 && address < 0x6000) {
            if self.chrram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        _ppu_sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        if self.mode == MODE_MMC3 {
            self.core.ppu_cycle(ppu_address_bus, scanline, dot, rendering_on)
        } else {
            false
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.mode == MODE_MMC3 {
            self.core.cpu_cycle()
        } else {
            false
        }
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_value
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_value = value;
    }

    fn take_irq_ack(&mut self) -> bool {
        self.core.take_irq_ack()
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.core.save_core());
        state.push(self.mode);
        state.push(if self.chrram { 1 } else { 0 });
        state.push(self.latch_data);
        state.push(self.dip_value);
        state.push(self.mmc1_shift);
        state.push(self.mmc1_shift_count);
        state.push(self.mmc1_control);
        state.push(self.mmc1_chr0);
        state.push(self.mmc1_chr1);
        state.push(self.mmc1_prg);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.core.load_core(state, start);
        if p < state.len() { self.mode = state[p]; p += 1; }
        if p < state.len() { self.chrram = state[p] != 0; p += 1; }
        if p < state.len() { self.latch_data = state[p]; p += 1; }
        if p < state.len() { self.dip_value = state[p]; p += 1; }
        if p < state.len() { self.mmc1_shift = state[p]; p += 1; }
        if p < state.len() { self.mmc1_shift_count = state[p]; p += 1; }
        if p < state.len() { self.mmc1_control = state[p]; p += 1; }
        if p < state.len() { self.mmc1_chr0 = state[p]; p += 1; }
        if p < state.len() { self.mmc1_chr1 = state[p]; p += 1; }
        if p < state.len() { self.mmc1_prg = state[p]; p += 1; }
        self.update_banking_and_mode();
        p
    }

    fn vt03_4bpp_bg(&self) -> bool { (self.core.reg2000[0x10] & 0x02) != 0 }
    fn vt03_4bpp_sp(&self) -> bool { (self.core.reg2000[0x10] & 0x04) != 0 }
    fn vt03_reg2000_10(&self) -> u8 { self.core.reg2000[0x10] }
}
