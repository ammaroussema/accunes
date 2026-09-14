use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper479 {
    mmc3: MapperMMC3,
    submapper: u8,
    reg: [u8; 6],
    mirroring: u8,
}

impl Mapper479 {
    pub fn new(submapper: u8, header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let config = Mmc3Config {
            ax5202p: true,
            ..Mmc3Config::for_ines(header, 0, if chr_size == 0 { 0 } else { chr_size }, rom, rom_name)
        };
        Self {
            mmc3: MapperMMC3::new(config),
            submapper,
            reg: [0; 6],
            mirroring: 0,
        }
    }

    fn prg_and(&self) -> u16 {
        ((self.reg[5] as u16) << 1 & 0x3E) | 1
    }

    fn prg_or(&self) -> u16 {
        ((self.reg[1] as u16) << 1 & 0x3E) | ((self.reg[2] as u16) << 6)
    }

    fn chr_and(&self) -> u16 {
        ((self.reg[4] as u16) << 3 & 0xF8) | 7
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
        let prg_and = self.prg_and();
        let prg_or = self.prg_or();
        if (self.reg[3] & 0x0C) == 0x04 || (self.reg[4] & 0x80) != 0 {
            let bank32 = ((self.mmc3.bank_8c as u16) & (prg_and >> 2)) | (prg_or >> 2);
            bank32 * 4 + cpu_bank as u16
        } else if (self.reg[3] & 0x08) != 0 {
            let bank16 = ((self.mmc3.bank_8c as u16) & (prg_and >> 1)) | (prg_or >> 1);
            let fixed16 = (prg_and >> 1) | (prg_or >> 1);
            if cpu_bank < 2 {
                bank16 * 2 + (cpu_bank & 1) as u16
            } else {
                fixed16 * 2 + (cpu_bank & 1) as u16
            }
        } else {
            let raw = self.prg_raw_bank(cpu_bank);
            ((raw as u16) & prg_and) | prg_or
        }
    }

    fn chr_bank(&self, address: u16) -> u16 {
        (self.mmc3.chr_bank(address) as u16) & self.chr_and()
    }

    fn mirror_address(&self, address: u16) -> u16 {
        if (self.reg[3] & 0x0C) == 0x04 {
            if (self.mmc3.bank_8c & 0x10) != 0 {
                (address & 0x33FF) | 0x0400
            } else {
                address & 0x33FF
            }
        } else if (self.reg[4] & 0x40) == 0 || (self.mirroring & 0x02) != 0 {
            if (self.mirroring & 1) != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            }
        } else if (self.mirroring & 1) != 0 {
            (address & 0x33FF) | 0x0400
        } else {
            address & 0x33FF
        }
    }
}

impl Mapper for Mapper479 {
    fn reset(&mut self) {
        self.reg = [0; 6];
        if self.submapper != 0 {
            self.reg[4] = 0x1F
                | if (self.submapper & 1) != 0 { 0x40 } else { 0 }
                | if (self.submapper & 2) != 0 { 0x80 } else { 0 };
            self.reg[5] = 0x1F;
        } else {
            self.reg[5] = 0x03;
        }
        self.mirroring = 0;
        self.mmc3.reset();
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x6000..0x8000).contains(&address) {
            let bank = (self.reg[3] >> 6) as usize;
            let off = bank * 0x2000 + (address as usize & 0x1FFF);
            if off < cart.prg_ram.len() {
                return FetchResult {
                    data: cart.prg_ram[off],
                    driven: true,
                };
            }
            return FetchResult {
                data: 0,
                driven: false,
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
            let cpu_bank = ((address - 0x8000) / 0x2000) as u8;
            let bank = self.prg_bank8(cpu_bank);
            let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x4FF0..0x4FF6).contains(&address) && (self.reg[3] & 0x02) == 0 {
            self.reg[(address as usize) & 7] = data;
            return;
        }
        if (0x6000..0x8000).contains(&address) {
            if (self.mmc3.prg_ram_protect & 0x40) == 0 {
                let bank = (self.reg[3] >> 6) as usize;
                let off = bank * 0x2000 + (address as usize & 0x1FFF);
                if off < cart.prg_ram.len() {
                    cart.prg_ram[off] = data;
                }
            }
            return;
        }
        if address >= 0x8000 {
            if (self.reg[3] & 0x0C) != 0 {
                self.mmc3.bank_8c = data;
            } else {
                if (address & 0xE001) == 0xA000 {
                    self.mirroring = data;
                }
                self.mmc3.store_prg(cart, address, data);
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror_address(address)
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
        } else if address < 0x3F00 {
            let mirrored = self.mirror_address(address);
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() {
                    prg_vram[idx]
                } else {
                    0
                }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() && (self.reg[3] & 1) == 0 {
                let bank = self.chr_bank(address);
                let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_address(address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
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
        state.extend_from_slice(&self.reg);
        state.push(self.mirroring);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        for i in 0..self.reg.len() {
            if p < state.len() {
                self.reg[i] = state[p];
                p += 1;
            }
        }
        if p < state.len() {
            self.mirroring = state[p];
            p += 1;
        }
        p
    }
}
