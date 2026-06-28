use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const VS_FRAME_CYCLES: u64 = 29780;

pub struct Mapper68 {
    prg_bank: u8,
    chr_banks: [u8; 4],
    nt_regs: [u8; 2],
    use_chr_for_nametables: bool,
    mirroring: u8, 
    prg_ram_enabled: bool,
    licensing_timer: u32,
    using_external_rom: bool,
    external_page: u8,
    vsdip: u8,
    coinon: u8,
    coinon2: u8,
    service: u8,
    cycle_accum: u64,
}

impl Mapper68 {
    pub fn new() -> Self {
        Self {
            prg_bank: 0,
            chr_banks: [0; 4],
            nt_regs: [0; 2],
            use_chr_for_nametables: false,
            mirroring: 0,
            prg_ram_enabled: false,
            licensing_timer: 0,
            using_external_rom: false,
            external_page: 0,
            vsdip: 0,
            coinon: 0,
            coinon2: 0,
            service: 0,
            cycle_accum: 0,
        }
    }

    fn mirror_address(&self, address: u16) -> u16 {
        match self.mirroring {
            0 => address & 0x37FF, 
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1), 
            2 => address & 0x33FF, 
            3 => (address & 0x33FF) | 0x0400, 
            _ => address & 0x37FF,
        }
    }
}

impl Mapper for Mapper68 {
    fn adjust_controller_read(&self, address: u16, value: u8) -> u8 {
        if address & 0x1F == 0x16 {
            let mut vs = value & 0x01;
            if self.service > 0 { vs |= 0x04; }
            vs |= (self.vsdip & 0x03) << 3;
            if self.coinon > 0 { vs |= 0x20; }
            if self.coinon2 > 0 { vs |= 0x40; }
            vs
        } else if address & 0x1F == 0x17 {
            (value & 0x01) | (self.vsdip & 0xFC)
        } else {
            value
        }
    }

    fn insert_coin(&mut self, coin: u8) {
        match coin {
            0 => self.coinon = 6,
            1 => self.coinon2 = 6,
            _ => {}
        }
    }

    fn service_button(&mut self) {
        self.service = 6;
    }

    fn get_dip_switches(&self) -> u8 {
        self.vsdip
    }

    fn set_dip_switches(&mut self, value: u8) {
        self.vsdip = value;
    }

