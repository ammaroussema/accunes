use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper222 {
    prg_reg: [u8; 2],
    chr_reg: [u8; 8],
    mirr: u8,
    irq_counter: u8,
    irq_enabled: bool,
    prev_scanline: u16,
}

impl Mapper222 {
    pub fn new() -> Self {
        Mapper222 {
            prg_reg: [0; 2],
            chr_reg: [0; 8],
            mirr: 0,
            irq_counter: 0,
            irq_enabled: false,
            prev_scanline: 0,
        }
    }
}

impl Mapper for Mapper222 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = match address {
                0x8000..=0x9FFF => self.prg_reg[0] as u16,
                0xA000..=0xBFFF => self.prg_reg[1] as u16,
                0xC000..=0xDFFF => ((cart.prg_rom.len() / 0x2000 - 2) as u8) as u16,
                0xE000..=0xFFFF => ((cart.prg_rom.len() / 0x2000 - 1) as u8) as u16,
                _ => 0,
            };
            let offset = (bank as usize * 0x2000) + (address as usize & 0x1FFF);
            FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            match address & 0xF003 {
                0x8000 | 0x8001 | 0x8002 | 0x8003 => {
                    self.prg_reg[0] = data & 0x1F;
                }
                0x9000 | 0x9001 => {
                    self.mirr = data & 1;
                }
                0xA000 | 0xA001 | 0xA002 | 0xA003 => {
                    self.prg_reg[1] = data & 0x1F;
                }
                0xB000 => self.chr_reg[0] = data,
                0xB002 => self.chr_reg[1] = data,
                0xC000 => self.chr_reg[2] = data,
                0xC002 => self.chr_reg[3] = data,
                0xD000 => self.chr_reg[4] = data,
                0xD002 => self.chr_reg[5] = data,
                0xE000 => self.chr_reg[6] = data,
                0xE002 => self.chr_reg[7] = data,
                0xF000 => {
                    self.irq_counter = data;
                    self.irq_enabled = true;
                }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mirr & 1 != 0 {
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
            let bank = (address >> 10) as usize & 0x07;
            let chr_bank = self.chr_reg[bank] as usize;
            let offset = (chr_bank * 0x400) + (address as usize & 0x3FF);
            let data = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset & (chr_ram.len() - 1)]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else if !chr_ram.is_empty() {
                chr_ram[offset & (chr_ram.len() - 1)]
            } else {
                0
            };
            new_addr_bus |= data as u16;
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = if self.mirr & 1 != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let idx = (mirrored & 0x7FF) as usize;
            new_addr_bus |= vram[idx] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn ppu_clock(
        &mut self,
        _ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        scanline: u16,
        _dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        if !self.irq_enabled {
            self.prev_scanline = scanline;
            return false;
        }
        if scanline != self.prev_scanline && scanline < 240 {
            self.prev_scanline = scanline;
            self.irq_counter = self.irq_counter.wrapping_add(1);
            if self.irq_counter >= 238 {
                return true;
            }
        } else {
            self.prev_scanline = scanline;
        }
        false
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        false
    }

    fn take_irq_ack(&mut self) -> bool {
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg_reg);
        state.extend_from_slice(&self.chr_reg);
        state.push(self.mirr);
        state.push(self.irq_counter);
        state.push(self.irq_enabled as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 12 <= state.len() {
            for i in 0..2 {
                self.prg_reg[i] = state[start];
                start += 1;
            }
            for i in 0..8 {
                self.chr_reg[i] = state[start];
                start += 1;
            }
            self.mirr = state[start];
            start += 1;
            self.irq_counter = state[start];
            start += 1;
            self.irq_enabled = state[start] != 0;
            start += 1;
        }
        start
    }

    fn reset(&mut self) {
        self.prg_reg = [0; 2];
        self.chr_reg = [0; 8];
        self.mirr = 0;
        self.irq_counter = 0;
        self.irq_enabled = false;
        self.prev_scanline = 0;
    }
}
