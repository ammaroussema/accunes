use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

pub struct Mapper500 {
    reg: [u8; 2],
    latch_data: u8,
}

impl Mapper500 {
    pub fn new() -> Self {
        Self {
            reg: [0; 2],
            latch_data: 0,
        }
    }

    fn registers_locked(&self) -> bool {
        (self.reg[1] & 0x80) != 0
    }

    fn mirror_h(&self) -> bool {
        (self.reg[1] & 1) == 0
    }

    fn prg_bank_8000(&self) -> usize {
        (self.reg[0] as usize) << 3 | (self.latch_data as usize & 7)
    }

    fn prg_bank_c000(&self) -> usize {
        (self.reg[0] as usize) << 3 | 7
    }
}

impl Mapper for Mapper500 {
    fn reset(&mut self) {
        self.latch_data = 0;
    }

    fn reset_power_cycle(&mut self) {
        self.reg = [0; 2];
        self.latch_data = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x7000 && address < 0x8000 {
            let off = (address - 0x7000) as usize;
            if off < cart.prg_ram.len() {
                return FetchResult {
                    data: cart.prg_ram[off],
                    driven: true,
                };
            }
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        if address >= 0x8000 {
            let prg_len = cart.prg_rom.len();
            if prg_len == 0 {
                return FetchResult {
                    data: 0,
                    driven: false,
                };
            }
            let bank = if address < 0xC000 {
                self.prg_bank_8000()
            } else {
                self.prg_bank_c000()
            };
            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            return FetchResult {
                data: cart.prg_rom[offset % prg_len],
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x7000 {
            if !self.registers_locked() {
                self.reg[(address & 1) as usize] = data;
            }
            return;
        }
        if address >= 0x7000 && address < 0x8000 {
            let off = (address - 0x7000) as usize;
            if off < cart.prg_ram.len() {
                cart.prg_ram[off] = data;
            }
            return;
        }
        if address >= 0x8000 {
            self.latch_data = data;
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
            if using_chr_ram && !chr_ram.is_empty() {
                let len = chr_ram.len();
                new_addr_bus |= chr_ram[(address as usize) % len] as u16;
            }
        } else {
            let mirrored = mirror_h_or_v(self.mirror_h(), address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize) % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = mirror_h_or_v(self.mirror_h(), address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.latch_data, self.reg[0], self.reg[1]]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.latch_data = state[p];
            p += 1;
        }
        if p < state.len() {
            self.reg[0] = state[p];
            p += 1;
        }
        if p < state.len() {
            self.reg[1] = state[p];
            p += 1;
        }
        p
    }
}
