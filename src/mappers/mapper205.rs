use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::MapperMMC3;
use crate::mappers::mmc3::Mmc3Config;

pub struct Mapper205 {
    mmc3: MapperMMC3,
    reg: u8,
    solder_pad: u8,
}

impl Mapper205 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = header[5];
        let cfg = Mmc3Config::for_ines(header, 0, chr_size, rom, rom_name);
        Self {
            mmc3: MapperMMC3::new(cfg),
            reg: 0,
            solder_pad: 2,
        }
    }
}

impl Mapper for Mapper205 {
    fn reset(&mut self) {
        self.reg = 0;
        self.solder_pad ^= 2;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            return self.mmc3.fetch_prg(cart, address);
        }
        let prg_len = cart.prg_rom.len();
        if prg_len == 0 {
            return FetchResult { data: 0, driven: false };
        }
        let last = prg_len / 0x2000 - 1;
        let second_last = last.saturating_sub(1);
        let swap = (self.mmc3.r8000 & 0x40) != 0;
        let raw_bank = if address < 0xA000 {
            if swap { second_last } else { self.mmc3.bank_8c as usize }
        } else if address < 0xC000 {
            self.mmc3.bank_a as usize
        } else if address < 0xE000 {
            if swap { self.mmc3.bank_8c as usize } else { second_last }
        } else {
            last
        };
        let prg_and = if self.reg & 0x02 != 0 { 0x0F } else { 0x1F };
        let bank = ((self.reg as usize) << 4) | (raw_bank & prg_and);
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        FetchResult { data: cart.prg_rom[offset % prg_len], driven: true }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            self.reg = data & 3;
            if data & 1 != 0 {
                self.reg |= self.solder_pad;
            }
            self.mmc3.store_prg(cart, address, data);
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
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        if address >= 0x2000 {
            return self.mmc3.fetch_ppu(
                _prg_rom, chr_rom, _prg_ram, chr_ram, _prg_vram,
                using_chr_ram, _nametable_horizontal_mirroring,
                _alternative_nametable_arrangement,
                ppu_address_bus, ppu_octal_latch, vram,
            );
        }
        let raw_bank = self.mmc3.chr_bank(address);
        let chr_and = if self.reg & 0x02 != 0 { 0x7F_u16 } else { 0xFF_u16 };
        let bank = ((self.reg as u16) << 7) | (raw_bank as u16 & chr_and);
        let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
        let byte = if using_chr_ram && !chr_ram.is_empty() {
            chr_ram[offset % chr_ram.len()]
        } else if !chr_rom.is_empty() {
            chr_rom[offset % chr_rom.len()]
        } else { 0 };
        let new_addr_bus = (ppu_address_bus & 0xFF00) | byte as u16;
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let raw_bank = self.mmc3.chr_bank(address);
                let chr_and = if self.reg & 0x02 != 0 { 0x7F_u16 } else { 0xFF_u16 };
                let bank = ((self.reg as u16) << 7) | (raw_bank as u16 & chr_and);
                let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            self.mmc3.store_ppu(cart, address, data, vram);
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
        state.push(self.reg);
        state.push(self.solder_pad);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let p = self.mmc3.load_mapper_registers(cart, state, start);
        if p + 2 <= state.len() {
            self.reg = state[p];
            self.solder_pad = state[p + 1];
            p + 2
        } else { p }
    }
}
