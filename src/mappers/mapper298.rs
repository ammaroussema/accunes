use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper298 {
    chr_regs: [u8; 8],
    prg_regs: [u8; 2],
    swap_prg: bool,
    horizontal_mirroring: bool,
    irq_counter: u8,
    irq_reload_value: u8,
    irq_scaler: i16,
    irq_enabled: bool,
    irq_pending: bool,
}

impl Mapper298 {
    pub fn new() -> Self {
        let mut m = Mapper298 {
            chr_regs: [0; 8],
            prg_regs: [0; 2],
            swap_prg: false,
            horizontal_mirroring: false,
            irq_counter: 0,
            irq_reload_value: 0,
            irq_scaler: 0,
            irq_enabled: false,
            irq_pending: false,
        };
        m.reset();
        m
    }

    fn decode_addr(addr: u16) -> u16 {
        (addr & 0xF003) | ((addr & 0x000C) >> 2)
    }

    fn resolve_prg_bank(cart: &Cartridge, bank: i16) -> usize {
        let bank_count = (cart.prg_rom.len() / 0x2000).max(1) as i16;
        let idx = if bank >= 0 {
            bank % bank_count
        } else {
            let from_end = (-bank) as i16;
            bank_count - from_end
        };
        idx.clamp(0, bank_count - 1) as usize
    }

    fn prg_bank_for_slot(&self, slot: usize) -> i16 {
        match slot {
            0 => {
                if self.swap_prg { -2 } else { self.prg_regs[0] as i16 }
            }
            1 => self.prg_regs[1] as i16,
            2 => {
                if self.swap_prg { self.prg_regs[0] as i16 } else { -2 }
            }
            3 => -1,
            _ => 0,
        }
    }

    fn resolve_chr_bank(chr_len: usize, bank: u8) -> usize {
        let bank_count = (chr_len / 0x400).max(1);
        (bank as usize) % bank_count
    }

    fn write_register(&mut self, addr: u16, value: u8) {
        let addr = Self::decode_addr(addr);
        if (0xB000..=0xE003).contains(&addr) {
            let slot = ((((addr >> 11) as i16 - 6) as i16) | (addr as i16 & 0x01)) as u8 & 0x07;
            let shift = ((addr & 0x0002) << 1) as u8; 
            let keep_mask = 0xF0u8 >> shift;
            self.chr_regs[slot as usize] =
                (self.chr_regs[slot as usize] & keep_mask) | ((value & 0x0F) << shift);
            return;
        }
        match addr & 0xF003 {
            0x8000 => {
                self.prg_regs[0] = value;
            }
            0xA000 => {
                self.prg_regs[1] = value;
            }
            0x9000 => {
                self.horizontal_mirroring = (value & 0x01) != 0;
            }
            0x9001 => {
                self.swap_prg = (value & 0x03) != 0;
            }
            0xF000 => {
                self.irq_reload_value = (self.irq_reload_value & 0xF0) | (value & 0x0F);
            }
            0xF002 => {
                self.irq_reload_value = (self.irq_reload_value & 0x0F) | (value << 4);
            }
            0xF001 => {
                self.irq_enabled = (value & 0x02) != 0;
                if self.irq_enabled {
                    self.irq_scaler = 341;
                    self.irq_counter = self.irq_reload_value;
                }
                self.irq_pending = false;
            }
            0xF003 => {
                self.irq_pending = false;
            }
            _ => {}
        }
    }
}

impl Mapper for Mapper298 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let slot = ((address - 0x8000) / 0x2000) as usize; 
            let bank = self.prg_bank_for_slot(slot);
            let bank_idx = Self::resolve_prg_bank(cart, bank);
            let offset = bank_idx * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if address >= 0x6000 {
            let offset = address as usize & 0x1FFF;
            if offset < cart.prg_ram.len() {
                FetchResult {
                    data: cart.prg_ram[offset],
                    driven: true,
                }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            self.write_register(address, data);
        } else if address >= 0x6000 {
            let offset = address as usize & 0x1FFF;
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else if self.horizontal_mirroring {
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
            let slot = ((address >> 10) & 0x07) as usize; 
            let bank = self.chr_regs[slot];
            let byte = if using_chr_ram {
                if chr_ram.is_empty() {
                    0
                } else {
                    let bank_idx = Self::resolve_chr_bank(chr_ram.len(), bank);
                    let offset = bank_idx * 0x400 + (address as usize & 0x03FF);
                    chr_ram[offset % chr_ram.len()]
                }
            } else if chr_rom.is_empty() {
                0
            } else {
                let bank_idx = Self::resolve_chr_bank(chr_rom.len(), bank);
                let offset = bank_idx * 0x400 + (address as usize & 0x03FF);
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if self.horizontal_mirroring {
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
            let slot = ((address >> 10) & 0x07) as usize;
            let bank = self.chr_regs[slot];
            let len = cart.chr_ram.len();
            if len > 0 {
                let bank_idx = Self::resolve_chr_bank(len, bank);
                let offset = (bank_idx * 0x400 + (address as usize & 0x03FF)) % len;
                cart.chr_ram[offset] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.irq_enabled {
            for _ in 0..cycles {
                self.irq_scaler -= 3;
                if self.irq_scaler <= 0 {
                    self.irq_scaler += 341;
                    self.irq_counter = self.irq_counter.wrapping_add(1);
                    if self.irq_counter == 0 {
                        self.irq_pending = true;
                    }
                }
            }
        }
        let fired = self.irq_pending;
        if fired {
            self.irq_pending = false;
        }
        fired
    }

    fn reset(&mut self) {
        self.chr_regs = [0; 8];
        self.prg_regs = [0; 2];
        self.swap_prg = false;
        self.horizontal_mirroring = false;
        self.irq_counter = 0;
        self.irq_reload_value = 0;
        self.irq_scaler = 0;
        self.irq_enabled = false;
        self.irq_pending = false;
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.extend_from_slice(&self.chr_regs);
        state.extend_from_slice(&self.prg_regs);
        state.push(self.swap_prg as u8);
        state.push(self.horizontal_mirroring as u8);
        state.push(self.irq_counter);
        state.push(self.irq_reload_value);
        state.extend_from_slice(&self.irq_scaler.to_le_bytes());
        state.push(self.irq_enabled as u8);
        state.push(self.irq_pending as u8);
        state
    }

    fn load_mapper_registers(
        &mut self,
        cart: &mut Cartridge,
        state: &[u8],
        mut start: usize,
    ) -> usize {
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
        if start + 8 <= state.len() {
            self.chr_regs.copy_from_slice(&state[start..start + 8]);
            start += 8;
        }
        if start + 2 <= state.len() {
            self.prg_regs.copy_from_slice(&state[start..start + 2]);
            start += 2;
        }
        if start + 1 <= state.len() {
            self.swap_prg = state[start] != 0;
            start += 1;
        }
        if start + 1 <= state.len() {
            self.horizontal_mirroring = state[start] != 0;
            start += 1;
        }
        if start + 1 <= state.len() {
            self.irq_counter = state[start];
            start += 1;
        }
        if start + 1 <= state.len() {
            self.irq_reload_value = state[start];
            start += 1;
        }
        if start + 2 <= state.len() {
            self.irq_scaler = i16::from_le_bytes([state[start], state[start + 1]]);
            start += 2;
        }
        if start + 1 <= state.len() {
            self.irq_enabled = state[start] != 0;
            start += 1;
        }
        if start + 1 <= state.len() {
            self.irq_pending = state[start] != 0;
            start += 1;
        }
        start
    }
}
