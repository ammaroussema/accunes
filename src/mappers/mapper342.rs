use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

const SAVE_FLASH_SIZE: usize = 8 * 1024 * 1024;
const FLASH_SECTOR_SIZE: u32 = 128 * 1024;
const FLASH_BANK_THRESHOLD: u32 = 0x20000 - (SAVE_FLASH_SIZE as u32 / 1024 / 8);

const CFI_DATA: [u8; 128] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x51, 0x52, 0x59, 0x02, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x27, 0x36, 0x00, 0x00, 0x06,
    0x06, 0x09, 0x13, 0x03, 0x05, 0x03, 0x02, 0x1E, 0x02, 0x00, 0x06, 0x00, 0x01, 0xFF, 0x03, 0x00,
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF,
    0x50, 0x52, 0x49, 0x31, 0x33, 0x14, 0x02, 0x01, 0x00, 0x08, 0x00, 0x00, 0x02, 0xB5, 0xC5, 0x05,
    0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum PrgSlot6000 {
    OpenBus,
    Wram,
    Rom,
}

pub struct Mapper342 {
    wram_enabled: u8,
    map_rom_on_6000: u8,
    wram_page: u8,
    can_write_chr_ram: u8,
    can_write_prg: u8,
    can_write_flash: u8,
    flash_state: u8,
    flash_buffer_a: [u16; 10],
    flash_buffer_v: [u8; 10],
    cfi_mode: u8,
    save_flash: Vec<u8>,
    flags: u8,
    mapper: u8,
    mirroring: u8,
    four_screen: u8,
    lockout: u8,
    prg_base: u16,
    prg_mask: u8,
    prg_mode: u8,
    prg_bank_6000: u8,
    prg_bank_a: u8,
    prg_bank_b: u8,
    prg_bank_c: u8,
    prg_bank_d: u8,
    prg_bank_6000_mapped: u32,
    prg_bank_a_mapped: u32,
    prg_bank_b_mapped: u32,
    prg_bank_c_mapped: u32,
    prg_bank_d_mapped: u32,
    chr_mask: u8,
    chr_mode: u8,
    chr_bank_a: u8,
    chr_bank_b: u8,
    chr_bank_c: u8,
    chr_bank_d: u8,
    chr_bank_e: u8,
    chr_bank_f: u8,
    chr_bank_g: u8,
    chr_bank_h: u8,
    chr_bank_a_alt: u8,
    chr_bank_b_alt: u8,
    chr_bank_c_alt: u8,
    chr_bank_d_alt: u8,
    chr_bank_e_alt: u8,
    chr_bank_g_alt: u8,
    ppu_latch0: u8,
    ppu_latch1: u8,
    ppu_mapper163_latch: u8,
    tksmir: [u8; 8],
    scanline_irq_enabled: u8,
    scanline_irq_counter: u8,
    scanline_irq_latch: u8,
    scanline_irq_reload: u8,
    scanline2_irq_enabled: u8,
    scanline2_irq_line: u8,
    scanline2_irq_pending: u8,
    cpu_irq_value: u16,
    cpu_irq_control: u8,
    cpu_irq_latch: u16,
    vrc4_irq_prescaler: u8,
    vrc4_irq_prescaler_counter: u8,
    r0: u8,
    r1: u8,
    r2: u8,
    r3: u8,
    r4: u8,
    r5: u8,
    mul1: u8,
    mul2: u8,
    mapper67_irq_enabled: u8,
    mapper67_irq_latch: u8,
    mapper67_irq_counter: u16,
    mapper83_irq_enabled: u8,
    mapper83_irq_enabled_latch: u8,
    mapper83_irq_counter: u16,
    chr_1k_banks: [u32; 8],
    prg_slot_6000: PrgSlot6000,
    irq_asserted: bool,
    irq_ack: bool,
    last_ppu_scanline: i32,
    last_ppu_is_rendering: i32,
}

