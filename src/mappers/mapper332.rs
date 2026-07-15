use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper332 {
    reg: [u8; 2],
    latch_data: u8,
}

impl Mapper332 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self { reg: [0; 2], latch_data: 0 }
    }
}

impl Mapper for Mapper332 {
    fn reset(&mut self) {
        self.reg = [0; 2];
        self.latch_data = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }
        if address >= 0x8000 {
            let dip = self.reg[1] & 0xC0;
            if dip != 0 && (dip & self.get_dip_switches()) != 0 {
                return FetchResult { data: 0, driven: false };
            }
            let prg = (self.reg[0] & 0x07) | (self.reg[0] >> 3 & 0x08);
            let nrom256 = (self.reg[0] & 0x08) == 0;
            if nrom256 {
                let bank0 = (prg as usize & !1) * 0x4000;
                let bank1 = (prg as usize | 1) * 0x4000;
                let offset = address as usize & 0x3FFF;
                let base = if address < 0xC000 { bank0 } else { bank1 };
                return FetchResult {
                    data: cart.prg_rom[(base + offset) % cart.prg_rom.len().max(1)],
                    driven: true,
                };
            } else {
                let bank = prg as usize;
                let offset = bank * 0x8000 + (address as usize & 0x7FFF);
                return FetchResult {
                    data: cart.prg_rom[offset % cart.prg_rom.len().max(1)],
                    driven: true,
                };
            }
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            let lock = (self.reg[0] & 0x20) != 0;
            if !lock {
                self.reg[address as usize & 1] = data;
            }
            return;
        }
        if address >= 0x8000 {
            self.latch_data = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v((self.reg[0] & 0x10) != 0, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let chr_base = (self.reg[1] & 0x07) | (self.reg[0] >> 3 & 0x08);
            let chr_mask = if (self.reg[1] & 0x10) != 0 { 0 }
                else if (self.reg[1] & 0x20) != 0 { 1 }
                else { 3 };
            let bank = (chr_base as usize & !chr_mask) | (self.latch_data as usize & chr_mask);
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            new_addr_bus |= vram[(mirror_h_or_v((self.reg[0] & 0x10) != 0, address) & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let chr_base = (self.reg[1] & 0x07) | (self.reg[0] >> 3 & 0x08);
            let chr_mask = if (self.reg[1] & 0x10) != 0 { 0 }
                else if (self.reg[1] & 0x20) != 0 { 1 }
                else { 3 };
            let bank = (chr_base as usize & !chr_mask) | (self.latch_data as usize & chr_mask);
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::with_capacity(3);
        state.extend_from_slice(&self.reg);
        state.push(self.latch_data);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 > state.len() { return p; }
        self.reg.copy_from_slice(&state[p..p+2]);
        p += 2;
        if p < state.len() {
            self.latch_data = state[p];
            p + 1
        } else { p }
    }
}
