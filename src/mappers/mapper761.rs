use std::collections::VecDeque;
use crate::cartridge::Cartridge;
use crate::mapper::{FetchResult, Mapper};

const DOR_SELECT_DRIVE: u8 = 0x03;
const DOR_NOT_RESET: u8 = 0x04;
const DOR_MOTOR_A: u8 = 0x10;
const DOR_MOTOR_B: u8 = 0x20;
const DOR_MOTOR_C: u8 = 0x40;
const DOR_MOTOR_D: u8 = 0x80;
const MSR_READY: u8 = 0x80;
const MSR_BUSY: u8 = 0x10;
const MSR_DIRECTION: u8 = 0x40;
const MSR_NO_DMA: u8 = 0x20;
const CMD_MULTI_TRACK: u8 = 0x80;
const ST0_ABNORMAL_TERMINATION: u8 = 0x40;
const ST0_INVALID_COMMAND: u8 = 0x80;
const ST0_SEEK_COMPLETE: u8 = 0x20;
const ST0_POLLING: u8 = 0xC0;
const ST1_END_OF_TRACK: u8 = 0x80;
const ST1_NO_DATA: u8 = 0x04;
const ST1_NOT_WRITEABLE: u8 = 0x02;
const ST1_MISSING_ADDRESS_MARK: u8 = 0x01;
const ST2_WRONG_CYLINDER: u8 = 0x10;
const SRA_IRQ: u8 = 0x80;
const CFG_NOT_POLL: u8 = 0x10;

struct CommandInfo {
    length: u8,
}

const COMMAND_INFO: [CommandInfo; 32] = [
    CommandInfo { length: 1 },
    CommandInfo { length: 1 },
    CommandInfo { length: 9 },
    CommandInfo { length: 3 },
    CommandInfo { length: 2 },
    CommandInfo { length: 9 },
    CommandInfo { length: 9 },
    CommandInfo { length: 2 },
    CommandInfo { length: 1 },
    CommandInfo { length: 9 },
    CommandInfo { length: 2 },
    CommandInfo { length: 1 },
    CommandInfo { length: 9 },
    CommandInfo { length: 6 },
    CommandInfo { length: 1 },
    CommandInfo { length: 3 },
    CommandInfo { length: 1 },
    CommandInfo { length: 9 },
    CommandInfo { length: 2 },
    CommandInfo { length: 4 },
    CommandInfo { length: 1 },
    CommandInfo { length: 1 },
    CommandInfo { length: 9 },
    CommandInfo { length: 1 },
    CommandInfo { length: 1 },
    CommandInfo { length: 9 },
    CommandInfo { length: 1 },
    CommandInfo { length: 1 },
    CommandInfo { length: 1 },
    CommandInfo { length: 9 }, 
    CommandInfo { length: 1 }, 
    CommandInfo { length: 1 }, 
];

const DATA_RATES: [i32; 4] = [500, 300, 250, 1000];

struct Drive {
    exists: bool,
    ready: bool,
    inserted: bool,
    running: bool,
    write_protected: bool,
    changed: bool,
    cylinder: i32,
    cylinders: i32,
    sides: i32,
    sectors: i32,
    correct_data_rate: i32,
    data: Vec<u8>,
}

impl Drive {
    fn new() -> Self {
        Self {
            exists: true,
            ready: true,
            inserted: false,
            running: false,
            write_protected: false,
            changed: false,
            cylinder: 0,
            cylinders: 0,
            sides: 0,
            sectors: 0,
            correct_data_rate: 0,
            data: Vec::new(),
        }
    }
}

enum FdcPhase {
    Command,
    Result,
    ExecuteReadData,
    ExecuteWriteData,
    ExecuteFormatTrack,
}

struct Fdc {
    drives: [Drive; 4],
    active_drive: usize,
    dor: u8,
    msr: u8,
    st0: u8,
    st1: u8,
    st2: u8,
    sra: u8,
    config: u8,
    latch: u8,
    cylinder_pos: u8,
    head_pos: u8,
    sector_number: u8,
    sector_size: usize,
    bytes_left: usize,
    data_read: *const u8,
    data_write: *mut u8,
    data_end: *const u8,
    reset_pin: bool,
    use_dma: bool,
    dma_request: bool,
    dma_terminate: bool,
    data_rate: i32,
    command: Vec<u8>,
    result_queue: VecDeque<u8>,
    poll_timer: i32,
    poll_drive: u8,
    previously_ready: [bool; 4],
    phase: FdcPhase,
}

