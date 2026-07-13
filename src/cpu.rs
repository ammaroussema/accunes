/* 
    hey so this is the cpu implementation like the 6502 nes variant logic should be fairly accurate 
    considering it passes all accuracycoin tests, it has all opcodes (legal and illegal) and it handles dma and interrupts properly
    of course no idea if there's some details missed because i don't own an actual nes but this is a close as it'll get haha
*/
use crate::emulator::Emulator;
use std::sync::atomic::{AtomicU32, Ordering};

static CPU_TRACE: AtomicU32 = AtomicU32::new(0);

impl Emulator {
    pub fn cpu_tick(&mut self) {
        // dma handling logic
        if (self.do_dmc_dma && (self.apu_status_dmc || self.apu_implicit_abort_dmc_4015) && self.cpu_read)
            || (self.do_oam_dma && self.cpu_read)
        {
            // these opcodes ignore h during dma i guess
            if (self.op_code == 0x93 && self.operation_cycle == 4)
                || (self.op_code == 0x9B && self.operation_cycle == 3)
                || (self.op_code == 0x9C && self.operation_cycle == 3)
                || (self.op_code == 0x9E && self.operation_cycle == 3)
                || (self.op_code == 0x9F && self.operation_cycle == 3)
            {
                self.ignore_h = true;
            }

            if self.do_oam_dma && self.first_cycle_of_oam_dma {
                self.first_cycle_of_oam_dma = false;
                if !self.apu_put_cycle {
                    self.oam_dma_halt = true;
                }
            }

            if self.apu_put_cycle {
                // dma handling during put cycles
                if self.do_dmc_dma && self.do_oam_dma {
                    if self.dmc_dma_halt && self.oam_dma_halt {
                        self.oam_dma_halted();
                    } else if !self.oam_dma_halt && self.dmc_dma_halt {
                        self.oam_dma_put();
                    } else if self.oam_dma_halt && !self.dmc_dma_halt {
                        self.dmc_dma_put();
                    } else {
                        self.oam_dma_put();
                    }
                } else if self.do_dmc_dma {
                    if self.dmc_dma_halt {
                        self.dmc_dma_halted();
                    } else {
                        self.dmc_dma_put();
                    }
                } else {
                    if self.oam_dma_halt {
                        self.oam_dma_halted();
                    } else {
                        self.oam_dma_put();
                    }
                }
            } else {
                // dma handling during get cycles
                if self.do_dmc_dma && self.do_oam_dma {
                    if self.dmc_dma_halt && self.oam_dma_halt {
                        self.dmc_dma_halted();
                    } else if !self.oam_dma_halt && self.dmc_dma_halt {
                        self.oam_dma_get();
                    } else if self.oam_dma_halt && !self.dmc_dma_halt {
                        self.dmc_dma_get();
                    } else {
                        self.dmc_dma_get();
                    }
                } else if self.do_dmc_dma {
                    if self.dmc_dma_halt {
                        self.dmc_dma_halted();
                    } else {
                        self.dmc_dma_get();
                    }
                } else {
                    if self.oam_dma_halt {
                        self.oam_dma_halted();
                    } else {
                        self.oam_dma_get();
                    }
                }
                self.dmc_dma_halt = false;
                self.oam_dma_halt = false;
            }
            if self.do_dmc_dma && self.apu_implicit_abort_dmc_4015 {
                self.apu_implicit_abort_dmc_4015 = false;
            }
            return;
        }

        // in cycle 0, we fetch the opcode
        if self.operation_cycle == 0 {
            self.address_bus = self.program_counter;
            self.op_code = self.fetch(self.address_bus);

            if self.do_nmi {
                self.op_code = 0x00;
            } else if self.do_irq {
                self.op_code = 0x00;
            } else if self.do_reset {
                self.op_code = 0x00;
            } else if self.op_code == 0x00 {
                self.do_brk = true;
            }

            if CPU_TRACE.load(Ordering::Relaxed) > 0 {
                let pc = self.program_counter;
                let _op = self.op_code;
                if pc != 0x501A && pc != 0x501D {
                    CPU_TRACE.fetch_sub(1, Ordering::Relaxed);
                }
            }

            if !self.do_nmi && !self.do_irq && !self.do_reset {
                self.program_counter = self.program_counter.wrapping_add(1);
                self.address_bus = self.program_counter;
            }

            self.operation_cycle = 1;
            return;
        }

        // then we execute it
        self.execute_opcode();

        // might as well check for implicit dma aborts
        if self.do_dmc_dma && self.apu_implicit_abort_dmc_4015 {
            self.apu_implicit_abort_dmc_4015 = false;
        }

        self.operation_cycle = self.operation_cycle.wrapping_add(1);
    }

    // the dma handling helper function thingies

    fn oam_dma_get(&mut self) {
        let addr = ((self.dma_page as u16) << 8) | (self.dma_address as u16);
        self.oam_dma_aligned = true;
        self.oam_internal_bus = self.fetch(addr);
    }

    fn oam_dma_halted(&mut self) {
        self.fetch(self.address_bus);
    }

    fn oam_dma_put(&mut self) {
        if self.oam_dma_aligned {
            self.store(self.oam_internal_bus, 0x2004);
            self.dma_address = self.dma_address.wrapping_add(1);
            if self.dma_address == 0 {
                self.do_oam_dma = false;
                self.oam_dma_aligned = false;
                return;
            }
        } else {
            self.fetch(self.address_bus);
        }
    }

    fn dmc_dma_get(&mut self) {
        self.apu_dmc_buffer = self.fetch(self.apu_dmc_address_counter);
        self.apu_dmc_address_counter = self.apu_dmc_address_counter.wrapping_add(1);
        if self.apu_dmc_address_counter == 0 {
            self.apu_dmc_address_counter = 0x8000;
        }
        if self.apu_dmc_bytes_remaining > 0 {
            self.apu_dmc_bytes_remaining -= 1;
        }
        if self.apu_dmc_bytes_remaining == 0 {
            if !self.apu_dmc_loop {
                self.apu_status_dmc = false;
                if self.apu_dmc_enable_irq {
                    self.irq_level_detector = true;
                    self.apu_status_dmc_interrupt = true;
                }
            } else {
                self.start_dmc_sample();
            }
        }
        self.do_dmc_dma = false;
        self.oam_dma_aligned = false;
        self.cannot_run_dmc_dma_right_now = 2;
    }

    fn dmc_dma_halted(&mut self) {
        self.fetch(self.address_bus);
    }

    fn dmc_dma_put(&mut self) {
        self.fetch(self.address_bus);
    }

    // this is how we execute the opcodes

