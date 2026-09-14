use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper73 {
    prg_bank: u8,
    irq: u8,
    counter: u16,
    latch: u16,
    irq_pending: bool,
}

impl Mapper73 {
    pub fn new() -> Self {
        Self {
            prg_bank: 0,
            irq: 0,
            counter: 0,
            latch: 0,
            irq_pending: false,
        }
    }
}

impl Mapper for Mapper73 {
    fn reset(&mut self) {
        self.prg_bank = 0;
        self.irq = 0;
        self.counter = 0;
        self.latch = 0;
        self.irq_pending = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let off = (address as usize - 0x6000) % cart.prg_ram.len();
                return FetchResult {
                    data: cart.prg_ram[off],
                    driven: true,
                };
            }
            return FetchResult { data: 0, driven: false };
        }
        if address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }
        let num_16k = cart.prg_rom.len() / 0x4000;
        if num_16k == 0 {
            return FetchResult { data: 0, driven: true };
        }
        let bank = if address >= 0xC000 {
            num_16k - 1
        } else {
            self.prg_bank as usize % num_16k
        };
        let offset = bank * 0x4000 + (address as usize & 0x3FFF);
        FetchResult {
            data: cart.prg_rom[offset % cart.prg_rom.len()],
            driven: true,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let off = (address as usize - 0x6000) % cart.prg_ram.len();
                cart.prg_ram[off] = data;
            }
            return;
        }
        if address < 0x8000 {
            return;
        }
        match address & 0xF000 {
            0x8000 | 0x9000 | 0xA000 | 0xB000 => {
                let shift = (((address >> 12) & 0x3) as u16) << 2;
                let nibble = (data as u16 & 0x0F) << shift;
                self.latch = (self.latch & !(0xF << shift)) | nibble;
            }
            0xC000 => {
                self.irq = data;
                if self.irq & 0x02 != 0 {
                    self.counter = self.latch;
                }
                self.irq_pending = false;
            }
            0xD000 => {
                self.irq = (self.irq & !0x02) | ((self.irq << 1) & 0x01);
                self.irq_pending = false;
            }
            0xF000 => {
                self.prg_bank = data;
            }
            _ => {}
        }
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
        chr_ram: &[u8],
        _prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let offset = address as usize & 0x1FFF;
            let byte = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[offset % chr_ram.len()] }
            } else if chr_rom.is_empty() {
                0
            } else {
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if nametable_horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let offset = address as usize & 0x1FFF;
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn needs_cpu_clock(&self) -> bool { true }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        for _ in 0..cycles as u16 {
            if self.irq & 0x02 != 0 {
                let mask: u16 = if self.irq & 0x04 != 0 { 0x00FF } else { 0xFFFF };
                if self.counter & mask == mask {
                    self.counter = self.latch;
                    self.irq_pending = true;
                } else {
                    self.counter = self.counter.wrapping_add(1);
                }
            }
        }
        self.irq_pending
    }

    fn cpu_clock_irq_level(&self) -> bool {
        self.irq_pending
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.prg_bank);
        state.push(self.irq);
        state.extend_from_slice(&self.counter.to_le_bytes());
        state.extend_from_slice(&self.latch.to_le_bytes());
        state.push(self.irq_pending as u8);
        state.extend_from_slice(&cart.prg_ram);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut i = start;
        self.prg_bank = state[i]; i += 1;
        self.irq = state[i]; i += 1;
        self.counter = u16::from_le_bytes([state[i], state[i + 1]]); i += 2;
        self.latch = u16::from_le_bytes([state[i], state[i + 1]]); i += 2;
        self.irq_pending = state[i] != 0; i += 1;
        let prg_ram_len = cart.prg_ram.len();
        if prg_ram_len > 0 {
            let copy_len = prg_ram_len.min(state.len() - i);
            cart.prg_ram[..copy_len].copy_from_slice(&state[i..i + copy_len]);
            i += copy_len;
        }
        i - start
    }
}