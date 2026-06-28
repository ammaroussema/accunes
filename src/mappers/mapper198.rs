use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper198 {
    mmc3: MapperMMC3,
    prg_ram: [u8; 0x1000],
    raw_reg6: u8,
    raw_reg7: u8,
}

fn apply_masked_bank(bank: u8) -> u8 {
    if bank >= 0x40 { bank & 0x4F } else { bank }
}

impl Mapper198 {
    pub fn new() -> Self {
        Self {
            mmc3: MapperMMC3::new(Mmc3Config::embedded()),
            prg_ram: [0; 0x1000],
            raw_reg6: 0,
            raw_reg7: 1,
        }
    }

    fn prg_bank(&self, bank: u8) -> usize {
        apply_masked_bank(bank) as usize & 0x7F
    }
}

impl Mapper for Mapper198 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.raw_reg6 = 0;
        self.raw_reg7 = 1;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x5000 {
            return FetchResult { data: 0, driven: false };
        }
        if address < 0x6000 {
            return FetchResult { data: self.prg_ram[(address & 0xFFF) as usize], driven: true };
        }
        if address < 0x8000 {
            return self.mmc3.fetch_prg(cart, address);
        }
        let prg_len = cart.prg_rom.len();
        if prg_len == 0 {
            return FetchResult { data: 0, driven: false };
        }
        let banks = prg_len / 0x2000;
        let bank_num = match address & 0xE000 {
            0x8000 => {
                if (self.mmc3.r8000 & 0x40) == 0 {
                    self.prg_bank(self.raw_reg6) % banks
                } else {
                    banks.saturating_sub(2) % banks
                }
            }
            0xA000 => self.prg_bank(self.raw_reg7) % banks,
            0xC000 => {
                if (self.mmc3.r8000 & 0x40) != 0 {
                    self.prg_bank(self.raw_reg6) % banks
                } else {
                    banks.saturating_sub(2) % banks
                }
            }
            0xE000 => banks.saturating_sub(1) % banks,
            _ => return FetchResult { data: 0, driven: false },
        };
        let offset = bank_num * 0x2000 + (address as usize & 0x1FFF);
        if offset < prg_len {
            FetchResult { data: cart.prg_rom[offset], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x5000 {
            return;
        }
        if address < 0x6000 {
            self.prg_ram[(address & 0xFFF) as usize] = data;
            return;
        }
        if address >= 0x8000 && (address & 0xE001) == 0x8001 {
            let mode = self.mmc3.r8000 & 0x07;
            match mode {
                6 => self.raw_reg6 = data,
                7 => self.raw_reg7 = data,
                _ => {}
            }
        }
        self.mmc3.store_prg(cart, address, data);
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mmc3.mirror_nametable(cart, address)
    }

    fn fetch_ppu(
        &mut self,
        prg_rom: &[u8],
        chr_rom: &[u8],
        prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        self.mmc3.fetch_ppu(
            prg_rom, chr_rom, prg_ram, chr_ram, prg_vram,
            using_chr_ram, nametable_horizontal_mirroring,
            alternative_nametable_arrangement,
            ppu_address_bus, ppu_octal_latch, vram,
        )
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        self.mmc3.store_ppu(cart, address, data, vram);
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
        state.extend_from_slice(&self.prg_ram);
        state.push(self.raw_reg6);
        state.push(self.raw_reg7);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        for i in 0..self.prg_ram.len() {
            if p < state.len() {
                self.prg_ram[i] = state[p];
                p += 1;
            }
        }
        if p < state.len() {
            self.raw_reg6 = state[p];
            p += 1;
        }
        if p < state.len() {
            self.raw_reg7 = state[p];
            p += 1;
        }
        p
    }
}
