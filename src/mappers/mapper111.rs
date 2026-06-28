use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
enum V111 { Gtrom, ChineseMmc1 }

pub struct Mapper111 {
    variant: V111,
    gtrom_reg: u8,
    mmc1_regs: [u8; 4],
    num_prg_16k: usize,
    wram_enabled: bool,
    chr_rom_present: bool,
}

impl Mapper111 {
    pub fn new(prg_16k_count: u8, chr_rom_non_empty: bool) -> Self {
        Self {
            variant: if chr_rom_non_empty { V111::ChineseMmc1 } else { V111::Gtrom },
            gtrom_reg: 0,
            mmc1_regs: [0x0C, 0, 0, 0],
            num_prg_16k: (prg_16k_count as usize).max(1),
            wram_enabled: true,
            chr_rom_present: chr_rom_non_empty,
        }
    }

    fn mmc1_chr_bank(&self, bank: usize) -> usize {
        let raw = if (self.mmc1_regs[0] & 0x10) != 0 {
            self.mmc1_regs[1 + bank]
        } else {
            (self.mmc1_regs[1] & 0x1E) | (bank as u8)
        };
        (raw & 0x3F) as usize
    }

    fn mmc1_mirror_addr(&self, address: u16) -> u16 {
        match self.mmc1_regs[0] & 0x03 {
            0 => address & 0x23FF,
            1 => (address & 0x23FF) | 0x0400,
            2 => address & 0x37FF,
            3 | _ => (address & 0x33FF) | ((address & 0x0800) >> 1),
        }
    }

    fn prg_16k(&self, cart: &Cartridge, address: u16, bank: usize) -> usize {
        let num_16k = cart.prg_rom.len() / 0x4000;
        let b = if num_16k > 0 { bank % num_16k } else { 0 };
        b * 0x4000 + (address as usize & 0x3FFF)
    }

    fn mmc1_prg_offset(&self, cart: &Cartridge, address: u16) -> usize {
        let mode = (self.mmc1_regs[0] >> 2) & 0x03;
        let prg = (self.mmc1_regs[3] & 0x0F) as usize;
        match mode {
            0 | 1 => {
                let bank = prg & 0x0E;
                let sub = if address >= 0xC000 { 1 } else { 0 };
                let bank16 = (bank + sub).min(self.num_prg_16k - 1);
                self.prg_16k(cart, address, bank16)
            }
            2 => {
                if address >= 0xC000 {
                    self.prg_16k(cart, address, prg.min(self.num_prg_16k - 1))
                } else {
                    self.prg_16k(cart, address, 0)
                }
            }
            3 | _ => {
                if address >= 0xC000 {
                    self.prg_16k(cart, address, 0x0F.min(self.num_prg_16k - 1))
                } else {
                    self.prg_16k(cart, address, prg.min(self.num_prg_16k - 1))
                }
            }
        }
    }
}

