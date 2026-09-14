use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::um6578::Um6578Hw;

pub fn has_24c02_eeprom(header: &[u8]) -> bool {
    header.len() > 10 && (header[7] & 0x0C) == 0x08 && header[10] == 0x20
}

pub fn prg_ram_size(header: &[u8]) -> usize {
    if has_24c02_eeprom(header) {
        return 256;
    }
    if header.len() > 10 && (header[7] & 0x0C) == 0x08 {
        let volatile = (header[10] & 0x0F) as usize;
        let battery = ((header[10] >> 4) & 0x0F) as usize;
        let mut size = 0;
        if volatile != 0 {
            size += 64usize << volatile;
        }
        if battery != 0 {
            size += 64usize << battery;
        }
        if size != 0 {
            return size;
        }
    }
    0x2000
}

struct Eeprom24C02 {
    address: u16,
    bit: u8,
    latch: u8,
    state: u8,
    clock: bool,
    data: bool,
    output: bool,
    read_mode: bool,
}

impl Eeprom24C02 {
    fn new() -> Self {
        Self {
            address: 0,
            bit: 0,
            latch: 0,
            state: 0,
            clock: true,
            data: true,
            output: true,
            read_mode: false,
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn get_data(&self) -> bool {
        self.output
    }

    fn set_pins(&mut self, new_clock: bool, new_data: bool, rom: &mut [u8]) {
        if self.clock && new_clock && self.data && !new_data {
            self.state = 1;
        } else if self.clock && new_clock && !self.data && new_data {
            self.state = 0;
        } else if self.clock && !new_clock {
            self.receive_bit(rom);
        }
        self.clock = new_clock;
        self.data = new_data;
    }

    fn receive_bit(&mut self, rom: &mut [u8]) {
        const DEVICE_TYPE: u8 = 0b1010;
        const DEVICE_ADDR: u8 = 0;
        const ADDRESS_MASK: u16 = 0x00FF;

        match self.state {
            1 => self.state += 1,
            2 => self.state = if ((DEVICE_TYPE & 8) != 0) == self.data { self.state + 1 } else { 0 },
            3 => self.state = if ((DEVICE_TYPE & 4) != 0) == self.data { self.state + 1 } else { 0 },
            4 => self.state = if ((DEVICE_TYPE & 2) != 0) == self.data { self.state + 1 } else { 0 },
            5 => self.state = if ((DEVICE_TYPE & 1) != 0) == self.data { self.state + 1 } else { 0 },
            6 => self.state = if ((DEVICE_ADDR & 4) != 0) == self.data { self.state + 1 } else { 0 },
            7 => self.state = if ((DEVICE_ADDR & 2) != 0) == self.data { self.state + 1 } else { 0 },
            8 => self.state = if ((DEVICE_ADDR & 1) != 0) == self.data { self.state + 1 } else { 0 },
            9 => {
                self.read_mode = self.data;
                self.state += 1;
            }
            10 => {
                self.bit = 0;
                if self.read_mode {
                    let idx = (self.address & ADDRESS_MASK) as usize;
                    self.latch = rom.get(idx).copied().unwrap_or(0);
                    self.state = 11;
                } else {
                    self.latch = 0;
                    self.state = 12;
                }
            }
            11 => {
                self.bit = self.bit.wrapping_add(1);
                if self.bit == 8 {
                    self.address = (self.address & !0xFF) | ((self.address.wrapping_add(1)) & 0xFF);
                    self.state -= 1;
                }
            }
            12 => {
                if self.data {
                    self.latch |= 0x80 >> self.bit;
                }
                self.bit = self.bit.wrapping_add(1);
                if self.bit == 8 {
                    self.address = (self.address & !0xFF) | self.latch as u16;
                    self.state += 1;
                }
            }
            13 => {
                self.bit = 0;
                self.latch = 0;
                self.state += 1;
            }
            14 => {
                if self.data {
                    self.latch |= 0x80 >> self.bit;
                }
                self.bit = self.bit.wrapping_add(1);
                if self.bit == 8 {
                    let idx = (self.address & ADDRESS_MASK) as usize;
                    if idx < rom.len() {
                        rom[idx] = self.latch;
                    }
                    self.address = (self.address & !0xFF) | ((self.address.wrapping_add(1)) & 0xFF);
                    self.state -= 1;
                }
            }
            _ => {}
        }

        match self.state {
            10 | 13 => self.output = false,
            11 => self.output = (self.latch & (0x80 >> self.bit)) != 0,
            _ => self.output = true,
        }
    }

    fn save(&self) -> Vec<u8> {
        vec![
            (self.address & 0xFF) as u8,
            (self.address >> 8) as u8,
            self.bit,
            self.latch,
            self.state,
            self.clock as u8,
            self.data as u8,
            self.output as u8,
            self.read_mode as u8,
        ]
    }

    fn load(&mut self, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 9 > state.len() {
            return p;
        }
        self.address = u16::from_le_bytes([state[p], state[p + 1]]);
        p += 2;
        self.bit = state[p];
        p += 1;
        self.latch = state[p];
        p += 1;
        self.state = state[p];
        p += 1;
        self.clock = state[p] != 0;
        p += 1;
        self.data = state[p] != 0;
        p += 1;
        self.output = state[p] != 0;
        p += 1;
        self.read_mode = state[p] != 0;
        p += 1;
        p
    }
}

pub struct Mapper405 {
    hw: Um6578Hw,
    eeprom: Option<Eeprom24C02>,
}

impl Default for Mapper405 {
    fn default() -> Self {
        Self::new(false)
    }
}

impl Mapper405 {
    pub fn new(has_eeprom: bool) -> Self {
        Self {
            hw: Um6578Hw::new(),
            eeprom: if has_eeprom { Some(Eeprom24C02::new()) } else { None },
        }
    }

    fn get_prg_bank(&self, slot: usize, prg_len: usize) -> usize {
        let n = (prg_len / 0x1000).max(1);
        (self.hw.prg[slot & 7] as usize) % n
    }
}

impl Mapper for Mapper405 {
    fn is_um6578(&self) -> bool {
        true
    }

    fn um6578_chr(&self) -> u8 {
        self.hw.chr
    }

    fn reset(&mut self) {
        self.hw.reset();
        if let Some(ref mut eeprom) = self.eeprom {
            eeprom.reset();
        }
    }

    fn store_prg(&mut self, c: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 && !c.prg_ram.is_empty() {
            let offset = (address as usize - 0x6000) % c.prg_ram.len();
            c.prg_ram[offset] = data;
            return;
        }
        if address == 0x4026 {
            self.hw.write_register(address, data);
            if let Some(ref mut eeprom) = self.eeprom {
                if self.hw.reg4026 & 0x80 == 0 {
                    eeprom.set_pins((data & 0x02) != 0, (data & 0x01) != 0, &mut c.prg_ram);
                }
            }
            return;
        }
        self.hw.write_register(address, data);
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address == 0x4026 {
            let mut result = self.hw.reg4026;
            if let Some(ref eeprom) = self.eeprom {
                if result & 0x80 == 0 {
                    result = if eeprom.get_data() { 0x01 } else { 0x00 };
                }
            }
            return FetchResult { data: result, driven: true };
        }
        if let Some(data) = self.hw.read_register(address) {
            return FetchResult { data, driven: true };
        }
        if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let offset = (address as usize - 0x6000) % cart.prg_ram.len();
                return FetchResult {
                    data: cart.prg_ram[offset],
                    driven: true,
                };
            }
            return FetchResult { data: 0, driven: true };
        }
        if address >= 0x8000 {
            let slot = ((address - 0x8000) >> 12) as usize;
            let bank = self.get_prg_bank(slot, cart.prg_rom.len());
            let offset = bank * 0x1000 + (address as usize & 0xFFF);
            let data = if cart.prg_rom.is_empty() {
                0
            } else {
                cart.prg_rom[offset % cart.prg_rom.len()]
            };
            return FetchResult { data, driven: true };
        }
        FetchResult { data: 0, driven: false }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        address & 0x37FF
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        _chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        _vram: &[u8],
    ) -> (u8, u16) {
        let addr = (ppu_address_bus & 0x7FFF) | ppu_octal_latch as u16;
        (addr as u8, addr)
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.hw.timer_control & 0x20 == 0 {
            self.hw.clock_timer();
        }
        self.hw.irq_pending()
    }

