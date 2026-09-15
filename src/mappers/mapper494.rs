use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

pub struct Mapper494 {
    addr: u16,
    dip_value: u8,
}

impl Mapper494 {
    pub fn new() -> Self {
        Self {
            addr: 0,
            dip_value: 0,
        }
    }

    fn ob4_active(&self) -> bool {
        (self.addr & 0x100) == 0 && (self.addr & 1) != 0 && (self.dip_value & 1) != 0
    }

    fn chr_bank(&self) -> usize {
        ((self.addr as usize >> 5) & 7) | ((self.addr as usize >> 1) & 8)
    }

    fn mirror_h(&self) -> bool {
        (self.addr & 2) != 0
    }

    fn prg_offset(&self, address: u16) -> usize {
        if (self.addr & 0x100) != 0 {
            if (self.addr & 1) != 0 {
                (self.addr as usize >> 2) * 0x4000 + (address as usize & 0x3FFF)
            } else {
                (self.addr as usize >> 3) * 0x8000 + (address as usize & 0x7FFF)
            }
        } else {
            let bank = if address >= 0xC000 {
                (self.addr as usize >> 2) | 7
            } else {
                self.addr as usize >> 2
            };
            bank * 0x4000 + (address as usize & 0x3FFF)
        }
    }
}

impl Mapper for Mapper494 {
    fn reset(&mut self) {
        *self = Self::new();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if self.ob4_active() {
                return FetchResult {
                    data: 0,
                    driven: false,
                };
            }
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            return FetchResult {
                data: cart.prg_rom[self.prg_offset(address) % len],
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, _data: u8) {
        if address >= 0x8000 {
            self.addr = address;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v(self.mirror_h(), address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        _chr_ram: &[u8],
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
            let byte = if chr_rom.is_empty() {
                0
            } else {
                let offset = self.chr_bank() * 0x2000 + (address as usize & 0x1FFF);
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mirror_h_or_v(self.mirror_h(), address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, _cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if (0x2000..0x3F00).contains(&address) {
            let mirrored = mirror_h_or_v(self.mirror_h(), address);
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
        self.addr.to_le_bytes().to_vec()
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start + 1 < state.len() {
            self.addr = u16::from_le_bytes([state[start], state[start + 1]]);
            start + 2
        } else {
            start
        }
    }
}
