use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::um6578::Um6578Hw;

pub struct Mapper600 {
    prg: [u8; 8],
    hw: Um6578Hw,
}
impl Default for Mapper600 {
    fn default() -> Self { Self::new() }
}
impl Mapper600 {
    pub fn new() -> Self {
        Self { prg: [0,1,2,3,4,5,6,7], hw: Um6578Hw::new() }
    }
    fn get_bank(&self, slot: usize, len: usize) -> usize {
        let n = (len / 0x1000).max(1);
        let hi = ((self.hw.reg_4016 as usize & 0x02) << 7) & !0xFF;
        ((self.prg[slot & 7] as usize) | hi) % n
    }
}
impl Mapper for Mapper600 {
    fn is_um6578(&self) -> bool { true }
    fn reset(&mut self) { self.prg = [0,1,2,3,4,5,6,7]; self.hw.reset(); }
    fn store_prg(&mut self, _c: &mut Cartridge, a: u16, d: u8) {
        match a {
            0x4040..=0x4047 => self.prg[a as usize & 7] = d,
            0x4032..=0x4036 | 0x4016 | 0x4026 => { let _ = self.hw.write_reg(a, d); },
            _ => { let _ = self.hw.write_reg(a, d); }
        }
    }
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if (0x4032..=0x4036).contains(&address) || address == 0x4016 || address == 0x4026 {
            let v = match address {
                0x4032 => self.hw.irq_mask,
                0x4033 => self.hw.irq_status,
                0x4034 => self.hw.timer_control,
                0x4035 => self.hw.timer_latch,
                0x4036 => self.hw.timer_value,
                0x4016 => self.hw.reg_4016,
                0x4026 => self.hw.reg_4026,
                _ => 0,
            };
            return FetchResult { data: v, driven: true };
        }
        if address >= 0x8000 {
            let s = ((address - 0x8000) >> 12) as usize;
            let b = self.get_bank(s, cart.prg_rom.len());
            let o = b * 0x1000 + (address as usize & 0xFFF);
            let d = if cart.prg_rom.is_empty() { 0 } else { cart.prg_rom[o % cart.prg_rom.len()] };
            return FetchResult { data: d, driven: true };
        }
        FetchResult { data: 0, driven: false }
    }
    fn mirror_nametable(&self, _c: &Cartridge, a: u16) -> u16 { a & 0x37FF }
    fn fetch_ppu(&mut self, _pr: &[u8], _ch: &[u8], _pr2: &[u8], _cr: &[u8], _pv: &[u8], _u: bool, _n: bool, _a: bool, p: u16, o: u8, _v: &[u8]) -> (u8, u16) {
        let a = (p & 0x7FFF) | o as u16;
        (a as u8, a)
    }
    fn cpu_clock(&mut self, _c: u8) -> bool { self.hw.cpu_cycle() }
    fn ppu_clock(&mut self, _p: u16, _a: bool, s: u16, d: u16, _x: bool, r: bool) -> bool { self.hw.ppu_cycle(s, d, r) }
    fn take_irq_ack(&mut self) -> bool { self.hw.take_irq() }
    fn save_mapper_registers(&self, _c: &Cartridge) -> Vec<u8> {
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
    fn load_mapper_registers(&mut self, _c: &mut Cartridge, s: &[u8], st: usize) -> usize {
        let mut p = st;
        if p + 8 <= s.len() { self.prg.copy_from_slice(&s[p..p+8]); p += 8; }
        if p < s.len() { self.hw.irq_mask = s[p]; p += 1; }
        if p < s.len() { self.hw.irq_status = s[p]; p += 1; }
        if p < s.len() { self.hw.timer_control = s[p]; p += 1; }
        if p < s.len() { self.hw.timer_latch = s[p]; p += 1; }
        if p < s.len() { self.hw.timer_value = s[p]; p += 1; }
        if p < s.len() { self.hw.reg_4016 = s[p]; p += 1; }
        if p < s.len() { self.hw.reg_4026 = s[p]; p += 1; }
        p
    }
}
