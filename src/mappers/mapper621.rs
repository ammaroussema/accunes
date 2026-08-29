use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc1::{MapperMMC1, Mmc1Config};

pub struct Mapper621 {
    mmc1: MapperMMC1,
}

impl Mapper621 {
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
            155,
            0,
            header[4],
            using_chr_ram,
            has_battery,
        );
        Self {
            mmc1: MapperMMC1::new(config),
        }
    }
}

impl Mapper for Mapper621 {
    fn reset(&mut self) {
        self.mmc1.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if cart.prg_rom.is_empty() {
                return FetchResult { data: 0, driven: false };
            }
            let offset = self.mmc1.core.prg_rom_offset(cart, address);
            FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.mmc1.core.write_register(cart, address, data, cart.mapper_cpu_cycle);
        } else if (0x6000..0x8000).contains(&address) {
            if (data & 0x80) != 0 {
                self.mmc1.core.control |= 0x0C;
                self.mmc1.core.shift_register = 0x10;
            } else {
                match address & 3 {
                    0 => self.mmc1.core.control = data,
                    1 => self.mmc1.core.chr0 = data,
                    2 => self.mmc1.core.chr1 = data,
                    _ => self.mmc1.core.prg = data,
                }
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mmc1.mirror_nametable(cart, address)
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
        self.mmc1.fetch_ppu(
            prg_rom, chr_rom, prg_ram, chr_ram, prg_vram,
            using_chr_ram, nametable_horizontal_mirroring,
            alternative_nametable_arrangement,
            ppu_address_bus, ppu_octal_latch, vram,
        )
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        self.mmc1.store_ppu(cart, address, data, vram);
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc1.cpu_clock_rise(ppu_address_bus)
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.mmc1.cpu_clock(cycles)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        self.mmc1.save_mapper_registers(cart)
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        self.mmc1.load_mapper_registers(cart, state, start)
    }
}
