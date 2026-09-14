
use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc1::{mmc1_mirror_for_ppu, MapperMMC1, Mmc1Config};

pub struct Mapper374 {
    mmc1: MapperMMC1,
    game: u8,
    first_reset: bool,
}

impl Mapper374 {
    pub fn new(header: &[u8], rom: &[u8], _rom_name: &str, using_chr_ram: bool, has_battery: bool) -> Self {
        let config = Mmc1Config::for_ines(
            header,
            rom,
            155,
            0,
            header[4],
            using_chr_ram,
            has_battery,
        );
        Self {
            mmc1: MapperMMC1::new(config),
            game: 0,
            first_reset: true,
        }
    }

    fn get_chr_bank_4k(&self, slot: usize) -> usize {
        let chr4k_mode = (self.mmc1.core.control & 0x10) != 0;
        if chr4k_mode {
            match slot {
                0 => self.mmc1.core.chr0 as usize,
                _ => self.mmc1.core.chr1 as usize,
            }
        } else {
            (self.mmc1.core.chr0 as usize & !1) | slot
        }
    }

}

impl Mapper for Mapper374 {
    fn reset(&mut self) {
        if self.first_reset {
            self.first_reset = false;
            self.game = 0;
        } else {
            self.game = self.game.wrapping_add(1);
        }
        self.mmc1.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let num_16k = len / 0x4000;
            let raw_offset = self.mmc1.core.prg_rom_offset(cart, address);
            let raw_bank = raw_offset / 0x4000;
            let effective_bank = (raw_bank & 0x07) | ((self.game as usize) << 3);
            let offset = (effective_bank % num_16k) * 0x4000 + (address as usize & 0x3FFF);
            FetchResult { data: cart.prg_rom[offset % len], driven: true }
        } else if address >= 0x6000 {
            self.mmc1.fetch_prg(cart, address)
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        self.mmc1.store_prg(cart, address, data);
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mmc1.mirror_nametable(cart, address)
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
            let slot = (address >> 12) as usize; 

            let raw_4k_bank = self.get_chr_bank_4k(slot);

            let effective_4k_bank = (raw_4k_bank & 0x1F) | ((self.game as usize) << 5);

            let offset = effective_4k_bank * 0x1000 + (address as usize & 0x0FFF);
            let byte = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[offset % chr_ram.len()] }
            } else {
                if chr_rom.is_empty() { 0 } else { chr_rom[offset % chr_rom.len()] }
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mmc1_mirror_for_ppu(&self.mmc1.core, nametable_horizontal_mirroring, address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }

        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let slot = (address >> 12) as usize;
                let raw_4k_bank = self.get_chr_bank_4k(slot);
                let effective_4k_bank = (raw_4k_bank & 0x1F) | ((self.game as usize) << 5);
                let offset = effective_4k_bank * 0x1000 + (address as usize & 0x0FFF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
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
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut idx = self.mmc1.load_mapper_registers(cart, state, start);
        if idx < state.len() {
            self.game = state[idx];
            idx += 1;
        }
        idx
    }
}
