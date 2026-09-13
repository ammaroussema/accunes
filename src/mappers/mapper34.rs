use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mapper34Kind {
    NINA001,
    BNROM,
    Nesticle34,
}

pub struct Mapper34 {
    kind: Mapper34Kind,
    regs: [u8; 3],
    latch: u8,
}

impl Mapper34 {
    pub fn new(kind: Mapper34Kind) -> Self {
        Self {
            kind,
            regs: [0; 3],
            latch: 0,
        }
    }
}

impl Mapper for Mapper34 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = if self.kind == Mapper34Kind::BNROM {
                self.latch as usize
            } else {
                self.regs[0] as usize
            };
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if (0x6000..0x8000).contains(&address) {
            let offset = (address - 0x6000) as usize;
            if offset < cart.prg_ram.len() {
                FetchResult {
                    data: cart.prg_ram[offset],
                    driven: true,
                }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if self.kind == Mapper34Kind::BNROM {
            if address >= 0x8000 {
                let bank = self.latch as usize;
                let offset = bank * 0x8000 + (address as usize & 0x7FFF);
                let rom_byte = if !cart.prg_rom.is_empty() {
                    cart.prg_rom[offset % cart.prg_rom.len()]
                } else {
                    0xFF
                };
                self.latch = data & rom_byte;
            } else if address >= 0x6000 {
                let offset = (address - 0x6000) as usize;
                if offset < cart.prg_ram.len() {
                    cart.prg_ram[offset] = data;
                }
            }
            return;
        }
        if address >= 0x8000 {
            if self.kind == Mapper34Kind::Nesticle34 {
                self.regs[0] = data;
            }
        } else if address >= 0x6000 {
            let offset = (address - 0x6000) as usize;
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
            if address >= 0x7FFD {
                self.regs[(address - 0x7FFD) as usize] = data;
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
            let offset = if self.kind == Mapper34Kind::BNROM {
                address as usize & 0x1FFF
            } else {
                let bank = if address < 0x1000 {
                    self.regs[1] as usize
                } else {
                    self.regs[2] as usize
                };
                bank * 0x1000 + (address as usize & 0x0FFF)
            };
            if using_chr_ram {
                if !chr_ram.is_empty() {
                    new_addr_bus |= chr_ram[offset % chr_ram.len()] as u16;
                }
            } else if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else {
            let mirrored = if !nametable_horizontal_mirroring {
                address & 0x37FF
            } else {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&self.regs);
        state.push(self.latch);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..cart.prg_ram.len() {
            if p < state.len() {
                cart.prg_ram[i] = state[p];
                p += 1;
            }
        }
        for i in 0..3 {
            if p < state.len() {
                self.regs[i] = state[p];
                p += 1;
            }
        }
        if p < state.len() {
            self.latch = state[p];
            p += 1;
        }
        p
    }

    fn reset(&mut self) {
        self.regs = [0; 3];
        self.latch = 0;
    }

    fn reset_with_cart(&mut self, cart: &mut Cartridge) {
        self.reset();
        if self.kind == Mapper34Kind::Nesticle34 {
            for (i, &byte) in cart.misc_rom.iter().enumerate() {
                let target = 0x1000 + i;
                if target < cart.prg_ram.len() {
                    cart.prg_ram[target] = byte;
                }
            }
        }
    }
}