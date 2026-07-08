use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper286 {
    prg: [u8; 4],
    chr: [u8; 4],
    mirroring: u8,
    dip_switches: u8,
}

impl Mapper286 {
    pub fn new() -> Self {
        Self { prg: [0; 4], chr: [0; 4], mirroring: 0, dip_switches: 0 }
    }
}

impl Mapper for Mapper286 {
    fn reset(&mut self) {
        for i in 0..4 {
            self.prg[i] = 0xC | i as u8;
            self.chr[i] = 0;
        }
        self.mirroring = 0;
        self.dip_switches = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let num_8k = cart.prg_rom.len() / 0x2000;
            if num_8k == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let slot = ((address - 0x8000) >> 13) as usize;
            let bank = (self.prg[slot] as usize) % num_8k;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, _data: u8) {
        if address >= 0x8000 {
            let ah = address & 0xF000;
            if ah == 0x8000 || ah == 0x9000 {
                self.chr[((address >> 10) & 3) as usize] = (address & 0x1F) as u8;
        } else if ah == 0xA000 || ah == 0xB000 {
            if self.dip_switches == 0 || (address & self.dip_switches as u16) != 0 {
                self.prg[((address >> 10) & 3) as usize] = (address & 0x0F) as u8;
            }
            } else if ah == 0xC000 {
                self.mirroring = address as u8 & 1;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mirroring != 0 {
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
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let slot = (address >> 11) as usize & 3;
            let bank = (self.chr[slot] as usize) & 0x1F;
            let src = if !chr_rom.is_empty() { chr_rom } else { chr_ram };
            let offset = bank * 0x800 + (address as usize & 0x7FF);
            let byte = if offset < src.len() { src[offset] } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if self.mirroring != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let slot = (address >> 11) as usize & 3;
                let bank = (self.chr[slot] as usize) & 0x1F;
                let offset = bank * 0x800 + (address as usize & 0x7FF);
                if offset < cart.chr_ram.len() {
                    cart.chr_ram[offset] = data;
                }
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = if self.mirroring != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::with_capacity(10);
        state.extend_from_slice(&self.prg);
        state.extend_from_slice(&self.chr);
        state.push(self.mirroring);
        state.push(self.dip_switches);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 4 <= state.len() { self.prg.copy_from_slice(&state[p..p+4]); p += 4; }
        if p + 4 <= state.len() { self.chr.copy_from_slice(&state[p..p+4]); p += 4; }
        if p < state.len() { self.mirroring = state[p]; p += 1; }
        if p < state.len() { self.dip_switches = state[p]; p += 1; }
        p
    }
}
