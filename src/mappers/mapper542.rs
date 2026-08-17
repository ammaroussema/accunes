use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper542 {
    prg: [u8; 2],
    chr: [u16; 8],
    mirroring: u8,
    prg_flip: bool,
    wram_enable: bool,
    map_ciram: bool,

    irq_latch: u8,
    irq_counter: u8,
    irq_prescaler: i16,
    irq_enabled: bool,
    irq_mode: bool,
    irq_enable_on_ack: bool,
    irq_ack: bool,
}

impl Mapper542 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self {
            prg: [0, 1],
            chr: [0; 8],
            mirroring: 0,
            prg_flip: false,
            wram_enable: true,
            map_ciram: false,

            irq_latch: 0,
            irq_counter: 0,
            irq_prescaler: 0,
            irq_enabled: false,
            irq_mode: false,
            irq_enable_on_ack: false,
            irq_ack: false,
        }
    }

    fn write_chr(&mut self, group: usize, a0: bool, a1: bool, data: u8) {
        let slot = (group << 1) | if a1 { 1 } else { 0 };
        if a0 {
            self.chr[slot] = (self.chr[slot] & 0x000F) | ((data as u16) << 4);
        } else {
            self.chr[slot] = (self.chr[slot] & 0x0FF0) | ((data as u16) & 0x0F);
        }
    }

    fn nt_map(&self, alternative_nametable_arrangement: bool, address: u16) -> u16 {
        if alternative_nametable_arrangement {
            address
        } else {
            let nt_page = (address >> 10) & 3;
            if self.map_ciram && nt_page == 3 {
                0x2400 | (address & 0x03FF)
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
    }
}

impl Mapper for Mapper542 {
    fn reset(&mut self) {
        self.prg = [0, 1];
        self.chr = [0; 8];
        self.mirroring = 0;
        self.prg_flip = false;
        self.wram_enable = true;
        self.map_ciram = false;

        self.irq_latch = 0;
        self.irq_counter = 0;
        self.irq_prescaler = 0;
        self.irq_enabled = false;
        self.irq_mode = false;
        self.irq_enable_on_ack = false;
        self.irq_ack = false;
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        let len = cart.prg_rom.len();
        if len == 0 {
            return FetchResult {
                data: 0,
                driven: false,
            };
        }

        if (0x6000..0x8000).contains(&address) {
            let offset = 0x0F * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            };
        }

        if address >= 0x8000 {
            let page = ((address - 0x8000) / 0x2000) as usize;
            let bank = match (page, self.prg_flip) {
                (0, false) => (self.prg[0] & 0x1F) as usize,
                (0, true) => 0x1E,
                (1, _) => (self.prg[1] & 0x1F) as usize,
                (2, false) => 0x1E,
                (2, true) => (self.prg[0] & 0x1F) as usize,
                (3, _) => 0x1F,
                _ => 0,
            };
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            };
        }

        FetchResult {
            data: 0,
            driven: false,
        }
    }
fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let a0 = (address & 0x01) != 0;
            let a1 = (address & 0x02) != 0;
            let reg = (if a1 { 2 } else { 0 }) | (if a0 { 1 } else { 0 });
            match address & 0xF000 {
                0x8000 => self.prg[0] = data & 0x1F,
                0x9000 => match reg {
                    0 | 1 => self.mirroring = data & 3,
                    2 => {
                        self.wram_enable = (data & 1) != 0;
                        self.prg_flip = (data & 2) != 0;
                    }
                    _ => {}
                },
                0xA000 => self.prg[1] = data & 0x1F,
                0xB000 => self.write_chr(0, a0, a1, data),
                0xC000 => self.write_chr(1, a0, a1, data),
                0xD000 => {
                    if (address & 0x0800) != 0 {
                        self.map_ciram = true;
                    } else {
                        self.write_chr(2, a0, a1, data);
                    }
                }
                0xE000 => {
                    if (address & 0x0800) != 0 {
                        self.map_ciram = false;
                    } else {
                        self.write_chr(3, a0, a1, data);
                    }
                }
                0xF000 => match reg {
                    0 => self.irq_latch = (self.irq_latch & 0xF0) | (data & 0x0F),
                    1 => self.irq_latch = (self.irq_latch & 0x0F) | (data << 4),
                    2 => {
                        self.irq_mode = (data & 4) != 0;
                        self.irq_enabled = (data & 2) != 0;
                        self.irq_enable_on_ack = (data & 1) != 0;
                        if self.irq_enabled {
                            self.irq_counter = self.irq_latch;
                            self.irq_prescaler = 341;
                        }
                        self.irq_ack = true;
                    }
                    3 => {
                        self.irq_enabled = self.irq_enable_on_ack;
                        self.irq_ack = true;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.nt_map(cart.alternative_nametable_arrangement, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        _chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = ((address >> 10) as usize) & 7;
            let chr_page = (self.chr[bank] & 0x1FF) as usize;
            let offset = chr_page * 0x0400 + (address as usize & 0x03FF);
            let byte = if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = self.nt_map(alternative_nametable_arrangement, address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.nt_map(cart.alternative_nametable_arrangement, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock_rise(&mut self, _ppu_address_bus: u16) -> bool {
        if self.irq_enabled && !self.irq_mode {
            self.irq_prescaler -= 3;
            if self.irq_prescaler <= 0 {
                self.irq_prescaler += 341;
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

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack;
        self.irq_ack = false;
        ack
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg);
        for &c in &self.chr {
            state.extend_from_slice(&c.to_le_bytes());
        }
        state.push(self.mirroring);
        state.push(self.prg_flip as u8);
        state.push(self.wram_enable as u8);
        state.push(self.map_ciram as u8);
        state.push(self.irq_latch);
        state.push(self.irq_counter);
        state.extend_from_slice(&self.irq_prescaler.to_le_bytes());
        state.push(self.irq_enabled as u8);
        state.push(self.irq_mode as u8);
        state.push(self.irq_enable_on_ack as u8);
        state.push(self.irq_ack as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
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
            self.prg_flip = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.wram_enable = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.map_ciram = state[p] != 0;
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
        if p < state.len() {
            self.irq_ack = state[p] != 0;
            p += 1;
        }
        p
    }
}

