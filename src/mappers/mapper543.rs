use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc1::{mmc1_mirror_for_ppu, MapperMMC1, Mmc1Config};

// iNES mapper 543 (CH-501 "5-in-1"): an MMC1B-based multicart (AX5904 MMC1
// clone in SOROM/SNROM configuration) with an extra write-only outer-bank
// register at $5000-$5FFF, clocked serially from bit 3 of the data bus, 4
// bits LSB-first with no shift-register reset. The outer bank extends PRG ROM
// to 2 MiB and selects between two 32 KiB battery-backed PRG-RAM chips, of
// which an 8 KiB window is mapped at $6000. CHR is RAM only (32 KiB on the
// board, 8 KiB used) with 4K banks masked to $7.

pub struct Mapper543 {
    mmc1: MapperMMC1,
    outer_bank: u8,
    shift: u8,
    bits: u8,
}

impl Mapper543 {
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
            543,
            0,
            header[4],
            using_chr_ram,
            has_battery,
        );
        Self {
            mmc1: MapperMMC1::new(config),
            outer_bank: 0,
            shift: 0,
            bits: 0,
        }
    }

    fn mmc1_prg_raw_bank(&self, bank: usize) -> usize {
        let prg = self.mmc1.core.prg as usize;
        let control = self.mmc1.core.control as usize;
        if control & 0x08 != 0 {
            if control & 0x04 != 0 {
                prg | (bank * 0x0F)
            } else {
                prg & (bank * 0x0F)
            }
        } else {
            prg & !1 | bank
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

    fn wram_bank(&self) -> usize {
        let outer = self.outer_bank as usize;
        if outer & 2 != 0 {
            (outer & 1) | ((outer & 4) >> 1) | 4
        } else {
            let chr0 = self.mmc1_chr_raw_bank(0);
            ((chr0 & 8) >> 3) | ((outer & 1) << 1)
        }
    }

    fn wram_open_bus(&self) -> bool {
        (self.mmc1.core.prg & 0x10) != 0
    }

    fn wram_offset(&self, address: u16) -> Option<usize> {
        if self.wram_open_bus() {
            return None;
        }
        Some(self.wram_bank() * 0x2000 + (address as usize & 0x1FFF))
    }

    fn chr_ram_offset(&self, address: u16) -> usize {
        let (bank, mask) = self.mmc1.core.chr_bank_and_mask(address);
        (bank & 0x07) * 0x1000 + (address as usize & mask)
    }
}

impl Mapper for Mapper543 {
    fn reset(&mut self) {
        self.outer_bank = 0;
        self.shift = 0;
        self.bits = 0;
        self.mmc1.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if cart.prg_rom.is_empty() {
                return FetchResult { data: 0, driven: true };
            }
            let bank = if address >= 0xC000 { 1 } else { 0 };
            let raw = self.mmc1_prg_raw_bank(bank);
            let bank16 = (raw & 0x0F) | ((self.outer_bank as usize) << 4);
            let offset = bank16 * 0x4000 + (address as usize & 0x3FFF);
            let len = cart.prg_rom.len();
            FetchResult { data: cart.prg_rom[offset % len], driven: true }
        } else if address >= 0x6000 {
            if let Some(off) = self.wram_offset(address) {
                let len = cart.prg_ram.len();
                if len == 0 {
                    return FetchResult { data: 0, driven: false };
                }
                FetchResult { data: cart.prg_ram[off % len], driven: true }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.mmc1.store_prg(cart, address, data);
        } else if address >= 0x6000 {
            if let Some(off) = self.wram_offset(address) {
                let len = cart.prg_ram.len();
                if len > 0 {
                    cart.prg_ram[off % len] = data;
                }
            }
        } else if address >= 0x5000 {
            if (data & 8) != 0 {
                self.shift |= 1 << self.bits;
            }
            self.bits += 1;
            if self.bits == 4 {
                self.outer_bank = self.shift;
                self.shift = 0;
                self.bits = 0;
            }
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
            let offset = self.chr_ram_offset(address);
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
                let offset = self.chr_ram_offset(address);
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
        state.push(self.shift);
        state.push(self.bits);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut idx = self.mmc1.load_mapper_registers(cart, state, start);
        if idx + 2 < state.len() {
            self.outer_bank = state[idx];
            idx += 1;
            self.shift = state[idx];
            idx += 1;
            self.bits = state[idx];
            idx += 1;
        }
        idx
    }
}
