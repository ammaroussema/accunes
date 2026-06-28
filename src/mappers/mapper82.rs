use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const MODE: usize = 6;
const PROTECT_A: usize = 7;
const PROTECT_B: usize = 8;
const PROTECT_C: usize = 9;
const PRG8: usize = 10;
const PRGA: usize = 11;
const PRGC: usize = 12;
const LATCH: usize = 13;
const IRQ: usize = 14;
const MODE_VERTICAL_MIRRORING: u8 = 0x01;
const MODE_CHR_FLIP: u8 = 0x02;
const IRQ_COUNTING: u8 = 0x01;
const IRQ_ENABLED: u8 = 0x02;
const IRQ_SOURCE: u8 = 0x04;

pub struct Mapper82 {
    reg: [u8; 16],
    latch: [u8; 8],
    counter: u16,
    irq_pending: bool,
}

impl Mapper82 {
    pub fn new() -> Self {
        let mut m = Mapper82 {
            reg: [0; 16],
            latch: [0; 8],
            counter: 0,
            irq_pending: false,
        };
        m.reset();
        m
    }

    fn prg_bits(&self, val: u8) -> u8 {
        val >> 2
    }

    fn sync(&mut self) {
    }

    fn chr_bank_2k(&self, chr_rom_len: usize, reg_idx: usize) -> usize {
        let bank = (self.reg[reg_idx] >> 1) as usize;
        let bank_count = (chr_rom_len / 0x800).max(1);
        bank % bank_count
    }

    fn chr_bank_1k(&self, chr_rom_len: usize, reg_idx: usize) -> usize {
        let bank = self.reg[reg_idx] as usize;
        let bank_count = (chr_rom_len / 0x400).max(1);
        bank % bank_count
    }

    fn get_chr_bank(&self, chr_rom_len: usize, address: u16) -> (usize, bool) {
        let flip = self.chr_flip();
        let ppu_bank = match address {
            0x0000..=0x07FF => 0,
            0x0800..=0x0FFF => 2,
            0x1000..=0x13FF => 4,
            0x1400..=0x17FF => 5,
            0x1800..=0x1BFF => 6,
            0x1C00..=0x1FFF => 7,
            _ => 0,
        };
        let reg_idx = if flip == 0 {
            match ppu_bank {
                0 => 0,  
                2 => 1,  
                4 => 2,  
                5 => 3,  
                6 => 4,  
                7 => 5,  
                _ => 0,
            }
        } else {
            match ppu_bank {
                0 => 2,  
                2 => 4,  
                4 => 0,  
                5 => 3,  
                6 => 1,  
                7 => 5,  
                _ => 0,
            }
        };
        let is_2k = ppu_bank == 0 || ppu_bank == 2;
        let bank = if is_2k {
            self.chr_bank_2k(chr_rom_len, reg_idx)
        } else {
            self.chr_bank_1k(chr_rom_len, reg_idx)
        };
        (bank, is_2k)
    }

    fn is_prg_ram_accessible(&self, address: u16) -> bool {
        if address >= 0x6000 && address <= 0x67FF {
            self.reg[PROTECT_A] == 0xCA
        } else if address >= 0x6800 && address <= 0x6FFF {
            self.reg[PROTECT_B] == 0x69
        } else if address >= 0x7000 && address <= 0x73FF {
            self.reg[PROTECT_C] == 0x84
        } else {
            false
        }
    }

    fn vertical_mirroring(&self) -> bool {
        (self.reg[MODE] & MODE_VERTICAL_MIRRORING) != 0
    }

    fn chr_flip(&self) -> usize {
        if self.reg[MODE] & MODE_CHR_FLIP != 0 { 4 } else { 0 }
    }
}

