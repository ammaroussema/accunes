const PIC_ROM_MASK: u16 = 0x1ff;
const PIC_RAM_MASK: u8 = 0x1f;
const ADDR_MASK: u16 = 0x7ff;

const TMR0: usize = 1;
const PCL: usize = 2;
const STATUS: usize = 3;
const FSR: usize = 4;
const PORTA: usize = 5;
const PORTB: usize = 6;
const PORTC: usize = 7;

const PA_REG: u8 = 0xe0;
const TO_FLAG: u8 = 0x10;
const PD_FLAG: u8 = 0x08;
const Z_FLAG: u8 = 0x04;
const DC_FLAG: u8 = 0x02;
const C_FLAG: u8 = 0x01;

const T0CS_FLAG: u8 = 0x20;
const T0SE_FLAG: u8 = 0x10;
const PSA_FLAG: u8 = 0x08;
const PS_REG: u8 = 0x07;

const WDTE_FLAG: u16 = 0x04;

const PORT_A: i32 = 0;
const PORT_B: i32 = 1;

const BIT_CLR: [u8; 8] = [0xfe, 0xfd, 0xfb, 0xf7, 0xef, 0xdf, 0xbf, 0x7f];
const BIT_SET: [u8; 8] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80];

#[derive(Clone, Copy)]
enum Op {
    Nop,
    Illegal,
    Movwf,
    Clrw,
    Clrf,
    Subwf,
    Decf,
    Iorwf,
    Andwf,
    Xorwf,
    Addwf,
    Movf,
    Comf,
    Incf,
    Decfsz,
    Rrf,
    Rlf,
    Swapf,
    Incfsz,
    Bcf,
    Bsf,
    Btfsc,
    Btfss,
    Retlw,
    Call,
    Goto,
    Movlw,
    Iorlw,
    Andlw,
    Xorlw,
    Option,
    Sleepic,
    Clrwdt,
    Tris,
}