impl Mapper for Mapper111 {
    fn reset(&mut self) {
        self.gtrom_reg = 0;
        self.mmc1_regs = [0x0C, 0, 0, 0];
        self.wram_enabled = true;
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        match self.variant {
            V111::Gtrom => {
                if address >= 0x8000 {
                    let bank = (self.gtrom_reg & 0x0F) as usize;
                    let len = cart.prg_rom.len();
                    if len == 0 {
                        return FetchResult { data: 0, driven: false };
                    }
                    let num_32k = len / 0x8000;
                    let b = if num_32k > 0 { bank % num_32k } else { 0 };
                    let offset = b * 0x8000 + (address as usize & 0x7FFF);
                    FetchResult { data: cart.prg_rom[offset % len], driven: true }
                } else if address >= 0x5000 && address < 0x6000 {
                    FetchResult { data: 0, driven: false }
                } else if address >= 0x6000 && address < 0x8000 {
                    if !cart.prg_ram.is_empty() {
                        let offset = (address as usize - 0x6000) % cart.prg_ram.len();
                        FetchResult { data: cart.prg_ram[offset], driven: true }
                    } else {
                        FetchResult { data: 0, driven: false }
                    }
                } else {
                    FetchResult { data: 0, driven: false }
                }
            }
            V111::ChineseMmc1 => {
                if address >= 0x8000 {
                    let offset = self.mmc1_prg_offset(cart, address);
                    FetchResult {
                        data: cart.prg_rom[offset % cart.prg_rom.len()],
                        driven: true,
                    }
                } else if address >= 0x6000 {
                    if !self.wram_enabled {
                        return FetchResult { data: 0, driven: false };
                    }
                    let idx = (address - 0x6000) as usize;
                    if idx < cart.prg_ram.len() {
                        FetchResult { data: cart.prg_ram[idx], driven: true }
                    } else {
                        FetchResult { data: 0, driven: false }
                    }
                } else {
                    FetchResult { data: 0, driven: false }
                }
            }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        match self.variant {
            V111::Gtrom => {
                if address >= 0x5000 && address < 0x8000 {
                    self.gtrom_reg = data;
                } else if address >= 0x6000 && address < 0x8000 {
                    if !cart.prg_ram.is_empty() {
                        let offset = (address as usize - 0x6000) % cart.prg_ram.len();
                        cart.prg_ram[offset] = data;
                    }
                }
            }
            V111::ChineseMmc1 => {
                if address >= 0x8000 {
                    let idx = ((address >> 13) & 3) as usize;
                    self.mmc1_regs[idx] = data;
                    if idx == 3 {
                        self.wram_enabled = (data & 0x10) == 0;
                    }
                } else if address >= 0x6000 {
                    if self.wram_enabled {
                        let idx = (address - 0x6000) as usize;
                        if idx < cart.prg_ram.len() {
                            cart.prg_ram[idx] = data;
                        }
                    }
                }
            }
        }
    }

    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16 {
        match self.variant {
            V111::Gtrom => {
                if cart.nametable_horizontal_mirroring {
                    (address & 0x33FF) | ((address & 0x0800) >> 1)
                } else {
                    address & 0x37FF
                }
            }
            V111::ChineseMmc1 => self.mmc1_mirror_addr(address),
        }
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            match self.variant {
                V111::Gtrom => {
                    let bank_lo = ((self.gtrom_reg >> 4) & 1) as usize;
                    let bank_hi = (((self.gtrom_reg >> 5) & 1) | 2) as usize;
                    let bank = if address < 0x2000 { bank_lo } else { bank_hi };
                    let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                    if !chr_ram.is_empty() {
                        new_addr_bus |= chr_ram[offset % chr_ram.len()] as u16;
                    }
                }
                V111::ChineseMmc1 => {
                    let bank_num = (address >> 12) as usize & 1;
                    let bank = self.mmc1_chr_bank(bank_num);
                    let offset = bank * 0x1000 + (address as usize & 0x0FFF);
                    if self.chr_rom_present {
                        if !chr_rom.is_empty() {
                            new_addr_bus |= chr_rom[offset % chr_rom.len()] as u16;
                        }
                    } else if using_chr_ram && !chr_ram.is_empty() {
                        new_addr_bus |= chr_ram[offset % chr_ram.len()] as u16;
                    }
                }
            }
        } else {
            let mirrored = match self.variant {
                V111::Gtrom => {
                    if nametable_horizontal_mirroring {
                        (address & 0x33FF) | ((address & 0x0800) >> 1)
                    } else {
                        address & 0x37FF
                    }
                }
                V111::ChineseMmc1 => self.mmc1_mirror_addr(address),
            };
            new_addr_bus |= vram[(mirrored & 0x7FF) as usize] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram && !cart.chr_ram.is_empty() {
                match self.variant {
                    V111::Gtrom => {
                        let bank_lo = ((self.gtrom_reg >> 4) & 1) as usize;
                        let bank_hi = (((self.gtrom_reg >> 5) & 1) | 2) as usize;
                        let bank = if address < 0x2000 { bank_lo } else { bank_hi };
                        let offset = bank * 0x2000 + (address as usize & 0x1FFF);
                        let len = cart.chr_ram.len();
                        cart.chr_ram[offset % len] = data;
                    }
                    V111::ChineseMmc1 => {
                        let bank_num = (address >> 12) as usize & 1;
                        let bank = self.mmc1_chr_bank(bank_num);
                        let offset = bank * 0x1000 + (address as usize & 0x0FFF);
                        let len = cart.chr_ram.len();
                        cart.chr_ram[offset % len] = data;
                    }
                }
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(0); 
        state.push(self.gtrom_reg);
        state.extend_from_slice(&self.mmc1_regs);
        state.push(if self.wram_enabled { 1 } else { 0 });
        if cart.using_chr_ram {
            state.extend_from_slice(&cart.chr_ram);
        }
        state.extend_from_slice(&cart.prg_ram);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() {
            p += 1; 
        }
        if p < state.len() {
            self.gtrom_reg = state[p];
            p += 1;
        }
        if p + 4 <= state.len() {
            self.mmc1_regs.copy_from_slice(&state[p..p + 4]);
            p += 4;
        }
        if p < state.len() {
            self.wram_enabled = state[p] != 0;
            p += 1;
        }
        if cart.using_chr_ram {
            for i in 0..cart.chr_ram.len() {
                if p < state.len() {
                    cart.chr_ram[i] = state[p];
                    p += 1;
                }
            }
        }
        for i in 0..cart.prg_ram.len() {
            if p < state.len() {
                cart.prg_ram[i] = state[p];
                p += 1;
            }
        }
        p
    }
}
