use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};
const X24C0X_STANDBY: u8 = 0;
const X24C0X_ADDRESS: u8 = 1;
const X24C0X_WORD: u8 = 2;
const X24C0X_READ: u8 = 3;
const X24C0X_WRITE: u8 = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BandaiKind {
    Mapper16,
    Mapper159,
    Mapper153,
    Mapper157,
}

pub fn prg_ram_size(mapper_id: u8) -> usize {
    if mapper_id == 153 {
        8192
    } else {
        0
    }
}
struct X24c01 {
    state: u8,
    addr: u8,
    word: u8,
    latch: u8,
    bitcount: u8,
    sda: u8,
    scl: u8,
    out: u8,
}

impl X24c01 {
    fn new() -> Self {
        Self {
            state: X24C0X_STANDBY,
            addr: 0,
            word: 0,
            latch: 0,
            bitcount: 0,
            sda: 0,
            scl: 0,
            out: 0,
        }
    }

    fn init(&mut self) {
        *self = Self::new();
    }

    fn write(&mut self, data: u8, eeprom: &mut [u8; 512]) {
        let scl = (data >> 5) & 1;
        let sda = (data >> 6) & 1;
        if self.scl != 0 && scl != 0 {
            if self.sda != 0 && sda == 0 {
                self.state = X24C0X_ADDRESS;
                self.bitcount = 0;
                self.addr = 0;
            } else if self.sda == 0 && sda != 0 {
                self.state = X24C0X_STANDBY;
            }
        } else if self.scl == 0 && scl != 0 {
            match self.state {
                X24C0X_ADDRESS => {
                    if self.bitcount < 7 {
                        self.addr = (self.addr << 1) | sda;
                    } else {
                        self.word = self.addr;
                        if sda != 0 {
                            self.state = X24C0X_READ;
                        } else {
                            self.state = X24C0X_WRITE;
                        }
                    }
                    self.bitcount += 1;
                }
                X24C0X_READ => {
                    if self.bitcount == 8 {
                        self.out = 0;
                        self.latch = eeprom[self.word as usize];
                        self.bitcount = 0;
                    } else {
                        self.out = self.latch >> 7;
                        self.latch <<= 1;
                        self.bitcount += 1;
                        if self.bitcount == 8 {
                            self.word = self.word.wrapping_add(1);
                        }
                    }
                }
                X24C0X_WRITE => {
                    if self.bitcount == 8 {
                        self.out = 0;
                        self.latch = 0;
                        self.bitcount = 0;
                    } else {
                        self.latch = (self.latch << 1) | sda;
                        self.bitcount += 1;
                        if self.bitcount == 8 {
                            eeprom[self.word as usize] = self.latch;
                            self.word = self.word.wrapping_add(1);
                        }
                    }
                }
                _ => {}
            }
        }
        self.sda = sda;
        self.scl = scl;
    }
}
struct X24c02 {
    state: u8,
    addr: u8,
    word: u8,
    latch: u8,
    bitcount: u8,
    sda: u8,
    scl: u8,
    out: u8,
}

impl X24c02 {
    fn new() -> Self {
        Self {
            state: X24C0X_STANDBY,
            addr: 0,
            word: 0,
            latch: 0,
            bitcount: 0,
            sda: 0,
            scl: 0,
            out: 0,
        }
    }

    fn init(&mut self) {
        *self = Self::new();
    }

