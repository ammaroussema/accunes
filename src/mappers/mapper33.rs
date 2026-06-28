use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper33 {
    regs: [u8; 8],
    mirror_vertical: bool,
}

impl Mapper33 {
    pub fn new() -> Self {
        Self {
            regs: [0; 8],
            mirror_vertical: true, 
        }
    }
}

impl Mapper for Mapper33 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let num_banks = cart.prg_rom.len() / 0x2000;
            let last_bank = num_banks - 1;
            let bank = match address {
                0x8000..=0x9FFF => self.regs[0] as usize,
                0xA000..=0xBFFF => self.regs[1] as usize,
                0xC000..=0xDFFF => last_bank.saturating_sub(1),
                0xE000..=0xFFFF => last_bank,
                _ => 0,
            };
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let addr = address & 0xF003;
            match addr {
                0x8000 => {
                    self.regs[0] = data & 0x3F;
                    if !self.mirror_vertical {
                        self.mirror_vertical = ((data >> 6) & 1) == 0;
                    }
                }
                0x8001 => self.regs[1] = data & 0x3F,
                0x8002 => self.regs[2] = data,
                0x8003 => self.regs[3] = data,
                0xA000 => self.regs[4] = data,
                0xA001 => self.regs[5] = data,
                0xA002 => self.regs[6] = data,
                0xA003 => self.regs[7] = data,
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mirror_vertical {
            address & 0x37FF 
        } else {
            (address & 0x33FF) | ((address & 0x0800) >> 1) 
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
            let (bank, bank_size) = match address {
                0x0000..=0x07FF => (self.regs[2] as usize, 0x0800),
                0x0800..=0x0FFF => (self.regs[3] as usize, 0x0800),
                0x1000..=0x13FF => (self.regs[4] as usize, 0x0400),
                0x1400..=0x17FF => (self.regs[5] as usize, 0x0400),
                0x1800..=0x1BFF => (self.regs[6] as usize, 0x0400),
                0x1C00..=0x1FFF => (self.regs[7] as usize, 0x0400),
                _ => (0, 0x0400),
            };
            let offset = bank * bank_size + (address as usize & (bank_size - 1));
            if using_chr_ram {
                if !chr_ram.is_empty() {
                    let mask = chr_ram.len() - 1;
                    new_addr_bus |= chr_ram[offset & mask] as u16;
                }
            } else if !chr_rom.is_empty() {
                let mask = chr_rom.len() - 1;
                new_addr_bus |= chr_rom[offset & mask] as u16;
            }
        } else {
            let mirrored = if self.mirror_vertical {
                address & 0x37FF 
            } else {
                (address & 0x33FF) | ((address & 0x0800) >> 1) 
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.regs);
        state.push(if self.mirror_vertical { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..8 {
            if p < state.len() {
                self.regs[i] = state[p];
                p += 1;
            }
        }
        if p < state.len() {
            self.mirror_vertical = state[p] != 0;
            p += 1;
        }
        p
    }

    fn reset(&mut self) {
        self.regs = [0; 8];
        self.mirror_vertical = true;
    }
}
