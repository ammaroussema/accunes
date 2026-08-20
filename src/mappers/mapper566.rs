use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, mmc3_chr_bank};

pub struct Mapper566 {
    mmc3: MapperMMC3,
    reg: u8,
}

fn prg_rom_read(cart: &Cartridge, offset: usize) -> u8 {
    if cart.prg_rom.is_empty() { 0 } else { cart.prg_rom[offset % cart.prg_rom.len()] }
}

impl Mapper566 {
    pub fn new() -> Self {
        let mut config = Mmc3Config::embedded();
        config.ax5202p = true;
        Self {
            mmc3: MapperMMC3::new(config),
            reg: 0,
        }
    }

    fn prg_and(&self) -> u16 {
        if (self.reg & 0x04) != 0 { 0x0F } else { 0x1F }
    }

    fn chr_and(&self) -> u16 {
        if (self.reg & 0x02) != 0 { 0x7F } else { 0xFF }
    }

    fn prg_or(&self) -> u16 {
        let r = self.reg as u16;
        ((r << 4) & 0x10) | ((r << 2) & 0x20) | ((r << 1) & 0xC0)
    }

    fn chr_or(&self) -> u16 {
        let r = self.reg as u16;
        ((r << 7) & 0x80) | ((r << 4) & 0x700)
    }

    fn prg_raw_bank(&self, cart: &Cartridge, bank8: usize) -> u16 {
        let num = cart.prg_rom.len() / 0x2000;
        match bank8 {
            0 => {
                if (self.mmc3.r8000 & 0x40) != 0 {
                    if num < 2 { 0 } else { (num - 2) as u16 }
                } else {
                    self.mmc3.bank_8c as u16
                }
            }
            1 => self.mmc3.bank_a as u16,
            2 => {
                if (self.mmc3.r8000 & 0x40) != 0 {
                    self.mmc3.bank_8c as u16
                } else {
                    if num < 2 { 0 } else { (num - 2) as u16 }
                }
            }
            _ => {
                if num == 0 { 0 } else { (num - 1) as u16 }
            }
        }
    }

    fn mirror_a17(&self, address: u16) -> u16 {
        let slot = (address >> 10) & 3;
        let bank = mmc3_chr_bank(
            self.mmc3.r8000,
            self.mmc3.chr_2k0,
            self.mmc3.chr_2k8,
            self.mmc3.chr_1k0,
            self.mmc3.chr_1k4,
            self.mmc3.chr_1k8,
            self.mmc3.chr_1kc,
            slot * 0x400,
        );
        (address & 0x3FF) | ((((bank >> 7) & 1) as u16) * 0x400)
    }
}

impl Mapper for Mapper566 {
    fn reset(&mut self) {
        self.reg = 0;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x6000 {
            return FetchResult { data: 0, driven: false };
        }
        if address < 0x8000 {
            let off = (address - 0x6000) as usize;
            if off < cart.prg_ram.len() {
                return FetchResult { data: cart.prg_ram[off], driven: true };
            }
            return FetchResult { data: 0, driven: false };
        }
        let bank8 = (address as usize - 0x8000) / 0x2000;
        if bank8 > 3 {
            return FetchResult { data: 0, driven: false };
        }
        let raw = self.prg_raw_bank(cart, bank8);
        let page = (raw & self.prg_and()) | (self.prg_or() & !self.prg_and());
        let offset = (page as usize) * 0x2000 + (address as usize & 0x1FFF);
        FetchResult {
            data: prg_rom_read(cart, offset),
            driven: true,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if (self.mmc3.prg_ram_protect & 0x40) == 0 {
                let off = (address - 0x6000) as usize;
                if off < cart.prg_ram.len() {
                    cart.prg_ram[off] = data;
                }
                self.reg = (address & 0xFF) as u8;
            }
            return;
        }
        if address >= 0x8000 {
            self.mmc3.store_prg(cart, address, data);
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if (self.reg & 0x80) != 0 {
            self.mirror_a17(address)
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
            let raw_bank = mmc3_chr_bank(
                self.mmc3.r8000,
                self.mmc3.chr_2k0,
                self.mmc3.chr_2k8,
                self.mmc3.chr_1k0,
                self.mmc3.chr_1k4,
                self.mmc3.chr_1k8,
                self.mmc3.chr_1kc,
                address,
            ) as u16;
            let bank = (raw_bank & self.chr_and()) | (self.chr_or() & !self.chr_and());
            let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if (self.reg & 0x80) != 0 {
                self.mirror_a17(address)
            } else if self.mmc3.nametable_mirroring() {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() { prg_vram[idx] } else { 0 }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let raw_bank = mmc3_chr_bank(
                    self.mmc3.r8000,
                    self.mmc3.chr_2k0,
                    self.mmc3.chr_2k8,
                    self.mmc3.chr_1k0,
                    self.mmc3.chr_1k4,
                    self.mmc3.chr_1k8,
                    self.mmc3.chr_1kc,
                    address,
                ) as u16;
                let bank = (raw_bank & self.chr_and()) | (self.chr_or() & !self.chr_and());
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
        state.push(self.reg);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        if p < state.len() {
            self.reg = state[p];
            p += 1;
        }
        p
    }
}