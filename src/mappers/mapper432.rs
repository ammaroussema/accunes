
use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper432 {
    mmc3: MapperMMC3,
    reg: [u8; 2],
    dip_value: u8,
    submapper: u8,
    irq_clear_pending: bool,
}

impl Mapper432 {
    pub fn new(submapper: u8, header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let mut config = Mmc3Config::for_ines(header, 0, chr_size, rom, rom_name);
        config.ax5202p = true;
        Self {
            mmc3: MapperMMC3::new(config),
            reg: [0; 2],
            dip_value: 0,
            submapper,
            irq_clear_pending: false,
        }
    }

    fn pad_active(&self) -> bool {
        if self.submapper == 1 {
            (self.reg[1] & 0x20) != 0
        } else {
            (self.reg[0] & 0x01) != 0
        }
    }

    fn prg_and(&self) -> usize {
        if (self.reg[1] & 0x02) != 0 {
            0x0F
        } else {
            0x1F
        }
    }

    fn prg_or(&self) -> usize {
        ((self.reg[1] as usize) << 4 & 0x10) | ((self.reg[1] as usize) << 1 & 0x60)
    }

    fn mmc3_default_prg_bank(&self, bank: usize) -> usize {
        let mode = (self.mmc3.r8000 & 0x40) != 0;
        match bank & 3 {
            0 => {
                if mode {
                    0xFE
                } else {
                    self.mmc3.bank_8c as usize
                }
            }
            1 => self.mmc3.bank_a as usize,
            2 => {
                if mode {
                    self.mmc3.bank_8c as usize
                } else {
                    0xFE
                }
            }
            _ => 0xFF,
        }
    }

    fn prg_raw_bank(&self, bank: usize) -> usize {
        if (self.reg[1] & 0x40) != 0 {
            let mask_bit = if self.submapper == 2 { 0x20 } else { 0x80 };
            let mask = if (self.reg[1] & mask_bit) != 0 { 3 } else { 1 };
            (self.mmc3_default_prg_bank(bank & 1) & !mask) | (bank & mask)
        } else {
            self.mmc3_default_prg_bank(bank)
        }
    }

    fn mmc3_get_chr_bank(&self, bank: usize) -> usize {
        let mut b = bank;
        if (self.mmc3.r8000 & 0x80) != 0 {
            b ^= 4;
        }
        if b & 4 != 0 {
            match b {
                4 => self.mmc3.chr_1k0 as usize,
                5 => self.mmc3.chr_1k4 as usize,
                6 => self.mmc3.chr_1k8 as usize,
                _ => self.mmc3.chr_1kc as usize,
            }
        } else {
            let base = if b < 2 {
                self.mmc3.chr_2k0 as usize
            } else {
                self.mmc3.chr_2k8 as usize
            };
            (base & !1) | (b & 1)
        }
    }

    fn chr_2k_mode(&self) -> bool {
        self.submapper == 3 && (self.reg[1] & 0x20) != 0
    }

    fn chr_and(&self) -> usize {
        if (self.reg[1] & 0x04) != 0 {
            0x7F
        } else {
            0xFF
        }
    }

    fn chr_or(&self) -> usize {
        ((self.reg[1] as usize) << 7 & 0x80)
            | ((self.reg[1] as usize) << 5 & 0x100)
            | ((self.reg[1] as usize) << 4 & 0x200)
    }

    fn chr_offset(&self, address: u16) -> usize {
        if self.chr_2k_mode() {
            let raw = match (address >> 11) as usize {
                0 => self.mmc3_get_chr_bank(0),
                1 => self.mmc3_get_chr_bank(3),
                2 => self.mmc3_get_chr_bank(4),
                _ => self.mmc3_get_chr_bank(7),
            };
            let mut chr_and = self.chr_and();
            chr_and |= 0x100;
            chr_and >>= 1;
            let mut chr_or = self.chr_or();
            chr_or >>= 1;
            let bank = (raw & chr_and) | (chr_or & !chr_and);
            bank * 0x800 + (address as usize & 0x7FF)
        } else {
            let raw = self.mmc3.chr_bank(address) as usize;
            let bank = (raw & self.chr_and()) | (self.chr_or() & !self.chr_and());
            bank * 0x400 + (address as usize & 0x3FF)
        }
    }
}

impl Mapper for Mapper432 {
    fn reset(&mut self) {
        self.reg = [0; 2];
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if self.pad_active() && (0x8000..0xF000).contains(&address) {
            return FetchResult {
                data: self.dip_value,
                driven: true,
            };
        }
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            let bank = match address {
                0xE000..=0xFFFF => 3,
                0xC000..=0xDFFF => 2,
                0xA000..=0xBFFF => 1,
                _ => 0,
            };
            let raw = self.prg_raw_bank(bank);
            let bank8 = (raw & self.prg_and()) | (self.prg_or() & !self.prg_and());
            let offset = bank8 * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            }
        } else if address >= 0x6000 {
            self.mmc3.fetch_prg(cart, address)
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) {
            if (self.mmc3.prg_ram_protect & 0x40) == 0 {
                let off = (address - 0x6000) as usize;
                if self.mmc3.config.prg_ram_size > 0 && off < self.mmc3.config.prg_ram_size {
                    cart.prg_ram[off] = data;
                }
                if !(self.submapper == 3 && (self.reg[1] & 0x80) != 0) {
                    self.reg[address as usize & 1] = data;
                }
            }
        } else {
            self.mmc3.store_prg(cart, address, data);
            if (address & 0xE001) == 0xE000 {
                self.irq_clear_pending = true;
            }
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_clear_pending;
        self.irq_clear_pending = false;
        ack
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mmc3.mirror_nametable(cart, address)
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
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                let offset = self.chr_offset(address);
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                let offset = self.chr_offset(address);
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
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
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let offset = self.chr_offset(address);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else {
            self.mmc3.store_ppu(cart, address, data, vram);
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
        self.mmc3
            .ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
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
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let idx = self.mmc3.load_mapper_registers(cart, state, start);
        if idx + 2 <= state.len() {
            self.reg.copy_from_slice(&state[idx..idx + 2]);
        }
        idx + 2
    }
}
