use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};
use crate::mappers::one_bus::{OneBus, OneBusBanking, OneBusMangle};

pub struct Mapper408 {
    core: OneBus,
}

impl Default for Mapper408 {
    fn default() -> Self {
        Self::new()
    }
}

impl Mapper408 {
    pub fn new() -> Self {
        Self {
            core: OneBus::new(&[], &[], OneBusBanking::MAPPER256),
        }
    }
}

impl Mapper for Mapper408 {
    fn reset(&mut self) {
        self.core.reset();
    }

    fn handle_cpu_write(&mut self, address: u16, data: u8) {
        let mangle = OneBusMangle::IDENTITY;
        if (0x2000..0x2100).contains(&address) {
            self.core.write_ppu(address, data, &mangle);
        } else if (0x4100..0x4200).contains(&address) {
            self.core.write_apu(address, data, &mangle);
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.core.store_prg_mmc3(address, data, &OneBusMangle::IDENTITY);
        }
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
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
        let is_chr_ram = self.core.reg4100[0x00] == 0x6A;
        if address < 0x2000 {
            let byte = self.core.fetch_chr_byte(prg_rom, chr_rom, chr_ram, address, is_chr_ram);
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mirror_h_or_v(self.core.hv() != 0, address);
            let byte = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            let is_chr_ram = self.core.reg4100[0x00] == 0x6A;
            if is_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
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
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.core.save_core());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        self.core.load_core(state, start)
    }

    fn vt03_4bpp_bg(&self) -> bool { (self.core.reg2000[0x10] & 0x02) != 0 }
    fn vt03_4bpp_sp(&self) -> bool { (self.core.reg2000[0x10] & 0x04) != 0 }
}
