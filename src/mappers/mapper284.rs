use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
struct DripAudioChannel {
    fifo: [u8; 256],
    read_pos: u8,
    write_pos: u8,
    is_full: bool,
    is_empty: bool,
    freq: u16,       
    vol: u8,         
    timer: u16,      
    pos: i16,        
}

impl DripAudioChannel {
    fn new() -> Self {
        Self {
            fifo: [0; 256],
            read_pos: 0,
            write_pos: 0,
            is_full: false,
            is_empty: true,
            freq: 0,
            vol: 0,
            timer: 0,
            pos: 0,
        }
    }

    fn reset(&mut self) {
        self.fifo = [0; 256];
        self.read_pos = 0;
        self.write_pos = 0;
        self.is_full = false;
        self.is_empty = true;
        self.freq = 0;
        self.vol = 0;
        self.timer = 0;
        self.pos = 0;
    }

    fn silence(&mut self) {
        self.fifo = [0; 256];
        self.read_pos = 0;
        self.write_pos = 0;
        self.is_full = false;
        self.is_empty = true;
        self.pos = 0;
        self.timer = self.freq;
    }

    fn write_data(&mut self, data: u8) {
        if self.read_pos == self.write_pos {
            self.is_empty = false;
            self.pos = ((data as i16) - 0x80) * self.vol as i16;
            self.timer = self.freq;
        }
        self.fifo[self.write_pos as usize] = data;
        self.write_pos = self.write_pos.wrapping_add(1);
        if self.read_pos == self.write_pos {
            self.is_full = true;
        }
    }

    fn set_period_low(&mut self, val: u8) {
        self.freq = (self.freq & 0xF00) | val as u16;
    }

    fn set_period_high_volume(&mut self, val: u8) {
        self.freq = (self.freq & 0x0FF) | ((val as u16 & 0x0F) << 8);
        self.vol = (val >> 4) & 0x0F;
        if !self.is_empty {
            self.pos = ((self.fifo[self.read_pos as usize] as i16) - 0x80) * self.vol as i16;
        }
    }

    fn generate_wave(&mut self, cycles: i32) -> i32 {
        let mut z = 0;
        for _ in 0..cycles {
            if self.is_empty {
                break;
            }
            if self.timer == 0 {
                self.timer = self.freq;
                if self.read_pos == self.write_pos {
                    self.is_full = false;
                }
                self.pos = ((self.fifo[(self.read_pos.wrapping_add(1)) as usize] as i16) - 0x80) * self.vol as i16;
                self.read_pos = self.read_pos.wrapping_add(1);
                if self.read_pos == self.write_pos {
                    self.is_empty = true;
                }
            } else {
                self.timer -= 1;
            }
            z += self.pos as i32;
        }
        if cycles > 0 {
            z / cycles
        } else {
            0
        }
    }

    fn read_status(&self) -> u8 {
        let mut result = 0;
        if self.is_full {
            result |= 0x80;
        }
        if self.is_empty {
            result |= 0x40;
        }
        result
    }

    fn save(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.fifo);
        out.push(self.read_pos);
        out.push(self.write_pos);
        out.push(if self.is_full { 1 } else { 0 });
        out.push(if self.is_empty { 1 } else { 0 });
        out.push((self.freq & 0xFF) as u8);
        out.push((self.freq >> 8) as u8);
        out.push(self.vol);
        out.push((self.timer & 0xFF) as u8);
        out.push((self.timer >> 8) as u8);
        out
    }

    fn load(&mut self, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.fifo.copy_from_slice(&state[p..p + 256]);
        p += 256;
        self.read_pos = state[p]; p += 1;
        self.write_pos = state[p]; p += 1;
        self.is_full = state[p] != 0; p += 1;
        self.is_empty = state[p] != 0; p += 1;
        self.freq = state[p] as u16 | ((state[p + 1] as u16) << 8); p += 2;
        self.vol = state[p]; p += 1;
        self.timer = state[p] as u16 | ((state[p + 1] as u16) << 8); p += 2;
        p
    }
}

