use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

const MODE_MMC3: u8 = 0;
const MODE_MMC1: u8 = 1;
const MODE_UNROM: u8 = 2;
const MODE_CNROM: u8 = 3;

fn prg_off(reg2c: u8, reg2e: u8) -> usize {
    let mut off = 0usize;
    if (reg2c & 1) != 0 { off |= 0x1000; }
    if (reg2c & 4) != 0 { off |= 0x2000; }
    if (reg2e & 1) != 0 { off |= 0x4000; }
    off
}

fn chr_off(reg2c: u8, reg2e: u8) -> usize {
    let mut off = 0usize;
    if (reg2c & 2) != 0 { off |= 0x8000; }
    if (reg2c & 8) != 0 { off |= 0x10000; }
    if (reg2e & 1) != 0 { off |= 0x20000; }
    off
}

fn descramble_byte(val: u8) -> u8 {
    (val << 4 & 0x90) | (val >> 4 & 0x09) | (val << 1 & 0x44) | (val >> 1 & 0x22)
}

pub struct Mapper296 {
    mode: u8,
    chrram_mode: bool,
    reg1e: u8,
    reg2c: u8,
    reg2e: u8,
    latch: u8,

    r8000: u8,
    bank_8c: u8,
    bank_a: u8,
    chr_2k0: u8,
    chr_2k8: u8,
    chr_1k0: u8,
    chr_1k4: u8,
    chr_1k8: u8,
    chr_1kc: u8,
    irq_latch: u8,
    irq_counter: u8,
    enable_irq: bool,
    reload_irq: bool,
    mmc3_mirror: bool,

    mmc1_shift: u8,
    mmc1_shift_count: u8,
    mmc1_control: u8,
    mmc1_chr0: u8,
    mmc1_chr1: u8,
    mmc1_prg: u8,
    mmc1_last_write_cycle: i64,

    irq: bool,
    a12_filter: u8,
}

impl Mapper296 {
    pub fn new() -> Self {
        Self {
            mode: 0,
            chrram_mode: false,
            reg1e: 0,
            reg2c: 0,
            reg2e: 0,
            latch: 0,

            r8000: 0,
            bank_8c: 0,
            bank_a: 1,
            chr_2k0: 0,
            chr_2k8: 2,
            chr_1k0: 4,
            chr_1k4: 5,
            chr_1k8: 6,
            chr_1kc: 7,
            irq_latch: 0,
            irq_counter: 0,
            enable_irq: false,
            reload_irq: false,
            mmc3_mirror: false,

            mmc1_shift: 0x10,
            mmc1_shift_count: 0,
            mmc1_control: 0x1F,
            mmc1_chr0: 0,
            mmc1_chr1: 0,
            mmc1_prg: 0,
            mmc1_last_write_cycle: -2,

            irq: false,
            a12_filter: 0,
        }
    }

    fn bank_offset(&self) -> usize {
        prg_off(self.reg2c, self.reg2e)
    }

    fn chr_bank_offset(&self) -> usize {
        chr_off(self.reg2c, self.reg2e)
    }

    fn descramble(&self) -> bool {
        (self.reg1e & 0xC0) != 0
    }

    fn mmc1_write_register(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            return;
        }
        if (data & 0x80) != 0 {
            self.mmc1_control |= 0x0C;
            self.mmc1_shift = 0x10;
            self.mmc1_shift_count = 0;
            self.mmc1_last_write_cycle = cart.mapper_cpu_cycle;
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
        self.mmc1_shift >>= 1;
        if (data & 1) != 0 {
            self.mmc1_shift |= 0x10;
        }
        self.mmc1_last_write_cycle = cart.mapper_cpu_cycle;
        if done {
            let reg = ((address >> 13) as u8).wrapping_sub(4);
            match reg {
                0 => self.mmc1_control = self.mmc1_shift,
                1 => self.mmc1_chr0 = self.mmc1_shift,
                2 => self.mmc1_chr1 = self.mmc1_shift,
                3 => self.mmc1_prg = self.mmc1_shift,
                _ => {}
            }
            self.mmc1_shift = 0x10;
            self.mmc1_shift_count = 0;
        }
    }

