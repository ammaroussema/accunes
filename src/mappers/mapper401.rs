// Mapper 401 - KC885 (MMC3-based with AX5202P four-register latch)
//
// Reference: NintendulatorNRS-DBG MMC3-based/mapper401.cpp
//
// The MMC3 core handles $8000-$FFFF (bank select, mirroring, IRQ). The AX5202P
// four-register latch is written through the $6000-$7FFF window: the reference
// passes its writeReg as the WRAM write callback (6th arg of MMC3::load), so a
// CPU write there writes PRG RAM and latches reg[index++ & 3]. Register values
// AND/OR-mask the MMC3 PRG/CHR banks, and the dip switches select alternative
// PRG bits and can force the whole $8000+ space to open bus.

use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper401 {
    mmc3: MapperMMC3,
    index: u8,
    reg: [u8; 4],
    dip_switches: u8,
    irq_clear_pending: bool,
}

impl Mapper401 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let mut config = Mmc3Config::for_ines(header, 0, chr_size, rom, rom_name);
        config.ax5202p = true;
        Self {
            mmc3: MapperMMC3::new(config),
            index: 0,
            reg: [0x00, 0x00, 0x0F, 0x00],
            dip_switches: 0,
            irq_clear_pending: false,
        }
    }

    fn prg_and(&self) -> usize {
        (!self.reg[3] & 0x1F) as usize
    }

    fn prg_or(&self) -> u8 {
        let mut or = (self.reg[1] & 0x1F) | (self.reg[2] & 0x80);
        if (self.dip_switches & 2) != 0 {
            or |= self.reg[2] & 0x20;
        } else {
            or |= (self.reg[1] >> 1) & 0x20;
        }
        if (self.dip_switches & 4) != 0 {
            or |= self.reg[2] & 0x40;
        } else {
            or |= (self.reg[1] << 1) & 0x40;
        }
        or
    }

    fn chr_and(&self) -> usize {
        0xFFusize >> ((!self.reg[2] & 0x0F) as usize)
    }

    fn chr_or(&self) -> usize {
        (self.reg[0] as usize) | (((self.reg[2] as usize) << 4) & 0xF00)
    }
}

impl Mapper for Mapper401 {
    fn reset(&mut self) {
        self.index = 0;
        self.reg = [0x00, 0x00, 0x0F, 0x00];
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if (self.dip_switches & 1) != 0 && (self.reg[1] & 0x80) != 0 {
                return FetchResult {
                    data: 0,
                    driven: false,
                };
            }
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: false,
                };
            }
            let last = (len / 0x2000).saturating_sub(1);
            let second_last = last.saturating_sub(1);
            let mode = (self.mmc3.r8000 & 0x40) != 0;
            let page = (address - 0x8000) / 0x2000;
            let mmc3_bank = match (page, mode) {
                (0, false) => self.mmc3.bank_8c as usize,
                (0, true) => second_last,
                (1, _) => self.mmc3.bank_a as usize,
                (2, false) => second_last,
                (2, true) => self.mmc3.bank_8c as usize,
                (_, _) => last,
            };
            let bank = (mmc3_bank & self.prg_and()) | self.prg_or() as usize;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            }
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
            // Reference wramWrite_MMC3 for AX5202P: writes to $6000-$7FFF go to
            // PRG RAM and feed the four-register latch (cbWRAMWrite), gated on
            // wramControl bit 6 being clear.
            if (self.mmc3.prg_ram_protect & 0x40) == 0 {
                if !cart.prg_ram.is_empty() {
                    let off = (address - 0x6000) as usize;
                    if off < cart.prg_ram.len() {
                        cart.prg_ram[off] = data;
                    }
                }
                if (self.reg[3] & 0x40) == 0 {
                    self.reg[self.index as usize & 3] = data;
                    self.index = self.index.wrapping_add(1);
                }
            }
        } else {
            self.mmc3.store_prg(cart, address, data);
            if (address & 0xE001) == 0xE000 {
                self.irq_clear_pending = true;
            }
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
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let chr_bank = self.mmc3.chr_bank(address) as usize;
            let bank = (chr_bank & self.chr_and()) | self.chr_or();
            let offset = bank * 0x400 + (address as usize & 0x3FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
            (new_addr_bus as u8, new_addr_bus)
        } else {
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

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.index);
        state.extend_from_slice(&self.reg);
        state.push(self.dip_switches);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        if p < state.len() {
            self.index = state[p];
            p += 1;
        }
        for i in 0..4 {
            if p < state.len() {
                self.reg[i] = state[p];
                p += 1;
            }
        }
        if p < state.len() {
            self.dip_switches = state[p];
            p += 1;
        }
        p
    }
}
