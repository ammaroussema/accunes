use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mapper176::{Mapper176, Mapper176Config};

pub struct Mapper523(Mapper176);

impl Mapper523 {
    pub fn new(
        header: &[u8],
        rom: &[u8],
        prg_size: u8,
        using_chr_ram: bool,
        has_battery: bool,
    ) -> Self {
        let mut config = Mapper176Config::for_ines(
            header,
            0,
            rom,
            prg_size,
            using_chr_ram,
            has_battery,
        );
        config.is_523 = true;
        config.header_horizontal_mirroring = if header.len() > 6 {
            (header[6] & 1) == 0
        } else {
            false
        };
        Self(Mapper176::new(config))
    }
}

impl Mapper for Mapper523 {
    fn reset(&mut self) {
        self.0.reset();
    }

    fn reset_power_cycle(&mut self) {
        self.0.reset_power_cycle();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        self.0.fetch_prg(cart, address)
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        self.0.store_prg(cart, address, data);
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.0.mirror_nametable(cart, address)
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
        self.0.fetch_ppu(
            prg_rom,
            chr_rom,
            prg_ram,
            chr_ram,
            prg_vram,
            using_chr_ram,
            nametable_horizontal_mirroring,
            alternative_nametable_arrangement,
            ppu_address_bus,
            ppu_octal_latch,
            vram,
        )
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        self.0.store_ppu(cart, address, data, vram);
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
        self.0.ppu_clock(
            ppu_address_bus,
            ppu_a12_prev,
            scanline,
            dot,
            ppu_sprite_x16,
            rendering_on,
        )
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.0.cpu_clock(cycles)
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.0.cpu_clock_rise(ppu_address_bus)
    }

    fn take_irq_ack(&mut self) -> bool {
        self.0.take_irq_ack()
    }

    fn get_dip_switches(&self) -> u8 {
        self.0.get_dip_switches()
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.0.set_dip_switches(value);
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        self.0.save_mapper_registers(cart)
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        self.0.load_mapper_registers(cart, state, start)
    }

    fn battery_save_data(&self, cart: &Cartridge) -> Option<Vec<u8>> {
        self.0.battery_save_data(cart)
    }

    fn load_battery_save(&mut self, cart: &mut Cartridge, data: &[u8]) {
        self.0.load_battery_save(cart, data);
    }
}
