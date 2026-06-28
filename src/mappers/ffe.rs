use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const FFE_WRAM_SIZE: usize = 8192;
const TRAINER_WRAM_OFFSET: usize = 0x1000;

pub fn install_trainer(trainer: &[u8], prg_ram: &mut [u8]) {
    if trainer.is_empty() || prg_ram.len() < TRAINER_WRAM_OFFSET + 512 {
        return;
    }
    let len = trainer.len().min(512);
    prg_ram[TRAINER_WRAM_OFFSET..TRAINER_WRAM_OFFSET + len]
        .copy_from_slice(&trainer[..len]);
}
const IRQ_THRESHOLD: u32 = 0x1_0000;
const IRQ_CYCLES_PER_INSTRUCTION: u32 = 4;

#[derive(Clone, Debug)]
pub struct FfeConfig {
    pub wram_size: usize,
    pub battery_save_size: usize,
    pub extended_mode: bool,
    pub initial_mirr: u8,
}

impl FfeConfig {
    pub fn mapper6(header: &[u8], _submapper: u8, has_battery: bool) -> Self {
        Self::for_ines(header, has_battery, false)
    }

    pub fn mapper17(header: &[u8], has_battery: bool) -> Self {
        Self::for_ines(header, has_battery, true)
    }

    fn for_ines(header: &[u8], has_battery: bool, extended_mode: bool) -> Self {
        let vertical = header.len() >= 16 && (header[6] & 1) != 0;
        let initial_mirr = ((!vertical as u8) ^ 1) | 2;
        Self {
            wram_size: FFE_WRAM_SIZE,
            battery_save_size: if has_battery { FFE_WRAM_SIZE } else { 0 },
            extended_mode,
            initial_mirr,
        }
    }
}

pub struct MapperFfe {
    cfg: FfeConfig,
    latch: u8,
    preg: [u8; 4],
    creg: [u8; 8],
    mirr: u8,
    irq_active: bool,
    irq_count: u32,
}

impl MapperFfe {
    pub fn new(cfg: FfeConfig) -> Self {
        let mirr = cfg.initial_mirr;
        Self {
            cfg,
            latch: 0,
            preg: [0xFF; 4],
            creg: [0; 8],
            mirr,
            irq_active: false,
            irq_count: 0,
        }
    }

    fn write_mirr_register(&mut self, address: u16, data: u8) {
        self.mirr = (((address as u8) << 1) & 2) | ((data >> 4) & 1);
    }

    fn ciram_offset_fceux(&self, address: u16) -> usize {
        let slot = (address >> 10) & 3;
        let off = (address & 0x03FF) as usize;
        match self.mirr {
            0 => off,
            1 => 0x400 | off,
            2 => {
                if slot & 1 == 0 {
                    off
                } else {
                    0x400 | off
                }
            }
            3 | _ => {
                if slot >= 2 {
                    0x400 | off
                } else {
                    off
                }
            }
        }
    }

    fn ciram_offset(&self, address: u16) -> usize {
        self.ciram_offset_fceux(address)
    }

    fn prg_read(&self, cart: &Cartridge, address: u16) -> u8 {
        let prg_len = cart.prg_rom.len();
        if prg_len == 0 {
            return 0;
        }
        if self.cfg.extended_mode {
            let region = ((address - 0x8000) >> 13) as usize;
            let bank = self.preg[region] as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            return cart.prg_rom[offset % prg_len];
        }
        let banks_16k = (prg_len / 0x4000).max(1);
        let bank = if address < 0xC000 {
            ((self.latch >> 2) & 0x3F) as usize % banks_16k
        } else {
            7 % banks_16k
        };
        let offset = bank * 0x4000 + (address as usize & 0x3FFF);
        cart.prg_rom[offset % prg_len]
    }

