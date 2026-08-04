use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper, mirror_h_or_v};

pub struct Mapper416 {
    latch_data: u8,
    prg_smb2j: u8,
    irq: u8,
    counter: u16,
    irq_ack_pending: bool,
}

impl Mapper416 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self {
            latch_data: 0,
            prg_smb2j: 0,
            irq: 0,
            counter: 0,
            irq_ack_pending: false,
        }
    }

    fn prg_value(&self) -> usize {
        ((self.latch_data & 0x20) >> 5) as usize
            | ((self.latch_data & 0x80) >> 6) as usize
            | ((self.latch_data & 0x08) >> 1) as usize
    }

    fn prg_fetch(&self, cart: &Cartridge, address: u16, bank: usize, size: usize) -> FetchResult {
        let len = cart.prg_rom.len();
        if len == 0 {
            return FetchResult { data: 0, driven: false };
        }
        let offset = bank * size + (address as usize & (size - 1));
        FetchResult { data: cart.prg_rom[offset % len], driven: true }
    }
}

impl Mapper for Mapper416 {
    fn reset(&mut self) {
        self.latch_data = 0;
        self.prg_smb2j = 0;
        self.irq = 0;
        self.counter = 0;
        self.irq_ack_pending = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        match address {
            0x6000..=0x7FFF => self.prg_fetch(cart, address, 0x07, 0x2000),
            0x8000..=0xFFFF => {
                if (self.latch_data & 0x08) == 0 {
                    let smb2j_bank = (self.prg_smb2j as usize & 0x08)
                        | ((self.prg_smb2j as usize) << 2 & 0x04)
                        | ((self.prg_smb2j as usize) >> 1 & 0x03);
                    let bank = match address {
                        0x8000..=0x9FFF => 0,
                        0xA000..=0xBFFF => 1,
                        0xC000..=0xDFFF => smb2j_bank,
                        _ => 3,
                    };
                    self.prg_fetch(cart, address, bank, 0x2000)
                } else if (self.latch_data & 0x80) != 0 {
                    self.prg_fetch(cart, address, self.prg_value() >> 1, 0x8000)
                } else if (self.latch_data & 0x40) != 0 {
                    self.prg_fetch(cart, address, self.prg_value(), 0x4000)
                } else {
                    self.prg_fetch(cart, address, self.prg_value() << 1, 0x2000)
                }
            }
            _ => FetchResult { data: 0, driven: false },
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        match address {
            0x4020..=0x5FFF => {
                if (address & 0x20) != 0 && (address & 0x40) == 0 {
                    if (address & 0x100) != 0 {
                        self.irq = data;
                        if (data & 1) == 0 {
                            self.irq_ack_pending = true;
                        }
                    } else {
                        self.prg_smb2j = data;
                    }
                }
            }
            0x8000..=0x9FFF => self.latch_data = data,
            _ => {}
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack_pending;
        self.irq_ack_pending = false;
        ack
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v((self.latch_data & 4) != 0, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        _chr_ram: &[u8],
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
            let byte = if chr_rom.is_empty() {
                0
            } else {
                let bank = ((self.latch_data >> 1) & 3) as usize;
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mir = mirror_h_or_v((self.latch_data & 4) != 0, address);
            new_addr_bus |= vram[(mir & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, _cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address >= 0x2000 {
            let mir = mirror_h_or_v((self.latch_data & 4) != 0, address);
            vram[(mir & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if (self.irq & 1) != 0 {
            self.counter = self.counter.wrapping_add(1);
            (self.counter & 0x1000) != 0
        } else {
            self.counter = 0;
            false
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.latch_data);
        state.push(self.prg_smb2j);
        state.push(self.irq);
        state.extend_from_slice(&self.counter.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.latch_data = state[p];
            p += 1;
        }
        if p < state.len() {
            self.prg_smb2j = state[p];
            p += 1;
        }
        if p < state.len() {
            self.irq = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.counter = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        p
    }
}
