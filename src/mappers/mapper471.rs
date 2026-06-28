use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper471 {
    reg: u16,
    irq_pending: bool,
    last_spr_adr: u16,
    last_bck_adr: u16,
    last_r2006: u16,
}

impl Mapper471 {
    pub fn new() -> Self {
        Mapper471 {
            reg: 0,
            irq_pending: false,
            last_spr_adr: 0,
            last_bck_adr: 0,
            last_r2006: 0,
        }
    }
}

impl Mapper for Mapper471 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = (self.reg & 0xFF) as usize;
            let offset = (bank * 0x8000) + (address as usize & 0x7FFF);
            let final_offset = offset % cart.prg_rom.len();
            FetchResult { data: cart.prg_rom[final_offset], driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, _data: u8) {
        if address >= 0x8000 {
            self.reg = address;
            self.irq_pending = false;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if !cart.nametable_horizontal_mirroring {
            address & 0x37FF
        } else {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        }
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
            let bank = (self.reg & 0xFF) as usize;
            let offset = (bank * 0x2000) + (address as usize & 0x1FFF);
            if using_chr_ram {
                new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
            } else {
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else {
            let mirrored = if !nametable_horizontal_mirroring { address & 0x37FF } else { (address & 0x33FF) | ((address & 0x0800) >> 1) };
            let idx = (mirrored & 0x7FF) as usize;
            new_addr_bus |= vram[idx] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        _ppu_sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        if !rendering_on {
            return false;
        }
        if (dot & 7) != 3 {
            return false;
        }
        if scanline < 240 && dot >= 256 && dot <= 319 {
            let current_spr_page = ppu_address_bus & 0x1000;
            let last_spr_page = self.last_spr_adr & 0x1000;
            if current_spr_page > last_spr_page {
                self.irq_pending = true;
            }
            self.last_spr_adr = ppu_address_bus;
        }
        else if scanline < 240 && dot >= 320 && dot <= 340 {
            let current_bck_page = ppu_address_bus & 0x1000;
            let last_bck_page = self.last_bck_adr & 0x1000;
            if current_bck_page > last_bck_page {
                self.irq_pending = true;
            }
            self.last_bck_adr = ppu_address_bus;
        }
        false
    }

    fn cpu_clock_rise(&mut self, _ppu_address_bus: u16) -> bool {
        let irq = self.irq_pending;
        self.irq_pending = false; 
        irq
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.reg.to_le_bytes());
        state.extend_from_slice(&self.last_spr_adr.to_le_bytes());
        state.extend_from_slice(&self.last_bck_adr.to_le_bytes());
        state.extend_from_slice(&self.last_r2006.to_le_bytes());
        state.push(self.irq_pending as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 9 <= state.len() {
            self.reg = u16::from_le_bytes([state[start], state[start + 1]]);
            self.last_spr_adr = u16::from_le_bytes([state[start + 2], state[start + 3]]);
            self.last_bck_adr = u16::from_le_bytes([state[start + 4], state[start + 5]]);
            self.last_r2006 = u16::from_le_bytes([state[start + 6], state[start + 7]]);
            self.irq_pending = state[start + 8] != 0;
            start += 9;
        }
        start
    }

    fn reset(&mut self) {
        self.reg = 0;
        self.irq_pending = false;
        self.last_spr_adr = 0;
        self.last_bck_adr = 0;
        self.last_r2006 = 0;
    }
}
