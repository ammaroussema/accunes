use std::cell::Cell;
use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};
use crate::mappers::one_bus::{descramble_chr_byte, OneBus, OneBusBanking, OneBusChrCtx, OneBusMangle};

const MODE_MMC3: u8 = 0;
const MODE_MMC1: u8 = 1;
const MODE_UNROM: u8 = 2;
const MODE_CNROM: u8 = 3;

#[derive(Clone, Copy, Default)]
struct Vt32Channel {
    count: i32,
    period: u32,
    address: u32,
    sample: i16,
    volume: u8,
    playing: bool,
    adpcm: bool,
    predictor: i32,
    step_index: i32,
    step_size: i32,
    second_nibble: bool,
}
impl Vt32Channel {
    fn reset(&mut self) {
        self.count = 0;
        self.address = 0;
        self.period = 0x6F;
        self.sample = 0;
        self.volume = 128;
        self.playing = false;
        self.adpcm = false;
        self.predictor = 0;
        self.step_index = 0;
        self.step_size = 0;
        self.second_nibble = false;
    }
    fn start(&mut self, mode: u8, address: u32, prg_len: usize) {
        self.adpcm = (mode & 0x40) != 0;
        if self.adpcm {
            self.predictor = 0;
            self.step_index = 0;
            self.step_size = 0;
            self.second_nibble = false;
        }
        self.sample = 0;
        self.count = 0;
        self.address = if prg_len == 0 { 0 } else { address % prg_len as u32 };
        self.playing = true;
    }
    fn stop(&mut self) {
        self.sample = 0;
        self.count = 0;
        self.playing = false;
    }
    fn run(&mut self, prg: &[u8]) {
        while self.playing && self.count <= 0 {
            self.count += self.period as i32;
            if prg.is_empty() { self.stop(); break; }
            let len = prg.len();
            if self.address as usize >= len { self.address %= len as u32; }
            if self.adpcm {
                if self.address as usize + 2 < len
                    && prg[self.address as usize] == 0x00
                    && prg[self.address as usize + 1] == 0x00
                    && prg[self.address as usize + 2] == 0xFF
                {
                    self.stop();
                    break;
                }
                let mut nibble = prg[self.address as usize] as i32;
                if self.second_nibble {
                    nibble &= 0x0F;
                    self.address = (self.address + 1) % len as u32;
                } else {
                    nibble >>= 4;
                }
                self.second_nibble = !self.second_nibble;
                const STEP_TABLE: [i32; 59] = [
                    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45,
                    50, 55, 60, 66, 73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307,
                    337, 371, 408, 449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878,
                ];
                const INDEX_TABLE: [i32; 16] = [-1,-1,-1,-1,1,2,4,6,-1,-1,-1,-1,1,2,4,6];
                self.step_size = STEP_TABLE[self.step_index as usize];
                let mut diff = 0;
                if nibble & 4 !=0 { diff += self.step_size; }
                if nibble & 2 !=0 { diff += self.step_size>>1; }
                if nibble & 1 !=0 { diff += self.step_size>>2; }
                diff += self.step_size>>3;
                if nibble & 8 !=0 { diff = -diff; }
                self.predictor += diff;
                if self.predictor > 2047 { self.predictor = 2047; }
                if self.predictor < -2048 { self.predictor = -2048; }
                self.sample = self.predictor as i16;
                self.step_index += INDEX_TABLE[nibble as usize];
                if self.step_index <0 { self.step_index =0; }
                if self.step_index >58 { self.step_index =58; }
            } else {
                let v = prg[self.address as usize];
                self.address = (self.address + 1) % len as u32;
                if v == 0xFF {
                    self.stop();
                    break;
                }
                self.sample = ((v as i16 - 0x80) <<4) as i16;
            }
        }
        self.count -=1;
    }
}

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
    mmc1_filter: u8,
    vt32_opcode_enc: Cell<u8>,
    vt32_next_enc: Cell<u8>,
    vt32_pending: Cell<bool>,
    vt32_pcm_address: Cell<u32>,
    vt32_channels: [Vt32Channel; 2],
    vt32_alu14: Cell<u32>,
    vt32_alu56: Cell<u16>,
    vt32_alu67: Cell<u16>,
    vt32_alu_busy: Cell<u8>,
    vt32_pcm_filter_prev: Cell<f32>,
}