const OPCODE_MAIN: [(u8, Op); 256] = [
    (1, Op::Nop), (1, Op::Illegal), (1, Op::Movwf), (1, Op::Movwf),
    (1, Op::Clrw), (1, Op::Illegal), (1, Op::Clrf), (1, Op::Clrf),
    (1, Op::Subwf), (1, Op::Subwf), (1, Op::Subwf), (1, Op::Subwf),
    (1, Op::Decf), (1, Op::Decf), (1, Op::Decf), (1, Op::Decf),
    (1, Op::Iorwf), (1, Op::Iorwf), (1, Op::Iorwf), (1, Op::Iorwf),
    (1, Op::Andwf), (1, Op::Andwf), (1, Op::Andwf), (1, Op::Andwf),
    (1, Op::Xorwf), (1, Op::Xorwf), (1, Op::Xorwf), (1, Op::Xorwf),
    (1, Op::Addwf), (1, Op::Addwf), (1, Op::Addwf), (1, Op::Addwf),
    (1, Op::Movf), (1, Op::Movf), (1, Op::Movf), (1, Op::Movf),
    (1, Op::Comf), (1, Op::Comf), (1, Op::Comf), (1, Op::Comf),
    (1, Op::Incf), (1, Op::Incf), (1, Op::Incf), (1, Op::Incf),
    (1, Op::Decfsz), (1, Op::Decfsz), (1, Op::Decfsz), (1, Op::Decfsz),
    (1, Op::Rrf), (1, Op::Rrf), (1, Op::Rrf), (1, Op::Rrf),
    (1, Op::Rlf), (1, Op::Rlf), (1, Op::Rlf), (1, Op::Rlf),
    (1, Op::Swapf), (1, Op::Swapf), (1, Op::Swapf), (1, Op::Swapf),
    (1, Op::Incfsz), (1, Op::Incfsz), (1, Op::Incfsz), (1, Op::Incfsz),
    (1, Op::Bcf), (1, Op::Bcf), (1, Op::Bcf), (1, Op::Bcf),
    (1, Op::Bcf), (1, Op::Bcf), (1, Op::Bcf), (1, Op::Bcf),
    (1, Op::Bcf), (1, Op::Bcf), (1, Op::Bcf), (1, Op::Bcf),
    (1, Op::Bcf), (1, Op::Bcf), (1, Op::Bcf), (1, Op::Bcf),
    (1, Op::Bsf), (1, Op::Bsf), (1, Op::Bsf), (1, Op::Bsf),
    (1, Op::Bsf), (1, Op::Bsf), (1, Op::Bsf), (1, Op::Bsf),
    (1, Op::Bsf), (1, Op::Bsf), (1, Op::Bsf), (1, Op::Bsf),
    (1, Op::Bsf), (1, Op::Bsf), (1, Op::Bsf), (1, Op::Bsf),
    (1, Op::Btfsc), (1, Op::Btfsc), (1, Op::Btfsc), (1, Op::Btfsc),
    (1, Op::Btfsc), (1, Op::Btfsc), (1, Op::Btfsc), (1, Op::Btfsc),
    (1, Op::Btfsc), (1, Op::Btfsc), (1, Op::Btfsc), (1, Op::Btfsc),
    (1, Op::Btfsc), (1, Op::Btfsc), (1, Op::Btfsc), (1, Op::Btfsc),
    (1, Op::Btfss), (1, Op::Btfss), (1, Op::Btfss), (1, Op::Btfss),
    (1, Op::Btfss), (1, Op::Btfss), (1, Op::Btfss), (1, Op::Btfss),
    (1, Op::Btfss), (1, Op::Btfss), (1, Op::Btfss), (1, Op::Btfss),
    (1, Op::Btfss), (1, Op::Btfss), (1, Op::Btfss), (1, Op::Btfss),
    (2, Op::Retlw), (2, Op::Retlw), (2, Op::Retlw), (2, Op::Retlw),
    (2, Op::Retlw), (2, Op::Retlw), (2, Op::Retlw), (2, Op::Retlw),
    (2, Op::Retlw), (2, Op::Retlw), (2, Op::Retlw), (2, Op::Retlw),
    (2, Op::Retlw), (2, Op::Retlw), (2, Op::Retlw), (2, Op::Retlw),
    (2, Op::Call), (2, Op::Call), (2, Op::Call), (2, Op::Call),
    (2, Op::Call), (2, Op::Call), (2, Op::Call), (2, Op::Call),
    (2, Op::Call), (2, Op::Call), (2, Op::Call), (2, Op::Call),
    (2, Op::Call), (2, Op::Call), (2, Op::Call), (2, Op::Call),
    (2, Op::Goto), (2, Op::Goto), (2, Op::Goto), (2, Op::Goto),
    (2, Op::Goto), (2, Op::Goto), (2, Op::Goto), (2, Op::Goto),
    (2, Op::Goto), (2, Op::Goto), (2, Op::Goto), (2, Op::Goto),
    (2, Op::Goto), (2, Op::Goto), (2, Op::Goto), (2, Op::Goto),
    (2, Op::Goto), (2, Op::Goto), (2, Op::Goto), (2, Op::Goto),
    (2, Op::Goto), (2, Op::Goto), (2, Op::Goto), (2, Op::Goto),
    (2, Op::Goto), (2, Op::Goto), (2, Op::Goto), (2, Op::Goto),
    (1, Op::Movlw), (1, Op::Movlw), (1, Op::Movlw), (1, Op::Movlw),
    (1, Op::Movlw), (1, Op::Movlw), (1, Op::Movlw), (1, Op::Movlw),
    (1, Op::Movlw), (1, Op::Movlw), (1, Op::Movlw), (1, Op::Movlw),
    (1, Op::Movlw), (1, Op::Movlw), (1, Op::Movlw), (1, Op::Movlw),
    (1, Op::Iorlw), (1, Op::Iorlw), (1, Op::Iorlw), (1, Op::Iorlw),
    (1, Op::Iorlw), (1, Op::Iorlw), (1, Op::Iorlw), (1, Op::Iorlw),
    (1, Op::Iorlw), (1, Op::Iorlw), (1, Op::Iorlw), (1, Op::Iorlw),
    (1, Op::Iorlw), (1, Op::Iorlw), (1, Op::Iorlw), (1, Op::Iorlw),
    (1, Op::Andlw), (1, Op::Andlw), (1, Op::Andlw), (1, Op::Andlw),
    (1, Op::Andlw), (1, Op::Andlw), (1, Op::Andlw), (1, Op::Andlw),
    (1, Op::Andlw), (1, Op::Andlw), (1, Op::Andlw), (1, Op::Andlw),
    (1, Op::Andlw), (1, Op::Andlw), (1, Op::Andlw), (1, Op::Andlw),
    (1, Op::Xorlw), (1, Op::Xorlw), (1, Op::Xorlw), (1, Op::Xorlw),
    (1, Op::Xorlw), (1, Op::Xorlw), (1, Op::Xorlw), (1, Op::Xorlw),
    (1, Op::Xorlw), (1, Op::Xorlw), (1, Op::Xorlw), (1, Op::Xorlw),
    (1, Op::Xorlw), (1, Op::Xorlw), (1, Op::Xorlw), (1, Op::Xorlw),
    (1, Op::Xorlw), (1, Op::Xorlw), (1, Op::Xorlw), (1, Op::Xorlw),
];