    fn reset(&mut self) {
        self.prg_bank = 0;
        self.chr_banks = [0; 4];
        self.nt_regs = [0; 2];
        self.use_chr_for_nametables = false;
        self.mirroring = 0;
        self.prg_ram_enabled = false;
        self.licensing_timer = 0;
        self.using_external_rom = false;
        self.external_page = 0;
        self.vsdip = 0;
        self.coinon = 0;
        self.coinon2 = 0;
        self.service = 0;
        self.cycle_accum = 0;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x6000 && address < 0x8000 {
            if self.prg_ram_enabled && !cart.prg_ram.is_empty() {
                let off = (address as usize - 0x6000) % cart.prg_ram.len();
                return FetchResult {
                    data: cart.prg_ram[off],
                    driven: true,
                };
            }
            return FetchResult { data: 0, driven: false };
        }
        if address < 0x8000 {
            return FetchResult { data: 0, driven: false };
        }
        let num_16k = cart.prg_rom.len() / 0x4000;
        if num_16k == 0 {
            return FetchResult { data: 0, driven: true };
        }
        if address >= 0xC000 {
            let offset = (num_16k - 1) * 0x4000 + (address as usize & 0x3FFF);
            return FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            };
        }
        if self.using_external_rom {
            if self.licensing_timer == 0 {
                return FetchResult { data: 0, driven: false };
            } else {
                let offset = (self.external_page as usize % num_16k) * 0x4000 + (address as usize & 0x3FFF);
                return FetchResult {
                    data: cart.prg_rom[offset % cart.prg_rom.len()],
                    driven: true,
                };
            }
        }
        let offset = (self.prg_bank as usize % num_16k) * 0x4000 + (address as usize & 0x3FFF);
        FetchResult {
            data: cart.prg_rom[offset % cart.prg_rom.len()],
            driven: true,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x6000 && address < 0x8000 {
            self.licensing_timer = 1024 * 105;
            if self.prg_ram_enabled && !cart.prg_ram.is_empty() {
                let off = (address as usize - 0x6000) % cart.prg_ram.len();
                cart.prg_ram[off] = data;
            }
            return;
        }
        if address < 0x8000 {
            return;
        }
        match address & 0xF000 {
            0x8000 => self.chr_banks[0] = data,
            0x9000 => self.chr_banks[1] = data,
            0xA000 => self.chr_banks[2] = data,
            0xB000 => self.chr_banks[3] = data,
            0xC000 => self.nt_regs[0] = data | 0x80,
            0xD000 => self.nt_regs[1] = data | 0x80,
            0xE000 => {
                self.mirroring = data & 0x03;
                self.use_chr_for_nametables = (data & 0x10) != 0;
            }
            0xF000 => {
                let num_16k = cart.prg_rom.len() / 0x4000;
                let external_prg = (data & 0x08) == 0;
                if external_prg && num_16k > 8 {
                    self.using_external_rom = true;
                    self.external_page = 0x08 | ((data & 0x07) % (num_16k as u8 - 0x08));
                } else {
                    self.using_external_rom = false;
                    self.prg_bank = data & 0x07;
                }
                self.prg_ram_enabled = (data & 0x10) != 0;
            }
            _ => {}
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        self.mirror_address(address)
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
        if address < 0x2000 {
            let slot = (address >> 11) & 3;
            let bank = self.chr_banks[slot as usize] as usize;
            let offset = bank * 0x0800 + (address as usize & 0x07FF);
            let byte = if using_chr_ram {
                if chr_ram.is_empty() { 0 } else { chr_ram[offset % chr_ram.len()] }
            } else if chr_rom.is_empty() {
                0
            } else {
                chr_rom[offset % chr_rom.len()]
            };
            new_addr_bus |= byte as u16;
        } else {
            if self.use_chr_for_nametables {
                let i = ((address - 0x2000) >> 10) & 3;
                let reg = match self.mirroring {
                    0 => (i & 1) as usize,
                    1 => ((i & 2) >> 1) as usize,
                    2 => 0,
                    3 => 1,
                    _ => 0,
                };
                let bank = self.nt_regs[reg] as usize;
                let offset = bank * 0x0400 + (address as usize & 0x03FF);
                let byte = if using_chr_ram {
                    if chr_ram.is_empty() { 0 } else { chr_ram[offset % chr_ram.len()] }
                } else if chr_rom.is_empty() {
                    0
                } else {
                    chr_rom[offset % chr_rom.len()]
                };
                new_addr_bus |= byte as u16;
            } else {
                let mirrored = self.mirror_address(address);
                new_addr_bus |= vram[mirrored as usize & 0x7FF] as u16;
            }
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            let slot = (address >> 11) & 3;
            let bank = self.chr_banks[slot as usize] as usize;
            let offset = bank * 0x0800 + (address as usize & 0x07FF);
            let len = cart.chr_ram.len();
            if len > 0 {
                cart.chr_ram[offset % len] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            if self.use_chr_for_nametables {
                if cart.using_chr_ram {
                    let i = ((address - 0x2000) >> 10) & 3;
                    let reg = match self.mirroring {
                        0 => (i & 1) as usize,
                        1 => ((i & 2) >> 1) as usize,
                        2 => 0,
                        3 => 1,
                        _ => 0,
                    };
                    let bank = self.nt_regs[reg] as usize;
                    let offset = bank * 0x0400 + (address as usize & 0x03FF);
                    let len = cart.chr_ram.len();
                    if len > 0 {
                        cart.chr_ram[offset % len] = data;
                    }
                }
            } else {
                let mirrored = self.mirror_address(address);
                vram[(mirrored & 0x7FF) as usize] = data;
            }
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.cycle_accum += _cycles as u64;
        if self.cycle_accum >= VS_FRAME_CYCLES {
            self.cycle_accum = 0;
            if self.coinon > 0 { self.coinon -= 1; }
            if self.coinon2 > 0 { self.coinon2 -= 1; }
            if self.service > 0 { self.service -= 1; }
        }
        if self.licensing_timer > 0 {
            self.licensing_timer = self.licensing_timer.saturating_sub(_cycles as u32);
        }
        false
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.prg_bank);
        state.extend_from_slice(&self.chr_banks);
        state.extend_from_slice(&self.nt_regs);
        state.push(if self.use_chr_for_nametables { 1 } else { 0 });
        state.push(self.mirroring);
        state.push(if self.prg_ram_enabled { 1 } else { 0 });
        state.extend_from_slice(&self.licensing_timer.to_le_bytes());
        state.push(if self.using_external_rom { 1 } else { 0 });
        state.push(self.external_page);
        state.push(self.vsdip);
        state.push(self.coinon);
        state.push(self.coinon2);
        state.push(self.service);
        state.extend_from_slice(&cart.prg_ram);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut i = start;
        self.prg_bank = state[i]; i += 1;
        for r in 0..4 { self.chr_banks[r] = state[i]; i += 1; }
        for r in 0..2 { self.nt_regs[r] = state[i]; i += 1; }
        self.use_chr_for_nametables = state[i] != 0; i += 1;
        self.mirroring = state[i]; i += 1;
        self.prg_ram_enabled = state[i] != 0; i += 1;
        let mut bytes = [0; 4];
        bytes.copy_from_slice(&state[i..i+4]);
        self.licensing_timer = u32::from_le_bytes(bytes);
        i += 4;
        self.using_external_rom = state[i] != 0; i += 1;
        self.external_page = state[i]; i += 1;
        self.vsdip = state.get(i).copied().unwrap_or(0); i += 1;
        self.coinon = state.get(i).copied().unwrap_or(0); i += 1;
        self.coinon2 = state.get(i).copied().unwrap_or(0); i += 1;
        self.service = state.get(i).copied().unwrap_or(0); i += 1;
        let prg_ram_len = cart.prg_ram.len();
        if prg_ram_len > 0 {
            let copy_len = prg_ram_len.min(state.len() - i);
            cart.prg_ram[..copy_len].copy_from_slice(&state[i..i+copy_len]);
            i += copy_len;
        }
        i - start
    }
}
