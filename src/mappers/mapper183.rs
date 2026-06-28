use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper183 {
    prg: [u8; 3],
    prg_6000: u8,
    chr: [u8; 8],
    mirr: u8,
    irq_counter: u8,
    irq_enabled: bool,
    irq_scaler: u8,
    irq_pending: bool,
    irq_ack: bool,
}

impl Mapper183 {
    pub fn new() -> Self {
        Self {
            prg: [0, 0, 0],
            prg_6000: 0,
            chr: [0; 8],
            mirr: 0,
            irq_counter: 0,
            irq_enabled: false,
            irq_scaler: 0,
            irq_pending: false,
            irq_ack: false,
        }
    }
}

impl Mapper for Mapper183 {
    fn reset(&mut self) {
        *self = Self::new();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            let len = cart.prg_rom.len();
            let offset = (self.prg_6000 as usize) * 0x2000 + (address as usize & 0x1FFF);
            FetchResult { data: if len > 0 { cart.prg_rom[offset % len] } else { 0 }, driven: true }
        } else if address >= 0x8000 {
            let len = cart.prg_rom.len();
            let slot = ((address - 0x8000) / 0x2000) as usize;
            let bank = match slot {
                0 => self.prg[0] as usize,
                1 => self.prg[1] as usize,
                2 => self.prg[2] as usize,
                _ => 0xFF,
            };
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult { data: if len > 0 { cart.prg_rom[offset % len] } else { 0 }, driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        let masked = address & 0xF80C;
        if (address & 0xF800) == 0x6800 {
            self.prg_6000 = (address & 0x3F) as u8;
        } else if masked >= 0xB000 && masked <= 0xE00C {
            let index = (((address >> 11) - 6) | (address >> 3)) as usize & 7;
            let part = (address & 4) as u8;
            self.chr[index] = (self.chr[index] & (0xF0u8 >> part)) | ((data & 0x0F) << part);
        } else {
            match masked {
                0x8800 => self.prg[0] = data,
                0xA800 => self.prg[1] = data,
                0xA000 => self.prg[2] = data,
                0x9800 => self.mirr = data & 3,
                0xF000 => self.irq_counter = (self.irq_counter & 0xF0) | (data & 0x0F),
                0xF004 => self.irq_counter = (self.irq_counter & 0x0F) | (data << 4),
                0xF008 => {
                    self.irq_enabled = data != 0;
                    if !self.irq_enabled {
                        self.irq_scaler = 0;
                        self.irq_pending = false;
                        self.irq_ack = true;
                    }
                }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        match self.mirr {
            0 => address & 0x37FF,
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            2 => address & 0x33FF,
            3 => (address & 0x33FF) | 0x0400,
            _ => address,
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.irq_ack {
            self.irq_ack = false;
            true
        } else {
            false
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
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = (address >> 10) as usize & 7;
            let offset = (self.chr[bank] as usize) * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = self.mirror_raw(address, alternative_nametable_arrangement);
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let len = cart.chr_ram.len();
            if len > 0 { cart.chr_ram[address as usize % len] = data; }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.irq_enabled {
            self.irq_scaler += 1;
            if self.irq_scaler >= 114 {
                self.irq_scaler -= 114;
                self.irq_counter = self.irq_counter.wrapping_add(1);
                if self.irq_counter == 0 {
                    self.irq_pending = true;
                }
            }
        }
        self.irq_pending
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut s = Vec::new();
        s.extend_from_slice(&self.prg);
        s.push(self.prg_6000);
        for &c in &self.chr {
            s.push(c);
        }
        s.push(self.mirr);
        s.push(self.irq_counter);
        s.push(if self.irq_enabled { 1 } else { 0 });
        s.push(self.irq_scaler);
        s.push(if self.irq_pending { 1 } else { 0 });
        s
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for b in self.prg.iter_mut() { if p < state.len() { *b = state[p]; p += 1; } }
        if p < state.len() { self.prg_6000 = state[p]; p += 1; }
        for c in self.chr.iter_mut() { if p < state.len() { *c = state[p]; p += 1; } }
        if p < state.len() { self.mirr = state[p]; p += 1; }
        if p < state.len() { self.irq_counter = state[p]; p += 1; }
        if p < state.len() { self.irq_enabled = state[p] != 0; p += 1; }
        if p < state.len() { self.irq_scaler = state[p]; p += 1; }
        if p < state.len() { self.irq_pending = state[p] != 0; p += 1; }
        p
    }
}

impl Mapper183 {
    fn mirror_raw(&self, address: u16, alternative: bool) -> u16 {
        if alternative { return address; }
        match self.mirr {
            0 => address & 0x37FF,
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            2 => address & 0x33FF,
            3 => (address & 0x33FF) | 0x0400,
            _ => address,
        }
    }
}
