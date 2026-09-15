use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const SECURITY_UNLOCK: u8 = 0xA3;
const RAM_BASE: usize = 0x1F00; 

pub struct TaitoX1005 {
    alternate_mirroring: bool,
    prg_banks: [u8; 3],
    chr_banks: [u8; 8],
    security: u8,
    mirroring_vertical: bool,
    nt_pages: [u8; 4],
}

impl TaitoX1005 {
    pub fn new(alternate_mirroring: bool) -> Self {
        Self {
            alternate_mirroring,
            prg_banks: [0, 0, 0],
            chr_banks: [0, 1, 2, 3, 4, 5, 6, 7],
            security: 0,
            mirroring_vertical: false,
            nt_pages: [0; 4],
        }
    }

    pub fn mapper80() -> Self {
        Self::new(false)
    }

    pub fn mapper207() -> Self {
        Self::new(true)
    }

    fn ram_index(address: u16) -> usize {
        RAM_BASE + (address as usize & 0x7F)
    }

    fn ram_enabled(&self) -> bool {
        self.security == SECURITY_UNLOCK
    }

    fn mirror_address(&self, address: u16) -> u16 {
        if self.alternate_mirroring {
            let slot = ((address >> 10) & 3) as usize;
            let page = u16::from(self.nt_pages[slot] & 1);
            page * 0x400 | (address & 0x3FF)
        } else if self.mirroring_vertical {
            address & 0x37FF
        } else {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        }
    }

    fn chr_read_byte(
        &self,
        address: u16,
        chr_rom: &[u8],
        chr_ram: &[u8],
        using_chr_ram: bool,
    ) -> u8 {
        let slot = (address >> 10) as usize & 7;
        let bank = self.chr_banks[slot] as usize;
        let offset = bank * 0x400 + (address as usize & 0x3FF);
        if using_chr_ram && !chr_ram.is_empty() {
            chr_ram[offset % chr_ram.len()]
        } else if !chr_rom.is_empty() {
            chr_rom[offset % chr_rom.len()]
        } else {
            0
        }
    }
}

impl Mapper for TaitoX1005 {
    fn reset(&mut self) {
        self.prg_banks = [0, 0, 0];
        self.chr_banks = [0, 1, 2, 3, 4, 5, 6, 7];
        self.security = 0;
        self.mirroring_vertical = false;
        self.nt_pages = [0; 4];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let num_8k = cart.prg_rom.len() / 0x2000;
            if num_8k == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let offset_in_bank = address as usize & 0x1FFF;
            let bank = match address {
                0xE000..=0xFFFF => num_8k.saturating_sub(1),
                0xC000..=0xDFFF => self.prg_banks[2] as usize % num_8k,
                0xA000..=0xBFFF => self.prg_banks[1] as usize % num_8k,
                _ => self.prg_banks[0] as usize % num_8k,
            };
            let offset = bank * 0x2000 + offset_in_bank;
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if (0x7EF8..=0x7EF9).contains(&address) {
            FetchResult {
                data: self.security,
                driven: true,
            }
        } else if address >= 0x7F00 {
            if self.ram_enabled() {
                let idx = Self::ram_index(address);
                if idx < cart.prg_ram.len() {
                    return FetchResult {
                        data: cart.prg_ram[idx],
                        driven: true,
                    };
                }
            }
            FetchResult {
                data: (address >> 8) as u8,
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        match address {
            0x7EF0 => {
                self.chr_banks[0] = data;
                self.chr_banks[1] = data.wrapping_add(1);
                if self.alternate_mirroring {
                    let page = (data >> 7) & 1;
                    self.nt_pages[0] = page;
                    self.nt_pages[1] = page;
                }
            }
            0x7EF1 => {
                self.chr_banks[2] = data;
                self.chr_banks[3] = data.wrapping_add(1);
                if self.alternate_mirroring {
                    let page = (data >> 7) & 1;
                    self.nt_pages[2] = page;
                    self.nt_pages[3] = page;
                }
            }
            0x7EF2..=0x7EF5 => {
                self.chr_banks[4 + (address - 0x7EF2) as usize] = data;
            }
            0x7EF6 | 0x7EF7 if !self.alternate_mirroring => {
                self.mirroring_vertical = (data & 0x01) != 0;
            }
            0x7EF8 | 0x7EF9 => {
                self.security = data;
            }
            0x7EFA | 0x7EFB => self.prg_banks[0] = data,
            0x7EFC | 0x7EFD => self.prg_banks[1] = data,
            0x7EFE | 0x7EFF => self.prg_banks[2] = data,
            0x7F00..=0x7FFF if self.ram_enabled() => {
                let idx = Self::ram_index(address);
                if idx < cart.prg_ram.len() {
                    cart.prg_ram[idx] = data;
                }
            }
            _ => {}
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
            let byte = self.chr_read_byte(address, chr_rom, chr_ram, using_chr_ram);
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else {
                self.mirror_address(address)
            };
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
                let slot = (address >> 10) as usize & 7;
                let bank = self.chr_banks[slot] as usize;
                let len = cart.chr_ram.len();
                let offset = (bank * 0x400 + (address as usize & 0x3FF)) % len;
                cart.chr_ram[offset] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_address(address);
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
        state.push(if self.alternate_mirroring { 1 } else { 0 });
        state.extend_from_slice(&self.prg_banks);
        state.extend_from_slice(&self.chr_banks);
        state.push(self.security);
        state.push(if self.mirroring_vertical { 1 } else { 0 });
        state.extend_from_slice(&self.nt_pages);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut i = start;
        if state.len() >= i + 18 {
            self.alternate_mirroring = state[i] != 0;
            i += 1;
            for r in 0..3 {
                self.prg_banks[r] = state[i];
                i += 1;
            }
            for r in 0..8 {
                self.chr_banks[r] = state[i];
                i += 1;
            }
            self.security = state[i];
            i += 1;
            self.mirroring_vertical = state[i] != 0;
            i += 1;
            for r in 0..4 {
                self.nt_pages[r] = state[i];
                i += 1;
            }
        } else if state.len() >= i + 12 {
            for r in 0..3 {
                self.prg_banks[r] = state[i];
                i += 1;
            }
            for r in 0..8 {
                self.chr_banks[r] = state[i];
                i += 1;
            }
            self.security = state[i];
            i += 1;
            self.mirroring_vertical = state[i] != 0;
            i += 1;
        }
        i
    }
}
