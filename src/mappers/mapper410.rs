
use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper410 {
    mmc3: MapperMMC3,
    reg_index: u8,
    reg: [u8; 4],
    irq_clear_pending: bool,
}

impl Mapper410 {
    pub fn new(header: &[u8], rom: &[u8], rom_name: &str) -> Self {
        let chr_size = if header.len() > 5 { header[5] } else { 0 };
        let mut config = Mmc3Config::for_ines(
            header,
            0,
            if chr_size == 0 { 0 } else { chr_size },
            rom,
            rom_name,
        );
        config.ax5202p = true;
        Self {
            mmc3: MapperMMC3::new(config),
            reg_index: 0,
            reg: [0x00, 0x00, 0x0F, 0x00],
            irq_clear_pending: false,
        }
    }

    fn prg_and(&self) -> usize {
        (!self.reg[3] & 0x3F) as usize
    }

    fn prg_or(&self) -> usize {
        (self.reg[1] as usize) | (((self.reg[2] as usize) << 2) & 0x300)
    }

    fn chr_and(&self) -> usize {
        (0xFFu32 >> (0x0F - (self.reg[2] & 0x0F))) as usize
    }

    fn chr_or(&self) -> usize {
        (self.reg[0] as usize) | (((self.reg[2] as usize) << 4) & 0xF00)
    }

    fn chr_ram_selected(&self) -> bool {
        (self.reg[2] & 0x40) != 0
    }
}

impl Mapper for Mapper410 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.reg_index = 0;
        self.reg = [0x00, 0x00, 0x0F, 0x00];
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
            let bank8 = (raw & self.prg_and()) | self.prg_or();
            let offset = (bank8 * 0x2000 + (address as usize & 0x1FFF)) % len;
            FetchResult {
                data: cart.prg_rom[offset],
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
        if (0x6000..0x8000).contains(&address) {
            if (self.mmc3.prg_ram_protect & 0x40) == 0 && (self.reg[3] & 0x40) == 0 {
                self.reg[self.reg_index as usize & 3] = data;
                self.reg_index = self.reg_index.wrapping_add(1);
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
            let byte = if self.chr_ram_selected() {
                if using_chr_ram && !chr_ram.is_empty() {
                    chr_ram[address as usize & 0x1FFF]
                } else {
                    0
                }
            } else if !chr_rom.is_empty() {
                let raw = self.mmc3.chr_bank(address) as usize;
                let bank = (raw & self.chr_and()) | self.chr_or();
                let offset = (bank * 0x400 + (address as usize & 0x3FF)) % chr_rom.len();
                chr_rom[offset]
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
            if self.chr_ram_selected() && cart.using_chr_ram && !cart.chr_ram.is_empty() {
                cart.chr_ram[address as usize & 0x1FFF] = data;
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
        idx
    }
}
