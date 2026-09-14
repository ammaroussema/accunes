use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper189 {
    mmc3: MapperMMC3,
    reg: u8,
}

impl Mapper189 {
    pub fn new() -> Self {
        let config = Mmc3Config::embedded();
        Self { mmc3: MapperMMC3::new(config), reg: 3 }
    }
}

impl Mapper for Mapper189 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.reg = 3;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            self.mmc3.fetch_prg(cart, address)
        } else if address >= 0x8000 {
            let len = cart.prg_rom.len();
            let bank = ((self.reg as usize) | (self.reg as usize >> 4)) & 0x0F;
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            FetchResult { data: if len > 0 { cart.prg_rom[offset % len] } else { 0 }, driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x4000 && address < 0x6000 {
            if address & 0x100 != 0 {
                self.reg = data;
            } else if address < 0x4020 {
            }
        } else if address >= 0x6000 && address < 0x8000 {
            self.reg = data;
        } else if address >= 0x8000 {
            self.mmc3.store_prg(cart, address, data);
        }
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

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.mmc3.cpu_clock(cycles)
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, sprite_x16, rendering_on)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut s = self.mmc3.save_mapper_registers(cart);
        s.push(self.reg);
        s
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        self.reg = state[p]; p += 1;
        p
    }
}
