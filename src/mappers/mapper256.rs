use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mapper::mirror_h_or_v;
use crate::mappers::mmc3::{mmc3_chr_bank, Mmc3Config};

const PPU_MANGLE: [[u8; 6]; 16] = [
    [0, 1, 2, 3, 4, 5],  // 0: Normal
    [1, 0, 5, 4, 3, 2],  // 1: Waixing VT03
    [0, 1, 2, 3, 4, 5],  // 2: Trump Grand
    [5, 4, 3, 2, 0, 1],  // 3: Zechess
    [2, 5, 0, 4, 3, 1],  // 4: Qishenglong
    [1, 0, 5, 4, 3, 2],  // 5: Waixing VT02
    [0, 1, 2, 3, 4, 5],  // 6
    [0, 1, 2, 3, 4, 5],  // 7
    [0, 1, 2, 3, 4, 5],  // 8
    [0, 1, 2, 3, 4, 5],  // 9
    [0, 1, 2, 3, 4, 5],  // A
    [0, 1, 2, 3, 4, 5],  // B
    [0, 1, 2, 3, 4, 5],  // C
    [0, 1, 2, 3, 4, 5],  // D: Cube Tech
    [0, 1, 2, 3, 4, 5],  // E: Karaoto
    [0, 1, 2, 3, 4, 5],  // F: Jungletac
];

const CPU_MANGLE: [[u8; 4]; 16] = [
    [0, 1, 2, 3],  // 0: Normal
    [0, 1, 2, 3],  // 1: Waixing VT03
    [1, 0, 2, 3],  // 2: Trump Grand
    [0, 1, 2, 3],  // 3: Zechess
    [0, 1, 2, 3],  // 4: Qishenglong
    [0, 1, 2, 3],  // 5: Waixing VT02
    [0, 1, 2, 3],  // 6
    [0, 1, 2, 3],  // 7
    [0, 1, 2, 3],  // 8
    [0, 1, 2, 3],  // 9
    [0, 1, 2, 3],  // A
    [0, 1, 2, 3],  // B
    [0, 1, 2, 3],  // C
    [0, 1, 2, 3],  // D: Cube Tech
    [0, 1, 2, 3],  // E: Karaoto
    [0, 1, 2, 3],  // F: Jungletac
];

const MMC3_MANGLE: [[u8; 8]; 16] = [
    [0, 1, 2, 3, 4, 5, 6, 7],  // 0: Normal
    [5, 4, 3, 2, 1, 0, 6, 7],  // 1: Waixing VT03
    [0, 1, 2, 3, 4, 5, 7, 6],  // 2: Trump Grand
    [0, 1, 2, 3, 4, 5, 6, 7],  // 3: Zechess
    [0, 1, 2, 3, 4, 5, 6, 7],  // 4: Qishenglong
    [0, 1, 2, 3, 4, 5, 6, 7],  // 5: Waixing VT02
    [0, 1, 2, 3, 4, 5, 6, 7],  // 6
    [0, 1, 2, 3, 4, 5, 6, 7],  // 7
    [0, 1, 2, 3, 4, 5, 6, 7],  // 8
    [0, 1, 2, 3, 4, 5, 6, 7],  // 9
    [0, 1, 2, 3, 4, 5, 6, 7],  // A
    [0, 1, 2, 3, 4, 5, 6, 7],  // B
    [0, 1, 2, 3, 4, 5, 6, 7],  // C
    [0, 1, 2, 3, 4, 5, 6, 7],  // D: Cube Tech
    [0, 1, 2, 3, 4, 5, 6, 7],  // E: Karaoto
    [0, 1, 2, 3, 4, 5, 6, 7],  // F: Jungletac
];

fn prg_mask(ps: u8) -> u16 {
    if ps == 7 { 0xFF } else { 0x3F >> ps }
}

fn prg_rom_read(rom: &[u8], offset: usize) -> u8 {
    if rom.is_empty() { 0 } else { rom[offset % rom.len()] }
}

