use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

pub struct Mapper454 {
    latch_addr: u16,
    latch_data: u8,
}

impl Mapper454 {
    pub fn new() -> Self {
        Self {
            latch_addr: 0,
            latch_data: 0,
        }
    }

    fn cpu_a14(&self) -> bool {
        self.latch_addr & 0x001 != 0
    }

    fn mirror_h(&self) -> bool {
        self.latch_addr & 0x002 != 0
    }

    fn nrom(&self) -> bool {
        self.latch_addr & 0x080 != 0
    }

    fn unrom(&self) -> bool {
        self.latch_addr & 0x100 != 0
    }

    fn prg_bank(&self) -> u16 {
        ((self.latch_addr >> 2) & 0x1F) | ((self.latch_addr >> 3) & 0x20)
    }

    fn bank_16_lo(&self) -> u16 {
        let prg = self.prg_bank();
        if self.unrom() {
            (prg & 0xFFF8) | self.latch_data as u16
        } else if self.cpu_a14() {
            prg & 0xFFFE
        } else {
            prg
        }
    }

    fn bank_16_hi(&self) -> u16 {
        let prg = self.prg_bank();
        let a = prg | if self.cpu_a14() { 1 } else { 0 };
        if self.unrom() {
            a | 7
        } else if self.nrom() {
            a
        } else {
            a & 0xFFE0
        }
    }

    fn read_wram(&self, cart: &Cartridge, address: u16) -> FetchResult {
        if cart.prg_ram.is_empty() {
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        let idx = (address as usize & 0x1FFF) % cart.prg_ram.len();
        FetchResult {
            data: cart.prg_ram[idx],
            driven: true,
        }
    }

    fn write_wram(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if cart.prg_ram.is_empty() {
            return;
        }
        let idx = (address as usize & 0x1FFF) % cart.prg_ram.len();
        cart.prg_ram[idx] = data;
    }
}

impl Mapper for Mapper454 {
    fn reset(&mut self) {
        *self = Self::new();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            return self.read_wram(cart, address);
        }
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            let bank = if address < 0xC000 {
                self.bank_16_lo()
            } else {
                self.bank_16_hi()
            };
            let offset = (bank as usize) * 0x4000 + (address as usize & 0x3FFF);
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
            self.write_wram(cart, address, data);
        } else if address >= 0x8000 {
            if self.unrom() {
                self.latch_data = data & 7;
            } else {
                self.latch_data = data;
                self.latch_addr = address;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v(self.mirror_h(), address)
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
            let byte = if !chr_ram.is_empty() {
                chr_ram[(address as usize & 0x1FFF) % chr_ram.len()]
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
            if !self.nrom() && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = mirror_h_or_v(self.mirror_h(), address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.latch_addr.to_le_bytes());
        state.push(self.latch_data);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 1 < state.len() {
            self.latch_addr = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.latch_data = state[p];
            p += 1;
        }
        p
    }
}
