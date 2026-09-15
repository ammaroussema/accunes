use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper429 {
    submapper: u8,
    has_battery: bool,
    latch_data: u8,
}

impl Mapper429 {
    pub fn new(submapper_id: u8, has_battery: bool) -> Self {
        Self {
            submapper: submapper_id,
            has_battery,
            latch_data: 0,
        }
    }

    fn mirror_addr(&self, horizontal: bool, alternative: bool, address: u16) -> u16 {
        if self.submapper == 1 {
            let page: u16 = if (self.latch_data & 0x80) != 0 { 1 } else { 0 };
            0x2000 + page * 0x400 + (address & 0x3FF)
        } else if alternative {
            address & 0x2FFF
        } else if horizontal {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn chr_fetch(&self, address: u16, mem: &[u8]) -> u8 {
        let offset = (self.latch_data as usize) * 0x2000 + (address as usize & 0x1FFF);
        mem[offset % mem.len()]
    }
}

impl Mapper for Mapper429 {
    fn reset(&mut self) {
        self.latch_data = 0x04;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x6000..0x8000).contains(&address) {
            if self.has_battery && !cart.prg_ram.is_empty() {
                return FetchResult {
                    data: cart.prg_ram[(address - 0x6000) as usize],
                    driven: true,
                };
            }
            return FetchResult {
                data: 0,
                driven: false,
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
            let offset = ((self.latch_data as usize) >> 2) * 0x8000 + (address as usize & 0x7FFF);
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
        if (0x6000..0x8000).contains(&address) {
            if self.has_battery && !cart.prg_ram.is_empty() {
                cart.prg_ram[(address - 0x6000) as usize] = data;
            }
        } else if address >= 0x8000 {
            self.latch_data = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mirror_addr(
            cart.nametable_horizontal_mirroring,
            cart.alternative_nametable_arrangement,
            address,
        )
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                self.chr_fetch(address, chr_ram)
            } else if !chr_rom.is_empty() {
                self.chr_fetch(address, chr_rom)
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mir = self.mirror_addr(
                nametable_horizontal_mirroring,
                alternative_nametable_arrangement,
                address,
            );
            let byte = if (mir & 0x0800) != 0 {
                let idx = (mir & 0x7FF) as usize;
                if idx < prg_vram.len() {
                    prg_vram[idx]
                } else {
                    0
                }
            } else {
                vram[(mir & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                let offset = (self.latch_data as usize) * 0x2000 + (address as usize & 0x1FFF);
                cart.chr_ram[offset % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mir = self.mirror_addr(
                cart.nametable_horizontal_mirroring,
                cart.alternative_nametable_arrangement,
                address,
            );
            if (mir & 0x0800) != 0 {
                let idx = (mir & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mir & 0x7FF) as usize] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.latch_data]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        if start < state.len() {
            self.latch_data = state[start];
        }
        start + 1
    }
}
