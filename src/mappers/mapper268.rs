use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, mmc3_chr_bank};

pub struct Mapper268 {
    mmc3: MapperMMC3,
    reg: [u8; 8],
}

impl Mapper268 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let config = Mmc3Config {
            ax5202p: true,
            ..Mmc3Config::for_ines(header, 0, if chr_size == 0 { 0 } else { chr_size }, rom, rom_name)
        };
        Self { mmc3: MapperMMC3::new(config), reg: [0; 8] }
    }

    fn fixed_last(&self, cart: &Cartridge) -> u8 {
        ((cart.prg_rom.len() / 0x2000).saturating_sub(1)) as u8
    }

    fn fixed_second_last(&self, cart: &Cartridge) -> u8 {
        ((cart.prg_rom.len() / 0x2000).saturating_sub(2)) as u8
    }

    fn mmc3_prg_bank(&self, slot: usize, cart: &Cartridge) -> u8 {
        match slot {
            0 => {
                if (self.mmc3.r8000 & 0x40) == 0 { self.mmc3.bank_8c } else { self.fixed_second_last(cart) }
            }
            1 => self.mmc3.bank_a,
            2 => {
                if (self.mmc3.r8000 & 0x40) != 0 { self.mmc3.bank_8c } else { self.fixed_second_last(cart) }
            }
            3 => self.fixed_last(cart),
            _ => 0,
        }
    }

    fn prg_masks_and_offset(&self) -> (u16, u16, u16) {
        let r0 = self.reg[0];
        let r1 = self.reg[1];
        let r3 = self.reg[3];

        let mut prg_mask_mmc3 = 0u16;
        if r3 & 0x10 == 0 { prg_mask_mmc3 |= 0x0F; }
        if r0 & 0x40 == 0 { prg_mask_mmc3 |= 0x10; }
        if r1 & 0x80 == 0 { prg_mask_mmc3 |= 0x20; }
        if r1 & 0x40 != 0 { prg_mask_mmc3 |= 0x40; }
        if r1 & 0x20 != 0 { prg_mask_mmc3 |= 0x80; }

        let prg_mask_gnrom: u16 = if r3 & 0x10 != 0 {
            if r1 & 0x02 != 0 { 0x03 } else { 0x01 }
        } else { 0 };

        let prg_offset = (r3 & 0x0E) as u16
            | ((r0 as u16 & 0x07) << 4)
            | if r1 & 0x10 != 0 { 0x80 } else { 0 }
            | ((r1 as u16 & 0x0C) << 6)
            | ((r0 as u16 & 0x30) << 6)
            | ((!r1 as u16 & 1) << 12);

        (prg_mask_mmc3, prg_mask_gnrom, prg_offset)
    }

    fn chr_masks_and_offset(&self) -> (u16, u16, u16) {
        let r0 = self.reg[0];
        let r2 = self.reg[2];
        let r3 = self.reg[3];

        let chr_mask_mmc3: u16 = if r3 & 0x10 != 0 {
            0
        } else if r0 & 0x80 != 0 {
            0x7F
        } else {
            0xFF
        };

        let chr_offset = ((r2 as u16) << 3 & 0x78)
            | ((r0 as u16) << 4 & 0x380)
            | ((r0 as u16) << 9 & 0xC00);

        let chr_mask_gnrom: u16 = if r3 & 0x10 != 0 { 0x07 } else { 0 };

        (chr_mask_mmc3, chr_mask_gnrom, chr_offset)
    }

    fn effective_prg_bank(&self, mmc3_bank: u8, slot: usize) -> usize {
        let (prg_mask_mmc3, prg_mask_gnrom, prg_offset) = self.prg_masks_and_offset();
        let masked_offset = prg_offset & !(prg_mask_mmc3 | prg_mask_gnrom);
        (mmc3_bank as u16 & prg_mask_mmc3 | masked_offset | (slot as u16 & prg_mask_gnrom)) as usize
    }

    fn mmc4_prg_bank(&self, slot: usize) -> usize {
        let (prg_mask_mmc3, prg_mask_gnrom, prg_offset) = self.prg_masks_and_offset();
        let masked_offset = prg_offset & !(prg_mask_mmc3 | prg_mask_gnrom);
        (masked_offset | (slot as u16 & prg_mask_gnrom)) as usize
    }

    fn effective_chr_bank(&self, mmc3_chr_bank: u8, slot: usize) -> (usize, bool) {
        let r4 = self.reg[4];
        let (chr_mask_mmc3, chr_mask_gnrom, chr_offset) = self.chr_masks_and_offset();
        let masked_offset = chr_offset & !(chr_mask_mmc3 | chr_mask_gnrom);
        let bank = (mmc3_chr_bank as u16 & chr_mask_mmc3) | masked_offset | (slot as u16 & chr_mask_gnrom);
        let chr_ram_override = r4 & 0x01 != 0 && (mmc3_chr_bank & 0xFE) == (r4 & 0xFE);
        (bank as usize, chr_ram_override)
    }

    fn mmc4_chr_bank(&self, slot: usize) -> usize {
        let (chr_mask_mmc3, chr_mask_gnrom, chr_offset) = self.chr_masks_and_offset();
        let masked_offset = chr_offset & !(chr_mask_mmc3 | chr_mask_gnrom);
        let base = match slot & 3 {
            0 => self.mmc3.chr_2k0 as u16,
            1 => 0,
            2 => self.mmc3.chr_2k8 as u16,
            3 => 0,
            _ => 0,
        };
        let slot_gnrom = slot & 3;
        let bank = (base & chr_mask_mmc3) | masked_offset | (slot_gnrom as u16 & chr_mask_gnrom);
        bank as usize
    }

    fn mmc4_active(&self) -> bool {
        self.reg[3] & 0x40 != 0
    }

    fn mmc4_chr_slot_overridden(&self, slot: usize) -> bool {
        ((self.mmc3.r8000 & 0x80) == 0) == (slot < 4)
    }
}

