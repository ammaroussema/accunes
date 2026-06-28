use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper63 {
    open_bus: bool,
    prg_banks: [usize; 4],
    mirror_horizontal: bool,
}

impl Mapper63 {
    pub fn new() -> Self {
        Self {
            open_bus: false,
            prg_banks: [0; 4],
            mirror_horizontal: false,
        }
    }
}

impl Mapper for Mapper63 {
    fn reset(&mut self) {
        self.open_bus = false;
        self.store_prg_register(0x8000);
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            if self.open_bus && address < 0xC000 {
                return FetchResult { data: 0, driven: false };
            }
            let num_8k_banks = cart.prg_rom.len() / 0x2000;
            if num_8k_banks == 0 {
                return FetchResult { data: 0, driven: true };
            }
            let slot = ((address as usize - 0x8000) >> 13) & 3; 
            let bank = self.prg_banks[slot] % num_8k_banks;
            let offset = bank * 0x2000 + (address as usize & 0x1FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, _data: u8) {
        if address >= 0x8000 {
            self.store_prg_register(address);
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        if self.mirror_horizontal {
            (address & 0x33FF) | ((address & 0x0800) >> 1)
        } else {
            address & 0x37FF
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
            if using_chr_ram {
                let len = chr_ram.len();
                if len > 0 {
                    new_addr_bus |= chr_ram[(address as usize & 0x1FFF) % len] as u16;
                }
            } else {
                let len = chr_rom.len();
                if len > 0 {
                    new_addr_bus |= chr_rom[(address as usize & 0x1FFF) % len] as u16;
                }
            }
        } else {
            let mirrored = if alternative_nametable_arrangement {
                address
            } else if self.mirror_horizontal {
                (address & 0x33FF) | ((address & 0x0800) >> 1)
            } else {
                address & 0x37FF
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
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
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
        state.push(self.open_bus as u8);
        for &bank in &self.prg_banks {
            state.extend_from_slice(&bank.to_le_bytes());
        }
        state.push(self.mirror_horizontal as u8);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        let size_needed = 1 + (std::mem::size_of::<usize>() * 4) + 1;
        if start + size_needed <= state.len() {
            self.open_bus = state[start] != 0; start += 1;
            for i in 0..4 {
                let mut bytes = [0u8; std::mem::size_of::<usize>()];
                bytes.copy_from_slice(&state[start..start + std::mem::size_of::<usize>()]);
                self.prg_banks[i] = usize::from_le_bytes(bytes);
                start += std::mem::size_of::<usize>();
            }
            self.mirror_horizontal = state[start] != 0; start += 1;
        }
        start
    }
}

impl Mapper63 {
    fn store_prg_register(&mut self, addr: u16) {
        let addr_val = addr as usize;
        self.open_bus = (addr & 0x0300) == 0x0300;
        self.prg_banks[0] = (addr_val >> 1 & 0x1FC) | (if (addr & 2) != 0 { 0 } else { addr_val >> 1 & 2 });
        self.prg_banks[1] = (addr_val >> 1 & 0x1FC) | (if (addr & 2) != 0 { 1 } else { (addr_val >> 1 & 2) | 1 });
        self.prg_banks[2] = (addr_val >> 1 & 0x1FC) | (if (addr & 2) != 0 { 2 } else { addr_val >> 1 & 2 });
        self.prg_banks[3] = if (addr & 0x800) != 0 {
            (addr_val & 0x07C) | (if (addr & 0x06) != 0 { 3 } else { 1 })
        } else {
            (addr_val >> 1 & 0x01FC) | (if (addr & 2) != 0 { 3 } else { (addr_val >> 1 & 2) | 1 })
        };
        self.mirror_horizontal = (addr & 0x01) != 0;
    }
}
