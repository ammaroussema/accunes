use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

pub struct Mapper466 {
    reg: [u8; 2],
    data: u8,
}

impl Mapper466 {
    pub fn new() -> Self {
        Self {
            reg: [0; 2],
            data: 0,
        }
    }

    fn prg(&self) -> usize {
        ((self.reg[1] << 5) | ((self.reg[0] << 1) & 0x1E) | ((self.reg[0] >> 5) & 1)) as usize
    }

    fn nrom(&self) -> bool {
        (self.reg[0] & 0x40) != 0
    }

    fn nrom128(&self) -> bool {
        (self.reg[0] & 0x10) != 0
    }

    fn mirror_h(&self) -> bool {
        (self.reg[0] & 0x80) != 0
    }

    fn prg_slot_bank(&self, slot: usize) -> usize {
        let prg = self.prg();
        if self.nrom() {
            if self.nrom128() {
                prg * 2 + (slot & 1)
            } else {
                (prg & !1) * 2 + slot
            }
        } else {
            let bank = if slot < 2 {
                (prg & !7) | (self.data as usize & 7)
            } else {
                (prg & !7) | 7
            };
            bank * 2 + (slot & 1)
        }
    }
}

impl Mapper for Mapper466 {
    fn reset(&mut self) {
        *self = Self::new();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x6000..0x8000).contains(&address) {
            if !cart.prg_ram.is_empty() {
                let offset = (address as usize & 0x1FFF) % cart.prg_ram.len();
                return FetchResult {
                    data: cart.prg_ram[offset],
                    driven: true,
                };
            }
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            if (self.prg() & 0x20) != 0 && len < 0x100000 {
                return FetchResult {
                    data: 0,
                    driven: false,
                };
            }
            let slot = ((address >> 13) & 3) as usize;
            let bank = self.prg_slot_bank(slot);
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
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
        match address {
            0x5000..=0x57FF => self.reg[0] = (address & 0xFF) as u8,
            0x5800..=0x5FFF => self.reg[1] = (address & 0xFF) as u8,
            0x6000..=0x7FFF => {
                if !cart.prg_ram.is_empty() {
                    let offset = (address as usize & 0x1FFF) % cart.prg_ram.len();
                    cart.prg_ram[offset] = data;
                }
            }
            0x8000..=0xFFFF => self.data = data,
            _ => {}
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v(self.mirror_h(), address)
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
            let offset = (address as usize) & 0x1FFF;
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mirror_h_or_v(self.mirror_h(), address);
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
            let mirrored = mirror_h_or_v(self.mirror_h(), address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.data, self.reg[0], self.reg[1]]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.data = state.get(p).copied().unwrap_or(0);
        p += 1;
        self.reg[0] = state.get(p).copied().unwrap_or(0);
        p += 1;
        self.reg[1] = state.get(p).copied().unwrap_or(0);
        p += 1;
        p
    }
}
