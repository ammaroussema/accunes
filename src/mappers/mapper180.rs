use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::uxrom::{mirror_address, UxromConfig};

pub struct Mapper180 {
    config: UxromConfig,
    bank_select: u8,
}

impl Mapper180 {
    pub fn new(config: UxromConfig) -> Self {
        Self {
            config,
            bank_select: 0,
        }
    }
}

impl Mapper for Mapper180 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult { data: 0, driven: false };
            }
            let banks_16k = len / 0x4000;
            let bank = if address < 0xC000 {
                0
            } else {
                self.bank_select as usize % banks_16k
            };
            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            }
        } else {
            let off = (address.saturating_sub(0x6000)) as usize;
            if self.config.prg_ram_size > 0 && address >= 0x6000 && address < 0x8000 && off < self.config.prg_ram_size {
                FetchResult {
                    data: cart.prg_ram[off],
                    driven: true,
                }
            } else {
                FetchResult {
                    data: 0,
                    driven: false,
                }
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        let off = (address.saturating_sub(0x6000)) as usize;
        if self.config.prg_ram_size > 0 && address >= 0x6000 && address < 0x8000 && off < self.config.prg_ram_size {
            cart.prg_ram[off] = data;
        } else if address >= 0x8000 {
            let val = if self.config.bus_conflict {
                let len = cart.prg_rom.len();
                if len > 0 {
                    let banks_16k = len / 0x4000;
                    let bank = if address < 0xC000 { 0 } else { self.bank_select as usize % banks_16k };
                    let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                    data & cart.prg_rom[offset % len]
                } else {
                    data
                }
            } else {
                data
            };
            self.bank_select = val & 0x0F;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        mirror_address(
            cart.alternative_nametable_arrangement,
            cart.nametable_horizontal_mirroring,
            address,
        )
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            if using_chr_ram && !chr_ram.is_empty() {
                let mask = chr_ram.len() - 1;
                new_addr_bus |= chr_ram[(address as usize) & mask] as u16;
            } else if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[address as usize % chr_rom.len()] as u16;
            }
        } else {
            let mirrored = mirror_address(
                alternative_nametable_arrangement,
                nametable_horizontal_mirroring,
                address,
            );
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() {
                    prg_vram[idx]
                } else {
                    0
                }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let mask = cart.chr_ram.len() - 1;
                cart.chr_ram[(address as usize) & mask] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
            }
        }
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.push(self.bank_select);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..cart.prg_ram.len() {
            cart.prg_ram[i] = state[p];
            p += 1;
        }
        for i in 0..cart.chr_ram.len() {
            cart.chr_ram[i] = state[p];
            p += 1;
        }
        self.bank_select = state[p];
        p + 1
    }

    fn reset(&mut self) {
        self.bank_select = 0;
    }
}
