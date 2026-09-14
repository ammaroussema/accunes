use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper56 {
    irq_reload_value: u16,
    irq_counter: u16,
    irq_control: u8,
    selected_reg: u8,
    prg_regs: [u8; 4],
    use_rom: bool,
    chr_banks: [u8; 8],
    irq_pending: bool,
    mirroring_vertical: bool,
}

impl Mapper56 {
    pub fn new() -> Self {
        Self {
            irq_reload_value: 0,
            irq_counter: 0,
            irq_control: 0,
            selected_reg: 0,
            prg_regs: [0; 4],
            use_rom: false,
            chr_banks: [0; 8],
            irq_pending: false,
            mirroring_vertical: false,
        }
    }
}

impl Mapper for Mapper56 {
    fn reset(&mut self) {
        self.irq_reload_value = 0;
        self.irq_counter = 0;
        self.irq_control = 0;
        self.selected_reg = 0;
        self.prg_regs = [0; 4];
        self.use_rom = false;
        self.chr_banks = [0; 8];
        self.irq_pending = false;
        self.mirroring_vertical = false;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        let num_8k_banks = cart.prg_rom.len() / 0x2000;
        if num_8k_banks == 0 {
            return FetchResult { data: 0, driven: address >= 0x6000 };
        }
        if address >= 0x8000 {
            let slot = ((address as usize - 0x8000) >> 13) & 3; 
            let bank = if slot == 3 {
                num_8k_banks - 1 
            } else {
                self.prg_regs[slot] as usize % num_8k_banks
            };
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if address >= 0x6000 {
            let offset = (address as usize - 0x6000) & 0x1FFF;
            if self.use_rom {
                let bank = self.prg_regs[3] as usize % num_8k_banks;
                let rom_offset = bank * 0x2000 + offset;
                FetchResult {
                    data: cart.prg_rom[rom_offset % cart.prg_rom.len()],
                    driven: true,
                }
            } else if !cart.prg_ram.is_empty() {
                FetchResult {
                    data: cart.prg_ram[offset % cart.prg_ram.len()],
                    driven: true,
                }
            } else {
                FetchResult { data: 0, driven: false }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            if !self.use_rom && !cart.prg_ram.is_empty() {
                let ram_len = cart.prg_ram.len();
                let offset = (address as usize - 0x6000) & 0x1FFF;
                cart.prg_ram[offset % ram_len] = data;
            }
            return;
        }
        if address >= 0x8000 {
            match address & 0xF000 {
                0x8000 => self.irq_reload_value = (self.irq_reload_value & 0xFFF0) | (data as u16 & 0x0F),
                0x9000 => self.irq_reload_value = (self.irq_reload_value & 0xFF0F) | ((data as u16 & 0x0F) << 4),
                0xA000 => self.irq_reload_value = (self.irq_reload_value & 0xF0FF) | ((data as u16 & 0x0F) << 8),
                0xB000 => self.irq_reload_value = (self.irq_reload_value & 0x0FFF) | ((data as u16 & 0x0F) << 12),
                0xC000 => {
                    self.irq_control = data;
                    if (self.irq_control & 0x02) != 0 {
                        self.irq_counter = self.irq_reload_value;
                    }
                    self.irq_pending = false;
                }
                0xD000 => {
                    self.irq_pending = false;
                }
                0xE000 => {
                    let reg_val = data & 0x07;
                    if reg_val > 0 {
                        self.selected_reg = reg_val - 1;
                    } else {
                        self.selected_reg = 7; 
                    }
                }
                0xF000 => {
                    if self.selected_reg < 4 {
                        let idx = self.selected_reg as usize;
                        self.prg_regs[idx] = (self.prg_regs[idx] & 0x10) | (data & 0x0F);
                    } else if self.selected_reg == 4 {
                        self.use_rom = (data & 0x04) != 0;
                    }
                    match address & 0xFC00 {
                        0xF000 => {
                            let bank = (address & 0x03) as usize;
                            self.prg_regs[bank] = (data & 0x10) | (self.prg_regs[bank] & 0x0F);
                        }
                        0xF800 => {
                            self.mirroring_vertical = (data & 0x01) != 0;
                        }
                        0xFC00 => {
                            let chr_slot = (address & 0x07) as usize;
                            self.chr_banks[chr_slot] = data;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        let mut irq_triggered = false;
        for _ in 0..cycles {
            if (self.irq_control & 0x02) != 0 {
                self.irq_counter = self.irq_counter.wrapping_add(1);
                if self.irq_counter == 0 {
                    self.irq_counter = self.irq_reload_value;
                    self.irq_control &= !0x02;
                    self.irq_pending = true;
                    irq_triggered = true;
                }
            }
        }
        irq_triggered
    }

    fn cpu_clock_rise(&mut self, _ppu_address_bus: u16) -> bool {
        self.irq_pending
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mirroring_vertical {
            address & 0x37FF
        } else {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        prg_vram: &[u8],
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
            let slot = (address as usize >> 10) & 7; 
            let bank = self.chr_banks[slot] as usize;
            let offset = bank * 0x0400 + (address as usize & 0x03FF);
            if using_chr_ram {
                let len = chr_ram.len();
                if len > 0 {
                    new_addr_bus |= chr_ram[offset % len] as u16;
                }
            } else {
                let len = chr_rom.len();
                if len > 0 {
                    new_addr_bus |= chr_rom[offset % len] as u16;
                }
            }
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if self.mirroring_vertical {
                address & 0x37FF
            } else {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            };
            let byte = if alternative_nametable_arrangement && (mirrored & 0x0800) != 0 {
                let idx = (mirrored & 0x7FF) as usize;
                if idx < prg_vram.len() { prg_vram[idx] } else { 0 }
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
                let slot = (address as usize >> 10) & 7;
                let bank = self.chr_banks[slot] as usize;
                let offset = bank * 0x0400 + (address as usize & 0x03FF);
                let len = cart.chr_ram.len();
                cart.chr_ram[offset % len] = data;
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

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.irq_reload_value.to_le_bytes());
        state.extend_from_slice(&self.irq_counter.to_le_bytes());
        state.push(self.irq_control);
        state.push(self.selected_reg);
        state.extend_from_slice(&self.prg_regs);
        state.push(self.use_rom as u8);
        state.extend_from_slice(&self.chr_banks);
        state.push(self.irq_pending as u8);
        state.push(self.mirroring_vertical as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 21 <= state.len() {
            self.irq_reload_value = u16::from_le_bytes([state[start], state[start + 1]]);
            self.irq_counter = u16::from_le_bytes([state[start + 2], state[start + 3]]);
            self.irq_control = state[start + 4];
            self.selected_reg = state[start + 5];
            self.prg_regs.copy_from_slice(&state[start + 6..start + 10]);
            self.use_rom = state[start + 10] != 0;
            self.chr_banks.copy_from_slice(&state[start + 11..start + 19]);
            self.irq_pending = state[start + 19] != 0;
            self.mirroring_vertical = state[start + 20] != 0;
            start += 21;
        }
        start
    }
}
