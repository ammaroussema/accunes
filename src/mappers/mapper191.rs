use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper191 {
    mmc3: MapperMMC3,
}

impl Mapper191 {
    pub fn new() -> Self {
        Self { mmc3: MapperMMC3::new(Mmc3Config::embedded()) }
    }
}

impl Mapper for Mapper191 {
    fn reset(&mut self) {
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        self.mmc3.fetch_prg(cart, address)
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
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
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = self.mmc3.chr_bank(address);
            let offset = (address as usize) & 0x03FF;
            let byte = if bank & 0x80 != 0 {
                let idx = ((bank & 1) as usize) * 0x0400 + offset;
                if !chr_ram.is_empty() && idx < chr_ram.len() { chr_ram[idx] } else { 0 }
            } else {
                let idx = (bank as usize) * 0x0400 + offset;
                if !chr_rom.is_empty() && idx < chr_rom.len() { chr_rom[idx] }
                else if !chr_ram.is_empty() && idx < chr_ram.len() { chr_ram[idx] }
                else { 0 }
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
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            let bank = self.mmc3.chr_bank(address);
            if bank & 0x80 != 0 {
                let idx = ((bank & 1) as usize) * 0x0400 + (address as usize & 0x03FF);
                if !cart.chr_ram.is_empty() && idx < cart.chr_ram.len() {
                    cart.chr_ram[idx] = data;
                }
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
        self.mmc3.save_mapper_registers(cart)
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        self.mmc3.load_mapper_registers(cart, state, start)
    }
}
