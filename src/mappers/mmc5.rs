use crate::cartridge::Cartridge;
use crate::crc::crc32;
use crate::mapper::{FetchResult, Mapper};
const WRAM_INDEX_LEN: usize = 128;
const WRAM_INDEX_DISABLED: u8 = 255;
const MMC5_CART_WRAM: &[(u32, u8)] = &[
    (0x6f4e_4312, 4),
    (0x15fe_6d0f, 2),
    (0x671f_23a8, 0),
    (0xcd4e_7430, 0),
    (0xed24_65be, 0),
    (0xfe34_88d1, 2),
    (0x0ec6_c023, 1),
    (0x0afb_395e, 0),
    (0x1ced_086f, 2),
    (0x9cba_dc25, 1),
    (0x6396_b988, 2),
    (0x9c18_762b, 2),
    (0xb048_0ae9, 0),
    (0xb473_5fac, 0),
    (0xf540_677b, 4),
    (0xeee9_a682, 2),
    (0xf9b4_240f, 2),
    (0x8ce4_78db, 2),
    (0xf011_e490, 4),
    (0xbc80_fb52, 1),
    (0x184c_2124, 4),
    (0xee8e_6553, 4),
    (0xd532_e98f, 1),
    (0x39f2_ce4b, 2),
    (0xbb7f_829a, 0),
    (0xaca1_5643, 2),
];

#[derive(Clone, Debug)]
pub struct Mmc5Config {
    pub wram_size: usize,
    pub battery_save_size: usize,
}

impl Mmc5Config {
    pub fn for_ines(header: &[u8], rom: &[u8], has_battery: bool) -> Self {
        let nes2 = header.len() >= 16 && (header[7] & 0x0C) == 0x08;
        let wram_kb = if nes2 {
            let volatile_kb = nes20_ram_kb(header[10] & 0x0F);
            let battery_kb = nes20_ram_kb((header[10] >> 4) & 0x0F);
            volatile_kb + battery_kb
        } else {
            detect_mmc5_wram_kb(ines_image_crc(header, rom))
        };
        let wram_size = wram_kb * 1024;
        let battery_save_size = if !has_battery {
            0
        } else if nes2 {
            nes20_ram_kb((header[10] >> 4) & 0x0F) * 1024
        } else if wram_kb <= 16 {
            8192
        } else if wram_kb >= 64 {
            64 * 1024
        } else {
            32768
        };
        Self {
            wram_size,
            battery_save_size,
        }
    }
}

fn nes20_ram_kb(shift: u8) -> usize {
    if shift == 0 {
        0
    } else {
        (64usize << shift) / 1024
    }
}

fn ines_image_crc(header: &[u8], rom: &[u8]) -> u32 {
    let has_trainer = header.len() >= 16 && (header[6] & 4) != 0;
    let trainer_len = if has_trainer { 512 } else { 0 };
    let prg_size = header[4] as usize * 0x4000;
    let chr_size = header[5] as usize * 0x2000;
    let start = 16 + trainer_len;
    let end = (start + prg_size + chr_size).min(rom.len());
    if start >= end {
        return 0;
    }
    crc32(&rom[start..end])
}

fn detect_mmc5_wram_kb(crc: u32) -> usize {
    for &(c, size) in MMC5_CART_WRAM {
        if crc == c {
            return size as usize * 8;
        }
    }
    64
}

fn build_wram_index_table(wram_banks_8k: u8) -> [u8; WRAM_INDEX_LEN] {
    let mut table = [WRAM_INDEX_DISABLED; WRAM_INDEX_LEN];
    let mut other = false;
    for x in 0..8usize {
        table[x] = match wram_banks_8k {
            0 => WRAM_INDEX_DISABLED,
            1 => {
                if x > 3 {
                    WRAM_INDEX_DISABLED
                } else {
                    0
                }
            }
            2 => ((x & 4) >> 2) as u8,
            4 => {
                if x > 3 {
                    WRAM_INDEX_DISABLED
                } else {
                    (x & 3) as u8
                }
            }
            8 => x as u8,
            _ => {
                other = true;
                x as u8
            }
        };
    }
    if other {
        let banks = wram_banks_8k as usize;
        for x in 0..WRAM_INDEX_LEN.min(banks) {
            table[x] = x as u8;
        }
        for x in banks..WRAM_INDEX_LEN {
            table[x] = table[x - banks];
        }
    } else {
        for x in 8..WRAM_INDEX_LEN {
            table[x] = table[x & 7];
        }
    }
    table
}

#[derive(Clone)]
struct Mmc5PrgBank {
    is_rom: bool,
    offset: usize,
}

