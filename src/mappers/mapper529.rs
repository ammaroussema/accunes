use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

const OPCODE_MISC: u8 = 0;
const OPCODE_WRITE: u8 = 1;
const OPCODE_READ: u8 = 2;
const OPCODE_ERASE: u8 = 3;
const OPCODE_WRITE_DISABLE: u8 = 10;
const OPCODE_WRITE_ALL: u8 = 11;
const OPCODE_ERASE_ALL: u8 = 12;
const OPCODE_WRITE_ENABLE: u8 = 13;

const STATE_STANDBY: u8 = 0;
const STATE_START_BIT: u8 = 1;
const STATE_OPCODE: u8 = 3;
const STATE_ADDRESS_16: u8 = 11;
const STATE_DATA_16: u8 = 27;
const STATE_FINISHED: u8 = 99;

#[derive(Clone)]
pub struct Eeprom93C66 {
    pub storage: [u8; 256],
    opcode: u8,
    data: u16,
    address: usize,
    state: u8,
    last_clk: bool,
    write_enabled: bool,
    output: bool,
}

impl Eeprom93C66 {
    pub fn new() -> Self {
        Self {
            storage: [0xFF; 256],
            opcode: 0,
            data: 0,
            address: 0,
            state: STATE_STANDBY,
            last_clk: false,
            write_enabled: false,
            output: true,
        }
    }

    pub fn write(&mut self, cs: bool, clk: bool, dat: bool) {
        if !cs && self.state <= STATE_ADDRESS_16 {
            self.state = STATE_STANDBY;
        } else if self.state == STATE_STANDBY && cs && clk && !self.last_clk {
            if dat {
                self.state = STATE_START_BIT;
            }
            self.opcode = 0;
            self.address = 0;
            self.output = true;
        } else if clk && !self.last_clk && self.state >= STATE_START_BIT {
            let dat_bit = if dat { 1 } else { 0 };
            if self.state >= STATE_START_BIT && self.state < STATE_OPCODE {
                self.opcode = (self.opcode << 1) | dat_bit;
            } else if self.state >= STATE_OPCODE && self.state < STATE_ADDRESS_16 {
                self.address = (self.address << 1) | (dat_bit as usize);
            } else if self.state >= STATE_ADDRESS_16 && self.state < STATE_DATA_16 {
                if self.opcode == OPCODE_WRITE || self.opcode == OPCODE_WRITE_ALL {
                    self.data = (self.data << 1) | (dat_bit as u16);
                } else if self.opcode == OPCODE_READ {
                    self.output = (self.data & 0x8000) != 0;
                    self.data <<= 1;
                }
            }

            self.state += 1;

            if self.state == STATE_ADDRESS_16 {
                match self.opcode {
                    OPCODE_MISC => {
                        self.opcode = ((self.address >> 6) as u8) + 10;
                        match self.opcode {
                            OPCODE_WRITE_DISABLE => {
                                self.write_enabled = false;
                                self.state = STATE_FINISHED;
                            }
                            OPCODE_WRITE_ENABLE => {
                                self.write_enabled = true;
                                self.state = STATE_FINISHED;
                            }
                            OPCODE_ERASE_ALL => {
                                if self.write_enabled {
                                    self.storage.fill(0xFF);
                                }
                                self.state = STATE_FINISHED;
                            }
                            OPCODE_WRITE_ALL => {
                                self.address = 0;
                            }
                            _ => {}
                        }
                    }
                    OPCODE_ERASE => {
                        if self.write_enabled && (self.address * 2 + 1) < 256 {
                            self.storage[self.address * 2] = 0xFF;
                            self.storage[self.address * 2 + 1] = 0xFF;
                        }
                        self.state = STATE_FINISHED;
                    }
                    OPCODE_READ => {
                        if (self.address * 2 + 1) < 256 {
                            self.data = (self.storage[self.address * 2] as u16)
                                | ((self.storage[self.address * 2 + 1] as u16) << 8);
                        } else {
                            self.data = 0xFFFF;
                        }
                        self.address += 1;
                    }
                    _ => {}
                }
            } else if self.state == STATE_DATA_16 {
                if self.opcode == OPCODE_WRITE {
                    if self.write_enabled && (self.address * 2 + 1) < 256 {
                        self.storage[self.address * 2] = (self.data & 0xFF) as u8;
                        self.storage[self.address * 2 + 1] = (self.data >> 8) as u8;
                        self.address += 1;
                    }
                    self.state = STATE_FINISHED;
                } else if self.opcode == OPCODE_WRITE_ALL {
                    if self.write_enabled && (self.address * 2 + 1) < 256 {
                        self.storage[self.address * 2] = (self.data & 0xFF) as u8;
                        self.storage[self.address * 2 + 1] = (self.data >> 8) as u8;
                        self.address += 1;
                    }
                    self.state = if cs && (self.address * 2 < 256) {
                        STATE_ADDRESS_16
                    } else {
                        STATE_FINISHED
                    };
                } else if self.opcode == OPCODE_READ {
                    if self.address * 2 < 256 {
                        self.data = (self.storage[self.address * 2] as u16)
                            | ((self.storage[self.address * 2 + 1] as u16) << 8);
                    }
                    self.address += 1;
                    self.state = if cs && (self.address * 2 <= 256) {
                        STATE_ADDRESS_16
                    } else {
                        STATE_FINISHED
                    };
                }
            }

            if self.state == STATE_FINISHED {
                self.output = false;
                self.state = STATE_STANDBY;
            }
        }

        if self.opcode == OPCODE_READ && self.state == (STATE_ADDRESS_16 - 2) {
            self.output = false;
        }

        self.last_clk = clk;
    }

