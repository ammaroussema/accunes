use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};
use crate::mappers::mmc3::mmc3_chr_bank;

const FLASH_MANUFACTURER_ID: u8 = 0x37;
const FLASH_MODEL_ID: u8 = 0x86;
const FLASH_CHIP_SIZE: usize = 65536;
const FLASH_SECTOR_SIZE: usize = 65536;
const FLASH_MAGIC_ADDR_1: u16 = 0x555;
const FLASH_MAGIC_ADDR_2: u16 = 0x2AA;

pub struct Mapper451 {
    pointer: u8,
    reg: [u8; 8],
    mirroring: u8,
    wram: u8,
    counter: u8,
    prescaler: u8,
    reload_value: u8,
    reload: bool,
    enable_irq: bool,
    m2_filter: u8,
    flash_state: u8,
    flash_time_out: u32,
    flash_data: Vec<u8>,
    irq_ack: bool,
}

impl Mapper451 {
    pub fn new(header: &[u8], rom: &[u8], prg_size: u8) -> Self {
        let has_trainer = (header.get(6).copied().unwrap_or(0) & 4) != 0;
        let prg_start: usize = 0x10 + if has_trainer { 512 } else { 0 };
        let prg_len: usize = (prg_size as usize) * 0x4000;
        let end = prg_start.saturating_add(prg_len).min(rom.len());
        let flash_data = if end > prg_start {
            rom[prg_start..end].to_vec()
        } else {
            Vec::new()
        };
        Self {
            pointer: 0,
            reg: [0x00, 0x02, 0x04, 0x05, 0x06, 0x07, 0x00, 0x01],
            mirroring: 0,
            wram: 0,
            counter: 0,
            prescaler: 7,
            reload_value: 0,
            reload: false,
            enable_irq: false,
            m2_filter: 0,
            flash_state: 0,
            flash_time_out: 0,
            flash_data,
            irq_ack: false,
        }
    }

    fn prg_invert(&self) -> bool {
        (self.pointer & 0x40) != 0
    }

    fn prg_bank8(&self, slot: u8) -> u16 {
        match slot {
            0 => 0x00,
            1 => (self.reg[7] as u16) & 0x3F,
            2 => {
                if self.prg_invert() {
                    (self.reg[6] as u16) & 0x3F
                } else {
                    0x3E
                }
            }
            _ => 0x30,
        }
    }

    fn flash_offset(&self, bank: u8, addr: u16) -> usize {
        let slot = ((bank >> 1) & 0x03) as u8;
        let bank8 = self.prg_bank8(slot);
        let half = if bank & 1 != 0 { 0x1000 } else { 0 };
        (bank8 as usize) * 0x2000 + half + (addr as usize & 0xFFF)
    }

    fn flash_read(&self, bank: u8, addr: u16) -> u8 {
        if self.flash_state == 0x90 {
            return if addr & 1 != 0 {
                FLASH_MODEL_ID
            } else {
                FLASH_MANUFACTURER_ID
            };
        }
        let raw = {
            let len = self.flash_data.len();
            if len == 0 {
                0
            } else {
                let offset = self.flash_offset(bank, addr);
                self.flash_data[offset % len]
            }
        };
        if self.flash_time_out > 0 {
            let xor = if self.flash_time_out & 1 != 0 { 0x40 } else { 0 };
            (raw ^ xor) & !0x88
        } else {
            raw
        }
    }

    fn flash_write(&mut self, bank: u8, addr: u16, val: u8) {
        match self.flash_state {
            0x01 => {
                if addr == FLASH_MAGIC_ADDR_2 && val == 0x55 {
                    self.flash_state = 0x02;
                }
            }
            0x02 => {
                if addr == FLASH_MAGIC_ADDR_1 {
                    self.flash_state = val;
                }
            }
            0x80 => {
                if addr == FLASH_MAGIC_ADDR_1 && val == 0xAA {
                    self.flash_state = 0x81;
                }
            }
            0x81 => {
                if addr == FLASH_MAGIC_ADDR_2 && val == 0x55 {
                    self.flash_state = 0x82;
                }
            }
            0x82 => {
                if val == 0x30 {
                    let offset = self.flash_offset(bank, addr);
                    if offset < FLASH_CHIP_SIZE {
                        let sector_start = offset & !(FLASH_SECTOR_SIZE - 1);
                        if sector_start < self.flash_data.len() {
                            let end = (sector_start + FLASH_SECTOR_SIZE).min(self.flash_data.len());
                            for b in &mut self.flash_data[sector_start..end] {
                                *b = 0xFF;
                            }
                        }
                        self.flash_time_out = FLASH_SECTOR_SIZE as u32;
                    }
                } else if val == 0x10 && addr == FLASH_MAGIC_ADDR_1 {
                    let len = self.flash_data.len();
                    for b in &mut self.flash_data[..FLASH_CHIP_SIZE.min(len)] {
                        *b = 0xFF;
                    }
                    self.flash_time_out = FLASH_CHIP_SIZE as u32;
                } else if val == 0xF0 {
                    self.flash_state = 0;
                }
            }
            0x90 => {
                if val == 0xF0 {
                    self.flash_state = 0;
                }
            }
            0xA0 => {
                let len = self.flash_data.len();
                if len > 0 {
                    let offset = self.flash_offset(bank, addr);
                    self.flash_data[offset % len] = val;
                }
                self.flash_state = 0;
            }
            _ => {
                if addr == FLASH_MAGIC_ADDR_1 && val == 0xAA {
                    self.flash_state = 0x01;
                }
            }
        }
    }

