use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper627 {
    latch: u16,
}

impl Mapper627 {
    pub fn new() -> Self {
        Self { latch: 0 }
    }

    fn prg_offset(&self, cart: &Cartridge, address: u16) -> usize {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }
        let offset = if (self.latch & 0x40) != 0 {
            ((self.latch >> 3) as usize) * 0x8000 + (address as usize & 0x7FFF)
        } else {
            ((self.latch >> 2) as usize) * 0x4000 + (address as usize & 0x3FFF)
        };
        offset % len
    }

    fn chr_offset(&self, address: u16) -> usize {
        let bank = (((self.latch >> 1) & 0x1C) | (self.latch & 0x03)) as usize;
        bank * 0x2000 + (address as usize & 0x1FFF)
    }

    fn mirror_horizontal(&self) -> bool {
        (self.latch & 0x80) != 0
    }
}

impl Mapper for Mapper627 {
    fn reset(&mut self) {
        self.latch = 0;
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            let len = cart.prg_ram.len();
            if len > 0 {
                let offset = (address as usize & 0x1FFF) % len;
                return FetchResult { data: cart.prg_ram[offset], driven: true };
            }
            return FetchResult { data: 0, driven: false };
        }
        if address < 0x8000 || cart.prg_rom.is_empty() {
            return FetchResult { data: 0, driven: false };
        }
        let offset = self.prg_offset(cart, address);
        FetchResult {
            data: cart.prg_rom[offset],
            driven: true,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            let len = cart.prg_ram.len();
            if len > 0 {
                let offset = (address as usize & 0x1FFF) % len;
                cart.prg_ram[offset] = data;
            }
            return;
        }
        if address >= 0x8000 {
            self.latch = address;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        crate::mapper::mirror_h_or_v(self.mirror_horizontal(), address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;

        if address < 0x2000 {
            let offset = self.chr_offset(address);
            let byte = if using_chr_ram {
                if !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else {
                    0
                }
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = crate::mapper::mirror_h_or_v(self.mirror_horizontal(), address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }

        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let offset = self.chr_offset(address);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        self.latch.to_le_bytes().to_vec()
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 2 <= state.len() {
            self.latch = u16::from_le_bytes([state[start], state[start + 1]]);
            start += 2;
        }
        start
    }
}
