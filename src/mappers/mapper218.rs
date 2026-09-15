use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const NT_MAP: [u8; 16] = [0,0,0,0, 1,1,1,1, 0,0,0,0, 1,1,1,1];

pub struct Mapper218;

impl Mapper218 {
    pub fn new() -> Self { Self }
}

impl Mapper for Mapper218 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let offset = address as usize & 0x7FFF;
            FetchResult { data: cart.prg_rom[offset % cart.prg_rom.len()], driven: true }
        } else { FetchResult { data: 0, driven: false } }
    }
    fn store_prg(&mut self, _cart: &mut Cartridge, _address: u16, _data: u8) {}
    fn mirror_nametable(&self, _cart: &Cartridge, _address: u16) -> u16 { 0 }
    fn fetch_ppu(&mut self, _prg_rom: &[u8], chr_rom: &[u8], _prg_ram: &[u8], chr_ram: &[u8], _prg_vram: &[u8], using_chr_ram: bool, _nametable_horizontal_mirroring: bool, _alternative_nametable_arrangement: bool, ppu_address_bus: u16, ppu_octal_latch: u8, vram: &[u8]) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let data = if using_chr_ram && !chr_ram.is_empty() { chr_ram[address as usize % chr_ram.len()] } else { chr_rom[address as usize % chr_rom.len()] };
            new_addr_bus |= data as u16;
        } else {
            let nt = NT_MAP[((address >> 10) & 0xF) as usize];
            let nt_base = 0x2000 + nt as u16 * 0x400;
            new_addr_bus |= vram[((address & 0x3FF) + nt_base) as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }
    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 { if cart.using_chr_ram && !cart.chr_ram.is_empty() { let len = cart.chr_ram.len(); cart.chr_ram[address as usize % len] = data; } }
        else if address >= 0x2000 && address < 0x3F00 {
            let nt = NT_MAP[((address >> 10) & 0xF) as usize];
            let nt_base = 0x2000 + nt as u16 * 0x400;
            vram[((address & 0x3FF) + nt_base) as usize & 0x7FF] = data;
        }
    }
    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> { Vec::new() }
    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, _state: &[u8], start: usize) -> usize { start }
    fn reset(&mut self) {}
}