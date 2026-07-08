use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{mmc3_chr_bank, MapperMMC3, Mmc3Config};

pub struct Mapper287 {
    mmc3: MapperMMC3,
    ext_reg: u8,
    dip_switches: u8,
}

impl Mapper287 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let config = Mmc3Config {
            ax5202p: true,
            ..Mmc3Config::for_ines(
                header,
                0,
                if chr_size == 0 { 0 } else { chr_size },
                rom,
                rom_name,
            )
        };
        Self { mmc3: MapperMMC3::new(config), ext_reg: 0, dip_switches: 0 }
    }
}

impl Mapper for Mapper287 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.ext_reg = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();

            if (self.ext_reg & 0x04) != 0 && len < 1024 * 1024 && self.dip_switches != 0 {
                return FetchResult { data: 0, driven: false };
            }

            if (self.ext_reg & 0x08) != 0 {
                let bank32 =
                    ((self.ext_reg as usize >> 4) & 3) | ((self.ext_reg as usize) << 2);
                let offset = bank32 * 0x8000 + (address as usize & 0x7FFF);
                return FetchResult {
                    data: if offset < len { cart.prg_rom[offset] } else { 0 },
                    driven: true,
                };
            }

            let num_8k = len / 0x2000;
            let invert = (self.mmc3.r8000 & 0x40) != 0;
            let raw_bank = match address {
                0xE000..=0xFFFF => num_8k.saturating_sub(1),
                0xC000..=0xDFFF => {
                    if invert { self.mmc3.bank_8c as usize }
                    else { num_8k.saturating_sub(2) }
                }
                0xA000..=0xBFFF => self.mmc3.bank_a as usize,
                0x8000..=0x9FFF => {
                    if invert { num_8k.saturating_sub(2) }
                    else { self.mmc3.bank_8c as usize }
                }
                _ => 0,
            };

            let bank = (raw_bank & 0x0F) | ((self.ext_reg as usize & 0x0F) << 4);
            let num_banks = if num_8k > 0 { num_8k } else { 1 };
            let bank = bank % num_banks;

            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: if offset < len { cart.prg_rom[offset] } else { 0 },
                driven: true,
            }
        } else {
            self.mmc3.fetch_prg(cart, address)
        }
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address <= 0x5FFF {
            self.ext_reg = (address & 0xFF) as u8;
        } else {
            self.mmc3.store_prg(cart, address, data);
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        if cart.alternative_nametable_arrangement {
            address
        } else {
            let horz = self.mmc3.nametable_mirroring();
            if horz {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
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
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let raw = mmc3_chr_bank(
                self.mmc3.r8000,
                self.mmc3.chr_2k0,
                self.mmc3.chr_2k8,
                self.mmc3.chr_1k0,
                self.mmc3.chr_1k4,
                self.mmc3.chr_1k8,
                self.mmc3.chr_1kc,
                address,
            );
            let bank = (raw as u16 & 0x7F) | ((self.ext_reg as u16 & 0x7F) << 7);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                let offset = (bank as usize) * 0x0400 + (address as usize & 0x03FF);
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else {
                let horz = self.mmc3.nametable_mirroring();
                if horz {
                    (address & 0x33FF) | ((address & 0x0800) >> 1)
                } else {
                    address & 0x37FF
                }
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let raw = mmc3_chr_bank(
                    self.mmc3.r8000,
                    self.mmc3.chr_2k0,
                    self.mmc3.chr_2k8,
                    self.mmc3.chr_1k0,
                    self.mmc3.chr_1k4,
                    self.mmc3.chr_1k8,
                    self.mmc3.chr_1kc,
                    address,
                );
                let bank = (raw as u16 & 0x7F) | ((self.ext_reg as u16 & 0x7F) << 7);
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
        scanline: u16,
        dot: u16,
        ppu_sprite_x16: bool,
        rendering_on: bool,
    ) -> bool {
        self.mmc3.ppu_clock(
            ppu_address_bus,
            ppu_a12_prev,
            scanline,
            dot,
            ppu_sprite_x16,
            rendering_on,
        )
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.mmc3.cpu_clock(cycles)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.ext_reg);
        state.push(self.dip_switches);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        if p < state.len() {
            self.ext_reg = state[p];
            p += 1;
        }
        if p < state.len() {
            self.dip_switches = state[p];
            p += 1;
        }
        p
    }
}
