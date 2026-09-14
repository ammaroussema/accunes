use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::mmc3_chr_bank;
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};
const ADDR_LUT: [[u16; 8]; 8] = [
    [0x8000, 0x8001, 0xA000, 0xA001, 0xC000, 0xC001, 0xE000, 0xE001],
    [0xA001, 0xA000, 0x8000, 0xC000, 0x8001, 0xC001, 0xE000, 0xE001],
    [0x8000, 0x8001, 0xA000, 0xA001, 0xC000, 0xC001, 0xE000, 0xE001],
    [0xC001, 0x8000, 0x8001, 0xA000, 0xA001, 0xE001, 0xE000, 0xC000],
    [0xA001, 0x8001, 0x8000, 0xC000, 0xA000, 0xC001, 0xE000, 0xE001],
    [0x8000, 0x8001, 0xA000, 0xA001, 0xC000, 0xC001, 0xE000, 0xE001],
    [0x8000, 0x8001, 0xA000, 0xA001, 0xC000, 0xC001, 0xE000, 0xE001],
    [0x8000, 0x8001, 0xA000, 0xA001, 0xC000, 0xC001, 0xE000, 0xE001],
];
const DATA_LUT: [[u8; 8]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 2, 6, 1, 7, 3, 4, 5],
    [0, 5, 4, 1, 7, 2, 6, 3],
    [0, 6, 3, 7, 5, 2, 4, 1],
    [0, 2, 5, 3, 6, 1, 7, 4],
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 1, 2, 3, 4, 5, 6, 7],
];
const READ_LUT: [[u8; 8]; 8] = [
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x03, 0x04, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x05, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x01, 0x02, 0x04, 0x0F, 0x00, 0x00, 0x00],
];

pub struct Mapper215 {
    mmc3: MapperMMC3,
    reg: [u8; 8],
}

impl Mapper215 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let using_chr_ram = chr_size == 0;
        let config = Mmc3Config::for_ines(header, 0, if using_chr_ram { 0 } else { chr_size }, rom, rom_name);
        Self { mmc3: MapperMMC3::new(config), reg: [0; 8] }
    }

    fn prg_and(&self) -> u8 {
        if (self.reg[0] & 0x40) != 0 { 0x0F } else { 0x1F }
    }

    fn chr_and(&self) -> u8 {
        if (self.reg[0] & 0x40) != 0 { 0x7F } else { 0xFF }
    }

    fn prg_or(&self) -> u8 {
        (self.reg[1] & 0x10) | ((self.reg[1] << 5) & 0x60) | ((self.reg[1] << 4) & 0x80)
    }

    fn chr_or(&self) -> u16 {
        let shift = 6;
        (u16::from(self.reg[1]) << 2 & 0x80) | (u16::from(self.reg[1]) << shift & 0x700)
    }

    fn mmc3_raw_chr_bank(&self, ppu_addr: u16) -> u8 {
        mmc3_chr_bank(
            self.mmc3.r8000,
            self.mmc3.chr_2k0,
            self.mmc3.chr_2k8,
            self.mmc3.chr_1k0,
            self.mmc3.chr_1k4,
            self.mmc3.chr_1k8,
            self.mmc3.chr_1kc,
            ppu_addr,
        )
    }
}

impl Mapper for Mapper215 {
    fn reset(&mut self) {
        self.reg = [0; 8];
        self.reg[1] = 0xFF;
        self.reg[2] = 7;
        self.reg[7] = 4;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            if address >= 0x5000 && address < 0x6000 {
                let idx = (address as usize) & 7;
                let v = READ_LUT[self.reg[2] as usize][idx] & 0x0F;
                return FetchResult { data: v, driven: true };
            }
            return self.mmc3.fetch_prg(cart, address);
        }
        let prg_len = cart.prg_rom.len();
        if prg_len == 0 {
            return FetchResult { data: 0, driven: false };
        }
        let prg_and = self.prg_and();
        let prg_or = self.prg_or();
        if (self.reg[0] & 0x80) != 0 {
            let prg_and = prg_and >> 1;
            let prg_or = prg_or >> 1;
            let prg = (self.reg[0] & 0x0F) & prg_and | prg_or & !prg_and;
            let slot = ((address as usize - 0x8000) >> 14) & 1;
            let bank = if (self.reg[0] & 0x20) != 0 {
                (prg & 0xFE) | (slot as u8)
            } else {
                prg
            };
            let num_16k = (prg_len / 0x4000).max(1);
            let offset = (bank as usize % num_16k) * 0x4000 + (address as usize & 0x3FFF);
            FetchResult { data: cart.prg_rom[offset % prg_len], driven: true }
        } else {
            let raw_bank = if address >= 0xE000 {
                (prg_len / 0x2000).saturating_sub(1) as u8
            } else if address >= 0xC000 {
                if (self.mmc3.r8000 & 0x40) != 0 {
                    self.mmc3.bank_8c
                } else {
                    (prg_len / 0x2000).saturating_sub(2) as u8
                }
            } else if address >= 0xA000 {
                self.mmc3.bank_a
            } else {
                if (self.mmc3.r8000 & 0x40) == 0 {
                    self.mmc3.bank_8c
                } else {
                    (prg_len / 0x2000).saturating_sub(2) as u8
                }
            };
            let bank = (raw_bank & prg_and) | (prg_or & !prg_and);
            let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
            FetchResult { data: cart.prg_rom[offset % prg_len], driven: true }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x6000 {
            let idx = (address as usize) & 7;
            self.reg[idx] = data;
            return;
        }
        if address >= 0x8000 {
            let bank_idx = ((address as usize) >> 12) & 6;
            let addr_bit = (address as usize) & 1;
            let lut_row = (self.reg[7] & 7) as usize;
            let lut_value = ADDR_LUT[lut_row][bank_idx | addr_bit];
            let value = if lut_value == 0x8000 {
                (data & 0xC0) | DATA_LUT[lut_row][(data & 7) as usize]
            } else {
                data
            };
            self.mmc3.store_prg(cart, lut_value, value);
            return;
        }
        self.mmc3.store_prg(cart, address, data);
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else if self.mmc3.nametable_mirroring() {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
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
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let raw_bank = self.mmc3_raw_chr_bank(address);
            let chr_and = self.chr_and();
            let chr_or = self.chr_or();
            let bank = (raw_bank & chr_and) | ((chr_or & !u16::from(chr_and)) as u8);
            let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if nametable_horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            self.mmc3.store_ppu(cart, address, data, vram);
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
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
        state.extend_from_slice(&self.reg);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let p = self.mmc3.load_mapper_registers(cart, state, start);
        let end = (p + 8).min(state.len());
        for i in p..end {
            self.reg[i - p] = state[i];
        }
        end
    }
}
