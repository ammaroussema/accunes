use crate::emulator::Emulator;
use crate::{draw_rect, draw_text, point_in_rect, UiColors, MenuState};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AddrMode {
    Imp,
    Acc,
    Imm,
    Zp,
    ZpX,
    ZpY,
    Rel,
    Abs,
    AbsX,
    AbsY,
    Ind,
    IndX,
    IndY,
}

pub fn op_info(op: u8) -> (&'static str, AddrMode, u8) {
    match op {
        0x00 => ("BRK", AddrMode::Imp, 1),
        0x01 => ("ORA", AddrMode::IndX, 2),
        0x02 => ("JAM", AddrMode::Imp, 1),
        0x03 => ("SLO", AddrMode::IndX, 2),
        0x04 => ("NOP", AddrMode::Zp, 2),
        0x05 => ("ORA", AddrMode::Zp, 2),
        0x06 => ("ASL", AddrMode::Zp, 2),
        0x07 => ("SLO", AddrMode::Zp, 2),
        0x08 => ("PHP", AddrMode::Imp, 1),
        0x09 => ("ORA", AddrMode::Imm, 2),
        0x0A => ("ASL", AddrMode::Acc, 1),
        0x0B => ("ANC", AddrMode::Imm, 2),
        0x0C => ("NOP", AddrMode::Abs, 3),
        0x0D => ("ORA", AddrMode::Abs, 3),
        0x0E => ("ASL", AddrMode::Abs, 3),
        0x0F => ("SLO", AddrMode::Abs, 3),

        0x10 => ("BPL", AddrMode::Rel, 2),
        0x11 => ("ORA", AddrMode::IndY, 2),
        0x12 => ("JAM", AddrMode::Imp, 1),
        0x13 => ("SLO", AddrMode::IndY, 2),
        0x14 => ("NOP", AddrMode::ZpX, 2),
        0x15 => ("ORA", AddrMode::ZpX, 2),
        0x16 => ("ASL", AddrMode::ZpX, 2),
        0x17 => ("SLO", AddrMode::ZpX, 2),
        0x18 => ("CLC", AddrMode::Imp, 1),
        0x19 => ("ORA", AddrMode::AbsY, 3),
        0x1A => ("NOP", AddrMode::Imp, 1),
        0x1B => ("SLO", AddrMode::AbsY, 3),
        0x1C => ("NOP", AddrMode::AbsX, 3),
        0x1D => ("ORA", AddrMode::AbsX, 3),
        0x1E => ("ASL", AddrMode::AbsX, 3),
        0x1F => ("SLO", AddrMode::AbsX, 3),

        0x20 => ("JSR", AddrMode::Abs, 3),
        0x21 => ("AND", AddrMode::IndX, 2),
        0x22 => ("JAM", AddrMode::Imp, 1),
        0x23 => ("RLA", AddrMode::IndX, 2),
        0x24 => ("BIT", AddrMode::Zp, 2),
        0x25 => ("AND", AddrMode::Zp, 2),
        0x26 => ("ROL", AddrMode::Zp, 2),
        0x27 => ("RLA", AddrMode::Zp, 2),
        0x28 => ("PLP", AddrMode::Imp, 1),
        0x29 => ("AND", AddrMode::Imm, 2),
        0x2A => ("ROL", AddrMode::Acc, 1),
        0x2B => ("ANC", AddrMode::Imm, 2),
        0x2C => ("BIT", AddrMode::Abs, 3),
        0x2D => ("AND", AddrMode::Abs, 3),
        0x2E => ("ROL", AddrMode::Abs, 3),
        0x2F => ("RLA", AddrMode::Abs, 3),

        0x30 => ("BMI", AddrMode::Rel, 2),
        0x31 => ("AND", AddrMode::IndY, 2),
        0x32 => ("JAM", AddrMode::Imp, 1),
        0x33 => ("RLA", AddrMode::IndY, 2),
        0x34 => ("NOP", AddrMode::ZpX, 2),
        0x35 => ("AND", AddrMode::ZpX, 2),
        0x36 => ("ROL", AddrMode::ZpX, 2),
        0x37 => ("RLA", AddrMode::ZpX, 2),
        0x38 => ("SEC", AddrMode::Imp, 1),
        0x39 => ("AND", AddrMode::AbsY, 3),
        0x3A => ("NOP", AddrMode::Imp, 1),
        0x3B => ("RLA", AddrMode::AbsY, 3),
        0x3C => ("NOP", AddrMode::AbsX, 3),
        0x3D => ("AND", AddrMode::AbsX, 3),
        0x3E => ("ROL", AddrMode::AbsX, 3),
        0x3F => ("RLA", AddrMode::AbsX, 3),

        0x40 => ("RTI", AddrMode::Imp, 1),
        0x41 => ("EOR", AddrMode::IndX, 2),
        0x42 => ("JAM", AddrMode::Imp, 1),
        0x43 => ("SRE", AddrMode::IndX, 2),
        0x44 => ("NOP", AddrMode::Zp, 2),
        0x45 => ("EOR", AddrMode::Zp, 2),
        0x46 => ("LSR", AddrMode::Zp, 2),
        0x47 => ("SRE", AddrMode::Zp, 2),
        0x48 => ("PHA", AddrMode::Imp, 1),
        0x49 => ("EOR", AddrMode::Imm, 2),
        0x4A => ("LSR", AddrMode::Acc, 1),
        0x4B => ("ALR", AddrMode::Imm, 2),
        0x4C => ("JMP", AddrMode::Abs, 3),
        0x4D => ("EOR", AddrMode::Abs, 3),
        0x4E => ("LSR", AddrMode::Abs, 3),
        0x4F => ("SRE", AddrMode::Abs, 3),

        0x50 => ("BVC", AddrMode::Rel, 2),
        0x51 => ("EOR", AddrMode::IndY, 2),
        0x52 => ("JAM", AddrMode::Imp, 1),
        0x53 => ("SRE", AddrMode::IndY, 2),
        0x54 => ("NOP", AddrMode::ZpX, 2),
        0x55 => ("EOR", AddrMode::ZpX, 2),
        0x56 => ("LSR", AddrMode::ZpX, 2),
        0x57 => ("SRE", AddrMode::ZpX, 2),
        0x58 => ("CLI", AddrMode::Imp, 1),
        0x59 => ("EOR", AddrMode::AbsY, 3),
        0x5A => ("NOP", AddrMode::Imp, 1),
        0x5B => ("SRE", AddrMode::AbsY, 3),
        0x5C => ("NOP", AddrMode::AbsX, 3),
        0x5D => ("EOR", AddrMode::AbsX, 3),
        0x5E => ("LSR", AddrMode::AbsX, 3),
        0x5F => ("SRE", AddrMode::AbsX, 3),

        0x60 => ("RTS", AddrMode::Imp, 1),
        0x61 => ("ADC", AddrMode::IndX, 2),
        0x62 => ("JAM", AddrMode::Imp, 1),
        0x63 => ("RRA", AddrMode::IndX, 2),
        0x64 => ("NOP", AddrMode::Zp, 2),
        0x65 => ("ADC", AddrMode::Zp, 2),
        0x66 => ("ROR", AddrMode::Zp, 2),
        0x67 => ("RRA", AddrMode::Zp, 2),
        0x68 => ("PLA", AddrMode::Imp, 1),
        0x69 => ("ADC", AddrMode::Imm, 2),
        0x6A => ("ROR", AddrMode::Acc, 1),
        0x6B => ("ARR", AddrMode::Imm, 2),
        0x6C => ("JMP", AddrMode::Ind, 3),
        0x6D => ("ADC", AddrMode::Abs, 3),
        0x6E => ("ROR", AddrMode::Abs, 3),
        0x6F => ("RRA", AddrMode::Abs, 3),

        0x70 => ("BVS", AddrMode::Rel, 2),
        0x71 => ("ADC", AddrMode::IndY, 2),
        0x72 => ("JAM", AddrMode::Imp, 1),
        0x73 => ("RRA", AddrMode::IndY, 2),
        0x74 => ("NOP", AddrMode::ZpX, 2),
        0x75 => ("ADC", AddrMode::ZpX, 2),
        0x76 => ("ROR", AddrMode::ZpX, 2),
        0x77 => ("RRA", AddrMode::ZpX, 2),
        0x78 => ("SEI", AddrMode::Imp, 1),
        0x79 => ("ADC", AddrMode::AbsY, 3),
        0x7A => ("NOP", AddrMode::Imp, 1),
        0x7B => ("RRA", AddrMode::AbsY, 3),
        0x7C => ("NOP", AddrMode::AbsX, 3),
        0x7D => ("ADC", AddrMode::AbsX, 3),
        0x7E => ("ROR", AddrMode::AbsX, 3),
        0x7F => ("RRA", AddrMode::AbsX, 3),

        0x80 => ("NOP", AddrMode::Imm, 2),
        0x81 => ("STA", AddrMode::IndX, 2),
        0x82 => ("NOP", AddrMode::Imm, 2),
        0x83 => ("SAX", AddrMode::IndX, 2),
        0x84 => ("STY", AddrMode::Zp, 2),
        0x85 => ("STA", AddrMode::Zp, 2),
        0x86 => ("STX", AddrMode::Zp, 2),
        0x87 => ("SAX", AddrMode::Zp, 2),
        0x88 => ("DEY", AddrMode::Imp, 1),
        0x89 => ("NOP", AddrMode::Imm, 2),
        0x8A => ("TXA", AddrMode::Imp, 1),
        0x8B => ("XAA", AddrMode::Imm, 2),
        0x8C => ("STY", AddrMode::Abs, 3),
        0x8D => ("STA", AddrMode::Abs, 3),
        0x8E => ("STX", AddrMode::Abs, 3),
        0x8F => ("SAX", AddrMode::Abs, 3),

        0x90 => ("BCC", AddrMode::Rel, 2),
        0x91 => ("STA", AddrMode::IndY, 2),
        0x92 => ("JAM", AddrMode::Imp, 1),
        0x93 => ("AHX", AddrMode::IndY, 2),
        0x94 => ("STY", AddrMode::ZpX, 2),
        0x95 => ("STA", AddrMode::ZpX, 2),
        0x96 => ("STX", AddrMode::ZpY, 2),
        0x97 => ("SAX", AddrMode::ZpY, 2),
        0x98 => ("TYA", AddrMode::Imp, 1),
        0x99 => ("STA", AddrMode::AbsY, 3),
        0x9A => ("TXS", AddrMode::Imp, 1),
        0x9B => ("TAS", AddrMode::AbsY, 3),
        0x9C => ("SHY", AddrMode::AbsX, 3),
        0x9D => ("STA", AddrMode::AbsX, 3),
        0x9E => ("SHX", AddrMode::AbsY, 3),
        0x9F => ("AHX", AddrMode::AbsY, 3),

        0xA0 => ("LDY", AddrMode::Imm, 2),
        0xA1 => ("LDA", AddrMode::IndX, 2),
        0xA2 => ("LDX", AddrMode::Imm, 2),
        0xA3 => ("LAX", AddrMode::IndX, 2),
        0xA4 => ("LDY", AddrMode::Zp, 2),
        0xA5 => ("LDA", AddrMode::Zp, 2),
        0xA6 => ("LDX", AddrMode::Zp, 2),
        0xA7 => ("LAX", AddrMode::Zp, 2),
        0xA8 => ("TAY", AddrMode::Imp, 1),
        0xA9 => ("LDA", AddrMode::Imm, 2),
        0xAA => ("TAX", AddrMode::Imp, 1),
        0xAB => ("LAX", AddrMode::Imm, 2),
        0xAC => ("LDY", AddrMode::Abs, 3),
        0xAD => ("LDA", AddrMode::Abs, 3),
        0xAE => ("LDX", AddrMode::Abs, 3),
        0xAF => ("LAX", AddrMode::Abs, 3),

        0xB0 => ("BCS", AddrMode::Rel, 2),
        0xB1 => ("LDA", AddrMode::IndY, 2),
        0xB2 => ("JAM", AddrMode::Imp, 1),
        0xB3 => ("LAX", AddrMode::IndY, 2),
        0xB4 => ("LDY", AddrMode::ZpX, 2),
        0xB5 => ("LDA", AddrMode::ZpX, 2),
        0xB6 => ("LDX", AddrMode::ZpY, 2),
        0xB7 => ("LAX", AddrMode::ZpY, 2),
        0xB8 => ("CLV", AddrMode::Imp, 1),
        0xB9 => ("LDA", AddrMode::AbsY, 3),
        0xBA => ("TSX", AddrMode::Imp, 1),
        0xBB => ("LAS", AddrMode::AbsY, 3),
        0xBC => ("LDY", AddrMode::AbsX, 3),
        0xBD => ("LDA", AddrMode::AbsX, 3),
        0xBE => ("LDX", AddrMode::AbsY, 3),
        0xBF => ("LAX", AddrMode::AbsY, 3),

        0xC0 => ("CPY", AddrMode::Imm, 2),
        0xC1 => ("CMP", AddrMode::IndX, 2),
        0xC2 => ("NOP", AddrMode::Imm, 2),
        0xC3 => ("DCP", AddrMode::IndX, 2),
        0xC4 => ("CPY", AddrMode::Zp, 2),
        0xC5 => ("CMP", AddrMode::Zp, 2),
        0xC6 => ("DEC", AddrMode::Zp, 2),
        0xC7 => ("DCP", AddrMode::Zp, 2),
        0xC8 => ("INY", AddrMode::Imp, 1),
        0xC9 => ("CMP", AddrMode::Imm, 2),
        0xCA => ("DEX", AddrMode::Imp, 1),
        0xCB => ("AXS", AddrMode::Imm, 2),
        0xCC => ("CPY", AddrMode::Abs, 3),
        0xCD => ("CMP", AddrMode::Abs, 3),
        0xCE => ("DEC", AddrMode::Abs, 3),
        0xCF => ("DCP", AddrMode::Abs, 3),

        0xD0 => ("BNE", AddrMode::Rel, 2),
        0xD1 => ("CMP", AddrMode::IndY, 2),
        0xD2 => ("JAM", AddrMode::Imp, 1),
        0xD3 => ("DCP", AddrMode::IndY, 2),
        0xD4 => ("NOP", AddrMode::ZpX, 2),
        0xD5 => ("CMP", AddrMode::ZpX, 2),
        0xD6 => ("DEC", AddrMode::ZpX, 2),
        0xD7 => ("DCP", AddrMode::ZpX, 2),
        0xD8 => ("CLD", AddrMode::Imp, 1),
        0xD9 => ("CMP", AddrMode::AbsY, 3),
        0xDA => ("NOP", AddrMode::Imp, 1),
        0xDB => ("DCP", AddrMode::AbsY, 3),
        0xDC => ("NOP", AddrMode::AbsX, 3),
        0xDD => ("CMP", AddrMode::AbsX, 3),
        0xDE => ("DEC", AddrMode::AbsX, 3),
        0xDF => ("DCP", AddrMode::AbsX, 3),

        0xE0 => ("CPX", AddrMode::Imm, 2),
        0xE1 => ("SBC", AddrMode::IndX, 2),
        0xE2 => ("NOP", AddrMode::Imm, 2),
        0xE3 => ("ISC", AddrMode::IndX, 2),
        0xE4 => ("CPX", AddrMode::Zp, 2),
        0xE5 => ("SBC", AddrMode::Zp, 2),
        0xE6 => ("INC", AddrMode::Zp, 2),
        0xE7 => ("ISC", AddrMode::Zp, 2),
        0xE8 => ("INX", AddrMode::Imp, 1),
        0xE9 => ("SBC", AddrMode::Imm, 2),
        0xEA => ("NOP", AddrMode::Imp, 1),
        0xEB => ("SBC", AddrMode::Imm, 2),
        0xEC => ("CPX", AddrMode::Abs, 3),
        0xED => ("SBC", AddrMode::Abs, 3),
        0xEE => ("INC", AddrMode::Abs, 3),
        0xEF => ("ISC", AddrMode::Abs, 3),

        0xF0 => ("BEQ", AddrMode::Rel, 2),
        0xF1 => ("SBC", AddrMode::IndY, 2),
        0xF2 => ("JAM", AddrMode::Imp, 1),
        0xF3 => ("ISC", AddrMode::IndY, 2),
        0xF4 => ("NOP", AddrMode::ZpX, 2),
        0xF5 => ("SBC", AddrMode::ZpX, 2),
        0xF6 => ("INC", AddrMode::ZpX, 2),
        0xF7 => ("ISC", AddrMode::ZpX, 2),
        0xF8 => ("SED", AddrMode::Imp, 1),
        0xF9 => ("SBC", AddrMode::AbsY, 3),
        0xFA => ("NOP", AddrMode::Imp, 1),
        0xFB => ("ISC", AddrMode::AbsY, 3),
        0xFC => ("NOP", AddrMode::AbsX, 3),
        0xFD => ("SBC", AddrMode::AbsX, 3),
        0xFE => ("INC", AddrMode::AbsX, 3),
        0xFF => ("ISC", AddrMode::AbsX, 3),
    }
}

