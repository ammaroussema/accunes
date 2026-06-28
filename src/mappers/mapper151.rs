use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const VS_FRAME_CYCLES: u64 = 29780;

pub struct Mapper151 {
    prg_banks: [u8; 3],
    chr_banks: [u8; 2],
    misc: u8,
    vsdip: u8,
    coinon: u8,
    coinon2: u8,
    service: u8,
    cycle_accum: u64,
}

impl Mapper151 {
    pub fn new() -> Self {
        Self {
            prg_banks: [0, 0, 0],
            chr_banks: [0, 0],
            misc: 0,
            vsdip: 0,
            coinon: 0,
            coinon2: 0,
            service: 0,
            cycle_accum: 0,
        }
    }
}

impl Mapper for Mapper151 {
    fn reset(&mut self) {
        self.prg_banks = [0, 0, 0];
        self.chr_banks = [0, 0];
        self.misc = 0;
        self.coinon = 0;
        self.coinon2 = 0;
        self.service = 0;
        self.cycle_accum = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }
        let num_8k = cart.prg_rom.len() / 0x2000;
        if num_8k == 0 {
            return FetchResult { data: 0, driven: true };
        }
        let slot = ((address as usize - 0x8000) >> 13) & 3;
        let bank = if slot == 3 {
            num_8k - 1
        } else {
            (self.prg_banks[slot] & 0x1F) as usize % num_8k
        };
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        FetchResult {
            data: cart.prg_rom[offset % cart.prg_rom.len()],
            driven: true,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        match address & 0xF000 {
            0x8000 => self.prg_banks[0] = data,
            0x9000 => self.misc = data,
            0xA000 => self.prg_banks[1] = data,
            0xC000 => self.prg_banks[2] = data,
            0xE000 => self.chr_banks[0] = data & 0x0F,
            0xF000 => self.chr_banks[1] = data & 0x0F,
            _ => {}
        }
    }

    fn adjust_controller_read(&self, address: u16, value: u8) -> u8 {
        if address & 0x1F == 0x16 {
            let mut vs = value & 0x01;
            if self.service > 0 { vs |= 0x04; }
            vs |= (self.vsdip & 0x03) << 3;
            if self.coinon > 0 { vs |= 0x20; }
            if self.coinon2 > 0 { vs |= 0x40; }
            vs
        } else if address & 0x1F == 0x17 {
            (value & 0x01) | (self.vsdip & 0xFC)
        } else {
            value
        }
    }

    fn insert_coin(&mut self, coin: u8) {
        match coin {
            0 => self.coinon = 6,
            1 => self.coinon2 = 6,
            _ => {}
        }
    }

    fn service_button(&mut self) {
        self.service = 6;
    }

    fn get_dip_switches(&self) -> u8 {
        self.vsdip
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.vsdip = value;
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        address
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
            let slot = (address >> 12) & 1;
            let bank = ((self.chr_banks[slot as usize] & 0x0F) | ((self.misc << (3 - slot as u8)) & 0x10)) as usize;
            let offset = bank * 0x1000 + (address as usize & 0x0FFF);
            let byte = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[offset % chr_ram.len()] }
            } else if chr_rom.is_empty() {
                0
            } else {
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            new_addr_bus |= vram[(address & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let slot = (address >> 12) & 1;
            let bank = ((self.chr_banks[slot as usize] & 0x0F) | ((self.misc << (3 - slot as u8)) & 0x10)) as usize;
            let offset = bank * 0x1000 + (address as usize & 0x0FFF);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            vram[(address & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.cycle_accum += _cycles as u64;
        if self.cycle_accum >= VS_FRAME_CYCLES {
            self.cycle_accum = 0;
            if self.coinon > 0 { self.coinon -= 1; }
            if self.coinon2 > 0 { self.coinon2 -= 1; }
            if self.service > 0 { self.service -= 1; }
        }
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg_banks);
        state.extend_from_slice(&self.chr_banks);
        state.push(self.misc);
        state.push(self.vsdip);
        state.push(self.coinon);
        state.push(self.coinon2);
        state.push(self.service);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut i = start;
        for r in 0..3 { self.prg_banks[r] = state[i]; i += 1; }
        for r in 0..2 { self.chr_banks[r] = state[i]; i += 1; }
        self.misc = state[i]; i += 1;
        self.vsdip = if i < state.len() { state[i] } else { 0 }; i += 1;
        self.coinon = if i < state.len() { state[i] } else { 0 }; i += 1;
        self.coinon2 = if i < state.len() { state[i] } else { 0 }; i += 1;
        self.service = if i < state.len() { state[i] } else { 0 }; i += 1;
        i - start
    }
}
