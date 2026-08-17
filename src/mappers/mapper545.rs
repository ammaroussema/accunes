use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{Mmc3Config, MapperMMC3};

pub struct Mapper545 {
    mmc3: MapperMMC3,
    reg: u8,
    irq_ack: bool,
}

impl Mapper545 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let config = Mmc3Config {
            ax5202p: true,
            ..Mmc3Config::for_ines(header, 0, 0, rom, rom_name)
        };
        Self {
            mmc3: MapperMMC3::new(config),
            reg: 0,
            irq_ack: false,
        }
    }

    fn chr_or(&self) -> u8 {
        ((self.reg & 1) << 7) | if (self.reg & 4) == 0 { 0x40 } else { 0 }
    }

    fn prg_bank(&self, base: u8) -> u8 {
        let result = if (self.reg & 0x08) != 0 && (base & 0x10) == 0 {
            0x40 | (base & 0x0F)
        } else {
            ((self.reg & 0x03) << 4) | (base & 0x0F)
        };
        result & 0x7F
    }

    fn prg_base(&self, slot: usize) -> u8 {
        let high = (self.mmc3.r8000 & 0x40) != 0;
        match slot {
            0 => {
                if high {
                    0xFE
                } else {
                    self.mmc3.bank_8c
                }
            }
            1 => self.mmc3.bank_a,
            2 => {
                if high {
                    self.mmc3.bank_8c
                } else {
                    0xFE
                }
            }
            _ => 0xFF,
        }
    }
}

impl Mapper for Mapper545 {
    fn reset(&mut self) {
        self.reg = 0;
        self.irq_ack = false;
        self.mmc3.reset();
    }

    fn reset_power_cycle(&mut self) {
        self.reg = 0;
        self.irq_ack = false;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            return self.mmc3.fetch_prg(cart, address);
        }
        let len = cart.prg_rom.len();
        if len == 0 {
            return FetchResult {
                data: 0,
                driven: true,
            };
        }
        let slot = ((address - 0x8000) / 0x2000) as usize;
        let bank = self.prg_bank(self.prg_base(slot)) as usize;
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        FetchResult {
            data: cart.prg_rom[offset % len],
            driven: true,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 && (address & 0xF120) == 0xF120 {
            self.reg = data;
        } else {
            if (address & 0xE001) == 0xE000 {
                self.irq_ack = true;
            }
            self.mmc3.store_prg(cart, address, data);
        }
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
        prg_vram: &[u8],
        using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = (self.mmc3.chr_bank(address) & 0x7F) | self.chr_or();
            let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
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
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() {
                    prg_vram[idx]
                } else {
                    0
                }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let bank = (self.mmc3.chr_bank(address) & 0x7F) | self.chr_or();
                let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mmc3.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
            }
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
        self.mmc3
            .ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack;
        self.irq_ack = false;
        ack
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.mmc3.cpu_clock(cycles)
    }

    fn get_dip_switches(&self) -> u8 {
        self.mmc3.get_dip_switches()
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.mmc3.set_dip_switches(value);
    }


    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.reg);
        state.push(if self.irq_ack { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        if p < state.len() {
            self.reg = state[p];
            p += 1;
        }
        if p < state.len() {
            self.irq_ack = state[p] != 0;
            p += 1;
        }
        p
    }
}

