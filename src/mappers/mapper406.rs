// Mapper 406 - Impact Soft (MMC3-based with AMD/Macronix flash ROM)
//
// Reference: NintendulatorNRS-DBG MMC3-based/mapper406.cpp
//
// The MMC3 core (AX5202P type) banks PRG/CHR, while the flash chip is the PRG
// ROM itself: the flash data aliases the PRG ROM data in place, so program and
// sector/chip erase operations modify the running ROM directly. Every $8000+
// read goes through the flash (software ID and toggle-bit states), and every
// $8000+ write feeds the flash command state machine plus the MMC3 registers.
// The MMC3 register decode is scrambled per submapper: submapper 1 swaps the
// $8000/$E000 register regions and uses A0, submapper 0 uses A1 as A0.

use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

const SECTOR_SIZE: usize = 65536;
const MAGIC_ADDR1: u16 = 0x5555;
const MAGIC_ADDR2: u16 = 0x2AAA;

pub struct Mapper406 {
    mmc3: MapperMMC3,
    sub_mapper_1: bool,
    manufacturer_id: u8,
    model_id: u8,
    flash_state: u8,
    time_out: u32,
    irq_clear_pending: bool,
}

impl Mapper406 {
    pub fn new(submapper_id: u8, header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let mut config = Mmc3Config::for_ines(header, 0, chr_size, rom, rom_name);
        config.ax5202p = true;
        config.irq_revision_b = false;
        let sub_mapper_1 = submapper_id == 1;
        Self {
            mmc3: MapperMMC3::new(config),
            sub_mapper_1,
            manufacturer_id: if sub_mapper_1 { 0x01 } else { 0xC2 },
            model_id: 0xA4,
            flash_state: 0,
            time_out: 0,
            irq_clear_pending: false,
        }
    }

    // Current 8KB PRG bank for the given $8000+ address, masked with the 0x3F
    // sync AND (MMC3::syncPRG(0x3F, 0) in the reference).
    fn prg_bank8(&self, cart: &Cartridge, address: u16) -> usize {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }
        let last = (len / 0x2000).saturating_sub(1);
        let second_last = last.saturating_sub(1);
        let mode = (self.mmc3.r8000 & 0x40) != 0;
        let page = ((address - 0x8000) / 0x2000) as usize;
        let mmc3_bank = match (page, mode) {
            (0, false) => self.mmc3.bank_8c as usize,
            (0, true) => second_last,
            (1, _) => self.mmc3.bank_a as usize,
            (2, false) => second_last,
            (2, true) => self.mmc3.bank_8c as usize,
            (_, _) => last,
        };
        mmc3_bank & 0x3F
    }

    // Reference writeFlash address reconstruction: the low two bits of the PRG
    // bank drive A14/A13 of the flash and bit 12 of the CPU address drives A12,
    // so the command addresses ($5555/$2AAA) are recognized regardless of the
    // currently-mapped 8KB window.
    fn flash_addr(&self, cart: &Cartridge, address: u16) -> u16 {
        let bank = self.prg_bank8(cart, address);
        (address & 0x1FFF) | (((bank & 3) as u16) << 13)
    }

    fn flash_write(&mut self, cart: &mut Cartridge, addr: u16, offset: usize, val: u8) {
        match self.flash_state {
            // command start
            0x01 => {
                if addr == MAGIC_ADDR2 && val == 0x55 {
                    self.flash_state = 0x02;
                }
            }
            0x02 => {
                if addr == MAGIC_ADDR1 {
                    self.flash_state = val;
                }
            }
            // sector or chip erase
            0x80 => {
                if addr == MAGIC_ADDR1 && val == 0xAA {
                    self.flash_state = 0x81;
                }
            }
            0x81 => {
                if addr == MAGIC_ADDR2 && val == 0x55 {
                    self.flash_state = 0x82;
                }
            }
            0x82 => {
                if val == 0x30 {
                    // sector erase
                    let len = cart.prg_rom.len();
                    if offset < len {
                        let start = offset & !(SECTOR_SIZE - 1);
                        let end = (start + SECTOR_SIZE).min(len);
                        for b in &mut cart.prg_rom[start..end] {
                            *b = 0xFF;
                        }
                        self.time_out = SECTOR_SIZE as u32;
                    }
                } else if val == 0x10 && addr == MAGIC_ADDR1 {
                    // chip erase
                    for b in cart.prg_rom.iter_mut() {
                        *b = 0xFF;
                    }
                    self.time_out = cart.prg_rom.len() as u32;
                } else if val == 0xF0 {
                    self.flash_state = 0;
                }
            }
            // software ID
            0x90 => {
                if val == 0xF0 {
                    self.flash_state = 0;
                }
            }
            // byte program
            0xA0 => {
                let len = cart.prg_rom.len();
                if offset < len {
                    cart.prg_rom[offset] = val;
                }
                self.flash_state = 0;
            }
            _ => {
                if addr == MAGIC_ADDR1 && val == 0xAA {
                    self.flash_state = 0x01;
                }
            }
        }
    }

    // Translate a CPU $8000+ address into the MMC3 register address: submapper
    // 1 swaps the $8000 and $E000 register regions (bank ^ 6) and keeps A0;
    // submapper 0 keeps the region and uses A1 as A0.
    fn mmc3_addr(&self, address: u16) -> u16 {
        if self.sub_mapper_1 {
            let region = match address & 0xE000 {
                0x8000 => 0xE000,
                0xE000 => 0x8000,
                r => r,
            };
            region | (address & 1)
        } else {
            (address & 0xE000) | ((address >> 1) & 1)
        }
    }
}

