use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper48 {
    mmc3: MapperMMC3,
    submapper: u8,
    irq_latch: u8,
    irq_counter: u8,
    irq_enabled: bool,
    irq_reload: bool,
    irq_delay: u8,
    irq_pending: bool,
    irq_clear_requested: bool,
}

impl Mapper48 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let using_chr_ram = chr_size == 0;
        let config = Mmc3Config::for_ines(
            header,
            0,
            if using_chr_ram { 0 } else { chr_size },
            rom,
            rom_name,
        );
        let mut mmc3 = MapperMMC3::new(config);
        mmc3.r8000 = 0;
        let submapper = if header.len() > 8 { (header[8] >> 4) & 0x0F } else { 0 };
        Self {
            mmc3,
            submapper,
            irq_latch: 0,
            irq_counter: 0,
            irq_enabled: false,
            irq_reload: false,
            irq_delay: 0,
            irq_pending: false,
            irq_clear_requested: false,
        }
    }

    fn prg_rom_read(cart: &Cartridge, bank_8k: usize, offset_in_bank: usize) -> u8 {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }
        let offset = (bank_8k * 0x2000 + offset_in_bank) % len;
        cart.prg_rom[offset]
    }
}

impl Mapper for Mapper48 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.mmc3.r8000 = 0;
        self.irq_latch = 0;
        self.irq_counter = 0;
        self.irq_enabled = false;
        self.irq_reload = false;
        self.irq_delay = 0;
        self.irq_pending = false;
        self.irq_clear_requested = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let num_8k = len / 0x2000;
            let offset_in_bank = address as usize & 0x1FFF;
            let bank = match address {
                0xE000..=0xFFFF => num_8k.saturating_sub(1),
                0xC000..=0xDFFF => num_8k.saturating_sub(2),
                0xA000..=0xBFFF => self.mmc3.bank_a as usize,
                _ => self.mmc3.bank_8c as usize,
            };
            FetchResult {
                data: Self::prg_rom_read(cart, bank, offset_in_bank),
                driven: true,
            }
        } else if address >= 0x6000 {
            self.mmc3.fetch_prg(cart, address)
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        match address & 0xE003 {
            0x8000 => {
                self.mmc3.bank_8c = data & 0x3F;
            }
            0x8001 => {
                self.mmc3.bank_a = data & 0x3F;
            }
            0x8002 => {
                self.mmc3.chr_2k0 = (data * 2) & 0xFE;
            }
            0x8003 => {
                self.mmc3.chr_2k8 = (data * 2) & 0xFE;
            }
            0xA000 => {
                self.mmc3.chr_1k0 = data;
            }
            0xA001 => {
                self.mmc3.chr_1k4 = data;
            }
            0xA002 => {
                self.mmc3.chr_1k8 = data;
            }
            0xA003 => {
                self.mmc3.chr_1kc = data;
            }
            0xC000 => {
                self.irq_pending = false;
                self.irq_clear_requested = true;
                self.irq_latch = (data ^ 0xFF) + if self.submapper == 1 { 1 } else { 0 };
            }
            0xC001 => {
                self.irq_pending = false;
                self.irq_clear_requested = true;
                self.irq_counter = 0;
                self.irq_reload = true;
            }
            0xC002 => {
                self.irq_enabled = true;
            }
            0xC003 => {
                self.irq_enabled = false;
                self.irq_pending = false;
                self.irq_clear_requested = true;
            }
            0xE000 => {
                self.mmc3.set_nametable_horizontal((data & 0x40) != 0);
            }
            _ => {}
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.irq_clear_requested {
            self.irq_clear_requested = false;
            true
        } else if self.irq_pending {
            self.irq_pending = false;
            true
        } else {
            false
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
            prg_rom, chr_rom, prg_ram, chr_ram, prg_vram,
            using_chr_ram, nametable_horizontal_mirroring,
            alternative_nametable_arrangement,
            ppu_address_bus, ppu_octal_latch, vram,
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
        let a12 = (ppu_address_bus & 0x1000) != 0;
        if !ppu_a12_prev && a12 && self.mmc3.m2_filter == 3 {
            let prev = self.irq_counter;
            if self.irq_counter == 0 || self.irq_reload {
                self.irq_counter = self.irq_latch;
                self.irq_reload = false;
            } else {
                self.irq_counter = prev.wrapping_sub(1);
            }
            if self.irq_counter == 0 && self.irq_enabled {
                self.irq_delay = if self.submapper == 1 { 6 } else { 22 };
            }
        }
        self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on);
        false
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        let mut irq = false;
        for _ in 0..cycles {
            if self.irq_delay > 0 {
                self.irq_delay -= 1;
                if self.irq_delay == 0 {
                    irq = true;
                    self.irq_pending = true;
                }
            }
        }
        irq
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus);
        self.irq_pending
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.irq_latch);
        state.push(self.irq_counter);
        state.push(if self.irq_enabled { 1 } else { 0 });
        state.push(if self.irq_reload { 1 } else { 0 });
        state.push(self.irq_delay);
        state.push(if self.irq_pending { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut idx = self.mmc3.load_mapper_registers(cart, state, start);
        if idx < state.len() { self.irq_latch = state[idx]; idx += 1; }
        if idx < state.len() { self.irq_counter = state[idx]; idx += 1; }
        if idx < state.len() { self.irq_enabled = state[idx] != 0; idx += 1; }
        if idx < state.len() { self.irq_reload = state[idx] != 0; idx += 1; }
        if idx < state.len() { self.irq_delay = state[idx]; idx += 1; }
        if idx < state.len() { self.irq_pending = state[idx] != 0; idx += 1; }
        idx
    }
}
