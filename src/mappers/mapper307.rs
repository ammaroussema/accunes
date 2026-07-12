use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper307 {
    pointer: u8,
    reg: [u8; 8],
}

impl Mapper307 {
    pub fn new() -> Self {
        Self {
            pointer: 0,
            reg: [0, 2, 4, 5, 6, 7, 0, 1],
        }
    }
}

impl Mapper for Mapper307 {
    fn reset(&mut self) {
        self.pointer = 0;
        self.reg = [0, 2, 4, 5, 6, 7, 0, 1];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0xE000 {
            let offset = 15 * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0xC000 {
            let bank = (self.reg[7] as usize) * 0x2000;
            let offset = bank + (address as usize & 0x1FFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0xB000 {
            let offset = 0x1000 + (address as usize & 0xFFF);
            FetchResult {
                data: if offset < cart.prg_ram.len() { cart.prg_ram[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0xA000 {
            let offset = 28 * 0x1000 + (address as usize & 0xFFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0x8000 {
            let bank = (self.reg[6] as usize) * 0x2000;
            let offset = bank + (address as usize & 0x1FFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0x7000 {
            let offset = 15 * 0x1000 + (address as usize & 0xFFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0x6000 {
            let offset = address as usize & 0xFFF;
            FetchResult {
                data: if offset < cart.prg_ram.len() { cart.prg_ram[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0x4020 && address <= 0x40FF {
            // bootleg FDS audio registers - return 0 to avoid game hanging on status polls
            FetchResult { data: 0, driven: true }
        } else if address >= 0x4100 {
            FetchResult { data: 0, driven: false }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x4020 && address <= 0x40FF {
            // bootleg FDS audio writes - not implemented
        } else if address >= 0x8000 && address < 0xA000 {
            if address & 1 == 0 {
                self.pointer = data;
            } else {
                self.reg[(self.pointer & 7) as usize] = data;
            }
        } else if address >= 0x6000 && address < 0x7000 {
            let offset = address as usize & 0xFFF;
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
        } else if address >= 0xB000 && address < 0xC000 {
            let offset = 0x1000 + (address as usize & 0xFFF);
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let nt = ((address >> 10) & 3) as usize;
        let sel = [self.reg[2] & 1, self.reg[4] & 1, self.reg[3] & 1, self.reg[5] & 1];
        0x2000 | ((sel[nt] as u16) << 10) | (address & 0x3FF)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        } else if address < 0x2000 && cart.using_chr_ram {
            let offset = address as usize & 0x1FFF;
            if offset < cart.chr_ram.len() {
                cart.chr_ram[offset] = data;
            }
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
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
            if !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[(address & 0x1FFF) as usize] as u16;
            } else if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[(address & 0x1FFF) as usize] as u16;
            }
        } else {
            let nt = ((address >> 10) & 3) as usize;
            let sel = [self.reg[2] & 1, self.reg[4] & 1, self.reg[3] & 1, self.reg[5] & 1];
            let mirrored = 0x2000 | ((sel[nt] as u16) << 10) | (address & 0x3FF);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.pointer);
        for r in &self.reg {
            state.push(*r);
        }
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.pointer = state.get(p).copied().unwrap_or(0); p += 1;
        for r in self.reg.iter_mut() {
            *r = state.get(p).copied().unwrap_or(0); p += 1;
        }
        p
    }
}
