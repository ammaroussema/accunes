use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, Mmc3IrqHack};

pub struct Mapper245 {
    mmc3: MapperMMC3,
    chr_latch: u8,
}

impl Mapper245 {
    pub fn new() -> Self {
        let config = Mmc3Config {
            prg_ram_size: 0x2000,
            chr_ram_size: 0x2000,
            mmc6: false,
            irq_revision_b: false,
            irq_hack: Mmc3IrqHack::None,
            header_horizontal_mirror: false,
        };
        Self {
            mmc3: MapperMMC3::new(config),
            chr_latch: 0,
        }
    }

    fn prg_rom_read(cart: &Cartridge, offset: usize) -> u8 {
        let len = cart.prg_rom.len();
        if len == 0 {
            0
        } else {
            cart.prg_rom[offset % len]
        }
    }

    fn prg_offset(&self) -> u8 {
        if (self.chr_latch & 0x02) != 0 {
            0x40
        } else {
            0x00
        }
    }

    fn remap_prg(&self, raw: u8) -> u8 {
        (raw & 0x3F) | self.prg_offset()
    }
}

impl Mapper for Mapper245 {
    fn reset(&mut self) {
        self.chr_latch = 0;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            return self.mmc3.fetch_prg(cart, address);
        }
        let len = cart.prg_rom.len();
        if len == 0 {
            return FetchResult { data: 0, driven: true };
        }
        let num_8k = len / 0x2000;
        let invert = (self.mmc3.r8000 & 0x40) != 0;
        let offset_in_bank = address as usize & 0x1FFF;
        let bank = match address {
            0xE000..=0xFFFF => {
                let raw = (num_8k.saturating_sub(1) as u8) & 0x3F;
                self.remap_prg(raw) as usize % num_8k
            }
            0xC000..=0xDFFF => {
                if invert {
                    self.remap_prg(self.mmc3.bank_8c) as usize % num_8k
                } else {
                    let raw = (num_8k.saturating_sub(2) as u8) & 0x3F;
                    self.remap_prg(raw) as usize % num_8k
                }
            }
            0xA000..=0xBFFF => self.remap_prg(self.mmc3.bank_a) as usize % num_8k,
            _ => {
                if invert {
                    let raw = (num_8k.saturating_sub(2) as u8) & 0x3F;
                    self.remap_prg(raw) as usize % num_8k
                } else {
                    self.remap_prg(self.mmc3.bank_8c) as usize % num_8k
                }
            }
        };
        FetchResult {
            data: Self::prg_rom_read(cart, bank * 0x2000 + offset_in_bank),
            driven: true,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            if (address & 0xE001) == 0x8001 {
                let mode = self.mmc3.r8000 & 0x07;
                if mode <= 5 {
                    self.chr_latch = data;
                    self.mmc3.store_prg(cart, address, data & 0x07);
                    return;
                }
                if mode == 6 || mode == 7 {
                    self.mmc3.store_prg(cart, address, data & 0x3F);
                    return;
                }
            }
        }
        self.mmc3.store_prg(cart, address, data);
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

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
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
            ppu_address_bus,
            ppu_a12_prev,
            scanline,
            dot,
            ppu_sprite_x16,
            rendering_on,
        )
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.chr_latch);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut idx = self.mmc3.load_mapper_registers(cart, state, start);
        if idx < state.len() {
            self.chr_latch = state[idx];
            idx += 1;
        }
        idx
    }
}
