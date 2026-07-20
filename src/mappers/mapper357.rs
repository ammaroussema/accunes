use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper357 {
    bank_smb2j: u8,
    bank_unrom: u8,
    bank_swap: u8,
    irq_enabled: bool,
    irq_counter: u16,
    dip_switches: u8,
}

const BANK_TRANSLATE: [[u8; 8]; 2] = [
    [4, 3, 5, 3, 6, 3, 7, 3],
    [1, 1, 5, 1, 4, 1, 5, 1],
];

impl Mapper357 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self { bank_smb2j: 3, bank_unrom: 0, bank_swap: 0, irq_enabled: false, irq_counter: 0, dip_switches: 0 }
    }
}

impl Mapper for Mapper357 {
    fn reset(&mut self) {
        self.bank_smb2j = 3;
        self.bank_unrom = 0;
        self.bank_swap = 0;
        self.irq_enabled = false;
        self.irq_counter = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let dip = self.dip_switches;
            let page = (address as usize - 0x8000) / 0x2000;
            let len = cart.prg_rom.len().max(1);
            let bank_8k = if dip == 0 {
                if page == 0 {
                    if self.bank_swap != 0 { 0usize } else { 1 }
                } else if page == 1 {
                    0
                } else {
                    let sw = self.bank_swap as usize;
                    let idx = self.bank_smb2j as usize & 7;
                    if page == 2 {
                        BANK_TRANSLATE[sw][idx] as usize
                    } else {
                        if self.bank_swap != 0 { 8 } else { 10 }
                    }
                }
            } else {
                let bank_16k = (dip as usize) | (self.bank_unrom as usize);
                if page < 2 { bank_16k } else { (dip as usize) | 7 }
            };
            let offset = bank_8k * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult { data: cart.prg_rom[offset % len], driven: true };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, val: u8) {
        if address >= 0x4020 && address < 0x6000 {
            if (address & 0x71FF) == 0x4022 {
                self.bank_smb2j = val & 7;
            } else if (address & 0x71FF) == 0x4120 {
                self.bank_swap = val & 1;
            } else if (address & 0xF1FF) == 0x4122 {
                self.irq_enabled = (val & 1) != 0;
                self.irq_counter = 0;
            }
        }
        if (address & 0xF000) == 0x8000 {
            self.bank_unrom = val & 7;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.dip_switches == 0x18 {
            mirror_h_or_v(true, address)
        } else {
            mirror_h_or_v(false, address)
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        _chr_ram: &[u8],
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
            let byte = if using_chr_ram && !_chr_ram.is_empty() {
                _chr_ram[(address as usize) % _chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[(address as usize) % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let h = self.dip_switches == 0x18;
            let mir = mirror_h_or_v(!h, address);
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let len = cart.chr_ram.len();
            if len > 0 { cart.chr_ram[(address as usize) % len] = data; }
        } else if address >= 0x2000 && address < 0x3F00 {
            let h = self.dip_switches == 0x18;
            let mir = mirror_h_or_v(!h, address);
            vram[(mir & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.irq_enabled {
            self.irq_counter = self.irq_counter.wrapping_add(1);
            if self.irq_counter & 0xFFF == 0 {
                return true;
            }
        }
        false
    }

    fn get_dip_switches(&self) -> u8 { self.dip_switches }
    fn set_dip_switches(&mut self, value: u8) { self.dip_switches = value; }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = vec![self.bank_smb2j, self.bank_unrom, self.bank_swap];
        state.push(if self.irq_enabled { 1 } else { 0 });
        state.extend_from_slice(&self.irq_counter.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.bank_smb2j = state[p]; p += 1; }
        if p < state.len() { self.bank_unrom = state[p]; p += 1; }
        if p < state.len() { self.bank_swap = state[p]; p += 1; }
        if p < state.len() { self.irq_enabled = state[p] != 0; p += 1; }
        if p + 2 <= state.len() {
            self.irq_counter = u16::from_le_bytes([state[p], state[p+1]]);
            p += 2;
        }
        p
    }
}
