use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper303 {
    latch: u8,
    reg: u8,
    mirroring: u8,
    irq_counter: u16,
    irq_enabled: bool,
    irq_active: bool,
}

impl Mapper303 {
    pub fn new() -> Self {
        Self {
            latch: 0,
            reg: 0,
            mirroring: 0,
            irq_counter: 0,
            irq_enabled: false,
            irq_active: false,
        }
    }
}

impl Mapper for Mapper303 {
    fn reset(&mut self) {
        self.latch = 0;
        self.reg = 0;
        self.mirroring = 0;
        self.irq_counter = 0;
        self.irq_enabled = false;
        self.irq_active = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 && address < 0xC000 {
            let bank = self.reg as usize;
            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0xC000 {
            let offset = 0x02 * 0x4000 + (address as usize & 0x3FFF);
            FetchResult {
                data: if offset < cart.prg_rom.len() { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else if address >= 0x6000 {
            let idx = (address - 0x6000) as usize;
            if idx < cart.prg_ram.len() {
                FetchResult { data: cart.prg_ram[idx], driven: true }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else if address >= 0x4020 {
            let lo = (address & 0xFF) as u8;
            if (0x4020..=0x403F).contains(&address) {
                if lo == 0x30 {
                    let result = if self.irq_active { 1 } else { 0 };
                    self.irq_active = false;
                    FetchResult { data: result, driven: true }
                } else {
                    FetchResult { data: 0x40, driven: true }
                }
            } else {
                FetchResult { data: 0x40, driven: true }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (address & 0xFF00) == 0x4A00 {
            self.latch = (((address >> 2) & 3) | ((address >> 4) & 4)) as u8;
            return;
        }
        let bank = (address >> 12) as u8;
        let addr_lo = (address & 0xFF) as u8;
        if bank == 4 && address < 0x5000 {
            if addr_lo < 0x20 {
                return;
            }
            match addr_lo {
                0x20 => {
                    self.irq_counter = (self.irq_counter & 0xFF00) | data as u16;
                }
                0x21 => {
                    self.irq_counter = (self.irq_counter & 0x00FF) | ((data as u16) << 8);
                    self.irq_enabled = true;
                }
                0x25 => {
                    self.mirroring = data & 8;
                }
                _ => {}
            }
            return;
        }
        if bank == 5 && (address & 0xF00) == 0x100 {
            self.reg = self.latch;
            return;
        }
        if address >= 0x6000 && address < 0x8000 {
            let idx = (address - 0x6000) as usize;
            if idx < cart.prg_ram.len() {
                cart.prg_ram[idx] = data;
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            return address;
        }
        if (self.mirroring & 8) != 0 {
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
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            if !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[address as usize & 0x1FFF] as u16;
            } else if !chr_rom.is_empty() {
                new_addr_bus |= chr_rom[address as usize & 0x1FFF] as u16;
            }
        } else {
            let h = if alternative_nametable_arrangement {
                false
            } else {
                (self.mirroring & 8) != 0
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
        if self.irq_enabled && self.irq_counter > 0 {
            self.irq_counter -= 1;
            if self.irq_counter == 0 {
                self.irq_enabled = false;
                self.irq_active = true;
                return true;
            }
        }
        false
    }

    fn take_irq_ack(&mut self) -> bool {
        let irq = self.irq_active;
        if irq {
            self.irq_active = false;
        }
        irq
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.latch);
        state.push(self.reg);
        state.push(self.mirroring);
        state.extend_from_slice(&self.irq_counter.to_le_bytes());
        state.push(if self.irq_enabled { 1 } else { 0 });
        state.push(if self.irq_active { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        self.latch = state.get(p).copied().unwrap_or(0); p += 1;
        self.reg = state.get(p).copied().unwrap_or(0); p += 1;
        self.mirroring = state.get(p).copied().unwrap_or(0); p += 1;
        if p + 2 <= state.len() {
            self.irq_counter = u16::from_le_bytes(state[p..p+2].try_into().unwrap());
            p += 2;
        }
        self.irq_enabled = state.get(p).copied().unwrap_or(0) != 0; p += 1;
        self.irq_active = state.get(p).copied().unwrap_or(0) != 0; p += 1;
        p
    }
}
