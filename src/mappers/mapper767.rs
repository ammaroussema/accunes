use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper767 {
    reg5000: u8,
    reg5080: u8,
    reg5180: u8,
    reg5300: u8,
    reg5388: u8,
    reg5400: u8,
    chr_bank: u8,
    irq_pending: bool,
}

impl Mapper767 {
    pub fn new() -> Self {
        Self {
            reg5000: 0,
            reg5080: 0,
            reg5180: 0,
            reg5300: 0,
            reg5388: 0,
            reg5400: 0,
            chr_bank: 0,
            irq_pending: false,
        }
    }

    fn split_mode(&self) -> bool {
        self.reg5300 & 0x80 != 0
    }
}

impl Mapper for Mapper767 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x5000 && address < 0x6000 {
            match address & 0x7FF {
                0x000 => FetchResult { data: self.reg5000, driven: true },
                0x080 => FetchResult { data: self.reg5080, driven: true },
                0x0C0 => {
                    self.irq_pending = false;
                    FetchResult { data: 0, driven: true }
                }
                0x180 => FetchResult { data: self.reg5180, driven: true },
                0x300 => FetchResult { data: self.reg5300, driven: true },
                0x388 => FetchResult { data: self.reg5388, driven: true },
                0x400 => FetchResult { data: self.reg5400, driven: true },
                _ => FetchResult { data: 0xFF, driven: true },
            }
        } else if address >= 0x6000 && address < 0x8000 {
            let offset = (address as usize - 0x6000) % cart.prg_ram.len();
            FetchResult { data: cart.prg_ram[offset], driven: true }
        } else if address >= 0x8000 {
            if self.reg5000 == 0x50 {
                let bank = self.reg5400 as usize;
                let offset = (bank * 0x8000 + (address as usize - 0x8000)) % cart.prg_ram.len();
                FetchResult { data: cart.prg_ram[offset], driven: true }
            } else {
                let offset = (address as usize - 0x8000) % cart.prg_rom.len();
                FetchResult { data: cart.prg_rom[offset], driven: true }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x6000 {
            match address & 0x7FF {
                0x000 => self.reg5000 = data,
                0x080 => self.reg5080 = data,
                0x180 => self.reg5180 = data,
                0x300 => self.reg5300 = data,
                0x388 => self.reg5388 = data,
                0x400 => self.reg5400 = data,
                _ => {}
            }
        } else if address >= 0x6000 && address < 0x8000 {
            let offset = (address as usize - 0x6000) % _cart.prg_ram.len();
            _cart.prg_ram[offset] = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.split_mode() {
            address
        } else {
            let a10 = (address >> 11) & 1;
            0x2000 | (address & 0x03FF) | (a10 << 10)
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            if self.split_mode() {
                let bank = self.reg5388 as usize;
                let chr_len = chr_ram.len();
                if chr_len > 0 {
                    let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % chr_len;
                    new_addr_bus |= chr_ram[offset] as u16;
                }
            } else if !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[address as usize & 0x1FFF] as u16;
            } else {
                new_addr_bus |= _prg_rom.get(address as usize % _prg_rom.len()).copied().unwrap_or(0) as u16;
            }
        } else {
            if self.split_mode() {
                if (address & 0xFF) == 0 {
                    self.chr_bank = ((address >> 8) & 0x03) as u8;
                }
                let chr_len = chr_ram.len();
                if chr_len > 0 {
                    let bank_offset = self.chr_bank as usize * 0x800;
                    let offset = (bank_offset + (address as usize & 0x7FF)) % chr_len;
                    new_addr_bus |= chr_ram[offset] as u16;
                } else {
                    let vram_addr = (0x2000 | (address & 0x03FF)) as usize;
                    new_addr_bus |= vram[vram_addr & 0x7FF] as u16;
                }
            } else {
                let vram_addr = (0x2000 | (address & 0x03FF)) as usize;
                new_addr_bus |= vram[vram_addr & 0x7FF] as u16;
            }
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.irq_pending {
            self.irq_pending = false;
            true
        } else {
            false
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![
            self.reg5000, self.reg5080, self.reg5180,
            self.reg5300, self.reg5388, self.reg5400,
            self.chr_bank, self.irq_pending as u8,
        ]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if state.len() >= start + 8 {
            self.reg5000 = state[start];
            self.reg5080 = state[start + 1];
            self.reg5180 = state[start + 2];
            self.reg5300 = state[start + 3];
            self.reg5388 = state[start + 4];
            self.reg5400 = state[start + 5];
            self.chr_bank = state[start + 6];
            self.irq_pending = state[start + 7] != 0;
            start + 8
        } else {
            start
        }
    }
}