unsafe impl Send for Fdc {}

impl Fdc {
    fn new() -> Self {
        let mut fdc = Self {
            drives: [Drive::new(), Drive::new(), Drive::new(), Drive::new()],
            active_drive: 0,
            dor: 0,
            msr: MSR_READY,
            st0: 0,
            st1: 0,
            st2: 0,
            sra: 0,
            config: 0,
            latch: 0,
            cylinder_pos: 0,
            head_pos: 0,
            sector_number: 0,
            sector_size: 0,
            bytes_left: 0,
            data_read: std::ptr::null(),
            data_write: std::ptr::null_mut(),
            data_end: std::ptr::null(),
            reset_pin: false,
            use_dma: false,
            dma_request: false,
            dma_terminate: false,
            data_rate: DATA_RATES[0],
            command: Vec::new(),
            result_queue: VecDeque::new(),
            poll_timer: 0,
            poll_drive: 0,
            previously_ready: [false; 4],
            phase: FdcPhase::Command,
        };
        fdc.reset();
        fdc
    }

    fn reset(&mut self) {
        self.phase = FdcPhase::Command;
        self.msr = MSR_READY;
        self.sra = 0;
        self.st0 = 0;
        self.st1 = 0;
        self.st2 = 0;
        self.bytes_left = 0;
        self.data_read = std::ptr::null();
        self.data_write = std::ptr::null_mut();
        self.data_end = std::ptr::null();
        self.command.clear();
        self.result_queue.clear();
        self.dma_request = false;
        self.dma_terminate = false;
        self.poll_timer = 0;
        self.poll_drive = 0;
        self.previously_ready = [false; 4];
    }

    fn run(&mut self) {
        if self.dor & DOR_NOT_RESET != 0 && !self.reset_pin {
            match self.phase {
                FdcPhase::Command => self.ph_command(),
                FdcPhase::Result => self.ph_result(),
                FdcPhase::ExecuteReadData => self.ph_read_data(),
                FdcPhase::ExecuteWriteData => self.ph_write_data(),
                FdcPhase::ExecuteFormatTrack => self.ph_format_track(),
            }
        }
    }

    fn ph_command(&mut self) {
        self.poll_drives();
        if self.msr & MSR_READY == 0 {
            self.msr |= MSR_BUSY;
            self.command.push(self.latch);
            let cmd_num = (self.command[0] & 0x1F) as usize;
            if cmd_num < COMMAND_INFO.len()
                && self.command.len() == COMMAND_INFO[cmd_num].length as usize
            {
                self.st0 = self.command[1] & 7;
                self.st1 = 0;
                self.st2 = 0;
                self.start_execution(cmd_num);
            } else {
                self.msr |= MSR_READY;
            }
        }
    }

    fn start_execution(&mut self, cmd_num: usize) {
        match cmd_num {
            0x03 => {
                self.use_dma = self.command[2] & 1 == 0;
                self.end_command(false, &[]);
            }
            0x04 => self.cm_sense_drive_status(),
            0x05 => {
                self.phase = FdcPhase::ExecuteWriteData;
            }
            0x06 => {
                self.phase = FdcPhase::ExecuteReadData;
            }
            0x07 => self.cm_recalibrate(),
            0x08 => self.cm_sense_interrupt_status(),
            0x0A => self.cm_read_id(),
            0x0D => {
                self.phase = FdcPhase::ExecuteFormatTrack;
            }
            0x0F => self.cm_seek(),
            0x13 => {
                self.config = self.command[2];
                self.end_command(false, &[]);
            }
            _ => self.cm_invalid(),
        }
    }

    fn end_command(&mut self, raise_irq: bool, result: &[u8]) {
        for &b in result {
            self.result_queue.push_back(b);
        }
        if raise_irq {
            self.sra |= SRA_IRQ;
        }
        self.msr &= !MSR_NO_DMA;
        self.start_result_phase();
    }

