use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

fn get_mask(maxval: usize) -> usize {
    let mut result = 0usize;
    let mut mv = maxval;
    while mv > 0 {
        result = (result << 1) | 1;
        mv >>= 1;
    }
    result
}

pub struct Mapper561 {
    submapper: u8,
    vertical_mirror: bool,
    mc1_mode: u8,
    mc2_mode: u8,
    latch: u8,
    chr8k: u8,
    prg8k: [u8; 4],
    fds_io: u8,
    fds_counter: i16,
    sgd_counter: i16,
    irq_active: bool,
    prg_buf: Option<Vec<u8>>,
    trainer_init: u16,
    trainer_configured: bool,
    boot_stage: u8,
    boot_stub_active: bool,
}

impl Mapper561 {
    pub fn new(submapper_id: u8, vertical_mirror: bool, trainer: &[u8]) -> Self {
        let (trainer_init, trainer_configured) = if trainer.len() >= 4 {
            if trainer.len() == 512 {
                (0x7003, true)
            } else {
                let init = u16::from_le_bytes([trainer[2], trainer[3]]);
                (init, init != 0)
            }
        } else {
            (0, false)
        };
        let mirror = if vertical_mirror { 0x01 } else { 0x11 };
        Self {
            submapper: submapper_id,
            vertical_mirror,
            mc1_mode: (submapper_id & 7) << 5 | mirror | 0x02,
            mc2_mode: 0x03,
            latch: 0,
            chr8k: 0,
            prg8k: [0x1C, 0x1D, 0x1E, 0x1F],
            fds_io: 0,
            fds_counter: 0,
            sgd_counter: 0,
            irq_active: false,
            prg_buf: None,
            trainer_init,
            trainer_configured,
            boot_stage: if trainer_configured { 1 } else { 0 },
            boot_stub_active: trainer_configured,
        }
    }

    fn prg_mode_1m(&self) -> u8 {
        self.mc1_mode >> 5
    }

    fn prg_mode_2m(&self) -> bool {
        self.mc2_mode & 0x01 == 0
    }

    fn protect_prg(&self) -> bool {
        self.mc1_mode & 0x02 != 0
    }

    fn protect_chr(&self) -> bool {
        self.prg_mode_1m() >= 4
    }

    fn mirroring(&self) -> u8 {
        self.mc1_mode & 0x11
    }

    fn nt_offset(&self, address: u16) -> u16 {
        match self.mirroring() {
            0x00 => address & 0x3FF,
            0x10 => 0x400 | (address & 0x3FF),
            0x01 => address & 0x37FF,
            _ => (address & 0x33FF) | ((address & 0x0800) >> 1),
        }
    }

    fn prg_mask(&self, rom_len: usize) -> usize {
        let pages = rom_len / 0x1000;
        if pages == 0 {
            return 0;
        }
        if self.submapper == 3 && pages < 64 {
            63
        } else {
            get_mask(pages - 1)
        }
    }

    fn ensure_prg_buf(&mut self, prg_rom: &[u8]) {
        if self.prg_buf.is_none() {
            let buf_len = (self.prg_mask(prg_rom.len()) + 1) * 0x1000;
            let mut buf = vec![0u8; buf_len];
            let n = prg_rom.len().min(buf_len);
            buf[..n].copy_from_slice(&prg_rom[..n]);
            self.prg_buf = Some(buf);
        }
    }

    fn prg_offset(&self, address: u16, rom_len: usize) -> usize {
        let window = ((address - 0x8000) >> 13) as usize;
        let bank8k = self.prg_bank_for_window(window);
        let bank4k = bank8k * 2 + ((address as usize >> 12) & 1);
        let mask = self.prg_mask(rom_len);
        (bank4k & mask) * 0x1000 + (address as usize & 0xFFF)
    }

    fn prg_bank_for_window(&self, window: usize) -> usize {
        if self.prg_mode_2m() {
            return self.prg8k[window & 3] as usize;
        }
        match self.prg_mode_1m() {
            0 => {
                if window < 2 {
                    (self.latch & 7) as usize * 2 + window
                } else {
                    14 + (window - 2)
                }
            }
            1 => {
                if window < 2 {
                    ((self.latch >> 2) & 15) as usize * 2 + window
                } else {
                    14 + (window - 2)
                }
            }
            2 => {
                if window < 2 {
                    (self.latch & 15) as usize * 2 + window
                } else {
                    30 + (window - 2)
                }
            }
            3 => {
                if window < 2 {
                    30 + window
                } else {
                    (self.latch & 15) as usize * 2 + (window - 2)
                }
            }
            4 => ((self.latch >> 4) & 3) as usize * 4 + window,
            5 => 12 + window,
            6 => match window {
                0 => (self.latch & 0x0F) as usize,
                1 => (self.latch >> 4) as usize,
                _ => 14 + (window - 2),
            },
            _ => match window {
                0 => (self.latch & 0x0F & !1) as usize,
                1 => ((self.latch >> 4) | 1) as usize,
                _ => 14 + (window - 2),
            },
        }
    }

