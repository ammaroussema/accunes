use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper302 {
    chr: [u16; 8],
    mirroring: u8,
}

impl Mapper302 {
    pub fn new() -> Self {
        Self {
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            mirroring: 0,
        }
    }
}

impl Mapper for Mapper302 {
    fn reset(&mut self) {
        self.chr = [0, 1, 2, 3, 4, 5, 6, 7];
        self.mirroring = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0xA000 {
            let bank = ((address >> 12) as usize) - 6;
            let addr_lo = address as usize & 0xFFF;
            let index = (bank << 1) | ((addr_lo >> 11) & 1);
            let index = index ^ 4;
            let chr_bank = self.chr[index & 7] as usize;
            let offset = (chr_bank << 11) | (addr_lo & 0x7FF);
            let data = if offset < cart.prg_rom.len() {
                cart.prg_rom[offset]
            } else {
                0
            };
            FetchResult { data, driven: true }
        } else if address >= 0xA000 && address < 0xC000 {
            let offset = (0x0D as usize) * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0xC000 {
            let offset = (0x07 as usize) * 0x4000 + (address as usize & 0x3FFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0x6000 {
            FetchResult { data: 0, driven: false }
        } else if address >= 0x4020 {
            FetchResult { data: 0xFF, driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        match address & 0xF000 {
            0x8000 | 0xA000 => {
                // VRC24 PRG register - ignored (fixed banking)
            }
            0x9000 => {
                // VRC24 Misc - mirroring
                self.mirroring = data & 3;
            }
            0xB000 | 0xC000 | 0xD000 | 0xE000 => {
                let bank = ((address >> 12) - 0xB) as usize;
                let bit1 = ((address & 0x02) != 0) as usize;
                let reg = (bank << 1) | bit1;
                if reg < 8 {
                    if (address & 1) != 0 {
                        self.chr[reg] = (self.chr[reg] & 0x00F) | ((data as u16) << 4);
                    } else {
                        self.chr[reg] = (self.chr[reg] & 0xFF0) | (data as u16 & 0x0F);
                    }
                }
            }
            _ => {}
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            return address;
        }
        if (self.mirroring & 1) != 0 {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
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
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            if !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[address as usize & 0x1FFF] as u16;
            } else if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[address as usize & 0x1FFF] as u16;
            }
        } else {
            let h = if alternative_nametable_arrangement {
                false
            } else {
                (self.mirroring & 1) != 0
            };
            let mirrored = if h {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
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

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        for c in &self.chr {
            state.extend_from_slice(&c.to_le_bytes());
        }
        state.push(self.mirroring);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for c in self.chr.iter_mut() {
            if p + 2 <= state.len() {
                *c = u16::from_le_bytes(state[p..p+2].try_into().unwrap());
                p += 2;
            }
        }
        self.mirroring = state.get(p).copied().unwrap_or(0); p += 1;
        p
    }
}