impl Mapper for Mapper82 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank_idx = match address {
                0x8000..=0x9FFF => PRG8,
                0xA000..=0xBFFF => PRGA,
                0xC000..=0xDFFF => PRGC,
                0xE000..=0xFFFF => {
                    let bank_count = (cart.prg_rom.len() / 0x2000).max(1);
                    let bank = bank_count - 1;
                    let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                    return FetchResult {
                        data: cart.prg_rom[offset % cart.prg_rom.len()],
                        driven: true,
                    };
                },
                _ => return FetchResult { data: 0, driven: false },
            };
            let bank = self.prg_bits(self.reg[bank_idx]) as usize;
            let bank_count = (cart.prg_rom.len() / 0x2000).max(1);
            let bank = bank % bank_count;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if address >= 0x6000 {
            let offset = (address - 0x6000) as usize;
            if offset < cart.prg_ram.len() && self.is_prg_ram_accessible(address) {
                if address & 0x3FF == 0 {
                    let latch_idx = (address >> 10) & 7;
                    FetchResult { data: self.latch[latch_idx as usize], driven: true }
                } else {
                    FetchResult { data: cart.prg_ram[offset], driven: true }
                }
            } else {
                FetchResult { data: 0, driven: true }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x7EF0 && address <= 0x7EFE {
            self.reg[(address - 0x7EF0) as usize] = data;
            self.sync();
        } else if address == 0x7EFF {
            self.counter = if self.reg[LATCH] != 0 {
                (self.reg[LATCH] as u16 + 1) * 16
            } else {
                1
            };
        } else if address >= 0x6000 {
            let offset = (address - 0x6000) as usize;
            if offset < cart.prg_ram.len() && self.is_prg_ram_accessible(address) {
                let latch_idx = (address >> 10) & 7;
                self.latch[latch_idx as usize] = data;
                if cart.prg_ram.len() >= 5120 {
                    cart.prg_ram[offset] = data;
                }
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else if self.vertical_mirroring() {
            address & 0x2FFF
        } else {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
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
            let chr_data = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[address as usize & 0x1FFF] }
            } else {
                let (bank, is_2k) = self.get_chr_bank(chr_rom.len(), address);
                let offset = if is_2k {
                    bank * 0x800 + (address as usize & 0x7FF)
                } else {
                    bank * 0x400 + (address as usize & 0x3FF)
                };
                if chr_rom.is_empty() { 0 } else { chr_rom[offset % chr_rom.len()] }
            };
            new_addr_bus |= chr_data as u16;
        } else if address < 0x3F00 {
            let mirrored = if self.vertical_mirroring() {
                address & 0x2FFF
            } else {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.reg[IRQ] & IRQ_COUNTING != 0 && self.reg[IRQ] & IRQ_SOURCE == 0 {
            if self.counter > 0 {
                self.counter -= 1;
            }
        } else {
            self.counter = if self.reg[LATCH] != 0 {
                (self.reg[LATCH] as u16 + 2) * 16
            } else {
                17
            };
        }
        self.irq_pending = self.reg[IRQ] & IRQ_ENABLED != 0 && self.counter == 0;
        self.irq_pending
    }

    fn reset(&mut self) {
        self.reg = [0; 16];
        self.latch = [0; 8];
        self.reg[PRG8] = 0x00;
        self.reg[PRGA] = 0x01;
        self.reg[PRGC] = 0xFE;
        self.counter = 0;
        self.irq_pending = false;
        self.sync();
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.extend_from_slice(&self.reg);
        state.extend_from_slice(&self.latch);
        state.extend_from_slice(&self.counter.to_le_bytes());
        state.push(self.irq_pending as u8);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        let prg_len = cart.prg_ram.len();
        if start + prg_len <= state.len() {
            cart.prg_ram.copy_from_slice(&state[start..start + prg_len]);
            start += prg_len;
        }
        let chr_len = cart.chr_ram.len();
        if start + chr_len <= state.len() {
            cart.chr_ram.copy_from_slice(&state[start..start + chr_len]);
            start += chr_len;
        }
        if start + 16 <= state.len() {
            self.reg.copy_from_slice(&state[start..start + 16]);
            start += 16;
        }
        if start + 8 <= state.len() {
            self.latch.copy_from_slice(&state[start..start + 8]);
            start += 8;
        }
        if start + 2 <= state.len() {
            self.counter = u16::from_le_bytes([state[start], state[start + 1]]);
            start += 2;
        }
        if start < state.len() {
            self.irq_pending = state[start] != 0;
            start += 1;
        }
        self.sync();
        start
    }
}
