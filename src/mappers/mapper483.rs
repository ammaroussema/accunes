use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc1::{mmc1_mirror_for_ppu, MapperMMC1, Mmc1Config, Mmc1Variant};
pub struct Mapper483 {
    mmc1: MapperMMC1,
    game: u8,
    latch: u8,
}
impl Mapper483 {
    pub fn new(
        header: &[u8],
        rom: &[u8],
        _rom_name: &str,
        using_chr_ram: bool,
        has_battery: bool,
    ) -> Self {
        let config = Mmc1Config::for_ines(
            header,
            rom,
            1,
            0,
            header[4],
            using_chr_ram,
            has_battery,
        );
        Self {
            mmc1: MapperMMC1::new(config),
            game: 0,
            latch: 0,
        }
    }
    fn mmc1_prg_raw_bank(&self, bank: usize) -> usize {
        let prg = self.mmc1.core.prg as usize;
        let control = self.mmc1.core.control as usize;
        let result = if control & 0x08 != 0 {
            if control & 0x04 != 0 {
                prg | (bank * 0x0F)
            } else {
                prg & (bank * 0x0F)
            }
        } else {
            prg & !1 | bank
        };
        if (prg & 0x10) != 0 && self.mmc1.core.config.variant == Mmc1Variant::Mmc1A {
            (result & 0x07) | (prg & 0x08)
        } else {
            result & 0x0F
        }
    }
    fn mmc1_chr_raw_bank(&self, slot: usize) -> usize {
        let control = self.mmc1.core.control;
        if (control & 0x10) != 0 {
            match slot {
                0 => self.mmc1.core.chr0 as usize,
                _ => self.mmc1.core.chr1 as usize,
            }
        } else {
            (self.mmc1.core.chr0 as usize & !1) | slot
        }
    }
    fn lookup_prg(&self, cart: &Cartridge, address: u16) -> usize {
        let offset = match self.game {
            0 | 1 | 2 => {
                let bank = if address >= 0xC000 { 1 } else { 0 };
                let raw = self.mmc1_prg_raw_bank(bank);
                let bank16 = (raw & 0x07) | ((self.game as usize) << 3);
                bank16 * 0x4000 + (address as usize & 0x3FFF)
            }
            3 | 4 | 5 => {
                let bank32 = (3 << 2) | (((self.game as usize) & 0xFF) - 3);
                bank32 * 0x8000 + (address as usize & 0x7FFF)
            }
            6 => {
                let bank32 = (3 << 2) | 3;
                bank32 * 0x8000 + (address as usize & 0x7FFF)
            }
            _ => unreachable!(),
        };
        let len = if cart.prg_rom.is_empty() { 1 } else { cart.prg_rom.len() };
        offset % len
    }
    fn lookup_chr_offset(&self, address: u16, chr_len: usize) -> usize {
        let addr = address as usize;
        let slot = (addr >> 12) & 0x03;
        let offset = match self.game {
            0 | 1 | 2 => {
                let raw = self.mmc1_chr_raw_bank(slot);
                let bank = (raw & 0x1F) | ((self.game as usize) << 5);
                bank * 0x1000 + (addr & 0x0FFF)
            }
            3 | 4 | 5 => {
                let bank = (3 << 4) | (((self.game as usize) - 3) << 2) | ((self.latch as usize) & 0x03);
                bank * 0x2000 + (addr & 0x1FFF)
            }
            6 => {
                let raw = self.mmc1_chr_raw_bank(slot);
                let bank = (raw & 0x07) | ((3 << 5) | (3 << 3));
                bank * 0x1000 + (addr & 0x0FFF)
            }
            _ => unreachable!(),
        };
        let chr_len = chr_len.max(1);
        offset % chr_len
    }
    fn chr_byte(&self, chr_rom: &[u8], chr_ram: &[u8], using_chr_ram: bool, offset: usize) -> u8 {
        let len = if using_chr_ram { chr_ram.len() } else { chr_rom.len() };
        if len == 0 {
            0
        } else if using_chr_ram {
            chr_ram[offset % len]
        } else {
            chr_rom[offset % len]
        }
    }
}
impl Mapper for Mapper483 {
    fn reset(&mut self) {
        self.game = (self.game + 1) % 7;
        self.latch = 0;
        self.mmc1.reset();
    }
    fn reset_power_cycle(&mut self) {
        self.game = 0;
        self.latch = 0;
        self.mmc1.reset();
    }
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if cart.prg_rom.is_empty() {
                return FetchResult { data: 0, driven: true };
            }
            let offset = self.lookup_prg(cart, address);
            FetchResult { data: cart.prg_rom[offset], driven: true }
        } else {
            self.mmc1.fetch_prg(cart, address)
        }
    }
    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            match self.game {
                0 | 1 | 2 | 6 => self.mmc1.store_prg(cart, address, data),
                3 | 4 | 5 => self.latch = data,
                _ => {}
            }
        } else {
            self.mmc1.store_prg(cart, address, data);
        }
    }
    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if self.game == 3 || self.game == 4 || self.game == 5 {
            address & 0x37FF
        } else {
            self.mmc1.mirror_nametable(cart, address)
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
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let chr_len = if using_chr_ram { chr_ram.len() } else { chr_rom.len() };
            let offset = self.lookup_chr_offset(address, chr_len);
            let byte = self.chr_byte(chr_rom, chr_ram, using_chr_ram, offset);
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if self.game == 3 || self.game == 4 || self.game == 5 {
                address & 0x37FF
            } else {
                mmc1_mirror_for_ppu(&self.mmc1.core, nametable_horizontal_mirroring, address)
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }
    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let offset = self.lookup_chr_offset(address, cart.chr_ram.len());
                cart.chr_ram[offset] = data;
            }
        } else if address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }
    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc1.cpu_clock_rise(ppu_address_bus)
    }
    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.mmc1.cpu_clock(cycles)
    }
    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc1.save_mapper_registers(cart);
        state.push(self.game);
        state.push(self.latch);
        state
    }
    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut idx = self.mmc1.load_mapper_registers(cart, state, start);
        if idx < state.len() {
            self.game = state[idx];
            idx += 1;
        }
        if idx < state.len() {
            self.latch = state[idx];
            idx += 1;
        }
        idx
    }
}
