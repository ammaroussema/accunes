use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

// Mapper 448 "830768C": VRC24 (VRC4) + data latch multicart + WRAM + CHR RAM.
// See rf/Furbtendulator-main/src/src-mappers/src/iNES/VRC-based/mapper448.cpp
pub struct Mapper448 {
    reg: u8,
    latch_data: u8,
    prg: [u8; 2],
    chr: [u16; 8],
    mirr: u8,
    prg_flip: u8,
    wram_enable: bool,
    irq: u8,
    counter: u8,
    latch: u8,
    cycles: i16,
    irq_raise_count: u8,
    irq_asserted: bool,
    irq_ack: bool,
}

impl Mapper448 {
    pub fn new() -> Self {
        Self {
            reg: 0,
            latch_data: 0,
            prg: [0, 1],
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            mirr: 0,
            prg_flip: 0,
            wram_enable: true,
            irq: 0,
            counter: 0,
            latch: 0,
            cycles: 0,
            irq_raise_count: 0,
            irq_asserted: false,
            irq_ack: false,
        }
    }

    fn prg_bank16(&self, slot: u8) -> usize {
        let reg = self.reg as usize;
        if self.reg & 0x04 != 0 {
            match slot {
                0 => (self.prg[0] as usize & 0x0F) | ((reg << 3) & 0xF0),
                _ => 0x0F | ((reg << 3) & 0xF0),
            }
        } else {
            match slot {
                0 => (self.prg[0] as usize & 0x07) | (reg << 3),
                _ => 0x07 | (reg << 3),
            }
        }
    }

    fn mirror(&self, address: u16) -> u16 {
        if self.reg & 0x08 != 0 {
            if self.latch_data & 0x10 != 0 {
                (address & 0x33FF) | 0x0400
            } else {
                address & 0x33FF
            }
        } else {
            match self.mirr & 3 {
                0 => address & 0x37FF,
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                2 => address & 0x33FF,
                _ => (address & 0x33FF) | 0x0400,
            }
        }
    }

