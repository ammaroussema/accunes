// iNES/ nes2.0 header editor!!!
use crate::status_viewer;
use crate::{draw_rect, draw_text, point_in_rect, UiColors, MenuState};

pub(crate) const VS_SYSTEM_NAMES: [&str; 15] = [
    "Normal",
    "RBI Baseball",
    "TKO Boxing",
    "Super Xevious",
    "Ice Climber",
    "Dual Normal",
    "Dual Raid on Bungeling Bay",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
];

pub(crate) const VS_PPU_NAMES: [&str; 16] = [
    "RP2C03B",
    "RP2C03G",
    "RP2C04-0001",
    "RP2C04-0002",
    "RP2C04-0003",
    "RP2C04-0004",
    "RC2C03B",
    "RC2C03C",
    "RC2C05-01",
    "RC2C05-02",
    "RC2C05-03",
    "RC2C05-04",
    "RC2C05-05",
    "Reserved",
    "Reserved",
    "Reserved",
];

pub(crate) const EXT_CONSOLE_NAMES: [&str; 16] = [
    "Normal",
    "VS. System",
    "Playchoice 10",
    "Bit Corp. Creator",
    "VT01 monochrome",
    "VT01 red/cyan",
    "VT02",
    "VT03",
    "VT09",
    "VT32",
    "VT369",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
];

pub(crate) const INPUT_DEVICE_NAMES: [&str; 53] = [
    "Unspecified",
    "Standard Controllers",
    "Four-score (NES)",
    "Four-score (Famicom)",
    "VS. System",
    "VS. System (swap)",
    "VS. Pinball (J)",
    "VS. Zapper",
    "Zapper",
    "Double Zappers",
    "Bandai Hyper Shot",
    "Power Pad Side A",
    "Power Pad Side B",
    "Family Trainer Side A",
    "Family Trainer Side B",
    "Arkanoid Paddle (NES)",
    "Arkanoid Paddle (Famicom)",
    "Double Arkanoid Paddle",
    "Konami Hyper Shot",
    "Pachinko",
    "Exciting Boxing Bag",
    "Jissen Mahjong",
    "Party Tap",
    "Oeka Kids Tablet",
    "Barcode Reader",
    "Miracle Piano",
    "Pokkun Moguraa",
    "Top Rider",
    "Double-Fisted",
    "Famicom 3D",
    "Doremikko Keyboard",
    "R.O.B. Gyro Set",
    "Famicom Data Recorder",
    "ASCII Turbo File",
    "IGS Storage Battle Box",
    "Family BASIC Keyboard",
    "PEC-586 Keyboard",
    "Bit Corp. Keyboard",
    "Subor Keyboard",
    "Subor Mouse A",
    "Subor Mouse B",
    "SNES Mouse",
    "Multicart",
    "Double SNES controllers",
    "RacerMate Bicycle",
    "U-Force",
    "R.O.B. Stack-Up",
    "City Patrolman Lightgun",
    "Sharp C1 Cassette",
    "Std ctrl swapped D-Pad/BA",
    "Excalibor Sudoku Pad",
    "ABL Pinball",
    "Golden Nugget extra btns",
];

const FOCUS_MAPPER: usize = 1;
const FOCUS_SUBMAPPER: usize = 2;
const FOCUS_PRG_ROM: usize = 3;
const FOCUS_PRG_RAM: usize = 4;
const FOCUS_PRG_NVRAM: usize = 5;
const FOCUS_CHR_ROM: usize = 6;
const FOCUS_CHR_RAM: usize = 7;
const FOCUS_CHR_NVRAM: usize = 8;
const FOCUS_MISC: usize = 9;

pub(crate) struct HeaderFieldRect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub focus: usize,
    pub label: &'static str,
}

pub(crate) struct HeaderEditorLayout {
    pub win_x: usize,
    pub win_y: usize,
    pub win_w: usize,
    pub win_h: usize,
    pub title_h: usize,
    pub close_x: usize,
    pub close_y: usize,
    pub close_w: usize,
    pub close_h: usize,
    pub col0_x: usize,
    pub fields: Vec<HeaderFieldRect>,
    pub version10: (usize, usize, usize, usize),
    pub version20: (usize, usize, usize, usize),
    pub mirr_h: (usize, usize, usize, usize),
    pub mirr_v: (usize, usize, usize, usize),
    pub mirr_4: (usize, usize, usize, usize),
    pub region_ntsc: (usize, usize, usize, usize),
    pub region_pal: (usize, usize, usize, usize),
    pub region_dual: (usize, usize, usize, usize),
    pub region_dendy: (usize, usize, usize, usize),
    pub system_normal: (usize, usize, usize, usize),
    pub system_vs: (usize, usize, usize, usize),
    pub system_pc10: (usize, usize, usize, usize),
    pub system_extend: (usize, usize, usize, usize),
    pub vs_sys_btn: (usize, usize, usize, usize),
    pub vs_ppu_btn: (usize, usize, usize, usize),
    pub ext_btn: (usize, usize, usize, usize),
    pub input_btn: (usize, usize, usize, usize),
    pub chk_trainer: (usize, usize, usize, usize),
    pub chk_battery: (usize, usize, usize, usize),
    pub chk_unofficial: (usize, usize, usize, usize),
    pub chk_un_prg_ram: (usize, usize, usize, usize),
    pub chk_un_region: (usize, usize, usize, usize),
    pub chk_un_bus: (usize, usize, usize, usize),
    pub restore_btn: (usize, usize, usize, usize),
    pub save_btn: (usize, usize, usize, usize),
    pub hex_y: usize,
    pub hex_h: usize,
    pub status_y: usize,
    pub status_h: usize,
    pub mapper_name_x: usize,
    pub mapper_name_y: usize,
}

