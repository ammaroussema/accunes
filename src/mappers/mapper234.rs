use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper234 {
    regs: [u8; 3],
}

impl Mapper234 {
    pub fn new() -> Self {
        Self { regs: [0; 3] }
    }

    fn sync(&self) -> (usize, usize) {
        let mode_nina = self.regs[0] & 0x40 != 0;
        let prg = if mode_nina {
            (self.regs[0] & 0x0E | self.regs[2] & 0x01) as usize
        } else {
            (self.regs[0] & 0x0F) as usize
        };
        let chr = if mode_nina {
            ((self.regs[0] as usize) << 2 & 0x38) | ((self.regs[2] as usize) >> 4 & 0x07)
        } else {
            ((self.regs[0] as usize) << 2 & 0x3C) | ((self.regs[2] as usize) >> 4 & 0x03)
        };
        let prg = if self.regs[0] & 0x10 == 0 {
            prg | ((self.regs[0] as usize >> 1) & 0x10)
        } else {
            prg
        };
        let chr = if self.regs[0] & 0x10 == 0 {
            chr | (((self.regs[0] as usize) << 1) & 0x40)
        } else {
            chr
        };
        (prg, chr)
    }

    fn mirrored_addr(&self, address: u16) -> u16 {
        if self.regs[0] & 0x80 != 0 {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn update_regs_on_read(&mut self, addr: usize, data: u8) {
        if addr >= 0xFF80 && addr < 0xFFA0 && self.regs[0] & 0x3F == 0 {
            self.regs[0] = data;
        } else if addr >= 0xFFC0 && addr < 0xFFE0 && self.regs[0] & 0x3F == 0 {
            self.regs[1] = data;
        } else if addr >= 0xFFE8 && addr < 0xFFF8 {
            self.regs[2] = data & 0x71;
        }
    }

    fn update_regs_on_write(&mut self, addr: usize, data: u8) {
        if addr >= 0xFF80 && addr < 0xFFA0 {
            if self.regs[0] & 0x3F == 0 {
                self.regs[0] = data;
            }
        } else if addr >= 0xFFE8 && addr < 0xFFF8 {
            self.regs[2] = data & 0x71;
        }
    }
}

impl Mapper for Mapper234 {
    fn reset(&mut self) {
        self.regs = [0; 3];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let (prg, _) = self.sync();
            let offset = prg * 0x8000 + (address as usize & 0x7FFF);
            let data = if cart.prg_rom.is_empty() { 0 } else { cart.prg_rom[offset % cart.prg_rom.len()] };
            self.update_regs_on_read(address as usize, data);
            return FetchResult { data, driven: true };
        }
        if address >= 0x6000 {
            return FetchResult { data: 0, driven: false };
        }
        let addr = address as usize;
        let prg_data = if !cart.prg_rom.is_empty() {
            cart.prg_rom[addr % cart.prg_rom.len()]
        } else { 0 };
        self.update_regs_on_read(addr, prg_data);
        FetchResult { data: prg_data, driven: true }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        self.update_regs_on_write(address as usize, data);
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirrored_addr(address)
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
            let (_, chr) = self.sync();
            let offset = chr * 0x2000 + (address as usize & 0x1FFF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            new_addr_bus |= vram[(self.mirrored_addr(address) & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram && !cart.chr_ram.is_empty() {
            let (_, chr) = self.sync();
            let offset = chr * 0x2000 + (address as usize & 0x1FFF);
            let len = cart.chr_ram.len();
            cart.chr_ram[offset % len] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            vram[(self.mirrored_addr(address) & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.regs[0], self.regs[1], self.regs[2]]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.regs[0] = state[p]; p += 1; }
        if p < state.len() { self.regs[1] = state[p]; p += 1; }
        if p < state.len() { self.regs[2] = state[p]; p += 1; }
        p
    }
}
