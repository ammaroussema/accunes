use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper100 {
    pointer: u8,
    prg: [u8; 4],
    chr: [u8; 8],
    irq_latch: u8,
    irq_counter: u8,
    enable_irq: bool,
    reload_irq_counter: bool,
    nametable_mirroring: bool,
    m2_filter: u8,
    boot_pc_7000: bool,
}

impl Mapper100 {
    pub fn new() -> Self {
        Self {
            pointer: 0,
            prg: [0, 1, 0xFE, 0xFF],
            chr: [0, 1, 2, 3, 4, 5, 6, 7],
            irq_latch: 0,
            irq_counter: 0,
            enable_irq: false,
            reload_irq_counter: false,
            nametable_mirroring: false,
            m2_filter: 0,
            boot_pc_7000: false,
        }
    }

    fn clock_irq_counter(&mut self) -> bool {
        let prev = self.irq_counter;
        let reset_reload = self.reload_irq_counter;
        if prev == 0 || reset_reload {
            self.irq_counter = self.irq_latch;
            self.reload_irq_counter = false;
        } else {
            self.irq_counter = prev.wrapping_sub(1);
        }
        if self.irq_counter == 0 && self.enable_irq {
            return prev != 0 || reset_reload;
        }
        false
    }
}

impl Mapper for Mapper100 {
    fn reset(&mut self) {
        self.pointer = 0;
        for i in 0..4 {
            self.prg[i] = if (i & 2) != 0 { (0xFC | i) as u8 } else { i as u8 };
        }
        for i in 0..8 {
            self.chr[i] = i as u8;
        }
        self.irq_latch = 0;
        self.irq_counter = 0;
        self.enable_irq = false;
        self.reload_irq_counter = false;
        self.nametable_mirroring = false;
        self.m2_filter = 0;
        self.boot_pc_7000 = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if self.boot_pc_7000 {
                if address == 0xFFFC {
                    return FetchResult { data: 0x00, driven: true };
                }
                if address == 0xFFFD {
                    return FetchResult { data: 0x70, driven: true };
                }
            }
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult { data: 0, driven: false };
            }
            let banks_8k = len / 0x2000;
            let window = ((address - 0x8000) / 0x2000) as usize;
            let bank = (self.prg[window] as usize) % banks_8k;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            }
        } else if address >= 0x6000 {
            let offset = (address - 0x6000) as usize;
            if offset < cart.prg_ram.len() {
                FetchResult {
                    data: cart.prg_ram[offset],
                    driven: true,
                }
            } else {
                FetchResult {
                    data: 0,
                    driven: false,
                }
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address < 0x8000 {
            if address >= 0x6000 {
                let offset = (address - 0x6000) as usize;
                if offset < cart.prg_ram.len() {
                    cart.prg_ram[offset] = data;
                }
            }
            return;
        }
        if address < 0xA000 {
            if (address & 1) == 0 {
                self.pointer = data;
            } else {
                match self.pointer {
                    0x00 => {
                        self.chr[0] = data & 0xFE;
                        self.chr[1] = data | 0x01;
                    }
                    0x01 => {
                        self.chr[2] = data & 0xFE;
                        self.chr[3] = data | 0x01;
                    }
                    0x02 => self.chr[4] = data,
                    0x03 => self.chr[5] = data,
                    0x04 => self.chr[6] = data,
                    0x05 => self.chr[7] = data,
                    0x06 => self.prg[0] = data,
                    0x07 => self.prg[1] = data,
                    0x46 => self.prg[2] = data,
                    0x47 => self.prg[1] = data,
                    0x80 => {
                        self.chr[4] = data & 0xFE;
                        self.chr[5] = data | 0x01;
                    }
                    0x81 => {
                        self.chr[6] = data & 0xFE;
                        self.chr[7] = data | 0x01;
                    }
                    0x82 => self.chr[0] = data,
                    0x83 => self.chr[1] = data,
                    0x84 => self.chr[2] = data,
                    0x85 => self.chr[3] = data,
                    _ => {}
                }
            }
        } else {
            match address & 0xE001 {
                0xA000 => self.nametable_mirroring = (data & 1) != 0,
                0xC000 => self.irq_latch = data,
                0xC001 => self.reload_irq_counter = true,
                0xE000 => self.enable_irq = false,
                0xE001 => self.enable_irq = true,
                _ => {}
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.nametable_mirroring {
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
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let sub_window = (address / 0x0400) as usize; 
            let bank = self.chr[sub_window & 7];
            let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if self.nametable_mirroring {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let sub_window = (address / 0x0400) as usize;
                let bank = self.chr[sub_window & 7];
                let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
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
        if !ppu_a12_prev && a12 && self.m2_filter == 3 {
            irq |= self.clock_irq_counter();
        }
        if a12 {
            self.m2_filter = 0;
        }
        irq
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        let a12 = (ppu_address_bus & 0x1000) != 0;
        if !a12 && self.m2_filter < 3 {
            self.m2_filter += 1;
        }
        false
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.push(self.pointer);
        state.extend_from_slice(&self.prg);
        state.extend_from_slice(&self.chr);
        state.push(self.irq_latch);
        state.push(self.irq_counter);
        state.push(if self.enable_irq { 1 } else { 0 });
        state.push(if self.reload_irq_counter { 1 } else { 0 });
        state.push(if self.nametable_mirroring { 1 } else { 0 });
        state.push(self.m2_filter);
        state.push(if self.boot_pc_7000 { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        for i in 0..cart.prg_ram.len() {
            cart.prg_ram[i] = state[p];
            p += 1;
        }
        for i in 0..cart.chr_ram.len() {
            cart.chr_ram[i] = state[p];
            p += 1;
        }
        self.pointer = state[p];
        p += 1;
        self.prg.copy_from_slice(&state[p..p+4]);
        p += 4;
        self.chr.copy_from_slice(&state[p..p+8]);
        p += 8;
        self.irq_latch = state[p];
        p += 1;
        self.irq_counter = state[p];
        p += 1;
        self.enable_irq = state[p] != 0;
        p += 1;
        self.reload_irq_counter = state[p] != 0;
        p += 1;
        self.nametable_mirroring = state[p] != 0;
        p += 1;
        self.m2_filter = state[p];
        p += 1;
        self.boot_pc_7000 = state[p] != 0;
        p + 1
    }
}

pub fn install_mapper100_trainer(cart: &mut Cartridge) {
    if cart.prg_ram.len() < 0x2000 {
        cart.prg_ram.resize(0x2000, 0);
    }
    if !cart.trainer.is_empty() && cart.prg_ram.len() >= 0x1000 + 512 {
        cart.prg_ram[0x1000..0x1000 + 512].copy_from_slice(&cart.trainer[..512]);
    }
}
