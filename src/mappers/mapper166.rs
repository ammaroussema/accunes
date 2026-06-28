use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper166 {
    reg: [u8; 4],
}

impl Mapper166 {
    pub fn new() -> Self {
        Self { reg: [0; 4] }
    }
}

fn sync_prg(reg: &[u8; 4], address: u16, prg_rom: &[u8]) -> u8 {
    let prg = ((reg[0] ^ reg[1]) << 1 & 0x20) | (reg[2] ^ reg[3]) & 0x1F;
    let (bank16, is_upper) = match (reg[1] >> 2) & 3 {
        0 => (prg, false),
        1 => (0x1F, true),
        _ => if address < 0xC000 { (prg & !1, false) } else { (prg | 1, true) },
    };
    if is_upper {
        let offset = bank16 as usize * 0x4000 + (address as usize & 0x3FFF);
        prg_rom[offset % prg_rom.len()]
    } else {
        let offset = bank16 as usize * 0x4000 + (address as usize & 0x3FFF);
        prg_rom[offset % prg_rom.len()]
    }
}

impl Mapper for Mapper166 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            FetchResult { data: sync_prg(&self.reg, address, &cart.prg_rom), driven: true }
        } else if address >= 0x6000 {
            let off = (address - 0x6000) as usize;
            if off < cart.prg_ram.len() {
                FetchResult { data: cart.prg_ram[off], driven: true }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let idx = ((address >> 13) & 3) as usize;
            self.reg[idx] = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let h = (self.reg[0] & 1) != 0;
        if h {
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
            let data = if using_chr_ram && !chr_ram.is_empty() { chr_ram[address as usize % chr_ram.len()] } else { chr_rom[address as usize % chr_rom.len()] };
            new_addr_bus |= data as u16;
        } else {
            let h = (self.reg[0] & 1) != 0;
            let mirrored = if h { (address & 0x33FF) | ((address & 0x0800) >> 1) } else { address & 0x37FF };
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
        for i in 0..4 {
            if p < state.len() { self.reg[i] = state[p]; p += 1; }
        }
        p
    }

    fn reset(&mut self) {
        self.reg = [0; 4];
    }
}