pub struct MapperMMC5 {
    cfg: Mmc5Config,
    wram_index: [u8; WRAM_INDEX_LEN],
    prg_banks: [u8; 4],
    wram_page: u8,
    chr_banks_a: [u16; 8],
    chr_banks_b: [u16; 4],
    prg_mode: u8,
    chr_mode: u8,
    exram_mode: u8,
    nametable_mirroring: u8,
    fill_tile: u8,
    fill_attr: u8,
    wram_mask_enable: [u8; 2],
    chr_high_bits: u8,
    mul_op1: u8,
    mul_op2: u8,
    exram: [u8; 1024],
    irq_scanline: u8,
    irq_enable: bool,
    irq_pending: bool,
    irq_in_frame: bool,
    irq_line_counter: u8,
    ppu_scanline: u16,
    ppu_dot: u16,
    large_sprites: bool,
    rendering_on: bool,
    nt_refresh_addr: u16,
    ex_attribute_last_nametable_fetch: u16,
    ex_attr_last_fetch_counter: i32,
    ex_attr_selected_chr_bank: u16,
    split_in_split_region: bool,
    split_tile: u16,
    split_tile_number: i32,
    active_prg: [Mmc5PrgBank; 4],
    prg_dirty: bool,
    mmc5_ab_mode: u8,
    split_mode: u8,
    split_scroll: u8,
    split_page: u8,
    sound_enable: u8,
    sound_running: u8,
    sound_wl: [u16; 2],
    sound_env: [u8; 2],
    sound_dcount: [i32; 2],
    sound_vcount: [i32; 2],
    sound_raw: u8,
    sound_raw_control: u8,
    current_audio_sample: f32,
    ppu_in_frame: bool,
    ppu_idle_counter: u32,
}

impl MapperMMC5 {
    pub fn new(cfg: Mmc5Config) -> Self {
        let wram_banks_8k = (cfg.wram_size / 8192).min(64) as u8;
        let dummy = Mmc5PrgBank {
            is_rom: true,
            offset: 0,
        };
        let mapper = Self {
            cfg,
            wram_index: build_wram_index_table(wram_banks_8k),
            prg_banks: [0xFF; 4],
            wram_page: 0,
            chr_banks_a: [0; 8],
            chr_banks_b: [0; 4],
            prg_mode: 3,
            chr_mode: 3,
            exram_mode: 0,
            nametable_mirroring: 0,
            fill_tile: 0,
            fill_attr: 0,
            wram_mask_enable: [0xFF, 0xFF],
            chr_high_bits: 0,
            mul_op1: 0,
            mul_op2: 0,
            exram: [0; 1024],
            irq_scanline: 0,
            irq_enable: false,
            irq_pending: false,
            irq_in_frame: false,
            irq_line_counter: 0,
            ppu_scanline: 0,
            ppu_dot: 0,
            large_sprites: false,
            rendering_on: false,
            nt_refresh_addr: 0,
            ex_attribute_last_nametable_fetch: 0,
            ex_attr_last_fetch_counter: 0,
            ex_attr_selected_chr_bank: 0,
            split_in_split_region: false,
            split_tile: 0,
            split_tile_number: -1,
            active_prg: [dummy.clone(), dummy.clone(), dummy.clone(), dummy],
            prg_dirty: true,
            mmc5_ab_mode: 0,
            split_mode: 0,
            split_scroll: 0,
            split_page: 0,
            sound_enable: 0,
            sound_running: 0,
            sound_wl: [0; 2],
            sound_env: [0; 2],
            sound_dcount: [0; 2],
            sound_vcount: [0; 2],
            sound_raw: 0,
            sound_raw_control: 0,
            current_audio_sample: 0.0,
            ppu_in_frame: false,
            ppu_idle_counter: 0,
        };
        mapper
    }

    fn wram_write_enabled(&self) -> bool {
        ((self.wram_mask_enable[0] & 3) | ((self.wram_mask_enable[1] & 3) << 2)) == 6
    }

    fn resolve_wram_offset(&self, page: u8, addr_lo: usize, wram_len: usize) -> Option<usize> {
        if wram_len == 0 {
            return None;
        }
        let bank = self.wram_index[(page & 0x7F) as usize];
        if bank == WRAM_INDEX_DISABLED {
            return None;
        }
        let base = (bank as usize) * 0x2000;
        Some((base + addr_lo) % wram_len)
    }

