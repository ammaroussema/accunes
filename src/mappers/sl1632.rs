use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};
use crate::mappers::uxrom::mirror_address;

pub struct MapperSL1632 {
    mmc3: MapperMMC3,
    chrcmd: [u8; 8],
    prg0: u8,
    prg1: u8,
    bbrk: u8,
    mirr: u8,
    #[allow(dead_code)]
    swap: u8,
}

impl MapperSL1632 {
    pub fn new() -> Self {
        let mut config = Mmc3Config::embedded();
        config.irq_revision_b = true;
        Self {
            mmc3: MapperMMC3::new(config),
            chrcmd: [0; 8],
            prg0: 0,
            prg1: 0,
            bbrk: 0,
            mirr: 0,
            swap: 0,
        }
    }

    fn mmc3_mode(&self) -> bool {
        (self.bbrk & 2) != 0
    }

    fn custom_mirror(&self, cart: &Cartridge, address: u16) -> u16 {
        mirror_address(
            cart.alternative_nametable_arrangement,
            (self.mirr & 1) != 0,
            address,
        )
    }

    fn mmc3_mode_chr_bank(&self, address: u16) -> usize {
        let page0 = ((self.bbrk & 0x08) as usize) << 5;
        let page1 = ((self.bbrk & 0x20) as usize) << 3;
        let page2 = ((self.bbrk & 0x80) as usize) << 1;
        let cbase = if (self.mmc3.r8000 & 0x80) != 0 {
            0x2000u16
        } else {
            0
        };
        let a = cbase ^ (address & 0x1FFF);
        let m = &self.mmc3;
        match a {
            0x0000..=0x03FF => page0 | (m.chr_2k0 & 0xFE) as usize,
            0x0400..=0x07FF => page0 | (m.chr_2k0 | 1) as usize,
            0x0800..=0x0BFF => page0 | (m.chr_2k8 & 0xFE) as usize,
            0x0C00..=0x0FFF => page0 | (m.chr_2k8 | 1) as usize,
            0x1000..=0x13FF => page1 | (m.chr_1k0 as usize),
            0x1400..=0x17FF => page1 | (m.chr_1k4 as usize),
            0x1800..=0x1BFF => page2 | (m.chr_1k8 as usize),
            _ => page2 | (m.chr_1kc as usize),
        }
    }

    fn custom_chr_bank(&self, address: u16) -> usize {
        let bank_idx = (address >> 10) as usize & 7;
        self.chrcmd[bank_idx] as usize
    }

    fn read_chr(
        &self,
        address: u16,
        chr_rom: &[u8],
        chr_ram: &[u8],
        bank: usize,
    ) -> u8 {
        let len = if !chr_ram.is_empty() {
            chr_ram.len()
        } else {
            chr_rom.len()
        };
        if len == 0 {
            return 0;
        }
        let offset = bank * 0x400 + (address as usize & 0x3FF);
        if !chr_ram.is_empty() {
            chr_ram[offset % len]
        } else {
            chr_rom[offset % len]
        }
    }

    fn custom_chr_write_offset(&self, address: u16, len: usize) -> usize {
        let bank = self.custom_chr_bank(address);
        (bank * 0x400 + (address as usize & 0x3FF)) % len
    }
}

