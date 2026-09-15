use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper246 {
    reg: [u8; 8],
}

impl Mapper246 {
    pub fn new() -> Self {
        Self { reg: [0xFF; 8] }
    }
}

impl Mapper for Mapper246 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0xE000 {
            let bank = if (address as usize & 0xFE4) == 0xFE4 {
                (self.reg[3] | 0x10) as usize
            } else {
                self.reg[3] as usize
            };
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
        } else if address >= 0xC000 {
            let bank = self.reg[2] as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
        } else if address >= 0xA000 {
            let bank = self.reg[1] as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
        } else if address >= 0x8000 {
            let bank = self.reg[0] as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
        } else if address >= 0x7000 {
            FetchResult { data: 0, driven: false } 
        } else if address >= 0x6000 {
            let is_ram = (address & 0x800) != 0;
            if is_ram {
                let off = (address & 0x7FF) as usize;
                if off < cart.prg_ram.len() {
                    FetchResult { data: cart.prg_ram[off], driven: true }
                } else {
                    FetchResult { data: 0, driven: false }
                }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            let addr_in_page = address & 0xFFF;
            if (addr_in_page & 0x800) != 0 {
                let off = (addr_in_page & 0x7FF) as usize;
                if off < cart.prg_ram.len() {
                    cart.prg_ram[off] = data;
                }
            } else if (addr_in_page & 0xFE0) == 0 {
                self.reg[(addr_in_page & 7) as usize] = data;
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.nametable_horizontal_mirroring {
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
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank_idx = (address >> 11) as usize;
            let bank = self.reg[4 + bank_idx] as usize;
            let offset = bank * 0x800 + (address as usize & 0x7FF);
            let data = if using_chr_ram { chr_ram[offset % chr_ram.len()] } else { chr_rom[offset % chr_rom.len()] };
            new_addr_bus |= data as u16;
        } else {
            let mirrored = if !nametable_horizontal_mirroring { address & 0x37FF } else { (address & 0x33FF) | ((address & 0x0800) >> 1) };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[address as usize % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() { cart.prg_vram[idx] = data; }
            } else {
                vram[mirrored as usize & 0x7FF] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        self.reg.to_vec()
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..8 {
            if p < state.len() { self.reg[i] = state[p]; p += 1; }
        }
        p
    }

    fn reset(&mut self) {
        self.reg = [0xFF; 8];
    }
}
