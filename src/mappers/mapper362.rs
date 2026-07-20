use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper362 {
    prg_reg: [u8; 2],
    chr_reg: [u8; 8],
    chr_hi: [u16; 8],
    mirr: u8,
    game: u8,
    current_chr_bank: u8,
    reg_cmd: u8,
    irq_latch: u8,
    irq_count: u16,
    irq_enabled: bool,
    irq_mode: bool,
    irq_cmd: u8,
    acount: u16,
    irq_ack_pending: bool,
}

impl Mapper362 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self {
            prg_reg: [0; 2],
            chr_reg: [0; 8],
            chr_hi: [0; 8],
            mirr: 0,
            game: 0,
            current_chr_bank: 0,
            reg_cmd: 0,
            irq_latch: 0,
            irq_count: 0,
            irq_enabled: false,
            irq_mode: false,
            irq_cmd: 0,
            acount: 0,
            irq_ack_pending: false,
        }
    }

    fn decode_address(&self, address: u16) -> u16 {
        let base = address & 0xF000;
        let bit1 = if address & 0x02 != 0 { 1 << 1 } else { 0 };
        let bit0 = if address & 0x01 != 0 { 1 } else { 0 };
        base | bit1 | bit0
    }

    fn chr_bank_raw(&self, index: usize) -> u16 {
        if index < 8 { self.chr_reg[index] as u16 | self.chr_hi[index] } else { 0 }
    }

    fn prg_or(&self) -> usize {
        if self.game == 0 {
            ((self.chr_bank_raw(self.current_chr_bank as usize) & 0x180) >> 3) as usize
        } else {
            0x40
        }
    }

    fn chror(&self) -> usize {
        if self.game == 0 {
            (self.chr_bank_raw(self.current_chr_bank as usize) & 0x180) as usize
        } else {
            0x200
        }
    }

    fn chr_mask(&self) -> usize {
        if self.game == 0 { 0x7F } else { 0x1FF }
    }

    fn mirror_fn(&self, address: u16) -> u16 {
        match self.mirr {
            0 => mirror_h_or_v(false, address),
            1 => mirror_h_or_v(true, address),
            2 => (address & 0xBFFF) | 0x0000,
            3 => (address & 0xBFFF) | 0x0400,
            _ => address,
        }
    }
}