    fn execute_opcode(&mut self) {
        match self.op_code {
            0x00 => self.op_brk(),

            // --- LDA ---
            0xA9 => { // LDA #imm
                self.poll_interrupts();
                self.get_immediate();
                self.a = self.dl;
                self.flag_negative = self.a >= 0x80;
                self.flag_zero = self.a == 0;
                self.complete_operation();
            }
            0xA5 => { // LDA zp
                match self.operation_cycle {
                    1 => { self.get_address_zero_page(); }
                    _ => {
                        self.poll_interrupts();
                        self.a = self.fetch(self.address_bus);
                        self.flag_negative = self.a >= 0x80;
                        self.flag_zero = self.a == 0;
                        self.complete_operation();
                    }
                }
            }
            0xB5 => { // LDA zp,X
                match self.operation_cycle {
                    1 | 2 => { self.get_address_zp_off_x(); }
                    _ => {
                        self.poll_interrupts();
                        self.a = self.fetch(self.address_bus);
                        self.flag_negative = self.a >= 0x80;
                        self.flag_zero = self.a == 0;
                        self.complete_operation();
                    }
                }
            }
            0xAD => { // LDA abs
                match self.operation_cycle {
                    1 | 2 => { self.get_address_absolute(); }
                    _ => {
                        self.poll_interrupts();
                        self.a = self.fetch(self.address_bus);
                        self.flag_negative = self.a >= 0x80;
                        self.flag_zero = self.a == 0;
                        self.complete_operation();
                    }
                }
            }
            0xBD => { // LDA abs,X
                match self.operation_cycle {
                    1 | 2 | 3 => { self.get_address_abs_off_x(true); }
                    _ => {
                        self.poll_interrupts();
                        self.a = self.fetch(self.address_bus);
                        self.flag_negative = self.a >= 0x80;
                        self.flag_zero = self.a == 0;
                        self.complete_operation();
                    }
                }
            }
            0xB9 => { // LDA abs,Y
                match self.operation_cycle {
                    1 | 2 | 3 => { self.get_address_abs_off_y(true); }
                    _ => {
                        self.poll_interrupts();
                        self.a = self.fetch(self.address_bus);
                        self.flag_negative = self.a >= 0x80;
                        self.flag_zero = self.a == 0;
                        self.complete_operation();
                    }
                }
            }
            0xA1 => { // LDA (ind,X)
                match self.operation_cycle {
                    1 | 2 | 3 | 4 => { self.get_address_ind_off_x(); }
                    _ => {
                        self.poll_interrupts();
                        self.a = self.fetch(self.address_bus);
                        self.flag_negative = self.a >= 0x80;
                        self.flag_zero = self.a == 0;
                        self.complete_operation();
                    }
                }
            }
            0xB1 => { // LDA (ind),Y
                match self.operation_cycle {
                    1 | 2 | 3 | 4 => { self.get_address_ind_off_y(true); }
                    _ => {
                        self.poll_interrupts();
                        self.a = self.fetch(self.address_bus);
                        self.flag_negative = self.a >= 0x80;
                        self.flag_zero = self.a == 0;
                        self.complete_operation();
                    }
                }
            }

            // --- LDX ---
            0xA2 => { // LDX #imm
                self.poll_interrupts();
                self.get_immediate();
                self.x = self.dl;
                self.flag_negative = self.x >= 0x80;
                self.flag_zero = self.x == 0;
                self.complete_operation();
            }
            0xA6 => { // LDX zp
                match self.operation_cycle {
                    1 => { self.get_address_zero_page(); }
                    _ => {
                        self.poll_interrupts();
                        self.x = self.fetch(self.address_bus);
                        self.flag_negative = self.x >= 0x80;
                        self.flag_zero = self.x == 0;
                        self.complete_operation();
                    }
                }
            }
            0xB6 => { // LDX zp,Y
                match self.operation_cycle {
                    1 | 2 => { self.get_address_zp_off_y(); }
                    _ => {
                        self.poll_interrupts();
                        self.x = self.fetch(self.address_bus);
                        self.flag_negative = self.x >= 0x80;
                        self.flag_zero = self.x == 0;
                        self.complete_operation();
                    }
                }
            }
            0xAE => { // LDX abs
                match self.operation_cycle {
                    1 | 2 => { self.get_address_absolute(); }
                    _ => {
                        self.poll_interrupts();
                        self.x = self.fetch(self.address_bus);
                        self.flag_negative = self.x >= 0x80;
                        self.flag_zero = self.x == 0;
                        self.complete_operation();
                    }
                }
            }
            0xBE => { // LDX abs,Y
                match self.operation_cycle {
                    1 | 2 | 3 => { self.get_address_abs_off_y(true); }
                    _ => {
                        self.poll_interrupts();
                        self.x = self.fetch(self.address_bus);
                        self.flag_negative = self.x >= 0x80;
                        self.flag_zero = self.x == 0;
                        self.complete_operation();
                    }
                }
            }

            // --- LDY ---
            0xA0 => { // LDY #imm
                self.poll_interrupts();
                self.get_immediate();
                self.y = self.dl;
                self.flag_negative = self.y >= 0x80;
                self.flag_zero = self.y == 0;
                self.complete_operation();
            }
            0xA4 => { // LDY zp
                match self.operation_cycle {
                    1 => { self.get_address_zero_page(); }
                    _ => {
                        self.poll_interrupts();
                        self.y = self.fetch(self.address_bus);
                        self.flag_negative = self.y >= 0x80;
                        self.flag_zero = self.y == 0;
                        self.complete_operation();
                    }
                }
            }
            0xB4 => { // LDY zp,X
                match self.operation_cycle {
                    1 | 2 => { self.get_address_zp_off_x(); }
                    _ => {
                        self.poll_interrupts();
                        self.y = self.fetch(self.address_bus);
                        self.flag_negative = self.y >= 0x80;
                        self.flag_zero = self.y == 0;
                        self.complete_operation();
                    }
                }
            }
            0xAC => { // LDY abs
                match self.operation_cycle {
                    1 | 2 => { self.get_address_absolute(); }
                    _ => {
                        self.poll_interrupts();
                        self.y = self.fetch(self.address_bus);
                        self.flag_negative = self.y >= 0x80;
                        self.flag_zero = self.y == 0;
                        self.complete_operation();
                    }
                }
            }
            0xBC => { // LDY abs,X
                match self.operation_cycle {
                    1 | 2 | 3 => { self.get_address_abs_off_x(true); }
                    _ => {
                        self.poll_interrupts();
                        self.y = self.fetch(self.address_bus);
                        self.flag_negative = self.y >= 0x80;
                        self.flag_zero = self.y == 0;
                        self.complete_operation();
                    }
                }
            }

            // --- LAX ---
            0xA7 => { // LAX zp
                match self.operation_cycle {
                    1 => { self.get_address_zero_page(); }
                    _ => {
                        self.poll_interrupts();
                        self.a = self.fetch(self.address_bus);
                        self.x = self.a;
                        self.flag_negative = self.a >= 0x80;
                        self.flag_zero = self.a == 0;
                        self.complete_operation();
                    }
                }
            }
            0xB7 => { // LAX zp,Y
                match self.operation_cycle {
                    1 | 2 => { self.get_address_zp_off_y(); }
                    _ => {
                        self.poll_interrupts();
                        self.a = self.fetch(self.address_bus);
                        self.x = self.a;
                        self.flag_negative = self.a >= 0x80;
                        self.flag_zero = self.a == 0;
                        self.complete_operation();
                    }
                }
            }
            0xAF => { // LAX abs
                match self.operation_cycle {
                    1 | 2 => { self.get_address_absolute(); }
                    _ => {
                        self.poll_interrupts();
                        self.a = self.fetch(self.address_bus);
                        self.x = self.a;
                        self.flag_negative = self.a >= 0x80;
                        self.flag_zero = self.a == 0;
                        self.complete_operation();
                    }
                }
            }
            0xBF => { // LAX abs,Y
                match self.operation_cycle {
                    1 | 2 | 3 => { self.get_address_abs_off_y(true); }
                    _ => {
                        self.poll_interrupts();
                        self.a = self.fetch(self.address_bus);
                        self.x = self.a;
                        self.flag_negative = self.a >= 0x80;
                        self.flag_zero = self.a == 0;
                        self.complete_operation();
                    }
                }
            }
            0xA3 => { // LAX (ind,X)
                match self.operation_cycle {
                    1 | 2 | 3 | 4 => { self.get_address_ind_off_x(); }
                    _ => {
                        self.poll_interrupts();
                        self.a = self.fetch(self.address_bus);
                        self.x = self.a;
                        self.flag_negative = self.a >= 0x80;
                        self.flag_zero = self.a == 0;
                        self.complete_operation();
                    }
                }
            }
            0xB3 => { // LAX (ind),Y
                match self.operation_cycle {
                    1 | 2 | 3 | 4 => { self.get_address_ind_off_y(true); }
                    _ => {
                        self.poll_interrupts();
                        self.a = self.fetch(self.address_bus);
                        self.x = self.a;
                        self.flag_negative = self.a >= 0x80;
                        self.flag_zero = self.a == 0;
                        self.complete_operation();
                    }
                }
            }

            // --- STA ---
            0x85 => { // STA zp
                match self.operation_cycle {
                    1 => { self.get_address_zero_page(); }
                    _ => {
                        self.poll_interrupts();
                        self.store(self.a, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0x95 => { // STA zp,X
                match self.operation_cycle {
                    1 | 2 => { self.get_address_zp_off_x(); }
                    _ => {
                        self.poll_interrupts();
                        self.store(self.a, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0x8D => { // STA abs
                match self.operation_cycle {
                    1 => { self.get_address_absolute(); }
                    2 => {
                        self.get_address_absolute();
                        self.cpu_read = false;
                    }
                    _ => {
                        self.poll_interrupts();
                        self.store(self.a, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0x9D => { // STA abs,X
                match self.operation_cycle {
                    1 | 2 | 3 => { self.get_address_abs_off_x(false); }
                    _ => {
                        self.poll_interrupts();
                        self.store(self.a, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0x99 => { // STA abs,Y
                match self.operation_cycle {
                    1 | 2 | 3 => { self.get_address_abs_off_y(false); }
                    _ => {
                        self.poll_interrupts();
                        self.store(self.a, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0x81 => { // STA (ind,X)
                match self.operation_cycle {
                    1 | 2 | 3 | 4 => { self.get_address_ind_off_x(); }
                    _ => {
                        self.poll_interrupts();
                        self.store(self.a, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0x91 => { // STA (ind),Y
                match self.operation_cycle {
                    1 | 2 | 3 | 4 => { self.get_address_ind_off_y(false); }
                    _ => {
                        self.poll_interrupts();
                        self.store(self.a, self.address_bus);
                        self.complete_operation();
                    }
                }
            }

            // --- STX ---
            0x86 => { // STX zp
                match self.operation_cycle {
                    1 => { self.get_address_zero_page(); }
                    _ => {
                        self.poll_interrupts();
                        self.store(self.x, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0x96 => { // STX zp,Y
                match self.operation_cycle {
                    1 | 2 => { self.get_address_zp_off_y(); }
                    _ => {
                        self.poll_interrupts();
                        self.store(self.x, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0x8E => { // STX abs
                match self.operation_cycle {
                    1 | 2 => { self.get_address_absolute(); }
                    _ => {
                        self.poll_interrupts();
                        self.store(self.x, self.address_bus);
                        self.complete_operation();
                    }
                }
            }

            // --- STY ---
            0x84 => { // STY zp
                match self.operation_cycle {
                    1 => { self.get_address_zero_page(); }
                    _ => {
                        self.poll_interrupts();
                        self.store(self.y, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0x94 => { // STY zp,X
                match self.operation_cycle {
                    1 | 2 => { self.get_address_zp_off_x(); }
                    _ => {
                        self.poll_interrupts();
                        self.store(self.y, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0x8C => { // STY abs
                match self.operation_cycle {
                    1 | 2 => { self.get_address_absolute(); }
                    _ => {
                        self.poll_interrupts();
                        self.store(self.y, self.address_bus);
                        self.complete_operation();
                    }
                }
            }

            // --- SAX ---
            0x87 => { // SAX zp
                match self.operation_cycle {
                    1 => { self.get_address_zero_page(); }
                    _ => {
                        self.poll_interrupts();
                        self.store(self.a & self.x, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0x97 => { // SAX zp,Y
                match self.operation_cycle {
                    1 | 2 => { self.get_address_zp_off_y(); }
                    _ => {
                        self.poll_interrupts();
                        self.store(self.a & self.x, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0x8F => { // SAX abs
                match self.operation_cycle {
                    1 | 2 => { self.get_address_absolute(); }
                    _ => {
                        self.poll_interrupts();
                        self.store(self.a & self.x, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0x83 => { // SAX (ind,X)
                match self.operation_cycle {
                    1 | 2 | 3 | 4 => { self.get_address_ind_off_x(); }
                    _ => {
                        self.poll_interrupts();
                        self.store(self.a & self.x, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            // --- ORA ---
            0x09 => { self.poll_interrupts(); self.get_immediate(); self.op_ora(self.dl); self.complete_operation(); }
            0x05 => { match self.operation_cycle { 1 => { self.get_address_zero_page(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_ora(v); self.complete_operation(); } } }
            0x15 => { match self.operation_cycle { 1 | 2 => { self.get_address_zp_off_x(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_ora(v); self.complete_operation(); } } }
            0x0D => { match self.operation_cycle { 1 | 2 => { self.get_address_absolute(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_ora(v); self.complete_operation(); } } }
            0x1D => { match self.operation_cycle { 1 | 2 | 3 => { self.get_address_abs_off_x(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_ora(v); self.complete_operation(); } } }
            0x19 => { match self.operation_cycle { 1 | 2 | 3 => { self.get_address_abs_off_y(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_ora(v); self.complete_operation(); } } }
            0x01 => { match self.operation_cycle { 1 | 2 | 3 | 4 => { self.get_address_ind_off_x(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_ora(v); self.complete_operation(); } } }
            0x11 => { match self.operation_cycle { 1 | 2 | 3 | 4 => { self.get_address_ind_off_y(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_ora(v); self.complete_operation(); } } }

            // --- AND ---
            0x29 => { self.poll_interrupts(); self.get_immediate(); self.op_and(self.dl); self.complete_operation(); }
            0x25 => { match self.operation_cycle { 1 => { self.get_address_zero_page(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_and(v); self.complete_operation(); } } }
            0x35 => { match self.operation_cycle { 1 | 2 => { self.get_address_zp_off_x(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_and(v); self.complete_operation(); } } }
            0x2D => { match self.operation_cycle { 1 | 2 => { self.get_address_absolute(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_and(v); self.complete_operation(); } } }
            0x3D => { match self.operation_cycle { 1 | 2 | 3 => { self.get_address_abs_off_x(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_and(v); self.complete_operation(); } } }
            0x39 => { match self.operation_cycle { 1 | 2 | 3 => { self.get_address_abs_off_y(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_and(v); self.complete_operation(); } } }
            0x21 => { match self.operation_cycle { 1 | 2 | 3 | 4 => { self.get_address_ind_off_x(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_and(v); self.complete_operation(); } } }
            0x31 => { match self.operation_cycle { 1 | 2 | 3 | 4 => { self.get_address_ind_off_y(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_and(v); self.complete_operation(); } } }

            // --- EOR ---
            0x49 => { self.poll_interrupts(); self.get_immediate(); self.op_eor(self.dl); self.complete_operation(); }
            0x45 => { match self.operation_cycle { 1 => { self.get_address_zero_page(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_eor(v); self.complete_operation(); } } }
            0x55 => { match self.operation_cycle { 1 | 2 => { self.get_address_zp_off_x(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_eor(v); self.complete_operation(); } } }
            0x4D => { match self.operation_cycle { 1 | 2 => { self.get_address_absolute(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_eor(v); self.complete_operation(); } } }
            0x5D => { match self.operation_cycle { 1 | 2 | 3 => { self.get_address_abs_off_x(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_eor(v); self.complete_operation(); } } }
            0x59 => { match self.operation_cycle { 1 | 2 | 3 => { self.get_address_abs_off_y(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_eor(v); self.complete_operation(); } } }
            0x41 => { match self.operation_cycle { 1 | 2 | 3 | 4 => { self.get_address_ind_off_x(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_eor(v); self.complete_operation(); } } }
            0x51 => { match self.operation_cycle { 1 | 2 | 3 | 4 => { self.get_address_ind_off_y(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_eor(v); self.complete_operation(); } } }

            // --- ADC ---
            0x69 => { self.poll_interrupts(); self.get_immediate(); self.op_adc(self.dl); self.complete_operation(); }
            0x65 => { match self.operation_cycle { 1 => { self.get_address_zero_page(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_adc(v); self.complete_operation(); } } }
            0x75 => { match self.operation_cycle { 1 | 2 => { self.get_address_zp_off_x(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_adc(v); self.complete_operation(); } } }
            0x6D => { match self.operation_cycle { 1 | 2 => { self.get_address_absolute(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_adc(v); self.complete_operation(); } } }
            0x7D => { match self.operation_cycle { 1 | 2 | 3 => { self.get_address_abs_off_x(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_adc(v); self.complete_operation(); } } }
            0x79 => { match self.operation_cycle { 1 | 2 | 3 => { self.get_address_abs_off_y(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_adc(v); self.complete_operation(); } } }
            0x61 => { match self.operation_cycle { 1 | 2 | 3 | 4 => { self.get_address_ind_off_x(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_adc(v); self.complete_operation(); } } }
            0x71 => { match self.operation_cycle { 1 | 2 | 3 | 4 => { self.get_address_ind_off_y(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_adc(v); self.complete_operation(); } } }

            // --- SBC ---
            0xE9 | 0xEB => { self.poll_interrupts(); self.get_immediate(); self.op_sbc(self.dl); self.complete_operation(); }
            0xE5 => { match self.operation_cycle { 1 => { self.get_address_zero_page(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_sbc(v); self.complete_operation(); } } }
            0xF5 => { match self.operation_cycle { 1 | 2 => { self.get_address_zp_off_x(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_sbc(v); self.complete_operation(); } } }
            0xED => { match self.operation_cycle { 1 | 2 => { self.get_address_absolute(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_sbc(v); self.complete_operation(); } } }
            0xFD => { match self.operation_cycle { 1 | 2 | 3 => { self.get_address_abs_off_x(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_sbc(v); self.complete_operation(); } } }
            0xF9 => { match self.operation_cycle { 1 | 2 | 3 => { self.get_address_abs_off_y(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_sbc(v); self.complete_operation(); } } }
            0xE1 => { match self.operation_cycle { 1 | 2 | 3 | 4 => { self.get_address_ind_off_x(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_sbc(v); self.complete_operation(); } } }
            0xF1 => { match self.operation_cycle { 1 | 2 | 3 | 4 => { self.get_address_ind_off_y(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_sbc(v); self.complete_operation(); } } }

            // --- CMP ---
            0xC9 => { self.poll_interrupts(); self.get_immediate(); self.op_cmp(self.dl); self.complete_operation(); }
            0xC5 => { match self.operation_cycle { 1 => { self.get_address_zero_page(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_cmp(v); self.complete_operation(); } } }
            0xD5 => { match self.operation_cycle { 1 | 2 => { self.get_address_zp_off_x(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_cmp(v); self.complete_operation(); } } }
            0xCD => { match self.operation_cycle { 1 | 2 => { self.get_address_absolute(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_cmp(v); self.complete_operation(); } } }
            0xDD => { match self.operation_cycle { 1 | 2 | 3 => { self.get_address_abs_off_x(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_cmp(v); self.complete_operation(); } } }
            0xD9 => { match self.operation_cycle { 1 | 2 | 3 => { self.get_address_abs_off_y(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_cmp(v); self.complete_operation(); } } }
            0xC1 => { match self.operation_cycle { 1 | 2 | 3 | 4 => { self.get_address_ind_off_x(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_cmp(v); self.complete_operation(); } } }
            0xD1 => { match self.operation_cycle { 1 | 2 | 3 | 4 => { self.get_address_ind_off_y(true); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_cmp(v); self.complete_operation(); } } }

            // --- CPX ---
            0xE0 => { self.poll_interrupts(); self.get_immediate(); self.op_cpx(self.dl); self.complete_operation(); }
            0xE4 => { match self.operation_cycle { 1 => { self.get_address_zero_page(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_cpx(v); self.complete_operation(); } } }
            0xEC => { match self.operation_cycle { 1 | 2 => { self.get_address_absolute(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_cpx(v); self.complete_operation(); } } }

            // --- CPY ---
            0xC0 => { self.poll_interrupts(); self.get_immediate(); self.op_cpy(self.dl); self.complete_operation(); }
            0xC4 => { match self.operation_cycle { 1 => { self.get_address_zero_page(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_cpy(v); self.complete_operation(); } } }
            0xCC => { match self.operation_cycle { 1 | 2 => { self.get_address_absolute(); } _ => { self.poll_interrupts(); let v = self.fetch(self.address_bus); self.op_cpy(v); self.complete_operation(); } } }

            // --- BIT ---
            0x24 => { // BIT zp
                match self.operation_cycle {
                    1 => { self.get_address_zero_page(); }
                    _ => {
                        self.poll_interrupts();
                        self.dl = self.fetch(self.address_bus);
                        self.flag_zero = (self.a & self.dl) == 0;
                        self.flag_negative = (self.dl & 0x80) != 0;
                        self.flag_overflow = (self.dl & 0x40) != 0;
                        self.complete_operation();
                    }
                }
            }
            0x2C => { // BIT abs
                match self.operation_cycle {
                    1 | 2 => { self.get_address_absolute(); }
                    _ => {
                        self.poll_interrupts();
                        self.dl = self.fetch(self.address_bus);
                        self.flag_zero = (self.a & self.dl) == 0;
                        self.flag_negative = (self.dl & 0x80) != 0;
                        self.flag_overflow = (self.dl & 0x40) != 0;
                        self.complete_operation();
                    }
                }
            }
            // --- ASL A / LSR A / ROL A / ROR A ---
            0x0A => { self.poll_interrupts(); self.fetch(self.address_bus); self.op_asl_a(); self.complete_operation(); }
            0x4A => { self.poll_interrupts(); self.fetch(self.address_bus); self.op_lsr_a(); self.complete_operation(); }
            0x2A => { self.poll_interrupts(); self.fetch(self.address_bus); self.op_rol_a(); self.complete_operation(); }
            0x6A => { self.poll_interrupts(); self.fetch(self.address_bus); self.op_ror_a(); self.complete_operation(); }

            // --- RMW zp: ASL/LSR/ROL/ROR/INC/DEC ---
            0x06 | 0x46 | 0x26 | 0x66 | 0xE6 | 0xC6 => {
                match self.operation_cycle {
                    1 => { self.get_address_zero_page(); }
                    2 => { self.dl = self.fetch(self.address_bus); self.cpu_read = false; }
                    3 => { self.store(self.dl, self.address_bus); }
                    _ => { self.poll_interrupts(); self.do_rmw_op(); self.complete_operation(); }
                }
            }
            // --- RMW zp,X: ASL/LSR/ROL/ROR/INC/DEC ---
            0x16 | 0x56 | 0x36 | 0x76 | 0xF6 | 0xD6 => {
                match self.operation_cycle {
                    1 | 2 => { self.get_address_zp_off_x(); }
                    3 => { self.dl = self.fetch(self.address_bus); self.cpu_read = false; }
                    4 => { self.store(self.dl, self.address_bus); }
                    _ => { self.poll_interrupts(); self.do_rmw_op(); self.complete_operation(); }
                }
            }
            // --- RMW abs: ASL/LSR/ROL/ROR/INC/DEC ---
            0x0E | 0x4E | 0x2E | 0x6E | 0xEE | 0xCE => {
                match self.operation_cycle {
                    1 | 2 => { self.get_address_absolute(); }
                    3 => { self.dl = self.fetch(self.address_bus); self.cpu_read = false; }
                    4 => { self.store(self.dl, self.address_bus); }
                    _ => { self.poll_interrupts(); self.do_rmw_op(); self.complete_operation(); }
                }
            }
            // --- RMW abs,X: ASL/LSR/ROL/ROR/INC/DEC ---
            0x1E | 0x5E | 0x3E | 0x7E | 0xFE | 0xDE => {
                match self.operation_cycle {
                    1 | 2 | 3 | 4 => { self.get_address_abs_off_x(false); if self.operation_cycle == 4 { self.cpu_read = false; } }
                    5 => { self.store(self.dl, self.address_bus); }
                    _ => { self.poll_interrupts(); self.do_rmw_op(); self.complete_operation(); }
                }
            }

            // --- undocumented RMW: SLO/RLA/SRE/RRA/DCP/ISC ---
            // zp
            0x07 | 0x27 | 0x47 | 0x67 | 0xC7 | 0xE7 => {
                match self.operation_cycle {
                    1 => { self.get_address_zero_page(); }
                    2 => { self.dl = self.fetch(self.address_bus); self.cpu_read = false; }
                    3 => { self.store(self.dl, self.address_bus); }
                    _ => { self.poll_interrupts(); self.do_rmw_op(); self.complete_operation(); }
                }
            }
            // zp,X
            0x17 | 0x37 | 0x57 | 0x77 | 0xD7 | 0xF7 => {
                match self.operation_cycle {
                    1 | 2 => { self.get_address_zp_off_x(); }
                    3 => { self.dl = self.fetch(self.address_bus); self.cpu_read = false; }
                    4 => { self.store(self.dl, self.address_bus); }
                    _ => { self.poll_interrupts(); self.do_rmw_op(); self.complete_operation(); }
                }
            }
            // abs
            0x0F | 0x2F | 0x4F | 0x6F | 0xCF | 0xEF => {
                match self.operation_cycle {
                    1 | 2 => { self.get_address_absolute(); }
                    3 => { self.dl = self.fetch(self.address_bus); self.cpu_read = false; }
                    4 => { self.store(self.dl, self.address_bus); }
                    _ => { self.poll_interrupts(); self.do_rmw_op(); self.complete_operation(); }
                }
            }
            // abs,X
            0x1F | 0x3F | 0x5F | 0x7F | 0xDF | 0xFF => {
                match self.operation_cycle {
                    1 | 2 | 3 | 4 => { self.get_address_abs_off_x(false); if self.operation_cycle == 4 { self.cpu_read = false; } }
                    5 => { self.store(self.dl, self.address_bus); }
                    _ => { self.poll_interrupts(); self.do_rmw_op(); self.complete_operation(); }
                }
            }
            // abs,Y
            0x1B | 0x3B | 0x5B | 0x7B | 0xDB | 0xFB => {
                match self.operation_cycle {
                    1 | 2 | 3 | 4 => { self.get_address_abs_off_y(false); if self.operation_cycle == 4 { self.cpu_read = false; } }
                    5 => { self.store(self.dl, self.address_bus); }
                    _ => { self.poll_interrupts(); self.do_rmw_op(); self.complete_operation(); }
                }
            }
            // (ind,X)
            0x03 | 0x23 | 0x43 | 0x63 | 0xC3 | 0xE3 => {
                match self.operation_cycle {
                    1 | 2 | 3 | 4 => { self.get_address_ind_off_x(); }
                    5 => { self.dl = self.fetch(self.address_bus); self.cpu_read = false; }
                    6 => { self.store(self.dl, self.address_bus); }
                    _ => { self.poll_interrupts(); self.do_rmw_op(); self.complete_operation(); }
                }
            }
            // (ind),Y
            0x13 | 0x33 | 0x53 | 0x73 | 0xD3 | 0xF3 => {
                match self.operation_cycle {
                    1 | 2 | 3 | 4 => { self.get_address_ind_off_y(false); }
                    5 => { self.dl = self.fetch(self.address_bus); self.cpu_read = false; }
                    6 => { self.store(self.dl, self.address_bus); }
                    _ => { self.poll_interrupts(); self.do_rmw_op(); self.complete_operation(); }
                }
            }

            // --- branches ---
            0x10 => { self.do_branch(!self.flag_negative); }  // BPL
            0x30 => { self.do_branch(self.flag_negative); }   // BMI
            0x50 => { self.do_branch(!self.flag_overflow); }  // BVC
            0x70 => { self.do_branch(self.flag_overflow); }   // BVS
            0x90 => { self.do_branch(!self.flag_carry); }     // BCC
            0xB0 => { self.do_branch(self.flag_carry); }      // BCS
            0xD0 => { self.do_branch(!self.flag_zero); }      // BNE
            0xF0 => { self.do_branch(self.flag_zero); }       // BEQ

            // --- stack ops ---
            0x08 => { // PHP
                if self.operation_cycle == 1 { self.fetch(self.address_bus); }
                else { self.poll_interrupts(); let s = self.get_status_byte(true); self.push(s); self.complete_operation(); }
            }
            0x48 => { // PHA
                if self.operation_cycle == 1 { self.fetch(self.address_bus); }
                else { self.poll_interrupts(); self.push(self.a); self.complete_operation(); }
            }
            0x28 => { // PLP
                match self.operation_cycle {
                    1 => { self.fetch(self.address_bus); }
                    2 => { self.fetch(0x100 | self.stack_pointer as u16); self.stack_pointer = self.stack_pointer.wrapping_add(1); }
                    _ => { self.poll_interrupts(); let v = self.fetch(0x100 | self.stack_pointer as u16); self.set_status_byte(v); self.complete_operation(); }
                }
            }
            0x68 => { // PLA
                match self.operation_cycle {
                    1 => { self.fetch(self.address_bus); }
                    2 => { self.fetch(0x100 | self.stack_pointer as u16); self.stack_pointer = self.stack_pointer.wrapping_add(1); }
                    _ => {
                        self.poll_interrupts();
                        self.a = self.fetch(0x100 | self.stack_pointer as u16);
                        self.flag_negative = self.a >= 0x80;
                        self.flag_zero = self.a == 0;
                        self.complete_operation();
                    }
                }
            }
            0x20 => { // JSR
                match self.operation_cycle {
                    1 => {
                        self.address_bus = self.program_counter;
                        self.dl = self.fetch(self.address_bus);
                        self.program_counter = self.program_counter.wrapping_add(1);
                    }
                    2 => {
                        self.address_bus = 0x100 | self.stack_pointer as u16;
                        self.stack_pointer = self.dl;
                        self.cpu_read = false;
                        self.fetch(self.address_bus);
                    }
                    3 => {
                        self.store((self.program_counter >> 8) as u8, self.address_bus);
                        self.address_bus = ((self.address_bus as u8).wrapping_sub(1) as u16) | 0x100;
                    }
                    4 => {
                        self.store(self.program_counter as u8, self.address_bus);
                        self.address_bus = ((self.address_bus as u8).wrapping_sub(1) as u16) | 0x100;
                        self.special_bus = self.address_bus as u8;
                        self.cpu_read = true;
                    }
                    _ => {
                        self.poll_interrupts();
                        self.address_bus = self.program_counter;
                        self.program_counter = ((self.fetch(self.address_bus) as u16) << 8) | self.stack_pointer as u16;
                        self.stack_pointer = self.special_bus;
                        self.complete_operation();
                    }
                }
            }
            0x60 => { // RTS
                match self.operation_cycle {
                    1 => { self.fetch(self.address_bus); }
                    2 => { self.fetch(0x100 | self.stack_pointer as u16); self.stack_pointer = self.stack_pointer.wrapping_add(1); }
                    3 => {
                        self.program_counter = (self.program_counter & 0xFF00) | self.fetch(0x100 | self.stack_pointer as u16) as u16;
                        self.stack_pointer = self.stack_pointer.wrapping_add(1);
                    }
                    4 => {
                        self.program_counter = (self.program_counter & 0x00FF) | ((self.fetch(0x100 | self.stack_pointer as u16) as u16) << 8);
                    }
                    _ => {
                        self.poll_interrupts();
                        self.fetch(self.program_counter);
                        self.program_counter = self.program_counter.wrapping_add(1);
                        self.complete_operation();
                    }
                }
            }
            0x40 => { // RTI
                match self.operation_cycle {
                    1 => { self.fetch(self.address_bus); }
                    2 => { self.fetch(0x100 | self.stack_pointer as u16); self.stack_pointer = self.stack_pointer.wrapping_add(1); }
                    3 => {
                        let v = self.fetch(0x100 | self.stack_pointer as u16);
                        self.set_status_byte(v);
                        self.stack_pointer = self.stack_pointer.wrapping_add(1);
                    }
                    4 => {
                        self.program_counter = (self.program_counter & 0xFF00) | self.fetch(0x100 | self.stack_pointer as u16) as u16;
                        self.stack_pointer = self.stack_pointer.wrapping_add(1);
                    }
                    _ => {
                        self.poll_interrupts();
                        self.program_counter = (self.program_counter & 0x00FF) | ((self.fetch(0x100 | self.stack_pointer as u16) as u16) << 8);
                        self.complete_operation();
                    }
                }
            }

            // --- JMP abs ---
            0x4C => {
                match self.operation_cycle {
                    1 => { self.dl = self.fetch(self.program_counter); self.program_counter = self.program_counter.wrapping_add(1); }
                    _ => {
                        self.poll_interrupts();
                        self.program_counter = self.dl as u16 | ((self.fetch(self.program_counter) as u16) << 8);
                        self.complete_operation();
                    }
                }
            }
            // --- JMP (ind) ---
            0x6C => {
                match self.operation_cycle {
                    1 | 2 => { self.get_address_absolute(); }
                    3 => { self.dl = self.fetch(self.address_bus); }
                    _ => {
                        self.poll_interrupts();
                        // JMP indirect bug: wraps within page
                        let hi_addr = (self.address_bus & 0xFF00) | ((self.address_bus.wrapping_add(1)) & 0x00FF);
                        self.program_counter = self.dl as u16 | ((self.fetch(hi_addr) as u16) << 8);
                        self.complete_operation();
                    }
                }
            }

            // --- implied ---
            0xEA => { self.poll_interrupts(); self.fetch(self.address_bus); self.complete_operation(); } // NOP
            0x18 => { self.poll_interrupts(); self.fetch(self.address_bus); self.flag_carry = false; self.complete_operation(); } // CLC
            0x38 => { self.poll_interrupts(); self.fetch(self.address_bus); self.flag_carry = true; self.complete_operation(); } // SEC
            0x58 => { self.poll_interrupts(); self.fetch(self.address_bus); self.flag_interrupt = false; self.complete_operation(); } // CLI
            0x78 => { self.poll_interrupts(); self.fetch(self.address_bus); self.flag_interrupt = true; self.complete_operation(); } // SEI
            0xB8 => { self.poll_interrupts(); self.fetch(self.address_bus); self.flag_overflow = false; self.complete_operation(); } // CLV
            0xD8 => { self.poll_interrupts(); self.fetch(self.address_bus); self.flag_decimal = false; self.complete_operation(); } // CLD
            0xF8 => { self.poll_interrupts(); self.fetch(self.address_bus); self.flag_decimal = true; self.complete_operation(); } // SED
            0xAA => { // TAX
                self.poll_interrupts(); self.fetch(self.address_bus);
                self.x = self.a; self.flag_negative = self.x >= 0x80; self.flag_zero = self.x == 0;
                self.complete_operation();
            }
            0x8A => { // TXA
                self.poll_interrupts(); self.fetch(self.address_bus);
                self.a = self.x; self.flag_negative = self.a >= 0x80; self.flag_zero = self.a == 0;
                self.complete_operation();
            }
            0xA8 => { // TAY
                self.poll_interrupts(); self.fetch(self.address_bus);
                self.y = self.a; self.flag_negative = self.y >= 0x80; self.flag_zero = self.y == 0;
                self.complete_operation();
            }
            0x98 => { // TYA
                self.poll_interrupts(); self.fetch(self.address_bus);
                self.a = self.y; self.flag_negative = self.a >= 0x80; self.flag_zero = self.a == 0;
                self.complete_operation();
            }
            0xBA => { // TSX
                self.poll_interrupts(); self.fetch(self.address_bus);
                self.x = self.stack_pointer; self.flag_negative = self.x >= 0x80; self.flag_zero = self.x == 0;
                self.complete_operation();
            }
            0x9A => { // TXS
                self.poll_interrupts(); self.fetch(self.address_bus);
                self.stack_pointer = self.x;
                self.complete_operation();
            }
            0xE8 => { // INX
                self.poll_interrupts(); self.fetch(self.address_bus);
                self.x = self.x.wrapping_add(1); self.flag_negative = self.x >= 0x80; self.flag_zero = self.x == 0;
                self.complete_operation();
            }
            0xCA => { // DEX
                self.poll_interrupts(); self.fetch(self.address_bus);
                self.x = self.x.wrapping_sub(1); self.flag_negative = self.x >= 0x80; self.flag_zero = self.x == 0;
                self.complete_operation();
            }
            0xC8 => { // INY
                self.poll_interrupts(); self.fetch(self.address_bus);
                self.y = self.y.wrapping_add(1); self.flag_negative = self.y >= 0x80; self.flag_zero = self.y == 0;
                self.complete_operation();
            }
            0x88 => { // DEY
                self.poll_interrupts(); self.fetch(self.address_bus);
                self.y = self.y.wrapping_sub(1); self.flag_negative = self.y >= 0x80; self.flag_zero = self.y == 0;
                self.complete_operation();
            }

            // --- undocumented immediate ops ---
            0x0B | 0x2B => { // ANC
                self.poll_interrupts(); self.get_immediate();
                self.op_and(self.dl); self.flag_carry = self.flag_negative;
                self.complete_operation();
            }
            0x4B => { // ALR
                self.poll_interrupts(); self.get_immediate();
                self.op_and(self.dl); self.op_lsr_a();
                self.complete_operation();
            }
            0x6B => { // ARR
                self.poll_interrupts(); self.get_immediate();
                self.op_and(self.dl); self.op_ror_a();
                self.flag_carry = (self.a & 0x40) != 0;
                self.flag_overflow = ((self.a >> 6) ^ (self.a >> 5)) & 1 != 0;
                self.complete_operation();
            }
            0x8B => { // XAA
                self.poll_interrupts(); self.get_immediate();
                self.a = (self.a | 0xEE) & self.x & self.dl;
                self.flag_negative = self.a >= 0x80; self.flag_zero = self.a == 0;
                self.complete_operation();
            }
            0xAB => { // LAX imm 
                self.poll_interrupts(); self.get_immediate();
                self.a = (self.a | 0xEE) & self.dl;
                self.x = self.a;
                self.flag_negative = self.a >= 0x80; self.flag_zero = self.a == 0;
                self.complete_operation();
            }
            0xCB => { // AXS
                self.poll_interrupts(); self.get_immediate();
                let tmp = (self.a & self.x) as i32 - self.dl as i32;
                self.flag_carry = tmp >= 0;
                self.x = tmp as u8;
                self.flag_negative = self.x >= 0x80; self.flag_zero = self.x == 0;
                self.complete_operation();
            }

            // --- undocumented NOPs ---
            // DOP
            0x80 | 0x82 | 0xC2 | 0xE2 | 0x89 => {
                self.poll_interrupts(); self.get_immediate(); self.complete_operation();
            }
            // DOP zp
            0x04 | 0x44 | 0x64 => {
                match self.operation_cycle {
                    1 => { self.get_address_zero_page(); }
                    _ => { self.poll_interrupts(); self.fetch(self.address_bus); self.complete_operation(); }
                }
            }
            // DOP zp,X
            0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 => {
                match self.operation_cycle {
                    1 | 2 => { self.get_address_zp_off_x(); }
                    _ => { self.poll_interrupts(); self.fetch(self.address_bus); self.complete_operation(); }
                }
            }
            // TOP abs
            0x0C => {
                match self.operation_cycle {
                    1 | 2 => { self.get_address_absolute(); }
                    _ => { self.poll_interrupts(); self.fetch(self.address_bus); self.complete_operation(); }
                }
            }
            // TOP abs,X
            0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => {
                match self.operation_cycle {
                    1 | 2 | 3 => { self.get_address_abs_off_x(true); }
                    _ => { self.poll_interrupts(); self.fetch(self.address_bus); self.complete_operation(); }
                }
            }
            // implied NOP
            0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => {
                self.poll_interrupts(); self.fetch(self.address_bus); self.complete_operation();
            }

            // --- undocumented store ops with address high byte ---
            0x9C => { // SHY abs,X
                match self.operation_cycle {
                    1 | 2 | 3 => { self.get_address_abs_off_x(false); }
                    _ => {
                        self.poll_interrupts();
                        if (self.temporary_address & 0xFF00) != (self.address_bus & 0xFF00) {
                            self.address_bus = (self.address_bus as u8 as u16) | ((((self.address_bus >> 8) as u8) & self.y) as u16) << 8;
                        }
                        if self.ignore_h { self.h = 0xFF; }
                        self.store(self.y & self.h, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0x9E => { // SHX abs,Y
                match self.operation_cycle {
                    1 | 2 | 3 => { self.get_address_abs_off_y(false); }
                    _ => {
                        self.poll_interrupts();
                        if (self.temporary_address & 0xFF00) != (self.address_bus & 0xFF00) {
                            self.address_bus = (self.address_bus as u8 as u16) | ((((self.address_bus >> 8) as u8) & self.x) as u16) << 8;
                        }
                        if self.ignore_h { self.h = 0xFF; }
                        self.store(self.x & self.h, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0x93 => { // SHA (ind),Y
                match self.operation_cycle {
                    1 | 2 | 3 | 4 => { self.get_address_ind_off_y(false); }
                    _ => {
                        self.poll_interrupts();
                        if (self.temporary_address & 0xFF00) != (self.address_bus & 0xFF00) {
                            self.address_bus = (self.address_bus as u8 as u16) | ((((self.address_bus >> 8) as u8) & self.x) as u16) << 8;
                        }
                        if self.ignore_h { self.h = 0xFF; }
                        self.store(self.a & (self.x | 0xF5) & self.h, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0x9F => { // SHA abs,Y
                match self.operation_cycle {
                    1 | 2 | 3 => { self.get_address_abs_off_y(false); }
                    _ => {
                        self.poll_interrupts();
                        if (self.temporary_address & 0xFF00) != (self.address_bus & 0xFF00) {
                            self.address_bus = (self.address_bus as u8 as u16) | ((((self.address_bus >> 8) as u8) & self.x) as u16) << 8;
                        }
                        if self.ignore_h { self.h = 0xFF; }
                        self.store(self.a & (self.x | 0xF5) & self.h, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0x9B => { // TAS abs,Y
                match self.operation_cycle {
                    1 | 2 | 3 => { self.get_address_abs_off_y(false); }
                    _ => {
                        self.poll_interrupts();
                        if (self.temporary_address & 0xFF00) != (self.address_bus & 0xFF00) {
                            self.address_bus = (self.address_bus as u8 as u16) | ((((self.address_bus >> 8) as u8) & self.x) as u16) << 8;
                        }
                        self.stack_pointer = self.a & self.x;
                        if self.ignore_h { self.h = 0xFF; }
                        self.store(self.a & (self.x | 0xF5) & self.h, self.address_bus);
                        self.complete_operation();
                    }
                }
            }
            0xBB => { // LAS abs,Y
                match self.operation_cycle {
                    1 | 2 | 3 => { self.get_address_abs_off_y(true); }
                    _ => {
                        self.poll_interrupts();
                        let v = self.fetch(self.address_bus) & self.stack_pointer;
                        self.a = v; self.x = v; self.stack_pointer = v;
                        self.flag_negative = v >= 0x80; self.flag_zero = v == 0;
                        self.complete_operation();
                    }
                }
            }

            // --- HLT ---
            0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xB2 | 0xD2 | 0xF2 => {
                self.op_hlt();
            }
        }
    }

    // === BRK / IRQ / NMI / RESET ===
    fn op_brk(&mut self) {
        match self.operation_cycle {
            1 => {
                if !self.do_brk {
                    self.fetch(self.address_bus);
                } else {
                    self.get_immediate();
                }
            }
            2 => {
                if !self.do_reset {
                    self.push((self.program_counter >> 8) as u8);
                } else {
                    self.reset_read_push();
                }
            }
            3 => {
                if !self.do_reset {
                    self.push(self.program_counter as u8);
                } else {
                    self.reset_read_push();
                }
            }
            4 => {
                if !self.do_reset {
                    let status = self.get_status_byte(self.do_brk);
                    self.push(status);
                } else {
                    self.reset_read_push();
                }
                self.poll_interrupts();
            }
            5 => {
                if self.do_nmi {
                    self.program_counter = (self.program_counter & 0xFF00) | self.fetch(0xFFFA) as u16;
                } else if self.do_reset {
                    self.program_counter = (self.program_counter & 0xFF00) | self.fetch(0xFFFC) as u16;
                } else {
                    self.program_counter = (self.program_counter & 0xFF00) | self.fetch(0xFFFE) as u16;
                }
            }
            6 => {
                if self.do_nmi {
                    self.program_counter = (self.program_counter & 0xFF) | ((self.fetch(0xFFFB) as u16) << 8);
                } else if self.do_reset {
                    self.program_counter = (self.program_counter & 0xFF) | ((self.fetch(0xFFFD) as u16) << 8);
                } else {
                    self.program_counter = (self.program_counter & 0xFF) | ((self.fetch(0xFFFF) as u16) << 8);
                }
                self.complete_operation();
                self.do_reset = false;
                self.do_nmi = false;
                self.do_irq = false;
                self.irq_line = false;
                self.do_brk = false;
                self.flag_interrupt = true;
            }
            _ => {}
        }
    }

    // rmw operation handler
    fn do_rmw_op(&mut self) {
        let dl = self.dl;
        let addr = self.address_bus;
        match self.op_code & 0xE0 {
            // column x6/x7: row 0 = ASL/SLO
            0x00 => { if self.op_code & 1 == 0 { self.op_asl(dl, addr); } else { self.op_slo(dl, addr); } }
            // row 1 = ROL/RLA
            0x20 => { if self.op_code & 1 == 0 { self.op_rol(dl, addr); } else { self.op_rla(dl, addr); } }
            // row 2 = LSR/SRE
            0x40 => { if self.op_code & 1 == 0 { self.op_lsr(dl, addr); } else { self.op_sre(dl, addr); } }
            // row 3 = ROR/RRA
            0x60 => { if self.op_code & 1 == 0 { self.op_ror(dl, addr); } else { self.op_rra(dl, addr); } }
            // row 6 = DEC/DCP
            0xC0 => { if self.op_code & 1 == 0 { self.op_dec(addr); } else { self.op_dcp(addr); } }
            // row 7 = INC/ISC
            0xE0 => { if self.op_code & 1 == 0 { self.op_inc(addr); } else { self.op_isc(addr); } }
            _ => {}
        }
    }

    /// branches handler
    fn do_branch(&mut self, condition: bool) {
        match self.operation_cycle {
            1 => {
                self.poll_interrupts();
                self.get_immediate();
                if !condition {
                    self.complete_operation();
                }
            }
            2 => {
                self.fetch(self.address_bus); // dummy read
                let offset = if self.dl >= 0x80 { -(256i32 - self.dl as i32) } else { self.dl as i32 };
                self.temporary_address = (self.program_counter as i32 + offset) as u16;
                self.program_counter = (self.program_counter & 0xFF00) | ((self.program_counter as u8).wrapping_add(self.dl) as u16);
                self.address_bus = self.program_counter;
                if (self.temporary_address & 0xFF00) == (self.program_counter & 0xFF00) {
                    self.complete_operation();
                }
            }
            _ => {
                self.poll_interrupts_cant_disable_irq();
                self.fetch(self.address_bus); // dummy read
                self.program_counter = (self.program_counter & 0xFF) | (self.temporary_address & 0xFF00);
                self.complete_operation();
            }
        }
    }
}

impl Emulator {
    // addressing mode handlers

    pub fn get_address_ind_off_x(&mut self) {
        match self.operation_cycle {
            1 => { self.address_bus = self.fetch(self.program_counter) as u16; self.program_counter = self.program_counter.wrapping_add(1); }
            2 => { self.fetch(self.address_bus); self.address_bus = (self.address_bus as u8).wrapping_add(self.x) as u16; }
            3 => { self.dl = self.fetch(self.address_bus as u8 as u16); }
            4 => { self.address_bus = self.dl as u16 | ((self.fetch((self.address_bus as u8).wrapping_add(1) as u16) as u16) << 8); }
            _ => {}
        }
    }

    pub fn get_address_ind_off_y(&mut self, skip_if_no_page_cross: bool) {
        if skip_if_no_page_cross {
            match self.operation_cycle {
                1 => { self.address_bus = self.fetch(self.program_counter) as u16; self.program_counter = self.program_counter.wrapping_add(1); }
                2 => { self.dl = self.fetch(self.address_bus as u8 as u16); }
                3 => {
                    self.address_bus = self.dl as u16 | ((self.fetch((self.address_bus as u8).wrapping_add(1) as u16) as u16) << 8);
                    self.temporary_address = self.address_bus;
                    self.h = (self.address_bus >> 8) as u8;
                    if (self.temporary_address.wrapping_add(self.y as u16) & 0xFF00) == (self.temporary_address & 0xFF00) {
                        self.operation_cycle += 1;
                    }
                    self.address_bus = (self.address_bus & 0xFF00) | ((self.address_bus.wrapping_add(self.y as u16)) & 0xFF);
                }
                4 => {
                    self.dl = self.fetch(self.address_bus);
                    self.h = (self.address_bus >> 8) as u8;
                    self.h = self.h.wrapping_add(1);
                    self.address_bus = self.address_bus.wrapping_add(0x100);
                }
                _ => {}
            }
        } else {
            match self.operation_cycle {
                1 => { self.address_bus = self.fetch(self.program_counter) as u16; self.program_counter = self.program_counter.wrapping_add(1); }
                2 => { self.dl = self.fetch(self.address_bus as u8 as u16); }
                3 => {
                    self.address_bus = self.dl as u16 | ((self.fetch((self.address_bus as u8).wrapping_add(1) as u16) as u16) << 8);
                    self.temporary_address = self.address_bus;
                    self.address_bus = (self.address_bus & 0xFF00) | ((self.address_bus.wrapping_add(self.y as u16)) & 0xFF);
                }
                4 => {
                    self.dl = self.fetch(self.address_bus);
                    self.h = (self.address_bus >> 8) as u8;
                    self.h = self.h.wrapping_add(1);
                    if (self.temporary_address.wrapping_add(self.y as u16) & 0xFF00) != (self.temporary_address & 0xFF00) {
                        self.address_bus = self.address_bus.wrapping_add(0x100);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn get_address_zp_off_x(&mut self) {
        if self.operation_cycle == 1 {
            self.address_bus = self.fetch(self.program_counter) as u16;
            self.program_counter = self.program_counter.wrapping_add(1);
        } else {
            self.dl = self.fetch(self.address_bus);
            self.address_bus = (self.address_bus as u8).wrapping_add(self.x) as u16;
        }
    }

    pub fn get_address_zp_off_y(&mut self) {
        if self.operation_cycle == 1 {
            self.address_bus = self.fetch(self.program_counter) as u16;
            self.program_counter = self.program_counter.wrapping_add(1);
        } else {
            self.dl = self.fetch(self.address_bus);
            self.address_bus = (self.address_bus as u8).wrapping_add(self.y) as u16;
        }
    }

    pub fn get_address_abs_off_x(&mut self, skip_if_no_page_cross: bool) {
        if skip_if_no_page_cross {
            match self.operation_cycle {
                1 => { self.dl = self.fetch(self.program_counter); self.program_counter = self.program_counter.wrapping_add(1); }
                2 => {
                    self.address_bus = self.dl as u16 | ((self.fetch(self.program_counter) as u16) << 8);
                    self.temporary_address = self.address_bus;
                    self.h = (self.address_bus >> 8) as u8;
                    if (self.temporary_address.wrapping_add(self.x as u16) & 0xFF00) == (self.temporary_address & 0xFF00) {
                        self.operation_cycle += 1;
                        self.fix_high_byte = false;
                    } else {
                        self.fix_high_byte = true;
                    }
                    self.address_bus = (self.address_bus & 0xFF00) | ((self.address_bus.wrapping_add(self.x as u16)) & 0xFF);
                    self.program_counter = self.program_counter.wrapping_add(1);
                }
                3 => {
                    self.dl = self.fetch(self.address_bus);
                    self.h = (self.address_bus >> 8) as u8;
                    self.h = self.h.wrapping_add(1);
                    if self.fix_high_byte { self.address_bus = self.address_bus.wrapping_add(0x100); }
                }
                4 => { self.dl = self.fetch(self.address_bus); }
                _ => {}
            }
        } else {
            match self.operation_cycle {
                1 => { self.dl = self.fetch(self.program_counter); self.program_counter = self.program_counter.wrapping_add(1); }
                2 => {
                    self.address_bus = self.dl as u16 | ((self.fetch(self.program_counter) as u16) << 8);
                    self.temporary_address = self.address_bus;
                    self.address_bus = (self.address_bus & 0xFF00) | ((self.address_bus.wrapping_add(self.x as u16)) & 0xFF);
                    self.program_counter = self.program_counter.wrapping_add(1);
                }
                3 => {
                    self.dl = self.fetch(self.address_bus);
                    self.h = (self.address_bus >> 8) as u8;
                    self.h = self.h.wrapping_add(1);
                    if (self.temporary_address.wrapping_add(self.x as u16) & 0xFF00) != (self.temporary_address & 0xFF00) {
                        self.address_bus = self.address_bus.wrapping_add(0x100);
                    }
                }
                4 => { self.dl = self.fetch(self.address_bus); }
                _ => {}
            }
        }
    }

    pub fn get_address_abs_off_y(&mut self, skip_if_no_page_cross: bool) {
        if skip_if_no_page_cross {
            match self.operation_cycle {
                1 => { self.dl = self.fetch(self.program_counter); self.program_counter = self.program_counter.wrapping_add(1); }
                2 => {
                    self.address_bus = self.dl as u16 | ((self.fetch(self.program_counter) as u16) << 8);
                    self.temporary_address = self.address_bus;
                    self.h = (self.address_bus >> 8) as u8;
                    if (self.temporary_address.wrapping_add(self.y as u16) & 0xFF00) == (self.temporary_address & 0xFF00) {
                        self.operation_cycle += 1;
                        self.fix_high_byte = false;
                    } else {
                        self.fix_high_byte = true;
                    }
                    self.address_bus = (self.address_bus & 0xFF00) | ((self.address_bus.wrapping_add(self.y as u16)) & 0xFF);
                    self.program_counter = self.program_counter.wrapping_add(1);
                }
                3 => {
                    self.dl = self.fetch(self.address_bus);
                    self.h = (self.address_bus >> 8) as u8;
                    self.h = self.h.wrapping_add(1);
                    if self.fix_high_byte { self.address_bus = self.address_bus.wrapping_add(0x100); }
                }
                4 => { self.dl = self.fetch(self.address_bus); }
                _ => {}
            }
        } else {
            match self.operation_cycle {
                1 => { self.dl = self.fetch(self.program_counter); self.program_counter = self.program_counter.wrapping_add(1); }
                2 => {
                    self.address_bus = self.dl as u16 | ((self.fetch(self.program_counter) as u16) << 8);
                    self.temporary_address = self.address_bus;
                    self.address_bus = (self.address_bus & 0xFF00) | ((self.address_bus.wrapping_add(self.y as u16)) & 0xFF);
                    self.program_counter = self.program_counter.wrapping_add(1);
                }
                3 => {
                    self.dl = self.fetch(self.address_bus);
                    self.h = (self.address_bus >> 8) as u8;
                    self.h = self.h.wrapping_add(1);
                    if (self.temporary_address.wrapping_add(self.y as u16) & 0xFF00) != (self.temporary_address & 0xFF00) {
                        self.address_bus = self.address_bus.wrapping_add(0x100);
                    }
                }
                4 => { self.dl = self.fetch(self.address_bus); }
                _ => {}
            }
        }
    }

    pub fn get_immediate(&mut self) {
        self.dl = self.fetch(self.program_counter);
        self.program_counter = self.program_counter.wrapping_add(1);
        self.address_bus = self.program_counter;
    }

    pub fn get_address_absolute(&mut self) {
        if self.operation_cycle == 1 {
            self.dl = self.fetch(self.program_counter);
        } else {
            self.address_bus = self.dl as u16 | ((self.fetch(self.program_counter) as u16) << 8);
        }
        self.program_counter = self.program_counter.wrapping_add(1);
    }

    pub fn get_address_zero_page(&mut self) {
        self.address_bus = self.fetch(self.program_counter) as u16;
        self.program_counter = self.program_counter.wrapping_add(1);
    }

    // arithmetic cpu operation handlers

    pub fn op_ora(&mut self, input: u8) {
        self.a |= input;
        self.flag_negative = self.a >= 0x80;
        self.flag_zero = self.a == 0;
    }

    pub fn op_and(&mut self, input: u8) {
        self.a &= input;
        self.flag_negative = self.a >= 0x80;
        self.flag_zero = self.a == 0;
    }

    pub fn op_eor(&mut self, input: u8) {
        self.a ^= input;
        self.flag_negative = self.a >= 0x80;
        self.flag_zero = self.a == 0;
    }

    pub fn op_adc(&mut self, input: u8) {
        let result = self.a as i32 + input as i32 + if self.flag_carry { 1 } else { 0 };
        self.flag_overflow = (!(self.a ^ input) & (self.a ^ result as u8) & 0x80) != 0;
        self.flag_carry = result > 0xFF;
        self.a = result as u8;
        self.flag_negative = self.a >= 0x80;
        self.flag_zero = self.a == 0;
    }

    pub fn op_sbc(&mut self, input: u8) {
        let result = self.a as i32 - input as i32 - if !self.flag_carry { 1 } else { 0 };
        self.flag_overflow = ((self.a ^ input) & (self.a ^ result as u8) & 0x80) != 0;
        self.flag_carry = result >= 0;
        self.a = result as u8;
        self.flag_negative = self.a >= 0x80;
        self.flag_zero = self.a == 0;
    }

    pub fn op_asl(&mut self, input: u8, address: u16) {
        self.flag_carry = input >= 0x80;
        let result = input << 1;
        self.store(result, address);
        self.flag_negative = result >= 0x80;
        self.flag_zero = result == 0;
    }

    pub fn op_asl_a(&mut self) {
        self.flag_carry = self.a >= 0x80;
        self.a <<= 1;
        self.flag_negative = self.a >= 0x80;
        self.flag_zero = self.a == 0;
    }

    pub fn op_lsr(&mut self, input: u8, address: u16) {
        self.flag_carry = (input & 1) == 1;
        let result = input >> 1;
        self.store(result, address);
        self.flag_negative = result >= 0x80;
        self.flag_zero = result == 0;
    }

    pub fn op_lsr_a(&mut self) {
        self.flag_carry = (self.a & 1) == 1;
        self.a >>= 1;
        self.flag_negative = self.a >= 0x80;
        self.flag_zero = self.a == 0;
    }

    pub fn op_rol(&mut self, input: u8, address: u16) {
        let future_carry = input >= 0x80;
        let mut result = input << 1;
        if self.flag_carry { result |= 1; }
        self.store(result, address);
        self.flag_carry = future_carry;
        self.flag_negative = result >= 0x80;
        self.flag_zero = result == 0;
    }

    pub fn op_rol_a(&mut self) {
        let future_carry = self.a >= 0x80;
        self.a <<= 1;
        if self.flag_carry { self.a |= 1; }
        self.flag_carry = future_carry;
        self.flag_negative = self.a >= 0x80;
        self.flag_zero = self.a == 0;
    }

    pub fn op_ror(&mut self, input: u8, address: u16) {
        let future_carry = (input & 1) == 1;
        let mut result = input >> 1;
        if self.flag_carry { result |= 0x80; }
        self.store(result, address);
        self.flag_carry = future_carry;
        self.flag_negative = result >= 0x80;
        self.flag_zero = result == 0;
    }

    pub fn op_ror_a(&mut self) {
        let future_carry = (self.a & 1) == 1;
        self.a >>= 1;
        if self.flag_carry { self.a |= 0x80; }
        self.flag_carry = future_carry;
        self.flag_negative = self.a >= 0x80;
        self.flag_zero = self.a == 0;
    }

    pub fn op_slo(&mut self, input: u8, address: u16) {
        self.op_asl(input, address);
        self.op_ora(self.data_bus);
    }

    pub fn op_rla(&mut self, input: u8, address: u16) {
        self.op_rol(input, address);
        self.op_and(self.data_bus);
    }

    pub fn op_sre(&mut self, input: u8, address: u16) {
        self.op_lsr(input, address);
        self.op_eor(self.data_bus);
    }

    pub fn op_rra(&mut self, input: u8, address: u16) {
        self.op_ror(input, address);
        self.op_adc(self.data_bus);
    }

    pub fn op_cpx(&mut self, input: u8) {
        self.flag_zero = self.x == input;
        self.flag_carry = self.x >= input;
        self.flag_negative = self.x.wrapping_sub(input) >= 0x80;
    }

    pub fn op_cpy(&mut self, input: u8) {
        self.flag_zero = self.y == input;
        self.flag_carry = self.y >= input;
        self.flag_negative = self.y.wrapping_sub(input) >= 0x80;
    }

    pub fn op_inc(&mut self, address: u16) {
        self.dl = self.dl.wrapping_add(1);
        self.flag_zero = self.dl == 0;
        self.flag_negative = self.dl >= 0x80;
        self.store(self.dl, address);
    }

    pub fn op_dec(&mut self, address: u16) {
        self.dl = self.dl.wrapping_sub(1);
        self.flag_zero = self.dl == 0;
        self.flag_negative = self.dl >= 0x80;
        self.store(self.dl, address);
    }

    pub fn op_dcp(&mut self, address: u16) {
        self.op_dec(address);
        self.op_cmp(self.dl);
    }

    pub fn op_isc(&mut self, address: u16) {
        self.op_inc(address);
        self.op_sbc(self.dl);
    }

    pub fn get_status_byte(&self, b_flag: bool) -> u8 {
        let mut s = 0x20u8;
        if self.flag_carry { s |= 0x01; }
        if self.flag_zero { s |= 0x02; }
        if self.flag_interrupt { s |= 0x04; }
        if self.flag_decimal { s |= 0x08; }
        if b_flag { s |= 0x10; }
        if self.flag_overflow { s |= 0x40; }
        if self.flag_negative { s |= 0x80; }
        s
    }

    pub fn set_status_byte(&mut self, s: u8) {
        self.flag_carry = (s & 0x01) != 0;
        self.flag_zero = (s & 0x02) != 0;
        self.flag_interrupt = (s & 0x04) != 0;
        self.flag_decimal = (s & 0x08) != 0;
        self.flag_overflow = (s & 0x40) != 0;
        self.flag_negative = (s & 0x80) != 0;
    }

    pub fn op_hlt(&mut self) {
        match self.operation_cycle {
            1 => { self.dl = self.fetch(self.address_bus); }
            2 => { self.address_bus = 0xFFFF; self.fetch(self.address_bus); }
            3 | 4 => { self.address_bus = 0xFFFE; self.fetch(self.address_bus); }
            5 => { self.address_bus = 0xFFFF; self.fetch(self.address_bus); }
            6 => { self.address_bus = 0xFFFF; self.fetch(self.address_bus); self.operation_cycle = 5; }
            _ => {}
        }
    }

    pub fn op_cmp(&mut self, input: u8) {
        self.flag_zero = self.a == input;
        self.flag_carry = self.a >= input;
        self.flag_negative = self.a.wrapping_sub(input) >= 0x80;
    }

    // interrupt handling and cpu helpers

    pub fn poll_interrupts(&mut self) {
        self.nmi_previous_pins_signal = self.nmi_pins_signal;
        self.nmi_pins_signal = self.nmi_line;
        if self.nmi_pins_signal && !self.nmi_previous_pins_signal {
            self.do_nmi = true;
        }
        self.do_irq = self.irq_line && !self.flag_interrupt;
    }

    pub fn poll_interrupts_cant_disable_irq(&mut self) {
        self.nmi_previous_pins_signal = self.nmi_pins_signal;
        self.nmi_pins_signal = self.nmi_line;
        if self.nmi_pins_signal && !self.nmi_previous_pins_signal {
            self.do_nmi = true;
        }
        if !self.do_irq {
            self.do_irq = self.irq_line && !self.flag_interrupt;
        }
    }

    pub fn complete_operation(&mut self) {
        self.operation_cycle = 0xFF;
        self.address_bus = self.program_counter;
        self.cpu_read = true;
        self.ignore_h = false;
    }

    pub fn push(&mut self, val: u8) {
        self.store(val, 0x100 | self.stack_pointer as u16);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
    }

    pub fn reset_read_push(&mut self) {
        self.fetch(0x100 | self.stack_pointer as u16);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
    }

    pub fn start_dmc_sample(&mut self) {
        self.apu_dmc_address_counter = self.apu_dmc_sample_address;
        self.apu_dmc_bytes_remaining = self.apu_dmc_sample_length;
    }
}
