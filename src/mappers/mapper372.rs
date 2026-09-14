
use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper372 {
    mmc3: MapperMMC3,
    reg_index: u8,
    reg: [u8; 4],
}

impl Mapper372 {
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
            reg: [0x00, 0x00, 0x0F, 0x00],
        }
    }

    #[inline]
    fn locked(&self) -> bool {
        (self.reg[3] & 0x40) != 0
    }

    #[inline]
    fn chr_ram_mode(&self) -> bool {
        (self.reg[2] & 0x20) != 0
    }

    fn remap_prg(&self, raw: usize) -> usize {
        let prg_and = (!self.reg[3] & 0x3F) as usize;
        let prg_or  = (self.reg[1] as usize) | (((self.reg[2] as usize) << 2) & 0x300);
        (raw & prg_and) | (prg_or & !prg_and)
    }

    fn remap_chr(&self, raw: usize) -> usize {
        let shift    = (!self.reg[2] & 0x0F) as u32;
        let chr_and  = (0xFF_u32 >> shift) as usize;
        let chr_or   = (self.reg[0] as usize) | (((self.reg[2] as usize) << 4) & 0xF00);
        (raw & chr_and) | (chr_or & !chr_and)
    }

    fn prg_rom_read(cart: &Cartridge, bank_8k: usize, offset_in_bank: usize) -> u8 {
        let len = cart.prg_rom.len();
        if len == 0 { return 0; }
        cart.prg_rom[(bank_8k * 0x2000 + offset_in_bank) % len]
    }

    fn chr_read_byte(&self, address: u16, chr_rom: &[u8], chr_ram: &[u8]) -> u8 {
        if self.chr_ram_mode() {
            if chr_ram.is_empty() { return 0; }
            chr_ram[address as usize & (chr_ram.len() - 1)]
        } else {
            let raw  = self.mmc3.chr_bank(address) as usize;
            let bank = self.remap_chr(raw);
            let offset = bank * 0x400 + (address as usize & 0x3FF);
            if chr_rom.is_empty() { return 0; }
            chr_rom[offset % chr_rom.len()]
        }
    }
}

impl Mapper for Mapper372 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.reg_index = 0;
        self.reg = [0x00, 0x00, 0x0F, 0x00];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let num_8k = len / 0x2000;
            let invert = (self.mmc3.r8000 & 0x40) != 0;
            let offset_in_bank = address as usize & 0x1FFF;
            let bank = match address {
                0xE000..=0xFFFF => {
                    self.remap_prg(num_8k.saturating_sub(1)) % num_8k
                }
                0xC000..=0xDFFF => {
                    if invert {
                        self.remap_prg(self.mmc3.bank_8c as usize) % num_8k
                    } else {
                        self.remap_prg(num_8k.saturating_sub(2)) % num_8k
                    }
                }
                0xA000..=0xBFFF => {
                    self.remap_prg(self.mmc3.bank_a as usize) % num_8k
                }
                _ => {
                    if invert {
                        self.remap_prg(num_8k.saturating_sub(2)) % num_8k
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
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if !self.locked() {
                if (address & 1) == 1 {
                    self.reg_index = 0;
                    self.reg = [0x00, 0x00, 0x0F, 0x00];
                } else {
                    self.reg[self.reg_index as usize & 3] = data;
                    self.reg_index = self.reg_index.wrapping_add(1) & 3;
                }
            }
        } else {
            self.mmc3.store_prg(cart, address, data);
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
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = self.chr_read_byte(address, chr_rom, chr_ram);
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
            if self.chr_ram_mode() {
                if !cart.chr_ram.is_empty() {
                    let idx = address as usize & (cart.chr_ram.len() - 1);
                    cart.chr_ram[idx] = data;
                }
            } else if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let raw  = self.mmc3.chr_bank(address) as usize;
                let bank = self.remap_chr(raw);
                let len  = cart.chr_ram.len();
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

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        self.mmc3.cpu_clock(cycles)
    }

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.reg_index);
        state.extend_from_slice(&self.reg);
        state
    }

    fn load_mapper_registers(
        &mut self,
        cart: &mut Cartridge,
        state: &[u8],
        start: usize,
    ) -> usize {
        let mut idx = self.mmc3.load_mapper_registers(cart, state, start);
        if idx < state.len() {
            self.reg_index = state[idx];
            idx += 1;
        }
        if idx + 4 <= state.len() {
            self.reg.copy_from_slice(&state[idx..idx + 4]);
            idx += 4;
        }
        idx
    }
}
