use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper450 {
    prg: [u8; 2],
    chr: [u16; 8],
    mirr: u8,
    wires: u8,
    wires_mode: bool,
}

impl Mapper450 {
    pub fn new(wires_mode: bool) -> Self {
        Self {
            prg: [0, 1],
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            mirr: 0,
            wires: 0,
            wires_mode,
        }
    }

    fn prg_bank8(&self, slot: u8) -> usize {
        let or = (self.wires as usize) << 4;
        match slot {
            0 => (self.prg[0] as usize & 0x0F) | or,
            1 => (self.prg[1] as usize & 0x0F) | or,
            2 => 0x0E | or,
            _ => 0x0F | or,
        }
    }

    fn chr_bank(&self, index: usize) -> usize {
        (self.chr[index] as usize & 0x7F) | ((self.wires as usize) << 7)
    }

    fn mirror(&self, address: u16) -> u16 {
        if self.mirr & 1 != 0 {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn read_wram(&self, cart: &Cartridge, address: u16) -> FetchResult {
        if cart.prg_ram.is_empty() {
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        let idx = (address as usize & 0x1FFF) % cart.prg_ram.len();
        FetchResult {
            data: cart.prg_ram[idx],
            driven: true,
        }
    }

    fn write_wram(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if cart.prg_ram.is_empty() {
            return;
        }
        let idx = (address as usize & 0x1FFF) % cart.prg_ram.len();
        cart.prg_ram[idx] = data;
    }
}

impl Mapper for Mapper450 {
    fn reset(&mut self) {
        *self = Self::new(self.wires_mode);
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
            let slot = ((address - 0x8000) >> 13) as u8;
            let bank = self.prg_bank8(slot);
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            };
        }
        if address >= 0x6000 {
            if self.wires_mode && address < 0x7000 {
                // readWires: single status bit on the bus; leave bus undriven (open bus)
                return FetchResult {
                    data: 0,
                    driven: false,
                };
            }
            return self.read_wram(cart, address);
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) {
            if self.wires_mode && address < 0x7000 {
                self.wires = data & 7;
            } else {
                self.write_wram(cart, address, data);
            }
        } else if address >= 0x8000 {
            match address >> 12 {
                0x8 => self.prg[0] = data,
                0x9 => self.mirr = data & 3,
                0xA => self.prg[1] = data,
                0xB..=0xE => {
                    let index = ((((address >> 12) - 0xB) as usize) << 1)
                        | (if address & 0x02 != 0 { 1 } else { 0 });
                    if address & 0x01 != 0 {
                        self.chr[index] = (self.chr[index] & 0x000F) | ((data as u16) << 4);
                    } else {
                        self.chr[index] = (self.chr[index] & 0x0FF0) | ((data as u16) & 0x000F);
                    }
                }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror(address)
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
        if address >= 0x2000 {
            let mirrored = self.mirror(address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
            return (new_addr_bus as u8, new_addr_bus);
        }
        let bank = (address >> 10) as usize & 0x07;
        let chr_bank = self.chr_bank(bank);
        let offset = chr_bank * 0x0400 + (address as usize & 0x03FF);
        let byte = if using_chr_ram && !chr_ram.is_empty() {
            chr_ram[offset % chr_ram.len()]
        } else if !chr_rom.is_empty() {
            chr_rom[offset % chr_rom.len()]
        } else {
            0
        };
        new_addr_bus |= byte as u16;
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let bank = (address >> 10) as usize & 0x07;
                let chr_bank = self.chr_bank(bank);
                let offset = chr_bank * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror(address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        for p in &self.prg {
            state.push(*p);
        }
        for c in &self.chr {
            state.extend_from_slice(&c.to_le_bytes());
        }
        state.push(self.mirr);
        state.push(self.wires);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..2 {
            if p < state.len() {
                self.prg[i] = state[p];
                p += 1;
            }
        }
        for i in 0..8 {
            if p + 1 < state.len() {
                self.chr[i] = u16::from_le_bytes([state[p], state[p + 1]]);
                p += 2;
            }
        }
        if p < state.len() {
            self.mirr = state[p];
            p += 1;
        }
        if p < state.len() {
            self.wires = state[p];
            p += 1;
        }
        p
    }
}
