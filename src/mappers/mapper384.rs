use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const PRG_MASK: u8 = 0x0F;
const CHR_MASK: u8 = 0x7F;
pub struct Mapper384 {
    prg_reg: [u8; 2],
    chr_reg: [u8; 8],
    mirr: u8,
    reg_cmd: u8,
    irq_latch: u8,
    irq_count: u16,
    irq_enabled: bool,
    irq_mode: bool,
    acount: u16,
    irq_cmd: u8,
    outer_bank: u8,
}
impl Mapper384 {
    pub fn new() -> Self {
        Self {
            prg_reg: [0, 1],
            chr_reg: [0, 1, 2, 3, 4, 5, 6, 7],
            mirr: 0,
            reg_cmd: 0,
            irq_latch: 0,
            irq_count: 0,
            irq_enabled: false,
            irq_mode: false,
            acount: 0,
            irq_cmd: 0,
            outer_bank: 0,
        }
    }
    fn prg_bank(&self, slot: u16) -> usize {
        let flipped = (self.reg_cmd & 2) != 0;
        let inner = match (slot, flipped) {
            (0, false) => self.prg_reg[0] & PRG_MASK,
            (0, true)  => 0x0E,
            (1, _)     => self.prg_reg[1] & PRG_MASK,
            (2, false) => 0x0E,
            (2, true)  => self.prg_reg[0] & PRG_MASK,
            (3, _)     => 0x0F,
            _ => 0,
        };
        (inner as usize) | ((self.outer_bank as usize) << 4)
    }
    fn prg_read(&self, cart: &Cartridge, address: u16) -> u8 {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }
        let slot = ((address - 0x8000) / 0x2000) as u16;
        let bank = self.prg_bank(slot);
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        cart.prg_rom[offset % len]
    }
    fn vrc4_decode(address: u16) -> u8 {
        let bit0 = if (address & 0x04) != 0 { 1 } else { 0 };
        let bit1 = if (address & 0x08) != 0 { 2 } else { 0 };
        bit1 | bit0
    }
    fn chr_bank(&self, address: u16) -> usize {
        let index = (address >> 10) as usize & 7;
        let inner = (self.chr_reg[index] & CHR_MASK) as usize;
        inner | ((self.outer_bank as usize) << 7)
    }
}
impl Mapper for Mapper384 {
    fn reset(&mut self) {
        self.prg_reg = [0, 1];
        self.chr_reg = [0, 1, 2, 3, 4, 5, 6, 7];
        self.mirr = 0;
        self.reg_cmd = 0;
        self.irq_latch = 0;
        self.irq_count = 0;
        self.irq_enabled = false;
        self.irq_mode = false;
        self.acount = 0;
        self.irq_cmd = 0;
        self.outer_bank = 0;
    }
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            FetchResult {
                data: self.prg_read(cart, address),
                driven: true,
            }
        } else if address >= 0x6000 {
            let idx = (address as usize - 0x6000) & (cart.prg_ram.len() - 1);
            FetchResult {
                data: if !cart.prg_ram.is_empty() { cart.prg_ram[idx] } else { 0 },
                driven: !cart.prg_ram.is_empty(),
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }
    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let idx = (address as usize - 0x6000) & (cart.prg_ram.len() - 1);
                cart.prg_ram[idx] = data;
            }
            if (address & 0x800) != 0 && (self.outer_bank & 0x08) == 0 {
                self.outer_bank = data;
            }
            return;
        }
        if address < 0x8000 {
            return;
        }
        let bank_4k = (address >> 12) & 0xF;
        let reg = Self::vrc4_decode(address);
        match bank_4k {
            0x8 | 0xA => {
                let idx = (address >> 13) & 1;
                self.prg_reg[idx as usize] = data & 0x1F;
            }
            0x9 => {
                match reg {
                    0 | 1 => {
                        if data != 0xFF {
                            self.mirr = data;
                        }
                    }
                    2 => {
                        self.reg_cmd = data;
                    }
                    _ => {}
                }
            }
            0xB | 0xC | 0xD | 0xE => {
                let chr_index = (((bank_4k - 0xB) << 1) | ((address >> 3) & 1)) as usize;
                if (address & 0x04) != 0 {
                    self.chr_reg[chr_index as usize] = (self.chr_reg[chr_index as usize] & 0x0F) | (data << 4);
                } else {
                    self.chr_reg[chr_index as usize] = (self.chr_reg[chr_index as usize] & 0xF0) | (data & 0x0F);
                }
            }
            0xF => {
                match reg {
                    0 => self.irq_latch = (self.irq_latch & 0xF0) | (data & 0x0F),
                    1 => self.irq_latch = (self.irq_latch & 0x0F) | (data << 4),
                    2 => {
                        self.acount = 0;
                        self.irq_count = self.irq_latch as u16;
                        self.irq_mode = (data & 4) != 0;
                        self.irq_enabled = (data & 2) != 0;
                        self.irq_cmd = if (data & 1) != 0 { 1 } else { 0 };
                    }
                    3 => {
                        self.irq_enabled = self.irq_cmd != 0;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        match self.mirr & 3 {
            0 => address & 0x37FF,
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            2 => address & 0x3FFF,
            3 => (address & 0x3FFF) | 0x0400,
            _ => address,
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
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let chr_bank = self.chr_bank(address);
            let offset = chr_bank * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset & (chr_ram.len() - 1)]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = match self.mirr & 3 {
                0 => address & 0x37FF,
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                2 => address & 0x3FFF,
                3 => (address & 0x3FFF) | 0x0400,
                _ => address,
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }
    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let chr_bank = self.chr_bank(address);
                let offset = chr_bank * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset & (len - 1)] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }
    fn ppu_clock(
        &mut self,
        _ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        _scanline: u16,
        _dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        const LCYCS: u16 = 341;
        if self.irq_enabled && !self.irq_mode {
            self.acount += 3;
            if self.acount >= LCYCS {
                while self.acount >= LCYCS {
                    self.acount -= LCYCS;
                    self.irq_count += 1;
                    if self.irq_count & 0x100 != 0 {
                        self.irq_count = self.irq_latch as u16;
                        return true;
                    }
                }
            }
        }
        false
    }
    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.irq_enabled && self.irq_mode {
            self.acount += cycles as u16;
            while self.acount > 0 {
                self.acount -= 1;
                self.irq_count += 1;
                if self.irq_count & 0x100 != 0 {
                    self.irq_count = self.irq_latch as u16;
                    return true;
                }
            }
        }
        false
    }
    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg_reg);
        state.extend_from_slice(&self.chr_reg);
        state.extend_from_slice(&self.acount.to_le_bytes());
        state.push(self.irq_cmd);
        state.extend_from_slice(&self.irq_count.to_le_bytes());
        state.push(self.irq_latch);
        state.push(self.irq_enabled as u8);
        state.push(self.irq_mode as u8);
        state.push(self.reg_cmd);
        state.push(self.mirr);
        state.push(self.outer_bank);
        state
    }
    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            for i in 0..2 { self.prg_reg[i] = state[p]; p += 1; }
        }
        if p + 8 <= state.len() {
            for i in 0..8 { self.chr_reg[i] = state[p]; p += 1; }
        }
        if p + 2 <= state.len() {
            self.acount = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() { self.irq_cmd = state[p]; p += 1; }
        if p + 2 <= state.len() {
            self.irq_count = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() { self.irq_latch = state[p]; p += 1; }
        if p < state.len() { self.irq_enabled = state[p] != 0; p += 1; }
        if p < state.len() { self.irq_mode = state[p] != 0; p += 1; }
        if p < state.len() { self.reg_cmd = state[p]; p += 1; }
        if p < state.len() { self.mirr = state[p]; p += 1; }
        if p < state.len() { self.outer_bank = state[p]; p += 1; }
        p
    }
}