    fn write_reg(&mut self, d2: bool, val: u8) {
        if d2 {
            self.reg[(self.pointer & 7) as usize] = val;
        } else {
            self.pointer = val;
        }
    }

    fn chr_bank(&self, address: u16) -> u8 {
        mmc3_chr_bank(
            self.pointer,
            self.reg[0],
            self.reg[1],
            self.reg[2],
            self.reg[3],
            self.reg[4],
            self.reg[5],
            address,
        )
    }
}

impl Mapper for Mapper451 {
    fn reset(&mut self) {
        self.pointer = 0;
        self.reg = [0x00, 0x02, 0x04, 0x05, 0x06, 0x07, 0x00, 0x01];
        self.mirroring = 0;
        self.wram = 0;
        self.counter = 0;
        self.prescaler = 7;
        self.reload_value = 0;
        self.reload = false;
        self.enable_irq = false;
        self.m2_filter = 0;
        self.irq_ack = false;
    }

    fn fetch_prg(&mut self, _cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = (address >> 12) as u8;
            let addr = address & 0xFFF;
            return FetchResult {
                data: self.flash_read(bank, addr),
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            return;
        }
        let bank = (address >> 12) as u8;
        let addr = address & 0xFFF;
        let mut flash_addr = addr;
        match bank & !1 {
            0xA => {
                self.mirroring = (addr & 1) as u8;
            }
            0xC => {
                self.reload_value = (addr as u8).wrapping_sub(1);
                self.counter = 0;
                self.prescaler = 7;
                self.reload = true;
                self.enable_irq = addr != 0xFF;
                if !self.enable_irq {
                    self.irq_ack = true;
                }
            }
            0xE => {
                let a = (((addr << 2) & 8) | (addr & 1)) as u8;
                self.write_reg(false, 0x40);
                self.write_reg(true, (a << 3) | 0);
                self.write_reg(false, 0x41);
                self.write_reg(true, (a << 3) | 2);
                self.write_reg(false, 0x42);
                self.write_reg(true, (a << 3) | 4);
                self.write_reg(false, 0x43);
                self.write_reg(true, (a << 3) | 5);
                self.write_reg(false, 0x44);
                self.write_reg(true, (a << 3) | 6);
                self.write_reg(false, 0x45);
                self.write_reg(true, (a << 3) | 7);
                self.write_reg(false, 0x46);
                self.write_reg(true, 0x20 | a);
                self.write_reg(false, 0x47);
                self.write_reg(true, 0x10 | a);
                flash_addr = a as u16;
            }
            _ => {}
        }
        self.flash_write(bank, flash_addr, data);
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v((self.mirroring & 1) != 0, address)
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
        } else {
            let mirrored = mirror_h_or_v((self.mirroring & 1) != 0, address);
            let byte = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= byte as u16;
        }
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
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        ppu_a12_prev: bool,
        _scanline: u16,
        _dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        let a12 = (ppu_address_bus & 0x1000) != 0;
        let mut irq = false;
        if !ppu_a12_prev && a12 && self.m2_filter == 3 {
            let reset_reload = self.reload;
            if self.counter == 0 || reset_reload {
                self.counter = self.reload_value;
            } else {
                self.counter = self.counter.wrapping_sub(1);
            }
            if self.counter == 0 && self.enable_irq {
                irq = true;
            }
            self.reload = false;
        }
        if a12 {
            self.m2_filter = 0;
        }
        irq
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        let a12 = (ppu_address_bus & 0x1000) != 0;
        if !a12 && self.m2_filter < 3 {
            self.m2_filter += 1;
        }
        false
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.flash_time_out > 0 {
            self.flash_time_out -= 1;
            if self.flash_time_out == 0 {
                self.flash_state = 0;
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
        state.push(self.pointer);
        state.extend_from_slice(&self.reg);
        state.push(self.mirroring);
        state.push(self.wram);
        state.push(self.counter);
        state.push(self.prescaler);
        state.push(self.reload_value);
        state.push(self.reload as u8);
        state.push(self.enable_irq as u8);
        state.push(self.m2_filter);
        state.push(self.flash_state);
        state.extend_from_slice(&self.flash_time_out.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.pointer = state[p];
        p += 1;
        for i in 0..8 {
            self.reg[i] = state[p];
            p += 1;
        }
        self.mirroring = state[p];
        p += 1;
        self.wram = state[p];
        p += 1;
        self.counter = state[p];
        p += 1;
        self.prescaler = state[p];
        p += 1;
        self.reload_value = state[p];
        p += 1;
        self.reload = state[p] != 0;
        p += 1;
        self.enable_irq = state[p] != 0;
        p += 1;
        self.m2_filter = state[p];
        p += 1;
        self.flash_state = state.get(p).copied().unwrap_or(0);
        p += 1;
        let to = state.get(p..p + 4).unwrap_or(&[0, 0, 0, 0]);
        self.flash_time_out = u32::from_le_bytes([to[0], to[1], to[2], to[3]]);
        p += 4;
        p
    }
}
