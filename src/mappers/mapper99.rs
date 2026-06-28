use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const VS_FRAME_CYCLES: u64 = 29780;

pub struct Mapper99 {
    chr_bank: u8,
    vsdip: u8,
    coinon: u8,
    coinon2: u8,
    service: u8,
    cycle_accum: u64,
}

impl Mapper99 {
    pub fn new() -> Self {
        Self {
            chr_bank: 0,
            vsdip: 0,
            coinon: 0,
            coinon2: 0,
            service: 0,
            cycle_accum: 0,
        }
    }
}

impl Mapper for Mapper99 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult { data: 0, driven: false };
            }
            if len == 24576 { 
                if address < 0xA000 {
                    FetchResult { data: 0, driven: false } 
                } else {
                    let bank = ((address - 0xA000) / 0x2000) as usize;
                    let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                    FetchResult {
                        data: cart.prg_rom[offset % len],
                        driven: true,
                    }
                }
            } else if len == 49152 { 
                let bank = if address < 0xA000 {
                    (self.chr_bank as usize) * 4
                } else {
                    let rel = ((address - 0xA000) / 0x2000) as usize;
                    rel + 1
                };
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                FetchResult {
                    data: cart.prg_rom[offset % len],
                    driven: true,
                }
            } else { 
                let offset = address as usize & 0x7FFF;
                FetchResult {
                    data: cart.prg_rom[offset % len],
                    driven: true,
                }
            }
        } else if address >= 0x6000 && address < 0x8000 && !cart.prg_ram.is_empty() {
            let offset = (address as usize - 0x6000) % cart.prg_ram.len();
            FetchResult {
                data: cart.prg_ram[offset],
                driven: true,
            }
        } else if (0x4000..=0x401F).contains(&address) && (address & 0x1F) == 0x16 {
            FetchResult { data: 0, driven: false }
        } else if (0x4000..=0x401F).contains(&address) && (address & 0x1F) == 0x17 {
            FetchResult { data: 0, driven: false }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let offset = (address as usize - 0x6000) % cart.prg_ram.len();
                cart.prg_ram[offset] = data;
            }
        } else if address == 0x4016 {
            self.chr_bank = (data & 0x04) >> 2;
        }
    }

    fn adjust_controller_read(&self, address: u16, value: u8) -> u8 {
        if address & 0x1F == 0x16 {
            let mut vs = value & 0x01; 
            if self.service > 0 { vs |= 0x04; }
            vs |= (self.vsdip & 0x03) << 3;
            if self.coinon > 0 { vs |= 0x20; }
            if self.coinon2 > 0 { vs |= 0x40; }
            vs
        } else if address & 0x1F == 0x17 {
            (value & 0x01) | (self.vsdip & 0xFC)
        } else {
            value
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        address & 0x37FF
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
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            if using_chr_ram && !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[(address as usize) & (chr_ram.len() - 1)] as u16;
            } else if !chr_rom.is_empty() {
                let offset = (self.chr_bank as usize * 0x2000) + (address as usize & 0x1FFF);
                if offset < chr_rom.len() {
                    new_addr_bus |= chr_rom[offset] as u16;
                } else {
                    new_addr_bus |= 0;
                }
            }
        } else {
            let mirrored = address & 0x37FF;
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
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
            let mirrored = address & 0x37FF; 
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn insert_coin(&mut self, coin: u8) {
        match coin {
            0 => self.coinon = 6,
            1 => self.coinon2 = 6,
            _ => {}
        }
    }

    fn service_button(&mut self) {
        self.service = 6;
    }

    fn get_dip_switches(&self) -> u8 { self.vsdip }
    fn set_dip_switches(&mut self, value: u8) { self.vsdip = value; }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.cycle_accum += _cycles as u64;
        if self.cycle_accum >= VS_FRAME_CYCLES {
            self.cycle_accum = 0;
            if self.coinon > 0 { self.coinon -= 1; }
            if self.coinon2 > 0 { self.coinon2 -= 1; }
            if self.service > 0 { self.service -= 1; }
        }
        false
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        vec![self.chr_bank, self.vsdip, self.coinon, self.coinon2, self.service]
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        self.chr_bank = state[start];
        self.vsdip = state.get(start + 1).copied().unwrap_or(0);
        self.coinon = state.get(start + 2).copied().unwrap_or(0);
        self.coinon2 = state.get(start + 3).copied().unwrap_or(0);
        self.service = state.get(start + 4).copied().unwrap_or(0);
        start + 5
    }

    fn reset(&mut self) {
        self.chr_bank = 0;
        self.coinon = 0;
        self.coinon2 = 0;
        self.service = 0;
        self.cycle_accum = 0;
    }
}
