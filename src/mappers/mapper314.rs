use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper314 {
    reg: [u8; 4],
    latch_data: u8,
    has_chr_rom: bool,
}

impl Mapper314 {
    pub fn new(has_chr_rom: bool) -> Self {
        Self {
            reg: [0x80, 0x43, 0x00, 0x00],
            latch_data: 0,
            has_chr_rom,
        }
    }
}

impl Mapper for Mapper314 {
    fn reset(&mut self) {
        self.reg = [0x80, 0x43, 0x00, 0x00];
        self.latch_data = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let prg = (self.reg[0] as usize) << 7 & 0x80
                | (self.reg[1] as usize) << 1 & 0x7E
                | (self.reg[1] as usize) >> 6 & 0x01;
            let bank = if (self.reg[0] & 0x80) != 0 {
                if (self.reg[1] & 0x80) != 0 {
                    prg >> 1
                } else {
                    prg
                }
            } else if address < 0xC000 {
                (prg & !7) | (self.latch_data as usize & 7)
            } else {
                prg | 7
            };
            let (page_size, address_mask) = if (self.reg[0] & 0x80) != 0 && (self.reg[1] & 0x80) != 0 {
                (0x8000, 0x7FFF)
            } else {
                (0x4000, 0x3FFF)
            };
            let offset = bank * page_size + (address as usize & address_mask);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len().max(1)],
                driven: true,
            }
        } else if address >= 0x6000 {
            let offset = address as usize & 0x1FFF;
            FetchResult {
                data: if offset < cart.prg_ram.len() { cart.prg_ram[offset] } else { 0 },
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x6000 {
            let idx = (address as usize) & (if self.has_chr_rom { 3 } else { 1 });
            self.reg[idx] = data;
        } else if address >= 0x8000 {
            self.latch_data = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if (self.reg[0] & 0x20) != 0 {
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
            let bank = ((self.reg[2] as usize) << 2) | ((self.reg[0] as usize) >> 1 & 3);
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let byte = if !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let horz = (self.reg[0] & 0x20) != 0;
            let mirrored = if horz {
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
            let bank = ((self.reg[2] as usize) << 2) | ((self.reg[0] as usize) >> 1 & 3);
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let len = cart.chr_ram.len();
            cart.chr_ram[offset % len] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = self.reg.to_vec();
        state.push(self.latch_data);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..4 {
            self.reg[i] = state.get(p).copied().unwrap_or(if i == 0 { 0x80 } else { 0 });
            p += 1;
        }
        self.latch_data = state.get(p).copied().unwrap_or(0);
        p + 1
    }
}