impl Mapper for Mapper406 {
    fn reset(&mut self) {
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if self.flash_state == 0x90 {
                // software ID
                let data = if address & 1 != 0 {
                    self.model_id
                } else {
                    self.manufacturer_id
                };
                return FetchResult { data, driven: true };
            }
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: false,
                };
            }
            let offset =
                (self.prg_bank8(cart, address) * 0x2000 + (address as usize & 0x1FFF)) % len;
            let raw = cart.prg_rom[offset];
            let data = if self.time_out > 0 {
                (raw ^ (if self.time_out & 1 != 0 { 0x40 } else { 0 })) & 0x77
            } else {
                raw
            };
            FetchResult { data, driven: true }
        } else if address >= 0x6000 {
            self.mmc3.fetch_prg(cart, address)
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            self.mmc3.store_prg(cart, address, data);
            return;
        }
        let offset = self.prg_bank8(cart, address) * 0x2000 + (address as usize & 0x1FFF);
        let addr = self.flash_addr(cart, address);
        self.flash_write(cart, addr, offset, data);
        let mmc3_addr = self.mmc3_addr(address);
        self.mmc3.store_prg(cart, mmc3_addr, data);
        if (mmc3_addr & 0xE001) == 0xE000 {
            self.irq_clear_pending = true;
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_clear_pending;
        self.irq_clear_pending = false;
        ack
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mmc3.mirror_nametable(cart, address)
    }

    fn fetch_ppu(
        &mut self,
        prg_rom: &[u8],
        chr_rom: &[u8],
        prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        self.mmc3.fetch_ppu(
            prg_rom,
            chr_rom,
            prg_ram,
            chr_ram,
            prg_vram,
            using_chr_ram,
            nametable_horizontal_mirroring,
            alternative_nametable_arrangement,
            ppu_address_bus,
            ppu_octal_latch,
            vram,
        )
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        self.mmc3.store_ppu(cart, address, data, vram);
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

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.time_out > 0 {
            self.time_out = self.time_out.saturating_sub(cycles as u32);
            if self.time_out == 0 {
                self.flash_state = 0;
            }
        }
        false
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.flash_state);
        state.extend_from_slice(&self.time_out.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        if p + 5 <= state.len() {
            self.flash_state = state[p];
            p += 1;
            self.time_out = u32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
            p += 4;
        }
        p
    }
}
