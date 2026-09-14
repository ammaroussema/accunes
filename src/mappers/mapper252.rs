use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper252 {
    prg: [u8; 2],
    chr: [u8; 8],
    irq_latch: u8,
    irq_counter: u8,
    irq_clock: u16,
    irq_enabled: bool,
    irq_ack: bool,
}

impl Mapper252 {
    pub fn new() -> Self {
        Self {
            prg: [0; 2],
            chr: [0; 8],
            irq_latch: 0,
            irq_counter: 0,
            irq_clock: 0,
            irq_enabled: false,
            irq_ack: false,
        }
    }
}

impl Mapper for Mapper252 {
    fn reset(&mut self) {
        self.prg = [0; 2];
        self.chr = [0; 8];
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
        if address >= 0xB000 && address <= 0xEFFF {
            let ind = ((((address & 8) | ((address >> 8) as u16)) >> 3) + 2) as usize & 7;
            let sar = (address & 4) as u8;
            self.chr[ind] = (self.chr[ind] & (0xF0 >> sar)) | ((data & 0x0F) << sar);
        } else {
            match address & 0xF00C {
                0x8000 | 0x8004 | 0x8008 | 0x800C => self.prg[0] = data,
                0xA000 | 0xA004 | 0xA008 | 0xA00C => self.prg[1] = data,
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

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else if cart.nametable_horizontal_mirroring {
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
        _using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = (address >> 10) as usize & 0x07;
            let chr_val = self.chr[bank];
            let byte = if chr_val == 6 || chr_val == 7 {
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
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if nametable_horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
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
        let mut state = Vec::with_capacity(14);
        state.extend_from_slice(&self.prg);
        state.extend_from_slice(&self.chr);
        state.push(self.irq_latch);
        state.push(self.irq_counter);
        state.extend_from_slice(&self.irq_clock.to_le_bytes());
        state.push(self.irq_enabled as u8);
        state.push(self.irq_ack as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..2 {
            if p < state.len() { self.prg[i] = state[p]; p += 1; }
        }
        for i in 0..8 {
            if p < state.len() { self.chr[i] = state[p]; p += 1; }
        }
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