impl Mapper for Mapper268 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.reg = [0; 8];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let slot = match address {
                0x8000..=0x9FFF => 0,
                0xA000..=0xBFFF => 1,
                0xC000..=0xDFFF => 2,
                0xE000..=0xFFFF => 3,
                _ => 0,
            };
            let bank = if self.mmc4_active() && (self.mmc3.r8000 & 0x40) == 0 && slot >= 2 {
                self.mmc4_prg_bank(slot)
            } else {
                let mmc3_bank = self.mmc3_prg_bank(slot, cart);
                self.effective_prg_bank(mmc3_bank, slot)
            };
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: if !cart.prg_rom.is_empty() { cart.prg_rom[offset % cart.prg_rom.len()] } else { 0 },
                driven: true,
            }
        } else if address >= 0x6000 {
            self.mmc3.fetch_prg(cart, address)
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x6000 {
            self.mmc3.store_prg(cart, address, data);
        } else if address >= 0x6000 && address < 0x8000 {
            let addr = (address & 7) as usize;
            if (self.reg[3] & 0x80) == 0 || addr == 2 {
                let val = if addr == 2 {
                    let v = if self.reg[2] & 0x80 != 0 { data & 0x0F | self.reg[2] & !0x0F } else { data };
                    v & (!((self.reg[2] >> 3) & 0x0E) | 0xF1)
                } else {
                    data
                };
                if addr <= 5 {
                    self.reg[addr] = val;
                }
            }
            self.mmc3.store_prg(cart, address, data);
        } else {
            self.mmc3.store_prg(cart, address, data);
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
        prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let slot = (address as usize >> 10) & 7;
            let raw_bank = mmc3_chr_bank(
                self.mmc3.r8000, self.mmc3.chr_2k0, self.mmc3.chr_2k8,
                self.mmc3.chr_1k0, self.mmc3.chr_1k4, self.mmc3.chr_1k8, self.mmc3.chr_1kc, address,
            );
            let (bank, chr_ram_ovr) = if self.mmc4_active() && self.mmc4_chr_slot_overridden(slot) {
                (self.mmc4_chr_bank(slot), true)
            } else {
                self.effective_chr_bank(raw_bank, slot)
            };
            let offset = bank * 0x0400 + (address as usize & 0x03FF);
            let byte = if chr_ram_ovr && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else if !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else { 0 };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if self.mmc3.nametable_mirroring() {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() { prg_vram[idx] } else { 0 }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && !cart.chr_ram.is_empty() {
            let slot = (address as usize >> 10) & 7;
            let raw_bank = mmc3_chr_bank(
                self.mmc3.r8000, self.mmc3.chr_2k0, self.mmc3.chr_2k8,
                self.mmc3.chr_1k0, self.mmc3.chr_1k4, self.mmc3.chr_1k8, self.mmc3.chr_1kc, address,
            );
            let bank = if self.mmc4_active() && self.mmc4_chr_slot_overridden(slot) {
                self.mmc4_chr_bank(slot)
            } else {
                let (b, _) = self.effective_chr_bank(raw_bank, slot);
                b
            };
            let offset = bank * 0x0400 + (address as usize & 0x03FF);
            let len = cart.chr_ram.len();
            cart.chr_ram[offset % len] = data;
        } else {
            self.mmc3.store_ppu(cart, address, data, vram);
        }
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

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.extend_from_slice(&self.reg);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        if p + 8 <= state.len() {
            self.reg.copy_from_slice(&state[p..p + 8]);
            p += 8;
        }
        p
    }
}