    fn rebuild_prg(&mut self, prg_rom_len: usize) {
        if !self.prg_dirty {
            return;
        }
        self.prg_dirty = false;
        let wram_len = self.cfg.wram_size;
        match self.prg_mode & 3 {
            0 => {
                let rom_bank = ((self.prg_banks[3] & 0x7F) >> 2) as usize;
                let offset = (rom_bank * 0x8000) % prg_rom_len.max(1);
                for i in 0..4 {
                    self.active_prg[i] = Mmc5PrgBank {
                        is_rom: true,
                        offset: offset + i * 0x2000,
                    };
                }
            }
            1 => {
                if (self.prg_banks[1] & 0x80) != 0 {
                    let rom_bank = ((self.prg_banks[1] & 0x7F) >> 1) as usize;
                    let offset = (rom_bank * 0x4000) % prg_rom_len.max(1);
                    self.active_prg[0] = Mmc5PrgBank {
                        is_rom: true,
                        offset,
                    };
                    self.active_prg[1] = Mmc5PrgBank {
                        is_rom: true,
                        offset: offset + 0x2000,
                    };
                } else {
                    let page = self.prg_banks[1] & 0x7E;
                    self.active_prg[0] = Mmc5PrgBank {
                        is_rom: false,
                        offset: self
                            .resolve_wram_offset(page, 0, wram_len)
                            .unwrap_or(0),
                    };
                    self.active_prg[1] = Mmc5PrgBank {
                        is_rom: false,
                        offset: self
                            .resolve_wram_offset(page.wrapping_add(1), 0, wram_len)
                            .unwrap_or(0),
                    };
                }
                let rom_bank = ((self.prg_banks[3] & 0x7F) >> 1) as usize;
                let offset = (rom_bank * 0x4000) % prg_rom_len.max(1);
                self.active_prg[2] = Mmc5PrgBank {
                    is_rom: true,
                    offset,
                };
                self.active_prg[3] = Mmc5PrgBank {
                    is_rom: true,
                    offset: offset + 0x2000,
                };
            }
            2 => {
                if (self.prg_banks[1] & 0x80) != 0 {
                    let rom_bank = ((self.prg_banks[1] & 0x7F) >> 1) as usize;
                    let offset = (rom_bank * 0x4000) % prg_rom_len.max(1);
                    self.active_prg[0] = Mmc5PrgBank {
                        is_rom: true,
                        offset,
                    };
                    self.active_prg[1] = Mmc5PrgBank {
                        is_rom: true,
                        offset: offset + 0x2000,
                    };
                } else {
                    let page = self.prg_banks[1] & 0x7E;
                    self.active_prg[0] = Mmc5PrgBank {
                        is_rom: false,
                        offset: self
                            .resolve_wram_offset(page, 0, wram_len)
                            .unwrap_or(0),
                    };
                    self.active_prg[1] = Mmc5PrgBank {
                        is_rom: false,
                        offset: self
                            .resolve_wram_offset(page.wrapping_add(1), 0, wram_len)
                            .unwrap_or(0),
                    };
                }
                if (self.prg_banks[2] & 0x80) != 0 {
                    let rom_bank = (self.prg_banks[2] & 0x7F) as usize;
                    let offset = (rom_bank * 0x2000) % prg_rom_len.max(1);
                    self.active_prg[2] = Mmc5PrgBank {
                        is_rom: true,
                        offset,
                    };
                } else {
                    self.active_prg[2] = Mmc5PrgBank {
                        is_rom: false,
                        offset: self
                            .resolve_wram_offset(self.prg_banks[2] & 0x7F, 0, wram_len)
                            .unwrap_or(0),
                    };
                }
                let rom_bank = (self.prg_banks[3] & 0x7F) as usize;
                let offset = (rom_bank * 0x2000) % prg_rom_len.max(1);
                self.active_prg[3] = Mmc5PrgBank {
                    is_rom: true,
                    offset,
                };
            }
            3 | _ => {
                for x in 0..3 {
                    if (self.prg_banks[x] & 0x80) != 0 {
                        let rom_bank = (self.prg_banks[x] & 0x7F) as usize;
                        let offset = (rom_bank * 0x2000) % prg_rom_len.max(1);
                        self.active_prg[x] = Mmc5PrgBank {
                            is_rom: true,
                            offset,
                        };
                    } else {
                        self.active_prg[x] = Mmc5PrgBank {
                            is_rom: false,
                            offset: self
                                .resolve_wram_offset(self.prg_banks[x] & 0x7F, 0, wram_len)
                                .unwrap_or(0),
                        };
                    }
                }
                let rom_bank = (self.prg_banks[3] & 0x7F) as usize;
                let offset = (rom_bank * 0x2000) % prg_rom_len.max(1);
                self.active_prg[3] = Mmc5PrgBank {
                    is_rom: true,
                    offset,
                };
            }
        }
    }

    fn mark_prg_dirty(&mut self) {
        self.prg_dirty = true;
    }

    fn apply_chr_high_bits(&mut self) {
        let hi = ((self.chr_high_bits & 3) as u16) << 8;
        for bank in &mut self.chr_banks_a {
            *bank = (*bank & 0xFF) | hi;
        }
        for bank in &mut self.chr_banks_b {
            *bank = (*bank & 0xFF) | hi;
        }
    }

    fn chr_bank_offset_a(&self, addr: u16, chr_len: usize) -> usize {
        let page = match self.chr_mode & 3 {
            0 => self.chr_banks_a[7],
            1 => {
                if addr < 0x1000 {
                    self.chr_banks_a[3]
                } else {
                    self.chr_banks_a[7]
                }
            }
            2 => {
                let bank_idx = (addr >> 11) as usize;
                self.chr_banks_a[bank_idx * 2 + 1]
            }
            3 | _ => {
                let bank_idx = (addr >> 10) as usize;
                self.chr_banks_a[bank_idx]
            }
        };
        let size = [0x2000, 0x1000, 0x800, 0x400][(self.chr_mode & 3) as usize];
        (page as usize * size + (addr as usize & (size - 1))) % chr_len.max(1)
    }

