use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};

pub struct Mapper134 {
    mmc3: MapperMMC3,
    reg: [u8; 4],
}

impl Mapper134 {
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
        Self { mmc3: MapperMMC3::new(config), reg: [0; 4] }
    }

    fn prg_and(&self) -> u8 { if (self.reg[1] & 0x04) != 0 { 0x0F } else { 0x1F } }
    fn chr_and(&self) -> u8 { if (self.reg[1] & 0x40) != 0 { 0x7F } else { 0xFF } }
    fn prg_or(&self) -> u8 { ((self.reg[1] << 4) & 0x30) | ((self.reg[0] << 2) & 0x40) }
    fn chr_or(&self) -> u16 { ((self.reg[1] as u16) << 3 & 0x180) | ((self.reg[0] as u16) << 4 & 0x200) }

    fn get_prg_bank(&self, slot: usize) -> (u8, u8) {
        let bank_mask = if (self.reg[1] & 0x80) != 0 { 0 } else { 3 };
        let addr_mask = if (self.reg[1] & 0x80) == 0 {
            0
        } else if (self.reg[1] & 0x08) != 0 {
            1
        } else {
            3
        };
        let slot_u8 = slot as u8;
        (slot_u8 & bank_mask, addr_mask)
    }

    fn mmc3_raw_prg_bank(&self, slot: usize, last: u8, second_last: u8) -> u8 {
        match slot {
            0 => {
                if (self.mmc3.r8000 & 0x40) == 0 { self.mmc3.bank_8c } else { second_last }
            }
            1 => self.mmc3.bank_a,
            2 => {
                if (self.mmc3.r8000 & 0x40) != 0 { self.mmc3.bank_8c } else { second_last }
            }
            _ => last,
        }
    }

    fn prg_final_bank(&self, cart: &Cartridge, slot: usize) -> usize {
        let num_8k = cart.prg_rom.len() / 0x2000;
        let last = num_8k.saturating_sub(1) as u8;
        let second_last = num_8k.saturating_sub(2) as u8;
        let (masked_slot, addr_mask) = self.get_prg_bank(slot);
        let mmc3_bank = self.mmc3_raw_prg_bank(masked_slot as usize, last, second_last);
        let bank = (mmc3_bank & !addr_mask) | (slot as u8 & addr_mask);
        let and = self.prg_and();
        let or_val = self.prg_or();
        ((bank & and) | or_val) as usize % num_8k
    }
}

impl Mapper for Mapper134 {
    fn reset(&mut self) {
        self.mmc3.reset();
        self.reg = [0; 4];
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let len = cart.prg_rom.len();
            if len == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let slot = ((address - 0x8000) / 0x2000) as usize;
            let bank = self.prg_final_bank(cart, slot);
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult { data: cart.prg_rom[offset % len], driven: true }
        } else if address >= 0x6000 {
            if (self.reg[0] & 0x40) != 0 {
                FetchResult { data: 0, driven: true }
            } else {
                self.mmc3.fetch_prg(cart, address)
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            let idx = (address & 3) as usize;
            if (self.reg[0] & 0x80) == 0 {
                self.reg[idx] = data;
            } else if idx == 2 {
                self.reg[2] = (self.reg[2] & !3) | (data & 3);
            }
        } else if address >= 0x8000 {
            self.mmc3.store_prg(cart, address, data);
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        self.mmc3.mirror_nametable(cart, address)
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
        alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = if (self.reg[0] & 0x08) != 0 {
                let chr_and = self.chr_and();
                let chr_or = self.chr_or();
                let bank = ((self.reg[2] as usize) & (chr_and as usize >> 3)) | (chr_or as usize >> 3);
                let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                if using_chr_ram && !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else if !chr_rom.is_empty() {
                    chr_rom[offset % chr_rom.len()]
                } else {
                    0
                }
            } else {
                let raw = self.mmc3.chr_bank(address);
                let chr_and = self.chr_and();
                let chr_or = self.chr_or();
                let bank = ((raw & chr_and) as u16 | chr_or) as usize;
                let offset = bank * 0x0400 + (address as usize & 0x03FF);
                if using_chr_ram && !chr_ram.is_empty() {
                    chr_ram[offset % chr_ram.len()]
                } else if !chr_rom.is_empty() {
                    chr_rom[offset % chr_rom.len()]
                } else {
                    0
                }
            };
            new_addr_bus |= byte as u16;
        } else if address < 0x3F00 {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if self.mmc3.nametable_mirroring() {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                if (self.reg[0] & 0x08) != 0 {
                    let chr_and = self.chr_and();
                    let chr_or = self.chr_or();
                    let bank = ((self.reg[2] as usize) & (chr_and as usize >> 3)) | (chr_or as usize >> 3);
                    let len = cart.chr_ram.len();
                    let offset = (bank * 0x2000 + (address as usize & 0x1FFF)) % len;
                    cart.chr_ram[offset] = data;
                } else {
                    let raw = self.mmc3.chr_bank(address);
                    let chr_and = self.chr_and();
                    let chr_or = self.chr_or();
                    let bank = ((raw & chr_and) as u16 | chr_or) as usize;
                    let len = cart.chr_ram.len();
                    let offset = (bank * 0x0400 + (address as usize & 0x03FF)) % len;
                    cart.chr_ram[offset] = data;
                }
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            if cart.alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < cart.prg_vram.len() { cart.prg_vram[idx] = data; }
            } else {
                vram[(mirrored & 0x7FF) as usize] = data;
            }
        }
    }

    fn cpu_clock_rise(&mut self, ppu_address_bus: u16) -> bool {
        self.mmc3.cpu_clock_rise(ppu_address_bus)
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

    fn take_irq_ack(&mut self) -> bool {
        self.mmc3.take_irq_ack()
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = self.mmc3.save_mapper_registers(cart);
        state.extend_from_slice(&self.reg);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut idx = self.mmc3.load_mapper_registers(cart, state, start);
        for i in 0..4 {
            if idx < state.len() { self.reg[i] = state[idx]; idx += 1; }
        }
        idx
    }
}
