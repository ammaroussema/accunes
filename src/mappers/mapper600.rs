use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
use crate::mappers::um6578::Um6578Hw;

pub struct Mapper600 {
    hw: Um6578Hw,
}
impl Default for Mapper600 {
    fn default() -> Self { Self::new() }
}
impl Mapper600 {
    pub fn new() -> Self {
        Self { hw: Um6578Hw::new() }
    }
    fn get_bank(&self, slot: usize, len: usize) -> usize {
        let n = (len / 0x1000).max(1);
        let bank_val = (self.hw.prg[slot & 7] as usize) % n;
        let a20 = ((self.hw.reg4016 as usize) << 7) & 0x100;
        (bank_val | a20) % ((len + 0xFFF) / 0x1000).max(1)
    }
}
impl Mapper for Mapper600 {
    fn is_um6578(&self) -> bool { true }
    fn reset(&mut self) { self.hw.reset(); }
    fn store_prg(&mut self, c: &mut Cartridge, a: u16, d: u8) {
        if a >= 0x6000 && a < 0x8000 && !c.prg_ram.is_empty() {
            let offset = (a as usize - 0x6000) % c.prg_ram.len();
            c.prg_ram[offset] = d;
            return;
        }
        self.hw.write_register(a, d);
    }
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if let Some(data) = self.hw.read_register(address) {
            return FetchResult { data, driven: true };
        }
        if address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let offset = (address as usize - 0x6000) % cart.prg_ram.len();
                return FetchResult { data: cart.prg_ram[offset], driven: true };
            }
            return FetchResult { data: 0, driven: true };
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
    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        if self.hw.timer_control & 0x20 == 0 {
            self.hw.clock_timer();
        }
        self.hw.irq_pending()
    }
    fn cpu_clock_irq_level(&self) -> bool { true }
    fn ppu_clock(&mut self, _ppu_address_bus: u16, _ppu_a12_prev: bool, scanline: u16, dot: u16, _ppu_sprite_x16: bool, _rendering_on: bool) -> bool {
        if self.hw.timer_control & 0x20 != 0 && dot >= 330 {
            let sl = scanline as i32;
            if sl != self.hw.prev_scanline {
                self.hw.prev_scanline = sl;
                self.hw.clock_timer();
            }
        }
        false
    }
    fn save_mapper_registers(&self, _c: &Cartridge) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&self.hw.prg);
        v.push(self.hw.pcm);
        v.push(self.hw.irq_mask);
        v.push(self.hw.irq_status);
        v.push(self.hw.timer_control);
        v.push(self.hw.timer_latch);
        v.push(self.hw.timer_value);
        v.push(self.hw.prescaler);
        v.push(self.hw.reg4016);
        v.push(self.hw.reg4026);
        v
    }
    fn load_mapper_registers(&mut self, _c: &mut Cartridge, s: &[u8], st: usize) -> usize {
        let mut p = st;
        if p + 8 <= s.len() { self.hw.prg.copy_from_slice(&s[p..p+8]); p += 8; }
        if p < s.len() { self.hw.pcm = s[p]; p += 1; }
        if p < s.len() { self.hw.irq_mask = s[p]; p += 1; }
        if p < s.len() { self.hw.irq_status = s[p]; p += 1; }
        if p < s.len() { self.hw.timer_control = s[p]; p += 1; }
        if p < s.len() { self.hw.timer_latch = s[p]; p += 1; }
        if p < s.len() { self.hw.timer_value = s[p]; p += 1; }
        if p < s.len() { self.hw.prescaler = s[p]; p += 1; }
        if p < s.len() { self.hw.reg4016 = s[p]; p += 1; }
        if p < s.len() { self.hw.reg4026 = s[p]; p += 1; }
        p
    }
}
