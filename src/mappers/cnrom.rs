use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

#[derive(Clone, Debug)]
pub struct CnromConfig {
    pub prg_ram_size: usize,
    pub bus_conflict: bool,
}

impl CnromConfig {
    pub fn for_ines(header: &[u8], sub_mapper: u8) -> Self {
        let nes2 = is_nes20(header);
        let prg_ram_size = if nes2 {
            let volatile_kb = nes20_ram_kb(header[10] & 0x0F);
            let battery_kb = nes20_ram_kb((header[10] >> 4) & 0x0F);
            (volatile_kb + battery_kb) * 1024
        } else {
            let units = header[8];
            if units == 0 {
                0
            } else {
                (units as usize) * 0x2000
            }
        };
        let bus_conflict = nes2 && sub_mapper == 2;
        Self {
            prg_ram_size,
            bus_conflict,
        }
    }
}

fn is_nes20(header: &[u8]) -> bool {
    header.len() >= 16 && (header[7] & 0x0C) == 0x08
}

fn nes20_ram_kb(shift: u8) -> usize {
    if shift == 0 {
        0
    } else {
        (64usize << shift) / 1024
    }
}
pub(crate) fn read_prg_fixed(cart: &Cartridge, address: u16) -> u8 {
    let len = cart.prg_rom.len();
    if len == 0 || address < 0x8000 {
        return 0;
    }
    if len.is_power_of_two() {
        cart.prg_rom[(address as usize) & (len - 1)]
    } else {
        cart.prg_rom[(address as usize - 0x8000) % len]
    }
}
pub(crate) fn mirror_address(
    alternative_nametable: bool,
    horizontal_mirroring: bool,
    address: u16,
) -> u16 {
    if alternative_nametable {
        address
    } else if horizontal_mirroring {
        (address & 0x33FF) | ((address & 0x0800) >> 1)
    } else {
        address & 0x37FF
    }
}

fn prg_ram_index(config: &CnromConfig, address: u16) -> Option<usize> {
    if config.prg_ram_size == 0 || address < 0x6000 || address >= 0x8000 {
        return None;
    }
    let off = (address - 0x6000) as usize;
    if off >= config.prg_ram_size {
        return None;
    }
    Some(off)
}

fn chr_rom_offset(chr_bank: u8, address: u16, chr_len: usize) -> usize {
    if chr_len == 0 {
        return 0;
    }
    let offset = (chr_bank as usize) * 0x2000 + (address as usize & 0x1FFF);
    offset % chr_len
}

pub struct MapperCNROM {
    config: CnromConfig,
    chr_bank_select: u8,
}

impl MapperCNROM {
    pub fn new(config: CnromConfig) -> Self {
        Self {
            config,
            chr_bank_select: 0,
        }
    }
}

impl Mapper for MapperCNROM {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            FetchResult {
                data: read_prg_fixed(cart, address),
                driven: true,
            }
        } else if let Some(idx) = prg_ram_index(&self.config, address) {
            FetchResult {
                data: cart.prg_ram[idx],
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
        if let Some(idx) = prg_ram_index(&self.config, address) {
            cart.prg_ram[idx] = data;
        } else if address >= 0x8000 {
            self.chr_bank_select = if self.config.bus_conflict {
                data & read_prg_fixed(cart, address)
            } else {
                data
            };
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        mirror_address(
            cart.alternative_nametable_arrangement,
            cart.nametable_horizontal_mirroring,
            address,
        )
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
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
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            if using_chr_ram && !chr_ram.is_empty() {
                let mask = chr_ram.len() - 1;
                new_addr_bus |= chr_ram[(address as usize) & mask] as u16;
            } else if !chr_rom.is_empty() {
                let offset = chr_rom_offset(self.chr_bank_select, address, chr_rom.len());
                new_addr_bus |= chr_rom[offset] as u16;
            }
        } else {
            let mirrored = mirror_address(
                alternative_nametable_arrangement,
                nametable_horizontal_mirroring,
                address,
            );
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() {
                    prg_vram[idx]
                } else {
                    0
                }
            } else {
                vram[(mirrored & 0x7FF) as usize]
            };
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let mask = cart.chr_ram.len() - 1;
                cart.chr_ram[(address as usize) & mask] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() {
                    cart.prg_vram[idx] = data;
                }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
            }
        }
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.push(self.chr_bank_select);
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
        self.chr_bank_select = state[p];
        p + 1
    }

    fn reset(&mut self) {
        self.chr_bank_select = 0;
    }
}
