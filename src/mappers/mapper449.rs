use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

// Mapper 449 "22-in-1 King Series": address-latch multicart + DIP switch reads.
// See rf/Furbtendulator-main/src/src-mappers/src/iNES/multicart address latch/mapper449.cpp
pub struct Mapper449 {
    latch_addr: u16,
    latch_data: u8,
    dip_switches: u8,
}

impl Mapper449 {
    pub fn new() -> Self {
        Self {
            latch_addr: 0,
            latch_data: 0,
            dip_switches: 0,
        }
    }

    fn cpu_a14(&self) -> bool {
        self.latch_addr & 0x001 != 0
    }

    fn mirror_h(&self) -> bool {
        self.latch_addr & 0x002 != 0
    }

    fn nrom(&self) -> bool {
        self.latch_addr & 0x080 != 0
    }

    fn dip(&self) -> bool {
        self.latch_addr & 0x200 != 0 && self.dip_switches != 0
    }

    fn prg_bank(&self) -> u16 {
        ((self.latch_addr >> 2) & 0x1F) | ((self.latch_addr >> 3) & 0x20)
    }

    fn prg_bank16(&self, slot: u8) -> u16 {
        let prg = self.prg_bank();
        let a14nrom = if self.cpu_a14() && self.nrom() { 1 } else { 0 };
        if slot == 0 {
            prg & !a14nrom
        } else {
            let nrom_fix = if self.nrom() { 0 } else { 7 };
            prg | a14nrom | nrom_fix
        }
    }

    fn mirror(&self, address: u16) -> u16 {
        if self.mirror_h() {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
        }
    }

    fn read_wram(&self, cart: &Cartridge, address: u16) -> FetchResult {
        if cart.prg_ram.is_empty() {
            return FetchResult {
                data: 0,
                driven: false,
            };
        }
        let idx = (address as usize & 0x1FFF) % cart.prg_ram.len();
        FetchResult {
            data: cart.prg_ram[idx],
            driven: true,
        }
    }

    fn write_wram(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if cart.prg_ram.is_empty() {
            return;
        }
        let idx = (address as usize & 0x1FFF) % cart.prg_ram.len();
        cart.prg_ram[idx] = data;
    }
}

impl Mapper for Mapper449 {
    fn reset(&mut self) {
        *self = Self::new();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            let slot = ((address - 0x8000) >> 13) as usize;
            let bank16 = self.prg_bank16(if slot < 2 { 0 } else { 1 });
            let sub_offset = if self.dip() {
                (address & 0x1FFF) | (self.dip_switches as u16)
            } else {
                address & 0x1FFF
            };
            let offset =
                (bank16 as usize) * 0x4000 + (slot & 1) * 0x2000 + (sub_offset & 0x1FFF) as usize;
            return FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            };
        }
        if address >= 0x6000 {
            return self.read_wram(cart, address);
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) {
            self.write_wram(cart, address, data);
        } else if address >= 0x8000 {
            self.latch_data = data;
            self.latch_addr = address;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror(address)
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
        if address >= 0x2000 {
            let mirrored = self.mirror(address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
            return (new_addr_bus as u8, new_addr_bus);
        }
        let offset = (self.latch_data as usize) * 0x2000 + (address as usize & 0x1FFF);
        let byte = if using_chr_ram && !chr_ram.is_empty() {
            chr_ram[(address as usize & 0x1FFF) % chr_ram.len()]
        } else if !chr_rom.is_empty() {
            chr_rom[offset % chr_rom.len()]
        } else {
            0
        };
        new_addr_bus |= byte as u16;
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror(address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.latch_addr.to_le_bytes());
        state.push(self.latch_data);
        state.push(self.dip_switches);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 1 < state.len() {
            self.latch_addr = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.latch_data = state[p];
            p += 1;
        }
        if p < state.len() {
            self.dip_switches = state[p];
            p += 1;
        }
        p
    }
}
