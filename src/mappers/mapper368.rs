use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper368 {
    prg: u8,
    latch: u8,
    counting: bool,
    counter: u16,
    irq_ack_pending: bool,
}

impl Mapper368 {
    pub fn new() -> Self {
        Self { prg: 0, latch: 0, counting: false, counter: 0, irq_ack_pending: false }
    }
}

impl Mapper for Mapper368 {
    fn reset(&mut self) {
        self.prg = 0;
        self.latch = 0;
        self.counting = false;
        self.counter = 0;
        self.irq_ack_pending = false;
    }

    fn fetch_prg(&mut self, _cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x4000 && address < 0x6000 {
            if address & 0x1FF == 0x122 {
                return FetchResult { data: 0x8A | (self.latch & 0x35), driven: true };
            }
            return FetchResult { data: 0, driven: false };
        }
        if address >= 0x6000 {
            let len = _cart.prg_rom.len().max(1);
            let banks = len / 0x2000;
            let bank = match address {
                0x6000..=0x7FFF => 2usize,
                0x8000..=0x9FFF => 1usize,
                0xA000..=0xBFFF => 0usize,
                0xC000..=0xDFFF => (self.prg as usize) % banks,
                0xE000..=0xFFFF => (banks - 1).min(8),
                _ => 0,
            };
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult { data: _cart.prg_rom[offset % len], driven: true };
        }
        FetchResult { data: 0, driven: false }
    }

    fn handle_cpu_write(&mut self, address: u16, val: u8) {
        if address >= 0x4000 && address < 0x6000 {
            match address & 0x1FF {
                0x022 => {
                    self.prg = if val & 1 != 0 { 3 } else { (val >> 1) | 4 };
                }
                0x122 => {
                    self.latch = val;
                    self.counting = val & 1 != 0;
                    if !self.counting {
                        self.counter = 0;
                    }
                }
                _ => {}
            }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, _address: u16, _data: u8) {}

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        crate::mapper::mirror_h_or_v(true, address)
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
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[(address as usize) % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[(address as usize) % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mir = crate::mapper::mirror_h_or_v(true, address);
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram && !cart.chr_ram.is_empty() {
            let len = cart.chr_ram.len();
            cart.chr_ram[(address as usize) % len] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mir = crate::mapper::mirror_h_or_v(true, address);
            vram[(mir & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.counting {
            self.counter = self.counter.wrapping_add(cycles as u16);
            if self.counter & 0x0FFF == 0 {
                self.irq_ack_pending = true;
                return true;
            }
        }
        false
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.irq_ack_pending {
            self.irq_ack_pending = false;
            return true;
        }
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.prg);
        state.push(self.latch);
        state.push(self.counting as u8);
        state.extend_from_slice(&self.counter.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.prg = state[p]; p += 1; }
        if p < state.len() { self.latch = state[p]; p += 1; }
        if p < state.len() { self.counting = state[p] != 0; p += 1; }
        if p + 2 <= state.len() { self.counter = u16::from_le_bytes([state[p], state[p+1]]); p += 2; }
        p
    }
}
