use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper224 {
    mmc3: MapperMMC3,
    outer_bank: u8,
}

impl Mapper224 {
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
            outer_bank: 0,
        }
    }

    fn prg_bank(&self, cart: &Cartridge, raw_bank: u8) -> usize {
        let outer = (self.outer_bank as u16) << 6;
        let full_bank = (raw_bank as u16) | outer;
        let num_8k = cart.prg_rom.len() / 0x2000;
        (full_bank as usize) % num_8k
    }

    fn prg_offset(&self, cart: &Cartridge, bank: u8, address: u16) -> usize {
        self.prg_bank(cart, bank) * 0x2000 + (address as usize & 0x1FFF)
    }

    fn fixed_last(&self, cart: &Cartridge) -> u8 {
        ((cart.prg_rom.len() / 0x2000).saturating_sub(1)) as u8
    }

    fn fixed_second_last(&self, cart: &Cartridge) -> u8 {
        ((cart.prg_rom.len() / 0x2000).saturating_sub(2)) as u8
    }
}

impl Mapper for Mapper224 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.outer_bank = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = match address {
                0x8000..=0x9FFF => {
                    if (self.mmc3.r8000 & 0x40) == 0 {
                        self.mmc3.bank_8c
                    } else {
                        self.fixed_second_last(cart)
                    }
                }
                0xA000..=0xBFFF => self.mmc3.bank_a,
                0xC000..=0xDFFF => {
                    if (self.mmc3.r8000 & 0x40) != 0 {
                        self.mmc3.bank_8c
                    } else {
                        self.fixed_second_last(cart)
                    }
                }
                0xE000..=0xFFFF => self.fixed_last(cart),
                _ => 0,
            };
            let offset = self.prg_offset(cart, bank, address);
            FetchResult {
                data: if offset < cart.prg_rom.len() {
                    cart.prg_rom[offset]
                } else {
                    0
                },
                driven: true,
            }
        } else {
            self.mmc3.fetch_prg(cart, address)
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address == 0x5000 {
            self.outer_bank = (data >> 2) & 0x01;
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
        self.mmc3
            .ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.outer_bank);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let p = self.mmc3.load_mapper_registers(cart, state, start);
        if p < state.len() {
            self.outer_bank = state[p];
            p + 1
        } else {
            p
        }
    }
}
