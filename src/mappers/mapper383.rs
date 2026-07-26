// Mapper 383 - 晶太 YY840708C (MMC3-based with PAL logic)
//
// Reference: NintendulatorNRS-DBG MMC3-based/mapper383.cpp

use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{mmc3_chr_bank, MapperMMC3, Mmc3Config};

pub struct Mapper383 {
    mmc3: MapperMMC3,
    a15: u8,
    a16: u8,
    a17a18: u8,
}

impl Mapper383 {
    pub fn new(header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        let chr_size = header.get(5).copied().unwrap_or(0);
        let config = Mmc3Config {
            prg_ram_size: 0x2000,
            chr_ram_size: if chr_size == 0 { 0x2000 } else { 0 },
            mmc6: false,
            ax5202p: true,
            irq_revision_b: true,
            irq_hack: crate::mappers::mmc3::Mmc3IrqHack::None,
            header_horizontal_mirror: (header.get(6).copied().unwrap_or(0) & 1) == 0,
        };
        Self {
            mmc3: MapperMMC3::new(config),
            a15: 0,
            a16: 0,
            a17a18: 0,
        }
    }

    fn fixed_last_index(cart: &Cartridge) -> usize {
        let len = cart.prg_rom.len();
        if len == 0 { 0 } else { (len / 0x2000).saturating_sub(1) }
    }

    fn fixed_second_last_index(cart: &Cartridge) -> usize {
        let len = cart.prg_rom.len();
        if len < 0x4000 { 0 } else { (len / 0x2000).saturating_sub(2) }
    }

    fn raw_mmc3_prg_bank(&self, cart: &Cartridge, slot: usize) -> u8 {
        match slot {
            0 => {
                if (self.mmc3.r8000 & 0x40) == 0 {
                    self.mmc3.bank_8c
                } else {
                    Self::fixed_second_last_index(cart) as u8
                }
            }
            1 => self.mmc3.bank_a,
            2 => {
                if (self.mmc3.r8000 & 0x40) != 0 {
                    self.mmc3.bank_8c
                } else {
                    Self::fixed_second_last_index(cart) as u8
                }
            }
            3 => Self::fixed_last_index(cart) as u8,
            _ => 0,
        }
    }

    fn apply_prg_mask_value(raw: u8, mask: u8, value: u8) -> u8 {
        (raw & mask) | value
    }

    fn get_prg_bank_6000(&self, cart: &Cartridge) -> u8 {
        let raw = self.raw_mmc3_prg_bank(cart, 3);
        Self::apply_prg_mask_value(raw, 0x0B, 0x30)
    }

    fn get_prg_banks(&self, cart: &Cartridge) -> [u8; 4] {
        let mut banks = [0u8; 4];
        let raw = [
            self.raw_mmc3_prg_bank(cart, 0),
            self.raw_mmc3_prg_bank(cart, 1),
            self.raw_mmc3_prg_bank(cart, 2),
            self.raw_mmc3_prg_bank(cart, 3),
        ];

        if self.a17a18 == 0x00 {
            let mask = if self.a16 != 0 { 0x07 } else { 0x03 };
            let value = if self.a16 != 0 {
                self.a16 | self.a17a18
            } else {
                self.a15 | self.a16 | self.a17a18
            };
            for i in 0..4 {
                banks[i] = Self::apply_prg_mask_value(raw[i], mask, value);
            }
        } else if self.a17a18 == 0x30 {
            banks[0] = Self::apply_prg_mask_value(raw[2], 0x0F, 0x30);
            banks[1] = Self::apply_prg_mask_value(raw[3], 0x0F, 0x30);
            banks[2] = Self::apply_prg_mask_value(raw[0], 0x0F, 0x30);
            banks[3] = Self::apply_prg_mask_value(raw[1], 0x0F, 0x30);
        } else {
            for i in 0..4 {
                banks[i] = Self::apply_prg_mask_value(raw[i], 0x0F, self.a17a18);
            }
        }
        banks
    }

    fn prg_read(&self, cart: &Cartridge, address: u16) -> u8 {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }

        if self.a17a18 == 0x30 && (0x6000..0x8000).contains(&address) {
            let bank = self.get_prg_bank_6000(cart) as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            return cart.prg_rom[offset % len];
        }

        if address < 0x8000 {
            return 0;
        }

        let banks = self.get_prg_banks(cart);
        let slot = ((address - 0x8000) / 0x2000) as usize;
        let bank = banks[slot.min(3)] as usize;
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        cart.prg_rom[offset % len]
    }

    fn chr_bank_raw(&self, address: u16) -> u8 {
        mmc3_chr_bank(
            self.mmc3.r8000,
            self.mmc3.chr_2k0,
            self.mmc3.chr_2k8,
            self.mmc3.chr_1k0,
            self.mmc3.chr_1k4,
            self.mmc3.chr_1k8,
            self.mmc3.chr_1kc,
            address,
        )
    }
}

impl Mapper for Mapper383 {
    fn reset(&mut self) {
        self.a15 = 0;
        self.a16 = 0;
        self.a17a18 = 0;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            if self.a17a18 == 0x30 {
                return FetchResult {
                    data: self.prg_read(cart, address),
                    driven: true,
                };
            }
            if !cart.prg_ram.is_empty() {
                let offset = (address as usize & 0x1FFF) % cart.prg_ram.len();
                return FetchResult {
                    data: cart.prg_ram[offset],
                    driven: true,
                };
            }
            return FetchResult {
                data: 0,
                driven: false,
            };
        }

        if address >= 0x8000 {
            if self.a17a18 == 0x00 && address < 0xC000 {
                let slot = ((address - 0x8000) / 0x2000) as usize;
                let raw = self.raw_mmc3_prg_bank(cart, slot);
                self.a16 = raw & 0x08;
            }
            return FetchResult {
                data: self.prg_read(cart, address),
                driven: true,
            };
        }

        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            if (address & 0x100) != 0 {
                self.a15 = if (address & 0x2000) != 0 { 0x04 } else { 0x00 };
                self.a17a18 = (address as u8) & 0x30;
            }
            self.mmc3.store_prg(cart, address, data);
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else if self.mmc3.nametable_mirroring() {
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
        prg_vram: &[u8],
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
            let raw_chr = self.chr_bank_raw(address) as u16;
            let bank = (raw_chr & 0x7F) | ((self.a17a18 as u16) << 3);
            let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
            (byte, new_addr_bus)
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if self.mmc3.nametable_mirroring() {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() { prg_vram[idx] } else { 0 }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
            (new_addr_bus as u8, new_addr_bus)
        }
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let raw_chr = self.chr_bank_raw(address) as u16;
                let bank = (raw_chr & 0x7F) | ((self.a17a18 as u16) << 3);
                let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mir = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mir & 0x0800) != 0 {
                let idx = (mir & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mir & 0x7FF) as usize] = data;
            }
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

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.mmc3.cpu_clock(cycles)
    }

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.a15);
        state.push(self.a16);
        state.push(self.a17a18);
        state
    }

    fn load_mapper_registers(
        &mut self,
        cart: &mut Cartridge,
        state: &[u8],
        start: usize,
    ) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        if p < state.len() {
            self.a15 = state[p];
            p += 1;
        }
        if p < state.len() {
            self.a16 = state[p];
            p += 1;
        }
        if p < state.len() {
            self.a17a18 = state[p];
            p += 1;
        }
        p
    }
}
