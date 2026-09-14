use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper219 {
    mmc3: MapperMMC3,
    ex_regs: [u8; 3],
    prg_pages: [u8; 4],
    chr_pages: [u8; 8],
    prg_len: usize,
    chr_len: usize,
}

impl Mapper219 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let using_chr_ram = chr_size == 0;
        let config = Mmc3Config::for_ines(header, 0, if using_chr_ram { 0 } else { chr_size }, rom, rom_name);
        let prg_len = rom.len() / 0x4000 * 0x4000;
        let chr_len = if using_chr_ram { 0 } else { chr_size as usize * 0x2000 };
        Self {
            mmc3: MapperMMC3::new(config),
            ex_regs: [0; 3],
            prg_pages: [0; 4],
            chr_pages: [0; 8],
            prg_len,
            chr_len,
        }
    }

    fn set_prg_4x(&mut self) {
        let num_8k = (self.prg_len / 0x2000).max(4);
        for i in 0..4 {
            self.prg_pages[i] = (num_8k - 4 + i) as u8;
        }
    }

    fn select_prg_page(&mut self, slot: usize, bank: u8) {
        let num_8k = (self.prg_len / 0x2000).max(1);
        self.prg_pages[slot & 3] = (bank as usize % num_8k) as u8;
    }

    fn select_chr_page(&mut self, slot: usize, bank: u8) {
        let num_1k = (self.chr_len / 0x0400).max(1);
        self.chr_pages[slot & 7] = (bank as usize % num_1k) as u8;
    }

    fn init_pages(&mut self, cart: &Cartridge) {
        let prg_len = cart.prg_rom.len();
        if self.prg_len != prg_len {
            self.prg_len = prg_len;
        }
        let chr_len = if cart.using_chr_ram { cart.chr_ram.len() } else { cart.chr_rom.len() };
        if self.chr_len != chr_len {
            self.chr_len = chr_len;
        }
        self.set_prg_4x();
        for i in 0..8 {
            self.chr_pages[i] = 0;
        }
        self.ex_regs = [0; 3];
    }
}

impl Mapper for Mapper219 {
    fn reset(&mut self) {
        self.prg_len = 0;
        self.ex_regs = [0; 3];
        self.mmc3.reset();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if self.prg_len == 0 {
            self.init_pages(cart);
        }
        if address < 0x8000 {
            return self.mmc3.fetch_prg(cart, address);
        }
        if self.prg_len == 0 {
            return FetchResult { data: 0, driven: false };
        }
        let slot = ((address as usize - 0x8000) / 0x2000) & 3;
        let bank = self.prg_pages[slot] as usize;
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        FetchResult { data: cart.prg_rom[offset % self.prg_len], driven: true }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if self.prg_len == 0 {
            self.init_pages(cart);
        }
        if address >= 0x8000 && address < 0xA000 {
            match address & 0xE003 {
                0x8000 => {
                    self.ex_regs[0] = 0;
                    self.ex_regs[1] = data;
                }
                0x8001 => {
                    if self.ex_regs[0] >= 0x23 && self.ex_regs[0] <= 0x26 {
                        let prg_bank = ((data & 0x20) >> 5)
                            | ((data & 0x10) >> 3)
                            | ((data & 0x08) >> 1)
                            | ((data & 0x04) << 1);
                        self.select_prg_page(0x26 - self.ex_regs[0] as usize, prg_bank);
                    }
                    match self.ex_regs[1] {
                        0x08 | 0x0A | 0x0E | 0x12 | 0x16 | 0x1A | 0x1E => {
                            self.ex_regs[2] = data << 4;
                        }
                        0x09 => {
                            self.select_chr_page(0, self.ex_regs[2] | ((data >> 1) & 0x0E));
                        }
                        0x0B => {
                            self.select_chr_page(1, self.ex_regs[2] | ((data >> 1) | 0x01));
                        }
                        0x0C | 0x0D => {
                            self.select_chr_page(2, self.ex_regs[2] | ((data >> 1) & 0x0E));
                        }
                        0x0F => {
                            self.select_chr_page(3, self.ex_regs[2] | ((data >> 1) | 0x01));
                        }
                        0x10 | 0x11 => {
                            self.select_chr_page(4, self.ex_regs[2] | ((data >> 1) & 0x0F));
                        }
                        0x14 | 0x15 => {
                            self.select_chr_page(5, self.ex_regs[2] | ((data >> 1) & 0x0F));
                        }
                        0x18 | 0x19 => {
                            self.select_chr_page(6, self.ex_regs[2] | ((data >> 1) & 0x0F));
                        }
                        0x1C | 0x1D => {
                            self.select_chr_page(7, self.ex_regs[2] | ((data >> 1) & 0x0F));
                        }
                        _ => {}
                    }
                }
                0x8002 => {
                    self.ex_regs[0] = data;
                    self.ex_regs[1] = 0;
                }
                _ => {}
            }
            return;
        }
        self.mmc3.store_prg(cart, address, data);
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else if self.mmc3.nametable_mirroring() {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
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
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let slot = (address as usize) / 0x0400;
            let bank = self.chr_pages.get(slot).copied().unwrap_or(0) as usize;
            let chr_len = if using_chr_ram { chr_ram.len() } else { chr_rom.len() };
            let offset = bank * 0x0400 + (address as usize & 0x03FF);
            let byte = if chr_len > 0 && using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_len]
            } else if chr_len > 0 && !chr_rom.is_empty() {
                chr_rom[offset % chr_len]
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
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let slot = (address as usize) / 0x0400;
            let bank = self.chr_pages.get(slot).copied().unwrap_or(0) as usize;
            let offset = bank * 0x0400 + (address as usize & 0x03FF);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
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

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.extend_from_slice(&self.ex_regs);
        state.extend_from_slice(&self.prg_pages);
        state.extend_from_slice(&self.chr_pages);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let p = self.mmc3.load_mapper_registers(cart, state, start);
        let mut pos = p;
        for i in 0..3 {
            if pos < state.len() { self.ex_regs[i] = state[pos]; pos += 1; }
        }
        for i in 0..4 {
            if pos < state.len() { self.prg_pages[i] = state[pos]; pos += 1; }
        }
        for i in 0..8 {
            if pos < state.len() { self.chr_pages[i] = state[pos]; pos += 1; }
        }
        pos
    }
}
