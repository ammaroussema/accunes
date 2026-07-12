use crate::cartridge::Cartridge;
use crate::crc::crc32;
use crate::mapper::{FetchResult, Mapper};
const VS_FRAME_CYCLES: u64 = 29780;
const CRC_LOW_G_MAN_1: u32 = 0x9399_1433;
const CRC_LOW_G_MAN_2: u32 = 0xaf65_aa84;
const CRC_KICK_MASTER: u32 = 0x5104_833e;
const CRC_SHOUGI_A: u32 = 0x5a68_60f1;
const CRC_SHOUGI_B: u32 = 0xae28_0e20;
const CRC_PAL_STAR_WARS: u32 = 0xfcd7_72eb;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mmc3IrqHack {
    None,
    KickMaster,
    PalStarWars,
}

#[derive(Clone, Debug)]
pub struct Mmc3Config {
    pub prg_ram_size: usize,
    pub chr_ram_size: usize,
    pub mmc6: bool,
    pub ax5202p: bool,
    pub irq_revision_b: bool,
    pub irq_hack: Mmc3IrqHack,
    pub header_horizontal_mirror: bool,
}

impl Mmc3Config {
    pub fn for_ines(
        header: &[u8],
        sub_mapper: u8,
        chr_size: u8,
        rom: &[u8],
        rom_name: &str,
    ) -> Self {
        let nes2 = is_nes20(header);
        let crc = crc32(rom);
        let mut prg_ram_size = if nes2 {
            let volatile_kb = nes20_ram_kb(header[10] & 0x0F);
            let battery_kb = nes20_ram_kb((header[10] >> 4) & 0x0F);
            (volatile_kb + battery_kb) * 1024
        } else if crc == CRC_LOW_G_MAN_1 || crc == CRC_LOW_G_MAN_2 {
            0
        } else {
            0x2000
        };
        if sub_mapper == 1 {
            prg_ram_size = prg_ram_size.max(0x400);
        }
        let chr_ram_size = if chr_size > 0 {
            0
        } else if nes2 {
            let volatile_kb = nes20_ram_kb(header[11] & 0x0F);
            let battery_kb = nes20_ram_kb((header[11] >> 4) & 0x0F);
            (volatile_kb + battery_kb) * 1024
        } else {
            0x2000
        };
        let name_lower = rom_name.to_lowercase();
        let irq_revision_a = sub_mapper == 1
            || name_lower.contains("rev_a")
            || name_lower.contains("rev-a")
            || name_lower.contains("mmc6")
            || name_lower.contains("mmc3_alt");
        let irq_revision_b = !irq_revision_a;
        let irq_hack = if crc == CRC_KICK_MASTER || crc == CRC_SHOUGI_A || crc == CRC_SHOUGI_B {
            Mmc3IrqHack::KickMaster
        } else if crc == CRC_PAL_STAR_WARS {
            Mmc3IrqHack::PalStarWars
        } else {
            Mmc3IrqHack::None
        };
        Self {
            prg_ram_size,
            chr_ram_size,
            mmc6: sub_mapper == 1,
            ax5202p: false,
            irq_revision_b,
            irq_hack,
            header_horizontal_mirror: (header[6] & 1) == 0,
        }
    }

    pub fn embedded() -> Self {
        Self {
            prg_ram_size: 0x2000,
            chr_ram_size: 0x2000,
            mmc6: false,
            ax5202p: false,
            irq_revision_b: false,
            irq_hack: Mmc3IrqHack::None,
            header_horizontal_mirror: false,
        }
    }
}

fn is_nes20(header: &[u8]) -> bool {
    header.len() >= 16 && (header[7] & 0x0C) == 0x08
}

fn nes20_ram_kb(shift: u8) -> usize {
    if shift == 0 {
        0
    } else {
        (64usize << shift) / 1024
    }
}

fn prg8_mask(cart: &Cartridge) -> u8 {
    let banks = cart.prg_rom.len() / 0x2000;
    if banks == 0 {
        0
    } else {
        (banks - 1) as u8
    }
}

fn prg_rom_read(cart: &Cartridge, offset: usize) -> u8 {
    let len = cart.prg_rom.len();
    if len == 0 {
        0
    } else {
        cart.prg_rom[offset % len]
    }
}

