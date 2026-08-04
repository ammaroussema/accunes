use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper428 {
    reg: [u8; 4],
    latch_data: u8,
    dip_value: u8,
}

impl Mapper428 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self {
            reg: [0; 4],
            latch_data: 0,
            dip_value: 0,
        }
    }

    fn chr_bank(&self) -> usize {
        let shift = self.reg[2] >> 6;
        ((self.reg[1] & 0x07) & !shift | self.latch_data & shift) as usize
    }

    fn prg_offset(&self, address: u16) -> usize {
        if (self.reg[1] & 0x10) != 0 {
            (self.reg[1] as usize >> 6) * 0x8000 + (address as usize & 0x7FFF)
        } else {
            (self.reg[1] as usize >> 5) * 0x4000 + (address as usize & 0x3FFF)
        }
    }
}

impl Mapper for Mapper428 {
    fn reset(&mut self) {
        self.reg = [0; 4];
        self.latch_data = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x6000..0x8000).contains(&address) {
            return FetchResult {
                data: self.dip_value & 3,
                driven: true,
            };
        }
        if address >= 0x8000 {
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

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) {
            self.reg[address as usize & 3] = data;
        } else if address >= 0x8000 {
            self.latch_data = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v((self.reg[1] & 0x08) != 0, address)
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
            let mir = mirror_h_or_v((self.reg[1] & 0x08) != 0, address);
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, _cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if (0x2000..0x3F00).contains(&address) {
            let mir = mirror_h_or_v((self.reg[1] & 0x08) != 0, address);
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
        let mut state = Vec::new();
        state.push(self.latch_data);
        state.extend_from_slice(&self.reg);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.latch_data = state[p];
            p += 1;
        }
        if p + 4 <= state.len() {
            self.reg.copy_from_slice(&state[p..p + 4]);
            p += 4;
        }
        p
    }
}
