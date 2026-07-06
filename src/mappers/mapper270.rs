use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper270 {
    reg4100: [u8; 0x100],
    reg2000: [u8; 0x100],
    reg4242: u8,
    submapper: u8,
}

impl Mapper270 {
    pub fn new(submapper_id: u8) -> Self {
        let mut reg4100 = [0u8; 0x100];
        reg4100[0x07] = 0x00;
        reg4100[0x08] = 0x01;
        reg4100[0x09] = 0xFE;
        reg4100[0x0A] = 0x00;
        Self {
            reg4100,
            reg2000: [0; 0x100],
            reg4242: 0,
            submapper: submapper_id,
        }
    }

    fn prg_offset(&self) -> u16 {
        let reg2c = self.reg4100[0x2C];
        match self.submapper {
            1 => {
                if reg2c & 0x02 != 0 { 0x0800 } else { 0x0000 }
            }
            2 => {
                let mut or_val = 0u16;
                if reg2c & 0x02 != 0 { or_val |= 0x0800; }
                if reg2c & 0x01 != 0 { or_val |= 0x1000; }
                or_val
            }
            3 => {
                if reg2c & 0x04 != 0 { 0x0800 } else { 0x0000 }
            }
            _ => {
                let mut or_val = 0u16;
                if reg2c & 0x06 != 0 { or_val |= 0x0800; }
                if reg2c & 0x01 != 0 { or_val |= 0x1000; }
                or_val
            }
        }
    }

    fn chr_offset(&self) -> u16 {
        self.prg_offset() << 3
    }

    fn prg_bank(&self, slot: usize) -> u16 {
        let ps = (self.reg4100[0x0B] & 0x07) as u16;
        let prg_and = if ps == 7 { 0xFF } else { 0x3F >> ps };
        let pq3 = self.reg4100[0x0A] as u16;
        let pa21 = (self.reg4100[0x00] >> 4) as u16;
        let prg_or = (pq3 | (pa21 << 8)) & !prg_and;
        let _rel = (self.reg4100[0x60] as u16) | ((self.reg4100[0x61] as u16) << 8 & 0xF00);
        let pq2en = (self.reg4100[0x0B] & 0x40) != 0;
        let pq = match slot {
            0 => self.reg4100[0x07] as u16,
            1 => self.reg4100[0x08] as u16,
            2 => {
                if pq2en { self.reg4100[0x09] as u16 } else { 0xFE }
            }
            3 => 0xFF,
            _ => 0,
        };
        let and = 0x07FFu16;
        let or_val = self.prg_offset();
        ((pq & prg_and) | prg_or).wrapping_add(_rel) & and | or_val
    }

    fn chr_bank_1k(&self, slot: usize) -> u16 {
        let vb0s = (self.reg4100[0x1A] & 0x07) as usize;
        let vb0s_table: [u8; 8] = [0, 1, 2, 0, 3, 4, 5, 1];
        let shift = vb0s_table[vb0s.min(7)] as u16;
        let chr_and = 0xFFu16 >> shift;
        let rv6 = (self.reg4100[0x1A] >> 3) as u16;
        let chr_or = (rv6 << 3) & !chr_and;
        let va18 = ((self.reg4100[0x18] >> 4) & 7) as u16;
        let chr_or_va = chr_or | (va18 << 8);
        let va21 = (self.reg4100[0x00] & 0x0F) as u16;
        let _rel = (self.reg4100[0x60] as u16) | ((self.reg4100[0x61] as u16) << 8 & 0xF00);
        let bank_reg = match slot {
            0 => (self.reg2000[0x16] & !1) as u16,
            1 => (self.reg2000[0x16] | 1) as u16,
            2 => (self.reg2000[0x17] & !1) as u16,
            3 => (self.reg2000[0x17] | 1) as u16,
            4 => self.reg2000[0x12] as u16,
            5 => self.reg2000[0x13] as u16,
            6 => self.reg2000[0x14] as u16,
            7 => self.reg2000[0x15] as u16,
            _ => 0,
        };
        let and = 0x3FFFu16;
        let or_val = self.chr_offset();
        ((bank_reg & chr_and) | chr_or_va | (va21 << 11) & and) | or_val
    }

    fn mirror_vertical(&self) -> bool {
        (self.reg4100[0x06] & 0x01) == 0
    }
}

impl Mapper for Mapper270 {
    fn reset(&mut self) {
        self.reg4242 = 0;
        self.reg4100 = [0; 0x100];
        self.reg2000 = [0; 0x100];
        self.reg4100[0x07] = 0x00;
        self.reg4100[0x08] = 0x01;
        self.reg4100[0x09] = 0xFE;
        self.reg4100[0x0A] = 0x00;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let slot = ((address - 0x8000) >> 13) as usize;
            let bank = self.prg_bank(slot) as usize;
            let num_8k = cart.prg_rom.len() / 0x2000;
            let bank = bank % num_8k.max(1);
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0x6000 && address < 0x8000 {
            let idx = (address as usize - 0x6000) & 0x1FFF;
            if idx < cart.prg_ram.len() {
                FetchResult { data: cart.prg_ram[idx], driven: true }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            let idx = (address as usize - 0x6000) & 0x1FFF;
            if idx < _cart.prg_ram.len() {
                _cart.prg_ram[idx] = data;
            }
        } else if address >= 0x8000 {
            let decoded = (address & 0xF000) >> 8;
            match decoded {
                0x41 => {
                    if address & 0xFF == 0x2C {
                        self.reg4100[0x2C] = data;
                    }
                }
                0x42 => {
                    if address & 0xFF == 0x42 {
                        self.reg4242 = data;
                    }
                }
                _ => {
                    if (address & 0xFF00) == 0x4100 {
                        let idx = (address & 0xFF) as usize;
                        self.reg4100[idx] = data;
                    }
                }
            }
        } else if (address & 0xF000) == 0x5000 {
            let idx = (address & 0xFF) as usize;
            if idx < 0x100 {
                self.reg4100[idx] = data;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mirror_vertical() {
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
        if address < 0x2000 {
            let byte = if (self.reg4242 & 1) != 0 {
                if !chr_ram.is_empty() {
                    chr_ram[address as usize & 0x1FFF]
                } else { 0 }
            } else {
                let slot = (address >> 10) as usize & 7;
                let bank = self.chr_bank_1k(slot) as usize;
                let offset = (bank * 0x400) + (address as usize & 0x3FF);
                if !chr_rom.is_empty() {
                    chr_rom[offset % chr_rom.len()]
                } else if !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else { 0 }
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if self.mirror_vertical() {
                address & 0x37FF
            } else {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if (self.reg4242 & 1) != 0 && !cart.chr_ram.is_empty() {
                cart.chr_ram[address as usize & 0x1FFF] = data;
            } else if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let slot = (address >> 10) as usize & 7;
                let bank = self.chr_bank_1k(slot) as usize;
                let offset = bank * 0x400 + (address as usize & 0x3FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = if self.mirror_vertical() {
                address & 0x37FF
            } else {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            };
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.reg4100);
        state.extend_from_slice(&self.reg2000);
        state.push(self.reg4242);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 0x100 <= state.len() {
            self.reg4100.copy_from_slice(&state[p..p + 0x100]);
            p += 0x100;
        }
        if p + 0x100 <= state.len() {
            self.reg2000.copy_from_slice(&state[p..p + 0x100]);
            p += 0x100;
        }
        if p < state.len() {
            self.reg4242 = state[p];
            p += 1;
        }
        p
    }
}
