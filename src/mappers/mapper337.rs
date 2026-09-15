use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper337 {
    reg: u16,
    latch_data: u8,
    latch_addr: u16,
}

impl Mapper337 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self { reg: 0, latch_data: 0, latch_addr: 0 }
    }
}

impl Mapper for Mapper337 {
    fn reset(&mut self) {
        self.reg = 0;
        self.latch_data = 0;
        self.latch_addr = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            let offset = (address as usize - 0x6000) % cart.prg_ram.len().max(1);
            return FetchResult { data: cart.prg_ram[offset], driven: true };
        }
        if address >= 0x8000 {
            let reg = self.reg;
            let _bank_mod = match reg >> 6 {
                0 => {
                    let bank = reg as usize;
                    let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                    return FetchResult {
                        data: cart.prg_rom[offset % cart.prg_rom.len().max(1)],
                        driven: true,
                    };
                }
                1 => {
                    let bank = (reg >> 1) as usize;
                    let offset = bank * 0x8000 + (address as usize & 0x7FFF);
                    return FetchResult {
                        data: cart.prg_rom[offset % cart.prg_rom.len().max(1)],
                        driven: true,
                    };
                }
                _ => {
                    let bank0 = reg as usize;
                    let bank1 = (reg | 7) as usize;
                    let offset = address as usize & 0x3FFF;
                    let base = if address < 0xC000 { bank0 * 0x4000 } else { bank1 * 0x4000 };
                    return FetchResult {
                        data: cart.prg_rom[(base + offset) % cart.prg_rom.len().max(1)],
                        driven: true,
                    };
                }
            };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            let offset = (address as usize - 0x6000) % cart.prg_ram.len().max(1);
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
            return;
        }
        if address >= 0x8000 {
            self.latch_data = data;
            self.latch_addr = address;
            if (address & 0x4000) != 0 {
                self.reg = (self.reg & !7) | (data as u16 & 7);
            } else {
                self.reg = (self.reg & 7) | (data as u16 & !7);
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v((self.reg & 0x20) != 0, address)
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
            new_addr_bus |= vram[(mirror_h_or_v((self.reg & 0x20) != 0, address) & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram && (self.reg & 0x80) != 0 {
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
        let mut state = Vec::with_capacity(4);
        state.extend_from_slice(&self.reg.to_le_bytes());
        state.push(self.latch_data);
        state.extend_from_slice(&self.latch_addr.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 > state.len() { return p; }
        self.reg = u16::from_le_bytes([state[p], state[p+1]]);
        p += 2;
        if p < state.len() {
            self.latch_data = state[p];
            p += 1;
        }
        if p + 1 < state.len() {
            self.latch_addr = u16::from_le_bytes([state[p], state[p+1]]);
            p + 2
        } else { p }
    }
}
