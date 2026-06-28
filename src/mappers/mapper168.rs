use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper168 {
    reg: u8,
    counter: u16,
    disable_irq: bool,
    protect_chr: bool,
    pending_ack: bool,
    chr_ram: Vec<u8>,
}

impl Mapper168 {
    pub fn new() -> Self {
        Self {
            reg: 0,
            counter: 0,
            disable_irq: false,
            protect_chr: true,
            pending_ack: false,
            chr_ram: vec![0; 0x10000],
        }
    }

    fn second_half_bank(&self) -> usize {
        ((self.reg & 0x0F) ^ 8) as usize
    }
}

impl Mapper for Mapper168 {
    fn reset(&mut self) {
        self.reg = 0;
        self.counter = 0;
        self.disable_irq = false;
        self.protect_chr = true;
        self.pending_ack = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x6000 {
            return FetchResult { data: 0, driven: true };
        }
        if address >= 0x6000 && address < 0x8000 {
            return FetchResult { data: 0, driven: true };
        }
        let len = cart.prg_rom.len();
        if len == 0 {
            return FetchResult { data: 0, driven: true };
        }
        let bank_idx = if address < 0xC000 {
            ((self.reg >> 6) as usize) * 0x4000
        } else {
            (len / 0x4000 - 1) * 0x4000
        };
        let offset = bank_idx + (address as usize & 0x3FFF);
        FetchResult { data: cart.prg_rom[offset % len], driven: true }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0xC000 {
            self.reg = data;
        } else {
            let new_disable_irq = (address & 0x80) != 0;
            if self.disable_irq && !new_disable_irq {
                self.protect_chr = false;
            }
            if !self.disable_irq && new_disable_irq {
                self.pending_ack = true;
            }
            self.disable_irq = new_disable_irq;
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
        _chr_rom: &[u8],
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
            let offset = if address < 0x1000 {
                address as usize & 0x0FFF
            } else {
                self.second_half_bank() * 0x1000 + (address as usize & 0x0FFF)
            };
            if self.protect_chr {
                new_addr_bus |= 0xFF;
            } else {
                let byte = if !self.chr_ram.is_empty() {
                    self.chr_ram[offset % self.chr_ram.len()]
                } else if using_chr_ram && !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else {
                    0
                };
                new_addr_bus |= byte as u16;
            }
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
        if address < 0x2000 && !self.protect_chr {
            let offset = if address < 0x1000 {
                address as usize & 0x0FFF
            } else {
                self.second_half_bank() * 0x1000 + (address as usize & 0x0FFF)
            };
            if !self.chr_ram.is_empty() {
                let len = self.chr_ram.len();
                self.chr_ram[offset % len] = data;
            } else if cart.using_chr_ram {
                let len = cart.chr_ram.len();
                if len > 0 {
                    cart.chr_ram[offset % len] = data;
                }
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.disable_irq {
            self.counter = 0;
            false
        } else {
            self.counter += 1;
            (self.counter & 1024) != 0
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.pending_ack {
            self.pending_ack = false;
            true
        } else {
            false
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.reg);
        state.extend_from_slice(&self.counter.to_le_bytes());
        state.push(if self.disable_irq { 1 } else { 0 });
        state.push(if self.protect_chr { 1 } else { 0 });
        state.push(if self.pending_ack { 1 } else { 0 });
        state.extend_from_slice(&self.chr_ram);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.reg = state[p]; p += 1;
        self.counter = u16::from_le_bytes([state[p], state[p + 1]]); p += 2;
        self.disable_irq = state[p] != 0; p += 1;
        self.protect_chr = state[p] != 0; p += 1;
        self.pending_ack = state[p] != 0; p += 1;
        for b in self.chr_ram.iter_mut() {
            if p < state.len() {
                *b = state[p];
                p += 1;
            }
        }
        p
    }
}
