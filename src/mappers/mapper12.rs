use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper12 {
    mmc3: MapperMMC3,
    preg: u8,
    expreg0: u8,
    expreg1: u8,
    language_bit: u8,
}

impl Mapper12 {
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
            preg: 0,
            expreg0: 0,
            expreg1: 0,
            language_bit: 1,
        }
    }

    fn in_expansion_bus(address: u16) -> bool {
        (0x4100..=0x5FFF).contains(&address)
    }

    fn prg_read(&self, cart: &Cartridge, address: u16) -> u8 {
        if address >= 0x6000 && address < 0x8000 {
            if cart.prg_ram.is_empty() {
                return 0;
            }
            let off = (address - 0x6000) as usize % cart.prg_ram.len();
            return cart.prg_ram[off];
        }
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }
        if address >= 0xC000 {
            let bank = (len / 0x4000).saturating_sub(1);
            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            return cart.prg_rom[offset % len];
        }
        if address >= 0x8000 {
            let banks_16k = (len / 0x4000).max(1);
            let bank = self.preg as usize % banks_16k;
            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            return cart.prg_rom[offset % len];
        }
        0
    }

    fn chr_read(
        &self,
        address: u16,
        chr_rom: &[u8],
        chr_ram: &[u8],
        using_chr_ram: bool,
    ) -> u8 {
        let len = if using_chr_ram {
            chr_ram.len()
        } else {
            chr_rom.len()
        };
        if len == 0 {
            return 0;
        }
        let base = self.mmc3.chr_bank(address) as usize;
        let ext = if address < 0x1000 {
            self.expreg0
        } else {
            self.expreg1
        };
        let banks_1k = len / 0x400;
        let bank = if banks_1k > 0 {
            (base | ((ext as usize) << 8)) % banks_1k
        } else {
            0
        };
        let offset = bank * 0x400 + (address as usize & 0x3FF);
        if using_chr_ram {
            chr_ram[offset % len]
        } else {
            chr_rom[offset % len]
        }
    }
}

impl Mapper for Mapper12 {
    fn reset(&mut self) {
        self.preg = 0;
        self.expreg0 = 0;
        self.expreg1 = 0;
        self.language_bit ^= 1;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if Self::in_expansion_bus(address) {
            return FetchResult {
                data: self.language_bit,
                driven: true,
            };
        }
        if address >= 0x6000 {
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
        if Self::in_expansion_bus(address) {
            self.expreg0 = data & 0x01;
            self.expreg1 = (data & 0x10) >> 4;
            return;
        }
        if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let off = (address - 0x6000) as usize % cart.prg_ram.len();
                cart.prg_ram[off] = data;
            }
            return;
        }
        if address == 0xA000 {
            self.preg = data;
            return;
        }
        if address >= 0x8000 {
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
            let byte = self.chr_read(address, chr_rom, chr_ram, using_chr_ram);
            new_addr_bus |= byte as u16;
            (new_addr_bus as u8, new_addr_bus)
        } else {
            self.mmc3.fetch_ppu(
                _prg_rom,
                chr_rom,
                _prg_ram,
                chr_ram,
                _prg_vram,
                using_chr_ram,
                nametable_horizontal_mirroring,
                alternative_nametable_arrangement,
                ppu_address_bus,
                ppu_octal_latch,
                vram,
            )
        }
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                let base = self.mmc3.chr_bank(address) as usize;
                let ext = if address < 0x1000 {
                    self.expreg0
                } else {
                    self.expreg1
                };
                let banks_1k = len / 0x400;
                let bank = if banks_1k > 0 {
                    (base | ((ext as usize) << 8)) % banks_1k
                } else {
                    0
                };
                let offset = (bank * 0x400 + (address as usize & 0x3FF)) % len;
                cart.chr_ram[offset] = data;
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
        state.push(self.preg);
        state.push(self.expreg0);
        state.push(self.expreg1);
        state.push(self.language_bit);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut idx = self.mmc3.load_mapper_registers(cart, state, start);
        if idx < state.len() {
            self.preg = state[idx];
            idx += 1;
        }
        if idx < state.len() {
            self.expreg0 = state[idx];
            idx += 1;
        }
        if idx < state.len() {
            self.expreg1 = state[idx];
            idx += 1;
        }
        if idx < state.len() {
            self.language_bit = state[idx];
            idx += 1;
        }
        idx
    }
}