pub(crate) fn compute_header_editor_layout(width: usize, height: usize, scale: f32) -> HeaderEditorLayout {
    let win_w_cap = width.saturating_sub(16) as f32;
    let win_h_cap = height.saturating_sub(36) as f32;
    let measure = |fit: f32| -> (f32, f32) {
        let s = scale * fit;
        let r = |v: f32| -> f32 { (v * s * 2.0).round() / 2.0 };
        let w = r(10.0) + (r(74.0) + r(64.0) + r(12.0)) * 2.0 + r(150.0) + r(10.0);
        let h = r(24.0) + r(6.0)
            + r(18.0) + r(6.0)
            + r(21.0) * 11.0 + r(6.0) + r(18.0) + r(6.0) + r(16.0) + r(3.0) + r(16.0) + r(6.0);
        (w, h)
    };
    let fit = {
        let est = ((win_w_cap / (462.0 * scale)).min(win_h_cap / (332.0 * scale))).min(1.0).max(0.2);
        let (w0, h0) = measure(est);
        let f1 = est * (win_w_cap / w0.max(1.0)).min(win_h_cap / h0.max(1.0));
        let f1 = f1.min(1.0).max(0.2);
        let (w1, h1) = measure(f1);
        let f2 = f1 * (win_w_cap / w1.max(1.0)).min(win_h_cap / h1.max(1.0));
        f2.min(f1 * 1.02).min(1.0).max(0.2) * 0.96
    };
    let sc = scale * fit;
    let pad_x = (10.0 * sc).round() as usize;
    let pad_y = (6.0 * sc).round() as usize;
    let title_h = (24.0 * sc).round() as usize;
    let close_w = (20.0 * sc).round() as usize;
    let close_h = (20.0 * sc).round() as usize;

    let label_w = (74.0 * sc).round() as usize;
    let field_w = (64.0 * sc).round() as usize;
    let field_h = (18.0 * sc).round() as usize;
    let row_h = (21.0 * sc).round() as usize;
    let btn_h = (18.0 * sc).round() as usize;
    let small_btn_w = (40.0 * sc).round() as usize;
    let gap = (3.0 * sc).round() as usize;
    let col_gap = (12.0 * sc).round() as usize;

    let col0_x = pad_x;
    let col1_x = col0_x + label_w + field_w + col_gap;
    let col2_x = col1_x + label_w + field_w + col_gap;

    let body_y = title_h + pad_y;
    let version10 = (pad_x, body_y, (70.0 * sc).round() as usize, btn_h);
    let version20 = (version10.0.saturating_add(version10.2).saturating_add(gap), body_y, (64.0 * sc).round() as usize, btn_h);

    let y = body_y + btn_h + (6.0 * sc).round() as usize;

    let mut fields: Vec<HeaderFieldRect> = Vec::new();
    let fx0 = col0_x + label_w;
    let fx1 = col1_x + label_w;
    fields.push(HeaderFieldRect { x: fx0, y: y + 0 * row_h, w: field_w, h: field_h, focus: FOCUS_MAPPER, label: "Mapper" });
    fields.push(HeaderFieldRect { x: fx0, y: y + 1 * row_h, w: field_w, h: field_h, focus: FOCUS_SUBMAPPER, label: "Submapper" });
    fields.push(HeaderFieldRect { x: fx0, y: y + 2 * row_h, w: field_w, h: field_h, focus: FOCUS_PRG_ROM, label: "PRG ROM" });
    fields.push(HeaderFieldRect { x: fx0, y: y + 3 * row_h, w: field_w, h: field_h, focus: FOCUS_PRG_RAM, label: "PRG RAM" });
    fields.push(HeaderFieldRect { x: fx0, y: y + 4 * row_h, w: field_w, h: field_h, focus: FOCUS_PRG_NVRAM, label: "PRG NVRAM" });
    fields.push(HeaderFieldRect { x: fx1, y: y + 0 * row_h, w: field_w, h: field_h, focus: FOCUS_CHR_ROM, label: "CHR ROM" });
    fields.push(HeaderFieldRect { x: fx1, y: y + 1 * row_h, w: field_w, h: field_h, focus: FOCUS_CHR_RAM, label: "CHR RAM" });
    fields.push(HeaderFieldRect { x: fx1, y: y + 2 * row_h, w: field_w, h: field_h, focus: FOCUS_CHR_NVRAM, label: "CHR NVRAM" });
    fields.push(HeaderFieldRect { x: fx1, y: y + 3 * row_h, w: field_w, h: field_h, focus: FOCUS_MISC, label: "Misc ROMs" });

    let mirr_y = y;
    let region_y = mirr_y + row_h;
    let system_y = region_y + 2 * row_h;

    let mirr_h = (col2_x, mirr_y, small_btn_w + (6.0 * sc).round() as usize, btn_h);
    let mirr_v = (mirr_h.0.saturating_add(mirr_h.2).saturating_add(gap), mirr_y, small_btn_w, btn_h);
    let mirr_4 = (mirr_v.0.saturating_add(mirr_v.2).saturating_add(gap), mirr_y, small_btn_w + (24.0 * sc).round() as usize, btn_h);

    let region_ntsc = (col2_x, region_y, small_btn_w + (6.0 * sc).round() as usize, btn_h);
    let region_pal = (region_ntsc.0.saturating_add(region_ntsc.2).saturating_add(gap), region_y, small_btn_w + (4.0 * sc).round() as usize, btn_h);
    let region_dual = (col2_x, region_y + row_h, small_btn_w + (6.0 * sc).round() as usize, btn_h);
    let region_dendy = (region_dual.0.saturating_add(region_dual.2).saturating_add(gap), region_y + row_h, small_btn_w + (16.0 * sc).round() as usize, btn_h);

    let system_normal = (col2_x, system_y, small_btn_w + (14.0 * sc).round() as usize, btn_h);
    let system_vs = (system_normal.0.saturating_add(system_normal.2).saturating_add(gap), system_y, small_btn_w + (2.0 * sc).round() as usize, btn_h);
    let system_pc10 = (col2_x, system_y + row_h, small_btn_w + (10.0 * sc).round() as usize, btn_h);
    let system_extend = (system_pc10.0.saturating_add(system_pc10.2).saturating_add(gap), system_y + row_h, small_btn_w + (22.0 * sc).round() as usize, btn_h);

    let wide_btn_w = (150.0 * sc).round() as usize;
    let vs_sys_btn = (col2_x, system_y + 2 * row_h, wide_btn_w, btn_h);
    let vs_ppu_btn = (col2_x, system_y + 3 * row_h, wide_btn_w, btn_h);
    let ext_btn = (col2_x, system_y + 4 * row_h, wide_btn_w, btn_h);
    let input_btn = (col2_x, system_y + 5 * row_h, wide_btn_w, btn_h);

    let chk_y = y + 5 * row_h;
    let chk_w = (14.0 * sc).round() as usize;
    let chk_trainer = (col0_x, chk_y, chk_w, btn_h);
    let chk_battery = (col1_x, chk_y, chk_w, btn_h);
    let chk_unofficial = (col0_x, chk_y + row_h, chk_w, btn_h);
    let chk_un_prg_ram = (col0_x, chk_y + 2 * row_h, chk_w, btn_h);
    let chk_un_region = (col1_x, chk_y + 2 * row_h, chk_w, btn_h);
    let chk_un_bus = (col1_x, chk_y + 3 * row_h, chk_w, btn_h);

    let left_bottom = chk_y + 4 * row_h;
    let right_bottom = system_y + 6 * row_h;
    let btn_row_y = left_bottom.max(right_bottom) + (6.0 * sc).round() as usize;
    let restore_btn = (col0_x, btn_row_y, (86.0 * sc).round() as usize, btn_h);
    let save_btn = (restore_btn.0.saturating_add(restore_btn.2).saturating_add(gap), btn_row_y, (100.0 * sc).round() as usize, btn_h);

    let hex_y = btn_row_y + btn_h + (6.0 * sc).round() as usize;
    let hex_h = (16.0 * sc).round() as usize;
    let status_y = hex_y + hex_h + gap;
    let status_h = (16.0 * sc).round() as usize;

    let content_h = status_y + status_h + pad_y;
    let win_h = title_h + content_h;
    let win_w = mirr_4.0.saturating_add(mirr_4.2).max(col2_x.saturating_add(wide_btn_w)).saturating_add(pad_x + (6.0 * sc).round() as usize);
    let win_x = (width.saturating_sub(win_w)) / 2;
    let win_y = (height.saturating_sub(win_h)) / 2;

    let close_x = win_x.saturating_add(win_w.saturating_sub(close_w).saturating_sub((6.0 * sc).round() as usize));
    let close_y = win_y + (2.0 * sc).round() as usize;

    let map_x = |x: usize| -> usize { win_x + x };
    let map_rect = |r: (usize, usize, usize, usize)| -> (usize, usize, usize, usize) {
        (map_x(r.0), win_y + r.1, r.2, r.3)
    };
    let mut mapped_fields: Vec<HeaderFieldRect> = Vec::new();
    for f in fields {
        mapped_fields.push(HeaderFieldRect {
            x: map_x(f.x),
            y: win_y + f.y,
            w: f.w,
            h: f.h,
            focus: f.focus,
            label: f.label,
        });
    }
    let mapper_name_rel = version20.0.saturating_add(version20.2).saturating_add((12.0 * sc).round() as usize);

    HeaderEditorLayout {
        win_x,
        win_y,
        win_w,
        win_h,
        title_h,
        close_x,
        close_y,
        close_w,
        close_h,
        col0_x,
        fields: mapped_fields,
        version10: map_rect(version10),
        version20: map_rect(version20),
        mirr_h: map_rect(mirr_h),
        mirr_v: map_rect(mirr_v),
        mirr_4: map_rect(mirr_4),
        region_ntsc: map_rect(region_ntsc),
        region_pal: map_rect(region_pal),
        region_dual: map_rect(region_dual),
        region_dendy: map_rect(region_dendy),
        system_normal: map_rect(system_normal),
        system_vs: map_rect(system_vs),
        system_pc10: map_rect(system_pc10),
        system_extend: map_rect(system_extend),
        vs_sys_btn: map_rect(vs_sys_btn),
        vs_ppu_btn: map_rect(vs_ppu_btn),
        ext_btn: map_rect(ext_btn),
        input_btn: map_rect(input_btn),
        chk_trainer: map_rect(chk_trainer),
        chk_battery: map_rect(chk_battery),
        chk_unofficial: map_rect(chk_unofficial),
        chk_un_prg_ram: map_rect(chk_un_prg_ram),
        chk_un_region: map_rect(chk_un_region),
        chk_un_bus: map_rect(chk_un_bus),
        restore_btn: map_rect(restore_btn),
        save_btn: map_rect(save_btn),
        hex_y: win_y + hex_y,
        hex_h,
        status_y: win_y + status_y,
        status_h,
        mapper_name_x: map_x(mapper_name_rel),
        mapper_name_y: win_y + body_y + (btn_h.saturating_sub((8.0 * sc).round() as usize)) / 2,
    }
}

