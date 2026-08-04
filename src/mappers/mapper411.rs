use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};
pub struct Mapper411 {
    mmc3: MapperMMC3,
    sub_mapper: u8,
    reg: [u8; 2],
    dip_switches: u8,
    irq_clear_pending: bool,
}
impl Mapper411 {
    pub fn new(submapper_id: u8, header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let mut config = Mmc3Config::for_ines(header, 0, chr_size, rom, rom_name);
        config.ax5202p = true;
        Self {
            mmc3: MapperMMC3::new(config),
            sub_mapper: submapper_id,
            reg: [0x00, 0x03],
            dip_switches: 0,
            irq_clear_pending: false,
        }
    }
    fn prg_and(&self) -> usize {
        let test = if self.sub_mapper == 2 { 0x01 } else { 0x02 };
        if (self.reg[1] & test) != 0 {
            0x1F
        } else {
            0x0F
        }
    }
    fn chr_and(&self) -> usize {
        if (self.reg[1] & 0x02) != 0 {
            0xFF
        } else {
            0x7F
        }
    }
    fn prg_or(&self) -> usize {
        (((self.reg[1] as usize) << 1) & 0x10) | (((self.reg[1] as usize) >> 1) & 0x60)
    }
    fn chr_or(&self) -> usize {
        let part = if self.sub_mapper == 1 {
            ((self.reg[1] as usize) << 2) & 0x100
        } else {
            (((self.reg[0] as usize) << 4) & 0x100) | (((self.reg[1] as usize) << 2) & 0x200)
        };
        (((self.reg[1] as usize) << 5) & 0x080) | part
    }
    fn nrom_mode(&self) -> bool {
        (self.reg[0] & 0x40) != 0
    }
    fn nrom_bank(&self) -> usize {
        (self.reg[0] as usize & 0x05) | ((self.reg[0] as usize >> 2) & 0x02) | (self.prg_or() >> 1)
    }
    fn chr_bank(&self, address: u16) -> usize {
        let raw = self.mmc3.chr_bank(address) as usize;
        (raw & self.chr_and()) | (self.chr_or() & !self.chr_and())
    }
}
impl Mapper for Mapper411 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.reg = [0x00, 0x03];
    }
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x5000..0x6000).contains(&address) {
            return FetchResult {
                data: self.dip_switches,
                driven: true,
            };
        }
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult {
                    data: 0,
                    driven: true,
                };
            }
            let offset = if self.nrom_mode() {
                let prg = self.nrom_bank();
                if (self.reg[0] & 0x02) != 0 {
                    (prg >> 1) * 0x8000 + (address as usize & 0x7FFF)
                } else {
                    prg * 0x4000 + (address as usize & 0x3FFF)
                }
            } else {
                let mode = (self.mmc3.r8000 & 0x40) != 0;
                let raw = match address {
                    0xE000..=0xFFFF => 0xFF,
                    0xC000..=0xDFFF => {
                        if mode {
                            self.mmc3.bank_8c as usize
                        } else {
                            0xFE
                        }
                    }
                    0xA000..=0xBFFF => self.mmc3.bank_a as usize,
                    _ => {
                        if mode {
                            0xFE
                        } else {
                            self.mmc3.bank_8c as usize
                        }
                    }
                };
                let bank8 = (raw & self.prg_and()) | (self.prg_or() & !self.prg_and());
                bank8 * 0x2000 + (address as usize & 0x1FFF)
            };
            FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            }
        } else if address >= 0x6000 {
            self.mmc3.fetch_prg(cart, address)
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }
    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x5000..0x6000).contains(&address) {
            if self.sub_mapper == 2 || (address & 0x800) != 0 {
                self.reg[address as usize & 1] = data;
            }
        } else if (0x6000..0x8000).contains(&address) {
            if (self.mmc3.prg_ram_protect & 0x40) == 0 && !cart.prg_ram.is_empty() {
                let off = (address - 0x6000) as usize;
                if off < cart.prg_ram.len() {
                    cart.prg_ram[off] = data;
                }
            }
        } else {
            self.mmc3.store_prg(cart, address, data);
            if (address & 0xE001) == 0xE000 {
                self.irq_clear_pending = true;
            }
        }
    }
    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_clear_pending;
        self.irq_clear_pending = false;
        ack
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
            let bank = self.chr_bank(address);
            let offset = bank * 0x400 + (address as usize & 0x3FF);
            let byte = if using_chr_ram && !chr_ram.is_empty() {
                chr_ram[offset % chr_ram.len()]
            } else if !chr_rom.is_empty() {
                chr_rom[offset % chr_rom.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
            (new_addr_bus as u8, new_addr_bus)
        } else {
            self.mmc3.fetch_ppu(
                prg_rom,
                chr_rom,
                prg_ram,
                chr_ram,
                prg_vram,
                using_chr_ram,
                nametable_horizontal_mirroring,
                alternative_nametable_arrangement,
                ppu_address_bus,
                ppu_octal_latch,
                vram,
            )
        }
    }
    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let bank = self.chr_bank(address);
                let offset = bank * 0x400 + (address as usize & 0x3FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
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
    fn get_dip_switches(&self) -> u8 {
        self.dip_switches
    }
    fn set_dip_switches(&mut self, value: u8) {
        self.dip_switches = value;
    }
    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.extend_from_slice(&self.reg);
        state.push(self.dip_switches);
        state
    }
    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut idx = self.mmc3.load_mapper_registers(cart, state, start);
        for i in 0..2 {
            if idx < state.len() {
                self.reg[i] = state[idx];
                idx += 1;
            }
        }
        if idx < state.len() {
            self.dip_switches = state[idx];
            idx += 1;
        }
        idx
    }
}