    pub fn read(&self) -> bool {
        self.output
    }
}

pub struct Mapper529 {
    prg: [u8; 2],
    chr: [u16; 8],
    mirroring: u8,
    eeprom: Eeprom93C66,

    irq_latch: u8,
    irq_counter: u8,
    irq_prescaler: i16,
    irq_enabled: bool,
    irq_mode: bool,
    irq_enable_on_ack: bool,
}

impl Mapper529 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self {
            prg: [0, 0],
            chr: [0; 8],
            mirroring: 0,
            eeprom: Eeprom93C66::new(),

            irq_latch: 0,
            irq_counter: 0,
            irq_prescaler: 0,
            irq_enabled: false,
            irq_mode: false,
            irq_enable_on_ack: false,
        }
    }
}

impl Mapper for Mapper529 {
    fn reset(&mut self) {
        self.prg = [0, 0];
        self.chr = [0; 8];
        self.mirroring = 0;

        self.irq_latch = 0;
        self.irq_counter = 0;
        self.irq_prescaler = 0;
        self.irq_enabled = false;
        self.irq_mode = false;
        self.irq_enable_on_ack = false;
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x5000..0x6000).contains(&address) {
            let val = if self.eeprom.read() { 0x01 } else { 0x00 };
            FetchResult {
                data: val,
                driven: true,
            }
        } else if (0x6000..0x8000).contains(&address) {
            if !cart.prg_ram.is_empty() {
                let offset = (address - 0x6000) as usize;
                FetchResult {
                    data: cart.prg_ram[offset % cart.prg_ram.len()],
                    driven: true,
                }
            } else {
                FetchResult {
                    data: 0,
                    driven: false,
                }
            }
        } else if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }

            let page_16k = (address as usize - 0x8000) / 0x4000;
            let bank = if page_16k == 0 {
                self.prg[1] as usize
            } else {
                (len / 0x4000).saturating_sub(1)
            };

            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) {
            if !cart.prg_ram.is_empty() {
                let offset = (address - 0x6000) as usize;
                let len = cart.prg_ram.len();
                cart.prg_ram[offset % len] = data;
            }
        } else if address >= 0x8000 {
            if (address & 0x0800) != 0 {
                let cs = (address & 0x04) != 0;
                let clk = (address & 0x02) != 0;
                let di = (address & 0x01) != 0;
                self.eeprom.write(cs, clk, di);
            } else {
                let bit0 = if (address & 0x04) != 0 { 1 } else { 0 };
                let bit1 = if (address & 0x08) != 0 { 2 } else { 0 };
                let decoded_reg = bit1 | bit0;

                match address & 0xF000 {
                    0x8000 => {
                        self.prg[0] = data & 0x1F;
                    }
                    0x9000 => match decoded_reg {
                        0 | 1 => {
                            self.mirroring = data & 3;
                        }
                        _ => {}
                    },
                    0xA000 => {
                        self.prg[1] = data & 0x1F;
                    }
                    0xB000..=0xE000 => {
                        let bank_idx = (((address >> 12) & 0xF) - 0xB) as usize;
                        let slot = (bank_idx << 1) | if (decoded_reg & 2) != 0 { 1 } else { 0 };
                        if (decoded_reg & 1) != 0 {
                            self.chr[slot] = (self.chr[slot] & 0x0F) | (((data as u16) & 0x1F) << 4);
                        } else {
                            self.chr[slot] = (self.chr[slot] & 0x1F0) | ((data as u16) & 0x0F);
                        }
                    }
                    0xF000 => match decoded_reg {
                        0 => {
                            self.irq_latch = (self.irq_latch & 0xF0) | (data & 0x0F);
                        }
                        1 => {
                            self.irq_latch = (self.irq_latch & 0x0F) | ((data & 0x0F) << 4);
                        }
                        2 => {
                            self.irq_mode = (data & 4) != 0;
                            self.irq_enabled = (data & 2) != 0;
                            self.irq_enable_on_ack = (data & 1) != 0;
                            if self.irq_enabled {
                                self.irq_counter = self.irq_latch;
                                self.irq_prescaler = 341;
                            }
                        }
                        3 => {
                            self.irq_enabled = self.irq_enable_on_ack;
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else {
            match self.mirroring & 3 {
                0 => address & 0x37FF,                               
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                2 => address & 0x33FF,                              
                3 => (address & 0x33FF) | 0x0400,                  
                _ => address,
            }
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;

        if !ciram {
            let bank = (address >> 10) as usize & 7;
            let chr_page = (self.chr[bank] & 0x1FF) as usize;
            let offset = chr_page * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else {
                match self.mirroring & 3 {
                    0 => address & 0x37FF,
                    1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                    2 => address & 0x33FF,
                    3 => (address & 0x33FF) | 0x0400,
                    _ => address,
                }
            };

            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() {
                    prg_vram[idx]
                } else {
                    0
                }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }

        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let bank = (address >> 10) as usize & 7;
                let chr_page = (self.chr[bank] & 0x1FF) as usize;
                let offset = chr_page * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
            }
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
        if self.irq_enabled && !self.irq_mode {
            self.irq_prescaler += 3;
            if self.irq_prescaler >= 341 {
                while self.irq_prescaler >= 341 {
                    self.irq_prescaler -= 341;
                    if self.irq_counter == 0xFF {
                        self.irq_counter = self.irq_latch;
                        return true;
                    } else {
                        self.irq_counter += 1;
                    }
                }
            }
        }
        false
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.irq_enabled && self.irq_mode {
            for _ in 0..cycles {
                if self.irq_counter == 0xFF {
                    self.irq_counter = self.irq_latch;
                    return true;
                } else {
                    self.irq_counter += 1;
                }
            }
        }
        false
    }

    fn battery_save_data(&self, _cart: &Cartridge) -> Option<Vec<u8>> {
        Some(self.eeprom.storage.to_vec())
    }

    fn load_battery_save(&mut self, _cart: &mut Cartridge, data: &[u8]) {
        let len = data.len().min(self.eeprom.storage.len());
        self.eeprom.storage[..len].copy_from_slice(&data[..len]);
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg);
        for &c in &self.chr {
            state.extend_from_slice(&c.to_le_bytes());
        }
        state.push(self.mirroring);
        state.push(self.irq_latch);
        state.push(self.irq_counter);
        state.extend_from_slice(&self.irq_prescaler.to_le_bytes());
        state.push(self.irq_enabled as u8);
        state.push(self.irq_mode as u8);
        state.push(self.irq_enable_on_ack as u8);
        state.extend_from_slice(&self.eeprom.storage);
        state.extend_from_slice(&cart.prg_ram);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.prg[0] = state[p];
            self.prg[1] = state[p + 1];
            p += 2;
        }
        if p + 16 <= state.len() {
            for i in 0..8 {
                self.chr[i] = u16::from_le_bytes([state[p], state[p + 1]]);
                p += 2;
            }
        }
        if p < state.len() {
            self.mirroring = state[p];
            p += 1;
        }
        if p < state.len() {
            self.irq_latch = state[p];
            p += 1;
        }
        if p < state.len() {
            self.irq_counter = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.irq_prescaler = i16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.irq_enabled = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.irq_mode = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.irq_enable_on_ack = state[p] != 0;
            p += 1;
        }
        if p + 256 <= state.len() {
            self.eeprom.storage.copy_from_slice(&state[p..p + 256]);
            p += 256;
        }
        if p < state.len() && !cart.prg_ram.is_empty() {
            let copy_len = cart.prg_ram.len().min(state.len() - p);
            cart.prg_ram[..copy_len].copy_from_slice(&state[p..p + copy_len]);
            p += copy_len;
        }
        p
    }
}
