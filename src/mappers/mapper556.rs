use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper556 {
    mmc3: MapperMMC3,
    reg_num: u8,
    prg_and: u8,
    prg_or: u16,
    chr_and: u8,
    chr_or: u16,
    vrc4mode: bool,
    locked: bool,
    irq_ack_pending: bool,
    vrc_prg: [u8; 2],
    vrc_chr: [u16; 8],
    vrc_mirroring: u8,
    vrc_prg_flip: u8,
    vrc_wram_enable: bool,
    vrc_irq: u8,
    vrc_counter: u8,
    vrc_latch: u8,
    vrc_cycles: i32,
}

impl Mapper556 {
    pub fn new(
        header: &[u8],
        submapper_id: u8,
        chr_size: u8,
        rom: &[u8],
        rom_name: &str,
        has_battery: bool,
    ) -> Self {
        let mut config = Mmc3Config::for_ines(header, submapper_id, chr_size, rom, rom_name);
        config.ax5202p = true;
        config.irq_revision_b = false;
        if has_battery {
            config.prg_ram_size = config.prg_ram_size.max(0x2000);
        }
        Self {
            mmc3: MapperMMC3::new(config),
            reg_num: 0,
            prg_and: 0x3F,
            prg_or: 0,
            chr_and: 0xFF,
            chr_or: 0,
            vrc4mode: false,
            locked: false,
            irq_ack_pending: false,
            vrc_prg: [0, 1],
            vrc_chr: [0, 1, 2, 3, 4, 5, 6, 7],
            vrc_mirroring: 0,
            vrc_prg_flip: 0,
            vrc_wram_enable: true,
            vrc_irq: 0,
            vrc_counter: 0,
            vrc_latch: 0,
            vrc_cycles: 341,
        }
    }

    fn write_reg(&mut self, data: u8) {
        if !self.locked {
            match self.reg_num & 3 {
                0 => self.chr_or = (self.chr_or & !0x00FF) | data as u16,
                1 => self.prg_or = (self.prg_or & !0x00FF) | data as u16,
                2 => {
                    self.chr_and = (0xFFu16 >> ((!data) & 0xF)) as u8;
                    self.chr_or = (self.chr_or & !0x0F00) | (((data & 0xF0) as u16) << 4);
                    self.vrc4mode = (data & 0x80) != 0;
                }
                3 => {
                    self.prg_and = !data & 0x3F;
                    self.prg_or = (self.prg_or & !0x0100) | (((data & 0x40) as u16) << 2);
                    self.chr_or = (self.chr_or & !0x1000) | (((data & 0x40) as u16) << 6);
                    self.locked = (data & 0x80) != 0;
                }
                _ => {}
            }
            self.reg_num = self.reg_num.wrapping_add(1);
        }
    }

    fn mmc3_prg_bank_raw(&self, address: u16) -> u16 {
        let invert = (self.mmc3.r8000 & 0x40) != 0;
        match address & 0xE000 {
            0xE000 => 0xFF,
            0xC000 => {
                if invert { self.mmc3.bank_8c as u16 } else { 0xFE }
            }
            0xA000 => self.mmc3.bank_a as u16,
            _ => {
                if invert { 0xFE } else { self.mmc3.bank_8c as u16 }
            }
        }
    }

    fn vrc4_prg_bank_raw(&self, address: u16) -> u16 {
        let flip = self.vrc_prg_flip != 0;
        let bank_8000 = if flip { 0xFE } else { self.vrc_prg[0] as u16 };
        let bank_c000 = if flip { self.vrc_prg[0] as u16 } else { 0xFE };
        match address & 0xE000 {
            0xE000 => 0xFF,
            0xC000 => bank_c000,
            0xA000 => self.vrc_prg[1] as u16,
            _ => bank_8000,
        }
    }

    fn prg_bank(&self, address: u16) -> usize {
        let raw = if self.vrc4mode {
            self.vrc4_prg_bank_raw(address)
        } else {
            self.mmc3_prg_bank_raw(address)
        };
        ((raw & self.prg_and as u16) | self.prg_or) as usize
    }

    fn chr_bank(&self, address: u16) -> usize {
        let raw = if self.vrc4mode {
            self.vrc_chr[(address >> 10) as usize & 7]
        } else {
            self.mmc3.chr_bank(address) as u16
        };
        ((raw & self.chr_and as u16) | self.chr_or) as usize
    }

