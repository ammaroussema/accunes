use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper42 {
    irq_counter: u16,
    irq_enabled: bool,
    prg_reg: u8,
    chr_bank: u8,
    mirr_horizontal: bool,
    irq_pending: bool,
    irq_ack: bool,
}

impl Mapper42 {
    pub fn new() -> Self {
        Self {
            irq_counter: 0,
            irq_enabled: false,
            prg_reg: 0,
            chr_bank: 0,
            mirr_horizontal: false,
            irq_pending: false,
            irq_ack: false,
        }
    }
}

impl Mapper for Mapper42 {
    fn reset(&mut self) {
        self.irq_counter = 0;
        self.irq_enabled = false;
        self.prg_reg = 0;
        self.chr_bank = 0;
        self.mirr_horizontal = false;
        self.irq_pending = false;
        self.irq_ack = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        let num_banks = cart.prg_rom.len() / 0x2000;
        if num_banks == 0 {
            return FetchResult { data: 0, driven: true };
        }
        let bank = match address {
            0x6000..=0x7FFF => (self.prg_reg & 0x0F) as usize % num_banks,
            0x8000..=0x9FFF => ((num_banks as i32 - 4).rem_euclid(num_banks as i32)) as usize,
            0xA000..=0xBFFF => ((num_banks as i32 - 3).rem_euclid(num_banks as i32)) as usize,
            0xC000..=0xDFFF => ((num_banks as i32 - 2).rem_euclid(num_banks as i32)) as usize,
            0xE000..=0xFFFF => ((num_banks as i32 - 1).rem_euclid(num_banks as i32)) as usize,
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
            match address & 0xE003 {
                0x8000 => {
                    self.chr_bank = data & 0x0F;
                }
                0xE000 => {
                    self.prg_reg = data & 0x0F;
                }
                0xE001 => {
                    self.mirr_horizontal = (data & 0x08) != 0;
                }
                0xE002 => {
                    self.irq_enabled = data == 0x02;
                    if !self.irq_enabled {
                        self.irq_pending = false;
                        self.irq_ack = true;
                        self.irq_counter = 0;
                    }
                }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mirr_horizontal {
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
            if using_chr_ram {
                let len = chr_ram.len();
                if len > 0 {
                    new_addr_bus |= chr_ram[(address as usize & 0x1FFF) % len] as u16;
                }
            } else {
                let bank = self.chr_bank as usize;
                let len = chr_rom.len();
                if len > 0 {
                    new_addr_bus |= chr_rom[(bank * 0x2000 + (address as usize & 0x1FFF)) % len] as u16;
                }
            }
        } else {
            let mirrored = if !self.mirr_horizontal {
                address & 0x37FF
            } else {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                let offset = (address as usize & 0x1FFF) % len;
                cart.chr_ram[offset] = data;
            }
        } else if address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.irq_enabled {
            self.irq_counter = self.irq_counter.wrapping_add(1);
            if self.irq_counter >= 0x8000 {
                self.irq_counter -= 0x8000;
            }
            if self.irq_counter >= 0x6000 {
                if !self.irq_pending {
                    self.irq_pending = true;
                    return true;
                }
            } else if self.irq_pending {
                self.irq_pending = false;
                self.irq_ack = true;
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
        state.push(self.irq_enabled as u8);
        state.push(self.prg_reg);
        state.push(self.chr_bank);
        state.push(self.mirr_horizontal as u8);
        state.push(self.irq_pending as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 7 <= state.len() {
            self.irq_counter = u16::from_le_bytes([state[start], state[start + 1]]);
            start += 2;
            self.irq_enabled = state[start] != 0;
            start += 1;
            self.prg_reg = state[start];
            start += 1;
            self.chr_bank = state[start];
            start += 1;
            self.mirr_horizontal = state[start] != 0;
            start += 1;
            self.irq_pending = state[start] != 0;
            start += 1;
        }
        start
    }
}