fn fit_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let keep = max_chars.saturating_sub(1);
        let mut s: String = text.chars().take(keep).collect();
        s.push('…');
        s
    }
}

fn field_enabled(ms: &MenuState, focus: usize) -> bool {
    match focus {
        FOCUS_SUBMAPPER | FOCUS_PRG_NVRAM | FOCUS_CHR_RAM | FOCUS_CHR_NVRAM | FOCUS_MISC => ms.header_ines20,
        _ => true,
    }
}

fn cycle_value(cur: usize, max: usize) -> usize {
    if cur >= max { 0 } else { cur + 1 }
}

fn vs_sys_label(v: usize) -> String {
    format!("${:X} {}", v, VS_SYSTEM_NAMES[v])
}

fn vs_ppu_label(v: usize) -> String {
    format!("${:X} {}", v, VS_PPU_NAMES[v])
}

fn ext_label(v: usize) -> String {
    format!("${:X} {}", v, EXT_CONSOLE_NAMES[v])
}

fn input_label_for(v: usize) -> String {
    if v < INPUT_DEVICE_NAMES.len() {
        format!("${:02X} {}", v, INPUT_DEVICE_NAMES[v])
    } else {
        format!("${:02X} Reserved", v)
    }
}

fn parse_size(s: &str) -> Option<u32> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let chars: Vec<char> = s.chars().collect();
    let mut split = None;
    for (i, c) in chars.iter().enumerate() {
        if !c.is_ascii_digit() {
            split = Some(i);
            break;
        }
    }
    let split = split?;
    let num: String = chars[..split].iter().collect();
    let unit: String = chars[split..].iter().map(|c| c.to_ascii_uppercase()).collect();
    let value: u64 = num.parse().ok()?;
    let mult: u64 = match unit.as_str() {
        "B" => 1,
        "K" | "KB" | "KIB" => 1024,
        "M" | "MB" | "MIB" => 1024 * 1024,
        _ => return None,
    };
    let total = value.checked_mul(mult)?;
    if total > u32::MAX as u64 {
        None
    } else {
        Some(total as u32)
    }
}

fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0B".to_string();
    }
    if bytes >= 1024 * 1024 && bytes % (1024 * 1024) == 0 {
        return format!("{}MB", bytes / (1024 * 1024));
    }
    if bytes >= 1024 && bytes % 1024 == 0 {
        return format!("{}KB", bytes / 1024);
    }
    format!("{}B", bytes)
}

fn scrub_garbage(h: &mut [u8; 16]) {
    if &h[7..16] == b"DiskDude!" {
        for b in h[7..16].iter_mut() {
            *b = 0;
        }
    } else if &h[7..15] == b"demiforce" {
        for b in h[7..16].iter_mut() {
            *b = 0;
        }
    } else if &h[10..14] == b"Ni03" {
        if &h[7..10] == b"Dis" {
            for b in h[7..16].iter_mut() {
                *b = 0;
            }
        } else {
            for b in h[10..16].iter_mut() {
                *b = 0;
            }
        }
    }
}

pub(crate) fn load_header_into_state(ms: &mut MenuState, path: &str) -> Result<(), String> {
    let rom = std::fs::read(path).map_err(|e| format!("Error opening {}: {}!", path, e))?;
    if rom.len() < 16 {
        return Err("Invalid NES header.".to_string());
    }
    if &rom[0..4] == b"FDS\x1a" {
        return Err("Editing header of an FDS file is not supported.".to_string());
    }
    if &rom[0..4] == b"UNIF" {
        return Err("Editing header of a UNIF file is not supported.".to_string());
    }
    if &rom[0..4] == b"NESM" || &rom[0..4] == b"NSFE" {
        return Err("Editing header of an NSF file is not supported.".to_string());
    }
    if &rom[0..4] == b"STBX" {
        return Err("Editing header of a Study Box file is not supported.".to_string());
    }
    if &rom[0..4] == b"mfc\x00" {
        return Err("Editing header of an MFC file is not supported.".to_string());
    }
    if &rom[0..4] != b"NES\x1a" {
        return Err("Invalid NES header.".to_string());
    }

    let mut h = [0u8; 16];
    h.copy_from_slice(&rom[0..16]);
    scrub_garbage(&mut h);
    apply_header_bytes(ms, &h);
    ms.header_edit_path = Some(path.to_string());
    ms.header_edit_orig = h;
    ms.header_focus = 0;
    ms.header_caret = 0;
    ms.header_status.clear();
    Ok(())
}

