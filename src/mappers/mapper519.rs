use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

pub struct Mapper519 {
    latch_addr: u16,
    latch_data: u8,
    scratch: [u8; 4],
    dip_switch: u8,
}

impl Mapper519 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self {
            latch_addr: 0,
            latch_data: 0,
            scratch: [0; 4],
            dip_switch: 0,
        }
    }
}

impl Mapper for Mapper519 {
    fn reset(&mut self) {
        self.latch_addr = 0;
        self.latch_data = 0;
        self.scratch = [0; 4];
    }

    fn reset_power_cycle(&mut self) {
        self.latch_addr = 0;
        self.latch_data = 0;
        self.scratch = [0; 4];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x5000..0x6000).contains(&address) {
            if (address & 0x0800) != 0 {
                FetchResult {
                    data: self.scratch[(address & 3) as usize],
                    driven: true,
                }
            } else {
                FetchResult {
                    data: 0,
                    driven: false,
                }
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
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }

            let effective_address = if (self.latch_addr & 0x40) != 0 {
                (address & !0x000F) | ((self.dip_switch as u16) & 0x000F)
            } else {
                address
            };

            let offset = if (self.latch_addr & 0x80) != 0 {
                (self.latch_addr as usize) * 0x4000 + (effective_address as usize & 0x3FFF)
            } else {
                ((self.latch_addr >> 1) as usize) * 0x8000 + (effective_address as usize & 0x7FFF)
            };

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
        if (0x5000..0x6000).contains(&address) {
            if (address & 0x0800) != 0 {
                self.scratch[(address & 3) as usize] = data;
            }
        } else if (0x6000..0x8000).contains(&address) {
            if !cart.prg_ram.is_empty() {
                let offset = (address - 0x6000) as usize;
                let len = cart.prg_ram.len();
                cart.prg_ram[offset % len] = data;
            }
        } else if address >= 0x8000 {
            if (address & 0x0100) != 0 {
                self.latch_data = (data & 3) | (self.latch_data & !3);
            } else {
                self.latch_addr = address;
                self.latch_data = data;
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else {
            let h_mirror = (self.latch_data & 0x80) != 0;
            mirror_h_or_v(h_mirror, address)
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
            let offset = (self.latch_data as usize) * 0x2000 + (address as usize & 0x1FFF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let h_mirror = (self.latch_data & 0x80) != 0;
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
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let offset = (self.latch_data as usize) * 0x2000 + (address as usize & 0x1FFF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let h_mirror = (self.latch_data & 0x80) != 0;
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

    fn get_dip_switches(&self) -> u8 {
        self.dip_switch
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switch = value;
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.latch_addr.to_le_bytes());
        state.push(self.latch_data);
        state.extend_from_slice(&self.scratch);
        state.push(self.dip_switch);
        state.extend_from_slice(&cart.prg_ram);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.latch_addr = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.latch_data = state[p];
            p += 1;
        }
        if p + 4 <= state.len() {
            self.scratch.copy_from_slice(&state[p..p + 4]);
            p += 4;
        }
        if p < state.len() {
            self.dip_switch = state[p];
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
