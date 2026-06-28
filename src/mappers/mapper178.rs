use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper178 {
    reg: [u8; 4],
    prg_ram: [u8; 0x8000],
}

impl Mapper178 {
    pub fn new() -> Self {
        Self { reg: [0; 4], prg_ram: [0; 0x8000] }
    }
}

impl Mapper for Mapper178 {
    fn reset(&mut self) {
        self.reg = [0; 4];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            let bank = (self.reg[3] as usize & 0x03) * 0x2000;
            let offset = bank + (address as usize & 0x1FFF);
            FetchResult { data: self.prg_ram[offset % self.prg_ram.len()], driven: true }
        } else if address >= 0x8000 {
            let len = cart.prg_rom.len();
            let sbank = self.reg[1] & 0x07;
            let bbank = self.reg[2] as usize;
            if self.reg[0] & 0x02 != 0 {
                let bank16 = (bbank << 3) | sbank as usize;
                if address < 0xC000 {
                    let offset = bank16 * 0x4000 + (address as usize & 0x3FFF);
                    FetchResult { data: if len > 0 { cart.prg_rom[offset % len] } else { 0 }, driven: true }
                } else {
                    let bank16b = if self.reg[0] & 0x04 != 0 {
                        (bbank << 3) | 6 | (self.reg[1] as usize & 1)
                    } else {
                        (bbank << 3) | 7
                    };
                    let offset = bank16b * 0x4000 + (address as usize & 0x3FFF);
                    FetchResult { data: if len > 0 { cart.prg_rom[offset % len] } else { 0 }, driven: true }
                }
            } else {
                let bank = (bbank << 3) | sbank as usize;
                if self.reg[0] & 0x04 != 0 {
                    let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                    FetchResult { data: if len > 0 { cart.prg_rom[offset % len] } else { 0 }, driven: true }
                } else {
                    let offset = (bank >> 1) * 0x8000 + (address as usize & 0x7FFF);
                    FetchResult { data: if len > 0 { cart.prg_rom[offset % len] } else { 0 }, driven: true }
                }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            let bank = (self.reg[3] as usize & 0x03) * 0x2000;
            let offset = bank + (address as usize & 0x1FFF);
            self.prg_ram[offset % self.prg_ram.len()] = data;
        } else if address >= 0x4800 && address <= 0x4FFF {
            self.reg[(address & 0x03) as usize] = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.reg[0] & 0x01 != 0 {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
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
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[address as usize % chr_ram.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if self.reg[0] & 0x01 != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let chr_len = cart.chr_ram.len();
            if chr_len > 0 {
                cart.chr_ram[address as usize % chr_len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = if self.reg[0] & 0x01 != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut s = Vec::new();
        s.extend_from_slice(&self.reg);
        s.extend_from_slice(&self.prg_ram);
        s
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for b in self.reg.iter_mut() { if p < state.len() { *b = state[p]; p += 1; } }
        for b in self.prg_ram.iter_mut() { if p < state.len() { *b = state[p]; p += 1; } }
        p
    }
}