pub struct Mapper284 {
    prg_bank: u8,
    chr: [u8; 4],
    mirroring: u8,         
    ext_attr_enabled: bool,
    sram_write_enabled: bool,
    irq_counter: u16,      
    irq_latch_low: u8,     
    irq_enabled: bool,
    irq_pending: bool,      
    ext_attr: [[u8; 0x400]; 2], 
    last_nt_fetch_addr: u16,    
    audio: [DripAudioChannel; 2],
}

impl Mapper284 {
    pub fn new() -> Self {
        Self {
            prg_bank: 0,
            chr: [0; 4],
            mirroring: 0,
            ext_attr_enabled: false,
            sram_write_enabled: false,
            irq_counter: 0,
            irq_latch_low: 0,
            irq_enabled: false,
            irq_pending: false,
            ext_attr: [[0; 0x400]; 2],
            last_nt_fetch_addr: 0,
            audio: [DripAudioChannel::new(), DripAudioChannel::new()],
        }
    }

    fn mirror_address(&self, address: u16) -> u16 {
        let addr = address & 0x2FFF;
        match self.mirroring {
            0 => addr & 0x37FF,                                      
            1 => (addr & 0x33FF) | ((addr & 0x0800) >> 1),           
            2 => addr & 0x23FF,                                      
            3 => (addr & 0x23FF) | 0x0400,                           
            _ => addr & 0x37FF,
        }
    }

    fn ext_attr_bank(&self, _address: u16) -> u8 {
        match self.mirroring {
            2 => 0,                                       
            3 => 1,                                       
            1 => if _address & 0x800 != 0 { 1 } else { 0 },  
            0 => if _address & 0x400 != 0 { 1 } else { 0 },  
            _ => 0,
        }
    }
}

