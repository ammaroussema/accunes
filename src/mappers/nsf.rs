// the nsf mapper!!!

use crate::cartridge::Cartridge;
use crate::mapper::{ExpansionAudioType, FetchResult, Mapper};
use crate::mappers::fds::Mapper20;
use crate::mappers::fme7::MapperFME7;
use crate::mappers::mmc5::{MapperMMC5, Mmc5Config};
use crate::mappers::n106::Mapper19;
use crate::mappers::rainbow_audio::{Vrc6Pulse, Vrc6Saw};
use crate::mappers::vrc7::Vrc7;
use crate::nsf::NsfInfo;

pub struct NsfMapper {
    pub info: NsfInfo,
    pub current_track: u8,
    pub prg_banks: [u8; 8],
    pub fds_banks: [u8; 2],
    pub has_bankswitching: bool,
    pub bios: [u8; 0x20],
    pub prg_ram: [u8; 0x8000],
    pub fds_ram: [u8; 0x6000],
    pub mmc5_exram: [u8; 0x400],
    pub mmc5_multiplier: [u8; 2],
    pub irq_counter: u32,
    pub irq_reload: u32,
    pub irq_asserted: bool,
    pub cpu_clock_rate: f64,

    pub vrc6_pulse1: Option<Vrc6Pulse>,
    pub vrc6_pulse2: Option<Vrc6Pulse>,
    pub vrc6_saw: Option<Vrc6Saw>,
    pub vrc7: Option<Vrc7>,
    pub mmc5: Option<MapperMMC5>,
    pub namco163: Option<Mapper19>,
    pub sunsoft5b: Option<MapperFME7>,
    pub fds: Option<Mapper20>,
}

impl NsfMapper {
    pub fn new(info: NsfInfo) -> Self {
        let has_bankswitching = info.bank_setup.iter().any(|&b| b != 0);
        let track = info.starting_song.saturating_sub(1);

        let mut bios = [0u8; 0x20];
        bios[0x00] = 0x58; 
        bios[0x01] = 0x20; 
        bios[0x02] = (info.init_address & 0xFF) as u8;
        bios[0x03] = (info.init_address >> 8) as u8;
        bios[0x04] = 0x8D;
        bios[0x05] = 0x00;
        bios[0x06] = 0x41;
        bios[0x07] = 0x58; 
        bios[0x08] = 0x4C;
        bios[0x09] = 0x08;
        bios[0x0A] = 0x41;

        bios[0x10] = 0x8D; 
        bios[0x11] = 0x00;
        bios[0x12] = 0x41;
        bios[0x13] = 0x20;
        bios[0x14] = (info.play_address & 0xFF) as u8;
        bios[0x15] = (info.play_address >> 8) as u8;
        bios[0x16] = 0x58; 
        bios[0x17] = 0x40;

        let clock = 1_789_773.0;
        let speed_us = if info.is_pal() { info.play_speed_pal } else { info.play_speed_ntsc };
        let irq_reload = (speed_us as f64 * clock / 1_000_000.0).round().max(100.0) as u32;

        let mut mapper = Self {
            info,
            current_track: track,
            prg_banks: [0; 8],
            fds_banks: [0; 2],
            has_bankswitching,
            bios,
            prg_ram: [0; 0x8000],
            fds_ram: [0; 0x6000],
            mmc5_exram: [0; 0x400],
            mmc5_multiplier: [0; 2],
            irq_counter: irq_reload,
            irq_reload,
            irq_asserted: false,
            cpu_clock_rate: clock,
            vrc6_pulse1: None,
            vrc6_pulse2: None,
            vrc6_saw: None,
            vrc7: None,
            mmc5: None,
            namco163: None,
            sunsoft5b: None,
            fds: None,
        };

        mapper.init_track(track);
        mapper
    }

    fn setup_nonbanked(&mut self) {
        let start_bank = (self.info.load_address / 0x1000) as i32;
        for i in 0..16i32 {
            let slot = start_bank + i - 8;
            if slot >= 0 && slot < 8 {
                self.prg_banks[slot as usize] = i as u8;
            }
        }
    }

