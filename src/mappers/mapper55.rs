use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper55 {
    prg_ram: [u8; 0x1000], 
}

impl Mapper55 {
    pub fn new() -> Self {
        Self {
            prg_ram: [0; 0x1000],
        }
    }
}

impl Mapper for Mapper55 {
    fn reset(&mut self) {
        self.prg_ram = [0; 0x1000];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let offset = (address as usize - 0x8000) % cart.prg_rom.len();
            FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            }
        } else if address >= 0x7000 && address < 0x8000 {
            let offset = (address as usize - 0x7000) & 0x07FF;
            FetchResult {
                data: self.prg_ram[offset],
                driven: true,
            }
        } else if address >= 0x6000 && address < 0x7000 {
            let prg_len = cart.prg_rom.len();
            if prg_len >= 0x8800 {
                let offset = 0x8000 + (address as usize & 0x07FF);
                FetchResult {
                    data: cart.prg_rom[offset],
                    driven: true,
                }
            } else {
                let offset = (address as usize & 0x07FF) % prg_len;
                FetchResult {
                    data: cart.prg_rom[offset],
                    driven: true,
                }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x7000 && address < 0x8000 {
            let offset = (address as usize - 0x7000) & 0x07FF;
            self.prg_ram[offset] = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.nametable_horizontal_mirroring {
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
            if using_chr_ram {
                let len = chr_ram.len();
                if len > 0 {
                    new_addr_bus |= chr_ram[(address as usize & 0x1FFF) % len] as u16;
                }
            } else {
                let len = chr_rom.len();
                if len > 0 {
                    new_addr_bus |= chr_rom[(address as usize & 0x1FFF) % len] as u16;
                }
            }
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

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        self.prg_ram.to_vec()
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        let len = self.prg_ram.len();
        if start + len <= state.len() {
            self.prg_ram.copy_from_slice(&state[start..start + len]);
            start += len;
        }
        start
    }
}
