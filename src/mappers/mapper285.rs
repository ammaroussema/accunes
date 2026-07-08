use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper285 {
    latch: u8,
    submapper: u8,
}

impl Mapper285 {
    pub fn new(submapper_id: u8) -> Self {
        Self { latch: 0, submapper: submapper_id }
    }

    fn mirror_address(&self, address: u16) -> u16 {
        if self.latch & 0x80 != 0 {
            if self.latch & 0x20 != 0 {
                (address & 0x33FF) | 0x0400
            } else {
                address & 0x33FF
            }
        } else if self.latch & 0x20 != 0 {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }
}

impl Mapper for Mapper285 {
    fn reset(&mut self) {
        self.latch = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult { data: 0, driven: true };
            }
            if self.latch & 0x40 != 0 {
                let bank = if self.submapper == 1 {
                    ((self.latch as usize >> 1) & 3) | ((self.latch as usize >> 2) & !3)
                } else {
                    self.latch as usize >> 1
                };
                let num_32k = len / 0x8000;
                let bank = if num_32k > 0 { bank % num_32k } else { 0 };
                let offset = bank * 0x8000 + (address as usize & 0x7FFF);
                FetchResult { data: cart.prg_rom[offset % len], driven: true }
            } else {
                let num_16k = len / 0x4000;
                if num_16k == 0 {
                    return FetchResult { data: 0, driven: true };
                }
                let (b0, b1) = if self.submapper == 1 {
                    let b = ((self.latch as usize >> 1) & !7) | (self.latch as usize & 7);
                    (b, ((self.latch as usize >> 1) & !7) | 7)
                } else {
                    (self.latch as usize, self.latch as usize | 7)
                };
                let bank = if address < 0xC000 { b0 } else { b1 } % num_16k;
                let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                FetchResult { data: cart.prg_rom[offset % len], driven: true }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.latch = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror_address(address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = if !chr_ram.is_empty() {
                chr_ram[address as usize & 0x1FFF]
            } else if !chr_rom.is_empty() {
                chr_rom[address as usize & 0x1FFF]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = self.mirror_address(address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                cart.chr_ram[address as usize & 0x1FFF] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_address(address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.latch]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start < state.len() {
            self.latch = state[start];
            start + 1
        } else { start }
    }
}
