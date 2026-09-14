use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper563 {
    prg: [u8; 2],
    chr: [u16; 8],
    mirroring: u8,
    wires: u8,
    irq_enabled: bool,
    irq_counter: u8,
    irq_active: bool,
}

impl Mapper563 {
    pub fn new() -> Self {
        Self {
            prg: [0, 1],
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            mirroring: 0,
            wires: 0,
            irq_enabled: false,
            irq_counter: 0,
            irq_active: false,
        }
    }
}

impl Mapper for Mapper563 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            let len = cart.prg_ram.len();
            if len > 0 {
                return FetchResult {
                    data: cart.prg_ram[(address as usize & 0x1FFF) % len],
                    driven: true,
                };
            }
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        if address >= 0x8000 {
            let bank8k = match address {
                0x8000..=0x9FFF => self.prg[0] & 0x1F,
                0xA000..=0xBFFF => self.prg[1] & 0x1F,
                0xC000..=0xDFFF => 0x1E,
                _ => 0x1F,
            } as usize;
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            let offset = (bank8k * 0x2000 + (address as usize & 0x1FFF)) % len;
            return FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            let len = cart.prg_ram.len();
            if len > 0 {
                cart.prg_ram[(address as usize & 0x1FFF) % len] = data;
            }
            return;
        }
        match address {
            0x8000..=0x8FFF => {
                self.prg[0] = data;
            }
            0xA000..=0xAFFF => {
                self.prg[1] = data;
            }
            0x9000..=0x9FFF => {
                self.mirroring = data & 3;
            }
            0xB000..=0xEFFF => {
                let bank = (address >> 12) as usize;
                let reg = ((bank - 0xB) << 1) | if address & 0x02 != 0 { 1 } else { 0 };
                if address & 0x01 != 0 {
                    self.chr[reg] = (self.chr[reg] & 0x0F) | ((data as u16) << 4);
                } else {
                    self.chr[reg] = (self.chr[reg] & 0xFF0) | ((data & 0x0F) as u16);
                }
            }
            0xF000..=0xFFFF => {
                self.irq_active = false;
                match address & 0x1C {
                    0x08 => self.irq_enabled = true,
                    0x0C => self.irq_enabled = false,
                    0x1C => self.irq_counter = 0,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mirroring & 1 == 0 {
            address & 0x37FF
        } else {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
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
        let byte;
        if address < 0x2000 {
            let bank = (address >> 10) as usize & 7;
            let chr_bank = (self.chr[bank] & 0xFF) as usize;
            let offset = chr_bank * 0x400 + (address as usize & 0x3FF);
            if !chr_rom.is_empty() {
                byte = chr_rom[offset % chr_rom.len()];
            } else if !chr_ram.is_empty() {
                byte = chr_ram[offset % chr_ram.len()];
            } else {
                byte = 0;
            }
        } else if address < 0x3F00 {
            let mirrored = if self.mirroring & 1 == 0 {
                address & 0x37FF
            } else {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            };
            byte = vram[(mirrored & 0x7FF) as usize];
        } else {
            return (ppu_address_bus as u8, new_addr_bus);
        }
        new_addr_bus |= byte as u16;
        (new_addr_bus as u8, new_addr_bus)
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        ppu_a12_prev: bool,
        _scanline: u16,
        _dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        let a12 = (ppu_address_bus & 0x1000) != 0;
        if ppu_a12_prev && !a12 {
            self.irq_counter = self.irq_counter.wrapping_add(1);
            if self.irq_counter & 15 == 0 && self.irq_enabled {
                self.irq_active = true;
            }
        }
        self.irq_active
    }

    fn cpu_clock_irq_level(&self) -> bool {
        true
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut s = Vec::new();
        s.extend_from_slice(&self.prg);
        for c in &self.chr {
            s.extend_from_slice(&c.to_le_bytes());
        }
        s.push(self.mirroring);
        s.push(self.wires);
        s.push(if self.irq_enabled { 1 } else { 0 });
        s.extend_from_slice(&(self.irq_counter as u16).to_le_bytes());
        s
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.prg.copy_from_slice(&state[p..p + 2]);
            p += 2;
        }
        if p + 16 <= state.len() {
            for i in 0..8 {
                self.chr[i] = u16::from_le_bytes([state[p + i * 2], state[p + i * 2 + 1]]);
            }
            p += 16;
        }
        if p < state.len() {
            self.mirroring = state[p];
            p += 1;
        }
        if p < state.len() {
            self.wires = state[p];
            p += 1;
        }
        if p < state.len() {
            self.irq_enabled = state[p] != 0;
            p += 1;
        }
        if p + 2 <= state.len() {
            self.irq_counter = u16::from_le_bytes([state[p], state[p + 1]]) as u8;
            p += 2;
        }
        p - start
    }

    fn reset(&mut self) {
        self.irq_active = false;
    }

    fn reset_power_cycle(&mut self) {
        self.irq_enabled = false;
        self.irq_counter = 0;
        self.prg = [0, 1];
        self.chr = [0, 1, 2, 3, 4, 5, 6, 7];
        self.wires = 0;
        self.irq_active = false;
    }
}