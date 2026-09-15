use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper527 {
    prg: [u8; 2],
    chr: [u16; 8],
    mirroring: u8,
}

impl Mapper527 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self {
            prg: [0, 1],
            chr: [0; 8],
            mirroring: 0,
        }
    }
}

impl Mapper for Mapper527 {
    fn reset(&mut self) {
        self.prg = [0, 1];
        self.chr = [0; 8];
        self.mirroring = 0;
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x6000..0x8000).contains(&address) {
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
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }

            let page = (address as usize - 0x8000) / 0x2000;
            let bank = match page {
                0 => (self.prg[0] & 0x1F) as usize,
                1 => (self.prg[1] & 0x1F) as usize,
                2 => 0x1E,
                3 => 0x1F,
                _ => 0,
            };

            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) {
            if !cart.prg_ram.is_empty() {
                let offset = (address - 0x6000) as usize;
                let len = cart.prg_ram.len();
                cart.prg_ram[offset % len] = data;
            }
        } else if address >= 0x8000 {
            match address & 0xF000 {
                0x8000 => {
                    self.prg[0] = data & 0x1F;
                }
                0x9000 => {
                    self.mirroring = data & 1;
                }
                0xA000 => {
                    self.prg[1] = data & 0x1F;
                }
                0xB000..=0xE000 => {
                    let bank_idx = (((address >> 12) & 0xF) - 0xB) as usize;
                    let slot = (bank_idx << 1) | if (address & 0x02) != 0 { 1 } else { 0 };
                    if (address & 0x01) != 0 {
                        self.chr[slot] = (self.chr[slot] & 0x0F) | (((data as u16) & 0x0F) << 4);
                    } else {
                        self.chr[slot] = (self.chr[slot] & 0xFF0) | ((data as u16) & 0x0F);
                    }
                }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else {
            let quadrant = (address >> 10) & 3;
            let nt_bank = match quadrant {
                0 | 1 => ((self.chr[0] >> 7) & 1) as u16,
                2 | 3 => ((self.chr[1] >> 7) & 1) as u16,
                _ => 0,
            };
            0x2000 | (nt_bank << 10) | (address & 0x03FF)
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        using_chr_ram: bool,
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
            let bank = (address >> 10) as usize & 7;
            let chr_page = (self.chr[bank] & 0xFFF) as usize;
            let offset = chr_page * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else {
                let quadrant = (address >> 10) & 3;
                let nt_bank = match quadrant {
                    0 | 1 => ((self.chr[0] >> 7) & 1) as u16,
                    2 | 3 => ((self.chr[1] >> 7) & 1) as u16,
                    _ => 0,
                };
                0x2000 | (nt_bank << 10) | (address & 0x03FF)
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
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let bank = (address >> 10) as usize & 7;
                let chr_page = (self.chr[bank] & 0xFFF) as usize;
                let offset = chr_page * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = if cart.alternative_nametable_arrangement {
                address
            } else {
                let quadrant = (address >> 10) & 3;
                let nt_bank = match quadrant {
                    0 | 1 => ((self.chr[0] >> 7) & 1) as u16,
                    2 | 3 => ((self.chr[1] >> 7) & 1) as u16,
                    _ => 0,
                };
                0x2000 | (nt_bank << 10) | (address & 0x03FF)
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
        state.extend_from_slice(&self.prg);
        for &c in &self.chr {
            state.extend_from_slice(&c.to_le_bytes());
        }
        state.push(self.mirroring);
        state.extend_from_slice(&cart.prg_ram);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.prg[0] = state[p];
            self.prg[1] = state[p + 1];
            p += 2;
        }
        if p + 16 <= state.len() {
            for i in 0..8 {
                self.chr[i] = u16::from_le_bytes([state[p], state[p + 1]]);
                p += 2;
            }
        }
        if p < state.len() {
            self.mirroring = state[p];
            p += 1;
        }
        if p < state.len() && !cart.prg_ram.is_empty() {
            let copy_len = cart.prg_ram.len().min(state.len() - p);
            cart.prg_ram[..copy_len].copy_from_slice(&state[p..p + copy_len]);
            p += copy_len;
        }
        p
    }
}
