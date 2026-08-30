use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper623 {
    reg: u8,
    data_latch: u8,
}

impl Mapper623 {
    pub fn new() -> Self {
        Self { reg: 0, data_latch: 0 }
    }

    fn prg_offset(&self, cart: &Cartridge, address: u16) -> usize {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }
        let offset = if (self.reg & 0x08) != 0 {
            let bank = if address < 0xC000 {
                self.data_latch & 0x0F
            } else {
                0x0F
            };
            (bank as usize) * 0x4000 + (address as usize & 0x3FFF)
        } else {
            ((self.data_latch >> 1) as usize) * 0x8000 + (address as usize & 0x7FFF)
        };
        offset % len
    }
}

impl Mapper for Mapper623 {
    fn reset(&mut self) {
        self.reg = 0;
        self.data_latch = 0;
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 || cart.prg_rom.is_empty() {
            return FetchResult { data: 0, driven: false };
        }
        let offset = self.prg_offset(cart, address);
        FetchResult {
            data: cart.prg_rom[offset],
            driven: true,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x6000 {
            self.reg = data;
        } else if address >= 0x8000 {
            self.data_latch = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.nametable_horizontal_mirroring {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;

        if address < 0x2000 {
            let offset = (address as usize) & 0x1FFF;
            let byte = if !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if !nametable_horizontal_mirroring {
                address & 0x37FF
            } else {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }

        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let offset = (address as usize) & 0x1FFF;
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[mirrored as usize & 0x7FF] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.data_latch, self.reg]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start < state.len() {
            self.data_latch = state[start];
            start += 1;
        }
        if start < state.len() {
            self.reg = state[start];
            start += 1;
        }
        start
    }
}
