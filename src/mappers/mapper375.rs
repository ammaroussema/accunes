use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};
pub struct Mapper375 {
    latch_addr: u16,
    latch_data: u8,
    submapper: u8,
}
impl Mapper375 {
    pub fn new(header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        let submapper = if header.len() > 8 { header[8] >> 4 } else { 0 };
        Self { latch_addr: 0, latch_data: 0, submapper }
    }
    fn prg_bank(&self) -> usize {
        let a = self.latch_addr as usize;
        (a >> 2 & 0x1F) | (a >> 3 & 0x20) | (a >> 4 & 0x40)
    }
    fn chr_write_protected(&self) -> bool {
        (self.latch_addr & 0x080) != 0
    }
    fn is_horizontal_mirroring(&self) -> bool {
        (self.latch_addr & 0x002) != 0
    }
    fn vram_index(&self, address: u16) -> usize {
        let offset = address as usize & 0x3FF;
        let page = if self.is_horizontal_mirroring() {
            if (address & 0x0800) != 0 { 1 } else { 0 }
        } else {
            if (address & 0x0400) != 0 { 1 } else { 0 }
        };
        (page << 10) | offset
    }
}
impl Mapper for Mapper375 {
    fn reset(&mut self) {
        self.latch_addr = 0;
        self.latch_data = 0;
    }
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let offset = address as usize & 0x1FFF;
                return FetchResult { data: cart.prg_ram[offset % cart.prg_ram.len()], driven: true };
            }
            return FetchResult { data: 0, driven: false };
        }
        if address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }
        let len = cart.prg_rom.len();
        if len == 0 {
            return FetchResult { data: 0, driven: true };
        }
        let num_16k = (len / 0x4000).max(1);
        let prg = self.prg_bank();
        let nrom_mode = (self.latch_addr & 0x080) != 0;
        let bank_16k = if nrom_mode {
            if (self.latch_addr & 0x001) != 0 {
                let half = if address >= 0xC000 { 1usize } else { 0 };
                (prg >> 1) * 2 + half
            } else {
                prg
            }
        } else {
            if address >= 0xC000 {
                if (self.latch_addr & 0x200) != 0 {
                    prg | 7
                } else if self.submapper == 2 {
                    0
                } else {
                    prg & !7
                }
            } else {
                if (self.latch_addr & 0x800) != 0 {
                    (prg & !7) | (self.latch_data as usize & 7)
                } else {
                    prg
                }
            }
        };
        let offset = (bank_16k % num_16k) * 0x4000 + (address as usize & 0x3FFF);
        FetchResult { data: cart.prg_rom[offset % len], driven: true }
    }
    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let len = cart.prg_ram.len();
                cart.prg_ram[(address as usize & 0x1FFF) % len] = data;
            }
            return;
        }
        if address < 0x8000 { return; }
        if (self.latch_addr & 0x800) != 0 {
            self.latch_data = data;
        } else {
            self.latch_addr = address;
            self.latch_data = data;
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
            } else { 0 };
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
    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = self.latch_addr.to_le_bytes().to_vec();
        state.push(self.latch_data);
        state.push(self.submapper);
        state
    }
    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.latch_addr = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() { self.latch_data = state[p]; p += 1; }
        if p < state.len() { self.submapper = state[p]; p += 1; }
        p
    }
}
