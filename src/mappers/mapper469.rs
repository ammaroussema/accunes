use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

const FDS_CPU_CYCLES_PER_SECOND: u32 = 1789772;

pub struct Mapper469 {
    rom_data: Vec<u8>,
    prg_start: usize,

    fpga_mode: u8,
    fpga_bank: u8,
    fds_disk: u8,
    disk_pointer: u32,

    data_counter: u32,
    fds_control: u8,
    pending_data: bool,

    inserted: bool,
    change_count: u32,
    eject_count: u32,
    change_state: u8,

    timer_counter: u16,
    timer_latch: u16,
    timer_enabled: bool,
    timer_repeat: bool,
    pending_timer: bool,

    irq_active: bool,
}

impl Mapper469 {
    pub fn new(rom_data: Vec<u8>, prg_start: usize) -> Self {
        Self {
            rom_data,
            prg_start,
            fpga_mode: 0x00,
            fpga_bank: 0xFF,
            fds_disk: 0x00,
            disk_pointer: 0,
            data_counter: 0,
            fds_control: 0x00,
            pending_data: false,
            inserted: true,
            change_count: 0,
            eject_count: 0,
            change_state: 0,
            timer_counter: 0,
            timer_latch: 0,
            timer_enabled: false,
            timer_repeat: false,
            pending_timer: false,
            irq_active: false,
        }
    }

    fn mapper_mode(&self) -> u8 {
        self.fpga_mode >> 4
    }

    fn irq_condition(&self) -> bool {
        (self.fds_control & 0x80 != 0 && self.pending_data) || self.pending_timer
    }

    fn prg_read_mode4(&self, cart: &Cartridge, address: u16) -> u8 {
        if address >= 0xE000 {
            let bank = (self.fpga_bank as usize) << 5 | 7;
            let prg_len = cart.prg_rom.len();
            if prg_len == 0 {
                return 0;
            }
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            return cart.prg_rom[offset % prg_len];
        }
        if address >= 0x6000 {
            let idx = (address - 0x6000) as usize;
            if idx < cart.prg_ram.len() {
                return cart.prg_ram[idx];
            }
        }
        0
    }

    fn prg_read_default(&self, cart: &Cartridge, address: u16) -> u8 {
        if address >= 0x8000 {
            let prg_len = cart.prg_rom.len();
            if prg_len == 0 {
                return 0;
            }
            let bank = self.fpga_bank as usize;
            let offset = bank * 0x8000 + (address as usize & 0x7FFF);
            return cart.prg_rom[offset % prg_len];
        }
        0
    }

    fn store_prg_mode4(&self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0xE000).contains(&address) {
            let idx = (address - 0x6000) as usize;
            if idx < cart.prg_ram.len() {
                cart.prg_ram[idx] = data;
            }
        }
    }

    fn read_register(&mut self, _cart: &Cartridge, address: u16) -> FetchResult {
        let reg = address & 0xFFF;
        match reg {
            0x030 => {
                let result = (if self.pending_timer { 0x01 } else { 0x00 })
                    | (if !self.pending_timer && self.pending_data {
                        0x02
                    } else {
                        0x00
                    });
                self.pending_data = false;
                self.pending_timer = false;
                FetchResult {
                    data: result,
                    driven: true,
                }
            }
            0x032 => {
                if self.eject_count == 0 {
                    match self.change_state {
                        0 => {
                            if self.change_count >= FDS_CPU_CYCLES_PER_SECOND * 3 {
                                self.change_state = 1;
                                self.eject_count = 50000;
                                self.inserted = true;
                            }
                        }
                        1 => {
                            self.change_state = 2;
                            self.eject_count = 50000;
                            self.inserted = false;
                        }
                        2 => {
                            self.change_state = 0;
                            self.inserted = true;
                            self.fds_disk ^= 1;
                        }
                        _ => {}
                    }
                }
                let result = if self.inserted { 0x00 } else { 0x05 };
                self.change_count = 0;
                FetchResult {
                    data: result,
                    driven: true,
                }
            }
            0x103 => {
                let bank_32k = ((self.fds_disk as i32 - 2).unsigned_abs() as usize) << 1
                    | (self.disk_pointer >> 15 & 1) as usize;
                let disk_offset = bank_32k << 18 | 0x38000 | (self.disk_pointer as usize & 0x7FFF);
                let file_offset = self.prg_start + disk_offset;
                let data = if file_offset < self.rom_data.len() {
                    self.rom_data[file_offset]
                } else {
                    0
                };
                FetchResult {
                    data,
                    driven: true,
                }
            }
            _ => FetchResult {
                data: 0,
                driven: false,
            },
        }
    }

    fn write_register(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        let reg = address & 0xFFF;
        match reg {
            0x020 => {
                self.timer_latch = (self.timer_latch & 0xFF00) | data as u16;
            }
            0x021 => {
                self.timer_latch = (self.timer_latch & 0x00FF) | ((data as u16) << 8);
            }
            0x022 => {
                self.timer_repeat = (data & 1) != 0;
                self.timer_enabled = (data & 2) != 0;
                if self.timer_enabled {
                    self.timer_counter = self.timer_latch;
                } else {
                    self.pending_timer = false;
                }
            }
            0x025 => {
                self.fds_control = data;
                self.data_counter = 0;
            }
            0x100 => {
                self.disk_pointer = 0xFFFB;
                self.change_count = 0;
            }
            0x102 => {
                self.disk_pointer = self.disk_pointer.wrapping_add(1);
                self.pending_data = false;
            }
            0x110 => {
                self.fds_disk = data;
                self.change_count = 0;
            }
            0x700 => {
                self.fpga_mode = data;
            }
            0x701 => {
                self.fpga_bank = data;
            }
            _ => {}
        }
    }
}

