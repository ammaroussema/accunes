use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper324 {
    latch_data: u8,
}

impl Mapper324 {
    pub fn new() -> Self {
        Self { latch_data: 0 }
    }

    fn prg_bank(&self) -> usize {
        (self.latch_data as usize & 7) | ((self.latch_data as usize) >> 1 & 0x38)
    }

    fn is_locked(&self) -> bool {
        (self.latch_data & 8) != 0
    }
}

impl Mapper for Mapper324 {
    fn reset(&mut self) {
        self.latch_data = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = self.prg_bank();
            let is_high = address >= 0xC000;
            let prg_bank = if is_high { bank | 7 } else { bank };
            let offset = prg_bank * 0x4000 + (address as usize & 0x3FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len().max(1)],
                driven: true,
            }
        } else if address >= 0x6000 {
            let offset = address as usize & 0x1FFF;
            FetchResult {
                data: if offset < cart.prg_ram.len() { cart.prg_ram[offset] } else { 0 },
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) {
            let offset = address as usize & 0x1FFF;
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
        } else if address >= 0x8000 {
            let index = (address as usize) % cart.prg_rom.len().max(1);
            let rom_data = cart.prg_rom[index];
            let bus_data = data & rom_data;
            if self.is_locked() || (self.latch_data & 0x80) != 0 || (bus_data & 0x80) == 0 {
                self.latch_data = (self.latch_data & !7) | (bus_data & 7);
            } else {
                self.latch_data = bus_data;
            }
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
            new_addr_bus |= if len > 0 { chr_ram[(address as usize & 0x1FFF) % len] as u16 } else { 0 };
        } else {
            new_addr_bus |= vram[(mirror_h_or_v(_nametable_horizontal_mirroring, address) & 0x7FF) as usize] as u16;
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
        vec![self.latch_data]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.latch_data = state.get(p).copied().unwrap_or(0); p += 1;
        p
    }
}
