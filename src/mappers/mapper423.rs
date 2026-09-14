use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};
use crate::mappers::one_bus::{OneBus, OneBusBanking, OneBusChrCtx, OneBusMangle};

pub struct Mapper423 {
    core: OneBus,
}

impl Default for Mapper423 {
    fn default() -> Self {
        Self::new()
    }
}

impl Mapper423 {
    pub fn new() -> Self {
        let mut core = OneBus::new(&[], &[], OneBusBanking::MAPPER256);
        core.console_type_vt03 = true;
        Self { core }
    }

    fn sync(&mut self) {
        let prg_or = (self.core.reg4100[0x50] as u16) << 12;
        let chr_or = (self.core.reg4100[0x50] as usize) << 15;
        self.core.banking = OneBusBanking {
            prg_and: 0x0FFF,
            prg_or,
            chr_and: 0x7FFF,
            chr_or,
        };
    }

    fn prg_or(&self) -> u16 {
        (self.core.reg4100[0x50] as u16) << 12
    }
}

impl Mapper for Mapper423 {
    fn reset(&mut self) {
        self.core.reset();
        self.core.reg4100[0x50] = 0;
        self.sync();
    }

    fn handle_cpu_write(&mut self, address: u16, data: u8) {
        let mangle = OneBusMangle::IDENTITY;
        if (0x2000..0x2100).contains(&address) {
            self.core.write_ppu(address, data, &mangle);
        } else if (0x4100..0x4200).contains(&address) {
            self.core.write_apu(address, data, &mangle);
            if address == 0x4150 {
                self.sync();
            }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.core.store_prg_mmc3(address, data, &OneBusMangle::IDENTITY);
        }
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address == 0x4153 {
            return FetchResult { data: 0x80, driven: true };
        }
        if address >= 0x4100 && address < 0x4200 {
            if let Some(data) = self.core.read_apu(address) {
                return FetchResult { data, driven: true };
            }
        }
        if address >= 0x8000 {
            let slot = ((address - 0x8000) >> 13) as usize;
            let bank = (self.core.get_prg_bank(slot) & 0x0FFF) | (self.prg_or() as usize);
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            let data = if !cart.prg_rom.is_empty() {
                cart.prg_rom[offset % cart.prg_rom.len()]
            } else {
                0
            };
            return FetchResult { data, driven: true };
        }
        FetchResult { data: 0, driven: false }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v(self.core.hv() != 0, address)
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
        self.fetch_ppu_with_ctx(
            prg_rom, chr_rom, prg_ram, chr_ram, prg_vram,
            using_chr_ram, nametable_horizontal_mirroring, alternative_nametable_arrangement,
            ppu_address_bus, ppu_octal_latch, vram,
            OneBusChrCtx::default(),
        )
    }

    fn fetch_ppu_with_ctx(
        &mut self,
        prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
        ctx: OneBusChrCtx,
    ) -> (u8, u16) {
        let raw_address = (ppu_address_bus & 0x7FFF) | (ppu_octal_latch as u16);
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        let is_vt = self.core.console_type_vt03 || self.core.console_type_vt09 || self.core.console_type_vt369 || true;
        let is_chr_fetch = raw_address < 0x2000
            || (raw_address >= 0x4000 && raw_address < 0x6000)
            || (is_vt && ctx.active && ((raw_address >= 0x2000 && raw_address < 0x4000) || (raw_address >= 0x6000 && raw_address < 0x8000)));
        if is_chr_fetch {
            let high_plane = (raw_address & 0x4000) != 0;
            let chr_addr = raw_address & 0x1FFF;
            let ext_address = if ctx.active && is_vt {
                ctx.map_chr_address(if high_plane { 0x4000 | chr_addr } else { chr_addr })
            } else if high_plane {
                0x4000 | chr_addr
            } else {
                chr_addr
            };
            let (is_bg, is_sprite, chr_eva) = if ctx.active && is_vt {
                (ctx.is_bg, ctx.is_sprite, ctx.eva)
            } else {
                (false, false, 0)
            };
            let byte = self.core.fetch_chr_byte_ext(prg_rom, chr_rom, chr_ram, ext_address, false, is_bg, is_sprite, chr_eva);
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mirror_h_or_v(self.core.hv() != 0, raw_address);
            let byte = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 || (address >= 0x4000 && address < 0x6000) {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let slot = ((address >> 10) as usize & 7) ^ if self.core.comr7() { 4 } else { 0 };
                let bank = self.core.chr_bank_1k(slot);
                let offset = bank * 0x400 + (address as usize & 0x3FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
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
        self.core.ppu_cycle(ppu_address_bus, scanline, dot, rendering_on)
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.core.cpu_cycle()
    }

    fn take_irq_ack(&mut self) -> bool {
        self.core.take_irq_ack()
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        self.core.save_core()
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let p = self.core.load_core(state, start);
        self.sync();
        p
    }

    fn vt03_4bpp_bg(&self) -> bool { (self.core.reg2000[0x10] & 0x02) != 0 }
    fn vt03_4bpp_sp(&self) -> bool { (self.core.reg2000[0x10] & 0x04) != 0 }
    fn vt03_reg2000_10(&self) -> u8 { self.core.reg2000[0x10] }
    fn onebus_vt03_ppu(&self) -> bool { true }
    fn is_vt32(&self) -> bool { false }
    fn onebus_chr_routing_ppu(&self) -> bool { true }
}
