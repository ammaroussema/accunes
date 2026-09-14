use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, mmc3_chr_bank};

pub struct Mapper353 {
    mmc3: MapperMMC3,
    reg: u8,
}

impl Mapper353 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let config = Mmc3Config::for_ines(header, 0, if header.len() > 5 { header[5] } else { 0 }, rom, rom_name);
        Self { mmc3: MapperMMC3::new(config), reg: 0 }
    }
}

impl Mapper for Mapper353 {
    fn reset(&mut self) {
        self.reg = 0;
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            if cart.prg_ram.len() > 0 {
                return FetchResult { data: cart.prg_ram[address as usize & 0x1FFF], driven: true };
            }
            return FetchResult { data: 0, driven: false };
        }
        if address >= 0x8000 {
            let page = (address as usize - 0x8000) / 0x2000;
            let mask_8k = cart.prg_rom.len() / 0x2000;
            if mask_8k == 0 { return FetchResult { data: 0, driven: false }; }
            let last = mask_8k - 1;
            let second_last = if last > 0 { last - 1 } else { 0 };
            let mode = (self.mmc3.r8000 & 0x40) != 0;
            let mut mmc3_bank = match (page, mode) {
                (0, false) => self.mmc3.bank_8c as usize,
                (0, true) => second_last,
                (1, _) => self.mmc3.bank_a as usize,
                (2, false) => second_last,
                (2, true) => self.mmc3.bank_8c as usize,
                (3, _) => last,
                _ => 0,
            };
            let override_c_e = self.reg == 3 && (self.mmc3.r8000 & 0x80) == 0;
            if override_c_e && (page == 2 || page == 3) {
                let base = if page == 2 { self.mmc3.bank_8c as usize } else { self.mmc3.bank_a as usize };
                mmc3_bank = base | 0x70;
            } else if self.reg == 2 {
                let outer = ((self.mmc3.r8000 as usize >> 3) & 0x10) | (self.reg as usize) << 5;
                mmc3_bank = (mmc3_bank & 0x0F) | outer;
            } else {
                let outer = (self.reg as usize) << 5;
                mmc3_bank = (mmc3_bank & 0x1F) | outer;
            }
            let offset = mmc3_bank * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len().max(1)], driven: true };
        }
        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, val: u8) {
        if address >= 0x8000 {
            if (address & 0x80) != 0 {
                self.reg = ((address >> 13) & 3) as u8;
            } else {
                self.mmc3.store_prg(cart, address, val);
            }
        } else {
            self.mmc3.store_prg(cart, address, val);
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if self.reg == 0 {
            if cart.alternative_nametable_arrangement {
                address
            } else {
                mirror_h_or_v(self.mmc3.nametable_mirroring(), address)
            }
        } else {
            self.mmc3.mirror_nametable(cart, address)
        }
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
            let using_chr_ram_actual = using_chr_ram || (self.reg == 2 && (self.mmc3.r8000 & 0x80) != 0);
            if using_chr_ram_actual && !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[(address as usize) % chr_ram.len()] as u16;
            } else if !chr_rom.is_empty() {
                let chr_bank = mmc3_chr_bank(
                    self.mmc3.r8000, self.mmc3.chr_2k0, self.mmc3.chr_2k8,
                    self.mmc3.chr_1k0, self.mmc3.chr_1k4, self.mmc3.chr_1k8, self.mmc3.chr_1kc,
                    address,
                ) as usize;
                let bank = (chr_bank & 0x7F) | ((self.reg as usize) << 7);
                let offset = bank * 0x400 + (address as usize & 0x3FF);
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else {
            let mmc3_mirror_h = self.mmc3.nametable_mirroring();
            if self.reg == 0 {
                new_addr_bus |= vram[(mirror_h_or_v(mmc3_mirror_h, address) & 0x7FF) as usize] as u16;
            } else {
                let mir = if mmc3_mirror_h { (address & 0x33FF) | ((address & 0x0800) >> 1) } else { address & 0x37FF };
                new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
            }
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        let using_chr_ram = cart.using_chr_ram || (self.reg == 2 && (self.mmc3.r8000 & 0x80) != 0);
        if address < 0x2000 && using_chr_ram && !cart.chr_ram.is_empty() {
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

    fn audio_sample(&self) -> f32 { 0.0 }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.reg);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let p = self.mmc3.load_mapper_registers(cart, state, start);
        if p < state.len() { self.reg = state[p]; p + 1 } else { p }
    }
}