const OPCODE_00X: [(u8, Op); 16] = [
    (1, Op::Nop), (1, Op::Illegal), (1, Op::Option), (1, Op::Sleepic),
    (1, Op::Clrwdt), (1, Op::Tris), (1, Op::Tris), (1, Op::Tris),
    (1, Op::Illegal), (1, Op::Illegal), (1, Op::Illegal), (1, Op::Illegal),
    (1, Op::Illegal), (1, Op::Illegal), (1, Op::Illegal), (1, Op::Illegal),
];

pub struct Pic16C54 {
    pc: u16,
    prev_pc: u16,
    w: u8,
    option: u8,
    config: u16,
    alu: u8,
    wdt: u16,
    trisa: u8,
    trisb: u8,
    trisc: u8,
    stack: [u16; 2],
    prescaler: u16,
    opcode: u16,
    internal_ram: [u8; 128],
    rom: Vec<u8>,
    icount: i32,
    temp_config: u16,
    delay_timer: i32,
    rtcc: i32,
    count_pending: bool,
    old_data: u8,
    inst_cycles: i32,
    clock2cycle: i32,
    cpu_address: u16,
}

impl Pic16C54 {
    pub fn new(rom: Vec<u8>) -> Self {
        let mut pic = Pic16C54 {
            pc: 0,
            prev_pc: 0,
            w: 0,
            option: 0,
            config: 0,
            alu: 0,
            wdt: 0,
            trisa: 0xff,
            trisb: 0xff,
            trisc: 0xff,
            stack: [0; 2],
            prescaler: 0,
            opcode: 0,
            internal_ram: [0; 128],
            rom,
            icount: 0,
            temp_config: 0,
            delay_timer: 0,
            rtcc: 0,
            count_pending: false,
            old_data: 0,
            inst_cycles: 0,
            clock2cycle: 0,
            cpu_address: 0,
        };
        pic.reset(true);
        pic
    }

    pub fn reset(&mut self, hard: bool) {
        if hard {
            self.internal_ram.fill(0);
            self.reset_regs();
            self.internal_ram[STATUS] &= !PA_REG;
            self.internal_ram[STATUS] |= TO_FLAG | PD_FLAG;
            self.icount = 0;
            self.clock2cycle = 0;
        } else {
            self.internal_ram[STATUS] |= TO_FLAG | PD_FLAG | Z_FLAG | DC_FLAG | C_FLAG;
            self.reset_regs();
        }
    }