    fn start_result_phase(&mut self) {
        self.phase = FdcPhase::Result;
        self.command.clear();
        self.dma_request = false;
        if self.result_queue.is_empty() {
            self.msr &= !MSR_DIRECTION;
            self.msr &= !MSR_BUSY;
        } else {
            self.msr |= MSR_DIRECTION;
            self.latch = *self.result_queue.front().unwrap();
            self.result_queue.pop_front();
        }
        self.msr |= MSR_READY;
    }

    fn ph_result(&mut self) {
        if self.msr & MSR_READY == 0 {
            if self.result_queue.is_empty() {
                self.msr &= !MSR_DIRECTION;
                self.msr &= !MSR_BUSY;
                self.phase = FdcPhase::Command;
                self.dma_request = false;
            } else {
                self.msr |= MSR_DIRECTION;
                self.latch = *self.result_queue.front().unwrap();
                self.result_queue.pop_front();
            }
            self.msr |= MSR_READY;
        }
    }

    fn next_sector(&mut self) {
        if self.data_read.is_null() {
            self.cylinder_pos = self.command[2];
            self.head_pos = self.command[3];
            self.sector_number = self.command[4];
            self.sector_size = 128 << self.command[5] as usize;
            if self.sector_size > 8192 {
                self.sector_size = 8192;
            }
        } else if self.sector_number == self.command[6] {
            if self.command[0] & CMD_MULTI_TRACK != 0 && self.head_pos == 0 {
                self.head_pos ^= 1;
                self.command[3] ^= 1;
                self.sector_number = 1;
            } else if self.use_dma && self.dma_terminate {
                self.cylinder_pos += 1;
                self.head_pos ^= 1;
                self.sector_number = 1;
            } else {
                self.st0 |= ST0_ABNORMAL_TERMINATION;
                self.st1 |= ST1_END_OF_TRACK;
                self.end_command(
                    true,
                    &[self.st0, self.st1, self.st2, self.cylinder_pos, self.head_pos, self.sector_number, self.command[5]],
                );
                return;
            }
        } else {
            self.sector_number += 1;
        }

        if self.use_dma && self.dma_terminate {
            self.st0 |= 0x00;
            self.end_command(
                true,
                &[self.st0, self.st1, self.st2, self.cylinder_pos, self.head_pos, self.sector_number, self.command[5]],
            );
            self.dma_terminate = false;
            return;
        }

        let drive = &self.drives[self.active_drive];
        if !drive.inserted
            || self.cylinder_pos as i32 >= drive.cylinders
            || self.head_pos as i32 >= drive.sides
            || !drive.running
            || self.data_rate != drive.correct_data_rate
        {
            self.st1 |= ST1_MISSING_ADDRESS_MARK;
            self.st0 |= ST0_ABNORMAL_TERMINATION;
        } else if self.cylinder_pos != drive.cylinder as u8 {
            self.st1 |= ST1_NO_DATA;
            self.st2 |= ST2_WRONG_CYLINDER;
            self.st0 |= ST0_ABNORMAL_TERMINATION;
        } else if self.sector_number < 1
            || self.sector_number as i32 > drive.sectors
            || self.sector_size != 512
        {
            self.st1 |= ST1_NO_DATA;
            self.st0 |= ST0_ABNORMAL_TERMINATION;
        }

        if self.st0 & ST0_ABNORMAL_TERMINATION != 0 {
            self.end_command(
                true,
                &[self.st0, self.st1, self.st2, self.cylinder_pos, self.head_pos, self.sector_number, self.command[5]],
            );
        } else {
            let drive = &self.drives[self.active_drive];
            let lba = (self.cylinder_pos as i32 * drive.sides + self.head_pos as i32) * drive.sectors
                + self.sector_number as i32
                - 1;
            self.bytes_left = self.sector_size;
            let offset = lba as usize * self.sector_size;
            let base = drive.data.as_ptr();
            self.data_read = unsafe { base.add(offset) };
            self.data_write = unsafe { (base as *mut u8).add(offset) };
            self.data_end = unsafe { base.add(offset + self.sector_size) };
        }
    }

