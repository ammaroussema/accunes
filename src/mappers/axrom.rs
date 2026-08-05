use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct MapperAxROM {
    bank_select: u8,
    single_screen_high: bool,
    submapper_id: u8,
}

impl MapperAxROM {
    pub fn new(submapper_id: u8) -> Self {
        Self {
            bank_select: 0,
            single_screen_high: false,
            submapper_id,
        }
    }

    fn ciram_offset(&self, address: u16) -> usize {
        let off = (address & 0x03FF) as usize;
        if self.single_screen_high {
            0x400 | off
        } else {
            off
        }
    }
}

impl Mapper for MapperAxROM {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let banks_32k = (cart.prg_rom.len() / 0x8000).max(1);
            let bank = (self.bank_select & 0x0F) as usize % banks_32k;
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let mut value = data;
            if self.submapper_id == 2 && !cart.prg_rom.is_empty() {
                let banks_32k = (cart.prg_rom.len() / 0x8000).max(1);
                let bank = (self.bank_select & 0x0F) as usize % banks_32k;
                let prg_offset = bank * 0x8000 + (address as usize & 0x7FFF);
                let prg_data = cart.prg_rom[prg_offset % cart.prg_rom.len()];
                value &= prg_data;
            }
            self.bank_select = value & 0x0F;
            self.single_screen_high = (value & 0x10) != 0;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.ciram_offset(address) as u16
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
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let byte = if using_chr_ram {
                chr_ram[address as usize & 0x1FFF]
            } else {
                chr_rom[address as usize % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            new_addr_bus |= vram[self.ciram_offset(address)] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn reset(&mut self) {
        self.bank_select = 0;
        self.single_screen_high = false;
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![
            self.bank_select,
            if self.single_screen_high { 1 } else { 0 },
        ]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if state.len() >= start + 2 {
            self.bank_select = state[start];
            self.single_screen_high = state[start + 1] != 0;
            start + 2
        } else {
            start
        }
    }
}
