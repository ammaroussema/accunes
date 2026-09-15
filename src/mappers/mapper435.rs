
use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper435 {
    latch_addr: u16,
    dip_value: u8,
    submapper: u8,
}

impl Mapper435 {
    pub fn new(submapper: u8) -> Self {
        Self {
            latch_addr: 0,
            dip_value: 0,
            submapper,
        }
    }

    fn prg_bank(&self) -> usize {
        let a = self.latch_addr as usize;
        (a >> 2 & 0x1F) | (a >> 3 & 0x20) | (a >> 4 & 0x40)
    }

    fn ob4_active(&self) -> bool {
        let mask_bit = if self.submapper == 1 { 0x001 } else { 0x0400 };
        (self.latch_addr & mask_bit) != 0 && self.dip_value != 0
    }

    fn prg_offset(&self, address: u16) -> usize {
        let prg = self.prg_bank();
        if (self.latch_addr & 0x200) != 0 {
            if (self.latch_addr & 0x001) != 0 {
                prg * 0x4000 + (address as usize & 0x3FFF)
            } else {
                (prg >> 1) * 0x8000 + (address as usize & 0x7FFF)
            }
        } else {
            let bank = if address >= 0xC000 { prg | 7 } else { prg };
            bank * 0x4000 + (address as usize & 0x3FFF)
        }
    }

    fn chr_protected(&self) -> bool {
        (self.latch_addr & 0x200) != 0
    }
}

impl Mapper for Mapper435 {
    fn reset(&mut self) {
        self.latch_addr = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x6000..0x8000).contains(&address) {
            if !cart.prg_ram.is_empty() {
                let len = cart.prg_ram.len();
                return FetchResult {
                    data: cart.prg_ram[(address as usize & 0x1FFF) % len],
                    driven: true,
                };
            }
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
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

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) {
            if !cart.prg_ram.is_empty() {
                let len = cart.prg_ram.len();
                cart.prg_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if address >= 0x8000 {
            self.latch_addr = address;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v((self.latch_addr & 0x002) != 0, address)
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
            let mir = mirror_h_or_v((self.latch_addr & 0x002) != 0, address);
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !self.chr_protected() && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mir = mirror_h_or_v((self.latch_addr & 0x002) != 0, address);
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
        self.latch_addr.to_le_bytes().to_vec()
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start + 2 <= state.len() {
            self.latch_addr = u16::from_le_bytes([state[start], state[start + 1]]);
        }
        start + 2
    }
}
