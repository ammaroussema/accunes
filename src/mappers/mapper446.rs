use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

const MAPPER_UNROM: u8 = 0x00;
const MAPPER_MMC3: u8 = 0x01;
const MAPPER_BNROM: u8 = 0x02;
const MAPPER_CNROM: u8 = 0x03;
const MAPPER_ANROM: u8 = 0x04;
const MAPPER_SLROM: u8 = 0x05;
const MAPPER_SNROM: u8 = 0x06;
const MAPPER_SUROM: u8 = 0x07;
const MAPPER_GNROM: u8 = 0x08;
const MAPPER_PNROM: u8 = 0x09;
const MAPPER_HKROM: u8 = 0x0A;
const MAPPER_BANDAI152: u8 = 0x0B;
const MAPPER_TLSROM: u8 = 0x0E;
const MAPPER_189: u8 = 0x0F;
const MAPPER_VRC6_24: u8 = 0x10;
const MAPPER_VRC6_26: u8 = 0x11;
const MAPPER_VRC2_22: u8 = 0x12;
const MAPPER_VRC3: u8 = 0x13;
const MAPPER_VRC4_25: u8 = 0x15;
const MAPPER_VRC4_23: u8 = 0x18;
const MAPPER_VRC4_21: u8 = 0x19;
const MAPPER_VRC1: u8 = 0x1A;
const MAPPER_H3001: u8 = 0x1F;

#[derive(Clone, Copy)]
struct Mmc1 {
    reg: [u8; 4],
    shift: u8,
    bits: u8,
    filter: u8,
}

#[derive(Clone, Copy)]
struct Mmc2 {
    prg: u8,
    chr: [u8; 4],
    state: [u8; 2],
    mirroring: u8,
}

#[derive(Clone, Copy)]
struct Mmc3 {
    index: u8,
    reg: [u8; 8],
    mirroring: u8,
    wram_control: u8,
    enable_irq: bool,
    reload: bool,
    counter: u8,
    reload_value: u8,
    pa12_filter: u8,
    is_mmc6: bool,
}

#[derive(Clone, Copy)]
struct Vrc1 {
    prg: [u8; 3],
    chr: [u8; 2],
    misc: u8,
}

#[derive(Clone, Copy)]
struct Vrc24 {
    prg: [u8; 2],
    chr: [u16; 8],
    mirroring: u8,
    pins: u8,
    latch: u8,
    mode: u8,
    counter: u8,
    cycles: i16,
    misc: u8,
    is_vrc4: bool,
    a0: u8,
    a1: u8,
}

#[derive(Clone, Copy)]
struct Vrc3 {
    prg: u8,
    irq: u8,
    counter: u16,
    latch: u16,
}

#[derive(Clone, Copy)]
struct Vrc6 {
    irq_control: u8,
    irq_counter: u8,
    irq_latch: u8,
    irq_cycles: i16,
    mode: u8,
    prg: [u8; 2],
    chr: [u8; 8],
    a0: u8,
    a1: u8,
}

#[derive(Clone, Copy)]
struct H3001 {
    prg_invert: u8,
    prg: [u8; 2],
    chr: [u8; 8],
    mirroring: u8,
    irq: u8,
    counter: u16,
    latch: u16,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChrSource {
    Ram,
    Rom,
    Auto,
}

impl Mmc1 {
    fn prg16_bank(&self, bank: u8, andh: u16, orh: u16) -> u16 {
        let prg = self.reg[3] as u16;
        let result = if self.reg[0] & 0x08 != 0 {
            if self.reg[0] & 0x04 != 0 {
                prg | bank.wrapping_mul(0xF) as u16
            } else {
                prg & bank.wrapping_mul(0xF) as u16
            }
        } else {
            prg & !1 | bank as u16
        };
        (result & 0x0F) & andh | orh
    }

    fn chr_bank(&self, bank: u8) -> u16 {
        if self.reg[0] & 0x10 != 0 {
            self.reg[1 + bank as usize] as u16
        } else {
            self.reg[1] as u16 & !1 | bank as u16
        }
    }
}

impl Mmc3 {
    fn prg_bank(&self, bank: usize) -> u8 {
        let mut b = bank as u8;
        if self.index & 0x40 != 0 && b & 1 == 0 {
            b ^= 2;
        }
        if b & 2 != 0 {
            0xFE | b & 1
        } else {
            self.reg[6 | (b & 1) as usize]
        }
    }

    fn chr_bank(&self, bank: usize) -> u8 {
        let mut b = bank as u8;
        if self.index & 0x80 != 0 {
            b ^= 4;
        }
        if b & 4 != 0 {
            self.reg[(b - 2) as usize]
        } else {
            self.reg[(b >> 1) as usize] & !1 | b & 1
        }
    }
}

impl Vrc24 {
    fn prg_banks(&self) -> [u8; 4] {
        let mut out = [0u8; 4];
        for bank in 0..4 {
            let mut b = bank as u8;
            if b & 1 == 0 && self.misc & 2 != 0 {
                b ^= 2;
            }
            out[bank] = if b & 2 != 0 {
                0xFE | b & 1
            } else {
                self.prg[(b & 1) as usize]
            };
        }
        out
    }
}

pub struct Mapper446 {
    submapper_id: u8,
    reg: [u8; 8],
    latch_addr: u16,
    latch_data: u8,
    latch_189: u8,
    mmc1: Mmc1,
    mmc2: Mmc2,
    mmc3: Mmc3,
    vrc1: Vrc1,
    vrc24: Vrc24,
    vrc3: Vrc3,
    vrc6: Vrc6,
    h3001: H3001,
    flash_chip_size: usize,
    flash_state: u8,
    flash_time_out: u32,
    irq_ack_pending: bool,
}

impl Mapper446 {
    pub fn new(submapper_id: u8, _header: &[u8], rom: &[u8], _rom_name: &str) -> Self {
        let mut m = Self {
            submapper_id,
            reg: [0; 8],
            latch_addr: 0,
            latch_data: 0,
            latch_189: 0,
            mmc1: Mmc1 { reg: [0x0C, 0, 0, 0], shift: 0, bits: 0, filter: 0 },
            mmc2: Mmc2 { prg: 0, chr: [0; 4], state: [0; 2], mirroring: 0 },
            mmc3: Mmc3 {
                index: 0,
                reg: [0, 2, 4, 5, 6, 7, 0, 1],
                mirroring: 0,
                wram_control: 0,
                enable_irq: false,
                reload: false,
                counter: 0,
                reload_value: 0,
                pa12_filter: 0,
                is_mmc6: false,
            },
            vrc1: Vrc1 { prg: [0; 3], chr: [0; 2], misc: 0 },
            vrc24: Vrc24 {
                prg: [0, 1],
                chr: [0, 1, 2, 3, 4, 5, 6, 7],
                mirroring: 0,
                pins: 0,
                latch: 0,
                mode: 0,
                counter: 0,
                cycles: 0,
                misc: 0,
                is_vrc4: true,
                a0: 0x01,
                a1: 0x02,
            },
            vrc3: Vrc3 { prg: 0, irq: 0, counter: 0, latch: 0 },
            vrc6: Vrc6 {
                irq_control: 0,
                irq_counter: 0,
                irq_latch: 0,
                irq_cycles: 0,
                mode: 0,
                prg: [0, 0xFE],
                chr: [0, 1, 2, 3, 4, 5, 6, 7],
                a0: 0x01,
                a1: 0x02,
            },
            h3001: H3001 {
                prg_invert: 0,
                prg: [0, 1],
                chr: [0, 1, 2, 3, 4, 5, 6, 7],
                mirroring: 0,
                irq: 0,
                counter: 0,
                latch: 0,
            },
            flash_chip_size: rom.len(),
            flash_state: 0,
            flash_time_out: 0,
            irq_ack_pending: false,
        };
        m.apply_mode();
        m
    }

