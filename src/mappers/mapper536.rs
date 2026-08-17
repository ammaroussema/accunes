use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper536 {
    mmc3: MapperMMC3,
    reg: [u8; 2],
    irq_ack: bool,
}

impl Mapper536 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let config = Mmc3Config {
            ax5202p: true,
            ..Mmc3Config::for_ines(header, 0, 0, rom, rom_name)
        };
        Self {
            mmc3: MapperMMC3::new(config),
            reg: [0x60, 0x00],
            irq_ack: false,
        }
    }

    fn mmc3_raw_bank(&self, page: usize) -> usize {
        let mode = (self.mmc3.r8000 & 0x40) != 0;
        match (page, mode) {
            (0, false) => (self.mmc3.bank_8c & 0x0F) as usize,
            (0, true) => 0x0E,
            (1, _) => (self.mmc3.bank_a & 0x0F) as usize,
            (2, false) => 0x0E,
            (2, true) => (self.mmc3.bank_8c & 0x0F) as usize,
            (3, _) => 0x0F,
            _ => 0,
        }
    }
}

impl Mapper for Mapper536 {
    fn reset(&mut self) {
        self.reg = [0x60, 0x00];
        self.irq_ack = false;
        self.mmc3.reset();
    }

    fn reset_power_cycle(&mut self) {
        self.reg = [0x60, 0x00];
        self.irq_ack = false;
        self.mmc3.reset();
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
            let prg = (self.reg[0] as usize & 0x1F) | (((self.reg[0] as usize) >> 1) & 0x20);
            let offset = if (self.reg[0] & 0x80) != 0 {
                let page = (address as usize - 0x8000) / 0x2000;
                let bank = self.mmc3_raw_bank(page) | (prg << 2);
                bank * 0x2000 + (address as usize & 0x1FFF)
            } else if (self.reg[0] & 0x20) != 0 {
                let bank32 = prg >> 1;
                bank32 * 0x8000 + (address as usize & 0x7FFF)
            } else {
                let bank16 = prg;
                bank16 * 0x4000 + (address as usize & 0x3FFF)
            };
            return FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            };
        }
        if address >= 0x6000 {
            return FetchResult {
                data: self.mmc3.get_dip_switches(),
                driven: true,
            };
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if (0x6000..0x8000).contains(&address) {
            self.reg[(address & 1) as usize] = data;
        }
        if (address & 0xE001) == 0xE000 {
            self.irq_ack = true;
        }
        self.mmc3.store_prg(cart, address, data);
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        let mmc3_h = self.mmc3.nametable_mirroring();
        if cart.alternative_nametable_arrangement {
            address
        } else {
            mirror_h_or_v(mmc3_h, address)
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
        if address < 0x2000 {
            if using_chr_ram && !chr_ram.is_empty() {
                new_addr_bus |= chr_ram[(address as usize) % chr_ram.len()] as u16;
            }
        } else {
            let mirrored = mirror_h_or_v(self.mmc3.nametable_mirroring(), address);
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram && !cart.chr_ram.is_empty() {
            let len = cart.chr_ram.len();
            cart.chr_ram[(address as usize) % len] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = mirror_h_or_v(self.mmc3.nametable_mirroring(), address);
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
        self.mmc3
            .ppu_clock(ppu_address_bus, ppu_a12_prev, scanline, dot, ppu_sprite_x16, rendering_on)
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack;
        self.irq_ack = false;
        ack
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.mmc3.cpu_clock(cycles)
    }

    fn get_dip_switches(&self) -> u8 {
        self.mmc3.get_dip_switches()
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.mmc3.set_dip_switches(value);
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.reg[0]);
        state.push(self.reg[1]);
        state.push(if self.irq_ack { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        if p < state.len() {
            self.reg[0] = state[p];
            p += 1;
        }
        if p < state.len() {
            self.reg[1] = state[p];
            p += 1;
        }
        if p < state.len() {
            self.irq_ack = state[p] != 0;
            p += 1;
        }
        p
    }
}