pub fn mmc3_chr_bank(
    r8000: u8,
    chr_2k0: u8,
    chr_2k8: u8,
    chr_1k0: u8,
    chr_1k4: u8,
    chr_1k8: u8,
    chr_1kc: u8,
    address: u16,
) -> u8 {
    let invert = (r8000 & 0x80) != 0;
    if !invert {
        if address < 0x0400 {
            chr_2k0 & 0xFE
        } else if address < 0x0800 {
            chr_2k0 | 1
        } else if address < 0x0C00 {
            chr_2k8 & 0xFE
        } else if address < 0x1000 {
            chr_2k8 | 1
        } else if address < 0x1400 {
            chr_1k0
        } else if address < 0x1800 {
            chr_1k4
        } else if address < 0x1C00 {
            chr_1k8
        } else {
            chr_1kc
        }
    } else if address < 0x0400 {
        chr_1k0
    } else if address < 0x0800 {
        chr_1k4
    } else if address < 0x0C00 {
        chr_1k8
    } else if address < 0x1000 {
        chr_1kc
    } else if address < 0x1400 {
        chr_2k0 & 0xFE
    } else if address < 0x1800 {
        chr_2k0 | 1
    } else if address < 0x1C00 {
        chr_2k8 & 0xFE
    } else {
        chr_2k8 | 1
    }
}

fn read_chr_byte(
    bank: u8,
    address: u16,
    chr_rom: &[u8],
    chr_ram: &[u8],
) -> u8 {
    let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
    if !chr_rom.is_empty() {
        chr_rom[offset % chr_rom.len()]
    } else if !chr_ram.is_empty() {
        chr_ram[offset % chr_ram.len()]
    } else {
        0
    }
}

pub struct MapperMMC3 {
    pub config: Mmc3Config,
    pub r8000: u8,
    pub bank_a: u8,
    pub bank_8c: u8,
    pub chr_2k0: u8,
    pub chr_2k8: u8,
    pub chr_1k0: u8,
    pub chr_1k4: u8,
    pub chr_1k8: u8,
    pub chr_1kc: u8,
    irq_latch: u8,
    irq_counter: u8,
    enable_irq: bool,
    reload_irq_counter: bool,
    nametable_mirroring: bool,
    pub prg_ram_protect: u8,
    pub m2_filter: u8,
    #[allow(dead_code)]
    pub force_rev_a_irq: bool,
    vsdip: u8,
    coinon: u8,
    coinon2: u8,
    service: u8,
    cycle_accum: u64,
}

impl MapperMMC3 {
    pub fn new(config: Mmc3Config) -> Self {
        let force_rev_a_irq = !config.irq_revision_b;
        Self {
            config,
            r8000: 0,
            bank_a: 1,
            bank_8c: 0,
            chr_2k0: 0,
            chr_2k8: 2,
            chr_1k0: 4,
            chr_1k4: 5,
            chr_1k8: 6,
            chr_1kc: 7,
            irq_latch: 0,
            irq_counter: 0,
            enable_irq: false,
            reload_irq_counter: false,
            nametable_mirroring: false,
            prg_ram_protect: 0,
            m2_filter: 0,
            force_rev_a_irq,
            vsdip: 0,
            coinon: 0,
            coinon2: 0,
            service: 0,
            cycle_accum: 0,
        }
    }

    pub fn nametable_mirroring(&self) -> bool {
        self.nametable_mirroring
    }

    pub fn set_nametable_horizontal(&mut self, horizontal: bool) {
        self.nametable_mirroring = horizontal;
    }

    pub fn chr_bank(&self, address: u16) -> u8 {
        mmc3_chr_bank(
            self.r8000,
            self.chr_2k0,
            self.chr_2k8,
            self.chr_1k0,
            self.chr_1k4,
            self.chr_1k8,
            self.chr_1kc,
            address,
        )
    }

    fn apply_register_defaults(&mut self) {
        self.chr_2k0 = 0;
        self.chr_2k8 = 2;
        self.chr_1k0 = 4;
        self.chr_1k4 = 5;
        self.chr_1k8 = 6;
        self.chr_1kc = 7;
        self.bank_8c = 0;
        self.bank_a = 1;
    }

    fn clock_irq_counter(&mut self) -> bool {
        let prev = self.irq_counter;
        let reset_reload = self.reload_irq_counter;
        if prev == 0 || reset_reload {
            self.irq_counter = self.irq_latch;
            self.reload_irq_counter = false;
        } else {
            self.irq_counter = prev.wrapping_sub(1);
        }
        if self.irq_counter == 0 && self.enable_irq {
            if self.config.irq_revision_b {
                return true;
            }
            return prev != 0 || reset_reload;
        }
        false
    }

