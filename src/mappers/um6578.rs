#[derive(Clone, Copy, Debug)]
pub struct Um6578Hw {
    pub irq_mask: u8,      
    pub irq_status: u8,    
    pub timer_control: u8, 
    pub timer_latch: u8, 
    pub timer_value: u8, 
    prescaler: u8,
    pub reg_4016: u8,
    pub reg_4026: u8,
    prev_scanline: u16,
}

impl Um6578Hw {
    pub fn new() -> Self {
        Self {
            irq_mask: 0xFF,
            irq_status: 0,
            timer_control: 0,
            timer_latch: 0,
            timer_value: 0,
            prescaler: 0,
            reg_4016: 0,
            reg_4026: 0x80,
            prev_scanline: 0xFFFF,
        }
    }

    pub fn reset(&mut self) {
        self.irq_mask = 0xFF;
        self.irq_status = 0;
        self.timer_control = 0;
        self.timer_latch = 0;
        self.timer_value = 0;
        self.prescaler = 0;
        self.reg_4016 = 0;
        self.reg_4026 = 0x80;
        self.prev_scanline = 0xFFFF;
    }

    fn clock_timer(&mut self) -> bool {
        let prescale = (self.timer_control & 0x0F) as u16 + 1;
        self.prescaler = self.prescaler.wrapping_add(1);
        if (self.prescaler as u16) < prescale {
            return false;
        }
        self.prescaler = 0;
        let latch = self.timer_latch as u16;
        let v = (self.timer_value as u16).wrapping_add(1);
        if v > latch {
            self.timer_value = 0;
            if self.timer_control & 0x80 != 0 {
                self.irq_status |= 0x80;
            }
            if self.timer_control & 0x40 == 0 {
                self.timer_control &= !0x80;
            }
            return true;
        }
        self.timer_value = v as u8;
        false
    }

    pub fn write_reg(&mut self, addr: u16, data: u8) -> bool {
        match address_low(addr) {
            0x32 => { self.irq_mask = data; self.irq_status &= !data; true }
            0x33 => true,
            0x34 => {
                self.timer_control = data;
                if data & 0x80 == 0 { self.irq_status &= !0x80; }
                true
            }
            0x35 => {
                self.timer_latch = data;
                self.timer_value = 0;
                self.irq_status &= !0x80;
                true
            }
            0x36 => { self.timer_value = data; true }
            0x16 => { self.reg_4016 = data; false }
            0x26 => { self.reg_4026 = data; false }
            _ => false,
        }
    }

    pub fn cpu_cycle(&mut self) -> bool {
        if self.timer_control & 0x20 == 0 {
            self.clock_timer();
        }
        (self.irq_status & !self.irq_mask) != 0
    }

    pub fn ppu_cycle(&mut self, scanline: u16, dot: u16, _rendering: bool) -> bool {
        if (self.timer_control & 0x20) != 0 && dot >= 330 && scanline != self.prev_scanline {
            self.prev_scanline = scanline;
            self.clock_timer();
        }
        (self.irq_status & !self.irq_mask) != 0
    }

    pub fn take_irq(&mut self) -> bool {
        let v = (self.irq_status & !self.irq_mask) != 0;
        if v { self.irq_status &= !0x80; }
        v
    }
}

fn address_low(addr: u16) -> u16 {
    addr & 0x3F
}
