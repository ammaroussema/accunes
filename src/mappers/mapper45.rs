use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper45 {
    mmc3: MapperMMC3,
    reg_index: u8,
    reg: [u8; 4],
    dip_switches: u8,
}

impl Mapper45 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let using_chr_ram = chr_size == 0;
        let config = Mmc3Config::for_ines(
            header,
            0,
            if using_chr_ram { 0 } else { chr_size },
            rom,
            rom_name,
        );
        let mut mmc3 = MapperMMC3::new(config);
        mmc3.chr_2k0 = 0;
        mmc3.chr_2k8 = 2;
        mmc3.chr_1k0 = 4;
        mmc3.chr_1k4 = 5;
        mmc3.chr_1k8 = 6;
        mmc3.chr_1kc = 7;
        Self {
            mmc3,
            reg_index: 0,
            reg: [0, 0, 0x0F, 0],
            dip_switches: 0,
        }
    }

    fn remap_prg(&self, raw: usize) -> usize {
        let mut page = raw;
        page &= (0x3F ^ (self.reg[3] & 0x3F)) as usize;
        page |= self.reg[1] as usize;
        page
    }

    fn remap_chr(&self, raw: usize, using_chr_ram: bool) -> usize {
        if !using_chr_ram {
            let mut page = raw;
            let shift = 0x0F - (self.reg[2] & 0x0F);
            let mask = (0xFF_u32 >> shift) as usize;
            page &= mask;
            page |= (self.reg[0] as usize) | (((self.reg[2] & 0xF0) as usize) << 4);
            page
        } else {
            raw
        }
    }

    fn prg_rom_read(cart: &Cartridge, bank_8k: usize, offset_in_bank: usize) -> u8 {
        let len = cart.prg_rom.len();
        if len == 0 {
            return 0;
        }
        let offset = (bank_8k * 0x2000 + offset_in_bank) % len;
        cart.prg_rom[offset]
    }

    fn chr_read_byte(&self, address: u16, chr_rom: &[u8], chr_ram: &[u8], using_chr_ram: bool) -> u8 {
        let raw = self.mmc3.chr_bank(address) as usize;
        let bank = self.remap_chr(raw, using_chr_ram);
        let offset_in_bank = address as usize & 0x03FF;
        if using_chr_ram && !chr_ram.is_empty() {
            chr_ram[(bank * 0x400 + offset_in_bank) % chr_ram.len()]
        } else if !chr_rom.is_empty() {
            chr_rom[(bank * 0x400 + offset_in_bank) % chr_rom.len()]
        } else {
            0
        }
    }
}

impl Mapper45 {
    fn dip_ob(&self) -> bool {
        let d = self.dip_switches;
        d != 0 && (d == 1 && self.reg[1] & 0x80 != 0
                || d == 2 && self.reg[2] & 0x40 != 0
                || d == 3 && self.reg[1] & 0x40 != 0
                || d == 4 && self.reg[2] & 0x20 != 0)
    }
}

impl Mapper for Mapper45 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.reg_index = 0;
        self.reg = [0; 4];
        self.reg[2] = 0x0F;
        self.dip_switches = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if self.dip_ob() {
                return FetchResult { data: 0, driven: false };
            }
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let num_8k = len / 0x2000;
            let invert = (self.mmc3.r8000 & 0x40) != 0;
            let offset_in_bank = address as usize & 0x1FFF;
            let bank = match address {
                0xE000..=0xFFFF => {
                    let raw_fixed = num_8k.saturating_sub(1);
                    self.remap_prg(raw_fixed) % num_8k
                }
                0xC000..=0xDFFF => {
                    if invert {
                        self.remap_prg(self.mmc3.bank_8c as usize) % num_8k
                    } else {
                        let raw_fixed = num_8k.saturating_sub(2);
                        self.remap_prg(raw_fixed) % num_8k
                    }
                }
                0xA000..=0xBFFF => {
                    self.remap_prg(self.mmc3.bank_a as usize) % num_8k
                }
                _ => {
                    if invert {
                        let raw_fixed = num_8k.saturating_sub(2);
                        self.remap_prg(raw_fixed) % num_8k
                    } else {
                        self.remap_prg(self.mmc3.bank_8c as usize) % num_8k
                    }
                }
            };
            FetchResult {
                data: Self::prg_rom_read(cart, bank, offset_in_bank),
                driven: true,
            }
        } else if address >= 0x6000 {
            self.mmc3.fetch_prg(cart, address)
        } else if address >= 0x5000 {
            let a = address & 0xFFF;
            let result = if a & 0xF00 != 0 || (a as u8) & !self.dip_switches != 0 { 1 } else { 0 };
            FetchResult { data: result, driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if (self.reg[3] & 0x40) != 0 {
                self.mmc3.store_prg(cart, address, data);
            } else if (address & 1) == 1 {
                self.reg_index = 0;
                self.reg = [0; 4];
                self.reg[2] = 0x0F;
            } else {
                self.reg[self.reg_index as usize] = data;
                self.reg_index = (self.reg_index + 1) & 0x03;
            }
        } else {
            self.mmc3.store_prg(cart, address, data);
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mmc3.mirror_nametable(cart, address)
    }

    fn fetch_ppu(
        &mut self,
        prg_rom: &[u8],
        chr_rom: &[u8],
        prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = self.chr_read_byte(address, chr_rom, chr_ram, using_chr_ram);
            new_addr_bus |= byte as u16;
            (new_addr_bus as u8, new_addr_bus)
        } else {
            self.mmc3.fetch_ppu(
                prg_rom, chr_rom, prg_ram, chr_ram, prg_vram,
                using_chr_ram, nametable_horizontal_mirroring,
                alternative_nametable_arrangement,
                ppu_address_bus, ppu_octal_latch, vram,
            )
        }
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let raw = self.mmc3.chr_bank(address) as usize;
                let bank = self.remap_chr(raw, cart.using_chr_ram);
                let len = cart.chr_ram.len();
                let offset = (bank * 0x400 + (address as usize & 0x3FF)) % len;
                cart.chr_ram[offset] = data;
            }
        } else {
            self.mmc3.store_ppu(cart, address, data, vram);
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
        self.mmc3.ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.extend_from_slice(&self.reg);
        state.push(self.reg_index);
        state.push(self.dip_switches);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut idx = self.mmc3.load_mapper_registers(cart, state, start);
        if idx + 4 <= state.len() {
            self.reg.copy_from_slice(&state[idx..idx + 4]);
            idx += 4;
        }
        if idx < state.len() {
            self.reg_index = state[idx];
            idx += 1;
        }
        if idx < state.len() {
            self.dip_switches = state[idx];
            idx += 1;
        }
        idx
    }
}
