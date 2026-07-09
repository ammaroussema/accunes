use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{mmc3_chr_bank, MapperMMC3, Mmc3Config};

pub struct Mapper292 {
    mmc3: MapperMMC3,
    index: u8,
    last_write: u8,
    ppu0000: [u8; 2],
    ppu0800: [u8; 2],
    ppu1000: u8,
}

impl Mapper292 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let config = Mmc3Config {
            ax5202p: true,
            ..Mmc3Config::for_ines(header, 0, if chr_size == 0 { 0 } else { chr_size }, rom, rom_name)
        };
        Self {
            mmc3: MapperMMC3::new(config),
            index: 0,
            last_write: 0,
            ppu0000: [0xFF; 2],
            ppu0800: [0xFF; 2],
            ppu1000: 0,
        }
    }
}

impl Mapper for Mapper292 {
    fn reset(&mut self) {
        self.index = 0;
        self.last_write = 0;
        self.ppu0000 = [0xFF; 2];
        self.ppu0800 = [0xFF; 2];
        self.ppu1000 = 0;
        self.mmc3.reset();
    }

    fn handle_cpu_write(&mut self, address: u16, data: u8) {
        if address < 0x1000 {
            self.last_write = data;
        } else if address == 0x4014 {
            self.ppu0000[0] = self.ppu0000[1];
            self.ppu0800[0] = self.ppu0800[1];
        }
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            let num_8k = len / 0x2000;
            let invert = (self.mmc3.r8000 & 0x40) != 0;
            let raw_bank = match address {
                0xE000..=0xFFFF => num_8k.saturating_sub(1),
                0xC000..=0xDFFF => {
                    if invert { self.mmc3.bank_8c as usize }
                    else { num_8k.saturating_sub(2) }
                }
                0xA000..=0xBFFF => self.mmc3.bank_a as usize,
                0x8000..=0x9FFF => {
                    if invert { num_8k.saturating_sub(2) }
                    else { self.mmc3.bank_8c as usize }
                }
                _ => 0,
            };
            let bank = raw_bank & 0x3F;
            let num_banks = if num_8k > 0 { num_8k } else { 1 };
            let bank = bank % num_banks;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: if offset < len { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0x5000 && address <= 0x7FFF {
            let bank0 = mmc3_chr_bank(
                self.mmc3.r8000, self.mmc3.chr_2k0, self.mmc3.chr_2k8,
                self.mmc3.chr_1k0, self.mmc3.chr_1k4, self.mmc3.chr_1k8, self.mmc3.chr_1kc,
                0x0000,
            );
            let bank2 = mmc3_chr_bank(
                self.mmc3.r8000, self.mmc3.chr_2k0, self.mmc3.chr_2k8,
                self.mmc3.chr_1k0, self.mmc3.chr_1k4, self.mmc3.chr_1k8, self.mmc3.chr_1kc,
                0x0800,
            );
            if (self.index & 0x20) != 0 {
                self.ppu0800[1] = ((self.last_write << 1) & 0x80) ^ ((bank2 as u8) >> 1);
                self.ppu1000 = self.last_write & 0x3F;
            } else {
                self.ppu0000[1] = self.last_write ^ ((bank0 as u8) >> 1);
            }
            FetchResult { data: 0, driven: false }
        } else {
            self.mmc3.fetch_prg(cart, address)
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address <= 0x7FFF {
            self.index = data;
        } else {
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
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = if address < 0x0800 {
                let bank = self.ppu0000[0] as usize;
                let offset = bank * 0x0800 + (address as usize & 0x07FF);
                if using_chr_ram && !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else if !chr_rom.is_empty() {
                    chr_rom[offset % chr_rom.len()]
                } else { 0 }
            } else if address < 0x1000 {
                let bank = self.ppu0800[0] as usize;
                let offset = bank * 0x0800 + (address as usize & 0x07FF);
                if using_chr_ram && !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else if !chr_rom.is_empty() {
                    chr_rom[offset % chr_rom.len()]
                } else { 0 }
            } else {
                let bank = self.ppu1000 as usize;
                let offset = bank * 0x1000 + (address as usize & 0x0FFF);
                if using_chr_ram && !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else if !chr_rom.is_empty() {
                    chr_rom[offset % chr_rom.len()]
                } else { 0 }
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else {
                let horz = self.mmc3.nametable_mirroring();
                if horz { (address & 0x33FF) | ((address & 0x0800) >> 1) } else { address & 0x37FF }
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let (bank, shift) = if address < 0x0800 { (self.ppu0000[0] as usize, 0x0800usize) }
                    else if address < 0x1000 { (self.ppu0800[0] as usize, 0x0800usize) }
                    else { (self.ppu1000 as usize, 0x1000usize) };
                let offset = bank * shift + (address as usize & (shift - 1));
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
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

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.index);
        state.push(self.last_write);
        state.push(self.ppu0000[0]);
        state.push(self.ppu0000[1]);
        state.push(self.ppu0800[0]);
        state.push(self.ppu0800[1]);
        state.push(self.ppu1000);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let p = self.mmc3.load_mapper_registers(cart, state, start);
        let mut p = p;
        if p < state.len() { self.index = state[p]; p += 1; }
        if p < state.len() { self.last_write = state[p]; p += 1; }
        if p < state.len() { self.ppu0000[0] = state[p]; p += 1; }
        if p < state.len() { self.ppu0000[1] = state[p]; p += 1; }
        if p < state.len() { self.ppu0800[0] = state[p]; p += 1; }
        if p < state.len() { self.ppu0800[1] = state[p]; p += 1; }
        if p < state.len() { self.ppu1000 = state[p]; p += 1; }
        p
    }
}
