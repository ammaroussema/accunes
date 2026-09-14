use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper445 {
    mmc3: MapperMMC3,
    reg: [u8; 5],
    dip_switches: u8,
    vrc4_prg: [u8; 2],
    vrc4_chr: [u16; 8],
    vrc4_mirroring: u8,
    vrc4_irq: u8,
    vrc4_counter: u8,
    vrc4_latch: u8,
    vrc4_cycles: i16,
    vrc4_prg_flip: u8,
    vrc4_wram_enable: bool,
    vrc4_irq_raise_count: u8,
    vrc4_irq_asserted: bool,
    vrc4_irq_ack: bool,
}

impl Mapper445 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let config = Mmc3Config {
            ax5202p: true,
            ..Mmc3Config::for_ines(header, 0, if chr_size == 0 { 0 } else { chr_size }, rom, rom_name)
        };
        Self {
            mmc3: MapperMMC3::new(config),
            reg: [0; 5],
            dip_switches: 0,
            vrc4_prg: [0, 1],
            vrc4_chr: [0, 1, 2, 3, 4, 5, 6, 7],
            vrc4_mirroring: 0,
            vrc4_irq: 0,
            vrc4_counter: 0,
            vrc4_latch: 0,
            vrc4_cycles: 0,
            vrc4_prg_flip: 0,
            vrc4_wram_enable: true,
            vrc4_irq_raise_count: 0,
            vrc4_irq_asserted: false,
            vrc4_irq_ack: false,
        }
    }

    fn vrc4_mode(&self) -> bool {
        (self.reg[3] & 0x10) != 0
    }

    fn prg_and(&self) -> u8 {
        (0x7F >> (self.reg[2] & 7)) & 0x1F
    }

    fn prg_or(&self) -> u8 {
        self.reg[0] & !self.prg_and()
    }

    fn chr_and(&self) -> u16 {
        (0x3FF >> ((self.reg[2] >> 3) & 7)) & 0xFF
    }

    fn chr_or(&self) -> u16 {
        ((self.reg[1] as u16) << 3) & !self.chr_and()
    }

    fn dip_lockout(&self) -> bool {
        (self.reg[0] & 0xC0) != 0 && (self.reg[0] & 0xC0) == self.dip_switches
    }

    fn vrc4_a0(&self) -> u16 {
        if (self.reg[3] & 1) != 0 { 0x0A } else { 0x05 }
    }

    fn vrc4_a1(&self) -> u16 {
        if (self.reg[3] & 1) != 0 { 0x05 } else { 0x0A }
    }

    fn vrc4_mirror(&self, address: u16) -> u16 {
        match self.vrc4_mirroring & 3 {
            0 => address & 0x37FF,
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            2 => address & 0x3FFF,
            _ => (address & 0x3FFF) | 0x0400,
        }
    }

    fn prg_raw_bank(&self, cart: &Cartridge, cpu_bank: u8) -> u8 {
        let num_banks = (cart.prg_rom.len() / 0x2000) as u8;
        match cpu_bank {
            0 => {
                if (self.mmc3.r8000 & 0x40) != 0 {
                    num_banks.saturating_sub(2)
                } else {
                    self.mmc3.bank_8c
                }
            }
            1 => self.mmc3.bank_a,
            2 => {
                if (self.mmc3.r8000 & 0x40) != 0 {
                    self.mmc3.bank_8c
                } else {
                    num_banks.saturating_sub(2)
                }
            }
            _ => num_banks.saturating_sub(1),
        }
    }

    fn prg_bank8(&self, cart: &Cartridge, cpu_bank: u8) -> u8 {
        (self.prg_raw_bank(cart, cpu_bank) & self.prg_and()) | self.prg_or()
    }

    fn nrom_bank(&self, cpu_bank: u8) -> u8 {
        (cpu_bank & self.prg_and()) | self.prg_or()
    }

    fn vrc4_prg_bank(&self, cpu_bank: u8) -> u8 {
        let raw = match cpu_bank {
            1 => self.vrc4_prg[1],
            3 => 0xFF,
            _ => {
                let slot = if self.vrc4_prg_flip == 0 { 0 } else { 2 };
                if cpu_bank == slot {
                    self.vrc4_prg[0]
                } else {
                    0xFE
                }
            }
        };
        (raw & self.prg_and()) | self.prg_or()
    }

    fn chr_bank(&self, address: u16) -> u16 {
        if (self.chr_and() & 0x20) != 0 {
            let inner = if self.vrc4_mode() {
                self.vrc4_chr[(address >> 10) as usize & 0x07]
            } else {
                self.mmc3.chr_bank(address) as u16
            };
            (inner & self.chr_and()) | self.chr_or()
        } else {
            let and = (self.chr_and() >> 3) as u8;
            ((self.reg[4] & and) | (self.reg[1] & !and)) as u16
        }
    }

    fn read_wram(&self, cart: &Cartridge, address: u16) -> FetchResult {
        if self.vrc4_mode() && !self.vrc4_wram_enable {
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        if cart.prg_ram.is_empty() {
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
        if self.vrc4_mode() && !self.vrc4_wram_enable {
            return;
        }
        if !cart.prg_ram.is_empty() {
            let idx = (address as usize & 0x1FFF) % cart.prg_ram.len();
            cart.prg_ram[idx] = data;
        }
    }

    fn vrc4_store(&mut self, address: u16, data: u8) {
        let a0 = self.vrc4_a0();
        let a1 = self.vrc4_a1();
        match address >> 12 {
            0x8 => self.vrc4_prg[0] = data,
            0x9 => {
                let reg = ((if address & a1 != 0 { 2 } else { 0 })
                    | (if address & a0 != 0 { 1 } else { 0 }))
                    & 3;
                match reg {
                    0 | 1 => self.vrc4_mirroring = data & 3,
                    2 => {
                        self.vrc4_wram_enable = (data & 1) != 0;
                        self.vrc4_prg_flip = if data & 2 != 0 { 4 } else { 0 };
                    }
                    _ => {}
                }
            }
            0xA => self.vrc4_prg[1] = data,
            0xB..=0xE => {
                let reg = (((address >> 12) - 0xB) << 1)
                    | (if address & a1 != 0 { 1 } else { 0 });
                let reg = reg as usize;
                if address & a0 != 0 {
                    self.vrc4_chr[reg] =
                        (self.vrc4_chr[reg] & 0x000F) | ((data as u16) << 4);
                } else {
                    self.vrc4_chr[reg] =
                        (self.vrc4_chr[reg] & 0x0FF0) | ((data as u16) & 0x000F);
                }
            }
            0xF => {
                let reg = (if address & a1 != 0 { 2 } else { 0 })
                    | (if address & a0 != 0 { 1 } else { 0 });
                match reg {
                    0 => {
                        self.vrc4_latch = (self.vrc4_latch & 0xF0) | (data & 0x0F);
                    }
                    1 => {
                        self.vrc4_latch = (self.vrc4_latch & 0x0F) | (data << 4);
                    }
                    2 => {
                        self.vrc4_irq = data;
                        if self.vrc4_irq & 2 != 0 {
                            self.vrc4_counter = self.vrc4_latch;
                            self.vrc4_cycles = 341;
                        }
                        self.vrc4_irq_asserted = false;
                        self.vrc4_irq_ack = true;
                    }
                    _ => {
                        self.vrc4_irq = (self.vrc4_irq & !2) | ((self.vrc4_irq & 1) << 1);
                        self.vrc4_irq_asserted = false;
                        self.vrc4_irq_ack = true;
                    }
                }
            }
            _ => {}
        }
    }

    fn vrc4_cpu_clock(&mut self) -> bool {
        if self.vrc4_irq_raise_count != 0 {
            self.vrc4_irq_raise_count -= 1;
            if self.vrc4_irq_raise_count == 0 {
                self.vrc4_irq_asserted = true;
            }
        }
        if self.vrc4_irq & 0x02 != 0 {
            let fast = (self.vrc4_irq & 0x04) != 0;
            let fired = if fast {
                true
            } else {
                self.vrc4_cycles -= 3;
                self.vrc4_cycles <= 0
            };
            if fired {
                if !fast {
                    self.vrc4_cycles += 341;
                }
                self.vrc4_counter = self.vrc4_counter.wrapping_add(1);
                if self.vrc4_counter == 0 {
                    self.vrc4_counter = self.vrc4_latch;
                    self.vrc4_irq_raise_count = 0;
                    self.vrc4_irq_asserted = true;
                }
            }
        }
        self.vrc4_irq_asserted
    }
}

impl Mapper for Mapper445 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.reg = [0; 5];
        self.vrc4_prg = [0, 1];
        self.vrc4_chr = [0, 1, 2, 3, 4, 5, 6, 7];
        self.vrc4_mirroring = 0;
        self.vrc4_irq = 0;
        self.vrc4_counter = 0;
        self.vrc4_latch = 0;
        self.vrc4_cycles = 0;
        self.vrc4_prg_flip = 0;
        self.vrc4_wram_enable = true;
        self.vrc4_irq_raise_count = 0;
        self.vrc4_irq_asserted = false;
        self.vrc4_irq_ack = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if self.dip_lockout() {
                return FetchResult {
                    data: 0,
                    driven: false,
                };
            }
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            let cpu_bank = ((address - 0x8000) / 0x2000) as u8;
            let bank = if (self.prg_and() & 0x04) != 0 {
                if self.vrc4_mode() {
                    self.vrc4_prg_bank(cpu_bank)
                } else {
                    self.prg_bank8(cart, cpu_bank)
                }
            } else {
                self.nrom_bank(cpu_bank)
            };
            let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
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
        if (0x5000..0x6000).contains(&address) {
            if (self.reg[3] & 0x20) == 0 {
                let idx = (address & 3) as usize;
                self.reg[idx] = data;
            }
        } else if (0x6000..0x8000).contains(&address) {
            self.write_wram(cart, address, data);
        } else if address >= 0x8000 {
            if (self.reg[2] >> 3) & 7 >= 5 {
                self.reg[4] = data;
            } else if self.vrc4_mode() {
                self.vrc4_store(address, data);
            } else {
                self.mmc3.store_prg(cart, address, data);
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if self.vrc4_mode() {
            self.vrc4_mirror(address)
        } else {
            self.mmc3.mirror_nametable(cart, address)
        }
    }

    fn fetch_ppu(
        &mut self,
        prg_rom: &[u8],
        chr_rom: &[u8],
        prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address >= 0x2000 {
            if self.vrc4_mode() {
                let mirrored = self.vrc4_mirror(address);
                new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
            } else {
                return self.mmc3.fetch_ppu(
                    prg_rom, chr_rom, prg_ram, chr_ram, prg_vram,
                    using_chr_ram, nametable_horizontal_mirroring,
                    alternative_nametable_arrangement, ppu_address_bus, ppu_octal_latch, vram,
                );
            }
            return (new_addr_bus as u8, new_addr_bus);
        }
        let bank = self.chr_bank(address);
        let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
        let byte = if using_chr_ram && !chr_ram.is_empty() {
            chr_ram[offset % chr_ram.len()]
        } else if !chr_rom.is_empty() {
            chr_rom[offset % chr_rom.len()]
        } else {
            0
        };
        new_addr_bus |= byte as u16;
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let bank = self.chr_bank(address);
                let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            if self.vrc4_mode() {
                let mirrored = self.vrc4_mirror(address);
                vram[(mirrored & 0x7FF) as usize] = data;
            } else {
                self.mmc3.store_ppu(cart, address, data, vram);
            }
        }
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        ppu_sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        if self.vrc4_mode() {
            false
        } else {
            self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
        }
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        if self.vrc4_mode() {
            self.vrc4_cpu_clock()
        } else {
            self.mmc3.cpu_clock_rise(ppu_address_bus)
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.vrc4_irq_ack;
        self.vrc4_irq_ack = false;
        ack
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        for p in &self.vrc4_prg {
            state.push(*p);
        }
        for c in &self.vrc4_chr {
            state.extend_from_slice(&c.to_le_bytes());
        }
        state.push(self.vrc4_mirroring);
        state.push(self.vrc4_irq);
        state.push(self.vrc4_counter);
        state.push(self.vrc4_latch);
        state.extend_from_slice(&self.vrc4_cycles.to_le_bytes());
        state.push(self.vrc4_prg_flip);
        state.push(self.vrc4_irq_raise_count);
        state.push(if self.vrc4_wram_enable { 1 } else { 0 });
        for r in &self.reg {
            state.push(*r);
        }
        state.push(if self.vrc4_irq_asserted { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        for i in 0..2 {
            if p < state.len() {
                self.vrc4_prg[i] = state[p];
                p += 1;
            }
        }
        for i in 0..8 {
            if p + 1 < state.len() {
                self.vrc4_chr[i] = u16::from_le_bytes([state[p], state[p + 1]]);
                p += 2;
            }
        }
        if p < state.len() {
            self.vrc4_mirroring = state[p];
            p += 1;
        }
        if p < state.len() {
            self.vrc4_irq = state[p];
            p += 1;
        }
        if p < state.len() {
            self.vrc4_counter = state[p];
            p += 1;
        }
        if p < state.len() {
            self.vrc4_latch = state[p];
            p += 1;
        }
        if p + 1 < state.len() {
            self.vrc4_cycles = i16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.vrc4_prg_flip = state[p];
            p += 1;
        }
        if p < state.len() {
            self.vrc4_irq_raise_count = state[p];
            p += 1;
        }
        if p < state.len() {
            self.vrc4_wram_enable = state[p] != 0;
            p += 1;
        }
        for i in 0..5 {
            if p < state.len() {
                self.reg[i] = state[p];
                p += 1;
            }
        }
        if p < state.len() {
            self.vrc4_irq_asserted = state[p] != 0;
            p += 1;
        }
        p
    }
}