    fn mapper(&self) -> u8 {
        self.reg[0] & 0x1F
    }

    fn locked(&self) -> bool {
        (self.reg[0] & 0x80) != 0
    }

    fn mirror_v(&self) -> bool {
        (self.reg[4] & 1) != 0
    }

    fn protect_chr(&self) -> bool {
        (self.reg[5] & 4) != 0
    }

    fn prg_or(&self) -> u16 {
        self.reg[1] as u16 | (self.reg[2] as u16) << 8
    }

    fn prg_and(&self) -> u16 {
        self.reg[3] as u16
    }

    fn chr_and(&self) -> u16 {
        if (self.reg[4] & 0x20) != 0 {
            if (self.reg[4] & 0x10) != 0 {
                0x1F
            } else {
                0x7F
            }
        } else {
            0xFF
        }
    }

    fn outer_chr(&self) -> u16 {
        self.reg[6] as u16
    }

    fn boot_slots(&self) -> [usize; 4] {
        let e = if self.submapper_id == 3 { 0x1Eu16 } else { 0x3Eu16 };
        let or = self.prg_or();
        [or as usize, (or + 1) as usize, e as usize, (e + 1) as usize]
    }

    fn prg_slots(&self) -> [usize; 4] {
        let and = self.prg_and();
        let or = self.prg_or();
        let andh = and >> 1;
        let orh = or >> 1;
        let map = |x: u16| (x & and | or) as usize;
        if !self.locked() {
            self.boot_slots()
        } else {
            match self.mapper() {
                MAPPER_UNROM => {
                    let b0 = self.latch_data as u16 & andh | orh;
                    let b2 = andh & 0x1F | orh;
                    [(b0 << 1) as usize, ((b0 << 1) | 1) as usize, (b2 << 1) as usize, ((b2 << 1) | 1) as usize]
                }
                MAPPER_BNROM => {
                    let b = (self.latch_data as u16) << 2;
                    [(b | 0) & and | or, (b | 1) & and | or, (b | 2) & and | or, (b | 3) & and | or]
                        .map(|x| x as usize)
                }
                MAPPER_CNROM => [
                    map(0),
                    map(1),
                    map(2),
                    map(3),
                ],
                MAPPER_ANROM => {
                    let b = self.latch_data as u16 & (and >> 2) | (or >> 2);
                    [(b << 2) as usize, ((b << 2) | 1) as usize, ((b << 2) | 2) as usize, ((b << 2) | 3) as usize]
                }
                MAPPER_GNROM => {
                    let b = (self.latch_data as u16) >> 4 & (and >> 2) | (or >> 2);
                    [(b << 2) as usize, ((b << 2) | 1) as usize, ((b << 2) | 2) as usize, ((b << 2) | 3) as usize]
                }
                MAPPER_BANDAI152 => {
                    let b0 = (self.latch_data as u16) >> 4 & andh | orh;
                    let b2 = 0xFF & andh | orh;
                    [(b0 << 1) as usize, ((b0 << 1) | 1) as usize, (b2 << 1) as usize, ((b2 << 1) | 1) as usize]
                }
                MAPPER_SLROM | MAPPER_SNROM => {
                    let a = self.mmc1.prg16_bank(0, andh, orh);
                    let b = self.mmc1.prg16_bank(1, andh, orh);
                    [(a << 1) as usize, ((a << 1) | 1) as usize, (b << 1) as usize, ((b << 1) | 1) as usize]
                }
                MAPPER_PNROM => [
                    map(self.mmc2.prg as u16),
                    map(0xD),
                    map(0xE),
                    map(0xF),
                ],
                MAPPER_MMC3 | MAPPER_HKROM => {
                    let a = and & 0x3F;
                    (0..4)
                        .map(|b| ((self.mmc3.prg_bank(b) as u16) & a | or) as usize)
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap()
                }
                MAPPER_TLSROM => (0..4)
                    .map(|b| ((self.mmc3.prg_bank(b) as u16) & and | or) as usize)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
                MAPPER_189 => {
                    let b = self.latch_189 as u16 & 3 | or >> 2;
                    [(b << 2) as usize, ((b << 2) | 1) as usize, ((b << 2) | 2) as usize, ((b << 2) | 3) as usize]
                }
                MAPPER_VRC1 => [
                    map(self.vrc1.prg[0] as u16),
                    map(self.vrc1.prg[1] as u16),
                    map(self.vrc1.prg[2] as u16),
                    map(0xFF),
                ],
                MAPPER_VRC2_22 | MAPPER_VRC4_21 | MAPPER_VRC4_23 | MAPPER_VRC4_25 => {
                    let banks = self.vrc24.prg_banks();
                    [
                        map(banks[0] as u16),
                        map(banks[1] as u16),
                        map(banks[2] as u16),
                        map(banks[3] as u16),
                    ]
                }
                MAPPER_VRC3 => {
                    let a = self.vrc3.prg as u16 & andh | orh;
                    let b = 0xFF & andh | orh;
                    [(a << 1) as usize, ((a << 1) | 1) as usize, (b << 1) as usize, ((b << 1) | 1) as usize]
                }
                MAPPER_VRC6_24 | MAPPER_VRC6_26 => {
                    let a = self.vrc6.prg[0] as u16 & andh | orh;
                    [
                        (a << 1) as usize,
                        ((a << 1) | 1) as usize,
                        (self.vrc6.prg[1] as u16 & and | or) as usize,
                        (0xFF & and | or) as usize,
                    ]
                }
                MAPPER_H3001 => [
                    map(if self.h3001.prg_invert & 0x80 != 0 { 0xFE } else { self.h3001.prg[0] as u16 }),
                    map(self.h3001.prg[1] as u16),
                    map(if self.h3001.prg_invert & 0x80 != 0 { self.h3001.prg[0] as u16 } else { 0xFE }),
                    map(0xFF),
                ],
                _ => self.boot_slots(),
            }
        }
    }

    fn chr_source(&self) -> ChrSource {
        if !self.locked() {
            return ChrSource::Ram;
        }
        match self.mapper() {
            MAPPER_UNROM
            | MAPPER_BNROM
            | MAPPER_CNROM
            | MAPPER_ANROM
            | MAPPER_GNROM
            | MAPPER_BANDAI152 => ChrSource::Ram,
            MAPPER_VRC2_22 | MAPPER_VRC3 => ChrSource::Ram,
            MAPPER_VRC6_24 | MAPPER_VRC6_26 => ChrSource::Rom,
            MAPPER_SLROM | MAPPER_SNROM | MAPPER_PNROM | MAPPER_MMC3 | MAPPER_HKROM | MAPPER_TLSROM
            | MAPPER_189 | MAPPER_VRC1 | MAPPER_VRC4_21 | MAPPER_VRC4_23 | MAPPER_VRC4_25
            | MAPPER_H3001 => ChrSource::Auto,
            _ => ChrSource::Ram,
        }
    }

