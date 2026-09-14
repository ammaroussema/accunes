use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{mmc3_chr_bank, MapperMMC3, Mmc3Config};

pub struct Mapper369 {
    mmc3: MapperMMC3,
    outer_bank: u8,
    smb2j_bank: u8,
    m2_counting: bool,
    m2_counter: u16,
    irq_pending: bool,
    irq_ack_flag: bool,
}

impl Mapper369 {
    pub fn new(header: &[u8], _rom: &[u8], _rom_name: &str) -> Self {
        let chr_size = header.get(5).copied().unwrap_or(0);
        let config = Mmc3Config {
            prg_ram_size: 0x2000,
            chr_ram_size: if chr_size == 0 { 0x2000 } else { 0 },
            mmc6: false,
            ax5202p: false,
            irq_revision_b: true,
            irq_hack: crate::mappers::mmc3::Mmc3IrqHack::None,
            header_horizontal_mirror: (header.get(6).copied().unwrap_or(0) & 1) == 0,
        };
        Self {
            mmc3: MapperMMC3::new(config),
            outer_bank: 0,
            smb2j_bank: 0,
            m2_counting: false,
            m2_counter: 0,
            irq_pending: false,
            irq_ack_flag: false,
        }
    }

    fn get_raw_mmc3_prg_bank(&self, address: u16) -> u8 {
        let mode = (self.mmc3.r8000 & 0x40) != 0;
        let page = (address as usize - 0x8000) / 0x2000;
        match (page, mode) {
            (0, false) => self.mmc3.bank_8c,
            (0, true) => 0xFE,
            (1, _) => self.mmc3.bank_a,
            (2, false) => 0xFE,
            (2, true) => self.mmc3.bank_8c,
            (3, _) => 0xFF,
            _ => 0,
        }
    }
}

impl Mapper for Mapper369 {
    fn reset(&mut self) {
        self.outer_bank = 0;
        self.smb2j_bank = 0;
        self.m2_counting = false;
        self.m2_counter = 0;
        self.irq_pending = false;
        self.irq_ack_flag = false;
        self.mmc3.reset();
        self.mmc3.set_nametable_horizontal(false);
    }

