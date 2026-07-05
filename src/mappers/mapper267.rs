use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, mmc3_chr_bank};

pub struct Mapper267 {
    mmc3: MapperMMC3,
    reg: u8,
}

impl Mapper267 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let config = Mmc3Config::for_ines(header, 0, if chr_size == 0 { 0 } else { chr_size }, rom, rom_name);
        Self { mmc3: MapperMMC3::new(config), reg: 0 }
    }

    fn outer_bank(&self) -> u8 {
        (self.reg >> 2 & 0x08) | (self.reg & 0x06)
    }

    fn fixed_last(&self, cart: &Cartridge) -> u8 {
        ((cart.prg_rom.len() / 0x2000).saturating_sub(1)) as u8
    }

    fn fixed_second_last(&self, cart: &Cartridge) -> u8 {
        ((cart.prg_rom.len() / 0x2000).saturating_sub(2)) as u8
    }
}

impl Mapper for Mapper267 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.reg = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let raw_bank = match address {
                0x8000..=0x9FFF => {
                    if (self.mmc3.r8000 & 0x40) == 0 { self.mmc3.bank_8c } else { self.fixed_second_last(cart) }
                }
                0xA000..=0xBFFF => self.mmc3.bank_a,
                0xC000..=0xDFFF => {
                    if (self.mmc3.r8000 & 0x40) != 0 { self.mmc3.bank_8c } else { self.fixed_second_last(cart) }
                }
                0xE000..=0xFFFF => self.fixed_last(cart),
                _ => 0,
            };
            let outer = self.outer_bank();
            let full_bank = ((raw_bank as u16) & 0x1F) | ((outer as u16) << 4);
            let num_8k = cart.prg_rom.len() / 0x2000;
            let bank = (full_bank as usize) % num_8k;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult { data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 }, driven: true }
        } else {
            self.mmc3.fetch_prg(cart, address)
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (address & 0xF000) == 0x5000 {
            if self.reg & 0x80 == 0 {
                self.reg = data;
            }
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
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let raw_bank = mmc3_chr_bank(
                self.mmc3.r8000, self.mmc3.chr_2k0, self.mmc3.chr_2k8,
                self.mmc3.chr_1k0, self.mmc3.chr_1k4, self.mmc3.chr_1k8, self.mmc3.chr_1kc, address,
            );
            let outer = self.outer_bank();
            let full_bank = ((raw_bank as u16) & 0x7F) | ((outer as u16) << 6);
            let offset = (full_bank as usize) * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if self.mmc3.nametable_mirroring() {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() { prg_vram[idx] } else { 0 }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram && !cart.chr_ram.is_empty() {
            let raw_bank = mmc3_chr_bank(
                self.mmc3.r8000, self.mmc3.chr_2k0, self.mmc3.chr_2k8,
                self.mmc3.chr_1k0, self.mmc3.chr_1k4, self.mmc3.chr_1k8, self.mmc3.chr_1kc, address,
            );
            let outer = self.outer_bank();
            let full_bank = ((raw_bank as u16) & 0x7F) | ((outer as u16) << 6);
            let offset = (full_bank as usize) * 0x0400 + (address as usize & 0x03FF);
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