    fn chr_1k_slots(&self, chr_rom: &[u8]) -> [usize; 8] {
        if !self.locked() {
            let c = self.outer_chr() as usize;
            return [
                c * 8,
                c * 8 + 1,
                c * 8 + 2,
                c * 8 + 3,
                c * 8 + 4,
                c * 8 + 5,
                c * 8 + 6,
                c * 8 + 7,
            ];
        }
        match self.mapper() {
            MAPPER_UNROM | MAPPER_BNROM | MAPPER_ANROM => {
                let c = self.outer_chr() as usize;
                [
                    c * 8,
                    c * 8 + 1,
                    c * 8 + 2,
                    c * 8 + 3,
                    c * 8 + 4,
                    c * 8 + 5,
                    c * 8 + 6,
                    c * 8 + 7,
                ]
            }
            MAPPER_CNROM | MAPPER_GNROM => {
                let c = (self.latch_data as usize) & 3;
                [
                    c * 8,
                    c * 8 + 1,
                    c * 8 + 2,
                    c * 8 + 3,
                    c * 8 + 4,
                    c * 8 + 5,
                    c * 8 + 6,
                    c * 8 + 7,
                ]
            }
            MAPPER_BANDAI152 => {
                let c = (self.latch_data as usize) & 0xF;
                [
                    c * 8,
                    c * 8 + 1,
                    c * 8 + 2,
                    c * 8 + 3,
                    c * 8 + 4,
                    c * 8 + 5,
                    c * 8 + 6,
                    c * 8 + 7,
                ]
            }
            MAPPER_SLROM | MAPPER_SNROM => {
                let or2 = (self.outer_chr() >> 2) as u16;
                let b0 = (self.mmc1.chr_bank(0) & 0x1F) | or2;
                let b1 = (self.mmc1.chr_bank(1) & 0x1F) | or2;
                self.chr_expand_4k(b0, b1)
            }
            MAPPER_PNROM => {
                let b0 = (self.mmc2.chr[self.mmc2.state[0] as usize] as u16) & 0x1F;
                let b1 = (self.mmc2.chr[(self.mmc2.state[1] | 2) as usize] as u16) & 0x1F;
                self.chr_expand_4k(b0, b1)
            }
            MAPPER_MMC3 | MAPPER_HKROM | MAPPER_TLSROM | MAPPER_189 => {
                let and = match self.mapper() {
                    MAPPER_TLSROM => 0x7F,
                    MAPPER_189 => 0xFF,
                    _ => self.chr_and(),
                };
                let or = self.outer_chr();
                let mut out = [0usize; 8];
                for b in 0..8 {
                    out[b] = ((self.mmc3.chr_bank(b) as u16) & and | or) as usize;
                }
                out
            }
            MAPPER_VRC1 => {
                let b0 = ((self.vrc1.chr[0] & 0x0F) as u16 | (((self.vrc1.misc as u16) << 3) & 0x10)) & 0x1F;
                let b1 = ((self.vrc1.chr[1] & 0x0F) as u16 | (((self.vrc1.misc as u16) << 2) & 0x10)) & 0x1F;
                self.chr_expand_4k(b0, b1)
            }
            MAPPER_VRC2_22 => {
                let mut out = [0usize; 8];
                for b in 0..8 {
                    out[b] = (self.vrc24.chr[b] >> 1) as usize;
                }
                out
            }
            MAPPER_VRC4_21 | MAPPER_VRC4_23 | MAPPER_VRC4_25 => {
                let mut out = [0usize; 8];
                for b in 0..8 {
                    out[b] = (self.vrc24.chr[b] & 0xFF) as usize;
                }
                out
            }
            MAPPER_VRC6_24 | MAPPER_VRC6_26 => {
                let mut out = [0usize; 8];
                for b in 0..8 {
                    out[b] = self.vrc6_slot(b, chr_rom);
                }
                out
            }
            MAPPER_H3001 => {
                let mut out = [0usize; 8];
                for b in 0..8 {
                    out[b] = (self.h3001.chr[b] & 0xFF) as usize;
                }
                out
            }
            MAPPER_VRC3 | _ => [0, 1, 2, 3, 4, 5, 6, 7],
        }
    }

    fn chr_expand_4k(&self, b0: u16, b1: u16) -> [usize; 8] {
        let b0 = b0 as usize * 4;
        let b1 = b1 as usize * 4;
        [
            b0,
            b0 + 1,
            b0 + 2,
            b0 + 3,
            b1,
            b1 + 1,
            b1 + 2,
            b1 + 3,
        ]
    }

    fn vrc6_4mbit(&self, chr_rom: &[u8]) -> bool {
        chr_rom.len() >= 0x80000
    }

    fn vrc6_slot(&self, slot: usize, chr_rom: &[u8]) -> usize {
        let c = self.vrc6.mode & 3;
        let mut val = if c == 0 {
            if slot < 8 {
                (self.vrc6.chr[slot] & 0xFF) as u16
            } else {
                (self.vrc6.chr[slot & 7] & 0xFF) as u16
            }
        } else {
            let reg = if slot < 4 { slot } else if slot < 6 { 4 } else { 5 };
            (self.vrc6.chr[reg] & 0xFF) as u16
        };
        if c != 0 && (self.vrc6.mode & 0x20) != 0 {
            val = if slot & 1 == 1 { val | 0x01 } else { val & 0xFE };
        }
        if self.vrc6_4mbit(chr_rom) {
            ((val << 1) | (slot as u16 & 1)) as usize
        } else {
            val as usize
        }
    }

    fn vrc6_nt_val(&self, screen: usize) -> usize {
        let c = self.vrc6.mode & 3;
        let m = (self.vrc6.mode >> 2) & 3;
        let reg = if c == 1 {
            4 | screen
        } else if ((c >> 1) ^ (m & 1)) != 0 {
            6 | (screen & 1)
        } else {
            6 | (screen >> 1)
        };
        let mut val = self.vrc6.chr[reg] as usize;
        if (self.vrc6.mode & 0x20) != 0 && (c == 0 || c == 3) {
            val &= !1;
            match m ^ (c & 1) {
                0 => val |= if screen & 1 != 0 { 1 } else { 0 },
                1 => val |= if screen & 2 != 0 { 1 } else { 0 },
                2 => {}
                3 => val |= 1,
                _ => {}
            }
        }
        val & 0xFF
    }

    fn chr_read(&self, chr_rom: &[u8], chr_ram: &[u8], using_chr_ram: bool, offset: usize) -> u8 {
        match self.chr_source() {
            ChrSource::Ram => {
                if !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else {
                    0
                }
            }
            ChrSource::Rom => {
                if !chr_rom.is_empty() {
                    chr_rom[offset % chr_rom.len()]
                } else {
                    0
                }
            }
            ChrSource::Auto => {
                if using_chr_ram && !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else if !chr_rom.is_empty() {
                    chr_rom[offset % chr_rom.len()]
                } else {
                    0
                }
            }
        }
    }

    fn nt_index(&self, alternative: bool, using_chr_ram: bool, address: u16) -> u16 {
        if alternative {
            return address;
        }
        if !self.locked() {
            return if self.mirror_v() {
                address & 0x37FF
            } else {
                (address & 0x33FF) | ((address & 0x800) >> 1)
            };
        }
        match self.mapper() {
            MAPPER_UNROM
            | MAPPER_BNROM
            | MAPPER_CNROM
            | MAPPER_GNROM
            | MAPPER_SUROM => {
                if self.mirror_v() {
                    address & 0x37FF
                } else {
                    (address & 0x33FF) | ((address & 0x800) >> 1)
                }
            }
            MAPPER_ANROM => {
                if self.latch_data & 0x10 != 0 {
                    address & 0x3FF | 0x400
                } else {
                    address & 0x3FF
                }
            }
            MAPPER_BANDAI152 => {
                if self.latch_data & 0x80 != 0 {
                    address & 0x3FF | 0x400
                } else {
                    address & 0x3FF
                }
            }
            MAPPER_SLROM | MAPPER_SNROM => match self.mmc1.reg[0] & 0x03 {
                0 => address & 0x3FF,
                1 => address & 0x3FF | 0x400,
                2 => address & 0x37FF,
                _ => (address & 0x33FF) | ((address & 0x800) >> 1),
            },
            MAPPER_PNROM => mirror_h_or_v((self.mmc2.mirroring & 1) != 0, address),
            MAPPER_MMC3 | MAPPER_HKROM | MAPPER_189 => {
                mirror_h_or_v((self.mmc3.mirroring & 1) != 0, address)
            }
            MAPPER_TLSROM => 0x3000 | (address & 0x3FF),
            MAPPER_VRC1 => {
                if self.vrc1.misc & 1 != 0 {
                    (address & 0x33FF) | ((address & 0x800) >> 1)
                } else {
                    address & 0x37FF
                }
            }
            MAPPER_VRC2_22 | MAPPER_VRC4_21 | MAPPER_VRC4_23 | MAPPER_VRC4_25 => {
                match self.vrc24.mirroring & if self.vrc24.is_vrc4 { 3 } else { 1 } {
                    0 => address & 0x37FF,
                    1 => (address & 0x33FF) | ((address & 0x800) >> 1),
                    2 => address & 0x3FF,
                    _ => address & 0x3FF | 0x400,
                }
            }
            MAPPER_VRC3 => {
                if self.mirror_v() {
                    address & 0x37FF
                } else {
                    (address & 0x33FF) | ((address & 0x800) >> 1)
                }
            }
            MAPPER_VRC6_24 | MAPPER_VRC6_26 => {
                if (self.vrc6.mode & 0x10) != 0 {
                    0x3000 | (address & 0x3FF)
                } else {
                    let screen = ((address >> 10) & 3) as usize;
                    let mask = if using_chr_ram { 3 } else { 1 };
                    ((self.vrc6_nt_val(screen) & mask) * 0x400 + (address as usize & 0x3FF)) as u16
                }
            }
            MAPPER_H3001 => match self.h3001.mirroring >> 6 {
                0 => address & 0x37FF,
                2 => (address & 0x33FF) | ((address & 0x800) >> 1),
                _ => address & 0x3FF,
            },
            _ => {
                if self.mirror_v() {
                    address & 0x37FF
                } else {
                    (address & 0x33FF) | ((address & 0x800) >> 1)
                }
            }
        }
    }

