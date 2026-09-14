use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper253 {
    chr_lo: [u8; 8],
    chr_hi: [u8; 8],
    prg: [u8; 2],
    mirr: u8,
    vlock: bool,
    irq_latch: u8,
    irq_counter: u8,
    irq_clock: u16,
    irq_enabled: bool,
    irq_ack: bool,
}

impl Mapper253 {
    pub fn new() -> Self {
        Self {
            chr_lo: [0; 8],
            chr_hi: [0; 8],
            prg: [0; 2],
            mirr: 0,
            vlock: false,
            irq_latch: 0,
            irq_counter: 0,
            irq_clock: 0,
            irq_enabled: false,
            irq_ack: false,
        }
    }
}

impl Mapper for Mapper253 {
    fn reset(&mut self) {
        self.chr_lo = [0; 8];
        self.chr_hi = [0; 8];
        self.prg = [0; 2];
        self.mirr = 0;
        self.vlock = false;
        self.irq_latch = 0;
        self.irq_counter = 0;
        self.irq_clock = 0;
        self.irq_enabled = false;
        self.irq_ack = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            let off = (address - 0x6000) as usize;
            if off < cart.prg_ram.len() {
                return FetchResult { data: cart.prg_ram[off], driven: true };
            }
            return FetchResult { data: 0, driven: false };
        }
        if address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }
        let len = cart.prg_rom.len();
        if len == 0 {
            return FetchResult { data: 0, driven: false };
        }
        let bank = match address {
            0xE000..=0xFFFF => (len / 0x2000).saturating_sub(1),
            0xC000..=0xDFFF => (len / 0x2000).saturating_sub(2),
            0xA000..=0xBFFF => self.prg[1] as usize,
            _ => self.prg[0] as usize,
        } % (len / 0x2000);
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        FetchResult { data: cart.prg_rom[offset % len], driven: true }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            let off = (address - 0x6000) as usize;
            if off < cart.prg_ram.len() {
                cart.prg_ram[off] = data;
            }
            return;
        }
        if address < 0x8000 {
            return;
        }
        if address >= 0xB000 && address <= 0xE00C {
            let ind = ((((address & 8) | ((address >> 8) as u16)) >> 3) + 2) as usize & 7;
            let sar = (address & 4) != 0;
            let clo = (self.chr_lo[ind] & (if sar { 0x0F } else { 0xF0 })) | ((data & 0x0F) << if sar { 4 } else { 0 });
            self.chr_lo[ind] = clo;
            if ind == 0 {
                if clo == 0xC8 {
                    self.vlock = false;
                } else if clo == 0x88 {
                    self.vlock = true;
                }
            }
            if sar {
                self.chr_hi[ind] = data >> 4;
            }
        } else {
            match address {
                0x8010 => self.prg[0] = data,
                0xA010 => self.prg[1] = data,
                0x9400 => self.mirr = data & 0x03,
                0xF000 => {
                    self.irq_ack = true;
                    self.irq_latch = (self.irq_latch & 0xF0) | (data & 0x0F);
                }
                0xF004 => {
                    self.irq_ack = true;
                    self.irq_latch = (self.irq_latch & 0x0F) | (data << 4);
                }
                0xF008 => {
                    self.irq_ack = true;
                    self.irq_clock = 0;
                    self.irq_counter = self.irq_latch;
                    self.irq_enabled = (data & 0x02) != 0;
                }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        match self.mirr {
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
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = (address >> 10) as usize & 0x07;
            let chr_val = (self.chr_hi[bank] as u16) << 8 | self.chr_lo[bank] as u16;
            let byte = if (self.chr_lo[bank] == 4 || self.chr_lo[bank] == 5) && !self.vlock {
                let offset = ((chr_val & 1) as usize) * 0x400 + (address as usize & 0x3FF);
                if !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else {
                    0
                }
            } else {
                let offset = (chr_val as usize) * 0x400 + (address as usize & 0x3FF);
                if !chr_rom.is_empty() {
                    chr_rom[offset % chr_rom.len()]
                } else {
                    0
                }
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = match self.mirr {
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
        if address < 0x2000 && !cart.chr_ram.is_empty() {
            let len = cart.chr_ram.len();
            cart.chr_ram[(address as usize) % len] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if !self.irq_enabled {
            return false;
        }
        const LCYCS: u16 = 341;
        self.irq_clock += 3;
        if self.irq_clock >= LCYCS {
            self.irq_clock -= LCYCS;
            let (new_count, overflow) = self.irq_counter.overflowing_add(1);
            if overflow {
                self.irq_counter = self.irq_latch;
                return true;
            }
            self.irq_counter = new_count;
        }
        false
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack;
        self.irq_ack = false;
        ack
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::with_capacity(23);
        state.extend_from_slice(&self.chr_lo);
        state.extend_from_slice(&self.chr_hi);
        state.extend_from_slice(&self.prg);
        state.push(self.mirr);
        state.push(self.vlock as u8);
        state.push(self.irq_latch);
        state.push(self.irq_counter);
        state.extend_from_slice(&self.irq_clock.to_le_bytes());
        state.push(self.irq_enabled as u8);
        state.push(self.irq_ack as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..8 { if p < state.len() { self.chr_lo[i] = state[p]; p += 1; } }
        for i in 0..8 { if p < state.len() { self.chr_hi[i] = state[p]; p += 1; } }
        for i in 0..2 { if p < state.len() { self.prg[i] = state[p]; p += 1; } }
        if p < state.len() { self.mirr = state[p] & 3; p += 1; }
        if p < state.len() { self.vlock = state[p] != 0; p += 1; }
        if p < state.len() { self.irq_latch = state[p]; p += 1; }
        if p < state.len() { self.irq_counter = state[p]; p += 1; }
        if p + 1 < state.len() {
            self.irq_clock = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() { self.irq_enabled = state[p] != 0; p += 1; }
        if p < state.len() { self.irq_ack = state[p] != 0; p += 1; }
        p
    }
}
