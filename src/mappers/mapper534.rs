use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, mmc3_chr_bank};

#[derive(Clone, Copy, PartialEq)]
pub enum Ax5202pVariant {
    Mapper126,
    Mapper422,
    Mapper534,
}

pub struct MapperAx5202p {
    variant: Ax5202pVariant,
    mmc3: MapperMMC3,
    reg: [u8; 4],
}

fn prg_rom_read(cart: &Cartridge, offset: usize) -> u8 {
    if cart.prg_rom.is_empty() { 0 } else { cart.prg_rom[offset % cart.prg_rom.len()] }
}

impl MapperAx5202p {
    pub fn new(variant: Ax5202pVariant) -> Self {
        Self {
            variant,
            mmc3: MapperMMC3::new(Mmc3Config::embedded()),
            reg: [0; 4],
        }
    }

    fn last_prg_bank_index(&self, cart: &Cartridge) -> usize {
        let num = cart.prg_rom.len() / 0x2000;
        if num == 0 { 0 } else { num - 1 }
    }

    fn second_last_prg_bank_index(&self, cart: &Cartridge) -> usize {
        let num = cart.prg_rom.len() / 0x2000;
        if num < 2 { 0 } else { num - 2 }
    }

    fn mmc3_prg_bank_val(&self, cart: &Cartridge, bank_index: usize) -> u8 {
        let mask = {
            let banks = cart.prg_rom.len() / 0x2000;
            if banks == 0 { 0 } else { (banks - 1) as u8 }
        };
        match bank_index {
            0 => {
                if (self.mmc3.r8000 & 0x40) == 0 {
                    self.mmc3.bank_8c & mask
                } else {
                    self.second_last_prg_bank_index(cart) as u8
                }
            }
            1 => self.mmc3.bank_a & mask,
            2 => {
                if (self.mmc3.r8000 & 0x40) != 0 {
                    self.mmc3.bank_8c & mask
                } else {
                    self.second_last_prg_bank_index(cart) as u8
                }
            }
            _ => self.last_prg_bank_index(cart) as u8,
        }
    }

    fn prg_page_transform_nrs(&self, raw_page: u16) -> u16 {
        let and_mask = if (self.reg[0] & 0x40) != 0 { 0x0Fu16 } else { 0x1Fu16 };
        let r0 = self.reg[0] as u16;
        let or_bits = (r0 << 4 & 0x70) | ((r0 ^ 0x20) << 3 & 0x180);
        let masked_or = or_bits & !and_mask;
        (raw_page & and_mask) | masked_or
    }

    fn prg_page_transform_mesen2(&self, raw_page: u16) -> u16 {
        let reg = self.reg[0] as u16;
        let mut p = raw_page;
        p &= ((!reg >> 2) & 0x10) | 0x0F;
        p |= (reg & (0x06 | ((reg & 0x40) >> 6))) << 4 | (reg & 0x10) << 3;
        p
    }

    fn prg_final_bank_nrs(&self, cart: &Cartridge, bank8: usize) -> usize {
        let raw = self.mmc3_prg_bank_val(cart, bank8) as u16;
        let mut transformed = self.prg_page_transform_nrs(raw);
        let reg3 = self.reg[3];
        if (reg3 & 0x08) != 0 {
            match reg3 & 0x03 {
                0 => transformed = (transformed & 3) | ((transformed << 1) & !3),
                1 => transformed = (bank8 as u16 & 3) | ((transformed << 1) & !1),
                2 => transformed = (transformed & 3) | ((transformed << 2) & !3),
                _ => transformed = (bank8 as u16 & 3) | ((transformed << 2) & !3),
            }
        } else if (reg3 & 0x01) != 0 {
            transformed = (bank8 as u16 & 1) | (transformed & !1);
            if (reg3 & 0x02) != 0 {
                transformed = (bank8 as u16 & 2) | (transformed & !2);
            }
        }
        let and_mask = if (self.reg[0] & 0x40) != 0 { 0x0Fu16 } else { 0x1Fu16 };
        let r0 = self.reg[0] as u16;
        let or_bits = (r0 << 4 & 0x70) | ((r0 ^ 0x20) << 3 & 0x180);
        let mut adjusted_and = and_mask;
        let mut adjusted_or = or_bits & !and_mask;
        if cart.sub_mapper == 1 {
            adjusted_or = (adjusted_or & 0x7F) | ((adjusted_or >> 1) & 0x80);
        }
        if cart.sub_mapper == 2 {
            adjusted_or = (adjusted_or & 0x7F) | ((self.reg[1] as u16) << 5 & 0x80);
        }
        if cart.sub_mapper == 3 && (self.reg[0] & 0x04) != 0 {
            adjusted_and &= !0x02;
            if (self.reg[0] & 0x20) == 0 {
                adjusted_or |= 0x02;
            }
        }
        (transformed & adjusted_and | adjusted_or & !adjusted_and) as usize
    }

