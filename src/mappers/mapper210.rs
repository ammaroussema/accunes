use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper210 {
    prg: [u8; 4],
    chr: [u8; 8],
    wram_write_enable: bool,
    submapper: u8,
}

impl Mapper210 {
    pub fn new(submapper: u8) -> Self {
        let mut prg = [0u8; 4];
        let mut chr = [0u8; 8];
        for i in 0..4 {
            prg[i] = 0xFC | (i as u8);
            chr[i] = i as u8;
            chr[i + 4] = (i + 4) as u8;
        }
        Self {
            prg,
            chr,
            wram_write_enable: false,
            submapper,
        }
    }

    fn prg_bank_offset(&self, cart: &Cartridge, address: u16) -> usize {
        let prg_len = cart.prg_rom.len();
        if prg_len == 0 {
            return 0;
        }
        let page = (address as usize - 0x8000) / 0x2000;
        let bank = (self.prg[page] & 0x3F) as usize;
        (bank * 0x2000 + (address as usize & 0x1FFF)) % prg_len
    }

    fn chr_bank_offset(&self, address: u16, chr_len: usize) -> usize {
        if chr_len == 0 {
            return 0;
        }
        let page = (address as usize) / 0x400;
        let bank = self.chr[page] as usize;
        (bank * 0x400 + (address as usize & 0x3FF)) % chr_len
    }
}

impl Mapper for Mapper210 {
    fn reset(&mut self) {
        for i in 0..4 {
            self.prg[i] = 0xFC | (i as u8);
            self.chr[i] = i as u8;
            self.chr[i + 4] = (i + 4) as u8;
        }
        self.wram_write_enable = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            FetchResult {
                data: cart.prg_rom[self.prg_bank_offset(cart, address)],
                driven: true,
            }
        } else if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let mask = cart.prg_ram.len() - 1;
                let offset = (address as usize - 0x6000) & mask;
                FetchResult {
                    data: cart.prg_ram[offset],
                    driven: true,
                }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if self.wram_write_enable && !cart.prg_ram.is_empty() {
                let mask = cart.prg_ram.len() - 1;
                let offset = (address as usize - 0x6000) & mask;
                cart.prg_ram[offset] = data;
            }
        } else if address >= 0x8000 && address < 0xC000 {
            let reg_idx = (((address >> 12) & 3) << 1) | if (address & 0x800) != 0 { 1 } else { 0 };
            self.chr[reg_idx as usize] = data;
        } else if address >= 0xC000 && address < 0xD000 {
            if (address & 0x800) == 0 {
                self.wram_write_enable = (data & 1) != 0;
            }
        } else if address >= 0xE000 {
            let reg_idx = (((address >> 12) & 1) << 1) | if (address & 0x800) != 0 { 1 } else { 0 };
            if reg_idx < 3 {
                self.prg[reg_idx as usize] = data;
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if self.submapper == 1 {
            if cart.nametable_horizontal_mirroring {
                (address & 0x3FFF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            }
        } else {
            match self.prg[0] >> 6 {
                0 => 0x2000 | (address & 0x3FF),
                1 => address & 0x37FF,
                2 => (address & 0x3FFF) | ((address & 0x0800) >> 1),
                3 => 0x2000 | 0x400 | (address & 0x3FF),
                _ => address,
            }
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
            let len = if using_chr_ram { chr_ram.len() } else { chr_rom.len() };
            let offset = self.chr_bank_offset(address, len);
            let data = if using_chr_ram { chr_ram[offset] } else { chr_rom[offset] };
            new_addr_bus |= data as u16;
        } else {
            let mirrored = self.mirror_nametable(&Cartridge {
                name: String::new(),
                prg_rom: Vec::new(),
                chr_rom: Vec::new(),
                memory_mapper: 210,
                sub_mapper: self.submapper,
                prg_size: 0,
                chr_size: 0,
                prg_size_minus_1: 0,
                chr_ram: Vec::new(),
                using_chr_ram: false,
                prg_ram: Vec::new(),
                has_battery: false,
                alternative_nametable_arrangement: false,
                prg_vram: Vec::new(),
                nametable_horizontal_mirroring,
                fds_disks: Vec::new(),
                trainer: Vec::new(),
                misc_rom: Vec::new(),
                mapper_chip: Box::new(crate::mapper::MapperNROM::new(crate::mapper::NromConfig::default())),
                mapper_cpu_cycle: 0,
                prg_rom_crc32: 0,
                chr_rom_crc32: 0,
                overall_crc32: 0,
                is_vs_system: false,
                tv_system: crate::region::TvSystem::Unknown,
            }, address);
            let data = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= data as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg[0..3]);
        state.extend_from_slice(&self.chr);
        state.push(self.wram_write_enable as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 3 + 8 + 1 <= state.len() {
            self.prg[0..3].copy_from_slice(&state[start..start + 3]);
            self.chr.copy_from_slice(&state[start + 3..start + 11]);
            self.wram_write_enable = state[start + 11] != 0;
            start += 12;
        }
        start
    }
}
