use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, mmc3_chr_bank};
const VS_FRAME_CYCLES: u64 = 29780;

fn prg_rom_read(cart: &Cartridge, offset: usize) -> u8 {
    let len = cart.prg_rom.len();
    if len == 0 { 0 } else { cart.prg_rom[offset % len] }
}

pub struct Mapper124 {
    rega: u8,
    regb: u8,
    auden: u8,
    unrombank: u8,
    amrombank: u8,
    mmc1_shift: u8,
    mmc1_shift_count: u8,
    mmc1_control: u8,
    mmc1_chr0: u8,
    mmc1_chr1: u8,
    mmc1_prg_bank: u8,
    mmc3: MapperMMC3,
    vsdip: u8,
    coinon: u8,
    cycle_accum: u64,
}

fn mirror_addr(regb: u8, amrombank: u8, mmc1_control: u8, mmc3_horiz: bool, address: u16) -> u16 {
        match (regb >> 4) & 3 {
        0 => address & 0x37FF,
        1 => {
            if amrombank & 0x08 != 0 { address & 0x3BFF } else { address & 0x37FF }
        }
        2 => match mmc1_control & 3 {
            0 => (address & 0x33FF) | ((address & 0x0400) >> 1),
            1 => (address & 0x33FF) | ((address & 0x0800) >> 2),
            2 => address & 0x37FF,
            _ => (address & 0x33FF) | ((address & 0x0800) >> 1),
        },
        _ => {
            if mmc3_horiz {
                address & 0x37FF
            } else {
                address & 0x3BFF
            }
        }
    }
}

impl Mapper124 {
    pub fn new(dip_switches: u8) -> Self {
        Self {
            rega: 0,
            regb: 0,
            auden: 0,
            unrombank: 0,
            amrombank: 0,
            mmc1_shift: 0,
            mmc1_shift_count: 0,
            mmc1_control: 0,
            mmc1_chr0: 0,
            mmc1_chr1: 0,
            mmc1_prg_bank: 0,
            mmc3: MapperMMC3::new(Mmc3Config::embedded()),
            vsdip: dip_switches,
            coinon: 0,
            cycle_accum: 0,
        }
    }

    fn mmc1_write_shift(&mut self, address: u16, data: u8) {
        if data & 0x80 != 0 {
            self.mmc1_shift = 0;
            self.mmc1_shift_count = 0;
            self.mmc1_control |= 0x0C;
            return;
        }
        self.mmc1_shift >>= 1;
        self.mmc1_shift |= (data & 1) << 4;
        self.mmc1_shift_count += 1;
        if self.mmc1_shift_count < 5 {
            return;
        }
        self.mmc1_shift_count = 0;
        let value = self.mmc1_shift;
        self.mmc1_shift = 0;
        match address & 0xE000 {
            0x8000 => self.mmc1_control = value & 0x1F,
            0xA000 => self.mmc1_chr0 = value,
            0xC000 => self.mmc1_chr1 = value,
            0xE000 => self.mmc1_prg_bank = value,
            _ => {}
        }
    }

    fn mmc1_prg_val(&self, address: u16) -> u8 {
        let mode = (self.mmc1_control >> 2) & 3;
        match mode {
            0 | 1 => self.mmc1_prg_bank & 0x0F,
            2 => {
                if address < 0xC000 { 0 } else { self.mmc1_prg_bank & 0x0F }
            }
            _ => {
                if address < 0xC000 {
                    self.mmc1_prg_bank & 0x0F
                } else {
                    0x0F
                }
            }
        }
    }

    fn mmc1_chr_val(&self, _address: u16) -> u8 {
        let mode = (self.mmc1_control >> 4) & 1;
        if mode == 0 {
            self.mmc1_chr0 & 0x1F
        } else {
            self.mmc1_chr0 & 0x1F
        }
    }

    fn prg_bank_unrom(&self, address: u16) -> u16 {
        let prga17 = if self.rega & 0x20 != 0 { self.rega & 1 } else { 0 };
        let unprg = if address & 0x4000 != 0 { 7 } else { self.unrombank & 7 };
        (((self.rega >> 1) & 0x0F) as u16) << 4
            | (prga17 as u16) << 3
            | (unprg as u16) << 1
            | ((address >> 13) & 1) as u16
    }

    fn prg_bank_amrom(&self, address: u16) -> u16 {
        let prga17b = if self.rega & 0x20 != 0 {
            (self.amrombank >> 2) & 1
        } else {
            0
        };
        (((self.rega >> 1) & 0x0F) as u16) << 4
            | (prga17b as u16) << 3
            | ((self.amrombank & 3) as u16) << 2
            | ((address >> 13) & 3) as u16
    }

