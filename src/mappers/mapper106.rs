use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper106 {
    prg_banks: [u8; 4],
    chr_banks: [u8; 8],
    mirroring: u8,
    irq_enabled: bool,
    irq_counter: u16,
}

impl Mapper106 {
    pub fn new() -> Self {
        Self {
            prg_banks: [0; 4],
            chr_banks: [0; 8],
            mirroring: 0,
            irq_enabled: false,
            irq_counter: 0,
        }
    }
}

impl Mapper for Mapper106 {
    fn reset(&mut self) {
        self.prg_banks = [0; 4];
        self.chr_banks = [0; 8];
        self.mirroring = 0;
        self.irq_enabled = false;
        self.irq_counter = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x6000 {
            return FetchResult { data: 0, driven: false };
        } else if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let offset = (address as usize - 0x6000) % cart.prg_ram.len();
                return FetchResult {
                    data: cart.prg_ram[offset],
                    driven: true,
                };
            }
            return FetchResult { data: 0, driven: false };
        } else if address >= 0x8000 {
            let num_8k = cart.prg_rom.len() / 0x2000;
            if num_8k == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let bank_idx = match address {
                0x8000..=0x9FFF => 0,
                0xA000..=0xBFFF => 1,
                0xC000..=0xDFFF => 2,
                0xE000..=0xFFFF => 3,
                _ => 0,
            };
            let bank = (self.prg_banks[bank_idx] as usize) % num_8k;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let offset = (address as usize - 0x6000) % cart.prg_ram.len();
                cart.prg_ram[offset] = data;
            }
        } else if address >= 0x8000 {
            let reg = address & 0xF;
            match reg {
                0..=7 => {
                    let chr_idx = (reg & 7) as usize;
                    let masked_data = if (reg & 4) != 0 {
                        data & 0xFF  
                    } else {
                        (data & 0xFE) | ((reg & 1) as u8)  
                    };
                    self.chr_banks[chr_idx] = masked_data;
                }
                8..=11 => {
                    let prg_idx = (reg & 3) as usize;
                    self.prg_banks[prg_idx] = data;
                }
                12 => {
                    self.mirroring = data & 1;
                }
                13 => {
                    self.irq_enabled = false;
                    self.irq_counter = 0;
                }
                14 => {
                    self.irq_counter = (self.irq_counter & 0xFF00) | (data as u16);
                }
                15 => {
                    self.irq_counter = (self.irq_counter & 0x00FF) | ((data as u16) << 8);
                    self.irq_enabled = true;
                }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mirroring & 1 != 0 {
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
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = (address >> 10) as usize & 0x07;
            let chr_bank = self.chr_banks[bank] as usize;
            let offset = (chr_bank * 0x400) + (address as usize & 0x3FF);
            let byte = if using_chr_ram {
                if chr_ram.is_empty() {
                    0
                } else {
                    chr_ram[offset % chr_ram.len()]
                }
            } else if chr_rom.is_empty() {
                0
            } else {
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if self.mirroring & 1 != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let bank = (address >> 10) as usize & 0x07;
                let chr_bank = self.chr_banks[bank] as usize;
                let offset = (chr_bank * 0x400) + (address as usize & 0x3FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.irq_counter != 0xFFFF {
            self.irq_counter = self.irq_counter.wrapping_add(1);
        }
        self.irq_counter == 0xFFFF && self.irq_enabled
    }

    fn take_irq_ack(&mut self) -> bool {
        let should_fire = self.irq_counter == 0xFFFF && self.irq_enabled;
        should_fire
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg_banks);
        state.extend_from_slice(&self.chr_banks);
        state.push(self.mirroring);
        state.push(if self.irq_enabled { 1 } else { 0 });
        state.extend_from_slice(&self.irq_counter.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 4 <= state.len() {
            self.prg_banks.copy_from_slice(&state[p..p + 4]);
            p += 4;
        }
        if p + 8 <= state.len() {
            self.chr_banks.copy_from_slice(&state[p..p + 8]);
            p += 8;
        }
        if p < state.len() {
            self.mirroring = state[p];
            p += 1;
        }
        if p < state.len() {
            self.irq_enabled = state[p] != 0;
            p += 1;
        }
        if p + 2 <= state.len() {
            self.irq_counter = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        p
    }
}
