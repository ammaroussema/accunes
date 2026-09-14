use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const VS_FRAME_CYCLES: u64 = 29780;

pub struct Mapper67 {
    prg_bank: u8,
    chr_banks: [u8; 4],
    mirroring: u8, 
    irq_latch: bool,
    irq_enabled: bool,
    irq_counter: u16,
    irq_pending: bool,
    vsdip: u8,
    coinon: u8,
    coinon2: u8,
    service: u8,
    cycle_accum: u64,
}

impl Mapper67 {
    pub fn new() -> Self {
        Self {
            prg_bank: 0,
            chr_banks: [0; 4],
            mirroring: 0,
            irq_latch: false,
            irq_enabled: false,
            irq_counter: 0,
            irq_pending: false,
            vsdip: 0,
            coinon: 0,
            coinon2: 0,
            service: 0,
            cycle_accum: 0,
        }
    }

    fn mirror_address(&self, address: u16) -> u16 {
        match self.mirroring {
            0 => address & 0x37FF, 
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1), 
            2 => address & 0x33FF, 
            3 => (address & 0x33FF) | 0x0400, 
            _ => address & 0x37FF,
        }
    }
}

impl Mapper for Mapper67 {
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

    fn reset(&mut self) {
        self.prg_bank = 0;
        self.chr_banks = [0; 4];
        self.mirroring = 0;
        self.irq_latch = false;
        self.irq_enabled = false;
        self.irq_counter = 0;
        self.irq_pending = false;
        self.vsdip = 0;
        self.coinon = 0;
        self.coinon2 = 0;
        self.service = 0;
        self.cycle_accum = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }
        let num_16k = cart.prg_rom.len() / 0x4000;
        if num_16k == 0 {
            return FetchResult { data: 0, driven: true };
        }
        let bank = if address >= 0xC000 {
            num_16k - 1
        } else {
            self.prg_bank as usize % num_16k
        };
        let offset = bank * 0x4000 + (address as usize & 0x3FFF);
        FetchResult {
            data: cart.prg_rom[offset % cart.prg_rom.len()],
            driven: true,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            return;
        }
        match address & 0xF800 {
            0x8800 => self.chr_banks[0] = data,
            0x9800 => self.chr_banks[1] = data,
            0xA800 => self.chr_banks[2] = data,
            0xB800 => self.chr_banks[3] = data,
            0xC800 => {
                self.irq_counter = if self.irq_latch {
                    (self.irq_counter & 0xFF00) | (data as u16)
                } else {
                    (self.irq_counter & 0x00FF) | ((data as u16) << 8)
                };
                self.irq_latch = !self.irq_latch;
            }
            0xD800 => {
                self.irq_enabled = (data & 0x10) != 0;
                self.irq_latch = false;
                self.irq_pending = false;
            }
            0xE800 => {
                self.mirroring = data & 0x03;
            }
            0xF800 => {
                self.prg_bank = data;
            }
            _ => {}
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror_address(address)
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
            let slot = (address >> 11) & 3;
            let bank = self.chr_banks[slot as usize] as usize;
            let offset = bank * 0x0800 + (address as usize & 0x07FF);
            let byte = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[offset % chr_ram.len()] }
            } else if chr_rom.is_empty() {
                0
            } else {
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = self.mirror_address(address);
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let slot = (address >> 11) & 3;
            let bank = self.chr_banks[slot as usize] as usize;
            let offset = bank * 0x0800 + (address as usize & 0x07FF);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_address(address);
            vram[(mirrored & 0x7FF) as usize] = data;
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
        if self.irq_enabled {
            if self.irq_counter == 0 {
                self.irq_counter = 0xFFFF;
                self.irq_enabled = false;
                self.irq_pending = true;
            } else {
                self.irq_counter -= 1;
            }
        }
        let fired = self.irq_pending;
        if fired {
            self.irq_pending = false;
        }
        fired
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.prg_bank);
        state.extend_from_slice(&self.chr_banks);
        state.push(self.mirroring);
        state.push(if self.irq_latch { 1 } else { 0 });
        state.push(if self.irq_enabled { 1 } else { 0 });
        state.push((self.irq_counter >> 8) as u8);
        state.push(self.irq_counter as u8);
        state.push(self.vsdip);
        state.push(self.coinon);
        state.push(self.coinon2);
        state.push(self.service);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut i = start;
        self.prg_bank = state[i]; i += 1;
        for r in 0..4 { self.chr_banks[r] = state[i]; i += 1; }
        self.mirroring = state[i]; i += 1;
        self.irq_latch = state[i] != 0; i += 1;
        self.irq_enabled = state[i] != 0; i += 1;
        self.irq_counter = ((state[i] as u16) << 8) | state[i + 1] as u16; i += 2;
        self.vsdip = state.get(i).copied().unwrap_or(0); i += 1;
        self.coinon = state.get(i).copied().unwrap_or(0); i += 1;
        self.coinon2 = state.get(i).copied().unwrap_or(0); i += 1;
        self.service = state.get(i).copied().unwrap_or(0); i += 1;
        i - start
    }
}
