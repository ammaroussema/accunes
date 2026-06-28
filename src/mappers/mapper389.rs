use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper389 {
    prg_hi: u8,
    chr_hi: u8,
    inner_bank: u8,
    horizontal_mirroring: bool,
}

impl Mapper389 {
    pub fn new() -> Self {
        let mut m = Mapper389 {
            prg_hi: 0,
            chr_hi: 0,
            inner_bank: 0,
            horizontal_mirroring: false,
        };
        m.reset();
        m
    }

    fn sync(&mut self) {
        self.horizontal_mirroring = (self.prg_hi & 1) != 0;
    }

    fn prg_bank_16k_low(&self, cart: &Cartridge) -> usize {
        let bank = ((self.prg_hi >> 2) as usize) | ((self.inner_bank >> 2) & 3) as usize;
        let bank_count = (cart.prg_rom.len() / 0x4000).max(1);
        bank % bank_count
    }

    fn prg_bank_16k_high(&self, cart: &Cartridge) -> usize {
        let bank = ((self.prg_hi >> 2) as usize) | 3;
        let bank_count = (cart.prg_rom.len() / 0x4000).max(1);
        bank % bank_count
    }

    fn prg_bank_32k(&self, cart: &Cartridge) -> usize {
        let bank = (self.prg_hi >> 3) as usize;
        let bank_count = (cart.prg_rom.len() / 0x8000).max(1);
        bank % bank_count
    }

    fn get_chr_bank(&self) -> usize {
        ((self.chr_hi >> 1) & 0xFC) as usize | (self.inner_bank & 0x03) as usize
    }
}

impl Mapper for Mapper389 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let (bank, bank_size) = if (self.chr_hi & 2) != 0 {
                if address < 0xC000 {
                    (self.prg_bank_16k_low(cart), 0x4000)
                } else {
                    (self.prg_bank_16k_high(cart), 0x4000)
                }
            } else {
                (self.prg_bank_32k(cart), 0x8000)
            };
            let offset = bank * bank_size + (address as usize & (bank_size - 1));
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if address >= 0x6000 {
            let offset = (address - 0x6000) as usize;
            if offset < cart.prg_ram.len() {
                FetchResult { data: cart.prg_ram[offset], driven: true }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        let _ = data; 
        if address >= 0x8000 && address <= 0x8FFF {
            self.prg_hi = (address & 0xFF) as u8;
            self.sync();
        } else if address >= 0x9000 && address <= 0x9FFF {
            self.chr_hi = (address & 0xFF) as u8;
            self.sync();
        } else if address >= 0xA000 {
            self.inner_bank = (address & 0xFF) as u8;
            self.sync();
        } else if address >= 0x6000 {
            let offset = (address - 0x6000) as usize;
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else if self.horizontal_mirroring {
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
            let chr_data = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[address as usize & 0x1FFF] }
            } else {
                let bank = self.get_chr_bank();
                let bank_count = (chr_rom.len() / 0x2000).max(1);
                let bank = bank % bank_count;
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                if chr_rom.is_empty() { 0 } else { chr_rom[offset % chr_rom.len()] }
            };
            new_addr_bus |= chr_data as u16;
        } else if address < 0x3F00 {
            let mirrored = if self.horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn reset(&mut self) {
        self.prg_hi = 0;
        self.chr_hi = 0;
        self.inner_bank = 0;
        self.sync();
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.push(self.prg_hi);
        state.push(self.chr_hi);
        state.push(self.inner_bank);
        state.push(self.horizontal_mirroring as u8);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        let prg_len = cart.prg_ram.len();
        if start + prg_len <= state.len() {
            cart.prg_ram.copy_from_slice(&state[start..start + prg_len]);
            start += prg_len;
        }
        let chr_len = cart.chr_ram.len();
        if start + chr_len <= state.len() {
            cart.chr_ram.copy_from_slice(&state[start..start + chr_len]);
            start += chr_len;
        }
        if start < state.len() {
            self.prg_hi = state[start];
            start += 1;
        }
        if start < state.len() {
            self.chr_hi = state[start];
            start += 1;
        }
        if start < state.len() {
            self.inner_bank = state[start];
            start += 1;
        }
        if start < state.len() {
            self.horizontal_mirroring = state[start] != 0;
            start += 1;
        } else {
            self.sync();
        }
        start
    }
}
