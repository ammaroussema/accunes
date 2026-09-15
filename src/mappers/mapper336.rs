use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper336 {
    latch_data: u8,
    sub_mapper: u8,
}

impl Mapper336 {
    pub fn new(header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self { latch_data: 0, sub_mapper: header.get(0x18).copied().unwrap_or(0) }
    }
}

impl Mapper for Mapper336 {
    fn reset(&mut self) {
        self.latch_data = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank0 = self.latch_data as usize;
            let bank1 = (self.latch_data | 7) as usize;
            let offset = address as usize & 0x3FFF;
            let base = if address < 0xC000 { bank0 * 0x4000 } else { bank1 * 0x4000 };
            return FetchResult {
                data: cart.prg_rom[(base + offset) % cart.prg_rom.len().max(1)],
                driven: true,
            };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let rom_data = self.latch_data;
            self.latch_data = data & rom_data & !8 | rom_data & 8;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let mirror_bit = if self.sub_mapper == 2 { 0x08 } else { 0x20 };
        mirror_h_or_v((self.latch_data & mirror_bit) != 0, address)
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
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[(address as usize) % chr_ram.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            new_addr_bus |= vram[(mirror_h_or_v(
                if self.sub_mapper == 2 { (self.latch_data & 0x08) != 0 } else { (self.latch_data & 0x20) != 0 },
                address) & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[(address as usize) % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.latch_data]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start < state.len() {
            self.latch_data = state[start];
            start + 1
        } else { start }
    }
}
