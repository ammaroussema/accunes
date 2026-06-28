use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper265 {
    latch_addr: u16,
    latch_data: u8,
}

impl Mapper265 {
    pub fn new() -> Self {
        Self { latch_addr: 0, latch_data: 0 }
    }

    fn prg(&self) -> u8 {
        (self.latch_data & 0x07) | ((self.latch_addr >> 2) as u8 & 0x18) | ((self.latch_addr >> 3) as u8 & 0xE0)
    }

    fn cpu_a14(&self) -> bool { (self.latch_addr & 0x0001) != 0 }
    fn mirror_h(&self) -> bool { (self.latch_addr & 0x0002) != 0 }
    fn nrom(&self) -> bool { (self.latch_addr & 0x0080) != 0 }
    fn locked(&self) -> bool { (self.latch_addr & 0x2000) != 0 }
}

impl Mapper for Mapper265 {
    fn reset(&mut self) {
        self.latch_addr = 0;
        self.latch_data = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }
        let prg = self.prg();
        let nrom = self.nrom();
        let cpu_a14 = self.cpu_a14();
        let num_16k = cart.prg_rom.len() / 0x4000;
        if num_16k == 0 {
            return FetchResult { data: 0, driven: false };
        }
        let bank = if address < 0xC000 {
            (prg & !(if nrom { 1 } else { 0 }) & !(if cpu_a14 { 1 } else { 0 })) as usize
        } else {
            (prg | (if !nrom { 7 } else { 0 }) | (if cpu_a14 { 1 } else { 0 })) as usize
        } % num_16k;
        let offset = bank * 0x4000 + (address as usize & 0x3FFF);
        FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            if self.locked() {
                self.latch_data = data;
            } else {
                self.latch_addr = address;
                self.latch_data = data;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mirror_h() {
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
            let data = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[address as usize & 0x1FFF]
            } else if !chr_rom.is_empty() {
                chr_rom[address as usize % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= data as u16;
        } else {
            let mirrored = if self.mirror_h() {
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
                cart.chr_ram[address as usize & 0x1FFF] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() { cart.prg_vram[idx] = data; }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut s = Vec::with_capacity(3);
        s.extend_from_slice(&self.latch_addr.to_le_bytes());
        s.push(self.latch_data);
        s
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.latch_addr = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() { self.latch_data = state[p]; p += 1; }
        p
    }
}
