use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};
use crate::mappers::mmc3::Mmc3Config;
use crate::mappers::one_bus::{OneBus, OneBusBanking, OneBusMangle};

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
}

impl Mapper256 {
    pub fn new(_config: Mmc3Config, submapper: u8) -> Self {
        Self {
            core: OneBus::new(&[], &[], OneBusBanking::MAPPER256),
            submapper,
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
        self.core.reset();
    }

    fn handle_cpu_write(&mut self, address: u16, data: u8) {
        let mangle = self.mangle();
        if (0x2000..0x2100).contains(&address) {
            self.core.write_ppu(address, data, &mangle);
        } else if (0x4100..0x4200).contains(&address) {
            self.core.write_apu(address, data, &mangle);
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            if address >= 0x6000 && !cart.prg_ram.is_empty() && self.prg_ram_writable() {
                cart.prg_ram[(address - 0x6000) as usize] = data;
            }
            return;
        }
        self.core.store_prg_mmc3(address, data, &self.mangle());
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x4100 && address < 0x4200 {
            if let Some(data) = self.core.read_apu(address) {
                return FetchResult { data, driven: true };
            }
        }
        if address >= 0x2010 && address < 0x2100 {
            let idx = (address & 0xFF) as usize;
            return FetchResult {
                data: self.core.reg2000[idx],
                driven: true,
            };
        }
        if address >= 0x6000 && address < 0x8000 {
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
            mirror_h_or_v(self.core.hv() != 0, address)
        }
    }

    fn fetch_ppu(
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
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x7F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = self.core.fetch_chr_byte(prg_rom, chr_rom, chr_ram, address, false);
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else {
                mirror_h_or_v(self.core.hv() != 0, address)
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() {
                    prg_vram[idx]
                } else {
                    0
                }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let slot = ((address >> 10) as usize & 7) ^ if self.core.comr7() { 4 } else { 0 };
                let bank = self.core.chr_bank_1k(slot);
                let offset = bank * 0x400 + (address as usize & 0x3FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
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
        false
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
        p
    }

    fn insert_coin(&mut self, _coin: u8) {}
    fn service_button(&mut self) {}
    fn get_dip_switches(&self) -> u8 {
        0
    }
    fn set_dip_switches(&mut self, _value: u8) {}
    fn vt03_4bpp_bg(&self) -> bool { (self.core.reg2000[0x10] & 0x82) != 0 }
    fn vt03_4bpp_sp(&self) -> bool { (self.core.reg2000[0x10] & 0x84) != 0 }
    fn battery_save_data(&self, _cart: &Cartridge) -> Option<Vec<u8>> {
        None
    }
    fn load_battery_save(&mut self, _cart: &mut Cartridge, _data: &[u8]) {}
}