pub fn instruction_len(op: u8) -> u8 {
    op_info(op).2
}

pub fn instruction_down(emu: &mut Emulator, from: u16) -> u16 {
    let op = emu.debug_peek_mem(from);
    let len = instruction_len(op).max(1);
    from.wrapping_add(len as u16)
}

pub fn instruction_up(emu: &mut Emulator, from: u16) -> u16 {
    let limit = 16u16.min(from);
    let mut i = limit;
    while i > 0 {
        let mut j = i;
        while j > 0 {
            let b = emu.debug_peek_mem(from.wrapping_sub(j));
            let len = instruction_len(b);
            if len == 0 || len > j as u8 {
                break;
            }
            if len == j as u8 {
                return from.wrapping_sub(j);
            }
            j -= len as u16;
        }
        i -= 1;
    }
    from.saturating_sub(1)
}

pub struct Instruction {
    pub addr: u16,
    pub bytes: Vec<u8>,
    pub mnemonic: &'static str,
    pub operand: String,
    pub len: u8,
}

pub fn disassemble_instruction(emu: &mut Emulator, addr: u16) -> Instruction {
    let op = emu.debug_peek_mem(addr);
    let (mnemonic, mode, len) = op_info(op);
    let mut bytes = vec![op];
    let b1 = if len > 1 {
        let b = emu.debug_peek_mem(addr.wrapping_add(1));
        bytes.push(b);
        b
    } else {
        0
    };
    let b2 = if len > 2 {
        let b = emu.debug_peek_mem(addr.wrapping_add(2));
        bytes.push(b);
        b
    } else {
        0
    };
    let operand = match mode {
        AddrMode::Imp => String::new(),
        AddrMode::Acc => "A".to_string(),
        AddrMode::Imm => format!("#${:02X}", b1),
        AddrMode::Zp => format!("${:02X}", b1),
        AddrMode::ZpX => format!("${:02X},X", b1),
        AddrMode::ZpY => format!("${:02X},Y", b1),
        AddrMode::Rel => {
            let target = addr.wrapping_add(2).wrapping_add((b1 as i8) as i16 as u16);
            format!("${:04X}", target)
        }
        AddrMode::Abs => {
            let target = ((b2 as u16) << 8) | (b1 as u16);
            format!("${:04X}", target)
        }
        AddrMode::AbsX => {
            let target = ((b2 as u16) << 8) | (b1 as u16);
            format!("${:04X},X", target)
        }
        AddrMode::AbsY => {
            let target = ((b2 as u16) << 8) | (b1 as u16);
            format!("${:04X},Y", target)
        }
        AddrMode::Ind => {
            let target = ((b2 as u16) << 8) | (b1 as u16);
            format!("(${:04X})", target)
        }
        AddrMode::IndX => format!("(${:02X},X)", b1),
        AddrMode::IndY => format!("(${:02X}),Y", b1),
    };
    Instruction {
        addr,
        bytes,
        mnemonic,
        operand,
        len,
    }
}