impl Mapper for Mapper469 {
    fn reset(&mut self) {
        self.fpga_mode = 0x00;
        self.fpga_bank = 0xFF;
        self.fds_disk = 0x00;

        self.fds_control = 0x00;
        self.data_counter = 0;
        self.pending_data = false;

        self.timer_counter = 0;
        self.timer_latch = 0;
        self.timer_enabled = false;
        self.timer_repeat = false;
        self.pending_timer = false;

        self.inserted = true;
        self.change_count = 0;
        self.eject_count = 0;
        self.change_state = 0;
        self.disk_pointer = 0;
        self.irq_active = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x5000 && address < 0x6000 {
            return self.read_register(cart, address);
        }
        if self.mapper_mode() == 4 {
            FetchResult {
                data: self.prg_read_mode4(cart, address),
                driven: address >= 0x6000,
            }
        } else {
            FetchResult {
                data: self.prg_read_default(cart, address),
                driven: address >= 0x8000,
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address < 0x6000 {
            self.write_register(cart, address, data);
            return;
        }
        if self.mapper_mode() == 4 && address >= 0x6000 {
            self.store_prg_mode4(cart, address, data);
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let horizontal = if self.mapper_mode() == 4 {
            (self.fds_control & 8) != 0
        } else {
            true
        };
        let norm = address & 0x2FFF;
        if horizontal {
            (norm & 0x33FF) | ((norm & 0x0800) >> 1)
        } else {
            norm & 0x37FF
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
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
        if address < 0x2000 {
            let offset = (address as usize & 0x1FFF) % chr_ram.len().max(1);
            new_addr_bus |= chr_ram.get(offset).copied().unwrap_or(0) as u16;
        } else {
            let horizontal = if self.mapper_mode() == 4 {
                (self.fds_control & 8) != 0
            } else {
                true
            };
            let norm = address & 0x2FFF;
            let mirrored = if horizontal {
                (norm & 0x33FF) | ((norm & 0x0800) >> 1)
            } else {
                norm & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && !cart.chr_ram.is_empty() {
            let offset = (address as usize & 0x1FFF) % cart.chr_ram.len();
            cart.chr_ram[offset] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.timer_enabled {
            if self.timer_counter == 0 {
                self.pending_timer = true;
                if self.timer_repeat {
                    self.timer_counter = self.timer_latch;
                } else {
                    self.timer_enabled = false;
                }
            } else {
                self.timer_counter -= 1;
            }
        }

        self.data_counter += 3;
        while self.data_counter >= 448 {
            self.data_counter -= 448;
            self.pending_data = true;
        }

        let irq = self.irq_condition();
        if irq && !self.irq_active {
            self.irq_active = true;
        } else if !irq {
            self.irq_active = false;
        }

        self.change_count += 1;
        if self.eject_count > 0 {
            self.eject_count -= 1;
        }

        irq
    }

    fn take_irq_ack(&mut self) -> bool {
        !self.irq_active
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.fpga_mode);
        state.push(self.fpga_bank);
        state.push(self.fds_disk);
        state
    }

    fn load_mapper_registers(
        &mut self,
        _cart: &mut Cartridge,
        state: &[u8],
        start: usize,
    ) -> usize {
        let mut p = start;
        if p < state.len() {
            self.fpga_mode = state[p];
            p += 1;
        }
        if p < state.len() {
            self.fpga_bank = state[p];
            p += 1;
        }
        if p < state.len() {
            self.fds_disk = state[p];
            p += 1;
        }
        p
    }
}