    pub fn run(&mut self, cpu_address: u16, irq_out: &mut bool) {
        self.cpu_address = cpu_address;

        self.clock2cycle = self.clock2cycle.wrapping_add(1);
        if (self.clock2cycle & 3) == 0 {
            self.icount += 1;
        }

        while self.icount > 0 {
            if self.internal_ram[STATUS] & PD_FLAG == 0 {
                self.count_pending = false;
                self.inst_cycles = 1;
                if self.config & WDTE_FLAG != 0 {
                    self.update_watchdog(1);
                }
            } else {
                if self.count_pending {
                    self.count_pending = false;
                    self.update_timer(1);
                }

                self.prev_pc = self.pc;
                self.opcode = self.rd_op(self.pc);
                self.pc = self.pc.wrapping_add(1);
                self.internal_ram[PCL] = self.internal_ram[PCL].wrapping_add(1);

                let (cycles, op) = if self.opcode & 0xff0 != 0 {
                    OPCODE_MAIN[((self.opcode >> 4) & 0xff) as usize]
                } else {
                    OPCODE_00X[(self.opcode as u8 & 0x1f) as usize]
                };
                self.inst_cycles = cycles as i32;
                self.execute_op(op, irq_out);

                if self.option & T0CS_FLAG == 0 {
                    if self.delay_timer != 0 {
                        self.delay_timer -= 1;
                    } else {
                        self.update_timer(self.inst_cycles);
                    }
                }
                if self.config & WDTE_FLAG != 0 {
                    self.update_watchdog(self.inst_cycles);
                }
            }

            self.icount -= self.inst_cycles;
        }
    }

    pub fn save_state(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(171);
        data.extend_from_slice(&self.pc.to_le_bytes());
        data.extend_from_slice(&self.prev_pc.to_le_bytes());
        data.push(self.w);
        data.push(self.option);
        data.extend_from_slice(&self.config.to_le_bytes());
        data.push(self.alu);
        data.extend_from_slice(&self.wdt.to_le_bytes());
        data.push(self.trisa);
        data.push(self.trisb);
        data.push(self.trisc);
        data.extend_from_slice(&self.stack[0].to_le_bytes());
        data.extend_from_slice(&self.stack[1].to_le_bytes());
        data.extend_from_slice(&self.prescaler.to_le_bytes());
        data.extend_from_slice(&self.opcode.to_le_bytes());
        data.extend_from_slice(&self.internal_ram);
        data.extend_from_slice(&(self.icount as i32).to_le_bytes());
        data.extend_from_slice(&(self.delay_timer as i32).to_le_bytes());
        data.extend_from_slice(&(self.rtcc as i32).to_le_bytes());
        data.push(u8::from(self.count_pending));
        data.extend_from_slice(&(self.inst_cycles as i32).to_le_bytes());
        data.extend_from_slice(&(self.clock2cycle as i32).to_le_bytes());
        data
    }

    pub fn load_state(&mut self, data: &[u8]) -> usize {
        if data.len() < 171 {
            return 0;
        }
        let mut offset = 0;
        self.pc = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        self.prev_pc = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        self.w = data[offset];
        offset += 1;
        self.option = data[offset];
        offset += 1;
        self.config = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        self.alu = data[offset];
        offset += 1;
        self.wdt = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        self.trisa = data[offset];
        offset += 1;
        self.trisb = data[offset];
        offset += 1;
        self.trisc = data[offset];
        offset += 1;
        self.stack[0] = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        self.stack[1] = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        self.prescaler = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        self.opcode = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        self.internal_ram.copy_from_slice(&data[offset..offset + 128]);
        offset += 128;
        self.icount = i32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        offset += 4;
        self.delay_timer = i32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        offset += 4;
        self.rtcc = i32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        offset += 4;
        self.count_pending = data[offset] != 0;
        offset += 1;
        self.inst_cycles = i32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        offset += 4;
        self.clock2cycle = i32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        offset += 4;
        offset
    }

    fn reset_regs(&mut self) {
        self.pc = PIC_ROM_MASK;
        self.config = self.temp_config;
        self.trisa = 0xff;
        self.trisb = 0xff;
        self.trisc = 0xff;
        self.option = T0CS_FLAG | T0SE_FLAG | PSA_FLAG | PS_REG;
        self.internal_ram[PCL] = 0xff;
        self.internal_ram[FSR] |= !PIC_RAM_MASK;
        self.prescaler = 0;
        self.delay_timer = 0;
        self.inst_cycles = 0;
        self.count_pending = false;
    }

