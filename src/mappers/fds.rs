use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

const MOD_BIAS: [i32; 8] = [0, 1, 2, 4, 0, -4, -2, -1];

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
    sound_enable: bool,
    fds_control: u8,
    eject_counter: i32,
    next_disk: usize,
    irq_reload_value: u16,
    irq_counter: u16,
    irq_enabled: bool,
    irq_repeat_enabled: bool,
    timer_irq_pending: bool,
    disk_irq_pending: bool,

    wave_table: [u8; 64],
    wave_freq: u16,
    wave_phase: u32,

    vol_gain: u8,
    vol_env_speed: u8,
    vol_env_dir: bool,
    vol_env_dis: bool,
    vol_env_timer: u32,

    mod_gain: u8,
    mod_env_speed: u8,
    mod_env_dir: bool,
    mod_env_dis: bool,
    mod_env_timer: u32,

    master_env_speed: u8,

    mod_wave: [u8; 64],
    mod_freq: u16,
    mod_phase: u32,
    mod_pos: i32,
    mod_halt: bool,
    mod_write_pos: u8,

    wav_halt: bool,
    env_halt: bool,
    master_vol: u8,
    wave_write_enabled: bool,

    ext_port: u8,

    current_audio_sample: f32,

    byte_xfer_irq_pending: bool,
    disk_write_latch: u8,

    prg_ram_cycles: u8,
    refresh_counter: u8,
    cycle_timer: u16,
    watchdog_irq_enable: bool,
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
            sound_enable: false,
            fds_control: 0x26,
            eject_counter: 0,
            next_disk: 0,
            irq_reload_value: 0,
            irq_counter: 0,
            irq_enabled: false,
            irq_repeat_enabled: false,
            timer_irq_pending: false,
            disk_irq_pending: false,
            wave_table: [0; 64],
            wave_freq: 0,
            wave_phase: 0,
            vol_gain: 0,
            vol_env_speed: 0,
            vol_env_dir: false,
            vol_env_dis: true,
            vol_env_timer: 0,
            mod_gain: 0,
            mod_env_speed: 0,
            mod_env_dir: false,
            mod_env_dis: true,
            mod_env_timer: 0,
            master_env_speed: 0xFF,
            mod_wave: [0; 64],
            mod_freq: 0,
            mod_phase: 0,
            mod_pos: 0,
            mod_halt: true,
            mod_write_pos: 0,
            wav_halt: true,
            env_halt: true,
            master_vol: 0,
            wave_write_enabled: false,
            ext_port: 0,
            current_audio_sample: 0.0,
            byte_xfer_irq_pending: false,
            disk_write_latch: 0,
            prg_ram_cycles: 0,
            refresh_counter: 0,
            cycle_timer: 4095,
            watchdog_irq_enable: true,
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
        self.sound_enable = false;
        self.fds_control = 0x26;
        self.eject_counter = 0;
        self.next_disk = 0;
        self.irq_reload_value = 0;
        self.irq_counter = 0;
        self.irq_enabled = false;
        self.irq_repeat_enabled = false;
        self.timer_irq_pending = false;
        self.disk_irq_pending = false;
        self.wave_table = [0; 64];
        self.wave_freq = 0;
        self.wave_phase = 0;
        self.vol_gain = 0;
        self.vol_env_speed = 0;
        self.vol_env_dir = false;
        self.vol_env_dis = true;
        self.vol_env_timer = 0;
        self.mod_gain = 0;
        self.mod_env_speed = 0;
        self.mod_env_dir = false;
        self.mod_env_dis = true;
        self.mod_env_timer = 0;
        self.master_env_speed = 0xFF;
        self.mod_wave = [0; 64];
        self.mod_freq = 0;
        self.mod_phase = 0;
        self.mod_pos = 0;
        self.mod_halt = true;
        self.mod_write_pos = 0;
        self.wav_halt = true;
        self.env_halt = true;
        self.master_vol = 0;
        self.wave_write_enabled = false;
        self.ext_port = 0;
        self.current_audio_sample = 0.0;
        self.byte_xfer_irq_pending = false;
        self.disk_write_latch = 0;
        self.prg_ram_cycles = 0;
        self.refresh_counter = 0;
        self.cycle_timer = 4095;
        self.watchdog_irq_enable = true;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0xE000 {
            self.prg_ram_cycles = self.prg_ram_cycles.saturating_add(1);
        }
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
            let data = match address {
                0x4030 => {
                    let mut v = 0u8;
                    v |= self.fds_control & 0x08;
                    v |= if self.timer_irq_pending { 0x01 } else { 0 };
                    v |= if self.disk_irq_pending { 0x02 } else { 0 };
                    v |= if self.byte_transfer_flag { 0x80 } else { 0 };
                    self.timer_irq_pending = false;
                    self.disk_irq_pending  = false;
                    let disk_len = if self.disk_inserted() {
                        self.fds_disks[self.disk_number].len()
                    } else {
                        0
                    };
                    if self.disk_address >= disk_len { v |= 0x40; }
                    v
                }
                0x4031 => {
                    let v = self.shift_register_latch;
                    self.byte_transfer_flag = false;
                    self.byte_xfer_irq_pending = false;
                    v
                }
                0x4032 => {
                    let mut v = 0u8;
                    if self.disk_state == FdsDiskState::Inserting {
                        v |= 1;
                    }
                    if !((self.fds_control & 2) == 0
                         && self.disk_state == FdsDiskState::Running)
                    {
                        v |= 2;
                    }
                    v
                }
                0x4033 => {
                    let battery = if (self.fds_control & 0x02) == 0
                        && (self.disk_state == FdsDiskState::SpinUp || self.disk_state == FdsDiskState::Running)
                    {
                        0x80
                    } else {
                        0x00
                    };
                    if self.disk_reg_enabled {
                        battery | (self.ext_port & 0x7F)
                    } else {
                        // Output disabled: external pull-ups make bits 0-6 read as 1
                        battery | 0x7F
                    }
                }
                _ => 0x80,
            };
            FetchResult { data, driven: true }
        } else if address >= 0x4040 && address <= 0x409F {
            let data = match address {
                0x4040..=0x407F => {
                    if self.wave_write_enabled {
                        self.wave_table[(address - 0x4040) as usize]
                    } else {
                        self.wave_table[((self.wave_phase >> 16) & 0x3F) as usize]
                    }
                }
                0x4090 => self.vol_gain | 0x40,
                0x4091 => ((self.wave_phase >> 12) & 0xFF) as u8,
                0x4092 => self.mod_gain | 0x40,
                0x4093 => ((self.mod_phase >> 5) & 0x7F) as u8,
                0x4094 => {
                    let temp = self.mod_pos as i32 * self.mod_gain as i32;
                    ((temp >> 4) & 0xFF) as u8
                }
                0x4095 => {
                    const INC_TABLE: [u8; 8] = [0, 1, 2, 4, 12, 12, 14, 15];
                    let idx = ((self.mod_phase >> 16) & 0x3F) as usize;
                    INC_TABLE[(self.mod_wave[idx] & 0x07) as usize]
                }
                0x4096 => {
                    let idx = ((self.wave_phase >> 16) & 0x3F) as usize;
                    self.wave_table[idx] | 0x40
                }
                0x4097 => (self.mod_pos as u8) & 0x7F,
                _ => 0x40,
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
                self.prg_ram_cycles = self.prg_ram_cycles.saturating_add(1);
            }
            return;
        }
        if address >= 0x4040 && address <= 0x407F {
            if self.sound_enable && self.wave_write_enabled {
                self.wave_table[(address - 0x4040) as usize] = data & 0x3F;
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
                self.sound_enable = (data & 0x02) != 0;
                if (data & 0x80) == 0 {
                    self.disk_irq_pending = false;
                    self.watchdog_irq_enable = false;
                } else {
                    self.watchdog_irq_enable = true;
                }
                if (data & 0x01) == 0 {
                    self.irq_reload_value = 0;
                    self.irq_enabled = false;
                    self.irq_repeat_enabled = false;
                    self.timer_irq_pending = false;
                    self.disk_irq_pending = false;
                    self.byte_transfer_flag = false;
                    self.byte_xfer_irq_pending = false;
                    self.fds_control = 0x06;
                    self.ext_port = 0x7F;
                    self.prg_ram_cycles = 0;
                    self.refresh_counter = 0;
                    self.cycle_timer = 4095;
                }
                if (data & 0x02) == 0 {
                    self.wave_table = [0; 64];
                    self.wave_freq = 0;
                    self.wave_phase = 0;
                    self.vol_gain = 0;
                    self.vol_env_speed = 0;
                    self.vol_env_dir = false;
                    self.vol_env_dis = true;
                    self.vol_env_timer = 0;
                    self.mod_gain = 0;
                    self.mod_env_speed = 0;
                    self.mod_env_dir = false;
                    self.mod_env_dis = true;
                    self.mod_env_timer = 0;
                    self.master_env_speed = 0xFF;
                    self.mod_wave = [0; 64];
                    self.mod_freq = 0;
                    self.mod_phase = 0;
                    self.mod_pos = 0;
                    self.mod_halt = true;
                    self.mod_write_pos = 0;
                    self.wav_halt = true;
                    self.env_halt = true;
                    self.master_vol = 0;
                    self.wave_write_enabled = false;
                }
            }
            0x4024 => {
                if self.disk_reg_enabled {
                    self.disk_write_latch = data;
                    self.byte_transfer_flag = false;
                    self.byte_xfer_irq_pending = false;
                }
            }
            0x4025 => {
                if self.disk_reg_enabled {
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
            }
            0x4026 => {
                if self.disk_reg_enabled {
                    self.ext_port = data & 0x7F;
                }
            }
            0x4080 => {
                if self.sound_enable {
                    self.vol_env_dis = (data & 0x80) != 0;
                    self.vol_env_dir = (data & 0x40) != 0;
                    self.vol_env_speed = data & 0x3F;
                    self.vol_env_timer = 0;
                    if self.vol_env_dis {
                        self.vol_gain = self.vol_env_speed;
                    }
                }
            }
            0x4082 => {
                if self.sound_enable {
                    self.wave_freq = (self.wave_freq & 0x0F00) | (data as u16);
                }
            }
            0x4083 => {
                if self.sound_enable {
                    self.wave_freq = (self.wave_freq & 0x00FF) | (((data & 0x0F) as u16) << 8);
                    self.env_halt = (data & 0x40) != 0;
                    self.wav_halt = (data & 0x80) != 0;
                    if self.wav_halt {
                        self.wave_phase = 0;
                    }
                    if self.env_halt {
                        self.vol_env_timer = 0;
                        self.mod_env_timer = 0;
                    }
                }
            }
            0x4084 => {
                if self.sound_enable {
                    self.mod_env_dis = (data & 0x80) != 0;
                    self.mod_env_dir = (data & 0x40) != 0;
                    self.mod_env_speed = data & 0x3F;
                    self.mod_env_timer = 0;
                    if self.mod_env_dis {
                        self.mod_gain = self.mod_env_speed;
                    }
                }
            }
            0x4085 => {
                if self.sound_enable {
                    let val = (data & 0x7F) as i32;
                    self.mod_pos = if val >= 64 { val - 128 } else { val };
                    self.mod_phase = (self.mod_write_pos as u32) << 16;
                }
            }
            0x4086 => {
                if self.sound_enable {
                    self.mod_freq = (self.mod_freq & 0x0F00) | (data as u16);
                }
            }
            0x4087 => {
                if self.sound_enable {
                    self.mod_freq = (self.mod_freq & 0x00FF) | (((data & 0x0F) as u16) << 8);
                    self.mod_halt = (data & 0x80) != 0;
                    if self.mod_halt {
                        self.mod_phase = 0;
                    }
                }
            }
            0x4088 => {
                if self.sound_enable && self.mod_halt {
                    let idx = (self.mod_phase >> 16) as usize;
                    let val = data & 0x07;
                    self.mod_wave[idx & 0x3F] = val;
                    self.mod_phase = self.mod_phase.wrapping_add(0x010000) & 0x3FFFFF;
                    let idx2 = (self.mod_phase >> 16) as usize;
                    self.mod_wave[idx2 & 0x3F] = val;
                    self.mod_phase = self.mod_phase.wrapping_add(0x010000) & 0x3FFFFF;
                    self.mod_write_pos = (self.mod_phase >> 16) as u8;
                    self.wave_phase = (self.wave_phase.wrapping_add(0x10000)) & 0x3FFFFF;
                }
            }
            0x4089 => {
                if self.sound_enable {
                    self.wave_write_enabled = (data & 0x80) != 0;
                    self.master_vol = data & 0x03;
                }
            }
            0x408A => {
                if self.sound_enable {
                    self.master_env_speed = data;
                    self.vol_env_timer = 0;
                    self.mod_env_timer = 0;
                }
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
        let c = cycles as u32;

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
        if self.disk_reg_enabled {
            let prg = self.prg_ram_cycles as u32;
            self.prg_ram_cycles = 0;
            let non_prg = c - prg.min(c);
            // Non-PRG-RAM cycles: increment the 7-bit refresh counter
            for _ in 0..non_prg {
                let old = self.refresh_counter;
                self.refresh_counter = (self.refresh_counter + 1) & 0x7F;
                if self.refresh_counter < old {
                    // overflow 127→0: reload cycle timer to 3245
                    self.cycle_timer = 3245;
                } else {
                    // no overflow: decrement cycle timer
                    if self.cycle_timer == 0 {
                        self.cycle_timer = 4095;
                        if self.watchdog_irq_enable {
                            self.disk_irq_pending = true;
                        }
                    } else {
                        self.cycle_timer -= 1;
                    }
                }
            }
            // PRG-RAM cycles: only decrement cycle timer, no refresh counter increment
            for _ in 0..(c - non_prg) {
                if self.cycle_timer == 0 {
                    self.cycle_timer = 4095;
                    if self.watchdog_irq_enable {
                        self.disk_irq_pending = true;
                    }
                } else {
                    self.cycle_timer -= 1;
                }
            }
        } else {
            // Disk disabled: pause both counters at reset values
            self.prg_ram_cycles = 0;
            self.refresh_counter = 0;
            self.cycle_timer = 4095;
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

        // --- FDS Audio ---
        if !self.wav_halt || !self.mod_halt || !self.env_halt {
            // 1. Clock envelopes
            if !self.env_halt && !self.wav_halt && self.master_env_speed != 0 {
                let master = self.master_env_speed as u32 + 1;
                if !self.vol_env_dis && self.vol_env_speed <= 63 {
                    self.vol_env_timer += c;
                    let period_v = ((self.vol_env_speed as u32 + 1) * master) << 3;
                    if self.vol_env_timer >= period_v {
                        let steps = (self.vol_env_timer / period_v).min(32);
                        self.vol_env_timer %= period_v;
                        for _ in 0..steps {
                            if self.vol_env_dir { if self.vol_gain < 32 { self.vol_gain += 1; } }
                            else { if self.vol_gain > 0 { self.vol_gain -= 1; } }
                        }
                    }
                }
                if !self.mod_env_dis && self.mod_env_speed <= 63 {
                    self.mod_env_timer += c;
                    let period_m = ((self.mod_env_speed as u32 + 1) * master) << 3;
                    if self.mod_env_timer >= period_m {
                        let steps = (self.mod_env_timer / period_m).min(32);
                        self.mod_env_timer %= period_m;
                        for _ in 0..steps {
                            if self.mod_env_dir { if self.mod_gain < 32 { self.mod_gain += 1; } }
                            else { if self.mod_gain > 0 { self.mod_gain -= 1; } }
                        }
                    }
                }
            }

            // 2. Clock modulation table
            if !self.mod_halt && self.mod_freq > 0 {
                let prev = self.mod_phase >> 16;
                self.mod_phase = (self.mod_phase.wrapping_add(c * self.mod_freq as u32)) & 0x3FFFFF;
                let now = self.mod_phase >> 16;
                if now != prev {
                    let mut p = prev;
                    loop {
                        let wv = self.mod_wave[(p & 0x3F) as usize];
                        if wv == 4 {
                            self.mod_pos = 0;
                        } else {
                            self.mod_pos = (self.mod_pos + MOD_BIAS[wv as usize]) & 0x7F;
                            if self.mod_pos >= 64 {
                                self.mod_pos -= 128;
                            }
                        }
                        if p == now { break; }
                        p = (p + 1) & 0x3F;
                    }
                }
            }

            // 3. Compute modulation output
            let mut mod_out = 0i32;
            let mod_gain = self.mod_gain as i32;
            if !self.mod_halt && mod_gain != 0 && self.mod_freq > 0 {
                let pos = self.mod_pos;
                let temp = pos * mod_gain;
                let rem = temp & 0x0F;
                let mut mt = temp >> 4;
                if rem > 0 && (mt & 0x80) == 0 {
                    if pos < 0 { mt -= 1; } else { mt += 2; }
                }
                if mt >= 192 { mt -= 256; }
                else if mt < -64 { mt += 256; }
                mt = (self.wave_freq as i32) * mt;
                let rem2 = mt & 0x3F;
                mt >>= 6;
                if rem2 >= 32 { mt += 1; }
                mod_out = mt;
            }

            // 4. Advance wave phase
            if !self.wav_halt {
                let f = (self.wave_freq as i32) + mod_out;
                if f > 0 {
                    self.wave_phase = (self.wave_phase.wrapping_add(c * f as u32)) & 0x3FFFFF;
                }
            }

            // 5. Compute output sample — when write_enabled, output holds last value
            if !self.wave_write_enabled {
                let idx = ((self.wave_phase >> 16) & 0x3F) as usize;
                let sample = self.wave_table[idx] as i32;
                let vol = (self.vol_gain.min(32)) as i32;
                let fout = sample * vol - vol * 31;

                let master_scale = match self.master_vol & 3 {
                    0 => 1.0, 1 => 2.0 / 3.0, 2 => 0.5, 3 => 0.4, _ => 1.0,
                };
                self.current_audio_sample = (fout as f32) / (32.0 * 32.0) * master_scale;
            }
            // else: hold previous sample value
        }

        // --- Disk state machine ---
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
                                        self.byte_xfer_irq_pending = true;
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
        self.timer_irq_pending || self.disk_irq_pending || self.byte_xfer_irq_pending
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
        self.byte_xfer_irq_pending = false;
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