    fn write(&mut self, data: u8, eeprom: &mut [u8; 512]) {
        let scl = (data >> 5) & 1;
        let sda = (data >> 6) & 1;
        if self.scl != 0 && scl != 0 {
            if self.sda != 0 && sda == 0 {
                self.state = X24C0X_ADDRESS;
                self.bitcount = 0;
                self.addr = 0;
            } else if self.sda == 0 && sda != 0 {
                self.state = X24C0X_STANDBY;
            }
        } else if self.scl == 0 && scl != 0 {
            match self.state {
                X24C0X_ADDRESS => {
                    if self.bitcount < 7 {
                        self.addr = (self.addr << 1) | sda;
                    } else if (self.addr & 0x78) != 0x50 {
                        self.out = 1;
                        self.state = X24C0X_STANDBY;
                    } else if sda != 0 {
                        self.state = X24C0X_READ;
                    } else {
                        self.state = X24C0X_WORD;
                    }
                    self.bitcount += 1;
                }
                X24C0X_WORD => {
                    if self.bitcount == 8 {
                        self.word = 0;
                        self.out = 0;
                    } else {
                        self.word = (self.word << 1) | sda;
                        if self.bitcount == 16 {
                            self.bitcount = 7;
                            self.state = X24C0X_WRITE;
                        }
                    }
                    self.bitcount += 1;
                }
                X24C0X_READ => {
                    if self.bitcount == 8 {
                        self.out = 0;
                        self.latch = eeprom[self.word as usize | 0x100];
                        self.bitcount = 0;
                    } else {
                        self.out = self.latch >> 7;
                        self.latch <<= 1;
                        self.bitcount += 1;
                        if self.bitcount == 8 {
                            self.word = self.word.wrapping_add(1);
                        }
                    }
                }
                X24C0X_WRITE => {
                    if self.bitcount == 8 {
                        self.out = 0;
                        self.latch = 0;
                        self.bitcount = 0;
                    } else {
                        self.latch = (self.latch << 1) | sda;
                        self.bitcount += 1;
                        if self.bitcount == 8 {
                            eeprom[self.word as usize | 0x100] = self.latch;
                            self.word = self.word.wrapping_add(1);
                        }
                    }
                }
                _ => {}
            }
        }
        self.sda = sda;
        self.scl = scl;
    }
}

pub struct MapperBandai {
    kind: BandaiKind,
    reg: [u8; 16],
    irq_enable: bool,
    irq_count: i16,
    irq_latch: u16,
    x24c0x_data: [u8; 512],
    x24c01: X24c01,
    x24c02: X24c02,
    use_x24c02: bool,
    barcode_data: [u8; 256],
    barcode_read_pos: usize,
    barcode_cycle_count: i32,
    barcode_out: u8,
    irq_ack_pending: bool,
}

impl MapperBandai {
    pub fn new(kind: BandaiKind) -> Self {
        let use_x24c02 = matches!(kind, BandaiKind::Mapper16 | BandaiKind::Mapper157);
        let mut m = Self {
            kind,
            reg: [0; 16],
            irq_enable: false,
            irq_count: 0,
            irq_latch: 0,
            x24c0x_data: [0; 512],
            x24c01: X24c01::new(),
            x24c02: X24c02::new(),
            use_x24c02,
            barcode_data: [0; 256],
            barcode_read_pos: 0,
            barcode_cycle_count: 0,
            barcode_out: 0,
            irq_ack_pending: false,
        };
        if matches!(kind, BandaiKind::Mapper157) {
            m.barcode_data[0] = 0xFF;
        }
        m
    }

    fn is153(&self) -> bool {
        self.kind == BandaiKind::Mapper153
    }

    fn is157(&self) -> bool {
        self.kind == BandaiKind::Mapper157
    }

    fn mirror_address(&self, address: u16) -> u16 {
        match self.reg[9] & 3 {
            0 => address & 0x37FF,
            1 => (address & 0x33FF) | ((address & 0x0800) >> 1),
            2 => address & 0x33FF,
            3 => (address & 0x33FF) | 0x0400,
            _ => address & 0x37FF,
        }
    }

    fn chr_bank_index(&self, address: u16) -> usize {
        if self.is153() || self.is157() {
            0
        } else {
            let slot = (address >> 10) as usize & 7;
            self.reg[slot] as usize
        }
    }

    fn read_chr(&self, address: u16, chr_rom: &[u8], chr_ram: &[u8]) -> u8 {
        let len = if !chr_ram.is_empty() {
            chr_ram.len()
        } else {
            chr_rom.len()
        };
        if len == 0 {
            return 0;
        }
        let bank = self.chr_bank_index(address);
        let offset = bank * 0x400 + (address as usize & 0x3FF);
        if !chr_ram.is_empty() {
            chr_ram[offset % len]
        } else {
            chr_rom[offset % len]
        }
    }

    fn chr_write_offset(&self, address: u16, len: usize) -> usize {
        let bank = self.chr_bank_index(address);
        (bank * 0x400 + (address as usize & 0x3FF)) % len
    }

    fn prg_bank_16k(&self, address: u16, prg_banks: usize) -> usize {
        if address < 0xC000 {
            if self.is153() {
                let base = (self.reg[0] & 1) as usize * 16;
                ((self.reg[8] & 0x0F) as usize | base) % prg_banks
            } else {
                (self.reg[8] as usize) % prg_banks
            }
        } else if self.is153() {
            let base = (self.reg[0] & 1) as usize * 16;
            (0x0F | base) % prg_banks
        } else {
            prg_banks.saturating_sub(1)
        }
    }

