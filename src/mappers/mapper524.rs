use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

pub struct Mapper524 {
    prg: [u8; 2],
    chr: [u16; 8],
    mirroring: u8,
    irq_enabled: bool,
    irq_counter: u16,
    irq_ack: bool,
}

impl Mapper524 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self {
            prg: [0, 1],
            chr: [0; 8],
            mirroring: 0,
            irq_enabled: false,
            irq_counter: 0,
            irq_ack: false,
        }
    }
}

impl Mapper for Mapper524 {
    fn reset(&mut self) {
        self.prg = [0, 1];
        self.chr = [0; 8];
        self.mirroring = 0;
        self.irq_enabled = false;
        self.irq_counter = 0;
        self.irq_ack = false;
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
                        self.chr[slot] = (self.chr[slot] & 0xF0) | ((data as u16) & 0x0F);
                    }
                }
                0xF000 => match address & 0x000C {
                    0x0008 => {
                        self.irq_enabled = true;
                    }
                    0x000C => {
                        self.irq_enabled = false;
                        self.irq_counter = 0;
                        self.irq_ack = true;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else {
            let h_mirror = (self.mirroring & 1) != 0;
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
            let bank = (address >> 10) as usize & 7;
            let chr_page = (self.chr[bank] & 0xFF) as usize;
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
            let h_mirror = (self.mirroring & 1) != 0;
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
                let bank = (address >> 10) as usize & 7;
                let chr_page = (self.chr[bank] & 0xFF) as usize;
                let offset = chr_page * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let h_mirror = (self.mirroring & 1) != 0;
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

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.irq_enabled {
            let mut fire = false;
            for _ in 0..cycles {
                self.irq_counter = self.irq_counter.wrapping_add(1);
                if (self.irq_counter & 1024) != 0 {
                    fire = true;
                }
            }
            return fire;
        }
        false
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack;
        self.irq_ack = false;
        ack
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg);
        for &c in &self.chr {
            state.extend_from_slice(&c.to_le_bytes());
        }
        state.push(self.mirroring);
        state.push(self.irq_enabled as u8);
        state.extend_from_slice(&self.irq_counter.to_le_bytes());
        state.push(self.irq_ack as u8);
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
        if p < state.len() {
            self.irq_enabled = state[p] != 0;
            p += 1;
        }
        if p + 2 <= state.len() {
            self.irq_counter = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.irq_ack = state[p] != 0;
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
