use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ChipMode {
    WaitingForCommand,
    Write,
    Erase,
}
struct FlashSST39SF040 {
    mode: ChipMode,
    cycle: u8,
    software_id: bool,
}

impl FlashSST39SF040 {
    fn new() -> Self {
        Self {
            mode: ChipMode::WaitingForCommand,
            cycle: 0,
            software_id: false,
        }
    }

    fn read(&self, addr: u32) -> Option<u8> {
        if self.software_id {
            match addr & 0x1FF {
                0x00 => Some(0xBF),
                0x01 => Some(0xB7),
                _ => Some(0xFF),
            }
        } else {
            None
        }
    }

    fn reset_state(&mut self) {
        self.mode = ChipMode::WaitingForCommand;
        self.cycle = 0;
    }

    fn write(&mut self, prg_rom: &mut [u8], addr: u32, value: u8) {
        let cmd = addr & 0x7FFF;
        if self.mode == ChipMode::WaitingForCommand {
            if self.cycle == 0 {
                if cmd == 0x5555 && value == 0xAA {
                    self.cycle += 1;
                } else if value == 0xF0 {
                    self.reset_state();
                    self.software_id = false;
                }
            } else if self.cycle == 1 && cmd == 0x2AAA && value == 0x55 {
                self.cycle += 1;
            } else if self.cycle == 2 && cmd == 0x5555 {
                self.cycle += 1;
                match value {
                    0x80 => self.mode = ChipMode::Erase,
                    0x90 => {
                        self.reset_state();
                        self.software_id = true;
                    }
                    0xA0 => self.mode = ChipMode::Write,
                    0xF0 => {
                        self.reset_state();
                        self.software_id = false;
                    }
                    _ => {}
                }
            } else {
                self.cycle = 0;
            }
        } else if self.mode == ChipMode::Write {
            if (addr as usize) < prg_rom.len() {
                prg_rom[addr as usize] &= value;
            }
            self.reset_state();
        } else if self.mode == ChipMode::Erase {
            if self.cycle == 3 {
                if cmd == 0x5555 && value == 0xAA {
                    self.cycle += 1;
                } else {
                    self.reset_state();
                }
            } else if self.cycle == 4 {
                if cmd == 0x2AAA && value == 0x55 {
                    self.cycle += 1;
                } else {
                    self.reset_state();
                }
            } else if self.cycle == 5 {
                if cmd == 0x5555 && value == 0x10 {
                    for byte in prg_rom.iter_mut() {
                        *byte = 0xFF;
                    }
                } else if value == 0x30 {
                    let offset = (addr & 0x7F000) as usize;
                    if offset + 0x1000 <= prg_rom.len() {
                        for byte in &mut prg_rom[offset..offset + 0x1000] {
                            *byte = 0xFF;
                        }
                    }
                }
                self.reset_state();
            }
        }
    }
}

pub struct Mapper30 {
    flash: FlashSST39SF040,
    enable_mirroring_bit: bool,
    prg_bank: u8,
    sub_mapper: u8,
    has_battery: bool,
    mirroring: u8, 
}

impl Mapper30 {
    pub fn new(sub_mapper: u8, has_battery: bool, header: &[u8]) -> Self {
        let mut enable_mirroring_bit = false;
        let mut mirroring = 1; 
        if sub_mapper == 3 {
            enable_mirroring_bit = true;
            mirroring = 1; 
        } else {
            let layout = header[6] & 0x09;
            match layout {
                0 => mirroring = 0, 
                1 => mirroring = 1, 
                8 => {
                    mirroring = 2; 
                    enable_mirroring_bit = true;
                }
                9 => mirroring = 4, 
                _ => {}
            }
        }
        Self {
            flash: FlashSST39SF040::new(),
            enable_mirroring_bit,
            prg_bank: 0,
            sub_mapper,
            has_battery,
            mirroring,
        }
    }
}

