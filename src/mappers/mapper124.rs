use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};
use crate::mappers::mmc1::{Mmc1Config, Mmc1Core, Mmc1Variant, mmc1_mirror_for_ppu};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, mmc3_chr_bank};

const MAPPER_UNROM: u8 = 0;
const MAPPER_AMROM: u8 = 1;
const MAPPER_MMC1: u8 = 2;
const MAPPER_MMC3: u8 = 3;

const OVERLAY_4K: usize = 0x385 * 0x1000;
const OVERLAY_8K: usize = 0x1C3 * 0x2000;
const OVERLAY_32K: usize = 0x71 * 0x8000;

const VS_FRAME_CYCLES: u64 = 29780;

pub struct Mapper124 {
    regb: u8,
    rega: u8,
    latch_data: u8,
    work_ram: [u8; 4096],
    mmc1: Mmc1Core,
    mmc3: MapperMMC3,
    combined: Vec<u8>,
    prg_len: usize,
    chr_len: usize,
    dip_switches: u8,
    coin_on: u8,
    cycle_accum: u64,
    irq_ack_pulse: bool,
}

fn combined_rom_from_ines(header: &[u8], rom: &[u8]) -> (Vec<u8>, usize) {
    let has_trainer = header.len() > 6 && (header[6] & 4) != 0;
    let data_start = 16 + if has_trainer { 512 } else { 0 };
    if data_start >= rom.len() {
        return (Vec::new(), 0);
    }
    let nes2 = header.len() >= 16 && (header[7] & 0x0C) == 0x08;
    let chr_size_val = if nes2 {
        (header[5] as usize) | (((header[9] & 0xF0) as usize) << 4)
    } else {
        header[5] as usize
    };
    let prg_rom_len = if header[4] == 0 {
        let chr_bytes = chr_size_val * 0x2000;
        let remaining = rom.len().saturating_sub(data_start + chr_bytes);
        remaining / 0x4000 * 0x4000
    } else if nes2 && (header[9] & 0x0F) == 0x0F {
        let lo = header[4] as usize;
        ((2 * (lo & 3) + 1) << (lo >> 2)) as usize
    } else if nes2 {
        let prg_size_12bit = (header[4] as usize) | (((header[9] & 0x0F) as usize) << 8);
        prg_size_12bit * 0x4000
    } else {
        (header[4] as usize) * 0x4000
    };
    let chr_rom_len = chr_size_val * 0x2000;
    let file_prg_avail = rom.len().saturating_sub(data_start + chr_rom_len);
    let header_has_misc = nes2 && header.len() > 14 && (header[14] & 0x03) != 0;
    let prg_len = if file_prg_avail > prg_rom_len && !header_has_misc {
        file_prg_avail
    } else {
        prg_rom_len
    };
    let total = prg_len + chr_rom_len;
    if data_start + total <= rom.len() {
        (rom[data_start..data_start + total].to_vec(), prg_len)
    } else {
        let data = rom[data_start..].to_vec();
        let prg_len = prg_len.min(data.len());
        (data, prg_len)
    }
}

