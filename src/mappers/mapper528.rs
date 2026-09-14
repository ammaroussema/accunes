use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper528 {
    prg: [u8; 3],
    chr: [u8; 8],
    mirroring: u8,
    outer_bank: usize,

    irq_control: u8,
    irq_counter: u8,
    irq_latch: u8,
    irq_cycles: i16,
    irq_ack: bool,
}

impl Mapper528 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self {
            prg: [0; 3],
            chr: [0; 8],
            mirroring: 0,
            outer_bank: 0,

            irq_control: 0,
            irq_counter: 0,
            irq_latch: 0,
            irq_cycles: 0,
            irq_ack: false,
        }
    }
}

impl Mapper for Mapper528 {
    fn reset(&mut self) {
        self.prg = [0; 3];
        self.chr = [0; 8];
        self.mirroring = 0;
        self.outer_bank = 0;

        self.irq_control = 0;
        self.irq_counter = 0;
        self.irq_latch = 0;
        self.irq_cycles = 0;
        self.irq_ack = false;
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        let prg_mask = 0x0F | self.outer_bank;
        let len = cart.prg_rom.len();

        if (0x6000..0x8000).contains(&address) {
            if self.prg[0] == 0x01 {
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
            } else if len > 0 {
                let bank = ((self.prg[0] as usize) & prg_mask) + self.outer_bank;
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
        } else if address >= 0x8000 {
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }

            let page = (address as usize - 0x8000) / 0x2000;
            let bank = match page {
                0 => ((self.prg[1] as usize) & prg_mask) + self.outer_bank,
                1 => ((self.prg[2] as usize) & prg_mask) + self.outer_bank,
                2 => (0xFE & prg_mask) + self.outer_bank,
                3 => (0xFF & prg_mask) + self.outer_bank,
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
            if self.prg[0] == 0x01 && !cart.prg_ram.is_empty() {
                let offset = (address - 0x6000) as usize;
                let len = cart.prg_ram.len();
                cart.prg_ram[offset % len] = data;
            }
        } else if (0xA000..0xB000).contains(&address) || (0xC000..0xD000).contains(&address) {
            let reg = (address & 0x0F) as usize;
            match reg {
                0..=7 => {
                    self.chr[reg] = data;
                }
                8 => {
                    self.prg[0] = data;
                }
                9 => {
                    self.prg[1] = data;
                }
                0xA => {
                    self.prg[2] = data;
                }
                0xC => {
                    self.mirroring = data & 3;
                }
                0xD => {
                    self.irq_control = data;
                    if (self.irq_control & 0x02) != 0 {
                        self.irq_counter = self.irq_latch;
                        self.irq_cycles = 341;
                    }
                    self.irq_ack = true;
                }
                0xE => {
                    if (self.irq_control & 0x01) != 0 {
                        self.irq_control |= 0x02;
                    } else {
                        self.irq_control &= !0x02;
                    }
                    self.irq_ack = true;
                }
                0xF => {
                    self.irq_latch = data;
                }
                _ => {}
            }
            self.outer_bank = if (address >> 12) == 0xC { 0x10 } else { 0x00 };
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else {
            match self.mirroring & 3 {
                0 => address & 0x37FF,                               
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                2 => address & 0x33FF,                              
                3 => (address & 0x33FF) | 0x0400,                    
                _ => address,
            }
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
            let chr_page = (self.chr[bank] as usize) | (self.outer_bank << 4);
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
                match self.mirroring & 3 {
                    0 => address & 0x37FF,
                    1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                    2 => address & 0x33FF,
                    3 => (address & 0x33FF) | 0x0400,
                    _ => address,
                }
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
                let chr_page = (self.chr[bank] as usize) | (self.outer_bank << 4);
                let offset = chr_page * 0x0400 + (address as usize & 0x03FF);
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
                vram[(mirrored & 0x7FF) as usize] = data;
            }
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        let mut fire = false;
        for _ in 0..cycles {
            if (self.irq_control & 0x02) != 0 {
                let cycle_mode = (self.irq_control & 0x04) != 0;
                let mut tick = false;
                if cycle_mode {
                    tick = true;
                } else {
                    self.irq_cycles -= 3;
                    if self.irq_cycles <= 0 {
                        self.irq_cycles += 341;
                        tick = true;
                    }
                }
                if tick {
                    if self.irq_counter == 0xFF {
                        self.irq_counter = self.irq_latch;
                        fire = true;
                    } else {
                        self.irq_counter = self.irq_counter.wrapping_add(1);
                    }
                }
            }
        }
        fire
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack;
        self.irq_ack = false;
        ack
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg);
        state.extend_from_slice(&self.chr);
        state.push(self.mirroring);
        state.push(self.outer_bank as u8);
        state.push(self.irq_control);
        state.push(self.irq_counter);
        state.push(self.irq_latch);
        state.extend_from_slice(&self.irq_cycles.to_le_bytes());
        state.push(self.irq_ack as u8);
        state.extend_from_slice(&cart.prg_ram);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 3 <= state.len() {
            self.prg.copy_from_slice(&state[p..p + 3]);
            p += 3;
        }
        if p + 8 <= state.len() {
            self.chr.copy_from_slice(&state[p..p + 8]);
            p += 8;
        }
        if p < state.len() {
            self.mirroring = state[p];
            p += 1;
        }
        if p < state.len() {
            self.outer_bank = state[p] as usize;
            p += 1;
        }
        if p < state.len() {
            self.irq_control = state[p];
            p += 1;
        }
        if p < state.len() {
            self.irq_counter = state[p];
            p += 1;
        }
        if p < state.len() {
            self.irq_latch = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.irq_cycles = i16::from_le_bytes([state[p], state[p + 1]]);
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
