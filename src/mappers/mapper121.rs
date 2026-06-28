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
    expregs: [u8; 8],
    a9713: bool,
}

impl Mapper121 {
    pub fn new(config: Mmc3Config, prg_size_bytes: usize, _chr_size_bytes: usize) -> Self {
        Self {
            mmc3: MapperMMC3::new(config),
            expregs: [0; 8],
            a9713: prg_size_bytes > 256 * 1024,
        }
    }

    fn a18(&self) -> u8 {
        (self.expregs[3] & 0x80) >> 2
    }

    fn sync(&mut self) {
        match self.expregs[5] & 0x3F {
            0x20 | 0x29 | 0x2B | 0x3C | 0x3F => {
                self.expregs[7] = 1;
                self.expregs[0] = self.expregs[6];
            }
            0x26 => {
                self.expregs[7] = 0;
                self.expregs[0] = self.expregs[6];
            }
            0x2C => {
                self.expregs[7] = 1;
                if self.expregs[6] != 0 {
                    self.expregs[0] = self.expregs[6];
                }
            }
            0x28 => {
                self.expregs[7] = 0;
                self.expregs[1] = self.expregs[6];
            }
            0x2A => {
                self.expregs[7] = 0;
                self.expregs[2] = self.expregs[6];
            }
            0x2F => {}
            _ => {
                self.expregs[5] = 0;
            }
        }
    }

    fn prg_offset(&self, _cart: &Cartridge, bank: u8, address: u16) -> usize {
        let bank = (bank as usize) | (self.a18() as usize);
        bank * 0x2000 + (address as usize & 0x1FFF)
    }

    fn read_prg(&self, cart: &Cartridge, offset: usize) -> u8 {
        let len = cart.prg_rom.len();
        if len == 0 { 0 } else { cart.prg_rom[offset % len] }
    }

    fn chr_ext_bit(&self, address: u16) -> u16 {
        if self.a9713 {
            if (self.expregs[3] & 0x80) != 0 { 0x100 } else { 0 }
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

    fn fixed_last(&self, cart: &Cartridge, address: u16) -> usize {
        let len = cart.prg_rom.len();
        if len == 0 { return 0; }
        let bank = (len / 0x2000).saturating_sub(1);
        bank * 0x2000 + (address as usize & 0x1FFF)
    }

    fn fixed_second_last(&self, cart: &Cartridge, address: u16) -> usize {
        let len = cart.prg_rom.len();
        if len < 0x4000 { return 0; }
        let bank = (len / 0x2000).saturating_sub(2);
        bank * 0x2000 + (address as usize & 0x1FFF)
    }
}

impl Mapper for Mapper121 {
    fn reset(&mut self) {
        self.expregs = [0; 8];
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x5000 && address <= 0x5FFF {
            return FetchResult {
                data: self.expregs[4],
                driven: true,
            };
        }
        let is_protected = (self.expregs[5] & 0x3F) != 0;
        if address >= 0xE000 {
            if is_protected {
                let offset = self.prg_offset(cart, self.expregs[0], address);
                return FetchResult { data: self.read_prg(cart, offset), driven: true };
            }
            return FetchResult { data: self.read_prg(cart, self.fixed_last(cart, address)), driven: true };
        }
        if address >= 0xC000 {
            if is_protected {
                let offset = self.prg_offset(cart, self.expregs[1], address);
                return FetchResult { data: self.read_prg(cart, offset), driven: true };
            }
            if (self.mmc3.r8000 & 0x40) != 0 {
                let offset = self.prg_offset(cart, self.mmc3.bank_8c, address);
                return FetchResult { data: self.read_prg(cart, offset), driven: true };
            }
            return FetchResult { data: self.read_prg(cart, self.fixed_second_last(cart, address)), driven: true };
        }
        if address >= 0xA000 {
            if is_protected {
                let offset = self.prg_offset(cart, self.expregs[2], address);
                return FetchResult { data: self.read_prg(cart, offset), driven: true };
            }
            let offset = self.prg_offset(cart, self.mmc3.bank_a, address);
            return FetchResult { data: self.read_prg(cart, offset), driven: true };
        }
        if address >= 0x8000 {
            if (self.mmc3.r8000 & 0x40) == 0 {
                let offset = self.prg_offset(cart, self.mmc3.bank_8c, address);
                return FetchResult { data: self.read_prg(cart, offset), driven: true };
            }
            return FetchResult { data: self.read_prg(cart, self.fixed_second_last(cart, address)), driven: true };
        }
        self.mmc3.fetch_prg(cart, address)
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address <= 0x5FFF {
            let addr_off = address & 0xFFF;
            let idx = ((addr_off >> 6) & 4) as usize | (data as usize & 3);
            self.expregs[4] = PROT_LUT[idx & 7];
            if (addr_off & 0x100) != 0 {
                self.expregs[3] = data;
            }
            return;
        }
        if address >= 0x8000 && address <= 0x9FFF {
            match address & 0xE003 {
                0x8000 => {
                    self.mmc3.store_prg(cart, address, data);
                }
                0x8001 => {
                    self.expregs[6] = bit_reverse6(data & 0x3F);
                    if self.expregs[7] == 0 {
                        self.sync();
                    }
                    self.mmc3.store_prg(cart, address, data);
                }
                0x8003 => {
                    self.expregs[5] = data & 0x3F;
                    self.sync();
                    self.mmc3.store_prg(cart, 0x8000, data);
                }
                _ => {
                    self.mmc3.store_prg(cart, address, data);
                }
            }
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
        if address < 0x2000 && !cart.chr_ram.is_empty() {
            let bank = self.mmc3.chr_bank(address) as usize | self.chr_ext_bit(address) as usize;
            let offset = (bank * 0x0400 + (address as usize & 0x03FF)) % cart.chr_ram.len();
            cart.chr_ram[offset] = data;
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

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.extend_from_slice(&self.expregs);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut idx = self.mmc3.load_mapper_registers(cart, state, start);
        if idx + 8 <= state.len() {
            self.expregs.copy_from_slice(&state[idx..idx + 8]);
            idx += 8;
        }
        idx
    }
}
