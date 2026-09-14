use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::mmc3_chr_bank;
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper262 {
    mmc3: MapperMMC3,
    reg: u8,
    dip_value: u8,
}

impl Mapper262 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let config =
            Mmc3Config::for_ines(header, 0, if chr_size == 0 { 0 } else { chr_size }, rom, rom_name);
        Self { mmc3: MapperMMC3::new(config), reg: 0, dip_value: 0 }
    }

    fn effective_chr_bank(&self, ppu_addr: u16) -> u16 {
        let bank = mmc3_chr_bank(
            self.mmc3.r8000,
            self.mmc3.chr_2k0,
            self.mmc3.chr_2k8,
            self.mmc3.chr_1k0,
            self.mmc3.chr_1k4,
            self.mmc3.chr_1k8,
            self.mmc3.chr_1kc,
            ppu_addr,
        ) as u16;
        let pair = (ppu_addr >> 11) as usize;
        bank | ((self.reg as u16) << (5 + pair) & 0x100)
    }
}

impl Mapper for Mapper262 {
    fn reset(&mut self) {
        self.reg = 0;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x4000 && address < 0x5000 {
            if (address & 0x100) != 0 {
                return FetchResult { data: self.dip_value, driven: true };
            }
            return FetchResult { data: 0, driven: false };
        }
        self.mmc3.fetch_prg(cart, address)
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x4000 && address < 0x6000 {
            if (address & 0x100) != 0 {
                self.reg = (data & !3) | ((data << 1) & 2) | ((data >> 1) & 1);
            }
            return;
        }
        self.mmc3.store_prg(cart, address, data);
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        address
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            if (self.reg & 0x40) != 0 {
                let offset = (address as usize) & 0x1FFF;
                let byte = if !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else {
                    0
                };
                new_addr_bus |= byte as u16;
            } else {
                let bank = self.effective_chr_bank(address);
                let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
                let byte = if !chr_rom.is_empty() {
                    chr_rom[offset % chr_rom.len()]
                } else {
                    0
                };
                new_addr_bus |= byte as u16;
            }
        } else {
            let byte = if (address & 0x0800) != 0 {
                let idx = (address & 0x7FF) as usize;
                if idx < prg_vram.len() { prg_vram[idx] } else { 0 }
            } else {
                vram[(address & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if (self.reg & 0x40) != 0 && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                let offset = (address as usize) & 0x1FFF;
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            if (address & 0x0800) != 0 {
                let idx = (address & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(address & 0x7FF) as usize] = data;
            }
        }
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
        self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_value
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_value = value;
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.reg);
        state.push(self.dip_value);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let p = self.mmc3.load_mapper_registers(cart, state, start);
        if p < state.len() {
            self.reg = state[p];
            if p + 1 < state.len() {
                self.dip_value = state[p + 1];
                p + 2
            } else {
                p + 1
            }
        } else {
            p
        }
    }
}
