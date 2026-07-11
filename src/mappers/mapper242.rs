use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper242 {
    reg: u16,
    dip_switches: u8,
}

impl Mapper242 {
    pub fn new() -> Self {
        Self { reg: 0, dip_switches: 0 }
    }

    fn dip_enabled(&self) -> bool {
        (self.reg & 0x100) != 0 && self.dip_switches != 0
    }
}

impl Mapper for Mapper242 {
    fn reset(&mut self) {
        self.reg = 0;
        self.dip_switches = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            let offset = (address as usize - 0x6000) % cart.prg_ram.len().max(1);
            return FetchResult { data: cart.prg_ram.get(offset).copied().unwrap_or(0), driven: true };
        }
        if address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }
        let eff = if self.dip_enabled() {
            self.reg | (self.dip_switches as u16)
        } else {
            self.reg
        };
        let prg = ((eff >> 2) & 0x1F) as usize;
        let cpu_a14 = (eff & 1) as usize;
        let nrom = (eff & 0x80) != 0;
        let last_bit = (eff & 0x200) != 0;
        let len = cart.prg_rom.len();
        if len == 0 {
            return FetchResult { data: 0, driven: false };
        }
        let addr = address as usize;
        if nrom {
            let bank = prg & !1;
            let offset = bank * 0x8000 + (addr & 0x7FFF);
            FetchResult { data: cart.prg_rom[offset % len], driven: true }
        } else {
            let bank16 = if addr < 0xC000 {
                prg & !cpu_a14
            } else {
                let base = prg | cpu_a14;
                (base & !7) | (if last_bit { 7 } else { 0 })
            };
            let offset = bank16 * 0x4000 + (addr & 0x3FFF);
            FetchResult { data: cart.prg_rom[offset % len], driven: true }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            let offset = (address as usize - 0x6000) % cart.prg_ram.len().max(1);
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
        } else if address >= 0x8000 {
            self.reg = address & 0x7FFF;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if (self.reg & 2) != 0 {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
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
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let len = chr_ram.len();
            if len == 0 {
                return (0, new_addr_bus);
            }
            new_addr_bus |= chr_ram[(address as usize & 0x1FFF) % len] as u16;
        } else {
            let mirrored = if (self.reg & 2) != 0 {
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
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::with_capacity(3);
        state.extend_from_slice(&self.reg.to_le_bytes());
        state.push(self.dip_switches);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 2 <= state.len() {
            self.reg = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        self.dip_switches = state.get(p).copied().unwrap_or(0); p += 1;
        p
    }
}