    fn handle_cpu_write(&mut self, address: u16, val: u8) {
        if (0x4000..=0x4FFF).contains(&address) && (address & 0x0100) != 0 {
            self.outer_bank = val;
            if val == 0x00 || val == 0x01 {
                self.mmc3.set_nametable_horizontal(false);
            }
        }
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        let len = cart.prg_rom.len().max(1);
        if (0x6000..0x8000).contains(&address) {
            if self.outer_bank == 0x13 {
                let offset = 0x0E * 0x2000 + (address as usize & 0x1FFF);
                return FetchResult {
                    data: cart.prg_rom[offset % len],
                    driven: true,
                };
            } else if !cart.prg_ram.is_empty() {
                let offset = (address as usize & 0x1FFF) % cart.prg_ram.len();
                return FetchResult {
                    data: cart.prg_ram[offset],
                    driven: true,
                };
            } else {
                return FetchResult {
                    data: 0,
                    driven: false,
                };
            }
        }

        if address >= 0x8000 {
            let offset = match self.outer_bank {
                0x00 => address as usize & 0x7FFF,
                0x01 => 0x8000 + (address as usize & 0x7FFF),
                0x13 => {
                    let page = (address as usize - 0x8000) / 0x2000;
                    let bank = match page {
                        0 => 0x0C,
                        1 => 0x0D,
                        2 => 0x08 | (self.smb2j_bank as usize & 0x03),
                        3 => 0x0F,
                        _ => 0,
                    };
                    bank * 0x2000 + (address as usize & 0x1FFF)
                }
                0x37 => {
                    let raw = self.get_raw_mmc3_prg_bank(address);
                    let bank = ((raw & 0x0F) | 0x10) as usize;
                    bank * 0x2000 + (address as usize & 0x1FFF)
                }
                0xFF => {
                    let raw = self.get_raw_mmc3_prg_bank(address);
                    let bank = ((raw & 0x1F) | 0x20) as usize;
                    bank * 0x2000 + (address as usize & 0x1FFF)
                }
                _ => return self.mmc3.fetch_prg(cart, address),
            };
            return FetchResult {
                data: cart.prg_rom[offset % len],
                driven: true,
            };
        }

        FetchResult {
            data: 0,
            driven: false,
        }
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
            let is_ram = using_chr_ram || chr_rom.is_empty();
            let buf = if is_ram { chr_ram } else { chr_rom };
            if buf.is_empty() {
                return (0, new_addr_bus);
            }
            let offset = match self.outer_bank {
                0x00 => (address & 0x1FFF) as usize,
                0x01 => 0x2000 + (address & 0x1FFF) as usize,
                0x13 => 3 * 0x2000 + (address & 0x1FFF) as usize,
                0x37 => {
                    let mmc3_chr = mmc3_chr_bank(
                        self.mmc3.r8000,
                        self.mmc3.chr_2k0,
                        self.mmc3.chr_2k8,
                        self.mmc3.chr_1k0,
                        self.mmc3.chr_1k4,
                        self.mmc3.chr_1k8,
                        self.mmc3.chr_1kc,
                        address,
                    ) as usize;
                    let bank = (mmc3_chr & 0x7F) | 0x80;
                    bank * 0x0400 + (address as usize & 0x03FF)
                }
                0xFF => {
                    let mmc3_chr = mmc3_chr_bank(
                        self.mmc3.r8000,
                        self.mmc3.chr_2k0,
                        self.mmc3.chr_2k8,
                        self.mmc3.chr_1k0,
                        self.mmc3.chr_1k4,
                        self.mmc3.chr_1k8,
                        self.mmc3.chr_1kc,
                        address,
                    ) as usize;
                    let bank = (mmc3_chr & 0x7F) | 0x100;
                    bank * 0x0400 + (address as usize & 0x03FF)
                }
                _ => {
                    return self.mmc3.fetch_ppu(
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
                    );
                }
            };
            let byte = buf[offset % buf.len()];
            new_addr_bus |= byte as u16;
            (byte, new_addr_bus)
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
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                let offset = match self.outer_bank {
                    0x00 => (address & 0x1FFF) as usize,
                    0x01 => 0x2000 + (address & 0x1FFF) as usize,
                    0x13 => 3 * 0x2000 + (address & 0x1FFF) as usize,
                    0x37 => {
                        let mmc3_chr = mmc3_chr_bank(
                            self.mmc3.r8000,
                            self.mmc3.chr_2k0,
                            self.mmc3.chr_2k8,
                            self.mmc3.chr_1k0,
                            self.mmc3.chr_1k4,
                            self.mmc3.chr_1k8,
                            self.mmc3.chr_1kc,
                            address,
                        ) as usize;
                        let bank = (mmc3_chr & 0x7F) | 0x80;
                        bank * 0x0400 + (address as usize & 0x03FF)
                    }
                    0xFF => {
                        let mmc3_chr = mmc3_chr_bank(
                            self.mmc3.r8000,
                            self.mmc3.chr_2k0,
                            self.mmc3.chr_2k8,
                            self.mmc3.chr_1k0,
                            self.mmc3.chr_1k4,
                            self.mmc3.chr_1k8,
                            self.mmc3.chr_1kc,
                            address,
                        ) as usize;
                        let bank = (mmc3_chr & 0x7F) | 0x100;
                        bank * 0x0400 + (address as usize & 0x03FF)
                    }
                    _ => {
                        self.mmc3.store_ppu(cart, address, data, vram);
                        return;
                    }
                };
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
            }
        } else {
            self.mmc3.store_ppu(cart, address, data, vram);
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, val: u8) {
        if (0x6000..0x8000).contains(&address) {
            if self.outer_bank != 0x13 && !cart.prg_ram.is_empty() {
                let offset = (address as usize & 0x1FFF) % cart.prg_ram.len();
                cart.prg_ram[offset] = val;
            }
            return;
        }

        if address >= 0x8000 {
            if self.outer_bank == 0x13 {
                match address & 0xE000 {
                    0x8000 => {
                        self.irq_ack_flag = true;
                        self.irq_pending = false;
                        self.m2_counting = false;
                        self.mmc3.store_prg(cart, address, val);
                    }
                    0xA000 => {
                        self.m2_counting = (val & 0x02) != 0;
                        self.mmc3.store_prg(cart, address, val);
                    }
                    0xC000 => {
                        self.mmc3.store_prg(cart, address, val);
                    }
                    0xE000 => {
                        self.smb2j_bank = val;
                    }
                    _ => {}
                }
            } else {
                self.mmc3.store_prg(cart, address, val);
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mmc3.mirror_nametable(cart, address)
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
        if self.outer_bank == 0x13 {
            false
        } else {
            self.mmc3.ppu_clock(
                ppu_address_bus,
                ppu_a12_prev,
                scanline,
                dot,
                ppu_sprite_x16,
                rendering_on,
            )
        }
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        if self.outer_bank == 0x13 {
            false
        } else {
            self.mmc3.cpu_clock_rise(ppu_address_bus)
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.outer_bank == 0x13 {
            if self.m2_counting {
                self.m2_counter = self.m2_counter.wrapping_add(1);
                if (self.m2_counter & 0x0FFF) == 0 {
                    self.irq_pending = true;
                }
            }
            self.irq_pending
        } else {
            self.mmc3.cpu_clock(_cycles)
        }
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.outer_bank == 0x13 {
            let ack = self.irq_ack_flag || self.irq_pending;
            self.irq_ack_flag = false;
            self.irq_pending = false;
            ack
        } else {
            self.mmc3.take_irq_ack()
        }
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.push(self.outer_bank);
        state.push(self.smb2j_bank);
        state.push(if self.m2_counting { 1 } else { 0 });
        state.extend_from_slice(&self.m2_counter.to_le_bytes());
        state.push(if self.irq_pending { 1 } else { 0 });
        state.push(if self.irq_ack_flag { 1 } else { 0 });
        state
    }

    fn load_mapper_registers(
        &mut self,
        cart: &mut Cartridge,
        state: &[u8],
        start: usize,
    ) -> usize {
        let mut p = self.mmc3.load_mapper_registers(cart, state, start);
        if p < state.len() {
            self.outer_bank = state[p];
            p += 1;
        }
        if p < state.len() {
            self.smb2j_bank = state[p];
            p += 1;
        }
        if p < state.len() {
            self.m2_counting = state[p] != 0;
            p += 1;
        }
        if p + 1 < state.len() {
            self.m2_counter = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.irq_pending = state[p] != 0;
            p += 1;
        }
        if p < state.len() {
            self.irq_ack_flag = state[p] != 0;
            p += 1;
        }
        p
    }
}
