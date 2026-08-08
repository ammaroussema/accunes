use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};

#[derive(Clone, Copy, PartialEq, Eq)]
enum PrgWin {
    Rom(u16),
    Ram,
}

pub struct Mapper452 {
    latch_addr: u16,
    latch_data: u8,
    sub1: bool,
    windows: [PrgWin; 4],
    mirroring_h: bool,
}

impl Mapper452 {
    pub fn new(sub1: bool) -> Self {
        let mut m = Self {
            latch_addr: 0,
            latch_data: 0,
            sub1,
            windows: [PrgWin::Rom(0); 4],
            mirroring_h: false,
        };
        m.sync();
        m
    }

    fn sync(&mut self) {
        if self.sub1 {
            self.sync1();
        } else {
            self.sync0();
        }
    }

    fn sync0(&mut self) {
        let d = self.latch_data;
        let a = self.latch_addr;
        let win_a = ((d >> 3) & 6 | 8) as usize;
        if d & 0x02 != 0 {
            let page = a >> 1;
            self.windows = [
                PrgWin::Rom(page),
                PrgWin::Rom(page),
                PrgWin::Rom(page),
                PrgWin::Rom(page),
            ];
            self.windows[(win_a ^ 4) >> 1 & 3] = PrgWin::Ram;
            self.windows[(win_a >> 1) & 3] = PrgWin::Ram;
        } else if d & 0x08 != 0 {
            let base = a >> 1 & !1;
            let e = base | 3 | (d & 0x04) as u16 | (if d & 0x04 != 0 && d & 0x40 != 0 { 8 } else { 0 });
            self.windows[0] = PrgWin::Rom(base | 0);
            self.windows[1] = PrgWin::Rom(base | 1);
            self.windows[2] = PrgWin::Rom(base | 2);
            self.windows[3] = PrgWin::Rom(e);
            self.windows[(win_a >> 1) & 3] = PrgWin::Ram;
        } else {
            let page = a >> 2;
            self.windows[0] = PrgWin::Rom(page << 1);
            self.windows[1] = PrgWin::Rom((page << 1) | 1);
            self.windows[2] = PrgWin::Rom(0);
            self.windows[3] = PrgWin::Rom(1);
            self.windows[(win_a >> 1) & 3] = PrgWin::Ram;
        }
        self.mirroring_h = d & 0x01 != 0;
    }

    fn sync1(&mut self) {
        let a = self.latch_addr;
        match a & 0xF000 {
            0xA000 => {
                let page = a >> 1;
                self.windows[0] = PrgWin::Rom(page << 1);
                self.windows[1] = PrgWin::Rom((page << 1) | 1);
                self.windows[2] = PrgWin::Rom(0);
                let ram = ((a >> 8) & 6 | 8) as usize >> 1 & 3;
                self.windows[ram] = PrgWin::Ram;
            }
            0xC000 => {
                let p1 = a >> 1;
                let p2 = (a >> 1) | 1;
                self.windows[0] = PrgWin::Rom(p1 << 1);
                self.windows[1] = PrgWin::Rom((p1 << 1) | 1);
                self.windows[2] = PrgWin::Rom(p2 << 1);
                self.windows[3] = PrgWin::Rom((p2 << 1) | 1);
                let ram = ((a >> 8) & 6 | 8) as usize >> 1 & 3;
                self.windows[ram] = PrgWin::Ram;
            }
            0xD000 => {
                self.windows[0] = PrgWin::Rom(a);
                self.windows[1] = PrgWin::Rom(a);
                self.windows[2] = PrgWin::Rom(a);
                self.windows[3] = PrgWin::Rom(a);
                let r1 = ((a >> 8) & 2 | 8) as usize >> 1 & 3;
                let r2 = ((a >> 8) & 2 | 0xC) as usize >> 1 & 3;
                self.windows[r1] = PrgWin::Ram;
                self.windows[r2] = PrgWin::Ram;
            }
            0xE000 => {
                let page = a >> 1;
                let page2 = if a & 0x100 != 0 { (a >> 1) | 7 } else { 0 };
                self.windows[0] = PrgWin::Rom(page << 1);
                self.windows[1] = PrgWin::Rom((page << 1) | 1);
                self.windows[2] = PrgWin::Rom(page2 << 1);
                self.windows[3] = PrgWin::Rom((page2 << 1) | 1);
                let ram = ((a >> 8) & 6 | 8) as usize >> 1 & 3;
                self.windows[ram] = PrgWin::Ram;
            }
            _ => {
                let page = a >> 1;
                self.windows[0] = PrgWin::Rom(page << 1);
                self.windows[1] = PrgWin::Rom((page << 1) | 1);
                self.windows[2] = PrgWin::Rom(0);
                self.windows[3] = PrgWin::Rom(1);
            }
        }
        self.mirroring_h = a & 0x0800 != 0;
    }

    fn window_is_ram(&self, address: u16) -> bool {
        matches!(self.windows[((address >> 13) & 3) as usize], PrgWin::Ram)
    }
}

impl Mapper for Mapper452 {
    fn reset(&mut self) {
        self.latch_addr = 0;
        self.latch_data = 0;
        self.mirroring_h = false;
        self.sync();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            match self.windows[((address >> 13) & 3) as usize] {
                PrgWin::Ram => {
                    if cart.prg_ram.is_empty() {
                        return FetchResult {
                            data: 0,
                            driven: false,
                        };
                    }
                    let idx = (address as usize & 0x1FFF) % cart.prg_ram.len();
                    return FetchResult {
                        data: cart.prg_ram[idx],
                        driven: true,
                    };
                }
                PrgWin::Rom(bank) => {
                    let len = cart.prg_rom.len();
                    if len == 0 {
                        return FetchResult {
                            data: 0,
                            driven: true,
                        };
                    }
                    let offset = (bank as usize) * 0x2000 + (address as usize & 0x1FFF);
                    return FetchResult {
                        data: cart.prg_rom[offset % len],
                        driven: true,
                    };
                }
            }
        }
        FetchResult {
            data: 0,
            driven: false,
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x8000 {
            let bank = (address >> 12) as u8;
            let is_ram = self.window_is_ram(address);
            if is_ram {
                if !cart.prg_ram.is_empty() {
                    let idx = (address as usize & 0x1FFF) % cart.prg_ram.len();
                    cart.prg_ram[idx] = data;
                }
            }
            if !(self.sub1 && bank < 0xA) && !is_ram {
                self.latch_data = data;
                self.latch_addr = address;
                self.sync();
            }
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v(self.mirroring_h, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let address = (ppu_address_bus & 0x3F00) | ppu_octal_latch as u16;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = if !chr_ram.is_empty() {
                chr_ram[(address as usize & 0x1FFF) % chr_ram.len()]
            } else {
                0
            };
            new_addr_bus |= byte as u16;
        } else {
            let mirrored = mirror_h_or_v(self.mirroring_h, address);
            let byte = vram[(mirrored & 0x7FF) as usize];
            new_addr_bus |= byte as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                cart.chr_ram[(address as usize & 0x1FFF) % len] = data;
            }
        } else if (0x2000..0x3F00).contains(&address) {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.latch_addr.to_le_bytes());
        state.push(self.latch_data);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p + 1 < state.len() {
            self.latch_addr = u16::from_le_bytes([state[p], state[p + 1]]);
            p += 2;
        }
        if p < state.len() {
            self.latch_data = state[p];
            p += 1;
        }
        self.sync();
        p
    }
}
