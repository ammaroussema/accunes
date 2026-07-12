use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper309 {
    prg: u8,
    mirroring: bool,
}

impl Mapper309 {
    pub fn new() -> Self {
        Self { prg: 0, mirroring: false }
    }
}

impl Mapper for Mapper309 {
    fn reset(&mut self) {
        self.prg = 0;
        self.mirroring = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0xE000 {
            let offset = (255 * 0x2000 + (address as usize & 0x1FFF)) % cart.prg_rom.len().max(1);
            FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            }
        } else if address >= 0xC000 {
            let offset = (254 * 0x2000 + (address as usize & 0x1FFF)) % cart.prg_rom.len().max(1);
            FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            }
        } else if address >= 0xA000 {
            let offset = (253 * 0x2000 + (address as usize & 0x1FFF)) % cart.prg_rom.len().max(1);
            FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            }
        } else if address >= 0x8000 {
            let offset = ((self.prg as usize) * 0x2000 + (address as usize & 0x1FFF)) % cart.prg_rom.len().max(1);
            FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            }
        } else if address >= 0x6000 {
            let offset = address as usize & 0x1FFF;
            FetchResult {
                data: if offset < cart.prg_ram.len() { cart.prg_ram[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0x4020 {
            FetchResult { data: 0, driven: false }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 && address < 0x9000 {
            self.prg = data;
        } else if address >= 0xF000 {
            self.mirroring = (data & 0x08) != 0;
        } else if address >= 0x6000 && address < 0x8000 {
            let offset = (address as usize & 0x1FFF) % cart.prg_ram.len().max(1);
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
        }
    }

    fn handle_cpu_write(&mut self, address: u16, _data: u8) {
        if (0x4020..=0x40FF).contains(&address) {
            // FDS audio writes - not implemented
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            return address;
        }
        if self.mirroring {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            if !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[(address & 0x1FFF) as usize] as u16;
            } else if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[(address & 0x1FFF) as usize] as u16;
            }
        } else {
            if alternative_nametable_arrangement {
                new_addr_bus |= vram[(address & 0x7FF) as usize] as u16;
            } else if self.mirroring {
                let mirrored = (address & 0x33FF) | ((address & 0x0800) >> 1);
                new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
            } else {
                new_addr_bus |= vram[(address & 0x37FF & 0x7FF) as usize] as u16;
            }
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        } else if address < 0x2000 && cart.using_chr_ram {
            let offset = address as usize & 0x1FFF;
            if offset < cart.chr_ram.len() {
                cart.chr_ram[offset] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.prg, if self.mirroring { 1 } else { 0 }]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.prg = state.get(p).copied().unwrap_or(0); p += 1;
        self.mirroring = state.get(p).copied().unwrap_or(0) != 0; p += 1;
        p
    }
}