pub(crate) fn apply_header_bytes(ms: &mut MenuState, h: &[u8; 16]) {
    let ines20 = (h[7] & 0x0C) == 0x08;
    ms.header_ines20 = ines20;

    let mut mapper = ((h[6] >> 4) as u16) | ((h[7] & 0xF0) as u16);
    if ines20 {
        mapper |= ((h[8] & 0x0F) as u16) << 8;
    }
    ms.header_mapper = format!("{}", mapper);
    ms.header_submapper = if ines20 { format!("{}", (h[8] >> 4) & 0x0F) } else { "0".to_string() };

    let prg_rom = if ines20 && (h[9] & 0x0F) == 0x0F {
        let lo = h[4] as u64;
        (2 * (lo & 3) + 1) << (lo >> 2)
    } else if ines20 {
        ((h[4] as u64) | (((h[9] & 0x0F) as u64) << 8)) * 16384
    } else {
        h[4] as u64 * 16384
    };
    ms.header_prg_rom = format_size(prg_rom);

    let prg_ram = if ines20 {
        let shift = h[10] & 0x0F;
        if shift == 0 { 0 } else { 64u64 << shift }
    } else if h[10] & 0x10 == 0 {
        h[8] as u64 * 8192
    } else {
        0
    };
    ms.header_prg_ram = format_size(prg_ram);

    let prg_nvram = if ines20 {
        let shift = h[10] >> 4;
        if shift == 0 { 0 } else { 64u64 << shift }
    } else {
        0
    };
    ms.header_prg_nvram = format_size(prg_nvram);

    let chr_rom = if ines20 && (h[9] & 0xF0) == 0xF0 {
        let lo = h[5] as u64;
        (2 * (lo & 3) + 1) << (lo >> 2)
    } else if ines20 {
        ((h[5] as u64) | (((h[9] & 0xF0) as u64) << 4)) * 8192
    } else {
        h[5] as u64 * 8192
    };
    ms.header_chr_rom = format_size(chr_rom);

    let chr_ram = if ines20 {
        let shift = h[11] & 0x0F;
        if shift == 0 { 0 } else { 64u64 << shift }
    } else {
        0
    };
    ms.header_chr_ram = format_size(chr_ram);

    let chr_nvram = if ines20 {
        let shift = h[11] >> 4;
        if shift == 0 { 0 } else { 64u64 << shift }
    } else {
        0
    };
    ms.header_chr_nvram = format_size(chr_nvram);

    ms.header_mirroring = if h[6] & 8 != 0 { 2 } else if h[6] & 1 != 0 { 1 } else { 0 };

    ms.header_unofficial = false;
    ms.header_unofficial_region = false;
    if ines20 {
        ms.header_region = match h[12] & 3 {
            1 => 1,
            2 => 2,
            3 => 3,
            _ => 0,
        };
    } else {
        let region = h[10] & 3;
        if region == 3 || region == 1 {
            ms.header_region = 2;
            ms.header_unofficial = true;
            ms.header_unofficial_region = true;
        } else {
            ms.header_region = if h[9] & 1 != 0 { 1 } else { 0 };
        }
    }

    ms.header_system = match h[7] & 3 {
        1 => 1,
        2 => {
            if !ines20 {
                ms.header_unofficial = true;
            }
            2
        }
        3 => {
            if ines20 {
                3
            } else {
                0
            }
        }
        _ => 0,
    };

    ms.header_misc_roms = if ines20 { format!("{}", h[14] & 3) } else { "0".to_string() };

    ms.header_trainer = h[6] & 4 != 0;
    ms.header_battery = if ines20 {
        prg_nvram > 0
    } else {
        h[6] & 2 != 0
    };
    ms.header_unofficial_prg_ram = !ines20 && h[10] & 0x10 == 0;
    ms.header_unofficial_bus = !ines20 && h[10] & 0x20 != 0;

    if ines20 {
        ms.header_vs_system = ((h[13] >> 4) as usize).min(VS_SYSTEM_NAMES.len() - 1);
        ms.header_vs_ppu = (h[13] & 0x0F) as usize;
        ms.header_extend_console = (h[13] & 0x3F) as usize;
        ms.header_input_device = (h[15] & 0x3F) as usize;
    } else {
        ms.header_vs_system = 0;
        ms.header_vs_ppu = 0;
        ms.header_extend_console = 0;
        ms.header_input_device = 0;
    }

    rebuild_hex(ms);
}

fn parse_mapper(s: &str) -> Option<u32> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(hex) = s.strip_prefix('$') {
        return u32::from_str_radix(hex, 16).ok();
    }
    s.parse::<u32>().ok()
}

fn parse_dec(s: &str) -> Option<u32> {
    s.trim().parse::<u32>().ok()
}

fn build_header(ms: &MenuState) -> Result<[u8; 16], (usize, String)> {
    let ines20 = ms.header_ines20;
    let mut h = [0u8; 16];
    h[0] = b'N';
    h[1] = b'E';
    h[2] = b'S';
    h[3] = 0x1A;

    let mapper = parse_mapper(&ms.header_mapper).ok_or((FOCUS_MAPPER, "Invalid mapper number.".to_string()))?;
    if mapper >= 4096 {
        return Err((FOCUS_MAPPER, "Mapper must be below 4096.".to_string()));
    }
    if !ines20 && mapper >= 256 {
        return Err((FOCUS_MAPPER, "Mapper must be below 256 in iNES 1.0.".to_string()));
    }

    let submapper = if ines20 {
        let v = parse_dec(&ms.header_submapper).ok_or((FOCUS_SUBMAPPER, "Invalid submapper number.".to_string()))?;
        if v >= 16 {
            return Err((FOCUS_SUBMAPPER, "Submapper must be below 16 in iNES 2.0.".to_string()));
        }
        v as u8
    } else {
        0
    };

    let prg_rom = parse_size(&ms.header_prg_rom).ok_or((FOCUS_PRG_ROM, "Invalid PRG ROM size, use e.g. 128KB.".to_string()))?;
    if prg_rom == 0 {
        return Err((FOCUS_PRG_ROM, "PRG ROM size must not be zero.".to_string()));
    }
    if ines20 {
        if prg_rom % 16384 != 0 && (prg_rom & (prg_rom - 1)) != 0 {
            return Err((FOCUS_PRG_ROM, "PRG ROM size is not encodable in iNES 2.0.".to_string()));
        }
    } else if prg_rom % 16384 != 0 {
        return Err((FOCUS_PRG_ROM, "PRG ROM size must be a multiple of 16KB in iNES 1.0.".to_string()));
    }
    let prg_banks = prg_rom / 16384;
    if prg_banks > 0xEFF {
        return Err((FOCUS_PRG_ROM, "PRG ROM size is too large for the format.".to_string()));
    }

    let prg_ram = parse_size(&ms.header_prg_ram).unwrap_or(0);
    let prg_nvram = if ines20 { parse_size(&ms.header_prg_nvram).unwrap_or(0) } else { 0 };
    let chr_rom = parse_size(&ms.header_chr_rom).unwrap_or(0);
    let chr_ram = if ines20 { parse_size(&ms.header_chr_ram).unwrap_or(0) } else { 0 };
    let chr_nvram = if ines20 { parse_size(&ms.header_chr_nvram).unwrap_or(0) } else { 0 };

    if !ines20 {
        if prg_ram % 8192 != 0 {
            return Err((FOCUS_PRG_RAM, "PRG RAM size must be a multiple of 8KB in iNES 1.0.".to_string()));
        }
        if prg_ram / 8192 > 255 {
            return Err((FOCUS_PRG_RAM, "PRG RAM size is too large for iNES 1.0.".to_string()));
        }
        if chr_rom % 8192 != 0 {
            return Err((FOCUS_CHR_ROM, "CHR ROM size must be a multiple of 8KB in iNES 1.0.".to_string()));
        }
        if chr_rom / 8192 > 255 {
            return Err((FOCUS_CHR_ROM, "CHR ROM size is too large for iNES 1.0.".to_string()));
        }
    } else {
        for (val, focus, name) in [
            (prg_ram, FOCUS_PRG_RAM, "PRG RAM"),
            (prg_nvram, FOCUS_PRG_NVRAM, "PRG NVRAM"),
            (chr_ram, FOCUS_CHR_RAM, "CHR RAM"),
            (chr_nvram, FOCUS_CHR_NVRAM, "CHR NVRAM"),
        ] {
            if val == 0 {
                continue;
            }
            if val % 64 != 0 || val & (val - 1) != 0 || val > 4194304 {
                return Err((focus, format!("{} size must be a power of two between 64B and 4MB.", name)));
            }
        }
        if chr_rom != 0 && chr_rom % 8192 != 0 && (chr_rom & (chr_rom - 1)) != 0 {
            return Err((FOCUS_CHR_ROM, "CHR ROM size is not encodable in iNES 2.0.".to_string()));
        }
    }

    let misc = if ines20 {
        let v = parse_dec(&ms.header_misc_roms).ok_or((FOCUS_MISC, "Invalid miscellaneous ROMs count.".to_string()))?;
        if v > 3 {
            return Err((FOCUS_MISC, "Miscellaneous ROMs must be 0-3.".to_string()));
        }
        v as u8
    } else {
        0
    };

    if ines20 && prg_rom % 16384 != 0 {
        let e = prg_rom.trailing_zeros() as u8;
        h[4] = e << 2;
        h[9] |= 0x0F;
    } else {
        h[4] = (prg_banks & 0xFF) as u8;
        h[9] |= ((prg_banks >> 8) & 0x0F) as u8;
    }

    if ines20 && chr_rom != 0 && chr_rom % 8192 != 0 {
        let e = chr_rom.trailing_zeros() as u8;
        h[5] = e << 2;
        h[9] |= 0xF0;
    } else {
        h[5] = (chr_rom / 8192 & 0xFF) as u8;
        h[9] |= (((chr_rom / 8192) >> 4) & 0xF0) as u8;
    }

    if ines20 {
        let shift = |v: u32| -> Option<u8> {
            if v == 0 {
                return Some(0);
            }
            let s = (v / 64).trailing_zeros();
            if 64u64 << s != v as u64 {
                return None;
            }
            Some(s as u8)
        };
        let ram_shift = shift(prg_ram).ok_or((FOCUS_PRG_RAM, "PRG RAM must be a power-of-two multiple of 64 bytes.".to_string()))?;
        let nvram_shift = shift(prg_nvram).ok_or((FOCUS_PRG_NVRAM, "PRG NVRAM must be a power-of-two multiple of 64 bytes.".to_string()))?;
        let cram_shift = shift(chr_ram).ok_or((FOCUS_CHR_RAM, "CHR RAM must be a power-of-two multiple of 64 bytes.".to_string()))?;
        let cnvram_shift = shift(chr_nvram).ok_or((FOCUS_CHR_NVRAM, "CHR NVRAM must be a power-of-two multiple of 64 bytes.".to_string()))?;
        h[10] = ram_shift | (nvram_shift << 4);
        h[11] = cram_shift | (cnvram_shift << 4);
    } else {
        h[8] = (prg_ram / 8192) as u8;
        if ms.header_unofficial {
            if !ms.header_unofficial_prg_ram {
                h[10] |= 0x10;
            }
            if ms.header_unofficial_bus {
                h[10] |= 0x20;
            }
            if ms.header_unofficial_region && ms.header_region == 2 {
                h[10] |= 0x03;
            }
        }
    }

    h[6] |= ((mapper & 0x0F) as u8) << 4;
    h[7] |= (mapper & 0xF0) as u8;
    if ines20 {
        h[7] |= 0x08;
        h[8] |= submapper << 4;
        h[8] |= ((mapper >> 8) & 0x0F) as u8;
    }

    if ms.header_mirroring == 1 {
        h[6] |= 1;
    } else if ms.header_mirroring == 2 {
        h[6] |= 8;
    }
    if ms.header_trainer {
        h[6] |= 4;
    }
    if (!ines20 && ms.header_battery) || (ines20 && prg_nvram > 0) {
        h[6] |= 2;
    }

    match ms.header_region {
        1 => {
            if ines20 {
                h[12] |= 1;
            } else {
                h[9] |= 1;
            }
        }
        2 => {
            if ines20 {
                h[12] |= 2;
            } else if ms.header_unofficial && ms.header_unofficial_region {
                h[10] |= 0x03;
            }
        }
        3 => {
            if ines20 {
                h[12] |= 3;
            }
        }
        _ => {}
    }

    match ms.header_system {
        1 => h[7] |= 1,
        2 => h[7] |= 2,
        3 => {
            if ines20 {
                h[7] |= 3;
            }
        }
        _ => {}
    }

    if ines20 && ms.header_system == 1 {
        h[13] = ((ms.header_vs_system as u8) << 4) | (ms.header_vs_ppu as u8 & 0x0F);
    }
    if ines20 && ms.header_system == 3 {
        h[13] = ms.header_extend_console as u8 & 0x3F;
    }

    if ines20 {
        h[14] = misc & 3;
        h[15] = ms.header_input_device as u8 & 0x3F;
    }

    Ok(h)
}

