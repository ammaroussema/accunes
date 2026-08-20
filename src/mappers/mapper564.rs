use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper564 {
    data: u8,
    data_locked: u8,
}

impl Mapper564 {
    pub fn new() -> Self {
        Self {
            data: 0,
            data_locked: 0,
        }
    }

    fn sync(&mut self) {
        let d = self.data;
        self.data_locked = if d & 0x20 != 0 {
            if d & 0x08 != 0 {
                0x28
            } else {
                0x2C
            }
        } else {
            0x00
        };
    }

    fn nt_offset(&self, address: u16) -> u16 {
        let d = self.data;
        if d & 0x20 != 0 {
            if d & 0x10 != 0 {
                0x400 | (address & 0x3FF)
            } else {
                address & 0x3FF
            }
        } else if d & 0x10 != 0 {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }
}

impl Mapper for Mapper564 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let num_32k = cart.prg_rom.len() / 0x8000;
            if num_32k == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            let bank = self.data as usize % num_32k;
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() {
                    cart.prg_rom[offset]
                } else {
                    0
                },
                driven: true,
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.data = (self.data & self.data_locked) | (data & !self.data_locked);
            self.sync();
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.nt_offset(address)
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
            let byte = if !chr_ram.is_empty() {
                chr_ram[address as usize & 0x1FFF]
            } else if !chr_rom.is_empty() {
                chr_rom[address as usize & 0x1FFF]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else if address < 0x3F00 {
            let mirrored = self.nt_offset(address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        } else {
            return (ppu_address_bus as u8, new_addr_bus);
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                cart.chr_ram[address as usize & 0x1FFF] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.nt_offset(address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.data, self.data_locked]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start + 2 <= state.len() {
            self.data = state[start];
            self.data_locked = state[start + 1];
            self.sync();
            start + 2
        } else {
            start
        }
    }

    fn reset(&mut self) {
        self.data = 0;
        self.data_locked = 0;
        self.sync();
    }
}