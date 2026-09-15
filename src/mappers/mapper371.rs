
use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper371 {
    prg: u8,
    rom512: bool,
    vertical_mirroring: bool,
    chr1bpp: bool,
    last_nt_addr: u16,
}

impl Mapper371 {
    pub fn new() -> Self {
        let mut m = Mapper371 {
            prg: 0,
            rom512: false,
            vertical_mirroring: false,
            chr1bpp: false,
            last_nt_addr: 0,
        };
        m.reset();
        m
    }

    #[inline]
    fn switchable_prg_bank(&self) -> usize {
        if self.rom512 {
            (self.prg as usize).wrapping_add(4)
        } else {
            (self.prg & 0x03) as usize
        }
    }

    #[inline]
    fn fixed_prg_bank(&self) -> usize {
        if self.rom512 {
            (self.prg as usize).wrapping_add(4)
        } else {
            3
        }
    }

    #[inline]
    fn intercept_chr_addr(&self, chr_addr: u16) -> usize {
        if !self.chr1bpp {
            return chr_addr as usize;
        }
        let mut bank = (chr_addr >> 10) as u16;
        let mut addr = chr_addr & 0x3FF;       

        addr &= !0x08;
        addr |= if (self.last_nt_addr & 0x001) != 0 { 0x08 } else { 0x00 };

        bank &= !0x04;
        bank |= if (self.last_nt_addr & 0x200) != 0 { 0x04 } else { 0x00 };

        ((bank as usize) * 0x400) | (addr as usize)
    }

    #[inline]
    fn mirror_nt(&self, address: u16) -> u16 {
        if self.vertical_mirroring {
            address & 0x37FF
        } else {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        }
    }
}

impl Mapper for Mapper371 {
    fn reset(&mut self) {
        self.prg = 0;
        self.rom512 = false;
        self.vertical_mirroring = false;
        self.chr1bpp = false;
        self.last_nt_addr = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        match address {
            0x6000..=0x7FFF => {
                if !cart.prg_ram.is_empty() {
                    let offset = (address as usize & 0x1FFF) % cart.prg_ram.len();
                    FetchResult { data: cart.prg_ram[offset], driven: true }
                } else {
                    FetchResult { data: 0, driven: false }
                }
            }
            0x8000..=0xBFFF => {
                let len = cart.prg_rom.len().max(1);
                let bank = self.switchable_prg_bank();
                let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                FetchResult { data: cart.prg_rom[offset % len], driven: true }
            }
            0xC000..=0xFFFF => {
                let len = cart.prg_rom.len().max(1);
                let bank = self.fixed_prg_bank();
                let offset = bank * 0x4000 + (address as usize & 0x3FFF);
                FetchResult { data: cart.prg_rom[offset % len], driven: true }
            }
            0x5000..=0x5FFF => {
                let reg = (address >> 8) & 0x07;
                if reg == 5 {
                    FetchResult { data: 0x00, driven: true }
                } else {
                    FetchResult { data: 0, driven: false }
                }
            }
            _ => FetchResult { data: 0, driven: false },
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        match address {
            0x6000..=0x7FFF => {
                if !cart.prg_ram.is_empty() {
                    let offset = (address as usize & 0x1FFF) % cart.prg_ram.len();
                    cart.prg_ram[offset] = data;
                }
            }
            0x5000..=0x5FFF => {
                let reg = (address >> 8) & 0x07;
                match reg {
                    0 => {
                        self.prg = (self.prg & 0x10) | (data & 0x0F);
                        self.rom512 = (data & 0x70) == 0x50;
                        self.chr1bpp = (data & 0x80) != 0;
                    }
                    1 => {
                        self.prg = (self.prg & 0x0F) | if (data & 0x01) != 0 { 0x10 } else { 0x00 };
                        self.vertical_mirroring = (data & 0x02) != 0;
                    }
                    2 => {

                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else {
            self.mirror_nt(address)
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;

        if address < 0x2000 {
            if chr_ram.is_empty() {
                return (0, new_addr_bus);
            }
            let actual_offset = self.intercept_chr_addr(address) % chr_ram.len();
            let byte = chr_ram[actual_offset];
            new_addr_bus |= byte as u16;
            (byte, new_addr_bus)
        } else if address < 0x3F00 {

            let nt_offset = address & 0x3FF;
            if nt_offset < 0x3C0 {
                self.last_nt_addr = address;
            }

            let mirrored = if alternative_nametable_arrangement {
                address
            } else if nametable_horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let byte = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= byte as u16;
            (byte, new_addr_bus)
        } else {
            (0, new_addr_bus)
        }
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let actual_offset = self.intercept_chr_addr(address) % cart.chr_ram.len();
                cart.chr_ram[actual_offset] = data;
            }
        } else if address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.chr_ram);
        state.extend_from_slice(&cart.prg_ram);
        state.push(self.prg);
        state.push(self.rom512 as u8);
        state.push(self.vertical_mirroring as u8);
        state.push(self.chr1bpp as u8);
        state.extend_from_slice(&self.last_nt_addr.to_le_bytes());
        state
    }

    fn load_mapper_registers(
        &mut self,
        cart: &mut Cartridge,
        state: &[u8],
        mut start: usize,
    ) -> usize {
        let chr_len = cart.chr_ram.len();
        if start + chr_len <= state.len() {
            cart.chr_ram.copy_from_slice(&state[start..start + chr_len]);
            start += chr_len;
        }
        let prg_len = cart.prg_ram.len();
        if start + prg_len <= state.len() {
            cart.prg_ram.copy_from_slice(&state[start..start + prg_len]);
            start += prg_len;
        }
        if start < state.len() {
            self.prg = state[start];
            start += 1;
        }
        if start < state.len() {
            self.rom512 = state[start] != 0;
            start += 1;
        }
        if start < state.len() {
            self.vertical_mirroring = state[start] != 0;
            start += 1;
        }
        if start < state.len() {
            self.chr1bpp = state[start] != 0;
            start += 1;
        }
        if start + 1 < state.len() {
            self.last_nt_addr = u16::from_le_bytes([state[start], state[start + 1]]);
            start += 2;
        }
        start
    }
}
