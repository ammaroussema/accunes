use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper319 {
    prg: u8,
    chr: u8,
    latch_data: u8,
    dip_value: u8,
}

impl Mapper319 {
    pub fn new() -> Self {
        Self { prg: 0, chr: 0, latch_data: 0, dip_value: 0 }
    }
}

impl Mapper for Mapper319 {
    fn reset(&mut self) {
        self.prg = 0;
        self.chr = 0;
        self.latch_data = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x5000 && address < 0x6000 {
            return FetchResult { data: self.dip_value, driven: true };
        }
        if address >= 0x8000 {
            let base = if (self.prg & 0x40) != 0 {
                ((self.prg as usize) >> 3 & 3) * 0x4000
            } else {
                (((self.prg as usize) >> 2 & 6) | ((self.prg as usize) >> 5 & 1)) * 0x4000
            };
            let mask = if (self.prg & 0x40) != 0 { 0x7FFF } else { 0x3FFF };
            let offset = base + (address as usize & mask);
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
        if (0x6000..0x8000).contains(&address) {
            if (address & 4) != 0 {
                self.prg = data;
            } else {
                self.chr = data;
            }
        } else if address >= 0x8000 {
            self.latch_data = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if (self.prg & 0x80) != 0 {
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
            let chr_bank = if (self.chr & 1) == 0 {
                (self.chr as usize) >> 4
            } else {
                ((self.chr as usize) >> 4) & !4 | ((self.latch_data as usize) & 1) << 2
            };
            let offset = chr_bank * 0x2000 + (address as usize & 0x1FFF);
            let byte = if !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let horz = (self.prg & 0x80) != 0;
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
            let chr_bank = if (self.chr & 1) == 0 {
                (self.chr as usize) >> 4
            } else {
                ((self.chr as usize) >> 4) & !4 | ((self.latch_data as usize) & 1) << 2
            };
            let offset = chr_bank * 0x2000 + (address as usize & 0x1FFF);
            let len = cart.chr_ram.len();
            cart.chr_ram[offset % len] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_value
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_value = value;
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.prg, self.chr, self.latch_data, self.dip_value]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.prg = state.get(p).copied().unwrap_or(0); p += 1;
        self.chr = state.get(p).copied().unwrap_or(0); p += 1;
        self.latch_data = state.get(p).copied().unwrap_or(0); p += 1;
        self.dip_value = state.get(p).copied().unwrap_or(0); p += 1;
        p
    }
}
