use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper280 {
    addr: u16,
    data: u8,
    submapper: u8,
    prg_bank0: usize,
    prg_bank1: usize,
}

impl Mapper280 {
    pub fn new(submapper_id: u8) -> Self {
        Self {
            addr: 0,
            data: 0,
            submapper: submapper_id,
            prg_bank0: 0,
            prg_bank1: 0,
        }
    }

    fn sync(&mut self, num_16k: usize) {
        if num_16k == 0 {
            return;
        }
        if self.addr & 0x100 != 0 {
            if self.submapper == 1 {
                self.prg_bank0 = (((self.addr as usize >> 2) & 7) | 0x20) % num_16k;
            } else {
                self.prg_bank0 = (0x20 | (self.data as usize & 7)) % num_16k;
            }
            self.prg_bank1 = 0x27 % num_16k;
        } else if self.addr & 0x080 != 0 {
            if self.addr & 0x001 != 0 {
                let bank = ((self.addr as usize >> 3) & 0x0F) % num_16k;
                self.prg_bank0 = bank;
                self.prg_bank1 = bank;
            } else {
                let bank = ((self.addr as usize >> 2) & 0x1F) % num_16k;
                self.prg_bank0 = bank;
                self.prg_bank1 = bank;
            }
        } else {
            self.prg_bank0 = ((self.addr as usize >> 2) % num_16k).max(0);
            self.prg_bank1 = 0;
        }
    }

    fn mirror_h(&self) -> bool {
        self.addr & 2 != 0
    }

    fn mirror_address(&self, address: u16) -> u16 {
        if self.mirror_h() {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }
}

impl Mapper for Mapper280 {
    fn reset(&mut self) {
        self.addr = 0;
        self.data = 0;
        self.prg_bank0 = 0;
        self.prg_bank1 = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let num_16k = cart.prg_rom.len() / 0x4000;
            if num_16k == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let bank = match address {
                0x8000..=0xBFFF => self.prg_bank0 % num_16k,
                0xC000..=0xFFFF => self.prg_bank1 % num_16k,
                _ => 0,
            };
            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.addr = (address & !0x100) | (0x00 & 0x100);
            self.data = data;
            self.sync(cart.prg_rom.len() / 0x4000);
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
            let byte = if !chr_ram.is_empty() {
                chr_ram[address as usize & 0x1FFF]
            } else if !chr_rom.is_empty() {
                chr_rom[address as usize & 0x1FFF]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = self.mirror_address(address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                cart.chr_ram[address as usize & 0x1FFF] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_address(address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.addr.to_le_bytes());
        state.push(self.data);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.addr = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.data = state[p];
            p += 1;
        }
        self.sync(cart.prg_rom.len() / 0x4000);
        p
    }
}
