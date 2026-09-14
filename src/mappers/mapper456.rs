use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};
use crate::mappers::mmc3::mmc3_chr_bank;

pub struct Mapper456 {
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
    reg_latch: u8,
}

impl Mapper456 {
    pub fn new() -> Self {
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
            reg_latch: 0,
        }
    }

    fn prg_invert(&self) -> bool {
        (self.pointer & 0x40) != 0
    }

    fn prg_slot_bank(&self, slot: u8) -> u16 {
        let mut bank = slot;
        if bank & 1 == 0 && self.prg_invert() {
            bank ^= 2;
        }
        let base = if bank & 2 != 0 {
            0xFE | (bank & 1)
        } else {
            self.reg[(6 | (bank & 1)) as usize]
        };
        ((base as u16) & 0x0F) | ((self.reg_latch as u16) << 4)
    }

    fn chr_bank(&self, address: u16) -> u16 {
        let bank = mmc3_chr_bank(
            self.pointer,
            self.reg[0],
            self.reg[1],
            self.reg[2],
            self.reg[3],
            self.reg[4],
            self.reg[5],
            address,
        );
        ((bank as u16) & 0x7F) | ((self.reg_latch as u16) << 7)
    }
}

impl Mapper for Mapper456 {
    fn reset(&mut self) {
        *self = Self::new();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            if (self.wram & 0x80) != 0 && !cart.prg_ram.is_empty() {
                let idx = (address as usize & 0x1FFF) % cart.prg_ram.len();
                return FetchResult {
                    data: cart.prg_ram[idx],
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
            let slot = ((address >> 13) & 3) as u8;
            let bank = self.prg_slot_bank(slot);
            let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
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
            if (self.wram & 0x80) != 0 && (self.wram & 0x40) == 0 && !cart.prg_ram.is_empty() {
                let idx = (address as usize & 0x1FFF) % cart.prg_ram.len();
                cart.prg_ram[idx] = data;
            }
            return;
        }
        if address < 0x8000 {
            return;
        }
        match address & 0xE001 {
            0x8000 => self.pointer = data,
            0x8001 => self.reg[(self.pointer & 7) as usize] = data,
            0xA000 => self.mirroring = data,
            0xA001 => self.wram = data,
            0xC000 => self.reload_value = data,
            0xC001 => {
                self.counter = 0;
                self.prescaler = 7;
                self.reload = true;
            }
            0xE000 => self.enable_irq = false,
            0xE001 => self.enable_irq = true,
            _ => {}
        }
    }

    fn handle_cpu_write(&mut self, address: u16, data: u8) {
        if (0x4000..0x6000).contains(&address) && (address & 0x100) != 0 {
            self.reg_latch = data;
        }
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
            let mirrored = mirror_h_or_v((self.mirroring & 1) != 0, address);
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
        state.push(self.reg_latch);
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
        self.reg_latch = state.get(p).copied().unwrap_or(0);
        p += 1;
        p
    }
}
