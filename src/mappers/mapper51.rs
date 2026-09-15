use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper51 {
    bank: u8,
    mode: u8,
}

impl Mapper51 {
    pub fn new() -> Self {
        Self {
            bank: 0,
            mode: 1,
        }
    }
}

impl Mapper for Mapper51 {
    fn reset(&mut self) {
        self.bank = 0;
        self.mode = 1;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        let num_banks = cart.prg_rom.len() / 0x2000;
        if num_banks == 0 {
            return FetchResult { data: 0, driven: true };
        }
        let bank = match address {
            0x6000..=0x7FFF => {
                if (self.mode & 0x01) != 0 {
                    (0x23 | (self.bank << 2)) as usize
                } else {
                    (0x2F | (self.bank << 2)) as usize
                }
            }
            0x8000..=0x9FFF => {
                if (self.mode & 0x01) != 0 {
                    ((self.bank << 2) + 0) as usize
                } else {
                    (((self.bank << 2) | self.mode) + 0) as usize
                }
            }
            0xA000..=0xBFFF => {
                if (self.mode & 0x01) != 0 {
                    ((self.bank << 2) + 1) as usize
                } else {
                    (((self.bank << 2) | self.mode) + 1) as usize
                }
            }
            0xC000..=0xDFFF => {
                if (self.mode & 0x01) != 0 {
                    ((self.bank << 2) + 2) as usize
                } else {
                    (((self.bank << 2) | 0x0E) + 0) as usize
                }
            }
            0xE000..=0xFFFF => {
                if (self.mode & 0x01) != 0 {
                    ((self.bank << 2) + 3) as usize
                } else {
                    (((self.bank << 2) | 0x0E) + 1) as usize
                }
            }
            _ => { return FetchResult { data: 0, driven: false }; }
        };
        let offset = (bank % num_banks) * 0x2000 + (address as usize & 0x1FFF);
        FetchResult {
            data: cart.prg_rom[offset % cart.prg_rom.len()],
            driven: true,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address <= 0x7FFF {
            self.mode = ((data >> 3) & 0x02) | ((data >> 1) & 0x01);
        } else if address >= 0xC000 && address <= 0xDFFF {
            self.bank = data & 0x0F;
            self.mode = ((data >> 3) & 0x02) | (self.mode & 0x01);
        } else if address >= 0x8000 {
            self.bank = data & 0x0F;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mode == 0x03 {
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
        _nametable_horizontal_mirroring: bool,
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
            } else if self.mode == 0x03 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() {
                    prg_vram[idx]
                } else {
                    0
                }
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
        vec![self.bank, self.mode]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 2 <= state.len() {
            self.bank = state[start];
            start += 1;
            self.mode = state[start];
            start += 1;
        }
        start
    }
}
