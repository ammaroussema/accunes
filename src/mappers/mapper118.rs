use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper118 {
    mmc3: MapperMMC3,
    tk_nametables: [u8; 4],
}

impl Mapper118 {
    pub fn new(config: Mmc3Config) -> Self {
        Self {
            mmc3: MapperMMC3::new(config),
            tk_nametables: [0; 4],
        }
    }

    fn vram_addr(&self, address: u16) -> u16 {
        let nt = ((address >> 10) & 3) as usize;
        if self.tk_nametables[nt] == 0 {
            (address & 0x03FF) | 0x2000
        } else {
            (address & 0x03FF) | 0x2400
        }
    }
}

impl Mapper for Mapper118 {
    fn reset(&mut self) {
        self.tk_nametables = [0; 4];
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        self.mmc3.fetch_prg(cart, address)
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            match address & 0xE001 {
                0x8001 => {
                    let bit7 = ((data >> 7) & 1) as u8;
                    let invert = (self.mmc3.r8000 & 0x80) != 0;
                    let reg = self.mmc3.r8000 & 0x07;
                    if !invert {
                        match reg {
                            0 => { self.tk_nametables[0] = bit7; self.tk_nametables[1] = bit7; }
                            1 => { self.tk_nametables[2] = bit7; self.tk_nametables[3] = bit7; }
                            _ => {}
                        }
                    } else {
                        match reg {
                            2 => self.tk_nametables[0] = bit7,
                            3 => self.tk_nametables[1] = bit7,
                            4 => self.tk_nametables[2] = bit7,
                            5 => self.tk_nametables[3] = bit7,
                            _ => {}
                        }
                    }
                }
                0xA000 => {
                    return;
                }
                _ => {}
            }
        }
        self.mmc3.store_prg(cart, address, data);
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.vram_addr(address)
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
            let bank = self.mmc3.chr_bank(address);
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
            let mirrored = self.vram_addr(address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let bank = self.mmc3.chr_bank(address);
            let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.vram_addr(address);
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
        self.mmc3.ppu_clock(
            ppu_address_bus, ppu_a12_prev, scanline, dot,
            ppu_sprite_x16, rendering_on,
        )
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.extend_from_slice(&self.tk_nametables);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut idx = self.mmc3.load_mapper_registers(cart, state, start);
        if idx + 4 <= state.len() {
            self.tk_nametables.copy_from_slice(&state[idx..idx + 4]);
            idx += 4;
        }
        idx
    }
}
