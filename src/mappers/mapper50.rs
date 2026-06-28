use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper50 {
    irq_counter: u16,
    irq_enabled: bool,
    bank_c: u8,
    irq_pending: bool,
    irq_clear_requested: bool,
}

impl Mapper50 {
    pub fn new() -> Self {
        Self {
            irq_counter: 0,
            irq_enabled: false,
            bank_c: 0x08, 
            irq_pending: false,
            irq_clear_requested: false,
        }
    }
}

impl Mapper for Mapper50 {
    fn reset(&mut self) {
        self.irq_counter = 0;
        self.irq_enabled = false;
        self.bank_c = 0x08;
        self.irq_pending = false;
        self.irq_clear_requested = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        let num_banks = cart.prg_rom.len() / 0x2000;
        if num_banks == 0 {
            return FetchResult { data: 0, driven: true };
        }
        let bank = match address {
            0x6000..=0x7FFF => 0x0F % num_banks,
            0x8000..=0x9FFF => 0x08 % num_banks,
            0xA000..=0xBFFF => 0x09 % num_banks,
            0xC000..=0xDFFF => self.bank_c as usize % num_banks,
            0xE000..=0xFFFF => 0x0B % num_banks,
            _ => { return FetchResult { data: 0, driven: false }; }
        };
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        FetchResult {
            data: cart.prg_rom[offset % cart.prg_rom.len()],
            driven: true,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x4020 && address <= 0x5FFF {
            match address & 0x4120 {
                0x4020 => {
                    self.bank_c = (data & 0x08) | ((data & 0x01) << 2) | ((data & 0x06) >> 1);
                }
                0x4120 => {
                    if (data & 0x01) != 0 {
                        self.irq_enabled = true;
                    } else {
                        self.irq_enabled = false;
                        self.irq_counter = 0;
                        self.irq_pending = false;
                        self.irq_clear_requested = true;
                    }
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
        prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
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
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if nametable_horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() {
                    prg_vram[idx]
                } else {
                    0
                }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
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

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        let mut irq = false;
        for _ in 0..cycles {
            if self.irq_enabled {
                self.irq_counter = self.irq_counter.wrapping_add(1);
                if self.irq_counter == 0x1000 {
                    self.irq_pending = true;
                    self.irq_enabled = false;
                    irq = true;
                }
            }
        }
        irq
    }

    fn cpu_clock_rise(&mut self, _ppu_address_bus: u16) -> bool {
        self.irq_pending
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.irq_clear_requested {
            self.irq_clear_requested = false;
            true
        } else {
            false
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.irq_counter.to_le_bytes());
        state.push(if self.irq_enabled { 1 } else { 0 });
        state.push(self.bank_c);
        state.push(if self.irq_pending { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 5 <= state.len() {
            self.irq_counter = u16::from_le_bytes([state[start], state[start + 1]]);
            start += 2;
            self.irq_enabled = state[start] != 0;
            start += 1;
            self.bank_c = state[start];
            start += 1;
            self.irq_pending = state[start] != 0;
            start += 1;
        }
        start
    }
}
