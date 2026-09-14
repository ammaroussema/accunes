use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper272 {
    prg: [u8; 2],
    chr: [u8; 8],
    mirr_s: u8,
    mirr_hv: u8,
    irq_enabled: bool,
    irq_counter: u8,
    last_a13: bool,
    irq_pending: bool,
    irq_ack_requested: bool,
}

impl Mapper272 {
    pub fn new() -> Self {
        Self {
            prg: [0; 2],
            chr: [0; 8],
            mirr_s: 0,
            mirr_hv: 0,
            irq_enabled: false,
            irq_counter: 0,
            last_a13: false,
            irq_pending: false,
            irq_ack_requested: false,
        }
    }

    fn mirror_address(&self, address: u16) -> u16 {
        match self.mirr_s {
            2 => address & 0x33FF,
            3 => (address & 0x33FF) | 0x0400,
            _ => {
                if self.mirr_hv != 0 {
                    (address & 0x33FF) | ((address & 0x0800) >> 1)
                } else {
                    address & 0x37FF
                }
            }
        }
    }
}

impl Mapper for Mapper272 {
    fn reset(&mut self) {
        self.prg = [0; 2];
        self.chr = [0; 8];
        self.mirr_s = 0;
        self.mirr_hv = 0;
        self.irq_enabled = false;
        self.irq_counter = 0;
        self.last_a13 = false;
        self.irq_pending = false;
        self.irq_ack_requested = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let num_8k = cart.prg_rom.len() / 0x2000;
            if num_8k == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let bank = match address {
                0x8000..=0x9FFF => self.prg[0] as usize % num_8k,
                0xA000..=0xBFFF => self.prg[1] as usize % num_8k,
                0xC000..=0xDFFF => (num_8k - 2).max(0) % num_8k,
                0xE000..=0xFFFF => (num_8k - 1).max(0) % num_8k,
                _ => 0,
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
        if address < 0x8000 {
            return;
        }
        let bank = (address >> 12) as u8;
        let addr_bits = (address & 0x000C) as u8;
        match bank & 0x0C {
            0x08 => {
                if addr_bits == 0x04 {
                    self.mirr_s = data & 3;
                } else if addr_bits == 0x0C {
                    self.irq_pending = false;
                    self.irq_ack_requested = true;
                }
            }
            0x0C => {
                if addr_bits == 0x04 {
                    self.irq_pending = false;
                    self.irq_ack_requested = true;
                } else if addr_bits == 0x08 {
                    self.irq_enabled = true;
                } else if addr_bits == 0x0C {
                    self.irq_enabled = false;
                    self.irq_counter = 0;
                    self.irq_pending = false;
                    self.irq_ack_requested = true;
                }
            }
            _ => {}
        }
        match bank {
            0x08 => self.prg[0] = data,
            0x09 => self.mirr_hv = data & 1,
            0x0A => self.prg[1] = data,
            0x0F => {}
            _ => {
                let reg = ((bank.wrapping_sub(0x0B)) << 1) + ((address as u8 >> 1) & 1);
                if reg < 8 {
                    if (address & 1) == 0 {
                        self.chr[reg as usize] = (self.chr[reg as usize] & 0xF0) | (data & 0x0F);
                    } else {
                        self.chr[reg as usize] = (self.chr[reg as usize] & 0x0F) | (data << 4);
                    }
                }
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror_address(address)
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
            let slot = (address >> 10) as usize & 7;
            let bank = self.chr[slot] as usize;
            let offset = bank * 0x400 + (address as usize & 0x3FF);
            let byte = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[offset % chr_ram.len()] }
            } else if chr_rom.is_empty() {
                0
            } else {
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = self.mirror_address(address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram && !cart.chr_ram.is_empty() {
            let slot = (address >> 10) as usize & 7;
            let bank = self.chr[slot] as usize;
            let offset = bank * 0x400 + (address as usize & 0x3FF);
            let len = cart.chr_ram.len();
            cart.chr_ram[offset % len] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_address(address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        _scanline: u16,
        _dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        let a13 = (ppu_address_bus & 0x2000) != 0;
        if self.last_a13 && !a13 && self.irq_enabled {
            self.irq_counter = self.irq_counter.wrapping_add(1);
            if self.irq_counter == 84 {
                self.irq_counter = 0;
                self.irq_pending = true;
            }
        }
        self.last_a13 = a13;
        self.irq_pending
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.irq_ack_requested {
            self.irq_ack_requested = false;
            true
        } else {
            false
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg);
        state.extend_from_slice(&self.chr);
        state.push(self.mirr_s);
        state.push(self.mirr_hv);
        state.push(self.irq_enabled as u8);
        state.push(self.irq_counter);
        state.push(self.last_a13 as u8);
        state.push(self.irq_pending as u8);
        state.push(self.irq_ack_requested as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.prg.copy_from_slice(&state[p..p + 2]);
            p += 2;
        }
        if p + 8 <= state.len() {
            self.chr.copy_from_slice(&state[p..p + 8]);
            p += 8;
        }
        if p < state.len() { self.mirr_s = state[p]; p += 1; }
        if p < state.len() { self.mirr_hv = state[p]; p += 1; }
        if p < state.len() { self.irq_enabled = state[p] != 0; p += 1; }
        if p < state.len() { self.irq_counter = state[p]; p += 1; }
        if p < state.len() { self.last_a13 = state[p] != 0; p += 1; }
        if p < state.len() { self.irq_pending = state[p] != 0; p += 1; }
        if p < state.len() { self.irq_ack_requested = state[p] != 0; p += 1; }
        p
    }
}