pub fn step_cpu_instruction(emu: &mut Emulator) {
    if emu.is_pal() {
        loop {
            emu.emulator_core_pal();
            if emu.operation_cycle != 0 {
                break;
            }
        }
        while emu.operation_cycle != 0 {
            emu.emulator_core_pal();
        }
    } else if emu.is_dendy() {
        loop {
            emu.emulator_core_dendy();
            if emu.operation_cycle != 0 {
                break;
            }
        }
        while emu.operation_cycle != 0 {
            emu.emulator_core_dendy();
        }
    } else {
        loop {
            emu.emulator_core_ntsc();
            if emu.operation_cycle != 0 {
                break;
            }
        }
        while emu.operation_cycle != 0 {
            emu.emulator_core_ntsc();
        }
    }
}

pub fn step_over(emu: &mut Emulator, paused: &AtomicBool) {
    let op = emu.debug_peek_mem(emu.program_counter);
    if op == 0x20 {
        emu.step_over_target = Some(emu.program_counter.wrapping_add(3));
        paused.store(false, Ordering::Relaxed);
    } else {
        step_cpu_instruction(emu);
    }
}

pub fn step_out(emu: &mut Emulator, paused: &AtomicBool) {
    emu.step_out = true;
    paused.store(false, Ordering::Relaxed);
}

pub fn parse_hex(s: &str) -> Option<u16> {
    let clean = s.trim().trim_start_matches('$').trim_start_matches("0x").trim_start_matches("0X");
    u16::from_str_radix(clean, 16).ok()
}