    fn fixed_second_last_offset(cart: &Cartridge, address: u16) -> usize {
        let len = cart.prg_rom.len();
        if len < 0x4000 {
            return 0;
        }
        let bank = (len / 0x2000).saturating_sub(2);
        bank * 0x2000 + (address as usize & 0x1FFF)
    }

    fn fixed_last_offset(cart: &Cartridge, address: u16) -> usize {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }
        let bank = (len / 0x2000).saturating_sub(1);
        bank * 0x2000 + (address as usize & 0x1FFF)
    }
}

impl Mapper for MapperMMC3 {
    fn reset(&mut self) {
        self.r8000 = 0;
        self.apply_register_defaults();
        self.irq_latch = 0;
        self.irq_counter = 0;
        self.enable_irq = false;
        self.reload_irq_counter = false;
        self.nametable_mirroring = self.config.header_horizontal_mirror;
        self.prg_ram_protect = 0;
        self.m2_filter = 0;
        self.vsdip = 0;
        self.coinon = 0;
        self.coinon2 = 0;
        self.service = 0;
        self.cycle_accum = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0xE000 {
            FetchResult {
                data: prg_rom_read(cart, Self::fixed_last_offset(cart, address)),
                driven: true,
            }
        } else if address >= 0xC000 {
            if (self.r8000 & 0x40) != 0 {
                let offset =
                    (self.bank_8c as usize) * 0x2000 + (address as usize & 0x1FFF);
                FetchResult {
                    data: prg_rom_read(cart, offset),
                    driven: true,
                }
            } else {
                FetchResult {
                    data: prg_rom_read(cart, Self::fixed_second_last_offset(cart, address)),
                    driven: true,
                }
            }
        } else if address >= 0xA000 {
            let offset = (self.bank_a as usize) * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: prg_rom_read(cart, offset),
                driven: true,
            }
        } else if address >= 0x8000 {
            if (self.r8000 & 0x40) == 0 {
                let offset =
                    (self.bank_8c as usize) * 0x2000 + (address as usize & 0x1FFF);
                FetchResult {
                    data: prg_rom_read(cart, offset),
                    driven: true,
                }
            } else {
                FetchResult {
                    data: prg_rom_read(cart, Self::fixed_second_last_offset(cart, address)),
                    driven: true,
                }
            }
        } else if address >= 0x6000 {
            if self.config.mmc6 || cart.sub_mapper == 1 {
                if (self.r8000 & 0x20) != 0 {
                    if (0x7000..=0x71FF).contains(&address) {
                        if (self.prg_ram_protect & 0x20) != 0 {
                            return FetchResult {
                                data: cart.prg_ram[address as usize & 0x3FF],
                                driven: true,
                            };
                        }
                    } else if (0x7200..=0x73FF).contains(&address) {
                        if (self.prg_ram_protect & 0x80) != 0 {
                            return FetchResult {
                                data: cart.prg_ram[address as usize & 0x3FF],
                                driven: true,
                            };
                        }
                    }
                }
                FetchResult {
                    data: 0,
                    driven: false,
                }
            } else if (self.config.ax5202p || (self.prg_ram_protect & 0x80) != 0) && self.config.prg_ram_size > 0 {
                let off = (address - 0x6000) as usize;
                if off < self.config.prg_ram_size {
                    FetchResult {
                        data: cart.prg_ram[off],
                        driven: true,
                    }
                } else {
                    FetchResult {
                        data: 0,
                        driven: false,
                    }
                }
            } else {
                FetchResult {
                    data: 0,
                    driven: false,
                }
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            if self.config.mmc6 || cart.sub_mapper == 1 {
                if (self.r8000 & 0x20) != 0 {
                    if (0x7000..=0x71FF).contains(&address) {
                        if (self.prg_ram_protect & 0x10) != 0 {
                            cart.prg_ram[address as usize & 0x3FF] = data;
                        }
                    } else if (0x7200..=0x73FF).contains(&address) {
                        if (self.prg_ram_protect & 0x40) != 0 {
                            cart.prg_ram[address as usize & 0x3FF] = data;
                        }
                    }
                }
            } else if address >= 0x6000
                && self.config.prg_ram_size > 0
                && if self.config.ax5202p {
                    (self.prg_ram_protect & 0x40) != 0
                } else {
                    (self.prg_ram_protect & 0xC0) != 0
                }
            {
                let off = (address - 0x6000) as usize;
                if off < self.config.prg_ram_size {
                    cart.prg_ram[off] = data;
                }
            }
            return;
        }
        match address & 0xE001 {
            0x8000 => {
                eprintln!("[MMC3] $8000 <- {:#04x} (mode={})", data, data & 7);
                self.r8000 = data;
            }
            0x8001 => {
                let mask = prg8_mask(cart);
                let mode = self.r8000 & 0x07;
                eprintln!("[MMC3] $8001 <- {:#04x} (bank={}, mode={})", data, self.r8000 & 7, mode);
                match mode {
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
            0xA000 => {
                eprintln!("[MMC3] $A000 <- {:#04x} (mirror={})", data, data & 1);
                self.nametable_mirroring = (data & 1) != 0;
            }
            0xA001 => {
                eprintln!("[MMC3] $A001 <- {:#04x}", data);
                self.prg_ram_protect = data;
            }
            0xC000 => {
                eprintln!("[MMC3] $C000 <- {:#04x} (IRQ latch)", data);
                self.irq_latch = data;
            }
            0xC001 => {
                eprintln!("[MMC3] $C001 <- {:#04x} (IRQ reload)", data);
                self.reload_irq_counter = true;
            }
            0xE000 => {
                eprintln!("[MMC3] $E000 <- {:#04x} (IRQ disable)", data);
                self.enable_irq = false;
            }
            0xE001 => {
                eprintln!("[MMC3] $E001 <- {:#04x} (IRQ enable)", data);
                self.enable_irq = true;
            }
            _ => {}
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else if self.nametable_mirroring {
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
        prg_vram: &[u8],
        using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let bank = self.chr_bank(address);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                read_chr_byte(bank, address, &[], chr_ram)
            } else if !chr_rom.is_empty() {
                read_chr_byte(bank, address, chr_rom, &[])
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if self.nametable_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() {
                    prg_vram[idx]
                } else {
                    0
                }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let bank = self.chr_bank(address);
                let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
            }
        }
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        ppu_a12_prev: bool,
        scanline: u16,
        _dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        let a12 = (ppu_address_bus & 0x1000) != 0;
        let mut irq = false;
        if !ppu_a12_prev && a12 && self.m2_filter == 3 {
            irq |= self.clock_irq_counter();
            match self.config.irq_hack {
                Mmc3IrqHack::KickMaster if scanline == 238 => {
                    irq |= self.clock_irq_counter();
                }
                Mmc3IrqHack::PalStarWars if scanline == 240 => {
                    irq |= self.clock_irq_counter();
                }
                _ => {}
            }
        }
        if a12 {
            self.m2_filter = 0;
        }
        irq
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        let a12 = (ppu_address_bus & 0x1000) != 0;
        if !a12 && self.m2_filter < 3 {
            self.m2_filter += 1;
        }
        false
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
        false
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.push(self.r8000);
        state.push(self.bank_a);
        state.push(self.bank_8c);
        state.push(self.chr_2k0);
        state.push(self.chr_2k8);
        state.push(self.chr_1k0);
        state.push(self.chr_1k4);
        state.push(self.chr_1k8);
        state.push(self.chr_1kc);
        state.push(self.irq_latch);
        state.push(self.irq_counter);
        state.push(if self.enable_irq { 1 } else { 0 });
        state.push(if self.reload_irq_counter { 1 } else { 0 });
        state.push(if self.nametable_mirroring { 1 } else { 0 });
        state.push(self.prg_ram_protect);
        state.push(self.m2_filter);
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
        self.r8000 = state[p];
        p += 1;
        self.bank_a = state[p];
        p += 1;
        self.bank_8c = state[p];
        p += 1;
        self.chr_2k0 = state[p];
        p += 1;
        self.chr_2k8 = state[p];
        p += 1;
        self.chr_1k0 = state[p];
        p += 1;
        self.chr_1k4 = state[p];
        p += 1;
        self.chr_1k8 = state[p];
        p += 1;
        self.chr_1kc = state[p];
        p += 1;
        self.irq_latch = state[p];
        p += 1;
        self.irq_counter = state[p];
        p += 1;
        self.enable_irq = state[p] != 0;
        p += 1;
        self.reload_irq_counter = state[p] != 0;
        p += 1;
        self.nametable_mirroring = state[p] != 0;
        p += 1;
        self.prg_ram_protect = state[p];
        p += 1;
        self.m2_filter = state[p];
        p += 1;
        self.vsdip = state.get(p).copied().unwrap_or(0); p += 1;
        self.coinon = state.get(p).copied().unwrap_or(0); p += 1;
        self.coinon2 = state.get(p).copied().unwrap_or(0); p += 1;
        self.service = state.get(p).copied().unwrap_or(0); p += 1;
        p
    }
}