    fn eeprom_read_bit(&self) -> u8 {
        if self.use_x24c02 {
            self.x24c02.out
        } else {
            self.x24c01.out
        }
    }

    fn bandai_read_low(&self, open_bus: u8) -> u8 {
        let eeprom_bit = self.eeprom_read_bit();
        if self.is157() {
            (open_bus & 0xE7) | ((eeprom_bit & 1) << 4) | self.barcode_out
        } else {
            (open_bus & 0xEF) | ((eeprom_bit & 1) << 4)
        }
    }

    fn write_register(&mut self, reg: u8, data: u8) {
        if reg < 0x0A {
            self.reg[reg as usize] = data;
            return;
        }
        match reg {
            0x0A => {
                self.irq_ack_pending = true;
                self.irq_enable = (data & 1) != 0;
                self.irq_count = self.irq_latch as i16;
            }
            0x0B => {
                self.irq_latch = (self.irq_latch & 0xFF00) | u16::from(data);
            }
            0x0C => {
                self.irq_latch = (self.irq_latch & 0x00FF) | (u16::from(data) << 8);
            }
            0x0D => {
                if self.use_x24c02 {
                    self.x24c02.write(data, &mut self.x24c0x_data);
                } else {
                    self.x24c01.write(data, &mut self.x24c0x_data);
                }
            }
            _ => {}
        }
    }

    fn barcode_sync(&mut self) {
        let _ = self.reg[8];
    }

    fn barcode_write(&mut self, reg: u8, data: u8) {
        match reg {
            0x00 => {
                self.reg[0] = (data & 8) << 2;
                self.x24c01
                    .write(self.reg[0x0D] | self.reg[0], &mut self.x24c0x_data);
            }
            0x08 | 0x09 => {
                self.reg[reg as usize] = data;
                self.barcode_sync();
            }
            0x0A => {
                self.irq_ack_pending = true;
                self.irq_enable = (data & 1) != 0;
                self.irq_count = self.irq_latch as i16;
            }
            0x0B => {
                self.irq_latch = (self.irq_latch & 0xFF00) | u16::from(data);
            }
            0x0C => {
                self.irq_latch = (self.irq_latch & 0x00FF) | (u16::from(data) << 8);
            }
            0x0D => {
                self.reg[0x0D] = data & !0x20;
                self.x24c01
                    .write(self.reg[0x0D] | self.reg[0], &mut self.x24c0x_data);
                self.x24c02.write(data, &mut self.x24c0x_data);
            }
            _ => {}
        }
    }

    fn tick_barcode(&mut self) {
        self.barcode_cycle_count += 1;
        if self.barcode_cycle_count >= 1000 {
            self.barcode_cycle_count -= 1000;
            if self.barcode_data[self.barcode_read_pos] == 0xFF {
                self.barcode_out = 0;
            } else {
                self.barcode_out = (self.barcode_data[self.barcode_read_pos] ^ 1) << 3;
                self.barcode_read_pos += 1;
            }
        }
    }

