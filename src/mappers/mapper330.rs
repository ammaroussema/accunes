use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper330 {
    prg: [u8; 3],
    chr: [u8; 8],
    nt: [u8; 4],
    chip_ram: [u8; 128],
    irq_counter: u16,
    irq_pending: bool,
}

impl Mapper330 {
    pub fn new(_header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        Self {
            prg: [0, 1, 2],
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            nt: [0, 0, 1, 1],
            chip_ram: [0; 128],
            irq_counter: 0,
            irq_pending: false,
        }
    }
}

impl Mapper for Mapper330 {
    fn reset(&mut self) {
        self.prg = [0, 1, 2];
        self.chr = [0, 1, 2, 3, 4, 5, 6, 7];
        self.nt = [0, 0, 1, 1];
        self.irq_counter = 0;
        self.irq_pending = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        match address {
            0x4000..=0x4FFF => {
                if (address & 0x800) != 0 {
                    FetchResult { data: 0, driven: false }
                } else {
                    let idx = (address & 0x7F) as usize;
                    FetchResult { data: self.chip_ram[idx], driven: true }
                }
            }
            0x5000..=0x5FFF => FetchResult { data: 0, driven: false },
            0x6000..=0x7FFF => {
                let offset = (address as usize - 0x6000) % cart.prg_ram.len().max(1);
                FetchResult { data: cart.prg_ram[offset], driven: true }
            }
            0x8000..=0xDFFF => {
                let slot = (address as usize - 0x8000) / 0x2000;
                let bank = self.prg[slot.min(2)] as usize;
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                FetchResult {
                    data: cart.prg_rom[offset % cart.prg_rom.len().max(1)],
                    driven: true,
                }
            }
            0xE000..=0xFFFF => {
                let prg_len = cart.prg_rom.len().max(0x2000);
                let last_bank = prg_len / 0x2000 - 1;
                let offset = last_bank * 0x2000 + (address as usize & 0x1FFF);
                FetchResult {
                    data: cart.prg_rom[offset % prg_len],
                    driven: true,
                }
            }
            _ => FetchResult { data: 0, driven: false },
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        match address {
            0x4020..=0x4FFF => {
                let idx = (address & 0x7F) as usize;
                self.chip_ram[idx] = data;
            }
            0x6000..=0x7FFF => {
                let offset = (address as usize - 0x6000) % cart.prg_ram.len().max(1);
                if offset < cart.prg_ram.len() {
                    cart.prg_ram[offset] = data;
                }
            }
            0x8000..=0xBFFF => {
                let addr_off = address & 0xFFF;
                if (addr_off & 0x400) != 0 {
                    if (address & 0x2000) != 0 {
                        self.irq_counter = (self.irq_counter & 0x00FF) | ((data as u16) << 8);
                        self.irq_pending = true;
                    } else {
                        self.irq_counter = (self.irq_counter & 0xFF00) | data as u16;
                    }
                } else {
                    let bank = ((address >> 12 & 3) << 1) | ((addr_off >> 11) & 1);
                    self.chr[bank as usize] = data;
                }
            }
            0xC000..=0xDFFF => {
                let addr_off = address & 0xFFF;
                if (addr_off & 0x400) == 0 {
                    let idx = ((address >> 12 & 1) << 1) | ((addr_off >> 11) & 1);
                    self.nt[idx as usize] = data & 1;
                }
            }
            0xE000..=0xFFFF => {
                let bank = ((address >> 12) & 0xF) as usize;
                let addr_off = (address & 0xFFF) as usize;
                if bank == 0xF && (addr_off & 0x800) != 0 {
                    // $F800-$FFFF: N163 sound (stub)
                } else if (addr_off & 0x400) == 0 {
                    let idx = ((bank & 1) << 1) | ((addr_off >> 11) & 1);
                    if idx < 3 {
                        self.prg[idx] = data;
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_cpu_write(&mut self, address: u16, data: u8) {
        if address >= 0x4000 && address <= 0x401F {
            let idx = (address & 0x7F) as usize;
            self.chip_ram[idx] = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let nt_idx = ((address >> 10) & 3) as usize;
        let page = self.nt[nt_idx] as u16;
        (page << 10) | (address & 0x3FF)
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
            let bank = self.chr[(address >> 10) as usize] as usize;
            let offset = bank * 0x400 + (address as usize & 0x3FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let nt_idx = ((address >> 10) & 3) as usize;
            let page = self.nt[nt_idx] as usize;
            let mir_addr = page * 0x400 + (address as usize & 0x3FF);
            let byte = if alternative_nametable_arrangement && mir_addr >= 0x800 {
                let idx = mir_addr & 0x7FF;
                vram.get(idx).copied().unwrap_or(0)
            } else {
                vram.get(mir_addr & 0x7FF).copied().unwrap_or(0)
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let bank = self.chr[(address >> 10) as usize] as usize;
            let offset = bank * 0x400 + (address as usize & 0x3FF);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if (self.irq_counter & 0x8000) != 0 {
            self.irq_counter = self.irq_counter.wrapping_add(1);
            if self.irq_counter == 0 {
                return true;
            }
        }
        false
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.irq_pending {
            self.irq_pending = false;
            return true;
        }
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::with_capacity(3 + 8 + 4 + 128 + 2);
        state.extend_from_slice(&self.prg);
        state.extend_from_slice(&self.chr);
        state.extend_from_slice(&self.nt);
        state.extend_from_slice(&self.chip_ram);
        state.extend_from_slice(&self.irq_counter.to_le_bytes());
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 3 > state.len() { return p; }
        self.prg.copy_from_slice(&state[p..p+3]);
        p += 3;
        if p + 8 > state.len() { return p; }
        self.chr.copy_from_slice(&state[p..p+8]);
        p += 8;
        if p + 4 > state.len() { return p; }
        self.nt.copy_from_slice(&state[p..p+4]);
        p += 4;
        if p + 128 > state.len() { return p; }
        self.chip_ram.copy_from_slice(&state[p..p+128]);
        p += 128;
        if p + 2 > state.len() { return p; }
        self.irq_counter = u16::from_le_bytes([state[p], state[p+1]]);
        p + 2
    }
}
