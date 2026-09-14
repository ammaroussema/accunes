use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper304 {
    prg: u8,
    irq: bool,
}

impl Mapper304 {
    pub fn new() -> Self {
        Self { prg: 0, irq: false }
    }
}

impl Mapper for Mapper304 {
    fn reset(&mut self) {
        self.prg = 0;
        self.irq = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let offset = address as usize & 0x7FFF;
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0x6000 {
            let bank = (self.prg as usize) | 4;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0x4020 {
            let lo = address & 0xFF;
            if lo <= 0x20 {
                FetchResult { data: 0, driven: false }
            } else {
                FetchResult { data: 0xFF, driven: true }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        let lo = address & 0xFF;
        if lo == 0x27 {
            self.prg = data & 1;
        } else if lo == 0x68 {
            self.irq = (data & 1) == 0;
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else if cart.nametable_horizontal_mirroring {
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
        _using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[(address & 0x1FFF) as usize] as u16;
            } else if !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[(address & 0x1FFF) as usize] as u16;
            }
        } else {
            let h = if alternative_nametable_arrangement {
                false
            } else {
                nametable_horizontal_mirroring
            };
            let mirrored = if h {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        } else if address < 0x2000 && cart.using_chr_ram {
            let offset = address as usize & 0x1FFF;
            if offset < cart.chr_ram.len() {
                cart.chr_ram[offset] = data;
            }
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.irq
    }

    fn take_irq_ack(&mut self) -> bool {
        let irq = self.irq;
        self.irq = false;
        irq
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.prg);
        state.push(if self.irq { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.prg = state.get(p).copied().unwrap_or(0); p += 1;
        self.irq = state.get(p).copied().unwrap_or(0) != 0; p += 1;
        p
    }
}
