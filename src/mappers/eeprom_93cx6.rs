pub struct Eeprom93Cx6 {
    capacity: usize,
    storage: Vec<u8>,
    opcode: u8,
    data: u16,
    address: u16,
    state: u8,
    last_clk: bool,
    write_enabled: bool,
    output: bool,
    word_size: u8,
    state_address: u8,
    state_data: u8,
}

impl Eeprom93Cx6 {
    pub fn new(capacity: usize, word_size: u8) -> Self {
        let state_address = if word_size == 16 { 11 } else { 12 };
        let state_data = if word_size == 16 { 27 } else { 20 };
        let (state_address, state_data) = if capacity == 128 {
            (state_address - 2, state_data - 2)
        } else {
            (state_address, state_data)
        };
        Eeprom93Cx6 {
            capacity,
            storage: vec![0xFF; capacity],
            opcode: 0,
            data: 0,
            address: 0,
            state: 0,
            last_clk: false,
            write_enabled: false,
            output: true,
            word_size,
            state_address,
            state_data,
        }
    }

    pub fn write(&mut self, cs: bool, clk: bool, dat: bool) {
        if !cs && self.state <= 1 {
            self.state = 0;
        } else if self.state == 0 && cs && clk && !self.last_clk {
            if dat { self.state = 1; }
            self.opcode = 0;
            self.address = 0;
            self.output = true;
        } else if clk && !self.last_clk && self.state >= 1 {
            if self.state >= 1 && self.state < 3 {
                self.opcode = (self.opcode << 1) | (if dat { 1 } else { 0 });
            } else if self.state >= 3 && self.state < self.state_address {
                self.address = (self.address << 1) | (if dat { 1 } else { 0 });
            } else if self.state >= self.state_address && self.state < self.state_data {
                match self.opcode {
                    1 | 11 => { self.data = (self.data << 1) | (if dat { 1 } else { 0 }); }
                    2 => {
                        if self.word_size == 16 {
                            self.output = (self.data & 0x8000) != 0;
                        } else {
                            self.output = (self.data & 0x80) != 0;
                        }
                        self.data <<= 1;
                    }
                    _ => {}
                }
            }
            self.state += 1;
            if self.state == self.state_address {
                match self.opcode {
                    0 => {
                        self.opcode = (if self.word_size == 16 {
                            (self.address >> 6) + 10
                        } else {
                            (self.address >> 7) + 10
                        }) as u8;
                        match self.opcode {
                            10 => { self.write_enabled = false; self.state = 99; }
                            13 => { self.write_enabled = true; self.state = 99; }
                            12 => {
                                if self.write_enabled {
                                    for b in &mut self.storage { *b = 0xFF; }
                                }
                                self.state = 99;
                            }
                            11 => { self.address = 0; }
                            _ => {}
                        }
                    }
                    3 => {
                        if self.write_enabled {
                            if self.word_size == 16 {
                                let addr = self.address as usize * 2;
                                self.storage[addr] = 0xFF;
                                self.storage[addr + 1] = 0xFF;
                            } else {
                                self.storage[self.address as usize] = 0xFF;
                            }
                        }
                        self.state = 99;
                    }
                    2 => {
                        if self.word_size == 16 {
                            let addr = self.address as usize * 2;
                            if addr + 1 < self.capacity {
                                self.data = self.storage[addr] as u16 | (self.storage[addr + 1] as u16) << 8;
                            }
                            self.address += 1;
                        } else {
                            self.data = self.storage[self.address as usize] as u16;
                            self.address += 1;
                        }
                    }
                    _ => {}
                }
            } else if self.state == self.state_data {
                match self.opcode {
                    1 => {
                        if self.word_size == 16 {
                            let addr = self.address as usize * 2;
                            if addr + 1 < self.capacity {
                                self.storage[addr] = self.data as u8;
                                self.storage[addr + 1] = (self.data >> 8) as u8;
                            }
                            self.address += 1;
                        } else {
                            self.storage[self.address as usize] = self.data as u8;
                            self.address += 1;
                        }
                        self.state = 99;
                    }
                    11 => {
                        if self.word_size == 16 {
                            let addr = self.address as usize * 2;
                            if addr + 1 < self.capacity {
                                self.storage[addr] = self.data as u8;
                                self.storage[addr + 1] = (self.data >> 8) as u8;
                            }
                            self.address += 1;
                        } else {
                            self.storage[self.address as usize] = self.data as u8;
                            self.address += 1;
                        }
                        self.state = if cs && (self.address as usize) < self.capacity { self.state_address } else { 99 };
                    }
                    2 => {
                        if (self.address as usize) <= self.capacity {
                            if self.word_size == 16 {
                                let addr = (self.address as usize - 1) * 2;
                                if addr + 1 < self.capacity {
                                    self.data = self.storage[addr] as u16 | (self.storage[addr + 1] as u16) << 8;
                                }
                            } else {
                                self.data = self.storage[(self.address as usize - 1) % self.capacity] as u16;
                            }
                        }
                        self.state = if cs && (self.address as usize) <= self.capacity { self.state_address } else { 99 };
                    }
                    _ => {}
                }
            }
            if self.state == 99 {
                self.output = false;
                self.state = 0;
            }
        }
        if self.opcode == 2 && self.state == self.state_address - 2 {
            self.output = false;
        }
        self.last_clk = clk;
    }

    pub fn read(&self) -> bool {
        self.output
    }

    pub fn save(&self) -> Vec<u8> {
        let mut state = Vec::new();
        state.push(self.opcode);
        state.extend_from_slice(&self.data.to_le_bytes());
        state.extend_from_slice(&self.address.to_le_bytes());
        state.push(if self.last_clk { 1 } else { 0 });
        state.push(if self.write_enabled { 1 } else { 0 });
        state.push(if self.output { 1 } else { 0 });
        state
    }

    pub fn load(&mut self, state: &[u8], start: usize) -> usize {
        let mut p = start;
        if p < state.len() { self.opcode = state[p]; p += 1; }
        if p + 2 <= state.len() { self.data = u16::from_le_bytes([state[p], state[p+1]]); p += 2; }
        if p + 2 <= state.len() { self.address = u16::from_le_bytes([state[p], state[p+1]]); p += 2; }
        if p < state.len() { self.last_clk = state[p] != 0; p += 1; }
        if p < state.len() { self.write_enabled = state[p] != 0; p += 1; }
        if p < state.len() { self.output = state[p] != 0; p += 1; }
        p
    }
}