fn rebuild_hex(ms: &mut MenuState) {
    match build_header(ms) {
        Ok(b) => {
            ms.header_hex = b.iter().map(|x| format!("{:02X}", x)).collect::<Vec<_>>().join(" ");
        }
        Err(_) => {
            ms.header_hex = "-- invalid --".to_string();
        }
    }
}

fn sync_after_change(ms: &mut MenuState) {
    rebuild_hex(ms);
}

pub(crate) fn render_header_editor_window(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    ms: &MenuState,
    colors: &UiColors,
    scale: f32,
) {
    let l = compute_header_editor_layout(width, height, scale);
    let sc = scale;
    let (mx, my) = ms.mouse_pos;

    draw_rect(buffer, l.win_x, l.win_y, l.win_w, l.win_h, width, colors.window_bg);
    draw_rect(buffer, l.win_x, l.win_y, l.win_w, 1, width, colors.window_border);
    draw_rect(buffer, l.win_x, l.win_y + l.win_h.saturating_sub(1), l.win_w, 1, width, colors.window_border);
    draw_rect(buffer, l.win_x, l.win_y, 1, l.win_h, width, colors.window_border);
    draw_rect(buffer, l.win_x + l.win_w.saturating_sub(1), l.win_y, 1, l.win_h, width, colors.window_border);

    draw_rect(buffer, l.win_x + 1, l.win_y + 1, l.win_w.saturating_sub(2), l.title_h, width, colors.dropdown_bg);
    draw_rect(buffer, l.win_x, l.win_y + l.title_h + 1, l.win_w, 1, width, colors.window_border);
    let title_ty = l.win_y + ((l.title_h as f32 - 8.0 * sc) / 2.0).round() as usize;
    draw_text(buffer, l.win_x + (10.0 * sc).round() as usize, title_ty, width, "Header", colors.menu_text, sc);

    let close_hover = point_in_rect(mx, my, l.close_x, l.close_y, l.close_w, l.close_h);
    let close_bg = if close_hover { colors.menu_highlight } else { colors.close_bg };
    draw_rect(buffer, l.close_x, l.close_y, l.close_w, l.close_h, width, close_bg);
    draw_rect(buffer, l.close_x, l.close_y, l.close_w, 1, width, colors.btn_border);
    draw_rect(buffer, l.close_x, l.close_y + l.close_h.saturating_sub(1), l.close_w, 1, width, colors.btn_border);
    draw_rect(buffer, l.close_x, l.close_y, 1, l.close_h, width, colors.btn_border);
    draw_rect(buffer, l.close_x + l.close_w.saturating_sub(1), l.close_y, 1, l.close_h, width, colors.btn_border);
    let close_ty = l.close_y + ((l.close_h as f32 - 8.0 * sc) / 2.0).round() as usize;
    draw_text(buffer, l.close_x + (6.0 * sc).round() as usize, close_ty, width, "X", colors.menu_text, sc);

    let radio = |buf: &mut [u32], r: (usize, usize, usize, usize), text: &str, selected: bool, enabled: bool| {
        let (bx, by, bw, bh) = r;
        let hover = enabled && point_in_rect(mx, my, bx, by, bw, bh);
        let bg = if hover { colors.box_bg_hover } else { colors.box_bg_default };
        draw_rect(buf, bx, by, bw, bh, width, bg);
        let border = if selected { colors.menu_text } else { colors.btn_border };
        draw_rect(buf, bx, by, bw, 1, width, border);
        draw_rect(buf, bx, by + bh.saturating_sub(1), bw, 1, width, border);
        draw_rect(buf, bx, by, 1, bh, width, border);
        draw_rect(buf, bx + bw.saturating_sub(1), by, 1, bh, width, border);
        let text_w = (text.chars().count() as f32 * 8.0 * sc).round() as usize;
        let tx = (bx + (bw.saturating_sub(text_w)) / 2).min(bx + bw.saturating_sub(text_w));
        let ty = by + ((bh as f32 - 8.0 * sc) / 2.0).round() as usize;
        let color = if enabled { colors.menu_text } else { colors.disabled_text };
        draw_text(buf, tx, ty, width, text, color, sc);
        if selected {
            draw_rect(buf, bx + 2, by + bh.saturating_sub(3), bw.saturating_sub(4), 1, width, colors.menu_text);
        }
    };

    let check = |buf: &mut [u32], r: (usize, usize, usize, usize), text: &str, checked: bool, enabled: bool| {
        let (bx, by, bw, bh) = r;
        let hover = enabled && point_in_rect(mx, my, bx, by, bw, bw);
        if hover {
            draw_rect(buf, bx, by, bw + text.chars().count() * (8.0 * sc).round() as usize + (6.0 * sc).round() as usize, bh, width, colors.menu_highlight);
        }
        draw_rect(buf, bx, by, bw, bw, width, if enabled { colors.btn_border } else { colors.disabled_btn_bg });
        draw_rect(buf, bx + 1, by + 1, bw.saturating_sub(2), bw.saturating_sub(2), width, colors.box_bg_default);
        if checked {
            draw_rect(buf, bx + 3, by + 3, bw.saturating_sub(6), bw.saturating_sub(6), width, colors.menu_text);
        }
        let ty = by + ((bh as f32 - 8.0 * sc) / 2.0).round() as usize;
        let color = if enabled { colors.menu_text } else { colors.disabled_text };
        draw_text(buf, bx + bw + (6.0 * sc).round() as usize, ty, width, text, color, sc);
    };

    let ines20 = ms.header_ines20;
    let label_px = (8.0 * sc).round() as usize;
    let win_right = l.win_x + l.win_w;
    let win_bottom = l.win_y + l.win_h;
    let max_label = ((l.win_w / 3).saturating_sub((14.0 * sc).round() as usize) / label_px.max(1)).max(3);

    radio(buffer, l.version10, "iNES 1.0", !ines20, true);
    radio(buffer, l.version20, "NES 2.0", ines20, true);

    for f in &l.fields {
        let value = match f.focus {
            FOCUS_MAPPER => ms.header_mapper.clone(),
            FOCUS_SUBMAPPER => ms.header_submapper.clone(),
            FOCUS_PRG_ROM => ms.header_prg_rom.clone(),
            FOCUS_PRG_RAM => ms.header_prg_ram.clone(),
            FOCUS_PRG_NVRAM => ms.header_prg_nvram.clone(),
            FOCUS_CHR_ROM => ms.header_chr_rom.clone(),
            FOCUS_CHR_RAM => ms.header_chr_ram.clone(),
            FOCUS_CHR_NVRAM => ms.header_chr_nvram.clone(),
            _ => ms.header_misc_roms.clone(),
        };
        let enabled = field_enabled(ms, f.focus);
        let ly = f.y + ((f.h as f32 - 8.0 * sc) / 2.0).round() as usize;
        let label_shown = fit_text(f.label, max_label);
        let lx = f.x.saturating_sub((6.0 * sc).round() as usize + label_shown.chars().count() * label_px).max(l.win_x + l.col0_x);
        draw_text(buffer, lx, ly, width, &label_shown, if enabled { colors.menu_text } else { colors.disabled_text }, sc);
        let focus_here = ms.header_focus == f.focus;
        let border = if focus_here { colors.rebind_border } else { colors.btn_border };
        let bg = if enabled { colors.box_bg_default } else { colors.disabled_btn_bg };
        draw_rect(buffer, f.x, f.y, f.w, f.h, width, bg);
        draw_rect(buffer, f.x, f.y, f.w, 1, width, border);
        draw_rect(buffer, f.x, f.y + f.h.saturating_sub(1), f.w, 1, width, border);
        draw_rect(buffer, f.x, f.y, 1, f.h, width, border);
        draw_rect(buffer, f.x + f.w.saturating_sub(1), f.y, 1, f.h, width, border);
        let text_color = if enabled { colors.menu_text } else { colors.disabled_text };
        let tx = f.x + (6.0 * sc).round() as usize;
        let ty = f.y + ((f.h as f32 - 8.0 * sc) / 2.0).round() as usize;
        let max_chars = (f.w.saturating_sub((8.0 * sc).round() as usize) / (8.0 * sc).round() as usize).max(1);
        let shown: String = value.chars().skip(value.chars().count().saturating_sub(max_chars)).collect();
        let draw_len = shown.chars().count();
        draw_text(buffer, tx, ty, width, &shown, text_color, sc);
        if focus_here {
            let caret_x = (tx + draw_len * (8.0 * sc).round() as usize).min(f.x + f.w.saturating_sub(2));
            draw_rect(buffer, caret_x, f.y + (3.0 * sc).round() as usize, (2.0 * sc).round() as usize, f.h.saturating_sub((6.0 * sc).round() as usize), width, text_color);
        }
        if f.focus == FOCUS_MAPPER {
            if let Some(m) = parse_mapper(&value) {
                let name = status_viewer::get_mapper_name(m as u16);
                let color = if name == "Unknown Mapper" { colors.disabled_text } else { colors.btn_sub_label };
                let name_max = (win_right.saturating_sub(l.mapper_name_x) / label_px.max(1)).max(3);
                let name_shown = fit_text(name, name_max);
                draw_text(buffer, l.mapper_name_x, l.mapper_name_y, width, &name_shown, color, sc);
            }
        }
    }

    radio(buffer, l.mirr_h, "H", ms.header_mirroring == 0, true);
    radio(buffer, l.mirr_v, "V", ms.header_mirroring == 1, true);
    radio(buffer, l.mirr_4, "4-screen", ms.header_mirroring == 2, true);

    radio(buffer, l.region_ntsc, "NTSC", ms.header_region == 0, true);
    radio(buffer, l.region_pal, "PAL", ms.header_region == 1, true);
    radio(buffer, l.region_dual, "Dual", ms.header_region == 2, ines20 || ms.header_unofficial && ms.header_unofficial_region);
    radio(buffer, l.region_dendy, "Dendy", ms.header_region == 3, ines20);

    radio(buffer, l.system_normal, "Normal", ms.header_system == 0, true);
    radio(buffer, l.system_vs, "VS", ms.header_system == 1, true);
    radio(buffer, l.system_pc10, "PC-10", ms.header_system == 2, ines20 || ms.header_unofficial);
    radio(buffer, l.system_extend, "Extend", ms.header_system == 3, ines20);

    let vs_enabled = ines20 && ms.header_system == 1;
    let ext_enabled = ines20 && ms.header_system == 3;
    let wide_max = ((l.vs_sys_btn.2.saturating_sub((8.0 * sc).round() as usize)) / label_px.max(1)).max(3);
    radio(buffer, l.vs_sys_btn, &fit_text(&vs_sys_label(ms.header_vs_system), wide_max), false, vs_enabled);
    radio(buffer, l.vs_ppu_btn, &fit_text(&vs_ppu_label(ms.header_vs_ppu), wide_max), false, vs_enabled);
    radio(buffer, l.ext_btn, &fit_text(&ext_label(ms.header_extend_console), wide_max), false, ext_enabled);
    radio(buffer, l.input_btn, &fit_text(&input_label_for(ms.header_input_device), wide_max), false, ines20);

    check(buffer, l.chk_trainer, "Trainer", ms.header_trainer, true);
    check(buffer, l.chk_battery, "Battery", ms.header_battery, !ines20);
    check(buffer, l.chk_unofficial, "Unofficial Properties", ms.header_unofficial, !ines20);
    check(buffer, l.chk_un_prg_ram, "PRG RAM present", ms.header_unofficial_prg_ram, !ines20 && ms.header_unofficial);
    check(buffer, l.chk_un_region, "Dual region", ms.header_unofficial_region, !ines20 && ms.header_unofficial);
    check(buffer, l.chk_un_bus, "Bus conflicts", ms.header_unofficial_bus, !ines20 && ms.header_unofficial);

    let draw_btn = |buf: &mut [u32], r: (usize, usize, usize, usize), text: &str| {
        let (bx, by, bw, bh) = r;
        let hover = point_in_rect(mx, my, bx, by, bw, bh);
        let bg = if hover { colors.box_bg_hover } else { colors.box_bg_default };
        draw_rect(buf, bx, by, bw, bh, width, bg);
        draw_rect(buf, bx, by, bw, 1, width, colors.btn_border);
        draw_rect(buf, bx, by + bh.saturating_sub(1), bw, 1, width, colors.btn_border);
        draw_rect(buf, bx, by, 1, bh, width, colors.btn_border);
        draw_rect(buf, bx + bw.saturating_sub(1), by, 1, bh, width, colors.btn_border);
        let text_w = (text.chars().count() as f32 * 8.0 * sc).round() as usize;
        let tx = bx + (bw.saturating_sub(text_w)) / 2;
        let ty = by + ((bh as f32 - 8.0 * sc) / 2.0).round() as usize;
        draw_text(buf, tx, ty, width, text, colors.menu_text, sc);
    };
    draw_btn(buffer, l.restore_btn, "Restore");
    draw_btn(buffer, l.save_btn, "Save As...");

    let hex_ty = l.hex_y + ((l.hex_h as f32 - 8.0 * sc) / 2.0).round() as usize;
    let hex_max = (l.win_w.saturating_sub(l.col0_x * 2) / label_px.max(1)).max(3);
    draw_text(buffer, l.win_x + l.col0_x, hex_ty, width, &fit_text(&ms.header_hex, hex_max), colors.menu_text, sc);

    let status_color = if ms.header_status.is_empty() { colors.disabled_text } else { colors.menu_text };
    let status_text = if ms.header_status.is_empty() {
        ms.header_edit_path.clone().unwrap_or_default()
    } else {
        ms.header_status.clone()
    };
    let status_max = (l.win_w.saturating_sub(l.col0_x * 2) / label_px.max(1)).max(3);
    draw_text(buffer, l.win_x + l.col0_x, l.status_y + ((l.status_h as f32 - 8.0 * sc) / 2.0).round() as usize, width, &fit_text(&status_text, status_max), status_color, sc);
    let _ = (win_right, win_bottom);
}

