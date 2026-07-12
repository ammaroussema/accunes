use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper308 {
    prg: [u8; 2],
    chr: [u16; 8],
    mirroring: u8,
    prg_flip: bool,
    irq_enabled: bool,
    irq_counter_low: u16,
    irq_counter_high: u8,
    irq_ack: bool,
}

impl Mapper308 {
    pub fn new() -> Self {
        Self {
            prg: [0, 1],
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            mirroring: 0,
            prg_flip: false,
            irq_enabled: false,
            irq_counter_low: 0,
            irq_counter_high: 0,
            irq_ack: false,
        }
    }
}

impl Mapper for Mapper308 {
    fn reset(&mut self) {
        self.prg = [0, 1];
        self.chr = [0, 1, 2, 3, 4, 5, 6, 7];
        self.mirroring = 0;
        self.prg_flip = false;
        self.irq_enabled = false;
        self.irq_counter_low = 0;
        self.irq_counter_high = 0;
        self.irq_ack = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0xE000 {
            let bank = (0xFF & 0x1F) as usize;
            let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % cart.prg_rom.len().max(1);
            FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            }
        } else if address >= 0xC000 {
            let bank = if self.prg_flip {
                (self.prg[0] as usize) & 0x1F
            } else {
                (0xFE & 0x1F) as usize
            };
            let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % cart.prg_rom.len().max(1);
            FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            }
        } else if address >= 0xA000 {
            let bank = (self.prg[1] as usize) & 0x1F;
            let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % cart.prg_rom.len().max(1);
            FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            }
        } else if address >= 0x8000 {
            let bank = if self.prg_flip {
                (0xFE & 0x1F) as usize
            } else {
                (self.prg[0] as usize) & 0x1F
            };
            let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % cart.prg_rom.len().max(1);
            FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            }
        } else if address >= 0x6000 && address < 0x8000 {
            let offset = (address as usize & 0x1FFF) % cart.prg_ram.len().max(1);
            FetchResult {
                data: if offset < cart.prg_ram.len() { cart.prg_ram[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0x4020 {
            FetchResult { data: 0, driven: false }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            let offset = (address as usize & 0x1FFF) % cart.prg_ram.len().max(1);
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
        } else if address >= 0xF000 {
            // VRC24 IRQ write ($F000-$FFFF)
            let a0 = (address & 1) != 0;
            let a1 = (address & 2) != 0;
            let reg = ((a1 as u8) << 1) | (a0 as u8);
            match reg {
                0 => {
                    self.irq_enabled = false;
                    self.irq_counter_low = 0;
                    self.irq_ack = true;
                }
                1 => self.irq_enabled = true,
                3 => self.irq_counter_high = data >> 4,
                _ => {}
            }
        } else if address >= 0xB000 && address < 0xF000 {
            // VRC24 CHR write ($B000-$EFFF)
            let bank = ((address >> 12) - 0xB) as usize;
            let a0 = (address & 1) != 0;
            let a1 = (address & 2) != 0;
            let idx = (bank << 1) | (a1 as usize);
            if idx < 8 {
                if a0 {
                    self.chr[idx] = (self.chr[idx] & 0x000F) | ((data as u16) << 4);
                } else {
                    self.chr[idx] = (self.chr[idx] & 0x0FF0) | (data as u16 & 0x000F);
                }
            }
        } else if address >= 0x9000 && address < 0xA000 {
            // VRC24 misc write ($9000-$9FFF) - mirroring and misc
            let a0 = (address & 1) != 0;
            let a1 = (address & 2) != 0;
            let reg = ((a1 as u8) << 1) | (a0 as u8);
            match reg {
                0 | 1 => self.mirroring = data & 3,
                2 => {
                    self.prg_flip = (data & 2) != 0;
                }
                _ => {}
            }
        } else if address >= 0xA000 && address < 0xB000 {
            // VRC24 PRG write - $A000-$AFFF = prg[1]
            self.prg[1] = data;
        } else if address >= 0x8000 && address < 0x9000 {
            // VRC24 PRG write - $8000-$8FFF = prg[0]
            self.prg[0] = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            return address;
        }
        match self.mirroring & 3 {
            0 => address & 0x37FF,                     // vertical
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1), // horizontal
            2 => 0x2000 | (address & 0x3FF),           // single screen (A)
            3 => 0x2400 | (address & 0x3FF),           // single screen (B)
            _ => address & 0x37FF,
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
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            if using_chr_ram || chr_ram.len() >= 0x2000 {
                new_addr_bus |= chr_ram[(address & 0x1FFF) as usize] as u16;
            } else if !chr_rom.is_empty() {
                let bank = (address as usize) >> 10;
                let offset = (address as usize) & 0x3FF;
                let chr_bank = (self.chr[bank.min(7)] as usize) & 0xFF;
                let rom_offset = chr_bank * 0x400 + offset;
                new_addr_bus |= if rom_offset < chr_rom.len() { chr_rom[rom_offset] } else { 0 } as u16;
            }
        } else {
            let h = if alternative_nametable_arrangement {
                false
            } else {
                nametable_horizontal_mirroring
            };
            let mirrored = if h {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        } else if address < 0x2000 && cart.using_chr_ram {
            let offset = address as usize & 0x1FFF;
            if offset < cart.chr_ram.len() {
                cart.chr_ram[offset] = data;
            }
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.irq_enabled {
            self.irq_counter_low = self.irq_counter_low.wrapping_add(1);
            if (self.irq_counter_low & 4095) == 2048 {
                self.irq_counter_high = self.irq_counter_high.wrapping_sub(1);
            }
            if self.irq_counter_high == 0 && (self.irq_counter_low & 4095) < 2048 {
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
        state.push(self.prg[0]);
        state.push(self.prg[1]);
        for c in &self.chr {
            state.extend_from_slice(&c.to_le_bytes());
        }
        state.push(self.mirroring);
        state.push(if self.prg_flip { 1 } else { 0 });
        state.push(if self.irq_enabled { 1 } else { 0 });
        state.extend_from_slice(&self.irq_counter_low.to_le_bytes());
        state.push(self.irq_counter_high);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.prg[0] = state.get(p).copied().unwrap_or(0); p += 1;
        self.prg[1] = state.get(p).copied().unwrap_or(0); p += 1;
        for c in self.chr.iter_mut() {
            if p + 1 < state.len() {
                *c = u16::from_le_bytes([state[p], state[p + 1]]);
                p += 2;
            }
        }
        self.mirroring = state.get(p).copied().unwrap_or(0); p += 1;
        self.prg_flip = state.get(p).copied().unwrap_or(0) != 0; p += 1;
        self.irq_enabled = state.get(p).copied().unwrap_or(0) != 0; p += 1;
        if p + 1 < state.len() {
            self.irq_counter_low = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        self.irq_counter_high = state.get(p).copied().unwrap_or(0); p += 1;
        p
    }
}
