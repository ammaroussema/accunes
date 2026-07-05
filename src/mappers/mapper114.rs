use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, Mmc3IrqHack};
const ADDR_LUT: [[u16; 8]; 4] = [
    [0xA001, 0xA000, 0x8000, 0xC000, 0x8001, 0xC001, 0xE000, 0xE001],
    [0xA001, 0x8001, 0x8000, 0xC001, 0xA000, 0xC000, 0xE000, 0xE001],
    [0xC001, 0x8000, 0x8001, 0xA000, 0xA001, 0xE001, 0xE000, 0xC000],
    [0x8000, 0x8001, 0xA000, 0xA001, 0xC000, 0xC001, 0xE000, 0xE001],
];
const DATA_LUT: [[u8; 8]; 4] = [
    [0, 3, 1, 5, 6, 7, 2, 4],
    [0, 2, 5, 3, 6, 1, 7, 4],
    [0, 6, 3, 7, 5, 2, 4, 1],
    [0, 1, 2, 3, 4, 5, 6, 7],
];

pub struct Mapper114 {
    mmc3: MapperMMC3,
    exregs: [u8; 4],
    submapper: usize,
    prg_16k_count: usize,
}

impl Mapper114 {
    pub fn new(prg_16k_count: u8, submapper: u8) -> Self {
        let config = Mmc3Config {
            prg_ram_size: 0,
            chr_ram_size: 0,
            mmc6: false,
            ax5202p: false,
            irq_revision_b: false,
            irq_hack: Mmc3IrqHack::None,
            header_horizontal_mirror: false,
        };
        Self {
            mmc3: MapperMMC3::new(config),
            exregs: [0; 4],
            submapper: (submapper as usize) & 3,
            prg_16k_count: (prg_16k_count as usize).max(1),
        }
    }

    fn prg_16k_offset(&self, address: u16, bank16: usize) -> usize {
        let num_16k = self.prg_16k_count;
        let b = bank16.min(num_16k - 1);
        b * 0x4000 + (address as usize & 0x3FFF)
    }

    fn prg_read_override(&self, cart: &Cartridge, address: u16) -> u8 {
        let prg = (self.exregs[0] & 0x0F) as usize;
        let is_32k = (self.exregs[0] & 0x20) != 0;
        let bank16 = if address >= 0xC000 {
            if is_32k { prg | 1 } else { prg }
        } else {
            if is_32k { prg & !1 } else { prg }
        };
        let offset = self.prg_16k_offset(address, bank16);
        let len = cart.prg_rom.len();
        if len == 0 { 0 } else { cart.prg_rom[offset % len] }
    }
}

impl Mapper for Mapper114 {
    fn reset(&mut self) {
        self.exregs = [0; 4];
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            let idx = (address as usize) & 3;
            if idx == 2 {
                let dip = 0;
                FetchResult { data: dip, driven: false }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else if address >= 0x8000 {
            if (self.exregs[0] & 0x80) != 0 {
                FetchResult {
                    data: self.prg_read_override(cart, address),
                    driven: true,
                }
            } else {
                self.mmc3.fetch_prg(cart, address)
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if self.exregs[1] & 1 == 0 {
                self.exregs[(address as usize) & 3] = data;
            }
            return;
        }
        if address >= 0x8000 {
            let bank = (address >> 12) as usize;
            let idx = (bank & 6) | (address as usize & 1);
            let remapped = ADDR_LUT[self.submapper][idx];
            let mut val = data;
            if remapped == 0x8000 {
                val = (val & 0xC0) | DATA_LUT[self.submapper][(val & 7) as usize];
            }
            self.mmc3.store_prg(cart, remapped, val);
        }
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
        self.mmc3.ppu_clock(
            ppu_address_bus,
            ppu_a12_prev,
            scanline,
            dot,
            ppu_sprite_x16,
            rendering_on,
        )
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.extend_from_slice(&self.exregs);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut idx = self.mmc3.load_mapper_registers(cart, state, start);
        if idx + 4 <= state.len() {
            self.exregs.copy_from_slice(&state[idx..idx + 4]);
            idx += 4;
        }
        idx
    }
}
