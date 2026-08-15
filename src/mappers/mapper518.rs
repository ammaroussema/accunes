use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

pub struct Mapper518 {
    reg: [u8; 2],
    chr: u8,
    secondary_ram: Box<[u8; 128 * 1024]>,
}

impl Mapper518 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self {
            reg: [0; 2],
            chr: 0,
            secondary_ram: Box::new([0u8; 128 * 1024]),
        }
    }
}

impl Mapper for Mapper518 {
    fn reset(&mut self) {
        self.reg = [0; 2];
        self.chr = 0;
    }

    fn reset_power_cycle(&mut self) {
        self.reg = [0; 2];
        self.chr = 0;
        self.secondary_ram.fill(0);
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x5000..0x6000).contains(&address) {
            match address & 0x0F00 {
                0x0300 => FetchResult {
                    data: 0x80,
                    driven: true,
                },
                _ => FetchResult {
                    data: 0,
                    driven: false,
                },
            }
        } else if (0x6000..0x8000).contains(&address) {
            if !cart.prg_ram.is_empty() {
                let offset = (address - 0x6000) as usize;
                FetchResult {
                    data: cart.prg_ram[offset % cart.prg_ram.len()],
                    driven: true,
                }
            } else {
                FetchResult {
                    data: 0,
                    driven: false,
                }
            }
        } else if address >= 0x8000 {
            if (self.reg[0] & 0x80) != 0 {
                if (self.reg[1] & 0x04) != 0 {
                    let offset = (((self.reg[0] as usize) << 15) & 0x18000) | (address as usize & 0x7FFF);
                    FetchResult {
                        data: self.secondary_ram[offset],
                        driven: true,
                    }
                } else if address < 0xC000 {
                    let offset = (((self.reg[0] as usize) << 14) & 0x1C000) | (address as usize & 0x3FFF);
                    FetchResult {
                        data: self.secondary_ram[offset],
                        driven: true,
                    }
                } else {
                    let offset = address as usize & 0x3FFF;
                    let len = cart.prg_rom.len();
                    FetchResult {
                        data: if len > 0 { cart.prg_rom[offset % len] } else { 0 },
                        driven: true,
                    }
                }
            } else {
                let len = cart.prg_rom.len();
                if len == 0 {
                    return FetchResult {
                        data: 0,
                        driven: true,
                    };
                }
                if (self.reg[1] & 0x04) != 0 {
                    let offset = (self.reg[0] as usize) * 0x8000 + (address as usize & 0x7FFF);
                    FetchResult {
                        data: cart.prg_rom[offset % len],
                        driven: true,
                    }
                } else if address < 0xC000 {
                    let offset = (self.reg[0] as usize) * 0x4000 + (address as usize & 0x3FFF);
                    FetchResult {
                        data: cart.prg_rom[offset % len],
                        driven: true,
                    }
                } else {
                    let offset = address as usize & 0x3FFF;
                    FetchResult {
                        data: cart.prg_rom[offset % len],
                        driven: true,
                    }
                }
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x5000..0x6000).contains(&address) {
            match address & 0x0F00 {
                0x0000 => {
                    self.reg[0] = data;
                }
                0x0200 => {
                    self.reg[1] = data;
                }
                _ => {}
            }
        } else if (0x6000..0x8000).contains(&address) {
            if !cart.prg_ram.is_empty() {
                let offset = (address - 0x6000) as usize;
                let len = cart.prg_ram.len();
                cart.prg_ram[offset % len] = data;
            }
        } else if address >= 0x8000 && (self.reg[0] & 0x80) != 0 {
            if (self.reg[1] & 0x04) != 0 {
                let offset = (((self.reg[0] as usize) << 15) & 0x18000) | (address as usize & 0x7FFF);
                self.secondary_ram[offset] = data;
            } else if address < 0xC000 {
                let offset = (((self.reg[0] as usize) << 14) & 0x1C000) | (address as usize & 0x3FFF);
                self.secondary_ram[offset] = data;
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else {
            let h_mirror = (self.reg[1] & 1) != 0;
            mirror_h_or_v(h_mirror, address)
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;

        if !ciram {
            let byte = if address < 0x1000 {
                let bank = if (self.reg[1] & 0x02) != 0 {
                    self.chr as usize
                } else {
                    0
                };
                let offset = bank * 0x1000 + (address as usize & 0x0FFF);
                if !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else {
                    0
                }
            } else {
                let offset = 0x1000 + (address as usize & 0x0FFF);
                if !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else {
                    0
                }
            };
            new_addr_bus |= byte as u16;
        } else {
            if (address & 0x03FF) < 0x03C0 && (self.reg[1] & 0x02) != 0 {
                let nt = ((address >> 10) & 3) as u8;
                let shift = self.reg[1] & 1;
                self.chr = (nt >> shift) & 1;
            }

            let h_mirror = (self.reg[1] & 1) != 0;
            let mirrored = if alternative_nametable_arrangement {
                address
            } else {
                mirror_h_or_v(h_mirror, address)
            };

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
        if address < 0x1000 {
            let offset = address as usize & 0x0FFF;
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else if address < 0x2000 {
            let offset = 0x1000 + (address as usize & 0x0FFF);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let h_mirror = (self.reg[1] & 1) != 0;
            let mirrored = if cart.alternative_nametable_arrangement {
                address
            } else {
                mirror_h_or_v(h_mirror, address)
            };

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
        state.extend_from_slice(&self.reg);
        state.push(self.chr);
        state.extend_from_slice(&self.secondary_ram[..]);
        state.extend_from_slice(&cart.prg_ram);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.reg[0] = state[p];
            self.reg[1] = state[p + 1];
            p += 2;
        }
        if p < state.len() {
            self.chr = state[p];
            p += 1;
        }
        if p + 128 * 1024 <= state.len() {
            self.secondary_ram.copy_from_slice(&state[p..p + 128 * 1024]);
            p += 128 * 1024;
        }
        if p < state.len() && !cart.prg_ram.is_empty() {
            let copy_len = cart.prg_ram.len().min(state.len() - p);
            cart.prg_ram[..copy_len].copy_from_slice(&state[p..p + copy_len]);
            p += copy_len;
        }
        p
    }
}
