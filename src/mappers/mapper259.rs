use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper259 {
    mmc3: MapperMMC3,
    reg: u8,
}

impl Mapper259 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let config = Mmc3Config::for_ines(header, 0, if chr_size == 0 { 0 } else { chr_size }, rom, rom_name);
        Self { mmc3: MapperMMC3::new(config), reg: 0 }
    }
}

impl Mapper for Mapper259 {
    fn reset(&mut self) {
        self.reg = 0;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let prg_len = cart.prg_rom.len();
            if prg_len == 0 {
                return FetchResult { data: 0, driven: false };
            }
            if (self.reg & 0x08) != 0 {
                let banks_32k = (prg_len / 0x8000).max(1);
                let bank = ((self.reg >> 1) as usize) % banks_32k;
                let offset = bank * 0x8000 + (address as usize & 0x7FFF);
                FetchResult { data: cart.prg_rom[offset % prg_len], driven: true }
            } else {
                let banks_16k = (prg_len / 0x4000).max(1);
                let bank = (self.reg & 0x07) as usize % banks_16k;
                let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                FetchResult { data: cart.prg_rom[offset % prg_len], driven: true }
            }
        } else {
            self.mmc3.fetch_prg(cart, address)
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if (self.mmc3.prg_ram_protect & 0x80) != 0 {
                self.reg = data & 0x0F;
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
        self.mmc3.ppu_clock(
            ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on,
        )
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
        let p = self.mmc3.load_mapper_registers(cart, state, start);
        if p < state.len() {
            self.reg = state[p];
            p + 1
        } else {
            p
        }
    }
}
