use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper40 {
    irq_counter: u16,
    bank_c: u8,
    irq_ack: bool,
    irq_pending: bool,
}

impl Mapper40 {
    pub fn new() -> Self {
        Self {
            irq_counter: 0,
            bank_c: 0,
            irq_ack: false,
            irq_pending: false,
        }
    }
}

impl Mapper for Mapper40 {
    fn reset(&mut self) {
        self.irq_counter = 0;
        self.bank_c = 0;
        self.irq_ack = false;
        self.irq_pending = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        let num_banks = cart.prg_rom.len() / 0x2000;
        if num_banks == 0 {
            return FetchResult { data: 0, driven: true };
        }
        let bank = match address {
            0x6000..=0x7FFF => 6 % num_banks,
            0x8000..=0x9FFF => 4 % num_banks,
            0xA000..=0xBFFF => 5 % num_banks,
            0xC000..=0xDFFF => self.bank_c as usize % num_banks,
            0xE000..=0xFFFF => 7 % num_banks,
            _ => { return FetchResult { data: 0, driven: false }; }
        };
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        FetchResult {
            data: cart.prg_rom[offset % cart.prg_rom.len()],
            driven: true,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            match address & 0xE000 {
                0x8000 => {
                    self.irq_counter = 0;
                    self.irq_pending = false;
                    self.irq_ack = true;
                }
                0xA000 => {
                    self.irq_counter = 4096;
                }
                0xE000 => {
                    self.bank_c = data;
                }
                _ => {}
            }
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
            let mirrored = if !nametable_horizontal_mirroring {
                address & 0x37FF
            } else {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.irq_counter > 0 {
            self.irq_counter -= 1;
            if self.irq_counter == 0 {
                self.irq_pending = true;
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
        state.extend_from_slice(&self.irq_counter.to_le_bytes());
        state.push(self.bank_c);
        state.push(self.irq_pending as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 4 <= state.len() {
            self.irq_counter = u16::from_le_bytes([state[start], state[start + 1]]);
            start += 2;
            self.bank_c = state[start];
            start += 1;
            self.irq_pending = state[start] != 0;
            start += 1;
        }
        start
    }
}