impl Mapper for MapperSL1632 {
    fn reset(&mut self) {
        self.chrcmd = [0; 8];
        self.prg0 = 0;
        self.prg1 = 0;
        self.bbrk = 0;
        self.mirr = 0;
        self.swap = 0;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if self.mmc3_mode() {
                self.mmc3.fetch_prg(cart, address)
            } else {
                let len = cart.prg_rom.len();
                if len == 0 {
                    return FetchResult { data: 0, driven: true };
                }
                let banks_8k = len / 0x2000;
                let bank = if address >= 0xE000 {
                    banks_8k.saturating_sub(1)
                } else if address >= 0xC000 {
                    banks_8k.saturating_sub(2)
                } else if address >= 0xA000 {
                    self.prg1 as usize % banks_8k.max(1)
                } else {
                    self.prg0 as usize % banks_8k.max(1)
                };
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                FetchResult {
                    data: cart.prg_rom[offset % len],
                    driven: true,
                }
            }
        } else {
            self.mmc3.fetch_prg(cart, address)
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x4100 {
            if address == 0xA131 {
                self.bbrk = data;
            }
            if self.mmc3_mode() {
                self.mmc3.store_prg(cart, address, data);
            } else if address >= 0xB000 && address <= 0xE003 {
                let ind = (((((address & 2) as usize) | ((address >> 10) as usize)) >> 1) + 2) & 7;
                let sar = ((address & 1) << 2) as usize;
                self.chrcmd[ind] =
                    (self.chrcmd[ind] & (0xF0 >> sar)) | ((data & 0x0F) << sar);
            } else {
                match address & 0xF003 {
                    0x8000 => self.prg0 = data,
                    0xA000 => self.prg1 = data,
                    0x9000 => self.mirr = data & 1,
                    _ => {}
                }
            }
        } else {
            self.mmc3.store_prg(cart, address, data);
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if self.mmc3_mode() {
            self.mmc3.mirror_nametable(cart, address)
        } else {
            self.custom_mirror(cart, address)
        }
    }

    fn fetch_ppu(
        &mut self,
        prg_rom: &[u8],
        chr_rom: &[u8],
        prg_ram: &[u8],
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
            let bank = if self.mmc3_mode() {
                self.mmc3_mode_chr_bank(address)
            } else {
                self.custom_chr_bank(address)
            };
            let byte = self.read_chr(address, chr_rom, chr_ram, bank);
            new_addr_bus |= byte as u16;
        } else if self.mmc3_mode() {
            return self.mmc3.fetch_ppu(
                prg_rom,
                chr_rom,
                prg_ram,
                chr_ram,
                prg_vram,
                using_chr_ram,
                nametable_horizontal_mirroring,
                alternative_nametable_arrangement,
                ppu_address_bus,
                ppu_octal_latch,
                vram,
            );
        } else {
            let mirrored = mirror_address(
                alternative_nametable_arrangement,
                (self.mirr & 1) != 0,
                address,
            );
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                let offset = if self.mmc3_mode() {
                    let bank = self.mmc3_mode_chr_bank(address);
                    (bank * 0x400 + (address as usize & 0x3FF)) % len
                } else {
                    self.custom_chr_write_offset(address, len)
                };
                cart.chr_ram[offset] = data;
            } else if !cart.chr_rom.is_empty() && !self.mmc3_mode() {
                let len = cart.chr_rom.len();
                let offset = self.custom_chr_write_offset(address, len);
                cart.chr_rom[offset] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        ppu_sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        self.mmc3.ppu_clock(
            ppu_address_bus,
            ppu_a12_prev,
            scanline,
            dot,
            ppu_sprite_x16,
            rendering_on,
        )
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.extend_from_slice(&self.chrcmd);
        state.push(self.prg0);
        state.push(self.prg1);
        state.push(self.bbrk);
        state.push(self.mirr);
        state.push(self.swap);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut idx = self.mmc3.load_mapper_registers(cart, state, start);
        if idx + 8 <= state.len() {
            self.chrcmd.copy_from_slice(&state[idx..idx + 8]);
            idx += 8;
        }
        if idx < state.len() {
            self.prg0 = state[idx];
            idx += 1;
        }
        if idx < state.len() {
            self.prg1 = state[idx];
            idx += 1;
        }
        if idx < state.len() {
            self.bbrk = state[idx];
            idx += 1;
        }
        if idx < state.len() {
            self.mirr = state[idx];
            idx += 1;
        }
        if idx < state.len() {
            self.swap = state[idx];
            idx += 1;
        }
        idx
    }
}
