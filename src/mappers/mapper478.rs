use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper478 {
    mmc3: MapperMMC3,
    submapper: u8,
    reg: u8,
    mode: u8,
}

impl Mapper478 {
    pub fn new(submapper: u8, header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let config = Mmc3Config {
            ax5202p: true,
            ..Mmc3Config::for_ines(header, 0, if chr_size == 0 { 0 } else { chr_size }, rom, rom_name)
        };
        Self {
            mmc3: MapperMMC3::new(config),
            submapper,
            reg: 0,
            mode: 0xF0,
        }
    }

    fn prg_and(&self) -> u8 {
        if self.submapper == 0 && (self.reg & 8) != 0 && (self.reg & 4) != 0 {
            0x03
        } else {
            0x0F
        }
    }

    fn chr_and(&self) -> u16 {
        if self.submapper == 0 && (self.reg & 8) != 0 && (self.reg & 4) != 0 {
            0x1F
        } else {
            0x7F
        }
    }

    fn prg_or(&self) -> u16 {
        let or = if self.submapper == 1 {
            (self.reg as u16) << 3
        } else {
            (self.reg as u16) << 2
        };
        or & !(self.prg_and() as u16)
    }

    fn chr_or(&self) -> u16 {
        let or = if self.submapper == 1 {
            (self.reg as u16) << 6
        } else {
            (self.reg as u16) << 5
        };
        or & !self.chr_and()
    }

    fn prg_raw_bank(&self, cpu_bank: u8) -> u8 {
        match cpu_bank {
            0 => {
                if (self.mmc3.r8000 & 0x40) != 0 {
                    0xFE
                } else {
                    self.mmc3.bank_8c
                }
            }
            1 => self.mmc3.bank_a,
            2 => {
                if (self.mmc3.r8000 & 0x40) != 0 {
                    self.mmc3.bank_8c
                } else {
                    0xFE
                }
            }
            _ => 0xFF,
        }
    }

    fn prg_bank8(&self, cpu_bank: u8) -> u16 {
        let raw = self.prg_raw_bank(cpu_bank);
        ((raw & self.prg_and()) as u16) | self.prg_or()
    }

    fn chr_bank(&self, address: u16) -> u16 {
        let raw = self.mmc3.chr_bank(address) as u16;
        (raw & self.chr_and()) | self.chr_or()
    }
}

impl Mapper for Mapper478 {
    fn reset(&mut self) {
        self.reg = 0;
        self.mode = 0xF0;
        self.mmc3.reset();
    }

    fn reset_power_cycle(&mut self) {
        self.reg = 0;
        self.mode = 0xF0;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            let cpu_bank = ((address - 0x8000) / 0x2000) as u8;
            let bank = self.prg_bank8(cpu_bank) as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            };
        }
        if address >= 0x6000 {
            return self.mmc3.fetch_prg(cart, address);
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) && (self.mmc3.prg_ram_protect & 0x40) == 0 {
            if (self.mode & 0x60) != 0 {
                self.reg = (address & 0xFF) as u8;
                self.mode = data;
            }
        }
        if address >= 0x8000 && self.submapper == 0 {
            if (self.mode & 0x80) != 0 {
                self.mmc3.store_prg(cart, address, data);
            } else {
                let bits = (data << 3) & 0x18;
                self.mmc3.chr_2k0 = (self.mmc3.chr_2k0 & !0x18) | bits;
                self.mmc3.chr_2k8 = (self.mmc3.chr_2k8 & !0x18) | bits;
                self.mmc3.chr_1k0 = (self.mmc3.chr_1k0 & !0x18) | bits;
                self.mmc3.chr_1k4 = (self.mmc3.chr_1k4 & !0x18) | bits;
                self.mmc3.chr_1k8 = (self.mmc3.chr_1k8 & !0x18) | bits;
                self.mmc3.chr_1kc = (self.mmc3.chr_1kc & !0x18) | bits;
            }
        } else {
            self.mmc3.store_prg(cart, address, data);
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mmc3.mirror_nametable(cart, address)
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
        if address >= 0x2000 {
            return self.mmc3.fetch_ppu(
                _prg_rom, chr_rom, _prg_ram, chr_ram, prg_vram,
                using_chr_ram, _nametable_horizontal_mirroring,
                alternative_nametable_arrangement, ppu_address_bus, ppu_octal_latch, vram,
            );
        }
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        let bank = self.chr_bank(address);
        let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
        let byte = if using_chr_ram && !chr_ram.is_empty() {
            chr_ram[offset % chr_ram.len()]
        } else if !chr_rom.is_empty() {
            chr_rom[offset % chr_rom.len()]
        } else {
            0
        };
        new_addr_bus |= byte as u16;
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram && !cart.chr_ram.is_empty() {
            let bank = self.chr_bank(address);
            let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
            let len = cart.chr_ram.len();
            cart.chr_ram[offset % len] = data;
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
        self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.reg);
        state.push(self.mode);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        if p < state.len() {
            self.reg = state[p];
            p += 1;
        }
        if p < state.len() {
            self.mode = state[p];
            p += 1;
        }
        p
    }
}
