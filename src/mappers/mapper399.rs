// Mapper 399 - BATMAP-000 (MMC3-based with AX5202P)
//
// Reference: NintendulatorNRS-DBG MMC3-based/mapper399.cpp

use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper399 {
    mmc3: MapperMMC3,
    prg: [u8; 2],
    chr: [u8; 2],
    sub_mapper_1: bool,
    irq_clear_pending: bool,
}

impl Mapper399 {
    pub fn new(submapper_id: u8, header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let mut config = Mmc3Config::for_ines(header, 0, chr_size, rom, rom_name);
        config.ax5202p = true;
        Self {
            mmc3: MapperMMC3::new(config),
            prg: [0, 1],
            chr: [0, 1],
            sub_mapper_1: submapper_id == 1,
            irq_clear_pending: false,
        }
    }

    fn write_reg(&mut self, address: u16, data: u8) {
        if (address & 1) != 0 {
            self.prg[(data >> 7) as usize] = data;
        } else {
            self.chr[(data >> 7) as usize] = data;
        }
    }

    fn prg8_mask(cart: &Cartridge) -> u8 {
        let banks = cart.prg_rom.len() / 0x2000;
        if banks == 0 {
            0
        } else {
            (banks - 1) as u8
        }
    }

    fn prg_rom_read(cart: &Cartridge, offset: usize) -> u8 {
        let len = cart.prg_rom.len();
        if len == 0 {
            0
        } else {
            cart.prg_rom[offset % len]
        }
    }
}

impl Mapper for Mapper399 {
    fn reset(&mut self) {
        self.mmc3.reset();
    }

    fn reset_power_cycle(&mut self) {
        self.prg = [0, 1];
        self.chr = [0, 1];
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank8 = ((address - 0x8000) / 0x2000) as usize;
            let bank = if self.sub_mapper_1 {
                match bank8 {
                    0 => self.prg[0] << 1,
                    1 => (self.prg[0] << 1) | 1,
                    2 => self.prg[1],
                    _ => 0xFF,
                }
            } else {
                match bank8 {
                    0 => 0x00,
                    1 => self.prg[0],
                    2 => self.prg[1],
                    _ => 0xFF,
                }
            };
            let mask = Self::prg8_mask(cart);
            let offset = ((bank & mask) as usize) * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: Self::prg_rom_read(cart, offset),
                driven: true,
            }
        } else if address >= 0x6000 {
            if self.sub_mapper_1 {
                let mask = Self::prg8_mask(cart);
                let offset = ((0xFE & mask) as usize) * 0x2000 + (address as usize & 0x1FFF);
                FetchResult {
                    data: Self::prg_rom_read(cart, offset),
                    driven: true,
                }
            } else {
                self.mmc3.fetch_prg(cart, address)
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if self.sub_mapper_1 {
            if address >= 0xE000 {
                self.write_reg(address, data);
            } else if address >= 0x8000 {
                let mmc3_addr = address + 0x2000;
                self.mmc3.store_prg(cart, mmc3_addr, data);
                if (mmc3_addr & 0xE001) == 0xE000 {
                    self.irq_clear_pending = true;
                }
            }
        } else if address >= 0x8000 {
            if address < 0xA000 {
                self.write_reg(address, data);
            } else {
                self.mmc3.store_prg(cart, address, data);
                if (address & 0xE001) == 0xE000 {
                    self.irq_clear_pending = true;
                }
            }
        } else {
            self.mmc3.store_prg(cart, address, data);
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_clear_pending;
        self.irq_clear_pending = false;
        ack
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
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = if address < 0x1000 { self.chr[0] } else { self.chr[1] };
            let offset = (bank as usize) * 0x1000 + (address as usize & 0x0FFF);
            let byte = if !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
            (new_addr_bus as u8, new_addr_bus)
        } else {
            self.mmc3.fetch_ppu(
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
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            let bank = if address < 0x1000 { self.chr[0] } else { self.chr[1] };
            let offset = (bank as usize) * 0x1000 + (address as usize & 0x0FFF);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else {
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
        state.push(self.prg[0]);
        state.push(self.prg[1]);
        state.push(self.chr[0]);
        state.push(self.chr[1]);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        if p < state.len() {
            self.prg[0] = state[p];
            p += 1;
        }
        if p < state.len() {
            self.prg[1] = state[p];
            p += 1;
        }
        if p < state.len() {
            self.chr[0] = state[p];
            p += 1;
        }
        if p < state.len() {
            self.chr[1] = state[p];
            p += 1;
        }
        p
    }
}
