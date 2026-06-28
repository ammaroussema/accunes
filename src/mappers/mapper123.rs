use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, Mmc3IrqHack};
const DATA_LUT: [u8; 8] = [0, 3, 1, 5, 6, 7, 2, 4];

pub struct Mapper123 {
    mmc3: MapperMMC3,
    exreg: u8,
    prg_16k_count: usize,
}

impl Mapper123 {
    pub fn new(prg_16k_count: u8) -> Self {
        let config = Mmc3Config {
            prg_ram_size: 0,
            chr_ram_size: 0,
            mmc6: false,
            irq_revision_b: false,
            irq_hack: Mmc3IrqHack::None,
            header_horizontal_mirror: false,
        };
        Self {
            mmc3: MapperMMC3::new(config),
            exreg: 0,
            prg_16k_count: (prg_16k_count as usize).max(1),
        }
    }

    fn prg_16k_offset(&self, address: u16, bank16: usize) -> usize {
        let num_16k = self.prg_16k_count;
        let b = bank16.min(num_16k - 1);
        b * 0x4000 + (address as usize & 0x3FFF)
    }

    fn prg_read_override(&self, cart: &Cartridge, address: u16) -> u8 {
        let prg = (self.exreg & 0x05) | ((self.exreg >> 2) & 0x0A);
        let bank16 = prg as usize;
        let is_32k = (self.exreg & 0x02) != 0;
        let b = if address >= 0xC000 {
            if is_32k { bank16 | 1 } else { bank16 }
        } else {
            if is_32k { bank16 & !1 } else { bank16 }
        };
        let offset = self.prg_16k_offset(address, b);
        let len = cart.prg_rom.len();
        if len == 0 { 0 } else { cart.prg_rom[offset % len] }
    }
}

fn prg_rom_read(cart: &Cartridge, offset: usize) -> u8 {
    let len = cart.prg_rom.len();
    if len == 0 { 0 } else { cart.prg_rom[offset % len] }
}

impl Mapper for Mapper123 {
    fn reset(&mut self) {
        self.exreg = 0;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if (self.exreg & 0x40) != 0 {
                FetchResult {
                    data: self.prg_read_override(cart, address),
                    driven: true,
                }
            } else {
                if address >= 0xE000 {
                    let offset = ((cart.prg_rom.len() / 0x2000).saturating_sub(1)) * 0x2000
                        + (address as usize & 0x1FFF);
                    return FetchResult { data: prg_rom_read(cart, offset), driven: true };
                }
                if address >= 0xC000 {
                    let bank8 = self.mmc3.bank_8c & 0x3F;
                    if (self.mmc3.r8000 & 0x40) != 0 {
                        let offset = (bank8 as usize) * 0x2000 + (address as usize & 0x1FFF);
                        return FetchResult { data: prg_rom_read(cart, offset), driven: true };
                    }
                    let offset = ((cart.prg_rom.len() / 0x2000).saturating_sub(2)) * 0x2000
                        + (address as usize & 0x1FFF);
                    return FetchResult { data: prg_rom_read(cart, offset), driven: true };
                }
                if address >= 0xA000 {
                    let bank8 = self.mmc3.bank_a & 0x3F;
                    let offset = (bank8 as usize) * 0x2000 + (address as usize & 0x1FFF);
                    return FetchResult { data: prg_rom_read(cart, offset), driven: true };
                }
                if address >= 0x8000 {
                    let bank8 = self.mmc3.bank_8c & 0x3F;
                    if (self.mmc3.r8000 & 0x40) == 0 {
                        let offset = (bank8 as usize) * 0x2000 + (address as usize & 0x1FFF);
                        return FetchResult { data: prg_rom_read(cart, offset), driven: true };
                    }
                    let offset = ((cart.prg_rom.len() / 0x2000).saturating_sub(2)) * 0x2000
                        + (address as usize & 0x1FFF);
                    return FetchResult { data: prg_rom_read(cart, offset), driven: true };
                }
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x6000 {
            if (address & 0x0800) != 0 {
                self.exreg = data;
            }
            return;
        }
        if address >= 0x8000 {
            if address < 0xA000 {
                let mut val = data;
                if (address & 1) == 0 {
                    val = (val & 0xC0) | DATA_LUT[(val & 7) as usize];
                }
                self.mmc3.store_prg(cart, address, val);
            } else {
                self.mmc3.store_prg(cart, address, data);
            }
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
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
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
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if self.mmc3.nametable_mirroring() {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() { prg_vram[idx] } else { 0 }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        self.mmc3.store_ppu(cart, address, data, vram);
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
        state.push(self.exreg);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut idx = self.mmc3.load_mapper_registers(cart, state, start);
        if idx < state.len() {
            self.exreg = state[idx];
            idx += 1;
        }
        idx
    }
}
