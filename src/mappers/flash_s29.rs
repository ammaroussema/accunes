#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChipMode {
    WaitingForCommand,
    Write,
    Erase,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChipModel {
    S29AL008,
    S29AL016,
    S29JL032,
    S29GL064S,
}

#[derive(Clone, Debug)]
pub struct FlashS29 {
    model: ChipModel,
    mode: ChipMode,
    cycle: u8,
    software_id: bool,
    unlock_bypass: bool,
    size: usize,
}

impl FlashS29 {
    pub fn new(size: usize) -> Self {
        let model = match size {
            0x100000 => ChipModel::S29AL008,
            0x200000 => ChipModel::S29AL016,
            0x400000 => ChipModel::S29JL032,
            0x800000 => ChipModel::S29GL064S,
            _ => {
                if size <= 0x100000 {
                    ChipModel::S29AL008
                } else if size <= 0x200000 {
                    ChipModel::S29AL016
                } else if size <= 0x400000 {
                    ChipModel::S29JL032
                } else {
                    ChipModel::S29GL064S
                }
            }
        };

        Self {
            model,
            mode: ChipMode::WaitingForCommand,
            cycle: 0,
            software_id: false,
            unlock_bypass: false,
            size,
        }
    }

    #[inline]
    pub fn is_software_id_mode(&self) -> bool {
        self.software_id
    }

    pub fn read(&self, addr: u32) -> Option<u8> {
        if self.software_id {
            match addr & 0x1FF {
                0x00 => Some(0x01),
                0x02 => match self.model {
                    ChipModel::S29AL008 => Some(0x5B),
                    ChipModel::S29AL016 => Some(0x49),
                    ChipModel::S29JL032 | ChipModel::S29GL064S => Some(0x7E),
                },
                0x1C => match self.model {
                    ChipModel::S29JL032 => Some(0x0A),
                    ChipModel::S29GL064S => Some(0x10),
                    _ => Some(0xFF),
                },
                0x1E => match self.model {
                    ChipModel::S29JL032 | ChipModel::S29GL064S => Some(0x00),
                    _ => Some(0xFF),
                },
                _ => Some(0xFF),
            }
        } else {
            None
        }
    }

    pub fn reset_state(&mut self) {
        self.mode = ChipMode::WaitingForCommand;
        self.cycle = 0;
    }

    fn process_unlock_bypass_mode(&mut self, value: u8) {
        if self.cycle == 0 {
            if value == 0xA0 {
                self.mode = ChipMode::Write;
            } else if value == 0x90 {
                self.cycle += 1;
            } else {
                self.reset_state();
            }
        } else if self.cycle == 1 {
            if value == 0x00 {
                self.unlock_bypass = false;
            }
            self.reset_state();
        }
    }

    pub fn write(&mut self, data: &mut [u8], addr: usize, value: u8) {
        let cmd = (addr & 0xFFF) as u16;
        if self.mode == ChipMode::WaitingForCommand {
            if self.unlock_bypass {
                self.process_unlock_bypass_mode(value);
                return;
            }

            if self.cycle == 0 {
                if cmd == 0xAAA && value == 0xAA {
                    self.cycle += 1;
                } else if value == 0xF0 {
                    self.reset_state();
                    self.software_id = false;
                }
            } else if self.cycle == 1 && cmd == 0x555 && value == 0x55 {
                self.cycle += 1;
            } else if self.cycle == 2 && cmd == 0xAAA {
                self.cycle += 1;
                match value {
                    0x20 => {
                        self.reset_state();
                        self.unlock_bypass = true;
                    }
                    0x80 => {
                        self.mode = ChipMode::Erase;
                    }
                    0x90 => {
                        self.reset_state();
                        self.software_id = true;
                    }
                    0xA0 => {
                        self.mode = ChipMode::Write;
                    }
                    0xF0 => {
                        self.reset_state();
                        self.software_id = false;
                    }
                    _ => {
                        self.cycle = 0;
                    }
                }
            } else {
                self.cycle = 0;
            }
        } else if self.mode == ChipMode::Write {
            if addr < data.len() {
                data[addr] &= value;
            }
            self.reset_state();
        } else if self.mode == ChipMode::Erase {
            if self.cycle == 3 {
                if cmd == 0xAAA && value == 0xAA {
                    self.cycle += 1;
                } else {
                    self.reset_state();
                }
            } else if self.cycle == 4 {
                if cmd == 0x555 && value == 0x55 {
                    self.cycle += 1;
                } else {
                    self.reset_state();
                }
            } else if self.cycle == 5 {
                if cmd == 0xAAA && value == 0x10 {
                    for byte in data.iter_mut() {
                        *byte = 0xFF;
                    }
                } else if value == 0x30 {
                    let total_size = if self.size != 0 { self.size } else { data.len() };
                    if total_size > 0 {
                        let page_count = total_size / 0x10000;
                        let page = addr / 0x10000;
                        if page_count > 0 && page == page_count - 1 {
                            let sector_sizes: &[usize] = match self.model {
                                ChipModel::S29AL008 | ChipModel::S29AL016 => &[32, 8, 8, 16],
                                ChipModel::S29JL032 | ChipModel::S29GL064S => &[8, 8, 8, 8, 8, 8, 8, 8],
                            };

                            let offset_kb = (addr & 0xFFFF) / 1024;
                            let mut seg_offset = 0;
                            let mut seg_size = 0;
                            for &sz in sector_sizes {
                                if seg_offset + sz > offset_kb {
                                    seg_size = sz;
                                    break;
                                }
                                seg_offset += sz;
                                seg_size = sz;
                            }

                            let start = page * 0x10000 + seg_offset * 1024;
                            let end = (start + seg_size * 1024).min(data.len());
                            if start < data.len() {
                                for byte in &mut data[start..end] {
                                    *byte = 0xFF;
                                }
                            }
                        } else {
                            let start = page * 0x10000;
                            let end = (start + 0x10000).min(data.len());
                            if start < data.len() {
                                for byte in &mut data[start..end] {
                                    *byte = 0xFF;
                                }
                            }
                        }
                    }
                }
                self.reset_state();
            }
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut s = Vec::with_capacity(8);
        s.push(match self.mode {
            ChipMode::WaitingForCommand => 0,
            ChipMode::Write => 1,
            ChipMode::Erase => 2,
        });
        s.push(self.cycle);
        s.push(if self.software_id { 1 } else { 0 });
        s.push(if self.unlock_bypass { 1 } else { 0 });
        s
    }

    pub fn deserialize(&mut self, data: &[u8], offset: &mut usize) -> Result<(), ()> {
        if *offset + 4 > data.len() {
            return Err(());
        }
        self.mode = match data[*offset] {
            0 => ChipMode::WaitingForCommand,
            1 => ChipMode::Write,
            2 => ChipMode::Erase,
            _ => ChipMode::WaitingForCommand,
        };
        *offset += 1;
        self.cycle = data[*offset];
        *offset += 1;
        self.software_id = data[*offset] != 0;
        *offset += 1;
        self.unlock_bypass = data[*offset] != 0;
        *offset += 1;
        Ok(())
    }
}
