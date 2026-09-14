use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper117 {
    prg_banks: [u8; 4],
    chr_banks: [u8; 8],
    mirroring: u8,
    irq_mode: u8,
    irq_enabled: bool,
    counter: u16,
    reload: bool,
    pa12_filter: u8,
    irq_ack: bool,
    irq_pending: bool,
}

impl Mapper117 {
    pub fn new() -> Self {
        Self {
            prg_banks: [0xFC, 0xFD, 0xFE, 0xFF],
            chr_banks: [0, 1, 2, 3, 4, 5, 6, 7],
            mirroring: 0,
            irq_mode: 0,
            irq_enabled: false,
            counter: 0,
            reload: false,
            pa12_filter: 0,
            irq_ack: false,
            irq_pending: false,
        }
    }

    fn prg_bank(&self, cart: &Cartridge, bank: u8) -> usize {
        let num_8k = cart.prg_rom.len() / 0x2000;
        if num_8k == 0 { 0 } else { bank as usize % num_8k }
    }
}

impl Mapper for Mapper117 {
    fn reset(&mut self) {
        self.prg_banks = [0xFC, 0xFD, 0xFE, 0xFF];
        self.chr_banks = [0, 1, 2, 3, 4, 5, 6, 7];
        self.mirroring = 0;
        self.irq_mode = 0;
        self.counter = 0;
        self.pa12_filter = 0;
        self.reload = false;
        self.irq_enabled = false;
        self.irq_ack = false;
        self.irq_pending = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x6000 {
            return FetchResult { data: 0, driven: false };
        }
        let (bank, offset_base) = if address < 0x8000 {
            (self.prg_banks[3] as usize, 0x6000)
        } else if address < 0xA000 {
            (self.prg_bank(cart, self.prg_banks[0]), 0x8000)
        } else if address < 0xC000 {
            (self.prg_bank(cart, self.prg_banks[1]), 0xA000)
        } else if address < 0xE000 {
            (self.prg_bank(cart, self.prg_banks[2]), 0xC000)
        } else {
            let num_8k = cart.prg_rom.len() / 0x2000;
            (num_8k.saturating_sub(1), 0xE000)
        };
        let offset = bank * 0x2000 + (address as usize - offset_base);
        FetchResult {
            data: cart.prg_rom[offset % cart.prg_rom.len()],
            driven: true,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            return;
        }
        match address & 0xF000 {
            0x8000 => self.prg_banks[(address & 0x03) as usize] = data,
            0x9000 => {}
            0xA000 | 0xB000 => {
                if address & 0x0008 == 0 {
                    self.chr_banks[(address & 0x07) as usize] = data;
                }
            }
            0xC000 => {
                match address & 0x0003 {
                    0 => self.counter = (self.counter & 0xFF00) | data as u16,
                    1 => {
                        self.counter = (self.counter & 0x00FF) | ((data as u16) << 8);
                        self.reload = true;
                    }
                    2 => self.irq_enabled = false,
                    3 => self.irq_enabled = true,
                    _ => {}
                }
                self.irq_ack = true;
                self.irq_pending = false;
            }
            0xD000 => self.mirroring = data & 0x03,
            0xE000 => {
                self.irq_mode = data;
                if (data & 0x01) == 0 {
                    self.irq_enabled = false;
                }
            }
            0xF000 => {}
            _ => {}
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        match self.mirroring {
            0 => address & 0x37FF,
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            2 => (address & 0x03FF) | 0x2000,
            3 => (address & 0x03FF) | 0x2400,
            _ => unreachable!(),
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
            let slot = (address >> 10) & 7;
            let bank = self.chr_banks[slot as usize] as usize;
            let offset = bank * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[offset % chr_ram.len()] }
            } else if chr_rom.is_empty() {
                0
            } else {
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = match self.mirroring {
                0 => address & 0x37FF,
                1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                2 => (address & 0x03FF) | 0x2000,
                3 => (address & 0x03FF) | 0x2400,
                _ => unreachable!(),
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let slot = (address >> 10) & 7;
            let bank = self.chr_banks[slot as usize] as usize;
            let offset = bank * 0x0400 + (address as usize & 0x03FF);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.pa12_filter > 0 {
            self.pa12_filter -= 1;
        }
        if self.irq_enabled && self.irq_mode & 0x02 == 0 {
            if self.counter > 0 {
                self.counter -= 1;
                if self.counter == 0 {
                    self.irq_pending = true;
                }
            }
        }
        let fired = self.irq_pending;
        if fired {
            self.irq_pending = false;
        }
        fired
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
        if self.irq_mode & 0x01 != 0 && ppu_address_bus & 0x1000 != 0 {
            if self.pa12_filter == 0 && self.irq_mode & 0x02 != 0 {
                let b0 = (self.counter & 0xFF) as u8;
                let b1 = (self.counter >> 8) as u8;
                let new_b0 = if b0 == 0 || self.reload { b1 } else { b0.wrapping_sub(1) };
                self.counter = (self.counter & 0xFF00) | new_b0 as u16;
                if new_b0 == 0 && self.irq_enabled {
                    self.irq_pending = true;
                }
                self.reload = false;
            }
            self.pa12_filter = 5;
        }
        let fired = self.irq_pending;
        if fired {
            self.irq_pending = false;
        }
        fired
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.irq_ack {
            self.irq_ack = false;
            true
        } else {
            false
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg_banks);
        state.extend_from_slice(&self.chr_banks);
        state.push(self.mirroring);
        state.push(self.irq_mode);
        state.push((self.counter >> 8) as u8);
        state.push(self.counter as u8);
        state.push(if self.reload { 1 } else { 0 });
        state.push(if self.irq_enabled { 1 } else { 0 });
        state.push(self.pa12_filter);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut i = start;
        for r in 0..4 { self.prg_banks[r] = state[i]; i += 1; }
        for r in 0..8 { self.chr_banks[r] = state[i]; i += 1; }
        self.mirroring = state[i]; i += 1;
        self.irq_mode = state[i]; i += 1;
        self.counter = ((state[i] as u16) << 8) | state[i + 1] as u16; i += 2;
        self.reload = state[i] != 0; i += 1;
        self.irq_enabled = state[i] != 0; i += 1;
        self.pa12_filter = state[i]; i += 1;
        self.irq_ack = false;
        self.irq_pending = false;
        i - start
    }
}
