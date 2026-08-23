use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::MapperMMC3;
use crate::mappers::mmc3::Mmc3Config;

pub struct Mapper593 {
    mmc3: MapperMMC3,
    reg: [u8; 8],
}

impl Mapper593 {
    pub fn new() -> Self {
        let mut config = Mmc3Config::embedded();
        config.ax5202p = true;
        Self {
            mmc3: MapperMMC3::new(config),
            reg: [0; 8],
        }
    }

    fn second_last_prg_bank(&self, cart: &Cartridge) -> usize {
        let num = cart.prg_rom.len() / 0x2000;
        if num < 2 { 0 } else { num - 2 }
    }

    fn last_prg_bank(&self, cart: &Cartridge) -> usize {
        let num = cart.prg_rom.len() / 0x2000;
        if num == 0 { 0 } else { num - 1 }
    }
}

impl Mapper for Mapper593 {
    fn reset(&mut self) {
        self.reg = [0; 8];
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = match (address >> 13) & 3 {
                0 => self.reg[2] as usize,
                1 => self.reg[3] as usize,
                2 => self.second_last_prg_bank(cart),
                _ => self.last_prg_bank(cart),
            };
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            if cart.prg_rom.is_empty() {
                return FetchResult { data: 0, driven: true };
            }
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else {
            self.mmc3.fetch_prg(cart, address)
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let bank = ((address >> 13) & 7) as usize;
            self.reg[bank] = data;
            if bank == 6 {
                self.mmc3.store_prg(cart, 0xC000, data.wrapping_sub(1));
                self.mmc3.store_prg(cart, 0xC001, data);
                self.mmc3.store_prg(cart, 0xE000, data);
                let addr = if data != 0 { 0xE001 } else { 0xE000 };
                self.mmc3.store_prg(cart, addr, data);
            }
            if bank == 7 {
                self.mmc3.set_nametable_horizontal((data & 0x01) != 0);
            }
        } else if address >= 0x6000 {
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
            let chr_bank = if address < 0x1000 {
                self.reg[0] as usize
            } else {
                self.reg[1] as usize
            };
            let offset = chr_bank * 0x1000 + (address as usize & 0x0FFF);
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
                let chr_bank = if address < 0x1000 {
                    self.reg[0] as usize
                } else {
                    self.reg[1] as usize
                };
                let offset = chr_bank * 0x1000 + (address as usize & 0x0FFF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
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
        self.mmc3.take_irq_ack()
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.extend_from_slice(&self.reg);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        for i in 0..8 {
            if p < state.len() {
                self.reg[i] = state[p];
                p += 1;
            }
        }
        p
    }
}