    fn vrc4_write(&mut self, address: u16, data: u8) {
        let bank = (address >> 12) & 0x0F;
        match bank {
            0x8 | 0xA => {
                self.vrc_prg[(bank >> 1 & 1) as usize] = data;
            }
            0x9 => {
                let reg = (((address & 0x0A) != 0) as u8 * 2) | ((address & 0x05) != 0) as u8;
                match reg & 3 {
                    0 | 1 => self.vrc_mirroring = data & 3,
                    2 => {
                        self.vrc_wram_enable = (data & 1) != 0;
                        self.vrc_prg_flip = if data & 2 != 0 { 4 } else { 0 };
                    }
                    3 => {}
                    _ => {}
                }
            }
            0xB | 0xC | 0xD | 0xE => {
                let reg = (((bank - 0xB) << 1) | ((address & 0x0A) != 0) as u16) as u8;
                let idx = reg as usize;
                if (address & 0x05) != 0 {
                    self.vrc_chr[idx] = (self.vrc_chr[idx] & 0x00F) | ((data as u16) << 4);
                } else {
                    self.vrc_chr[idx] = (self.vrc_chr[idx] & 0xFF0) | (data & 0xF) as u16;
                }
            }
            0xF => {
                let reg = (((address & 0x0A) != 0) as u8 * 2) | ((address & 0x05) != 0) as u8;
                match reg {
                    0 => self.vrc_latch = (self.vrc_latch & 0xF0) | (data & 0xF),
                    1 => self.vrc_latch = (self.vrc_latch & 0x0F) | (data << 4),
                    2 => {
                        self.vrc_irq = data;
                        if self.vrc_irq & 2 != 0 {
                            self.vrc_counter = self.vrc_latch;
                            self.vrc_cycles = 341;
                        }
                    }
                    3 => {
                        if self.vrc_irq & 1 != 0 {
                            self.vrc_irq |= 2;
                        } else {
                            self.vrc_irq &= !2;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn vrc4_cpu_cycle(&mut self) -> bool {
        if self.vrc_irq & 2 != 0 {
            let count = if self.vrc_irq & 4 != 0 {
                true
            } else {
                self.vrc_cycles -= 3;
                if self.vrc_cycles > 0 {
                    false
                } else {
                    self.vrc_cycles += 341;
                    true
                }
            };
            if count {
                self.vrc_counter = self.vrc_counter.wrapping_add(1);
                if self.vrc_counter == 0 {
                    self.vrc_counter = self.vrc_latch;
                    return true;
                }
            }
        }
        false
    }

    fn vrc_mirror(&self, address: u16) -> u16 {
        match self.vrc_mirroring & 3 {
            0 => address & 0x37FF,
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            2 => address & 0x3FFF,
            3 => (address & 0x3FFF) | 0x0400,
            _ => address,
        }
    }
}

impl Mapper for Mapper556 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.reg_num = 0;
        self.prg_and = 0x3F;
        self.prg_or = 0;
        self.chr_and = 0xFF;
        self.chr_or = 0;
        self.vrc4mode = false;
        self.locked = false;
        self.irq_ack_pending = false;
        self.vrc_prg = [0, 1];
        self.vrc_chr = [0, 1, 2, 3, 4, 5, 6, 7];
        self.vrc_mirroring = 0;
        self.vrc_prg_flip = 0;
        self.vrc_wram_enable = true;
        self.vrc_irq = 0;
        self.vrc_counter = 0;
        self.vrc_latch = 0;
        self.vrc_cycles = 341;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult { data: 0, driven: false };
            }
            let bank = self.prg_bank(address);
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            }
        } else if address >= 0x6000 && address < 0x8000 {
            if self.vrc4mode {
                if self.vrc_wram_enable {
                    let off = (address - 0x6000) as usize;
                    if off < cart.prg_ram.len() {
                        FetchResult { data: cart.prg_ram[off], driven: true }
                    } else {
                        FetchResult { data: 0, driven: false }
                    }
                } else {
                    FetchResult { data: 0, driven: false }
                }
            } else {
                self.mmc3.fetch_prg(cart, address)
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        self.irq_ack_pending = false;
        if address >= 0x5000 && address < 0x6000 {
            self.write_reg(data);
        } else if address >= 0x6000 && address < 0x8000 {
            if self.vrc4mode {
                if self.vrc_wram_enable {
                    let off = (address - 0x6000) as usize;
                    if off < cart.prg_ram.len() {
                        cart.prg_ram[off] = data;
                    }
                }
            } else {
                self.mmc3.store_prg(cart, address, data);
            }
        } else if address >= 0x8000 {
            if self.vrc4mode {
                if address & 0xF000 == 0xF000 {
                    self.irq_ack_pending = true;
                }
                self.vrc4_write(address, data);
            } else {
                if (address & 0xE001) == 0xE000 {
                    self.irq_ack_pending = true;
                }
                self.mmc3.store_prg(cart, address, data);
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if self.vrc4mode {
            self.vrc_mirror(address)
        } else {
            self.mmc3.mirror_nametable(cart, address)
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
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
            let bank = self.chr_bank(address);
            let offset = bank * 0x0400 + (address as usize & 0x03FF);
            let byte = if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else if !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if self.vrc4mode {
                self.vrc_mirror(address)
            } else if self.mmc3.nametable_mirroring() {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() { prg_vram[idx] } else { 0 }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.chr_rom.is_empty() && !cart.chr_ram.is_empty() {
                let bank = self.chr_bank(address);
                let offset = bank * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                if len > 0 {
                    cart.chr_ram[offset % len] = data;
                }
            }
        } else if address < 0x3F00 {
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
        ppu_address_bus: u16,
        ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        ppu_sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        self.mmc3.ppu_clock(
            ppu_address_bus,
            ppu_a12_prev,
            scanline,
            dot,
            ppu_sprite_x16,
            rendering_on,
        )
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.vrc4mode {
            self.vrc4_cpu_cycle()
        } else {
            self.mmc3.cpu_clock(_cycles)
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack_pending;
        self.irq_ack_pending = false;
        ack
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.reg_num);
        state.push(self.prg_and);
        state.extend_from_slice(&self.prg_or.to_le_bytes());
        state.push(self.chr_and);
        state.extend_from_slice(&self.chr_or.to_le_bytes());
        state.push(if self.vrc4mode { 1 } else { 0 });
        state.push(if self.locked { 1 } else { 0 });
        state.push(if self.irq_ack_pending { 1 } else { 0 });
        state.extend_from_slice(&self.vrc_prg);
        for &c in &self.vrc_chr {
            state.extend_from_slice(&c.to_le_bytes());
        }
        state.push(self.vrc_mirroring);
        state.push(self.vrc_prg_flip);
        state.push(if self.vrc_wram_enable { 1 } else { 0 });
        state.push(self.vrc_irq);
        state.push(self.vrc_counter);
        state.push(self.vrc_latch);
        state.extend_from_slice(&self.vrc_cycles.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        self.reg_num = state.get(p).copied().unwrap_or(0); p += 1;
        self.prg_and = state.get(p).copied().unwrap_or(0x3F); p += 1;
        if p + 2 <= state.len() {
            self.prg_or = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        self.chr_and = state.get(p).copied().unwrap_or(0xFF); p += 1;
        if p + 2 <= state.len() {
            self.chr_or = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        self.vrc4mode = state.get(p).copied().unwrap_or(0) != 0; p += 1;
        self.locked = state.get(p).copied().unwrap_or(0) != 0; p += 1;
        self.irq_ack_pending = state.get(p).copied().unwrap_or(0) != 0; p += 1;
        for i in 0..2 {
            self.vrc_prg[i] = state.get(p).copied().unwrap_or(if i == 0 { 0 } else { 1 });
            p += 1;
        }
        for i in 0..8 {
            if p + 2 <= state.len() {
                self.vrc_chr[i] = u16::from_le_bytes([state[p], state[p + 1]]);
                p += 2;
            }
        }
        self.vrc_mirroring = state.get(p).copied().unwrap_or(0); p += 1;
        self.vrc_prg_flip = state.get(p).copied().unwrap_or(0); p += 1;
        self.vrc_wram_enable = state.get(p).copied().unwrap_or(1) != 0; p += 1;
        self.vrc_irq = state.get(p).copied().unwrap_or(0); p += 1;
        self.vrc_counter = state.get(p).copied().unwrap_or(0); p += 1;
        self.vrc_latch = state.get(p).copied().unwrap_or(0); p += 1;
        if p + 4 <= state.len() {
            self.vrc_cycles = i32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
            p += 4;
        }
        p
    }
}