    fn read_wram(&self, cart: &Cartridge, address: u16) -> FetchResult {
        if !self.wram_enable || cart.prg_ram.is_empty() {
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        let idx = (address as usize & 0x1FFF) % cart.prg_ram.len();
        FetchResult {
            data: cart.prg_ram[idx],
            driven: true,
        }
    }

    fn write_wram(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if !self.wram_enable || cart.prg_ram.is_empty() {
            return;
        }
        let idx = (address as usize & 0x1FFF) % cart.prg_ram.len();
        cart.prg_ram[idx] = data;
    }

    fn vrc4_store(&mut self, address: u16, data: u8) {
        let a0 = 0x04;
        let a1 = 0x08;
        match address >> 12 {
            0x8 => self.prg[0] = data,
            0x9 => {
                let reg = (if address & a1 != 0 { 2 } else { 0 })
                    | (if address & a0 != 0 { 1 } else { 0 });
                match reg {
                    0 | 1 => self.mirr = data & 3,
                    2 => {
                        self.wram_enable = (data & 1) != 0;
                        self.prg_flip = if data & 2 != 0 { 4 } else { 0 };
                    }
                    _ => {}
                }
            }
            0xA => self.prg[1] = data,
            0xB..=0xE => {
                let index = ((((address >> 12) - 0xB) as usize) << 1)
                    | (if address & a1 != 0 { 1 } else { 0 });
                if address & a0 != 0 {
                    self.chr[index] = (self.chr[index] & 0x000F) | ((data as u16) << 4);
                } else {
                    self.chr[index] = (self.chr[index] & 0x0FF0) | ((data as u16) & 0x000F);
                }
            }
            0xF => {
                let reg = (if address & a1 != 0 { 2 } else { 0 })
                    | (if address & a0 != 0 { 1 } else { 0 });
                match reg {
                    0 => {
                        self.latch = (self.latch & 0xF0) | (data & 0x0F);
                    }
                    1 => {
                        self.latch = (self.latch & 0x0F) | (data << 4);
                    }
                    2 => {
                        self.irq = data;
                        if self.irq & 2 != 0 {
                            self.counter = self.latch;
                            self.cycles = 341;
                        }
                        self.irq_asserted = false;
                        self.irq_ack = true;
                    }
                    _ => {
                        self.irq = (self.irq & !2) | ((self.irq & 1) << 1);
                        self.irq_asserted = false;
                        self.irq_ack = true;
                    }
                }
            }
            _ => {}
        }
    }

    fn vrc4_cpu_clock(&mut self) -> bool {
        if self.irq_raise_count != 0 {
            self.irq_raise_count -= 1;
            if self.irq_raise_count == 0 {
                self.irq_asserted = true;
            }
        }
        if self.irq & 0x02 != 0 {
            let fast = (self.irq & 0x04) != 0;
            let fired = if fast {
                true
            } else {
                self.cycles -= 3;
                self.cycles <= 0
            };
            if fired {
                if !fast {
                    self.cycles += 341;
                }
                self.counter = self.counter.wrapping_add(1);
                if self.counter == 0 {
                    self.counter = self.latch;
                    self.irq_raise_count = 0;
                    self.irq_asserted = true;
                }
            }
        }
        self.irq_asserted
    }
}

impl Mapper for Mapper448 {
    fn reset(&mut self) {
        *self = Self::new();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            let addr = address as usize;
            if self.reg & 0x08 != 0 {
                let bank = (self.latch_data as usize & 0x07) | ((self.reg as usize) << 2 & 0xF8);
                let offset = bank * 0x8000 + (addr & 0x7FFF);
                return FetchResult {
                    data: cart.prg_rom[offset % len],
                    driven: true,
                };
            }
            let bank = self.prg_bank16(if addr < 0xC000 { 0 } else { 1 });
            let offset = bank * 0x4000 + (addr & 0x3FFF);
            return FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            };
        }
        if address >= 0x6000 {
            return self.read_wram(cart, address);
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) {
            if self.wram_enable {
                self.reg = (address & 0xFF) as u8;
            }
            self.write_wram(cart, address, data);
        } else if address >= 0x8000 {
            self.latch_data = data;
            self.vrc4_store(address, data);
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror(address)
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
        if address >= 0x2000 {
            let mirrored = self.mirror(address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
            return (new_addr_bus as u8, new_addr_bus);
        }
        let byte = if !chr_ram.is_empty() {
            chr_ram[(address as usize & 0x1FFF) % chr_ram.len()]
        } else if !chr_rom.is_empty() {
            chr_rom[(address as usize & 0x1FFF) % chr_rom.len()]
        } else {
            0
        };
        new_addr_bus |= byte as u16;
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror(address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock_rise(&mut self, _ppu_address_bus: u16) -> bool {
        self.vrc4_cpu_clock()
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack;
        self.irq_ack = false;
        ack
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        for p in &self.prg {
            state.push(*p);
        }
        for c in &self.chr {
            state.extend_from_slice(&c.to_le_bytes());
        }
        state.push(self.mirr);
        state.push(self.irq);
        state.push(self.counter);
        state.push(self.latch);
        state.extend_from_slice(&self.cycles.to_le_bytes());
        state.push(self.prg_flip);
        state.push(self.irq_raise_count);
        state.push(if self.wram_enable { 1 } else { 0 });
        state.push(if self.irq_asserted { 1 } else { 0 });
        state.push(self.reg);
        state.push(self.latch_data);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..2 {
            if p < state.len() {
                self.prg[i] = state[p];
                p += 1;
            }
        }
        for i in 0..8 {
            if p + 1 < state.len() {
                self.chr[i] = u16::from_le_bytes([state[p], state[p + 1]]);
                p += 2;
            }
        }
        if p < state.len() {
            self.mirr = state[p];
            p += 1;
        }
        if p < state.len() {
            self.irq = state[p];
            p += 1;
        }
        if p < state.len() {
            self.counter = state[p];
            p += 1;
        }
        if p < state.len() {
            self.latch = state[p];
            p += 1;
        }
        if p + 1 < state.len() {
            self.cycles = i16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.prg_flip = state[p];
            p += 1;
        }
        if p < state.len() {
            self.irq_raise_count = state[p];
            p += 1;
        }
        if p < state.len() {
            self.wram_enable = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.irq_asserted = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.reg = state[p];
            p += 1;
        }
        if p < state.len() {
            self.latch_data = state[p];
            p += 1;
        }
        p
    }
}