#[allow(dead_code)]
struct DebuggerLayout {
    win_x: usize,
    win_y: usize,
    win_w: usize,
    win_h: usize,
    title_h: usize,
    close_x: usize,
    close_y: usize,
    close_w: usize,
    close_h: usize,
    btn_y: usize,
    btn_h: usize,
    run_x: usize,
    run_w: usize,
    step_x: usize,
    step_w: usize,
    over_x: usize,
    over_w: usize,
    out_x: usize,
    out_w: usize,
    seek_x: usize,
    seek_w: usize,
    panel_y: usize,
    panel_h: usize,
    disasm_x: usize,
    disasm_w: usize,
    row_h: usize,
    num_rows: usize,
    nav_y: usize,
    nav_h: usize,
    up_x: usize,
    up_y: usize,
    up_w: usize,
    up_h: usize,
    down_x: usize,
    go_label_x: usize,
    seek_in_x: usize,
    seek_in_w: usize,
    go_btn_x: usize,
    go_btn_w: usize,
    right_x: usize,
    right_w: usize,
    reg_box_h: usize,
    reg_title_h: usize,
    r_text_x: usize,
    r_gap: usize,
    flag_w: usize,
    flag_h: usize,
    flag_gap: usize,
    bp_box_y: usize,
    bp_box_h: usize,
    bp_title_h: usize,
    bp_row_h: usize,
    bp_max_rows: usize,
    in_bp_x: usize,
    in_bp_y: usize,
    in_bp_w: usize,
    in_bp_h: usize,
    add_bp_x: usize,
    add_bp_w: usize,
    del_bp_x: usize,
    del_bp_w: usize,
    clr_bp_x: usize,
    clr_bp_w: usize,
}

fn compute_debugger_layout(width: usize, height: usize, scale: f32) -> DebuggerLayout {
    let sc = scale;
    let margin_x = (8.0 * sc).round() as usize;
    let win_w = width.saturating_sub(margin_x * 2).min((700.0 * sc).round() as usize);
    let win_h = ((height as f32 - 36.0 * sc).min(470.0 * sc)).round() as usize;
    let win_x = (width.saturating_sub(win_w)) / 2;
    let win_y = (height.saturating_sub(win_h)) / 2;

    let title_h = (28.0 * sc).round() as usize;
    let close_w = (20.0 * sc).round() as usize;
    let close_h = (20.0 * sc).round() as usize;
    let close_x = win_x + win_w - close_w - (6.0 * sc).round() as usize;
    let close_y = win_y + (4.0 * sc).round() as usize;

    let btn_y = win_y + title_h + (6.0 * sc).round() as usize;
    let btn_h = (22.0 * sc).round() as usize;
    let top_gap = (4.0 * sc).round() as usize;
    let mut bx = win_x + (8.0 * sc).round() as usize;

    let run_x = bx;
    let run_w = (52.0 * sc).round() as usize;
    bx += run_w + top_gap;

    let step_x = bx;
    let step_w = (78.0 * sc).round() as usize;
    bx += step_w + top_gap;

    let over_x = bx;
    let over_w = (78.0 * sc).round() as usize;
    bx += over_w + top_gap;

    let out_x = bx;
    let out_w = (72.0 * sc).round() as usize;
    bx += out_w + top_gap;

    let seek_x = bx;
    let seek_w = (68.0 * sc).round() as usize;

    let panel_y = btn_y + btn_h + (6.0 * sc).round() as usize;
    let panel_h = win_y + win_h - panel_y - (8.0 * sc).round() as usize;

    let pad_x = (8.0 * sc).round() as usize;
    let mid_gap = (8.0 * sc).round() as usize;
    let avail_w = win_w.saturating_sub(pad_x * 2 + mid_gap);

    let right_w = ((192.0 * sc).round() as usize).min(avail_w / 2);
    let disasm_w = avail_w.saturating_sub(right_w);
    let disasm_x = win_x + pad_x;
    let right_x = disasm_x + disasm_w + mid_gap;

    let nav_h = (24.0 * sc).round() as usize;
    let rows_h = panel_h.saturating_sub(nav_h + 4);
    let row_h = (18.0 * sc).round() as usize;
    let num_rows = rows_h / row_h.max(1);

    let nav_y = panel_y + panel_h - nav_h - 2;
    let up_w = (22.0 * sc).round() as usize;
    let up_h = (18.0 * sc).round() as usize;
    let up_x = disasm_x + (6.0 * sc).round() as usize;
    let up_y = nav_y + (3.0 * sc).round() as usize;

    let down_x = up_x + up_w + (4.0 * sc).round() as usize;
    let go_label_x = down_x + up_w + (10.0 * sc).round() as usize;
    let seek_in_x = go_label_x + (26.0 * sc).round() as usize;
    let seek_in_w = (52.0 * sc).round() as usize;
    let go_btn_x = seek_in_x + seek_in_w + (5.0 * sc).round() as usize;
    let go_btn_w = (32.0 * sc).round() as usize;

    let reg_box_h = (168.0 * sc).round() as usize;
    let reg_title_h = (20.0 * sc).round() as usize;
    let r_text_x = right_x + (8.0 * sc).round() as usize;
    let r_gap = (15.0 * sc).round() as usize;
    let flag_w = (14.0 * sc).round() as usize;
    let flag_h = (15.0 * sc).round() as usize;
    let flag_gap = (3.0 * sc).round() as usize;

    let bp_box_y = panel_y + reg_box_h + (6.0 * sc).round() as usize;
    let bp_box_h = panel_h.saturating_sub(reg_box_h + (6.0 * sc).round() as usize);
    let bp_title_h = (20.0 * sc).round() as usize;
    let bp_bottom_h = (26.0 * sc).round() as usize;
    let bp_row_h = (16.0 * sc).round() as usize;
    let bp_rows_h = bp_box_h.saturating_sub(bp_title_h + bp_bottom_h + 4);
    let bp_max_rows = bp_rows_h / bp_row_h.max(1);

    let bctl_y = bp_box_y + bp_box_h - bp_bottom_h;
    let in_bp_w = (46.0 * sc).round() as usize;
    let in_bp_h = (19.0 * sc).round() as usize;
    let in_bp_x = right_x + (6.0 * sc).round() as usize;
    let in_bp_y = bctl_y + (3.0 * sc).round() as usize;

    let bp_btn_gap = (3.0 * sc).round() as usize;
    let add_bp_w = (36.0 * sc).round() as usize;
    let add_bp_x = in_bp_x + in_bp_w + bp_btn_gap;

    let del_bp_w = (36.0 * sc).round() as usize;
    let del_bp_x = add_bp_x + add_bp_w + bp_btn_gap;

    let clr_bp_w = (42.0 * sc).round() as usize;
    let clr_bp_x = del_bp_x + del_bp_w + bp_btn_gap;

    DebuggerLayout {
        win_x, win_y, win_w, win_h,
        title_h,
        close_x, close_y, close_w, close_h,
        btn_y, btn_h,
        run_x, run_w,
        step_x, step_w,
        over_x, over_w,
        out_x, out_w,
        seek_x, seek_w,
        panel_y, panel_h,
        disasm_x, disasm_w,
        row_h, num_rows,
        nav_y, nav_h,
        up_x, up_y, up_w, up_h,
        down_x,
        go_label_x,
        seek_in_x, seek_in_w,
        go_btn_x, go_btn_w,
        right_x, right_w,
        reg_box_h, reg_title_h,
        r_text_x, r_gap,
        flag_w, flag_h, flag_gap,
        bp_box_y, bp_box_h,
        bp_title_h, bp_row_h, bp_max_rows,
        in_bp_x, in_bp_y, in_bp_w, in_bp_h,
        add_bp_x, add_bp_w,
        del_bp_x, del_bp_w,
        clr_bp_x, clr_bp_w,
    }
}

