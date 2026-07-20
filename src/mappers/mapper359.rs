use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

struct FdsBootleg {
    regs: [u8; 0x20],
}

impl FdsBootleg {
    fn new() -> Self { Self { regs: [0; 0x20] } }
    fn write(&mut self, addr: u16, val: u8) {
        let idx = (addr & 0x1F) as usize;
        if idx < 0x20 { self.regs[idx] = val; }
    }
    fn sample(&self) -> f32 { 0.0 }
}

pub struct Mapper359 {
    prg: [u8; 4],
    chr: [u8; 8],
    prg_and: u8,
    chr_and: u8,
    prg_or: u16,
    chr_or: u16,
    mirroring: u8,
    counter: u16,
    pa12_filter: u8,
    reload: bool,
    irq_enabled: bool,
    irq_pa12: bool,
    irq_auto_enable: bool,
    irq_pending: bool,
    pa12_prev: bool,
    fds: FdsBootleg,
}

impl Mapper359 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self {
            prg: [0xFC, 0xFD, 0xFE, 0xFF],
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            prg_and: 0x3F,
            chr_and: 0xFF,
            prg_or: 0,
            chr_or: 0,
            mirroring: 0,
            counter: 0,
            pa12_filter: 0,
            reload: false,
            irq_enabled: false,
            irq_pa12: false,
            irq_auto_enable: false,
            irq_pending: false,
            pa12_prev: false,
            fds: FdsBootleg::new(),
        }
    }

    fn prg_bank_8k(&self, raw: u8) -> usize {
        ((raw as usize) & self.prg_and as usize) | self.prg_or as usize
    }

    fn chr_bank_1k(&self, raw: u8) -> usize {
        ((raw as usize) & self.chr_and as usize) | self.chr_or as usize
    }

    fn mirror_fn(&self, address: u16) -> u16 {
        match self.mirroring {
            0 => mirror_h_or_v(false, address),
            1 => mirror_h_or_v(true, address),
            2 => (address & 0xBFFF) | 0x0000,
            3 => (address & 0xBFFF) | 0x0400,
            _ => address,
        }
    }
}

