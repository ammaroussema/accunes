use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper243 {
    cmd: u8,
    regs: [u8; 8],
}

impl Mapper243 {
    pub fn new() -> Self {
        Self {
            cmd: 0,
            regs: [0; 8],
        }
    }

    fn prg_bank(&self) -> usize {
        ((self.regs[5] >> 1) & 3) as usize
    }

    fn chr_bank(&self) -> usize {
        let r2 = (self.regs[2] & 1) as usize;
        let r4 = ((self.regs[4] & 1) as usize) << 1;
        let r6 = ((self.regs[6] & 3) as usize) << 2;
        r2 | r4 | r6
    }

    fn mirror_mode(&self) -> u8 {
        (self.regs[7] >> 1) & 3
    }

    fn mirror_addr(&self, address: u16) -> u16 {
        let nt = (address >> 10) & 3;
        let offset = address & 0x3FF;
        match self.mirror_mode() {
            0 => match nt {
                3 => 0x2400 | offset,
                _ => 0x2000 | offset,
            },
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            2 => address & 0x37FF,
            3 => 0x2400 | offset,
            _ => unreachable!(),
        }
    }
}

impl Mapper for Mapper243 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = self.prg_bank();
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x4100 || address > 0x7FFF {
            return;
        }
        let reg = address & 0x4101;
        if reg == 0x4100 {
            self.cmd = data & 7;
        } else {
            self.regs[self.cmd as usize] = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror_addr(address)
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
        if address < 0x2000 {
            let bank = self.chr_bank();
            let chr_offset = bank * 0x2000 + (address as usize & 0x1FFF);
            if using_chr_ram {
                if !chr_ram.is_empty() {
                    new_addr_bus |= chr_ram[chr_offset % chr_ram.len()] as u16;
                }
            } else if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[chr_offset % chr_rom.len()] as u16;
            }
        } else {
            let mirrored = self.mirror_addr(address);
            new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let bank = self.chr_bank();
            let chr_offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[chr_offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::with_capacity(9);
        state.push(self.cmd);
        state.extend_from_slice(&self.regs);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.cmd = state[p];
            p += 1;
        }
        for i in 0..8 {
            if p < state.len() {
                self.regs[i] = state[p];
                p += 1;
            }
        }
        p
    }

    fn reset(&mut self) {
        self.cmd = 0;
        self.regs = [0; 8];
    }
}
