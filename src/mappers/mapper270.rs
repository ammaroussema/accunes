use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::one_bus::{OneBus, OneBusBanking, OneBusMangle};

pub struct Mapper270 {
    core: OneBus,
    reg4242: u8,
    submapper: u8,
    dip_value: u8,
}

impl Mapper270 {
    pub fn new(submapper_id: u8) -> Self {
        Self {
            core: OneBus::new(&[], &[], OneBusBanking::mapper270(submapper_id, 0)),
            reg4242: 0,
            submapper: submapper_id,
            dip_value: 0,
        }
    }

    fn refresh_banking(&mut self) {
        self.core.banking = OneBusBanking::mapper270(self.submapper, self.core.reg4100[0x2C]);
    }

    fn chr_ram_flat(&self) -> bool {
        (self.reg4242 & 1) != 0
    }
}

impl Mapper for Mapper270 {
    fn reset(&mut self) {
        // Furbtendulator resets reg4242 and reg4100[0x2C] on every reset (hard + soft).
        self.reg4242 = 0;
        self.core.reset();
        self.core.reg4100[0x2C] = 0;
        self.refresh_banking();
    }

    fn reset_power_cycle(&mut self) {
        self.reg4242 = 0;
        self.core.reset();
        self.core.reg4100[0x2C] = 0;
        self.refresh_banking();
    }

    fn handle_cpu_write(&mut self, address: u16, data: u8) {
        let mangle = OneBusMangle::IDENTITY;
        if (0x2000..0x2100).contains(&address) {
            self.core.write_ppu(address, data, &mangle);
        } else if (0x4100..0x4200).contains(&address) {
            if address & 0xFF == 0x42 {
                self.reg4242 = data;
                self.refresh_banking();
            } else {
                self.core.write_apu(address, data, &mangle);
                if address & 0xFF == 0x2C {
                    self.refresh_banking();
                }
            }
        }
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address & 0xFF == 0x2C && address >= 0x4100 && address < 0x4200 {
            return FetchResult {
                data: self.dip_value,
                driven: true,
            };
        }
        if let Some(data) = self.core.read_apu(address) {
            return FetchResult { data, driven: true };
        }
        if address >= 0x6000 && address < 0x8000 {
            let idx = (address - 0x6000) as usize;
            if idx < cart.prg_ram.len() {
                return FetchResult {
                    data: cart.prg_ram[idx],
                    driven: true,
                };
            }
            return FetchResult { data: 0, driven: false };
        }
        if address >= 0x8000 {
            return FetchResult {
                data: self.core.fetch_prg_byte(&cart.prg_rom, address),
                driven: true,
            };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            let idx = (address - 0x6000) as usize;
            if idx < cart.prg_ram.len() {
                cart.prg_ram[idx] = data;
            }
            return;
        }
        if address >= 0x8000 {
            self.core.store_prg_mmc3(address, data, &OneBusMangle::IDENTITY);
            return;
        }
        if (address & 0xF000) == 0x5000 || (address & 0xFF00) == 0x4100 {
            let idx = (address & 0xFF) as usize;
            if idx == 0x2C {
                self.core.reg4100[0x2C] = data;
                self.refresh_banking();
            } else if idx == 0x42 {
                self.reg4242 = data;
            } else if idx < 0x100 {
                self.core.reg4100[idx] = data;
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.core.mirror_nametable_address(address)
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
        let raw_address = (ppu_address_bus & 0x7FFF) | (ppu_octal_latch as u16);
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        let is_chr_fetch = raw_address < 0x2000 || (raw_address >= 0x4000 && raw_address < 0x6000);
        if is_chr_fetch {
            let high_plane = raw_address >= 0x4000 && raw_address < 0x6000;
            let chr_addr = raw_address & 0x1FFF;
            let ext_address = if high_plane { 0x4000 | chr_addr } else { chr_addr };
            let byte = self.core.fetch_chr_byte_ext(
                prg_rom,
                chr_rom,
                chr_ram,
                ext_address,
                self.chr_ram_flat(),
                false,
                false,
            );
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = self.core.mirror_nametable_address(raw_address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 || (address >= 0x4000 && address < 0x6000) {
            if self.chr_ram_flat() && !cart.chr_ram.is_empty() {
                cart.chr_ram[address as usize & 0x1FFF] = data;
            } else if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let slot = (address >> 10) as usize & 7;
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
        self.core
            .ppu_cycle(ppu_address_bus, scanline, dot, rendering_on)
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.core.cpu_cycle()
    }

    fn take_irq_ack(&mut self) -> bool {
        self.core.take_irq_ack()
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_value
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_value = value;
    }

    fn vt03_4bpp_bg(&self) -> bool { (self.core.reg2000[0x10] & 0x02) != 0 }
    fn vt03_4bpp_sp(&self) -> bool { (self.core.reg2000[0x10] & 0x04) != 0 }
    fn vt03_reg2000_10(&self) -> u8 { self.core.reg2000[0x10] }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = self.core.save_core();
        state.push(self.reg4242);
        state.push(self.submapper);
        state.push(self.dip_value);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.core.load_core(state, start);
        if p < state.len() {
            self.reg4242 = state[p];
            p += 1;
        }
        if p < state.len() {
            self.submapper = state[p];
            p += 1;
        }
        if p < state.len() {
            self.dip_value = state[p];
            p += 1;
        }
        self.refresh_banking();
        let _ = cart;
        p
    }
}
