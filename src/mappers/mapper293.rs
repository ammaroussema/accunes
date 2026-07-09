use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper293 {
    reg1: u8,
    reg2: u8,
}

impl Mapper293 {
    pub fn new() -> Self {
        Self { reg1: 0, reg2: 0 }
    }

    fn sync(&self, cart: &Cartridge, address: u16) -> usize {
        let len = cart.prg_rom.len();
        if len == 0 { return 0; }
        let mode = ((self.reg1 & 0x08) != 0) as usize * 2 | ((self.reg2 & 0x40) != 0) as usize * 1;
        let outer = ((self.reg2 & 0x01) as usize * 0x20)
            | ((self.reg2 & 0x20) as usize * 0x10)
            | ((self.reg2 & 0x10) as usize * 0x08);
        let inner = (self.reg1 & 0x07) as usize;
        let bank16 = match mode {
            0 => {
                if address < 0xC000 { outer | inner }
                else { outer | 7 }
            }
            1 => {
                if address < 0xC000 { outer | (inner & !1) }
                else { outer | 7 }
            }
            2 => outer | inner,
            3 => (outer | inner) >> 1,
            _ => 0,
        };
        let offset = if mode == 3 {
            bank16 * 0x8000 + (address as usize & 0x7FFF)
        } else {
            bank16 * 0x4000 + (address as usize & 0x3FFF)
        };
        offset % len
    }
}

impl Mapper for Mapper293 {
    fn reset(&mut self) {
        self.reg1 = 0;
        self.reg2 = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let offset = self.sync(cart, address);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let bank = (address >> 12) as u8;
            if bank & 2 == 0 { self.reg1 = data; }
            if bank & 4 == 0 { self.reg2 = data; }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if (self.reg2 & 0x80) != 0 {
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
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[(address as usize) % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[(address as usize) % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if (self.reg2 & 0x80) != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize) % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.reg1, self.reg2]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.reg1 = state[p]; p += 1; }
        if p < state.len() { self.reg2 = state[p]; p += 1; }
        p
    }
}
