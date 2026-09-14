use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const IRQ_MASK: [u16; 4] = [0xFFFF, 0x0FFF, 0x00FF, 0x000F];

pub struct Mapper18 {
    prg_banks: [u8; 3],
    chr_banks: [u8; 8],
    irq_reload_value: [u8; 4],
    irq_counter: u16,
    irq_counter_size: u8, 
    irq_enabled: bool,
    irq_ack: bool,         
    irq_pending: bool,     
    mirr: u8,
}

impl Mapper18 {
    pub fn new() -> Self {
        Mapper18 {
            prg_banks: [0; 3],
            chr_banks: [0; 8],
            irq_reload_value: [0; 4],
            irq_counter: 0,
            irq_counter_size: 0,
            irq_enabled: false,
            irq_ack: false,
            irq_pending: false,
            mirr: 0,
        }
    }

    fn reload_irq_counter(&mut self) {
        self.irq_counter = self.irq_reload_value[0] as u16
            | ((self.irq_reload_value[1] as u16) << 4)
            | ((self.irq_reload_value[2] as u16) << 8)
            | ((self.irq_reload_value[3] as u16) << 12);
    }

    fn update_prg_bank(&mut self, bank_number: usize, value: u8, update_upper_bits: bool) {
        if update_upper_bits {
            self.prg_banks[bank_number] = (self.prg_banks[bank_number] & 0x0F) | (value << 4);
        } else {
            self.prg_banks[bank_number] = (self.prg_banks[bank_number] & 0xF0) | value;
        }
    }

    fn update_chr_bank(&mut self, bank_number: usize, value: u8, update_upper_bits: bool) {
        if update_upper_bits {
            self.chr_banks[bank_number] = (self.chr_banks[bank_number] & 0x0F) | (value << 4);
        } else {
            self.chr_banks[bank_number] = (self.chr_banks[bank_number] & 0xF0) | value;
        }
    }

    fn clock_irq_counter(&mut self) -> bool {
        if !self.irq_enabled {
            return false;
        }
        let mask = IRQ_MASK[self.irq_counter_size as usize];
        let masked = self.irq_counter & mask;
        let decremented = masked.wrapping_sub(1);
        self.irq_counter = (self.irq_counter & !mask) | (decremented & mask);
        decremented == 0
    }

    fn mirror_calc(&self, address: u16) -> u16 {
        match self.mirr {
            0 => (address & 0x33FF) | ((address & 0x0800) >> 1), 
            1 => address & 0x37FF,                               
            2 => address & 0x3BFF,                               
            3 => (address & 0x3BFF) | 0x0400,                   
            _ => address,
        }
    }
}

