use crate::cartridge::Cartridge;
use crate::crc::crc32;
use crate::mapper::{FetchResult, Mapper};
const EPROM_CRC: u32 = 0x63794E25;

pub struct Mapper53 {
    regs: [u32; 2],
    eprom_first: bool,
}

impl Mapper53 {
    pub fn new(prg_rom: &[u8]) -> Self {
        let eprom_first = prg_rom.len() >= 0x8000 && crc32(&prg_rom[..0x8000]) == EPROM_CRC;
        Self {
            regs: [0; 2],
            eprom_first,
        }
    }

    fn prg_offset(&self, address: u16, prg_len: usize) -> usize {
        let r = (self.regs[0] << 3) & 0x78;
        let eprom_adj = if self.eprom_first { 2 } else { 0 };
        if address >= 0x6000 && address < 0x8000 {
            let bank = ((r << 1) | 0xF) + (if self.eprom_first { 4 } else { 0 });
            let bank_offset = (bank as usize * 0x2000) % prg_len;
            bank_offset + (address as usize & 0x1FFF)
        } else if address >= 0x8000 {
            let is_upper = address >= 0xC000;
            let bank16 = if is_upper {
                if (self.regs[0] & 0x10) != 0 {
                    (r | 7) + eprom_adj
                } else if self.eprom_first {
                    1
                } else {
                    0x81
                }
            } else {
                if (self.regs[0] & 0x10) != 0 {
                    (r | (self.regs[1] & 7)) + eprom_adj
                } else if self.eprom_first {
                    0
                } else {
                    0x80
                }
            };
            let bank_offset = (bank16 as usize * 0x4000) % prg_len;
            bank_offset + (address as usize & 0x3FFF)
        } else {
            0
        }
    }
}

impl Mapper for Mapper53 {
    fn reset(&mut self) {
        self.regs = [0; 2];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if cart.prg_rom.is_empty() {
            return FetchResult { data: 0, driven: address >= 0x6000 };
        }
        if address >= 0x6000 {
            let offset = self.prg_offset(address, cart.prg_rom.len());
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            self.regs[0] = data as u32;
        } else if address >= 0x8000 {
            self.regs[1] = data as u32;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if (self.regs[0] & 0x20) != 0 {
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
        prg_vram: &[u8],
        using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            if using_chr_ram {
                let len = chr_ram.len();
                if len > 0 {
                    new_addr_bus |= chr_ram[(address as usize & 0x1FFF) % len] as u16;
                }
            } else {
                let len = chr_rom.len();
                if len > 0 {
                    new_addr_bus |= chr_rom[(address as usize & 0x1FFF) % len] as u16;
                }
            }
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if (self.regs[0] & 0x20) != 0 {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() { prg_vram[idx] } else { 0 }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
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
        vec![self.regs[0] as u8, self.regs[1] as u8, if self.eprom_first { 1 } else { 0 }]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 3 <= state.len() {
            self.regs[0] = state[start] as u32; start += 1;
            self.regs[1] = state[start] as u32; start += 1;
            self.eprom_first = state[start] != 0; start += 1;
        }
        start
    }
}
