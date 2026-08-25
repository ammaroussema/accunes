use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::um6578::Um6578Hw;

pub struct Mapper405 {
    prg: [u8; 8],
    hw: Um6578Hw,
}

impl Default for Mapper405 {
    fn default() -> Self { Self::new() }
}

impl Mapper405 {
    pub fn new() -> Self {
        Self { prg: [0,1,2,3,4,5,6,7], hw: Um6578Hw::new() }
    }
    fn get_prg_bank(&self, slot: usize, prg_len: usize) -> usize {
        let n = (prg_len / 0x1000).max(1);
        (self.prg[slot & 7] as usize) % n
    }
}

impl Mapper for Mapper405 {
    fn is_um6578(&self) -> bool { true }

    fn reset(&mut self) {
        self.prg = [0,1,2,3,4,5,6,7];
        self.hw.reset();
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        match address {
            0x4040..=0x4047 => self.prg[address as usize & 7] = data,
            0x4032..=0x4036 | 0x4016 | 0x4026 => { let _ = self.hw.write_reg(address, data); }
            _ => { let _ = self.hw.write_reg(address, data); }
        }
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x4032..=0x4036).contains(&address) || address == 0x4016 || address == 0x4026
            || (0x4040..=0x4047).contains(&address)
        {
            let v = match address {
                0x4032 => self.hw.irq_mask,
                0x4033 => self.hw.irq_status,
                0x4034 => self.hw.timer_control,
                0x4035 => self.hw.timer_latch,
                0x4036 => self.hw.timer_value,
                0x4016 => self.hw.reg_4016,
                0x4026 => self.hw.reg_4026,
                0x4040..=0x4047 => self.prg[(address & 7) as usize],
                _ => 0,
            };
            return FetchResult { data: v, driven: true };
        }
        if address >= 0x8000 {
            let slot = ((address - 0x8000) >> 12) as usize;
            let bank = self.get_prg_bank(slot, cart.prg_rom.len());
            let offset = bank * 0x1000 + (address as usize & 0xFFF);
            let data = if cart.prg_rom.is_empty() { 0 } else { cart.prg_rom[offset % cart.prg_rom.len()] };
            return FetchResult { data, driven: true };
        }
        FetchResult { data: 0, driven: false }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        address & 0x37FF
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        _chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        _vram: &[u8],
    ) -> (u8, u16) {
        let addr = (ppu_address_bus & 0x7FFF) | ppu_octal_latch as u16;
        (addr as u8, addr)
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.hw.cpu_cycle()
    }
    fn ppu_clock(&mut self, _ppu_address_bus: u16, _a12: bool, scanline: u16, dot: u16, _x16: bool, rendering: bool) -> bool {
        self.hw.ppu_cycle(scanline, dot, rendering)
    }
    fn take_irq_ack(&mut self) -> bool {
        self.hw.take_irq()
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&self.prg);
        v.push(self.hw.irq_mask);
        v.push(self.hw.irq_status);
        v.push(self.hw.timer_control);
        v.push(self.hw.timer_latch);
        v.push(self.hw.timer_value);
        v.push(self.hw.reg_4016);
        v.push(self.hw.reg_4026);
        v
    }
    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 8 <= state.len() { self.prg.copy_from_slice(&state[p..p+8]); p+=8; }
        if p < state.len() { self.hw.irq_mask = state[p]; p+=1; }
        if p < state.len() { self.hw.irq_status = state[p]; p+=1; }
        if p < state.len() { self.hw.timer_control = state[p]; p+=1; }
        if p < state.len() { self.hw.timer_latch = state[p]; p+=1; }
        if p < state.len() { self.hw.timer_value = state[p]; p+=1; }
        if p < state.len() { self.hw.reg_4016 = state[p]; p+=1; }
        if p < state.len() { self.hw.reg_4026 = state[p]; p+=1; }
        p
    }
}
