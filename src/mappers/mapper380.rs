use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};
pub struct Mapper380 {
    latch_addr: u16,
    dip_switches: u8,
    submapper: u8,
}
impl Mapper380 {
    pub fn new(header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        let submapper = if header.len() > 8 { header[8] >> 4 } else { 0 };
        Self {
            latch_addr: 0,
            dip_switches: 0,
            submapper,
        }
    }
    fn prg_bank(&self) -> usize {
        (self.latch_addr as usize >> 2) & 0x1F
    }
    fn nrom_mode(&self) -> bool {
        (self.latch_addr & 0x200) != 0
    }
    fn chr_write_protected(&self) -> bool {
        (self.latch_addr & 0x080) != 0
    }
    fn is_horizontal_mirroring(&self) -> bool {
        if self.submapper == 2 {
            (self.latch_addr & 0x040) != 0
        } else {
            (self.latch_addr & 0x002) != 0
        }
    }
    fn dip_read_mode(&self) -> bool {
        self.submapper != 1 && (self.latch_addr & 0x100) != 0
    }
    fn read_address(&self, address: u16) -> u16 {
        if self.dip_read_mode() {
            (address & !0x0F) | (self.dip_switches as u16 & 0x0F)
        } else {
            address
        }
    }
    fn prg_offset(&self, cart: &Cartridge, address: u16) -> usize {
        let read_addr = self.read_address(address);
        let prg = self.prg_bank();
        if self.nrom_mode() {
            let num_32k = (cart.prg_rom.len() / 0x8000).max(1);
            if (self.latch_addr & 0x001) != 0 {
                let num_16k = (cart.prg_rom.len() / 0x4000).max(1);
                (prg % num_16k) * 0x4000 + (read_addr as usize & 0x3FFF)
            } else {
                (prg >> 1) % num_32k * 0x8000 + (read_addr as usize & 0x7FFF)
            }
        } else {
            let num_16k = (cart.prg_rom.len() / 0x4000).max(1);
            let bank = if address >= 0xC000 {
                let high = if self.submapper == 1 && (self.latch_addr & 0x100) != 0 {
                    15
                } else {
                    7
                };
                prg | high
            } else {
                prg
            };
            (bank % num_16k) * 0x4000 + (read_addr as usize & 0x3FFF)
        }
    }
    fn vram_index(&self, address: u16) -> usize {
        let offset = address as usize & 0x3FF;
        let page = if self.is_horizontal_mirroring() {
            if (address & 0x0800) != 0 {
                1
            } else {
                0
            }
        } else if (address & 0x0400) != 0 {
            1
        } else {
            0
        };
        (page << 10) | offset
    }
}
impl Mapper for Mapper380 {
    fn reset(&mut self) {
        self.latch_addr = 0;
    }
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let offset = address as usize & 0x1FFF;
                return FetchResult {
                    data: cart.prg_ram[offset % cart.prg_ram.len()],
                    driven: true,
                };
            }
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        if address < 0x8000 {
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        let len = cart.prg_rom.len();
        if len == 0 {
            return FetchResult {
                data: 0,
                driven: true,
            };
        }
        let offset = self.prg_offset(cart, address);
        FetchResult {
            data: cart.prg_rom[offset % len],
            driven: true,
        }
    }
    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let len = cart.prg_ram.len();
                cart.prg_ram[(address as usize & 0x1FFF) % len] = data;
            }
            return;
        }
        if address >= 0x8000 {
            self.latch_addr = address;
        }
    }
    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v(self.is_horizontal_mirroring(), address)
    }
    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = if !chr_ram.is_empty() {
                chr_ram[(address as usize) % chr_ram.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let idx = self.vram_index(address);
            new_addr_bus |= vram[idx % vram.len().max(1)] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }
    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !self.chr_write_protected() && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize) % len] = data;
            }
        } else if address < 0x3F00 {
            let idx = self.vram_index(address);
            let len = vram.len();
            if len > 0 {
                vram[idx % len] = data;
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
        let mut state = self.latch_addr.to_le_bytes().to_vec();
        state.push(self.dip_switches);
        state.push(self.submapper);
        state
    }
    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.latch_addr = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.dip_switches = state[p];
            p += 1;
        }
        if p < state.len() {
            self.submapper = state[p];
            p += 1;
        }
        p
    }
}