    fn rd_op(&self, addr: u16) -> u16 {
        let idx = ((addr & PIC_ROM_MASK) as usize) << 1;
        if idx + 1 < self.rom.len() {
            self.rom[idx] as u16 | (self.rom[idx + 1] as u16) << 8
        } else {
            0
        }
    }

    fn rd_ram(&self, addr: u8) -> u8 {
        self.internal_ram[(addr & PIC_RAM_MASK) as usize]
    }

    fn wr_ram(&mut self, addr: u8, val: u8) {
        self.internal_ram[(addr & PIC_RAM_MASK) as usize] = val;
    }

    fn addr(&self) -> u8 {
        self.opcode as u8 & 0x1f
    }

    fn pos(&self) -> usize {
        ((self.opcode >> 5) & 7) as usize
    }

    fn ps(&self) -> u8 {
        self.option & PS_REG
    }

    fn psa(&self) -> u8 {
        self.option & PSA_FLAG
    }

    fn calculate_z_flag(&mut self) {
        if self.alu == 0 {
            self.internal_ram[STATUS] |= Z_FLAG;
        } else {
            self.internal_ram[STATUS] &= !Z_FLAG;
        }
    }

    fn calculate_add_carry(&mut self) {
        if self.old_data > self.alu {
            self.internal_ram[STATUS] |= C_FLAG;
        } else {
            self.internal_ram[STATUS] &= !C_FLAG;
        }
    }

    fn calculate_add_digit_carry(&mut self) {
        if (self.old_data & 0x0f) > (self.alu & 0x0f) {
            self.internal_ram[STATUS] |= DC_FLAG;
        } else {
            self.internal_ram[STATUS] &= !DC_FLAG;
        }
    }

    fn calculate_sub_carry(&mut self) {
        if self.old_data < self.alu {
            self.internal_ram[STATUS] &= !C_FLAG;
        } else {
            self.internal_ram[STATUS] |= C_FLAG;
        }
    }

    fn calculate_sub_digit_carry(&mut self) {
        if (self.old_data & 0x0f) < (self.alu & 0x0f) {
            self.internal_ram[STATUS] &= !DC_FLAG;
        } else {
            self.internal_ram[STATUS] |= DC_FLAG;
        }
    }

    fn pop_stack(&mut self) -> u16 {
        let data = self.stack[1];
        self.stack[1] = self.stack[0];
        data & ADDR_MASK
    }

    fn push_stack(&mut self, data: u16) {
        self.stack[0] = self.stack[1];
        self.stack[1] = data & ADDR_MASK;
    }

    fn read_io(&self, port: i32) -> u8 {
        let addr = self.cpu_address;
        match port {
            PORT_A => {
                1 | (if addr & 0x0040 != 0 { 2 } else { 0 })
                    | (if addr & 0x0020 != 0 { 4 } else { 0 })
                    | (if addr & 0x0010 != 0 { 8 } else { 0 })
            }
            PORT_B => {
                (if addr & 0x1000 != 0 { 1 } else { 0 })
                    | (if addr & 0x0080 != 0 { 2 } else { 0 })
                    | (if addr & 0x0400 != 0 { 4 } else { 0 })
                    | (if addr & 0x0800 != 0 { 8 } else { 0 })
                    | (if addr & 0x0200 != 0 { 16 } else { 0 })
                    | (if addr & 0x0100 != 0 { 32 } else { 0 })
                    | (if addr & 0x2000 != 0 { 64 } else { 0 })
                    | (if addr & 0x4000 != 0 { 128 } else { 0 })
            }
            _ => 0xff,
        }
    }

    fn write_io(&self, port: i32, val: u16, irq_out: &mut bool) {
        if port == PORT_A {
            *irq_out = val & 1 != 0;
        }
    }

    pub fn irq_level(&self) -> bool {
        (self.internal_ram[PORTA] & !self.trisa & 1) != 0
    }