    fn save_slice(&self) -> &[u8] {
        match self.kind {
            BandaiKind::Mapper16 => &self.x24c0x_data[256..512],
            BandaiKind::Mapper159 => &self.x24c0x_data[0..128],
            BandaiKind::Mapper157 => &self.x24c0x_data[..512],
            BandaiKind::Mapper153 => &[],
        }
    }
}
#[allow(dead_code)]
pub fn datach_set_barcode(mapper: &mut MapperBandai, rcode: &[u8]) -> bool {
    if mapper.kind != BandaiKind::Mapper157 {
        return false;
    }
    let prefix_parity_type: [[i32; 6]; 10] = [
        [0, 0, 0, 0, 0, 0],
        [0, 0, 1, 0, 1, 1],
        [0, 0, 1, 1, 0, 1],
        [0, 0, 1, 1, 1, 0],
        [0, 1, 0, 0, 1, 1],
        [0, 1, 1, 0, 0, 1],
        [0, 1, 1, 1, 0, 0],
        [0, 1, 0, 1, 0, 1],
        [0, 1, 0, 1, 1, 0],
        [0, 1, 1, 0, 1, 0],
    ];
    let data_left_odd: [[i32; 7]; 10] = [
        [0, 0, 0, 1, 1, 0, 1],
        [0, 0, 1, 1, 0, 0, 1],
        [0, 0, 1, 0, 0, 1, 1],
        [0, 1, 1, 1, 1, 0, 1],
        [0, 1, 0, 0, 0, 1, 1],
        [0, 1, 1, 0, 0, 0, 1],
        [0, 1, 0, 1, 1, 1, 1],
        [0, 1, 1, 1, 0, 1, 1],
        [0, 1, 1, 0, 1, 1, 1],
        [0, 0, 0, 1, 0, 1, 1],
    ];
    let data_left_even: [[i32; 7]; 10] = [
        [0, 1, 0, 0, 1, 1, 1],
        [0, 1, 1, 0, 0, 1, 1],
        [0, 0, 1, 1, 0, 1, 1],
        [0, 1, 0, 0, 0, 0, 1],
        [0, 0, 1, 1, 1, 0, 1],
        [0, 1, 1, 1, 0, 0, 1],
        [0, 0, 0, 0, 1, 0, 1],
        [0, 0, 1, 0, 0, 0, 1],
        [0, 0, 0, 1, 0, 0, 1],
        [0, 0, 1, 0, 1, 1, 1],
    ];
    let data_right: [[i32; 7]; 10] = [
        [1, 1, 1, 0, 0, 1, 0],
        [1, 1, 0, 0, 1, 1, 0],
        [1, 1, 0, 1, 1, 0, 0],
        [1, 0, 0, 0, 0, 1, 0],
        [1, 0, 1, 1, 1, 0, 0],
        [1, 0, 0, 1, 1, 1, 0],
        [1, 0, 1, 0, 0, 0, 0],
        [1, 0, 0, 0, 1, 0, 0],
        [1, 0, 0, 1, 0, 0, 0],
        [1, 1, 1, 0, 1, 0, 0],
    ];
    let mut code = [0u8; 13];
    let mut len = 0usize;
    for i in 0..13 {
        if i >= rcode.len() || rcode[i] == 0 {
            break;
        }
        let digit = rcode[i].wrapping_sub(b'0');
        if digit > 9 {
            return false;
        }
        code[i] = digit;
        len += 1;
    }
    if !matches!(len, 7 | 8 | 12 | 13) {
        return false;
    }
    let mut barcode_data = [0u8; 256];
    let mut tmp_p = 0usize;
    let mut push_bit = |bit: u8| {
        if tmp_p < barcode_data.len() {
            barcode_data[tmp_p] = bit;
            tmp_p += 1;
        }
    };
    for _ in 0..32 {
        push_bit(0);
    }
    push_bit(1);
    push_bit(0);
    push_bit(1);
    if len == 13 || len == 12 {
        for i in 0..6 {
            let table = if prefix_parity_type[code[0] as usize][i] != 0 {
                data_left_even
            } else {
                data_left_odd
            };
            for j in 0..7 {
                push_bit(table[code[i + 1] as usize][j] as u8);
            }
        }
        push_bit(0);
        push_bit(1);
        push_bit(0);
        push_bit(1);
        push_bit(0);
        for i in 7..12 {
            for j in 0..7 {
                push_bit(data_right[code[i] as usize][j] as u8);
            }
        }
        let mut csum = 0u32;
        if len == 12 {
            for i in 0..12 {
                csum += u32::from(code[i]) * if (i & 1) != 0 { 3 } else { 1 };
            }
            code[12] = ((10 - (csum % 10)) % 10) as u8;
        }
        for j in 0..7 {
            push_bit(data_right[code[12] as usize][j] as u8);
        }
    } else {
        for i in 0..4 {
            for j in 0..7 {
                push_bit(data_left_odd[code[i] as usize][j] as u8);
            }
        }
        push_bit(0);
        push_bit(1);
        push_bit(0);
        push_bit(1);
        push_bit(0);
        for i in 4..7 {
            for j in 0..7 {
                push_bit(data_right[code[i] as usize][j] as u8);
            }
        }
        let mut csum = 0u32;
        for i in 0..7 {
            csum += if (i & 1) != 0 {
                u32::from(code[i])
            } else {
                u32::from(code[i]) * 3
            };
        }
        let check = ((10 - (csum % 10)) % 10) as u8;
        for j in 0..7 {
            push_bit(data_right[check as usize][j] as u8);
        }
    }
    push_bit(1);
    push_bit(0);
    push_bit(1);
    for _ in 0..32 {
        push_bit(0);
    }
    push_bit(0xFF);
    mapper.barcode_data = barcode_data;
    mapper.barcode_read_pos = 0;
    mapper.barcode_out = 0x8;
    mapper.barcode_cycle_count = 0;
    true
}