    fn sync_chr8k(&mut self) {
        if self.prg_mode_1m() == 1 && !self.prg_mode_2m() {
            self.chr8k = self.latch & 3;
        }
    }

    fn chr_offset(&self, address: u16, chr_len: usize) -> usize {
        let off = self.chr8k as usize * 0x2000 + address as usize;
        if chr_len < 0x400 {
            return usize::MAX;
        }
        let mask = get_mask(chr_len / 0x400 - 1);
        let idx = (off >> 10) & mask;
        idx * 0x400 + (off & 0x3FF)
    }
}

impl Mapper for Mapper561 {
    fn reset(&mut self) {
        self.boot_stage = 0;
        self.boot_stub_active = false;
        self.irq_active = false;
        self.sgd_counter = 0;
        self.fds_io = 0;
        self.fds_counter = 0;
        self.sync_chr8k();
    }

    fn reset_power_cycle(&mut self) {
        let mirror = if self.vertical_mirror { 0x01 } else { 0x11 };
        self.mc1_mode = (self.submapper & 7) << 5 | mirror | 0x02;
        self.mc2_mode = 0x03;
        self.latch = 0;
        self.chr8k = 0;
        for i in 0..4 {
            self.prg8k[3 - i] = 0x1F - i as u8;
        }
        self.boot_stage = if self.trainer_configured { 1 } else { 0 };
        self.boot_stub_active = self.trainer_configured;
        self.irq_active = false;
        self.sgd_counter = 0;
        self.fds_io = 0;
        self.fds_counter = 0;
        self.sync_chr8k();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address == 0xFFFC {
            if self.boot_stage == 1 {
                self.boot_stage = 2;
                return FetchResult {
                    data: 0x00,
                    driven: true,
                };
            }
            if self.boot_stub_active {
                self.boot_stub_active = false;
            }
        } else if address == 0xFFFD {
            if self.boot_stage == 2 {
                self.boot_stage = 0;
                return FetchResult {
                    data: 0x07,
                    driven: true,
                };
            }
        }
        if address >= 0x6000 && address < 0x8000 {
            let len = cart.prg_ram.len();
            if len > 0 {
                return FetchResult {
                    data: cart.prg_ram[(address as usize & 0x1FFF) % len],
                    driven: true,
                };
            }
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        if address >= 0x8000 {
            self.ensure_prg_buf(&cart.prg_rom);
            let offset = self.prg_offset(address, cart.prg_rom.len());
            let data = match &self.prg_buf {
                Some(buf) if offset < buf.len() => buf[offset],
                _ => 0,
            };
            return FetchResult {
                data,
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
            let len = cart.prg_ram.len();
            if len > 0 {
                cart.prg_ram[(address as usize & 0x1FFF) % len] = data;
            }
            return;
        }
        if address >= 0x8000 {
            if self.protect_prg() {
                self.latch = data;
                match self.prg_mode_1m() {
                    1 | 4 | 5 => self.chr8k = self.latch & 3,
                    3 => self.chr8k = (self.latch >> 4) & 3,
                    _ => {}
                }
                let window = ((address - 0x8000) >> 13) as usize;
                self.prg8k[window & 3] = data >> 2;
            } else {
                self.ensure_prg_buf(&cart.prg_rom);
                let offset = self.prg_offset(address, cart.prg_rom.len());
                if let Some(buf) = self.prg_buf.as_mut() {
                    if offset < buf.len() {
                        buf[offset] = data;
                    }
                }
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.nt_offset(address)
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
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        let byte;
        if address < 0x2000 {
            let (buf, blen) = if !chr_ram.is_empty() {
                (chr_ram, chr_ram.len())
            } else if !chr_rom.is_empty() {
                (chr_rom, chr_rom.len())
            } else {
                (chr_rom, 0)
            };
            byte = if blen == 0 {
                0
            } else {
                let offset = self.chr_offset(address, blen);
                if offset < blen {
                    buf[offset]
                } else {
                    0
                }
            };
        } else if address < 0x3F00 {
            let mirrored = self.nt_offset(address);
            byte = vram[(mirrored & 0x7FF) as usize];
        } else {
            return (ppu_address_bus as u8, new_addr_bus);
        }
        new_addr_bus |= byte as u16;
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !self.protect_chr() {
                if cart.chr_ram.is_empty() && !cart.chr_rom.is_empty() {
                    cart.chr_ram.resize(cart.chr_rom.len(), 0);
                    cart.chr_ram.copy_from_slice(&cart.chr_rom);
                }
                let blen = cart.chr_ram.len();
                if blen >= 0x400 {
                    let offset = self.chr_offset(address, blen);
                    if offset < blen {
                        cart.chr_ram[offset] = data;
                    }
                }
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.nt_offset(address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.fds_counter = self.fds_counter.wrapping_add(3);
        while self.fds_counter >= 448 && (self.fds_io & 0x80) != 0 {
            self.irq_active = true;
            self.fds_counter -= 448;
        }
        if self.sgd_counter < 0 {
            self.sgd_counter += 1;
            if self.sgd_counter == 0 {
                self.irq_active = true;
            }
        }
        self.irq_active
    }

    fn cpu_clock_irq_level(&self) -> bool {
        true
    }

    fn handle_cpu_write(&mut self, address: u16, data: u8) {
        match address {
            0x4024 => {
                self.irq_active = false;
            }
            0x4025 => {
                self.irq_active = false;
                self.fds_io = data;
                if data & 0x42 != 0 {
                    self.fds_counter = 0;
                }
            }
            0x4100 => {
                self.irq_active = false;
                self.sgd_counter = (self.sgd_counter as u16 & 0xFF00 | data as u16) as i16;
                if data == 0 {
                    self.sgd_counter = 0;
                }
            }
            0x4101 => {
                self.irq_active = false;
                self.sgd_counter = (self.sgd_counter as u16 & 0x00FF | ((data as u16) << 8)) as i16;
            }
            0x42FC..=0x42FF => {
                self.mc1_mode = (data & 0xF0) | (address as u8 & 0x03);
                self.sync_chr8k();
            }
            0x43FC..=0x43FF => {
                self.mc2_mode = (data & 0xF0) | (address as u8 & 0x03);
                self.chr8k = data & 0x03;
                self.sync_chr8k();
            }
            _ => {}
        }
    }

    fn cpu_ram_override(&self, address: u16) -> Option<u8> {
        if !self.boot_stub_active || address < 0x0700 || address > 0x0705 {
            return None;
        }
        match address - 0x0700 {
            0 => Some(0x20),
            1 => Some(self.trainer_init as u8),
            2 => Some((self.trainer_init >> 8) as u8),
            3 => Some(0x6C),
            4 => Some(0xFC),
            5 => Some(0xFF),
            _ => None,
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut s = Vec::new();
        s.push(self.mc1_mode);
        s.push(self.mc2_mode);
        s.push(self.latch);
        s.push(self.chr8k);
        s.extend_from_slice(&self.prg8k);
        s.extend_from_slice(&self.sgd_counter.to_le_bytes());
        s.push(self.fds_io);
        s.extend_from_slice(&self.fds_counter.to_le_bytes());
        s.push(self.boot_stage);
        s.push(if self.boot_stub_active { 1 } else { 0 });
        s
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            self.mc1_mode = state[p];
            p += 1;
        }
        if p < state.len() {
            self.mc2_mode = state[p];
            p += 1;
        }
        if p < state.len() {
            self.latch = state[p];
            p += 1;
        }
        if p < state.len() {
            self.chr8k = state[p];
            p += 1;
        }
        if p + 4 <= state.len() {
            self.prg8k.copy_from_slice(&state[p..p + 4]);
            p += 4;
        }
        if p + 2 <= state.len() {
            self.sgd_counter = i16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.fds_io = state[p];
            p += 1;
        }
        if p + 2 <= state.len() {
            self.fds_counter = i16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.boot_stage = state[p];
            p += 1;
        }
        if p < state.len() {
            self.boot_stub_active = state[p] != 0;
            p += 1;
        }
        self.sync_chr8k();
        p - start
    }
}

pub fn install_mapper561_trainer(cart: &mut Cartridge) {
    if cart.prg_ram.len() < 0x2000 {
        cart.prg_ram.resize(0x2000, 0);
    }
    let src: &[u8] = if cart.trainer.len() >= 4 {
        &cart.trainer
    } else {
        &cart.misc_rom
    };
    if src.len() < 4 {
        return;
    }
    let (trainer_addr, trainer_data): (u16, &[u8]) = if src.len() == 512 {
        (0x7000, &src[..])
    } else {
        let addr = u16::from_le_bytes([src[0], src[1]]);
        (addr, &src[4..])
    };
    let base = (trainer_addr as usize) & 0x1FFF;
    let n = trainer_data.len().min(cart.prg_ram.len().saturating_sub(base));
    if n > 0 {
        cart.prg_ram[base..base + n].copy_from_slice(&trainer_data[..n]);
    }
}