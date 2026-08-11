use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::Mmc3Config;
use crate::mappers::one_bus::{OneBus, OneBusBanking, OneBusChrCtx, OneBusMangle};

const PPU_MANGLE: [[u8; 6]; 16] = [
    [0, 1, 2, 3, 4, 5],
    [1, 0, 5, 4, 3, 2],
    [0, 1, 2, 3, 4, 5],
    [5, 4, 3, 2, 0, 1],
    [2, 5, 0, 4, 3, 1],
    [1, 0, 5, 4, 3, 2],
    [0, 1, 2, 3, 4, 5],
    [0, 1, 2, 3, 4, 5],
    [0, 1, 2, 3, 4, 5],
    [0, 1, 2, 3, 4, 5],
    [0, 1, 2, 3, 4, 5],
    [0, 1, 2, 3, 4, 5],
    [0, 1, 2, 3, 4, 5],
    [0, 1, 2, 3, 4, 5],
    [0, 1, 2, 3, 4, 5],
    [0, 1, 2, 3, 4, 5],
];

const CPU_MANGLE: [[u8; 4]; 16] = [
    [0, 1, 2, 3],
    [0, 1, 2, 3],
    [1, 0, 2, 3],
    [0, 1, 2, 3],
    [0, 1, 2, 3],
    [0, 1, 2, 3],
    [0, 1, 2, 3],
    [0, 1, 2, 3],
    [0, 1, 2, 3],
    [0, 1, 2, 3],
    [0, 1, 2, 3],
    [0, 1, 2, 3],
    [0, 1, 2, 3],
    [0, 1, 2, 3],
    [0, 1, 2, 3],
    [0, 1, 2, 3],
];

const MMC3_MANGLE: [[u8; 8]; 16] = [
    [0, 1, 2, 3, 4, 5, 6, 7],
    [5, 4, 3, 2, 1, 0, 6, 7],
    [0, 1, 2, 3, 4, 5, 7, 6],
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 1, 2, 3, 4, 5, 6, 7],
];

pub struct Mapper256 {
    core: OneBus,
    submapper: u8,
    vt369_vram: [u8; 0x800],
}

impl Mapper256 {
    pub fn new(_config: Mmc3Config, submapper: u8, header: &[u8]) -> Self {
        let is_nes20 = header.len() >= 16 && (header[7] & 0x0C) == 0x08;
        let extended_console = is_nes20 && (header[7] & 0x03) == 3;
        let console_id = if extended_console {
            header[13] & 0x0F
        } else {
            0x08
        };
        let is_vt369 = console_id == 0x0A;
        let is_vt09 = console_id == 0x08;
        let is_vt03 = console_id == 0x07;
        let mut core = OneBus::new(&[], &[], OneBusBanking::MAPPER256);
        core.submapper = submapper;
        core.opcode_encryption = submapper >= 12;
        if is_vt369 {
            core.console_type_vt369 = true;
        }
        if is_vt09 {
            core.console_type_vt09 = true;
        }
        if is_vt03 {
            core.console_type_vt03 = true;
        }
        Self {
            core,
            submapper,
            vt369_vram: [0u8; 0x800],
        }
    }

    fn mangle(&self) -> OneBusMangle {
        let s = self.submapper as usize;
        OneBusMangle {
            ppu: PPU_MANGLE[s],
            cpu: CPU_MANGLE[s],
            mmc3: MMC3_MANGLE[s],
        }
    }

    fn prg_ram_writable(&self) -> bool {
        (self.core.prg_ram_protect & 0x80) != 0 && (self.core.prg_ram_protect & 0x40) != 0
    }
}

impl Mapper for Mapper256 {
    fn reset(&mut self) {
        let is_vt369 = self.core.console_type_vt369;
        let is_vt09 = self.core.console_type_vt09;
        let is_vt03 = self.core.console_type_vt03;
        self.core.reset();
        self.core.console_type_vt369 = is_vt369;
        self.core.console_type_vt09 = is_vt09;
        self.core.console_type_vt03 = is_vt03;
    }