    fn prg_offset_mmc3(&self, cart: &Cartridge, address: u16) -> usize {
        let po = self.bank_offset();
        let prg_len = cart.prg_rom.len();
        let (bank, addr_lo) = if address >= 0xE000 {
            ((prg_len / 0x2000).saturating_sub(1), address as usize & 0x1FFF)
        } else if address >= 0xC000 {
            if (self.r8000 & 0x40) != 0 {
                (self.bank_8c as usize, address as usize & 0x1FFF)
            } else {
                ((prg_len / 0x2000).saturating_sub(2), address as usize & 0x1FFF)
            }
        } else if address >= 0xA000 {
            (self.bank_a as usize, address as usize & 0x1FFF)
        } else {
            if (self.r8000 & 0x40) == 0 {
                (self.bank_8c as usize, address as usize & 0x1FFF)
            } else {
                ((prg_len / 0x2000).saturating_sub(2), address as usize & 0x1FFF)
            }
        };
        let actual = ((bank & 0x0FFF) | po) * 0x2000 + addr_lo;
        actual % prg_len.max(1)
    }

    fn chr_offset_mmc3(&self, chr_len: usize, address: u16) -> usize {
        let co = self.chr_bank_offset();
        let bank = crate::mappers::mmc3::mmc3_chr_bank(
            self.r8000,
            self.chr_2k0,
            self.chr_2k8,
            self.chr_1k0,
            self.chr_1k4,
            self.chr_1k8,
            self.chr_1kc,
            address,
        ) as usize;
        let actual = ((bank & 0x7FFF) | co) * 0x400 + (address as usize & 0x3FF);
        actual % chr_len.max(1)
    }

    fn prg_offset_mmc1(&self, cart: &Cartridge, address: u16) -> usize {
        let po = self.bank_offset();
        let prg_len = cart.prg_rom.len();
        let prg_reg = (self.mmc1_prg & 0x0F) as usize;
        let num_banks = (prg_len / 0x4000).max(1);
        let off = if (self.mmc1_chr0 & 0x10) != 0 { 16 } else { 0 };
        let mode = (self.mmc1_control >> 2) & 0x03;
        let bank = match mode {
            0 | 1 => {
                let bank32 = ((prg_reg & 0x0E) + off).min(num_banks.saturating_sub(2));
                (bank32 & 0x0FFF) | po
            }
            2 => {
                if address >= 0xC000 {
                    ((prg_reg + off) & 0x0FFF) | po
                } else {
                    (off & 0x0FFF) | po
                }
            }
            3 => {
                if address >= 0xC000 {
                    ((0x0F + off) & 0x0FFF) | po
                } else {
                    ((prg_reg + off) & 0x0FFF) | po
                }
            }
            _ => 0,
        };
        (bank * 0x4000 + (address as usize & 0x3FFF)) % prg_len.max(1)
    }

    fn chr_offset_mmc1(&self, chr_len: usize, address: u16) -> usize {
        let co = self.chr_bank_offset();
        let mode = (self.mmc1_control >> 4) & 1;
        if mode != 0 {
            let bank = if address < 0x1000 {
                self.mmc1_chr0 as usize
            } else {
                self.mmc1_chr1 as usize
            };
            let actual = ((bank & 0x7FFF) | co) * 0x1000 + (address as usize & 0xFFF);
            actual % chr_len.max(1)
        } else {
            let bank = (self.mmc1_chr0 & 0x1E) as usize;
            let actual = ((bank & 0x7FFF) | co) * 0x1000 + (address as usize & 0x1FFF);
            actual % chr_len.max(1)
        }
    }

    fn mirror_mmc1(&self) -> bool {
        (self.mmc1_control & 0x03) != 0
    }
}