    fn prg_bank_mmc1(&self, address: u16) -> u16 {
        let prga17 = if self.rega & 0x20 != 0 { self.rega & 1 } else { 0 };
        (((self.rega >> 1) & 0x0F) as u16) << 4
            | (prga17 as u16) << 3
            | (self.mmc1_prg_val(address) & 0x0F) as u16
    }

    fn mmc3_prg_bank(&self, bank_idx: u8) -> u8 {
        let mut b = bank_idx;
        if self.mmc3.r8000 & 0x40 != 0 && b & 1 == 0 {
            b ^= 2;
        }
        if b & 2 != 0 {
            0xFE | (b & 1)
        } else {
            if b & 1 == 0 { self.mmc3.bank_8c } else { self.mmc3.bank_a }
        }
    }

    fn prg_bank_mmc3(&self, address: u16) -> u16 {
        let bank_idx = match address {
            0x8000..=0x9FFF => 0,
            0xA000..=0xBFFF => 1,
            0xC000..=0xDFFF => 2,
            _ => 3,
        };
        let raw = self.mmc3_prg_bank(bank_idx);
        let prg_and = if self.rega & 0x20 != 0 { 0x0F } else { 0x1F };
        let prg_or = ((self.rega as u16) << 4) & 0x1F0;
        ((raw as u16) & prg_and as u16) | prg_or
    }

    fn ctrl_bank(&self, address: u16) -> u16 {
        (0x38u16 << 3) | ((address >> 13) & 7) as u16
    }

    fn prg_bank(&self, address: u16) -> u16 {
        if address >= 0x8000 {
            if self.rega & 0x80 == 0 {
                return self.ctrl_bank(address);
            }
            match (self.regb >> 4) & 3 {
                0 => self.prg_bank_unrom(address),
                1 => self.prg_bank_amrom(address),
                2 => self.prg_bank_mmc1(address),
                _ => self.prg_bank_mmc3(address),
            }
        } else if address >= 0x6000 {
            if self.rega & 0x20 != 0 {
                return self.ctrl_bank(address);
            }
            0
        } else if address >= 0x5000 {
            self.ctrl_bank(address)
        } else {
            0
        }
    }

    fn chr_bank_12(&self, ppu_addr: u16) -> usize {
        let vram_en = self.rega & 0x40 != 0;
        if !vram_en {
            return 0x800 | ((ppu_addr >> 10) & 7) as usize;
        }
        let chr_base = ((self.regb >> 1) & 7) as usize;
        let raw = mmc3_chr_bank(
            self.mmc3.r8000,
            self.mmc3.chr_2k0,
            self.mmc3.chr_2k8,
            self.mmc3.chr_1k0,
            self.mmc3.chr_1k4,
            self.mmc3.chr_1k8,
            self.mmc3.chr_1kc,
            ppu_addr,
        );
        let chra17 = if self.regb & 0x40 != 0 {
            (self.regb & 1) as usize
        } else {
            (raw >> 7) as usize
        };
        let mapper_bit4 = (self.regb >> 4) & 1;
        if mapper_bit4 == 0 {
            let m1 = self.mmc1_chr_val(ppu_addr) as usize;
            (chr_base << 8) | (chra17 << 7) | (m1 << 2) | ((ppu_addr as usize >> 10) & 3)
        } else {
            (chr_base << 8) | (chra17 << 7) | (raw as usize & 0x7F)
        }
    }
}

