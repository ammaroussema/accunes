use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, mmc3_chr_bank};

pub struct Mapper187 {
    mmc3: MapperMMC3,
    reg: u8,
}

impl Mapper187 {
    pub fn new() -> Self {
        Self {
            mmc3: MapperMMC3::new(Mmc3Config::embedded()),
            reg: 0,
        }
    }
}

impl Mapper for Mapper187 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.reg = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x5000 {
            return FetchResult { data: 0, driven: false };
        }
        if address < 0x6000 {
            return FetchResult { data: 0x80, driven: true };
        }
        if address < 0x8000 {
            return self.mmc3.fetch_prg(cart, address);
        }
        let bank = ((address - 0x8000) / 0x2000) as usize;
        let prg_len = cart.prg_rom.len();
        let prg_bank = if self.reg & 0x80 != 0 {
            let prg = (self.reg >> 1) as usize;
            let bank16 = if self.reg & 0x20 != 0 {
                if bank < 2 { prg & !1 } else { prg | 1 }
            } else {
                prg
            };
            bank16 * 2 + (bank & 1)
        } else {
            let total_banks = if prg_len == 0 { 1 } else { prg_len / 0x2000 };
            let mask = if total_banks > 0x40 { 0x3F } else { (total_banks - 1) as u8 };
            match bank {
                0 => {
                    if (self.mmc3.r8000 & 0x40) == 0 {
                        (self.mmc3.bank_8c & mask) as usize
                    } else {
                        (total_banks - 1).wrapping_sub(1)
                    }
                }
                1 => (self.mmc3.bank_a & mask) as usize,
                2 => {
                    if (self.mmc3.r8000 & 0x40) != 0 {
                        (self.mmc3.bank_8c & mask) as usize
                    } else {
                        (total_banks - 1).wrapping_sub(1)
                    }
                }
                3 => (total_banks - 1).wrapping_sub(0),
                _ => return FetchResult { data: 0, driven: false },
            }
        };
        let offset = prg_bank * 0x2000 + (address as usize & 0x1FFF);
        FetchResult {
            data: if prg_len > 0 { cart.prg_rom[offset % prg_len] } else { 0 },
            driven: true,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x6000 {
            if address & 1 == 0 {
                self.reg = data;
            }
        } else {
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
        _prg_vram: &[u8],
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
            );
            let slot = address >> 10;
            let bank = raw_bank as u16 | if (slot & 4) != 0 { 0x100 } else { 0 };
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
            let byte = vram[(mirrored & 0x7FF) as usize];
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
        self.mmc3
            .ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
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
