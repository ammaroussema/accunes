use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::nrom::mirror_address;
use crate::mappers::vrc7::Vrc7;

pub struct Mapper515 {
    latch: u8,
    adc_data: u8,
    vrc7: Vrc7,
}

impl Mapper515 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self {
            latch: 0,
            adc_data: 0,
            vrc7: Vrc7::new(0),
        }
    }

    fn switchable_bank(&self, cart: &Cartridge) -> usize {
        let total_16k = cart.prg_rom.len() / 0x4000;
        if (self.latch & 0x80) == 0 {
            (self.latch & 0x3F) as usize
        } else {
            let offset = total_16k.saturating_sub(64);
            ((self.latch & 0x3F) as usize).wrapping_add(offset)
        }
    }
}

impl Mapper for Mapper515 {
    fn reset(&mut self) {
        self.latch = 0;
        self.adc_data = 0;
        self.vrc7.reset();
    }

    fn reset_power_cycle(&mut self) {
        self.latch = 0;
        self.adc_data = 0;
        self.vrc7.reset_power_cycle();
    }

    fn set_cpu_clock(&mut self, clock: f64) {
        self.vrc7.set_audio_clock(clock);
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        match address {
            0x6003 => {
                let val = self.adc_data & 0x80;
                self.adc_data = self.adc_data.wrapping_shl(1);
                FetchResult {
                    data: val,
                    driven: true,
                }
            }
            0x6000..=0x7FFF => FetchResult {
                data: 0,
                driven: false,
            },
            0x8000..=0xBFFF => {
                let len = cart.prg_rom.len();
                if len == 0 {
                    return FetchResult {
                        data: 0,
                        driven: true,
                    };
                }
                let bank = self.switchable_bank(cart);
                let offset = (bank * 0x4000 + (address as usize & 0x3FFF)) % len;
                FetchResult {
                    data: cart.prg_rom[offset],
                    driven: true,
                }
            }
            0xC000..=0xFFFF => {
                let len = cart.prg_rom.len();
                if len == 0 {
                    return FetchResult {
                        data: 0,
                        driven: true,
                    };
                }
                let last_bank = (len / 0x4000).saturating_sub(1);
                let offset = last_bank * 0x4000 + (address as usize & 0x3FFF);
                FetchResult {
                    data: cart.prg_rom[offset % len],
                    driven: true,
                }
            }
            _ => FetchResult {
                data: 0,
                driven: false,
            },
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        match address {
            0x6000..=0x6FFF => {
                self.vrc7.write_sound((address & 1) != 0, data);
            }
            0x8000..=0xBFFF | 0xD000..=0xFFFF => {
                self.latch = data;
            }
            0xC000..=0xCFFF => match address & 0x0FFF {
                2 => {
                    self.adc_data = 0;
                }
                3 => {}
                _ => {}
            },
            _ => {}
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

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.vrc7.clock_audio(cycles);
        false
    }

    fn audio_sample(&self) -> f32 {
        self.vrc7.get_audio_sample() * 2.0
    }

    fn get_dip_switches(&self) -> u8 {
        0
    }

    fn set_dip_switches(&mut self, _value: u8) {}

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = self.vrc7.save_mapper_registers(_cart);
        state.push(self.latch);
        state.push(self.adc_data);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.vrc7.load_mapper_registers(cart, state, start);
        if p < state.len() {
            self.latch = state[p];
            p += 1;
        }
        if p < state.len() {
            self.adc_data = state[p];
            p += 1;
        }
        p
    }
}
