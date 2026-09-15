use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper560 {
    prg: u8,
    tile_bank: u8,
    tile_ram: [u8; 1024],
    subor: bool,
}

impl Mapper560 {
    pub fn new(submapper_id: u8) -> Self {
        Self {
            prg: 1,
            tile_bank: 0,
            tile_ram: [0u8; 1024],
            subor: submapper_id == 1,
        }
    }

    fn read_chr_standard(chr_rom: &[u8], chr_ram: &[u8], bank: usize, addr: usize) -> u8 {
        let offset = bank * 0x400 + addr;
        if !chr_rom.is_empty() {
            chr_rom[offset % chr_rom.len()]
        } else if !chr_ram.is_empty() {
            chr_ram[offset % chr_ram.len()]
        } else {
            0
        }
    }
}

impl Mapper for Mapper560 {
    fn reset(&mut self) {
        self.prg = 1;
        self.tile_ram = [0u8; 1024];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            let len = cart.prg_ram.len();
            if len > 0 {
                return FetchResult {
                    data: cart.prg_ram[(address as usize & 0x1FFF) % len],
                    driven: true,
                };
            }
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        if address >= 0x8000 {
            let bank = if self.prg != 0 { 1 } else { 0 };
            let offset = bank as usize * 0x8000 + (address as usize & 0x7FFF);
            return FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            let len = cart.prg_ram.len();
            if len > 0 {
                cart.prg_ram[(address as usize & 0x1FFF) % len] = data;
            }
            return;
        }
        if address >= 0x8000 && address < 0xF000 {
            self.prg ^= 1;
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
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        let byte;
        if address < 0x2000 {
            let bank = (address as usize >> 10) & 7;
            let addr = address & 0x3FF;
            if self.subor {
                if self.prg != 0 && (self.tile_bank & 0xC0) != 0 {
                    let low = (addr & 8) == 0 && (self.tile_bank & 0x80) != 0;
                    let high = (addr & 8) != 0 && (self.tile_bank & 0x40) != 0;
                    if low || high {
                        let offset = (((self.tile_bank as u32) << 12 & 0x1F000)
                            | ((bank as u32) << 10 & 0xC00)
                            | (addr as u32 & 0x3F0)
                            | ((self.tile_bank as u32 >> 2) & 8)
                            | (addr as u32 & 7)) as usize;
                        byte = if chr_rom.is_empty() {
                            0
                        } else {
                            chr_rom[offset % chr_rom.len()]
                        };
                    } else {
                        byte = 0;
                    }
                } else {
                    byte = Self::read_chr_standard(chr_rom, chr_ram, bank, addr as usize);
                }
            } else if self.prg != 0 {
                let offset = (((self.tile_bank as u32) << 11 & 0x1F800)
                    | ((bank as u32) << 9 & 0x600)
                    | ((addr as u32 >> 1) & 0x1F8)
                    | (addr as u32 & 7)) as usize;
                byte = if chr_rom.is_empty() {
                    0
                } else {
                    chr_rom[offset % chr_rom.len()]
                };
            } else {
                let offset = (((addr as u32 & 8) << 13)
                    | ((bank as u32) << 9 & 0xE00)
                    | ((addr as u32 >> 1) & 0x1F8)
                    | (addr as u32 & 7)) as usize;
                byte = if chr_rom.is_empty() {
                    0
                } else {
                    chr_rom[offset % chr_rom.len()]
                };
            }
        } else if address < 0x3F00 {
            let nt_addr = (address as usize) & 0x3FF;
            if nt_addr < 0x3C0 {
                self.tile_bank = self.tile_ram[nt_addr];
            }
            let mirrored = if nametable_horizontal_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            byte = vram[(mirrored & 0x7FF) as usize];
        } else {
            return (ppu_address_bus as u8, new_addr_bus);
        }
        new_addr_bus |= byte as u16;
        (new_addr_bus as u8, new_addr_bus)
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.nametable_horizontal_mirroring {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram && !cart.chr_ram.is_empty() {
            let len = cart.chr_ram.len();
            cart.chr_ram[(address as usize) % len] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
            if (address & 0xF000) == 0x2400 {
                self.tile_ram[(address & 0x3FF) as usize] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut s = Vec::new();
        s.push(self.prg);
        s.push(self.tile_bank);
        s.extend_from_slice(&self.tile_ram);
        s
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.prg = state[p];
            p += 1;
        }
        if p < state.len() {
            self.tile_bank = state[p];
            p += 1;
        }
        let len = (state.len() - p).min(self.tile_ram.len());
        self.tile_ram[..len].copy_from_slice(&state[p..p + len]);
        p += len;
        p - start
    }
}