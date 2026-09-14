use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper472 {
    mmc3: MapperMMC3,
    reg: u8,
    dip_switches: u8,
}

impl Mapper472 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let config = Mmc3Config {
            ax5202p: true,
            ..Mmc3Config::for_ines(header, 0, if chr_size == 0 { 0 } else { chr_size }, rom, rom_name)
        };
        Self {
            mmc3: MapperMMC3::new(config),
            reg: 0,
            dip_switches: 0,
        }
    }

    fn prg_raw_bank(&self, cart: &Cartridge, cpu_bank: u8) -> u8 {
        let num_banks = (cart.prg_rom.len() / 0x2000) as u8;
        match cpu_bank {
            0 => {
                if (self.mmc3.r8000 & 0x40) != 0 {
                    num_banks.saturating_sub(2)
                } else {
                    self.mmc3.bank_8c
                }
            }
            1 => self.mmc3.bank_a,
            2 => {
                if (self.mmc3.r8000 & 0x40) != 0 {
                    self.mmc3.bank_8c
                } else {
                    num_banks.saturating_sub(2)
                }
            }
            _ => num_banks.saturating_sub(1),
        }
    }

    fn prg_bank8(&self, cart: &Cartridge, cpu_bank: u8) -> u8 {
        (self.prg_raw_bank(cart, cpu_bank) & 0x0F) | (self.reg & 0xF0)
    }

    fn chr_bank(&self, address: u16) -> u16 {
        let raw = self.mmc3.chr_bank(address) as u16;
        if (self.reg & 0x20) != 0 {
            (raw & 0x7F) | (((self.reg as u16) & 0xF0) << 3)
        } else {
            (raw & 0xFF) | (((self.reg as u16) & 0xE0) << 3)
        }
    }
}

impl Mapper for Mapper472 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.reg = 0;
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
            let bank = self.prg_bank8(cart, cpu_bank);
            let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            };
        }
        if address >= 0x6000 {
            if (self.mmc3.prg_ram_protect & 0x80) != 0 {
                return FetchResult {
                    data: self.dip_switches,
                    driven: true,
                };
            }
            return self.mmc3.fetch_prg(cart, address);
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) {
            let wram_enabled = (self.mmc3.prg_ram_protect & 0x80) != 0;
            let wram_protected = (self.mmc3.prg_ram_protect & 0x40) != 0;
            if wram_enabled && !wram_protected {
                self.reg = data;
            }
        }
        self.mmc3.store_prg(cart, address, data);
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

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.reg);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let p = self.mmc3.load_mapper_registers(cart, state, start);
        if p < state.len() {
            self.reg = state[p];
            p + 1
        } else {
            p
        }
    }
}