impl Mapper124 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let mmc1_config = Mmc1Config {
            variant: Mmc1Variant::Mmc1A,
            serom: false,
            wram_size: 0,
            battery_wram_size: 0,
            snrom: false,
        };
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let mut mmc3_config = Mmc3Config::for_ines(header, 0, chr_size, rom, rom_name);
        mmc3_config.prg_ram_size = mmc3_config.prg_ram_size.max(0x2000);
        mmc3_config.ax5202p = true;
        let mut mmc3 = MapperMMC3::new(mmc3_config);
        mmc3.reset();
        let mut mmc1 = Mmc1Core::new(mmc1_config);
        mmc1.reset();
        let (combined, prg_len) = combined_rom_from_ines(header, rom);
        let chr_len = combined.len().saturating_sub(prg_len);
        Self {
            regb: 0,
            rega: 0,
            latch_data: 0,
            work_ram: [0; 4096],
            mmc1,
            mmc3,
            combined,
            prg_len,
            chr_len,
            dip_switches: 0,
            coin_on: 0,
            cycle_accum: 0,
            irq_ack_pulse: false,
        }
    }

    fn mapper(&self) -> u8 {
        self.regb >> 4 & 3
    }

    fn prg_or(&self) -> usize {
        (self.rega as usize) << 4 & 0x1F0
    }

    fn chr_or(&self) -> usize {
        (self.regb as usize) << 7 & 0x780
    }

    fn rom6(&self) -> bool {
        (self.rega & 0x20) != 0
    }

    fn rom8(&self) -> bool {
        (self.rega & 0x80) == 0
    }

    fn chrram(&self) -> bool {
        (self.rega & 0x40) == 0
    }

    fn read_full(&self, offset: usize) -> u8 {
        if self.combined.is_empty() {
            0
        } else {
            self.combined[offset % self.combined.len()]
        }
    }

    fn read_prg(&self, cart: &Cartridge, offset: usize) -> u8 {
        if cart.prg_rom.is_empty() {
            if self.prg_len == 0 {
                0
            } else {
                self.combined[offset % self.prg_len]
            }
        } else {
            cart.prg_rom[offset % cart.prg_rom.len()]
        }
    }

    fn mmc3_inner_bank(&self, page: usize) -> u8 {
        let invert = (self.mmc3.r8000 & 0x40) != 0;
        match page {
            0 => {
                if invert {
                    0xFE
                } else {
                    self.mmc3.bank_8c
                }
            }
            1 => self.mmc3.bank_a,
            2 => {
                if invert {
                    self.mmc3.bank_8c
                } else {
                    0xFE
                }
            }
            3 => 0xFF,
            _ => 0,
        }
    }

    fn mmc3_prg_bank(&self, page: usize) -> usize {
        let prg_and = if (self.rega & 0x20) != 0 {
            0x0F
        } else {
            0x1F
        };
        (self.mmc3_inner_bank(page) as usize & prg_and) | self.prg_or()
    }

    fn read_chr(&self, offset: usize) -> u8 {
        if self.chr_len == 0 {
            0
        } else {
            self.combined[self.prg_len + (offset % self.chr_len)]
        }
    }

    fn unrom_prg_offset(&self, address: u16) -> usize {
        let prg_or = (self.prg_or() >> 1) as usize;
        let bank16 = if address >= 0xC000 {
            prg_or | 7
        } else {
            prg_or | (self.latch_data as usize & 7)
        };
        bank16 * 0x4000 + (address as usize & 0x3FFF)
    }

    fn amrom_prg_offset(&self, address: u16) -> usize {
        let bank32 = ((self.prg_or() >> 2) as usize) | (self.latch_data as usize & 7);
        bank32 * 0x8000 + (address as usize & 0x7FFF)
    }

    fn mmc1_prg_offset(&self, address: u16) -> usize {
        let prg_or = (self.prg_or() >> 1) as usize;
        let prg_reg = (self.mmc1.prg & 0x0F) as usize;
        let mode = (self.mmc1.control >> 2) & 3;
        let bank16 = |index: usize| (index & 0x07) | prg_or;
        let bank = match mode {
            0 | 1 => {
                let half = if address >= 0xC000 { 1 } else { 0 };
                bank16((prg_reg & 0x0E) | half)
            }
            2 => {
                if address >= 0xC000 {
                    bank16(prg_reg)
                } else {
                    bank16(0)
                }
            }
            3 => {
                if address >= 0xC000 {
                    bank16(0x0F)
                } else {
                    bank16(prg_reg)
                }
            }
            _ => prg_or,
        };
        bank * 0x4000 + (address as usize & 0x3FFF)
    }

    fn mmc3_prg_offset(&self, address: u16) -> usize {
        let page = ((address - 0x8000) >> 13) as usize;
        self.mmc3_prg_bank(page) * 0x2000 + (address as usize & 0x1FFF)
    }

    fn game_prg_offset(&self, address: u16) -> usize {
        match self.mapper() {
            MAPPER_UNROM => self.unrom_prg_offset(address),
            MAPPER_AMROM => self.amrom_prg_offset(address),
            MAPPER_MMC1 => self.mmc1_prg_offset(address),
            MAPPER_MMC3 => self.mmc3_prg_offset(address),
            _ => address as usize & 0x7FFF,
        }
    }

    fn mmc1_chr_offset(&self, address: u16) -> usize {
        let chr_and = 0x1F;
        let chr_or = (self.chr_or() >> 2) as usize;
        let chr_mode = (self.mmc1.control >> 4) & 1;
        if chr_mode != 0 {
            let bank = if address < 0x1000 {
                ((self.mmc1.chr0 as usize) & chr_and) | chr_or
            } else {
                ((self.mmc1.chr1 as usize) & chr_and) | chr_or
            };
            bank * 0x1000 + (address as usize & 0xFFF)
        } else {
            let bank = (((self.mmc1.chr0 as usize) & 0xFE) & chr_and) | chr_or;
            bank * 0x1000 + (address as usize & 0x1FFF)
        }
    }

    fn mmc3_chr_bank_index(&self, address: u16) -> usize {
        let chr_and = if (self.regb & 0x40) != 0 {
            0x7F
        } else {
            0xFF
        };
        let chr_or = self.chr_or();
        let raw = mmc3_chr_bank(
            self.mmc3.r8000,
            self.mmc3.chr_2k0,
            self.mmc3.chr_2k8,
            self.mmc3.chr_1k0,
            self.mmc3.chr_1k4,
            self.mmc3.chr_1k8,
            self.mmc3.chr_1kc,
            address,
        ) as usize;
        (raw & chr_and) | chr_or
    }

    fn coin_dip_read(&self) -> u8 {
        let mut data = self.dip_switches;
        if self.coin_on > 0 {
            data |= 0x80;
        }
        data
    }

    fn write_asic(&mut self, cart: &mut Cartridge, address: u16, val: u8) {
        if (address & 0x10) != 0 {
            let _ = val;
        } else if (address & 1) != 0 {
            if self.rega != val {
                self.rega = val;
                self.on_mode_registers_changed(cart);
            }
        } else {
            let regb = val & 0x7F;
            if self.regb != regb {
                self.regb = regb;
                self.on_mode_registers_changed(cart);
            }
        }
    }

    fn on_mode_registers_changed(&mut self, cart: &mut Cartridge) {
        self.latch_data = 0;
        self.mmc1.reset();
        self.mmc3.reset();
        self.irq_ack_pulse = false;
        cart.mapper_cpu_cycle = 0;
    }

    fn latch_write(&mut self, cart: &Cartridge, address: u16, val: u8) {
        let offset = if self.rom8() {
            OVERLAY_32K + (address as usize & 0x7FFF)
        } else {
            self.game_prg_offset(address)
        };
        self.latch_data = val
            & if self.rom8() {
                self.read_full(offset)
            } else {
                self.read_prg(cart, offset)
            };
    }

    fn mirror_single_screen(upper: bool, address: u16) -> u16 {
        if upper {
            (address & 0x23FF) | 0x0400
        } else {
            address & 0x23FF
        }
    }
}

