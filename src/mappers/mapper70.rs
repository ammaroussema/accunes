use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper70 {
    prg_bank: u8,
    chr_bank: u8,
    enable_mirroring_control: bool,
    mirroring_b: bool,
    mirroring_vertical: bool,
}

impl Mapper70 {
    pub fn new(_header_horizontal_mirror: bool, enable_mirroring_control: bool) -> Self {
        Self {
            prg_bank: 0,
            chr_bank: 0,
            enable_mirroring_control,
            mirroring_b: false,
            mirroring_vertical: true,
        }
    }

    fn mirror_address(&self, address: u16) -> u16 {
        if self.enable_mirroring_control {
            if self.mirroring_b {
                (address & 0x33FF) | 0x0400 
            } else {
                address & 0x33FF 
            }
        } else {
            if self.mirroring_vertical {
                address & 0x37FF
            } else {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            }
        }
    }
}

impl Mapper for Mapper70 {
    fn reset(&mut self) {
        self.prg_bank = 0;
        self.chr_bank = 0;
        self.mirroring_b = false;
        self.mirroring_vertical = true;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }
        let num_16k = cart.prg_rom.len() / 0x4000;
        if num_16k == 0 {
            return FetchResult { data: 0, driven: true };
        }
        let bank = if address >= 0xC000 {
            num_16k - 1
        } else {
            self.prg_bank as usize % num_16k
        };
        let offset = bank * 0x4000 + (address as usize & 0x3FFF);
        FetchResult {
            data: cart.prg_rom[offset % cart.prg_rom.len()],
            driven: true,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            return;
        }
        let mirroring_bit = (data & 0x80) != 0;
        if mirroring_bit {
            self.enable_mirroring_control = true;
        }
        if self.enable_mirroring_control {
            self.mirroring_b = mirroring_bit;
        }
        self.prg_bank = (data >> 4) & 0x07;
        self.chr_bank = data & 0x0F;
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
            let offset = self.chr_bank as usize * 0x2000 + (address as usize & 0x1FFF);
            let byte = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[offset % chr_ram.len()] }
            } else if chr_rom.is_empty() {
                0
            } else {
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = self.mirror_address(address);
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let offset = self.chr_bank as usize * 0x2000 + (address as usize & 0x1FFF);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_address(address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![
            self.prg_bank,
            self.chr_bank,
            if self.enable_mirroring_control { 1 } else { 0 },
            if self.mirroring_b { 1 } else { 0 },
            if self.mirroring_vertical { 1 } else { 0 },
        ]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if state.len() >= start + 5 {
            self.prg_bank = state[start];
            self.chr_bank = state[start + 1];
            self.enable_mirroring_control = state[start + 2] != 0;
            self.mirroring_b = state[start + 3] != 0;
            self.mirroring_vertical = state[start + 4] != 0;
            start + 5
        } else {
            start
        }
    }
}