pub(crate) fn handle_header_editor_click(
    ms: &mut MenuState,
    button: winit::event::MouseButton,
    mx: usize,
    my: usize,
    width: usize,
    height: usize,
    scale: f32,
) -> bool {
    if button != winit::event::MouseButton::Left {
        return false;
    }
    let l = compute_header_editor_layout(width, height, scale);

    if point_in_rect(mx, my, l.close_x, l.close_y, l.close_w, l.close_h) {
        ms.show_header_window = false;
        return true;
    }

    if point_in_rect(mx, my, l.version10.0, l.version10.1, l.version10.2, l.version10.3) {
        ms.header_ines20 = false;
        if ms.header_region == 3 {
            ms.header_region = 1;
        }
        if ms.header_system == 3 {
            ms.header_system = 0;
        }
        sync_after_change(ms);
        return true;
    }
    if point_in_rect(mx, my, l.version20.0, l.version20.1, l.version20.2, l.version20.3) {
        ms.header_ines20 = true;
        sync_after_change(ms);
        return true;
    }

    for f in &l.fields {
        if point_in_rect(mx, my, f.x, f.y, f.w, f.h) {
            if field_enabled(ms, f.focus) {
                let value = field_value(ms, f.focus);
                ms.header_focus = f.focus;
                ms.header_caret = value.len();
            }
            return true;
        }
    }

    if point_in_rect(mx, my, l.mirr_h.0, l.mirr_h.1, l.mirr_h.2, l.mirr_h.3) {
        ms.header_mirroring = 0;
        sync_after_change(ms);
        return true;
    }
    if point_in_rect(mx, my, l.mirr_v.0, l.mirr_v.1, l.mirr_v.2, l.mirr_v.3) {
        ms.header_mirroring = 1;
        sync_after_change(ms);
        return true;
    }
    if point_in_rect(mx, my, l.mirr_4.0, l.mirr_4.1, l.mirr_4.2, l.mirr_4.3) {
        ms.header_mirroring = 2;
        sync_after_change(ms);
        return true;
    }

    let region_enabled = |r: usize| -> bool {
        match r {
            2 => ms.header_ines20 || (ms.header_unofficial && ms.header_unofficial_region),
            3 => ms.header_ines20,
            _ => true,
        }
    };
    for (r, rect) in [(0usize, l.region_ntsc), (1, l.region_pal), (2, l.region_dual), (3, l.region_dendy)] {
        if point_in_rect(mx, my, rect.0, rect.1, rect.2, rect.3) {
            if region_enabled(r) {
                ms.header_region = r;
                sync_after_change(ms);
            }
            return true;
        }
    }

    let sys_enabled = |s: usize| -> bool {
        match s {
            2 => ms.header_ines20 || ms.header_unofficial,
            3 => ms.header_ines20,
            _ => true,
        }
    };
    for (s, rect) in [(0usize, l.system_normal), (1, l.system_vs), (2, l.system_pc10), (3, l.system_extend)] {
        if point_in_rect(mx, my, rect.0, rect.1, rect.2, rect.3) {
            if sys_enabled(s) {
                ms.header_system = s;
                sync_after_change(ms);
            }
            return true;
        }
    }

    if point_in_rect(mx, my, l.vs_sys_btn.0, l.vs_sys_btn.1, l.vs_sys_btn.2, l.vs_sys_btn.3) {
        if ms.header_ines20 && ms.header_system == 1 {
            ms.header_vs_system = cycle_value(ms.header_vs_system, VS_SYSTEM_NAMES.len() - 1);
            sync_after_change(ms);
        }
        return true;
    }
    if point_in_rect(mx, my, l.vs_ppu_btn.0, l.vs_ppu_btn.1, l.vs_ppu_btn.2, l.vs_ppu_btn.3) {
        if ms.header_ines20 && ms.header_system == 1 {
            ms.header_vs_ppu = cycle_value(ms.header_vs_ppu, VS_PPU_NAMES.len() - 1);
            sync_after_change(ms);
        }
        return true;
    }
    if point_in_rect(mx, my, l.ext_btn.0, l.ext_btn.1, l.ext_btn.2, l.ext_btn.3) {
        if ms.header_ines20 && ms.header_system == 3 {
            ms.header_extend_console = cycle_value(ms.header_extend_console, EXT_CONSOLE_NAMES.len() - 1);
            sync_after_change(ms);
        }
        return true;
    }
    if point_in_rect(mx, my, l.input_btn.0, l.input_btn.1, l.input_btn.2, l.input_btn.3) {
        if ms.header_ines20 {
            ms.header_input_device = cycle_value(ms.header_input_device, 63);
            sync_after_change(ms);
        }
        return true;
    }

    if point_in_rect(mx, my, l.chk_trainer.0, l.chk_trainer.1, l.chk_trainer.2, l.chk_trainer.3) {
        ms.header_trainer = !ms.header_trainer;
        sync_after_change(ms);
        return true;
    }
    if point_in_rect(mx, my, l.chk_battery.0, l.chk_battery.1, l.chk_battery.2, l.chk_battery.3) {
        if !ms.header_ines20 {
            ms.header_battery = !ms.header_battery;
            sync_after_change(ms);
        }
        return true;
    }
    if point_in_rect(mx, my, l.chk_unofficial.0, l.chk_unofficial.1, l.chk_unofficial.2, l.chk_unofficial.3) {
        if !ms.header_ines20 {
            ms.header_unofficial = !ms.header_unofficial;
            if !ms.header_unofficial {
                ms.header_unofficial_prg_ram = true;
                ms.header_unofficial_region = false;
                ms.header_unofficial_bus = false;
                if ms.header_system == 2 {
                    ms.header_system = 0;
                }
                if ms.header_region == 2 {
                    ms.header_region = 0;
                }
            }
            sync_after_change(ms);
        }
        return true;
    }
    if point_in_rect(mx, my, l.chk_un_prg_ram.0, l.chk_un_prg_ram.1, l.chk_un_prg_ram.2, l.chk_un_prg_ram.3) {
        if !ms.header_ines20 && ms.header_unofficial {
            ms.header_unofficial_prg_ram = !ms.header_unofficial_prg_ram;
            sync_after_change(ms);
        }
        return true;
    }
    if point_in_rect(mx, my, l.chk_un_region.0, l.chk_un_region.1, l.chk_un_region.2, l.chk_un_region.3) {
        if !ms.header_ines20 && ms.header_unofficial {
            ms.header_unofficial_region = !ms.header_unofficial_region;
            sync_after_change(ms);
        }
        return true;
    }
    if point_in_rect(mx, my, l.chk_un_bus.0, l.chk_un_bus.1, l.chk_un_bus.2, l.chk_un_bus.3) {
        if !ms.header_ines20 && ms.header_unofficial {
            ms.header_unofficial_bus = !ms.header_unofficial_bus;
            sync_after_change(ms);
        }
        return true;
    }

    if point_in_rect(mx, my, l.restore_btn.0, l.restore_btn.1, l.restore_btn.2, l.restore_btn.3) {
        let orig = ms.header_edit_orig;
        apply_header_bytes(ms, &orig);
        ms.header_focus = 0;
        ms.header_caret = 0;
        ms.header_status.clear();
        return true;
    }

    if point_in_rect(mx, my, l.save_btn.0, l.save_btn.1, l.save_btn.2, l.save_btn.3) {
        save_header_from_state(ms);
        return true;
    }

    false
}

