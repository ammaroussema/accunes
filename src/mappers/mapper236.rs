use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper236 {
    reg: [u8; 2],
    has_chr_rom: bool,
}

impl Mapper236 {
    pub fn new(has_chr_rom: bool) -> Self {
        Self { reg: [0; 2], has_chr_rom }
    }

    fn mode(&self) -> u8 {
        (self.reg[1] >> 4) & 3
    }

    fn prg_bank(&self) -> usize {
        if self.has_chr_rom {
            (self.reg[1] & 0x0F) as usize
        } else {
            ((self.reg[1] & 0x07) | (self.reg[0] << 3)) as usize
        }
    }

    fn chr_bank(&self) -> usize {
        if self.has_chr_rom {
            (self.reg[0] & 0x0F) as usize
        } else {
            0
        }
    }
}

impl Mapper for Mapper236 {
    fn reset(&mut self) {
        self.reg = [0; 2];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let prg = self.prg_bank();
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult { data: 0, driven: false };
            }
            let offset = match self.mode() {
                0 | 1 => {
                    if address < 0xC000 {
                        prg * 0x4000 + (address as usize & 0x3FFF)
                    } else {
                        (prg | 7) * 0x4000 + (address as usize & 0x3FFF)
                    }
                }
                2 => {
                    (prg >> 1) * 0x8000 + (address as usize - 0x8000)
                }
                _ => {
                    prg * 0x4000 + (address as usize & 0x3FFF)
                }
            };
            FetchResult { data: cart.prg_rom[offset % len], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, _data: u8) {
        if address >= 0x8000 {
            self.reg[(address as usize >> 14) & 1] = address as u8;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.reg[0] & 0x20 != 0 {
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
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let chr = self.chr_bank();
            let offset = chr * 0x2000 + (address as usize & 0x1FFF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if self.reg[0] & 0x20 != 0 {
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
            let len = cart.chr_ram.len();
            if len > 0 {
                let chr = self.chr_bank();
                let offset = chr * 0x2000 + (address as usize & 0x1FFF);
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.reg[0], self.reg[1]]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.reg[0] = state[p]; p += 1; }
        if p < state.len() { self.reg[1] = state[p]; p += 1; }
        p
    }
}
