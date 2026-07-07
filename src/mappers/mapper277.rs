use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper277 {
    latch: u8,
}

impl Mapper277 {
    pub fn new() -> Self {
        Self { latch: 0x08 }
    }

    fn nrom(&self) -> bool {
        self.latch & 0x08 != 0
    }

    fn cpu_a14(&self) -> bool {
        self.latch & 0x01 == 0 && self.nrom()
    }

    fn prg_val(&self) -> u8 {
        self.latch & 0x0F
    }

    fn mirror_h(&self) -> bool {
        self.latch & 0x10 != 0
    }

    fn mirror_address(&self, address: u16) -> u16 {
        if self.mirror_h() {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }
}

impl Mapper for Mapper277 {
    fn reset(&mut self) {
        self.latch = 0x08;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let num_16k = cart.prg_rom.len() / 0x4000;
            if num_16k == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let bank = if address < 0xC000 {
                let b = self.prg_val() & !(self.cpu_a14() as u8);
                b as usize % num_16k
            } else {
                let b = self.prg_val()
                    | (self.cpu_a14() as u8)
                    | ((!self.nrom() as u8) * 7);
                b as usize % num_16k
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
        if address >= 0x8000 && self.latch & 0x20 == 0 {
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
            if using_chr_ram && !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[address as usize & 0x1FFF] as u16;
            } else if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[address as usize & 0x1FFF] as u16;
            }
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
        } else {
            start
        }
    }
}
