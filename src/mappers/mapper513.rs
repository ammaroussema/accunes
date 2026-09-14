use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};
use crate::mappers::mmc3::{mmc3_chr_bank, MapperMMC3, Mmc3Config};

pub struct Mapper513 {
    mmc3: MapperMMC3,
    irq_ack: bool,
}

impl Mapper513 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let config = Mmc3Config {
            ax5202p: true,
            ..Mmc3Config::for_ines(header, 0, 0, rom, rom_name)
        };
        Self {
            mmc3: MapperMMC3::new(config),
            irq_ack: false,
        }
    }

    fn prg_bank(&self, cart: &Cartridge, page: usize) -> usize {
        let num_8k = cart.prg_rom.len() / 0x2000;
        let last = if num_8k > 0 { num_8k - 1 } else { 0 };
        let second_last = if last > 0 { last - 1 } else { 0 };
        let swapped = (self.mmc3.r8000 & 0x40) != 0;

        let (raw, is_fixed) = match (page, swapped) {
            (0, false) => (self.mmc3.bank_8c as usize, false),
            (0, true)  => (second_last,                 true),
            (1, _)     => (self.mmc3.bank_a as usize,   false),
            (2, false) => (second_last,                 true),
            (2, true)  => (self.mmc3.bank_8c as usize,  false),
            (3, _)     => (last,                        true),
            _          => (0,                           true),
        };

        if is_fixed {
            raw & 0x3F
        } else {
            (raw & 0x3F) | (self.mmc3.r8000 as usize & 0xC0)
        }
    }
}

impl Mapper for Mapper513 {
    fn reset(&mut self) {
        self.irq_ack = false;
        self.mmc3.reset();
    }

    fn reset_power_cycle(&mut self) {
        self.irq_ack = false;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let page = (address as usize - 0x8000) / 0x2000;
            let bank = self.prg_bank(cart, page);
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            };
        }
        if address >= 0x6000 {
            return self.mmc3.fetch_prg(cart, address);
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (address & 0xE001) == 0xE000 {
            self.irq_ack = true;
        }
        self.mmc3.store_prg(cart, address, data);
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        let mmc3_h = self.mmc3.nametable_mirroring();
        if cart.alternative_nametable_arrangement {
            address
        } else {
            mirror_h_or_v(mmc3_h, address)
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
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
            let chr_bank = mmc3_chr_bank(
                self.mmc3.r8000,
                self.mmc3.chr_2k0,
                self.mmc3.chr_2k8,
                self.mmc3.chr_1k0,
                self.mmc3.chr_1k4,
                self.mmc3.chr_1k8,
                self.mmc3.chr_1kc,
                address,
            ) as usize;
            let bank = chr_bank & 0x3F;
            let offset = bank * 0x0400 + (address as usize & 0x03FF);
            let byte = if !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            return self.mmc3.fetch_ppu(
                _prg_rom,
                _chr_rom,
                _prg_ram,
                chr_ram,
                _prg_vram,
                true,
                _nametable_horizontal_mirroring,
                alternative_nametable_arrangement,
                ppu_address_bus,
                ppu_octal_latch,
                vram,
            );
        }

        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && !cart.chr_ram.is_empty() {
            let chr_bank = mmc3_chr_bank(
                self.mmc3.r8000,
                self.mmc3.chr_2k0,
                self.mmc3.chr_2k8,
                self.mmc3.chr_1k0,
                self.mmc3.chr_1k4,
                self.mmc3.chr_1k8,
                self.mmc3.chr_1kc,
                address,
            ) as usize;
            let bank = chr_bank & 0x3F;
            let offset = bank * 0x0400 + (address as usize & 0x03FF);
            let len = cart.chr_ram.len();
            cart.chr_ram[offset % len] = data;
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
        state.push(if self.irq_ack { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        if p < state.len() {
            self.irq_ack = state[p] != 0;
            p += 1;
        }
        p
    }
}
