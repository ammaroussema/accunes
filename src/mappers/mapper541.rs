use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

pub struct Mapper541 {
    addr: u16,
}

impl Mapper541 {
    pub fn new() -> Self {
        Self { addr: 0 }
    }
}

impl Mapper for Mapper541 {
    fn reset(&mut self) {
        self.addr = 0;
    }

    fn reset_power_cycle(&mut self) {
        self.addr = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
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
        let offset = if (self.addr & 2) != 0 {
            let bank16 = (self.addr >> 2) as usize;
            bank16 * 0x4000 + (address as usize & 0x3FFF)
        } else {
            let bank32 = (self.addr >> 3) as usize;
            bank32 * 0x8000 + (address as usize & 0x7FFF)
        };
        FetchResult {
            data: cart.prg_rom[offset % len],
            driven: true,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, _val: u8) {
        if address >= 0xB000 {
            self.addr = address;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else if (self.addr & 1) != 0 {
            mirror_h_or_v(false, address)
        } else {
            mirror_h_or_v(true, address)
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
            if using_chr_ram && !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[(address as usize) % chr_ram.len()] as u16;
            }
        } else {
            let mirrored = if (self.addr & 1) != 0 {
                mirror_h_or_v(false, address)
            } else {
                mirror_h_or_v(true, address)
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram && !cart.chr_ram.is_empty() {
            let len = cart.chr_ram.len();
            cart.chr_ram[(address as usize) % len] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = if (self.addr & 1) != 0 {
                mirror_h_or_v(false, address)
            } else {
                mirror_h_or_v(true, address)
            };
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        self.addr.to_le_bytes().to_vec()
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.addr = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        p
    }
}
