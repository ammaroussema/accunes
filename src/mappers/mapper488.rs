use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

pub struct Mapper488 {
    addr: u16,
    dip_switches: u8,
}

impl Mapper488 {
    pub fn new() -> Self {
        Self {
            addr: 0,
            dip_switches: 0,
        }
    }
}

impl Mapper for Mapper488 {
    fn reset(&mut self) {
        *self = Self::new();
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
            let effective = if (self.addr & 0x100) != 0 {
                (address & 0xFFF0) | (self.dip_switches & 0xF) as u16
            } else {
                address
            };
            let offset = if (self.addr & 0x4) != 0 {
                (self.addr as usize) * 0x8000 + (effective as usize & 0x7FFF)
            } else {
                let bank = (self.addr << 1) | ((self.addr >> 4) & 1);
                (bank as usize) * 0x4000 + (effective as usize & 0x3FFF)
            };
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

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, _data: u8) {
        if address >= 0x8000 {
            self.addr = address;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v(cart.nametable_horizontal_mirroring, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let offset = if using_chr_ram && !chr_ram.is_empty() {
                (address as usize) & 0x1FFF
            } else if !chr_rom.is_empty() {
                (self.addr as usize) * 0x2000 + (address as usize & 0x1FFF)
            } else {
                0
            };
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mirror_h_or_v(nametable_horizontal_mirroring, address);
            let byte = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= byte as u16;
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
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.addr as u8]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        self.addr = state.get(start).copied().unwrap_or(0) as u16;
        start + 1
    }
}