impl Mapper for Mapper362 {
    fn reset(&mut self) {
        self.prg_reg = [0; 2];
        self.chr_reg = [0; 8];
        self.chr_hi = [0; 8];
        self.mirr = 0;
        self.reg_cmd = 0;
        self.current_chr_bank = 0;
        self.irq_latch = 0;
        self.irq_count = 0;
        self.irq_enabled = false;
        self.irq_mode = false;
        self.irq_cmd = 0;
        self.acount = 0;
        self.irq_ack_pending = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            if cart.prg_ram.len() > 0 {
                return FetchResult { data: cart.prg_ram[(address as usize - 0x6000) % cart.prg_ram.len()], driven: true };
            }
            return FetchResult { data: 0, driven: false };
        }
        if address >= 0x8000 {
            let len = cart.prg_rom.len().max(1);
            let prg_or = self.prg_or();
            let bank = if self.reg_cmd & 2 != 0 {
                match address {
                    0x8000..=0x9FFF => (!1usize & 0x0F) | prg_or,
                    0xA000..=0xBFFF => (self.prg_reg[1] as usize & 0x0F) | prg_or,
                    0xC000..=0xDFFF => (self.prg_reg[0] as usize & 0x0F) | prg_or,
                    0xE000..=0xFFFF => (!0usize & 0x0F) | prg_or,
                    _ => 0,
                }
            } else {
                match address {
                    0x8000..=0x9FFF => (self.prg_reg[0] as usize & 0x0F) | prg_or,
                    0xA000..=0xBFFF => (self.prg_reg[1] as usize & 0x0F) | prg_or,
                    0xC000..=0xDFFF => (!1usize & 0x0F) | prg_or,
                    0xE000..=0xFFFF => (!0usize & 0x0F) | prg_or,
                    _ => 0,
                }
            };
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult { data: cart.prg_rom[offset % len], driven: true };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, val: u8) {
        if address < 0x8000 {
            if address >= 0x6000 && address < 0x8000 && self.reg_cmd & 1 != 0 {
                let len = cart.prg_ram.len();
                if len > 0 {
                    cart.prg_ram[(address as usize - 0x6000) % len] = val;
                }
            }
            return;
        }
        let decoded = self.decode_address(address);
        if decoded >= 0xB000 && decoded <= 0xE003 {
            let i = (((decoded >> 1) & 1) | ((decoded - 0xB000) >> 11)) as usize;
            if i < 8 {
                let nibble = (decoded & 1) << 2;
                self.chr_reg[i] = (self.chr_reg[i] & (0xF0 >> nibble)) | ((val & 0xF) << nibble);
                if nibble != 0 {
                    self.chr_hi[i] = ((val & 0x10) << 4) as u16;
                }
            }
        } else {
            match decoded & 0xF003 {
                0x8000 | 0x8001 | 0x8002 | 0x8003 => {
                    self.prg_reg[0] = val & 0x1F;
                }
                0xA000 | 0xA001 | 0xA002 | 0xA003 => {
                    self.prg_reg[1] = val & 0x1F;
                }
                0x9000 | 0x9001 => {
                    self.mirr = val & 0x03;
                }
                0x9002 | 0x9003 => {
                    self.reg_cmd = val;
                }
                0xF000 => {
                    self.irq_latch = (self.irq_latch & 0xF0) | (val & 0x0F);
                }
                0xF001 => {
                    self.irq_latch = (self.irq_latch & 0x0F) | ((val & 0x0F) << 4);
                }
                0xF002 => {
                    self.acount = 0;
                    self.irq_count = self.irq_latch as u16;
                    self.irq_mode = (val & 4) != 0;
                    self.irq_enabled = (val & 2) != 0;
                    self.irq_cmd = if val & 1 != 0 { 1 } else { 0 };
                    self.irq_ack_pending = true;
                }
                0xF003 => {
                    self.irq_ack_pending = true;
                }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror_fn(address)
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
            let bank = (address >> 10) as usize;
            if self.game == 0 {
                self.current_chr_bank = bank as u8;
            }
            let chr_mask = self.chr_mask();
            let chror = self.chror();
            if using_chr_ram && !chr_ram.is_empty() {
                let chr_val = self.chr_bank_raw(bank) as usize;
                let bank_idx = (chr_val & chr_mask) | chror;
                let offset = bank_idx * 0x400 + (address as usize & 0x3FF);
                new_addr_bus |= chr_ram[offset % chr_ram.len()] as u16;
            } else if !chr_rom.is_empty() {
                let chr_val = self.chr_bank_raw(bank) as usize;
                let bank_idx = (chr_val & chr_mask) | chror;
                let offset = bank_idx * 0x400 + (address as usize & 0x3FF);
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else {
            let mir = self.mirror_fn(address);
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram && !cart.chr_ram.is_empty() {
            let len = cart.chr_ram.len();
            cart.chr_ram[(address as usize) % len] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mir = self.mirror_fn(address);
            vram[(mir & 0x7FF) as usize] = data;
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
        if self.irq_mode {
            return false;
        }
        if self.irq_enabled {
            const LCYCS: u16 = 341;
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
        if !self.irq_mode {
            return false;
        }
        if self.irq_enabled {
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

    fn take_irq_ack(&mut self) -> bool {
        if self.irq_ack_pending {
            self.irq_ack_pending = false;
            return true;
        }
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        for &r in &self.prg_reg { state.push(r); }
        for &r in &self.chr_reg { state.push(r); }
        for &r in &self.chr_hi { state.extend_from_slice(&r.to_le_bytes()); }
        state.push(self.mirr);
        state.push(self.game);
        state.push(self.current_chr_bank);
        state.push(self.reg_cmd);
        state.push(self.irq_latch);
        state.extend_from_slice(&self.irq_count.to_le_bytes());
        state.push(self.irq_enabled as u8 | (self.irq_mode as u8) << 1 | (self.irq_cmd) << 2);
        state.extend_from_slice(&self.acount.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..2 { if p < state.len() { self.prg_reg[i] = state[p]; p += 1; } }
        for i in 0..8 { if p < state.len() { self.chr_reg[i] = state[p]; p += 1; } }
        for i in 0..8 { if p + 2 <= state.len() { self.chr_hi[i] = u16::from_le_bytes([state[p], state[p+1]]); p += 2; } }
        if p < state.len() { self.mirr = state[p]; p += 1; }
        if p < state.len() { self.game = state[p]; p += 1; }
        if p < state.len() { self.current_chr_bank = state[p]; p += 1; }
        if p < state.len() { self.reg_cmd = state[p]; p += 1; }
        if p < state.len() { self.irq_latch = state[p]; p += 1; }
        if p + 2 <= state.len() { self.irq_count = u16::from_le_bytes([state[p], state[p+1]]); p += 2; }
        if p < state.len() {
            let flags = state[p]; p += 1;
            self.irq_enabled = (flags & 1) != 0;
            self.irq_mode = (flags & 2) != 0;
            self.irq_cmd = (flags >> 2) & 1;
        }
        if p + 2 <= state.len() { self.acount = u16::from_le_bytes([state[p], state[p+1]]); p += 2; }
        p
    }
}
