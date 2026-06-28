use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper19 {
    prg: [u8; 3],
    chr: [u8; 8],
    nta: [u8; 4],
    dopol: u8,
    gorfus: u8,
    gorko: u8,
    iram: [u8; 128],
    irq_count: u16,
    irq_enable: bool,
    irq_pending: bool,
}

impl Mapper19 {
    pub fn new() -> Self {
        Mapper19 {
            prg: [0; 3],
            chr: [0; 8],
            nta: [0xFF; 4],
            dopol: 0,
            gorfus: 0xFF,
            gorko: 0,
            iram: [0; 128],
            irq_count: 0,
            irq_enable: false,
            irq_pending: false,
        }
    }
}

impl Mapper for Mapper19 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank = match address {
                0x8000..=0x9FFF => self.prg[0] as usize,
                0xA000..=0xBFFF => self.prg[1] as usize,
                0xC000..=0xDFFF => self.prg[2] as usize,
                0xE000..=0xFFFF => 0x3F, 
                _ => unreachable!(),
            };
            let num_banks = cart.prg_rom.len() / 0x2000;
            let final_bank = if bank == 0x3F { num_banks.saturating_sub(1) } else { bank % num_banks.max(1) };
            let final_offset = (final_bank * 0x2000) + (address as usize & 0x1FFF);
            FetchResult { data: cart.prg_rom[final_offset], driven: true }
        } else if address >= 0x6000 && address < 0x8000 {
            FetchResult { data: cart.prg_ram[address as usize & 0x1FFF], driven: true }
        } else {
            match address & 0xF800 {
                0x4800 => {
                    let data = self.iram[(self.dopol & 0x7F) as usize];
                    if (self.dopol & 0x80) != 0 {
                        self.dopol = (self.dopol & 0x80) | ((self.dopol + 1) & 0x7F);
                    }
                    FetchResult { data, driven: true }
                }
                0x5000 => FetchResult { data: (self.irq_count & 0xFF) as u8, driven: true },
                0x5800 => FetchResult { data: (self.irq_count >> 8) as u8, driven: true },
                _ => FetchResult { data: 0, driven: false },
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 && address < 0xB800 {
            let index = ((address - 0x8000) >> 11) as usize;
            self.chr[index] = data;
        } else if address >= 0xC000 && address < 0xE000 {
            let index = ((address - 0xC000) >> 11) as usize;
            self.nta[index] = data;
        } else if address >= 0x6000 && address < 0x8000 {
            cart.prg_ram[address as usize & 0x1FFF] = data;
        } else {
            match address & 0xF800 {
                0x4800 => {
                    self.iram[(self.dopol & 0x7F) as usize] = data;
                    if (self.dopol & 0x80) != 0 {
                        self.dopol = (self.dopol & 0x80) | ((self.dopol + 1) & 0x7F);
                    }
                }
                0x5000 => {
                    self.irq_count = (self.irq_count & 0xFF00) | (data as u16);
                    self.irq_pending = false;
                }
                0x5800 => {
                    self.irq_count = (self.irq_count & 0x00FF) | (((data & 0x7F) as u16) << 8);
                    self.irq_enable = (data & 0x80) != 0;
                    self.irq_pending = false;
                }
                0xE000 => {
                    self.prg[0] = data & 0x3F;
                    self.gorko = data >> 6;
                }
                0xE800 => {
                    self.prg[1] = data & 0x3F;
                    self.gorfus = data >> 6;
                }
                0xF000 => {
                    self.prg[2] = data & 0x3F;
                }
                0xF800 => {
                    self.dopol = data;
                }
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        address
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
            let index = (address >> 10) as usize;
            let bank = self.chr[index];
            let use_vram = bank >= 0xE0 && (self.gorfus & (1 << (index >> 2))) == 0;
            if use_vram {
                let vram_offset = ((bank as usize & 1) << 10) | (address as usize & 0x03FF);
                new_addr_bus |= vram[vram_offset & 0x07FF] as u16;
            } else {
                let offset = (bank as usize * 0x0400) + (address as usize & 0x03FF);
                if using_chr_ram {
                    new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
                } else {
                    new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
                }
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let index = ((address - 0x2000) >> 10) as usize;
            let bank = self.nta[index % 4];
            if bank >= 0xE0 {
                let vram_offset = ((bank as usize & 1) << 10) | (address as usize & 0x03FF);
                new_addr_bus |= vram[vram_offset & 0x07FF] as u16;
            } else {
                let offset = (bank as usize * 0x0400) + (address as usize & 0x03FF);
                if using_chr_ram {
                    new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
                } else {
                    new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
                }
            }
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            let index = (address >> 10) as usize;
            let bank = self.chr[index];
            let use_vram = bank >= 0xE0 && (self.gorfus & (1 << (index >> 2))) == 0;
            if use_vram {
                let vram_offset = ((bank as usize & 1) << 10) | (address as usize & 0x03FF);
                vram[vram_offset & 0x07FF] = data;
            } else if cart.using_chr_ram {
                let offset = (bank as usize * 0x0400) + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset & (len - 1)] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let index = ((address - 0x2000) >> 10) as usize;
            let bank = self.nta[index % 4];
            if bank >= 0xE0 {
                let vram_offset = ((bank as usize & 1) << 10) | (address as usize & 0x03FF);
                vram[vram_offset & 0x07FF] = data;
            } else if cart.using_chr_ram {
                let offset = (bank as usize * 0x0400) + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset & (len - 1)] = data;
            }
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.irq_enable {
            self.irq_count = self.irq_count.wrapping_add(cycles as u16);
            if self.irq_count >= 0x7FFF {
                self.irq_pending = true;
                self.irq_enable = false;
                self.irq_count = 0x7FFF;
            }
        }
        self.irq_pending
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.extend_from_slice(&self.prg);
        state.extend_from_slice(&self.chr);
        state.extend_from_slice(&self.nta);
        state.push(self.dopol);
        state.push(self.gorfus);
        state.push(self.gorko);
        state.extend_from_slice(&self.iram);
        state.extend_from_slice(&self.irq_count.to_le_bytes());
        state.push(if self.irq_enable { 1 } else { 0 });
        state.push(if self.irq_pending { 1 } else { 0 });
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
        if start + 3 <= state.len() {
            self.prg.copy_from_slice(&state[start..start + 3]);
            start += 3;
        }
        if start + 8 <= state.len() {
            self.chr.copy_from_slice(&state[start..start + 8]);
            start += 8;
        }
        if start + 4 <= state.len() {
            self.nta.copy_from_slice(&state[start..start + 4]);
            start += 4;
        }
        if start + 135 <= state.len() {
            self.dopol = state[start];
            self.gorfus = state[start + 1];
            self.gorko = state[start + 2];
            self.iram.copy_from_slice(&state[start + 3..start + 131]);
            self.irq_count = u16::from_le_bytes([state[start + 131], state[start + 132]]);
            self.irq_enable = state[start + 133] != 0;
            self.irq_pending = state[start + 134] != 0;
            start += 135;
        }
        start
    }
}
