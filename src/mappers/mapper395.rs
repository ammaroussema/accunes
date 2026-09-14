use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, mmc3_chr_bank};

pub struct Mapper395 {
    mmc3: MapperMMC3,
    reg: [u8; 2],
}

impl Mapper395 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let mut config = Mmc3Config::for_ines(header, 0, chr_size, rom, rom_name);
        config.ax5202p = true;
        config.prg_ram_size = 0;
        Self { mmc3: MapperMMC3::new(config), reg: [0; 2] }
    }

    fn prg_and(&self) -> u8 {
        if (self.reg[1] & 0x08) != 0 { 0x0F } else { 0x1F }
    }

    fn prg_or(&self) -> u8 {
        ((self.reg[0] << 1) & 0x60) | ((self.reg[0] << 4) & 0x80) | ((self.reg[1] << 4) & 0x10)
    }

    fn chr_and(&self) -> u8 {
        if (self.reg[1] & 0x40) != 0 { 0x7F } else { 0xFF }
    }

    fn chr_or(&self) -> u16 {
        ((self.reg[0] as u16) << 4 & 0x300) | ((self.reg[1] as u16) << 5 & 0x400) | ((self.reg[1] as u16) << 3 & 0x80)
    }

    fn prg_raw_bank_val(&self, cart: &Cartridge, cpu_bank: u8) -> u16 {
        let num_banks = (cart.prg_rom.len() / 0x2000) as u16;
        match cpu_bank {
            0 => {
                if (self.mmc3.r8000 & 0x40) != 0 {
                    num_banks.saturating_sub(2)
                } else {
                    self.mmc3.bank_8c as u16
                }
            }
            1 => self.mmc3.bank_a as u16,
            2 => {
                if (self.mmc3.r8000 & 0x40) != 0 {
                    self.mmc3.bank_8c as u16
                } else {
                    num_banks.saturating_sub(2)
                }
            }
            _ => num_banks.saturating_sub(1),
        }
    }
}

impl Mapper for Mapper395 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.reg = [0; 2];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let cpu_bank = ((address - 0x8000) / 0x2000) as u8;
            let raw_bank = self.prg_raw_bank_val(cart, cpu_bank);
            let bank = ((raw_bank as u8) & self.prg_and()) | self.prg_or();
            let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
            let len = cart.prg_rom.len();
            let data = if len > 0 { cart.prg_rom[offset % len] } else { 0 };
            FetchResult { data, driven: true }
        } else {
            self.mmc3.fetch_prg(cart, address)
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            self.mmc3.store_prg(cart, address, data);
            if (self.mmc3.prg_ram_protect & 0x40) == 0 && (self.reg[1] & 0x80) == 0 {
                self.reg[(address as usize >> 4) & 1] = data;
            }
        } else if address >= 0x8000 {
            self.mmc3.store_prg(cart, address, data);
        } else {
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
        let raw_bank = mmc3_chr_bank(
            self.mmc3.r8000, self.mmc3.chr_2k0, self.mmc3.chr_2k8,
            self.mmc3.chr_1k0, self.mmc3.chr_1k4, self.mmc3.chr_1k8, self.mmc3.chr_1kc, address,
        );
        let bank = ((raw_bank & self.chr_and()) as u16) | self.chr_or();
        let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
        let byte = if !chr_rom.is_empty() {
            chr_rom[offset % chr_rom.len()]
        } else if !chr_ram.is_empty() {
            chr_ram[offset % chr_ram.len()]
        } else { 0 };
        new_addr_bus |= byte as u16;
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && !cart.chr_ram.is_empty() {
            let raw_bank = mmc3_chr_bank(
                self.mmc3.r8000, self.mmc3.chr_2k0, self.mmc3.chr_2k8,
                self.mmc3.chr_1k0, self.mmc3.chr_1k4, self.mmc3.chr_1k8, self.mmc3.chr_1kc, address,
            );
            let bank = ((raw_bank & self.chr_and()) as u16) | self.chr_or();
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

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.reg[0]);
        state.push(self.reg[1]);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let p = self.mmc3.load_mapper_registers(cart, state, start);
        if p + 1 < state.len() {
            self.reg[0] = state[p];
            self.reg[1] = state[p + 1];
            p + 2
        } else if p < state.len() {
            self.reg[0] = state[p];
            p + 1
        } else {
            p
        }
    }
}
