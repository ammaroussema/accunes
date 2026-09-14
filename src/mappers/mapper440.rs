use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

const PROT_LUT: [u8; 16] = [
    0x00, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
];

const ADDRESS_ORDER: [u8; 15] = [11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 12, 13, 14];

pub struct Mapper440 {
    mode: u8,
    index: u8,
    prg_sw: u8,
    last_nt_addr: u16,
    blah: u8,
}

impl Mapper440 {
    pub fn new() -> Self {
        Self {
            mode: 0,
            index: 0,
            prg_sw: 0,
            last_nt_addr: 0,
            blah: 0,
        }
    }

    fn prg_offset(&self, address: u16) -> usize {
        let mut offset = ((self.mode >> 5) & 3) as usize * 0x8000;
        if (self.mode & 1) != 0 {
            let encrypted = address & 0x7FFF;
            let mut decrypted: u16 = 0;
            for (bit, &order) in ADDRESS_ORDER.iter().enumerate() {
                if (encrypted >> order) & 1 != 0 {
                    decrypted |= 1u16 << bit;
                }
            }
            offset += (decrypted & 0x7FFF) as usize;
        } else {
            offset += (address & 0x7FFF) as usize;
        }
        offset
    }

    fn read_5(&mut self, address: u16) -> FetchResult {
        match address & 0x700 {
            0x300 => {
                self.blah ^= 4;
                FetchResult {
                    data: self.blah,
                    driven: true,
                }
            }
            0x500 => FetchResult {
                data: PROT_LUT[(self.index >> 4) as usize],
                driven: true,
            },
            _ => FetchResult {
                data: 0,
                driven: false,
            },
        }
    }
}

impl Mapper for Mapper440 {
    fn reset_power_cycle(&mut self) {
        self.index = 0;
        self.mode = 0x0E;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x5000..0x6000).contains(&address) {
            return self.read_5(address);
        }
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
            return FetchResult {
                data: cart.prg_rom[self.prg_offset(address) % len],
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
            match address & 0x700 {
                0x000 => self.mode = data,
                0x100 => self.prg_sw = data,
                0x400 => self.index = data,
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v((self.prg_sw & 2) == 0, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let mut read_addr = address;
            if (self.mode & 0x80) != 0 {
                read_addr = (read_addr & !0x1000)
                    | if (self.last_nt_addr & 0x200) != 0 { 0x1000 } else { 0 };
                read_addr = (read_addr & !0x0008)
                    | if (self.last_nt_addr & 0x001) != 0 { 0x0008 } else { 0 };
            }
            let byte = if chr_ram.is_empty() {
                0
            } else {
                chr_ram[(read_addr as usize & 0x1FFF) % chr_ram.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            if (address & 0x3FF) < 0x3C0 {
                self.last_nt_addr = address & 0x3FF;
            }
            let mir = mirror_h_or_v((self.prg_sw & 2) == 0, address);
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mir = mirror_h_or_v((self.prg_sw & 2) == 0, address);
            vram[(mir & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.mode, self.index]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.mode = state[p];
            p += 1;
        }
        if p < state.len() {
            self.index = state[p];
            p += 1;
        }
        p
    }
}