    fn get_regfile(&self, mut addr: u32) -> u8 {
        if addr == 0 {
            addr = (self.internal_ram[FSR] & PIC_RAM_MASK) as u32;
        }

        if (addr & 0x10) == 0 {
            addr &= 0x0f;
        }

        match addr {
            0 => 0,
            4 => self.internal_ram[FSR] | !PIC_RAM_MASK,
            5 => {
                let mut data = self.read_io(PORT_A);
                data &= self.trisa;
                data |= (!self.trisa) & self.internal_ram[PORTA];
                data & 0x0f
            }
            6 => {
                let mut data = self.read_io(PORT_B);
                data &= self.trisb;
                data |= (!self.trisb) & self.internal_ram[PORTB];
                data
            }
            7 => self.rd_ram(addr as u8),
            _ => self.rd_ram(addr as u8),
        }
    }

    fn store_regfile(&mut self, mut addr: u32, data: u8, irq_out: &mut bool) {
        if addr == 0 {
            addr = (self.internal_ram[FSR] & PIC_RAM_MASK) as u32;
        }

        if (addr & 0x10) == 0 {
            addr &= 0x0f;
        }

        match addr {
            0 => {}
            1 => {
                self.delay_timer = 2;
                if self.psa() == 0 {
                    self.prescaler = 0;
                }
                self.internal_ram[TMR0] = data;
            }
            2 => {
                self.internal_ram[PCL] = data;
                self.pc = ((self.internal_ram[STATUS] & PA_REG) as u16) << 4 | data as u16;
            }
            3 => {
                self.internal_ram[STATUS] = (self.internal_ram[STATUS] & (TO_FLAG | PD_FLAG))
                    | (data & !(TO_FLAG | PD_FLAG));
            }
            4 => {
                self.internal_ram[FSR] = data | !PIC_RAM_MASK;
            }
            5 => {
                let data = data & 0x0f;
                self.write_io(PORT_A, (data & !self.trisa) as u16, irq_out);
                self.internal_ram[PORTA] = data;
            }
            6 => {
                self.write_io(PORT_B, (data & !self.trisb) as u16, irq_out);
                self.internal_ram[PORTB] = data;
            }
            7 => {
                self.internal_ram[PORTC] = data;
            }
            _ => self.wr_ram(addr as u8, data),
        }
    }

    fn store_result(&mut self, addr: u8, data: u8, irq_out: &mut bool) {
        if self.opcode as u8 & 0x20 != 0 {
            self.store_regfile(addr as u32, data, irq_out);
        } else {
            self.w = data;
        }
    }

    fn skip_next(&mut self) {
        self.pc = self.pc.wrapping_add(1);
        self.internal_ram[PCL] = self.internal_ram[PCL].wrapping_add(1);
        self.inst_cycles += 1;
    }

