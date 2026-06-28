use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::eeprom_93cx6::Eeprom93Cx6;

pub struct Mapper164 {
    reg: [u8; 8],
    prg_ram: [u8; 0x2000],
    pa00: bool,
    pa09: bool,
    pa13: bool,
    eeprom: Option<Eeprom93Cx6>,
}

impl Mapper164 {
    pub fn new(wram_capacity: usize) -> Self {
        let eeprom = if wram_capacity >= 512 { Some(Eeprom93Cx6::new(512, 8)) } else { None };
        Mapper164 {
            reg: [0; 8],
            prg_ram: [0; 0x2000],
            pa00: false,
            pa09: false,
            pa13: false,
            eeprom,
        }
    }

    fn mode(&self) -> u8 {
        ((self.reg[0] >> 5) & 2) | ((self.reg[0] >> 4) & 1)
    }

    fn prg_low(&self) -> u8 {
        (self.reg[0] & 0x0F) | ((self.reg[0] >> 1) & 0x10)
    }

    fn prg_high(&self) -> u8 {
        self.reg[1] << 5
    }

    fn mirror_h(&self) -> bool {
        (self.reg[0] & 0x10) != 0 && (self.reg[3] & 0x80) == 0
    }

    fn mode_1bpp(&self) -> bool {
        (self.reg[0] & 0x80) != 0
    }
}

impl Mapper for Mapper164 {
    fn reset(&mut self) {
        self.reg = [0; 8];
        self.prg_ram = [0; 0x2000];
        self.pa00 = false;
        self.pa09 = false;
        self.pa13 = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x5000 && address < 0x5800 {
            let data = if (address & 0x800) == 0 && (address & 0x400) != 0 {
                if let Some(ref eeprom) = self.eeprom {
                    if eeprom.read() { 0x00 } else { 0x04 }
                } else {
                    0
                }
            } else {
                0
            };
            return FetchResult { data, driven: true };
        }
        if address < 0x6000 {
            return FetchResult { data: 0, driven: true };
        }
        if address >= 0x6000 && address < 0x8000 {
            return FetchResult { data: self.prg_ram[(address & 0x1FFF) as usize], driven: true };
        }
        let num_32k = cart.prg_rom.len() / 0x8000;
        if num_32k == 0 {
            return FetchResult { data: 0, driven: true };
        }
        let prg_low = self.prg_low() as usize;
        let prg_high = self.prg_high() as usize;
        match self.mode() {
            0 => {
                let bank = (prg_high | prg_low) % num_32k;
                let bank2 = (prg_high | 0x1F) % num_32k;
                if address >= 0xC000 {
                    let offset = bank2 * 0x8000 + (address as usize & 0x7FFF);
                    FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
                } else {
                    let offset = bank * 0x8000 + (address as usize & 0x7FFF);
                    FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
                }
            }
            2 => {
                let bank = (prg_high | prg_low) % num_32k;
                let fixed = if prg_low >= 0x1C { 0x1C } else { 0x1E };
                let bank2 = (prg_high | fixed) % num_32k;
                if address >= 0xC000 {
                    let offset = bank2 * 0x8000 + (address as usize & 0x7FFF);
                    FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
                } else {
                    let offset = bank * 0x8000 + (address as usize & 0x7FFF);
                    FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
                }
            }
            3 => {
                if (prg_low & 0x10) != 0 {
                    let bank_a = (prg_high | (prg_low & 0x0F) | ((prg_low << 1) & 0x10)) % num_32k;
                    let bank_c = (prg_high | 0x0F | ((prg_low << 1) & 0x10)) % num_32k;
                    if address >= 0xC000 {
                        let offset = bank_c * 0x8000 + (address as usize & 0x7FFF);
                        FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
                    } else {
                        let offset = bank_a * 0x8000 + (address as usize & 0x7FFF);
                        FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
                    }
                } else {
                    let bank_prg = ((prg_high >> 1) | prg_low) % num_32k;
                    let offset = bank_prg * 0x8000 + (address as usize & 0x7FFF);
                    FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
                }
            }
            _ => FetchResult { data: 0, driven: true },
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            self.prg_ram[(address & 0x1FFF) as usize] = data;
        } else if address >= 0x5000 && address < 0x5800 {
            if (address & 0x800) == 0 {
                let index = ((address >> 8) & 7) as usize;
                if index == 1 {
                }
                self.reg[index] = if (address & 0x400) != 0 { 0 } else { data };
                if let Some(ref mut eeprom) = self.eeprom {
                    eeprom.write(
                        (self.reg[2] & 0x10) != 0,
                        (self.reg[2] & 0x04) != 0,
                        (self.reg[2] & 0x01) != 0,
                    );
                }
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mirror_h() {
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
        let pa13_new = (address & 0x2000) != 0;
        if !self.pa13 && pa13_new {
            self.pa00 = (address & 0x001) != 0;
            self.pa09 = (address & 0x0200) != 0;
        }
        self.pa13 = pa13_new;
        if address < 0x2000 {
            let offset = if self.mode_1bpp() {
                let group = if self.pa09 { 0x1000 } else { 0 };
                let a0_adj = if self.pa00 { 8 } else { 0 };
                let bank_low = (address as usize >> 10) & 3;
                group + bank_low * 0x400 + ((address as usize & 0x3FF) & !8) | a0_adj
            } else {
                address as usize & 0x1FFF
            };
            let byte = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[offset % chr_ram.len()] }
            } else if chr_rom.is_empty() {
                0
            } else {
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if self.mirror_h() {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[address as usize % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.reg);
        state.extend_from_slice(&self.prg_ram);
        state.push(if self.pa00 { 1 } else { 0 });
        state.push(if self.pa09 { 1 } else { 0 });
        state.push(if self.pa13 { 1 } else { 0 });
        if let Some(ref eeprom) = self.eeprom {
            state.push(1);
            state.extend_from_slice(&eeprom.save());
        } else {
            state.push(0);
        }
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for r in &mut self.reg { *r = state[p]; p += 1; }
        for b in self.prg_ram.iter_mut() { *b = state[p]; p += 1; }
        self.pa00 = state[p] != 0; p += 1;
        self.pa09 = state[p] != 0; p += 1;
        self.pa13 = state[p] != 0; p += 1;
        if p < state.len() && state[p] != 0 {
            p += 1;
            if let Some(ref mut eeprom) = self.eeprom {
                p = eeprom.load(state, p);
            }
        }
        p - start
    }
}
