use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, mmc3_chr_bank};

pub struct Mapper367 {
    mmc3: MapperMMC3,
    reg: u8,
    dip_switches: u8,
}

impl Mapper367 {
    pub fn new(header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let config = Mmc3Config {
            prg_ram_size: 0,
            chr_ram_size: if chr_size == 0 { 0x2000 } else { 0 },
            mmc6: false,
            ax5202p: true,
            irq_revision_b: true,
            irq_hack: crate::mappers::mmc3::Mmc3IrqHack::None,
            header_horizontal_mirror: (header.get(6).copied().unwrap_or(0) & 1) == 0,
        };
        let dip = if header.len() > 7 { header[7] } else { 0 };
        Self { mmc3: MapperMMC3::new(config), reg: 0, dip_switches: dip }
    }
}

impl Mapper for Mapper367 {
    fn reset(&mut self) {
        self.reg = 0;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len().max(1);
            let mask_8k = len / 0x2000;
            let last = if mask_8k > 0 { mask_8k - 1 } else { 0 };
            let second_last = if last > 0 { last - 1 } else { 0 };
            let mode = (self.mmc3.r8000 & 0x40) != 0;
            let page = (address as usize - 0x8000) / 0x2000;
            let mmc3_bank = match (page, mode) {
                (0, false) => self.mmc3.bank_8c as usize,
                (0, true) => second_last,
                (1, _) => self.mmc3.bank_a as usize,
                (2, false) => second_last,
                (2, true) => self.mmc3.bank_8c as usize,
                (3, _) => last,
                _ => 0,
            };
            let prg_and = if (self.reg & 0x02) != 0 { 0x0F } else { 0x1F };
            let prg_or = ((self.reg as usize) << 4) & !prg_and;
            let bank = (mmc3_bank & prg_and) | prg_or;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult { data: cart.prg_rom[offset % len], driven: true };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, val: u8) {
        if address >= 0x6000 && address < 0x8000 {
            let mut r = (address & 0xFF) as u8;
            if self.dip_switches != 0 && (r & 1) != 0 {
                r |= 2;
            }
            self.reg = r;
        } else if address >= 0x8000 {
            self.mmc3.store_prg(cart, address, val);
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        let mmc3_h = self.mmc3.nametable_mirroring();
        if cart.alternative_nametable_arrangement { address } else { mirror_h_or_v(mmc3_h, address) }
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
            if using_chr_ram && !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[(address as usize) % chr_ram.len()] as u16;
            } else if !chr_rom.is_empty() {
                let chr_bank = mmc3_chr_bank(
                    self.mmc3.r8000, self.mmc3.chr_2k0, self.mmc3.chr_2k8,
                    self.mmc3.chr_1k0, self.mmc3.chr_1k4, self.mmc3.chr_1k8, self.mmc3.chr_1kc,
                    address,
                ) as usize;
                let chr_and = if (self.reg & 0x02) != 0 { 0x7F } else { 0xFF };
                let chr_or = ((self.reg as usize) << 7) & !chr_and;
                let bank = (chr_bank & chr_and) | chr_or;
                let offset = bank * 0x400 + (address as usize & 0x3FF);
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else {
            let mir = mirror_h_or_v(self.mmc3.nametable_mirroring(), address);
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram && !cart.chr_ram.is_empty() {
            let len = cart.chr_ram.len();
            cart.chr_ram[(address as usize) % len] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mir = self.mirror_nametable(cart, address);
            vram[(mir & 0x7FF) as usize] = data;
        }
    }

    fn ppu_clock(&mut self, ppu_address_bus: u16, ppu_a12_prev: bool, scanline: u16, dot: u16, ppu_sprite_x16: bool, rendering_on: bool) -> bool {
        self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.reg);
        state.push(self.dip_switches);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let p = self.mmc3.load_mapper_registers(cart, state, start);
        let mut p = p;
        if p < state.len() { self.reg = state[p]; p += 1; }
        if p < state.len() { self.dip_switches = state[p]; p += 1; }
        p
    }
}
