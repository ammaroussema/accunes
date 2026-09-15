use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MirroringMode {
    HolyDiver = 0,
    SingleScreen = 1,
}

pub struct Mapper78 {
    prg_bank: u8,
    chr_bank: u8,
    mirroring_mode: MirroringMode,
    mirroring_vertical: bool,
    mirroring_b: bool,
}

impl Mapper78 {
    pub fn new(submapper: u8, ines_alt_nametables: bool) -> Self {
        let mirroring_mode = match submapper {
            3 => MirroringMode::HolyDiver,
            1 => MirroringMode::SingleScreen,
            _ => {
                if ines_alt_nametables {
                    MirroringMode::HolyDiver
                } else {
                    MirroringMode::SingleScreen
                }
            }
        };
        Self {
            prg_bank: 0,
            chr_bank: 0,
            mirroring_mode,
            mirroring_vertical: false,
            mirroring_b: false,
        }
    }

    fn prg_rom_byte(cart: &Cartridge, prg_bank: u8, address: u16) -> u8 {
        let num_16k = cart.prg_rom.len() / 0x4000;
        if num_16k == 0 {
            return 0;
        }
        let bank = if address >= 0xC000 {
            num_16k - 1
        } else {
            prg_bank as usize % num_16k
        };
        let offset = bank * 0x4000 + (address as usize & 0x3FFF);
        cart.prg_rom[offset % cart.prg_rom.len()]
    }

    fn mirror_address(&self, address: u16) -> u16 {
        match self.mirroring_mode {
            MirroringMode::HolyDiver => {
                if self.mirroring_vertical {
                    address & 0x37FF
                } else {
                    (address & 0x33FF) | ((address & 0x0800) >> 1)
                }
            }
            MirroringMode::SingleScreen => {
                if self.mirroring_b {
                    (address & 0x33FF) | 0x0400
                } else {
                    address & 0x33FF
                }
            }
        }
    }
}

impl Mapper for Mapper78 {
    fn reset(&mut self) {
        self.prg_bank = 0;
        self.chr_bank = 0;
        self.mirroring_vertical = false;
        self.mirroring_b = false;
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

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            return;
        }
        let rom_byte = Self::prg_rom_byte(cart, self.prg_bank, address);
        let data = data & rom_byte;
        self.prg_bank = data & 0x07;
        self.chr_bank = (data >> 4) & 0x0F;
        let mirroring_bit = (data & 0x08) != 0;
        match self.mirroring_mode {
            MirroringMode::HolyDiver => self.mirroring_vertical = mirroring_bit,
            MirroringMode::SingleScreen => self.mirroring_b = mirroring_bit,
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
            let offset = self.chr_bank as usize * 0x2000 + (address as usize & 0x1FFF);
            let byte = if using_chr_ram {
                if chr_ram.is_empty() {
                    0
                } else {
                    chr_ram[offset % chr_ram.len()]
                }
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

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![
            self.prg_bank,
            self.chr_bank,
            self.mirroring_mode as u8,
            if self.mirroring_vertical { 1 } else { 0 },
            if self.mirroring_b { 1 } else { 0 },
        ]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if state.len() >= start + 5 {
            self.prg_bank = state[start];
            self.chr_bank = state[start + 1];
            self.mirroring_mode = if state[start + 2] == 0 {
                MirroringMode::HolyDiver
            } else {
                MirroringMode::SingleScreen
            };
            self.mirroring_vertical = state[start + 3] != 0;
            self.mirroring_b = state[start + 4] != 0;
            start + 5
        } else {
            start
        }
    }
}
