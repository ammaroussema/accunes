use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper165 {
    mmc3: MapperMMC3,
    prg_ram: [u8; 0x2000],
    chr_ram: [u8; 0x1000],
    chr_latch: [bool; 2],
}

impl Mapper165 {
    pub fn new() -> Self {
        Self {
            mmc3: MapperMMC3::new(Mmc3Config::embedded()),
            prg_ram: [0; 0x2000],
            chr_ram: [0; 0x1000],
            chr_latch: [false; 2],
        }
    }

    fn last_prg_bank_index(&self, cart: &Cartridge) -> usize {
        let num = cart.prg_rom.len() / 0x2000;
        if num == 0 { 0 } else { num - 1 }
    }

    fn second_last_prg_bank_index(&self, cart: &Cartridge) -> usize {
        let num = cart.prg_rom.len() / 0x2000;
        if num < 2 { 0 } else { num - 2 }
    }

    fn prg_offset(&self, cart: &Cartridge, slot: usize) -> usize {
        let num_8k = cart.prg_rom.len() / 0x2000;
        if num_8k == 0 {
            return 0;
        }
        let last = self.last_prg_bank_index(cart);
        let second_last = self.second_last_prg_bank_index(cart);
        let bank8 = match slot {
            0 => {
                if (self.mmc3.r8000 & 0x40) == 0 {
                    self.mmc3.bank_8c as usize
                } else {
                    second_last
                }
            }
            1 => self.mmc3.bank_a as usize,
            2 => {
                if (self.mmc3.r8000 & 0x40) != 0 {
                    self.mmc3.bank_8c as usize
                } else {
                    second_last
                }
            }
            _ => last,
        };
        let bank_idx = (bank8 & 0x3F) % num_8k;
        bank_idx * 0x2000
    }

    fn page4k_for_region(&self, region: usize) -> (usize, bool) {
        let slot_addr = if region == 0 {
            if self.chr_latch[0] { 0x0800 } else { 0x0000 }
        } else {
            if self.chr_latch[1] { 0x1800 } else { 0x1000 }
        };
        let bank1k = self.mmc3.chr_bank(slot_addr) as usize;
        let page4k = bank1k >> 2;
        (page4k, page4k == 0)
    }
}

impl Mapper for Mapper165 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.prg_ram = [0; 0x2000];
        self.chr_ram = [0; 0x1000];
        self.chr_latch = [false; 2];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x6000 {
            return FetchResult { data: 0, driven: true };
        }
        if address >= 0x6000 && address < 0x8000 {
            return FetchResult { data: self.prg_ram[(address & 0x1FFF) as usize], driven: true };
        }
        let len = cart.prg_rom.len();
        if len == 0 {
            return FetchResult { data: 0, driven: true };
        }
        let slot = ((address - 0x8000) / 0x2000) as usize;
        let base = self.prg_offset(cart, slot);
        let offset = base + (address as usize & 0x1FFF);
        FetchResult { data: cart.prg_rom[offset % len], driven: true }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            self.prg_ram[(address & 0x1FFF) as usize] = data;
        } else if address >= 0x8000 {
            self.mmc3.store_prg(cart, address, data);
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mmc3.mirror_nametable(cart, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        _chr_ram: &[u8],
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
            let addr_match = address & 0x2FF8;
            if addr_match == 0x0FD0 || addr_match == 0x0FE8 {
                let idx = ((address >> 12) & 1) as usize;
                self.chr_latch[idx] = (address & 0x08) != 0;
            }
            let region = (address >> 12) as usize;
            let (page4k, use_ram) = self.page4k_for_region(region);
            let byte = if use_ram {
                self.chr_ram[(address as usize) & 0x0FFF]
            } else {
                let offset = page4k * 0x1000 + (address as usize & 0x0FFF);
                if chr_rom.is_empty() { 0 } else { chr_rom[offset % chr_rom.len()] }
            };
            new_addr_bus |= byte as u16;
        } else {
            let ntm = self.mmc3.nametable_mirroring();
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if ntm {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            self.chr_ram[(address as usize) & 0x0FFF] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.mmc3.cpu_clock(cycles)
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        ppu_sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.mmc3.save_mapper_registers(cart));
        state.extend_from_slice(&self.prg_ram);
        state.extend_from_slice(&self.chr_ram);
        state.push(if self.chr_latch[0] { 1 } else { 0 });
        state.push(if self.chr_latch[1] { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let p = self.mmc3.load_mapper_registers(cart, state, start);
        let mut offset = p;
        for b in self.prg_ram.iter_mut() {
            if offset < state.len() {
                *b = state[offset];
                offset += 1;
            }
        }
        for b in self.chr_ram.iter_mut() {
            if offset < state.len() {
                *b = state[offset];
                offset += 1;
            }
        }
        if offset < state.len() {
            self.chr_latch[0] = state[offset] != 0;
            offset += 1;
        }
        if offset < state.len() {
            self.chr_latch[1] = state[offset] != 0;
            offset += 1;
        }
        offset
    }
}
