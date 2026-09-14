use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

fn bit_reverse6(v: u8) -> u8 {
    ((v & 1) << 5) | ((v & 2) << 3) | ((v & 4) << 1)
        | ((v & 8) >> 1) | ((v & 0x10) >> 3) | ((v & 0x20) >> 5)
}
const PROT_LUT: [u8; 8] = [0x83, 0x83, 0x42, 0x00, 0x00, 0x02, 0x02, 0x03];

pub struct Mapper121 {
    mmc3: MapperMMC3,
    prg: [u8; 3],
    a18: u8,
    lut_index: usize,
    prot_index: u8,
    prot_value: u8,
    a9713: bool,
}

impl Mapper121 {
    pub fn new(config: Mmc3Config, prg_size_bytes: usize, _chr_size_bytes: usize) -> Self {
        Self {
            mmc3: MapperMMC3::new(config),
            prg: [0; 3],
            a18: 0,
            lut_index: 0,
            prot_index: 0,
            prot_value: 0,
            a9713: prg_size_bytes > 256 * 1024,
        }
    }

    fn chr_ext_bit(&self, address: u16) -> u16 {
        if self.a9713 {
            if (self.a18 & 0x80) != 0 { 0x100 } else { 0 }
        } else {
            if (address & 0x1000) != 0 { 0x100 } else { 0 }
        }
    }

    fn read_chr(&self, chr_rom: &[u8], chr_ram: &[u8], bank: usize, address: u16) -> u8 {
        let offset = bank * 0x0400 + (address as usize & 0x03FF);
        if !chr_rom.is_empty() {
            chr_rom[offset % chr_rom.len()]
        } else if !chr_ram.is_empty() {
            chr_ram[offset % chr_ram.len()]
        } else {
            0
        }
    }
}

impl Mapper for Mapper121 {
    fn reset(&mut self) {
        self.prg = [0; 3];
        self.a18 = 0;
        self.lut_index = 0;
        self.prot_index = 0;
        self.prot_value = 0;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x5000 && address <= 0x5FFF {
            return FetchResult {
                data: PROT_LUT[self.lut_index & 7],
                driven: true,
            };
        }
        if address >= 0x8000 {
            let prg_len = cart.prg_rom.len();
            if prg_len == 0 {
                return FetchResult { data: 0, driven: false };
            }
            let a18_offset = (self.a18 as usize) >> 2;
            let bank = if (self.prot_index & 0x20) != 0 {
                match address {
                    0x8000..=0x9FFF => {
                        if (self.mmc3.r8000 & 0x40) == 0 {
                            (self.mmc3.bank_8c as usize & 0x1F) | a18_offset
                        } else {
                            0x1E | a18_offset
                        }
                    }
                    0xA000..=0xBFFF => (self.prg[0] as usize & 0x1F) | a18_offset,
                    0xC000..=0xDFFF => (self.prg[1] as usize & 0x1F) | a18_offset,
                    _ => (self.prg[2] as usize & 0x1F) | a18_offset,
                }
            } else {
                match address {
                    0x8000..=0x9FFF => {
                        if (self.mmc3.r8000 & 0x40) == 0 {
                            (self.mmc3.bank_8c as usize & 0x1F) | a18_offset
                        } else {
                            0x1E | a18_offset
                        }
                    }
                    0xA000..=0xBFFF => (self.mmc3.bank_a as usize & 0x1F) | a18_offset,
                    0xC000..=0xDFFF => {
                        if (self.mmc3.r8000 & 0x40) != 0 {
                            (self.mmc3.bank_8c as usize & 0x1F) | a18_offset
                        } else {
                            0x1E | a18_offset
                        }
                    }
                    _ => 0x1F | a18_offset,
                }
            };
            let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % prg_len;
            return FetchResult {
                data: cart.prg_rom[offset],
                driven: true,
            };
        }
        self.mmc3.fetch_prg(cart, address)
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address <= 0x5FFF {
            let addr_off = address & 0xFFF;
            self.lut_index = (data as usize & 3) | (((addr_off >> 6) & 4) as usize);
            if (addr_off & 0x100) != 0 {
                self.a18 = data & 0x80;
            }
            return;
        }
        if address >= 0x8000 && address <= 0x9FFF {
            match address & 3 {
                1 => {
                    self.prot_value = bit_reverse6(data & 0x3F);
                    if self.prot_index == 0x26 || self.prot_index == 0x28 || self.prot_index == 0x2A {
                        let idx = 0x15usize.saturating_sub(self.prot_index as usize >> 1);
                        if idx < 3 {
                            self.prg[idx] = self.prot_value;
                        }
                    }
                }
                3 => {
                    self.prot_index = data & 0x3F;
                    if (self.prot_index & 0x20) != 0 && self.prot_value != 0 {
                        self.prg[2] = self.prot_value;
                    }
                }
                _ => {}
            }
            self.mmc3.store_prg(cart, address, data);
            return;
        }
        self.mmc3.store_prg(cart, address, data);
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mmc3.mirror_nametable(cart, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let bank = self.mmc3.chr_bank(address) as usize | self.chr_ext_bit(address) as usize;
            let byte = self.read_chr(chr_rom, chr_ram, bank, address);
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if self.mmc3.nametable_mirroring() {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x07FF) as usize;
                if idx < prg_vram.len() { prg_vram[idx] } else { 0 }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        self.mmc3.store_ppu(cart, address, data, vram);
    }

    fn needs_cpu_clock(&self) -> bool { true }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.mmc3.cpu_clock(_cycles)
    }

    fn needs_ppu_clock(&self) -> bool { true }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        ppu_sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.extend_from_slice(&self.prg);
        state.push(self.a18);
        state.extend_from_slice(&(self.lut_index as u32).to_le_bytes());
        state.push(self.prot_index);
        state.push(self.prot_value);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let next = self.mmc3.load_mapper_registers(cart, state, start);
        if next + 3 <= state.len() {
            self.prg.copy_from_slice(&state[next..next + 3]);
        }
        if next + 4 <= state.len() {
            self.a18 = state[next + 3];
        }
        if next + 8 <= state.len() {
            self.lut_index = u32::from_le_bytes(state[next + 4..next + 8].try_into().unwrap_or([0; 4])) as usize;
        }
        if next + 9 <= state.len() {
            self.prot_index = state[next + 8];
        }
        if next + 10 <= state.len() {
            self.prot_value = state[next + 9];
        }
        next + 10
    }
}
