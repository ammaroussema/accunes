use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper197 {
    mmc3: MapperMMC3,
    reg: u8,
    submapper: u8,
}
const CHR_SLOT_MAP: [[usize; 4]; 3] = [
    [0, 1, 4, 5],  
    [2, 3, 6, 7],  
    [0, 3, 4, 7],  
];

impl Mapper197 {
    pub fn new(submapper: u8) -> Self {
        Self {
            mmc3: MapperMMC3::new(Mmc3Config::embedded()),
            reg: 0,
            submapper,
        }
    }

    fn chr_bank_value(&self, slot: u8) -> u8 {
        let invert = (self.mmc3.r8000 & 0x80) != 0;
        match (invert, slot) {
            (false, 0) | (true, 4) => self.mmc3.chr_2k0 & 0xFE,
            (false, 1) | (true, 5) => self.mmc3.chr_2k0 | 1,
            (false, 2) | (true, 6) => self.mmc3.chr_2k8 & 0xFE,
            (false, 3) | (true, 7) => self.mmc3.chr_2k8 | 1,
            (false, 4) | (true, 0) => self.mmc3.chr_1k0,
            (false, 5) | (true, 1) => self.mmc3.chr_1k4,
            (false, 6) | (true, 2) => self.mmc3.chr_1k8,
            (false, 7) | (true, 3) => self.mmc3.chr_1kc,
            _ => 0,
        }
    }

    fn chr_addr(&self, address: u16) -> usize {
        let group = (address >> 11) as usize;
        let sub_page = ((address >> 10) & 1) as usize;
        if self.submapper == 3 {
            let slot = CHR_SLOT_MAP[0][group];
            let base = self.chr_bank_value(slot as u8) as usize;
            let extra = ((self.reg as usize) << 7) & 0x100;
            let page_base = base | extra;
            let page = page_base * 2 + sub_page;
            page * 0x0400 + (address as usize & 0x03FF)
        } else {
            let slot = CHR_SLOT_MAP[self.submapper as usize % 3][group];
            let base = self.chr_bank_value(slot as u8) as usize;
            let page = base * 2 + sub_page;
            page * 0x0400 + (address as usize & 0x03FF)
        }
    }
}

impl Mapper for Mapper197 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.reg = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            if self.submapper == 3 && address >= 0x5000 && address < 0x6000 {
                return FetchResult { data: 0x80, driven: true };
            }
            return self.mmc3.fetch_prg(cart, address);
        }
        let prg_len = cart.prg_rom.len();
        if prg_len == 0 {
            return FetchResult { data: 0, driven: false };
        }
        if self.submapper == 3 {
            let mask = if self.reg & 0x08 != 0 { 0x0F } else { 0x1F };
            let base = (self.reg as usize) << 4;
            let bank8 = ((address - 0x8000) / 0x2000) as usize;
            let banks = prg_len / 0x2000;
            let prg_bank = match bank8 {
                0 => {
                    if (self.mmc3.r8000 & 0x40) == 0 {
                        (self.mmc3.bank_8c & mask) as usize + base
                    } else {
                        banks.wrapping_sub(2)
                    }
                }
                1 => (self.mmc3.bank_a & mask) as usize + base,
                2 => {
                    if (self.mmc3.r8000 & 0x40) != 0 {
                        (self.mmc3.bank_8c & mask) as usize + base
                    } else {
                        banks.wrapping_sub(2)
                    }
                }
                3 => banks.wrapping_sub(1),
                _ => return FetchResult { data: 0, driven: false },
            };
            let offset = (prg_bank % banks) * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult { data: cart.prg_rom[offset], driven: true };
        }
        self.mmc3.fetch_prg(cart, address)
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x6000 {
            if self.submapper == 3 {
                self.reg = data;
            }
            return;
        }
        self.mmc3.store_prg(cart, address, data);
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
            let offset = self.chr_addr(address);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if self.mmc3.nametable_mirroring() {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let offset = self.chr_addr(address);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
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
        if self.submapper == 3 {
            state.push(self.reg);
        }
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        if self.submapper == 3 && p < state.len() {
            self.reg = state[p];
            p += 1;
        }
        p
    }
}
