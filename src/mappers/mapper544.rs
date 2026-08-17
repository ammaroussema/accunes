use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

const COMPARE_MASKS: [u8; 8] = [0x28, 0x00, 0x4C, 0x64, 0x46, 0x7C, 0x04, 0xFF];

pub struct Mapper544 {
    prg: [u8; 2],
    chr: [u16; 8],
    mirroring: u8,
    prg_flip: bool,
    wram_enable: bool,
    cpu_c: u8,
    nt: [u8; 4],
    mask_chr_bank: u8,
    mask_compare: u8,

    irq_latch: u8,
    irq_counter: u8,
    irq_prescaler: i16,
    irq_enabled: bool,
    irq_mode: bool,
    irq_enable_on_ack: bool,
    irq_ack: bool,
}

impl Mapper544 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self {
            prg: [0, 1],
            chr: [0; 8],
            mirroring: 0,
            prg_flip: false,
            wram_enable: true,
            cpu_c: 0xFE,
            nt: [0xE0, 0xE0, 0xE1, 0xE1],
            mask_chr_bank: 0xFC,
            mask_compare: 0x28,

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

    fn update_mask_from_chr(&mut self, chr_val: u16) {
        if (chr_val & 0x80) != 0 {
            self.mask_chr_bank = if (chr_val & 0x10) != 0 {
                0x00
            } else if (chr_val & 0x40) != 0 {
                0xFE
            } else {
                0xFC
            };
            let idx = ((chr_val >> 1) & 1) | ((chr_val >> 2) & 2) | ((chr_val >> 4) & 4);
            self.mask_compare = COMPARE_MASKS[(idx & 7) as usize];
        }
    }

    fn is_ram_bank(&self, bank: usize) -> bool {
        let v = self.chr[bank];
        (v & self.mask_chr_bank as u16) == self.mask_compare as u16
    }

    fn nt_map(&self, alternative_nametable_arrangement: bool, address: u16) -> u16 {
        if alternative_nametable_arrangement {
            address
        } else {
            let idx = ((address >> 10) & 3) as usize;
            let page = (self.nt[idx] & 3) as u16;
            0x2000 | (page << 10) | (address & 0x03FF)
        }
    }
}
impl Mapper for Mapper544 {
    fn reset(&mut self) {
        self.prg = [0, 1];
        self.chr = [0; 8];
        self.mirroring = 0;
        self.prg_flip = false;
        self.wram_enable = true;
        self.cpu_c = 0xFE;
        self.nt = [0xE0, 0xE0, 0xE1, 0xE1];
        self.mask_chr_bank = 0xFC;
        self.mask_compare = 0x28;

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
        if (0x6000..0x8000).contains(&address) {
            if self.wram_enable && !cart.prg_ram.is_empty() {
                let len = cart.prg_ram.len();
                return FetchResult {
                    data: cart.prg_ram[(address as usize - 0x6000) % len],
                    driven: true,
                };
            }
            return FetchResult {
                data: 0,
                driven: false,
            };
        }

        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            let page = ((address - 0x8000) / 0x2000) as usize;
            let bank = match (page, self.prg_flip) {
                (0, false) => (self.prg[0] & 0x1F) as usize,
                (0, true) => 0x1E,
                (1, _) => (self.prg[1] & 0x1F) as usize,
                (2, _) => self.cpu_c as usize,
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

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) {
            if self.wram_enable && !cart.prg_ram.is_empty() {
                let len = cart.prg_ram.len();
                cart.prg_ram[(address as usize - 0x6000) % len] = data;
            }
        } else if address >= 0x8000 {
            let bit10 = (address & 0x400) != 0; 
            let bit11 = (address & 0x800) != 0;
            match address & 0xF000 {
                0x8000 => self.prg[0] = data & 0x1F,
                0x9000 => match (bit11, bit10) {
                    (false, _) => self.mirroring = data & 3,
                    (true, false) => {
                        self.wram_enable = (data & 1) != 0;
                        self.prg_flip = (data & 2) != 0;
                    }
                    (true, true) => {
                        if (address & 4) != 0 {
                            self.nt[(address & 3) as usize] = data;
                        } else {
                            self.cpu_c = data;
                        }
                    }
                },
                0xA000 => self.prg[1] = data & 0x1F,
                0xB000..=0xE000 => {
                    let group = (((address >> 12) & 0x0F) - 0xB) as usize;
                    self.write_chr(group, bit10, bit11, data);
                }
                0xF000 => match (bit11, bit10) {
                    (false, false) => self.irq_latch = (self.irq_latch & 0xF0) | (data & 0x0F),
                    (false, true) => self.irq_latch = (self.irq_latch & 0x0F) | (data << 4),
                    (true, false) => {
                        self.irq_mode = (data & 4) != 0;
                        self.irq_enabled = (data & 2) != 0;
                        self.irq_enable_on_ack = (data & 1) != 0;
                        if self.irq_enabled {
                            self.irq_counter = self.irq_latch;
                            self.irq_prescaler = 341;
                        }
                        self.irq_ack = true;
                    }
                    (true, true) => {
                        self.irq_enabled = self.irq_enable_on_ack;
                        self.irq_ack = true;
                    }
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
        chr_ram: &[u8],
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
            let chr_page = self.chr[bank] as usize;
            let offset = chr_page * 0x0400 + (address as usize & 0x03FF);
            let byte = if self.is_ram_bank(bank) && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
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
        if address < 0x2000 {
            let bank = ((address >> 10) as usize) & 7;
            self.update_mask_from_chr(self.chr[bank]);
            if self.is_ram_bank(bank) && !cart.chr_ram.is_empty() {
                let chr_page = self.chr[bank] as usize;
                let offset = chr_page * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
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


    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg);
        for &c in &self.chr {
            state.extend_from_slice(&c.to_le_bytes());
        }
        state.push(self.mirroring);
        state.push(self.prg_flip as u8);
        state.push(self.wram_enable as u8);
        state.push(self.cpu_c);
        state.extend_from_slice(&self.nt);
        state.push(self.mask_chr_bank);
        state.push(self.mask_compare);
        state.push(self.irq_latch);
        state.push(self.irq_counter);
        state.extend_from_slice(&self.irq_prescaler.to_le_bytes());
        state.push(self.irq_enabled as u8);
        state.push(self.irq_mode as u8);
        state.push(self.irq_enable_on_ack as u8);
        state.push(self.irq_ack as u8);
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
            self.prg_flip = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.wram_enable = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.cpu_c = state[p];
            p += 1;
        }
        if p + 4 <= state.len() {
            self.nt.copy_from_slice(&state[p..p + 4]);
            p += 4;
        }
        if p < state.len() {
            self.mask_chr_bank = state[p];
            p += 1;
        }
        if p < state.len() {
            self.mask_compare = state[p];
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
        if p < state.len() && !cart.prg_ram.is_empty() {
            let copy_len = cart.prg_ram.len().min(state.len() - p);
            cart.prg_ram[..copy_len].copy_from_slice(&state[p..p + copy_len]);
            p += copy_len;
        }
        p
    }
}

