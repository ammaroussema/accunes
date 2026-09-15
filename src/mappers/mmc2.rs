use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct MapperMMC2 {
    preg: u8,
    creg: [u8; 4],
    latch0_fe: bool,
    latch1_fe: bool,
    horizontal_mirror: bool,
}

impl MapperMMC2 {
    pub fn new() -> Self {
        Self {
            preg: 0,
            creg: [0; 4],
            latch0_fe: true,
            latch1_fe: true,
            horizontal_mirror: true,
        }
    }

    fn banks_8k(cart: &Cartridge) -> usize {
        (cart.prg_rom.len() / 0x2000).max(1)
    }

    fn prg_read(&self, cart: &Cartridge, address: u16) -> u8 {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }
        let banks = Self::banks_8k(cart);
        let bank = if address < 0xA000 {
            self.preg as usize % banks
        } else if address < 0xC000 {
            (banks.saturating_sub(3)) % banks
        } else if address < 0xE000 {
            (banks.saturating_sub(2)) % banks
        } else {
            (banks.saturating_sub(1)) % banks
        };
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        cart.prg_rom[offset % len]
    }

    fn chr_bank(&self, address: u16) -> u8 {
        if address < 0x1000 {
            if self.latch0_fe {
                self.creg[1]
            } else {
                self.creg[0]
            }
        } else if self.latch1_fe {
            self.creg[3]
        } else {
            self.creg[2]
        }
    }

    fn ppu_latch(&mut self, address: u16) {
        let h = address >> 8;
        if h >= 0x20 || (h & 0xF) != 0xF {
            return;
        }
        let l = address & 0xF0;
        if h < 0x10 {
            if l == 0xD0 {
                self.latch0_fe = false;
            } else if l == 0xE0 {
                self.latch0_fe = true;
            }
        } else if l == 0xD0 {
            self.latch1_fe = false;
        } else if l == 0xE0 {
            self.latch1_fe = true;
        }
    }

    fn ciram_offset(&self, address: u16) -> usize {
        let mirrored = if self.horizontal_mirror {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        };
        (mirrored & 0x7FF) as usize
    }
}

impl Mapper for MapperMMC2 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            FetchResult {
                data: self.prg_read(cart, address),
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
        match address & 0xF000 {
            0xA000 => self.preg = data,
            0xB000 => self.creg[0] = data,
            0xC000 => self.creg[1] = data,
            0xD000 => self.creg[2] = data,
            0xE000 => self.creg[3] = data,
            0xF000 => self.horizontal_mirror = (data & 1) != 0,
            _ => {}
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.ciram_offset(address) as u16
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
            self.ppu_latch(address);
            let bank = self.chr_bank(address) as usize;
            let byte = if using_chr_ram {
                let len = chr_ram.len();
                if len == 0 {
                    0
                } else {
                    chr_ram[(bank * 0x1000 + (address as usize & 0x0FFF)) % len]
                }
            } else if chr_rom.is_empty() {
                0
            } else {
                chr_rom[(bank * 0x1000 + (address as usize & 0x0FFF)) % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            new_addr_bus |= vram[self.ciram_offset(address)] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            self.ppu_latch(address);
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let bank = self.chr_bank(address) as usize;
                let len = cart.chr_ram.len();
                let offset = (bank * 0x1000 + (address as usize & 0x0FFF)) % len;
                cart.chr_ram[offset] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            vram[self.ciram_offset(address)] = data;
        }
    }

    fn reset(&mut self) {
        self.preg = 0;
        self.latch0_fe = true;
        self.latch1_fe = true;
        self.horizontal_mirror = true;
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![
            self.preg,
            self.creg[0],
            self.creg[1],
            self.creg[2],
            self.creg[3],
            if self.horizontal_mirror { 1 } else { 0 },
            if self.latch0_fe { 1 } else { 0 },
            if self.latch1_fe { 1 } else { 0 },
        ]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if state.len() >= start + 8 {
            self.preg = state[start];
            self.creg[0] = state[start + 1];
            self.creg[1] = state[start + 2];
            self.creg[2] = state[start + 3];
            self.creg[3] = state[start + 4];
            self.horizontal_mirror = state[start + 5] != 0;
            self.latch0_fe = state[start + 6] != 0;
            self.latch1_fe = state[start + 7] != 0;
            start + 8
        } else {
            start
        }
    }
}