    fn update_irq_reload(&mut self) {
        let speed_us = if self.info.is_pal() {
            self.info.play_speed_pal
        } else {
            self.info.play_speed_ntsc
        };
        self.irq_reload = (speed_us as f64 * self.cpu_clock_rate / 1_000_000.0).round().max(100.0) as u32;
    }

    pub fn init_track(&mut self, track: u8) {
        self.current_track = track;
        self.prg_ram.fill(0);
        self.fds_ram.fill(0);
        self.mmc5_exram.fill(0);
        self.mmc5_multiplier = [0, 0];
        self.irq_asserted = false;
        self.irq_counter = self.irq_reload;

        if self.has_bankswitching {
            self.prg_banks.copy_from_slice(&self.info.bank_setup);
            if self.info.has_fds() {
                self.fds_banks[0] = self.info.bank_setup[6];
                self.fds_banks[1] = self.info.bank_setup[7];
            }
        } else {
            self.setup_nonbanked();
        }

        if self.info.has_vrc6() {
            self.vrc6_pulse1 = Some(Vrc6Pulse::new());
            self.vrc6_pulse2 = Some(Vrc6Pulse::new());
            self.vrc6_saw = Some(Vrc6Saw::new());
        }
        if self.info.has_vrc7() {
            let mut chip = Vrc7::new(0);
            chip.set_audio_clock(self.cpu_clock_rate);
            self.vrc7 = Some(chip);
        }
        if self.info.has_mmc5() {
            let mut chip = MapperMMC5::new(Mmc5Config {
                wram_size: 0x2000,
                battery_save_size: 0,
            });
            chip.set_cpu_clock(self.cpu_clock_rate);
            self.mmc5 = Some(chip);
        }
        if self.info.has_namco163() {
            let mut chip = Mapper19::new();
            chip.set_cpu_clock(self.cpu_clock_rate);
            self.namco163 = Some(chip);
        }
        if self.info.has_sunsoft5b() {
            let mut chip = MapperFME7::new();
            chip.set_cpu_clock(self.cpu_clock_rate);
            self.sunsoft5b = Some(chip);
        }
        if self.info.has_fds() {
            let mut chip = Mapper20::new(vec![]);
            chip.set_cpu_clock(self.cpu_clock_rate);
            self.fds = Some(chip);
        }
    }
}

impl Mapper for NsfMapper {
    fn is_nsf(&self) -> bool {
        true
    }

    fn nsf_info(&self) -> Option<&crate::nsf::NsfInfo> {
        Some(&self.info)
    }

    fn init_nsf_track(&mut self, track: u8) {
        self.init_track(track);
    }