    fn cpu_clock_irq_level(&self) -> bool {
        true
    }

    fn ppu_clock(
        &mut self,
        _ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        if self.hw.timer_control & 0x20 != 0 && dot >= 330 {
            let sl = scanline as i32;
            if sl != self.hw.prev_scanline {
                self.hw.prev_scanline = sl;
                self.hw.clock_timer();
            }
        }
        false
    }

    fn audio_sample(&self) -> f32 {
        (self.hw.pcm as i16) as f32 * 128.0
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&self.hw.prg);
        v.push(self.hw.chr);
        v.push(self.hw.pcm);
        v.push(self.hw.irq_mask);
        v.push(self.hw.irq_status);
        v.push(self.hw.timer_control);
        v.push(self.hw.timer_latch);
        v.push(self.hw.timer_value);
        v.push(self.hw.prescaler);
        v.push(self.hw.reg4016);
        v.push(self.hw.reg4026);
        if let Some(ref eeprom) = self.eeprom {
            v.push(1);
            v.extend_from_slice(&eeprom.save());
        } else {
            v.push(0);
        }
        v
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 8 <= state.len() {
            self.hw.prg.copy_from_slice(&state[p..p + 8]);
            p += 8;
        }
        if p < state.len() {
            self.hw.chr = state[p];
            p += 1;
        }
        if p < state.len() {
            self.hw.pcm = state[p];
            p += 1;
        }
        if p < state.len() {
            self.hw.irq_mask = state[p];
            p += 1;
        }
        if p < state.len() {
            self.hw.irq_status = state[p];
            p += 1;
        }
        if p < state.len() {
            self.hw.timer_control = state[p];
            p += 1;
        }
        if p < state.len() {
            self.hw.timer_latch = state[p];
            p += 1;
        }
        if p < state.len() {
            self.hw.timer_value = state[p];
            p += 1;
        }
        if p < state.len() {
            self.hw.prescaler = state[p];
            p += 1;
        }
        if p < state.len() {
            self.hw.reg4016 = state[p];
            p += 1;
        }
        if p < state.len() {
            self.hw.reg4026 = state[p];
            p += 1;
        }
        if p < state.len() {
            let has = state[p];
            p += 1;
            if has != 0 {
                if self.eeprom.is_none() {
                    self.eeprom = Some(Eeprom24C02::new());
                }
                if let Some(ref mut eeprom) = self.eeprom {
                    p = eeprom.load(state, p);
                }
            }
        }
        p
    }
}
