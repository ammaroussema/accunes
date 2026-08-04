use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper414 {
    latch_addr: u16,
    latch_data: u8,
    dip_value: u8,
}

impl Mapper414 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self { latch_addr: 0, latch_data: 0, dip_value: 0 }
    }

    fn ob4_active(&self) -> bool {
        (self.latch_addr & 0x100) == 0 && (self.latch_addr & (self.dip_value as u16)) != 0
    }

    fn prg_read(&self, cart: &Cartridge, address: u16) -> u8 {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }
        let (base, size) = if (self.latch_addr & 0x2000) != 0 {
            ((self.latch_addr as usize >> 2) * 0x8000, 0x8000)
        } else {
            ((self.latch_addr as usize >> 1) * 0x4000, 0x4000)
        };
        let offset = base + (address as usize & (size - 1));
        cart.prg_rom[offset % len]
    }
}

impl Mapper for Mapper414 {
    fn reset(&mut self) {
        self.latch_addr = 0;
        self.latch_data = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if self.ob4_active() && address >= 0xC000 {
                return FetchResult { data: 0, driven: false };
            }
            return FetchResult { data: self.prg_read(cart, address), driven: true };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let rom_byte = if self.ob4_active() && address >= 0xC000 {
                data
            } else {
                self.prg_read(cart, address)
            };
            self.latch_addr = address;
            self.latch_data = data & rom_byte;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v((self.latch_addr & 0x01) != 0, address)
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
                let offset = (self.latch_data as usize) * 0x2000 + (address as usize & 0x1FFF);
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mir = mirror_h_or_v((self.latch_addr & 0x01) != 0, address);
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, _cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address >= 0x2000 {
            let mir = mirror_h_or_v((self.latch_addr & 0x01) != 0, address);
            vram[(mir & 0x7FF) as usize] = data;
        }
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_value
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_value = value;
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = self.latch_addr.to_le_bytes().to_vec();
        state.push(self.latch_data);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.latch_addr = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.latch_data = state[p];
            p += 1;
        }
        p
    }
}
