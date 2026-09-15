use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

pub struct Mapper465 {
    addr: u16,
    data: u8,
}

impl Mapper465 {
    pub fn new() -> Self {
        Self {
            addr: 0,
            data: 0,
        }
    }

    fn prg(&self) -> usize {
        (((self.addr >> 2) & 0x1F) | ((self.addr >> 5) & 0x20)) as usize
    }

    fn cpu_a14(&self) -> bool {
        (self.addr & 0x001) != 0
    }

    fn mirror_h(&self) -> bool {
        (self.addr & 0x002) != 0
    }

    fn unrom(&self) -> bool {
        (self.addr & 0x200) != 0
    }

    fn prg_8000(&self) -> usize {
        let prg = self.prg();
        let bank = if self.cpu_a14() { prg & !1 } else { prg };
        if self.unrom() {
            bank | (self.data as usize)
        } else {
            bank
        }
    }

    fn prg_c000(&self) -> usize {
        let prg = self.prg();
        let bank = if self.cpu_a14() { prg | 1 } else { prg };
        if self.unrom() {
            bank | 7
        } else {
            bank
        }
    }
}

impl Mapper for Mapper465 {
    fn reset(&mut self) {
        *self = Self::new();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            let slot = ((address >> 13) & 3) as usize;
            let bank = if slot < 2 {
                self.prg_8000() * 2 + slot
            } else {
                self.prg_c000() * 2 + (slot - 2)
            };
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            if self.unrom() {
                self.data = data;
            } else {
                self.addr = address;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v(self.mirror_h(), address)
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
            let offset = (address as usize) & 0x1FFF;
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mirror_h_or_v(self.mirror_h(), address);
            let byte = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let offset = (address as usize) & 0x1FFF;
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = mirror_h_or_v(self.mirror_h(), address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = self.addr.to_le_bytes().to_vec();
        state.push(self.data);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 1 < state.len() {
            self.addr = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        self.data = state.get(p).copied().unwrap_or(0);
        p += 1;
        p
    }
}
