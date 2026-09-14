use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

const PAGE_TABLE: [u8; 0x24] = [
    0x0, 0x0, 0x2, 0x2, 0x1, 0x1, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xA, 0xB, 0xC, 0xD, 0xE, 0xF, 0x0, 0x1,
    0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xA, 0xB, 0xC, 0xD, 0xE, 0xF, 0xD, 0xD,
];

pub struct Mapper547 {
    reg: [u8; 16],
    ntram: [u8; 2048],
    irq_counter: u16,
    qt_byte: u8,
    last_nt_addr: u16,
    is_sprite: bool,
    irq_ack: bool,
}

impl Mapper547 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self {
            reg: [0; 16],
            ntram: [0; 2048],
            irq_counter: 0,
            qt_byte: 0,
            last_nt_addr: 0,
            is_sprite: false,
            irq_ack: false,
        }
    }

    fn write_to_exram(&self) -> bool {
        (self.reg[10] & 1) != 0
    }

    fn horizontal_mirroring(&self) -> bool {
        (self.reg[10] & 2) != 0
    }

    fn irq_enabled(&self) -> bool {
        (self.reg[9] & 2) != 0
    }

    fn get_nt_bank(&self, bank: u16) -> usize {
        let bit = if self.horizontal_mirroring() {
            (bank >> 1) & 1
        } else {
            bank & 1
        };
        (bit as usize) * 1024
    }

    fn prg_bank(&self, r: u8) -> u8 {
        if (r & 0x40) != 0 {
            (r & 0x3F) + 0x10
        } else {
            r & 0x0F
        }
    }

    fn jis_glyph(&self, addr: u16) -> u8 {
        let row = self.reg[13].wrapping_sub(0x20);
        let col = self.reg[12].wrapping_sub(0x20);
        if row < 0x60 && col < 0x60 {
            let row = row as u16;
            let col = col as u16;
            let code = (col % 32) + (row % 16) * 32 + (col / 32) * 512 + (row / 16) * 1536;
            let glyph = (code & 0xFF) | ((PAGE_TABLE[(code >> 8) as usize] as u16) << 8);
            let tile = glyph * 4;
            if addr == 0xC00 {
                (tile & 0xFF) as u8 | (self.reg[11] & 3)
            } else {
                (tile >> 8) as u8 | if (self.reg[11] & 4) != 0 { 0x80 } else { 0 } | 0x40
            }
        } else {
            0
        }
    }

    fn nt_map(&self, alternative_nametable_arrangement: bool, address: u16) -> u16 {
        if alternative_nametable_arrangement {
            address
        } else if self.horizontal_mirroring() {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn chr_read(&mut self, chr_rom: &[u8], chr_ram: &[u8], bank: usize, addr: u16) -> u8 {
        if (self.qt_byte & 0x40) != 0 {
            if (addr & 0x08) != 0 {
                if (self.qt_byte & 0x80) != 0 {
                    0xFF
                } else {
                    0x00
                }
            } else {
                let mut full_addr =
                    (((self.qt_byte & 0x3F) as usize) << 12) | ((bank & 3) << 10) | (addr as usize);
                if chr_rom.len() == 128 * 1024 {
                    full_addr = ((full_addr & 0x00007) << 1)
                        | ((full_addr & 0x00010) >> 4)
                        | ((full_addr & 0x3FFE0) >> 1);
                }
                if !chr_rom.is_empty() {
                    chr_rom[full_addr % chr_rom.len()]
                } else {
                    0
                }
            }
        } else if !self.is_sprite {
            let offset =
                (((self.qt_byte & 1) as usize) << 12) | (((bank << 10) | (addr as usize)) & 0xFFF);
            if !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else {
                0
            }
        } else {
            let phys = (bank << 10) | (addr as usize);
            let page = if phys < 0x1000 {
                (self.reg[5] & 1) as usize
            } else {
                1
            };
            let offset = page * 0x1000 + (phys & 0xFFF);
            if !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else {
                0
            }
        }
    }
}
impl Mapper for Mapper547 {
    fn reset(&mut self) {
        self.reg = [0; 16];
        self.irq_counter = 0;
        self.qt_byte = 0;
        self.last_nt_addr = 0;
        self.is_sprite = false;
        self.irq_ack = false;
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x6000..0x8000).contains(&address) {
            let page = if address < 0x7000 {
                (self.reg[0] & 1) | (self.reg[0] >> 2)
            } else {
                (self.reg[1] & 1) | (self.reg[1] >> 2)
            };
            if !cart.prg_ram.is_empty() {
                let offset = (page as usize) * 0x1000 + (address as usize & 0xFFF);
                let len = cart.prg_ram.len();
                return FetchResult {
                    data: cart.prg_ram[offset % len],
                    driven: true,
                };
            }
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        if address >= 0x8000 {
            if address == 0xDC00 || address == 0xDD00 {
                return FetchResult {
                    data: self.jis_glyph(address & 0xFFF),
                    driven: true,
                };
            }
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            let bank = match address {
                0x8000..=0x9FFF => self.prg_bank(self.reg[2]),
                0xA000..=0xBFFF => self.prg_bank(self.reg[3]),
                0xC000..=0xDFFF => self.prg_bank(self.reg[4]),
                _ => 0x4F,
            };
            let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
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
            let page = if address < 0x7000 {
                (self.reg[0] & 1) | (self.reg[0] >> 2)
            } else {
                (self.reg[1] & 1) | (self.reg[1] >> 2)
            };
            if !cart.prg_ram.is_empty() {
                let offset = (page as usize) * 0x1000 + (address as usize & 0xFFF);
                let len = cart.prg_ram.len();
                cart.prg_ram[offset % len] = data;
            }
        } else if (0xD000..0xE000).contains(&address) {
            let idx = ((address >> 8) & 0xF) as usize;
            self.reg[idx] = data;
            match idx {
                8 => {
                    if (self.reg[9] & 1) != 0 {
                        self.reg[9] |= 2;
                    } else {
                        self.reg[9] &= !2;
                    }
                    self.irq_ack = true;
                }
                9 => {
                    self.reg[9] = data;
                    if (self.reg[9] & 2) != 0 {
                        self.irq_counter = (self.reg[6] as u16) | ((self.reg[7] as u16) << 8);
                    }
                    self.irq_ack = true;
                }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.nt_map(cart.alternative_nametable_arrangement, address)
    }


    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = ((address >> 10) & 0x7) as usize;
            let addr = address & 0x3FF;
            let byte = self.chr_read(chr_rom, chr_ram, bank, addr);
            new_addr_bus |= byte as u16;
        } else {
            let bank = (address >> 10) & 0xF;
            let addr = address & 0x3FF;
            if addr < 0x3C0 {
                let idx = self.get_nt_bank(bank) + addr as usize;
                if idx < self.ntram.len() {
                    self.qt_byte = self.ntram[idx];
                }
            }
            self.is_sprite = self.last_nt_addr == addr;
            self.last_nt_addr = addr;
            let mirrored = self.nt_map(alternative_nametable_arrangement, address);
            let byte = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let page = if address < 0x1000 {
                    (self.reg[5] & 1) as usize
                } else {
                    1
                };
                let offset = page * 0x1000 + (address as usize & 0xFFF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            if self.write_to_exram() {
                let bank = (address >> 10) & 0xF;
                let addr = address & 0x3FF;
                let idx = self.get_nt_bank(bank) + addr as usize;
                if idx < self.ntram.len() {
                    self.ntram[idx] = data;
                }
            } else {
                let mirrored = self.nt_map(cart.alternative_nametable_arrangement, address);
                vram[(mirrored & 0x7FF) as usize] = data;
            }
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.irq_enabled() {
            for _ in 0..cycles {
                self.irq_counter = self.irq_counter.wrapping_add(1);
                if self.irq_counter == 0 {
                    self.irq_counter = (self.reg[6] as u16) | ((self.reg[7] as u16) << 8);
                    return true;
                }
            }
        }
        false
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack;
        self.irq_ack = false;
        ack
    }


    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.reg);
        state.extend_from_slice(&self.ntram);
        state.extend_from_slice(&self.irq_counter.to_le_bytes());
        state.push(self.qt_byte);
        state.extend_from_slice(&self.last_nt_addr.to_le_bytes());
        state.push(self.is_sprite as u8);
        state.push(self.irq_ack as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 16 <= state.len() {
            self.reg.copy_from_slice(&state[p..p + 16]);
            p += 16;
        }
        if p + 2048 <= state.len() {
            self.ntram.copy_from_slice(&state[p..p + 2048]);
            p += 2048;
        }
        if p + 2 <= state.len() {
            self.irq_counter = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.qt_byte = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.last_nt_addr = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.is_sprite = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.irq_ack = state[p] != 0;
            p += 1;
        }
        p
    }
}

