use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config, mmc3_chr_bank};

pub struct Mapper455 {
    mmc3: MapperMMC3,
    extra_val: u8,
    extra_addr: u8,
    prg_and: u8,
    chr_and: u8,
    prg_or: u8,
    chr_or: u8,
}

impl Mapper455 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let config = Mmc3Config::for_ines(header, 0, header[5], rom, rom_name);
        let mmc3 = MapperMMC3::new(config);
        Mapper455 {
            mmc3,
            extra_val: 0,
            extra_addr: 0,
            prg_and: 0x1F,
            chr_and: 0xFF,
            prg_or: 0,
            chr_or: 0,
        }
    }

    fn prg_rom_read(cart: &Cartridge, offset: usize) -> u8 {
        let len = cart.prg_rom.len();
        if len == 0 {
            0
        } else {
            cart.prg_rom[offset % len]
        }
    }

    fn sync(&mut self) {
        if (self.extra_val & 1) != 0 {
            if (self.extra_val & 2) != 0 {
                self.mmc3.bank_a = (self.extra_val >> 3) as u8;
                self.mmc3.bank_8c = ((self.extra_val >> 3) | 1) as u8;
            } else {
                self.mmc3.bank_a = (self.extra_val >> 2) as u8;
                self.mmc3.bank_8c = (self.extra_val >> 2) as u8;
            }
            self.mmc3.chr_2k0 = 0;
            self.mmc3.chr_2k8 = 0;
            self.mmc3.chr_1k0 = 0;
            self.mmc3.chr_1k4 = 0;
            self.mmc3.chr_1k8 = 0;
            self.mmc3.chr_1kc = 0;
        } else {
            self.prg_and = 0x1F;
            self.chr_and = if (self.extra_addr & 2) != 0 { 0x7F } else { 0xFF };
            self.prg_or = (self.extra_val >> 1) as u8;
            self.chr_or = (self.extra_addr << 6) as u8;
        }
    }

    fn get_prg_bank(&self, bank: u8) -> u8 {
        (bank & self.prg_and) | (self.prg_or & !self.prg_and)
    }

    fn get_chr_bank(&self, bank: u8) -> u8 {
        (bank & self.chr_and) | (self.chr_or & !self.chr_and)
    }
}

impl Mapper for Mapper455 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if (self.extra_val & 1) != 0 {
                self.mmc3.fetch_prg(cart, address)
            } else {
                let bank = if address >= 0xE000 {
                    let len = cart.prg_rom.len();
                    if len == 0 { 0 } else { ((len / 0x2000).saturating_sub(1)) as u8 }
                } else if address >= 0xC000 {
                    if (self.mmc3.r8000 & 0x40) != 0 {
                        self.get_prg_bank(self.mmc3.bank_8c)
                    } else {
                        let len = cart.prg_rom.len();
                        if len < 0x4000 { 0 } else { ((len / 0x2000).saturating_sub(2)) as u8 }
                    }
                } else if address >= 0xA000 {
                    self.get_prg_bank(self.mmc3.bank_a)
                } else {
                    if (self.mmc3.r8000 & 0x40) == 0 {
                        self.get_prg_bank(self.mmc3.bank_8c)
                    } else {
                        let len = cart.prg_rom.len();
                        if len < 0x4000 { 0 } else { ((len / 0x2000).saturating_sub(2)) as u8 }
                    }
                };
                let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
                FetchResult {
                    data: Self::prg_rom_read(cart, offset),
                    driven: true,
                }
            }
        } else if address >= 0x6000 {
            self.mmc3.fetch_prg(cart, address)
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x4000 && address <= 0x4FFF && (address & 0x100) != 0 {
            self.extra_val = data;
            self.extra_addr = (address & 0xFF) as u8;
            self.sync();
        } else if address >= 0x8000 {
            self.mmc3.store_prg(cart, address, data);
        } else if address >= 0x6000 {
            self.mmc3.store_prg(cart, address, data);
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mmc3.mirror_nametable(cart, address)
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
        if address < 0x2000 {
            let bank = if (self.extra_val & 1) != 0 {
                0
            } else {
                let chr_bank = mmc3_chr_bank(
                    self.mmc3.r8000,
                    self.get_chr_bank(self.mmc3.chr_2k0),
                    self.get_chr_bank(self.mmc3.chr_2k8),
                    self.get_chr_bank(self.mmc3.chr_1k0),
                    self.get_chr_bank(self.mmc3.chr_1k4),
                    self.get_chr_bank(self.mmc3.chr_1k8),
                    self.get_chr_bank(self.mmc3.chr_1kc),
                    address,
                );
                chr_bank
            };
            let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
            let chr_data = if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else if !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else {
                0
            };
            new_addr_bus |= chr_data as u16;
        } else if address < 0x3F00 {
            let mirrored = self.mmc3.mirror_nametable(
                &Cartridge {
                    name: String::new(),
                    prg_rom: Vec::new(),
                    chr_rom: Vec::new(),
                    memory_mapper: 0,
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
                    mapper_chip: Box::new(crate::mapper::MapperNROM::new(crate::mapper::NromConfig::default())),
                    mapper_cpu_cycle: 0,
                    prg_rom_crc32: 0,
                    chr_rom_crc32: 0,
                    overall_crc32: 0,
                    is_vs_system: false,
                    tv_system: crate::region::TvSystem::Unknown,
                },
                address,
            );
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn ppu_clock(
        &mut self,
        ppu_address_bus: u16,
        ppu_a12_prev: bool,
        scanline: u16,
        dot: u16,
        ppu_sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.mmc3.cpu_clock(cycles)
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn reset(&mut self) {
        self.extra_val = 0;
        self.extra_addr = 0;
        self.prg_and = 0x1F;
        self.chr_and = 0xFF;
        self.prg_or = 0;
        self.chr_or = 0;
        self.mmc3.reset();
        self.sync();
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.extra_val);
        state.push(self.extra_addr);
        state.push(self.prg_and);
        state.push(self.chr_and);
        state.push(self.prg_or);
        state.push(self.chr_or);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mmc3_end = self.mmc3.load_mapper_registers(cart, state, start);
        let mut pos = mmc3_end;
        if pos < state.len() {
            self.extra_val = state[pos];
            pos += 1;
        }
        if pos < state.len() {
            self.extra_addr = state[pos];
            pos += 1;
        }
        if pos < state.len() {
            self.prg_and = state[pos];
            pos += 1;
        }
        if pos < state.len() {
            self.chr_and = state[pos];
            pos += 1;
        }
        if pos < state.len() {
            self.prg_or = state[pos];
            pos += 1;
        }
        if pos < state.len() {
            self.chr_or = state[pos];
            pos += 1;
        }
        self.sync();
        pos
    }

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
    }
}
