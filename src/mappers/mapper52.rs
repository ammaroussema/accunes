use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper52 {
    mmc3: MapperMMC3,
    extra_reg: u8,
    locked: bool,
}

impl Mapper52 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let config = Mmc3Config::for_ines(header, 0, chr_size, rom, rom_name);
        Self {
            mmc3: MapperMMC3::new(config),
            extra_reg: 0,
            locked: false,
        }
    }

    fn remap_prg(&self, bank: usize) -> usize {
        let r = self.extra_reg as usize;
        let base = ((r & 0x06) | ((r >> 3) & r & 0x01)) << 4;
        let mask = ((r << 1) & 0x10) ^ 0x1F;
        base | (bank & mask)
    }

    fn remap_chr(&self, bank: usize) -> usize {
        let r = self.extra_reg as usize;
        let base = (((r >> 3) & 0x04) | ((r >> 1) & 0x02) | (((r >> 6) & (r >> 4)) & 0x01)) << 7;
        let mask = ((r & 0x40) << 1) ^ 0xFF;
        base | (bank & mask)
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
        let raw = self.mmc3.chr_bank(address) as usize;
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

impl Mapper for Mapper52 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.extra_reg = 0;
        self.locked = false;
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
            let raw = match address {
                0xE000..=0xFFFF => num_8k.saturating_sub(1),
                0xC000..=0xDFFF => {
                    if invert {
                        self.mmc3.bank_8c as usize
                    } else {
                        num_8k.saturating_sub(2)
                    }
                }
                0xA000..=0xBFFF => self.mmc3.bank_a as usize,
                _ => {
                    if invert {
                        num_8k.saturating_sub(2)
                    } else {
                        self.mmc3.bank_8c as usize
                    }
                }
            };
            let bank = self.remap_prg(raw) % num_8k;
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
        if address >= 0x6000 && address < 0x8000 {
            if self.locked {
                self.mmc3.store_prg(cart, address, data);
            } else {
                self.locked = true;
                self.extra_reg = data;
            }
        } else {
            self.mmc3.store_prg(cart, address, data);
        }
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
                let raw = self.mmc3.chr_bank(address) as usize;
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
        state.push(self.extra_reg);
        state.push(if self.locked { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut idx = self.mmc3.load_mapper_registers(cart, state, start);
        if idx < state.len() { self.extra_reg = state[idx]; idx += 1; }
        if idx < state.len() { self.locked = state[idx] != 0; idx += 1; }
        idx
    }
}
