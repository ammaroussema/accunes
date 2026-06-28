use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper103 {
    prg_ram_disabled: bool,
    prg_reg: u8,
    horizontal_mirroring: bool,
    chr_bank: u8,
}

impl Mapper103 {
    pub fn new() -> Self {
        Self {
            prg_ram_disabled: false,
            prg_reg: 0,
            horizontal_mirroring: false,
            chr_bank: 0,
        }
    }

    fn update_state(&mut self, _cart: &mut Cartridge) {
    }
}

impl Mapper for Mapper103 {
    fn reset(&mut self) {
        self.prg_ram_disabled = false;
        self.prg_reg = 0;
        self.horizontal_mirroring = false;
        self.chr_bank = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let prg_len = cart.prg_rom.len();
            if prg_len == 0 {
                return FetchResult { data: 0, driven: false };
            }
            let num_banks = prg_len / 0x2000;
            if num_banks >= 4 {
                let bank = (num_banks - 4) + ((address - 0x8000) / 0x2000) as usize;
                let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % prg_len;
                return FetchResult {
                    data: cart.prg_rom[offset],
                    driven: true,
                };
            } else {
                let offset = (address as usize & 0x7FFF) % prg_len;
                return FetchResult {
                    data: cart.prg_rom[offset],
                    driven: true,
                };
            }
        } else if address >= 0x6000 && address < 0x8000 {
            if self.prg_ram_disabled {
                let prg_len = cart.prg_rom.len();
                if prg_len == 0 {
                    return FetchResult { data: 0, driven: false };
                }
                let bank = (self.prg_reg & 0x0F) as usize;
                let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % prg_len;
                return FetchResult {
                    data: cart.prg_rom[offset],
                    driven: true,
                };
            } else {
                let offset = (address - 0x6000) as usize;
                if offset < cart.prg_ram.len() {
                    return FetchResult {
                        data: cart.prg_ram[offset],
                        driven: true,
                    };
                }
            }
        } else if address >= 0xB800 && address < 0xD800 {
            if !self.prg_ram_disabled {
                let offset = 0x2000 + (address - 0xB800) as usize;
                if offset < cart.prg_ram.len() {
                    return FetchResult {
                        data: cart.prg_ram[offset],
                        driven: true,
                    };
                }
            }
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        match address & 0xF000 {
            0x6000 | 0x7000 => {
                let offset = (address - 0x6000) as usize;
                if offset < cart.prg_ram.len() {
                    cart.prg_ram[offset] = data;
                }
            }
            0x8000 => {
                self.prg_reg = data & 0x0F;
                self.update_state(cart);
            }
            0xB000 | 0xC000 | 0xD000 => {
                if address >= 0xB800 && address < 0xD800 {
                    let offset = 0x2000 + (address - 0xB800) as usize;
                    if offset < cart.prg_ram.len() {
                        cart.prg_ram[offset] = data;
                    }
                }
            }
            0xE000 => {
                self.horizontal_mirroring = (data & 0x08) != 0;
            }
            0xF000 => {
                self.prg_ram_disabled = (data & 0x10) == 0x10;
                self.update_state(cart);
            }
            _ => {}
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let norm = address & 0x2FFF;
        if self.horizontal_mirroring {
            (norm & 0x33FF) | ((norm & 0x0800) >> 1)
        } else {
            norm & 0x37FF
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
            let len = if using_chr_ram { chr_ram.len() } else { chr_rom.len() };
            if len == 0 {
                return (0, new_addr_bus);
            }
            let bank = self.chr_bank as usize;
            let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % len;
            let byte = if using_chr_ram {
                chr_ram[offset]
            } else {
                chr_rom[offset]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if nametable_horizontal_mirroring {
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
                let offset = (address as usize & 0x1FFF) % cart.chr_ram.len();
                cart.chr_ram[offset] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![
            if self.prg_ram_disabled { 1 } else { 0 },
            self.prg_reg,
            if self.horizontal_mirroring { 1 } else { 0 },
            self.chr_bank,
        ]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.prg_ram_disabled = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.prg_reg = state[p];
            p += 1;
        }
        if p < state.len() {
            self.horizontal_mirroring = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.chr_bank = state[p];
            p += 1;
        }
        p
    }
}
