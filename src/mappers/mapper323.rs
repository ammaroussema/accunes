use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};
use crate::mappers::mmc1::{Mmc1Core, Mmc1Config, Mmc1Variant};

pub struct Mapper323 {
    core: Mmc1Core,
    reg: u8,
}

impl Mapper323 {
    pub fn new() -> Self {
        let config = Mmc1Config {
            variant: Mmc1Variant::Standard,
            serom: false,
            wram_size: 0,
            battery_wram_size: 0,
            snrom: false,
        };
        Self { core: Mmc1Core::new(config), reg: 0 }
    }

    fn outer_bank(&self) -> usize {
        ((self.reg & 0xF0) >> 4) as usize
    }

    fn prg_offset(&self, cart: &Cartridge, address: u16) -> usize {
        let outer = self.outer_bank();
        let base_prg = self.core.prg_rom_offset(cart, address);
        (base_prg & 0xFFFF) | (outer << 16)
    }

    fn chr_offset(&self, address: u16, chr_len: usize) -> usize {
        let outer = self.outer_bank();
        let base_chr = self.core.chr_offset(address, chr_len);
        if chr_len == 0 {
            base_chr
        } else {
            (base_chr & 0x7FFF) | (outer << 15)
        }
    }
}

impl Mapper for Mapper323 {
    fn reset(&mut self) {
        self.reg = 0;
        self.core.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x6000..0x8000).contains(&address) {
            return FetchResult { data: 0, driven: false };
        }
        if address >= 0x8000 {
            let offset = self.prg_offset(cart, address);
            return FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len().max(1)],
                driven: true,
            };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) {
            if (self.core.control & 0x10) == 0 && (self.reg & 8) == 0 {
                self.reg = data;
            }
            return;
        }
        self.core.write_register(cart, address, data, -1);
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.core.mirror_nametable(cart, address)
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
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            if using_chr_ram {
                let offset = self.chr_offset(address, chr_ram.len());
                new_addr_bus |= if !chr_ram.is_empty() { chr_ram[offset % chr_ram.len()] } else { 0 } as u16;
            } else {
                let offset = self.chr_offset(address, chr_rom.len());
                new_addr_bus |= if !chr_rom.is_empty() { chr_rom[offset % chr_rom.len()] } else { 0 } as u16;
            }
        } else {
            new_addr_bus |= vram[(mirror_h_or_v(nametable_horizontal_mirroring, address) & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let offset = self.chr_offset(address, cart.chr_ram.len());
            let len = cart.chr_ram.len();
            cart.chr_ram[offset % len] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.core.cpu_clock_irq()
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        self.core.append_save_state(&mut state);
        state.push(self.reg);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..cart.prg_ram.len() {
            cart.prg_ram[i] = state[p]; p += 1;
        }
        for i in 0..cart.chr_ram.len() {
            cart.chr_ram[i] = state[p]; p += 1;
        }
        p = self.core.load_save_state(state, p);
        self.reg = state.get(p).copied().unwrap_or(0); p += 1;
        p
    }
}
