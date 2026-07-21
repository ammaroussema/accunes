use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, mmc3_chr_bank};

pub struct Mapper369 {
    mmc3: MapperMMC3,
    outer_bank: u8,
    smb2j_bank: u8,
    m2_counting: bool,
    m2_counter: u16,
    irq_ack_pending: bool,
}

impl Mapper369 {
    pub fn new(header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let config = Mmc3Config {
            prg_ram_size: 0,
            chr_ram_size: if chr_size == 0 { 0x2000 } else { 0 },
            mmc6: false,
            ax5202p: true,
            irq_revision_b: true,
            irq_hack: crate::mappers::mmc3::Mmc3IrqHack::None,
            header_horizontal_mirror: (header.get(6).copied().unwrap_or(0) & 1) == 0,
        };
        Self {
            mmc3: MapperMMC3::new(config),
            outer_bank: 0,
            smb2j_bank: 0,
            m2_counting: false,
            m2_counter: 0,
            irq_ack_pending: false,
        }
    }
}

impl Mapper for Mapper369 {
    fn reset(&mut self) {
        self.outer_bank = 0;
        self.smb2j_bank = 0;
        self.m2_counting = false;
        self.m2_counter = 0;
        self.irq_ack_pending = false;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len().max(1);
            let prg_rom = &cart.prg_rom;
            match self.outer_bank {
                0x00 => {
                    let bank = (address as usize - 0x8000) / 0x2000;
                    let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                    return FetchResult { data: prg_rom[offset % len], driven: true };
                }
                0x01 => {
                    let bank = ((address as usize - 0x8000) / 0x2000) + 4;
                    let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                    return FetchResult { data: prg_rom[offset % len], driven: true };
                }
                0x13 => {
                    let bank = match address {
                        0x8000..=0x9FFF => 0x0Cusize,
                        0xA000..=0xBFFF => 0x0D,
                        0xC000..=0xDFFF => 0x08 | (self.smb2j_bank as usize & 0x03),
                        0xE000..=0xFFFF => 0x0F,
                        _ => 0,
                    };
                    let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                    return FetchResult { data: prg_rom[offset % len], driven: true };
                }
                0x37 | 0xFF => {
                    let mask_8k = len / 0x2000;
                    let last = if mask_8k > 0 { mask_8k - 1 } else { 0 };
                    let second_last = if last > 0 { last - 1 } else { 0 };
                    let mode = (self.mmc3.r8000 & 0x40) != 0;
                    let page = (address as usize - 0x8000) / 0x2000;
                    let mmc3_bank = match (page, mode) {
                        (0, false) => self.mmc3.bank_8c as usize,
                        (0, true) => second_last,
                        (1, _) => self.mmc3.bank_a as usize,
                        (2, false) => second_last,
                        (2, true) => self.mmc3.bank_8c as usize,
                        (3, _) => last,
                        _ => 0,
                    };
                    let prg_and = if self.outer_bank == 0x37 { 0x0F } else { 0x1F };
                    let prg_or = if self.outer_bank == 0x37 { 0x10 } else { 0x20 };
                    let bank = (mmc3_bank & prg_and) | (prg_or & !prg_and);
                    let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                    return FetchResult { data: prg_rom[offset % len], driven: true };
                }
                _ => {}
            }
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, val: u8) {
        if address >= 0x8000 {
            match self.outer_bank {
                0x37 | 0xFF => {
                    self.mmc3.store_prg(cart, address, val);
                }
                0x13 => {
                    match address {
                        0x8000..=0x9FFF => {
                            self.irq_ack_pending = true;
                            self.mmc3.store_prg(cart, address, val);
                        }
                        0xA000..=0xBFFF => {
                            self.m2_counting = val & 2 != 0;
                            self.mmc3.store_prg(cart, address, val);
                        }
                        0xE000..=0xFFFF => {
                            self.smb2j_bank = val;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_cpu_write(&mut self, address: u16, val: u8) {
        if address >= 0x4000 && (address & 0x100) != 0 {
            self.outer_bank = val;
            self.m2_counting = false;
            self.m2_counter = 0;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        let mmc3_h = self.mmc3.nametable_mirroring();
        if cart.alternative_nametable_arrangement { address } else { mirror_h_or_v(mmc3_h, address) }
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
            match self.outer_bank {
                0x00 => {
                    if using_chr_ram && !chr_ram.is_empty() {
                        new_addr_bus |= chr_ram[(address as usize) % chr_ram.len()] as u16;
                    } else if !chr_rom.is_empty() {
                        let offset = address as usize & 0x1FFF;
                        new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
                    }
                }
                0x01 => {
                    if using_chr_ram && !chr_ram.is_empty() {
                        let offset = 0x2000 + (address as usize & 0x1FFF);
                        new_addr_bus |= chr_ram[offset % chr_ram.len()] as u16;
                    } else if !chr_rom.is_empty() {
                        let offset = 0x2000 + (address as usize & 0x1FFF);
                        new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
                    }
                }
                0x13 => {
                    if using_chr_ram && !chr_ram.is_empty() {
                        new_addr_bus |= chr_ram[(address as usize) % chr_ram.len()] as u16;
                    } else if !chr_rom.is_empty() {
                        let offset = 0x03 * 0x2000 + (address as usize & 0x1FFF);
                        new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
                    }
                }
                0x37 | 0xFF => {
                    if using_chr_ram && !chr_ram.is_empty() {
                        new_addr_bus |= chr_ram[(address as usize) % chr_ram.len()] as u16;
                    } else if !chr_rom.is_empty() {
                        let chr_bank = mmc3_chr_bank(
                            self.mmc3.r8000, self.mmc3.chr_2k0, self.mmc3.chr_2k8,
                            self.mmc3.chr_1k0, self.mmc3.chr_1k4, self.mmc3.chr_1k8, self.mmc3.chr_1kc,
                            address,
                        ) as usize;
                        let chr_and = 0x7F;
                        let chr_or = if self.outer_bank == 0x37 { 0x080 } else { 0x100 };
                        let bank = (chr_bank & chr_and) | (chr_or & !chr_and);
                        let offset = bank * 0x400 + (address as usize & 0x3FF);
                        new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
                    }
                }
                _ => {}
            }
        } else {
            let mir = match self.outer_bank {
                0x37 | 0xFF => {
                    let mmc3_h = self.mmc3.nametable_mirroring();
                    if _alternative_nametable_arrangement { address } else { mirror_h_or_v(mmc3_h, address) }
                }
                _ => {
                    if _alternative_nametable_arrangement {
                        address
                    } else if _nametable_horizontal_mirroring {
                        (address & 0x33FF) | ((address & 0x0800) >> 1)
                    } else {
                        address & 0x37FF
                    }
                }
            };
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram && !cart.chr_ram.is_empty() {
            let len = cart.chr_ram.len();
            cart.chr_ram[(address as usize) % len] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mir = self.mirror_nametable(cart, address);
            vram[(mir & 0x7FF) as usize] = data;
        }
    }

    fn ppu_clock(&mut self, ppu_address_bus: u16, ppu_a12_prev: bool, scanline: u16, dot: u16, ppu_sprite_x16: bool, rendering_on: bool) -> bool {
        if self.outer_bank == 0x13 {
            false
        } else {
            self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.outer_bank == 0x13 && self.m2_counting {
            self.m2_counter = self.m2_counter.wrapping_add(_cycles as u16);
            if self.m2_counter & 0x0FFF == 0 && self.m2_counter != 0 {
                return true;
            }
        }
        false
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.irq_ack_pending {
            self.irq_ack_pending = false;
            return true;
        }
        self.mmc3.take_irq_ack()
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.outer_bank);
        state.push(self.smb2j_bank);
        state.push(self.m2_counting as u8);
        state.extend_from_slice(&self.m2_counter.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let p = self.mmc3.load_mapper_registers(cart, state, start);
        let mut p = p;
        if p < state.len() { self.outer_bank = state[p]; p += 1; }
        if p < state.len() { self.smb2j_bank = state[p]; p += 1; }
        if p < state.len() { self.m2_counting = state[p] != 0; p += 1; }
        if p + 2 <= state.len() { self.m2_counter = u16::from_le_bytes([state[p], state[p+1]]); p += 2; }
        p
    }
}