impl Mapper for Mapper359 {
    fn reset(&mut self) {
        self.prg = [0xFC, 0xFD, 0xFE, 0xFF];
        self.chr = [0, 1, 2, 3, 4, 5, 6, 7];
        self.prg_and = 0x3F;
        self.chr_and = 0xFF;
        self.prg_or = 0;
        self.chr_or = 0;
        self.mirroring = 0;
        self.counter = 0;
        self.pa12_filter = 0;
        self.reload = false;
        self.irq_enabled = false;
        self.irq_pa12 = false;
        self.irq_auto_enable = false;
        self.irq_pending = false;
        self.pa12_prev = false;
        self.fds = FdsBootleg::new();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x6000 {
            return FetchResult { data: 0, driven: false };
        }
        let len = cart.prg_rom.len().max(1);
        let page = match address {
            0x6000..=0x7FFF => 3usize,
            0x8000..=0x9FFF => 0,
            0xA000..=0xBFFF => 1,
            0xC000..=0xDFFF => 2,
            0xE000..=0xFFFF => 3,
            _ => return FetchResult { data: 0, driven: false },
        };
        let bank_raw = if address >= 0xE000 { 0xFF } else { self.prg[page] };
        let bank = self.prg_bank_8k(bank_raw);
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        FetchResult { data: cart.prg_rom[offset % len], driven: true }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, val: u8) {
        if address >= 0x4020 && address <= 0x403F {
            self.fds.write(address, val);
            return;
        }
        match address & 0xF000 {
            0x8000 => {
                self.prg[address as usize & 3] = val;
            }
            0x9000 => {
                match address as usize & 3 {
                    0 => self.prg_or = (val as u16 & 0x38) << 1,
                    1 => {
                        self.prg_and = match val & 3 {
                            0 => 0x3F, 1 => 0x1F,
                            2 => 0x2F, _ => 0x0F,
                        };
                        self.chr_and = if val & 0x40 != 0 { 0xFF } else { 0x7F };
                    }
                    2 => self.mirroring = val & 3,
                    3 => self.chr_or = (val as u16) << 7,
                    _ => {}
                }
            }
            0xA000 | 0xB000 => {
                let bank = address as usize & 3 | if address & 0x1000 != 0 { 4 } else { 0 };
                self.chr[bank] = val;
            }
            0xC000 => {
                match address as usize & 3 {
                    0 => {
                        if self.irq_auto_enable { self.irq_enabled = false; }
                        self.counter = (self.counter & 0xFF00) | val as u16;
                    }
                    1 => {
                        if self.irq_auto_enable { self.irq_enabled = true; }
                        self.counter = (self.counter & 0x00FF) | (val as u16) << 8;
                        self.reload = true;
                    }
                    2 => {
                        self.irq_enabled = (val & 1) != 0;
                        self.irq_pa12 = (val & 2) != 0;
                        self.irq_auto_enable = (val & 4) != 0;
                    }
                    3 => {
                        self.irq_enabled = (val & 1) != 0;
                    }
                    _ => {}
                }
            }
            _ => {}
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
            if using_chr_ram && !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[(address as usize) % chr_ram.len()] as u16;
            } else if !chr_rom.is_empty() {
                let b = self.chr_bank_1k(self.chr[bank]);
                let offset = b * 0x400 + (address as usize & 0x3FF);
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

    fn cpu_clock_rise(&mut self, _ppu_address_bus: u16) -> bool {
        if self.pa12_filter > 0 { self.pa12_filter -= 1; }
        if self.irq_enabled && !self.irq_pa12 && self.counter > 0 {
            self.counter -= 1;
            if self.counter == 0 {
                self.irq_pending = true;
            }
        }
        self.irq_pending
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        _scanline: u16,
        _dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        let pa12 = (ppu_address_bus & 0x1000) != 0;
        let rising = pa12 && !self.pa12_prev;
        self.pa12_prev = pa12;
        if rising {
            if self.pa12_filter == 0 && self.irq_pa12 {
                if (self.counter as u8) == 0 || self.reload {
                    self.counter = (self.counter & 0xFF00) | (self.counter >> 8) as u8 as u16;
                } else {
                    self.counter = (self.counter & 0xFF00) | ((self.counter as u8) - 1) as u16;
                }
                if self.counter as u8 == 0 && self.irq_enabled {
                    self.irq_pending = true;
                }
                self.reload = false;
            }
            self.pa12_filter = 5;
        }
        false
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.irq_pending {
            self.irq_pending = false;
            true
        } else {
            false
        }
    }

    fn audio_sample(&self) -> f32 {
        self.fds.sample()
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        for &r in &self.prg { state.push(r); }
        for &r in &self.chr { state.push(r); }
        state.push(self.prg_and);
        state.push(self.chr_and);
        state.extend_from_slice(&self.prg_or.to_le_bytes());
        state.extend_from_slice(&self.chr_or.to_le_bytes());
        state.push(self.mirroring);
        state.extend_from_slice(&self.counter.to_le_bytes());
        state.push(self.pa12_filter);
        state.push(self.reload as u8);
        state.push(self.irq_enabled as u8 | (self.irq_pa12 as u8) << 1 | (self.irq_auto_enable as u8) << 2);
        state.push(self.irq_pending as u8);
        state.push(self.pa12_prev as u8);
        for &r in &self.fds.regs { state.push(r); }
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..4 { if p < state.len() { self.prg[i] = state[p]; p += 1; } }
        for i in 0..8 { if p < state.len() { self.chr[i] = state[p]; p += 1; } }
        if p < state.len() { self.prg_and = state[p]; p += 1; }
        if p < state.len() { self.chr_and = state[p]; p += 1; }
        if p + 2 <= state.len() { self.prg_or = u16::from_le_bytes([state[p], state[p+1]]); p += 2; }
        if p + 2 <= state.len() { self.chr_or = u16::from_le_bytes([state[p], state[p+1]]); p += 2; }
        if p < state.len() { self.mirroring = state[p]; p += 1; }
        if p + 2 <= state.len() { self.counter = u16::from_le_bytes([state[p], state[p+1]]); p += 2; }
        if p < state.len() { self.pa12_filter = state[p]; p += 1; }
        if p < state.len() { self.reload = state[p] != 0; p += 1; }
        if p < state.len() { let f = state[p]; p += 1; self.irq_enabled = (f & 1) != 0; self.irq_pa12 = (f & 2) != 0; self.irq_auto_enable = (f & 4) != 0; }
        if p < state.len() { self.irq_pending = state[p] != 0; p += 1; }
        if p < state.len() { self.pa12_prev = state[p] != 0; p += 1; }
        for i in 0..0x20 { if p < state.len() { self.fds.regs[i] = state[p]; p += 1; } }
        p
    }
}