impl Default for Mapper296 {
    fn default() -> Self {
        Self::new()
    }
}

impl Mapper296 {
    pub fn new() -> Self {
        let mut core = OneBus::new(&[], &[], OneBusBanking::MAPPER256);
        core.console_type_vt369 = false;
        core.console_type_vt03 = false;
        core.console_type_vt09 = false;
        let mut ch0 = Vt32Channel::default(); ch0.reset();
        let mut ch1 = Vt32Channel::default(); ch1.reset();
        Self {
            core,
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
            mmc1_filter: 0,
            vt32_opcode_enc: Cell::new(0),
            vt32_next_enc: Cell::new(0),
            vt32_pending: Cell::new(false),
            vt32_pcm_address: Cell::new(0),
            vt32_channels: [ch0, ch1],
            vt32_alu14: Cell::new(0),
            vt32_alu56: Cell::new(0),
            vt32_alu67: Cell::new(0),
            vt32_alu_busy: Cell::new(0),
            vt32_pcm_filter_prev: Cell::new(0.0),
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

    fn mmc1_get_chr_bank(&self, bank: usize) -> usize {
        if (self.mmc1_control & 0x10) != 0 {
            if bank == 0 { self.mmc1_chr0 as usize } else { self.mmc1_chr1 as usize }
        } else {
            ((self.mmc1_chr0 as usize) & !1) | (bank & 1)
        }
    }

    fn mmc1_get_prg_bank(&self, bank: usize) -> usize {
        let prg = self.mmc1_prg as usize;
        let ctrl = self.mmc1_control as usize;
        let result = if (ctrl & 0x08) != 0 {
            if (ctrl & 0x04) != 0 {
                prg | (bank * 0x0F)
            } else {
                prg & (bank * 0x0F)
            }
        } else {
            (prg & !1) | (bank & 1)
        };
        if (prg & 0x10) != 0 {
            (result & 0x07) | (prg & 0x08)
        } else {
            result & 0x0F
        }
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
                let chr0 = (self.mmc1_get_chr_bank(0) as usize) << 2;
                let chr1 = (self.mmc1_get_chr_bank(1) as usize) << 2;
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

    fn write_mmc1(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if self.mmc1_filter != 0 {
            self.mmc1_filter = 2;
            return;
        }
        if (data & 0x80) != 0 {
            self.mmc1_control |= 0x0C;
            self.mmc1_shift = 0x10;
            self.mmc1_shift_count = 0;
            self.mmc1_last_write_cycle = cart.mapper_cpu_cycle;
            self.mmc1_filter = 2;
            self.update_banking_and_mode();
            return;
        }
        self.mmc1_shift_count += 1;
        let done = self.mmc1_shift_count >= 5;
        self.mmc1_shift = (self.mmc1_shift >> 1) | ((data & 1) << 4);
        self.mmc1_last_write_cycle = cart.mapper_cpu_cycle;
        self.mmc1_filter = 2;
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
        self.core.reg4100[0x1E] = 0;
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
        self.mmc1_filter = 0;
        self.vt32_opcode_enc.set(0);
        self.vt32_next_enc.set(0);
        self.vt32_pending.set(false);
        self.vt32_pcm_address.set(0);
        for c in &mut self.vt32_channels { c.reset(); }
        self.vt32_alu14.set(0);
        self.vt32_alu56.set(0);
        self.vt32_alu67.set(0);
        self.vt32_alu_busy.set(0);
        self.vt32_pcm_filter_prev.set(0.0);
    }

    fn handle_cpu_write(&mut self, address: u16, data: u8) {
        let mangle = OneBusMangle::IDENTITY;
        let mut handled_vt32_pcm = false;
        match address {
            0x4010 => {
                if self.vt32_channels[0].playing || self.vt32_channels[1].playing {
                    handled_vt32_pcm = true;
                } else {
                    handled_vt32_pcm = false;
                }
            }
            0x4012 => {
                let cur = self.vt32_pcm_address.get();
                let v = (cur & !((0xFFu32) << 6)) | ((data as u32) << 6);
                self.vt32_pcm_address.set(v);
                handled_vt32_pcm = true;
            }
            0x4032 => {
                let p = if data !=0 { data as u32 } else { 0x6F };
                for ch in &mut self.vt32_channels { ch.period = p; }
                handled_vt32_pcm = true;
            }
            0x4033 => {
                let addr = self.vt32_pcm_address.get() & !0x3F;
                let prg_len = self.core.prg_rom.len();
                if (data & 0x10) !=0 && !self.vt32_channels[0].playing { self.vt32_channels[0].start(data, addr, prg_len); }
                if (data & 0x08) !=0 && !self.vt32_channels[1].playing { self.vt32_channels[1].start(data, addr, prg_len); }
                if (data & 0x10)==0 && (data & 0x80)!=0 && self.vt32_channels[0].playing { self.vt32_channels[0].stop(); }
                if (data & 0x08)==0 && (data & 0x80)!=0 && self.vt32_channels[1].playing { self.vt32_channels[1].stop(); }
                if (data & 0x80)==0 && (data & 0x40)==0 {
                    self.vt32_channels[0].stop(); self.vt32_channels[1].stop();
                }
                handled_vt32_pcm = true;
            }
            0x4034 => {
                let mut shift = (data >>1 &7) as u32;
                if shift==0 { shift=8; }
                self.core.dma_middle_addr = data & 0xF0;
                self.core.dma_length = 1u16 << shift;
                self.core.dma_target = if data &1 !=0 { 0x2007 } else { 0x2004 };
                handled_vt32_pcm = true;
            }
            0x4035 => {
                let cur = self.vt32_pcm_address.get();
                let v = (cur & !(0x7Fu32 <<14)) | ((data as u32 &0x7F)<<14);
                self.vt32_pcm_address.set(v);
                handled_vt32_pcm = true;
            }
            0x4036 => {
                let cur = self.vt32_pcm_address.get();
                let v = (cur & !(0xFFu32 <<21)) | ((data as u32)<<21);
                self.vt32_pcm_address.set(v);
                handled_vt32_pcm = true;
            }
            0x4037 => { self.vt32_channels[0].stop(); handled_vt32_pcm = true; }
            0x4038 => { self.vt32_channels[1].stop(); handled_vt32_pcm = true; }
            0x4039 => {
                if (data & 0x10)!=0 && self.vt32_channels[0].playing { self.vt32_channels[0].stop(); }
                if (data & 0x08)!=0 && self.vt32_channels[1].playing { self.vt32_channels[1].stop(); }
                handled_vt32_pcm = true;
            }
            0x411E => {
                self.vt32_pending.set(true);
                let nxt = match data & 7 {
                    5 => 0xA1,
                    7 => 0x7E,
                    _ => 0x00,
                };
                self.vt32_next_enc.set(nxt);
                handled_vt32_pcm = true;
            }
            0x4130 => { let v = self.vt32_alu14.get() & 0xFFFFFF00 | data as u32; self.vt32_alu14.set(v); handled_vt32_pcm = true; }
            0x4131 => { let v = self.vt32_alu14.get() & 0xFFFF00FF | ((data as u32) <<8); self.vt32_alu14.set(v); handled_vt32_pcm = true; }
            0x4132 => { let v = self.vt32_alu14.get() & 0xFF00FFFF | ((data as u32) <<16); self.vt32_alu14.set(v); handled_vt32_pcm = true; }
            0x4133 => { let v = self.vt32_alu14.get() & 0x00FFFFFF | ((data as u32) <<24); self.vt32_alu14.set(v); handled_vt32_pcm = true; }
            0x4134 => { let v = self.vt32_alu56.get() & 0xFF00 | data as u16; self.vt32_alu56.set(v); handled_vt32_pcm = true; }
            0x4135 => {
                let v = self.vt32_alu56.get() & 0x00FF | ((data as u16) <<8);
                self.vt32_alu56.set(v);
                let op1 = (self.vt32_alu14.get() & 0xFFFF) as u64;
                let op2 = v as u64;
                self.vt32_alu14.set((op1 * op2) as u32);
                self.vt32_alu_busy.set(16);
                handled_vt32_pcm = true;
            }
            0x4136 => { let v = self.vt32_alu67.get() & 0xFF00 | data as u16; self.vt32_alu67.set(v); handled_vt32_pcm = true; }
            0x4137 => {
                let v = self.vt32_alu67.get() & 0x00FF | ((data as u16) <<8);
                self.vt32_alu67.set(v);
                if v !=0 {
                    let num = self.vt32_alu14.get();
                    let den = v as u32;
                    self.vt32_alu56.set((num % den) as u16);
                    self.vt32_alu14.set(num / den);
                    self.vt32_alu_busy.set(32);
                }
                handled_vt32_pcm = true;
            }
            _ => {}
        }
        if (0x4020..=0x403F).contains(&address) || (0x4130..=0x4137).contains(&address) {
            if (address as usize) < 0x5000 {
                let idx = (address & 0xFF) as usize;
                if idx < 0x100 { self.core.reg4100[idx] = data; }
            }
        }
        if (0x2000..0x2100).contains(&address) {
            self.core.write_ppu(address, data, &mangle);
        } else if (0x4100..0x4200).contains(&address) {
            if !handled_vt32_pcm || matches!(address, 0x411E) {
                self.core.write_apu(address, data, &mangle);
            } else {
                self.core.write_apu(address, data, &mangle);
            }
            if address == 0x411D || address == 0x412C || address == 0x412E {
                self.update_banking_and_mode();
            }
        } else if (0x4000..=0x401F).contains(&address) && handled_vt32_pcm {
            let idx = (address & 0xFF) as usize;
            if idx < 0x100 { self.core.reg4100[idx] = data; }
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
        if self.core.prg_rom.is_empty() && !cart.prg_rom.is_empty() {
            self.core.prg_rom = cart.prg_rom.clone();
        }
        if address == 0x412D {
            return FetchResult { data: self.dip_value, driven: true };
        }
        match address {
            0x4014 => {
                let v = (if self.vt32_channels[0].playing {1} else {0}) | (if self.vt32_channels[1].playing {2} else {0});
                return FetchResult { data: v, driven: true };
            }
            0x4119 => return FetchResult { data: 0, driven: true },
            0x4130 | 0x4138 => return FetchResult { data: (self.vt32_alu14.get() &0xFF) as u8, driven: true },
            0x4131 | 0x4139 => return FetchResult { data: ((self.vt32_alu14.get()>>8)&0xFF) as u8, driven: true },
            0x4132 | 0x413A => return FetchResult { data: ((self.vt32_alu14.get()>>16)&0xFF) as u8, driven: true },
            0x4133 | 0x413B => return FetchResult { data: ((self.vt32_alu14.get()>>24)&0xFF) as u8, driven: true },
            0x4134 | 0x413C => return FetchResult { data: (self.vt32_alu56.get() &0xFF) as u8, driven: true },
            0x4135 | 0x413D => return FetchResult { data: ((self.vt32_alu56.get()>>8)&0xFF) as u8, driven: true },
            0x4136 => return FetchResult { data: self.vt32_alu_busy.get(), driven: true },
            _ => {}
        }
        if address >= 0x4100 && address < 0x4200 {
            if let Some(data) = self.core.read_apu(address) {
                if address == 0x4119 {
                    return FetchResult { data: 0, driven: true };
                }
                return FetchResult { data, driven: true };
            }
        }
        if (0x4000..=0x401F).contains(&address) {
            if address == 0x4014 {
                let v = (if self.vt32_channels[0].playing {1} else {0}) | (if self.vt32_channels[1].playing {2} else {0});
                return FetchResult { data: v, driven: true };
            }
        }
        if address >= 0x8000 {
            let prg_or = self.prg_or();
            let bank = match self.mode {
                MODE_MMC3 => self.core.get_prg_bank(((address - 0x8000) >> 13) as usize),
                MODE_MMC1 => {
                    let slot = ((address - 0x8000) >> 14) as usize;
                    let bank16 = self.mmc1_get_prg_bank(slot);
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
        prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        self.fetch_ppu_with_ctx(
            prg_rom, chr_rom, prg_ram, chr_ram, prg_vram,
            using_chr_ram, nametable_horizontal_mirroring, alternative_nametable_arrangement,
            ppu_address_bus, ppu_octal_latch, vram, OneBusChrCtx::default(),
        )
    }
    fn fetch_ppu_with_ctx(
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
        ctx: OneBusChrCtx,
    ) -> (u8, u16) {
        let raw_address = (ppu_address_bus & 0x7FFF) | (ppu_octal_latch as u16);
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        let is_vt = self.is_vt32() || self.core.console_type_vt03 || self.core.console_type_vt09 || self.core.console_type_vt369;
        let is_chr_fetch = raw_address < 0x2000
            || (raw_address >= 0x4000 && raw_address < 0x6000)
            || (is_vt && ctx.active && ((raw_address >= 0x2000 && raw_address < 0x4000) || (raw_address >= 0x6000 && raw_address < 0x8000)));
        if is_chr_fetch {
            let high_plane = (raw_address & 0x4000) != 0;
            let chr_addr = raw_address & 0x1FFF;
            let ext_address = if ctx.active && is_vt {
                ctx.map_chr_address(if high_plane { 0x4000 | chr_addr } else { chr_addr })
            } else if high_plane { 0x4000 | chr_addr } else { chr_addr };
            let (is_bg, is_sprite, chr_eva) = if ctx.active && is_vt { (ctx.is_bg, ctx.is_sprite, ctx.eva) } else { (false,false,0) };
            let descramble = (self.core.reg4100[0x1E] & 0xC0) != 0;
            let mut byte = self.core.fetch_chr_byte_ext(
                prg_rom, chr_rom, chr_ram, ext_address, self.chrram, is_bg, is_sprite, chr_eva,
            );
            if descramble { byte = descramble_chr_byte(byte); }
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
        if self.mmc1_filter > 0 {
            self.mmc1_filter -= 1;
        }
        if self.vt32_alu_busy.get() >0 {
            self.vt32_alu_busy.set(self.vt32_alu_busy.get()-1);
        }
        if self.vt32_channels[0].playing || self.vt32_channels[1].playing {
            let prg: &[u8] = &self.core.prg_rom;
            for ch in &mut self.vt32_channels {
                if ch.playing {
                    if ch.period ==0 { ch.period = 0x6F; }
                    ch.run(prg);
                }
            }
        }
        if self.mode == MODE_MMC3 {
            return self.core.cpu_cycle();
        } else if self.mode == MODE_MMC1 {
            return false;
        }
        false
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
        state.push(self.mmc1_filter);
        state.push(self.vt32_opcode_enc.get());
        state.push(self.vt32_next_enc.get());
        state.push(if self.vt32_pending.get() { 1 } else { 0 });
        state.extend_from_slice(&self.vt32_pcm_address.get().to_le_bytes());
        for ch in &self.vt32_channels {
            state.extend_from_slice(&ch.count.to_le_bytes());
            state.extend_from_slice(&ch.period.to_le_bytes());
            state.extend_from_slice(&ch.address.to_le_bytes());
            state.extend_from_slice(&ch.sample.to_le_bytes());
            state.push(ch.volume);
            state.push(if ch.playing {1} else {0});
            state.push(if ch.adpcm {1} else {0});
            state.extend_from_slice(&ch.predictor.to_le_bytes());
            state.extend_from_slice(&ch.step_index.to_le_bytes());
            state.extend_from_slice(&ch.step_size.to_le_bytes());
            state.push(if ch.second_nibble {1} else {0});
        }
        state.extend_from_slice(&self.vt32_alu14.get().to_le_bytes());
        state.extend_from_slice(&self.vt32_alu56.get().to_le_bytes());
        state.extend_from_slice(&self.vt32_alu67.get().to_le_bytes());
        state.push(self.vt32_alu_busy.get());
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
        if p < state.len() { self.mmc1_filter = state[p]; p += 1; }
        if p < state.len() { self.vt32_opcode_enc.set(state[p]); p += 1; }
        if p < state.len() { self.vt32_next_enc.set(state[p]); p += 1; }
        if p < state.len() { self.vt32_pending.set(state[p] != 0); p += 1; }
        if p +4 <= state.len() { self.vt32_pcm_address.set(u32::from_le_bytes([state[p],state[p+1],state[p+2],state[p+3]])); p+=4; }
        for ch in &mut self.vt32_channels {
            if p+4 <= state.len() { ch.count = i32::from_le_bytes([state[p],state[p+1],state[p+2],state[p+3]]); p+=4; }
            if p+4 <= state.len() { ch.period = u32::from_le_bytes([state[p],state[p+1],state[p+2],state[p+3]]); p+=4; }
            if p+4 <= state.len() { ch.address = u32::from_le_bytes([state[p],state[p+1],state[p+2],state[p+3]]); p+=4; }
            if p+2 <= state.len() { ch.sample = i16::from_le_bytes([state[p],state[p+1]]); p+=2; }
            if p < state.len() { ch.volume = state[p]; p+=1; }
            if p < state.len() { ch.playing = state[p]!=0; p+=1; }
            if p < state.len() { ch.adpcm = state[p]!=0; p+=1; }
            if p+4 <= state.len() { ch.predictor = i32::from_le_bytes([state[p],state[p+1],state[p+2],state[p+3]]); p+=4; }
            if p+4 <= state.len() { ch.step_index = i32::from_le_bytes([state[p],state[p+1],state[p+2],state[p+3]]); p+=4; }
            if p+4 <= state.len() { ch.step_size = i32::from_le_bytes([state[p],state[p+1],state[p+2],state[p+3]]); p+=4; }
            if p < state.len() { ch.second_nibble = state[p]!=0; p+=1; }
        }
        if p+4 <= state.len() { self.vt32_alu14.set(u32::from_le_bytes([state[p],state[p+1],state[p+2],state[p+3]])); p+=4; }
        if p+2 <= state.len() { self.vt32_alu56.set(u16::from_le_bytes([state[p],state[p+1]])); p+=2; }
        if p+2 <= state.len() { self.vt32_alu67.set(u16::from_le_bytes([state[p],state[p+1]])); p+=2; }
        if p < state.len() { self.vt32_alu_busy.set(state[p]); p+=1; }
        self.update_banking_and_mode();
        p
    }

    fn vt03_4bpp_bg(&self) -> bool { (self.core.reg2000[0x10] & 0x02) != 0 }
    fn vt03_4bpp_sp(&self) -> bool { (self.core.reg2000[0x10] & 0x04) != 0 }
    fn vt03_reg2000_10(&self) -> u8 { self.core.reg2000[0x10] }
    fn onebus_cpu_ram_4k(&self) -> bool { true }
    fn onebus_vt03_ppu(&self) -> bool { true }
    fn is_vt32(&self) -> bool { true }
    fn onebus_chr_routing_ppu(&self) -> bool { true }

    fn unscramble_opcode(&self, opcode: u8) -> u8 {
        let enc = self.vt32_opcode_enc.get();
        let descrambled = opcode ^ enc;
        if self.vt32_pending.get() && (descrambled == 0x4C || descrambled == 0x6C) {
            self.vt32_opcode_enc.set(self.vt32_next_enc.get());
            self.vt32_pending.set(false);
        }
        descrambled
    }
    fn audio_sample(&self) -> f32 {
        if !self.vt32_channels.iter().any(|c| c.playing) { return 0.0; }
        let mut sum: i32 = 0;
        for ch in &self.vt32_channels { if ch.playing { sum += (ch.sample as i32 * ch.volume as i32) >>4; } }
        sum as f32 / 32767.0
    }
}