impl Mapper342 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        let mut m = Self {
            wram_enabled: 0,
            map_rom_on_6000: 0,
            wram_page: 0,
            can_write_chr_ram: 0,
            can_write_prg: 0,
            can_write_flash: 0,
            flash_state: 0,
            flash_buffer_a: [0; 10],
            flash_buffer_v: [0; 10],
            cfi_mode: 0,
            save_flash: vec![0xFF; SAVE_FLASH_SIZE],
            flags: 0,
            mapper: 0,
            mirroring: 0,
            four_screen: 0,
            lockout: 0,
            prg_base: 0,
            prg_mask: 0xF8,
            prg_mode: 0,
            prg_bank_6000: 0,
            prg_bank_a: 0,
            prg_bank_b: 1,
            prg_bank_c: 0xFE,
            prg_bank_d: 0xFF,
            prg_bank_6000_mapped: 0,
            prg_bank_a_mapped: 0,
            prg_bank_b_mapped: 0,
            prg_bank_c_mapped: 0,
            prg_bank_d_mapped: 0,
            chr_mask: 0,
            chr_mode: 0,
            chr_bank_a: 0,
            chr_bank_b: 1,
            chr_bank_c: 2,
            chr_bank_d: 3,
            chr_bank_e: 4,
            chr_bank_f: 5,
            chr_bank_g: 6,
            chr_bank_h: 7,
            chr_bank_a_alt: 0,
            chr_bank_b_alt: 0,
            chr_bank_c_alt: 0,
            chr_bank_d_alt: 0,
            chr_bank_e_alt: 0,
            chr_bank_g_alt: 0,
            ppu_latch0: 0,
            ppu_latch1: 0,
            ppu_mapper163_latch: 0,
            tksmir: [0; 8],
            scanline_irq_enabled: 0,
            scanline_irq_counter: 0,
            scanline_irq_latch: 0,
            scanline_irq_reload: 0,
            scanline2_irq_enabled: 0,
            scanline2_irq_line: 0,
            scanline2_irq_pending: 0,
            cpu_irq_value: 0,
            cpu_irq_control: 0,
            cpu_irq_latch: 0,
            vrc4_irq_prescaler: 0,
            vrc4_irq_prescaler_counter: 0,
            r0: 0,
            r1: 0,
            r2: 0,
            r3: 0,
            r4: 0,
            r5: 0,
            mul1: 0,
            mul2: 0,
            mapper67_irq_enabled: 0,
            mapper67_irq_latch: 0,
            mapper67_irq_counter: 0,
            mapper83_irq_enabled: 0,
            mapper83_irq_enabled_latch: 0,
            mapper83_irq_counter: 0,
            chr_1k_banks: [0; 8],
            prg_slot_6000: PrgSlot6000::OpenBus,
            irq_asserted: false,
            irq_ack: false,
            last_ppu_scanline: 0,
            last_ppu_is_rendering: 0,
        };
        m.sync();
        m
    }

    fn set_irq(&mut self, level: u8) {
        if level == 0 {
            self.irq_asserted = true;
        } else {
            self.irq_asserted = false;
            self.irq_ack = true;
        }
    }

    fn chr_mask_1k(&self) -> u32 {
        ((((!self.chr_mask & 0x1F) as u32) + 1) * 0x2000 / 0x400).saturating_sub(1)
    }

    fn set_mapped_chr1(&mut self, bank: usize, val: u32) {
        if bank < 8 {
            self.chr_1k_banks[bank] = val & self.chr_mask_1k();
        }
    }

    fn set_mapped_chr2(&mut self, bank: usize, val: u32) {
        for i in 0..2 {
            self.set_mapped_chr1(bank + i, val * 2 + i as u32);
        }
    }

    fn set_mapped_chr4(&mut self, bank: usize, val: u32) {
        for i in 0..4 {
            self.set_mapped_chr1(bank + i, val * 4 + i as u32);
        }
    }

    fn set_mapped_chr8(&mut self, bank: usize, val: u32) {
        for i in 0..8 {
            self.set_mapped_chr1(bank + i, val * 8 + i as u32);
        }
    }

    fn sync_prg(&mut self) {
        let prg_mask_bits = (((!self.prg_mask & 0x7F) as u32) << 1) | 1;
        self.prg_bank_6000_mapped =
            ((self.prg_base as u32) << 1) | (self.prg_bank_6000 as u32 & prg_mask_bits);
        self.prg_bank_a_mapped =
            ((self.prg_base as u32) << 1) | (self.prg_bank_a as u32 & prg_mask_bits);
        self.prg_bank_b_mapped =
            ((self.prg_base as u32) << 1) | (self.prg_bank_b as u32 & prg_mask_bits);
        self.prg_bank_c_mapped =
            ((self.prg_base as u32) << 1) | (self.prg_bank_c as u32 & prg_mask_bits);
        self.prg_bank_d_mapped =
            ((self.prg_base as u32) << 1) | (self.prg_bank_d as u32 & prg_mask_bits);

        self.prg_slot_6000 = if self.map_rom_on_6000 != 0 {
            PrgSlot6000::Rom
        } else if self.wram_enabled != 0 {
            PrgSlot6000::Wram
        } else {
            PrgSlot6000::OpenBus
        };
    }

    fn chr_bank_2k(&self, lo: u8, hi: u8) -> u32 {
        if self.mapper == 35 && (self.flags & 4) != 0 {
            (((hi as u32) << 8) | lo as u32) >> 1
        } else {
            (lo >> 1) as u32
        }
    }

    fn clock_mmc3_scanline_irq(&mut self) -> bool {
        if self.scanline_irq_reload != 0 || self.scanline_irq_counter == 0 {
            self.scanline_irq_counter = self.scanline_irq_latch;
            self.scanline_irq_reload = 0;
        } else {
            self.scanline_irq_counter = self.scanline_irq_counter.wrapping_sub(1);
        }
        if self.scanline_irq_counter == 0 && self.scanline_irq_enabled != 0 {
            self.set_irq(0);
            return true;
        }
        false
    }

    fn uses_mmc3_scanline_irq(&self) -> bool {
        matches!(self.mapper, 13 | 20 | 22)
    }

    fn vrc24_address_bits(&self, addr: u16) -> (u8, u8) {
        let bit = |a: u16, n: u8| -> u8 { ((a >> n) & 1) as u8 };
        match self.flags & 5 {
            0 => (bit(addr, 7) | bit(addr, 2), bit(addr, 6) | bit(addr, 1)),
            1 => (bit(addr, 0), bit(addr, 1)),
            4 => (
                bit(addr, 5) | bit(addr, 3) | bit(addr, 1),
                bit(addr, 4) | bit(addr, 2) | bit(addr, 0),
            ),
            _ => (bit(addr, 2) | bit(addr, 0), bit(addr, 3) | bit(addr, 1)),
        }
    }

    fn sync_chr(&mut self) {
        if self.mapper == 31 {
            let lower = (self.r0 & 0x0F) as u32;
            let upper = ((self.r0 >> 4) & 0x0F) as u32;
            self.set_mapped_chr4(0, lower);
            self.set_mapped_chr4(4, upper);
            return;
        }
        match self.chr_mode & 7 {
            0 => self.set_mapped_chr8(0, (self.chr_bank_a >> 3) as u32),
            1 => {
                let latch = self.ppu_mapper163_latch as u32;
                self.set_mapped_chr4(0, latch);
                self.set_mapped_chr4(4, latch);
            }
            2 => {
                self.set_mapped_chr2(0, (self.chr_bank_a >> 1) as u32);
                self.tksmir[0] = self.chr_bank_a;
                self.tksmir[1] = self.chr_bank_a;
                self.set_mapped_chr2(2, (self.chr_bank_c >> 1) as u32);
                self.tksmir[2] = self.chr_bank_c;
                self.tksmir[3] = self.chr_bank_c;
                self.set_mapped_chr1(4, self.chr_bank_e as u32);
                self.tksmir[4] = self.chr_bank_e;
                self.set_mapped_chr1(5, self.chr_bank_f as u32);
                self.tksmir[5] = self.chr_bank_f;
                self.set_mapped_chr1(6, self.chr_bank_g as u32);
                self.tksmir[6] = self.chr_bank_g;
                self.set_mapped_chr1(7, self.chr_bank_h as u32);
                self.tksmir[7] = self.chr_bank_h;
            }
            3 => {
                self.set_mapped_chr1(0, self.chr_bank_e as u32);
                self.tksmir[0] = self.chr_bank_e;
                self.set_mapped_chr1(1, self.chr_bank_f as u32);
                self.tksmir[1] = self.chr_bank_f;
                self.set_mapped_chr1(2, self.chr_bank_g as u32);
                self.tksmir[2] = self.chr_bank_g;
                self.set_mapped_chr1(3, self.chr_bank_h as u32);
                self.tksmir[3] = self.chr_bank_h;
                self.set_mapped_chr2(4, (self.chr_bank_a >> 1) as u32);
                self.tksmir[4] = self.chr_bank_a;
                self.tksmir[5] = self.chr_bank_a;
                self.set_mapped_chr2(6, (self.chr_bank_c >> 1) as u32);
                self.tksmir[6] = self.chr_bank_c;
                self.tksmir[7] = self.chr_bank_c;
            }
            4 => {
                self.set_mapped_chr4(0, (self.chr_bank_a >> 2) as u32);
                self.set_mapped_chr4(4, (self.chr_bank_e >> 2) as u32);
            }
            5 => {
                if self.ppu_latch0 == 0 {
                    self.set_mapped_chr4(0, (self.chr_bank_a >> 2) as u32);
                } else {
                    self.set_mapped_chr4(0, (self.chr_bank_b >> 2) as u32);
                }
                if self.ppu_latch1 == 0 {
                    self.set_mapped_chr4(4, (self.chr_bank_e >> 2) as u32);
                } else {
                    self.set_mapped_chr4(4, (self.chr_bank_f >> 2) as u32);
                }
            }
            6 => {
                if self.mapper == 35 && (self.flags & 4) != 0 {
                    self.set_mapped_chr2(0, self.chr_bank_2k(self.chr_bank_a, self.chr_bank_a_alt));
                    self.set_mapped_chr2(2, self.chr_bank_2k(self.chr_bank_c, self.chr_bank_c_alt));
                    self.set_mapped_chr2(4, self.chr_bank_2k(self.chr_bank_e, self.chr_bank_e_alt));
                    self.set_mapped_chr2(6, self.chr_bank_2k(self.chr_bank_g, self.chr_bank_g_alt));
                } else {
                    self.set_mapped_chr2(0, (self.chr_bank_a >> 1) as u32);
                    self.set_mapped_chr2(2, (self.chr_bank_c >> 1) as u32);
                    self.set_mapped_chr2(4, (self.chr_bank_e >> 1) as u32);
                    self.set_mapped_chr2(6, (self.chr_bank_g >> 1) as u32);
                }
            }
            _ => {
                self.set_mapped_chr1(0, self.chr_bank_a as u32);
                self.set_mapped_chr1(1, self.chr_bank_b as u32);
                self.set_mapped_chr1(2, self.chr_bank_c as u32);
                self.set_mapped_chr1(3, self.chr_bank_d as u32);
                self.set_mapped_chr1(4, self.chr_bank_e as u32);
                self.set_mapped_chr1(5, self.chr_bank_f as u32);
                self.set_mapped_chr1(6, self.chr_bank_g as u32);
                self.set_mapped_chr1(7, self.chr_bank_h as u32);
            }
        }
    }

    fn sync(&mut self) {
        self.sync_prg();
        self.sync_chr();
    }

    fn mirror_single_0(address: u16) -> u16 {
        address & 0x23FF
    }

    fn mirror_single_1(address: u16) -> u16 {
        (address & 0x23FF) | 0x0400
    }

    fn mapped_bank_for_addr(&self, address: u16) -> u32 {
        let addr = address as usize;
        if self.mapper == 31 {
            return self.prg_bank_a_mapped;
        }
        match self.prg_mode & 7 {
            0 => {
                if addr < 0xC000 {
                    self.prg_bank_a_mapped
                } else {
                    self.prg_bank_c_mapped
                }
            }
            1 => {
                if addr < 0xC000 {
                    self.prg_bank_c_mapped
                } else {
                    self.prg_bank_a_mapped
                }
            }
            4 => {
                let banks = [
                    self.prg_bank_a_mapped,
                    self.prg_bank_b_mapped,
                    self.prg_bank_c_mapped,
                    self.prg_bank_d_mapped,
                ];
                let slot = (addr - 0x8000) / 0x2000;
                banks[slot]
            }
            5 => {
                let banks = [
                    self.prg_bank_c_mapped,
                    self.prg_bank_b_mapped,
                    self.prg_bank_a_mapped,
                    self.prg_bank_d_mapped,
                ];
                let slot = (addr - 0x8000) / 0x2000;
                banks[slot]
            }
            6 => self.prg_bank_b_mapped,
            7 => self.prg_bank_a_mapped,
            _ => 0,
        }
    }

    fn uses_flash_bank(&self, mapped: u32) -> bool {
        mapped >= FLASH_BANK_THRESHOLD
    }

    fn read_prg_byte(&self, cart: &Cartridge, address: u16) -> u8 {
        if self.cfi_mode != 0 {
            return CFI_DATA[(address as usize) % CFI_DATA.len()];
        }
        let mapped = self.mapped_bank_for_addr(address);
        if self.uses_flash_bank(mapped) {
            let flash_addr =
                mapped as u64 * 0x2000 + (address as u64 % 0x8000);
            let idx = (flash_addr as usize) % SAVE_FLASH_SIZE;
            return self.save_flash[idx];
        }
        if cart.prg_rom.is_empty() {
            return 0;
        }
        let off = self.prg_rom_offset(address);
        cart.prg_rom[off % cart.prg_rom.len()]
    }

    fn flash_write(&mut self, address: u16, val: u8) {
        if (self.flash_state as usize) < self.flash_buffer_a.len() {
            let idx = self.flash_state as usize;
            self.flash_buffer_a[idx] = address & 0xFFF;
            self.flash_buffer_v[idx] = val;
            self.flash_state += 1;

            if self.flash_state == 1
                && self.flash_buffer_a[0] == 0x0AAA
                && self.flash_buffer_v[0] == 0x98
            {
                self.cfi_mode = 1;
                self.flash_state = 0;
            }

            if self.flash_state == 6
                && self.flash_buffer_a[0] == 0x0AAA
                && self.flash_buffer_v[0] == 0xAA
                && self.flash_buffer_a[1] == 0x0555
                && self.flash_buffer_v[1] == 0x55
                && self.flash_buffer_a[2] == 0x0AAA
                && self.flash_buffer_v[2] == 0x80
                && self.flash_buffer_a[3] == 0x0AAA
                && self.flash_buffer_v[3] == 0xAA
                && self.flash_buffer_a[4] == 0x0555
                && self.flash_buffer_v[4] == 0x55
                && self.flash_buffer_v[5] == 0x30
            {
                let sector = self.prg_bank_a_mapped * 0x2000 / FLASH_SECTOR_SIZE;
                let sector_address = sector * FLASH_SECTOR_SIZE;
                for i in sector_address..sector_address + FLASH_SECTOR_SIZE {
                    self.save_flash[(i as usize) % SAVE_FLASH_SIZE] = 0xFF;
                }
                self.flash_state = 0;
            }

            if self.flash_state == 4
                && self.flash_buffer_a[0] == 0x0AAA
                && self.flash_buffer_v[0] == 0xAA
                && self.flash_buffer_a[1] == 0x0555
                && self.flash_buffer_v[1] == 0x55
                && self.flash_buffer_a[2] == 0x0AAA
                && self.flash_buffer_v[2] == 0xA0
            {
                let flash_addr =
                    self.prg_bank_a_mapped * 0x2000 + (address as u32 % 0x8000);
                let idx = (flash_addr as usize) % SAVE_FLASH_SIZE;
                if self.save_flash[idx] == 0xFF {
                    self.save_flash[idx] = val;
                }
                self.flash_state = 0;
            }
        }

        if (address & 0xFFF) != 0x0AAA && (address & 0xFFF) != 0x0555 {
            self.flash_state = 0;
        }

        if val == 0xF0 {
            self.flash_state = 0;
            self.cfi_mode = 0;
        }
    }

    fn prg_rom_offset(&self, address: u16) -> usize {
        let addr = address as usize;
        if self.mapper == 31 {
            return (self.prg_bank_a_mapped >> 2) as usize * 0x8000 + (addr & 0x7FFF);
        }
        match self.prg_mode & 7 {
            0 => {
                let bank16 = if addr < 0xC000 {
                    self.prg_bank_a_mapped >> 1
                } else {
                    self.prg_bank_c_mapped >> 1
                };
                bank16 as usize * 0x4000 + (addr & 0x3FFF)
            }
            1 => {
                let bank16 = if addr < 0xC000 {
                    self.prg_bank_c_mapped >> 1
                } else {
                    self.prg_bank_a_mapped >> 1
                };
                bank16 as usize * 0x4000 + (addr & 0x3FFF)
            }
            4 => {
                let banks = [
                    self.prg_bank_a_mapped,
                    self.prg_bank_b_mapped,
                    self.prg_bank_c_mapped,
                    self.prg_bank_d_mapped,
                ];
                let slot = (addr - 0x8000) / 0x2000;
                banks[slot] as usize * 0x2000 + (addr & 0x1FFF)
            }
            5 => {
                let banks = [
                    self.prg_bank_c_mapped,
                    self.prg_bank_b_mapped,
                    self.prg_bank_a_mapped,
                    self.prg_bank_d_mapped,
                ];
                let slot = (addr - 0x8000) / 0x2000;
                banks[slot] as usize * 0x2000 + (addr & 0x1FFF)
            }
            6 => (self.prg_bank_b_mapped >> 2) as usize * 0x8000 + (addr & 0x7FFF),
            7 => (self.prg_bank_a_mapped >> 2) as usize * 0x8000 + (addr & 0x7FFF),
            _ => addr & 0x7FFF,
        }
    }

    fn read_5(&mut self, addr: u16) -> FetchResult {
        if self.mapper == 6 {
            if (addr & 0x700) == 0x100 {
                return FetchResult {
                    data: self.r2 | self.r0 | self.r1 | !self.r3,
                    driven: true,
                };
            }
            if (addr & 0x700) == 0x500 {
                return FetchResult {
                    data: if self.r5 & 1 != 0 { self.r2 } else { self.r1 },
                    driven: true,
                };
            }
        }
        if self.mapper == 15 && addr == 0x204 {
            let p = self.scanline2_irq_pending;
            let r = if self.last_ppu_is_rendering == 0 || self.last_ppu_scanline + 1 >= 241 {
                0
            } else {
                1
            };
            self.set_irq(1);
            self.scanline2_irq_pending = 0;
            return FetchResult {
                data: (p << 7) | (r << 6),
                driven: true,
            };
        }
        if self.mapper == 35 {
            return FetchResult {
                data: self.flags & 3,
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: self.mapper == 0,
        }
    }

    fn write_nina_latch(&mut self, val: u8) -> bool {
        match self.mapper {
            27 => {
                self.chr_bank_a = (self.chr_bank_a & !0x38) | ((val & 7) << 3);
                self.prg_bank_a = (self.prg_bank_a & !4) | (val & 8);
                self.sync();
                true
            }
            28 => {
                self.chr_bank_a = (self.chr_bank_a & !0x18) | ((val & 3) << 3);
                self.prg_bank_a = (self.prg_bank_a & !4) | (val & 4);
                self.sync();
                true
            }
            _ => false,
        }
    }

    fn write_4(&mut self, addr: u16, val: u8) {
        if addr >= 0x20 && self.mapper == 20 && (self.flags & 2) != 0 {
            self.prg_bank_a = (self.prg_bank_a & 0xC3)
                | ((val & 0x0F) << 2)
                | ((val & 0xF0) >> 2);
        }
        self.sync();
    }

    fn write_5(&mut self, addr: u16, val: u8) {
        if self.lockout == 0 {
            match addr & 7 {
                0 => self.prg_base = (self.prg_base & 0xFF) | ((val as u16) << 8),
                1 => self.prg_base = (self.prg_base & 0xFF00) | val as u16,
                2 => self.prg_mask = val,
                3 => {
                    self.prg_mode = (val & 0xE0) >> 5;
                    self.chr_bank_a = (self.chr_bank_a & 7) | (val << 3);
                }
                4 => {
                    self.chr_mode = (val & 0xE0) >> 5;
                    self.chr_mask = val & 0x1F;
                }
                5 => {
                    self.prg_bank_a = (self.prg_bank_a & 0xC1) | ((val & 0x7C) >> 1);
                    self.wram_page = val & 3;
                }
                6 => {
                    self.flags = (val & 0xE0) >> 5;
                    self.mapper = (self.mapper & 0x20) | (val & 0x1F);
                }
                _ => {
                    self.lockout = (val & 0x80) >> 7;
                    self.mapper = (self.mapper & 0x1F) | ((val & 0x40) >> 1);
                    self.four_screen = (val & 0x20) >> 5;
                    self.mirroring = (val & 0x18) >> 3;
                    self.can_write_flash = (val & 4) >> 2;
                    self.can_write_chr_ram = (val & 2) >> 1;
                    self.wram_enabled = val & 1;
                    if self.mapper == 42 {
                        self.map_rom_on_6000 = 1;
                    }
                }
            }
            if self.mapper == 17 {
                self.prg_bank_b = 0xFD;
            }
            if self.mapper == 14 {
                self.prg_bank_b = 1;
            }
        }
        self.write_5_mapper163(addr, val);
        self.write_5_mapper15(addr, val);
        if self.mapper == 20 && (self.flags & 2) != 0 {
            self.prg_bank_a = (self.prg_bank_a & 0xC3)
                | ((val & 0x0F) << 2)
                | ((val & 0xF0) >> 2);
        }
        if self.mapper == 31 {
            self.map_rom_on_6000 = 0;
        }
        self.sync();
    }

    fn write_5_mapper163(&mut self, addr: u16, val: u8) {
        if self.mapper != 6 {
            return;
        }
        if addr == 0x101 {
            if self.r4 != 0 && val == 0 {
                self.r5 ^= 1;
            }
            self.r4 = val;
        } else if addr == 0x100 && val == 6 {
            self.prg_mode &= 0xFE;
            self.prg_bank_b = 12;
        } else {
            match (addr >> 8) & 3 {
                2 => {
                    self.prg_mode |= 1;
                    self.prg_bank_a = (self.prg_bank_a & 0x3F) | ((val & 3) << 6);
                    self.r0 = val;
                }
                0 => {
                    self.prg_mode |= 1;
                    self.prg_bank_a = (self.prg_bank_a & 0xC3) | ((val & 0x0F) << 2);
                    self.chr_mode = (self.chr_mode & 0xFE) | (val >> 7);
                    self.r1 = val;
                }
                3 => self.r2 = val,
                _ => self.r3 = val,
            }
        }
    }

    fn write_5_mapper15(&mut self, addr: u16, val: u8) {
        if self.mapper != 15 {
            return;
        }
        match addr {
            0x105 => {
                if val == 0xFF {
                    self.four_screen = 1;
                } else {
                    self.four_screen = 0;
                    self.mirroring = match ((val >> 2) & 1) | ((val >> 3) & 2) {
                        0 => 2,
                        1 => 0,
                        2 => 1,
                        _ => 3,
                    };
                }
            }
            0x115 => {
                self.prg_bank_a = val & !1;
                self.prg_bank_b = val | 1;
            }
            0x116 => self.prg_bank_c = val,
            0x117 => self.prg_bank_d = val,
            0x120 => self.chr_bank_a = val,
            0x121 => self.chr_bank_b = val,
            0x122 => self.chr_bank_c = val,
            0x123 => self.chr_bank_d = val,
            0x128 => self.chr_bank_e = val,
            0x129 => self.chr_bank_f = val,
            0x12A => self.chr_bank_g = val,
            0x12B => self.chr_bank_h = val,
            0x203 => {
                self.set_irq(1);
                self.scanline2_irq_pending = 0;
                self.scanline2_irq_line = val;
            }
            0x204 => {
                self.set_irq(1);
                self.scanline2_irq_pending = 0;
                self.scanline2_irq_enabled = val & 0x80;
            }
            _ => {}
        }
    }

    fn write_67(&mut self, cart: &mut Cartridge, address: u16, val: u8) {
        if self.wram_enabled != 0 && self.map_rom_on_6000 == 0 && !cart.prg_ram.is_empty() {
            let off = (self.wram_page as usize) * 0x2000 + (address as usize - 0x6000);
            let len = cart.prg_ram.len();
            cart.prg_ram[off % len] = val;
        } else if self.map_rom_on_6000 != 0 && !cart.prg_rom.is_empty() {
            let off = self.prg_bank_6000_mapped as usize * 0x2000 + (address as usize - 0x6000);
            let len = cart.prg_rom.len();
            cart.prg_rom[off % len] = val;
        }
        if self.mapper == 12 {
            self.chr_bank_a = (self.chr_bank_a & 0xE7)
                | ((val & 1) << 4)
                | ((val & 2) << 2);
            self.sync_chr();
        }
        if self.mapper == 20 && (self.flags & 2) != 0 {
            self.prg_bank_a = (self.prg_bank_a & 0xC3)
                | ((val & 0x0F) << 2)
                | ((val & 0xF0) >> 2);
            self.sync_prg();
        }
        if self.mapper == 31 && address >= 0x6000 {
            self.r0 = val;
            self.sync_chr();
        }
    }

    fn write_8f(&mut self, address: u16, val: u8) {
        let bank = address >> 12;
        let addr = address & 0xFFF;
        if self.mapper == 1 {
            if (self.flags & 1) == 0 || bank != 9 {
                self.prg_bank_a = (self.prg_bank_a & 0xC1) | ((val & 0x1F) << 1);
            } else {
                self.mirroring = 2 + ((val >> 4) & 1);
            }
        }
        if self.mapper == 2 {
            self.chr_bank_a = (self.chr_bank_a & 7) | (val << 3);
        }
        if self.mapper == 3 {
            self.prg_bank_a = (self.prg_bank_a & 0xF1) | ((val & 7) << 1);
            self.chr_bank_a = (self.chr_bank_a & 0x87) | ((val & 0xF0) >> 1);
            self.mirroring = ((val >> 3) & 1) ^ 1;
        }
        if self.mapper == 4 {
            self.prg_bank_a = (self.prg_bank_a & 0xE1) | ((val & 0x0F) << 1);
            self.mirroring = (val >> 6) ^ ((val >> 6) & 2);
        }
        if self.mapper == 5 {
            self.prg_bank_a = (self.prg_bank_a & 0xF1) | ((val & 0x70) >> 3);
            self.can_write_chr_ram = val & 1;
        }
        if self.mapper == 7 {
            match ((bank & 7) << 2) | (addr & 3) {
                0 => self.prg_bank_a = (self.prg_bank_a & 0xF0) | (val & 0x0F),
                1 => self.prg_bank_a = (self.prg_bank_a & 0x0F) | ((val & 0x0F) << 4),
                2 => self.prg_bank_b = (self.prg_bank_b & 0xF0) | (val & 0x0F),
                3 => self.prg_bank_b = (self.prg_bank_b & 0x0F) | ((val & 0x0F) << 4),
                4 => self.prg_bank_c = (self.prg_bank_c & 0xF0) | (val & 0x0F),
                5 => self.prg_bank_c = (self.prg_bank_c & 0x0F) | ((val & 0x0F) << 4),
                8 => self.chr_bank_a = (self.chr_bank_a & 0xF0) | (val & 0x0F),
                9 => self.chr_bank_a = (self.chr_bank_a & 0x0F) | ((val & 0x0F) << 4),
                10 => self.chr_bank_b = (self.chr_bank_b & 0xF0) | (val & 0x0F),
                11 => self.chr_bank_b = (self.chr_bank_b & 0x0F) | ((val & 0x0F) << 4),
                12 => self.chr_bank_c = (self.chr_bank_c & 0xF0) | (val & 0x0F),
                13 => self.chr_bank_c = (self.chr_bank_c & 0x0F) | ((val & 0x0F) << 4),
                14 => self.chr_bank_d = (self.chr_bank_d & 0xF0) | (val & 0x0F),
                15 => self.chr_bank_d = (self.chr_bank_d & 0x0F) | ((val & 0x0F) << 4),
                16 => self.chr_bank_e = (self.chr_bank_e & 0xF0) | (val & 0x0F),
                17 => self.chr_bank_e = (self.chr_bank_e & 0x0F) | ((val & 0x0F) << 4),
                18 => self.chr_bank_f = (self.chr_bank_f & 0xF0) | (val & 0x0F),
                19 => self.chr_bank_f = (self.chr_bank_f & 0x0F) | ((val & 0x0F) << 4),
                20 => self.chr_bank_g = (self.chr_bank_g & 0xF0) | (val & 0x0F),
                21 => self.chr_bank_g = (self.chr_bank_g & 0x0F) | ((val & 0x0F) << 4),
                22 => self.chr_bank_h = (self.chr_bank_h & 0xF0) | (val & 0x0F),
                23 => self.chr_bank_h = (self.chr_bank_h & 0x0F) | ((val & 0x0F) << 4),
                24 => self.cpu_irq_latch = (self.cpu_irq_latch & 0xFFF0) | (val as u16 & 0x0F),
                25 => self.cpu_irq_latch = (self.cpu_irq_latch & 0xFF0F) | ((val as u16 & 0x0F) << 4),
                26 => self.cpu_irq_latch = (self.cpu_irq_latch & 0xF0FF) | ((val as u16 & 0x0F) << 8),
                27 => self.cpu_irq_latch = (self.cpu_irq_latch & 0x0FFF) | ((val as u16 & 0x0F) << 12),
                28 => {
                    self.set_irq(1);
                    self.cpu_irq_value = self.cpu_irq_latch;
                }
                29 => {
                    self.set_irq(1);
                    self.cpu_irq_control = val & 0x0F;
                }
                30 => self.mirroring = val ^ (((val >> 1) & 1) ^ 1),
                _ => {}
            }
        }
        if self.mapper == 8 {
            self.prg_bank_a = (self.prg_bank_a & 0xC3) | ((val & 0xF) << 2);
            if (self.flags & 1) == 0 {
                self.mirroring = 2 + ((val >> 4) & 1);
            }
        }
        if self.mapper == 9 {
            self.prg_bank_a = (self.prg_bank_a & 0xC3) | ((address & 0x780) >> 5) as u8;
            self.chr_bank_a = (self.chr_bank_a & 7) | ((address & 7) as u8) << 5 | ((val & 3) << 3);
            self.mirroring = ((bank >> 1) & 1) as u8;
        }
        if self.mapper == 10 {
            self.prg_bank_a = (self.prg_bank_a & 0xF3) | ((val & 3) << 2);
            self.chr_bank_a = (self.chr_bank_a & 0x87) | ((val & 0xF0) >> 1);
        }
        if self.mapper == 11 {
            self.prg_bank_a = (self.prg_bank_a & 0xF3) | ((val & 0x30) >> 2);
            self.chr_bank_a = (self.chr_bank_a & 0xE7) | ((val & 3) << 3);
        }
        if self.mapper == 13 {
            match bank & 7 {
                0 => match addr & 3 {
                    0 => self.prg_bank_a = (self.prg_bank_a & 0xC0) | (val & 0x3F),
                    1 => self.prg_bank_b = (self.prg_bank_b & 0xC0) | (val & 0x3F),
                    2 => self.prg_bank_c = (self.prg_bank_c & 0xC0) | (val & 0x3F),
                    _ => self.prg_bank_d = (self.prg_bank_d & 0xC0) | (val & 0x3F),
                },
                1 => match addr & 7 {
                    0 => self.chr_bank_a = val,
                    1 => self.chr_bank_b = val,
                    2 => self.chr_bank_c = val,
                    3 => self.chr_bank_d = val,
                    4 => self.chr_bank_e = val,
                    5 => self.chr_bank_f = val,
                    6 => self.chr_bank_g = val,
                    _ => self.chr_bank_h = val,
                },
                4 => match addr & 7 {
                    0 => {
                        if val & 1 != 0 {
                            self.scanline_irq_enabled = 1;
                        } else {
                            self.scanline_irq_enabled = 0;
                            self.set_irq(1);
                        }
                    }
                    2 => {
                        self.scanline_irq_enabled = 0;
                        self.set_irq(1);
                    }
                    3 => self.scanline_irq_enabled = 1,
                    5 => {
                        self.scanline_irq_latch = val ^ self.r0;
                        self.scanline_irq_reload = 1;
                    }
                    6 => self.r0 = val,
                    _ => {}
                },
                5 => {
                    if (addr & 3) == 1 {
                        self.mirroring = val & 3;
                    }
                }
                _ => {}
            }
        }
        if self.mapper == 14 {
            match ((bank & 7) << 3) | (addr & 7) {
                0 => self.prg_bank_a = (self.prg_bank_a & 0xC0) | (val & 0x3F),
                9 => self.mirroring = (val >> 7) & 1,
                11 => {
                    self.cpu_irq_control = (self.cpu_irq_control & 0xFE) | ((val >> 7) & 1);
                    self.set_irq(1);
                }
                12 => {
                    self.cpu_irq_value = ((self.r0 as u16) << 8) | self.r1 as u16;
                    self.set_irq(1);
                }
                13 => self.r0 = val,
                14 => self.r1 = val,
                16 => self.prg_bank_b = (self.prg_bank_b & 0xC0) | (val & 0x3F),
                24 => self.chr_bank_a = val,
                25 => self.chr_bank_b = val,
                26 => self.chr_bank_c = val,
                27 => self.chr_bank_d = val,
                28 => self.chr_bank_e = val,
                29 => self.chr_bank_f = val,
                30 => self.chr_bank_g = val,
                31 => self.chr_bank_h = val,
                32 => self.prg_bank_c = (self.prg_bank_c & 0xC0) | (val & 0x3F),
                _ => {}
            }
        }
        if self.mapper == 16 {
            if val & 0x80 != 0 {
                self.r0 = (self.r0 & 0xC0) | 0x20;
                self.prg_mode = 0;
                self.prg_bank_c = (self.prg_bank_c & 0xE0) | 0x1E;
            } else {
                self.r0 = (self.r0 & 0xC0) | ((val & 1) << 5) | ((self.r0 & 0x3E) >> 1);
                if self.r0 & 1 != 0 {
                    match (bank >> 1) & 3 {
                        0 => {
                            if (self.r0 & 0x18) == 0x18 {
                                self.prg_mode = 0;
                                self.prg_bank_c = (self.prg_bank_c & 0xE0) | 0x1E;
                            } else if (self.r0 & 0x18) == 0x10 {
                                self.prg_mode = 1;
                                self.prg_bank_c = self.prg_bank_c & 0xE0;
                            } else {
                                self.prg_mode = 7;
                            }
                            if (self.r0 >> 5) & 1 != 0 {
                                self.chr_mode = 4;
                            } else {
                                self.chr_mode = 0;
                            }
                            self.mirroring = ((self.r0 >> 1) & 3) ^ 2;
                        }
                        1 => {
                            self.chr_bank_a = (self.chr_bank_a & 0x83) | ((self.r0 & 0x3E) << 1);
                            self.prg_bank_a = (self.prg_bank_a & 0xDF) | (self.r0 & 0x20);
                            self.prg_bank_c = (self.prg_bank_c & 0xDF) | (self.r0 & 0x20);
                        }
                        2 => {
                            self.chr_bank_e = (self.chr_bank_e & 0x83) | ((self.r0 & 0x3E) << 1);
                        }
                        _ => {
                            self.prg_bank_a = (self.prg_bank_a & 0xE1) | (self.r0 & 0x1E);
                            self.wram_enabled = (((self.r0 >> 5) & 1) ^ 1) as u8;
                        }
                    }
                    self.r0 = (self.r0 & 0xC0) | 0x20;
                    if (self.flags & 1) != 0 {
                        if self.chr_mode & 4 != 0 {
                            self.wram_page = 2 | ((self.chr_bank_a >> 6) ^ 1);
                        } else {
                            self.wram_page = 2 | ((self.chr_bank_a >> 5) ^ 1);
                        }
                    }
                }
            }
        }
        if self.mapper == 17 {
            match bank & 7 {
                2 => {
                    if (self.flags & 1) == 0 {
                        self.prg_bank_a = (self.prg_bank_a & 0xF0) | (val & 0x0F);
                    } else {
                        self.prg_bank_a = (self.prg_bank_a & 0xE1) | ((val & 0x0F) << 1);
                    }
                }
                3 => self.chr_bank_a = (self.chr_bank_a & 0x83) | ((val & 0x1F) << 2),
                4 => self.chr_bank_b = (self.chr_bank_b & 0x83) | ((val & 0x1F) << 2),
                5 => self.chr_bank_e = (self.chr_bank_e & 0x83) | ((val & 0x1F) << 2),
                6 => self.chr_bank_f = (self.chr_bank_f & 0x83) | ((val & 0x1F) << 2),
                7 => self.mirroring = val & 1,
                _ => {}
            }
        }
        if self.mapper == 18 {
            self.chr_bank_a = (self.chr_bank_a & 0x87) | ((val & 0x0F) << 3);
            self.prg_bank_a = (self.prg_bank_a & 0xF1) | ((val & 0x70) >> 3);
            self.mirroring = 2 | (val >> 7);
        }
        if self.mapper == 19 {
            match bank & 7 {
                0 => self.cpu_irq_latch = (self.cpu_irq_latch & 0xFFF0) | (val as u16 & 0x0F),
                1 => self.cpu_irq_latch = (self.cpu_irq_latch & 0xFF0F) | ((val as u16 & 0x0F) << 4),
                2 => self.cpu_irq_latch = (self.cpu_irq_latch & 0xF0FF) | ((val as u16 & 0x0F) << 8),
                3 => self.cpu_irq_latch = (self.cpu_irq_latch & 0x0FFF) | ((val as u16 & 0x0F) << 12),
                4 => {
                    self.set_irq(1);
                    self.cpu_irq_control = (self.cpu_irq_control & 0xF8) | (val & 7);
                    if self.cpu_irq_control & 2 != 0 {
                        self.cpu_irq_value = self.cpu_irq_latch;
                    }
                }
                5 => {
                    self.set_irq(1);
                    self.cpu_irq_control =
                        (self.cpu_irq_control & 0xFD) | ((self.cpu_irq_control & 1) << 1);
                }
                7 => self.prg_bank_a = (self.prg_bank_a & 0xF1) | ((val & 7) << 1),
                _ => {}
            }
        }
        if self.mapper == 20 {
            match (bank & 6) | (addr & 1) {
                0 => {
                    self.r0 = val & 7;
                    if (self.flags & 2) == 0 {
                        self.prg_mode = if val & 0x40 != 0 { 5 } else { 4 };
                    }
                    self.chr_mode = if val & 0x80 != 0 { 3 } else { 2 };
                }
                1 => match self.r0 & 7 {
                    0 => self.chr_bank_a = val,
                    1 => self.chr_bank_c = val,
                    2 => self.chr_bank_e = val,
                    3 => self.chr_bank_f = val,
                    4 => self.chr_bank_g = val,
                    5 => self.chr_bank_h = val,
                    6 => {
                        if (self.flags & 2) == 0 {
                            self.prg_bank_a = (self.prg_bank_a & 0xC0) | (val & 0x3F);
                        }
                    }
                    _ => {
                        if (self.flags & 2) == 0 {
                            self.prg_bank_b = (self.prg_bank_b & 0xC0) | (val & 0x3F);
                        }
                    }
                },
                2 => self.mirroring = val & 1,
                4 => self.scanline_irq_latch = val,
                5 => self.scanline_irq_reload = 1,
                6 => {
                    self.scanline_irq_enabled = 0;
                    self.set_irq(1);
                }
                7 => self.scanline_irq_enabled = 1,
                _ => {}
            }
        }
        if self.mapper == 21 {
            match (bank >> 1) & 3 {
                0 => self.r0 = (self.r0 & 0xF8) | (val & 7),
                1 => match self.r0 & 7 {
                    0 => self.prg_bank_a = (self.prg_bank_a & 0xC0) | (val & 0x3F),
                    1 => self.prg_bank_b = (self.prg_bank_b & 0xC0) | (val & 0x3F),
                    2 => self.chr_bank_a = val,
                    3 => self.chr_bank_c = val,
                    4 => self.chr_bank_e = val,
                    5 => self.chr_bank_f = val,
                    6 => self.chr_bank_g = val,
                    _ => self.chr_bank_h = val,
                },
                3 => self.mirroring = val & 1,
                _ => {}
            }
        }
        if self.mapper == 22 {
            let key = ((bank << 1) & 0xC) | (addr & 0x3);
            match key {
                0 => {
                    self.prg_bank_a = (self.prg_bank_a & 0xC0) | (val & 0x3F);
                    if (self.flags & 1) == 0 {
                        self.mirroring = (val >> 6) & 1;
                    }
                }
                1 => self.prg_bank_b = (self.prg_bank_b & 0xC0) | (val & 0x3F),
                2 => self.chr_bank_a = val << 1,
                3 => self.chr_bank_c = val << 1,
                4 => self.chr_bank_e = val,
                5 => self.chr_bank_f = val,
                6 => self.chr_bank_g = val,
                7 => self.chr_bank_h = val,
                8 => self.scanline_irq_latch = val,
                9 => self.scanline_irq_reload = 1,
                10 => self.scanline_irq_enabled = 1,
                11 => {
                    self.scanline_irq_enabled = 0;
                    self.set_irq(1);
                }
                12 => {
                    if (self.flags & 1) != 0 {
                        self.mirroring = (val >> 6) & 1;
                    }
                }
                _ => {}
            }
        }
        if self.mapper == 24 {
            let (vrc_hi, vrc_lo) = self.vrc24_address_bits(address);
            if ((address >> 12) & 7) == 7 {
                let irq_sel = (vrc_hi << 1) | vrc_lo;
                match irq_sel {
                    0 => self.cpu_irq_latch = (self.cpu_irq_latch & 0xF0) | (val as u16 & 0x0F),
                    1 => self.cpu_irq_latch = (self.cpu_irq_latch & 0x0F) | ((val as u16 & 0x0F) << 4),
                    2 => {
                        self.set_irq(1);
                        self.cpu_irq_control = (self.cpu_irq_control & 0xF8) | (val & 7);
                        if self.cpu_irq_control & 2 != 0 {
                            self.vrc4_irq_prescaler_counter = 0;
                            self.vrc4_irq_prescaler = 0;
                            self.cpu_irq_value = self.cpu_irq_latch as u16;
                        }
                    }
                    3 => {
                        self.set_irq(1);
                        self.cpu_irq_control =
                            (self.cpu_irq_control & 0xFD) | ((self.cpu_irq_control & 1) << 1);
                    }
                    _ => {}
                }
            } else {
                let key = (((address >> 12) & 7) << 2) as u8 | (vrc_hi << 1) | vrc_lo;
                match key {
                    0 | 1 | 2 | 3 => self.prg_bank_a = (self.prg_bank_a & 0xE0) | (val & 0x1F),
                    4 | 5 => {
                        if val != 0xFF {
                            self.mirroring = val & 3;
                        }
                    }
                    6 | 7 => self.prg_mode = (self.prg_mode & 0xFE) | ((val >> 1) & 1),
                    8 | 9 | 10 | 11 => self.prg_bank_b = (self.prg_bank_b & 0xE0) | (val & 0x1F),
                    _ => {
                        if (self.flags & 2) == 0 {
                            match key {
                                12 => self.chr_bank_a = (self.chr_bank_a & 0xF0) | (val & 0x0F),
                                13 => {
                                    self.chr_bank_a = (self.chr_bank_a & 0x0F) | ((val & 0x0F) << 4)
                                }
                                14 => self.chr_bank_b = (self.chr_bank_b & 0xF0) | (val & 0x0F),
                                15 => {
                                    self.chr_bank_b = (self.chr_bank_b & 0x0F) | ((val & 0x0F) << 4)
                                }
                                16 => self.chr_bank_c = (self.chr_bank_c & 0xF0) | (val & 0x0F),
                                17 => {
                                    self.chr_bank_c = (self.chr_bank_c & 0x0F) | ((val & 0x0F) << 4)
                                }
                                18 => self.chr_bank_d = (self.chr_bank_d & 0xF0) | (val & 0x0F),
                                19 => {
                                    self.chr_bank_d = (self.chr_bank_d & 0x0F) | ((val & 0x0F) << 4)
                                }
                                20 => self.chr_bank_e = (self.chr_bank_e & 0xF0) | (val & 0x0F),
                                21 => {
                                    self.chr_bank_e = (self.chr_bank_e & 0x0F) | ((val & 0x0F) << 4)
                                }
                                22 => self.chr_bank_f = (self.chr_bank_f & 0xF0) | (val & 0x0F),
                                23 => {
                                    self.chr_bank_f = (self.chr_bank_f & 0x0F) | ((val & 0x0F) << 4)
                                }
                                24 => self.chr_bank_g = (self.chr_bank_g & 0xF0) | (val & 0x0F),
                                25 => {
                                    self.chr_bank_g = (self.chr_bank_g & 0x0F) | ((val & 0x0F) << 4)
                                }
                                26 => self.chr_bank_h = (self.chr_bank_h & 0xF0) | (val & 0x0F),
                                27 => {
                                    self.chr_bank_h = (self.chr_bank_h & 0x0F) | ((val & 0x0F) << 4)
                                }
                                _ => {}
                            }
                        } else {
                            match key {
                                12 => self.chr_bank_a = (self.chr_bank_a & 0x78) | ((val & 0x0E) >> 1),
                                13 => {
                                    self.chr_bank_a = (self.chr_bank_a & 0x07) | ((val & 0x0F) << 3)
                                }
                                14 => self.chr_bank_b = (self.chr_bank_b & 0x78) | ((val & 0x0E) >> 1),
                                15 => {
                                    self.chr_bank_b = (self.chr_bank_b & 0x07) | ((val & 0x0F) << 3)
                                }
                                16 => self.chr_bank_c = (self.chr_bank_c & 0x78) | ((val & 0x0E) >> 1),
                                17 => {
                                    self.chr_bank_c = (self.chr_bank_c & 0x07) | ((val & 0x0F) << 3)
                                }
                                18 => self.chr_bank_d = (self.chr_bank_d & 0x78) | ((val & 0x0E) >> 1),
                                19 => {
                                    self.chr_bank_d = (self.chr_bank_d & 0x07) | ((val & 0x0F) << 3)
                                }
                                20 => self.chr_bank_e = (self.chr_bank_e & 0x78) | ((val & 0x0E) >> 1),
                                21 => {
                                    self.chr_bank_e = (self.chr_bank_e & 0x07) | ((val & 0x0F) << 3)
                                }
                                22 => self.chr_bank_f = (self.chr_bank_f & 0x78) | ((val & 0x0E) >> 1),
                                23 => {
                                    self.chr_bank_f = (self.chr_bank_f & 0x07) | ((val & 0x0F) << 3)
                                }
                                24 => self.chr_bank_g = (self.chr_bank_g & 0x78) | ((val & 0x0E) >> 1),
                                25 => {
                                    self.chr_bank_g = (self.chr_bank_g & 0x07) | ((val & 0x0F) << 3)
                                }
                                26 => self.chr_bank_h = (self.chr_bank_h & 0x78) | ((val & 0x0E) >> 1),
                                27 => {
                                    self.chr_bank_h = (self.chr_bank_h & 0x07) | ((val & 0x0F) << 3)
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        if self.mapper == 25 {
            if ((bank >> 1) & 3) == 0 {
                self.r0 = (self.r0 & 0xF0) | (val & 0x0F);
            }
            if ((bank >> 1) & 3) == 1 {
                match self.r0 & 0x0F {
                    0 => self.chr_bank_a = val,
                    1 => self.chr_bank_b = val,
                    2 => self.chr_bank_c = val,
                    3 => self.chr_bank_d = val,
                    4 => self.chr_bank_e = val,
                    5 => self.chr_bank_f = val,
                    6 => self.chr_bank_g = val,
                    7 => self.chr_bank_h = val,
                    8 => {
                        self.wram_enabled = (val >> 7) & 1;
                        self.map_rom_on_6000 = ((val >> 6) & 1) ^ 1;
                        self.prg_bank_6000 = val & 0x3F;
                    }
                    9 => self.prg_bank_a = (self.prg_bank_a & 0xC0) | (val & 0x3F),
                    10 => self.prg_bank_b = (self.prg_bank_b & 0xC0) | (val & 0x3F),
                    11 => self.prg_bank_c = (self.prg_bank_c & 0xC0) | (val & 0x3F),
                    12 => self.mirroring = val & 3,
                    13 => {
                        self.cpu_irq_control = ((val >> 6) & 1) | (val & 1);
                        self.set_irq(1);
                    }
                    14 => self.cpu_irq_value = (self.cpu_irq_value & 0xFF00) | val as u16,
                    15 => self.cpu_irq_value = (self.cpu_irq_value & 0x00FF) | ((val as u16) << 8),
                    _ => {}
                }
            }
        }
        if self.mapper == 26 {
            match bank & 3 {
                0 => self.prg_bank_a = (self.prg_bank_a & 0xC0) | (val & 0x3F),
                1 => {
                    self.prg_mode = (self.prg_mode & 6) | ((val >> 1) & 1);
                    self.mirroring = val & 1;
                }
                2 => self.prg_bank_b = (self.prg_bank_b & 0xC0) | (val & 0x3F),
                3 => match addr & 7 {
                    0 => self.chr_bank_a = val,
                    1 => self.chr_bank_b = val,
                    2 => self.chr_bank_c = val,
                    3 => self.chr_bank_d = val,
                    4 => self.chr_bank_e = val,
                    5 => self.chr_bank_f = val,
                    6 => self.chr_bank_g = val,
                    _ => self.chr_bank_h = val,
                },
                _ => {}
            }
        }
        if self.mapper == 30 {
            self.prg_bank_a = (self.prg_bank_a & 0xC1) | ((val & 0xF0) >> 3);
            self.chr_bank_a = (self.chr_bank_a & 0x87) | ((val & 0x0F) << 3);
        }
        if self.mapper == 29 && address != 0xFFFF && (address & 0xFFFE) != 0xFFFE {
            self.prg_bank_a = (self.prg_bank_a & 0xC3) | ((val & 0xF0) >> 2);
            self.chr_bank_a = (self.chr_bank_a & 0x87) | ((val & 0x0F) << 3);
            match (address >> 12) & 7 {
                0 => self.mirroring = 0,
                4 => self.mirroring = 1,
                _ => {}
            }
        }
        if self.mapper == 34 {
            match (address >> 12) & 7 {
                0 => self.prg_bank_a = (self.prg_bank_a & 0xF0) | (val & 0x0F),
                1 => {
                    self.mirroring = val & 1;
                    self.chr_bank_a = (self.chr_bank_a & 0xBF) | ((val & 0x02) << 5);
                    self.chr_bank_e = (self.chr_bank_e & 0xBF) | ((val & 0x04) << 4);
                }
                2 => self.prg_bank_b = (self.prg_bank_b & 0xF0) | (val & 0x0F),
                4 => self.prg_bank_c = (self.prg_bank_c & 0xF0) | (val & 0x0F),
                6 => self.chr_bank_a = (self.chr_bank_a & 0xC3) | ((val & 0x0F) << 2),
                7 => self.chr_bank_e = (self.chr_bank_e & 0xC3) | ((val & 0x0F) << 2),
                _ => {}
            }
        }
        if self.mapper == 36 {
            if (address & 0x800) != 0 {
                match (address >> 12) & 7 {
                    0 => self.chr_bank_a = (self.chr_bank_a & 0x81) | ((val & 0x3F) << 1),
                    1 => self.chr_bank_c = (self.chr_bank_c & 0x81) | ((val & 0x3F) << 1),
                    2 => self.chr_bank_e = (self.chr_bank_e & 0x81) | ((val & 0x3F) << 1),
                    3 => self.chr_bank_g = (self.chr_bank_g & 0x81) | ((val & 0x3F) << 1),
                    4 => {
                        self.mapper67_irq_latch ^= 1;
                        if self.mapper67_irq_latch != 0 {
                            self.mapper67_irq_counter =
                                (self.mapper67_irq_counter & 0x00FF) | ((val as u16) << 8);
                        } else {
                            self.mapper67_irq_counter =
                                (self.mapper67_irq_counter & 0xFF00) | val as u16;
                        }
                    }
                    5 => {
                        self.mapper67_irq_latch = 0;
                        self.mapper67_irq_enabled = (val >> 4) & 1;
                        self.set_irq(1);
                    }
                    6 => self.mirroring = val & 3,
                    7 => self.prg_bank_a = (self.prg_bank_a & 0xC1) | ((val & 0x0F) << 1),
                    _ => {}
                }
            } else {
                self.set_irq(1);
            }
        }
        if self.mapper == 35 {
            match (address >> 8) & 3 {
                0 => self.prg_bank_a = (self.prg_bank_a & 0xC1) | ((val & 0x0F) << 1),
                1 => {
                    self.mirroring = val & 3;
                    self.prg_mode = (self.prg_mode & !4) | (val & 0x10);
                    self.map_rom_on_6000 = (val >> 5) & 1;
                    self.mapper83_irq_enabled_latch = val >> 7;
                }
                2 => {
                    if (address & 1) == 0 {
                        self.set_irq(1);
                        self.mapper83_irq_counter =
                            (self.mapper83_irq_counter & 0xFF00) | val as u16;
                    } else {
                        self.mapper83_irq_enabled = self.mapper83_irq_enabled_latch;
                        self.mapper83_irq_counter =
                            (self.mapper83_irq_counter & 0x00FF) | ((val as u16) << 8);
                    }
                }
                3 => {
                    if (address & 0x10) == 0 {
                        match address & 3 {
                            0 => self.prg_bank_a = val,
                            1 => self.prg_bank_b = val,
                            2 => self.prg_bank_b = val,
                            _ => self.prg_bank_6000 = val,
                        }
                    } else if (self.flags & 4) == 0 {
                        match address & 7 {
                            0 => self.chr_bank_a = val,
                            1 => self.chr_bank_b = val,
                            2 => self.chr_bank_c = val,
                            3 => self.chr_bank_d = val,
                            4 => self.chr_bank_e = val,
                            5 => self.chr_bank_f = val,
                            6 => self.chr_bank_g = val,
                            _ => self.chr_bank_h = val,
                        }
                    } else {
                        let wide = (val as u16) << 1;
                        match address & 7 {
                            0 => {
                                self.chr_bank_a =
                                    (wide as u8 & 0xFE) | (self.chr_bank_a & 1);
                                self.chr_bank_a_alt = (wide >> 8) as u8;
                            }
                            1 => {
                                self.chr_bank_c =
                                    (wide as u8 & 0xFE) | (self.chr_bank_c & 1);
                                self.chr_bank_c_alt = (wide >> 8) as u8;
                            }
                            6 => {
                                self.chr_bank_e =
                                    (wide as u8 & 0xFE) | (self.chr_bank_e & 1);
                                self.chr_bank_e_alt = (wide >> 8) as u8;
                            }
                            7 => {
                                self.chr_bank_g =
                                    (wide as u8 & 0xFE) | (self.chr_bank_g & 1);
                                self.chr_bank_g_alt = (wide >> 8) as u8;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        if self.mapper == 37 {
            self.prg_bank_a = (self.prg_bank_a & 0xE1) | ((val & 0x70) >> 3);
            let chr_bits = ((val >> 7) & 1) << 3 | (val & 0x07);
            self.chr_bank_a = (self.chr_bank_a & 0x07) | (chr_bits << 3);
            self.mirroring = 2 | ((val >> 3) & 1);
        }
        self.sync();
    }

    fn mirror_address(&self, address: u16) -> u16 {
        if self.four_screen != 0 {
            return address & 0x2FFF;
        }
        if self.mapper == 20 && (self.flags & 1) != 0 {
            let slot = ((address & 0x1FFF) >> 10) as usize;
            if slot < 8 && self.tksmir[slot] != 0 {
                return Self::mirror_single_1(address);
            }
            return Self::mirror_single_0(address);
        }
        match self.mirroring {
            0 => mirror_h_or_v(false, address),
            1 => mirror_h_or_v(true, address),
            2 => Self::mirror_single_0(address),
            _ => Self::mirror_single_1(address),
        }
    }
}

impl Mapper for Mapper342 {
    fn reset(&mut self) {
        self.flash_state = 0;
        self.cfi_mode = 0;
        self.irq_asserted = false;
        self.irq_ack = false;
    }

    fn reset_power_cycle(&mut self) {
        self.wram_enabled = 0;
        self.wram_page = 0;
        self.can_write_chr_ram = 0;
        self.can_write_flash = 0;
        self.map_rom_on_6000 = 0;
        self.flags = 0;
        self.mapper = 0;
        self.can_write_prg = 0;
        self.mirroring = 0;
        self.four_screen = 0;
        self.lockout = 0;
        self.prg_base = 0;
        self.prg_mask = 0xF8;
        self.prg_mode = 0;
        self.prg_bank_6000 = 0;
        self.prg_bank_a = 0;
        self.prg_bank_b = 1;
        self.prg_bank_c = 0xFE;
        self.prg_bank_d = 0xFF;
        self.chr_mask = 0;
        self.chr_mode = 0;
        self.chr_bank_a = 0;
        self.chr_bank_b = 1;
        self.chr_bank_c = 2;
        self.chr_bank_d = 3;
        self.chr_bank_e = 4;
        self.chr_bank_f = 5;
        self.chr_bank_g = 6;
        self.chr_bank_h = 7;
        self.scanline_irq_enabled = 0;
        self.scanline_irq_counter = 0;
        self.scanline_irq_latch = 0;
        self.scanline_irq_reload = 0;
        self.scanline2_irq_enabled = 0;
        self.scanline2_irq_line = 0;
        self.scanline2_irq_pending = 0;
        self.cpu_irq_value = 0;
        self.cpu_irq_control = 0;
        self.cpu_irq_latch = 0;
        self.vrc4_irq_prescaler = 0;
        self.vrc4_irq_prescaler_counter = 0;
        self.r0 = 0;
        self.r1 = 0;
        self.r2 = 0;
        self.r3 = 0;
        self.r4 = 0;
        self.r5 = 0;
        self.mapper67_irq_enabled = 0;
        self.mapper67_irq_latch = 0;
        self.mapper67_irq_counter = 0;
        self.mapper83_irq_enabled = 0;
        self.mapper83_irq_enabled_latch = 0;
        self.mapper83_irq_counter = 0;
        self.ppu_latch0 = 0;
        self.ppu_latch1 = 0;
        self.ppu_mapper163_latch = 0;
        self.flash_state = 0;
        self.cfi_mode = 0;
        self.irq_asserted = false;
        self.irq_ack = false;
        self.sync();
    }

    fn battery_save_data(&self, _cart: &Cartridge) -> Option<Vec<u8>> {
        Some(self.save_flash.clone())
    }

    fn load_battery_save(&mut self, _cart: &mut Cartridge, data: &[u8]) {
        let copy_len = data.len().min(SAVE_FLASH_SIZE);
        self.save_flash[..copy_len].copy_from_slice(&data[..copy_len]);
        if copy_len < SAVE_FLASH_SIZE {
            self.save_flash[copy_len..].fill(0xFF);
        }
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if self.mapper == 29 && (address & 0xE100) == 0x4100 {
            return FetchResult {
                data: (self.prg_bank_a & 0x0C) << 2,
                driven: true,
            };
        }
        if address >= 0x5000 && address < 0x6000 {
            return self.read_5(address);
        }
        if address >= 0x6000 && address < 0x8000 {
            match self.prg_slot_6000 {
                PrgSlot6000::Rom => {
                    if cart.prg_rom.is_empty() {
                        return FetchResult { data: 0, driven: true };
                    }
                    let off = self.prg_bank_6000_mapped as usize * 0x2000
                        + (address as usize - 0x6000);
                    return FetchResult {
                        data: cart.prg_rom[off % cart.prg_rom.len()],
                        driven: true,
                    };
                }
                PrgSlot6000::Wram => {
                    if cart.prg_ram.is_empty() {
                        return FetchResult { data: 0, driven: true };
                    }
                    let off = (self.wram_page as usize) * 0x2000 + (address as usize - 0x6000);
                    return FetchResult {
                        data: cart.prg_ram[off % cart.prg_ram.len()],
                        driven: true,
                    };
                }
                PrgSlot6000::OpenBus => {}
            }
            return FetchResult { data: 0, driven: false };
        }
        if address >= 0x8000 {
            return FetchResult {
                data: self.read_prg_byte(cart, address),
                driven: true,
            };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (address & 0xE100) == 0x4100 && self.write_nina_latch(data) {
            return;
        }
        if address >= 0x4000 && address < 0x5000 {
            self.write_4(address & 0xFFF, data);
        } else if address >= 0x5000 && address < 0x6000 {
            self.write_5(address & 0xFFF, data);
        } else if address >= 0x6000 && address < 0x8000 {
            self.write_67(cart, address, data);
        } else if address >= 0x8000 {
            if self.can_write_flash != 0 {
                self.flash_write(address, data);
            }
            self.write_8f(address, data);
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror_address(address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            if !chr_ram.is_empty() {
                let bank = (address >> 10) as usize;
                let offset = self.chr_1k_banks[bank] as usize * 0x400 + (address as usize & 0x3FF);
                new_addr_bus |= chr_ram[offset % chr_ram.len()] as u16;
            }
        } else if address < 0x3F00 {
            let mirrored = self.mirror_address(address);
            let byte = if self.four_screen != 0 && (mirrored & 0x0800) != 0 {
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
            if self.can_write_chr_ram != 0 && !cart.chr_ram.is_empty() {
                let bank = (address >> 10) as usize;
                let offset = self.chr_1k_banks[bank] as usize * 0x400 + (address as usize & 0x3FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_address(address);
            if self.four_screen != 0 && (mirrored & 0x0800) != 0 {
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
        _ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        _ppu_sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        self.last_ppu_scanline = scanline as i32;
        self.last_ppu_is_rendering = if rendering_on { 1 } else { 0 };
        let mut fire = false;
        if self.uses_mmc3_scanline_irq()
            && rendering_on
            && scanline < 240
            && dot == 260
        {
            if self.clock_mmc3_scanline_irq() {
                fire = true;
            }
        }
        if rendering_on && dot == 260 {
            if self.scanline2_irq_line as u16 == scanline + 1 && self.scanline2_irq_enabled != 0 {
                self.set_irq(0);
                self.scanline2_irq_pending = 1;
                fire = true;
            }
            if self.mapper == 6 {
                if scanline == 239 {
                    self.ppu_mapper163_latch = 0;
                    self.sync_chr();
                } else if scanline == 127 {
                    self.ppu_mapper163_latch = 1;
                    self.sync_chr();
                }
            }
        }
        if self.mapper == 17 {
            let a = ppu_address_bus >> 4;
            if a == 0xFD {
                self.ppu_latch0 = 0;
                self.sync_chr();
            } else if a == 0xFE {
                self.ppu_latch0 = 1;
                self.sync_chr();
            } else if a == 0x1FD {
                self.ppu_latch1 = 0;
                self.sync_chr();
            } else if a == 0x1FE {
                self.ppu_latch1 = 1;
                self.sync_chr();
            }
        }
        if fire || self.irq_asserted {
            return self.irq_asserted;
        }
        false
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        let mut fire = false;
        if self.mapper == 24 && (self.cpu_irq_control & 2) != 0 {
            if (self.cpu_irq_control & 4) != 0 {
                self.cpu_irq_value = self.cpu_irq_value.wrapping_add(1);
                if (self.cpu_irq_value & 0xFF) == 0 {
                    self.set_irq(0);
                    self.cpu_irq_value = self.cpu_irq_latch as u16;
                    fire = true;
                }
            } else {
                self.vrc4_irq_prescaler = self.vrc4_irq_prescaler.wrapping_add(1);
                let prescale_hit = if (self.vrc4_irq_prescaler_counter & 2) == 0 {
                    self.vrc4_irq_prescaler == 114
                } else {
                    self.vrc4_irq_prescaler == 113
                };
                if prescale_hit {
                    self.cpu_irq_value = self.cpu_irq_value.wrapping_add(1);
                    self.vrc4_irq_prescaler = 0;
                    self.vrc4_irq_prescaler_counter =
                        self.vrc4_irq_prescaler_counter.wrapping_add(1);
                    if self.vrc4_irq_prescaler_counter == 3 {
                        self.vrc4_irq_prescaler_counter = 0;
                    }
                    if (self.cpu_irq_value & 0xFF) == 0 {
                        self.set_irq(0);
                        self.cpu_irq_value = self.cpu_irq_latch as u16;
                        fire = true;
                    }
                }
            }
        }
        if self.mapper == 19 && (self.cpu_irq_control & 2) != 0 {
            if (self.cpu_irq_control & 4) != 0 {
                self.cpu_irq_value = (self.cpu_irq_value & 0xFF00)
                    | (self.cpu_irq_value.wrapping_add(1) & 0xFF);
                if (self.cpu_irq_value & 0xFF) == 0 {
                    self.set_irq(0);
                    self.cpu_irq_value =
                        (self.cpu_irq_value & 0xFF00) | (self.cpu_irq_latch as u16 & 0xFF);
                    fire = true;
                }
            } else {
                self.cpu_irq_value = self.cpu_irq_value.wrapping_add(1);
                if (self.cpu_irq_value & 0xFFFF) == 0 {
                    self.set_irq(0);
                    self.cpu_irq_value = self.cpu_irq_latch;
                    fire = true;
                }
            }
        }
        if self.mapper == 25 {
            if self.cpu_irq_value == 0 && (self.cpu_irq_control & 1) != 0 {
                self.set_irq(0);
                fire = true;
            }
            self.cpu_irq_value = self.cpu_irq_value.wrapping_sub(1);
        }
        if self.mapper == 7 && (self.cpu_irq_control & 1) != 0 {
            if (self.cpu_irq_control & 8) != 0 {
                if (self.cpu_irq_value & 0x000F) == 0 {
                    self.set_irq(0);
                    fire = true;
                }
                self.cpu_irq_value = (self.cpu_irq_value & 0xFFF0)
                    | (self.cpu_irq_value.wrapping_sub(1) & 0x000F);
            } else if (self.cpu_irq_control & 4) != 0 {
                if (self.cpu_irq_value & 0x00FF) == 0 {
                    self.set_irq(0);
                    fire = true;
                }
                self.cpu_irq_value = (self.cpu_irq_value & 0xFF00)
                    | (self.cpu_irq_value.wrapping_sub(1) & 0x00FF);
            } else if (self.cpu_irq_control & 2) != 0 {
                if (self.cpu_irq_value & 0x0FFF) == 0 {
                    self.set_irq(0);
                    fire = true;
                }
                self.cpu_irq_value = (self.cpu_irq_value & 0xF000)
                    | (self.cpu_irq_value.wrapping_sub(1) & 0x0FFF);
            } else {
                if (self.cpu_irq_value & 0xFFFF) == 0 {
                    self.set_irq(0);
                    fire = true;
                }
                self.cpu_irq_value = self.cpu_irq_value.wrapping_sub(1) & 0xFFFF;
            }
        }
        if self.mapper == 14 && (self.cpu_irq_control & 1) != 0 && self.cpu_irq_value > 0 {
            self.cpu_irq_value = self.cpu_irq_value.wrapping_sub(1);
            if self.cpu_irq_value == 0 {
                self.set_irq(0);
                fire = true;
            }
        }
        if self.mapper == 36 && self.mapper67_irq_enabled != 0 {
            self.mapper67_irq_counter = self.mapper67_irq_counter.wrapping_sub(1);
            if self.mapper67_irq_counter == 0xFFFF {
                self.set_irq(0);
                self.mapper67_irq_enabled = 0;
                fire = true;
            }
        }
        if self.mapper == 35 && self.mapper83_irq_enabled != 0 {
            if self.mapper83_irq_counter == 0 {
                self.set_irq(0);
                fire = true;
            }
            self.mapper83_irq_counter = self.mapper83_irq_counter.wrapping_sub(1);
        }
        if fire || self.irq_asserted {
            return self.irq_asserted;
        }
        false
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack;
        self.irq_ack = false;
        ack
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut s = Vec::new();
        s.push(self.wram_enabled);
        s.push(self.map_rom_on_6000);
        s.push(self.wram_page);
        s.push(self.can_write_chr_ram);
        s.push(self.can_write_prg);
        s.push(self.can_write_flash);
        s.push(self.flash_state);
        s.extend_from_slice(&(self.flash_buffer_a[0].to_le_bytes()));
        s.extend_from_slice(&(self.flash_buffer_a[1].to_le_bytes()));
        s.extend_from_slice(&(self.flash_buffer_a[2].to_le_bytes()));
        s.extend_from_slice(&(self.flash_buffer_a[3].to_le_bytes()));
        s.extend_from_slice(&(self.flash_buffer_a[4].to_le_bytes()));
        s.extend_from_slice(&(self.flash_buffer_a[5].to_le_bytes()));
        s.extend_from_slice(&(self.flash_buffer_a[6].to_le_bytes()));
        s.extend_from_slice(&(self.flash_buffer_a[7].to_le_bytes()));
        s.extend_from_slice(&(self.flash_buffer_a[8].to_le_bytes()));
        s.extend_from_slice(&(self.flash_buffer_a[9].to_le_bytes()));
        s.extend_from_slice(&self.flash_buffer_v);
        s.push(self.cfi_mode);
        s.push(self.flags);
        s.push(self.mapper);
        s.push(self.mirroring);
        s.push(self.four_screen);
        s.push(self.lockout);
        s.extend_from_slice(&self.prg_base.to_le_bytes());
        s.push(self.prg_mask);
        s.push(self.prg_mode);
        s.push(self.prg_bank_6000);
        s.push(self.prg_bank_a);
        s.push(self.prg_bank_b);
        s.push(self.prg_bank_c);
        s.push(self.prg_bank_d);
        s.extend_from_slice(&self.prg_bank_6000_mapped.to_le_bytes());
        s.extend_from_slice(&self.prg_bank_a_mapped.to_le_bytes());
        s.extend_from_slice(&self.prg_bank_b_mapped.to_le_bytes());
        s.extend_from_slice(&self.prg_bank_c_mapped.to_le_bytes());
        s.extend_from_slice(&self.prg_bank_d_mapped.to_le_bytes());
        s.push(self.chr_mask);
        s.push(self.chr_mode);
        s.push(self.chr_bank_a);
        s.push(self.chr_bank_b);
        s.push(self.chr_bank_c);
        s.push(self.chr_bank_d);
        s.push(self.chr_bank_e);
        s.push(self.chr_bank_f);
        s.push(self.chr_bank_g);
        s.push(self.chr_bank_h);
        s.push(self.chr_bank_a_alt);
        s.push(self.chr_bank_b_alt);
        s.push(self.chr_bank_c_alt);
        s.push(self.chr_bank_d_alt);
        s.push(self.chr_bank_e_alt);
        s.push(self.chr_bank_g_alt);
        s.push(self.ppu_latch0);
        s.push(self.ppu_latch1);
        s.push(self.ppu_mapper163_latch);
        s.extend_from_slice(&self.tksmir);
        s.push(self.scanline_irq_enabled);
        s.push(self.scanline_irq_counter);
        s.push(self.scanline_irq_latch);
        s.push(self.scanline_irq_reload);
        s.push(self.scanline2_irq_enabled);
        s.push(self.scanline2_irq_line);
        s.push(self.scanline2_irq_pending);
        s.extend_from_slice(&self.cpu_irq_value.to_le_bytes());
        s.push(self.cpu_irq_control);
        s.extend_from_slice(&self.cpu_irq_latch.to_le_bytes());
        s.push(self.vrc4_irq_prescaler);
        s.push(self.vrc4_irq_prescaler_counter);
        s.push(self.r0);
        s.push(self.r1);
        s.push(self.r2);
        s.push(self.r3);
        s.push(self.r4);
        s.push(self.r5);
        s.push(self.mul1);
        s.push(self.mul2);
        s
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        macro_rules! byte {
            () => {{
                let v = state.get(p).copied().unwrap_or(0);
                p += 1;
                v
            }};
        }
        macro_rules! word {
            () => {{
                let v = if p + 1 < state.len() {
                    u16::from_le_bytes([state[p], state[p + 1]])
                } else {
                    0
                };
                p += 2;
                v
            }};
        }
        macro_rules! dword {
            () => {{
                let v = if p + 3 < state.len() {
                    u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]])
                } else {
                    0
                };
                p += 4;
                v
            }};
        }
        self.wram_enabled = byte!();
        self.map_rom_on_6000 = byte!();
        self.wram_page = byte!();
        self.can_write_chr_ram = byte!();
        self.can_write_prg = byte!();
        if state.len() > p {
            self.can_write_flash = byte!();
            self.flash_state = byte!();
            for slot in &mut self.flash_buffer_a {
                *slot = word!();
            }
            for slot in &mut self.flash_buffer_v {
                *slot = byte!();
            }
            self.cfi_mode = byte!();
        }
        self.flags = byte!();
        self.mapper = byte!();
        self.mirroring = byte!();
        self.four_screen = byte!();
        self.lockout = byte!();
        self.prg_base = word!();
        self.prg_mask = byte!();
        self.prg_mode = byte!();
        self.prg_bank_6000 = byte!();
        self.prg_bank_a = byte!();
        self.prg_bank_b = byte!();
        self.prg_bank_c = byte!();
        self.prg_bank_d = byte!();
        self.prg_bank_6000_mapped = dword!();
        self.prg_bank_a_mapped = dword!();
        self.prg_bank_b_mapped = dword!();
        self.prg_bank_c_mapped = dword!();
        self.prg_bank_d_mapped = dword!();
        self.chr_mask = byte!();
        self.chr_mode = byte!();
        self.chr_bank_a = byte!();
        self.chr_bank_b = byte!();
        self.chr_bank_c = byte!();
        self.chr_bank_d = byte!();
        self.chr_bank_e = byte!();
        self.chr_bank_f = byte!();
        self.chr_bank_g = byte!();
        self.chr_bank_h = byte!();
        self.chr_bank_a_alt = byte!();
        self.chr_bank_b_alt = byte!();
        self.chr_bank_c_alt = byte!();
        self.chr_bank_d_alt = byte!();
        self.chr_bank_e_alt = byte!();
        self.chr_bank_g_alt = byte!();
        self.ppu_latch0 = byte!();
        self.ppu_latch1 = byte!();
        self.ppu_mapper163_latch = byte!();
        for i in 0..8 {
            self.tksmir[i] = byte!();
        }
        self.scanline_irq_enabled = byte!();
        self.scanline_irq_counter = byte!();
        self.scanline_irq_latch = byte!();
        self.scanline_irq_reload = byte!();
        self.scanline2_irq_enabled = byte!();
        self.scanline2_irq_line = byte!();
        self.scanline2_irq_pending = byte!();
        self.cpu_irq_value = word!();
        self.cpu_irq_control = byte!();
        self.cpu_irq_latch = word!();
        self.vrc4_irq_prescaler = byte!();
        self.vrc4_irq_prescaler_counter = byte!();
        self.r0 = byte!();
        self.r1 = byte!();
        self.r2 = byte!();
        self.r3 = byte!();
        self.r4 = byte!();
        self.r5 = byte!();
        self.mul1 = byte!();
        self.mul2 = byte!();
        self.sync();
        p
    }
}
