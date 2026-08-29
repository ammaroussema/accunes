use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};
use crate::mappers::mmc3::{mmc3_chr_bank, MapperMMC3, Mmc3Config};

pub struct Mapper613 {
    reg: u8,
    soft_reset_count: u8,
    mmc1_reg: [u8; 4],
    mmc1_shift: u8,
    mmc1_bits: u8,
    mmc1_filter: u8,
    mmc3: MapperMMC3,
}

impl Mapper613 {
    pub fn new() -> Self {
        let mut config = Mmc3Config::embedded();
        config.ax5202p = true;
        Self {
            reg: 0,
            soft_reset_count: 0,
            mmc1_reg: [0x0C, 0, 0, 0],
            mmc1_shift: 0,
            mmc1_bits: 0,
            mmc1_filter: 0,
            mmc3: MapperMMC3::new(config),
        }
    }

    fn mmc1_prg_bank(&self, slot: usize) -> usize {
        let prg = self.mmc1_reg[3] as usize;
        let control = self.mmc1_reg[0] as usize;
        let result = if (control & 0x08) != 0 {
            if (control & 0x04) != 0 {
                prg | (slot * 0x0F)
            } else {
                prg & (slot * 0x0F)
            }
        } else {
            (prg & !1) | slot
        };
        let raw = if (self.mmc1_reg[3] & 0x10) != 0 {
            (result & 0x07) | (prg & 0x08)
        } else {
            result & 0x0F
        };
        (raw & 0x07) | ((self.reg as usize) << 3)
    }

    fn mmc1_chr_page(&self, address: u16) -> usize {
        let slot = (address as usize / 0x1000) & 1;
        let control = self.mmc1_reg[0];
        let raw = if (control & 0x10) != 0 {
            self.mmc1_reg[1 + slot] as usize
        } else {
            (self.mmc1_reg[1] as usize & !1) | slot
        };
        (raw & 0x1F) | ((self.reg as usize) << 5)
    }

    fn mmc1_mirror(&self, address: u16) -> u16 {
        match self.mmc1_reg[0] & 3 {
            0 => address & 0x33FF,
            1 => (address & 0x33FF) | 0x0400,
            2 => mirror_h_or_v(false, address),
            3 => mirror_h_or_v(true, address),
            _ => unreachable!(),
        }
    }

    fn mmc3_prg_bank(&self, slot: usize) -> usize {
        let swap = (self.mmc3.r8000 & 0x40) != 0;
        let raw = match slot {
            0 => if swap { 0x0E } else { self.mmc3.bank_8c as usize },
            1 => self.mmc3.bank_a as usize,
            2 => if swap { self.mmc3.bank_8c as usize } else { 0x0E },
            _ => 0x0F,
        };
        (raw & 0x0F) | ((self.reg as usize) << 4)
    }

    fn mmc3_chr_page(&self, address: u16) -> usize {
        let raw = mmc3_chr_bank(
            self.mmc3.r8000,
            self.mmc3.chr_2k0,
            self.mmc3.chr_2k8,
            self.mmc3.chr_1k0,
            self.mmc3.chr_1k4,
            self.mmc3.chr_1k8,
            self.mmc3.chr_1kc,
            address,
        ) as usize;
        (raw & 0x7F) | ((self.reg as usize) << 7)
    }
}

impl Mapper for Mapper613 {
    fn reset(&mut self) {
        self.soft_reset_count = self.soft_reset_count.wrapping_add(1);
        self.reg = self.soft_reset_count;
        self.mmc1_reg = [0x0C, 0, 0, 0];
        self.mmc1_shift = 0;
        self.mmc1_bits = 0;
        self.mmc1_filter = 0;
        self.mmc3.reset();
    }