fn field_value(ms: &MenuState, focus: usize) -> String {
    match focus {
        FOCUS_MAPPER => ms.header_mapper.clone(),
        FOCUS_SUBMAPPER => ms.header_submapper.clone(),
        FOCUS_PRG_ROM => ms.header_prg_rom.clone(),
        FOCUS_PRG_RAM => ms.header_prg_ram.clone(),
        FOCUS_PRG_NVRAM => ms.header_prg_nvram.clone(),
        FOCUS_CHR_ROM => ms.header_chr_rom.clone(),
        FOCUS_CHR_RAM => ms.header_chr_ram.clone(),
        FOCUS_CHR_NVRAM => ms.header_chr_nvram.clone(),
        _ => ms.header_misc_roms.clone(),
    }
}

fn field_set(ms: &mut MenuState, focus: usize, v: String) {
    match focus {
        FOCUS_MAPPER => ms.header_mapper = v,
        FOCUS_SUBMAPPER => ms.header_submapper = v,
        FOCUS_PRG_ROM => ms.header_prg_rom = v,
        FOCUS_PRG_RAM => ms.header_prg_ram = v,
        FOCUS_PRG_NVRAM => ms.header_prg_nvram = v,
        FOCUS_CHR_ROM => ms.header_chr_rom = v,
        FOCUS_CHR_RAM => ms.header_chr_ram = v,
        FOCUS_CHR_NVRAM => ms.header_chr_nvram = v,
        _ => ms.header_misc_roms = v,
    }
}

