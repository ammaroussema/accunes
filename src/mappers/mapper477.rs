use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

pub struct Mapper477 {
    latch: u8,
}

impl Mapper477 {
    pub fn new() -> Self {
        Self { latch: 0 }
    }

    fn prg_16k_banks(&self) -> (usize, usize) {
        if (self.latch & 0x0E) != 0 {
            (0xFF, self.latch as usize)
        } else {
            let bank32 = (self.latch >> 1) as usize;
            (bank32 * 2, bank32 * 2 + 1)
        }
    }
}

impl Mapper for Mapper477 {
    fn reset(&mut self) {
        self.latch = 0;
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 || cart.prg_rom.is_empty() {
            return FetchResult { data: 0, driven: false };
        }
        let (bank0, bank1) = self.prg_16k_banks();
        let bank = if address < 0xC000 { bank0 } else { bank1 };
        let offset = bank * 0x4000 + (address as usize & 0x3FFF);
        FetchResult {
            data: cart.prg_rom[offset % cart.prg_rom.len()],
            driven: true,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            return;
        }
        let rom_value = if !cart.prg_rom.is_empty() {
            let (bank0, bank1) = self.prg_16k_banks();
            let bank = if address < 0xC000 { bank0 } else { bank1 };
            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            cart.prg_rom[offset % cart.prg_rom.len()]
        } else {
            0
        };

        let new_value = if (self.latch & 8) != 0 {
            (data & rom_value & !15) | (rom_value & 15)
        } else {
            data & rom_value
        };
        self.latch = new_value;
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v(cart.nametable_horizontal_mirroring, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;

        if address < 0x2000 {
            let bank = self.latch as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let byte = if using_chr_ram || chr_rom.is_empty() {
                if !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else {
                    0
                }
            } else {
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mirror_h_or_v(nametable_horizontal_mirroring, address);
            let idx = (mirrored & 0x7FF) as usize;
            new_addr_bus |= vram[idx] as u16;
        }

        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let bank = self.latch as usize;
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.latch]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start < state.len() {
            self.latch = state[start];
            start += 1;
        }
        start
    }
}