    fn chr_bank_offset_b(&self, addr: u16, chr_len: usize) -> usize {
        let page = match self.chr_mode & 3 {
            0 => self.chr_banks_b[3],
            1 => self.chr_banks_b[3],
            2 => {
                let bank_idx = ((addr >> 11) & 1) as usize;
                self.chr_banks_b[bank_idx * 2 + 1]
            }
            3 | _ => {
                let bank_idx = (addr >> 10) as usize;
                self.chr_banks_b[bank_idx & 3]
            }
        };
        let size = [0x2000, 0x1000, 0x800, 0x400][(self.chr_mode & 3) as usize];
        (page as usize * size + (addr as usize & (size - 1))) % chr_len.max(1)
    }

    fn chr_use_bank_b(&self) -> bool {
        if self.large_sprites {
            self.rendering_on && !(self.ppu_dot >= 257 && self.ppu_dot <= 320)
        } else {
            self.mmc5_ab_mode != 0
        }
    }

    fn read_chr(
        &self,
        addr: u16,
        chr_rom: &[u8],
        chr_ram: &[u8],
        using_chr_ram: bool,
    ) -> u8 {
        let len = if using_chr_ram {
            chr_ram.len()
        } else {
            chr_rom.len()
        };
        if len == 0 {
            return 0;
        }
        let offset = if self.chr_use_bank_b() {
            self.chr_bank_offset_b(addr, len)
        } else {
            self.chr_bank_offset_a(addr, len)
        };
        if using_chr_ram {
            chr_ram[offset]
        } else {
            chr_rom[offset]
        }
    }

    fn split_enabled(&self) -> bool {
        (self.split_mode & 0x80) != 0 && self.exram_mode <= 1
    }

    fn update_split_region(&mut self) {
        self.split_in_split_region = false;
        if !self.split_enabled() {
            return;
        }
        let ht = (self.nt_refresh_addr >> 5) & 31;
        let target = (self.split_mode & 0x1F) as u16;
        let right_side = (self.split_mode & 0x40) != 0;
        let in_split = if right_side {
            ht >= target
        } else {
            ht < target
        };
        if in_split {
            self.split_in_split_region = true;
            let vertical = (self.split_scroll as u16 + self.ppu_scanline) % 240;
            self.split_tile = ((vertical & 0xF8) << 2) | ht;
        }
    }

    fn split_line_tile(&self) -> u16 {
        let sl = self.ppu_scanline as i32;
        let line = ((sl - 1).max(0) as u32 / 8) + self.split_scroll as u32;
        (line & 31) as u16
    }

    fn read_nametable_byte(&self, norm_addr: u16, vram: &[u8]) -> u8 {
        let quadrant = ((norm_addr - 0x2000) >> 10) & 3;
        let mode = (self.nametable_mirroring >> (quadrant * 2)) & 3;
        match mode {
            0 => vram[norm_addr as usize & 0x3FF],
            1 => vram[0x0400 | (norm_addr as usize & 0x3FF)],
            2 => {
                if self.exram_mode <= 1 {
                    self.exram[norm_addr as usize & 0x3FF]
                } else {
                    0
                }
            }
            3 | _ => {
                if (norm_addr & 0x3FF) < 0x3C0 {
                    self.fill_tile
                } else {
                    let attr = self.fill_attr & 3;
                    attr | (attr << 2) | (attr << 4) | (attr << 6)
                }
            }
        }
    }

    fn mmc5_hblank_irq(&mut self, scanline: u16) -> bool {
        let sl = scanline.wrapping_add(1);
        if !self.rendering_on || sl >= 241 {
            self.irq_pending = false;
            self.irq_in_frame = false;
            self.irq_line_counter = 0;
            return false;
        }
        if !self.irq_in_frame {
            self.irq_in_frame = true;
            self.irq_pending = false;
            self.irq_line_counter = 0;
            return false;
        }
        self.irq_line_counter = self.irq_line_counter.wrapping_add(1);
        if self.irq_line_counter == self.irq_scanline {
            self.irq_pending = true;
            return self.irq_enable;
        }
        false
    }

    fn power_on(&mut self) {
        self.prg_banks = [0xFF; 4];
        self.wram_page = 0;
        self.chr_banks_a = [0; 8];
        self.chr_banks_b = [0; 4];
        self.prg_mode = 3;
        self.chr_mode = 3;
        self.exram_mode = 0;
        self.nametable_mirroring = 0;
        self.fill_tile = 0;
        self.fill_attr = 0;
        self.wram_mask_enable = [0xFF, 0xFF];
        self.chr_high_bits = 0;
        self.mul_op1 = 0;
        self.mul_op2 = 0;
        self.irq_scanline = 0;
        self.irq_enable = false;
        self.irq_pending = false;
        self.irq_in_frame = false;
        self.irq_line_counter = 0;
        self.mmc5_ab_mode = 0;
        self.split_mode = 0;
        self.split_scroll = 0;
        self.split_page = 0;
        self.sound_enable = 0;
        self.sound_running = 0;
        self.sound_wl = [0; 2];
        self.sound_env = [0; 2];
        self.sound_dcount = [0; 2];
        self.sound_vcount = [0; 2];
        self.sound_raw = 0;
        self.sound_raw_control = 0;
        self.mark_prg_dirty();
    }
}

