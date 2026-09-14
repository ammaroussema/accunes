use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mapper115::Mapper115;

pub struct Mapper248 {
    inner: Mapper115,
}

impl Mapper248 {
    pub fn new(prg_16k_count: u8) -> Self {
        Self { inner: Mapper115::new(prg_16k_count) }
    }
}

impl Mapper for Mapper248 {
    fn reset(&mut self) { self.inner.reset(); }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        self.inner.fetch_prg(cart, address)
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        self.inner.store_prg(cart, address, data);
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.inner.mirror_nametable(cart, address)
    }

    fn fetch_ppu(
        &mut self,
        prg_rom: &[u8], chr_rom: &[u8], prg_ram: &[u8],
        chr_ram: &[u8], prg_vram: &[u8], using_chr_ram: bool,
        nametable_horizontal_mirroring: bool, alternative_nametable_arrangement: bool,
        ppu_address_bus: u16, ppu_octal_latch: u8, vram: &[u8],
    ) -> (u8, u16) {
        self.inner.fetch_ppu(
            prg_rom, chr_rom, prg_ram, chr_ram, prg_vram,
            using_chr_ram, nametable_horizontal_mirroring, alternative_nametable_arrangement,
            ppu_address_bus, ppu_octal_latch, vram,
        )
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        self.inner.store_ppu(cart, address, data, vram);
    }

    fn ppu_clock(
        &mut self, ppu_address_bus: u16, ppu_a12_prev: bool,
        scanline: u16, dot: u16, ppu_sprite_x16: bool, rendering_on: bool,
    ) -> bool {
        self.inner.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.inner.cpu_clock_rise(ppu_address_bus)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        self.inner.save_mapper_registers(cart)
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        self.inner.load_mapper_registers(cart, state, start)
    }
}
