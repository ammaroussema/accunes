use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc1::{mmc1_mirror_for_ppu, MapperMMC1, Mmc1Config, Mmc1Variant};
pub struct Mapper499 {
    mmc1: MapperMMC1,
    outer_bank: u8,
}
impl Mapper499 {
    pub fn new(
        header: &[u8],
        rom: &[u8],
        _rom_name: &str,
        using_chr_ram: bool,
        has_battery: bool,
    ) -> Self {
        let config = Mmc1Config::for_ines(
            header,
            rom,
            1,
            0,
            header[4],
            using_chr_ram,
            has_battery,
        );
        Self {
            mmc1: MapperMMC1::new(config),
            outer_bank: 0,
        }
    }
    fn mmc1_prg_raw_bank(&self, bank: usize) -> usize {
        let prg = self.mmc1.core.prg as usize;
        let control = self.mmc1.core.control as usize;
        let result = if control & 0x08 != 0 {
            if control & 0x04 != 0 {
                prg | (bank * 0x0F)
            } else {
                prg & (bank * 0x0F)
            }
        } else {
            prg & !1 | bank
        };
        if (prg & 0x10) != 0 && self.mmc1.core.config.variant == Mmc1Variant::Mmc1A {
            (result & 0x07) | (prg & 0x08)
        } else {
            result & 0x0F
        }
    }
    fn mmc1_chr_raw_bank(&self, slot: usize) -> usize {
        let control = self.mmc1.core.control;
        if (control & 0x10) != 0 {
            match slot {
                0 => self.mmc1.core.chr0 as usize,
                _ => self.mmc1.core.chr1 as usize,
            }
        } else {
            (self.mmc1.core.chr0 as usize & !1) | slot
        }
    }
    fn prg_16k_bank(&self, bank: usize) -> usize {
        let raw = self.mmc1_prg_raw_bank(bank);
        (raw & 0x07) | ((self.outer_bank as usize) << 3)
    }
    fn chr_4k_bank(&self, slot: usize) -> usize {
        let raw = self.mmc1_chr_raw_bank(slot);
        (raw & 0x1F) | ((self.outer_bank as usize) << 5)
    }
}
impl Mapper for Mapper499 {
    fn reset(&mut self) {
        self.outer_bank = 0;
        self.mmc1.reset();
    }
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if cart.prg_rom.is_empty() {
                return FetchResult { data: 0, driven: true };
            }
            let bank = if address >= 0xC000 { 1 } else { 0 };
            let effective = self.prg_16k_bank(bank);
            let len = cart.prg_rom.len();
            let offset = (effective * 0x4000 + (address as usize & 0x3FFF)) % len;
            FetchResult { data: cart.prg_rom[offset], driven: true }
        } else {
            self.mmc1.fetch_prg(cart, address)
        }
    }
    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            if (self.outer_bank & 0x08) == 0 {
                self.outer_bank = (address & 0xFF) as u8;
            }
        } else {
            self.mmc1.store_prg(cart, address, data);
        }
    }
    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mmc1.mirror_nametable(cart, address)
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
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let slot = (address >> 12) as usize;
            let bank = self.chr_4k_bank(slot);
            let offset = bank * 0x1000 + (address as usize & 0x0FFF);
            let byte = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[offset % chr_ram.len()] }
            } else {
                if chr_rom.is_empty() { 0 } else { chr_rom[offset % chr_rom.len()] }
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mmc1_mirror_for_ppu(&self.mmc1.core, nametable_horizontal_mirroring, address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }
    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let slot = (address >> 12) as usize;
                let bank = self.chr_4k_bank(slot);
                let offset = bank * 0x1000 + (address as usize & 0x0FFF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }
    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc1.cpu_clock_rise(ppu_address_bus)
    }
    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.mmc1.cpu_clock(cycles)
    }
    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc1.save_mapper_registers(cart);
        state.push(self.outer_bank);
        state
    }
    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut idx = self.mmc1.load_mapper_registers(cart, state, start);
        if idx < state.len() {
            self.outer_bank = state[idx];
            idx += 1;
        }
        idx
    }
}
