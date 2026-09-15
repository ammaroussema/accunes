use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper237 {
    addr: u16,
    data: u8,
    addr_locked: u16,
    data_locked: u8,
}

impl Mapper237 {
    pub fn new() -> Self {
        Self { addr: 0, data: 0, addr_locked: 0, data_locked: 0 }
    }

    fn prg_bank(&self) -> usize {
        (self.data & 0x1F) as usize | (((self.addr << 3) as usize) & 0x20)
    }
}

impl Mapper for Mapper237 {
    fn reset(&mut self) {
        self.addr = 0;
        self.data = 0;
        self.addr_locked = 0;
        self.data_locked = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if self.addr & 0x01 != 0 {
                return FetchResult { data: 0, driven: true };
            }
            let prg = self.prg_bank();
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult { data: 0, driven: false };
            }
            let offset = if self.data & 0x80 != 0 {
                if self.data & 0x40 != 0 {
                    (prg >> 1) * 0x8000 + (address as usize - 0x8000)
                } else {
                    prg * 0x4000 + (address as usize & 0x3FFF)
                }
            } else {
                if address < 0xC000 {
                    prg * 0x4000 + (address as usize & 0x3FFF)
                } else {
                    (prg | 7) * 0x4000 + (address as usize & 0x3FFF)
                }
            };
            FetchResult { data: cart.prg_rom[offset % len], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.addr = (self.addr & self.addr_locked) | (address & !self.addr_locked);
            self.data = (self.data & self.data_locked) | (data & !self.data_locked);
            if self.addr & 0x02 != 0 {
                self.addr_locked = 0xFF;
                self.data_locked = 0xF8;
            } else {
                self.addr_locked = 0;
                self.data_locked = 0;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.data & 0x20 != 0 {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
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
            let len = chr_ram.len();
            if len == 0 {
                return (0, new_addr_bus);
            }
            new_addr_bus |= chr_ram[(address as usize & 0x1FFF) % len] as u16;
        } else {
            let mirrored = if self.data & 0x20 != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut v = Vec::new();
        v.push(self.addr as u8);
        v.push((self.addr >> 8) as u8);
        v.push(self.data);
        v.push(self.addr_locked as u8);
        v.push((self.addr_locked >> 8) as u8);
        v.push(self.data_locked);
        v
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.addr = state[p] as u16; p += 1; }
        if p < state.len() { self.addr |= (state[p] as u16) << 8; p += 1; }
        if p < state.len() { self.data = state[p]; p += 1; }
        if p < state.len() { self.addr_locked = state[p] as u16; p += 1; }
        if p < state.len() { self.addr_locked |= (state[p] as u16) << 8; p += 1; }
        if p < state.len() { self.data_locked = state[p]; p += 1; }
        p
    }
}
