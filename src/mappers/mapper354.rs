use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper354 {
    latch_addr: u16,
    latch_data: u8,
}

impl Mapper354 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self { latch_addr: 0, latch_data: 0 }
    }
}

impl Mapper for Mapper354 {
    fn reset(&mut self) {
        self.latch_addr = 0;
        self.latch_data = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        let prg = (self.latch_data as usize & 0x3F)
                | (((self.latch_addr as usize) << 2) & 0x40)
                | (((self.latch_addr as usize) >> 5) & 0x80);
        let mode = self.latch_addr & 7;
        let len = cart.prg_rom.len().max(1);
        if address >= 0x6000 && address < 0x8000 {
            if mode == 5 {
                let bank = (prg << 1) | ((self.latch_data >> 7) as usize);
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                return FetchResult { data: cart.prg_rom[offset % len], driven: true };
            }
            return FetchResult { data: 0, driven: false };
        }
        if address >= 0x8000 {
            let page = (address as usize - 0x8000) / 0x2000;
            let bank = match mode {
                0 | 4 => prg >> 1,
                1 => {
                    if page < 2 { prg } else { prg | 7 }
                }
                2 | 6 => (prg << 1) | ((self.latch_data >> 7) as usize),
                3 | 7 => prg,
                5 => (prg >> 1) | 3,
                _ => 0,
            };
            let bank_8k = if mode == 2 || mode == 6 {
                bank
            } else if mode == 0 || mode == 4 || mode == 5 {
                bank * 4 + page
            } else {
                if page < 2 { bank * 2 + page } else { (bank | 7) * 2 + (page - 2) }
            };
            let offset = bank_8k * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult { data: cart.prg_rom[offset % len], driven: true };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.latch_data = data;
            self.latch_addr = address;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v((self.latch_data & 0x40) != 0, address)
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
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[(address as usize) % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[(address as usize) % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mir = mirror_h_or_v((self.latch_data & 0x40) != 0, address);
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        let protect = (self.latch_addr & 0x08) != 0;
        if address < 0x2000 && cart.using_chr_ram && !protect {
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[(address as usize) % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mir = mirror_h_or_v((self.latch_data & 0x40) != 0, address);
            vram[(mir & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = self.latch_addr.to_le_bytes().to_vec();
        state.push(self.latch_data);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.latch_addr = u16::from_le_bytes([state[p], state[p+1]]);
            p += 2;
        }
        if p < state.len() { self.latch_data = state[p]; p += 1; }
        p
    }
}
