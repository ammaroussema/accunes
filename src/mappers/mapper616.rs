use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};
use crate::mappers::mmc3::{mmc3_chr_bank, MapperMMC3, Mmc3Config};

pub struct Mapper616 {
    mmc3: MapperMMC3,
    reg: u8,
}

impl Mapper616 {
    pub fn new() -> Self {
        let mut config = Mmc3Config::embedded();
        config.ax5202p = true;
        Self {
            mmc3: MapperMMC3::new(config),
            reg: 0,
        }
    }

    fn mmc3_raw_prg_bank(&self, slot: usize) -> usize {
        let swap = (self.mmc3.r8000 & 0x40) != 0;
        match slot {
            0 => if swap { 0x0E } else { self.mmc3.bank_8c as usize },
            1 => self.mmc3.bank_a as usize,
            2 => if swap { self.mmc3.bank_8c as usize } else { 0x0E },
            _ => 0x0F,
        }
    }

    fn prg_page(&self, address: u16) -> usize {
        let slot = (address as usize - 0x8000) / 0x2000;
        let raw = self.mmc3_raw_prg_bank(slot);
        (raw & 0x0F) | ((self.reg as usize) << 4)
    }

    fn chr_page(&self, address: u16) -> usize {
        let raw = mmc3_chr_bank(
            self.mmc3.r8000,
            self.mmc3.chr_2k0,
            self.mmc3.chr_2k8,
            self.mmc3.chr_1k0,
            self.mmc3.chr_1k4,
            self.mmc3.chr_1k8,
            self.mmc3.chr_1kc,
            address,
        ) as usize;
        (raw & 0x7F) | ((self.reg as usize) << 7)
    }
}

impl Mapper for Mapper616 {
    fn reset(&mut self) {
        self.reg = 0;
        self.mmc3.reset();
    }

    fn reset_power_cycle(&mut self) {
        self.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if cart.prg_rom.is_empty() {
                return FetchResult { data: 0, driven: false };
            }
            let page = self.prg_page(address);
            let offset = page * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if (0x6000..0x8000).contains(&address) {
            if !cart.prg_ram.is_empty() {
                let offset = (address - 0x6000) as usize;
                FetchResult {
                    data: cart.prg_ram[offset % cart.prg_ram.len()],
                    driven: true,
                }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) {
            self.reg = (address & 0xFF) as u8;
            if !cart.prg_ram.is_empty() {
                let offset = (address - 0x6000) as usize;
                let len = cart.prg_ram.len();
                cart.prg_ram[offset % len] = data;
            }
        } else if address >= 0x8000 {
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
            let byte = if (self.reg & 0x04) != 0 {
                let offset = address as usize & 0x1FFF;
                if !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else {
                    0
                }
            } else {
                let bank = self.chr_page(address);
                let offset = bank * 0x0400 + (address as usize & 0x03FF);
                if using_chr_ram || chr_rom.is_empty() {
                    if !chr_ram.is_empty() {
                        chr_ram[offset % chr_ram.len()]
                    } else {
                        0
                    }
                } else {
                    chr_rom[offset % chr_rom.len()]
                }
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mirror_h_or_v(self.mmc3.nametable_mirroring(), address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }

        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if (self.reg & 0x04) != 0 {
                if !cart.chr_ram.is_empty() {
                    let offset = address as usize & 0x1FFF;
                    let len = cart.chr_ram.len();
                    cart.chr_ram[offset % len] = data;
                }
            } else if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let bank = self.chr_page(address);
                let offset = bank * 0x0400 + (address as usize & 0x03FF);
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
        let mut state = vec![self.reg];
        state.extend(self.mmc3.save_mapper_registers(cart));
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start < state.len() {
            self.reg = state[start];
            start += 1;
        }
        self.mmc3.load_mapper_registers(cart, state, start)
    }
}