    fn ph_read_data(&mut self) {
        if self.msr & MSR_READY == 0 && !self.dma_request {
            if self.bytes_left == 0 {
                self.st1 = 0;
                self.st2 = 0;
                self.next_sector();
            }
            if self.bytes_left > 0 {
                unsafe {
                    self.latch = *self.data_read;
                    self.data_read = self.data_read.add(1);
                }
                self.bytes_left -= 1;
                if self.use_dma {
                    self.dma_request = true;
                } else {
                    self.msr |= MSR_READY | MSR_NO_DMA | MSR_DIRECTION;
                    self.sra |= SRA_IRQ;
                }
            }
        }
    }

    fn ph_write_data(&mut self) {
        let drive = &self.drives[self.active_drive];
        if drive.write_protected {
            self.st0 |= ST0_ABNORMAL_TERMINATION;
            self.st1 |= ST1_NOT_WRITEABLE;
            self.end_command(
                true,
                &[self.st0, self.st1, self.st2, self.cylinder_pos, self.head_pos, self.sector_number, self.command[5]],
            );
            return;
        }
        if self.msr & MSR_READY == 0 && !self.dma_request {
            if self.bytes_left == 0 {
                self.st1 = 0;
                self.st2 = 0;
                self.next_sector();
            }
            if self.bytes_left > 0 {
                unsafe {
                    *self.data_write = self.latch;
                    self.data_write = self.data_write.add(1);
                    self.data_read = self.data_read.add(1);
                }
                self.bytes_left -= 1;
            }
            if self.bytes_left > 0 {
                if self.use_dma {
                    self.dma_request = true;
                } else {
                    self.msr |= MSR_READY | MSR_NO_DMA;
                    self.msr &= !MSR_DIRECTION;
                    self.sra |= SRA_IRQ;
                }
            }
        }
    }

    fn ph_format_track(&mut self) {
        let drive = &self.drives[self.active_drive];
        if drive.write_protected {
            self.st0 |= ST0_ABNORMAL_TERMINATION;
            self.st1 |= ST1_NOT_WRITEABLE;
            self.end_command(
                true,
                &[self.st0, self.st1, self.st2, self.cylinder_pos, self.head_pos, self.sector_number, self.command[2]],
            );
            return;
        }
        if self.msr & MSR_READY == 0 {
            if self.bytes_left == 0 {
                if self.data_read.is_null() {
                    self.cylinder_pos = self.drives[self.active_drive].cylinder as u8;
                    self.head_pos = (self.command[1] >> 2) & 1;
                    self.sector_number = 1;
                } else if self.sector_number as i32 + 1 > self.command[3] as i32 {
                    self.st0 |= 0x00;
                    self.end_command(
                        true,
                        &[self.st0, self.st1, self.st2, self.cylinder_pos, self.head_pos, self.sector_number, self.command[2]],
                    );
                    return;
                } else {
                    self.sector_number += 1;
                }
                self.sector_size = 128 << self.command[2] as usize;
                if self.sector_size > 8192 {
                    self.sector_size = 8192;
                }
                let drive = &self.drives[self.active_drive];
                let lba = (self.cylinder_pos as i32 * drive.sides + self.head_pos as i32) * drive.sectors
                    + self.sector_number as i32
                    - 1;
                self.bytes_left = self.sector_size;
                let offset = lba as usize * self.sector_size;
                let base = drive.data.as_ptr();
                self.data_read = unsafe { base.add(offset) };
                self.data_write = unsafe { (base as *mut u8).add(offset) };
            } else {
                unsafe {
                    *self.data_write = self.command[5];
                    self.data_write = self.data_write.add(1);
                    self.data_read = self.data_read.add(1);
                }
                self.bytes_left -= 1;
            }
            if self.bytes_left > 512 - 4 {
                self.msr |= MSR_READY | MSR_NO_DMA;
                self.msr &= !MSR_DIRECTION;
                self.sra |= SRA_IRQ;
            }
        }
    }

    fn cm_seek(&mut self) {
        let target = self.command[2] as i32;
        let drive = &mut self.drives[self.active_drive];
        if drive.cylinder != target {
            drive.changed = false;
        }
        drive.cylinder = target;
        self.st0 |= ST0_SEEK_COMPLETE;
        if !drive.exists {
            self.st0 |= ST0_ABNORMAL_TERMINATION;
        }
        self.end_command(true, &[self.st0]);
    }