impl Mapper for Mapper296 {
    fn reset(&mut self) {
        self.mode = 0;
        self.chrram_mode = false;
        self.reg1e = 0;
        self.reg2c = 0;
        self.reg2e = 0;
        self.latch = 0;

        self.r8000 = 0;
        self.bank_8c = 0;
        self.bank_a = 1;
        self.chr_2k0 = 0;
        self.chr_2k8 = 2;
        self.chr_1k0 = 4;
        self.chr_1k4 = 5;
        self.chr_1k8 = 6;
        self.chr_1kc = 7;
        self.irq_latch = 0;
        self.irq_counter = 0;
        self.enable_irq = false;
        self.reload_irq = false;
        self.mmc3_mirror = false;

        self.mmc1_shift = 0x10;
        self.mmc1_shift_count = 0;
        self.mmc1_control = 0x1F;
        self.mmc1_chr0 = 0;
        self.mmc1_chr1 = 0;
        self.mmc1_prg = 0;
        self.mmc1_last_write_cycle = -2;

        self.irq = false;
        self.a12_filter = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let offset = match self.mode {
                MODE_MMC3 => self.prg_offset_mmc3(cart, address),
                MODE_MMC1 => self.prg_offset_mmc1(cart, address),
                MODE_UNROM => {
                    let po = self.bank_offset();
                    let prg_len = cart.prg_rom.len();
                    if address >= 0xC000 {
                        let bank = (0xFF | po) & 0x4FFF;
                        (bank * 0x4000 + (address as usize & 0x3FFF)) % prg_len.max(1)
                    } else {
                        let bank = ((self.latch as usize) | po) & 0x4FFF;
                        (bank * 0x4000 + (address as usize & 0x3FFF)) % prg_len.max(1)
                    }
                }
                MODE_CNROM => {
                    let po = self.bank_offset();
                    let prg_len = cart.prg_rom.len();
                    let bank32 = po & 0x4FFF;
                    (bank32 * 0x8000 + (address as usize & 0x7FFF)) % prg_len.max(1)
                }
                _ => 0,
            };
            let data = if offset < cart.prg_rom.len() {
                cart.prg_rom[offset]
            } else {
                0
            };
            FetchResult { data, driven: true }
        } else if address >= 0x6000 {
            let idx = (address - 0x6000) as usize;
            if idx < cart.prg_ram.len() {
                FetchResult { data: cart.prg_ram[idx], driven: true }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else if address >= 0x4020 && ((address & 0xFF) == 0x12D) {
            FetchResult { data: 3, driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        let lo = address & 0xFF;
        if (0x4020..0x5000).contains(&address) {
            match lo {
                0x1D => {
                    self.mode = data & 3;
                    self.chrram_mode = (data & 4) != 0;
                }
                0x1E => self.reg1e = data,
                0x2C => self.reg2c = data,
                0x2E => self.reg2e = data,
                _ => {}
            }
            return;
        }
        if address < 0x6000 {
            return;
        }
        if address < 0x8000 {
            let idx = (address - 0x6000) as usize;
            if idx < cart.prg_ram.len() {
                cart.prg_ram[idx] = data;
            }
            return;
        }
        match self.mode {
            MODE_MMC3 => {
                let mmc3_idx = address & 0xE001;
                match mmc3_idx {
                    0x8000 => self.r8000 = data,
                    0x8001 => {
                        let mask = ((cart.prg_rom.len() / 0x2000).max(1) - 1) as u8;
                        let idx = self.r8000 & 0x07;
                        match idx {
                            0 => self.chr_2k0 = data & 0xFE,
                            1 => self.chr_2k8 = data & 0xFE,
                            2 => self.chr_1k0 = data,
                            3 => self.chr_1k4 = data,
                            4 => self.chr_1k8 = data,
                            5 => self.chr_1kc = data,
                            6 => self.bank_8c = data & mask,
                            7 => self.bank_a = data & mask,
                            _ => {}
                        }
                    }
                    0xA000 => self.mmc3_mirror = (data & 1) != 0,
                    0xA001 => {}
                    0xC000 => self.irq_latch = data,
                    0xC001 => self.reload_irq = true,
                    0xE000 => self.enable_irq = false,
                    0xE001 => self.enable_irq = true,
                    _ => {}
                }
            }
            MODE_MMC1 => {
                self.mmc1_write_register(cart, address, data);
            }
            MODE_UNROM | MODE_CNROM => {
                self.latch = data;
            }
            _ => {}
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            return address;
        }
        let h = match self.mode {
            MODE_MMC3 => self.mmc3_mirror,
            MODE_MMC1 => self.mirror_mmc1(),
            MODE_UNROM => {
                let po = self.bank_offset();
                (po & 1) != 0
            }
            MODE_CNROM => {
                let po = self.bank_offset();
                (po & 1) != 0
            }
            _ => cart.nametable_horizontal_mirroring,
        };
        if h {
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
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if ciram {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else {
                let h = match self.mode {
                    MODE_MMC3 => self.mmc3_mirror,
                    MODE_MMC1 => self.mirror_mmc1(),
                    MODE_UNROM | MODE_CNROM => (self.bank_offset() & 1) != 0,
                    _ => false,
                };
                if h {
                    (address & 0x33FF) | ((address & 0x0800) >> 1)
                } else {
                    address & 0x37FF
                }
            };
            let byte = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= byte as u16;
            return (new_addr_bus as u8, new_addr_bus);
        }
        let using_chr_ram = self.chrram_mode && !chr_ram.is_empty();
        let byte = if self.mode == MODE_MMC3 {
            let offset = self.chr_offset_mmc3(
                if using_chr_ram { chr_ram.len() } else { chr_rom.len() },
                address,
            );
            if using_chr_ram {
                chr_ram[offset % chr_ram.len().max(1)]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            }
        } else if self.mode == MODE_MMC1 {
            let offset = self.chr_offset_mmc1(
                if using_chr_ram { chr_ram.len() } else { chr_rom.len() },
                address,
            );
            if using_chr_ram {
                chr_ram[offset % chr_ram.len().max(1)]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            }
        } else {
            if using_chr_ram {
                let bank = self.latch as usize;
                let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % chr_ram.len().max(1);
                chr_ram[offset]
            } else if !chr_rom.is_empty() {
                let co = self.chr_bank_offset();
                if self.mode == MODE_CNROM {
                    let bank = (((self.latch as usize) & 0x7FFF) | co) & 0x3FFFF;
                    let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % chr_rom.len();
                    chr_rom[offset]
                } else {
                    let bank = (0usize & 0x7FFF) | co;
                    let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % chr_rom.len();
                    chr_rom[offset]
                }
            } else {
                0
            }
        };
        let byte = if self.descramble() {
            descramble_byte(byte)
        } else {
            byte
        };
        new_addr_bus |= byte as u16;
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let offset = if self.mode == MODE_MMC3 {
                self.chr_offset_mmc3(cart.chr_ram.len(), address)
            } else if self.mode == MODE_MMC1 {
                self.chr_offset_mmc1(cart.chr_ram.len(), address)
            } else if self.chrram_mode {
                let bank = self.latch as usize;
                let len = cart.chr_ram.len().max(1);
                (bank * 0x2000 + (address as usize & 0x1FFF)) % len
            } else {
                address as usize
            };
            if offset < cart.chr_ram.len() {
                cart.chr_ram[offset] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
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
        if self.mode != MODE_MMC3 {
            return false;
        }
        let a12 = (ppu_address_bus & 0x1000) != 0;
        if a12 && !ppu_a12_prev && self.a12_filter < 8 {
            self.a12_filter = self.a12_filter.wrapping_add(1);
        }
        if a12 && !ppu_a12_prev && self.a12_filter >= 8 {
            self.a12_filter = 0;
            if self.reload_irq {
                self.reload_irq = false;
            } else if self.irq_counter == 0 {
                self.irq_counter = self.irq_latch;
            } else {
                self.irq_counter = self.irq_counter.wrapping_sub(1);
            }
            if self.irq_counter == 0 && self.enable_irq {
                self.irq = true;
            }
        }
        false
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        let prev_irq = self.irq;
        self.irq = false;
        prev_irq
    }

    fn take_irq_ack(&mut self) -> bool {
        let irq = self.irq;
        self.irq = false;
        irq
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.mode);
        state.push(if self.chrram_mode { 1 } else { 0 });
        state.push(self.reg1e);
        state.push(self.reg2c);
        state.push(self.reg2e);
        state.push(self.latch);
        state.push(self.r8000);
        state.push(self.bank_8c);
        state.push(self.bank_a);
        state.push(self.chr_2k0);
        state.push(self.chr_2k8);
        state.push(self.chr_1k0);
        state.push(self.chr_1k4);
        state.push(self.chr_1k8);
        state.push(self.chr_1kc);
        state.push(self.irq_latch);
        state.push(self.irq_counter);
        state.push(if self.enable_irq { 1 } else { 0 });
        state.push(if self.reload_irq { 1 } else { 0 });
        state.push(if self.mmc3_mirror { 1 } else { 0 });
        state.push(self.mmc1_shift);
        state.push(self.mmc1_shift_count);
        state.push(self.mmc1_control);
        state.push(self.mmc1_chr0);
        state.push(self.mmc1_chr1);
        state.push(self.mmc1_prg);
        state.extend_from_slice(&self.mmc1_last_write_cycle.to_le_bytes());
        state.extend_from_slice(&cart.prg_ram);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p >= state.len() { return p; }
        self.mode = state[p]; p += 1;
        self.chrram_mode = state.get(p).copied().unwrap_or(0) != 0; p += 1;
        self.reg1e = state.get(p).copied().unwrap_or(0); p += 1;
        self.reg2c = state.get(p).copied().unwrap_or(0); p += 1;
        self.reg2e = state.get(p).copied().unwrap_or(0); p += 1;
        self.latch = state.get(p).copied().unwrap_or(0); p += 1;
        self.r8000 = state.get(p).copied().unwrap_or(0); p += 1;
        self.bank_8c = state.get(p).copied().unwrap_or(0); p += 1;
        self.bank_a = state.get(p).copied().unwrap_or(1); p += 1;
        self.chr_2k0 = state.get(p).copied().unwrap_or(0); p += 1;
        self.chr_2k8 = state.get(p).copied().unwrap_or(2); p += 1;
        self.chr_1k0 = state.get(p).copied().unwrap_or(4); p += 1;
        self.chr_1k4 = state.get(p).copied().unwrap_or(5); p += 1;
        self.chr_1k8 = state.get(p).copied().unwrap_or(6); p += 1;
        self.chr_1kc = state.get(p).copied().unwrap_or(7); p += 1;
        self.irq_latch = state.get(p).copied().unwrap_or(0); p += 1;
        self.irq_counter = state.get(p).copied().unwrap_or(0); p += 1;
        self.enable_irq = state.get(p).copied().unwrap_or(0) != 0; p += 1;
        self.reload_irq = state.get(p).copied().unwrap_or(0) != 0; p += 1;
        self.mmc3_mirror = state.get(p).copied().unwrap_or(0) != 0; p += 1;
        self.mmc1_shift = state.get(p).copied().unwrap_or(0x10); p += 1;
        self.mmc1_shift_count = state.get(p).copied().unwrap_or(0); p += 1;
        self.mmc1_control = state.get(p).copied().unwrap_or(0x1F); p += 1;
        self.mmc1_chr0 = state.get(p).copied().unwrap_or(0); p += 1;
        self.mmc1_chr1 = state.get(p).copied().unwrap_or(0); p += 1;
        self.mmc1_prg = state.get(p).copied().unwrap_or(0); p += 1;
        if p + 8 <= state.len() {
            self.mmc1_last_write_cycle = i64::from_le_bytes(state[p..p+8].try_into().unwrap());
            p += 8;
        }
        for i in 0..cart.prg_ram.len() {
            if p < state.len() {
                cart.prg_ram[i] = state[p];
                p += 1;
            }
        }
        p
    }
}
