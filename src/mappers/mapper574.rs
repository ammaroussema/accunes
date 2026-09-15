use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper574 {
    addr: u16,
    data: u8,
    addr_locked: u16,
    data_locked: u8,
}

impl Mapper574 {
    pub fn new() -> Self {
        Self {
            addr: 0,
            data: 0,
            addr_locked: 0,
            data_locked: 0,
        }
    }

    fn sync(&mut self) {
        if self.addr & 0x80 != 0 {
            self.addr_locked = 0x00FC;
        } else {
            self.addr_locked = 0x0000;
        }
        self.data_locked = 0x00;
    }

    fn prg32_page(&self) -> u16 {
        if self.addr & 0x40 != 0 {
            ((self.addr >> 2) & !0x07) as u16 | (self.data as u16 & 0x07)
        } else if self.addr & 0x20 != 0 {
            ((self.addr >> 2) & !0x03) as u16 | (self.data as u16 & 0x03)
        } else {
            ((self.addr >> 2) & !0x03) as u16 | (self.addr as u16 & 0x03)
        }
    }
}

impl Mapper for Mapper574 {
    fn reset(&mut self) {
        self.addr = 0;
        self.data = 0;
        self.addr_locked = 0;
        self.data_locked = 0;
        self.sync();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }
        if cart.prg_rom.is_empty() {
            return FetchResult { data: 0, driven: true };
        }
        let page = self.prg32_page();
        let offset = (page as usize) * 0x8000 + (address as usize & 0x7FFF);
        FetchResult {
            data: cart.prg_rom[offset % cart.prg_rom.len()],
            driven: true,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let new_addr = (self.addr & self.addr_locked) | (address & !self.addr_locked);
            let new_data = (self.data & self.data_locked) | (data & !self.data_locked);
            self.addr = new_addr;
            self.data = new_data;
            self.sync();
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.addr & 0x60 != 0 {
            if self.data & 0x10 != 0 {
                (address & 0x33FF) | 0x0800
            } else {
                address & 0x33FF
            }
        } else if self.data & 0x10 != 0 {
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
        prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
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
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if nametable_horizontal_mirroring {
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

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, _vram: &mut [u8]) {
        if address < 0x2000 && !cart.chr_ram.is_empty() {
            let offset = (address as usize) % cart.chr_ram.len();
            cart.chr_ram[offset] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.addr.to_le_bytes());
        state.push(self.data);
        state.extend_from_slice(&self.addr_locked.to_le_bytes());
        state.push(self.data_locked);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.addr = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.data = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.addr_locked = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.data_locked = state[p];
            p += 1;
        }
        self.sync();
        p
    }
}