    fn cm_recalibrate(&mut self) {
        let drive = &mut self.drives[self.active_drive];
        if drive.cylinder != 0 {
            drive.changed = false;
        }
        drive.cylinder = 0;
        self.st0 |= ST0_SEEK_COMPLETE;
        if !drive.exists {
            self.st0 |= ST0_ABNORMAL_TERMINATION;
        }
        self.end_command(true, &[self.st0]);
    }

    fn cm_sense_interrupt_status(&mut self) {
        self.sra &= !SRA_IRQ;
        self.end_command(false, &[self.st0, self.drives[self.active_drive].cylinder as u8]);
    }

    fn cm_read_id(&mut self) {
        self.head_pos = (self.command[1] >> 2) & 1;
        let drive = &self.drives[self.active_drive];
        if !drive.inserted
            || self.cylinder_pos as i32 >= drive.cylinders
            || self.head_pos as i32 >= drive.sides
            || !drive.running
            || self.data_rate != drive.correct_data_rate
        {
            self.st1 |= ST1_MISSING_ADDRESS_MARK;
            self.st0 |= ST0_ABNORMAL_TERMINATION;
        } else {
            self.cylinder_pos = self.drives[self.active_drive].cylinder as u8;
            self.sector_size = 512;
            if self.sector_number == 0 || self.sector_number as i32 >= self.drives[self.active_drive].sectors {
                self.sector_number = 1;
            }
            self.st0 |= 0x00;
        }
        self.end_command(
            true,
            &[self.st0, self.st1, self.st2, self.cylinder_pos, self.head_pos, self.sector_number, self.command[5]],
        );
    }

    fn poll_drives(&mut self) {
        if self.config & CFG_NOT_POLL != 0 {
            return;
        }
        if self.poll_timer == 0 {
            self.poll_timer = 394;
            let pd = self.poll_drive as usize;
            if self.previously_ready[pd] != self.drives[pd].ready {
                self.previously_ready[pd] = self.drives[pd].ready;
                self.st0 = (self.st0 & !ST0_SEEK_COMPLETE) | ST0_POLLING | self.poll_drive;
                self.sra |= SRA_IRQ;
            }
            self.poll_drive = (self.poll_drive + 1) & 3;
        } else {
            self.poll_timer -= 1;
        }
    }

    fn cm_sense_drive_status(&mut self) {
        let drive = &self.drives[self.active_drive];
        self.end_command(
            false,
            &[(self.dor & DOR_SELECT_DRIVE)
                | (self.head_pos << 2)
                | if drive.cylinder == 0 { 0x10 } else { 0x00 }
                | if drive.write_protected { 0x40 } else { 0x00 }
                | if drive.inserted { 0x20 } else { 0x00 }
                | 0x28],
        );
    }

    fn cm_invalid(&mut self) {
        self.st0 |= ST0_INVALID_COMMAND;
        self.end_command(false, &[self.st0]);
    }

    fn read_io(&mut self, addr: u32) -> u8 {
        match addr & 7 {
            0 => self.sra,
            2 => self.dor,
            3 => {
                if self.sra & SRA_IRQ != 0 {
                    0x40
                } else {
                    0
                }
            }
            4 => self.msr,
            5 => {
                if self.msr & MSR_READY != 0 && self.msr & MSR_DIRECTION != 0 {
                    self.msr &= !MSR_READY;
                    self.sra &= !SRA_IRQ;
                    self.latch
                } else {
                    0xFF
                }
            }
            7 => {
                let changed = self.drives[self.active_drive].changed;
                self.drives[self.active_drive].changed = false;
                if changed { 0x80 } else { 0x00 }
            }
            _ => 0xFF,
        }
    }

    fn write_io(&mut self, addr: u32, val: u8) {
        match addr & 7 {
            2 => {
                if self.dor & DOR_NOT_RESET != 0 && val & DOR_NOT_RESET == 0 {
                    self.reset();
                }
                self.dor = val;
                self.active_drive = (self.dor & DOR_SELECT_DRIVE) as usize;
                self.drives[0].running = self.dor & DOR_MOTOR_A != 0 && self.drives[0].exists;
                self.drives[1].running = self.dor & DOR_MOTOR_B != 0 && self.drives[1].exists;
                self.drives[2].running = self.dor & DOR_MOTOR_C != 0 && self.drives[2].exists;
                self.drives[3].running = self.dor & DOR_MOTOR_D != 0 && self.drives[3].exists;
            }
            3 => {
                self.sra &= !SRA_IRQ;
            }
            5 => {
                if self.msr & MSR_READY != 0 && self.msr & MSR_DIRECTION == 0 {
                    self.msr &= !MSR_READY;
                    self.sra &= !SRA_IRQ;
                    self.latch = val;
                }
            }
            7 => {
                let idx = (val & 3) as usize;
                self.data_rate = DATA_RATES[idx];
            }
            _ => {}
        }
    }

