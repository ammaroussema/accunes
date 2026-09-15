pub trait SecurityDevice: Send + Sync {
    fn set_pins_serial(&mut self, _select: bool, _clock: bool, _data: bool) {}
    fn set_pins_parallel(&mut self, _select: bool, _clock: bool, _data: u8) {}
    fn get_data_bit(&self) -> bool { true }
    fn get_clock_bit(&mut self) -> bool { true }
    fn reset(&mut self) {}
    #[allow(dead_code)]
    fn save_state(&self) -> Vec<u8> { Vec::new() }
    #[allow(dead_code)]
    fn load_state(&mut self, _state: &[u8], start: usize) -> usize { start }
}
#[derive(Clone, Copy)]
pub struct SerialBinding {
    pub select: u8,
    pub clock: u8,
    pub data: u8,
}
#[derive(Clone, Copy)]
pub struct ParallelBinding {
    pub select: u8,
    pub clock_to_device: u8,
    pub clock_from_device: u8,
    pub data_to_device: u8,
    pub data_to_device_mask: u8,
    pub data_from_device: u8,
    pub data_from_device_mask: u8,
}
pub enum DeviceBinding {
    Serial(SerialBinding),
    Parallel(ParallelBinding),
}
pub struct AttachedDevice {
    pub device: Box<dyn SecurityDevice>,
    pub binding: DeviceBinding,
}
pub struct GpioPort {
    pub mask: u8,
    pub latch: u8,
    pub state: u8,
    pub devices: Vec<AttachedDevice>,
}
impl Default for GpioPort {
    fn default() -> Self {
        Self::new()
    }
}
impl GpioPort {
    pub fn new() -> Self {
        Self {
            mask: 0,
            latch: 0,
            state: 0xFF,
            devices: Vec::new(),
        }
    }
    pub fn reset(&mut self) {
        self.mask = 0;
        self.latch = 0;
        self.state = 0xFF;
        // Clear attached devices on reset, matching Furbtendulator's serialDevices.clear().
        // Mapper reset functions re-attach devices after calling OneBus::reset(), so
        // clearing here prevents duplicate entries accumulating across resets.
        for dev in &mut self.devices {
            dev.device.reset();
        }
        self.devices.clear();
    }
    pub fn read(&mut self, address: u8) -> u8 {
        match address & 7 {
            0 => self.mask,
            2 => self.latch,
            3 => {
                self.state = !self.mask;
                for dev in &mut self.devices {
                    match dev.binding {
                        DeviceBinding::Serial(b) => {
                            let bit = if dev.device.get_data_bit() { 1 } else { 0 };
                            self.state &= (bit << b.data) | !(1 << b.data);
                        }
                        DeviceBinding::Parallel(b) => {
                            let data_bit = if dev.device.get_data_bit() { 1 } else { 0 };
                            self.state &= (data_bit << b.data_from_device)
                                | !(b.data_from_device_mask << b.data_from_device);
                            let clock_bit = if dev.device.get_clock_bit() { 1 } else { 0 };
                            self.state &= (clock_bit << b.clock_from_device)
                                | !(1 << b.clock_from_device);
                        }
                    }
                }
                self.state
            }
            _ => 0xFF,
        }
    }
    pub fn write(&mut self, address: u8, value: u8) {
        match address & 7 {
            0 => {
                self.mask = value;
                self.update_state_and_notify();
            }
            2 | 3 => {
                self.latch = value;
                self.update_state_and_notify();
            }
            _ => {}
        }
    }
    fn update_state_and_notify(&mut self) {
        self.state = (self.state & !self.mask) | (self.latch & self.mask);
        for dev in &mut self.devices {
            match dev.binding {
                DeviceBinding::Serial(b) => {
                    let sel = (self.state >> b.select & 1) != 0;
                    let clk = (self.state >> b.clock & 1) != 0;
                    let dat = (self.state >> b.data & 1) != 0;
                    dev.device.set_pins_serial(sel, clk, dat);
                }
                DeviceBinding::Parallel(b) => {
                    let sel = (self.state >> b.select & 1) != 0;
                    let clk = (self.state >> b.clock_to_device & 1) != 0;
                    let dat = (self.state >> b.data_to_device) & b.data_to_device_mask;
                    dev.device.set_pins_parallel(sel, clk, dat);
                }
            }
        }
    }
}
pub struct SerialRomDevice {
    bit_position: usize,
    command: u8,
    state: u8,
    clock: bool,
    output: bool,
    rom: Vec<u8>,
}
impl SerialRomDevice {
    pub fn new(rom: Vec<u8>) -> Self {
        Self {
            bit_position: 0,
            command: 0,
            state: 0,
            clock: true,
            output: true,
            rom,
        }
    }
}
impl SecurityDevice for SerialRomDevice {
    fn reset(&mut self) {
        self.bit_position = 0;
        self.command = 0;
        self.state = 0;
        self.clock = true;
        self.output = true;
    }
    fn get_data_bit(&self) -> bool {
        self.output
    }
    fn set_pins_serial(&mut self, select: bool, new_clock: bool, new_data: bool) {
        if select {
            self.state = 0;
        } else if !self.clock && new_clock {
            if self.state < 8 {
                self.command = (self.command << 1) | (new_data as u8);
                self.state += 1;
                if self.state == 8 && self.command != 0x30 {
                    self.state = 0;
                } else {
                    self.bit_position = 0;
                }
            } else if !self.rom.is_empty() {
                let byte_idx = self.bit_position >> 3;
                let bit_shift = 7 - (self.bit_position & 7);
                if byte_idx < self.rom.len() {
                    self.output = ((self.rom[byte_idx] >> bit_shift) & 1) != 0;
                } else {
                    self.output = false;
                }
                self.bit_position += 1;
                if self.bit_position >= 256 * 8 {
                    self.state = 0;
                }
            }
        }
        self.clock = new_clock;
    }
}
pub struct InverterDevice {
    command: u8,
    result: u8,
    state: u8,
    clock: bool,
    data: bool,
    output: bool,
}
impl Default for InverterDevice {
    fn default() -> Self {
        Self::new()
    }
}
impl InverterDevice {
    pub fn new() -> Self {
        Self {
            command: 0,
            result: 0,
            state: 0,
            clock: true,
            data: true,
            output: true,
        }
    }
}
impl SecurityDevice for InverterDevice {
    fn reset(&mut self) {
        self.command = 0;
        self.result = 0;
        self.state = 0;
        self.clock = true;
        self.data = true;
        self.output = true;
    }
    fn get_data_bit(&self) -> bool {
        self.output
    }
    fn set_pins_serial(&mut self, _select: bool, new_clock: bool, new_data: bool) {
        if self.clock && new_clock && self.data && !new_data {
            self.state = 1;
        } else if self.clock && new_clock && !self.data && new_data {
            self.state = 19;
        } else if !self.clock && new_clock {
            if self.state == 0 {
                self.data = new_data;
            } else if (1..9).contains(&self.state) {
                self.command = (self.command << 1) | (new_data as u8);
                self.state += 1;
                if self.state == 9 && self.command != 0x80 {
                    self.state = 0;
                }
            } else if self.state == 9 {
                self.state += 1;
            } else if (10..18).contains(&self.state) {
                self.result = (self.result << 1) | (new_data as u8);
                self.state += 1;
                if self.state == 18 {
                    let res = self.result;
                    let inverted = (res.wrapping_neg() >> 4 & 0x0F) | (res.wrapping_neg() << 4 & 0xF0);
                    self.result = inverted;
                }
            } else if self.state == 18 {
                self.state = 0;
            } else {
                self.output = (self.result & 0x80) != 0;
                self.result <<= 1;
            }
        }
        self.clock = new_clock;
        self.data = new_data;
    }
}
pub struct I2cEeprom24C04 {
    pub storage: [u8; 512],
    address: u16,
    bit: u8,
    latch: u8,
    state: u8,
    clock: bool,
    data: bool,
    output: bool,
    read_mode: bool,
}
impl Default for I2cEeprom24C04 {
    fn default() -> Self {
        Self::new()
    }
}
impl I2cEeprom24C04 {
    pub fn new() -> Self {
        Self {
            storage: [0; 512],
            address: 0,
            bit: 0,
            latch: 0,
            state: 0,
            clock: true,
            data: true,
            output: true,
            read_mode: false,
        }
    }
    fn receive_bit(&mut self) {
        match self.state {
            1 => self.state += 1,
            2 => {
                if new_bit_matches(0b1010 & 8, self.data) { self.state += 1; } else { self.state = 0; }
            }
            3 => {
                if new_bit_matches(0b1010 & 4, self.data) { self.state += 1; } else { self.state = 0; }
            }
            4 => {
                if new_bit_matches(0b1010 & 2, self.data) { self.state += 1; } else { self.state = 0; }
            }
            5 => {
                if new_bit_matches(0b1010 & 1, self.data) { self.state += 1; } else { self.state = 0; }
            }
            6 => self.state += 1,
            7 => self.state += 1,
            8 => {
                self.address = (self.address & !0x100) | ((if self.data { 1 } else { 0 }) << 8);
                self.state += 1;
                self.output = true;
            }
            9 => {
                self.read_mode = self.data;
                self.state += 1;
            }
            10 => {
                self.bit = 0;
                if self.read_mode {
                    self.latch = self.storage[(self.address & 0x1FF) as usize];
                    self.state = 11;
                } else {
                    self.latch = 0;
                    self.state = 12;
                }
            }
            11 => {
                self.bit += 1;
                if self.bit == 8 {
                    self.address = (self.address & !0xFF) | ((self.address + 1) & 0xFF);
                    self.state -= 1;
                }
            }
            12 => {
                if self.data {
                    self.latch |= 0x80 >> self.bit;
                }
                self.bit += 1;
                if self.bit == 8 {
                    self.address = (self.address & !0xFF) | self.latch as u16;
                    self.state += 1;
                }
            }
            13 => {
                self.bit = 0;
                self.latch = 0;
                self.state += 1;
            }
            14 => {
                if self.data {
                    self.latch |= 0x80 >> self.bit;
                }
                self.bit += 1;
                if self.bit == 8 {
                    self.storage[(self.address & 0x1FF) as usize] = self.latch;
                    self.address = (self.address & !0xFF) | ((self.address + 1) & 0xFF);
                    self.state -= 1;
                }
            }
            _ => {}
        }
        match self.state {
            10 | 13 => self.output = false,
            11 => self.output = (self.latch & (0x80 >> self.bit)) != 0,
            _ => self.output = true,
        }
    }
}
fn new_bit_matches(expected_bit_mask: u8, data: bool) -> bool {
    (expected_bit_mask != 0) == data
}
impl SecurityDevice for I2cEeprom24C04 {
    fn reset(&mut self) {
        self.address = 0;
        self.bit = 0;
        self.latch = 0;
        self.state = 0;
        self.clock = true;
        self.data = true;
        self.output = true;
        self.read_mode = false;
    }
    fn get_data_bit(&self) -> bool {
        self.output
    }
    fn set_pins_serial(&mut self, _select: bool, new_clock: bool, new_data: bool) {
        if self.clock && new_clock && self.data && !new_data {
            self.state = 1;
        } else if self.clock && new_clock && !self.data && new_data {
            self.state = 0;
        } else if self.clock && !new_clock {
            self.receive_bit();
        }
        self.clock = new_clock;
        self.data = new_data;
    }
}
pub struct InverterAdderDevice {
    command: u8,
    command_state: u8,
    latch: u8,
    sending: bool,
    clock_to_device: bool,
    data_from_device: bool,
    clock_from_device: bool,
}
impl Default for InverterAdderDevice {
    fn default() -> Self {
        Self::new()
    }
}
impl InverterAdderDevice {
    pub fn new() -> Self {
        Self {
            command: 0xFF,
            command_state: 0,
            latch: 0xFF,
            sending: false,
            clock_to_device: false,
            data_from_device: true,
            clock_from_device: false,
        }
    }
}
impl SecurityDevice for InverterAdderDevice {
    fn reset(&mut self) {
        self.command = 0xFF;
        self.command_state = 0;
        self.latch = 0xFF;
        self.sending = false;
        self.clock_to_device = false;
        self.data_from_device = true;
        self.clock_from_device = false;
    }
    fn get_data_bit(&self) -> bool {
        self.data_from_device
    }
    fn get_clock_bit(&mut self) -> bool {
        self.clock_from_device = !self.clock_from_device;
        self.clock_from_device
    }
    fn set_pins_parallel(&mut self, _select: bool, new_clock: bool, new_data: u8) {
        if self.sending && self.command_state > 0 {
            if new_clock ^ self.clock_to_device {
                self.latch <<= 1;
                self.command_state += 1;
                if self.command_state > 8 {
                    self.sending = false;
                    self.command = 0xFF;
                    self.command_state = 0;
                }
            }
        } else if new_clock {
            self.latch = (self.latch & 0x0F) | ((new_data << 4) & 0xF0);
        } else {
            self.latch = (self.latch & 0xF0) | (new_data & 0x0F);
            if self.command == 0xFF && self.latch == 0x55 {
                self.command = self.latch;
            } else if self.command == 0x55 && self.latch == 0xAA {
                self.reset();
            } else if self.command == 0xFF && self.latch == 0x00 {
                self.command = 0x00;
                self.command_state = 0;
            } else if self.command == 0xFF && self.latch == 0x02 {
                self.command = 0x02;
                self.command_state = 0;
            } else if self.command == 0x02 && self.command_state == 0 {
                self.sending = true;
                self.command_state = 1;
                self.latch = (!self.latch).wrapping_add(0xB0);
            } else if self.command == 0x00 && self.command_state == 0 {
                self.sending = true;
                self.command_state = 1;
                self.latch = 0x00;
            }
        }
        self.clock_to_device = new_clock;
        self.data_from_device = ((self.latch >> 7) & 1) != 0;
    }
}
