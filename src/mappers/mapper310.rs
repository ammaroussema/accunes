use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper310 {
    reg: [u8; 3],
}

impl Mapper310 {
    pub fn new() -> Self {
        Self { reg: [0; 3] }
    }
}

impl Mapper for Mapper310 {
    fn reset(&mut self) {
        self.reg = [0; 3];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let prg = (self.reg[0] as usize) & 0x3F | ((self.reg[1] as usize) << 4) & !0x3F;
            let mode = self.reg[1] & 3;
            let (offset, _size) = match mode {
                0 => {
                    // 32KB at $8000
                    let bank = prg >> 1;
                    (bank * 0x8000 + (address as usize - 0x8000), 0x8000)
                }
                1 => {
                    // 16KB at $8000, 16KB fixed at $C000
                    if address < 0xC000 {
                        (prg * 0x4000 + (address as usize - 0x8000), 0x4000)
                    } else {
                        ((prg | 7) * 0x4000 + (address as usize - 0xC000), 0x4000)
                    }
                }
                2 => {
                    // four 8KB banks
                    let bank = prg << 1 | ((self.reg[0] >> 7) as usize);
                    let _slot = (address as usize - 0x8000) / 0x2000;
                    (bank * 0x2000 + (address as usize & 0x1FFF), 0x2000)
                }
                _ => {
                    // 16KB at both $8000 and $C000, same
                    if address < 0xC000 {
                        (prg * 0x4000 + (address as usize - 0x8000), 0x4000)
                    } else {
                        ((prg | 0) * 0x4000 + (address as usize - 0xC000), 0x4000)
                    }
                }
            };
            if cart.prg_rom.is_empty() {
                FetchResult { data: 0, driven: false }
            } else {
                let offset = offset % cart.prg_rom.len();
                FetchResult { data: cart.prg_rom[offset], driven: true }
            }
        } else if address >= 0x6000 {
            FetchResult { data: 0, driven: false }
        } else if address >= 0x4020 {
            FetchResult { data: 0, driven: false }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 && address < 0xC000 {
            self.reg[0] = data;
        } else if address >= 0xC000 {
            self.reg[1] = (address & 0xFF) as u8;
            self.reg[2] = data;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            return address;
        }
        if self.reg[0] & 0x40 != 0 {
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
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            if !chr_ram.is_empty() {
                let bank = (self.reg[2] as usize) * 0x2000;
                let offset = bank + (address as usize & 0x1FFF);
                new_addr_bus |= if offset < chr_ram.len() { chr_ram[offset] } else { 0 } as u16;
            } else if !chr_rom.is_empty() {
                let bank = (self.reg[2] as usize) * 0x2000;
                let offset = bank + (address as usize & 0x1FFF);
                new_addr_bus |= if offset < chr_rom.len() { chr_rom[offset] } else { 0 } as u16;
            }
        } else {
            let h = if alternative_nametable_arrangement {
                false
            } else {
                nametable_horizontal_mirroring
            };
            if self.reg[0] & 0x40 != 0 {
                let mirrored = (address & 0x33FF) | ((address & 0x0800) >> 1);
                new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
            } else if h {
                let mirrored = (address & 0x33FF) | ((address & 0x0800) >> 1);
                new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
            } else {
                new_addr_bus |= vram[(address & 0x37FF & 0x7FF) as usize] as u16;
            }
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        } else if address < 0x2000 && cart.using_chr_ram {
            let bank = (self.reg[2] as usize) * 0x2000;
            let offset = bank + (address as usize & 0x1FFF);
            if offset < cart.chr_ram.len() {
                cart.chr_ram[offset] = data;
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        for r in &self.reg {
            state.push(*r);
        }
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for r in self.reg.iter_mut() {
            *r = state.get(p).copied().unwrap_or(0); p += 1;
        }
        p
    }
}
