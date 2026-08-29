use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::vrc2_4::{Vrc2And4, VrcVariant};

pub struct Mapper617 {
    vrc4: Vrc2And4,
    reg: u8,
}

impl Mapper617 {
    pub fn new() -> Self {
        Self {
            vrc4: Vrc2And4::new(VrcVariant::Mapper617),
            reg: 0,
        }
    }

    fn prg_page(&self, address: u16) -> usize {
        let slot = (address as usize - 0x8000) / 0x2000;
        let raw = self.vrc4.prg_slot_bank(slot) as usize;
        let and = if (self.reg & 0x04) != 0 { 0x1Fusize } else { 0x0Fusize };
        let or = ((self.reg as usize) << 4) & !and;
        (raw & and) | or
    }

    fn chr_page(&self, address: u16) -> usize {
        let bank = (address as usize >> 10) & 0x07;
        let raw = self.vrc4.chr_bank_raw(bank) as usize;
        let and = if (self.reg & 0x02) != 0 { 0x7Fusize } else { 0xFFusize };
        let or = ((self.reg as usize) << 7) & !and;
        (raw & and) | or
    }
}

impl Mapper for Mapper617 {
    fn reset(&mut self) {
        self.reg = 0;
        self.vrc4.reset();
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if cart.prg_rom.is_empty() {
                return FetchResult { data: 0, driven: false };
            }
            let page = self.prg_page(address);
            let offset = page * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if (0x6000..0x8000).contains(&address) {
            if !cart.prg_ram.is_empty() {
                let offset = (address - 0x6000) as usize;
                FetchResult {
                    data: cart.prg_ram[offset % cart.prg_ram.len()],
                    driven: true,
                }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) {
            self.vrc4.store_prg(cart, address, data);
        } else if address >= 0x8000 {
            if (address & 0xF000) == 0x9000
                && (address & 0x0F) == 0x0F
                && (self.reg & 0x08) == 0
            {
                self.reg = (address & 0xFF) as u8;
            } else {
                self.vrc4.store_prg(cart, address, data);
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.vrc4.mirror_nametable(cart, address)
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
        let mut new_addr_bus = ppu_address_bus & 0xFF00;

        if address < 0x2000 {
            let bank = self.chr_page(address);
            let offset = bank * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram || chr_rom.is_empty() {
                if !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else {
                    0
                }
            } else {
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = match self.vrc4.nametable_mirroring_value() & 0x3 {
                0 => address & 0x37FF,
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                2 => address & 0x3FFF,
                3 => (address & 0x3FFF) | 0x0400,
                _ => address,
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }

        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let bank = self.chr_page(address);
                let offset = bank * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
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
        self.vrc4
            .ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.vrc4.cpu_clock(cycles)
    }

    fn cpu_clock_irq_level(&self) -> bool {
        self.vrc4.cpu_clock_irq_level()
    }

    fn take_irq_ack(&mut self) -> bool {
        self.vrc4.take_irq_ack()
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = vec![self.reg];
        state.extend(self.vrc4.save_mapper_registers(cart));
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start < state.len() {
            self.reg = state[start];
            start += 1;
        }
        self.vrc4.load_mapper_registers(cart, state, start)
    }
}