    fn execute_op(&mut self, op: Op, irq_out: &mut bool) {
        match op {
            Op::Nop => {}
            Op::Illegal => {}
            Op::Addwf => {
                let addr = self.addr();
                self.old_data = self.get_regfile(addr as u32);
                self.alu = self.old_data.wrapping_add(self.w);
                self.store_result(addr, self.alu, irq_out);
                self.calculate_z_flag();
                self.calculate_add_carry();
                self.calculate_add_digit_carry();
            }
            Op::Andwf => {
                let addr = self.addr();
                self.alu = self.get_regfile(addr as u32) & self.w;
                self.store_result(addr, self.alu, irq_out);
                self.calculate_z_flag();
            }
            Op::Andlw => {
                self.alu = (self.opcode as u8) & self.w;
                self.w = self.alu;
                self.calculate_z_flag();
            }
            Op::Bcf => {
                let addr = self.addr();
                self.alu = self.get_regfile(addr as u32) & BIT_CLR[self.pos()];
                self.store_regfile(addr as u32, self.alu, irq_out);
            }
            Op::Bsf => {
                let addr = self.addr();
                self.alu = self.get_regfile(addr as u32) | BIT_SET[self.pos()];
                self.store_regfile(addr as u32, self.alu, irq_out);
            }
            Op::Btfss => {
                let addr = self.addr();
                if self.get_regfile(addr as u32) & BIT_SET[self.pos()] == BIT_SET[self.pos()] {
                    self.skip_next();
                }
            }
            Op::Btfsc => {
                let addr = self.addr();
                if self.get_regfile(addr as u32) & BIT_SET[self.pos()] == 0 {
                    self.skip_next();
                }
            }
            Op::Call => {
                self.push_stack(self.pc);
                self.pc = ((self.internal_ram[STATUS] & PA_REG) as u16) << 4
                    | (self.opcode as u8 as u16);
                self.pc &= 0x6ff;
                self.internal_ram[PCL] = self.pc as u8;
            }
            Op::Clrw => {
                self.w = 0;
                self.internal_ram[STATUS] |= Z_FLAG;
            }
            Op::Clrf => {
                let addr = self.addr();
                self.store_regfile(addr as u32, 0, irq_out);
                self.internal_ram[STATUS] |= Z_FLAG;
            }
            Op::Clrwdt => {
                self.wdt = 0;
                if self.psa() != 0 {
                    self.prescaler = 0;
                }
                self.internal_ram[STATUS] |= TO_FLAG | PD_FLAG;
            }
            Op::Comf => {
                let addr = self.addr();
                self.alu = !self.get_regfile(addr as u32);
                self.store_result(addr, self.alu, irq_out);
                self.calculate_z_flag();
            }
            Op::Decf => {
                let addr = self.addr();
                self.alu = self.get_regfile(addr as u32).wrapping_sub(1);
                self.store_result(addr, self.alu, irq_out);
                self.calculate_z_flag();
            }
            Op::Decfsz => {
                let addr = self.addr();
                self.alu = self.get_regfile(addr as u32).wrapping_sub(1);
                self.store_result(addr, self.alu, irq_out);
                if self.alu == 0 {
                    self.skip_next();
                }
            }
            Op::Goto => {
                self.pc = ((self.internal_ram[STATUS] & PA_REG) as u16) << 4
                    | (self.opcode & 0x1ff);
                self.pc &= ADDR_MASK;
                self.internal_ram[PCL] = self.pc as u8;
            }
            Op::Incf => {
                let addr = self.addr();
                self.alu = self.get_regfile(addr as u32).wrapping_add(1);
                self.store_result(addr, self.alu, irq_out);
                self.calculate_z_flag();
            }
            Op::Incfsz => {
                let addr = self.addr();
                self.alu = self.get_regfile(addr as u32).wrapping_add(1);
                self.store_result(addr, self.alu, irq_out);
                if self.alu == 0 {
                    self.skip_next();
                }
            }
            Op::Iorlw => {
                self.alu = (self.opcode as u8) | self.w;
                self.w = self.alu;
                self.calculate_z_flag();
            }
            Op::Iorwf => {
                let addr = self.addr();
                self.alu = self.get_regfile(addr as u32) | self.w;
                self.store_result(addr, self.alu, irq_out);
                self.calculate_z_flag();
            }
            Op::Movf => {
                let addr = self.addr();
                self.alu = self.get_regfile(addr as u32);
                self.store_result(addr, self.alu, irq_out);
                self.calculate_z_flag();
            }
            Op::Movlw => {
                self.w = self.opcode as u8;
            }
            Op::Movwf => {
                let addr = self.addr();
                self.store_regfile(addr as u32, self.w, irq_out);
            }
            Op::Option => {
                self.option = self.w & (T0CS_FLAG | T0SE_FLAG | PSA_FLAG | PS_REG);
            }
            Op::Retlw => {
                self.w = self.opcode as u8;
                self.pc = self.pop_stack();
                self.internal_ram[PCL] = self.pc as u8;
            }
            Op::Rlf => {
                let addr = self.addr();
                self.alu = self.get_regfile(addr as u32);
                let bit7 = self.alu & 0x80;
                self.alu <<= 1;
                if self.internal_ram[STATUS] & C_FLAG != 0 {
                    self.alu |= 1;
                }
                self.store_result(addr, self.alu, irq_out);
                if bit7 != 0 {
                    self.internal_ram[STATUS] |= C_FLAG;
                } else {
                    self.internal_ram[STATUS] &= !C_FLAG;
                }
            }
            Op::Rrf => {
                let addr = self.addr();
                self.alu = self.get_regfile(addr as u32);
                let bit0 = self.alu & 1;
                self.alu >>= 1;
                if self.internal_ram[STATUS] & C_FLAG != 0 {
                    self.alu |= 0x80;
                }
                self.store_result(addr, self.alu, irq_out);
                if bit0 != 0 {
                    self.internal_ram[STATUS] |= C_FLAG;
                } else {
                    self.internal_ram[STATUS] &= !C_FLAG;
                }
            }
            Op::Sleepic => {
                if self.config & WDTE_FLAG != 0 {
                    self.wdt = 0;
                }
                if self.psa() != 0 {
                    self.prescaler = 0;
                }
                self.internal_ram[STATUS] |= TO_FLAG;
                self.internal_ram[STATUS] &= !PD_FLAG;
            }
            Op::Subwf => {
                let addr = self.addr();
                self.old_data = self.get_regfile(addr as u32);
                self.alu = self.old_data.wrapping_sub(self.w);
                self.store_result(addr, self.alu, irq_out);
                self.calculate_z_flag();
                self.calculate_sub_carry();
                self.calculate_sub_digit_carry();
            }
            Op::Swapf => {
                let addr = self.addr();
                let val = self.get_regfile(addr as u32);
                self.alu = (val << 4) & 0xf0 | (val >> 4) & 0x0f;
                self.store_result(addr, self.alu, irq_out);
            }
            Op::Tris => {
                match self.opcode as u8 & 0x7 {
                    5 => {
                        if self.trisa != self.w {
                            self.trisa = self.w | 0xf0;
                            self.write_io(
                                PORT_A,
                                0x1000 | ((self.internal_ram[PORTA] & !self.trisa) & 0x0f) as u16,
                                irq_out,
                            );
                        }
                    }
                    6 => {
                        if self.trisb != self.w {
                            self.trisb = self.w;
                            self.write_io(
                                PORT_B,
                                0x1000 | (self.internal_ram[PORTB] & !self.trisb) as u16,
                                irq_out,
                            );
                        }
                    }
                    _ => {}
                }
            }
            Op::Xorlw => {
                self.alu = self.w ^ (self.opcode as u8);
                self.w = self.alu;
                self.calculate_z_flag();
            }
            Op::Xorwf => {
                let addr = self.addr();
                self.alu = self.get_regfile(addr as u32) ^ self.w;
                self.store_result(addr, self.alu, irq_out);
                self.calculate_z_flag();
            }
        }
    }

