use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper28 {
    reg: u8,
    chr: u8,
    prg: u8,
    mode: u8,
    outer: u8,
    prg_mask_16k: usize,
}

impl Mapper28 {
    pub fn new(prg_size_16k: usize) -> Self {
        Mapper28 {
            reg: 0,
            chr: 0,
            prg: 15,
            mode: 0,
            outer: 63,
            prg_mask_16k: prg_size_16k,
        }
    }

    fn sync_mirror(&self) -> bool {
        match self.mode & 3 {
            0 => false, 
            1 => true,  
            2 => false, 
            3 => true,  
            _ => false,
        }
    }

    fn mirror(&mut self, value: u8) {
        if (self.mode & 2) == 0 {
            self.mode = (self.mode & 0xFE) | ((value >> 4) & 1);
        }
    }

    fn sync(&self) -> (usize, usize, usize) {
        let outb = (self.outer as usize) << 1;
        let prg = self.prg as usize;
        let (prglo, prghi) = match self.mode & 0x3C {
            0x00 | 0x04 => (outb, outb | 1),
            0x10 | 0x14 => (
                outb & !2 | ((prg << 1) & 2),
                outb & !2 | ((prg << 1) & 2) | 1,
            ),
            0x20 | 0x24 => (
                outb & !6 | ((prg << 1) & 6),
                outb & !6 | ((prg << 1) & 6) | 1,
            ),
            0x30 | 0x34 => (
                outb & !14 | ((prg << 1) & 14),
                outb & !14 | ((prg << 1) & 14) | 1,
            ),
            0x08 => (outb, outb | (prg & 1)),
            0x18 => (outb, outb & !2 | (prg & 3)),
            0x28 => (outb, outb & !6 | (prg & 7)),
            0x38 => (outb, outb & !14 | (prg & 15)),
            0x0C => (outb | (prg & 1), outb | 1),
            0x1C => (outb & !2 | (prg & 3), outb | 1),
            0x2C => (outb & !6 | (prg & 7), outb | 1),
            0x3C => (outb & !14 | (prg & 15), outb | 1),
            _ => (outb, outb | 1),
        };
        let prglo = prglo & self.prg_mask_16k;
        let prghi = prghi & self.prg_mask_16k;
        let chr = (self.chr & 3) as usize;
        (prglo, prghi, chr)
    }
}

impl Mapper for Mapper28 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let (prglo, prghi, _) = self.sync();
            let bank = if address < 0xC000 { prglo } else { prghi };
            let offset = (bank * 0x4000) + (address as usize & 0x3FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if address >= 0x6000 {
            let offset = address as usize & 0x1FFF;
            if offset < cart.prg_ram.len() {
                FetchResult {
                    data: cart.prg_ram[offset],
                    driven: true,
                }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address <= 0x5FFF {
            self.reg = data & 0x81;
        } else if address >= 0x8000 {
            match self.reg {
                0x00 => {
                    self.chr = data & 3;
                    self.mirror(data);
                }
                0x01 => {
                    self.prg = data & 15;
                    self.mirror(data);
                }
                0x80 => {
                    self.mode = data & 63;
                }
                0x81 => {
                    self.outer = data & 63;
                }
                _ => {}
            }
        } else if address >= 0x6000 && address <= 0x7FFF {
            let offset = address as usize & 0x1FFF;
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let horizontal = self.sync_mirror();
        if horizontal {
            let nt = (address >> 10) & 1;
            (address & 0x03FF) | (nt << 10)
        } else {
            let nt = (address >> 11) & 1;
            (address & 0x03FF) | (nt << 10)
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
            let (_, _, chr) = self.sync();
            let offset = (chr * 0x2000) + (address as usize & 0x1FFF);
            if using_chr_ram {
                new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
            } else {
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let idx = (address & 0x7FF) as usize;
            new_addr_bus |= vram[idx] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram {
                let (_, _, chr) = self.sync();
                let offset = (chr * 0x2000) + (address as usize & 0x1FFF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset & (len - 1)] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let idx = (address & 0x7FF) as usize;
            vram[idx] = data;
        }
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.push(self.reg);
        state.push(self.chr);
        state.push(self.prg);
        state.push(self.mode);
        state.push(self.outer);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        let prg_len = cart.prg_ram.len();
        if start + prg_len <= state.len() {
            cart.prg_ram.copy_from_slice(&state[start..start + prg_len]);
            start += prg_len;
        }
        let chr_len = cart.chr_ram.len();
        if start + chr_len <= state.len() {
            cart.chr_ram.copy_from_slice(&state[start..start + chr_len]);
            start += chr_len;
        }
        if start + 1 <= state.len() {
            self.reg = state[start];
            start += 1;
        }
        if start + 1 <= state.len() {
            self.chr = state[start];
            start += 1;
        }
        if start + 1 <= state.len() {
            self.prg = state[start];
            start += 1;
        }
        if start + 1 <= state.len() {
            self.mode = state[start];
            start += 1;
        }
        if start + 1 <= state.len() {
            self.outer = state[start];
            start += 1;
        }
        start
    }
}
