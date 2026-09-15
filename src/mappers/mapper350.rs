use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper350 {
    outer_bank: u8,
    inner_bank: u8,
    locked: bool,
    header_horizontal: bool,
}

impl Mapper350 {
    pub fn new(header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self { outer_bank: 0, inner_bank: 0, locked: false, header_horizontal: (header.get(6).copied().unwrap_or(0) & 1) == 0 }
    }
}

impl Mapper for Mapper350 {
    fn reset(&mut self) {
        self.outer_bank = 0;
        self.inner_bank = 0;
        self.locked = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            let offset = 1 * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len().max(1)],
                driven: true,
            };
        }
        if address >= 0x8000 {
            let bank_raw = (self.outer_bank as usize & 0x18) | (self.inner_bank as usize & 0x07);
            let mode = (self.outer_bank >> 5) & 3;
            let page = (address as usize - 0x8000) / 0x2000;
            let bank_8k = match mode {
                0 => {
                    let b = bank_raw;
                    b * 2 + page
                }
                1 => {
                    let b = bank_raw >> 1;
                    b * 4 + page
                }
                _ => {
                    let mut b = bank_raw;
                    if (self.outer_bank & 0x20) != 0 { b &= 0x07; }
                    let bank_8000 = b | (self.outer_bank as usize & 0x20);
                    let bank_c000 = bank_8000 | 7;
                    if page < 2 { bank_8000 * 2 + page } else { bank_c000 * 2 + (page - 2) }
                }
            };
            let offset = bank_8k * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len().max(1)],
                driven: true,
            };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, val: u8) {
        if address >= 0x8000 && address < 0xC000 {
            if !self.locked {
                self.locked = (address & 0x2000) != 0;
                self.outer_bank = val;
            }
            return;
        }
        if address >= 0xC000 {
            if !self.locked {
                self.locked = (address & 0x2000) != 0;
                self.inner_bank = val;
            }
            return;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if (self.outer_bank & 0x40) != 0 {
            mirror_h_or_v(self.header_horizontal, address)
        } else if (self.outer_bank & 0x80) != 0 {
            mirror_h_or_v(true, address)
        } else {
            mirror_h_or_v(false, address)
        }
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
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[(address as usize) % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[(address as usize) % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mir = if (self.outer_bank & 0x40) != 0 {
                mirror_h_or_v(self.header_horizontal, address)
            } else if (self.outer_bank & 0x80) != 0 {
                mirror_h_or_v(true, address)
            } else {
                mirror_h_or_v(false, address)
            };
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram && (self.outer_bank & 0x40) != 0 {
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
        let mut state = vec![self.outer_bank, self.inner_bank];
        state.push(if self.locked { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.outer_bank = state[p]; p += 1; }
        if p < state.len() { self.inner_bank = state[p]; p += 1; }
        if p < state.len() { self.locked = state[p] != 0; p += 1; }
        p
    }
}