    fn prg_final_bank_mesen2(&self, cart: &Cartridge, bank8: usize) -> usize {
        let mode = self.reg[3] & 0x03;
        if mode == 0 {
            let raw = self.mmc3_prg_bank_val(cart, bank8) as u16;
            return self.prg_page_transform_mesen2(raw) as usize;
        }
        let prg_mode_swapped = (self.mmc3.r8000 & 0x40) != 0;
        let swapped_slot = if prg_mode_swapped { 2 } else { 0 };
        let raw = self.mmc3_prg_bank_val(cart, swapped_slot) as u16;
        let base = self.prg_page_transform_mesen2(raw);
        if mode == 3 {
            (base + bank8 as u16) as usize
        } else {
            (base + (bank8 & 1) as u16) as usize
        }
    }

    fn prg_final_bank(&self, cart: &Cartridge, bank8: usize) -> usize {
        match self.variant {
            Ax5202pVariant::Mapper126 => self.prg_final_bank_mesen2(cart, bank8),
            Ax5202pVariant::Mapper422 | Ax5202pVariant::Mapper534 => self.prg_final_bank_nrs(cart, bank8),
        }
    }

    fn store_regs_nrs(&mut self, address: u16, data: u8) {
        let idx = (address & 3) as usize;
        if idx == 2 {
            let mut mask = 0xFFu8;
            if (self.reg[2] & 0x80) != 0 { mask &= 0x0F; }
            mask &= !((self.reg[2] >> 3) & 0x0E);
            self.reg[2] = self.reg[2] & !mask | data & mask;
        } else if (self.reg[3] & 0x80) == 0 {
            self.reg[idx] = data;
        }
    }

    fn store_regs_mesen2(&mut self, address: u16, data: u8) {
        let idx = (address & 3) as usize;
        let writable = idx == 1
            || idx == 2
            || ((idx == 0 || idx == 3) && (self.reg[3] & 0x80) == 0);
        if writable {
            self.reg[idx] = data;
        }
    }

    fn chr_outer_bank_mesen2(&self) -> u16 {
        let reg = self.reg[0] as u16;
        let r2 = self.reg[2] as u16;
        ((!reg) & 0x0080 & r2)
            | ((reg << 4) & 0x0080 & reg)
            | ((reg << 3) & 0x0100)
            | ((reg << 5) & 0x0200)
    }

    fn chr_page_transform_mesen2(&self, raw_page: u8) -> u16 {
        let mask = if (self.reg[0] & 0x80) != 0 { 0x7Fu8 } else { 0xFFu8 };
        self.chr_outer_bank_mesen2() | ((raw_page & mask) as u16)
    }

    fn chr_page_transform_nrs(&self, raw_bank: u8) -> u8 {
        let chr_and = if (self.reg[0] & 0x80) != 0 { 0x7F } else { 0xFF };
        let chr_or = if self.variant == Ax5202pVariant::Mapper126 {
            (((self.reg[0] as u16) << 4) & 0x080
                | (((self.reg[0] as u16) ^ 0x20) << 3) & 0x100
                | ((self.reg[0] as u16) << 5) & 0x200) & !(chr_and as u16)
        } else {
            ((((self.reg[0] as u16) ^ 0x20) << 4) & 0x380
                | ((self.reg[0] as u16) << 8) & 0x400) & !(chr_and as u16)
        };
        ((raw_bank as u16 & chr_and as u16) | (chr_or & !(chr_and as u16))) as u8
    }
}