    fn insert_disk(&mut self, drive_num: usize, data: Vec<u8>) {
        let d = &mut self.drives[drive_num & 3];
        d.data = data;
        d.write_protected = false;
        if d.data.len() == 160 * 1024 { d.cylinders = 40; d.sides = 1; d.sectors = 8; d.correct_data_rate = 250; }
        else if d.data.len() == 180 * 1024 { d.cylinders = 40; d.sides = 1; d.sectors = 9; d.correct_data_rate = 250; }
        else if d.data.len() == 320 * 1024 { d.cylinders = 40; d.sides = 2; d.sectors = 8; d.correct_data_rate = 250; }
        else if d.data.len() == 360 * 1024 { d.cylinders = 40; d.sides = 2; d.sectors = 9; d.correct_data_rate = 250; }
        else if d.data.len() == 720 * 1024 { d.cylinders = 80; d.sides = 2; d.sectors = 9; d.correct_data_rate = 250; }
        else if d.data.len() == 1200 * 1024 { d.cylinders = 80; d.sides = 2; d.sectors = 15; d.correct_data_rate = 500; }
        else if d.data.len() == 1440 * 1024 { d.cylinders = 80; d.sides = 2; d.sectors = 18; d.correct_data_rate = 500; }
        else if d.data.len() == 2880 * 1024 { d.cylinders = 80; d.sides = 2; d.sectors = 36; d.correct_data_rate = 1000; }
        d.inserted = true;
    }

    fn eject_disk(&mut self, drive_num: usize) {
        let d = &mut self.drives[drive_num & 3];
        if d.inserted {
            d.changed = true;
        }
        d.inserted = false;
    }

    fn irq_raised(&self) -> bool {
        self.sra & SRA_IRQ != 0
    }

    fn save_state(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.dor);
        data.push(self.msr);
        data.push(self.st0);
        data.push(self.st1);
        data.push(self.st2);
        data.push(self.sra);
        data.push(self.config);
        data.push(self.latch);
        data.push(self.cylinder_pos);
        data.push(self.head_pos);
        data.push(self.sector_number);
        data.push(self.sector_size as u8);
        data.extend_from_slice(&(self.bytes_left as u32).to_le_bytes());
        data.push(self.use_dma as u8);
        data.push(self.dma_request as u8);
        data.push(self.dma_terminate as u8);
        data.extend_from_slice(&self.data_rate.to_le_bytes());
        data.push(self.poll_timer as u8);
        data.push(self.poll_drive);
        for &p in &self.previously_ready {
            data.push(p as u8);
        }
        let phase_byte = match self.phase {
            FdcPhase::Command => 0u8,
            FdcPhase::Result => 1,
            FdcPhase::ExecuteReadData => 2,
            FdcPhase::ExecuteWriteData => 3,
            FdcPhase::ExecuteFormatTrack => 4,
        };
        data.push(phase_byte);
        data.push(self.active_drive as u8);
        for d in &self.drives {
            data.push(d.inserted as u8);
            data.push(d.running as u8);
            data.push(d.write_protected as u8);
            data.push(d.changed as u8);
            data.extend_from_slice(&d.cylinder.to_le_bytes());
        }
        data
    }

    fn load_state(&mut self, data: &[u8], mut off: usize) -> usize {
        self.dor = data[off]; off += 1;
        self.msr = data[off]; off += 1;
        self.st0 = data[off]; off += 1;
        self.st1 = data[off]; off += 1;
        self.st2 = data[off]; off += 1;
        self.sra = data[off]; off += 1;
        self.config = data[off]; off += 1;
        self.latch = data[off]; off += 1;
        self.cylinder_pos = data[off]; off += 1;
        self.head_pos = data[off]; off += 1;
        self.sector_number = data[off]; off += 1;
        self.sector_size = data[off] as usize; off += 1;
        self.bytes_left = u32::from_le_bytes([data[off], data[off+1], data[off+2], data[off+3]]) as usize;
        off += 4;
        self.use_dma = data[off] != 0; off += 1;
        self.dma_request = data[off] != 0; off += 1;
        self.dma_terminate = data[off] != 0; off += 1;
        self.data_rate = i32::from_le_bytes([data[off], data[off+1], data[off+2], data[off+3]]);
        off += 4;
        self.poll_timer = data[off] as i32; off += 1;
        self.poll_drive = data[off]; off += 1;
        for p in &mut self.previously_ready {
            *p = data[off] != 0; off += 1;
        }
        self.phase = match data[off] {
            0 => FdcPhase::Command,
            1 => FdcPhase::Result,
            2 => FdcPhase::ExecuteReadData,
            3 => FdcPhase::ExecuteWriteData,
            _ => FdcPhase::ExecuteFormatTrack,
        };
        off += 1;
        self.active_drive = data[off] as usize; off += 1;
        for d in &mut self.drives {
            d.inserted = data[off] != 0; off += 1;
            d.running = data[off] != 0; off += 1;
            d.write_protected = data[off] != 0; off += 1;
            d.changed = data[off] != 0; off += 1;
            d.cylinder = i32::from_le_bytes([data[off], data[off+1], data[off+2], data[off+3]]);
            off += 4;
        }
        self.command.clear();
        self.result_queue.clear();
        self.data_read = std::ptr::null();
        self.data_write = std::ptr::null_mut();
        self.data_end = std::ptr::null();
        off
    }
}

