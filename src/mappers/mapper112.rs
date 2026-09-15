use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper112 {
    current_reg: u8,
    outer_chr_bank: u8,
    registers: [u8; 8],
}

impl Mapper112 {
    pub fn new() -> Self {
        Self {
            current_reg: 0,
            outer_chr_bank: 0,
            registers: [0; 8],
        }
    }
}

impl Mapper for Mapper112 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = if address >= 0xE000 {
                let num_banks = cart.prg_rom.len() / 0x2000;
                if num_banks > 0 { num_banks - 1 } else { 0 }
            } else if address >= 0xC000 {
                let num_banks = cart.prg_rom.len() / 0x2000;
                if num_banks > 1 { num_banks - 2 } else { 0 }
            } else if address >= 0xA000 {
                self.registers[1] as usize
            } else {
                self.registers[0] as usize
            };
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        match address & 0xE001 {
            0x8000 => {
                self.current_reg = data & 0x07;
            }
            0xA000 => {
                let reg = self.current_reg as usize;
                if reg < 8 {
                    self.registers[reg] = data;
                }
            }
            0xC000 => {
                self.outer_chr_bank = data;
            }
            0xE000 => {
            }
            _ => {}
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
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = if address < 0x0800 {
                (self.registers[2] as usize) + ((address as usize >> 10) & 1)
            } else if address < 0x1000 {
                (self.registers[3] as usize) + ((address as usize >> 10) & 1)
            } else if address < 0x1400 {
                (self.registers[4] as usize) | (((self.outer_chr_bank & 0x10) as usize) << 4)
            } else if address < 0x1800 {
                (self.registers[5] as usize) | (((self.outer_chr_bank & 0x20) as usize) << 3)
            } else if address < 0x1C00 {
                (self.registers[6] as usize) | (((self.outer_chr_bank & 0x40) as usize) << 2)
            } else {
                (self.registers[7] as usize) | (((self.outer_chr_bank & 0x80) as usize) << 1)
            };
            let chr_offset = bank * 0x0400 + (address as usize & 0x03FF);
            if using_chr_ram && !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[chr_offset % chr_ram.len()] as u16;
            } else if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[chr_offset % chr_rom.len()] as u16;
            }
        } else {
            let mirrored = if nametable_horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let bank = if address < 0x0800 {
                    (self.registers[2] as usize) + ((address as usize >> 10) & 1)
                } else if address < 0x1000 {
                    (self.registers[3] as usize) + ((address as usize >> 10) & 1)
                } else if address < 0x1400 {
                    (self.registers[4] as usize) | (((self.outer_chr_bank & 0x10) as usize) << 4)
                } else if address < 0x1800 {
                    (self.registers[5] as usize) | (((self.outer_chr_bank & 0x20) as usize) << 3)
                } else if address < 0x1C00 {
                    (self.registers[6] as usize) | (((self.outer_chr_bank & 0x40) as usize) << 2)
                } else {
                    (self.registers[7] as usize) | (((self.outer_chr_bank & 0x80) as usize) << 1)
                };
                let offset = bank * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[mirrored as usize & 0x7FF] = data;
            }
        }
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.registers);
        state.push(self.current_reg);
        state.push(self.outer_chr_bank);
        if cart.using_chr_ram {
            state.extend_from_slice(&cart.chr_ram);
        }
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..8 {
            if p < state.len() {
                self.registers[i] = state[p];
                p += 1;
            }
        }
        if p < state.len() {
            self.current_reg = state[p];
            p += 1;
        }
        if p < state.len() {
            self.outer_chr_bank = state[p];
            p += 1;
        }
        if cart.using_chr_ram {
            for i in 0..cart.chr_ram.len() {
                if p < state.len() {
                    cart.chr_ram[i] = state[p];
                    p += 1;
                }
            }
        }
        p
    }

    fn reset(&mut self) {
        self.current_reg = 0;
        self.outer_chr_bank = 0;
        self.registers = [0; 8];
    }
}