pub(crate) fn render_debugger_window(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    ms: &MenuState,
    colors: &UiColors,
    emu: &mut Emulator,
    is_paused: bool,
    scale: f32,
) {
    let sc = scale;
    let l = compute_debugger_layout(width, height, scale);

    draw_rect(buffer, l.win_x, l.win_y, l.win_w, l.win_h, width, colors.window_bg);
    let border_t = (2.0 * sc).round() as usize;
    draw_rect(buffer, l.win_x, l.win_y, l.win_w, border_t, width, colors.window_border);
    draw_rect(buffer, l.win_x, l.win_y, border_t, l.win_h, width, colors.window_border);
    draw_rect(buffer, l.win_x + l.win_w - border_t, l.win_y, border_t, l.win_h, width, colors.window_border);
    draw_rect(buffer, l.win_x, l.win_y + l.win_h - border_t, l.win_w, border_t, width, colors.window_border);

    draw_rect(buffer, l.win_x, l.win_y, l.win_w, l.title_h, width, colors.dropdown_bg);
    let title_text_y = l.win_y + (6.0 * sc).round() as usize;
    draw_text(buffer, l.win_x + (10.0 * sc).round() as usize, title_text_y, width, "CPU Debugger", colors.menu_text, scale);

    draw_rect(buffer, l.close_x, l.close_y, l.close_w, l.close_h, width, colors.close_bg);
    draw_text(buffer, l.close_x + (6.0 * sc).round() as usize, l.close_y + (5.0 * sc).round() as usize, width, "X", colors.menu_text, scale);

    let (mx, my) = ms.mouse_pos;

    let run_label = if is_paused { "Run" } else { "Pause" };
    let run_hov = point_in_rect(mx, my, l.run_x, l.btn_y, l.run_w, l.btn_h);
    draw_rect(buffer, l.run_x, l.btn_y, l.run_w, l.btn_h, width, colors.box_border);
    draw_rect(buffer, l.run_x + 1, l.btn_y + 1, l.run_w - 2, l.btn_h - 2, width, if run_hov { colors.box_bg_hover } else { colors.box_bg_default });
    let run_tw = (run_label.len() as f32 * 8.0 * sc).round() as usize;
    let run_tx = l.run_x + (l.run_w.saturating_sub(run_tw)) / 2;
    let run_ty = l.btn_y + (l.btn_h.saturating_sub((8.0 * sc).round() as usize)) / 2;
    draw_text(buffer, run_tx, run_ty, width, run_label, colors.menu_text, scale);

    let step_hov = point_in_rect(mx, my, l.step_x, l.btn_y, l.step_w, l.btn_h);
    draw_rect(buffer, l.step_x, l.btn_y, l.step_w, l.btn_h, width, colors.box_border);
    draw_rect(buffer, l.step_x + 1, l.btn_y + 1, l.step_w - 2, l.btn_h - 2, width, if step_hov { colors.box_bg_hover } else { colors.box_bg_default });
    let step_tw = (9.0 * 8.0 * sc).round() as usize;
    let step_tx = l.step_x + (l.step_w.saturating_sub(step_tw)) / 2;
    let step_ty = l.btn_y + (l.btn_h.saturating_sub((8.0 * sc).round() as usize)) / 2;
    draw_text(buffer, step_tx, step_ty, width, "Step Into", colors.menu_text, scale);

    let over_hov = point_in_rect(mx, my, l.over_x, l.btn_y, l.over_w, l.btn_h);
    draw_rect(buffer, l.over_x, l.btn_y, l.over_w, l.btn_h, width, colors.box_border);
    draw_rect(buffer, l.over_x + 1, l.btn_y + 1, l.over_w - 2, l.btn_h - 2, width, if over_hov { colors.box_bg_hover } else { colors.box_bg_default });
    let over_tw = (9.0 * 8.0 * sc).round() as usize;
    let over_tx = l.over_x + (l.over_w.saturating_sub(over_tw)) / 2;
    let over_ty = l.btn_y + (l.btn_h.saturating_sub((8.0 * sc).round() as usize)) / 2;
    draw_text(buffer, over_tx, over_ty, width, "Step Over", colors.menu_text, scale);

    let out_hov = point_in_rect(mx, my, l.out_x, l.btn_y, l.out_w, l.btn_h);
    draw_rect(buffer, l.out_x, l.btn_y, l.out_w, l.btn_h, width, colors.box_border);
    draw_rect(buffer, l.out_x + 1, l.btn_y + 1, l.out_w - 2, l.btn_h - 2, width, if out_hov { colors.box_bg_hover } else { colors.box_bg_default });
    let out_tw = (8.0 * 8.0 * sc).round() as usize;
    let out_tx = l.out_x + (l.out_w.saturating_sub(out_tw)) / 2;
    let out_ty = l.btn_y + (l.btn_h.saturating_sub((8.0 * sc).round() as usize)) / 2;
    draw_text(buffer, out_tx, out_ty, width, "Step Out", colors.menu_text, scale);

    let seek_hov = point_in_rect(mx, my, l.seek_x, l.btn_y, l.seek_w, l.btn_h);
    draw_rect(buffer, l.seek_x, l.btn_y, l.seek_w, l.btn_h, width, colors.box_border);
    draw_rect(buffer, l.seek_x + 1, l.btn_y + 1, l.seek_w - 2, l.btn_h - 2, width, if seek_hov { colors.box_bg_hover } else { colors.box_bg_default });
    let seek_tw = (7.0 * 8.0 * sc).round() as usize;
    let seek_tx = l.seek_x + (l.seek_w.saturating_sub(seek_tw)) / 2;
    let seek_ty = l.btn_y + (l.btn_h.saturating_sub((8.0 * sc).round() as usize)) / 2;
    draw_text(buffer, seek_tx, seek_ty, width, "Seek PC", colors.menu_text, scale);

    draw_rect(buffer, l.disasm_x, l.panel_y, l.disasm_w, l.panel_h, width, colors.box_border);
    draw_rect(buffer, l.disasm_x + 1, l.panel_y + 1, l.disasm_w - 2, l.panel_h - 2, width, colors.box_bg_default);

    let mut cur_addr = ms.debugger_scroll_addr;
    for i in 0..l.num_rows {
        let ry = l.panel_y + 2 + i * l.row_h;
        let inst = disassemble_instruction(emu, cur_addr);
        let is_pc = inst.addr == emu.program_counter;
        let has_bp = emu.breakpoints.contains(&inst.addr);
        let row_hov = point_in_rect(mx, my, l.disasm_x + 1, ry, l.disasm_w - 2, l.row_h);

        let bg_color = if is_pc {
            0xFF1B4965
        } else if row_hov {
            colors.box_bg_hover
        } else {
            colors.box_bg_default
        };
        draw_rect(buffer, l.disasm_x + 1, ry, l.disasm_w - 2, l.row_h, width, bg_color);

        let pc_mark = if is_pc { ">" } else { " " };
        draw_text(buffer, l.disasm_x + (3.0 * sc).round() as usize, ry + (4.0 * sc).round() as usize, width, pc_mark, 0xFF55FF55, scale);

        let bp_mark = if has_bp { "*" } else { " " };
        draw_text(buffer, l.disasm_x + (11.0 * sc).round() as usize, ry + (4.0 * sc).round() as usize, width, bp_mark, 0xFFFF4444, scale);

        let addr_str = format!("{:04X}:", inst.addr);
        draw_text(buffer, l.disasm_x + (20.0 * sc).round() as usize, ry + (4.0 * sc).round() as usize, width, &addr_str, colors.menu_text, scale);

        let mut bytes_str = String::new();
        for b in &inst.bytes {
            bytes_str.push_str(&format!("{:02X} ", b));
        }
        draw_text(buffer, l.disasm_x + (66.0 * sc).round() as usize, ry + (4.0 * sc).round() as usize, width, &bytes_str, 0xFFAAAAAA, scale);

        draw_text(buffer, l.disasm_x + (142.0 * sc).round() as usize, ry + (4.0 * sc).round() as usize, width, inst.mnemonic, 0xFFFFFFFF, scale);
        draw_text(buffer, l.disasm_x + (172.0 * sc).round() as usize, ry + (4.0 * sc).round() as usize, width, &inst.operand, 0xFFDDDDDD, scale);

        cur_addr = cur_addr.wrapping_add(inst.len as u16);
    }

    draw_rect(buffer, l.disasm_x + 1, l.nav_y, l.disasm_w - 2, l.nav_h, width, colors.dropdown_bg);

    let up_hov = point_in_rect(mx, my, l.up_x, l.up_y, l.up_w, l.up_h);
    draw_rect(buffer, l.up_x, l.up_y, l.up_w, l.up_h, width, colors.box_border);
    draw_rect(buffer, l.up_x + 1, l.up_y + 1, l.up_w - 2, l.up_h - 2, width, if up_hov { colors.box_bg_hover } else { colors.box_bg_default });
    let up_tw = (8.0 * sc).round() as usize;
    let up_tx = l.up_x + (l.up_w.saturating_sub(up_tw)) / 2;
    let up_ty = l.up_y + (l.up_h.saturating_sub((8.0 * sc).round() as usize)) / 2;
    draw_text(buffer, up_tx, up_ty, width, "^", colors.menu_text, scale);

    let down_hov = point_in_rect(mx, my, l.down_x, l.up_y, l.up_w, l.up_h);
    draw_rect(buffer, l.down_x, l.up_y, l.up_w, l.up_h, width, colors.box_border);
    draw_rect(buffer, l.down_x + 1, l.up_y + 1, l.up_w - 2, l.up_h - 2, width, if down_hov { colors.box_bg_hover } else { colors.box_bg_default });
    let down_tx = l.down_x + (l.up_w.saturating_sub(up_tw)) / 2;
    draw_text(buffer, down_tx, up_ty, width, "v", colors.menu_text, scale);

    draw_text(buffer, l.go_label_x, up_ty, width, "Go:", colors.menu_text, scale);

    let seek_in_border = if ms.debugger_input_focus == 2 { colors.rebind_border } else { colors.box_border };
    draw_rect(buffer, l.seek_in_x, l.up_y, l.seek_in_w, l.up_h, width, seek_in_border);
    draw_rect(buffer, l.seek_in_x + 1, l.up_y + 1, l.seek_in_w - 2, l.up_h - 2, width, colors.box_bg_default);
    draw_text(buffer, l.seek_in_x + (4.0 * sc).round() as usize, up_ty, width, &ms.debugger_seek_input, colors.menu_text, scale);

    let go_btn_hov = point_in_rect(mx, my, l.go_btn_x, l.up_y, l.go_btn_w, l.up_h);
    draw_rect(buffer, l.go_btn_x, l.up_y, l.go_btn_w, l.up_h, width, colors.box_border);
    draw_rect(buffer, l.go_btn_x + 1, l.up_y + 1, l.go_btn_w - 2, l.up_h - 2, width, if go_btn_hov { colors.box_bg_hover } else { colors.box_bg_default });
    let go_tw = (16.0 * sc).round() as usize;
    let go_tx = l.go_btn_x + (l.go_btn_w.saturating_sub(go_tw)) / 2;
    draw_text(buffer, go_tx, up_ty, width, "Go", colors.menu_text, scale);

    draw_rect(buffer, l.right_x, l.panel_y, l.right_w, l.reg_box_h, width, colors.box_border);
    draw_rect(buffer, l.right_x + 1, l.panel_y + 1, l.right_w - 2, l.reg_box_h - 2, width, colors.box_bg_default);

    draw_rect(buffer, l.right_x + 1, l.panel_y + 1, l.right_w - 2, l.reg_title_h, width, colors.dropdown_bg);
    draw_text(buffer, l.right_x + (8.0 * sc).round() as usize, l.panel_y + (5.0 * sc).round() as usize, width, "Registers", colors.menu_text, scale);

    let mut ry = l.panel_y + l.reg_title_h + (8.0 * sc).round() as usize;

    let pc_str = format!("PC: ${:04X}", emu.program_counter);
    draw_text(buffer, l.r_text_x, ry, width, &pc_str, 0xFF55FF55, scale);
    ry += l.r_gap;

    let a_str = format!("A: ${:02X}  X: ${:02X}", emu.a, emu.x);
    draw_text(buffer, l.r_text_x, ry, width, &a_str, colors.menu_text, scale);
    ry += l.r_gap;

    let y_str = format!("Y: ${:02X}  S: ${:02X}", emu.y, emu.stack_pointer);
    draw_text(buffer, l.r_text_x, ry, width, &y_str, colors.menu_text, scale);
    ry += l.r_gap;

    let p_byte = emu.get_status_byte(false);
    let p_str = format!("P: ${:02X}", p_byte);
    draw_text(buffer, l.r_text_x, ry, width, &p_str, colors.menu_text, scale);
    ry += l.r_gap;

    let flags = [
        ("N", emu.flag_negative),
        ("V", emu.flag_overflow),
        ("-", true),
        ("B", false),
        ("D", emu.flag_decimal),
        ("I", emu.flag_interrupt),
        ("Z", emu.flag_zero),
        ("C", emu.flag_carry),
    ];
    for (idx, (fname, fval)) in flags.iter().enumerate() {
        let fx = l.r_text_x + idx * (l.flag_w + l.flag_gap);
        let fbg = if *fval { 0xFF227722 } else { 0xFF222222 };
        draw_rect(buffer, fx, ry, l.flag_w, l.flag_h, width, colors.box_border);
        draw_rect(buffer, fx + 1, ry + 1, l.flag_w - 2, l.flag_h - 2, width, fbg);
        let ftext_color = if *fval { 0xFFFFFFFF } else { 0xFF666666 };
        let char_tw = (8.0 * sc).round() as usize;
        let cx = fx + (l.flag_w.saturating_sub(char_tw)) / 2;
        let cy = ry + (l.flag_h.saturating_sub(char_tw)) / 2;
        draw_text(buffer, cx, cy, width, fname, ftext_color, scale);
    }
    ry += l.flag_h + (6.0 * sc).round() as usize;

    let cyc_str = format!("Cycles: {}", emu.total_cycles);
    draw_text(buffer, l.r_text_x, ry, width, &cyc_str, 0xFFAAAAAA, scale);
    ry += l.r_gap;

    let ppu_str = format!("SL: {:3}  Dot: {:3}", emu.ppu_scanline, emu.ppu_dot_count);
    draw_text(buffer, l.r_text_x, ry, width, &ppu_str, 0xFFAAAAAA, scale);

    draw_rect(buffer, l.right_x, l.bp_box_y, l.right_w, l.bp_box_h, width, colors.box_border);
    draw_rect(buffer, l.right_x + 1, l.bp_box_y + 1, l.right_w - 2, l.bp_box_h - 2, width, colors.box_bg_default);

    draw_rect(buffer, l.right_x + 1, l.bp_box_y + 1, l.right_w - 2, l.bp_title_h, width, colors.dropdown_bg);
    draw_text(buffer, l.right_x + (8.0 * sc).round() as usize, l.bp_box_y + (5.0 * sc).round() as usize, width, "Breakpoints", colors.menu_text, scale);

    for (i, bp) in emu.breakpoints.iter().enumerate().take(l.bp_max_rows) {
        let bry = l.bp_box_y + l.bp_title_h + 2 + i * l.bp_row_h;
        let is_sel = ms.debugger_selected_bp == Some(i);
        let bp_hov = point_in_rect(mx, my, l.right_x + 2, bry, l.right_w - 4, l.bp_row_h);
        let bg = if is_sel { colors.menu_highlight } else if bp_hov { colors.box_bg_hover } else { colors.box_bg_default };
        draw_rect(buffer, l.right_x + 2, bry, l.right_w - 4, l.bp_row_h, width, bg);
        let bp_text = format!("* ${:04X}", bp);
        draw_text(buffer, l.right_x + (8.0 * sc).round() as usize, bry + (3.0 * sc).round() as usize, width, &bp_text, 0xFFFF7777, scale);
    }

    let in_bp_border = if ms.debugger_input_focus == 1 { colors.rebind_border } else { colors.box_border };
    draw_rect(buffer, l.in_bp_x, l.in_bp_y, l.in_bp_w, l.in_bp_h, width, in_bp_border);
    draw_rect(buffer, l.in_bp_x + 1, l.in_bp_y + 1, l.in_bp_w - 2, l.in_bp_h - 2, width, colors.box_bg_default);
    let in_ty = l.in_bp_y + (l.in_bp_h.saturating_sub((8.0 * sc).round() as usize)) / 2;
    draw_text(buffer, l.in_bp_x + (4.0 * sc).round() as usize, in_ty, width, &ms.debugger_bp_input, colors.menu_text, scale);

    let add_hov = point_in_rect(mx, my, l.add_bp_x, l.in_bp_y, l.add_bp_w, l.in_bp_h);
    draw_rect(buffer, l.add_bp_x, l.in_bp_y, l.add_bp_w, l.in_bp_h, width, colors.box_border);
    draw_rect(buffer, l.add_bp_x + 1, l.in_bp_y + 1, l.add_bp_w - 2, l.in_bp_h - 2, width, if add_hov { colors.box_bg_hover } else { colors.box_bg_default });
    let add_tw = (32.0 * sc).round() as usize;
    let add_tx = l.add_bp_x + (l.add_bp_w.saturating_sub(add_tw)) / 2;
    draw_text(buffer, add_tx, in_ty, width, "+Add", colors.menu_text, scale);

    let del_hov = point_in_rect(mx, my, l.del_bp_x, l.in_bp_y, l.del_bp_w, l.in_bp_h);
    draw_rect(buffer, l.del_bp_x, l.in_bp_y, l.del_bp_w, l.in_bp_h, width, colors.box_border);
    draw_rect(buffer, l.del_bp_x + 1, l.in_bp_y + 1, l.del_bp_w - 2, l.in_bp_h - 2, width, if del_hov { colors.box_bg_hover } else { colors.box_bg_default });
    let del_tw = (32.0 * sc).round() as usize;
    let del_tx = l.del_bp_x + (l.del_bp_w.saturating_sub(del_tw)) / 2;
    draw_text(buffer, del_tx, in_ty, width, "-Del", colors.menu_text, scale);

    let clr_hov = point_in_rect(mx, my, l.clr_bp_x, l.in_bp_y, l.clr_bp_w, l.in_bp_h);
    draw_rect(buffer, l.clr_bp_x, l.in_bp_y, l.clr_bp_w, l.in_bp_h, width, colors.box_border);
    draw_rect(buffer, l.clr_bp_x + 1, l.in_bp_y + 1, l.clr_bp_w - 2, l.in_bp_h - 2, width, if clr_hov { colors.box_bg_hover } else { colors.box_bg_default });
    let clr_tw = (40.0 * sc).round() as usize;
    let clr_tx = l.clr_bp_x + (l.clr_bp_w.saturating_sub(clr_tw)) / 2;
    draw_text(buffer, clr_tx, in_ty, width, "Clear", colors.menu_text, scale);
}