impl Mapper for Mapper284 {
    fn reset(&mut self) {
        self.prg_bank = 0;
        self.chr = [0; 4];
        self.mirroring = 0;
        self.ext_attr_enabled = false;
        self.sram_write_enabled = false;
        self.irq_counter = 0;
        self.irq_latch_low = 0;
        self.irq_enabled = false;
        self.irq_pending = false;
        self.ext_attr = [[0; 0x400]; 2];
        self.last_nt_fetch_addr = 0;
        self.audio[0].reset();
        self.audio[1].reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let prg_len = cart.prg_rom.len();
            if prg_len == 0 {
                return FetchResult { data: 0, driven: false };
            }
            let banks_16k = prg_len / 0x4000;
            if address < 0xC000 {
                let bank = (self.prg_bank as usize) % banks_16k;
                let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                FetchResult { data: cart.prg_rom[offset % prg_len], driven: true }
            } else {
                let bank = banks_16k.wrapping_sub(1);
                let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                FetchResult { data: cart.prg_rom[offset % prg_len], driven: true }
            }
        } else if address >= 0x6000 && address < 0x8000 {
            let offset = (address - 0x6000) as usize;
            if offset < cart.prg_ram.len() {
                FetchResult { data: cart.prg_ram[offset], driven: true }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else if address >= 0x5800 {
            FetchResult { data: self.audio[1].read_status(), driven: true }
        } else if address >= 0x5000 {
            FetchResult { data: self.audio[0].read_status(), driven: true }
        } else if address >= 0x4800 {
            FetchResult { data: 0x64, driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0xC000 {
            let bank = if address & 0x400 != 0 { 1 } else { 0 };
            self.ext_attr[bank][(address & 0x3FF) as usize] = data;
            return;
        }
        if address >= 0x8000 && address < 0xC000 {
            let reg = address & 0xF;
            match reg {
                0x0 => self.audio[0].silence(),
                0x1 => self.audio[0].write_data(data),
                0x2 => self.audio[0].set_period_low(data),
                0x3 => self.audio[0].set_period_high_volume(data),
                0x4 => self.audio[1].silence(),
                0x5 => self.audio[1].write_data(data),
                0x6 => self.audio[1].set_period_low(data),
                0x7 => self.audio[1].set_period_high_volume(data),
                0x8 => {
                    self.irq_latch_low = data;
                }
                0x9 => {
                    self.irq_counter = ((data as u16 & 0x7F) << 8) | self.irq_latch_low as u16;
                    self.irq_enabled = (data & 0x80) != 0;
                    self.irq_pending = false;
                }
                0xA => {
                    self.mirroring = data & 3;
                    self.ext_attr_enabled = (data & 4) != 0;
                    self.sram_write_enabled = (data & 8) != 0;
                }
                0xB => {
                    self.prg_bank = data & 0x0F;
                }
                0xC => self.chr[0] = data & 0x0F,
                0xD => self.chr[1] = data & 0x0F,
                0xE => self.chr[2] = data & 0x0F,
                0xF => self.chr[3] = data & 0x0F,
                _ => {}
            }
            return;
        }
        if address >= 0x6000 && address < 0x8000 {
            if self.sram_write_enabled {
                let offset = (address - 0x6000) as usize;
                if offset < cart.prg_ram.len() {
                    cart.prg_ram[offset] = data;
                }
            }
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        let pending = self.irq_pending;
        self.irq_pending = false;
        pending
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror_address(address)
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
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let window = (address / 0x0800) as usize & 3;
            let bank = self.chr[window] as usize;
            let offset = bank * 0x0800 + (address as usize & 0x07FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let is_tile_fetch = (address & 0x3FF) < 0x3C0;
            if is_tile_fetch {
                self.last_nt_fetch_addr = address & 0x03FF;
            }
            if self.ext_attr_enabled && !is_tile_fetch {
                let bank = self.ext_attr_bank(address);
                let attr_val = self.ext_attr[bank as usize][self.last_nt_fetch_addr as usize] & 0x03;
                let byte = (attr_val << 6) | (attr_val << 4) | (attr_val << 2) | attr_val;
                new_addr_bus |= byte as u16;
            } else {
                let mirrored = self.mirror_address(address);
                let byte = vram[(mirrored & 0x7FF) as usize];
                new_addr_bus |= byte as u16;
            }
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let window = (address / 0x0800) as usize & 3;
                let bank = self.chr[window] as usize;
                let offset = bank * 0x0800 + (address as usize & 0x07FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_address(address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock_rise(&mut self, _ppu_address_bus: u16) -> bool {
        self.audio[0].generate_wave(1);
        self.audio[1].generate_wave(1);
        if self.irq_enabled && self.irq_counter > 0 {
            self.irq_counter -= 1;
            if self.irq_counter == 0 {
                self.irq_enabled = false;
                self.irq_pending = true;
                return true;
            }
        }
        false
    }

    fn audio_sample(&self) -> f32 {
        let total = (self.audio[0].pos as i32 + self.audio[1].pos as i32) << 3;
        total as f32 / 32768.0
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend(self.audio[0].save());
        state.extend(self.audio[1].save());
        state.push((self.irq_counter & 0xFF) as u8);
        state.push((self.irq_counter >> 8) as u8);
        state.push(if self.irq_enabled { 1 } else { 0 });
        state.push(self.irq_latch_low);
        state.push(self.prg_bank);
        state.extend_from_slice(&self.chr);
        state.push(self.mirroring);
        state.push((self.last_nt_fetch_addr & 0xFF) as u8);
        state.push((self.last_nt_fetch_addr >> 8) as u8);
        state.extend_from_slice(&self.ext_attr[0]);
        state.extend_from_slice(&self.ext_attr[1]);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        p = self.audio[0].load(state, p);
        p = self.audio[1].load(state, p);
        self.irq_counter = state[p] as u16 | ((state[p + 1] as u16) << 8); p += 2;
        self.irq_enabled = state[p] != 0; p += 1;
        self.irq_latch_low = state[p]; p += 1;
        self.prg_bank = state[p]; p += 1;
        self.chr.copy_from_slice(&state[p..p + 4]); p += 4;
        self.mirroring = state[p]; p += 1;
        self.last_nt_fetch_addr = state[p] as u16 | ((state[p + 1] as u16) << 8); p += 2;
        self.ext_attr[0].copy_from_slice(&state[p..p + 0x400]); p += 0x400;
        self.ext_attr[1].copy_from_slice(&state[p..p + 0x400]); p += 0x400;
        p
    }
}