    fn read_8k_page(&self, p: &[u8], page: usize, address: u16) -> u8 {
        if p.is_empty() {
            return 0;
        }
        let offset = page * 0x2000 + (address as usize & 0x1FFF);
        p[offset % p.len()]
    }

    fn flash_read(&self, cart: &Cartridge, address: u16) -> u8 {
        let slots = self.boot_slots();
        let page = slots[((address as usize) >> 13) & 3];
        let byte = self.read_8k_page(&cart.prg_rom, page, address);
        if self.flash_state == 0x90 {
            if address & 1 != 0 {
                0x7E
            } else {
                0x01
            }
        } else if self.flash_time_out != 0 {
            (byte ^ if self.flash_time_out & 1 != 0 { 0x40 } else { 0x00 }) & !0x88
        } else {
            byte
        }
    }

    fn flash_write(&mut self, cart: &mut Cartridge, address: u16, val: u8) {
        let addr = (address & 0xFFF) as u16;
        match self.flash_state {
            0 => {
                if addr == 0xAAA && val == 0xAA {
                    self.flash_state += 1;
                }
            }
            0x01 => {
                if addr == 0x555 && val == 0x55 {
                    self.flash_state += 1;
                }
            }
            0x02 => {
                if addr == 0xAAA {
                    self.flash_state = val;
                }
            }
            0x80 => {
                if addr == 0xAAA && val == 0xAA {
                    self.flash_state += 1;
                }
            }
            0x81 => {
                if addr == 0x555 && val == 0x55 {
                    self.flash_state += 1;
                }
            }
            0x82 => {
                if val == 0x30 {
                    let slots = self.boot_slots();
                    let page = slots[((address as usize) >> 13) & 3];
                    let offset = page * 0x2000 + (address as usize & 0x1FFF);
                    if offset < self.flash_chip_size {
                        let start = offset & !(131072 - 1);
                        let end = (start + 131072).min(cart.prg_rom.len());
                        for b in cart.prg_rom[start..end].iter_mut() {
                            *b = 0xFF;
                        }
                        self.flash_time_out = 131072;
                    }
                } else if val == 0x10 && addr == 0xAAA {
                    for b in cart.prg_rom.iter_mut() {
                        *b = 0xFF;
                    }
                    self.flash_time_out = self.flash_chip_size as u32;
                } else if val == 0xF0 {
                    self.flash_state = 0;
                }
            }
            0x90 => {
                if val == 0xF0 {
                    self.flash_state = 0;
                }
            }
            0xA0 => {
                if !cart.prg_rom.is_empty() {
                    let slots = self.boot_slots();
                    let page = slots[((address as usize) >> 13) & 3];
                    let offset = page * 0x2000 + (address as usize & 0x1FFF);
                    if offset < cart.prg_rom.len() {
                        cart.prg_rom[offset] = val;
                    }
                }
                self.flash_state = 0;
            }
            _ => {}
        }
    }

    fn store_wram(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if cart.prg_ram.is_empty() {
            return;
        }
        let len = cart.prg_ram.len();
        cart.prg_ram[(address as usize & 0x1FFF) % len] = data;
    }

