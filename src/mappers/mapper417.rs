use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper417 {
    prg: [u8; 4],
    chr: [u8; 8],
    nt: [u8; 4],
    enable_irq: bool,
    counter: i32,
    irq_ack_pending: bool,
    submapper: u8,
}

impl Mapper417 {
    pub fn new(submapper: u8) -> Self {
        Self {
            prg: [0; 4],
            chr: [0; 8],
            nt: [0; 4],
            enable_irq: false,
            counter: 0,
            irq_ack_pending: false,
            submapper,
        }
    }

    fn prg_fetch(&self, cart: &Cartridge, address: u16, bank: usize) -> FetchResult {
        let len = cart.prg_rom.len();
        if len == 0 {
            return FetchResult { data: 0, driven: false };
        }
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        FetchResult { data: cart.prg_rom[offset % len], driven: true }
    }

    fn nametable_addr(&self, address: u16) -> u16 {
        let addr = address & 0x2FFF;
        let page = (self.nt[((addr >> 10) & 1) as usize] as u16) & 3;
        0x2000 + page * 0x400 + (addr & 0x3FF)
    }
}

impl Mapper for Mapper417 {
    fn reset(&mut self) {
        self.prg = [0; 4];
        self.chr = [0; 8];
        self.nt = [0; 4];
        self.enable_irq = false;
        self.counter = 0;
        self.irq_ack_pending = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        match address {
            0x8000..=0x9FFF => self.prg_fetch(cart, address, self.prg[0] as usize),
            0xA000..=0xBFFF => self.prg_fetch(cart, address, self.prg[1] as usize),
            0xC000..=0xDFFF => self.prg_fetch(cart, address, self.prg[2] as usize),
            0xE000..=0xFFFF => self.prg_fetch(cart, address, 0xFF),
            _ => FetchResult { data: 0, driven: false },
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            match (address >> 4) & 7 {
                0 => self.prg[(address & 3) as usize] = data,
                1 => {
                    self.chr[(address & 3) as usize] = data;
                    if self.submapper == 1 {
                        self.nt[(address & 3) as usize] = data >> 7;
                    }
                }
                2 => self.chr[((address & 3) as usize) | 4] = data,
                3 => {
                    self.enable_irq = true;
                    self.counter = 0;
                    self.irq_ack_pending = true;
                }
                4 => {
                    self.enable_irq = false;
                    self.irq_ack_pending = true;
                }
                5 => {
                    if self.submapper == 0 {
                        self.nt[(address & 3) as usize] = data;
                    }
                }
                _ => {}
            }
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack_pending;
        self.irq_ack_pending = false;
        ack
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.nametable_addr(address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        _chr_ram: &[u8],
        prg_vram: &[u8],
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
            let bank = (address >> 10) as usize;
            let byte = if chr_rom.is_empty() {
                0
            } else {
                let offset = (self.chr[bank] as usize) * 0x400 + (address as usize & 0x3FF);
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = self.nametable_addr(address);
            let byte = if (mirrored & 0x0800) != 0 {
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
        if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.nametable_addr(address);
            if (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
            }
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.counter = self.counter.wrapping_add(1);
        let mask = if self.submapper == 1 { 0x1000 } else { 0x0400 };
        self.enable_irq && (self.counter & mask) != 0
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.prg);
        state.extend_from_slice(&self.chr);
        state.extend_from_slice(&self.nt);
        state.push(self.enable_irq as u8);
        state.extend_from_slice(&self.counter.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..4 {
            if p < state.len() {
                self.prg[i] = state[p];
                p += 1;
            }
        }
        for i in 0..8 {
            if p < state.len() {
                self.chr[i] = state[p];
                p += 1;
            }
        }
        for i in 0..4 {
            if p < state.len() {
                self.nt[i] = state[p];
                p += 1;
            }
        }
        if p < state.len() {
            self.enable_irq = state[p] != 0;
            p += 1;
        }
        if p + 4 <= state.len() {
            self.counter = i32::from_le_bytes([state[p], state[p + 1], state[p + 2], state[p + 3]]);
            p += 4;
        }
        p
    }
}
