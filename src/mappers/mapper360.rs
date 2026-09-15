use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper360 {
    reg: u8,
    dip_switches: u8,
    submapper: u8,
}

impl Mapper360 {
    pub fn new(header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        let submapper = if header.len() > 15 { (header[15] & 0x0F) as u8 } else { 0 };
        let dip = if header.len() > 7 { header[7] } else { 0 };
        let reg = if submapper == 0 { dip | 0x20 } else { 0 };
        Self { reg, dip_switches: dip, submapper }
    }

    fn sync_from_dip(&mut self) {
        if self.submapper == 0 {
            self.reg = self.dip_switches | 0x20;
        }
    }
}

impl Mapper for Mapper360 {
    fn reset(&mut self) {
        self.reg = 0;
        self.sync_from_dip();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len().max(1);
            let bank = if (self.reg & 0x20) == 0 {
                0x40
            } else if (self.reg & 0x1F) < 2 {
                ((self.reg as usize) >> 1) & 0x0F
            } else {
                self.reg as usize & 0x1F
            };
            let page = (address as usize - 0x8000) / 0x2000;
            let mode = (self.reg & 0x20) == 0 || (self.reg & 0x1F) >= 2;
            let offset = if mode {
                (bank * 4 + page) * 0x2000 + (address as usize & 0x1FFF)
            } else {
                bank * 0x8000 + (address as usize & 0x7FFF)
            };
            FetchResult { data: cart.prg_rom[offset % len], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, val: u8) {
        if self.submapper != 0 && address >= 0x4000 && address < 0x5000 && (address & 0x100) != 0 {
            self.reg = val;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v((self.reg & 0x10) != 0, address)
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
            let bank = self.reg as usize;
            if using_chr_ram && !chr_ram.is_empty() {
                let len = chr_ram.len().max(1);
                new_addr_bus |= chr_ram[(bank * 0x2000 + address as usize) % len] as u16;
            } else if !chr_rom.is_empty() {
                let len = chr_rom.len().max(1);
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                new_addr_bus |= chr_rom[offset % len] as u16;
            }
        } else {
            let mir = mirror_h_or_v((self.reg & 0x10) != 0, address);
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn get_dip_switches(&self) -> u8 { self.dip_switches }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
        if self.submapper == 0 {
            self.reg = value | 0x20;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.reg, self.dip_switches, self.submapper]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.reg = state[p]; p += 1; }
        if p < state.len() { self.dip_switches = state[p]; p += 1; }
        if p < state.len() { self.submapper = state[p]; p += 1; }
        p
    }
}
