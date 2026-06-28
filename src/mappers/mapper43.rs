use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const PRG_LUT: [u8; 8] = [4, 3, 5, 3, 6, 3, 7, 3];

pub struct Mapper43 {
    reg: u8,
    swap: bool,
    irq_counter: u16,
    irq_enabled: bool,
    irq_ack: bool,
}

impl Mapper43 {
    pub fn new() -> Self {
        Self {
            reg: 0,
            swap: false,
            irq_counter: 0,
            irq_enabled: false,
            irq_ack: false,
        }
    }

    fn prg_bank_for(&self, slot: u8, num_banks: usize) -> usize {
        let b = match slot {
            5 => 8,
            6 => if self.swap { 0 } else { 2 },
            8 => 1,
            10 => 0,
            12 => self.reg as usize,
            14 => if self.swap { 8 } else { 9 },
            _ => 0,
        };
        b % num_banks.max(1)
    }
}

impl Mapper for Mapper43 {
    fn reset(&mut self) {
        self.reg = 0;
        self.swap = false;
        self.irq_counter = 0;
        self.irq_enabled = false;
        self.irq_ack = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        let num_banks = cart.prg_rom.len() / 0x2000;
        if num_banks == 0 {
            return FetchResult { data: 0, driven: true };
        }
        let (slot, local_offset) = match address {
            0x5000..=0x5FFF => (5u8,  (address as usize & 0x1FFF)),
            0x6000..=0x7FFF => (6u8,  (address as usize & 0x1FFF)),
            0x8000..=0x9FFF => (8u8,  (address as usize & 0x1FFF)),
            0xA000..=0xBFFF => (10u8, (address as usize & 0x1FFF)),
            0xC000..=0xDFFF => (12u8, (address as usize & 0x1FFF)),
            0xE000..=0xFFFF => (14u8, (address as usize & 0x1FFF)),
            _ => return FetchResult { data: 0, driven: false },
        };
        let bank = self.prg_bank_for(slot, num_banks);
        let offset = (bank * 0x2000 + local_offset) % cart.prg_rom.len();
        FetchResult { data: cart.prg_rom[offset], driven: true }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        match address & 0xF1FF {
            0x4022 => {
                self.reg = PRG_LUT[(data & 0x07) as usize];
            }
            0x4120 => {
                self.swap = (data & 0x01) != 0;
            }
            0x4122 | 0x8122 => {
                self.irq_enabled = (data & 0x01) != 0;
                self.irq_ack = true;
                self.irq_counter = 0;
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
            if using_chr_ram {
                let len = chr_ram.len();
                if len > 0 {
                    new_addr_bus |= chr_ram[(address as usize & 0x1FFF) % len] as u16;
                }
            } else {
                let len = chr_rom.len();
                if len > 0 {
                    new_addr_bus |= chr_rom[(address as usize & 0x1FFF) % len] as u16;
                }
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

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.irq_enabled {
            self.irq_counter = self.irq_counter.wrapping_add(1);
            if self.irq_counter >= 4096 {
                self.irq_enabled = false;
                return true;
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
        state.push(self.reg);
        state.push(self.swap as u8);
        state.extend_from_slice(&self.irq_counter.to_le_bytes());
        state.push(self.irq_enabled as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 5 <= state.len() {
            self.reg = state[start]; start += 1;
            self.swap = state[start] != 0; start += 1;
            self.irq_counter = u16::from_le_bytes([state[start], state[start + 1]]); start += 2;
            self.irq_enabled = state[start] != 0; start += 1;
        }
        start
    }
}