    fn handle_cpu_write(&mut self, address: u16, data: u8) {
        let mangle = self.mangle();
        if (0x2000..0x2100).contains(&address) {
            self.core.write_ppu(address, data, &mangle);
        } else if (0x4020..=0x403F).contains(&address) || (0x4100..0x4200).contains(&address) {
            self.core.write_apu(address, data, &mangle);
        } else if self.core.console_type_vt369 && (0x3000..0x4000).contains(&address) {
            let ppu_addr = (address & 0xFFF) | 0x2000;
            let mirrored = self.core.mirror_nametable_address(ppu_addr);
            let idx = (mirrored & 0x7FF) as usize;
            self.vt369_vram[idx] = data;
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            if self.core.console_type_vt369
                && address >= 0x1000
                && address < 0x2000
                && !cart.misc_rom.is_empty()
            {
                return;
            }
            if address >= 0x6000 && !cart.prg_ram.is_empty() && self.prg_ram_writable() {
                cart.prg_ram[(address - 0x6000) as usize] = data;
            }
            return;
        }
        let mangle = self.mangle();
        let val = if (0x8000..=0x9FFF).contains(&address) && (address & 1) == 0 {
            data & 0xF8 | mangle.mmc3[(data & 0x07) as usize]
        } else {
            data
        };
        self.core.write_mmc3(address, val, &mangle);
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x4020..0x4040).contains(&address) || (address >= 0x4100 && address < 0x4200) {
            if let Some(data) = self.core.read_apu(address) {
                return FetchResult { data, driven: true };
            }
        }
        if self.core.console_type_vt369 && address >= 0x1000 && address < 0x2000 {
            if !cart.misc_rom.is_empty() {
                let off = (address - 0x1000) as usize;
                if off < cart.misc_rom.len() {
                    return FetchResult {
                        data: cart.misc_rom[off],
                        driven: true,
                    };
                }
            }
            return FetchResult { data: 0, driven: false };
        }
        if address >= 0x2010 && address < 0x2100 {
            let idx = (address & 0xFF) as usize;
            return FetchResult {
                data: self.core.reg2000[idx],
                driven: true,
            };
        }
        if self.core.console_type_vt369 && (0x3000..0x4000).contains(&address) {
            let ppu_addr = (address & 0xFFF) | 0x2000;
            let mirrored = if cart.alternative_nametable_arrangement {
                ppu_addr
            } else {
                self.core.mirror_nametable_address(ppu_addr)
            };
            let data = if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() { cart.prg_vram[idx] } else { 0 }
            } else {
                self.vt369_vram[(mirrored & 0x7FF) as usize]
            };
            return FetchResult { data, driven: true };
        }
        if address >= 0x6000 && address < 0x8000 {
            if self.core.console_type_vt369 && (self.core.reg4100[0x1C] & 0x40) != 0 {
                let ps = self.core.ps();
                let prg_and = if ps == 7 { 0xFFu16 } else { 0x3Fu16 >> ps };
                let pa21 = (self.core.reg4100[0x00] >> 4) as u16;
                let prg_or = ((self.core.reg4100[0x0A] as u16) | (pa21 << 8)) & !prg_and;
                let raw_bank = self.core.reg4100[0x12] as u16;
                let bank = (((raw_bank & prg_and | prg_or) as u16 & self.core.banking.prg_and)
                    | self.core.banking.prg_or) as usize
                    + self.core.relative_8k;
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                let data = if !cart.prg_rom.is_empty() {
                    cart.prg_rom[offset % cart.prg_rom.len()]
                } else {
                    0
                };
                return FetchResult { data, driven: true };
            }
            if !cart.prg_ram.is_empty() {
                let off = (address - 0x6000) as usize;
                if off < cart.prg_ram.len() {
                    return FetchResult {
                        data: cart.prg_ram[off],
                        driven: true,
                    };
                }
            }
            return FetchResult { data: 0, driven: false };
        }
        if address >= 0x8000 {
            let data = self.core.fetch_prg_byte(&cart.prg_rom, address);
            return FetchResult {
                data,
                driven: true,
            };
        }
        FetchResult { data: 0, driven: false }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else {
            self.core.mirror_nametable_address(address)
        }
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
            OneBusChrCtx::default(),
        )
    }

    fn fetch_ppu_with_ctx(
        &mut self,
        prg_rom: &[u8],
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
        ctx: OneBusChrCtx,
    ) -> (u8, u16) {
        let raw_address = (ppu_address_bus & 0x7FFF) | (ppu_octal_latch as u16);
        let mut new_addr_bus = ppu_address_bus & 0xFF00;

        let is_chr_fetch = raw_address < 0x2000 || (raw_address >= 0x4000 && raw_address < 0x6000);

        if is_chr_fetch {
            let high_plane = raw_address >= 0x4000 && raw_address < 0x6000;
            let chr_addr = raw_address & 0x1FFF;
            let ext_address = if ctx.active
                && (self.core.console_type_vt03
                    || self.core.console_type_vt09
                    || self.core.console_type_vt369)
            {
                ctx.map_chr_address(if high_plane {
                    0x4000 | chr_addr
                } else {
                    chr_addr
                })
            } else if high_plane {
                0x4000 | chr_addr
            } else {
                chr_addr
            };

            let (is_bg, is_sprite, chr_eva) = if ctx.active
                && (self.core.console_type_vt03
                    || self.core.console_type_vt09
                    || self.core.console_type_vt369)
            {
                (ctx.is_bg, ctx.is_sprite, ctx.eva)
            } else {
                (false, false, 0)
            };

            let byte = self.core.fetch_chr_byte_ext(
                prg_rom,
                chr_rom,
                chr_ram,
                ext_address,
                false,
                is_bg,
                is_sprite,
                chr_eva,
            );
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                raw_address
            } else {
                self.core.mirror_nametable_address(raw_address)
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() { prg_vram[idx] } else { 0 }
            } else if self.core.console_type_vt369 {
                self.vt369_vram[(mirrored & 0x7FF) as usize]
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
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
            let idx = (mirrored & 0x7FF) as usize;
            if self.core.console_type_vt369 {
                self.vt369_vram[idx] = data;
            }
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[idx] = data;
            }
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
        self.core
            .ppu_cycle(ppu_address_bus, scanline, dot, rendering_on)
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.core.cpu_cycle()
    }

    fn take_irq_ack(&mut self) -> bool {
        self.core.take_irq_ack()
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.extend_from_slice(&self.core.save_core());
        state.push(self.submapper);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..cart.prg_ram.len() {
            if p < state.len() {
                cart.prg_ram[i] = state[p];
            }
            p += 1;
        }
        for i in 0..cart.chr_ram.len() {
            if p < state.len() {
                cart.chr_ram[i] = state[p];
            }
            p += 1;
        }
        p = self.core.load_core(state, p);
        if p < state.len() {
            self.submapper = state[p];
            p += 1;
        }
        self.core.submapper = self.submapper;
        p
    }

    fn insert_coin(&mut self, _coin: u8) {}
    fn service_button(&mut self) {}
    fn get_dip_switches(&self) -> u8 {
        0
    }
    fn set_dip_switches(&mut self, _value: u8) {}
    fn vt03_4bpp_bg(&self) -> bool { (self.core.reg2000[0x10] & 0x02) != 0 }
    fn vt03_4bpp_sp(&self) -> bool { (self.core.reg2000[0x10] & 0x04) != 0 }
    fn vt03_reg2000_10(&self) -> u8 { self.core.reg2000[0x10] }
    fn unscramble_opcode(&self, opcode: u8) -> u8 {
        self.core.unscramble_opcode(opcode)
    }
    fn onebus_cpu_ram_4k(&self) -> bool {
        self.core.console_type_vt09 || self.core.console_type_vt369
    }
    fn onebus_vt03_ppu(&self) -> bool {
        self.core.console_type_vt03
    }
    fn onebus_vt369_ppu(&self) -> bool {
        self.core.console_type_vt369
    }
    fn onebus_chr_routing_ppu(&self) -> bool {
        self.core.console_type_vt03
            || self.core.console_type_vt09
            || (self.core.console_type_vt369 && self.core.reg2000[0x1E] == 0)
    }
    fn onebus_vt369_enhanced_ppu(&self) -> bool {
        self.core.console_type_vt369 && self.core.reg2000[0x1E] != 0
    }
    fn vt369_reg2000(&self, idx: usize) -> u8 {
        self.core.reg2000.get(idx).copied().unwrap_or(0)
    }
    fn vt369_relative(&self) -> usize {
        self.core.vt369_relative
    }
    fn vt369_bg_data(&self) -> usize {
        self.core.vt369_bg_data
    }
    fn vt369_spr_data(&self) -> usize {
        self.core.vt369_spr_data
    }
    fn onebus_dma_config(&self) -> (u8, u16, u16) {
        (
            self.core.dma_middle_addr,
            self.core.dma_length,
            self.core.dma_target,
        )
    }
    fn battery_save_data(&self, _cart: &Cartridge) -> Option<Vec<u8>> {
        None
    }
    fn load_battery_save(&mut self, _cart: &mut Cartridge, _data: &[u8]) {}
}
