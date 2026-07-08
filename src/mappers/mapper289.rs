use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper289 {
    latch: u8,
    reg: [u8; 2],
    dip_switches: u8,
}

impl Mapper289 {
    pub fn new() -> Self {
        Self { latch: 0, reg: [0; 2], dip_switches: 0 }
    }

    fn prg_bank(&self, slot: usize) -> usize {
        let nrom256 = (self.reg[0] & 0x01) != 0;
        let unrom = (self.reg[0] & 0x02) != 0;
        let prg_inner = self.reg[1] & 0x07;
        let prg_outer = self.reg[1] & 0xF8;
        let bank = if slot == 0 {
            if unrom {
                if nrom256 { prg_outer | 7 } else { (self.latch & 7) | prg_outer }
            } else {
                if nrom256 { (prg_inner & !1) | prg_outer } else { prg_inner | prg_outer }
            }
        } else {
            if unrom {
                prg_outer | 7
            } else {
                if nrom256 { prg_inner | 1 | prg_outer } else { prg_inner | prg_outer }
            }
        };
        bank as usize
    }
}

impl Mapper for Mapper289 {
    fn reset(&mut self) {
        self.latch = 0;
        self.reg = [0; 2];
        self.dip_switches = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let slot = ((address - 0x8000) >> 14) as usize;
            let bank = self.prg_bank(slot);
            let num_16k = cart.prg_rom.len() / 0x4000;
            let bank = if num_16k > 0 { bank % num_16k } else { 0 };
            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0x6000 {
            FetchResult { data: self.dip_switches & 3, driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.latch = data;
        } else if address >= 0x6000 && address <= 0x6FFF {
            if address == 0x6000 || address == 0x6001 {
                self.reg[(address & 1) as usize] = data;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.reg[0] & 0x08 != 0 {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
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
                chr_ram[address as usize & 0x1FFF]
            } else if !chr_rom.is_empty() {
                chr_rom[address as usize & 0x1FFF]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if self.reg[0] & 0x08 != 0 {
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
            if (self.reg[0] & 0x04) == 0 && cart.using_chr_ram && !cart.chr_ram.is_empty() {
                cart.chr_ram[address as usize & 0x1FFF] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = if self.reg[0] & 0x08 != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = vec![self.latch, self.dip_switches];
        state.extend_from_slice(&self.reg);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.latch = state[p]; p += 1; }
        if p < state.len() { self.dip_switches = state[p]; p += 1; }
        if p + 2 <= state.len() { self.reg.copy_from_slice(&state[p..p+2]); p += 2; }
        p
    }
}