    fn reset_power_cycle(&mut self) {
        self.reg = 0;
        self.soft_reset_count = 0;
        self.mmc1_reg = [0x0C, 0, 0, 0];
        self.mmc1_shift = 0;
        self.mmc1_bits = 0;
        self.mmc1_filter = 0;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if cart.prg_rom.is_empty() {
                return FetchResult { data: 0, driven: false };
            }
            if (self.reg & 1) != 0 {
                let slot = ((address - 0x8000) / 0x2000) as usize;
                let bank = self.mmc3_prg_bank(slot);
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                FetchResult {
                    data: cart.prg_rom[offset % cart.prg_rom.len()],
                    driven: true,
                }
            } else {
                let slot = ((address - 0x8000) / 0x4000) as usize;
                let bank = self.mmc1_prg_bank(slot);
                let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                FetchResult {
                    data: cart.prg_rom[offset % cart.prg_rom.len()],
                    driven: true,
                }
            }
        } else if (0x6000..0x8000).contains(&address) {
            if !cart.prg_ram.is_empty() {
                let offset = (address - 0x6000) as usize;
                FetchResult {
                    data: cart.prg_ram[offset % cart.prg_ram.len()],
                    driven: true,
                }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            if (self.reg & 1) != 0 {
                self.mmc3.store_prg(cart, address, data);
            } else if (data & 0x80) != 0 {
                self.mmc1_reg[0] |= 0x0C;
                self.mmc1_shift = 0;
                self.mmc1_bits = 0;
                self.mmc1_filter = 2;
            } else if self.mmc1_filter == 0 {
                self.mmc1_shift |= (data & 1) << self.mmc1_bits;
                self.mmc1_bits += 1;
                if self.mmc1_bits == 5 {
                    let reg_idx = ((address >> 13) & 3) as usize;
                    self.mmc1_reg[reg_idx] = self.mmc1_shift;
                    self.mmc1_shift = 0;
                    self.mmc1_bits = 0;
                }
                self.mmc1_filter = 2;
            }
        } else if (0x6000..0x8000).contains(&address) && !cart.prg_ram.is_empty() {
            let offset = (address - 0x6000) as usize;
            let len = cart.prg_ram.len();
            cart.prg_ram[offset % len] = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if (self.reg & 1) != 0 {
            self.mmc3.mirror_nametable(cart, address)
        } else {
            self.mmc1_mirror(address)
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
            let (bank, mask, shift) = if (self.reg & 1) != 0 {
                (self.mmc3_chr_page(address), 0x03FF, 10)
            } else {
                (self.mmc1_chr_page(address), 0x0FFF, 12)
            };
            let offset = (bank << shift) + (address as usize & mask);
            let byte = if using_chr_ram || chr_rom.is_empty() {
                if !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else {
                    0
                }
            } else {
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if (self.reg & 1) != 0 {
                mirror_h_or_v(self.mmc3.nametable_mirroring(), address)
            } else {
                self.mmc1_mirror(address)
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }

        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let (bank, mask, shift) = if (self.reg & 1) != 0 {
                    (self.mmc3_chr_page(address), 0x03FF, 10)
                } else {
                    (self.mmc1_chr_page(address), 0x0FFF, 12)
                };
                let offset = (bank << shift) + (address as usize & mask);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
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
        if (self.reg & 1) != 0 {
            self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
        } else {
            false
        }
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        if (self.reg & 1) != 0 {
            self.mmc3.cpu_clock_rise(ppu_address_bus)
        } else {
            if self.mmc1_filter > 0 {
                self.mmc1_filter -= 1;
            }
            false
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        if (self.reg & 1) != 0 {
            self.mmc3.take_irq_ack()
        } else {
            false
        }
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = vec![
            self.reg,
            self.soft_reset_count,
            self.mmc1_reg[0],
            self.mmc1_reg[1],
            self.mmc1_reg[2],
            self.mmc1_reg[3],
            self.mmc1_shift,
            self.mmc1_bits,
            self.mmc1_filter,
        ];
        state.extend(self.mmc3.save_mapper_registers(cart));
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start < state.len() { self.reg = state[start]; start += 1; }
        if start < state.len() { self.soft_reset_count = state[start]; start += 1; }
        for r in &mut self.mmc1_reg {
            if start < state.len() { *r = state[start]; start += 1; }
        }
        if start < state.len() { self.mmc1_shift = state[start]; start += 1; }
        if start < state.len() { self.mmc1_bits = state[start]; start += 1; }
        if start < state.len() { self.mmc1_filter = state[start]; start += 1; }
        self.mmc3.load_mapper_registers(cart, state, start)
    }
}