impl Mapper for Mapper124 {
    fn reset(&mut self) {
        self.rega = 0;
        self.regb = 0;
        self.auden = 0;
        self.unrombank = 0;
        self.amrombank = 0;
        self.mmc1_shift = 0;
        self.mmc1_shift_count = 0;
        self.mmc1_control = 0;
        self.mmc1_chr0 = 0;
        self.mmc1_chr1 = 0;
        self.mmc1_prg_bank = 0;
        self.mmc3.reset();
        self.coinon = 0;
        self.cycle_accum = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address & 0xFF0F == 0x4F0F {
            let dip = self.vsdip;
            let mut sub = 0u8;
            if self.coinon == 0 { sub |= 0x80; } 
            sub |= ((dip >> 2) & 1) << 6;
            sub |= ((dip >> 4) & 1) << 5;
            sub |= ((dip >> 6) & 1) << 4;
            sub |= ((dip >> 7) & 1) << 3;
            sub |= ((dip >> 5) & 1) << 2;
            sub |= ((dip >> 3) & 1) << 1;
            sub |= ((dip >> 1) & 1) << 0;
            return FetchResult { data: sub, driven: true };
        }
        if address >= 0x5000 && address < 0x6000 {
            let bank = self.ctrl_bank(address);
            let off = bank as usize * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult { data: prg_rom_read(cart, off), driven: true };
        }
        if address >= 0x6000 && address < 0x8000 {
            if self.rega & 0x20 == 0 && !cart.prg_ram.is_empty() {
                let off = (address as usize - 0x6000) % cart.prg_ram.len();
                return FetchResult { data: cart.prg_ram[off], driven: true };
            }
            let bank = self.ctrl_bank(address);
            let off = bank as usize * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult { data: prg_rom_read(cart, off), driven: true };
        }
        if address >= 0x8000 {
            let bank = self.prg_bank(address);
            let off = bank as usize * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult { data: prg_rom_read(cart, off), driven: true };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address & 0xF000 == 0x5000 {
            match address & 0x000F {
                0x00 => self.auden = data & 1,
                0x01 => self.rega = data,
                0x02 => self.regb = data & 0x7F,
                _ => {}
            }
            return;
        }
        if address >= 0x6000 && address < 0x8000 {
            if self.rega & 0x20 == 0 && !cart.prg_ram.is_empty() {
                let off = (address as usize - 0x6000) % cart.prg_ram.len();
                cart.prg_ram[off] = data;
            }
            return;
        }
        if address >= 0x8000 {
            self.unrombank = data & 0x07;
            self.amrombank = (data & 0x10) >> 1 | (data & 0x07);
            self.mmc3.store_prg(cart, address, data);
            self.mmc1_write_shift(address, data);
        }
    }

    fn insert_coin(&mut self, coin: u8) {
        if coin == 0 {
            self.coinon = 6;
        }
    }

    fn get_dip_switches(&self) -> u8 {
        self.vsdip
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.vsdip = value;
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_addr(self.regb, self.amrombank, self.mmc1_control, self.mmc3.nametable_mirroring(), address)
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
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = if self.rega & 0x40 == 0 {
                if !chr_ram.is_empty() {
                    chr_ram[address as usize % chr_ram.len()]
                } else {
                    0
                }
            } else {
                let bank = self.chr_bank_12(address);
                if !chr_rom.is_empty() {
                    let off = bank * 0x400 + (address as usize & 0x3FF);
                    chr_rom[off % chr_rom.len()]
                } else if !chr_ram.is_empty() {
                    let off = bank * 0x400 + (address as usize & 0x3FF);
                    chr_ram[off % chr_ram.len()]
                } else {
                    0
                }
            };
            new_addr_bus |= byte as u16;
        } else if address < 0x3F00 {
            let mmc3_horiz = if (self.regb >> 4) & 3 == 3 { self.mmc3.nametable_mirroring() } else { false };
            let mirrored = mirror_addr(self.regb, self.amrombank, self.mmc1_control, mmc3_horiz, address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            let chr_len = cart.chr_ram.len();
            if chr_len > 0 {
                if self.rega & 0x40 == 0 {
                    cart.chr_ram[address as usize % chr_len] = data;
                } else {
                    let bank = self.chr_bank_12(address);
                    let offset = (bank * 0x400 + (address as usize & 0x3FF)) % chr_len;
                    cart.chr_ram[offset] = data;
                }
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
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
        if (self.regb >> 4) & 3 == 3 {
            self.mmc3
                .ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
        } else {
            false
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.cycle_accum += cycles as u64;
        if self.cycle_accum >= VS_FRAME_CYCLES {
            self.cycle_accum = 0;
            if self.coinon > 0 {
                self.coinon -= 1;
            }
        }
        if (self.regb >> 4) & 3 == 3 {
            self.mmc3.cpu_clock(cycles)
        } else {
            false
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        if (self.regb >> 4) & 3 == 3 {
            self.mmc3.take_irq_ack()
        } else {
            false
        }
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.rega);
        state.push(self.regb);
        state.push(self.auden);
        state.push(self.unrombank);
        state.push(self.amrombank);
        state.push(self.vsdip);
        state.push(self.coinon);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut i = self.mmc3.load_mapper_registers(cart, state, start);
        let end = i;
        if i + 7 <= state.len() {
            self.rega = state[i]; i += 1;
            self.regb = state[i]; i += 1;
            self.auden = state[i]; i += 1;
            self.unrombank = state[i]; i += 1;
            self.amrombank = state[i]; i += 1;
            self.vsdip = state[i]; i += 1;
            self.coinon = state[i]; i += 1;
        }
        i - end
    }
}
