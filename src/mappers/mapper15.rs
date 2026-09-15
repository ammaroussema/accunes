use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper15 {
    latch_addr: u16,
    latched: u8,
    chr_write_protect: bool,
}

impl Mapper15 {
    pub fn new() -> Self {
        Self {
            latch_addr: 0x8000,
            latched: 0,
            chr_write_protect: false,
        }
    }
}

impl Mapper for Mapper15 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let mode = self.latch_addr & 3;
            let bank_num = (address - 0x8000) >> 13; 
            let bank = match mode {
                0 => ((self.latched & 0x3F) << 1) as usize + bank_num as usize,
                2 => ((self.latched & 0x3F) << 1) as usize + ((self.latched >> 7) as usize),
                1 | 3 => {
                    let mut b = (self.latched & 0x3F) as usize;
                    if bank_num >= 2 && (self.latch_addr & 2) == 0 {
                        b |= 0x07;
                    }
                    (bank_num & 1) as usize + (b << 1)
                }
                _ => 0,
            };
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if address >= 0x6000 {
            let offset = (address - 0x6000) as usize;
            if offset < cart.prg_ram.len() {
                FetchResult {
                    data: cart.prg_ram[offset],
                    driven: true,
                }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.latch_addr = address;
            self.latched = data;
            self.chr_write_protect = (self.latch_addr & 3) == 3;
        } else if address >= 0x6000 {
            let offset = (address - 0x6000) as usize;
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if ((self.latched >> 6) & 1) == 0 {
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
            let offset = address as usize & 0x1FFF;
            if using_chr_ram {
                if !chr_ram.is_empty() {
                    new_addr_bus |= chr_ram[offset % chr_ram.len()] as u16;
                }
            } else if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else {
            let mirrored = if ((self.latched >> 6) & 1) == 0 {
                address & 0x37FF 
            } else {
                (address & 0x33FF) | ((address & 0x0800) >> 1) 
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            let chr_ram_len = cart.chr_ram.len();
            if cart.using_chr_ram && chr_ram_len > 0 {
                cart.chr_ram[address as usize % chr_ram_len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[mirrored as usize & 0x7FF] = data;
            }
        }
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.latch_addr.to_le_bytes());
        state.push(self.latched);
        state.extend_from_slice(&cart.prg_ram);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.latch_addr = u16::from_le_bytes([state[p], state[p + 1]]);
        p += 2;
        self.latched = state[p];
        p += 1;
        for i in 0..cart.prg_ram.len() {
            if p < state.len() {
                cart.prg_ram[i] = state[p];
                p += 1;
            }
        }
        p
    }

    fn reset(&mut self) {
        self.latch_addr = 0x8000;
        self.latched = 0;
        self.chr_write_protect = false;
    }
}