impl Mapper for Mapper30 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if self.has_battery {
                let flash_addr = (address as u32 & 0x3FFF) | ((self.prg_bank as u32) << 14);
                if let Some(val) = self.flash.read(flash_addr) {
                    return FetchResult {
                        data: val,
                        driven: true,
                    };
                }
            }
            let last_bank = (cart.prg_rom.len() / 0x4000) - 1;
            let bank = if address < 0xC000 {
                self.prg_bank as usize
            } else {
                last_bank
            };
            let offset = (bank * 0x4000) + (address as usize & 0x3FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            if !self.has_battery || address >= 0xC000 {
                self.prg_bank = data & 0x1F;
                if self.enable_mirroring_bit {
                    if self.sub_mapper == 3 {
                        self.mirroring = if (data & 0x80) != 0 { 1 } else { 0 };
                    } else {
                        self.mirroring = if (data & 0x80) != 0 { 3 } else { 2 };
                    }
                }
            } else {
                let flash_addr = (address as u32 & 0x3FFF) | ((self.prg_bank as u32) << 14);
                self.flash.write(&mut cart.prg_rom, flash_addr, data);
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        match self.mirroring {
            0 => {
                let nt = (address >> 10) & 1;
                (address & 0x03FF) | (nt << 10)
            }
            1 => {
                let nt = (address >> 11) & 1;
                (address & 0x03FF) | (nt << 10)
            }
            2 => {
                address & 0x03FF
            }
            3 => {
                (address & 0x03FF) | 0x0400
            }
            _ => {
                address
            }
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
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let chr_bank = ((self.prg_bank >> 5) & 0x03) as usize;
            let offset = (chr_bank * 0x2000) + (address as usize & 0x1FFF);
            if using_chr_ram {
                new_addr_bus |= chr_ram[offset & (chr_ram.len() - 1)] as u16;
            } else {
                new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(
                &Cartridge {
                    name: String::new(),
                    prg_rom: Vec::new(),
                    chr_rom: Vec::new(),
                    memory_mapper: 30,
                    sub_mapper: 0,
                    prg_size: 0,
                    chr_size: 0,
                    prg_size_minus_1: 0,
                    chr_ram: Vec::new(),
                    using_chr_ram: false,
                    prg_ram: Vec::new(),
                    has_battery: false,
                    alternative_nametable_arrangement: false,
                    prg_vram: Vec::new(),
                    nametable_horizontal_mirroring: false,
                    fds_disks: Vec::new(),
                    trainer: Vec::new(),
                    misc_rom: Vec::new(),
                    mapper_chip: Box::new(Mapper30 {
                        flash: FlashSST39SF040::new(),
                        enable_mirroring_bit: false,
                        prg_bank: 0,
                        sub_mapper: 0,
                        has_battery: false,
                        mirroring: 1,
                    }),
                    mapper_cpu_cycle: 0,
                    prg_rom_crc32: 0,
                    chr_rom_crc32: 0,
                    overall_crc32: 0,
                    is_vs_system: false,
                    tv_system: crate::region::TvSystem::Unknown,
                },
                address,
            );
            let idx = (mirrored & 0x7FF) as usize;
            new_addr_bus |= vram[idx] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram {
                let chr_bank = ((self.prg_bank >> 5) & 0x03) as usize;
                let offset = (chr_bank * 0x2000) + (address as usize & 0x1FFF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset & (len - 1)] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            let idx = (mirrored & 0x7FF) as usize;
            vram[idx] = data;
        }
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.push(self.prg_bank);
        state.push(self.mirroring);
        state.push(self.flash.cycle);
        state.push(self.flash.software_id as u8);
        state.push(match self.flash.mode {
            ChipMode::WaitingForCommand => 0,
            ChipMode::Write => 1,
            ChipMode::Erase => 2,
        });
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
        if start + 1 <= state.len() {
            self.prg_bank = state[start];
            start += 1;
        }
        if start + 1 <= state.len() {
            self.mirroring = state[start];
            start += 1;
        }
        if start + 1 <= state.len() {
            self.flash.cycle = state[start];
            start += 1;
        }
        if start + 1 <= state.len() {
            self.flash.software_id = state[start] != 0;
            start += 1;
        }
        if start + 1 <= state.len() {
            self.flash.mode = match state[start] {
                1 => ChipMode::Write,
                2 => ChipMode::Erase,
                _ => ChipMode::WaitingForCommand,
            };
            start += 1;
        }
        start
    }
}
