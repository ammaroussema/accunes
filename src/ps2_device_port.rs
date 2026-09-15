const PS2_CLOCKS: u8 = 70;

#[derive(Clone, Debug, Default)]
pub struct Ps2DevicePort {
    pub host_clock: bool,
    pub host_data: bool,
    pub device_clock: bool,
    pub device_data: bool,
    pub state: u16,
    pub cycles: i32,
    pub latch: u8,
    pub one_bits: u8,
    pub data_from_host: std::collections::VecDeque<u8>,
    pub data_to_host: std::collections::VecDeque<u8>,
}

impl Ps2DevicePort {
    pub fn new() -> Self {
        let mut p = Self::default();
        p.reset();
        p
    }

    pub fn reset(&mut self) {
        self.host_clock = true;
        self.host_data = true;
        self.device_clock = true;
        self.device_data = true;
        self.state = 0;
        self.cycles = 0;
        self.latch = 0;
        self.one_bits = 0;
        self.data_from_host.clear();
        self.data_to_host.clear();
    }

    pub fn set_host_clock(&mut self, setting: bool) {
        self.host_clock = setting;
    }

    pub fn set_host_data(&mut self, setting: bool) {
        self.host_data = setting;
    }

    fn set_device_clock(&mut self, setting: bool) {
        self.device_clock = setting;
    }

    fn set_device_data(&mut self, setting: bool) {
        self.device_data = setting;
    }

    pub fn get_clock(&self) -> bool {
        self.host_clock && self.device_clock
    }

    pub fn get_data(&self) -> bool {
        self.host_data && self.device_data
    }

    fn set_next_state(&mut self, new_state: u16) {
        self.cycles = PS2_CLOCKS as i32;
        self.state = new_state;
    }

    fn check_for_send_request(&mut self) {
        self.set_device_data(true);
        self.one_bits = 0;
        if !self.get_data() {
            self.set_next_state(100);
        } else if !self.data_to_host.is_empty() {
            self.latch = self.data_to_host.pop_front().unwrap_or(0);
            self.set_next_state(200);
        } else {
            self.state = 0;
        }
    }

    pub fn cpu_cycle(&mut self) {
        if self.cycles != 0 && self.device_clock == self.get_clock() {
            self.cycles -= 1;
            if self.cycles == 0 {
                self.set_device_clock(!self.device_clock);
                if !self.device_clock {
                    self.cycles = PS2_CLOCKS as i32;
                }
            }
        } else if self.device_clock {
            if !self.get_clock() {
                self.state = 0;
                self.cycles = 0;
            } else {
                match self.state {
                    0 => self.check_for_send_request(),
                    100 => self.set_next_state(if self.get_data() { 0 } else { 101 }),
                    101..=108 => {
                        self.latch = self.latch >> 1 | if self.get_data() { 0x80 } else { 0 };
                        if self.get_data() {
                            self.one_bits += 1;
                        }
                        self.set_next_state(self.state + 1);
                    }
                    109 => {
                        self.set_next_state(self.state + 1);
                    }
                    110 => {
                        if self.get_data() {
                            self.set_device_data(false);
                            self.data_from_host.push_back(self.latch);
                        } else {
                            self.set_device_data(true);
                            self.data_to_host.push_back(0xFE);
                        }
                        self.set_next_state(self.state + 1);
                    }
                    111 => {
                        self.set_device_data(true);
                        self.check_for_send_request();
                    }
                    200 => {
                        self.set_device_data(true);
                        if self.device_data && !self.get_data() {
                            self.data_to_host.clear();
                            self.set_next_state(101);
                            self.set_device_data(true);
                        } else {
                            self.set_next_state(self.state + 1);
                            self.set_device_data(false);
                        }
                    }
                    201..=208 => {
                        self.set_device_data(true);
                        if self.device_data && !self.get_data() {
                            self.data_to_host.clear();
                            self.set_next_state(101);
                            self.set_device_data(true);
                        } else {
                            self.set_next_state(self.state + 1);
                            self.set_device_data(self.latch & 0x01 != 0);
                            if self.latch & 0x01 != 0 {
                                self.one_bits += 1;
                            }
                            self.latch = self.latch >> 1;
                        }
                    }
                    209 => {
                        self.set_device_data(self.one_bits & 1 == 0);
                        self.set_next_state(self.state + 1);
                    }
                    210 => {
                        self.set_device_data(true);
                        self.set_next_state(self.state + 1);
                    }
                    211 => self.check_for_send_request(),
                    _ => {
                        self.state = 0;
                    }
                }
            }
        }
    }
}
