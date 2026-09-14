use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper555 {
    mmc3: MapperMMC3,
    reg: [u8; 2],
    counter: u32,
    counter_expired: bool,
    dip_switches: u8,
}

impl Mapper555 {
    pub fn new(
        header: &[u8],
        submapper_id: u8,
        chr_size: u8,
        rom: &[u8],
        rom_name: &str,
        has_battery: bool,
    ) -> Self {
        let mut config = Mmc3Config::for_ines(header, submapper_id, chr_size, rom, rom_name);
        config.irq_revision_b = true;
        config.prg_ram_size = config.prg_ram_size.max(0x4000);
        if has_battery {
            config.prg_ram_size = config.prg_ram_size.max(0x4000);
        }
        Self {
            mmc3: MapperMMC3::new(config),
            reg: [0; 2],
            counter: 0,
            counter_expired: false,
            dip_switches: 0,
        }
    }

    fn tqrom_mode(&self) -> bool {
        (self.reg[0] & 0x06) == 0x02
    }

    fn counter_reset(&self) -> bool {
        (self.reg[0] & 0x08) == 0
    }

    fn target(&self) -> u32 {
        (self.dip_switches as u32) << 25 | 0x20000000
    }

    fn prg_bank_for(&self, address: u16) -> usize {
        let a = (((self.reg[0] as usize) << 3) & 0x18) | 0x07;
        let o = ((self.reg[0] as usize) << 3) & 0x20;
        let invert = (self.mmc3.r8000 & 0x40) != 0;
        let fe = 0xFE & a;
        let raw_bank = match address & 0xE000 {
            0xE000 => 0xFF,
            0xC000 => {
                if invert { self.mmc3.bank_8c as usize } else { fe }
            }
            0xA000 => self.mmc3.bank_a as usize,
            _ => {
                if invert { fe } else { self.mmc3.bank_8c as usize }
            }
        };
        (raw_bank & a) | o
    }
}

impl Mapper for Mapper555 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.reg = [0; 2];
        self.counter = 0;
        self.counter_expired = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult { data: 0, driven: false };
            }
            let bank = self.prg_bank_for(address);
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            }
        } else if address >= 0x5000 && address < 0x6000 {
            if (address & 0x0800) != 0 {
                FetchResult {
                    data: (if self.counter_expired { 0x80 } else { 0x00 }) | 0x5C,
                    driven: true,
                }
            } else {
                let off = 0x2000 + (address as usize & 0x7FF);
                if off < cart.prg_ram.len() {
                    FetchResult { data: cart.prg_ram[off], driven: true }
                } else {
                    FetchResult { data: 0, driven: false }
                }
            }
        } else {
            self.mmc3.fetch_prg(cart, address)
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x6000 {
            if (address & 0x0800) != 0 {
                let idx = ((address >> 10) & 1) as usize;
                self.reg[idx] = data;
            } else {
                let off = 0x2000 + (address as usize & 0x7FF);
                if off < cart.prg_ram.len() {
                    cart.prg_ram[off] = data;
                }
            }
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
        prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let bank = self.mmc3.chr_bank(address);
            let or7 = ((self.reg[0] as usize) << 5) & 0x80;
            let byte = if self.tqrom_mode() {
                if (bank & 0x40) != 0 {
                    let b = ((bank & 7) as usize) | or7;
                    let offset = b * 0x0400 + (address as usize & 0x03FF);
                    if !chr_ram.is_empty() { chr_ram[offset % chr_ram.len()] } else { 0 }
                } else {
                    let b = (bank as usize) | or7;
                    let offset = b * 0x0400 + (address as usize & 0x03FF);
                    if !chr_rom.is_empty() { chr_rom[offset % chr_rom.len()] } else { 0 }
                }
            } else {
                let b = ((bank & 0x7F) as usize) | or7;
                let offset = b * 0x0400 + (address as usize & 0x03FF);
                if !chr_rom.is_empty() {
                    chr_rom[offset % chr_rom.len()]
                } else if !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else {
                    0
                }
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if self.mmc3.nametable_mirroring() {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() { prg_vram[idx] } else { 0 }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            let or7 = ((self.reg[0] as usize) << 5) & 0x80;
            if self.tqrom_mode() {
                let bank = self.mmc3.chr_bank(address);
                if (bank & 0x40) != 0 && !cart.chr_ram.is_empty() {
                    let b = ((bank & 7) as usize) | or7;
                    let offset = b * 0x0400 + (address as usize & 0x03FF);
                    let len = cart.chr_ram.len();
                    if len > 0 {
                        cart.chr_ram[offset % len] = data;
                    }
                }
            } else if cart.chr_rom.is_empty() {
                let bank = self.mmc3.chr_bank(address);
                let b = ((bank & 0x7F) as usize) | or7;
                let offset = b * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                if len > 0 {
                    cart.chr_ram[offset % len] = data;
                }
            }
        } else if address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
            }
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
        self.mmc3.ppu_clock(
            ppu_address_bus,
            ppu_a12_prev,
            scanline,
            dot,
            ppu_sprite_x16,
            rendering_on,
        )
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.mmc3.cpu_clock(_cycles);
        if self.counter_reset() {
            self.counter = 0;
            self.counter_expired = false;
        } else {
            self.counter = self.counter.wrapping_add(1);
            if self.counter == self.target() {
                self.counter_expired = true;
            }
        }
        false
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
    }

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.reg[0]);
        state.push(self.reg[1]);
        state.extend_from_slice(&self.counter.to_le_bytes());
        state.push(if self.counter_expired { 1 } else { 0 });
        state.push(self.dip_switches);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        if p < state.len() { self.reg[0] = state[p]; p += 1; }
        if p < state.len() { self.reg[1] = state[p]; p += 1; }
        if p + 4 <= state.len() {
            self.counter = u32::from_le_bytes([state[p], state[p+1], state[p+2], state[p+3]]);
            p += 4;
        }
        if p < state.len() { self.counter_expired = state[p] != 0; p += 1; }
        if p < state.len() { self.dip_switches = state[p]; p += 1; }
        p
    }
}
