use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper559 {
    prg: [u8; 2],
    chr: [u16; 8],
    nt: [u8; 4],
    nt_vram: [u8; 0x1000],
    cpu_c: u8,
    mirroring: u8,
    irq: u8,
    counter: u8,
    latch: u8,
    cycles: i32,
    prg_flip: u8,
    wram_enable: bool,
    irq_raise_count: u8,
    irq_ack_pending: bool,
}

impl Mapper559 {
    pub fn new() -> Self {
        Self {
            prg: [0, 1],
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            nt: [0xE0, 0xE0, 0xE1, 0xE1],
            nt_vram: [0u8; 0x1000],
            cpu_c: 0xFE,
            mirroring: 0,
            irq: 0,
            counter: 0,
            latch: 0,
            cycles: 0,
            prg_flip: 0,
            wram_enable: true,
            irq_raise_count: 0,
            irq_ack_pending: false,
        }
    }
}

impl Mapper for Mapper559 {
    fn reset(&mut self) {
        self.nt = [0xE0, 0xE0, 0xE1, 0xE1];
        self.cpu_c = 0xFE;
        self.prg = [0, 1];
        self.chr = [0, 1, 2, 3, 4, 5, 6, 7];
        self.mirroring = 0;
        self.irq = 0;
        self.counter = 0;
        self.latch = 0;
        self.cycles = 0;
        self.prg_flip = 0;
        self.wram_enable = true;
        self.irq_raise_count = 0;
        self.irq_ack_pending = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            if self.wram_enable {
                let len = cart.prg_ram.len();
                if len > 0 {
                    let offset = (address as usize & 0x1FFF) % len;
                    return FetchResult {
                        data: cart.prg_ram[offset],
                        driven: true,
                    };
                }
            }
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        if address >= 0x8000 {
            let bank = match (address >> 13) & 3 {
                0 => {
                    if self.prg_flip != 0 {
                        0x1E
                    } else {
                        self.prg[0] & 0x1F
                    }
                }
                1 => self.prg[1] & 0x1F,
                2 => self.cpu_c,
                _ => 0x1F,
            };
            let offset = bank as usize * 0x2000 + (address as usize & 0x1FFF);
            return FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if self.wram_enable {
                let len = cart.prg_ram.len();
                if len > 0 {
                    let offset = (address as usize & 0x1FFF) % len;
                    cart.prg_ram[offset] = data;
                }
            }
            return;
        }
        if address < 0x8000 {
            return;
        }
        match (address >> 12) & 0xF {
            0x8 => self.prg[0] = data,
            0x9 => {
                let addr = address & 0xFFF;
                let reg = ((((addr & 0x800) >> 11) << 1) | ((addr & 0x400) >> 10)) as usize;
                match reg {
                    0 | 1 => self.mirroring = data & 3,
                    2 => {
                        self.wram_enable = (data & 1) != 0;
                        self.prg_flip = if data & 2 != 0 { 4 } else { 0 };
                    }
                    3 => {
                        if addr & 4 != 0 {
                            self.nt[(addr & 3) as usize] = data;
                        } else {
                            self.cpu_c = data;
                        }
                    }
                    _ => {}
                }
            }
            0xA => self.prg[1] = data,
            0xB..=0xE => {
                let addr = address & 0xFFF;
                let reg = (((((address >> 12) & 0xF) - 0xB) << 1)
                    | if addr & 0x800 != 0 { 1 } else { 0 }) as usize;
                let high = addr & 0x400 != 0;
                let val = if high { data >> 4 } else { data };
                if high {
                    self.chr[reg] = (self.chr[reg] & 0x000F) | ((val as u16) << 4);
                } else {
                    self.chr[reg] = (self.chr[reg] & 0xFFF0) | (val as u16 & 0x000F);
                }
            }
            0xF => {
                let addr = address & 0xFFF;
                let reg = ((((addr & 0x800) >> 11) << 1) | ((addr & 0x400) >> 10)) as usize;
                let val = if addr & 0x400 != 0 { data >> 4 } else { data };
                match reg {
                    0 => self.latch = (self.latch & 0xF0) | (val & 0x0F),
                    1 => self.latch = (self.latch & 0x0F) | (val << 4),
                    2 => {
                        self.irq = val;
                        if self.irq & 2 != 0 {
                            self.counter = self.latch;
                            self.cycles = 341;
                        }
                        self.irq_ack_pending = true;
                    }
                    3 => {
                        self.irq = (self.irq & !2) | ((self.irq << 1) & 2);
                        self.irq_ack_pending = true;
                    }
                    _ => {}
                }
            }
            _ => {}
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
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        _vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        let byte;
        if address < 0x2000 {
            let bank = (self.chr[(address as usize >> 10) & 7] & 0x1FF) as usize;
            let offset = bank * 0x400 + (address as usize & 0x3FF);
            if !chr_rom.is_empty() {
                byte = chr_rom[offset % chr_rom.len()];
            } else if !chr_ram.is_empty() {
                byte = chr_ram[offset % chr_ram.len()];
            } else {
                byte = 0;
            }
        } else if address < 0x3F00 {
            let nt_idx = (address as usize >> 10) & 3;
            let row = (self.nt[nt_idx] & 3) as usize;
            let offset = row * 0x400 + (address as usize & 0x3FF);
            byte = self.nt_vram[offset];
        } else {
            return (ppu_address_bus as u8, new_addr_bus);
        }
        new_addr_bus |= byte as u16;
        (new_addr_bus as u8, new_addr_bus)
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.nametable_horizontal_mirroring {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, _vram: &mut [u8]) {
        if address < 0x2000 && cart.chr_rom.is_empty() && !cart.chr_ram.is_empty() {
            let bank = (self.chr[(address as usize >> 10) & 7] & 0x1FF) as usize;
            let offset = bank * 0x400 + (address as usize & 0x3FF);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let nt_idx = (address as usize >> 10) & 3;
            let row = (self.nt[nt_idx] & 3) as usize;
            let offset = row * 0x400 + (address as usize & 0x3FF);
            self.nt_vram[offset] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        let mut raised = false;
        if self.irq_raise_count > 0 {
            self.irq_raise_count -= 1;
            if self.irq_raise_count == 0 {
                raised = true;
            }
        }
        if self.irq & 2 != 0 {
            let count = if self.irq & 4 != 0 {
                true
            } else {
                self.cycles -= 3;
                self.cycles <= 0
            };
            if count {
                if self.irq & 4 == 0 {
                    self.cycles += 341;
                }
                self.counter = self.counter.wrapping_add(1);
                if self.counter == 0 {
                    self.counter = self.latch;
                    self.irq_raise_count = 1;
                    if self.irq_raise_count == 0 {
                        raised = true;
                    }
                }
            }
        }
        raised
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack_pending;
        self.irq_ack_pending = false;
        ack
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut s = Vec::new();
        s.extend_from_slice(&self.prg);
        for &c in &self.chr {
            s.extend_from_slice(&c.to_le_bytes());
        }
        s.push(self.mirroring);
        s.push(self.irq);
        s.push(self.counter);
        s.push(self.latch);
        s.extend_from_slice(&self.cycles.to_le_bytes());
        s.push(self.prg_flip);
        s.push(self.irq_raise_count);
        s.push(if self.wram_enable { 1 } else { 0 });
        s.push(self.cpu_c);
        s.extend_from_slice(&self.nt);
        s.extend_from_slice(&self.nt_vram);
        s.push(if self.irq_ack_pending { 1 } else { 0 });
        s
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for r in self.prg.iter_mut() {
            *r = state[p];
            p += 1;
        }
        for c in self.chr.iter_mut() {
            *c = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        self.mirroring = state[p];
        p += 1;
        self.irq = state[p];
        p += 1;
        self.counter = state[p];
        p += 1;
        self.latch = state[p];
        p += 1;
        self.cycles = i16::from_le_bytes([state[p], state[p + 1]]) as i32;
        p += 2;
        self.prg_flip = state[p];
        p += 1;
        self.irq_raise_count = state[p];
        p += 1;
        self.wram_enable = state[p] != 0;
        p += 1;
        self.cpu_c = state[p];
        p += 1;
        for n in self.nt.iter_mut() {
            *n = state[p];
            p += 1;
        }
        if p + 0x1000 <= state.len() {
            self.nt_vram.copy_from_slice(&state[p..p + 0x1000]);
            p += 0x1000;
        }
        if p < state.len() {
            self.irq_ack_pending = state[p] != 0;
            p += 1;
        }
        p - start
    }
}