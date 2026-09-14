use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::MapperMMC3;
use crate::mappers::mmc3::Mmc3Config;

fn prg_mask_shift(submapper: u8) -> (u8, u8) {
    match submapper {
        1 => (0x1F, 5),
        2 => (0x0F, 4),
        3 => (0x1F, 5),
        4 => (0x1F, 4),
        _ => (0x0F, 4),
    }
}

fn chr_mask_shift(submapper: u8) -> (u8, u8) {
    match submapper {
        2 => (0xFF, 8),
        3 => (0xFF, 8),
        _ => (0x7F, 7),
    }
}

fn apply_submapper4(submapper: u8, reg: u8) -> (u8, u8) {
    if submapper == 4 {
        let prg_mask = if reg != 0 { 0x0F } else { 0x1F };
        let prg_shift = 4;
        (prg_mask, prg_shift)
    } else {
        prg_mask_shift(submapper)
    }
}

pub struct Mapper313 {
    mmc3: MapperMMC3,
    reg: u8,
    submapper: u8,
}

impl Mapper313 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str, submapper: u8) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let config = Mmc3Config {
            ax5202p: true,
            ..Mmc3Config::for_ines(header, submapper, if chr_size == 0 { 0 } else { chr_size }, rom, rom_name)
        };
        Self {
            mmc3: MapperMMC3::new(config),
            reg: 0,
            submapper,
        }
    }
}

impl Mapper for Mapper313 {
    fn reset(&mut self) {
        self.reg = self.reg.wrapping_add(1);
        if self.submapper == 4 { self.reg &= 3; }
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let num_8k = cart.prg_rom.len() / 0x2000;
            if num_8k == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let (prg_mask, prg_shift) = apply_submapper4(self.submapper, self.reg);
            let reg_base = (self.reg as usize) << prg_shift;
            let invert = (self.mmc3.r8000 & 0x40) != 0;
            let raw_bank = match address {
                0xE000..=0xFFFF => num_8k.saturating_sub(1),
                0xC000..=0xDFFF => {
                    if invert { self.mmc3.bank_8c as usize }
                    else { num_8k.saturating_sub(2) }
                }
                0xA000..=0xBFFF => self.mmc3.bank_a as usize,
                0x8000..=0x9FFF => {
                    if invert { num_8k.saturating_sub(2) }
                    else { self.mmc3.bank_8c as usize }
                }
                _ => 0,
            };
            let bank = (raw_bank & prg_mask as usize) | reg_base;
            let bank = bank % num_8k;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len().max(1)],
                driven: true,
            }
        } else if address >= 0x6000 {
            let off = (address - 0x6000) as usize;
            FetchResult {
                data: if off < cart.prg_ram.len() { cart.prg_ram[off] } else { 0 },
                driven: !cart.prg_ram.is_empty() && self.mmc3.config.prg_ram_size > 0,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.mmc3.store_prg(cart, address, data);
        } else if address >= 0x6000 && address < 0x8000 {
            let off = (address - 0x6000) as usize;
            if off < cart.prg_ram.len() {
                cart.prg_ram[off] = data;
            }
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
            let (chr_mask, chr_shift) = chr_mask_shift(self.submapper);
            let reg_base = (self.reg as usize) << chr_shift;
            let bank_in = crate::mappers::mmc3::mmc3_chr_bank(
                self.mmc3.r8000,
                self.mmc3.chr_2k0,
                self.mmc3.chr_2k8,
                self.mmc3.chr_1k0,
                self.mmc3.chr_1k4,
                self.mmc3.chr_1k8,
                self.mmc3.chr_1kc,
                address,
            );
            let bank = ((bank_in as usize) & chr_mask as usize) | reg_base;
            let offset = bank * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let horz = self.mmc3.nametable_mirroring();
            let mirrored = if horz {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let (chr_mask, chr_shift) = chr_mask_shift(self.submapper);
                let reg_base = (self.reg as usize) << chr_shift;
                let bank_in = crate::mappers::mmc3::mmc3_chr_bank(
                    self.mmc3.r8000,
                    self.mmc3.chr_2k0,
                    self.mmc3.chr_2k8,
                    self.mmc3.chr_1k0,
                    self.mmc3.chr_1k4,
                    self.mmc3.chr_1k8,
                    self.mmc3.chr_1kc,
                    address,
                );
                let bank = ((bank_in as usize) & chr_mask as usize) | reg_base;
                let offset = bank * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn ppu_clock(&mut self, ppu_address_bus: u16, ppu_a12_prev: bool, scanline: u16, dot: u16, ppu_sprite_x16: bool, rendering_on: bool) -> bool {
        self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.reg);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let p = self.mmc3.load_mapper_registers(cart, state, start);
        self.reg = state.get(p).copied().unwrap_or(0);
        p + 1
    }
}