impl Mapper for Mapper18 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let num_banks = cart.prg_rom.len() / 0x2000;
            let bank = match address {
                0x8000..=0x9FFF => self.prg_banks[0] as usize % num_banks.max(1),
                0xA000..=0xBFFF => self.prg_banks[1] as usize % num_banks.max(1),
                0xC000..=0xDFFF => self.prg_banks[2] as usize % num_banks.max(1),
                _ => num_banks.saturating_sub(1), 
            };
            let offset = (bank * 0x2000) + (address as usize & 0x1FFF);
            FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
        } else if address >= 0x6000 {
            let idx = (address as usize - 0x6000) % cart.prg_ram.len().max(1);
            if cart.prg_ram.is_empty() {
                FetchResult { data: 0, driven: false }
            } else {
                FetchResult { data: cart.prg_ram[idx], driven: true }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let update_upper_bits = (address & 0x01) == 0x01;
            let value = data & 0x0F;
            match address & 0xF003 {
                0x8000 | 0x8001 => self.update_prg_bank(0, value, update_upper_bits),
                0x8002 | 0x8003 => self.update_prg_bank(1, value, update_upper_bits),
                0x9000 | 0x9001 => self.update_prg_bank(2, value, update_upper_bits),
                0xA000 | 0xA001 => self.update_chr_bank(0, value, update_upper_bits),
                0xA002 | 0xA003 => self.update_chr_bank(1, value, update_upper_bits),
                0xB000 | 0xB001 => self.update_chr_bank(2, value, update_upper_bits),
                0xB002 | 0xB003 => self.update_chr_bank(3, value, update_upper_bits),
                0xC000 | 0xC001 => self.update_chr_bank(4, value, update_upper_bits),
                0xC002 | 0xC003 => self.update_chr_bank(5, value, update_upper_bits),
                0xD000 | 0xD001 => self.update_chr_bank(6, value, update_upper_bits),
                0xD002 | 0xD003 => self.update_chr_bank(7, value, update_upper_bits),
                0xE000 | 0xE001 | 0xE002 | 0xE003 => {
                    self.irq_reload_value[(address & 0x03) as usize] = value;
                }
                0xF000 => {
                    self.irq_pending = false;
                    self.irq_ack = true;
                    self.reload_irq_counter();
                }
                0xF001 => {
                    self.irq_pending = false;
                    self.irq_ack = true;
                    self.irq_enabled = (value & 0x01) != 0;
                    self.irq_counter_size = if value & 0x08 != 0 {
                        3 
                    } else if value & 0x04 != 0 {
                        2 
                    } else if value & 0x02 != 0 {
                        1 
                    } else {
                        0 
                    };
                }
                0xF002 => {
                    self.mirr = value & 0x03;
                }
                0xF003 => {
                }
                _ => {}
            }
        } else if address >= 0x6000 {
            let idx = (address as usize - 0x6000) % cart.prg_ram.len().max(1);
            if !cart.prg_ram.is_empty() {
                cart.prg_ram[idx] = data;
            }
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack;
        self.irq_ack = false;
        ack
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror_calc(address)
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
            let bank = (address >> 10) as usize & 0x07;
            let chr_bank = self.chr_banks[bank] as usize;
            let offset = (chr_bank * 0x400) + (address as usize & 0x3FF);
            if using_chr_ram && !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
            } else if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else if address < 0x3F00 {
            let mirrored = self.mirror_calc(address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let bank = (address >> 10) as usize & 0x07;
                let chr_bank = self.chr_banks[bank] as usize;
                let offset = (chr_bank * 0x400) + (address as usize & 0x3FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.clock_irq_counter() {
            self.irq_pending = true;
            return true;
        }
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg_banks);
        state.extend_from_slice(&self.chr_banks);
        state.extend_from_slice(&self.irq_reload_value);
        state.extend_from_slice(&self.irq_counter.to_le_bytes());
        state.push(self.irq_counter_size);
        state.push(self.irq_enabled as u8);
        state.push(self.irq_pending as u8);
        state.push(self.mirr);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 3 + 8 + 4 + 2 + 1 + 1 + 1 + 1 <= state.len() {
            for i in 0..3 {
                self.prg_banks[i] = state[start];
                start += 1;
            }
            for i in 0..8 {
                self.chr_banks[i] = state[start];
                start += 1;
            }
            for i in 0..4 {
                self.irq_reload_value[i] = state[start];
                start += 1;
            }
            self.irq_counter = u16::from_le_bytes([state[start], state[start + 1]]);
            start += 2;
            self.irq_counter_size = state[start];
            start += 1;
            self.irq_enabled = state[start] != 0;
            start += 1;
            self.irq_pending = state[start] != 0;
            start += 1;
            self.mirr = state[start];
            start += 1;
        }
        start
    }

    fn reset(&mut self) {
        self.prg_banks = [0; 3];
        self.chr_banks = [0; 8];
        self.irq_reload_value = [0; 4];
        self.irq_counter = 0;
        self.irq_counter_size = 0;
        self.irq_enabled = false;
        self.irq_ack = false;
        self.irq_pending = false;
        self.mirr = 0;
    }
}
