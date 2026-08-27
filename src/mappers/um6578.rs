pub const UM6578_IRQ_TIMER: u8 = 0x80;

#[derive(Clone, Debug)]
pub struct Um6578Hw {
    pub prg: [u8; 8],
    pub chr: u8,
    pub pcm: u8,
    pub irq_mask: u8,
    pub irq_status: u8,
    pub timer_control: u8,
    pub timer_latch: u8,
    pub timer_value: u8,
    pub prescaler: u8,
    pub prev_scanline: i32,
    pub reg4016: u8,
    pub reg4026: u8,
}

impl Default for Um6578Hw {
    fn default() -> Self {
        Self::new()
    }
}

impl Um6578Hw {
    pub fn new() -> Self {
        Self {
            prg: [0; 8],
            chr: 0,
            pcm: 0,
            irq_mask: 0xFF,
            irq_status: 0x00,
            timer_control: 0,
            timer_latch: 0,
            timer_value: 0,
            prescaler: 0,
            prev_scanline: 0,
            reg4016: 0x00,
            reg4026: 0x80,
        }
    }

    pub fn reset(&mut self) {
        for i in 0..8 {
            self.prg[i] = i as u8;
        }
        self.chr = 0;
        self.pcm = 0;
        self.irq_mask = 0xFF;
        self.irq_status = 0x00;
        self.timer_control = 0;
        self.timer_latch = 0;
        self.timer_value = 0;
        self.prescaler = 0;
        self.prev_scanline = 0;
        self.reg4016 = 0x00;
        self.reg4026 = 0x80;
    }

    pub fn clock_timer(&mut self) {
        self.prescaler = self.prescaler.wrapping_add(1);
        if self.prescaler >= (self.timer_control & 0x0F) + 1 {
            self.prescaler = 0;
            self.timer_value = self.timer_value.wrapping_add(1);
            if self.timer_value >= self.timer_latch.wrapping_add(1) {
                self.timer_value = 0;
                if self.timer_control & 0x80 != 0 {
                    self.irq_status |= UM6578_IRQ_TIMER;
                }
                if self.timer_control & 0x40 == 0 {
                    self.timer_control &= !0x80;
                }
            }
        }
    }

    pub fn irq_pending(&self) -> bool {
        self.irq_status & !self.irq_mask != 0
    }

    pub fn write_register(&mut self, addr: u16, val: u8) {
        match addr {
            0x4016 => {
                self.reg4016 = val;
            }
            0x4026 => {
                self.reg4026 = val;
            }
            0x4027 => {
                self.pcm = val;
            }
            0x4032 => {
                self.irq_mask = val;
                self.irq_status &= !self.irq_mask;
            }
            0x4034 => {
                self.timer_control = val;
                if self.timer_control & 0x80 == 0 {
                    self.irq_status &= !UM6578_IRQ_TIMER;
                }
            }
            0x4035 => {
                self.timer_latch = val;
                self.timer_value = 0;
                self.irq_status &= !UM6578_IRQ_TIMER;
            }
            0x4040..=0x4047 => {
                self.prg[(addr & 7) as usize] = val;
            }
            0x4300 => {
                self.chr = val;
            }
            _ => {}
        }
    }

    pub fn read_register(&self, addr: u16) -> Option<u8> {
        match addr {
            0x4026 => Some(self.reg4026),
            0x4027 => Some(self.pcm),
            0x4032 => Some(self.irq_mask),
            0x4033 => Some(self.irq_status),
            0x4034 => Some(self.timer_control),
            0x4035 => Some(self.timer_latch),
            0x4036 => Some(self.timer_value),
            0x4040..=0x4047 => Some(self.prg[(addr & 7) as usize]),
            _ => None,
        }
    }
}
