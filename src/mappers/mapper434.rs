
use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper434 {
    latch_data: u8,
    outer_bank: u8,
}

impl Mapper434 {
    pub fn new() -> Self {
        Self {
            latch_data: 0,
            outer_bank: 0,
        }
    }

    fn prg_bank(&self, address: u16) -> usize {
        if address >= 0xC000 {
            7 | ((self.outer_bank as usize) << 3)
        } else {
            ((self.latch_data as usize) & 7) | ((self.outer_bank as usize) << 3)
        }
    }

    fn prg_read(&self, cart: &Cartridge, address: u16) -> u8 {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }
        let offset = self.prg_bank(address) * 0x4000 + (address as usize & 0x3FFF);
        cart.prg_rom[offset % len]
    }

    fn chr_fetch(&self, address: u16, mem: &[u8]) -> u8 {
        mem[(address as usize & 0x1FFF) % mem.len()]
    }
}

impl Mapper for Mapper434 {
    fn reset(&mut self) {
        self.latch_data = 0;
        self.outer_bank = 0;
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
            let offset = self.prg_bank(address) * 0x4000 + (address as usize & 0x3FFF);
            return FetchResult {
                data: cart.prg_rom[offset % len],
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
            self.outer_bank = data;
        } else if address >= 0x8000 {
            self.latch_data = data & self.prg_read(cart, address);
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v((self.outer_bank & 0x20) == 0, address)
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
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                self.chr_fetch(address, chr_ram)
            } else if !chr_rom.is_empty() {
                self.chr_fetch(address, chr_rom)
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mir = mirror_h_or_v((self.outer_bank & 0x20) == 0, address);
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mir = mirror_h_or_v((self.outer_bank & 0x20) == 0, address);
            vram[(mir & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.latch_data, self.outer_bank]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.latch_data = state[p];
            p += 1;
        }
        if p < state.len() {
            self.outer_bank = state[p];
            p += 1;
        }
        p
    }
}
