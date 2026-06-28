use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const VS_FRAME_CYCLES: u64 = 29780;

pub struct Mapper206 {
    pointer: u8,
    reg: [u8; 8],
    submapper: u8,
    vsdip: u8,
    coinon: u8,
    coinon2: u8,
    service: u8,
    cycle_accum: u64,
}

impl Mapper206 {
    pub fn new(submapper: u8) -> Self {
        let mut reg = [0u8; 8];
        reg[0] = 0x00;
        reg[1] = 0x02;
        reg[2] = 0x04;
        reg[3] = 0x05;
        reg[4] = 0x06;
        reg[5] = 0x07;
        reg[6] = 0x00;
        reg[7] = 0x01;
        Self {
            pointer: 0x00,
            reg,
            submapper,
            vsdip: 0,
            coinon: 0,
            coinon2: 0,
            service: 0,
            cycle_accum: 0,
        }
    }

    fn prg_bank_offset(&self, cart: &Cartridge, address: u16) -> usize {
        let prg_len = cart.prg_rom.len();
        if prg_len == 0 {
            return 0;
        }
        if self.submapper == 1 {
            (address as usize & 0x7FFF) % prg_len
        } else {
            let bank = match address & 0xE000 {
                0x8000 => (self.reg[6] & 0x0F) as usize,
                0xA000 => (self.reg[7] & 0x0F) as usize,
                0xC000 => {
                    let num_banks = prg_len / 0x2000;
                    num_banks.saturating_sub(2) & 0x0F
                }
                0xE000 => {
                    let num_banks = prg_len / 0x2000;
                    num_banks.saturating_sub(1) & 0x0F
                }
                _ => 0,
            };
            (bank * 0x2000 + (address as usize & 0x1FFF)) % prg_len
        }
    }

    fn chr_bank_offset(&self, address: u16, chr_len: usize) -> usize {
        if chr_len == 0 {
            return 0;
        }
        let page = address / 0x400;
        let bank = match page {
            0 => (self.reg[0] & 0xFE & 0x3F) as usize,
            1 => ((self.reg[0] | 1) & 0x3F) as usize,
            2 => (self.reg[1] & 0xFE & 0x3F) as usize,
            3 => ((self.reg[1] | 1) & 0x3F) as usize,
            4 => (self.reg[2] & 0x3F) as usize,
            5 => (self.reg[3] & 0x3F) as usize,
            6 => (self.reg[4] & 0x3F) as usize,
            7 => (self.reg[5] & 0x3F) as usize,
            _ => 0,
        };
        (bank * 0x400 + (address as usize & 0x3FF)) % chr_len
    }

    fn mirror_addr(horizontal: bool, address: u16) -> u16 {
        let norm = address & 0x2FFF;
        if horizontal {
            (norm & 0x33FF) | ((norm & 0x0800) >> 1)
        } else {
            norm & 0x37FF
        }
    }
}

impl Mapper for Mapper206 {
    fn adjust_controller_read(&self, address: u16, value: u8) -> u8 {
        if address & 0x1F == 0x16 {
            let mut vs = value & 0x01;
            if self.service > 0 { vs |= 0x04; }
            vs |= (self.vsdip & 0x03) << 3;
            if self.coinon > 0 { vs |= 0x20; }
            if self.coinon2 > 0 { vs |= 0x40; }
            vs
        } else if address & 0x1F == 0x17 {
            (value & 0x01) | (self.vsdip & 0xFC)
        } else {
            value
        }
    }

    fn insert_coin(&mut self, coin: u8) {
        match coin {
            0 => self.coinon = 6,
            1 => self.coinon2 = 6,
            _ => {}
        }
    }

    fn service_button(&mut self) {
        self.service = 6;
    }

    fn get_dip_switches(&self) -> u8 {
        self.vsdip
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.vsdip = value;
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.cycle_accum += _cycles as u64;
        if self.cycle_accum >= VS_FRAME_CYCLES {
            self.cycle_accum = 0;
            if self.coinon > 0 { self.coinon -= 1; }
            if self.coinon2 > 0 { self.coinon2 -= 1; }
            if self.service > 0 { self.service -= 1; }
        }
        false
    }

    fn reset(&mut self) {
        self.pointer = 0x00;
        self.reg[0] = 0x00;
        self.reg[1] = 0x02;
        self.reg[2] = 0x04;
        self.reg[3] = 0x05;
        self.reg[4] = 0x06;
        self.reg[5] = 0x07;
        self.reg[6] = 0x00;
        self.reg[7] = 0x01;
        self.vsdip = 0;
        self.coinon = 0;
        self.coinon2 = 0;
        self.service = 0;
        self.cycle_accum = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            FetchResult {
                data: cart.prg_rom[self.prg_bank_offset(cart, address)],
                driven: true,
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 && address < 0xA000 {
            if (address & 1) != 0 {
                self.reg[(self.pointer & 7) as usize] = data;
            } else {
                self.pointer = data;
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        Self::mirror_addr(cart.nametable_horizontal_mirroring, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let len = if using_chr_ram { chr_ram.len() } else { chr_rom.len() };
            let offset = self.chr_bank_offset(address, len);
            let data = if using_chr_ram { chr_ram[offset] } else { chr_rom[offset] };
            new_addr_bus |= data as u16;
        } else {
            let mirrored = Self::mirror_addr(nametable_horizontal_mirroring, address);
            let data = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= data as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = vec![self.pointer];
        state.extend_from_slice(&self.reg);
        state.push(self.vsdip);
        state.push(self.coinon);
        state.push(self.coinon2);
        state.push(self.service);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 9 <= state.len() {
            self.pointer = state[start];
            self.reg.copy_from_slice(&state[start + 1..start + 9]);
            start += 9;
        }
        self.vsdip = state.get(start).copied().unwrap_or(0); start += 1;
        self.coinon = state.get(start).copied().unwrap_or(0); start += 1;
        self.coinon2 = state.get(start).copied().unwrap_or(0); start += 1;
        self.service = state.get(start).copied().unwrap_or(0); start += 1;
        start
    }
}
