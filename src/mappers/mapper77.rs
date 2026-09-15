use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper77 {
    latch: u8,
}

impl Mapper77 {
    pub fn new() -> Self {
        Self { latch: 0 }
    }
}

impl Mapper for Mapper77 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = (self.latch & 7) as usize;
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.latch = data;
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
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            if address < 0x0800 {
                let bank = (self.latch >> 4) as usize;
                let offset = bank * 0x0800 + (address as usize & 0x07FF);
                if !chr_rom.is_empty() {
                    new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
                } else if !chr_ram.is_empty() {
                    new_addr_bus |= chr_ram[offset % chr_ram.len()] as u16;
                }
            } else if address < 0x1000 {
                let offset = 0x1000 + (address as usize & 0x07FF);
                if !chr_ram.is_empty() {
                    new_addr_bus |= chr_ram[offset % chr_ram.len()] as u16;
                }
            } else {
                let offset = 0x0000 + (address as usize & 0x0FFF);
                if !chr_ram.is_empty() {
                    new_addr_bus |= chr_ram[offset % chr_ram.len()] as u16;
                }
            }
        } else {
            let mirrored = if !nametable_horizontal_mirroring {
                address & 0x37FF
            } else {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            let chr_ram_len = cart.chr_ram.len();
            if address < 0x0800 {
                if cart.using_chr_ram && chr_ram_len > 0 {
                    let bank = (self.latch >> 4) as usize;
                    let offset = bank * 0x0800 + (address as usize & 0x07FF);
                    if cart.chr_rom.is_empty() {
                        cart.chr_ram[offset % chr_ram_len] = data;
                    }
                }
            } else if address < 0x1000 {
                if chr_ram_len > 0 {
                    let offset = 0x1000 + (address as usize & 0x07FF);
                    cart.chr_ram[offset % chr_ram_len] = data;
                }
            } else {
                if chr_ram_len > 0 {
                    let offset = 0x0000 + (address as usize & 0x0FFF);
                    cart.chr_ram[offset % chr_ram_len] = data;
                }
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

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.latch);
        state.extend_from_slice(&cart.chr_ram);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.latch = state[p];
        p += 1;
        for i in 0..cart.chr_ram.len() {
            if p < state.len() {
                cart.chr_ram[i] = state[p];
                p += 1;
            }
        }
        p
    }

    fn reset(&mut self) {
        self.latch = 0;
    }
}
