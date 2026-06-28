use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper20 {
    fds_disks: Vec<Vec<u8>>,
    disk_number: usize, 
    disk_state: FdsDiskState,
    disk_clock: i32,
    disk_address: usize,
    disk_address_fine: u8,
    shift_register: u8,
    shift_register_latch: u8,
    byte_transfer_flag: bool,
    looking_for_end_of_gap: bool,
    disk_reg_enabled: bool,
    fds_control: u8,
    eject_counter: i32,
    next_disk: usize,
    irq_reload_value: u16,
    irq_counter: u16,
    irq_enabled: bool,
    irq_repeat_enabled: bool,
    timer_irq_pending: bool,
    disk_irq_pending: bool,
    audio_wavetable: [u8; 64],
    audio_vol_env: u8,
    audio_freq: u16,
    audio_wave_disabled: bool,
    audio_master_vol: u8,
    audio_write_enable: bool,
    audio_phase: u32,
    current_audio_sample: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum FdsDiskState {
    Running,
    Inserting,
    SpinUp,
    Reset,
    Idle,
}

impl Mapper20 {
    pub fn new(fds_disks: Vec<Vec<u8>>) -> Self {
        let has_disk = !fds_disks.is_empty();
        Self {
            fds_disks,
            disk_number: if has_disk { 0 } else { usize::MAX },
            disk_state: if has_disk { FdsDiskState::Inserting } else { FdsDiskState::Idle },
            disk_clock: 0,
            disk_address: 0,
            disk_address_fine: 0,
            shift_register: 0,
            shift_register_latch: 0,
            byte_transfer_flag: false,
            looking_for_end_of_gap: false,
            disk_reg_enabled: false,
            fds_control: 0x26, 
            eject_counter: 0,
            next_disk: 0,
            irq_reload_value: 0,
            irq_counter: 0,
            irq_enabled: false,
            irq_repeat_enabled: false,
            timer_irq_pending: false,
            disk_irq_pending: false,
            audio_wavetable: [0; 64],
            audio_vol_env: 0,
            audio_freq: 0,
            audio_wave_disabled: true,
            audio_master_vol: 0,
            audio_write_enable: false,
            audio_phase: 0,
            current_audio_sample: 0.0,
        }
    }

    fn disk_inserted(&self) -> bool {
        self.disk_number != usize::MAX && self.disk_number < self.fds_disks.len()
    }
}

impl Mapper for Mapper20 {
    fn reset(&mut self) {
        let has_disk = !self.fds_disks.is_empty();
        self.disk_number = if has_disk { 0 } else { usize::MAX };
        self.disk_state = if has_disk { FdsDiskState::Inserting } else { FdsDiskState::Idle };
        self.disk_clock = 0;
        self.disk_address = 0;
        self.disk_address_fine = 0;
        self.shift_register = 0;
        self.shift_register_latch = 0;
        self.byte_transfer_flag = false;
        self.looking_for_end_of_gap = false;
        self.disk_reg_enabled = false;
        self.fds_control = 0x26; 
        self.eject_counter = 0;
        self.next_disk = 0;
        self.irq_reload_value = 0;
        self.irq_counter = 0;
        self.irq_enabled = false;
        self.irq_repeat_enabled = false;
        self.timer_irq_pending = false;
        self.disk_irq_pending = false;
        self.audio_wavetable = [0; 64];
        self.audio_vol_env = 0;
        self.audio_freq = 0;
        self.audio_wave_disabled = true;
        self.audio_master_vol = 0;
        self.audio_write_enable = false;
        self.audio_phase = 0;
        self.current_audio_sample = 0.0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0xE000 {
            let offset = address as usize & 0x1FFF;
            FetchResult { data: cart.prg_rom[offset], driven: true }
        } else if address >= 0x6000 {
            let offset = address as usize - 0x6000;
            if offset < cart.prg_ram.len() {
                FetchResult { data: cart.prg_ram[offset], driven: true }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else if address >= 0x4030 && address <= 0x4033 {
            if !self.disk_reg_enabled {
                return FetchResult { data: 0, driven: false };
            }
            let data = match address {
                0x4030 => {
                    let mut v = 0u8;
                    v |= self.fds_control & 0x08;            
                    v |= if self.timer_irq_pending { 0x01 } else { 0 }; 
                    v |= if self.disk_irq_pending { 0x02 } else { 0 };  
                    self.timer_irq_pending = false;
                    self.disk_irq_pending  = false;
                    let disk_len = if self.disk_inserted() {
                        self.fds_disks[self.disk_number].len()
                    } else {
                        0
                    };
                    if self.disk_address >= disk_len { v |= 0x40; } 
                    if self.byte_transfer_flag { v |= 0x80; }       
                    v
                }
                0x4031 => {
                    let v = self.shift_register_latch;
                    self.byte_transfer_flag = false;
                    self.disk_irq_pending = false; 
                    v
                }
                0x4032 => {
                    let mut v = 0u8;
                    if self.disk_state == FdsDiskState::Inserting {
                        v |= 1; 
                    }
                    if !((self.fds_control & 2) == 0
                         && (self.disk_state == FdsDiskState::Running
                             || self.disk_state == FdsDiskState::Idle))
                    {
                        v |= 2; 
                    }
                    v
                }
                _ => 0x80, 
            };
            FetchResult { data, driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0xE000 {
            let offset = address as usize - 0x6000;
            if offset < cart.prg_ram.len() {
                cart.prg_ram[offset] = data;
            }
            return;
        }
        if address >= 0x4040 && address <= 0x407F {
            if self.audio_write_enable {
                self.audio_wavetable[(address - 0x4040) as usize] = data & 0x3F;
            }
            return;
        }
        match address {
            0x4020 => {
                self.irq_reload_value = (self.irq_reload_value & 0xFF00) | (data as u16);
            }
            0x4021 => {
                self.irq_reload_value = (self.irq_reload_value & 0x00FF) | ((data as u16) << 8);
            }
            0x4022 => {
                if self.disk_reg_enabled {
                    self.irq_repeat_enabled = (data & 0x01) != 0;
                    self.irq_enabled        = (data & 0x02) != 0;
                    if self.irq_enabled {
                        self.irq_counter = self.irq_reload_value;
                    } else {
                        self.timer_irq_pending = false;
                    }
                }
            }
            0x4023 => {
                self.disk_reg_enabled = (data & 0x01) != 0;
                if !self.disk_reg_enabled {
                    self.irq_enabled       = false;
                    self.timer_irq_pending = false;
                    self.disk_irq_pending  = false;
                    self.fds_control &= 0xF3;
                    self.fds_control |= 6;
                }
            }
            0x4024 => {
                self.byte_transfer_flag = false;
            }
            0x4025 => {
                if (self.fds_control & 0x40) == 0 && (data & 0x40) != 0 {
                    self.looking_for_end_of_gap = true;
                }
                self.fds_control = data;
                if (data & 1) != 0 {
                    if self.disk_state == FdsDiskState::Idle {
                        self.disk_state = FdsDiskState::SpinUp;
                        self.disk_clock = 0;
                    }
                }
            }
            0x4080 => self.audio_vol_env = data,
            0x4082 => self.audio_freq = (self.audio_freq & 0x0F00) | (data as u16),
            0x4083 => {
                self.audio_freq = (self.audio_freq & 0x00FF) | (((data & 0x0F) as u16) << 8);
                self.audio_wave_disabled = (data & 0x80) != 0;
            }
            0x4089 => {
                self.audio_master_vol   = data & 0x03;
                self.audio_write_enable = (data & 0x80) != 0;
            }
            _ => {}
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
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
        let ciram = address >= 0x2000;
        if ciram {
            let masked_address = address & 0x2FFF;
            let offset = if ((self.fds_control >> 3) & 1) == 1 {
                (masked_address & 0x33FF) | ((masked_address & 0x0800) >> 1)
            } else {
                masked_address & 0x37FF
            };
            let data = vram[(offset & 0x07FF) as usize];
            new_addr_bus |= data as u16;
            return (data, new_addr_bus);
        }
        let offset = address as usize & 0x1FFF;
        let data = if using_chr_ram && offset < chr_ram.len() { chr_ram[offset] } else { 0 };
        new_addr_bus |= data as u16;
        (data, new_addr_bus)
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if ((self.fds_control >> 3) & 1) == 1 {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        Vec::new()
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, _state: &[u8], start: usize) -> usize {
        start
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.irq_enabled {
            if self.irq_counter <= cycles as u16 {
                self.timer_irq_pending = true;
                self.irq_counter = self.irq_reload_value;
                if !self.irq_repeat_enabled {
                    self.irq_enabled = false;
                }
            } else {
                self.irq_counter -= cycles as u16;
            }
        }
        if self.eject_counter > 0 {
            self.eject_counter -= 1;
            if self.eject_counter == 0 {
                self.disk_number    = self.next_disk;
                self.disk_state     = FdsDiskState::Inserting;
                self.disk_clock     = 0;
                self.disk_address   = 0;
                self.disk_address_fine = 0;
            }
        }
        if !self.audio_wave_disabled {
            self.audio_phase = self.audio_phase.wrapping_add(self.audio_freq as u32);
            let index  = (self.audio_phase >> 16) & 0x3F;
            let sample = self.audio_wavetable[index as usize];
            let direct_vol = (self.audio_vol_env & 0x80) != 0;
            let volume = if direct_vol {
                (self.audio_vol_env & 0x3F) as f32
            } else {
                let v = (self.audio_vol_env & 0x3F) as f32;
                if v > 0.0 { v } else { 32.0 }
            };
            let master_vol_scale = match self.audio_master_vol & 3 {
                0 => 1.0, 1 => 2.0 / 3.0, 2 => 0.5, 3 => 0.4, _ => 1.0,
            };
            let wave_out  = (sample as f32) - 32.0;
            let vol_scale = volume / 63.0;
            self.current_audio_sample = (wave_out / 32.0) * vol_scale * master_vol_scale;
        } else {
            self.current_audio_sample = 0.0;
        }
        for _ in 0..12 {
            self.disk_clock += 1;
            match self.disk_state {
                FdsDiskState::Running => {
                    if self.disk_clock == 244 {
                        self.disk_clock = 0;
                        if !self.disk_inserted() {
                            self.disk_state = FdsDiskState::Reset;
                            self.disk_clock = 0;
                            break;
                        }
                        let disk_len = self.fds_disks[self.disk_number].len();
                        if (self.fds_control & 0x2) == 0x2 {
                            self.disk_address += 625;
                        } else if (self.fds_control & 0x4) == 0x4 {
                            let shift_bit = if self.disk_address < disk_len {
                                (self.fds_disks[self.disk_number][self.disk_address] >> self.disk_address_fine) & 1
                            } else {
                                0
                            };
                            if self.looking_for_end_of_gap && (self.fds_control & 0x10) == 0 {
                                if shift_bit == 1 {
                                    self.looking_for_end_of_gap = false;
                                    self.disk_address_fine = 0;
                                    self.disk_address += 1;
                                } else {
                                    self.disk_address_fine += 1;
                                    if self.disk_address_fine == 8 {
                                        self.disk_address_fine = 0;
                                        self.disk_address += 1;
                                    }
                                }
                            } else {
                                self.shift_register >>= 1;
                                self.shift_register |= shift_bit * 0x80;
                                self.disk_address_fine += 1;
                                if self.disk_address_fine == 8 {
                                    self.disk_address_fine = 0;
                                    self.disk_address += 1;
                                    self.shift_register_latch = self.shift_register;
                                    self.byte_transfer_flag = true;
                                    if (self.fds_control & 0x80) != 0 {
                                        self.disk_irq_pending = true;
                                    }
                                }
                            }
                        } else {
                            self.disk_address_fine = 0;
                        }
                        if self.disk_address >= disk_len {
                            self.disk_state = FdsDiskState::Reset;
                            self.disk_clock = 0;
                        }
                    }
                }
                FdsDiskState::Reset | FdsDiskState::Inserting => {
                    if self.disk_clock >= 2_140_000 {
                        self.disk_clock = 0;
                        self.disk_address = 0;
                        self.disk_state = FdsDiskState::Idle;
                    }
                }
                FdsDiskState::SpinUp => {
                    if self.disk_clock >= 4_280_000 {
                        self.disk_clock = 0;
                        self.disk_state = FdsDiskState::Running;
                    }
                }
                FdsDiskState::Idle => {
                    self.disk_clock = 0;
                }
            }
        }
        self.timer_irq_pending || self.disk_irq_pending
    }

    fn change_disk(&mut self) {
        if self.fds_disks.is_empty() { return; }
        let next = if self.disk_number == usize::MAX {
            0
        } else {
            (self.disk_number + 1) % self.fds_disks.len()
        };
        self.disk_number        = usize::MAX;
        self.disk_state         = FdsDiskState::Inserting;
        self.disk_clock         = 0;
        self.disk_address       = 0;
        self.disk_address_fine  = 0;
        self.byte_transfer_flag = false;
        self.disk_irq_pending   = false;
        self.looking_for_end_of_gap = false;
        self.next_disk     = next;
        self.eject_counter = 900_000;
        eprintln!("FDS: Ejected disk, inserting side {} in ~0.5s", next);
    }

    fn audio_sample(&self) -> f32 {
        self.current_audio_sample
    }
}
