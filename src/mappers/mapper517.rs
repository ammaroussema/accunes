use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::nrom::mirror_address;

pub struct Mapper517 {
    latch: u8,
    adc_data: i32,
    adc_high: i32,
    adc_low: i32,
    state: u8,
}

impl Mapper517 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self {
            latch: 0,
            adc_data: 0,
            adc_high: 0,
            adc_low: 0x40,
            state: 0,
        }
    }
}

impl Mapper for Mapper517 {
    fn reset(&mut self) {
        self.latch = 0;
        self.adc_data = 0;
        self.adc_high = 0;
        self.adc_low = 0x40;
        self.state = 0;
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            let bank = if address < 0xC000 {
                self.latch as usize
            } else {
                (len / 0x4000).saturating_sub(1)
            };
            let offset = (bank * 0x4000 + (address as usize & 0x3FFF)) % len;
            FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            }
        } else if (0x6000..0x8000).contains(&address) {
            let result = if (address & 0x0FFF) == 0 {
                match self.state {
                    0 => {
                        self.state = 1;
                        0
                    }
                    1 => {
                        self.state = 2;
                        1
                    }
                    _ => {
                        if self.adc_low > 0 {
                            self.adc_low -= 1;
                            1
                        } else {
                            self.state = 0;
                            0
                        }
                    }
                }
            } else if self.adc_high > 0 {
                self.adc_high -= 1;
                0
            } else {
                1
            };
            FetchResult {
                data: result,
                driven: true,
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.adc_data = 0;
            self.adc_high = self.adc_data >> 2;
            self.adc_low = 0x40 - self.adc_high - ((self.adc_data & 3) << 2);
            self.state = 0;
            self.latch = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        mirror_address(
            cart.alternative_nametable_arrangement,
            cart.nametable_horizontal_mirroring,
            address,
        )
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        _using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let offset = address as usize & 0x1FFF;
            let byte = if !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mirror_address(
                alternative_nametable_arrangement,
                nametable_horizontal_mirroring,
                address,
            );
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() {
                    prg_vram[idx]
                } else {
                    0
                }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let offset = address as usize & 0x1FFF;
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

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.latch);
        state.extend_from_slice(&self.adc_data.to_le_bytes());
        state.extend_from_slice(&self.adc_high.to_le_bytes());
        state.extend_from_slice(&self.adc_low.to_le_bytes());
        state.push(self.state);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.latch = state[p];
            p += 1;
        }
        if p + 4 <= state.len() {
            self.adc_data = i32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
            p += 4;
        }
        if p + 4 <= state.len() {
            self.adc_high = i32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
            p += 4;
        }
        if p + 4 <= state.len() {
            self.adc_low = i32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
            p += 4;
        }
        if p < state.len() {
            self.state = state[p];
            p += 1;
        }
        p
    }
}
