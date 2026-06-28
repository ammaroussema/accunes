use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper89 {
    data: u8,
}

impl Mapper89 {
    pub fn new() -> Self {
        Self { data: 0 }
    }

    fn prg_read(&self, cart: &Cartridge, address: u16) -> u8 {
        let prg_len = cart.prg_rom.len();
        if prg_len == 0 {
            return 0;
        }
        let bank = if address < 0xC000 {
            ((self.data >> 4) & 0x07) as usize
        } else {
            (prg_len / 0x4000).saturating_sub(1)
        };
        let offset = (bank * 0x4000 + (address as usize & 0x3FFF)) % prg_len;
        cart.prg_rom[offset]
    }

    fn chr_read(
        &self,
        address: u16,
        chr_rom: &[u8],
        chr_ram: &[u8],
        using_chr_ram: bool,
    ) -> u8 {
        let len = if using_chr_ram { chr_ram.len() } else { chr_rom.len() };
        if len == 0 {
            return 0;
        }
        let bank = ((self.data as usize) & 0x07) | (((self.data as usize) & 0x80) >> 4);
        let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % len;
        if using_chr_ram {
            chr_ram[offset]
        } else {
            chr_rom[offset]
        }
    }
}

impl Mapper for Mapper89 {
    fn reset(&mut self) {
        self.data = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
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

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, mut data: u8) {
        if address >= 0x8000 {
            let prg_len = cart.prg_rom.len();
            if prg_len > 0 {
                let fetch_res = self.fetch_prg(cart, address);
                if fetch_res.driven {
                    data &= fetch_res.data;
                }
            }
            self.data = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if (self.data & 0x08) != 0 {
            0x2000 | 0x400 | (address & 0x3FF)
        } else {
            0x2000 | (address & 0x3FF)
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
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let data = self.chr_read(address, chr_rom, chr_ram, using_chr_ram);
            new_addr_bus |= data as u16;
        } else {
            let dummy_cart = Cartridge {
                name: String::new(),
                prg_rom: Vec::new(),
                chr_rom: Vec::new(),
                memory_mapper: 89,
                sub_mapper: 0,
                prg_size: 0,
                chr_size: 0,
                prg_size_minus_1: 0,
                chr_ram: Vec::new(),
                using_chr_ram: false,
                prg_ram: Vec::new(),
                has_battery: false,
                alternative_nametable_arrangement: false,
                prg_vram: Vec::new(),
                nametable_horizontal_mirroring: false,
                fds_disks: Vec::new(),
                trainer: Vec::new(),
                misc_rom: Vec::new(),
                mapper_chip: Box::new(crate::mapper::MapperNROM::new(crate::mapper::NromConfig::default())),
                mapper_cpu_cycle: 0,
                prg_rom_crc32: 0,
                chr_rom_crc32: 0,
                overall_crc32: 0,
                is_vs_system: false,
                tv_system: crate::region::TvSystem::Unknown,
            };
            let mirrored = self.mirror_nametable(&dummy_cart, address);
            let data = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= data as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.data]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start < state.len() {
            self.data = state[start];
            start + 1
        } else {
            start
        }
    }
}
