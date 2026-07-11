use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper305 {
    reg: [u8; 4],
}

impl Mapper305 {
    pub fn new() -> Self {
        Self { reg: [0; 4] }
    }
}

impl Mapper for Mapper305 {
    fn reset(&mut self) {
        self.reg = [0; 4];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = (15 - ((address as usize >> 11) & 0xF)) as usize;
            let offset = bank * 0x800 + (address as usize & 0x7FF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0x6000 {
            let idx = ((address - 0x6000) as usize >> 11) & 3;
            let bank = self.reg[idx] as usize;
            let offset = bank * 0x800 + (address as usize & 0x7FF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0x4020 {
            FetchResult { data: 0x40, driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let index = (((address >> 12) & 1) << 1) | ((address >> 11) & 1);
            self.reg[index as usize] = data & 0x3F;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else if cart.nametable_horizontal_mirroring {
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
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            if using_chr_ram && !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[address as usize & 0x1FFF] as u16;
            } else if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[address as usize % chr_rom.len()] as u16;
            }
        } else {
            let h = if alternative_nametable_arrangement {
                false
            } else {
                nametable_horizontal_mirroring
            };
            let mirrored = if h {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let mask = cart.chr_ram.len() - 1;
                cart.chr_ram[(address as usize) & mask] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.reg);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for r in self.reg.iter_mut() {
            *r = state.get(p).copied().unwrap_or(0);
            p += 1;
        }
        p
    }
}
