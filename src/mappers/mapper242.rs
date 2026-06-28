use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper242 {
    reg: u16,
}

impl Mapper242 {
    pub fn new() -> Self {
        Self { reg: 0 }
    }
}

impl Mapper for Mapper242 {
    fn reset(&mut self) {
        self.reg = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            let offset = (address as usize - 0x6000) % cart.prg_ram.len().max(1);
            return FetchResult { data: cart.prg_ram.get(offset).copied().unwrap_or(0), driven: true };
        }
        if address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }
        let bank = ((self.reg >> 2) & 0x1F) as usize;
        let a14 = (self.reg & 1) as usize;
        let base = bank * 2 + a14;
        let bank16 = if address < 0xC000 { base } else { base + 1 };
        let offset = bank16 * 0x4000 + (address as usize & 0x3FFF);
        let len = cart.prg_rom.len();
        if len == 0 {
            return FetchResult { data: 0, driven: false };
        }
        FetchResult { data: cart.prg_rom[offset % len], driven: true }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            let offset = (address as usize - 0x6000) % cart.prg_ram.len().max(1);
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
        } else if address >= 0x8000 {
            self.reg = address & 0x7FFF;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if (self.reg & 2) != 0 {
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
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let len = chr_ram.len();
            if len == 0 {
                return (0, new_addr_bus);
            }
            new_addr_bus |= chr_ram[(address as usize & 0x1FFF) % len] as u16;
        } else {
            let mirrored = if (self.reg & 2) != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::with_capacity(2);
        state.extend_from_slice(&self.reg.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start + 2 <= state.len() {
            self.reg = u16::from_le_bytes([state[start], state[start + 1]]);
            start + 2
        } else {
            start
        }
    }
}
