use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper413 {
    reg: [u8; 4],
    serial_addr: u32,
    control: u8,
    counter: u8,
    reload_value: u8,
    enable_irq: bool,
    pa12_filter: u8,
    irq_clear_pending: bool,
}

impl Mapper413 {
    pub fn new() -> Self {
        Self {
            reg: [0; 4],
            serial_addr: 0,
            control: 0,
            counter: 0,
            reload_value: 0,
            enable_irq: false,
            pa12_filter: 0,
            irq_clear_pending: false,
        }
    }

    fn serial_read(&mut self, cart: &Cartridge) -> FetchResult {
        let misc = &cart.misc_rom;
        if misc.is_empty() {
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        let idx = (self.serial_addr as usize) % misc.len();
        let data = misc[idx];
        if (self.control & 2) != 0 {
            self.serial_addr = self.serial_addr.wrapping_add(1);
        }
        FetchResult {
            data,
            driven: true,
        }
    }

    fn prg_fetch(&self, cart: &Cartridge, address: u16, bank: u8, size: usize) -> FetchResult {
        let len = cart.prg_rom.len();
        if len == 0 {
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        let offset = (bank as usize) * size + (address as usize & (size - 1));
        FetchResult {
            data: cart.prg_rom[offset % len],
            driven: true,
        }
    }
}

impl Mapper for Mapper413 {
    fn reset(&mut self) {
        self.reg = [0; 4];
        self.serial_addr = 0;
        self.control = 0;
        self.counter = 0;
        self.reload_value = 0;
        self.enable_irq = false;
        self.pa12_filter = 0;
        self.irq_clear_pending = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        match address {
            0x4000..=0x47FF => FetchResult {
                data: 0,
                driven: false,
            },
            0x4800..=0x4FFF => self.serial_read(cart),
            0x5000..=0x5FFF => self.prg_fetch(cart, address, 0x01, 0x1000),
            0x6000..=0x7FFF => self.prg_fetch(cart, address, self.reg[0], 0x2000),
            0x8000..=0x9FFF => self.prg_fetch(cart, address, self.reg[1], 0x2000),
            0xA000..=0xBFFF => self.prg_fetch(cart, address, self.reg[2], 0x2000),
            0xC000..=0xCFFF => self.serial_read(cart),
            0xD000..=0xDFFF => self.prg_fetch(cart, address, 0x07, 0x1000),
            0xE000..=0xFFFF => self.prg_fetch(cart, address, 0x04, 0x2000),
            _ => FetchResult {
                data: 0,
                driven: false,
            },
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        match address {
            0x8000..=0x8FFF => self.reload_value = data,
            0x9000..=0x9FFF => self.counter = 0,
            0xA000..=0xAFFF => {
                self.enable_irq = false;
                self.irq_clear_pending = true;
            }
            0xB000..=0xBFFF => self.enable_irq = true,
            0xC000..=0xCFFF => {
                self.serial_addr = (self.serial_addr << 1) | ((data >> 7) as u32);
            }
            0xD000..=0xDFFF => self.control = data,
            0xE000..=0xFFFF => {
                let idx = (data >> 6) as usize;
                self.reg[idx & 3] = data & 0x3F;
            }
            _ => {}
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_clear_pending;
        self.irq_clear_pending = false;
        ack
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.nametable_horizontal_mirroring {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        _chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = if chr_rom.is_empty() {
                0
            } else {
                let bank_base = if address < 0x1000 {
                    (self.reg[3] as usize) * 0x1000
                } else {
                    0xFD * 0x1000
                };
                chr_rom[(bank_base + (address as usize & 0xFFF)) % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if nametable_horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
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
        let mut irq = false;
        if (ppu_address_bus & 0x1000) != 0 {
            if self.pa12_filter == 0 {
                self.counter = if self.counter == 0 {
                    self.reload_value
                } else {
                    self.counter - 1
                };
                if self.counter == 0 && self.enable_irq {
                    irq = true;
                }
            }
            self.pa12_filter = 5;
        }
        irq
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.pa12_filter > 0 {
            self.pa12_filter = self.pa12_filter.saturating_sub(1);
        }
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.reg);
        state.extend_from_slice(&self.serial_addr.to_le_bytes());
        state.push(self.control);
        state.push(self.counter);
        state.push(self.reload_value);
        state.push(self.enable_irq as u8);
        state.push(self.pa12_filter);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..4 {
            if p < state.len() {
                self.reg[i] = state[p];
                p += 1;
            }
        }
        if p + 3 < state.len() {
            self.serial_addr = u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
            p += 4;
        }
        if p < state.len() {
            self.control = state[p];
            p += 1;
        }
        if p < state.len() {
            self.counter = state[p];
            p += 1;
        }
        if p < state.len() {
            self.reload_value = state[p];
            p += 1;
        }
        if p < state.len() {
            self.enable_irq = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.pa12_filter = state[p];
            p += 1;
        }
        p
    }
}
