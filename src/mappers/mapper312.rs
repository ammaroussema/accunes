use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper312 {
    prg: u8,
    latch_data: u8,
}

impl Mapper312 {
    pub fn new() -> Self {
        Self { prg: 0, latch_data: 0 }
    }
}

impl Mapper for Mapper312 {
    fn reset(&mut self) {
        self.prg = 0;
        self.latch_data = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = if address < 0xC000 {
                (self.prg as usize) * 0x4000
            } else {
                0xFF * 0x4000
            };
            let offset = bank + (address as usize & 0x3FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len().max(1)],
                driven: true,
            }
        } else if address >= 0x6000 {
            let offset = address as usize & 0x1FFF;
            FetchResult {
                data: if offset < cart.prg_ram.len() { cart.prg_ram[offset] } else { 0 },
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            self.prg = data;
        } else if address >= 0x8000 {
            self.latch_data = data;
        } else if address >= 0x4020 {
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.latch_data & 1 != 0 {
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
            if !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[(address & 0x1FFF) as usize] as u16;
            } else if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[(address & 0x1FFF) as usize] as u16;
            }
        } else {
            let horz = self.latch_data & 1 != 0;
            let mirrored = if horz {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let offset = address as usize & 0x1FFF;
            if offset < cart.chr_ram.len() {
                cart.chr_ram[offset] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.prg, self.latch_data]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.prg = state.get(p).copied().unwrap_or(0);
        p += 1;
        self.latch_data = state.get(p).copied().unwrap_or(0);
        p += 1;
        p
    }
}
