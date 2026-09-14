use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper65 {
    prg_banks: [u8; 3],   
    chr_banks: [u8; 8],   
    mirroring_vertical: bool,
    irq_enabled: bool,
    irq_counter: u16,
    irq_reload_value: u16,
    irq_pending: bool,
}

impl Mapper65 {
    pub fn new(header_horizontal_mirror: bool) -> Self {
        Self {
            prg_banks: [0, 1, 0xFE],
            chr_banks: [0; 8],
            mirroring_vertical: !header_horizontal_mirror,
            irq_enabled: false,
            irq_counter: 0,
            irq_reload_value: 0,
            irq_pending: false,
        }
    }
}

impl Mapper for Mapper65 {
    fn reset(&mut self) {
        self.prg_banks = [0, 1, 0xFE];
        self.chr_banks = [0; 8];
        self.irq_enabled = false;
        self.irq_counter = 0;
        self.irq_reload_value = 0;
        self.irq_pending = false;
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
            self.prg_banks[slot] as usize % num_8k
        };
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        FetchResult {
            data: cart.prg_rom[offset % cart.prg_rom.len()],
            driven: true,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            return;
        }
        match address {
            0x8000 => self.prg_banks[0] = data,
            0x9001 => {
                self.mirroring_vertical = (data & 0x80) == 0;
            }
            0x9003 => {
                self.irq_enabled = (data & 0x80) != 0;
                self.irq_pending = false; 
            }
            0x9004 => {
                self.irq_counter = self.irq_reload_value;
                self.irq_pending = false; 
            }
            0x9005 => {
                self.irq_reload_value = (self.irq_reload_value & 0x00FF) | ((data as u16) << 8);
            }
            0x9006 => {
                self.irq_reload_value = (self.irq_reload_value & 0xFF00) | (data as u16);
            }
            0xA000 => self.prg_banks[1] = data,
            0xB000 => self.chr_banks[0] = data,
            0xB001 => self.chr_banks[1] = data,
            0xB002 => self.chr_banks[2] = data,
            0xB003 => self.chr_banks[3] = data,
            0xB004 => self.chr_banks[4] = data,
            0xB005 => self.chr_banks[5] = data,
            0xB006 => self.chr_banks[6] = data,
            0xB007 => self.chr_banks[7] = data,
            0xC000 => self.prg_banks[2] = data,
            _ => {}
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mirroring_vertical {
            address & 0x37FF
        } else {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        }
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
            let slot = (address >> 10) & 7;
            let bank = self.chr_banks[slot as usize] as usize;
            let offset = bank * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[offset % chr_ram.len()] }
            } else if chr_rom.is_empty() {
                0
            } else {
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if self.mirroring_vertical {
                address & 0x37FF
            } else {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let slot = (address >> 10) & 7;
            let bank = self.chr_banks[slot as usize] as usize;
            let offset = bank * 0x0400 + (address as usize & 0x03FF);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.irq_enabled {
            if self.irq_counter == 0 {
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
        state.extend_from_slice(&self.prg_banks);
        state.extend_from_slice(&self.chr_banks);
        state.push(if self.mirroring_vertical { 1 } else { 0 });
        state.push(if self.irq_enabled { 1 } else { 0 });
        state.push((self.irq_counter >> 8) as u8);
        state.push(self.irq_counter as u8);
        state.push((self.irq_reload_value >> 8) as u8);
        state.push(self.irq_reload_value as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut i = start;
        for r in 0..3 { self.prg_banks[r] = state[i]; i += 1; }
        for r in 0..8 { self.chr_banks[r] = state[i]; i += 1; }
        self.mirroring_vertical = state[i] != 0; i += 1;
        self.irq_enabled = state[i] != 0; i += 1;
        self.irq_counter = ((state[i] as u16) << 8) | state[i + 1] as u16; i += 2;
        self.irq_reload_value = ((state[i] as u16) << 8) | state[i + 1] as u16; i += 2;
        i - start
    }
}