impl Mapper for Mapper124 {
    fn reset(&mut self) {
        self.mmc1.reset();
        self.mmc3.reset();
        self.latch_data = 0;
        self.irq_ack_pulse = false;
    }

    fn reset_power_cycle(&mut self) {
        self.regb = 0;
        self.rega = 0;
        self.latch_data = 0;
        self.mmc1.reset();
        self.mmc3.reset();
        self.coin_on = 0;
        self.cycle_accum = 0;
        self.irq_ack_pulse = false;
    }

    fn cpu_ram_override(&self, address: u16) -> Option<u8> {
        if address < 0x1000 {
            Some(self.work_ram[address as usize])
        } else {
            None
        }
    }

    fn cpu_ram_override_store(&mut self, address: u16, data: u8) -> bool {
        if address < 0x1000 {
            self.work_ram[address as usize] = data;
            true
        } else {
            false
        }
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x4000..=0x4FFF).contains(&address) && (address & 0xF0F) == 0xF0F {
            return FetchResult {
                data: self.coin_dip_read(),
                driven: true,
            };
        }
        if address >= 0x5000 && address < 0x6000 {
            return FetchResult {
                data: self.read_full(OVERLAY_4K + (address as usize & 0xFFF)),
                driven: true,
            };
        }
        if address >= 0x6000 && address < 0x8000 {
            if self.rom6() {
                return FetchResult {
                    data: self.read_full(OVERLAY_8K + (address as usize & 0x1FFF)),
                    driven: true,
                };
            }
            if !self.rom6() {
                let result = self.mmc3.fetch_prg(cart, address);
                if result.driven {
                    return result;
                }
            }
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        if address >= 0x8000 {
            let offset = if self.rom8() {
                OVERLAY_32K + (address as usize & 0x7FFF)
            } else {
                self.game_prg_offset(address)
            };
            let data = if self.rom8() {
                self.read_full(offset)
            } else {
                self.read_prg(cart, offset)
            };
            return FetchResult {
                data,
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, val: u8) {
        if address >= 0x5000 && address < 0x6000 {
            self.write_asic(cart, address, val);
            return;
        }
        if address >= 0x6000 && address < 0x8000 {
            if !self.rom6() {
                self.mmc3.store_prg(cart, address, val);
            }
            return;
        }
        if address >= 0x8000 {
            match self.mapper() {
                MAPPER_UNROM | MAPPER_AMROM => {
                    self.latch_write(cart, address, val);
                }
                MAPPER_MMC1 => {
                    self.mmc1
                        .write_register(cart, address, val, cart.mapper_cpu_cycle);
                }
                MAPPER_MMC3 => {
                    self.mmc3.store_prg(cart, address, val);
                    if (address & 0xE001) == 0xE000 {
                        self.irq_ack_pulse = true;
                    }
                }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        match self.mapper() {
            MAPPER_UNROM => mirror_h_or_v(false, address),
            MAPPER_AMROM => Self::mirror_single_screen((self.latch_data & 0x10) != 0, address),
            MAPPER_MMC1 => self.mmc1.mirror_nametable(cart, address),
            MAPPER_MMC3 => {
                if self.mmc3.nametable_mirroring() {
                    mirror_h_or_v(true, address)
                } else {
                    mirror_h_or_v(false, address)
                }
            }
            _ => mirror_h_or_v(cart.nametable_horizontal_mirroring, address),
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
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
            let byte = if self.chrram() && !chr_ram.is_empty() {
                chr_ram[(address as usize) % chr_ram.len()]
            } else {
                let offset = match self.mapper() {
                    MAPPER_MMC1 => self.mmc1_chr_offset(address),
                    MAPPER_MMC3 => {
                        self.mmc3_chr_bank_index(address) * 0x400 + (address as usize & 0x3FF)
                    }
                    _ => address as usize & 0x1FFF,
                };
                self.read_chr(offset)
            };
            new_addr_bus |= byte as u16;
        } else {
            let mir = match self.mapper() {
                MAPPER_MMC1 => {
                    mmc1_mirror_for_ppu(&self.mmc1, nametable_horizontal_mirroring, address)
                }
                MAPPER_UNROM => mirror_h_or_v(false, address),
                MAPPER_AMROM => {
                    Self::mirror_single_screen((self.latch_data & 0x10) != 0, address)
                }
                MAPPER_MMC3 => {
                    if self.mmc3.nametable_mirroring() {
                        mirror_h_or_v(true, address)
                    } else {
                        mirror_h_or_v(false, address)
                    }
                }
                _ => mirror_h_or_v(nametable_horizontal_mirroring, address),
            };
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if self.chrram() && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize) % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mir = self.mirror_nametable(cart, address);
            vram[(mir & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.cycle_accum += cycles as u64;
        if self.cycle_accum >= VS_FRAME_CYCLES {
            self.cycle_accum = 0;
            if self.coin_on > 0 {
                self.coin_on -= 1;
            }
        }
        match self.mapper() {
            MAPPER_MMC1 => self.mmc1.cpu_clock_irq(),
            _ => false,
        }
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        if self.mapper() == MAPPER_MMC3 {
            self.mmc3.cpu_clock_rise(ppu_address_bus);
        }
        false
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        ppu_sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        if self.mapper() == MAPPER_MMC3 {
            self.mmc3.ppu_clock(
                ppu_address_bus,
                ppu_a12_prev,
                scanline,
                dot,
                ppu_sprite_x16,
                rendering_on,
            )
        } else {
            false
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.mapper() != MAPPER_MMC3 {
            return false;
        }
        let ack = self.irq_ack_pulse;
        self.irq_ack_pulse = false;
        ack
    }

    fn insert_coin(&mut self, _coin: u8) {
        self.coin_on = 8;
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.latch_data);
        state.extend_from_slice(&self.mmc3.save_mapper_registers(cart));
        let mut mmc1_state = Vec::new();
        self.mmc1.append_save_state(&mut mmc1_state);
        state.extend_from_slice(&mmc1_state);
        state.push(self.regb);
        state.push(self.rega);
        state.extend_from_slice(&self.work_ram);
        state.push(self.dip_switches);
        state.push(self.coin_on);
        state.push(if self.irq_ack_pulse { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.latch_data = state[p];
            p += 1;
        }
        p = self.mmc3.load_mapper_registers(cart, state, p);
        p = self.mmc1.load_save_state(state, p);
        if p < state.len() {
            self.regb = state[p];
            p += 1;
        }
        if p < state.len() {
            self.rega = state[p];
            p += 1;
        }
        if p + 4096 <= state.len() {
            self.work_ram.copy_from_slice(&state[p..p + 4096]);
            p += 4096;
        }
        if p < state.len() {
            self.dip_switches = state[p];
            p += 1;
        }
        if p < state.len() {
            self.coin_on = state[p];
            p += 1;
        }
        if p < state.len() {
            self.irq_ack_pulse = state[p] != 0;
            p += 1;
        }
        p
    }
}
