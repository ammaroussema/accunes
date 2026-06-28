use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper93 {
    data: u8,
}

impl Mapper93 {
    pub fn new() -> Self {
        Self { data: 0 }
    }

    fn prg_read(&self, cart: &Cartridge, address: u16) -> u8 {
        let prg_len = cart.prg_rom.len();
        if prg_len == 0 {
            return 0;
        }
        let bank = if address < 0xC000 {
            (self.data >> 4) as usize
        } else {
            (prg_len / 0x4000).saturating_sub(1)
        };
        let offset = (bank * 0x4000 + (address as usize & 0x3FFF)) % prg_len;
        cart.prg_rom[offset]
    }
}

impl Mapper for Mapper93 {
    fn reset(&mut self) {
        self.data = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            return FetchResult {
                data: self.prg_read(cart, address),
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, mut data: u8) {
        if address >= 0x8000 {
            let prg_len = cart.prg_rom.len();
            if prg_len > 0 {
                let fetch_res = self.fetch_prg(cart, address);
                if fetch_res.driven {
                    data &= fetch_res.data;
                }
            }
            self.data = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.nametable_horizontal_mirroring {
            (address & 0x3FFF) | ((address & 0x0800) >> 1)
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
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let data = if (self.data & 0x01) != 0 {
                let len = if using_chr_ram { chr_ram.len() } else { chr_rom.len() };
                if len > 0 {
                    let offset = (address as usize & 0x1FFF) % len;
                    if using_chr_ram { chr_ram[offset] } else { chr_rom[offset] }
                } else {
                    0
                }
            } else {
                0 
            };
            new_addr_bus |= data as u16;
        } else {
            let mirrored = if nametable_horizontal_mirroring {
                (address & 0x3FFF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let data = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= data as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if (self.data & 0x01) != 0 && cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let mask = cart.chr_ram.len() - 1;
                cart.chr_ram[address as usize & mask] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.data]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start < state.len() {
            self.data = state[start];
            start + 1
        } else {
            start
        }
    }
}
