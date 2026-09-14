use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper44 {
    mmc3: MapperMMC3,
    selected_block: u8,
}

impl Mapper44 {
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
        Self {
            mmc3: MapperMMC3::new(config),
            selected_block: 0,
        }
    }

    fn remap_prg(&self, raw: u8) -> usize {
        let block = self.selected_block;
        let mask = if block <= 5 { 0x0F } else { 0x1F };
        let offset = block as usize * 0x10;
        ((raw & mask) as usize) | offset
    }

    fn remap_chr(&self, raw: u8) -> usize {
        let block = self.selected_block;
        let mask = if block <= 5 { 0x7F } else { 0xFF };
        let offset = block as usize * 0x80;
        ((raw as usize) & mask) | offset
    }

    fn prg_rom_read(cart: &Cartridge, bank_8k: usize, offset_in_bank: usize) -> u8 {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }
        let offset = (bank_8k * 0x2000 + offset_in_bank) % len;
        cart.prg_rom[offset]
    }

    fn chr_read_byte(&self, address: u16, chr_rom: &[u8], chr_ram: &[u8], using_chr_ram: bool) -> u8 {
        let raw = self.mmc3.chr_bank(address);
        let bank = self.remap_chr(raw);
        let offset_in_bank = address as usize & 0x03FF;
        if using_chr_ram && !chr_ram.is_empty() {
            chr_ram[(bank * 0x400 + offset_in_bank) % chr_ram.len()]
        } else if !chr_rom.is_empty() {
            chr_rom[(bank * 0x400 + offset_in_bank) % chr_rom.len()]
        } else {
            0
        }
    }
}

impl Mapper for Mapper44 {
    fn reset(&mut self) {
        self.selected_block = 0;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let num_8k = len / 0x2000;
            let invert = (self.mmc3.r8000 & 0x40) != 0;
            let offset_in_bank = address as usize & 0x1FFF;
            let bank = match address {
                0xE000..=0xFFFF => {
                    let raw_fixed = num_8k.saturating_sub(1) as u8;
                    self.remap_prg(raw_fixed) % num_8k
                }
                0xC000..=0xDFFF => {
                    if invert {
                        self.remap_prg(self.mmc3.bank_8c) % num_8k
                    } else {
                        let raw_fixed = num_8k.saturating_sub(2) as u8;
                        self.remap_prg(raw_fixed) % num_8k
                    }
                }
                0xA000..=0xBFFF => {
                    self.remap_prg(self.mmc3.bank_a) % num_8k
                }
                _ => {
                    if invert {
                        let raw_fixed = num_8k.saturating_sub(2) as u8;
                        self.remap_prg(raw_fixed) % num_8k
                    } else {
                        self.remap_prg(self.mmc3.bank_8c) % num_8k
                    }
                }
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

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            if (address & 0xE001) == 0xA001 {
                self.selected_block = data & 0x07;
                if self.selected_block == 7 {
                    self.selected_block = 6;
                }
            }
        }
        self.mmc3.store_prg(cart, address, data);
    }

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
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
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = self.chr_read_byte(address, chr_rom, chr_ram, using_chr_ram);
            new_addr_bus |= byte as u16;
            (new_addr_bus as u8, new_addr_bus)
        } else {
            self.mmc3.fetch_ppu(
                prg_rom, chr_rom, prg_ram, chr_ram, prg_vram,
                using_chr_ram, nametable_horizontal_mirroring,
                alternative_nametable_arrangement,
                ppu_address_bus, ppu_octal_latch, vram,
            )
        }
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let raw = self.mmc3.chr_bank(address);
                let bank = self.remap_chr(raw);
                let len = cart.chr_ram.len();
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
        self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.selected_block);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut idx = self.mmc3.load_mapper_registers(cart, state, start);
        if idx < state.len() {
            self.selected_block = state[idx];
            idx += 1;
        }
        idx
    }
}
