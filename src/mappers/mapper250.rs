use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper250 {
    mmc3: MapperMMC3,
    irq_ack: bool,
}

impl Mapper250 {
    pub fn new() -> Self {
        Self { mmc3: MapperMMC3::new(Mmc3Config::embedded()), irq_ack: false }
    }
}

impl Mapper for Mapper250 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.irq_ack = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        self.mmc3.fetch_prg(cart, address)
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, _data: u8) {
        if address < 0x8000 {
            self.mmc3.store_prg(cart, address, _data);
        } else {
            let scrambled = (address & 0xE000) | ((address & 0x0400) >> 10);
            let value = (address & 0xFF) as u8;
            self.irq_ack = scrambled == 0xE000;
            self.mmc3.store_prg(cart, scrambled, value);
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
            alternative_nametable_arrangement, ppu_address_bus,
            ppu_octal_latch, vram,
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

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack;
        self.irq_ack = false;
        ack
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(if self.irq_ack { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let end = self.mmc3.load_mapper_registers(cart, state, start);
        self.irq_ack = state.get(end).copied().unwrap_or(0) != 0;
        end + 1
    }
}
