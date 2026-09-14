use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};
use crate::mappers::mmc3::{mmc3_chr_bank, MapperMMC3, Mmc3Config};

pub struct Mapper618 {
    mmc3: MapperMMC3,
    reg: u8,
}

impl Mapper618 {
    pub fn new() -> Self {
        Self {
            mmc3: MapperMMC3::new(Mmc3Config::embedded()),
            reg: 0,
        }
    }

    fn mmc3_raw_prg_bank(&self, slot: usize) -> usize {
        let swap = (self.mmc3.r8000 & 0x40) != 0;
        match slot {
            0 => if swap { 0x0E } else { self.mmc3.bank_8c as usize },
            1 => self.mmc3.bank_a as usize,
            2 => if swap { self.mmc3.bank_8c as usize } else { 0x0E },
            _ => 0x0F,
        }
    }

    fn mmc3_get_chr_bank(&self, bank: usize) -> usize {
        let b = if (self.mmc3.r8000 & 0x80) != 0 { bank ^ 4 } else { bank };
        if b & 4 != 0 {
            match b {
                4 => self.mmc3.chr_1k0 as usize,
                5 => self.mmc3.chr_1k4 as usize,
                6 => self.mmc3.chr_1k8 as usize,
                _ => self.mmc3.chr_1kc as usize,
            }
        } else {
            let base = if b & 2 != 0 { self.mmc3.chr_2k8 } else { self.mmc3.chr_2k0 } as usize;
            (base & 0xFE) | (b & 1)
        }
    }

    fn prg_offset(&self, cart: &Cartridge, address: u16) -> usize {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }
        let offset = if (self.reg & 0x02) != 0 {
            let slot = (address as usize - 0x8000) / 0x2000;
            let raw = self.mmc3_raw_prg_bank(slot);
            let bank = (raw & 0x0F) | (((self.reg as usize & 0x40) >> 1) | ((self.reg as usize & 0x04) << 2));
            bank * 0x2000 + (address as usize & 0x1FFF)
        } else {
            let bank = ((self.reg as usize >> 3) & 0x08)
                | (self.reg as usize & 0x04)
                | ((self.reg as usize >> 4) & 0x01)
                | (((self.reg as usize) << 1) & 0x02);
            (bank & 0x0F) * 0x8000 + (address as usize & 0x7FFF)
        };
        offset % len
    }

    fn chr_page(&self, address: u16) -> usize {
        if (self.reg & 0x80) != 0 {
            let slot = address as usize >> 11;
            let bank = 0x100 | self.mmc3_get_chr_bank(slot);
            bank * 0x800 + (address as usize & 0x7FF)
        } else {
            let raw = mmc3_chr_bank(
                self.mmc3.r8000,
                self.mmc3.chr_2k0,
                self.mmc3.chr_2k8,
                self.mmc3.chr_1k0,
                self.mmc3.chr_1k4,
                self.mmc3.chr_1k8,
                self.mmc3.chr_1kc,
                address,
            ) as usize;
            let bank = (raw & 0xFF) | ((self.reg as usize & 0x04) << 6);
            bank * 0x400 + (address as usize & 0x3FF)
        }
    }
}

impl Mapper for Mapper618 {
    fn reset(&mut self) {
        self.reg = 0;
        self.mmc3.reset();
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if cart.prg_rom.is_empty() {
                return FetchResult { data: 0, driven: false };
            }
            let offset = self.prg_offset(cart, address);
            FetchResult {
                data: cart.prg_rom[offset],
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
            self.reg = data;
            if !cart.prg_ram.is_empty() {
                let offset = (address - 0x6000) as usize;
                let len = cart.prg_ram.len();
                cart.prg_ram[offset % len] = data;
            }
        } else if address >= 0x8000 {
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
            let offset = self.chr_page(address);
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
            let mirrored = mirror_h_or_v(self.mmc3.nametable_mirroring(), address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }

        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let offset = self.chr_page(address);
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
        self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = vec![self.reg];
        state.extend(self.mmc3.save_mapper_registers(cart));
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start < state.len() {
            self.reg = state[start];
            start += 1;
        }
        self.mmc3.load_mapper_registers(cart, state, start)
    }
}
