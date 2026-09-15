use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper59 {
    prg_bank: u8,
    prg_mode_16k: bool,
    chr_bank: u8,
    mirror_horizontal: bool,
    return_dip_switch: bool,
    dip_switches: u8, 
}

impl Mapper59 {
    pub fn new() -> Self {
        Self {
            prg_bank: 0,
            prg_mode_16k: false,
            chr_bank: 0,
            mirror_horizontal: false,
            return_dip_switch: false,
            dip_switches: 0, 
        }
    }
}

impl Mapper for Mapper59 {
    fn reset(&mut self) {
        self.prg_bank = 0;
        self.prg_mode_16k = false;
        self.chr_bank = 0;
        self.mirror_horizontal = false;
        self.return_dip_switch = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if self.return_dip_switch {
                return FetchResult {
                    data: self.dip_switches,
                    driven: true,
                };
            }
            let num_16k_banks = cart.prg_rom.len() / 0x4000;
            if num_16k_banks == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let slot = ((address as usize - 0x8000) >> 14) & 1; 
            let bank = if self.prg_mode_16k {
                self.prg_bank as usize % num_16k_banks
            } else {
                let base = (self.prg_bank as usize & 0x06) % num_16k_banks;
                base + slot
            };
            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, _data: u8) {
        if address >= 0x8000 {
            self.prg_bank = ((address & 0x70) >> 4) as u8;
            self.prg_mode_16k = (address & 0x80) != 0;
            self.chr_bank = (address & 0x07) as u8;
            self.mirror_horizontal = (address & 0x08) != 0;
            self.return_dip_switch = (address & 0x0100) != 0;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mirror_horizontal {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let offset = (self.chr_bank as usize * 0x2000) + (address as usize & 0x1FFF);
            if using_chr_ram {
                let len = chr_ram.len();
                if len > 0 {
                    new_addr_bus |= chr_ram[offset % len] as u16;
                }
            } else {
                let len = chr_rom.len();
                if len > 0 {
                    new_addr_bus |= chr_rom[offset % len] as u16;
                }
            }
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if self.mirror_horizontal {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() { prg_vram[idx] } else { 0 }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let offset = (self.chr_bank as usize * 0x2000) + (address as usize & 0x1FFF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
            }
        }
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![
            self.prg_bank,
            self.prg_mode_16k as u8,
            self.chr_bank,
            self.mirror_horizontal as u8,
            self.return_dip_switch as u8,
            self.dip_switches,
        ]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 6 <= state.len() {
            self.prg_bank = state[start]; start += 1;
            self.prg_mode_16k = state[start] != 0; start += 1;
            self.chr_bank = state[start]; start += 1;
            self.mirror_horizontal = state[start] != 0; start += 1;
            self.return_dip_switch = state[start] != 0; start += 1;
            self.dip_switches = state[start]; start += 1;
        }
        start
    }
}
