use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper439 {
    latch_data: u8,
    reg: [u8; 2],
}

impl Mapper439 {
    pub fn new() -> Self {
        Self {
            latch_data: 0,
            reg: [0; 2],
        }
    }

    fn prg_and(&self) -> u8 {
        0x07 | ((!self.reg[1]) >> 1 & 0x38)
    }

    fn prg_or(&self) -> u8 {
        (self.reg[0] >> 1) & 0x38
    }

    fn prg_bank_low(&self) -> u8 {
        let and = self.prg_and();
        let or = self.prg_or();
        (self.latch_data & and) | (or & !and)
    }

    fn prg_bank_high(&self) -> u8 {
        let and = self.prg_and();
        let or = self.prg_or();
        (0x3F & and) | (or & !and)
    }

    fn prg_offset(&self, address: u16) -> usize {
        let bank = if address >= 0xC000 {
            self.prg_bank_high()
        } else {
            self.prg_bank_low()
        };
        bank as usize * 0x4000 + (address as usize & 0x3FFF)
    }
}

impl Mapper for Mapper439 {
    fn reset(&mut self) {
        self.latch_data = 0;
        self.reg = [0; 2];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
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
            self.reg[(address as usize) & 1] = data;
        } else if address >= 0x8000 {
            let mask = (self.reg[1] & 0x80) | ((self.reg[1] >> 1) & 0x38);
            self.latch_data = (self.latch_data & mask) | (data & !mask);
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v((self.latch_data & 0x80) != 0, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
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
            let byte = if chr_ram.is_empty() {
                0
            } else {
                chr_ram[(address as usize & 0x1FFF) % chr_ram.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mir = mirror_h_or_v((self.latch_data & 0x80) != 0, address);
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mir = mirror_h_or_v((self.latch_data & 0x80) != 0, address);
            vram[(mir & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.latch_data, self.reg[0], self.reg[1]]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.latch_data = state[p];
            p += 1;
        }
        if p < state.len() {
            self.reg[0] = state[p];
            p += 1;
        }
        if p < state.len() {
            self.reg[1] = state[p];
            p += 1;
        }
        p
    }
}