impl Mapper for MapperMMC5 {
    fn reset(&mut self) {
        self.power_on();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if self.prg_dirty {
            self.rebuild_prg(cart.prg_rom.len());
        }
        if address >= 0x5200 && address <= 0x5206 {
            return match address {
                0x5204 => {
                    let status = (if self.irq_pending { 0x80 } else { 0 })
                        | (if self.irq_in_frame { 0x40 } else { 0 });
                    self.irq_pending = false;
                    FetchResult {
                        data: status,
                        driven: true,
                    }
                }
                0x5205 => {
                    let result = (self.mul_op1 as u32 * self.mul_op2 as u32) & 0xFF;
                    FetchResult {
                        data: result as u8,
                        driven: true,
                    }
                }
                0x5206 => {
                    let result =
                        ((self.mul_op1 as u32 * self.mul_op2 as u32) >> 8) & 0xFF;
                    FetchResult {
                        data: result as u8,
                        driven: true,
                    }
                }
                _ => FetchResult {
                    data: 0,
                    driven: false,
                },
            };
        }
        if address >= 0x5C00 && address <= 0x5FFF {
            return FetchResult {
                data: self.exram[(address & 0x3FF) as usize],
                driven: true,
            };
        }
        if address >= 0x6000 && address < 0x8000 {
            if let Some(offset) =
                self.resolve_wram_offset(self.wram_page, address as usize & 0x1FFF, cart.prg_ram.len())
            {
                return FetchResult {
                    data: cart.prg_ram[offset],
                    driven: true,
                };
            }
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        if address >= 0x8000 {
            let bank_idx = ((address - 0x8000) >> 13) as usize;
            let bank = &self.active_prg[bank_idx];
            let lo = address as usize & 0x1FFF;
            if bank.is_rom {
                let len = cart.prg_rom.len();
                let data = if len == 0 {
                    0
                } else {
                    cart.prg_rom[(bank.offset + lo) % len]
                };
                return FetchResult { data, driven: true };
            }
            if cart.prg_ram.is_empty() {
                return FetchResult {
                    data: 0,
                    driven: false,
                };
            }
            return FetchResult {
                data: cart.prg_ram[(bank.offset + lo) % cart.prg_ram.len()],
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5100 && address <= 0x5206 {
            match address {
                0x5100 => {
                    self.prg_mode = data;
                    self.mark_prg_dirty();
                }
                0x5101 => self.chr_mode = data,
                0x5102 => self.wram_mask_enable[0] = data,
                0x5103 => self.wram_mask_enable[1] = data,
                0x5104 => self.exram_mode = data,
                0x5105 => self.nametable_mirroring = data,
                0x5106 => self.fill_tile = data,
                0x5107 => self.fill_attr = data,
                0x5113 => self.wram_page = data,
                0x5114 | 0x5115 | 0x5116 | 0x5117 => {
                    self.prg_banks[(address & 3) as usize] = data;
                    self.mark_prg_dirty();
                }
                0x5120..=0x5127 => {
                    self.mmc5_ab_mode = 0;
                    let idx = (address & 7) as usize;
                    self.chr_banks_a[idx] =
                        data as u16 | ((self.chr_high_bits & 3) as u16) << 8;
                }
                0x5128..=0x512B => {
                    self.mmc5_ab_mode = 1;
                    let idx = (address & 3) as usize;
                    self.chr_banks_b[idx] =
                        data as u16 | ((self.chr_high_bits & 3) as u16) << 8;
                }
                0x5130 => {
                    self.chr_high_bits = data;
                    self.apply_chr_high_bits();
                }
                0x5200 => self.split_mode = data,
                0x5201 => self.split_scroll = data & 0x1F,
                0x5202 => self.split_page = data & 0x3F,
                0x5203 => {
                    self.irq_scanline = data;
                    self.irq_pending = false;
                }
                0x5204 => {
                    self.irq_enable = (data & 0x80) != 0;
                    self.irq_pending = false;
                }
                0x5205 => self.mul_op1 = data,
                0x5206 => self.mul_op2 = data,
                _ => {}
            }
            if self.prg_dirty {
                self.rebuild_prg(cart.prg_rom.len());
            }
            return;
        }
        if address >= 0x5000 && address <= 0x5015 {
            match address {
                0x5000 => self.sound_env[0] = data,
                0x5002 => {
                    self.sound_wl[0] &= !0x00FF;
                    self.sound_wl[0] |= data as u16;
                }
                0x5003 => {
                    self.sound_wl[0] &= !0x0700;
                    self.sound_wl[0] |= ((data & 0x07) as u16) << 8;
                    self.sound_running |= 1;
                }
                0x5004 => self.sound_env[1] = data,
                0x5006 => {
                    self.sound_wl[1] &= !0x00FF;
                    self.sound_wl[1] |= data as u16;
                }
                0x5007 => {
                    self.sound_wl[1] &= !0x0700;
                    self.sound_wl[1] |= ((data & 0x07) as u16) << 8;
                    self.sound_running |= 2;
                }
                0x5010 => self.sound_raw_control = data,
                0x5011 => self.sound_raw = data,
                0x5015 => {
                    self.sound_enable = data;
                    self.sound_running &= data;
                }
                _ => {}
            }
            return;
        }
        if address >= 0x5C00 && address <= 0x5FFF {
            if self.exram_mode != 3 {
                let mut val = data;
                if self.exram_mode <= 1 && !self.ppu_in_frame {
                    val = 0;
                }
                self.exram[(address & 0x3FF) as usize] = val;
            }
            return;
        }
        if address >= 0x6000 && address < 0x8000 {
            if self.wram_write_enabled() {
                if let Some(offset) = self.resolve_wram_offset(
                    self.wram_page,
                    address as usize & 0x1FFF,
                    cart.prg_ram.len(),
                ) {
                    cart.prg_ram[offset] = data;
                }
            }
            return;
        }
        if address >= 0x8000 && self.wram_write_enabled() {
            if self.prg_dirty {
                self.rebuild_prg(cart.prg_rom.len());
            }
            let bank_idx = ((address - 0x8000) >> 13) as usize;
            let bank = &self.active_prg[bank_idx];
            if !bank.is_rom && !cart.prg_ram.is_empty() {
                let offset = (bank.offset + (address as usize & 0x1FFF)) % cart.prg_ram.len();
                cart.prg_ram[offset] = data;
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        let norm = address & 0x2FFF;
        let quadrant = ((norm - 0x2000) >> 10) & 3;
        let mode = (self.nametable_mirroring >> (quadrant * 2)) & 3;
        let base = match mode {
            0 => norm & 0x3FF,
            1 => 0x0400 | (norm & 0x3FF),
            2 => norm & 0x3FF,
            3 | _ => norm & 0x3FF,
        };
        let mirrored = 0x2000 | base;
        if cart.nametable_horizontal_mirroring {
            (mirrored & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            mirrored & 0x37FF
        }
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram {
                cart.chr_ram[address as usize & 0x1FFF] = data;
            }
            return;
        }
        if address >= 0x2000 && address < 0x3F00 {
            let norm_addr = address & 0x2FFF;
            let quadrant = ((norm_addr - 0x2000) >> 10) & 3;
            let mode = (self.nametable_mirroring >> (quadrant * 2)) & 3;
            match mode {
                0 => {
                    vram[norm_addr as usize & 0x3FF] = data;
                }
                1 => {
                    vram[0x0400 | (norm_addr as usize & 0x3FF)] = data;
                }
                2 => {
                    if self.exram_mode <= 1 {
                        self.exram[norm_addr as usize & 0x3FF] = data;
                    }
                }
                3 | _ => {}
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
        self.ppu_in_frame = true;
        self.ppu_idle_counter = 9;
        let address = ppu_address_bus;
        let mut new_addr_bus = address & 0x3FFF;
        let mut data = ppu_octal_latch;
        let vertical_split_scroll = (self.split_scroll as u16 + self.ppu_scanline) % 240;
        let is_nt_fetch = address >= 0x2000 && address <= 0x2FFF && (address & 0x3FF) < 0x3C0;
        if is_nt_fetch {
            self.split_tile_number += 1;
            self.nt_refresh_addr = address;
            self.update_split_region();
        }
        let ex_gfx_active = self.exram_mode == 1
            && self.ppu_scanline < 240
            && (self.split_tile_number < 32 || self.split_tile_number >= 40);
        if address < 0x2000 {
            if self.split_in_split_region {
                let chr_len = if using_chr_ram {
                    chr_ram.len()
                } else {
                    chr_rom.len()
                };
                if chr_len > 0 {
                    let offset = (self.split_page as usize * 0x1000
                        + (((address & !0x07) | (vertical_split_scroll & 0x07)) as usize
                            & 0x0FFF))
                        % chr_len;
                    data = if using_chr_ram {
                        chr_ram[offset]
                    } else {
                        chr_rom[offset]
                    };
                }
            } else if ex_gfx_active && self.ex_attr_last_fetch_counter > 0 {
                self.ex_attr_last_fetch_counter -= 1;
                let chr_len = if using_chr_ram {
                    chr_ram.len()
                } else {
                    chr_rom.len()
                };
                if chr_len > 0 {
                    let offset = (self.ex_attr_selected_chr_bank as usize * 0x1000
                        + (address as usize & 0x0FFF))
                        % chr_len;
                    data = if using_chr_ram {
                        chr_ram[offset]
                    } else {
                        chr_rom[offset]
                    };
                }
            } else {
                data = self.read_chr(address, chr_rom, chr_ram, using_chr_ram);
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let norm_addr = address & 0x2FFF;
            if self.split_in_split_region {
                let line_tile = self.split_line_tile();
                if (norm_addr & 0x3FF) >= 0x3C0 {
                    let mut a = norm_addr & 0x3FF;
                    a &= !((0x1C) << 1);
                    a |= ((line_tile & 0x1C) << 1) as u16;
                    data = self.exram[a as usize & 0x3FF];
                } else {
                    let mut a = norm_addr & 0x3FF;
                    a &= !((0x1F << 5) | (1 << 0xB));
                    a |= (line_tile & 31) << 5;
                    data = self.exram[a as usize & 0x3FF];
                }
            } else if ex_gfx_active {
                if is_nt_fetch {
                    self.ex_attribute_last_nametable_fetch = norm_addr & 0x3FF;
                    self.ex_attr_last_fetch_counter = 3;
                    data = self.read_nametable_byte(norm_addr, vram);
                } else if self.ex_attr_last_fetch_counter > 0 {
                    self.ex_attr_last_fetch_counter -= 1;
                    if self.ex_attr_last_fetch_counter == 2 {
                        let exram_val =
                            self.exram[self.ex_attribute_last_nametable_fetch as usize & 0x3FF];
                        self.ex_attr_selected_chr_bank = (exram_val & 0x3F) as u16
                            | ((self.chr_high_bits & 0x03) as u16) << 6;
                        let pal = (exram_val >> 6) & 3;
                        data = pal | (pal << 2) | (pal << 4) | (pal << 6);
                    }
                }
            } else {
                data = self.read_nametable_byte(norm_addr, vram);
            }
        }
        new_addr_bus = (new_addr_bus & 0xFF00) | data as u16;
        (data, new_addr_bus)
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        ppu_sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        self.ppu_scanline = scanline;
        self.ppu_dot = dot;
        self.large_sprites = ppu_sprite_x16;
        self.rendering_on = rendering_on;
        if ppu_address_bus >= 0x2000 && (ppu_address_bus & 0x3FF) < 0x3C0 {
            self.nt_refresh_addr = ppu_address_bus;
        }
        if self.ppu_idle_counter > 0 {
            self.ppu_idle_counter -= 1;
            if self.ppu_idle_counter == 0 {
                self.ppu_in_frame = false;
            }
        }
        if dot == 0 {
            self.split_tile_number = -1;
            self.ex_attr_last_fetch_counter = 0;
            self.split_in_split_region = false;
            self.split_tile = 0;
        }
        if dot == 257 {
            return self.mmc5_hblank_irq(scanline);
        }
        false
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        for i in 0..2 {
            if self.sound_wl[i] >= 8
                && (self.sound_running & (1 << i)) != 0
                && (self.sound_enable & (1 << i)) != 0
            {
                self.sound_vcount[i] -= 1;
                if self.sound_vcount[i] <= 0 {
                    self.sound_vcount[i] = (self.sound_wl[i] as i32 + 1) * 2;
                    self.sound_dcount[i] = (self.sound_dcount[i] + 1) & 7;
                }
            }
        }
        let mut sq0_val = 0.0;
        let mut sq1_val = 0.0;
        let tal = [1, 2, 4, 6];
        if self.sound_wl[0] >= 8
            && (self.sound_running & 1) != 0
            && (self.sound_enable & 1) != 0
        {
            let duty = (self.sound_env[0] >> 6) & 3;
            if self.sound_dcount[0] < tal[duty as usize] {
                sq0_val = (self.sound_env[0] & 0xF) as f32;
            }
        }
        if self.sound_wl[1] >= 8
            && (self.sound_running & 2) != 0
            && (self.sound_enable & 2) != 0
        {
            let duty = (self.sound_env[1] >> 6) & 3;
            if self.sound_dcount[1] < tal[duty as usize] {
                sq1_val = (self.sound_env[1] & 0xF) as f32;
            }
        }
        let pcm_val = if (self.sound_raw_control & 0x40) == 0 {
            self.sound_raw as f32
        } else {
            0.0
        };
        self.current_audio_sample =
            (sq0_val + sq1_val) * 0.03 + (pcm_val / 255.0) * 0.15;
        false
    }

    fn audio_sample(&self) -> f32 {
        self.current_audio_sample
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg_banks);
        state.push(self.wram_page);
        for &b in &self.chr_banks_a {
            state.push((b & 0xFF) as u8);
            state.push((b >> 8) as u8);
        }
        for &b in &self.chr_banks_b {
            state.push((b & 0xFF) as u8);
            state.push((b >> 8) as u8);
        }
        state.push(self.prg_mode);
        state.push(self.chr_mode);
        state.push(self.exram_mode);
        state.push(self.nametable_mirroring);
        state.push(self.fill_tile);
        state.push(self.fill_attr);
        state.extend_from_slice(&self.wram_mask_enable);
        state.push(self.chr_high_bits);
        state.push(self.mul_op1);
        state.push(self.mul_op2);
        state.extend_from_slice(&self.exram);
        state.push(self.irq_scanline);
        state.push(if self.irq_enable { 1 } else { 0 });
        state.push(if self.irq_pending { 1 } else { 0 });
        state.push(if self.irq_in_frame { 1 } else { 0 });
        state.push(self.irq_line_counter);
        state.push(self.mmc5_ab_mode);
        state.push(self.split_mode);
        state.push(self.split_scroll);
        state.push(self.split_page);
        state.extend_from_slice(&self.sound_wl[0].to_le_bytes());
        state.extend_from_slice(&self.sound_wl[1].to_le_bytes());
        state.extend_from_slice(&self.sound_env);
        state.push(self.sound_enable);
        state.push(self.sound_running);
        state.extend_from_slice(&self.sound_dcount[0].to_le_bytes());
        state.extend_from_slice(&self.sound_dcount[1].to_le_bytes());
        state.extend_from_slice(&self.sound_vcount[0].to_le_bytes());
        state.extend_from_slice(&self.sound_vcount[1].to_le_bytes());
        state.push(self.sound_raw);
        state.push(self.sound_raw_control);
        state.extend_from_slice(&(cart.prg_ram.len() as u32).to_le_bytes());
        state.extend_from_slice(&cart.prg_ram);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if state.len() < p + 4 {
            return p;
        }
        self.prg_banks.copy_from_slice(&state[p..p + 4]);
        p += 4;
        self.wram_page = state[p];
        p += 1;
        for i in 0..8 {
            if p + 2 > state.len() {
                return p;
            }
            self.chr_banks_a[i] = state[p] as u16 | (state[p + 1] as u16) << 8;
            p += 2;
        }
        for i in 0..4 {
            if p + 2 > state.len() {
                return p;
            }
            self.chr_banks_b[i] = state[p] as u16 | (state[p + 1] as u16) << 8;
            p += 2;
        }
        if p + 10 > state.len() {
            return p;
        }
        self.prg_mode = state[p];
        p += 1;
        self.chr_mode = state[p];
        p += 1;
        self.exram_mode = state[p];
        p += 1;
        self.nametable_mirroring = state[p];
        p += 1;
        self.fill_tile = state[p];
        p += 1;
        self.fill_attr = state[p];
        p += 1;
        self.wram_mask_enable.copy_from_slice(&state[p..p + 2]);
        p += 2;
        self.chr_high_bits = state[p];
        p += 1;
        self.mul_op1 = state[p];
        p += 1;
        self.mul_op2 = state[p];
        p += 1;
        if p + 1024 > state.len() {
            return p;
        }
        self.exram.copy_from_slice(&state[p..p + 1024]);
        p += 1024;
        if p + 8 > state.len() {
            return p;
        }
        self.irq_scanline = state[p];
        p += 1;
        self.irq_enable = state[p] != 0;
        p += 1;
        self.irq_pending = state[p] != 0;
        p += 1;
        self.irq_in_frame = state[p] != 0;
        p += 1;
        self.irq_line_counter = state[p];
        p += 1;
        self.mmc5_ab_mode = state[p];
        p += 1;
        self.split_mode = state[p];
        p += 1;
        self.split_scroll = state[p];
        p += 1;
        self.split_page = state[p];
        p += 1;
        if p + 2 + 2 + 2 + 2 + 2 + 4 + 4 + 4 + 4 + 2 > state.len() {
            self.mark_prg_dirty();
            self.rebuild_prg(cart.prg_rom.len());
            return p;
        }
        self.sound_wl[0] = u16::from_le_bytes([state[p], state[p + 1]]);
        p += 2;
        self.sound_wl[1] = u16::from_le_bytes([state[p], state[p + 1]]);
        p += 2;
        self.sound_env.copy_from_slice(&state[p..p + 2]);
        p += 2;
        self.sound_enable = state[p];
        p += 1;
        self.sound_running = state[p];
        p += 1;
        self.sound_dcount[0] = i32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
        p += 4;
        self.sound_dcount[1] = i32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
        p += 4;
        self.sound_vcount[0] = i32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
        p += 4;
        self.sound_vcount[1] = i32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
        p += 4;
        self.sound_raw = state[p];
        p += 1;
        self.sound_raw_control = state[p];
        p += 1;
        if p + 4 <= state.len() {
            let wram_len = u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]) as usize;
            p += 4;
            if wram_len > 0 && p + wram_len <= state.len() && cart.prg_ram.len() == wram_len {
                cart.prg_ram.copy_from_slice(&state[p..p + wram_len]);
                p += wram_len;
            }
        }
        self.mark_prg_dirty();
        self.rebuild_prg(cart.prg_rom.len());
        p
    }
}
