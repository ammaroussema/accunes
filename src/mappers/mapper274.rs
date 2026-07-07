use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper274 {
    reg: [u8; 2],
}

impl Mapper274 {
    pub fn new() -> Self {
        Self { reg: [0; 2] }
    }

    fn prg_bank0(&self, num_16k: usize) -> usize {
        if num_16k == 0 {
            return 0;
        }
        if self.reg[1] & 0x80 != 0 {
            ((self.reg[1] as usize & 0x70) | (self.reg[0] as usize & 0x0F)) % num_16k
        } else {
            let mask = (num_16k - 1) & 0x0F;
            (0x80 | (self.reg[0] as usize & mask)) % num_16k
        }
    }

    fn prg_bank1(&self, num_16k: usize) -> usize {
        if num_16k == 0 {
            return 0;
        }
        (self.reg[1] as usize & 0x7F) % num_16k
    }

    fn mirror_horizontal(&self) -> bool {
        self.reg[0] & 0x10 != 0
    }

    fn mirror_address(&self, address: u16) -> u16 {
        if self.mirror_horizontal() {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }
}

impl Mapper for Mapper274 {
    fn reset(&mut self) {
        self.reg = [0; 2];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let num_16k = cart.prg_rom.len() / 0x4000;
            if num_16k == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let bank = match address {
                0x8000..=0xBFFF => self.prg_bank0(num_16k),
                0xC000..=0xFFFF => self.prg_bank1(num_16k),
                _ => 0,
            };
            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            if address < 0xC000 {
                self.reg[0] = data;
            } else {
                self.reg[1] = data & 0x7F | if address >= 0xE000 { 0x80 } else { 0 };
            }
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
            } else {
                0
            };
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
        vec![self.reg[0], self.reg[1]]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start + 2 <= state.len() {
            self.reg[0] = state[start];
            self.reg[1] = state[start + 1];
            start + 2
        } else {
            start
        }
    }
}