pub fn handle_debugger_click(
    ms: &mut MenuState,
    emu: &mut Emulator,
    paused: &AtomicBool,
    mx: usize,
    my: usize,
    width: usize,
    height: usize,
) {
    let l = compute_debugger_layout(width, height, ms.scale);

    if point_in_rect(mx, my, l.close_x, l.close_y, l.close_w, l.close_h) {
        ms.show_debugger_window = false;
        paused.store(false, Ordering::Relaxed);
        return;
    }

    if point_in_rect(mx, my, l.run_x, l.btn_y, l.run_w, l.btn_h) {
        let cur = paused.load(Ordering::Relaxed);
        paused.store(!cur, Ordering::Relaxed);
        if !cur {
            ms.debugger_scroll_addr = emu.program_counter;
        }
        return;
    }

    if point_in_rect(mx, my, l.step_x, l.btn_y, l.step_w, l.btn_h) {
        paused.store(true, Ordering::Relaxed);
        step_cpu_instruction(emu);
        ms.debugger_scroll_addr = emu.program_counter;
        return;
    }

    if point_in_rect(mx, my, l.over_x, l.btn_y, l.over_w, l.btn_h) {
        step_over(emu, paused);
        ms.debugger_scroll_addr = emu.program_counter;
        return;
    }

    if point_in_rect(mx, my, l.out_x, l.btn_y, l.out_w, l.btn_h) {
        step_out(emu, paused);
        return;
    }

    if point_in_rect(mx, my, l.seek_x, l.btn_y, l.seek_w, l.btn_h) {
        ms.debugger_scroll_addr = emu.program_counter;
        return;
    }

    if point_in_rect(mx, my, l.disasm_x + 1, l.panel_y + 2, l.disasm_w - 2, l.row_h * l.num_rows) {
        let mut row_addr = ms.debugger_scroll_addr;
        for i in 0..l.num_rows {
            let ry = l.panel_y + 2 + i * l.row_h;
            let inst = disassemble_instruction(emu, row_addr);
            if point_in_rect(mx, my, l.disasm_x + 1, ry, l.disasm_w - 2, l.row_h) {
                if let Some(pos) = emu.breakpoints.iter().position(|&a| a == inst.addr) {
                    emu.breakpoints.remove(pos);
                } else {
                    emu.breakpoints.push(inst.addr);
                }
                return;
            }
            row_addr = row_addr.wrapping_add(inst.len as u16);
        }
    }

    if point_in_rect(mx, my, l.up_x, l.up_y, l.up_w, l.up_h) {
        ms.debugger_scroll_addr = instruction_up(emu, ms.debugger_scroll_addr);
        return;
    }

    if point_in_rect(mx, my, l.down_x, l.up_y, l.up_w, l.up_h) {
        ms.debugger_scroll_addr = instruction_down(emu, ms.debugger_scroll_addr);
        return;
    }

    if point_in_rect(mx, my, l.seek_in_x, l.up_y, l.seek_in_w, l.up_h) {
        ms.debugger_input_focus = 2;
        return;
    }

    if point_in_rect(mx, my, l.go_btn_x, l.up_y, l.go_btn_w, l.up_h) {
        if let Some(target) = parse_hex(&ms.debugger_seek_input) {
            ms.debugger_scroll_addr = target;
        }
        return;
    }

    for (i, _) in emu.breakpoints.iter().enumerate().take(l.bp_max_rows) {
        let bry = l.bp_box_y + l.bp_title_h + 2 + i * l.bp_row_h;
        if point_in_rect(mx, my, l.right_x + 2, bry, l.right_w - 4, l.bp_row_h) {
            ms.debugger_selected_bp = Some(i);
            return;
        }
    }

    if point_in_rect(mx, my, l.in_bp_x, l.in_bp_y, l.in_bp_w, l.in_bp_h) {
        ms.debugger_input_focus = 1;
        return;
    }

    if point_in_rect(mx, my, l.add_bp_x, l.in_bp_y, l.add_bp_w, l.in_bp_h) {
        if let Some(target) = parse_hex(&ms.debugger_bp_input) {
            if !emu.breakpoints.contains(&target) {
                emu.breakpoints.push(target);
            }
            ms.debugger_bp_input.clear();
            ms.debugger_bp_caret = 0;
        }
        return;
    }

    if point_in_rect(mx, my, l.del_bp_x, l.in_bp_y, l.del_bp_w, l.in_bp_h) {
        if let Some(idx) = ms.debugger_selected_bp {
            if idx < emu.breakpoints.len() {
                emu.breakpoints.remove(idx);
                if idx >= emu.breakpoints.len() {
                    ms.debugger_selected_bp = emu.breakpoints.len().checked_sub(1);
                }
            }
        }
        return;
    }

    if point_in_rect(mx, my, l.clr_bp_x, l.in_bp_y, l.clr_bp_w, l.in_bp_h) {
        emu.breakpoints.clear();
        ms.debugger_selected_bp = None;
        return;
    }

    ms.debugger_input_focus = 0;
}

