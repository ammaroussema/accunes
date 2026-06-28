use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

pub struct Mapper31 {
    prg_banks: [u8; 8],
}

impl Mapper31 {
    pub fn new() -> Self {
        let mut mapper = Self {
            prg_banks: [0xFF; 8],
        };
        for bank in mapper.prg_banks.iter_mut() {
            *bank = 0xFF;
        }
        mapper
    }
}

impl Mapper for Mapper31 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let bank_index = ((address - 0x8000) / 0x1000) as usize;
            let bank = self.prg_banks[bank_index] as usize;
            let offset = (bank * 0x1000) + (address as usize & 0x0FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else {
            FetchResult {
                data: 0,
                driven: false,
            }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x5000 && address <= 0x5FFF {
            let bank_index = (address & 0x07) as usize;
            self.prg_banks[bank_index] = data;
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let nt = (address >> 11) & 1;
        (address & 0x03FF) | (nt << 10)
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
            if using_chr_ram {
                new_addr_bus |= chr_ram[address as usize & (chr_ram.len() - 1)] as u16;
            } else {
                new_addr_bus |= chr_rom[address as usize % chr_rom.len()] as u16;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(
                &Cartridge {
                    name: String::new(),
                    prg_rom: Vec::new(),
                    chr_rom: Vec::new(),
                    memory_mapper: 31,
                    sub_mapper: 0,
                    prg_size: 0,
                    chr_size: 0,
                    prg_size_minus_1: 0,
                    chr_ram: Vec::new(),
                    using_chr_ram: false,
                    prg_ram: Vec::new(),
                    has_battery: false,
                    alternative_nametable_arrangement: false,
                    prg_vram: Vec::new(),
                    nametable_horizontal_mirroring: false,
                    fds_disks: Vec::new(),
                    trainer: Vec::new(),
                    misc_rom: Vec::new(),
                    mapper_chip: Box::new(Mapper31 { prg_banks: [0xFF; 8] }),
                    mapper_cpu_cycle: 0,
                    prg_rom_crc32: 0,
                    chr_rom_crc32: 0,
                    overall_crc32: 0,
                    is_vs_system: false,
                    tv_system: crate::region::TvSystem::Unknown,
                },
                address,
            );
            let idx = (mirrored & 0x7FF) as usize;
            new_addr_bus |= vram[idx] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if cart.using_chr_ram {
                let len = cart.chr_ram.len();
                cart.chr_ram[address as usize & (len - 1)] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            let idx = (mirrored & 0x7FF) as usize;
            vram[idx] = data;
        }
    }

    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&cart.prg_ram);
        state.extend_from_slice(&cart.chr_ram);
        state.extend_from_slice(&self.prg_banks);
        state
    }

    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        let prg_len = cart.prg_ram.len();
        if start + prg_len <= state.len() {
            cart.prg_ram.copy_from_slice(&state[start..start + prg_len]);
            start += prg_len;
        }
        let chr_len = cart.chr_ram.len();
        if start + chr_len <= state.len() {
            cart.chr_ram.copy_from_slice(&state[start..start + chr_len]);
            start += chr_len;
        }
        if start + 8 <= state.len() {
            self.prg_banks.copy_from_slice(&state[start..start + 8]);
            start += 8;
        }
        start
    }
}