    fn wram_read(&self, cart: &Cartridge, address: u16) -> FetchResult {
        if self.locked() {
            match self.mapper() {
                MAPPER_MMC3 | MAPPER_TLSROM | MAPPER_189 => {
                    if self.mmc3.wram_control & 0x80 != 0 {
                        if !cart.prg_ram.is_empty() {
                            let len = cart.prg_ram.len();
                            return FetchResult {
                                data: cart.prg_ram[(address as usize & 0x1FFF) % len],
                                driven: true,
                            };
                        }
                        return FetchResult { data: 0, driven: true };
                    }
                    return FetchResult { data: 0, driven: false };
                }
                MAPPER_HKROM => {
                    let a = address & 0x3FF;
                    let enable = if (a & 0x200) == 0 {
                        self.mmc3.wram_control & 0x20 != 0
                    } else {
                        self.mmc3.wram_control & 0x80 != 0
                    };
                    if enable {
                        if !cart.prg_ram.is_empty() {
                            let len = cart.prg_ram.len();
                            return FetchResult {
                                data: cart.prg_ram[(0x1000 | (address as usize & 0xFFF)) % len],
                                driven: true,
                            };
                        }
                        return FetchResult { data: 0, driven: true };
                    }
                    let low = if (a & 0x200) == 0 {
                        self.mmc3.wram_control & 0x80 != 0
                    } else {
                        self.mmc3.wram_control & 0x20 != 0
                    };
                    return FetchResult {
                        data: if low { 0 } else { 0 },
                        driven: low,
                    };
                }
                MAPPER_VRC2_22 => {
                    if address < 0x7000 {
                        return FetchResult { data: self.vrc24.pins, driven: true };
                    }
                }
                MAPPER_VRC4_21 | MAPPER_VRC4_23 | MAPPER_VRC4_25 => {
                    if address < 0x7000 {
                        if self.vrc24.misc & 1 != 0 && !cart.prg_ram.is_empty() {
                            let len = cart.prg_ram.len();
                            return FetchResult {
                                data: cart.prg_ram[(address as usize & 0xFFF) % len],
                                driven: true,
                            };
                        }
                        return FetchResult { data: 0, driven: false };
                    }
                }
                _ => {}
            }
        }
        if !cart.prg_ram.is_empty() {
            let len = cart.prg_ram.len();
            return FetchResult {
                data: cart.prg_ram[(address as usize & 0x1FFF) % len],
                driven: true,
            };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_wram_gated(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if self.locked() {
            match self.mapper() {
                MAPPER_MMC3 | MAPPER_TLSROM | MAPPER_189 => {
                    if self.mmc3.wram_control & 0x80 != 0 && self.mmc3.wram_control & 0x40 == 0 {
                        self.store_wram(cart, address, data);
                    }
                    return;
                }
                MAPPER_HKROM => {
                    let a = address & 0x3FF;
                    let enable = if (a & 0x200) == 0 {
                        self.mmc3.wram_control & 0x20 != 0 && self.mmc3.wram_control & 0x10 != 0
                    } else {
                        self.mmc3.wram_control & 0x80 != 0 && self.mmc3.wram_control & 0x40 != 0
                    };
                    if enable && !cart.prg_ram.is_empty() {
                        let len = cart.prg_ram.len();
                        cart.prg_ram[(0x1000 | (address as usize & 0xFFF)) % len] = data;
                    }
                    return;
                }
                MAPPER_VRC2_22 => {
                    if address < 0x7000 {
                        self.vrc24.pins = data;
                        return;
                    }
                }
                MAPPER_VRC4_21 | MAPPER_VRC4_23 | MAPPER_VRC4_25 => {
                    if address < 0x7000 {
                        if self.vrc24.misc & 1 != 0 {
                            self.store_wram(cart, address, data);
                        }
                        return;
                    }
                }
                _ => {}
            }
        }
        self.store_wram(cart, address, data);
    }

    fn write_reg(&mut self, address: u16, mut val: u8) {
        let addr = (address as usize) & 7;
        let sub = self.submapper_id;
        if addr == 3 && sub != 2 {
            val = !val;
        }
        if addr == 0 {
            let m = val & 0x1F;
            let replace = |val: u8, mode: u8| (val & !0x1F) | mode;
            match sub {
                0 => match m {
                    0 => val = replace(val, MAPPER_UNROM),
                    1 => val = replace(val, MAPPER_SNROM),
                    4 => val = replace(val, MAPPER_MMC3),
                    6 => val = replace(val, MAPPER_VRC4_21),
                    7 => val = replace(val, MAPPER_VRC2_22),
                    8 => val = replace(val, MAPPER_VRC4_23),
                    9 => val = replace(val, MAPPER_VRC6_24),
                    10 => val = replace(val, MAPPER_VRC4_25),
                    11 => val = replace(val, MAPPER_VRC6_26),
                    12 => val = replace(val, MAPPER_VRC3),
                    _ => {}
                },
                2 => {
                    if m == 13 || m == 9 {
                        val = replace(val, MAPPER_MMC3);
                    }
                }
                3 => {
                    if m == 1 {
                        val = replace(val, MAPPER_H3001);
                    }
                }
                4 => match m {
                    0 => val = replace(val, MAPPER_UNROM),
                    1 => val = replace(val, MAPPER_MMC3),
                    3 => val = replace(val, MAPPER_VRC4_23),
                    6 => val = replace(val, MAPPER_VRC3),
                    _ => {}
                },
                _ => {}
            }
        }
        self.reg[addr] = val;
        if self.locked() {
            self.apply_mode();
            self.sync_chip();
        }
    }

    fn apply_mode(&mut self) {
        if !self.locked() {
            return;
        }
        match self.mapper() {
            MAPPER_UNROM | MAPPER_BNROM | MAPPER_CNROM | MAPPER_ANROM | MAPPER_GNROM
            | MAPPER_BANDAI152 => {
                self.latch_data = 0;
                self.latch_addr = 0;
            }
            MAPPER_SLROM | MAPPER_SNROM => {
                self.mmc1 = Mmc1 { reg: [0x0C, 0, 0, 0], shift: 0, bits: 0, filter: 0 };
            }
            MAPPER_PNROM => {
                self.mmc2 = Mmc2 { prg: 0, chr: [0; 4], state: [0; 2], mirroring: 0 };
            }
            MAPPER_MMC3 | MAPPER_HKROM | MAPPER_TLSROM | MAPPER_189 => {
                self.latch_189 = 0;
                self.mmc3 = Mmc3 {
                    index: 0,
                    reg: [0, 2, 4, 5, 6, 7, 0, 1],
                    mirroring: 0,
                    wram_control: 0,
                    enable_irq: false,
                    reload: false,
                    counter: 0,
                    reload_value: 0,
                    pa12_filter: 0,
                    is_mmc6: self.mapper() == MAPPER_HKROM,
                };
                self.irq_ack_pending = true;
            }
            MAPPER_VRC1 => {
                self.vrc1 = Vrc1 { prg: [0; 3], chr: [0; 2], misc: 0 };
            }
            MAPPER_VRC2_22 => {
                self.vrc24 = Vrc24 {
                    prg: [0, 1],
                    chr: [0, 1, 2, 3, 4, 5, 6, 7],
                    mirroring: 0,
                    pins: 0,
                    latch: 0,
                    mode: 0,
                    counter: 0,
                    cycles: 0,
                    misc: 0,
                    is_vrc4: false,
                    a0: 0x02,
                    a1: 0x01,
                };
                self.irq_ack_pending = true;
            }
            MAPPER_VRC4_21 => {
                self.vrc24 = self.vrc24_reset(true, 0x42, 0x84);
                self.irq_ack_pending = true;
            }
            MAPPER_VRC4_23 => {
                self.vrc24 = self.vrc24_reset(true, 0x05, 0x0A);
                self.irq_ack_pending = true;
            }
            MAPPER_VRC4_25 => {
                self.vrc24 = self.vrc24_reset(true, 0x0A, 0x05);
                self.irq_ack_pending = true;
            }
            MAPPER_VRC3 => {
                self.vrc3 = Vrc3 { prg: 0, irq: 0, counter: 0, latch: 0 };
            }
            MAPPER_VRC6_24 => {
                self.vrc6 = self.vrc6_reset(0x01, 0x02);
                self.irq_ack_pending = true;
            }
            MAPPER_VRC6_26 => {
                self.vrc6 = self.vrc6_reset(0x02, 0x01);
                self.irq_ack_pending = true;
            }
            MAPPER_H3001 => {
                self.h3001 = H3001 {
                    prg_invert: 0,
                    prg: [0, 1],
                    chr: [0, 1, 2, 3, 4, 5, 6, 7],
                    mirroring: 0,
                    irq: 0,
                    counter: 0,
                    latch: 0,
                };
            }
            _ => {}
        }
    }

    fn vrc24_reset(&self, is_vrc4: bool, a0: u8, a1: u8) -> Vrc24 {
        Vrc24 {
            prg: [0, 1],
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            mirroring: 0,
            pins: 0,
            latch: 0,
            mode: 0,
            counter: 0,
            cycles: 0,
            misc: 0,
            is_vrc4,
            a0,
            a1,
        }
    }

    fn vrc6_reset(&self, a0: u8, a1: u8) -> Vrc6 {
        Vrc6 {
            irq_control: 0,
            irq_counter: 0,
            irq_latch: 0,
            irq_cycles: 0,
            mode: 0,
            prg: [0, 0xFE],
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            a0,
            a1,
        }
    }

    fn sync_chip(&mut self) {
        if self.locked() {
            match self.mapper() {
                MAPPER_VRC2_22 | MAPPER_VRC4_21 | MAPPER_VRC4_23 | MAPPER_VRC4_25 => {
                    self.vrc24.misc |= 1;
                }
                _ => {}
            }
        }
    }

    fn mmc3_write(&mut self, address: u16, val: u8) {
        match (address >> 12) & 0xE {
            0x8 => {
                if address & 1 != 0 {
                    self.mmc3.reg[self.mmc3.index as usize & 7] = val;
                } else {
                    self.mmc3.index = val;
                }
            }
            0xA => {
                if address & 1 != 0 {
                    self.mmc3.wram_control = val;
                } else {
                    self.mmc3.mirroring = val;
                }
            }
            0xC => {
                if address & 1 != 0 {
                    self.mmc3.counter = 0;
                    self.mmc3.reload = true;
                } else {
                    self.mmc3.reload_value = val;
                }
            }
            0xE => {
                self.mmc3.enable_irq = (address & 1) != 0;
                if !self.mmc3.enable_irq {
                    self.irq_ack_pending = true;
                }
            }
            _ => {}
        }
    }

    fn mmc3_prg_fire(&mut self) -> bool {
        let prev = self.mmc3.counter;
        self.mmc3.counter = if self.mmc3.counter == 0 {
            self.mmc3.reload_value
        } else {
            self.mmc3.counter.wrapping_sub(1)
        };
        let fire = ((prev != 0) || self.mmc3.reload || !self.mmc3.is_mmc6)
            && self.mmc3.counter == 0
            && self.mmc3.enable_irq;
        self.mmc3.reload = false;
        fire
    }

    fn vrc24_write(&mut self, address: u16, val: u8) {
        let a0 = self.vrc24.a0 as u16;
        let a1 = self.vrc24.a1 as u16;
        let index = if address & a0 != 0 { 1 } else { 0 } | if address & a1 != 0 { 2 } else { 0 };
        let bank = (address >> 12) & 0xF;
        match bank {
            0x8 | 0xA => {
                self.vrc24.prg[((bank as usize) >> 1) & 1] = val;
            }
            0x9 => {
                if !self.vrc24.is_vrc4 || index == 0 {
                    self.vrc24.mirroring = val;
                } else if self.vrc24.is_vrc4 && index == 2 {
                    self.vrc24.misc = val;
                }
            }
            0xF => {
                if self.vrc24.is_vrc4 {
                    match index & 3 {
                        0 => self.vrc24.latch = self.vrc24.latch & 0xF0 | val & 0x0F,
                        1 => self.vrc24.latch = self.vrc24.latch & 0x0F | (val << 4),
                        2 => {
                            self.vrc24.mode = val;
                            if self.vrc24.mode & 0x02 != 0 {
                                self.vrc24.counter = self.vrc24.latch;
                                self.vrc24.cycles = 341;
                            }
                            self.irq_ack_pending = true;
                        }
                        3 => {
                            self.vrc24.mode = self.vrc24.mode & !0x02 | (self.vrc24.mode << 1) & 0x02;
                            self.irq_ack_pending = true;
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                let reg = (((bank as u16) - 0xB) << 1) | if address & a1 != 0 { 1 } else { 0 };
                let reg = reg as usize;
                if address & a0 != 0 {
                    self.vrc24.chr[reg] = self.vrc24.chr[reg] & 0x00F | ((val as u16) << 4);
                } else {
                    self.vrc24.chr[reg] = self.vrc24.chr[reg] & 0xFF0 | ((val & 0x0F) as u16);
                }
            }
        }
    }

    fn vrc6_write(&mut self, address: u16, val: u8) {
        let a0 = self.vrc6.a0 as u16;
        let a1 = self.vrc6.a1 as u16;
        let index = if address & a1 != 0 { 2 } else { 0 } | if address & a0 != 0 { 1 } else { 0 };
        let bank = (address >> 12) & 0xF;
        match bank {
            0x8 => {
                self.vrc6.prg[0] = val;
            }
            0xC => {
                self.vrc6.prg[1] = val;
            }
            0xB => {
                if index == 3 {
                    self.vrc6.mode = val;
                }
            }
            0x9 | 0xA => {}
            0xD | 0xE => {
                let reg = ((bank - 0xD) << 2) | if address & a1 != 0 { 2 } else { 0 } | if address & a0 != 0 { 1 } else { 0 };
                self.vrc6.chr[reg as usize] = val;
            }
            0xF => {
                match index {
                    0 => self.vrc6.irq_latch = val,
                    1 => {
                        self.vrc6.irq_control = val;
                        if self.vrc6.irq_control & 0x02 != 0 {
                            self.vrc6.irq_counter = self.vrc6.irq_latch;
                            self.vrc6.irq_cycles = 341;
                        }
                        self.irq_ack_pending = true;
                    }
                    2 => {
                        if self.vrc6.irq_control & 0x01 != 0 {
                            self.vrc6.irq_control |= 0x02;
                        } else {
                            self.vrc6.irq_control &= !0x02;
                        }
                        self.irq_ack_pending = true;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn h3001_write(&mut self, address: u16, val: u8) {
        let bank = (address >> 12) & 0xF;
        match bank {
            0x8 => {
                self.h3001.prg[0] = val;
            }
            0xA => {
                self.h3001.prg[1] = val;
            }
            0x9 => match address & 7 {
                0 => self.h3001.prg_invert = val,
                1 => self.h3001.mirroring = val,
                3 => {
                    self.h3001.irq = val;
                    self.irq_ack_pending = true;
                }
                4 => {
                    self.h3001.counter = self.h3001.latch;
                    self.irq_ack_pending = true;
                }
                5 => self.h3001.latch = self.h3001.latch & 0x00FF | ((val as u16) << 8),
                6 => self.h3001.latch = self.h3001.latch & 0xFF00 | val as u16,
                _ => {}
            },
            0xB => {
                self.h3001.chr[(address & 7) as usize] = val;
            }
            _ => {}
        }
    }

    fn vrc3_cpu_tick(&mut self) -> bool {
        if self.vrc3.irq & 2 != 0 {
            let mask = if self.vrc3.irq & 4 != 0 { 0xFF } else { 0xFFFF };
            if (self.vrc3.counter & mask) == mask {
                self.vrc3.counter = self.vrc3.latch;
                return true;
            } else {
                self.vrc3.counter = self.vrc3.counter.wrapping_add(1);
            }
        }
        false
    }

    fn vrc24_cpu_tick(&mut self) -> bool {
        if self.vrc24.mode & 2 != 0 && (self.vrc24.mode & 4 != 0 || {
            self.vrc24.cycles = self.vrc24.cycles.wrapping_sub(3);
            self.vrc24.cycles <= 0
        }) {
            if self.vrc24.mode & 4 == 0 {
                self.vrc24.cycles += 341;
            }
            self.vrc24.counter = self.vrc24.counter.wrapping_add(1);
            if self.vrc24.counter == 0 {
                self.vrc24.counter = self.vrc24.latch;
                return true;
            }
        }
        false
    }

    fn vrc6_cpu_tick(&mut self) -> bool {
        if self.vrc6.irq_control & 0x02 != 0
            && (self.vrc6.irq_control & 0x04 != 0 || {
                self.vrc6.irq_cycles = self.vrc6.irq_cycles.wrapping_sub(3);
                self.vrc6.irq_cycles <= 0
            })
        {
            if self.vrc6.irq_control & 0x04 == 0 {
                self.vrc6.irq_cycles += 341;
            }
            if self.vrc6.irq_counter == 0xFF {
                self.vrc6.irq_counter = self.vrc6.irq_latch;
                return true;
            } else {
                self.vrc6.irq_counter = self.vrc6.irq_counter.wrapping_add(1);
            }
        }
        false
    }
}

impl Mapper for Mapper446 {
    fn reset(&mut self) {
        self.reg = [0; 8];
        self.apply_mode();
        self.sync_chip();
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x6000..0x8000).contains(&address) {
            return self.wram_read(cart, address);
        }
        if address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }
        if !self.locked() {
            return FetchResult {
                data: self.flash_read(cart, address),
                driven: true,
            };
        }
        let slots = self.prg_slots();
        let page = slots[((address as usize) >> 13) & 3];
        FetchResult {
            data: self.read_8k_page(&cart.prg_rom, page, address),
            driven: true,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        let bank = (address >> 12) as usize;
        if !self.locked() {
            match bank {
                5 => self.write_reg(address, data),
                8..=0xF => self.flash_write(cart, address, data),
                _ => self.store_wram_gated(cart, address, data),
            }
            return;
        }
        match self.mapper() {
            MAPPER_UNROM | MAPPER_BNROM | MAPPER_CNROM | MAPPER_ANROM | MAPPER_GNROM
            | MAPPER_BANDAI152 => {
                if address >= 0x8000 {
                    self.latch_data = data;
                } else if (0x6000..0x8000).contains(&address) {
                    self.store_wram_gated(cart, address, data);
                }
            }
            MAPPER_SLROM | MAPPER_SNROM => {
                if address < 0x8000 {
                    self.store_wram_gated(cart, address, data);
                    return;
                }
                if data & 0x80 != 0 {
                    self.mmc1.reg[0] |= 0x0C;
                    self.mmc1.shift = 0;
                    self.mmc1.bits = 0;
                } else if self.mmc1.filter == 0 {
                    self.mmc1.shift |= (data & 1) << self.mmc1.bits;
                    self.mmc1.bits += 1;
                    if self.mmc1.bits == 5 {
                        let idx = ((bank >> 1) & 3) as usize;
                        self.mmc1.reg[idx] = self.mmc1.shift;
                        self.mmc1.shift = 0;
                        self.mmc1.bits = 0;
                    }
                }
                self.mmc1.filter = 2;
            }
            MAPPER_PNROM => match bank {
                0xA => self.mmc2.prg = data,
                0xB..=0xE => self.mmc2.chr[bank - 0xB] = data,
                0xF => self.mmc2.mirroring = data,
                _ => {}
            },
            MAPPER_189 => {
                if bank == 6 {
                    self.latch_189 = (address & 0xFF) as u8;
                } else if address >= 0x8000 {
                    self.mmc3_write(address, data);
                } else if (0x6000..0x8000).contains(&address) {
                    self.store_wram_gated(cart, address, data);
                }
            }
            MAPPER_MMC3 | MAPPER_HKROM | MAPPER_TLSROM => {
                if address >= 0x8000 {
                    self.mmc3_write(address, data);
                } else if (0x6000..0x8000).contains(&address) {
                    self.store_wram_gated(cart, address, data);
                }
            }
            MAPPER_VRC1 => match bank {
                0x8 | 0xA | 0xB | 0xC | 0xD => {
                    self.vrc1.prg[((bank >> 1) & 3) as usize] = data;
                }
                0x9 => self.vrc1.misc = data,
                0xE | 0xF => self.vrc1.chr[bank & 1] = data,
                _ => {}
            },
            MAPPER_VRC2_22 | MAPPER_VRC4_21 | MAPPER_VRC4_23 | MAPPER_VRC4_25 => {
                if address >= 0x8000 {
                    self.vrc24_write(address, data);
                } else if (0x6000..0x8000).contains(&address) {
                    let mode_is_vrc2 = self.mapper() == MAPPER_VRC2_22;
                    if address < 0x7000 {
                        if mode_is_vrc2 {
                            self.vrc24.pins = data;
                        } else if self.vrc24.misc & 1 != 0 {
                            self.store_wram(cart, address, data);
                        }
                    } else {
                        self.store_wram(cart, address, data);
                    }
                }
            }
            MAPPER_VRC3 => match bank {
                0x8..=0xB => {
                    let val = data & 0x0F;
                    let shift = (bank << 2) & 0xC;
                    self.vrc3.latch = self.vrc3.latch & !(0xF << shift) | (val as u16) << shift;
                }
                0xC => {
                    self.vrc3.irq = data;
                    if self.vrc3.irq & 2 != 0 {
                        self.vrc3.counter = self.vrc3.latch;
                    }
                    self.irq_ack_pending = true;
                }
                0xD => {
                    self.vrc3.irq = self.vrc3.irq & !0x02 | (self.vrc3.irq << 1) & 0x01;
                    self.irq_ack_pending = true;
                }
                0xF => self.vrc3.prg = data,
                _ => {
                    if (0x6000..0x8000).contains(&address) {
                        self.store_wram(cart, address, data);
                    }
                }
            },
            MAPPER_VRC6_24 | MAPPER_VRC6_26 => {
                if address >= 0x8000 {
                    self.vrc6_write(address, data);
                } else if (0x6000..0x8000).contains(&address) {
                    self.store_wram(cart, address, data);
                }
            }
            MAPPER_H3001 => match bank {
                0x8 | 0x9 | 0xA | 0xB => self.h3001_write(address, data),
                _ => {
                    if (0x6000..0x8000).contains(&address) {
                        self.store_wram(cart, address, data);
                    }
                }
            },
            _ => {
                if (0x6000..0x8000).contains(&address) {
                    self.store_wram_gated(cart, address, data);
                }
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.nt_index(cart.alternative_nametable_arrangement, cart.using_chr_ram, address)
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
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = ((address >> 10) & 7) as usize;
            let slots = self.chr_1k_slots(chr_rom);
            let offset = slots[bank] * 0x400 + (address as usize & 0x3FF);
            let byte = self.chr_read(chr_rom, chr_ram, using_chr_ram, offset);
            new_addr_bus |= byte as u16;
            if self.locked() && self.mapper() == MAPPER_PNROM {
                let cmp = address
                    & if address & 0x1000 != 0 {
                        0x3F8
                    } else {
                        0x3FF
                    };
                let idx = ((address >> 12) & 1) as usize;
                if cmp == 0x3D8 {
                    self.mmc2.state[idx] = 0;
                } else if cmp == 0x3E8 {
                    self.mmc2.state[idx] = 1;
                }
            }
        } else if self.locked() && self.mapper() == MAPPER_TLSROM {
            let screen = ((address >> 10) & 3) as usize;
            let page = self.mmc3.chr_bank(screen) >> 7;
            let offset = (page as usize) * 0x400 + (address as usize & 0x3FF);
            let byte = if !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else if self.locked()
            && matches!(self.mapper(), MAPPER_VRC6_24 | MAPPER_VRC6_26)
            && (self.vrc6.mode & 0x10) != 0
        {
            let screen = ((address >> 10) & 3) as usize;
            let slot = screen + 8;
            let page = self.vrc6_slot(slot, chr_rom);
            let offset = page * 0x400 + (address as usize & 0x3FF);
            let byte = if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else if address >= 0x2000 {
            let mirrored = self.nt_index(alternative_nametable_arrangement, using_chr_ram, address);
            let byte = if (mirrored & 0x800) != 0 {
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
            if !self.protect_chr() && !cart.chr_ram.is_empty() {
                let bank = ((address >> 10) & 7) as usize;
                let slots = self.chr_1k_slots(&cart.chr_rom);
                let offset = slots[bank] * 0x400 + (address as usize & 0x3FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if self.locked() && self.mapper() == MAPPER_TLSROM && address < 0x3F00 {
            let screen = ((address >> 10) & 3) as usize;
            let page = self.mmc3.chr_bank(screen) >> 7;
            let offset = (page as usize) * 0x400 + (address as usize & 0x3FF);
            if !self.protect_chr() && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.nt_index(cart.alternative_nametable_arrangement, cart.using_chr_ram, address);
            if (mirrored & 0x800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
            }
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if !self.locked() {
            return false;
        }
        let mapper = self.mapper();
        match mapper {
            MAPPER_SLROM | MAPPER_SNROM => {
                self.mmc1.filter = self.mmc1.filter.saturating_sub(cycles);
            }
            MAPPER_MMC3 | MAPPER_HKROM | MAPPER_TLSROM | MAPPER_189 => {
                for _ in 0..cycles {
                    if self.mmc3.pa12_filter != 0 {
                        self.mmc3.pa12_filter -= 1;
                    }
                }
            }
            MAPPER_VRC3 => {
                for _ in 0..cycles {
                    if self.vrc3_cpu_tick() {
                        return true;
                    }
                    if self.vrc24_cpu_tick() {
                        return true;
                    }
                }
            }
            MAPPER_VRC2_22 | MAPPER_VRC4_21 | MAPPER_VRC4_23 | MAPPER_VRC4_25 => {
                for _ in 0..cycles {
                    if self.vrc24_cpu_tick() {
                        return true;
                    }
                }
            }
            MAPPER_VRC6_24 | MAPPER_VRC6_26 => {
                for _ in 0..cycles {
                    if self.vrc6_cpu_tick() {
                        return true;
                    }
                }
            }
            MAPPER_H3001 => {
                for _ in 0..cycles {
                    if self.h3001.irq & 0x80 != 0 {
                        self.h3001.counter = self.h3001.counter.wrapping_sub(1);
                        if self.h3001.counter == 0 {
                            self.h3001.irq = 0;
                            return true;
                        }
                    }
                }
            }
            _ => {}
        }
        false
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        _scanline: u16,
        _dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        if !self.locked() {
            return false;
        }
        match self.mapper() {
            MAPPER_MMC3 | MAPPER_HKROM | MAPPER_TLSROM | MAPPER_189 => {
                if ppu_address_bus & 0x1000 != 0 {
                    let fire = if self.mmc3.pa12_filter == 0 {
                        self.mmc3_prg_fire()
                    } else {
                        false
                    };
                    self.mmc3.pa12_filter = 3;
                    fire
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack_pending;
        self.irq_ack_pending = false;
        ack
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.extend_from_slice(&self.reg);
        state.extend_from_slice(&self.latch_addr.to_le_bytes());
        state.push(self.latch_data);
        state.push(self.latch_189);
        let c = &self.mmc1;
        state.extend_from_slice(&c.reg);
        state.push(c.shift);
        state.push(c.bits);
        state.push(c.filter);
        let c = &self.mmc2;
        state.push(c.prg);
        state.extend_from_slice(&c.chr);
        state.extend_from_slice(&c.state);
        state.push(c.mirroring);
        let c = &self.mmc3;
        state.push(c.index);
        state.extend_from_slice(&c.reg);
        state.push(c.mirroring);
        state.push(c.wram_control);
        state.push(c.counter);
        state.push(c.reload_value);
        state.push(c.reload as u8);
        state.push(c.enable_irq as u8);
        state.push(c.pa12_filter);
        let c = &self.vrc1;
        state.extend_from_slice(&c.prg);
        state.extend_from_slice(&c.chr);
        state.push(c.misc);
        let c = &self.vrc24;
        state.extend_from_slice(&c.prg);
        for x in c.chr {
            state.extend_from_slice(&x.to_le_bytes());
        }
        state.push(c.mirroring);
        state.push(c.mode);
        state.push(c.counter);
        state.push(c.latch);
        state.extend_from_slice(&c.cycles.to_le_bytes());
        state.push(c.misc);
        state.push(c.pins);
        state.push(c.is_vrc4 as u8);
        let c = &self.vrc3;
        state.push(c.prg);
        state.push(c.irq);
        state.extend_from_slice(&c.counter.to_le_bytes());
        state.extend_from_slice(&c.latch.to_le_bytes());
        let c = &self.vrc6;
        state.push(c.irq_control);
        state.push(c.irq_counter);
        state.push(c.irq_latch);
        state.extend_from_slice(&c.irq_cycles.to_le_bytes());
        state.push(c.mode);
        state.extend_from_slice(&c.prg);
        state.extend_from_slice(&c.chr);
        let c = &self.h3001;
        state.push(c.prg_invert);
        state.extend_from_slice(&c.prg);
        state.extend_from_slice(&c.chr);
        state.push(c.mirroring);
        state.push(c.irq);
        state.extend_from_slice(&c.counter.to_le_bytes());
        state.extend_from_slice(&c.latch.to_le_bytes());
        state.push(self.flash_state);
        state.extend_from_slice(&self.flash_time_out.to_le_bytes());
        state.push(self.irq_ack_pending as u8);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..cart.prg_ram.len() {
            if p >= state.len() {
                return p;
            }
            cart.prg_ram[i] = state[p];
            p += 1;
        }
        for i in 0..cart.chr_ram.len() {
            if p >= state.len() {
                return p;
            }
            cart.chr_ram[i] = state[p];
            p += 1;
        }
        if p + 8 <= state.len() {
            self.reg.copy_from_slice(&state[p..p + 8]);
            p += 8;
        }
        if p + 2 <= state.len() {
            self.latch_addr = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.latch_data = state[p];
            p += 1;
        }
        if p < state.len() {
            self.latch_189 = state[p];
            p += 1;
        }
        if p + 4 <= state.len() {
            self.mmc1.reg.copy_from_slice(&state[p..p + 4]);
            p += 4;
        }
        if p < state.len() {
            self.mmc1.shift = state[p];
            p += 1;
        }
        if p < state.len() {
            self.mmc1.bits = state[p];
            p += 1;
        }
        if p < state.len() {
            self.mmc1.filter = state[p];
            p += 1;
        }
        if p < state.len() {
            self.mmc2.prg = state[p];
            p += 1;
        }
        if p + 4 <= state.len() {
            self.mmc2.chr.copy_from_slice(&state[p..p + 4]);
            p += 4;
        }
        if p + 2 <= state.len() {
            self.mmc2.state.copy_from_slice(&state[p..p + 2]);
            p += 2;
        }
        if p < state.len() {
            self.mmc2.mirroring = state[p];
            p += 1;
        }
        if p < state.len() {
            self.mmc3.index = state[p];
            p += 1;
        }
        if p + 8 <= state.len() {
            self.mmc3.reg.copy_from_slice(&state[p..p + 8]);
            p += 8;
        }
        if p < state.len() {
            self.mmc3.mirroring = state[p];
            p += 1;
        }
        if p < state.len() {
            self.mmc3.wram_control = state[p];
            p += 1;
        }
        if p < state.len() {
            self.mmc3.counter = state[p];
            p += 1;
        }
        if p < state.len() {
            self.mmc3.reload_value = state[p];
            p += 1;
        }
        if p < state.len() {
            self.mmc3.reload = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.mmc3.enable_irq = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.mmc3.pa12_filter = state[p];
            p += 1;
        }
        if p + 3 <= state.len() {
            self.vrc1.prg.copy_from_slice(&state[p..p + 3]);
            p += 3;
        }
        if p + 2 <= state.len() {
            self.vrc1.chr.copy_from_slice(&state[p..p + 2]);
            p += 2;
        }
        if p < state.len() {
            self.vrc1.misc = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.vrc24.prg.copy_from_slice(&state[p..p + 2]);
            p += 2;
        }
        for i in 0..8 {
            if p + 2 <= state.len() {
                self.vrc24.chr[i] = u16::from_le_bytes([state[p], state[p + 1]]);
                p += 2;
            }
        }
        if p < state.len() {
            self.vrc24.mirroring = state[p];
            p += 1;
        }
        if p < state.len() {
            self.vrc24.mode = state[p];
            p += 1;
        }
        if p < state.len() {
            self.vrc24.counter = state[p];
            p += 1;
        }
        if p < state.len() {
            self.vrc24.latch = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.vrc24.cycles = i16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.vrc24.misc = state[p];
            p += 1;
        }
        if p < state.len() {
            self.vrc24.pins = state[p];
            p += 1;
        }
        if p < state.len() {
            self.vrc24.is_vrc4 = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.vrc3.prg = state[p];
            p += 1;
        }
        if p < state.len() {
            self.vrc3.irq = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.vrc3.counter = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p + 2 <= state.len() {
            self.vrc3.latch = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.vrc6.irq_control = state[p];
            p += 1;
        }
        if p < state.len() {
            self.vrc6.irq_counter = state[p];
            p += 1;
        }
        if p < state.len() {
            self.vrc6.irq_latch = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.vrc6.irq_cycles = i16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.vrc6.mode = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.vrc6.prg.copy_from_slice(&state[p..p + 2]);
            p += 2;
        }
        if p + 8 <= state.len() {
            self.vrc6.chr.copy_from_slice(&state[p..p + 8]);
            p += 8;
        }
        if p < state.len() {
            self.h3001.prg_invert = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.h3001.prg.copy_from_slice(&state[p..p + 2]);
            p += 2;
        }
        if p + 8 <= state.len() {
            self.h3001.chr.copy_from_slice(&state[p..p + 8]);
            p += 8;
        }
        if p < state.len() {
            self.h3001.mirroring = state[p];
            p += 1;
        }
        if p < state.len() {
            self.h3001.irq = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.h3001.counter = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p + 2 <= state.len() {
            self.h3001.latch = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.flash_state = state[p];
            p += 1;
        }
        if p + 4 <= state.len() {
            self.flash_time_out = u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
            p += 4;
        }
        if p < state.len() {
            self.irq_ack_pending = state[p] != 0;
            p += 1;
        }
        self.sync_chip();
        p
    }
}