pub(crate) fn handle_header_editor_key(ms: &mut MenuState, keycode: winit::event::VirtualKeyCode) {
    match keycode {
        winit::event::VirtualKeyCode::Escape => {
            ms.show_header_window = false;
        }
        winit::event::VirtualKeyCode::Back => {
            if ms.header_focus != 0 {
                let v = field_value(ms, ms.header_focus);
                if ms.header_caret > 0 && ms.header_caret <= v.len() {
                    let c = ms.header_caret;
                    let mut nv = v.clone();
                    nv.remove(c - 1);
                    ms.header_caret = c - 1;
                    field_set(ms, ms.header_focus, nv);
                    sync_after_change(ms);
                }
            }
        }
        winit::event::VirtualKeyCode::Delete => {
            if ms.header_focus != 0 {
                let v = field_value(ms, ms.header_focus);
                if ms.header_caret < v.len() {
                    let c = ms.header_caret;
                    let mut nv = v.clone();
                    nv.remove(c);
                    field_set(ms, ms.header_focus, nv);
                    sync_after_change(ms);
                }
            }
        }
        winit::event::VirtualKeyCode::Left => {
            if ms.header_focus != 0 {
                ms.header_caret = ms.header_caret.saturating_sub(1);
            }
        }
        winit::event::VirtualKeyCode::Right => {
            if ms.header_focus != 0 {
                let v = field_value(ms, ms.header_focus);
                ms.header_caret = (ms.header_caret + 1).min(v.len());
            }
        }
        winit::event::VirtualKeyCode::Tab | winit::event::VirtualKeyCode::Down => {
            let mut next = ms.header_focus;
            loop {
                next = if next >= 9 { 0 } else { next + 1 };
                if next == 0 || field_enabled(ms, next) {
                    break;
                }
            }
            ms.header_focus = next;
            ms.header_caret = field_value(ms, next).len();
        }
        winit::event::VirtualKeyCode::Up => {
            let mut next = ms.header_focus;
            loop {
                next = if next == 0 { 9 } else { next - 1 };
                if next == 0 || field_enabled(ms, next) {
                    break;
                }
            }
            ms.header_focus = next;
            ms.header_caret = field_value(ms, next).len();
        }
        _ => {}
    }
}

pub(crate) fn handle_header_editor_char(ms: &mut MenuState, c: char) {
    if ms.header_focus == 0 {
        return;
    }
    if !c.is_ascii_graphic() {
        return;
    }
    let v = field_value(ms, ms.header_focus);
    let caret = ms.header_caret.min(v.len());
    let mut nv = v.clone();
    nv.insert(caret, c);
    if nv.len() > 16 {
        return;
    }
    ms.header_caret = caret + 1;
    field_set(ms, ms.header_focus, nv);
    sync_after_change(ms);
}

pub(crate) fn save_header_from_state(ms: &mut MenuState) {
    let path = match ms.header_edit_path.clone() {
        Some(p) => p,
        None => {
            ms.header_status = "No file loaded.".to_string();
            return;
        }
    };
    let built = match build_header(ms) {
        Ok(b) => b,
        Err((_, msg)) => {
            ms.header_status = msg;
            return;
        }
    };
    let rom = match std::fs::read(&path) {
        Ok(r) => r,
        Err(e) => {
            ms.header_status = format!("Error opening {}: {}!", path, e);
            return;
        }
    };
    if rom.len() < 16 {
        ms.header_status = "Invalid NES header.".to_string();
        return;
    }
    let default_name = {
        let stem = std::path::Path::new(&path)
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("rom");
        let hex: String = built[4..16].iter().map(|x| format!("{:02X}", x)).collect();
        format!("{} [{}].nes", stem, hex)
    };
    let target = match rfd::FileDialog::new()
        .set_file_name(&default_name)
        .add_filter("NES ROM", &["nes"])
        .save_file()
    {
        Some(t) => t,
        None => return,
    };
    let mut out = Vec::with_capacity(rom.len());
    out.extend_from_slice(&built);
    out.extend_from_slice(&rom[16..]);
    match std::fs::write(&target, &out) {
        Ok(_) => {
            let shown = target.to_string_lossy().to_string();
            status_viewer::log_status_message(ms, format!("Header saved to {}", shown));
            ms.header_status = format!("Saved: {}", shown);
            ms.header_edit_path = Some(shown);
            ms.header_edit_orig = built;
        }
        Err(e) => {
            ms.header_status = format!("Failed to write {}: {}", target.to_string_lossy(), e);
        }
    }
}
