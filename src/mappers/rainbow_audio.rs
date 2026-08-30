#[derive(Clone, Debug)]
pub struct Vrc6Pulse {
    volume: u8,
    duty_cycle: u8,
    ignore_duty: bool,
    frequency: u16,
    enabled: bool,
    timer: i32,
    step: u8,
    frequency_shift: u8,
}

impl Vrc6Pulse {
    pub fn new() -> Self {
        Self {
            volume: 0,
            duty_cycle: 0,
            ignore_duty: false,
            frequency: 1,
            enabled: false,
            timer: 1,
            step: 0,
            frequency_shift: 0,
        }
    }

    pub fn write_reg(&mut self, addr: u16, value: u8) {
        match addr & 0x03 {
            0 => {
                self.volume = value & 0x0F;
                self.duty_cycle = (value & 0x70) >> 4;
                self.ignore_duty = (value & 0x80) == 0x80;
            }
            1 => {
                self.frequency = (self.frequency & 0x0F00) | value as u16;
            }
            2 => {
                self.frequency = (self.frequency & 0x00FF) | (((value & 0x0F) as u16) << 8);
                self.enabled = (value & 0x80) == 0x80;
                if !self.enabled {
                    self.step = 0;
                }
            }
            _ => {}
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn set_frequency_shift(&mut self, shift: u8) {
        self.frequency_shift = shift;
    }

    pub fn clock(&mut self) {
        if self.enabled {
            self.timer -= 1;
            if self.timer <= 0 {
                self.step = (self.step + 1) & 0x0F;
                self.timer = ((self.frequency >> self.frequency_shift) as i32) + 1;
            }
        }
    }

    #[inline]
    pub fn get_volume(&self) -> u8 {
        if !self.enabled {
            0
        } else if self.ignore_duty {
            self.volume
        } else if self.step <= self.duty_cycle {
            self.volume
        } else {
            0
        }
    }
}

#[derive(Clone, Debug)]
pub struct Vrc6Saw {
    accumulator_rate: u8,
    accumulator: u8,
    frequency: u16,
    enabled: bool,
    timer: i32,
    step: u8,
    frequency_shift: u8,
}

impl Vrc6Saw {
    pub fn new() -> Self {
        Self {
            accumulator_rate: 0,
            accumulator: 0,
            frequency: 1,
            enabled: false,
            timer: 1,
            step: 0,
            frequency_shift: 0,
        }
    }

    pub fn write_reg(&mut self, addr: u16, value: u8) {
        match addr & 0x03 {
            0 => {
                self.accumulator_rate = value & 0x3F;
            }
            1 => {
                self.frequency = (self.frequency & 0x0F00) | value as u16;
            }
            2 => {
                self.frequency = (self.frequency & 0x00FF) | (((value & 0x0F) as u16) << 8);
                self.enabled = (value & 0x80) == 0x80;
                if !self.enabled {
                    self.accumulator = 0;
                    self.step = 0;
                }
            }
            _ => {}
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn set_frequency_shift(&mut self, shift: u8) {
        self.frequency_shift = shift;
    }

    pub fn clock(&mut self) {
        if self.enabled {
            self.timer -= 1;
            if self.timer <= 0 {
                self.step = (self.step + 1) % 14;
                self.timer = ((self.frequency >> self.frequency_shift) as i32) + 1;
                if self.step == 0 {
                    self.accumulator = 0;
                } else if (self.step & 0x01) == 0x00 {
                    self.accumulator = self.accumulator.wrapping_add(self.accumulator_rate);
                }
            }
        }
    }

    #[inline]
    pub fn get_volume(&self) -> u8 {
        if !self.enabled {
            0
        } else {
            self.accumulator >> 3
        }
    }
}

#[derive(Clone, Debug)]
pub struct RainbowAudio {
    pub pulse1: Vrc6Pulse,
    pub pulse2: Vrc6Pulse,
    pub saw: Vrc6Saw,
    pub output_exp_pin6: bool,
    pub output_exp_pin9: bool,
    pub output_to_4011: bool,
    pub volume: u8,
    pub last_output: u8,
}

impl RainbowAudio {
    pub fn new() -> Self {
        Self {
            pulse1: Vrc6Pulse::new(),
            pulse2: Vrc6Pulse::new(),
            saw: Vrc6Saw::new(),
            output_exp_pin6: false,
            output_exp_pin9: false,
            output_to_4011: false,
            volume: 0,
            last_output: 0,
        }
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.pulse1 = Vrc6Pulse::new();
        self.pulse2 = Vrc6Pulse::new();
        self.saw = Vrc6Saw::new();
        self.output_exp_pin6 = false;
        self.output_exp_pin9 = false;
        self.output_to_4011 = false;
        self.volume = 0;
        self.last_output = 0;
    }

    #[inline]
    pub fn get_last_output(&self) -> u8 {
        self.last_output
    }

    pub fn write_register(&mut self, addr: u16, value: u8) {
        let reg = addr & 0x0F;
        match reg {
            0x00..=0x02 => {
                self.pulse1.write_reg(reg, value);
            }
            0x03..=0x05 => {
                self.pulse2.write_reg(reg - 0x03, value);
            }
            0x06..=0x08 => {
                self.saw.write_reg(reg - 0x06, value);
            }
            0x09 => {
                self.output_exp_pin6 = (value & 0x01) != 0;
                self.output_exp_pin9 = (value & 0x02) != 0;
                self.output_to_4011 = (value & 0x04) != 0;
            }
            0x0A => {
                self.volume = value & 0x0F;
            }
            _ => {}
        }
    }

    pub fn clock(&mut self) {
        self.pulse1.clock();
        self.pulse2.clock();
        self.saw.clock();

        let output_level = self.pulse1.get_volume() + self.pulse2.get_volume() + self.saw.get_volume();
        self.last_output = output_level;
    }

    #[inline]
    pub fn sample(&self) -> f32 {
        if self.output_exp_pin6 || self.output_exp_pin9 {
            (self.last_output as f32 / 61.0) * (self.volume as f32 / 15.0)
        } else {
            0.0
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut s = Vec::with_capacity(32);
        s.push(self.pulse1.volume);
        s.push(self.pulse1.duty_cycle);
        s.push(if self.pulse1.ignore_duty { 1 } else { 0 });
        s.extend_from_slice(&self.pulse1.frequency.to_le_bytes());
        s.push(if self.pulse1.enabled { 1 } else { 0 });
        s.extend_from_slice(&self.pulse1.timer.to_le_bytes());
        s.push(self.pulse1.step);

        s.push(self.pulse2.volume);
        s.push(self.pulse2.duty_cycle);
        s.push(if self.pulse2.ignore_duty { 1 } else { 0 });
        s.extend_from_slice(&self.pulse2.frequency.to_le_bytes());
        s.push(if self.pulse2.enabled { 1 } else { 0 });
        s.extend_from_slice(&self.pulse2.timer.to_le_bytes());
        s.push(self.pulse2.step);

        s.push(self.saw.accumulator_rate);
        s.push(self.saw.accumulator);
        s.extend_from_slice(&self.saw.frequency.to_le_bytes());
        s.push(if self.saw.enabled { 1 } else { 0 });
        s.extend_from_slice(&self.saw.timer.to_le_bytes());
        s.push(self.saw.step);

        s.push(if self.output_exp_pin6 { 1 } else { 0 });
        s.push(if self.output_exp_pin9 { 1 } else { 0 });
        s.push(if self.output_to_4011 { 1 } else { 0 });
        s.push(self.volume);
        s.push(self.last_output);
        s
    }

    pub fn deserialize(&mut self, data: &[u8], offset: &mut usize) -> Result<(), ()> {
        if *offset + 30 > data.len() {
            return Err(());
        }
        self.pulse1.volume = data[*offset]; *offset += 1;
        self.pulse1.duty_cycle = data[*offset]; *offset += 1;
        self.pulse1.ignore_duty = data[*offset] != 0; *offset += 1;
        self.pulse1.frequency = u16::from_le_bytes([data[*offset], data[*offset + 1]]); *offset += 2;
        self.pulse1.enabled = data[*offset] != 0; *offset += 1;
        self.pulse1.timer = i32::from_le_bytes([data[*offset], data[*offset + 1], data[*offset + 2], data[*offset + 3]]); *offset += 4;
        self.pulse1.step = data[*offset]; *offset += 1;

        self.pulse2.volume = data[*offset]; *offset += 1;
        self.pulse2.duty_cycle = data[*offset]; *offset += 1;
        self.pulse2.ignore_duty = data[*offset] != 0; *offset += 1;
        self.pulse2.frequency = u16::from_le_bytes([data[*offset], data[*offset + 1]]); *offset += 2;
        self.pulse2.enabled = data[*offset] != 0; *offset += 1;
        self.pulse2.timer = i32::from_le_bytes([data[*offset], data[*offset + 1], data[*offset + 2], data[*offset + 3]]); *offset += 4;
        self.pulse2.step = data[*offset]; *offset += 1;

        self.saw.accumulator_rate = data[*offset]; *offset += 1;
        self.saw.accumulator = data[*offset]; *offset += 1;
        self.saw.frequency = u16::from_le_bytes([data[*offset], data[*offset + 1]]); *offset += 2;
        self.saw.enabled = data[*offset] != 0; *offset += 1;
        self.saw.timer = i32::from_le_bytes([data[*offset], data[*offset + 1], data[*offset + 2], data[*offset + 3]]); *offset += 4;
        self.saw.step = data[*offset]; *offset += 1;

        self.output_exp_pin6 = data[*offset] != 0; *offset += 1;
        self.output_exp_pin9 = data[*offset] != 0; *offset += 1;
        self.output_to_4011 = data[*offset] != 0; *offset += 1;
        self.volume = data[*offset]; *offset += 1;
        self.last_output = data[*offset]; *offset += 1;
        Ok(())
    }
}
