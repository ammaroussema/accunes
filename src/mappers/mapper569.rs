use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, mmc3_chr_bank};

pub struct Mapper569 {
    mmc3: MapperMMC3,
    reg: u8,
}

fn prg_rom_read(cart: &Cartridge, offset: usize) -> u8 {
    if cart.prg_rom.is_empty() { 0 } else { cart.prg_rom[offset % cart.prg_rom.len()] }
}

impl Mapper569 {
    pub fn new() -> Self {
        let mut config = Mmc3Config::embedded();
        config.irq_revision_b = true;
        Self {
            mmc3: MapperMMC3::new(config),
            reg: 0,
        }
    }

    fn std_prg_bank(&self, cart: &Cartridge, bank: usize) -> u16 {
        let num = cart.prg_rom.len() / 0x2000;
        let second_last = if num < 2 { 0 } else { (num - 2) as u16 };
        let last = if num == 0 { 0 } else { (num - 1) as u16 };
        match bank {
            0 => {
                if (self.mmc3.r8000 & 0x40) != 0 { second_last } else { self.mmc3.bank_8c as u16 }
            }
            1 => self.mmc3.bank_a as u16,
            2 => {
                if (self.mmc3.r8000 & 0x40) != 0 { self.mmc3.bank_8c as u16 } else { second_last }
            }
            _ => last,
        }
    }

    fn prg_raw_bank(&self, cart: &Cartridge, bank8: usize) -> u16 {
        if (self.reg & 0x08) != 0 {
            (self.std_prg_bank(cart, 0) & !3) | (bank8 as u16 & 3)
        } else {
            self.std_prg_bank(cart, bank8)
        }
    }

    fn chr_page(&self, address: u16) -> u16 {
        let chr_and = if (self.reg & 0x02) != 0 { 0xFF } else { 0x7F };
        let chr_or = (self.reg as u16) << 7;
        if (self.reg & 0x04) != 0 {
            let slot = (address >> 10) as u16;
            let base = match slot {
                0 | 1 => mmc3_chr_bank(
                    self.mmc3.r8000,
                    self.mmc3.chr_2k0,
                    self.mmc3.chr_2k8,
                    self.mmc3.chr_1k0,
                    self.mmc3.chr_1k4,
                    self.mmc3.chr_1k8,
                    self.mmc3.chr_1kc,
                    0x0000,
                ),
                2 | 3 => mmc3_chr_bank(
                    self.mmc3.r8000,
                    self.mmc3.chr_2k0,
                    self.mmc3.chr_2k8,
                    self.mmc3.chr_1k0,
                    self.mmc3.chr_1k4,
                    self.mmc3.chr_1k8,
                    self.mmc3.chr_1kc,
                    0x0C00,
                ),
                4 | 5 => mmc3_chr_bank(
                    self.mmc3.r8000,
                    self.mmc3.chr_2k0,
                    self.mmc3.chr_2k8,
                    self.mmc3.chr_1k0,
                    self.mmc3.chr_1k4,
                    self.mmc3.chr_1k8,
                    self.mmc3.chr_1kc,
                    0x1000,
                ),
                _ => mmc3_chr_bank(
                    self.mmc3.r8000,
                    self.mmc3.chr_2k0,
                    self.mmc3.chr_2k8,
                    self.mmc3.chr_1k0,
                    self.mmc3.chr_1k4,
                    self.mmc3.chr_1k8,
                    self.mmc3.chr_1kc,
                    0x1C00,
                ),
            };
            let bank = (base as u16 & chr_and) | (chr_or & !chr_and);
            (bank << 1) + (slot & 1)
        } else {
            let raw = mmc3_chr_bank(
                self.mmc3.r8000,
                self.mmc3.chr_2k0,
                self.mmc3.chr_2k8,
                self.mmc3.chr_1k0,
                self.mmc3.chr_1k4,
                self.mmc3.chr_1k8,
                self.mmc3.chr_1kc,
                address,
            ) as u16;
            (raw & chr_and) | (chr_or & !chr_and)
        }
    }
}

impl Mapper for Mapper569 {
    fn reset(&mut self) {
        self.reg = 0;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x6000 {
            return FetchResult { data: 0, driven: false };
        }
        if address < 0x8000 {
            if (self.mmc3.prg_ram_protect & 0x80) != 0 {
                let off = (address - 0x6000) as usize;
                if off < cart.prg_ram.len() {
                    return FetchResult { data: cart.prg_ram[off], driven: true };
                }
            }
            return FetchResult { data: 0, driven: false };
        }
        let bank8 = (address as usize - 0x8000) / 0x2000;
        if bank8 > 3 {
            return FetchResult { data: 0, driven: false };
        }
        let raw = self.prg_raw_bank(cart, bank8);
        let page = (raw & 0x0F) | ((self.reg as u16) << 4);
        let offset = (page as usize) * 0x2000 + (address as usize & 0x1FFF);
        FetchResult {
            data: prg_rom_read(cart, offset),
            driven: true,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if (self.mmc3.prg_ram_protect & 0xC0) == 0x80 {
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
        self.mmc3.mirror_nametable(cart, address)
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
            let bank = self.chr_page(address);
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
            let mirrored = if alternative_nametable_arrangement {
                address
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
                let bank = self.chr_page(address);
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