use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::MapperMMC3;
use crate::mappers::mmc3::Mmc3Config;
const PROT_LUT: [u8; 256] = [
    0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x49, 0x19, 0x09, 0x59, 0x49, 0x19, 0x09,
    0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x51, 0x41, 0x11, 0x01, 0x51, 0x41, 0x11, 0x01,
    0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x49, 0x19, 0x09, 0x59, 0x49, 0x19, 0x09,
    0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x51, 0x41, 0x11, 0x01, 0x51, 0x41, 0x11, 0x01,
    0x00, 0x10, 0x40, 0x50, 0x00, 0x10, 0x40, 0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x08, 0x18, 0x48, 0x58, 0x08, 0x18, 0x48, 0x58, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x10, 0x40, 0x50, 0x00, 0x10, 0x40, 0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x08, 0x18, 0x48, 0x58, 0x08, 0x18, 0x48, 0x58, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x58, 0x48, 0x18, 0x08, 0x58, 0x48, 0x18, 0x08,
    0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x50, 0x40, 0x10, 0x00, 0x50, 0x40, 0x10, 0x00,
    0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x58, 0x48, 0x18, 0x08, 0x58, 0x48, 0x18, 0x08,
    0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x50, 0x40, 0x10, 0x00, 0x50, 0x40, 0x10, 0x00,
    0x01, 0x11, 0x41, 0x51, 0x01, 0x11, 0x41, 0x51, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x09, 0x19, 0x49, 0x59, 0x09, 0x19, 0x49, 0x59, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x01, 0x11, 0x41, 0x51, 0x01, 0x11, 0x41, 0x51, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x09, 0x19, 0x49, 0x59, 0x09, 0x19, 0x49, 0x59, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

pub struct Mapper208 {
    mmc3: MapperMMC3,
    reg: u8,
    prot_index: u8,
    prot_data: [u8; 4],
}

impl Mapper208 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = header[5];
        let cfg = Mmc3Config::for_ines(header, 0, chr_size, rom, rom_name);
        Self {
            mmc3: MapperMMC3::new(cfg),
            reg: 3,
            prot_index: 0,
            prot_data: [0; 4],
        }
    }
}

impl Mapper for Mapper208 {
    fn reset(&mut self) {
        self.reg = 3;
        self.prot_index = 0;
        self.prot_data = [0; 4];
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x5800 && address < 0x6000 {
            return FetchResult {
                data: self.prot_data[(address as usize) & 3],
                driven: true,
            };
        }
        if address < 0x8000 {
            return self.mmc3.fetch_prg(cart, address);
        }
        let prg_len = cart.prg_rom.len();
        if prg_len == 0 {
            return FetchResult { data: 0, driven: false };
        }
        let bank = (self.reg as usize) & ((prg_len / 0x8000).max(1) - 1);
        let offset = bank * 0x8000 + (address as usize & 0x7FFF);
        FetchResult { data: cart.prg_rom[offset % prg_len], driven: true }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x4800 && address < 0x5000 || address >= 0x6800 && address < 0x7000 {
            self.reg = (data & 1) | ((data >> 3) & 2);
            return;
        }
        if address >= 0x5000 && address < 0x6000 {
            if address < 0x5800 {
                self.prot_index = data;
            } else {
                self.prot_data[(address as usize) & 3] = data ^ PROT_LUT[self.prot_index as usize];
            }
            return;
        }
        self.mmc3.store_prg(cart, address, data);
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let h = (self.reg & 0x20) != 0;
        if h {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
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
        if address < 0x2000 {
            return self.mmc3.fetch_ppu(
                _prg_rom, chr_rom, _prg_ram, chr_ram, _prg_vram,
                using_chr_ram, _nametable_horizontal_mirroring,
                _alternative_nametable_arrangement,
                ppu_address_bus, ppu_octal_latch, vram,
            );
        }
        let h = (self.reg & 0x20) != 0;
        let mirrored = if h {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        };
        let byte = vram[(mirrored & 0x7FF) as usize];
        let new_addr_bus = (ppu_address_bus & 0xFF00) | byte as u16;
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            self.mmc3.store_ppu(cart, address, data, vram);
        } else if address >= 0x2000 && address < 0x3F00 {
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

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.reg);
        state.push(self.prot_index);
        state.extend_from_slice(&self.prot_data);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        if p < state.len() {
            self.reg = state[p];
            p += 1;
        }
        if p < state.len() {
            self.prot_index = state[p];
            p += 1;
        }
        if p + 4 <= state.len() {
            self.prot_data.copy_from_slice(&state[p..p + 4]);
            p += 4;
        }
        p
    }
}