pub struct Mapper761 {
    fdc: Fdc,
    ram: [u8; 2048],
    prg_bank: u8,
    bios_bank: u8,
    enable_irq: bool,
    mask: u32,
    counter: u32,
    irq_clear_pending: bool,
    disk_data: Vec<Vec<u8>>,
    disk_number: usize,
    next_disk: usize,
}

impl Mapper761 {
    pub fn new() -> Self {
        Self {
            fdc: Fdc::new(),
            ram: [0; 2048],
            prg_bank: 0,
            bios_bank: 0,
            enable_irq: false,
            mask: 0,
            counter: 0,
            irq_clear_pending: false,
            disk_data: Vec::new(),
            disk_number: usize::MAX,
            next_disk: 0,
        }
    }

    fn sync(&mut self) {
        self.fdc.drives[0].inserted = false;
        self.fdc.drives[0].data.clear();
        if self.disk_number < self.disk_data.len() {
            let disk = self.disk_data[self.disk_number].clone();
            self.fdc.insert_disk(0, disk);
        }
    }
}

impl Mapper for Mapper761 {
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address >= 0x4000 && address < 0x5000 {
            if address & 0x0800 != 0 {
                let addr = (address & 0x07FF) as usize;
                FetchResult { data: self.ram[addr], driven: true }
            } else {
                match address & 0x0FFF {
                    0x040..=0x047 => {
                        let data = self.fdc.read_io(address as u32);
                        FetchResult { data, driven: true }
                    }
                    0x04C => {
                        let data = if self.fdc.irq_raised() { 0x80 } else { 0x00 };
                        FetchResult { data, driven: true }
                    }
                    _ => FetchResult { data: 0, driven: false },
                }
            }
        } else if address >= 0x6000 && address < 0xE000 {
            FetchResult { data: 0, driven: false }
        } else if address >= 0xE000 {
            let offset = (address as usize - 0xE000) + (self.bios_bank as usize * 0x2000);
            let data = if offset < cart.prg_rom.len() {
                cart.prg_rom[offset]
            } else {
                0
            };
            FetchResult { data, driven: true }
        } else {
            FetchResult { data: 0, driven: false }
        }
    }

    fn store_prg(&mut self, _cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x4000 && address < 0x5000 {
            if address & 0x0800 != 0 {
                let addr = (address & 0x07FF) as usize;
                self.ram[addr] = data;
            } else {
                match address & 0x0FFF {
                    0x040..=0x047 => {
                        self.fdc.write_io(address as u32, data);
                    }
                    0x048 => {
                        self.enable_irq = true;
                        self.counter = 0;
                    }
                    0x049 => {
                        self.enable_irq = false;
                        self.irq_clear_pending = true;
                    }
                    0x04B => {
                        self.enable_irq = false;
                        self.irq_clear_pending = true;
                    }
                    0x04C => {
                        self.irq_clear_pending = true;
                        self.mask = 0;
                        if data & 0x08 != 0 {
                            let which_bit = data & 7;
                            self.mask |= (1u32 << which_bit) << 7;
                            if which_bit == 0 {
                                self.irq_clear_pending = true;
                            }
                        }
                        self.mask |= 1 << 23;
                    }
                    _ => {}
                }
            }
        } else if address >= 0xE000 && address < 0xF000 {
            if address & 1 == 0 {
                self.prg_bank = data & 0x7F;
            } else {
                self.bios_bank = data & 0x01;
            }
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.fdc.run();
        if self.enable_irq {
            if (self.counter & self.mask) == 0 {
                self.counter = self.counter.wrapping_add(1);
                if (self.counter & self.mask) != 0 {
                    return true;
                }
            } else {
                self.counter = self.counter.wrapping_add(1);
            }
        }
        false
    }

    fn cpu_clock_irq_level(&self) -> bool {
        true
    }

    fn take_irq_ack(&mut self) -> bool {
        if self.irq_clear_pending {
            self.irq_clear_pending = false;
            true
        } else {
            false
        }
    }

    fn change_disk(&mut self) {
        if self.disk_data.is_empty() {
            return;
        }
        if self.disk_number == usize::MAX {
            self.disk_number = 0;
        } else {
            self.fdc.eject_disk(0);
            self.next_disk = (self.disk_number + 1) % self.disk_data.len();
            self.disk_number = self.next_disk;
        }
        self.sync();
    }

    fn disk_inserted(&self) -> bool {
        self.disk_number != usize::MAX && self.disk_number < self.disk_data.len()
    }

    fn eject_disk(&mut self) {
        self.fdc.eject_disk(0);
        self.disk_number = usize::MAX;
    }

    fn insert_disk(&mut self) {
        if self.next_disk < self.disk_data.len() {
            self.disk_number = self.next_disk;
            self.sync();
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        let a10 = (address >> 11) & 1;
        (0x2000 | (address & 0x03FF) | (a10 << 10)) as u16
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
        let ciram = address >= 0x2000;
        let mut new_addr_bus = ppu_address_bus & 0xFF00;
        if !ciram {
            let byte = chr_ram[address as usize & 0x1FFF];
            new_addr_bus |= byte as u16;
        } else {
            let a10 = (address >> 11) & 1;
            new_addr_bus |= vram[(0x2000 | (address & 0x03FF) | (a10 << 10)) as usize & 0x7FF] as u16;
        }
        (new_addr_bus as u8, new_addr_bus)
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.ram);
        data.push(self.prg_bank);
        data.push(self.bios_bank);
        data.push(self.enable_irq as u8);
        data.extend_from_slice(&self.mask.to_le_bytes());
        data.extend_from_slice(&self.counter.to_le_bytes());
        data.extend_from_slice(&self.fdc.save_state());
        data.extend_from_slice(&(self.disk_number as u32).to_le_bytes());
        data.extend_from_slice(&(self.next_disk as u32).to_le_bytes());
        data
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], mut off: usize) -> usize {
        for i in 0..2048 {
            self.ram[i] = state[off];
            off += 1;
        }
        self.prg_bank = state[off]; off += 1;
        self.bios_bank = state[off]; off += 1;
        self.enable_irq = state[off] != 0; off += 1;
        self.mask = u32::from_le_bytes([state[off], state[off+1], state[off+2], state[off+3]]); off += 4;
        self.counter = u32::from_le_bytes([state[off], state[off+1], state[off+2], state[off+3]]); off += 4;
        off = self.fdc.load_state(state, off);
        self.disk_number = u32::from_le_bytes([state[off], state[off+1], state[off+2], state[off+3]]) as usize; off += 4;
        self.next_disk = u32::from_le_bytes([state[off], state[off+1], state[off+2], state[off+3]]) as usize; off += 4;
        self.sync();
        off
    }
}
