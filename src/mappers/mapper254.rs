use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper254 {
    mmc3: MapperMMC3,
    ex_regs: [u8; 2],
}

impl Mapper254 {
    pub fn new() -> Self {
        Self { mmc3: MapperMMC3::new(Mmc3Config::embedded()), ex_regs: [0; 2] }
    }
}

impl Mapper for Mapper254 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.ex_regs = [0; 2];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            let off = (address - 0x6000) as usize;
            if off < cart.prg_ram.len() {
                let mut val = cart.prg_ram[off];
                if self.ex_regs[0] == 0 {
                    val ^= self.ex_regs[1];
                }
                return FetchResult { data: val, driven: true };
            }
            return FetchResult { data: 0, driven: false };
        }
        self.mmc3.fetch_prg(cart, address)
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            self.mmc3.store_prg(cart, address, data);
            return;
        }
        match address {
            0x8000 => {
                self.ex_regs[0] = 0xFF;
                self.mmc3.store_prg(cart, address, data);
            }
            0xA001 => {
                self.ex_regs[1] = data;
                self.mmc3.store_prg(cart, address, data);
            }
            _ => self.mmc3.store_prg(cart, address, data),
        }
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
        self.mmc3.fetch_ppu(
            prg_rom, chr_rom, prg_ram, chr_ram, prg_vram,
            using_chr_ram, nametable_horizontal_mirroring,
            alternative_nametable_arrangement, ppu_address_bus,
            ppu_octal_latch, vram,
        )
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        self.mmc3.store_ppu(cart, address, data, vram);
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

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.extend_from_slice(&self.ex_regs);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let end = self.mmc3.load_mapper_registers(cart, state, start);
        self.ex_regs[0] = state.get(end).copied().unwrap_or(0);
        self.ex_regs[1] = state.get(end + 1).copied().unwrap_or(0);
        end + 2
    }
}
