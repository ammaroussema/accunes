use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{mmc3_chr_bank, MapperMMC3, Mmc3Config};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Namco108Kind {
    Mapper76,
    Mapper88,
    Mapper95,
    Mapper154,
}

pub struct Namco108 {
    kind: Namco108Kind,
    mmc3: MapperMMC3,
    single_screen_b: bool,
    nt_pages: [u8; 4],
}

impl Namco108 {
    pub fn mapper76() -> Self {
        Self::new(Namco108Kind::Mapper76)
    }

    pub fn mapper88() -> Self {
        Self::new(Namco108Kind::Mapper88)
    }

    pub fn mapper95() -> Self {
        Self::new(Namco108Kind::Mapper95)
    }

    pub fn mapper154() -> Self {
        Self::new(Namco108Kind::Mapper154)
    }

    pub fn new(kind: Namco108Kind) -> Self {
        let mut mmc3 = MapperMMC3::new(Mmc3Config::embedded());
        mmc3.r8000 = 0;
        Self {
            kind,
            mmc3,
            single_screen_b: false,
            nt_pages: [0; 4],
        }
    }

    fn prg8_mask(cart: &Cartridge) -> u8 {
        let banks = cart.prg_rom.len() / 0x2000;
        if banks == 0 {
            0
        } else {
            (banks - 1) as u8
        }
    }

    fn prg_rom_read(cart: &Cartridge, bank_8k: usize, offset_in_bank: usize) -> u8 {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }
        cart.prg_rom[(bank_8k * 0x2000 + offset_in_bank) % len]
    }

    fn chr_bank(&self, address: u16) -> u8 {
        match self.kind {
            Namco108Kind::Mapper76 => match (address >> 11) & 3 {
                0 => self.mmc3.chr_1k0,
                1 => self.mmc3.chr_1k4,
                2 => self.mmc3.chr_1k8,
                _ => self.mmc3.chr_1kc,
            },
            Namco108Kind::Mapper88 | Namco108Kind::Mapper95 | Namco108Kind::Mapper154 => {
                let chr_2k0 = self.mmc3.chr_2k0 & 0x3F;
                let chr_2k8 = self.mmc3.chr_2k8 & 0x3F;
                let or40 = matches!(self.kind, Namco108Kind::Mapper88 | Namco108Kind::Mapper154);
                let chr_1k0 = if or40 {
                    self.mmc3.chr_1k0 | 0x40
                } else {
                    self.mmc3.chr_1k0
                };
                let chr_1k4 = if or40 {
                    self.mmc3.chr_1k4 | 0x40
                } else {
                    self.mmc3.chr_1k4
                };
                let chr_1k8 = if or40 {
                    self.mmc3.chr_1k8 | 0x40
                } else {
                    self.mmc3.chr_1k8
                };
                let chr_1kc = if or40 {
                    self.mmc3.chr_1kc | 0x40
                } else {
                    self.mmc3.chr_1kc
                };
                mmc3_chr_bank(0, chr_2k0, chr_2k8, chr_1k0, chr_1k4, chr_1k8, chr_1kc, address)
            }
        }
    }

    fn chr_page_size(&self) -> usize {
        if self.kind == Namco108Kind::Mapper76 {
            0x800
        } else {
            0x400
        }
    }

    fn chr_read_byte(
        &self,
        address: u16,
        chr_rom: &[u8],
        chr_ram: &[u8],
        using_chr_ram: bool,
    ) -> u8 {
        let bank = self.chr_bank(address) as usize;
        let page_size = self.chr_page_size();
        let offset = bank * page_size + (address as usize & (page_size - 1));
        if using_chr_ram && !chr_ram.is_empty() {
            chr_ram[offset % chr_ram.len()]
        } else if !chr_rom.is_empty() {
            chr_rom[offset % chr_rom.len()]
        } else {
            0
        }
    }

    fn update_nt_pages_from_regs(&mut self) {
        if self.kind == Namco108Kind::Mapper95 {
            self.nt_pages[0] = (self.mmc3.chr_2k0 >> 5) & 1;
            self.nt_pages[1] = self.nt_pages[0];
            self.nt_pages[2] = (self.mmc3.chr_2k8 >> 5) & 1;
            self.nt_pages[3] = self.nt_pages[2];
        }
    }

    fn mirror_address_for_ppu(
        &self,
        address: u16,
        header_horizontal: bool,
        alternative_nametable: bool,
    ) -> u16 {
        if alternative_nametable {
            return address;
        }
        match self.kind {
            Namco108Kind::Mapper154 => {
                if self.single_screen_b {
                    (address & 0x33FF) | 0x0400
                } else {
                    address & 0x33FF
                }
            }
            Namco108Kind::Mapper95 => {
                let slot = ((address >> 10) & 3) as usize;
                let page = u16::from(self.nt_pages[slot] & 1);
                page * 0x400 | (address & 0x3FF)
            }
            _ => {
                if header_horizontal {
                    (address & 0x33FF) | ((address & 0x0800) >> 1)
                } else {
                    address & 0x37FF
                }
            }
        }
    }

    fn mirror_address(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mirror_address_for_ppu(
            address,
            cart.nametable_horizontal_mirroring,
            cart.alternative_nametable_arrangement,
        )
    }
}

