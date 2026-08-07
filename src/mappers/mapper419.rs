use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};
use crate::mappers::adpcm3bit::Adpcm3Bit;
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

pub struct Mapper419 {
    core: OneBus,
    submapper: u8,
    adpcm: Adpcm3Bit,
}

impl Mapper419 {
    pub fn new(submapper: u8) -> Self {
        Self {
            core: OneBus::new(&[], &[], OneBusBanking::MAPPER256),
            submapper,
            adpcm: Adpcm3Bit::new(4_090_090, 1_789_772),
        }
    }

    fn mangle(&self) -> OneBusMangle {
        let s = self.submapper as usize;
        OneBusMangle {
            ppu: PPU_MANGLE[s],
            cpu: [0, 1, 2, 3],
            mmc3: MMC3_MANGLE[s],
        }
    }
}

impl Mapper for Mapper419 {
    fn reset(&mut self) {
        self.core.reset();
        self.adpcm.reset();
    }

    fn handle_cpu_write(&mut self, address: u16, data: u8) {
        let mangle = self.mangle();
        if (0x2000..0x2100).contains(&address) {
            self.core.write_ppu(address, data, &mangle);
        } else if address == 0x4016 {
            self.adpcm.set_clock((data & 0x04) != 0);
            self.core.write_apu(address, data, &mangle);
        } else if address == 0x410F {
            self.adpcm.set_data(data & 0x0F);
        } else if (0x4100..0x4200).contains(&address) {
            self.core.write_apu(address, data, &mangle);
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let mangle = self.mangle();
            if address <= 0x9FFF && (address & 1) == 0 {
                let mangled_val = data & 0xF8 | mangle.mmc3[(data & 0x07) as usize];
                self.core.write_mmc3(address, mangled_val, &mangle);
            } else {
                self.core.write_mmc3(address, data, &mangle);
            }
        }
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address == 0x4017 {
            let mut res = self.core.read_apu(address).unwrap_or(0);
            res = (res & !0x18)
                | (if self.adpcm.get_ready() { 0x10 } else { 0 })
                | (if self.adpcm.get_ack() { 0x08 } else { 0 });
            return FetchResult { data: res, driven: true };
        }
        if address == 0x410F {
            let mut res = self.core.read_apu(address).unwrap_or(0);
            res = (res & !0x30)
                | (if !self.adpcm.get_ready() { 0x20 } else { 0 })
                | (if !self.adpcm.get_ack() { 0x10 } else { 0 });
            return FetchResult { data: res, driven: true };
        }
        if address >= 0x4100 && address < 0x4200 {
            if let Some(data) = self.core.read_apu(address) {
                return FetchResult { data, driven: true };
            }
        }
        if address >= 0x8000 {
            let data = self.core.fetch_prg_byte(&cart.prg_rom, address);
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
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
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
            let mirrored = mirror_h_or_v(self.core.hv() != 0, address);
            let byte = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if (0x2000..0x3F00).contains(&address) {
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
        self.adpcm.run();
        self.core.cpu_cycle()
    }

    fn take_irq_ack(&mut self) -> bool {
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.core.save_core());
        state.push(self.submapper);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.core.load_core(state, start);
        if p < state.len() {
            self.submapper = state[p];
            p += 1;
        }
        p
    }

    fn vt03_4bpp_bg(&self) -> bool { (self.core.reg2000[0x10] & 0x02) != 0 }
    fn vt03_4bpp_sp(&self) -> bool { (self.core.reg2000[0x10] & 0x04) != 0 }
}
