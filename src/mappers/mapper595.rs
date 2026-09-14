use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

const MANUFACTURER_ID: u8 = 0xBF;
const MODEL_ID: u8 = 0xB7;
const SECTOR_SIZE: usize = 4096;
const MAGIC_ADDR1: u16 = 0x5555;
const MAGIC_ADDR2: u16 = 0x2AAA;

pub struct Mapper595 {
    reg: u8,
    flash_state: u8,
    time_out: u32,
}

impl Mapper595 {
    pub fn new() -> Self {
        Self {
            reg: 0,
            flash_state: 0,
            time_out: 0,
        }
    }

    fn prg_bank_low(&self) -> usize {
        (self.reg & 0x1F) as usize
    }

    fn flash_addr(&self, bank: u8, addr: u16) -> usize {
        let bank_bits = (bank as usize & 3) << 13;
        let reg_bit = ((self.reg as usize) << 14) & 0x4000;
        (bank_bits & 0x3000) | reg_bit | (addr as usize & 0x0FFF)
    }

    fn flash_write(&mut self, cart: &mut Cartridge, bank: u8, addr: u16, val: u8) {
        let chip_addr = self.flash_addr(bank, addr);
        let addr_low = addr & 0x0FFF;
        match self.flash_state {
            0x01 => {
                if addr_low == MAGIC_ADDR2 && val == 0x55 {
                    self.flash_state = 0x02;
                }
            }
            0x02 => {
                if addr_low == MAGIC_ADDR1 {
                    self.flash_state = val;
                }
            }
            0x80 => {
                if addr_low == MAGIC_ADDR1 && val == 0xAA {
                    self.flash_state = 0x81;
                }
            }
            0x81 => {
                if addr_low == MAGIC_ADDR2 && val == 0x55 {
                    self.flash_state = 0x82;
                }
            }
            0x82 => {
                if val == 0x30 {
                    let len = cart.prg_rom.len();
                    if chip_addr < len {
                        let start = chip_addr & !(SECTOR_SIZE - 1);
                        let end = (start + SECTOR_SIZE).min(len);
                        for b in &mut cart.prg_rom[start..end] {
                            *b = 0xFF;
                        }
                        self.time_out = SECTOR_SIZE as u32;
                    }
                    self.flash_state = 0;
                } else if val == 0x10 && addr_low == MAGIC_ADDR1 {
                    for b in cart.prg_rom.iter_mut() {
                        *b = 0xFF;
                    }
                    self.time_out = cart.prg_rom.len() as u32;
                    self.flash_state = 0;
                } else if val == 0xF0 {
                    self.flash_state = 0;
                }
            }
            0x90 => {
                if val == 0xF0 {
                    self.flash_state = 0;
                }
            }
            0xA0 => {
                let len = cart.prg_rom.len();
                if chip_addr < len {
                    cart.prg_rom[chip_addr] = val;
                }
                self.flash_state = 0;
            }
            _ => {
                if addr_low == MAGIC_ADDR1 && val == 0xAA {
                    self.flash_state = 0x01;
                }
            }
        }
    }

    fn fetch_flash(&self, cart: &Cartridge, address: u16) -> u8 {
        let bank = self.prg_bank_low();
        let offset = (bank * 0x4000 + (address as usize & 0x3FFF)) % cart.prg_rom.len().max(1);
        if self.flash_state == 0x90 {
            return if address & 1 != 0 { MODEL_ID } else { MANUFACTURER_ID };
        }
        let raw = if cart.prg_rom.is_empty() { 0 } else { cart.prg_rom[offset] };
        if self.time_out > 0 {
            (raw ^ (if self.time_out & 1 != 0 { 0x40 } else { 0 })) & !0x88
        } else {
            raw
        }
    }
}

impl Mapper for Mapper595 {
    fn reset(&mut self) {
        self.reg = 0;
        self.flash_state = 0;
        self.time_out = 0;
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 && address < 0xC000 {
            if cart.prg_rom.is_empty() {
                return FetchResult { data: 0, driven: true };
            }
            FetchResult { data: self.fetch_flash(cart, address), driven: true }
        } else if address >= 0xC000 {
            if cart.prg_rom.is_empty() {
                return FetchResult { data: 0, driven: true };
            }
            let last_bank = (cart.prg_rom.len() / 0x4000).saturating_sub(1);
            let offset = (last_bank * 0x4000 + (address as usize & 0x3FFF)) % cart.prg_rom.len();
            FetchResult { data: cart.prg_rom[offset], driven: true }
        } else if address >= 0x6000 {
            if cart.prg_ram.is_empty() {
                return FetchResult { data: 0, driven: false };
            }
            let offset = (address as usize - 0x6000) % cart.prg_ram.len();
            FetchResult { data: cart.prg_ram[offset], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 && address < 0xC000 {
            let bank = self.prg_bank_low() as u8;
            self.flash_write(cart, bank, address, data);
        } else if address >= 0xC000 {
            self.reg = (self.reg >> 1) | (data << 4 & 0x10);
        } else if address >= 0x6000 {
            if !cart.prg_ram.is_empty() {
                let offset = (address as usize - 0x6000) % cart.prg_ram.len();
                cart.prg_ram[offset] = data;
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v(cart.nametable_horizontal_mirroring, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = if address < 0x1000 { 0usize } else { 1usize };
            let offset = bank * 0x1000 + (address as usize & 0x0FFF);
            let byte = if !chr_ram.is_empty() { chr_ram[offset % chr_ram.len()] } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mirror_h_or_v(nametable_horizontal_mirroring, address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let bank = if address < 0x1000 { 0usize } else { 1usize };
                let offset = bank * 0x1000 + (address as usize & 0x0FFF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.time_out > 0 {
            self.time_out = self.time_out.saturating_sub(cycles as u32);
            if self.time_out == 0 {
                self.flash_state = 0;
            }
        }
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = vec![self.reg, self.flash_state];
        state.extend_from_slice(&self.time_out.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start < state.len() {
            self.reg = state[start];
            start += 1;
        }
        if start < state.len() {
            self.flash_state = state[start];
            start += 1;
        }
        if start + 4 <= state.len() {
            self.time_out = u32::from_le_bytes([state[start], state[start+1], state[start+2], state[start+3]]);
            start += 4;
        }
        start
    }
}
