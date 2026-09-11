use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::ffe::{FfeConfig, MapperFfe};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper12Mmc3 {
    mmc3: MapperMMC3,
    reg: u8,
}

impl Mapper12Mmc3 {
    pub fn new(
        header: &[u8],
        submapper_id: u8,
        chr_size: u8,
        rom: &[u8],
        rom_name: &str,
        has_battery: bool,
    ) -> Self {
        let mut config = Mmc3Config::for_ines(header, submapper_id, chr_size, rom, rom_name);
        config.irq_revision_b = false;
        if has_battery {
            config.prg_ram_size = config.prg_ram_size.max(0x2000);
        }
        let mut mmc3 = MapperMMC3::new(config);
        mmc3.set_nametable_horizontal((header[6] & 1) == 0);
        Self {
            mmc3,
            reg: 0,
        }
    }

    fn chr_offset(&self, address: u16, chr_len: usize) -> usize {
        let bank_1k_idx = (address >> 10) as usize;
        let base = self.mmc3.chr_bank(address) as usize;
        let outer_bit = if bank_1k_idx < 4 {
            self.reg & 1
        } else {
            (self.reg >> 4) & 1
        };
        let bank = base | ((outer_bit as usize) << 8);
        let banks_1k = (chr_len / 0x400).max(1);
        (bank % banks_1k) * 0x400 + (address as usize & 0x3FF)
    }
}

enum Mapper12Inner {
    Mmc3(Mapper12Mmc3),
    Ffe(MapperFfe),
}

pub struct Mapper12 {
    inner: Mapper12Inner,
}

impl Mapper12 {
    pub fn new(
        header: &[u8],
        submapper_id: u8,
        chr_size: u8,
        rom: &[u8],
        rom_name: &str,
        has_battery: bool,
        has_trainer: bool,
        trainer: &[u8],
    ) -> Self {
        if submapper_id == 1 {
            Self {
                inner: Mapper12Inner::Ffe(MapperFfe::new(FfeConfig::mapper12(header, has_battery, has_trainer, trainer))),
            }
        } else {
            Self {
                inner: Mapper12Inner::Mmc3(Mapper12Mmc3::new(
                    header,
                    submapper_id,
                    chr_size,
                    rom,
                    rom_name,
                    has_battery,
                )),
            }
        }
    }
}

impl Mapper for Mapper12 {
    fn reset(&mut self) {
        match &mut self.inner {
            Mapper12Inner::Mmc3(m) => {
                m.reg = 0;
                m.mmc3.reset();
            }
            Mapper12Inner::Ffe(m) => m.reset(),
        }
    }

    fn reset_with_cart(&mut self, cart: &mut Cartridge) {
        match &mut self.inner {
            Mapper12Inner::Mmc3(m) => {
                m.reg = 0;
                m.mmc3.reset();
            }
            Mapper12Inner::Ffe(m) => m.reset_with_cart(cart),
        }
    }

    fn initial_pc(&self) -> Option<u16> {
        match &self.inner {
            Mapper12Inner::Mmc3(_) => None,
            Mapper12Inner::Ffe(m) => m.initial_pc(),
        }
    }

