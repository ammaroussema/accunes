use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper297 {
    mode: u8,
    latch: u8,
    shift: u8,
    write_count: u8,
    control: u8,
    chr0: u8,
    chr1: u8,
    prg: u8,
}

impl Mapper297 {
    pub fn new() -> Self {
        Self {
            mode: 0,
            latch: 0,
            shift: 0,
            write_count: 0,
            control: 0x0C,
            chr0: 0,
            chr1: 0,
            prg: 0,
        }
    }

    fn prg_bank(&self, slot: u8, cart: &Cartridge) -> usize {
        let num_16k = (cart.prg_rom.len() / 0x4000).max(1);
        if self.mode & 1 != 0 {
            let adj = (self.prg & 0x07) | 0x08;
            match (self.control >> 2) & 3 {
                0 | 1 => (adj as usize >> 1) % num_16k,
                2 => {
                    if slot == 0 { 0 } else { adj as usize % num_16k }
                }
                _ => {
                    if slot == 0 { adj as usize % num_16k } else { (0x07 | 0x08) as usize % num_16k }
                }
            }
        } else {
            let bank = ((self.mode & 2) << 1) | (if slot == 0 { (self.latch >> 4) & 3 } else { 3 });
            (bank as usize) % num_16k
        }
    }
}

impl Mapper for Mapper297 {
    fn reset(&mut self) {
        self.mode = 0;
        self.latch = 0;
        self.shift = 0;
        self.write_count = 0;
        self.control = 0x0C;
        self.chr0 = 0;
        self.chr1 = 0;
        self.prg = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                return FetchResult { data: cart.prg_ram[(address & 0x1FFF) as usize], driven: true };
            }
            return FetchResult { data: 0, driven: false };
        }
        if address >= 0x8000 {
            let slot = if address < 0xC000 { 0 } else { 1 };
            let bank = self.prg_bank(slot, cart);
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
        if address >= 0x5000 && address <= 0x5FFF {
            self.mode = (address as u8) & 3;
            return;
        }
        if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                cart.prg_ram[(address & 0x1FFF) as usize] = data;
            }
            return;
        }
        if address >= 0x8000 {
            if self.mode & 1 != 0 {
                if data & 0x80 != 0 {
                    self.control |= 0x0C;
                    self.shift = 0;
                    self.write_count = 0;
                    return;
                }
                let done = (self.shift & 1) != 0;
                self.shift >>= 1;
                self.shift |= (data & 1) << 4;
                self.write_count += 1;
                if done || self.write_count >= 5 {
                    match ((address >> 13) & 3) as u8 {
                        0 => self.control = self.shift,
                        1 => self.chr0 = self.shift,
                        2 => self.chr1 = self.shift,
                        _ => self.prg = self.shift,
                    }
                    self.shift = 0;
                    self.write_count = 0;
                }
            } else {
                self.latch = data;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mode & 1 != 0 {
            match self.control & 3 {
                0 => address & 0x3FFF,
                1 => (address & 0x3FFF) | 0x400,
                2 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                _ => address & 0x37FF,
            }
        } else {
            address & 0x33FF
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
            let bank = if self.mode & 1 != 0 {
                let chr_mode = (self.control >> 4) & 1;
                if chr_mode == 0 {
                    ((self.chr0 & 0x1F) | 0x20) as usize >> 1
                } else if address < 0x1000 {
                    ((self.chr0 & 0x1F) | 0x20) as usize
                } else {
                    ((self.chr1 & 0x1F) | 0x20) as usize
                }
            } else {
                (self.latch & 0x0F) as usize
            };
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if self.mode & 1 != 0 {
                match self.control & 3 {
                    0 => address & 0x3FFF,
                    1 => (address & 0x3FFF) | 0x400,
                    2 => (address & 0x33FF) | ((address & 0x0800) >> 1),
                    _ => address & 0x37FF,
                }
            } else {
                address & 0x33FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let bank = if self.mode & 1 != 0 {
                    let chr_mode = (self.control >> 4) & 1;
                    if chr_mode == 0 {
                        ((self.chr0 & 0x1F) | 0x20) as usize >> 1
                    } else if address < 0x1000 {
                        ((self.chr0 & 0x1F) | 0x20) as usize
                    } else {
                        ((self.chr1 & 0x1F) | 0x20) as usize
                    }
                } else {
                    (self.latch & 0x0F) as usize
                };
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.mode, self.latch, self.shift, self.write_count, self.control, self.chr0, self.chr1, self.prg]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.mode = state[p]; p += 1; }
        if p < state.len() { self.latch = state[p]; p += 1; }
        if p < state.len() { self.shift = state[p]; p += 1; }
        if p < state.len() { self.write_count = state[p]; p += 1; }
        if p < state.len() { self.control = state[p]; p += 1; }
        if p < state.len() { self.chr0 = state[p]; p += 1; }
        if p < state.len() { self.chr1 = state[p]; p += 1; }
        if p < state.len() { self.prg = state[p]; p += 1; }
        p
    }
}
