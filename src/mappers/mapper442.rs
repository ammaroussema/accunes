use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::nrom::mirror_address;

pub struct Mapper442 {
    reg: [u8; 8],
    pa00: bool,
    pa09: bool,
    pa13: bool,
}

impl Mapper442 {
    pub fn new() -> Self {
        Self {
            reg: [0; 8],
            pa00: false,
            pa09: false,
            pa13: false,
        }
    }

    fn mode1bpp(&self) -> bool {
        (self.reg[0] & 0x80) != 0
    }

    fn prg_bank(&self) -> u8 {
        let prg = (self.reg[0] & 0x1F) | ((self.reg[0] >> 1) & 0x20);
        (prg & 0x07) | ((prg >> 1) & 0x08)
    }

    fn check_mode1bpp(&mut self, address: u16) {
        let pa13_new = (address & 0x2000) != 0;
        if !self.pa13 && pa13_new {
            self.pa00 = (address & 0x0001) != 0;
            self.pa09 = (address & 0x0200) != 0;
        }
        self.pa13 = pa13_new;
    }
}

impl Mapper for Mapper442 {
    fn reset(&mut self) {
        self.reg = [0; 8];
        self.pa00 = false;
        self.pa09 = false;
        self.pa13 = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x6000..0x8000).contains(&address) && !cart.prg_ram.is_empty() {
            let len = cart.prg_ram.len();
            return FetchResult {
                data: cart.prg_ram[(address as usize & 0x1FFF) % len],
                driven: true,
            };
        }
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            let bank = self.prg_bank() as usize;
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
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

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) && !cart.prg_ram.is_empty() {
            let len = cart.prg_ram.len();
            cart.prg_ram[(address as usize & 0x1FFF) % len] = data;
        }
    }

    fn handle_cpu_write(&mut self, address: u16, data: u8) {
        if (0x5000..0x6000).contains(&address) {
            self.reg[((address >> 8) & 7) as usize] = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        mirror_address(
            cart.alternative_nametable_arrangement,
            cart.nametable_horizontal_mirroring,
            address,
        )
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        _using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x3000 {
            self.check_mode1bpp(address);
        }
        if address < 0x2000 {
            let read_addr = if self.mode1bpp() && !self.pa13 {
                (address & !0x1008)
                    | if self.pa09 { 0x1000 } else { 0 }
                    | if self.pa00 { 0x0008 } else { 0 }
            } else {
                address
            };
            let byte = if chr_ram.is_empty() {
                0
            } else {
                chr_ram[(read_addr as usize & 0x1FFF) % chr_ram.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mirror_address(
                alternative_nametable_arrangement,
                nametable_horizontal_mirroring,
                address,
            );
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() {
                    prg_vram[idx]
                } else {
                    0
                }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x3000 {
            self.check_mode1bpp(address);
        }
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut out = self.reg.to_vec();
        out.push(self.pa00 as u8);
        out.push(self.pa09 as u8);
        out.push(self.pa13 as u8);
        out
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for slot in self.reg.iter_mut() {
            if p < state.len() {
                *slot = state[p];
                p += 1;
            }
        }
        if p < state.len() {
            self.pa00 = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.pa09 = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.pa13 = state[p] != 0;
            p += 1;
        }
        p
    }
}