impl Mapper for MapperAx5202p {
    fn reset(&mut self) {
        self.reg = [0; 4];
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x6000 {
            return FetchResult { data: 0, driven: false };
        }
        if address < 0x8000 {
            if address < 0x6004 {
                let idx = (address & 3) as usize;
                return FetchResult { data: self.reg[idx], driven: true };
            }
            let offset = (address - 0x6000) as usize;
            if offset < cart.prg_ram.len() {
                return FetchResult { data: cart.prg_ram[offset], driven: true };
            }
            return FetchResult { data: 0, driven: false };
        }
        let bank8 = (address as usize - 0x8000) / 0x2000;
        if bank8 > 3 {
            return FetchResult { data: 0, driven: false };
        }
        let final_bank = self.prg_final_bank(cart, bank8);
        let offset = final_bank * 0x2000 + (address as usize & 0x1FFF);
        FetchResult {
            data: prg_rom_read(cart, offset),
            driven: true,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            match self.variant {
                Ax5202pVariant::Mapper126 => self.store_regs_mesen2(address, data),
                Ax5202pVariant::Mapper422 | Ax5202pVariant::Mapper534 => self.store_regs_nrs(address, data),
            }
            return;
        }
        if address >= 0x8000 {
            match self.variant {
                Ax5202pVariant::Mapper126 => {
                    self.mmc3.store_prg(cart, address, data);
                }
                Ax5202pVariant::Mapper422 | Ax5202pVariant::Mapper534 => {
                    let addr = if (self.reg[3] & 0x08) != 0 {
                        (address & !1) | 1
                    } else {
                        address
                    };
                    if (self.reg[3] & 0x09) == 0x09 {
                        self.mmc3.store_prg(cart, addr & 0xE001, data);
                    } else {
                        self.mmc3.store_prg(cart, addr, data);
                    }
                }
            }
            if self.variant == Ax5202pVariant::Mapper534 {
                let upper = address & 0xE000;
                if upper == 0xC000 || upper == 0xE000 {
                    let mask = address & 1;
                    self.mmc3.store_prg(cart, 0xC000 | mask, data ^ 0xFF);
                }
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        let reg3 = self.reg[3];
        let reg1 = self.reg[1];
        if (reg3 & 0x20) != 0 {
            if (self.mmc3.bank_a & 0x10) != 0 {
                0x2000 | (address & 0x3FF)
            } else {
                address & 0x23FF
            }
        } else if (reg1 & 0x02) != 0 {
            let h = self.mmc3.nametable_mirroring();
            if h {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            }
        } else {
            self.mmc3.mirror_nametable(cart, address)
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
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address >= 0x2000 {
            let h = self.mmc3.nametable_mirroring();
            let mirrored = if h {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
            return (new_addr_bus as u8, new_addr_bus);
        }
        let use_cnrom = (self.reg[3] & 0x10) != 0;
        if use_cnrom {
            let base = match self.variant {
                Ax5202pVariant::Mapper126 => {
                    self.chr_outer_bank_mesen2() | (((self.reg[2] & 0x0F) as u16) << 3)
                }
                Ax5202pVariant::Mapper422 | Ax5202pVariant::Mapper534 => {
                    let chr_and = if (self.reg[0] & 0x80) != 0 { 0x7F } else { 0xFF };
                    let chr_or = ((((self.reg[0] as u16) ^ 0x20) << 4) & 0x380
                        | ((self.reg[0] as u16) << 8) & 0x400) & !(chr_and as u16);
                    let cnrom_bank = (self.reg[2] & (chr_and >> 3)) as u16 | ((chr_or >> 3) & !(chr_and as u16 >> 3));
                    cnrom_bank * 0x2000 / 0x0400 
                }
            };
            let slot = (address >> 10) as u16;
            let page = base + slot;
            let offset = (page as usize) * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[offset % chr_ram.len()] }
            } else if chr_rom.is_empty() { 0 } else { chr_rom[offset % chr_rom.len()] };
            new_addr_bus |= byte as u16;
        } else {
            let raw_bank = mmc3_chr_bank(
                self.mmc3.r8000,
                self.mmc3.chr_2k0,
                self.mmc3.chr_2k8,
                self.mmc3.chr_1k0,
                self.mmc3.chr_1k4,
                self.mmc3.chr_1k8,
                self.mmc3.chr_1kc,
                address,
            );
            let bank: u16 = match self.variant {
                Ax5202pVariant::Mapper126 => self.chr_page_transform_mesen2(raw_bank),
                Ax5202pVariant::Mapper422 | Ax5202pVariant::Mapper534 => self.chr_page_transform_nrs(raw_bank) as u16,
            };
            let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[offset % chr_ram.len()] }
            } else if chr_rom.is_empty() { 0 } else { chr_rom[offset % chr_rom.len()] };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[address as usize & 0x1FFF] = data;
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
        self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        for r in &self.reg {
            state.push(*r);
        }
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        for r in &mut self.reg {
            if p < state.len() {
                *r = state[p];
                p += 1;
            }
        }
        p
    }
}