    fn update_watchdog(&mut self, counts: i32) {
        if self.opcode == 3 || self.opcode == 4 {
            return;
        }

        let old_wdt = self.wdt;
        self.wdt = self.wdt.wrapping_sub(counts as u16);

        if self.wdt > 0x464f {
            self.wdt = 0x464f - (0xffff - self.wdt);
        }

        if (old_wdt != 0 && old_wdt < self.wdt) || self.wdt == 0 {
            if self.psa() != 0 {
                self.prescaler = self.prescaler.wrapping_add(1);
                if self.prescaler >= (1 << self.ps()) {
                    self.prescaler = 0;
                    self.internal_ram[STATUS] &= !TO_FLAG;
                }
            } else {
                self.internal_ram[STATUS] &= !TO_FLAG;
            }
        }
    }

    fn update_timer(&mut self, counts: i32) {
        if self.psa() == 0 {
            self.prescaler = self.prescaler.wrapping_add(counts as u16);
            let threshold = 2 << self.ps();
            if self.prescaler >= threshold {
                self.internal_ram[TMR0] = self
                    .internal_ram[TMR0]
                    .wrapping_add((self.prescaler / threshold) as u8);
                self.prescaler %= threshold;
            }
        } else {
            self.internal_ram[TMR0] = self.internal_ram[TMR0].wrapping_add(counts as u8);
        }
    }
}