pub struct Mapper256 {
    submapper: u8,
    reg2000: [u8; 256],
    reg4100: [u8; 256],
    relative8k: u32,
    irq_reload: u8,
    irq_counter: u8,
    irq_enable: bool,
    irq_delay: u8,
    pa12_filter: u8,
    prg_ram_protect: u8,
}

impl Mapper256 {
    pub fn new(_config: Mmc3Config, submapper: u8) -> Self {
        let mut m = Self {
            submapper,
            reg2000: [0; 256],
            reg4100: [0; 256],
            relative8k: 0,
            irq_reload: 0,
            irq_counter: 0,
            irq_enable: false,
            irq_delay: 0,
            pa12_filter: 0,
            prg_ram_protect: 0,
        };
        m.reset_registers();
        m
    }

    fn reset_registers(&mut self) {
        self.reg2000[0x10] = 0x00;
        self.reg2000[0x12] = 0x04;
        self.reg2000[0x13] = 0x05;
        self.reg2000[0x14] = 0x06;
        self.reg2000[0x15] = 0x07;
        self.reg2000[0x16] = 0x00;
        self.reg2000[0x17] = 0x02;
        self.reg2000[0x18] = 0x00;
        self.reg2000[0x1A] = 0x00;
        self.reg4100[0x00] = 0x00;
        self.reg4100[0x05] = 0x00;
        self.reg4100[0x07] = 0x00;
        self.reg4100[0x08] = 0x01;
        self.reg4100[0x09] = 0xFE;
        self.reg4100[0x0A] = 0x00;
        self.reg4100[0x0B] = 0x00;
        self.reg4100[0x0F] = 0xFF;
        self.reg4100[0x60] = 0x00;
        self.reg4100[0x61] = 0x00;
        self.relative8k = 0;
        self.irq_counter = 0;
        self.irq_reload = 0;
        self.irq_enable = false;
        self.irq_delay = 0;
        self.pa12_filter = 0;
    }

    fn get_prg_bank(&self, slot: u8) -> usize {
        let ps = self.reg4100[0x0B] & 0x07;
        let pq3 = self.reg4100[0x0A] as u16;
        let pa21 = (self.reg4100[0x00] >> 4) as u16;
        let pq2en = (self.reg4100[0x0B] & 0x40) != 0;
        let comr6 = (self.reg4100[0x05] & 0x40) != 0;

        let prg_and = prg_mask(ps);
        let prg_or = (pq3 | (pa21 << 8)) & !prg_and;

        let s = if comr6 {
            match slot { 0 => 2, 2 => 0, _ => slot }
        } else {
            slot
        };
        let val = match s {
            0 => self.reg4100[0x07] as u16,
            1 => self.reg4100[0x08] as u16,
            2 => {
                if pq2en { self.reg4100[0x09] as u16 } else { 0xFE }
            }
            3 => 0xFF,
            _ => 0,
        };
        ((val & prg_and | prg_or) as usize) + self.relative8k as usize
    }
}

impl Mapper for Mapper256 {
    fn reset(&mut self) {
        self.reset_registers();
    }

