use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const PPU_IRQ_DELAY: u8 = 2;
const CPU_IRQ_DELAY: u8 = 1;

pub struct Mapper64 {
    registers: [u8; 16],
    current_register: u8,
    irq_enabled: bool,
    irq_cycle_mode: bool,
    need_reload: bool,
    irq_counter: u8,
    irq_reload_value: u8,
    cpu_clock_counter: u8,
    need_irq_delay: u8,
    force_clock: bool,
    a12_prev: bool,
    a12_filter: u8,
    irq_pending: bool,
    mirroring_vertical: bool,
    mapper_158_mode: bool,
}

impl Mapper64 {
    pub fn new(header_horizontal_mirror: bool) -> Self {
        Self {
            registers: [0; 16],
            current_register: 0,
            irq_enabled: false,
            irq_cycle_mode: false,
            need_reload: false,
            irq_counter: 0,
            irq_reload_value: 0,
            cpu_clock_counter: 0,
            need_irq_delay: 0,
            force_clock: false,
            a12_prev: false,
            a12_filter: 0,
            irq_pending: false,
            mirroring_vertical: !header_horizontal_mirror,
            mapper_158_mode: false,
        }
    }

    pub fn new_mapper158() -> Self {
        Self {
            registers: [0; 16],
            current_register: 0,
            irq_enabled: false,
            irq_cycle_mode: false,
            need_reload: false,
            irq_counter: 0,
            irq_reload_value: 0,
            cpu_clock_counter: 0,
            need_irq_delay: 0,
            force_clock: false,
            a12_prev: false,
            a12_filter: 0,
            irq_pending: false,
            mirroring_vertical: false,
            mapper_158_mode: true,
        }
    }

    fn clock_irq_counter(&mut self, delay: u8) {
        if self.need_reload {
            if self.irq_reload_value <= 1 {
                self.irq_counter = self.irq_reload_value.wrapping_add(1);
            } else {
                self.irq_counter = self.irq_reload_value.wrapping_add(2);
            }
            self.need_reload = false;
        } else if self.irq_counter == 0 {
            self.irq_counter = self.irq_reload_value.wrapping_add(1);
        }
        self.irq_counter = self.irq_counter.wrapping_sub(1);
        if self.irq_counter == 0 && self.irq_enabled {
            self.need_irq_delay = delay;
        }
    }

    fn prg_bank(&self, cart: &Cartridge, reg_val: u8) -> usize {
        let num_8k = cart.prg_rom.len() / 0x2000;
        if num_8k == 0 { 0 } else { reg_val as usize % num_8k }
    }

    fn chr_bank_offset(&self, bank: u8, address: u16, chr_len: usize) -> usize {
        let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
        if chr_len == 0 { 0 } else { offset % chr_len }
    }

    fn chr_bank_for_address(&self, address: u16) -> u8 {
        let a12_inversion: u16 = if self.current_register & 0x80 != 0 { 4 } else { 0 };
        let slot = (address >> 10) & 7; 
        let effective = slot ^ a12_inversion;
        match effective {
            0 => self.registers[0],
            1 => {
                if self.current_register & 0x20 != 0 {
                    self.registers[8]
                } else {
                    self.registers[0].wrapping_add(1)
                }
            }
            2 => self.registers[1],
            3 => {
                if self.current_register & 0x20 != 0 {
                    self.registers[9]
                } else {
                    self.registers[1].wrapping_add(1)
                }
            }
            4 => self.registers[2],
            5 => self.registers[3],
            6 => self.registers[4],
            7 => self.registers[5],
            _ => unreachable!(),
        }
    }
}