pub fn handle_debugger_scroll(ms: &mut MenuState, emu: &mut Emulator, amount: i32) {
    if amount > 0 {
        for _ in 0..amount {
            ms.debugger_scroll_addr = instruction_up(emu, ms.debugger_scroll_addr);
        }
    } else if amount < 0 {
        for _ in 0..(-amount) {
            ms.debugger_scroll_addr = instruction_down(emu, ms.debugger_scroll_addr);
        }
    }
}

pub fn handle_debugger_char(ms: &mut MenuState, c: char) {
    if !c.is_ascii_hexdigit() {
        return;
    }
    let c_up = c.to_ascii_uppercase();
    if ms.debugger_input_focus == 1 {
        if ms.debugger_bp_input.len() < 4 {
            let caret = ms.debugger_bp_caret.min(ms.debugger_bp_input.len());
            ms.debugger_bp_input.insert(caret, c_up);
            ms.debugger_bp_caret = caret + 1;
        }
    } else if ms.debugger_input_focus == 2 {
        if ms.debugger_seek_input.len() < 4 {
            let caret = ms.debugger_seek_caret.min(ms.debugger_seek_input.len());
            ms.debugger_seek_input.insert(caret, c_up);
            ms.debugger_seek_caret = caret + 1;
        }
    }
}