    fn handle_cpu_write(&mut self, address: u16, data: u8) {
        if address >= 0x2000 && address < 0x2100 {
            let a = (address & 0xFF) as u8;
            let idx = if a >= 0x12 && a <= 0x17 {
                0x12 + PPU_MANGLE[self.submapper as usize][(a - 0x12) as usize]
            } else {
                a
            } as usize;
            self.reg2000[idx] = data;
        } else if address >= 0x4100 && address < 0x4200 {
            let a = (address & 0xFF) as u8;
            let idx = if a >= 0x07 && a <= 0x0A {
                0x07 + CPU_MANGLE[self.submapper as usize][(a - 0x07) as usize]
            } else {
                a
            } as usize;
            self.reg4100[idx] = data;
            match idx {
                0x01 => self.irq_reload = data,
                0x02 => self.irq_counter = 0,
                0x03 => { self.irq_enable = false; }
                0x04 => self.irq_enable = true,
                0x60 | 0x61 => {
                    if idx == 0x60 {
                        self.relative8k = (self.relative8k & 0xF00) | data as u32;
                    } else {
                        self.relative8k = (self.relative8k & 0x0FF) | ((data as u32 & 0x0F) << 8);
                    }
                }
                _ => {}
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            if address >= 0x6000 && !cart.prg_ram.is_empty() {
                let off = (address - 0x6000) as usize;
                if off < cart.prg_ram.len() {
                    cart.prg_ram[off] = data;
                }
            }
            return;
        }

        let mangled = if address & 0xE001 == 0x8000 {
            data & 0xF8 | MMC3_MANGLE[self.submapper as usize][(data & 0x07) as usize]
        } else {
            data
        };

        match address & 0xE001 {
            0x8000 => {
                self.reg4100[0x05] = mangled & !0x20;
            }
            0x8001 => {
                let pointer = (self.reg4100[0x05] & 0x07) as usize;
                if pointer < 2 {
                    self.reg2000[0x16 + pointer] = mangled;
                } else if pointer < 6 {
                    self.reg2000[0x10 + pointer] = mangled;
                } else {
                    self.reg4100[0x07 + pointer - 6] = mangled;
                }
            }
            0xA000 => {
                self.reg4100[0x06] = mangled & 1;
            }
            0xA001 => {
                self.prg_ram_protect = mangled;
            }
            0xC000 => {
                self.irq_reload = mangled;
            }
            0xC001 => {
                self.irq_counter = 0;
            }
            0xE000 => {
                self.irq_enable = false;
            }
            0xE001 => {
                self.irq_enable = true;
            }
            _ => {}
        }
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address < 0x8000 {
            if address >= 0x6000 && !cart.prg_ram.is_empty() {
                let off = (address - 0x6000) as usize;
                if off < cart.prg_ram.len() {
                    return FetchResult { data: cart.prg_ram[off], driven: true };
                }
            }
            return FetchResult { data: 0, driven: false };
        }

        let slot = if address >= 0xE000 {
            3
        } else if address >= 0xC000 {
            2
        } else if address >= 0xA000 {
            1
        } else {
            0
        };

        let bank = self.get_prg_bank(slot);
        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
        FetchResult {
            data: prg_rom_read(&cart.prg_rom, offset),
            driven: true,
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else {
            let hv = self.reg4100[0x06] & 1;
            mirror_h_or_v(hv != 0, address)
        }
    }

    fn fetch_ppu(
        &mut self,
        prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        _chr_ram: &[u8],
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
            let r8000 = if self.reg4100[0x05] & 0x80 != 0 { 0x80 } else { 0 };
            let chr_2k0 = self.reg2000[0x16];
            let chr_2k8 = self.reg2000[0x17];
            let chr_1k0 = self.reg2000[0x12];
            let chr_1k4 = self.reg2000[0x13];
            let chr_1k8 = self.reg2000[0x14];
            let chr_1kc = self.reg2000[0x15];
            let bank = mmc3_chr_bank(r8000, chr_2k0, chr_2k8, chr_1k0, chr_1k4, chr_1k8, chr_1kc, address);

            let va21 = (self.reg4100[0x00] & 0x0F) as u16;
            let va18 = ((self.reg2000[0x18] >> 4) & 0x07) as u16;
            let ext_offset = ((va21 << 11) | (va18 << 8)) as u32 * 0x0400;

            let vb0s_table: [u8; 8] = [0, 1, 2, 0, 3, 4, 5, 1];
            let vb0s = self.reg2000[0x1A] & 0x07;
            let rv6 = self.reg2000[0x1A] & 0xF8;
            let chr_and = 0xFFu8 >> vb0s_table[vb0s as usize];
            let chr_or = rv6 & !chr_and;
            let masked_bank = (bank & chr_and) | chr_or;

            let offset = (masked_bank as usize) * 0x0400 + (address as usize & 0x03FF) + ext_offset as usize;
            let byte = if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                prg_rom_read(prg_rom, offset)
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else {
                let hv = self.reg4100[0x06] & 1;
                mirror_h_or_v(hv != 0, address)
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < _prg_vram.len() {
                    _prg_vram[idx]
                } else {
                    0
                }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let r8000 = if self.reg4100[0x05] & 0x80 != 0 { 0x80 } else { 0 };
                let chr_2k0 = self.reg2000[0x16];
                let chr_2k8 = self.reg2000[0x17];
                let chr_1k0 = self.reg2000[0x12];
                let chr_1k4 = self.reg2000[0x13];
                let chr_1k8 = self.reg2000[0x14];
                let chr_1kc = self.reg2000[0x15];
                let bank = mmc3_chr_bank(r8000, chr_2k0, chr_2k8, chr_1k0, chr_1k4, chr_1k8, chr_1kc, address);
                let va21 = (self.reg4100[0x00] & 0x0F) as u16;
                let va18 = ((self.reg2000[0x18] >> 4) & 0x07) as u16;
                let ext_offset = ((va21 << 11) | (va18 << 8)) as u32 * 0x0400;
                let offset = (bank as u32) * 0x0400 + (address as u32 & 0x03FF) + ext_offset;
                let len = cart.chr_ram.len() as u32;
                cart.chr_ram[offset as usize % len as usize] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
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
        _scanline: u16,
        _dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        let a12 = (ppu_address_bus & 0x1000) != 0;
        let mut irq = false;
        if !ppu_a12_prev && a12 && self.pa12_filter == 3 {
            let prev = self.irq_counter;
            if prev == 0 {
                self.irq_counter = self.irq_reload;
            } else {
                self.irq_counter = prev.wrapping_sub(1);
            }
            if self.irq_counter == 0 && self.irq_enable {
                irq = true;
            }
        }
        if a12 {
            self.pa12_filter = 0;
        }
        irq
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        let a12 = (ppu_address_bus & 0x1000) != 0;
        if !a12 && self.pa12_filter < 3 {
            self.pa12_filter += 1;
        }
        false
    }

    fn take_irq_ack(&mut self) -> bool {
        false
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.irq_delay > 0 {
            self.irq_delay -= 1;
            if self.irq_delay == 0 {
                return true;
            }
        }
        false
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.extend_from_slice(&self.reg2000);
        state.extend_from_slice(&self.reg4100);
        state.extend_from_slice(&self.relative8k.to_le_bytes());
        state.push(self.irq_reload);
        state.push(self.irq_counter);
        state.push(if self.irq_enable { 1 } else { 0 });
        state.push(self.irq_delay);
        state.push(self.pa12_filter);
        state.push(self.prg_ram_protect);
        state.push(self.submapper);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..cart.prg_ram.len() {
            if p < state.len() { cart.prg_ram[i] = state[p]; }
            p += 1;
        }
        for i in 0..cart.chr_ram.len() {
            if p < state.len() { cart.chr_ram[i] = state[p]; }
            p += 1;
        }
        for i in 0..256usize {
            if p < state.len() { self.reg2000[i] = state[p]; }
            p += 1;
        }
        for i in 0..256usize {
            if p < state.len() { self.reg4100[i] = state[p]; }
            p += 1;
        }
        if p + 4 <= state.len() {
            self.relative8k = u32::from_le_bytes([state[p], state[p+1], state[p+2], state[p+3]]);
            p += 4;
        }
        if p < state.len() { self.irq_reload = state[p]; } p += 1;
        if p < state.len() { self.irq_counter = state[p]; } p += 1;
        if p < state.len() { self.irq_enable = state[p] != 0; } p += 1;
        if p < state.len() { self.irq_delay = state[p]; } p += 1;
        if p < state.len() { self.pa12_filter = state[p]; } p += 1;
        if p < state.len() { self.prg_ram_protect = state[p]; } p += 1;
        if p < state.len() { self.submapper = state[p]; } p += 1;
        p
    }

    fn insert_coin(&mut self, _coin: u8) {}
    fn service_button(&mut self) {}
    fn get_dip_switches(&self) -> u8 { 0 }
    fn set_dip_switches(&mut self, _value: u8) {}
    fn battery_save_data(&self, _cart: &Cartridge) -> Option<Vec<u8>> { None }
    fn load_battery_save(&mut self, _cart: &mut Cartridge, _data: &[u8]) {}
}
