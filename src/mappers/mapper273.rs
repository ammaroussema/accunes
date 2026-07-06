use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper273 {
    prg_reg: [u8; 2],
    chr_reg: [u8; 8],
    mirr: u8,
    irq_enabled: u8,
    irq_counter: u8,
    irq_prescaler: u8,
    irq_mask: u8,
    irq_pending: bool,
    irq_ack_requested: bool,
}

impl Mapper273 {
    pub fn new() -> Self {
        Self {
            prg_reg: [0; 2],
            chr_reg: [0; 8],
            mirr: 0,
            irq_enabled: 0,
            irq_counter: 0,
            irq_prescaler: 0,
            irq_mask: 0,
            irq_pending: false,
            irq_ack_requested: false,
        }
    }

    fn decode_address(&self, address: u16) -> u16 {
        let base = address & 0xF000;
        let bit1 = if address & 0x08 != 0 { 2 } else { 0 };
        let bit0 = if address & 0x04 != 0 { 1 } else { 0 };
        base | bit1 | bit0
    }

    fn mirror_address(&self, address: u16) -> u16 {
        match self.mirr & 3 {
            0 => address & 0x37FF,
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            2 => address & 0x3FFF,
            3 => (address & 0x3FFF) | 0x0400,
            _ => address,
        }
    }
}

impl Mapper for Mapper273 {
    fn reset(&mut self) {
        self.prg_reg = [0; 2];
        self.chr_reg = [0; 8];
        self.mirr = 0;
        self.irq_enabled = 0;
        self.irq_counter = 0;
        self.irq_prescaler = 0;
        self.irq_mask = 0;
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
                0x8000..=0x9FFF => (self.prg_reg[0] & 0x1F) as usize % num_8k,
                0xA000..=0xBFFF => (self.prg_reg[1] & 0x1F) as usize % num_8k,
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
        // IRQ registers at $F000-$FFFF
        if (address & 0xF000) == 0xF000 {
            if (address & 0x08) == 0 {
                self.irq_counter = data;
                self.irq_pending = false;
                self.irq_ack_requested = true;
            } else {
                self.irq_enabled = data;
                if (self.irq_enabled & 1) == 0 {
                    self.irq_prescaler = 0;
                    self.irq_mask = 0x7F;
                    self.irq_pending = false;
                    self.irq_ack_requested = true;
                }
            }
            return;
        }
        let decoded = self.decode_address(address);
        match decoded & 0xF003 {
            0x8000 | 0x8001 | 0x8002 | 0x8003 => {
                self.prg_reg[0] = data & 0x1F;
            }
            0xA000 | 0xA001 | 0xA002 | 0xA003 => {
                self.prg_reg[1] = data & 0x1F;
            }
            0x9000 | 0x9001 => {
                self.mirr = data;
            }
            _ => {
                if decoded >= 0xB000 && decoded <= 0xE003 {
                    let i = (((decoded >> 1) & 1) | ((decoded - 0xB000) >> 11)) as usize;
                    if i < 8 {
                        let nibble = (decoded & 1) << 2;
                        self.chr_reg[i] = (self.chr_reg[i] & (0xF0 >> nibble)) | ((data & 0xF) << nibble);
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
            let bank = self.chr_reg[slot] as usize;
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
            let bank = self.chr_reg[slot] as usize;
            let offset = bank * 0x400 + (address as usize & 0x3FF);
            let len = cart.chr_ram.len();
            cart.chr_ram[offset % len] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_address(address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if (self.irq_enabled & 1) != 0 {
            let prev = self.irq_prescaler;
            self.irq_prescaler = self.irq_prescaler.wrapping_add(1);
            if (prev & self.irq_mask) == 0 && (self.irq_prescaler & self.irq_mask) == 0 {
                self.irq_mask = 0xFF;
                self.irq_counter = self.irq_counter.wrapping_add(1);
                if self.irq_counter == 0 {
                    self.irq_pending = true;
                }
            }
        }
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
        state.extend_from_slice(&self.prg_reg);
        state.extend_from_slice(&self.chr_reg);
        state.push(self.mirr);
        state.push(self.irq_enabled);
        state.push(self.irq_counter);
        state.push(self.irq_prescaler);
        state.push(self.irq_mask);
        state.push(self.irq_pending as u8);
        state.push(self.irq_ack_requested as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.prg_reg.copy_from_slice(&state[p..p + 2]);
            p += 2;
        }
        if p + 8 <= state.len() {
            self.chr_reg.copy_from_slice(&state[p..p + 8]);
            p += 8;
        }
        if p < state.len() { self.mirr = state[p]; p += 1; }
        if p < state.len() { self.irq_enabled = state[p]; p += 1; }
        if p < state.len() { self.irq_counter = state[p]; p += 1; }
        if p < state.len() { self.irq_prescaler = state[p]; p += 1; }
        if p < state.len() { self.irq_mask = state[p]; p += 1; }
        if p < state.len() { self.irq_pending = state[p] != 0; p += 1; }
        if p < state.len() { self.irq_ack_requested = state[p] != 0; p += 1; }
        p
    }
}
