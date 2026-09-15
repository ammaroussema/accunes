use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::eeprom_93cx6::Eeprom93Cx6;

pub struct Mapper558 {
    reg: [u8; 8],
    pa09: bool,
    pa13: bool,
    eeprom: Option<Eeprom93Cx6>,
    wram_present: bool,
    rom_1_mib: bool,
}

impl Mapper558 {
    pub fn new(header: &[u8], prg_size_16k: u8) -> Self {
        let ines2_prgram = if header.len() > 10 { header[10] } else { 0 };
        let size_temp = if (ines2_prgram & 0x0F) != 0 {
            64 << (ines2_prgram & 0x0F)
        } else {
            0
        };
        let size_save = if (ines2_prgram & 0xF0) != 0 {
            64 << (ines2_prgram >> 4)
        } else {
            0
        };
        let (eeprom, wram_present) = if size_save == 512 {
            (Some(Eeprom93Cx6::new(512, 8)), size_temp > 0)
        } else {
            (None, size_temp > 0)
        };
        Mapper558 {
            reg: [0; 8],
            pa09: false,
            pa13: false,
            eeprom,
            wram_present,
            rom_1_mib: (prg_size_16k as usize) == 64,
        }
    }

    fn swap_d0_d1(val: u8) -> u8 {
        (val & !3) | ((val << 1) & 2) | ((val >> 1) & 1)
    }

    fn use_a15_a16(&self) -> bool {
        (self.reg[3] & 0x04) != 0
    }

    fn swap_bits(&self) -> bool {
        (self.reg[3] & 0x02) != 0
    }

    fn chr_split(&self) -> bool {
        (self.reg[0] & 0x80) != 0
    }

    fn prg_bank(&self) -> usize {
        let a = (self.reg[1] as u16) << 4;
        let b = (self.reg[0] as u16) & 0x0F;
        let c = if self.use_a15_a16() { 0 } else { 3 };
        let d = if self.rom_1_mib && self.swap_bits() {
            ((self.reg[1] as u16) << 3) & 0x10
        } else {
            0
        };
        (a | b | c | d) as usize
    }

    fn read_asic(&self) -> u8 {
        if let Some(ref eeprom) = self.eeprom {
            if eeprom.read() {
                0x04
            } else {
                0x00
            }
        } else {
            self.reg[2] & 0x04
        }
    }

    fn sync(&mut self) {
        if let Some(ref mut eeprom) = self.eeprom {
            eeprom.write(
                (self.reg[2] & 0x04) != 0,
                (self.reg[2] & 0x02) != 0,
                (self.reg[2] & 0x01) != 0,
            );
        }
    }
}

impl Mapper for Mapper558 {
    fn reset(&mut self) {
        self.reg = [0; 8];
        self.pa09 = false;
        self.pa13 = false;
        self.sync();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x5000 && address < 0x6000 {
            return FetchResult {
                data: self.read_asic(),
                driven: true,
            };
        }
        if address >= 0x6000 && address < 0x8000 {
            if self.wram_present {
                return FetchResult {
                    data: cart.prg_ram[(address & 0x1FFF) as usize],
                    driven: true,
                };
            }
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        if address >= 0x8000 {
            let bank = self.prg_bank();
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            return FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x5800 {
            let mut val = Self::swap_d0_d1(data);
            let index = ((address >> 8) & 7) as usize;
            if self.swap_bits() && index < 3 {
                val = Self::swap_d0_d1(val);
            }
            self.reg[index] = val;
            self.sync();
        } else if address >= 0x6000 && address < 0x8000 {
            if self.wram_present {
                cart.prg_ram[(address & 0x1FFF) as usize] = data;
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
        let pa13_new = (address & 0x2000) != 0;
        if !self.pa13 && pa13_new {
            self.pa09 = (address & 0x0200) != 0;
        }
        self.pa13 = pa13_new;
        if address < 0x2000 {
            let offset = if self.chr_split() && !self.pa13 {
                let bank = ((address as usize >> 10) & 3) | if self.pa09 { 4 } else { 0 };
                bank * 0x400 + (address as usize & 0x3FF)
            } else {
                address as usize & 0x1FFF
            };
            let data = if using_chr_ram {
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
            new_addr_bus |= data as u16;
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

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        let pa13_new = (address & 0x2000) != 0;
        if !self.pa13 && pa13_new {
            self.pa09 = (address & 0x0200) != 0;
        }
        self.pa13 = pa13_new;
        if address < 0x2000 && cart.using_chr_ram {
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[address as usize % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[mirrored as usize & 0x7FF] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.reg);
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
        for r in &mut self.reg {
            *r = state[p];
            p += 1;
        }
        self.pa09 = state[p] != 0;
        p += 1;
        self.pa13 = state[p] != 0;
        p += 1;
        if p < state.len() && state[p] != 0 {
            p += 1;
            if let Some(ref mut eeprom) = self.eeprom {
                p = eeprom.load(state, p);
            }
        }
        p - start
    }

    fn battery_save_data(&self, cart: &Cartridge) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        if let Some(ref eeprom) = self.eeprom {
            out.extend_from_slice(eeprom.storage());
        }
        out.extend_from_slice(&cart.prg_ram);
        Some(out)
    }

    fn load_battery_save(&mut self, cart: &mut Cartridge, data: &[u8]) {
        let mut p = 0;
        if let Some(ref mut eeprom) = self.eeprom {
            let n = data.len().min(eeprom.storage().len());
            eeprom.load_storage(&data[..n]);
            p = n;
        }
        let n = data.len().saturating_sub(p).min(cart.prg_ram.len());
        if n > 0 {
            cart.prg_ram[..n].copy_from_slice(&data[p..p + n]);
        }
    }
}