    fn set_cpu_clock(&mut self, clock: f64) {
        self.cpu_clock_rate = clock;
        self.update_irq_reload();
        if let Some(ref mut vrc7) = self.vrc7 {
            vrc7.set_audio_clock(clock);
        }
        if let Some(ref mut fds) = self.fds {
            fds.set_cpu_clock(clock);
        }
        if let Some(ref mut mmc5) = self.mmc5 {
            mmc5.set_cpu_clock(clock);
        }
        if let Some(ref mut n163) = self.namco163 {
            n163.set_cpu_clock(clock);
        }
        if let Some(ref mut fme7) = self.sunsoft5b {
            fme7.set_cpu_clock(clock);
        }
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x4100 && address <= 0x411F {
            return FetchResult {
                data: self.bios[(address - 0x4100) as usize],
                driven: true,
            };
        }

        match address {
            0xFFFC => return FetchResult { data: 0x00, driven: true },
            0xFFFD => return FetchResult { data: 0x41, driven: true },
            0xFFFE => return FetchResult { data: 0x10, driven: true },
            0xFFFF => return FetchResult { data: 0x41, driven: true },
            _ => {}
        }

        if self.info.has_mmc5() {
            if address == 0x5205 {
                let prod = (self.mmc5_multiplier[0] as u16) * (self.mmc5_multiplier[1] as u16);
                return FetchResult { data: (prod & 0xFF) as u8, driven: true };
            }
            if address == 0x5206 {
                let prod = (self.mmc5_multiplier[0] as u16) * (self.mmc5_multiplier[1] as u16);
                return FetchResult { data: (prod >> 8) as u8, driven: true };
            }
            if (0x5C00..=0x5FFF).contains(&address) {
                return FetchResult {
                    data: self.mmc5_exram[(address - 0x5C00) as usize],
                    driven: true,
                };
            }
        }

        if self.info.has_namco163() && (0x4800..=0x4FFF).contains(&address) {
            if let Some(ref mut n163) = self.namco163 {
                return n163.fetch_prg(cart, address);
            }
        }

        if self.info.has_fds() {
            if (0x4040..=0x4092).contains(&address) {
                if let Some(ref mut fds) = self.fds {
                    return fds.fetch_prg(cart, address);
                }
            }
            if (0x8000..=0xDFFF).contains(&address) {
                return FetchResult {
                    data: self.fds_ram[(address - 0x8000) as usize],
                    driven: true,
                };
            }
            if (0x6000..=0x6FFF).contains(&address) {
                let bank = self.fds_banks[0] as usize;
                let offset = (bank * 0x1000) + (address as usize & 0x0FFF);
                if offset < cart.prg_rom.len() {
                    return FetchResult { data: cart.prg_rom[offset], driven: true };
                }
            }
            if (0x7000..=0x7FFF).contains(&address) {
                let bank = self.fds_banks[1] as usize;
                let offset = (bank * 0x1000) + (address as usize & 0x0FFF);
                if offset < cart.prg_rom.len() {
                    return FetchResult { data: cart.prg_rom[offset], driven: true };
                }
            }
        }

        if (0x6000..=0x7FFF).contains(&address) {
            return FetchResult {
                data: self.prg_ram[(address - 0x6000) as usize],
                driven: true,
            };
        }

        if address >= 0x8000 {
            let slot = ((address - 0x8000) / 0x1000) as usize;
            let bank = self.prg_banks[slot] as usize;
            let offset = (bank * 0x1000) + (address as usize & 0x0FFF);
            if offset < cart.prg_rom.len() {
                return FetchResult {
                    data: cart.prg_rom[offset],
                    driven: true,
                };
            } else {
                return FetchResult { data: 0, driven: true };
            }
        }

        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address == 0x4100 {
            self.irq_asserted = false;
            self.irq_counter = self.irq_reload;
            return;
        }

        if address == 0x5FF6 {
            self.fds_banks[0] = data;
            return;
        }
        if address == 0x5FF7 {
            self.fds_banks[1] = data;
            return;
        }
        if (0x5FF8..=0x5FFF).contains(&address) {
            let slot = (address - 0x5FF8) as usize;
            self.prg_banks[slot] = data;
            return;
        }

        if self.info.has_mmc5() {
            if address == 0x5205 {
                self.mmc5_multiplier[0] = data;
                return;
            }
            if address == 0x5206 {
                self.mmc5_multiplier[1] = data;
                return;
            }
            if (0x5000..=0x5015).contains(&address) {
                if let Some(ref mut mmc5) = self.mmc5 {
                    mmc5.store_prg(cart, address, data);
                }
                return;
            }
            if (0x5C00..=0x5FFF).contains(&address) {
                self.mmc5_exram[(address - 0x5C00) as usize] = data;
                return;
            }
        }

        if self.info.has_vrc6() {
            match address {
                0x9000..=0x9002 => {
                    if let Some(ref mut p1) = self.vrc6_pulse1 {
                        p1.write_reg(address, data);
                    }
                    return;
                }
                0x9003 => {
                    let freq_shift = if (data & 0x04) != 0 { 8 } else if (data & 0x02) != 0 { 4 } else { 0 };
                    if let Some(ref mut p1) = self.vrc6_pulse1 { p1.set_frequency_shift(freq_shift); }
                    if let Some(ref mut p2) = self.vrc6_pulse2 { p2.set_frequency_shift(freq_shift); }
                    if let Some(ref mut s) = self.vrc6_saw { s.set_frequency_shift(freq_shift); }
                    return;
                }
                0xA000..=0xA002 => {
                    if let Some(ref mut p2) = self.vrc6_pulse2 {
                        p2.write_reg(address, data);
                    }
                    return;
                }
                0xB000..=0xB002 => {
                    if let Some(ref mut s) = self.vrc6_saw {
                        s.write_reg(address, data);
                    }
                    return;
                }
                _ => {}
            }
        }

        if self.info.has_vrc7() {
            if address == 0x9010 {
                if let Some(ref mut vrc7) = self.vrc7 {
                    vrc7.write_sound(false, data);
                }
                return;
            }
            if address == 0x9030 {
                if let Some(ref mut vrc7) = self.vrc7 {
                    vrc7.write_sound(true, data);
                }
                return;
            }
        }

        if self.info.has_namco163() {
            if (0x4800..=0x4FFF).contains(&address) || (0xF800..=0xFFFF).contains(&address) {
                if let Some(ref mut n163) = self.namco163 {
                    n163.store_prg(cart, address, data);
                }
                return;
            }
        }

        if self.info.has_sunsoft5b() && (0xC000..=0xFFFF).contains(&address) {
            if let Some(ref mut fme7) = self.sunsoft5b {
                fme7.store_prg(cart, address, data);
            }
            return;
        }

        if self.info.has_fds() {
            if (0x4040..=0x4092).contains(&address) {
                if let Some(ref mut fds) = self.fds {
                    fds.store_prg(cart, address, data);
                }
                return;
            }
            if (0x8000..=0xDFFF).contains(&address) {
                self.fds_ram[(address - 0x8000) as usize] = data;
                return;
            }
        }

        if (0x6000..=0x7FFF).contains(&address) {
            self.prg_ram[(address - 0x6000) as usize] = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        address
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        _chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        _ppu_octal_latch: u8,
        _vram: &[u8],
    ) -> (u8, u16) {
        (0, ppu_address_bus)
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.irq_counter > 0 {
            self.irq_counter = self.irq_counter.saturating_sub(cycles as u32);
            if self.irq_counter == 0 {
                self.irq_counter = self.irq_reload;
                self.irq_asserted = true;
            }
        }

        if let Some(ref mut p1) = self.vrc6_pulse1 { p1.clock(); }
        if let Some(ref mut p2) = self.vrc6_pulse2 { p2.clock(); }
        if let Some(ref mut s) = self.vrc6_saw { s.clock(); }
        if let Some(ref mut vrc7) = self.vrc7 { vrc7.clock_audio(cycles); }
        if let Some(ref mut mmc5) = self.mmc5 { mmc5.cpu_clock(cycles); }
        if let Some(ref mut n163) = self.namco163 { n163.cpu_clock(cycles); }
        if let Some(ref mut fme7) = self.sunsoft5b { fme7.cpu_clock(cycles); }
        if let Some(ref mut fds) = self.fds { fds.cpu_clock(cycles); }

        self.irq_asserted
    }

    fn cpu_clock_irq_level(&self) -> bool {
        true
    }

    fn audio_sample(&self) -> f32 {
        let mut sample = 0.0f32;
        if let (Some(ref p1), Some(ref p2), Some(ref s)) = (&self.vrc6_pulse1, &self.vrc6_pulse2, &self.vrc6_saw) {
            let vrc6_sum = (p1.get_volume() + p2.get_volume() + s.get_volume()) as f32;
            sample += vrc6_sum * 5.0;
        }
        if let Some(ref vrc7) = self.vrc7 {
            sample += vrc7.get_audio_sample() * 1.0;
        }
        if let Some(ref mmc5) = self.mmc5 {
            sample += mmc5.audio_sample() * 43.0;
        }
        if let Some(ref n163) = self.namco163 {
            sample += n163.audio_sample() * 20.0;
        }
        if let Some(ref fme7) = self.sunsoft5b {
            sample += fme7.audio_sample() * 15.0;
        }
        if let Some(ref fds) = self.fds {
            sample += fds.audio_sample() * 20.0;
        }
        sample
    }

    fn expansion_audio_type(&self) -> ExpansionAudioType {
        ExpansionAudioType::Nsf
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        Vec::new()
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, _state: &[u8], start: usize) -> usize {
        start
    }
}
