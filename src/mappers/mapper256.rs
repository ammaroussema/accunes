use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

const MMC3_MANGLE: [[u8; 8]; 16] = [
    [0, 1, 2, 3, 4, 5, 6, 7],  // 0: Normal
    [5, 4, 3, 2, 1, 0, 6, 7],  // 1: Waixing VT03
    [0, 1, 2, 3, 4, 5, 7, 6],  // 2: Trump Grand
    [0, 1, 2, 3, 4, 5, 6, 7],  // 3: Zechess
    [0, 1, 2, 3, 4, 5, 6, 7],  // 4: Qishenglong
    [0, 1, 2, 3, 4, 5, 6, 7],  // 5: Waixing VT02
    [0, 1, 2, 3, 4, 5, 6, 7],  // 6
    [0, 1, 2, 3, 4, 5, 6, 7],  // 7
    [0, 1, 2, 3, 4, 5, 6, 7],  // 8
    [0, 1, 2, 3, 4, 5, 6, 7],  // 9
    [0, 1, 2, 3, 4, 5, 6, 7],  // A
    [0, 1, 2, 3, 4, 5, 6, 7],  // B
    [0, 1, 2, 3, 4, 5, 6, 7],  // C
    [0, 1, 2, 3, 4, 5, 6, 7],  // D: Cube Tech
    [0, 1, 2, 3, 4, 5, 6, 7],  // E: Karaoto
    [0, 1, 2, 3, 4, 5, 6, 7],  // F: Jungletac
];

pub struct Mapper256 {
    mmc3: MapperMMC3,
    submapper: u8,
}

impl Mapper256 {
    pub fn new(config: Mmc3Config, submapper: u8) -> Self {
        Self {
            mmc3: MapperMMC3::new(config),
            submapper,
        }
    }
}

impl Mapper for Mapper256 {
    fn reset(&mut self) {
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        self.mmc3.fetch_prg(cart, address)
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8107 && address <= 0x810A {
            return;
        }
        if address & 0xE001 == 0x8000 {
            let mangled = data & 0xF8 | MMC3_MANGLE[self.submapper as usize][(data & 0x07) as usize];
            self.mmc3.store_prg(cart, address, mangled);
        } else {
            self.mmc3.store_prg(cart, address, data);
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mmc3.mirror_nametable(cart, address)
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
        self.mmc3.fetch_ppu(
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

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        self.mmc3.store_ppu(cart, address, data, vram);
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
            ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on,
        )
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.submapper);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let idx = self.mmc3.load_mapper_registers(cart, state, start);
        if idx < state.len() {
            self.submapper = state[idx];
            idx + 1
        } else {
            idx
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.mmc3.cpu_clock(cycles)
    }

    fn insert_coin(&mut self, coin: u8) {
        self.mmc3.insert_coin(coin);
    }

    fn service_button(&mut self) {
        self.mmc3.service_button();
    }

    fn get_dip_switches(&self) -> u8 {
        self.mmc3.get_dip_switches()
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.mmc3.set_dip_switches(value);
    }

    fn battery_save_data(&self, cart: &Cartridge) -> Option<Vec<u8>> {
        self.mmc3.battery_save_data(cart)
    }

    fn load_battery_save(&mut self, cart: &mut Cartridge, data: &[u8]) {
        self.mmc3.load_battery_save(cart, data);
    }
}