    fn chr_offset(&self, address: u16, chr_len: usize) -> usize {
        let offset = if self.cfg.extended_mode {
            let slot = (address >> 10) as usize;
            let page = self.creg[slot] as usize;
            page * 0x0400 + (address as usize & 0x03FF)
        } else {
            let page = (self.latch & 3) as usize;
            page * 0x2000 + (address as usize & 0x1FFF)
        };
        offset % chr_len
    }

    fn chr_read(
        &self,
        address: u16,
        chr_rom: &[u8],
        chr_ram: &[u8],
        using_chr_ram: bool,
    ) -> u8 {
        let len = if using_chr_ram { chr_ram.len() } else { chr_rom.len() };
        if len == 0 {
            return 0;
        }
        let offset = self.chr_offset(address, len);
        if using_chr_ram {
            chr_ram[offset]
        } else {
            chr_rom[offset]
        }
    }

    fn ffe_irq_tick(&mut self) -> bool {
        if !self.irq_active {
            return false;
        }
        self.irq_count = self.irq_count.saturating_add(IRQ_CYCLES_PER_INSTRUCTION);
        if self.irq_count >= IRQ_THRESHOLD {
            self.irq_active = false;
            self.irq_count = 0;
            return true;
        }
        false
    }
}

impl Mapper for MapperFfe {
    fn reset(&mut self) {
        self.latch = 0;
        self.preg = [0xFF; 4];
        self.creg = [0; 8];
        self.mirr = self.cfg.initial_mirr;
        self.irq_active = false;
        self.irq_count = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            if cart.prg_ram.is_empty() {
                return FetchResult {
                    data: 0,
                    driven: false,
                };
            }
            let offset = (address as usize - 0x6000) % cart.prg_ram.len();
            return FetchResult {
                data: cart.prg_ram[offset],
                driven: true,
            };
        }
        if address >= 0x8000 {
            return FetchResult {
                data: self.prg_read(cart, address),
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let offset = (address as usize - 0x6000) % cart.prg_ram.len();
                cart.prg_ram[offset] = data;
            }
            return;
        }
        if address >= 0x8000 {
            self.latch = data;
            return;
        }
        match address {
            0x42FE | 0x42FF => self.write_mirr_register(address, data),
            0x4500 => {}
            0x4501 => self.irq_active = false,
            0x4502 => {
                self.irq_count = (self.irq_count & 0xFFFF_0000) | data as u32;
            }
            0x4503 => {
                self.irq_count = (self.irq_count & 0x0000_00FF) | ((data as u32) << 8);
                self.irq_active = true;
            }
            0x4504..=0x4507 => {
                if self.cfg.extended_mode {
                    self.preg[(address & 3) as usize] = data;
                }
            }
            0x4510..=0x4517 => {
                if self.cfg.extended_mode {
                    self.creg[(address & 7) as usize] = data;
                }
            }
            _ => {}
        }
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let offset = self.chr_offset(address, cart.chr_ram.len());
                cart.chr_ram[offset] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            vram[self.ciram_offset(address)] = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.ciram_offset(address) as u16
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
            let byte = vram[self.ciram_offset(address)];
            new_addr_bus |= byte as u16;
        } else {
            let byte = self.chr_read(address, chr_rom, chr_ram, using_chr_ram);
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.ffe_irq_tick()
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::with_capacity(20);
        state.extend_from_slice(&self.preg);
        state.extend_from_slice(&self.creg);
        state.push(self.latch);
        state.push(self.mirr);
        state.push(self.irq_active as u8);
        state.extend_from_slice(&self.irq_count.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if state.len() < p + 16 {
            return p;
        }
        self.preg.copy_from_slice(&state[p..p + 4]);
        p += 4;
        self.creg.copy_from_slice(&state[p..p + 8]);
        p += 8;
        self.latch = state[p];
        p += 1;
        self.mirr = state[p];
        p += 1;
        self.irq_active = state[p] != 0;
        p += 1;
        if p + 4 <= state.len() {
            self.irq_count = u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
            p += 4;
        }
        p
    }
}