    fn reset_power_cycle(&mut self) {
        match &mut self.inner {
            Mapper12Inner::Mmc3(m) => {
                m.reg = 0;
                m.mmc3.reset_power_cycle();
            }
            Mapper12Inner::Ffe(m) => m.reset_power_cycle(),
        }
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        match &mut self.inner {
            Mapper12Inner::Mmc3(m) => {
                if (0x4100..=0x4FFF).contains(&address) && (address & 0x100) != 0 {
                    return FetchResult {
                        data: 0,
                        driven: true,
                    };
                }
                m.mmc3.fetch_prg(cart, address)
            }
            Mapper12Inner::Ffe(m) => m.fetch_prg(cart, address),
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        match &mut self.inner {
            Mapper12Inner::Mmc3(m) => {
                if (0x4100..=0x4FFF).contains(&address) && (address & 0x100) != 0 {
                    m.reg = data;
                } else if address >= 0x6000 {
                    m.mmc3.store_prg(cart, address, data);
                }
            }
            Mapper12Inner::Ffe(m) => m.store_prg(cart, address, data),
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        match &self.inner {
            Mapper12Inner::Mmc3(m) => m.mmc3.mirror_nametable(cart, address),
            Mapper12Inner::Ffe(m) => m.mirror_nametable(cart, address),
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
        match &mut self.inner {
            Mapper12Inner::Mmc3(m) => {
                let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
                let mut new_addr_bus = ppu_address_bus & 0xFF00;
                if address < 0x2000 {
                    let len = if using_chr_ram {
                        chr_ram.len()
                    } else {
                        chr_rom.len()
                    };
                    if len == 0 {
                        return (0, new_addr_bus);
                    }
                    let offset = m.chr_offset(address, len);
                    let byte = if using_chr_ram {
                        chr_ram[offset % chr_ram.len()]
                    } else {
                        chr_rom[offset % chr_rom.len()]
                    };
                    new_addr_bus |= byte as u16;
                    (new_addr_bus as u8, new_addr_bus)
                } else {
                    m.mmc3.fetch_ppu(
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
                    )
                }
            }
            Mapper12Inner::Ffe(m) => m.fetch_ppu(
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
            ),
        }
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        match &mut self.inner {
            Mapper12Inner::Mmc3(m) => {
                if address < 0x2000 {
                    if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                        let len = cart.chr_ram.len();
                        let offset = m.chr_offset(address, len);
                        cart.chr_ram[offset % len] = data;
                    }
                } else {
                    m.mmc3.store_ppu(cart, address, data, vram);
                }
            }
            Mapper12Inner::Ffe(m) => m.store_ppu(cart, address, data, vram),
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
        match &mut self.inner {
            Mapper12Inner::Mmc3(m) => m.mmc3.ppu_clock(
                ppu_address_bus,
                ppu_a12_prev,
                scanline,
                dot,
                ppu_sprite_x16,
                rendering_on,
            ),
            Mapper12Inner::Ffe(m) => m.ppu_clock(
                ppu_address_bus,
                ppu_a12_prev,
                scanline,
                dot,
                ppu_sprite_x16,
                rendering_on,
            ),
        }
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        match &mut self.inner {
            Mapper12Inner::Mmc3(m) => m.mmc3.cpu_clock_rise(ppu_address_bus),
            Mapper12Inner::Ffe(_) => false,
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        match &mut self.inner {
            Mapper12Inner::Mmc3(_) => false,
            Mapper12Inner::Ffe(m) => m.cpu_clock(cycles),
        }
    }

    fn cpu_clock_irq_level(&self) -> bool {
        match &self.inner {
            Mapper12Inner::Mmc3(m) => m.mmc3.cpu_clock_irq_level(),
            Mapper12Inner::Ffe(m) => m.cpu_clock_irq_level(),
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        match &mut self.inner {
            Mapper12Inner::Mmc3(m) => m.mmc3.take_irq_ack(),
            Mapper12Inner::Ffe(m) => m.take_irq_ack(),
        }
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        match &self.inner {
            Mapper12Inner::Mmc3(m) => {
                let mut state = m.mmc3.save_mapper_registers(cart);
                state.push(0);
                state.push(m.reg);
                state
            }
            Mapper12Inner::Ffe(m) => {
                let mut state = m.save_mapper_registers(cart);
                state.push(1);
                state
            }
        }
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        match &mut self.inner {
            Mapper12Inner::Mmc3(m) => {
                let mut idx = m.mmc3.load_mapper_registers(cart, state, start);
                if idx < state.len() {
                    let _type = state[idx];
                    idx += 1;
                }
                if idx < state.len() {
                    m.reg = state[idx];
                    idx += 1;
                }
                idx
            }
            Mapper12Inner::Ffe(m) => {
                let mut idx = m.load_mapper_registers(cart, state, start);
                if idx < state.len() {
                    let _type = state[idx];
                    idx += 1;
                }
                idx
            }
        }
    }
}