impl Mapper for MapperBandai {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x8000 {
            let prg_banks = cart.prg_rom.len() / 0x4000;
            if prg_banks == 0 {
                return FetchResult { data: 0, driven: false };
            }
            let bank = self.prg_bank_16k(address, prg_banks);
            let offset = bank * 0x4000 + (address as usize & 0x3FFF);
            FetchResult {
                data: cart.prg_rom[offset % cart.prg_rom.len()],
                driven: true,
            }
        } else if address >= 0x6000 && address < 0x8000 {
            if self.is153() {
                if cart.prg_ram.is_empty() {
                    return FetchResult { data: 0, driven: false };
                }
                let off = (address as usize - 0x6000) % cart.prg_ram.len();
                FetchResult {
                    data: cart.prg_ram[off],
                    driven: true,
                }
            } else {
                FetchResult {
                    data: self.bandai_read_low(0),
                    driven: true,
                }
            }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if self.is153() && address >= 0x6000 && address < 0x8000 {
            if !cart.prg_ram.is_empty() {
                let off = (address as usize - 0x6000) % cart.prg_ram.len();
                cart.prg_ram[off] = data;
            }
            return;
        }
        let writes_registers = if self.is157() {
            address >= 0x8000
        } else {
            address >= 0x6000
        };
        if writes_registers {
            let reg = (address & 0x0F) as u8;
            if self.is157() {
                self.barcode_write(reg, data);
            } else {
                self.write_register(reg, data);
            }
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
        let address = (ppu_address_bus & 0x3F00) | u16::from(ppu_octal_latch);
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if address < 0x2000 {
            let byte = if self.is153() || self.is157() {
                if using_chr_ram && !chr_ram.is_empty() {
                    let off = (address as usize & 0x1FFF) % chr_ram.len();
                    chr_ram[off]
                } else if !chr_rom.is_empty() {
                    let off = (address as usize & 0x1FFF) % chr_rom.len();
                    chr_rom[off]
                } else {
                    0
                }
            } else {
                self.read_chr(address, chr_rom, chr_ram)
            };
            new_addr_bus |= u16::from(byte);
        } else {
            let mirrored = self.mirror_address(address);
            new_addr_bus |= u16::from(vram[(mirrored & 0x7FF) as usize]);
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 {
            if self.is153() || self.is157() {
                if !cart.chr_ram.is_empty() {
                    let off = (address as usize & 0x1FFF) % cart.chr_ram.len();
                    cart.chr_ram[off] = data;
                }
            } else if !cart.chr_ram.is_empty() {
                let len = cart.chr_ram.len();
                let offset = self.chr_write_offset(address, len);
                cart.chr_ram[offset] = data;
            }
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_address(address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    fn cpu_clock(&mut self, cycles: u8) -> bool {
        if self.is157() {
            self.tick_barcode();
        }
        if self.irq_enable {
            self.irq_count -= cycles as i16;
            if self.irq_count < 0 {
                self.irq_enable = false;
                self.irq_count = -1;
                return true;
            }
        }
        false
    }

    fn take_irq_ack(&mut self) -> bool {
        let ack = self.irq_ack_pending;
        self.irq_ack_pending = false;
        ack
    }

    fn reset(&mut self) {
        self.irq_enable = false;
        self.irq_count = 0;
        if self.use_x24c02 {
            self.x24c02.init();
        } else {
            self.x24c01.init();
        }
        if self.is157() {
            self.x24c01.init();
            self.x24c02.init();
            self.barcode_data[0] = 0xFF;
            self.barcode_read_pos = 0;
            self.barcode_out = 0;
            self.barcode_cycle_count = 0;
        }
    }

    fn battery_save_data(&self, cart: &Cartridge) -> Option<Vec<u8>> {
        match self.kind {
            BandaiKind::Mapper153 => {
                if cart.prg_ram.is_empty() {
                    None
                } else {
                    Some(cart.prg_ram.clone())
                }
            }
            _ => {
                let slice = self.save_slice();
                if slice.is_empty() {
                    None
                } else {
                    Some(slice.to_vec())
                }
            }
        }
    }

    fn load_battery_save(&mut self, cart: &mut Cartridge, data: &[u8]) {
        match self.kind {
            BandaiKind::Mapper153 => {
                let copy_len = data.len().min(cart.prg_ram.len());
                cart.prg_ram[..copy_len].copy_from_slice(&data[..copy_len]);
            }
            BandaiKind::Mapper16 => {
                let copy_len = data.len().min(256);
                self.x24c0x_data[256..256 + copy_len]
                    .copy_from_slice(&data[..copy_len]);
            }
            BandaiKind::Mapper159 => {
                let copy_len = data.len().min(128);
                self.x24c0x_data[..copy_len].copy_from_slice(&data[..copy_len]);
            }
            BandaiKind::Mapper157 => {
                let copy_len = data.len().min(512);
                self.x24c0x_data[..copy_len].copy_from_slice(&data[..copy_len]);
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut state = Vec::new();
        state.extend_from_slice(&self.reg);
        state.push(u8::from(self.irq_enable));
        state.extend_from_slice(&self.irq_count.to_le_bytes());
        state.extend_from_slice(&self.irq_latch.to_le_bytes());
        state.extend_from_slice(&self.x24c0x_data);
        state.push(self.x24c01.state);
        state.extend_from_slice(&[
            self.x24c01.addr,
            self.x24c01.word,
            self.x24c01.latch,
            self.x24c01.bitcount,
            self.x24c01.sda,
            self.x24c01.scl,
            self.x24c01.out,
        ]);
        state.push(self.x24c02.state);
        state.extend_from_slice(&[
            self.x24c02.addr,
            self.x24c02.word,
            self.x24c02.latch,
            self.x24c02.bitcount,
            self.x24c02.sda,
            self.x24c02.scl,
            self.x24c02.out,
        ]);
        state.extend_from_slice(&self.barcode_data);
        state.extend_from_slice(&(self.barcode_read_pos as u32).to_le_bytes());
        state.extend_from_slice(&self.barcode_cycle_count.to_le_bytes());
        state.push(self.barcode_out);
        state
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut start: usize) -> usize {
        if start + 16 <= state.len() {
            self.reg.copy_from_slice(&state[start..start + 16]);
            start += 16;
        }
        if start + 5 <= state.len() {
            self.irq_enable = state[start] != 0;
            self.irq_count = i16::from_le_bytes([state[start + 1], state[start + 2]]);
            self.irq_latch =
                u16::from_le_bytes([state[start + 3], state[start + 4]]);
            start += 5;
        }
        if start + 512 <= state.len() {
            self.x24c0x_data.copy_from_slice(&state[start..start + 512]);
            start += 512;
        }
        if start + 8 <= state.len() {
            self.x24c01.state = state[start];
            self.x24c01.addr = state[start + 1];
            self.x24c01.word = state[start + 2];
            self.x24c01.latch = state[start + 3];
            self.x24c01.bitcount = state[start + 4];
            self.x24c01.sda = state[start + 5];
            self.x24c01.scl = state[start + 6];
            self.x24c01.out = state[start + 7];
            start += 8;
        }
        if start + 8 <= state.len() {
            self.x24c02.state = state[start];
            self.x24c02.addr = state[start + 1];
            self.x24c02.word = state[start + 2];
            self.x24c02.latch = state[start + 3];
            self.x24c02.bitcount = state[start + 4];
            self.x24c02.sda = state[start + 5];
            self.x24c02.scl = state[start + 6];
            self.x24c02.out = state[start + 7];
            start += 8;
        }
        if start + 256 <= state.len() {
            self.barcode_data.copy_from_slice(&state[start..start + 256]);
            start += 256;
        }
        if start + 9 <= state.len() {
            self.barcode_read_pos =
                u32::from_le_bytes([state[start], state[start + 1], state[start + 2], state[start + 3]])
                    as usize;
            self.barcode_cycle_count = i32::from_le_bytes([
                state[start + 4],
                state[start + 5],
                state[start + 6],
                state[start + 7],
            ]);
            self.barcode_out = state[start + 8];
            start += 9;
        }
        start
    }
}
