use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::mmc3_chr_bank;
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper260 {
    mmc3: MapperMMC3,
    reg: [u8; 4],
    dip_value: u8,
}

impl Mapper260 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let config = Mmc3Config::for_ines(header, 0, if chr_size == 0 { 0 } else { chr_size }, rom, rom_name);
        Self { mmc3: MapperMMC3::new(config), reg: [0; 4], dip_value: 0 }
    }

    fn mode(&self) -> u8 {
        self.reg[0] & 0x07
    }

    fn is_mmc3_mode(&self) -> bool {
        (self.reg[0] & 0x04) == 0
    }

    fn prg_and(&self) -> u8 {
        match self.mode() {
            0 | 1 => 0x1F,
            _ => 0x0F,
        }
    }

    fn prg_or(&self) -> u8 {
        (self.reg[1] << 1) & !self.prg_and()
    }

    fn chr_and(&self) -> u8 {
        match self.mode() {
            0 | 2 => 0xFF,
            _ => 0x7F,
        }
    }

    fn chr_or(&self) -> u16 {
        u16::from(self.reg[2]) << 3
    }

    fn mmc3_raw_chr_bank(&self, ppu_addr: u16) -> u8 {
        mmc3_chr_bank(
            self.mmc3.r8000,
            self.mmc3.chr_2k0,
            self.mmc3.chr_2k8,
            self.mmc3.chr_1k0,
            self.mmc3.chr_1k4,
            self.mmc3.chr_1k8,
            self.mmc3.chr_1kc,
            ppu_addr,
        )
    }

    fn non_mmc3_chr_bank(&self) -> u8 {
        match self.mode() {
            6 => (self.reg[2] & 0xFE) | (self.reg[3] & 0x01),
            7 => (self.reg[2] & 0xFC) | (self.reg[3] & 0x03),
            _ => self.reg[2],
        }
    }
}

impl Mapper for Mapper260 {
    fn reset(&mut self) {
        self.reg = [0; 4];
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x5000 && address < 0x6000 {
            return FetchResult { data: self.dip_value & 0x03, driven: true };
        }
        if address >= 0x8000 {
            let prg_len = cart.prg_rom.len();
            if prg_len == 0 {
                return FetchResult { data: 0, driven: false };
            }
            if self.is_mmc3_mode() {
                let prg_and = self.prg_and();
                let prg_or = self.prg_or();
                let raw_bank = if address >= 0xE000 {
                    (prg_len / 0x2000).saturating_sub(1) as u8
                } else if address >= 0xC000 {
                    if (self.mmc3.r8000 & 0x40) != 0 {
                        self.mmc3.bank_8c
                    } else {
                        (prg_len / 0x2000).saturating_sub(2) as u8
                    }
                } else if address >= 0xA000 {
                    self.mmc3.bank_a
                } else {
                    if (self.mmc3.r8000 & 0x40) == 0 {
                        self.mmc3.bank_8c
                    } else {
                        (prg_len / 0x2000).saturating_sub(2) as u8
                    }
                };
                let bank = (raw_bank & prg_and) | (prg_or & !prg_and);
                let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
                FetchResult { data: cart.prg_rom[offset % prg_len], driven: true }
            } else {
                let data = match self.mode() {
                    4 => {
                        let num_16k = (prg_len / 0x4000).max(1);
                        let bank = (self.reg[1] as usize) % num_16k;
                        let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                        cart.prg_rom[offset % prg_len]
                    }
                    _ => {
                        let num_32k = (prg_len / 0x8000).max(1);
                        let bank = ((self.reg[1] >> 1) as usize) % num_32k;
                        let offset = bank * 0x8000 + (address as usize & 0x7FFF);
                        cart.prg_rom[offset % prg_len]
                    }
                };
                FetchResult { data, driven: true }
            }
        } else {
            self.mmc3.fetch_prg(cart, address)
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x6000 {
            if (self.reg[0] & 0x80) == 0 {
                self.reg[(address as usize) & 3] = data;
            }
            return;
        }
        if address >= 0x8000 {
            if self.is_mmc3_mode() {
                self.mmc3.store_prg(cart, address, data);
            } else {
                self.reg[3] = data;
            }
            return;
        }
        self.mmc3.store_prg(cart, address, data);
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if self.is_mmc3_mode() {
            self.mmc3.mirror_nametable(cart, address)
        } else if (self.reg[3] & 0x04) != 0 {
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
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = if self.is_mmc3_mode() {
                let raw_bank = self.mmc3_raw_chr_bank(address);
                ((raw_bank & self.chr_and()) as u16 | (self.chr_or() & !u16::from(self.chr_and()))) as u8
            } else {
                self.non_mmc3_chr_bank()
            };
            let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if self.is_mmc3_mode() {
                if self.mmc3.nametable_mirroring() {
                    (address & 0x33FF) | ((address & 0x0800) >> 1)
                } else {
                    address & 0x37FF
                }
            } else if (self.reg[3] & 0x04) != 0 {
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
                let bank = if self.is_mmc3_mode() {
                    let raw_bank = self.mmc3_raw_chr_bank(address);
                    ((raw_bank & self.chr_and()) as u16 | (self.chr_or() & !u16::from(self.chr_and()))) as u8
                } else {
                    self.non_mmc3_chr_bank()
                };
                let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
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
            ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on,
        )
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_value
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_value = value;
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.extend_from_slice(&self.reg);
        state.push(self.dip_value);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        for i in 0..4 {
            if p < state.len() {
                self.reg[i] = state[p];
                p += 1;
            }
        }
        if p < state.len() {
            self.dip_value = state[p];
            p += 1;
        }
        p
    }
}