impl Mapper for Mapper64 {
    fn reset(&mut self) {
        self.registers = [0; 16];
        self.current_register = 0;
        self.irq_enabled = false;
        self.irq_cycle_mode = false;
        self.need_reload = false;
        self.irq_counter = 0;
        self.irq_reload_value = 0;
        self.cpu_clock_counter = 0;
        self.need_irq_delay = 0;
        self.force_clock = false;
        self.a12_prev = false;
        self.a12_filter = 0;
        self.irq_pending = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }
        let num_8k = cart.prg_rom.len() / 0x2000;
        if num_8k == 0 {
            return FetchResult { data: 0, driven: true };
        }
        let slot = ((address as usize - 0x8000) >> 13) & 3;
        let bank = if self.current_register & 0x40 != 0 {
            match slot {
                0 => self.prg_bank(cart, self.registers[15]),
                1 => self.prg_bank(cart, self.registers[6]),
                2 => self.prg_bank(cart, self.registers[7]),
                3 => num_8k - 1,
                _ => unreachable!(),
            }
        } else {
            match slot {
                0 => self.prg_bank(cart, self.registers[6]),
                1 => self.prg_bank(cart, self.registers[7]),
                2 => self.prg_bank(cart, self.registers[15]),
                3 => num_8k - 1,
                _ => unreachable!(),
            }
        };
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        FetchResult {
            data: cart.prg_rom[offset % cart.prg_rom.len()],
            driven: true,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            return;
        }
        match address & 0xE001 {
            0x8000 => {
                self.current_register = data;
            }
            0x8001 => {
                self.registers[(self.current_register & 0x0F) as usize] = data;
            }
             0xA000 => {
                if !self.mapper_158_mode {
                    self.mirroring_vertical = (data & 0x01) == 0;
                }
            }
            0xC000 => {
                self.irq_reload_value = data;
            }
            0xC001 => {
                if self.irq_cycle_mode && (data & 0x01) == 0x00 {
                    self.force_clock = true;
                }
                self.irq_cycle_mode = (data & 0x01) != 0;
                if self.irq_cycle_mode {
                    self.cpu_clock_counter = 0;
                }
                self.need_reload = true;
            }
            0xE000 => {
                self.irq_enabled = false;
                self.irq_pending = false;
            }
            0xE001 => {
                self.irq_enabled = true;
            }
            _ => {}
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mapper_158_mode {
            let chr_xor = if self.current_register & 0x80 != 0 { 4 } else { 0 };
            let slot = ((address >> 10) & 7) ^ chr_xor;
            let page = match slot {
                0 => self.registers[0] >> 7,
                1 => {
                (if self.current_register & 0x20 != 0 {
                    self.registers[8]
                } else {
                    self.registers[0]
                }) >> 7
                }
                2 => self.registers[1] >> 7,
                3 => ({
                    if self.current_register & 0x20 != 0 {
                        self.registers[9]
                    } else {
                        self.registers[1]
                    }
                }) >> 7,
                4 => self.registers[2] >> 7,
                5 => self.registers[3] >> 7,
                6 => self.registers[4] >> 7,
                7 => self.registers[5] >> 7,
                _ => 0,
            } & 1;
            ((page as u16) * 0x0400) | (address & 0x03FF)
        } else if self.mirroring_vertical {
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
            let bank = self.chr_bank_for_address(address);
            let byte = if using_chr_ram {
                if chr_ram.is_empty() {
                    0
                } else {
                    let offset = self.chr_bank_offset(bank, address, chr_ram.len());
                    chr_ram[offset]
                }
            } else if chr_rom.is_empty() {
                0
            } else {
                let offset = self.chr_bank_offset(bank, address, chr_rom.len());
                chr_rom[offset]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if self.mapper_158_mode {
                let chr_xor = if self.current_register & 0x80 != 0 { 4 } else { 0 };
                let slot = ((address >> 10) & 7) ^ chr_xor;
                let page = match slot {
                    0 => self.registers[0] >> 7,
                    1 => ({
                        if self.current_register & 0x20 != 0 {
                            self.registers[8]
                        } else {
                            self.registers[0]
                        }
                    }) >> 7,
                    2 => self.registers[1] >> 7,
                    3 => ({
                        if self.current_register & 0x20 != 0 {
                            self.registers[9]
                        } else {
                            self.registers[1]
                        }
                    }) >> 7,
                    4 => self.registers[2] >> 7,
                    5 => self.registers[3] >> 7,
                    6 => self.registers[4] >> 7,
                    7 => self.registers[5] >> 7,
                    _ => 0,
                } & 1;
                ((page as u16) * 0x0400) | (address & 0x03FF)
            } else if self.mirroring_vertical {
                address & 0x37FF
            } else {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let bank = self.chr_bank_for_address(address);
            if !cart.chr_ram.is_empty() {
                let offset = self.chr_bank_offset(bank, address, cart.chr_ram.len());
                cart.chr_ram[offset] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
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
        if self.need_irq_delay > 0 {
            self.need_irq_delay -= 1;
            if self.need_irq_delay == 0 {
                self.irq_pending = true;
            }
        }
        if !self.irq_cycle_mode {
            let a12 = (ppu_address_bus & 0x1000) != 0;
            if !self.a12_prev && a12 && self.a12_filter >= 3 {
                self.clock_irq_counter(PPU_IRQ_DELAY);
            }
            if a12 {
                self.a12_filter = 0;
            }
            self.a12_prev = a12;
        }
        let fired = self.irq_pending;
        if fired {
            self.irq_pending = false;
        }
        fired
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        let a12 = (ppu_address_bus & 0x1000) != 0;
        if !a12 && self.a12_filter < 3 {
            self.a12_filter += 1;
        }
        if self.irq_cycle_mode || self.force_clock {
            self.cpu_clock_counter = (self.cpu_clock_counter + 1) & 0x03;
            if self.cpu_clock_counter == 0 {
                self.clock_irq_counter(CPU_IRQ_DELAY);
                self.force_clock = false;
            }
        }
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.registers);
        state.push(self.current_register);
        state.push(self.irq_reload_value);
        state.push(self.irq_counter);
        state.push(if self.irq_enabled { 1 } else { 0 });
        state.push(if self.irq_cycle_mode { 1 } else { 0 });
        state.push(if self.need_reload { 1 } else { 0 });
        state.push(self.cpu_clock_counter);
        state.push(self.need_irq_delay);
        state.push(if self.force_clock { 1 } else { 0 });
        state.push(self.a12_filter);
        state.push(if self.a12_prev { 1 } else { 0 });
        state.push(if self.mirroring_vertical { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut i = start;
        for r in 0..16 {
            self.registers[r] = state[i]; i += 1;
        }
        self.current_register = state[i]; i += 1;
        self.irq_reload_value = state[i]; i += 1;
        self.irq_counter = state[i]; i += 1;
        self.irq_enabled = state[i] != 0; i += 1;
        self.irq_cycle_mode = state[i] != 0; i += 1;
        self.need_reload = state[i] != 0; i += 1;
        self.cpu_clock_counter = state[i]; i += 1;
        self.need_irq_delay = state[i]; i += 1;
        self.force_clock = state[i] != 0; i += 1;
        self.a12_filter = state[i]; i += 1;
        self.a12_prev = state[i] != 0; i += 1;
        self.mirroring_vertical = state[i] != 0; i += 1;
        i - start
    }
}