pub fn handle_debugger_key(
    ms: &mut MenuState,
    emu: &mut Emulator,
    paused: &AtomicBool,
    keycode: winit::event::VirtualKeyCode,
) {
    match keycode {
        winit::event::VirtualKeyCode::Escape => {
            ms.show_debugger_window = false;
            paused.store(false, Ordering::Relaxed);
        }
        winit::event::VirtualKeyCode::F5 => {
            let cur = paused.load(Ordering::Relaxed);
            paused.store(!cur, Ordering::Relaxed);
            if !cur {
                ms.debugger_scroll_addr = emu.program_counter;
            }
        }
        winit::event::VirtualKeyCode::F7 => {
            paused.store(true, Ordering::Relaxed);
            step_cpu_instruction(emu);
            ms.debugger_scroll_addr = emu.program_counter;
        }
        winit::event::VirtualKeyCode::F8 => {
            step_over(emu, paused);
            ms.debugger_scroll_addr = emu.program_counter;
        }
        winit::event::VirtualKeyCode::Back => {
            if ms.debugger_input_focus == 1 {
                if ms.debugger_bp_caret > 0 && ms.debugger_bp_caret <= ms.debugger_bp_input.len() {
                    ms.debugger_bp_caret -= 1;
                    let idx = ms.debugger_bp_caret;
                    ms.debugger_bp_input.remove(idx);
                }
            } else if ms.debugger_input_focus == 2 {
                if ms.debugger_seek_caret > 0 && ms.debugger_seek_caret <= ms.debugger_seek_input.len() {
                    ms.debugger_seek_caret -= 1;
                    let idx = ms.debugger_seek_caret;
                    ms.debugger_seek_input.remove(idx);
                }
            }
        }
        winit::event::VirtualKeyCode::Return => {
            if ms.debugger_input_focus == 1 {
                if let Some(target) = parse_hex(&ms.debugger_bp_input) {
                    if !emu.breakpoints.contains(&target) {
                        emu.breakpoints.push(target);
                    }
                    ms.debugger_bp_input.clear();
                    ms.debugger_bp_caret = 0;
                }
            } else if ms.debugger_input_focus == 2 {
                if let Some(target) = parse_hex(&ms.debugger_seek_input) {
                    ms.debugger_scroll_addr = target;
                }
            }
        }
        winit::event::VirtualKeyCode::Delete => {
            if ms.debugger_input_focus == 1 {
                if ms.debugger_bp_caret < ms.debugger_bp_input.len() {
                    let idx = ms.debugger_bp_caret;
                    ms.debugger_bp_input.remove(idx);
                }
            } else if ms.debugger_input_focus == 2 {
                if ms.debugger_seek_caret < ms.debugger_seek_input.len() {
                    let idx = ms.debugger_seek_caret;
                    ms.debugger_seek_input.remove(idx);
                }
            } else if let Some(idx) = ms.debugger_selected_bp {
                if idx < emu.breakpoints.len() {
                    emu.breakpoints.remove(idx);
                    if idx >= emu.breakpoints.len() {
                        ms.debugger_selected_bp = emu.breakpoints.len().checked_sub(1);
                    }
                }
            }
        }
        _ => {}
    }
}