impl Mapper for Namco108 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.mmc3.r8000 = 0;
        self.single_screen_b = false;
        self.nt_pages = [0; 4];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let num_8k = len / 0x2000;
            let offset_in_bank = address as usize & 0x1FFF;
            let bank = match address {
                0xE000..=0xFFFF => num_8k.saturating_sub(1),
                0xC000..=0xDFFF => num_8k.saturating_sub(2),
                0xA000..=0xBFFF => self.mmc3.bank_a as usize % num_8k,
                _ => self.mmc3.bank_8c as usize % num_8k,
            };
            FetchResult {
                data: Self::prg_rom_read(cart, bank, offset_in_bank),
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            return;
        }
        if self.kind == Namco108Kind::Mapper154 {
            self.single_screen_b = (data & 0x40) != 0;
        }
        match address & 0x8001 {
            0x8000 => self.mmc3.r8000 = data & 0x3F,
            0x8001 => {
                let mask = Self::prg8_mask(cart);
                match self.mmc3.r8000 & 0x07 {
                    0 => self.mmc3.chr_2k0 = data & 0xFE,
                    1 => self.mmc3.chr_2k8 = data & 0xFE,
                    2 => self.mmc3.chr_1k0 = data,
                    3 => self.mmc3.chr_1k4 = data,
                    4 => self.mmc3.chr_1k8 = data,
                    5 => self.mmc3.chr_1kc = data,
                    6 => self.mmc3.bank_8c = data & mask,
                    7 => self.mmc3.bank_a = data & mask,
                    _ => {}
                }
                self.update_nt_pages_from_regs();
            }
            _ => {}
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mirror_address(cart, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = self.chr_read_byte(address, chr_rom, chr_ram, using_chr_ram);
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = self.mirror_address_for_ppu(
                address,
                nametable_horizontal_mirroring,
                alternative_nametable_arrangement,
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
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let bank = self.chr_bank(address) as usize;
                let page_size = self.chr_page_size();
                let len = cart.chr_ram.len();
                let offset = (bank * page_size + (address as usize & (page_size - 1))) % len;
                cart.chr_ram[offset] = data;
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

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.kind as u8);
        state.push(if self.single_screen_b { 1 } else { 0 });
        state.extend_from_slice(&self.nt_pages);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mmc3_end = self.mmc3.load_mapper_registers(cart, state, start);
        if state.len() >= mmc3_end + 6 {
            let mut i = mmc3_end;
            i += 1; 
            self.single_screen_b = state[i] != 0;
            i += 1;
            for slot in 0..4 {
                self.nt_pages[slot] = state[i];
                i += 1;
            }
            i
        } else {
            mmc3_end
        }
    }
}
