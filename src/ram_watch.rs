// ram watching tool!!!
use std::sync::Mutex;

use crate::cheats::{CheatEntry, CheatType};
use crate::emulator::Emulator;
use crate::{draw_rect, draw_text, point_in_rect, MenuState, UiColors};

const RED_TEXT: u32 = 0xFF4040C0;
const LIST_ROWS: usize = 10;
const MAX_ADDR_LEN: usize = 6;
const MAX_NAME_LEN: usize = 32;

const ACT_UP: usize = 0;
const ACT_NEW: usize = 1;
const ACT_REMOVE: usize = 2;
const ACT_SEPARATOR: usize = 3;
const ACT_DOWN: usize = 4;
const ACT_EDIT: usize = 5;
const ACT_DUPLICATE: usize = 6;
const ACT_CHEAT: usize = 7;

const TOOL_NEWLIST: usize = 0;
const TOOL_OPEN: usize = 1;
const TOOL_SAVE: usize = 2;
const TOOL_SAVEAS: usize = 3;
const TOOL_APPEND: usize = 4;

const EDIT_FOCUS_ADDR: usize = 0;
const EDIT_FOCUS_NAME: usize = 1;

#[derive(Clone)]
struct RamWatchEntry {
    name: String,
    addr: u16,
    size: u8,
    signed: bool,
    is_sep: bool,
    val: u64,
}

struct RamWatchState {
    entries: Vec<RamWatchEntry>,
    filename: Option<String>,
    note: String,
    editor_active: bool,
    edit_index: Option<usize>,
    in_addr: String,
    in_name: String,
    addr_caret: usize,
    name_caret: usize,
    editor_focus: usize,
    edit_size: u8,
    edit_signed: bool,
}

fn new_entry(name: String, addr: u16, size: u8, signed: bool) -> RamWatchEntry {
    RamWatchEntry { name, addr, size, signed, is_sep: false, val: 0 }
}

fn new_separator(name: String) -> RamWatchEntry {
    RamWatchEntry { name, addr: 0, size: 1, signed: false, is_sep: true, val: 0 }
}

static RAM_WATCH_STATE: Mutex<RamWatchState> = Mutex::new(RamWatchState {
    entries: Vec::new(),
    filename: None,
    note: String::new(),
    editor_active: false,
    edit_index: None,
    in_addr: String::new(),
    in_name: String::new(),
    addr_caret: 0,
    name_caret: 0,
    editor_focus: EDIT_FOCUS_ADDR,
    edit_size: 1,
    edit_signed: false,
});

pub(crate) fn open_window() {
    let mut st = RAM_WATCH_STATE.lock().unwrap();
    st.editor_active = false;
    st.note.clear();
}

pub(crate) fn add_from_search(addr: u16, size: u8, signed: bool) -> Option<usize> {
    let mut st = RAM_WATCH_STATE.lock().unwrap();
    st.editor_active = false;
    st.entries.push(new_entry("Quick Watch Add".to_string(), addr, size, signed));
    let idx = st.entries.len() - 1;
    st.note = format!("Added watch {:04X} ({}-byte {})", addr, size, if signed { "signed" } else { "unsigned" });
    Some(idx)
}

pub(crate) fn on_frame(emu: &mut Emulator) {
    let mut st = RAM_WATCH_STATE.lock().unwrap();
    refresh(&mut st, emu);
}

fn refresh(st: &mut RamWatchState, emu: &mut Emulator) {
    for e in st.entries.iter_mut() {
        if e.is_sep {
            continue;
        }
        let mut v: u64 = 0;
        for k in 0..e.size as u16 {
            v |= (emu.debug_peek_mem(e.addr.wrapping_add(k)) as u64) << (8 * k);
        }
        e.val = v;
    }
}

fn ram_watch_fit_factor(scale: f32, width: usize, height: usize, editing: bool) -> f32 {
    let cap_w = width.saturating_sub(16) as f32;
    let cap_h = height.saturating_sub(36) as f32;
    let measure = |fit: f32| -> (f32, f32) {
        let s = scale * fit;
        let r = |v: f32| -> f32 { (v * s * 2.0).round() / 2.0 };
        let w = r(8.0) + r(320.0) + r(6.0) + r(196.0) + r(8.0);
        let h = r(22.0) + r(4.0) + r(15.0) + r(4.0) + r(14.0) + r(13.0) * 10.0 + r(8.0)
            + if editing { r(69.0) + r(8.0) } else { 0.0 }
            + r(12.0)
            + r(8.0);
        (w, h)
    };
    let est_h = if editing { 294.0 } else { 217.0 };
    let est = ((cap_w / (506.0 * scale)).min(cap_h / (est_h * scale))).min(1.0).max(0.2);
    let (w0, h0) = measure(est);
    let f1 = est * (cap_w / w0.max(1.0)).min(cap_h / h0.max(1.0));
    let f1 = f1.min(1.0).max(0.2);
    let (w1, h1) = measure(f1);
    let f2 = f1 * (cap_w / w1.max(1.0)).min(cap_h / h1.max(1.0));
    f2.min(f1 * 1.02).min(1.0).max(0.2) * 0.96
}

pub(crate) struct RamWatchLayout {
    pub win_x: usize,
    pub win_y: usize,
    pub win_w: usize,
    pub win_h: usize,
    pub title_h: usize,
    pub close: (usize, usize, usize, usize),
    pub toolbar: [(usize, usize, usize, usize); 5],
    pub list: (usize, usize, usize, usize),
    pub header_h: usize,
    pub row_h: usize,
    pub rows_visible: usize,
    pub col_x: [usize; 4],
    pub col_w: [usize; 4],
    pub act: [(usize, usize, usize, usize); 8],
    pub editor: Option<EditorLayout>,
    pub note_y: usize,
}

pub(crate) struct EditorLayout {
    pub box_pos: (usize, usize, usize, usize),
    pub addr_in: (usize, usize, usize, usize),
    pub name_in: (usize, usize, usize, usize),
    pub signed_opts: [(usize, usize, usize, usize); 2],
    pub size_opts: [(usize, usize, usize, usize); 3],
    pub ok: (usize, usize, usize, usize),
    pub cancel: (usize, usize, usize, usize),
}

pub(crate) fn compute_ram_watch_layout(width: usize, height: usize, scale: f32, editing: bool) -> RamWatchLayout {
    let fit = ram_watch_fit_factor(scale, width, height, editing);
    let sc = scale * fit;
    let u = |v: f32| -> usize { (v * sc).round() as usize };

    let pad = u(8.0);
    let title_h = u(22.0);
    let list_x = pad;
    let list_y = title_h + u(4.0) + u(15.0) + u(4.0);
    let header_h = u(14.0);
    let row_h = u(13.0);
    let rows_visible = LIST_ROWS;
    let list_w = u(320.0);
    let list_h = header_h + row_h * rows_visible;
    let col_off = [0usize, u(56.0), u(128.0), u(208.0)];
    let col_w = [u(56.0), u(72.0), u(80.0), list_w.saturating_sub(u(216.0))];

    let btn_x = list_x + list_w + u(6.0);
    let mut act = [(0usize, 0usize, 0usize, 0usize); 8];
    for i in 0..8 {
        let col = i / 4;
        let row = i % 4;
        let bx = btn_x + col * (u(96.0) + u(4.0));
        let by = list_y + row * (u(15.0) + u(5.0));
        act[i] = (bx, by, u(96.0), u(15.0));
    }

    let mut toolbar = [(0usize, 0usize, 0usize, 0usize); 5];
    let bar_y = title_h + u(4.0);
    for i in 0..5 {
        toolbar[i] = (pad + i * (u(84.0) + u(4.0)), bar_y, u(84.0), u(15.0));
    }

    let editor = if editing {
        let box_x = pad;
        let box_y = list_y + list_h + u(8.0);
        let box_w = list_w;
        let box_h = u(69.0);
        let radio_y = box_y + u(36.0);
        let radio_h = u(12.0);
        let ok_y = box_y + u(53.0);
        Some(EditorLayout {
            box_pos: (box_x, box_y, box_w, box_h),
            addr_in: (box_x + u(60.0), box_y + u(2.0), u(110.0), u(13.0)),
            name_in: (box_x + u(60.0), box_y + u(19.0), box_w.saturating_sub(u(68.0)), u(13.0)),
            signed_opts: [
                (box_x + u(42.0), radio_y, u(48.0), radio_h),
                (box_x + u(94.0), radio_y, u(48.0), radio_h),
            ],
            size_opts: [
                (box_x + u(184.0), radio_y, u(34.0), radio_h),
                (box_x + u(222.0), radio_y, u(34.0), radio_h),
                (box_x + u(260.0), radio_y, u(34.0), radio_h),
            ],
            ok: (box_x + box_w.saturating_sub(u(120.0)), ok_y, u(56.0), u(12.0)),
            cancel: (box_x + box_w.saturating_sub(u(60.0)), ok_y, u(56.0), u(12.0)),
        })
    } else {
        None
    };

    let note_y = match &editor {
        Some(e) => e.box_pos.1 + e.box_pos.3 + u(8.0),
        None => list_y + list_h + u(8.0),
    };
    let win_w = btn_x + u(196.0) + pad;
    let win_h = note_y + u(12.0) + pad;
    let win_x = (width.saturating_sub(win_w)) / 2;
    let win_y = (height.saturating_sub(win_h)) / 2;
    let close_w = u(20.0);
    let close_h = u(20.0);
    let close_x = win_x + win_w.saturating_sub(close_w + u(6.0));
    let close_y = win_y + u(2.0);

    let map = |r: (usize, usize, usize, usize)| -> (usize, usize, usize, usize) {
        (win_x + r.0, win_y + r.1, r.2, r.3)
    };
    let mut m_toolbar = [(0usize, 0usize, 0usize, 0usize); 5];
    for i in 0..5 {
        m_toolbar[i] = map(toolbar[i]);
    }
    let mut m_act = [(0usize, 0usize, 0usize, 0usize); 8];
    for i in 0..8 {
        m_act[i] = map(act[i]);
    }
    let m_editor = editor.map(|e| EditorLayout {
        box_pos: map(e.box_pos),
        addr_in: map(e.addr_in),
        name_in: map(e.name_in),
        signed_opts: [map(e.signed_opts[0]), map(e.signed_opts[1])],
        size_opts: [map(e.size_opts[0]), map(e.size_opts[1]), map(e.size_opts[2])],
        ok: map(e.ok),
        cancel: map(e.cancel),
    });
    let mut m_col_x = [0usize; 4];
    for i in 0..4 {
        m_col_x[i] = win_x + list_x + col_off[i];
    }

    RamWatchLayout {
        win_x,
        win_y,
        win_w,
        win_h,
        title_h,
        close: (close_x, close_y, close_w, close_h),
        toolbar: m_toolbar,
        list: (win_x + list_x, win_y + list_y, list_w, list_h),
        header_h,
        row_h,
        rows_visible,
        col_x: m_col_x,
        col_w,
        act: m_act,
        editor: m_editor,
        note_y: win_y + note_y,
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

fn format_dec(e: &RamWatchEntry) -> String {
    if e.signed {
        match e.size {
            1 => ((e.val as u8) as i8).to_string(),
            2 => ((e.val as u16) as i16).to_string(),
            _ => ((e.val as u32) as i32).to_string(),
        }
    } else {
        match e.size {
            1 => (e.val as u8).to_string(),
            2 => (e.val as u16).to_string(),
            _ => (e.val as u32).to_string(),
        }
    }
}

fn format_hex(e: &RamWatchEntry) -> String {
    match e.size {
        1 => format!("0x{:02X}", e.val as u8),
        2 => format!("0x{:04X}", e.val as u16),
        _ => format!("0x{:08X}", e.val as u32),
    }
}

pub(crate) fn render_ram_watch_window(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    ms: &MenuState,
    colors: &UiColors,
    emu: &mut Emulator,
    scale: f32,
) {
    let editing = RAM_WATCH_STATE.lock().unwrap().editor_active;
    let l = compute_ram_watch_layout(width, height, scale, editing);
    let sc = scale * ram_watch_fit_factor(scale, width, height, editing);
    let mut st = RAM_WATCH_STATE.lock().unwrap();
    refresh(&mut st, emu);
    let (mx, my) = ms.mouse_pos;
    let char_px = ((8.0 * sc).round() as usize).max(1);

    draw_rect(buffer, l.win_x, l.win_y, l.win_w, l.win_h, width, colors.window_bg);
    draw_rect(buffer, l.win_x, l.win_y, l.win_w, 1, width, colors.window_border);
    draw_rect(buffer, l.win_x, l.win_y + l.win_h.saturating_sub(1), l.win_w, 1, width, colors.window_border);
    draw_rect(buffer, l.win_x, l.win_y, 1, l.win_h, width, colors.window_border);
    draw_rect(buffer, l.win_x + l.win_w.saturating_sub(1), l.win_y, 1, l.win_h, width, colors.window_border);

    draw_rect(buffer, l.win_x + 1, l.win_y + 1, l.win_w.saturating_sub(2), l.title_h, width, colors.dropdown_bg);
    draw_rect(buffer, l.win_x, l.win_y + l.title_h + 1, l.win_w, 1, width, colors.window_border);
    let title_ty = l.win_y + ((l.title_h as f32 - 8.0 * sc) / 2.0).round() as usize;
    draw_text(buffer, l.win_x + (10.0 * sc).round() as usize, title_ty, width, "RAM Watch", colors.menu_text, sc);

    let (cx, cy, cw, ch) = l.close;
    let close_hover = point_in_rect(mx, my, cx, cy, cw, ch);
    draw_rect(buffer, cx, cy, cw, ch, width, if close_hover { colors.menu_highlight } else { colors.close_bg });
    let close_ty = cy + ((ch as f32 - 8.0 * sc) / 2.0).round() as usize;
    draw_text(buffer, cx + (6.0 * sc).round() as usize, close_ty, width, "X", colors.menu_text, sc);

    let toolbar_labels = ["New List", "Open", "Save", "Save As", "Append"];
    for i in 0..5 {
        let (bx, by, bw, bh) = l.toolbar[i];
        let hover = point_in_rect(mx, my, bx, by, bw, bh);
        draw_rect(buffer, bx, by, bw, bh, width, colors.btn_border);
        draw_rect(buffer, bx + 1, by + 1, bw.saturating_sub(2), bh.saturating_sub(2), width, if hover { colors.box_bg_hover } else { colors.box_bg_default });
        let label = fit_text(toolbar_labels[i], bw.saturating_sub((6.0 * sc).round() as usize) / char_px);
        let tw = label.chars().count() * char_px;
        let tx = bx + bw.saturating_sub(tw) / 2;
        let ty = by + ((bh as f32 - 8.0 * sc) / 2.0).round() as usize;
        draw_text(buffer, tx, ty, width, &label, colors.menu_text, sc);
    }

    let count = st.entries.len();
    let scroll = ms.ram_watch_scroll.min(count.saturating_sub(l.rows_visible));

    let (lx, ly, lw, lh) = l.list;
    draw_rect(buffer, lx, ly, lw, lh, width, colors.box_bg_default);
    draw_rect(buffer, lx, ly, lw, 1, width, colors.btn_border);
    draw_rect(buffer, lx, ly + lh.saturating_sub(1), lw, 1, width, colors.btn_border);
    draw_rect(buffer, lx, ly, 1, lh, width, colors.btn_border);
    draw_rect(buffer, lx + lw.saturating_sub(1), ly, 1, lh, width, colors.btn_border);

    draw_rect(buffer, lx + 1, ly + 1, lw.saturating_sub(2), l.header_h.saturating_sub(1), width, colors.dropdown_bg);
    let headers = ["Addr", "Decimal", "Hex", "Notes"];
    for i in 0..4 {
        let label = fit_text(headers[i], l.col_w[i] / char_px);
        let ty = ly + ((l.header_h as f32 - 8.0 * sc) / 2.0).round() as usize;
        draw_text(buffer, l.col_x[i] + (4.0 * sc).round() as usize, ty, width, &label, colors.btn_sub_label, sc);
    }

    for r in 0..l.rows_visible {
        let item = scroll + r;
        if item >= count {
            break;
        }
        let ry = ly + l.header_h + r * l.row_h;
        let selected = ms.ram_watch_selected == Some(item);
        if selected {
            draw_rect(buffer, lx + 1, ry, lw.saturating_sub(2), l.row_h, width, colors.menu_highlight);
        }
        let e = &st.entries[item];
        let row_ty = ry + ((l.row_h as f32 - 8.0 * sc) / 2.0).round() as usize;
        let max_name = l.col_x[3].saturating_sub(l.col_x[0]).saturating_sub((8.0 * sc).round() as usize) / char_px;
        if e.is_sep {
            let name = if e.name.is_empty() { "--------".to_string() } else { e.name.clone() };
            let shown = fit_text(&name, max_name);
            let tw = shown.chars().count() * char_px;
            let tx = lx + lw.saturating_sub(tw) / 2;
            draw_text(buffer, tx, row_ty, width, &shown, colors.menu_text, sc);
            continue;
        }
        let texts = [format!("{:04X}", e.addr), format_dec(e), format_hex(e), e.name.clone()];
        for i in 0..4 {
            let shown = fit_text(&texts[i], l.col_w[i].saturating_sub((6.0 * sc).round() as usize) / char_px);
            draw_text(buffer, l.col_x[i] + (4.0 * sc).round() as usize, row_ty, width, &shown, colors.menu_text, sc);
        }
    }

    let act_labels = ["Up", "New", "Remove", "Sep", "Down", "Edit", "Duplicate", "Add Cheat"];
    let act_enabled = [
        ms.ram_watch_selected.is_some() && count > 1,
        true,
        ms.ram_watch_selected.is_some(),
        true,
        ms.ram_watch_selected.is_some() && count > 1,
        ms.ram_watch_selected.is_some(),
        ms.ram_watch_selected.is_some(),
        ms.ram_watch_selected.is_some(),
    ];
    for i in 0..8 {
        let (bx, by, bw, bh) = l.act[i];
        let enabled = act_enabled[i];
        let hover = enabled && point_in_rect(mx, my, bx, by, bw, bh);
        draw_rect(buffer, bx, by, bw, bh, width, if enabled { colors.btn_border } else { colors.disabled_btn_bg });
        draw_rect(buffer, bx + 1, by + 1, bw.saturating_sub(2), bh.saturating_sub(2), width, if hover { colors.box_bg_hover } else if enabled { colors.box_bg_default } else { colors.disabled_btn_bg });
        let label = fit_text(act_labels[i], bw.saturating_sub((6.0 * sc).round() as usize) / char_px);
        let tw = label.chars().count() * char_px;
        let tx = bx + bw.saturating_sub(tw) / 2;
        let ty = by + ((bh as f32 - 8.0 * sc) / 2.0).round() as usize;
        draw_text(buffer, tx, ty, width, &label, if enabled { colors.menu_text } else { colors.disabled_text }, sc);
    }

    if let Some(ed) = &l.editor {
        let (bx, by, bw, bh) = ed.box_pos;
        draw_rect(buffer, bx, by, bw, bh, width, colors.menu_bg);
        draw_rect(buffer, bx, by, bw, 1, width, colors.btn_border);
        draw_rect(buffer, bx, by + bh.saturating_sub(1), bw, 1, width, colors.btn_border);
        draw_rect(buffer, bx, by, 1, bh, width, colors.btn_border);
        draw_rect(buffer, bx + bw.saturating_sub(1), by, 1, bh, width, colors.btn_border);
        let label_ty = |y: usize, h: usize| y + ((h as f32 - 8.0 * sc) / 2.0).round() as usize;

        let addr_label_y = by + (2.0 * sc).round() as usize;
        let name_label_y = by + (19.0 * sc).round() as usize;
        draw_text(buffer, bx + (4.0 * sc).round() as usize, label_ty(addr_label_y, (13.0 * sc).round() as usize), width, "Address", colors.btn_sub_label, sc);
        draw_text(buffer, bx + (4.0 * sc).round() as usize, label_ty(name_label_y, (13.0 * sc).round() as usize), width, "Notes", colors.btn_sub_label, sc);

        let input = |buf: &mut [u32], r: (usize, usize, usize, usize), value: &str, focused: bool| {
            let (ix, iy, iw, ih) = r;
            draw_rect(buf, ix, iy, iw, ih, width, colors.btn_border);
            draw_rect(buf, ix + 1, iy + 1, iw.saturating_sub(2), ih.saturating_sub(2), width, colors.box_bg_default);
            let max_chars = iw.saturating_sub((8.0 * sc).round() as usize) / char_px;
            let shown: String = value.chars().skip(value.chars().count().saturating_sub(max_chars)).collect();
            let ty = iy + ((ih as f32 - 8.0 * sc) / 2.0).round() as usize;
            draw_text(buf, ix + (5.0 * sc).round() as usize, ty, width, &shown, colors.menu_text, sc);
            if focused {
                let caret_x = ix + (5.0 * sc).round() as usize + shown.chars().count() * char_px;
                draw_rect(buf, caret_x.min(ix + iw.saturating_sub(3)), iy + (3.0 * sc).round() as usize, (2.0 * sc).round() as usize, ih.saturating_sub((6.0 * sc).round() as usize), width, colors.menu_text);
            }
        };

        input(buffer, ed.addr_in, &st.in_addr, st.editor_focus == EDIT_FOCUS_ADDR);
        input(buffer, ed.name_in, &st.in_name, st.editor_focus == EDIT_FOCUS_NAME);

        let radio = |buf: &mut [u32], r: (usize, usize, usize, usize), text: &str, selected: bool| {
            let (rx, ry, rw, rh) = r;
            let hover = point_in_rect(mx, my, rx, ry, rw, rh);
            if hover {
                draw_rect(buf, rx, ry, rw, rh, width, colors.menu_highlight);
            }
            let border = if selected { colors.menu_text } else { colors.btn_border };
            draw_rect(buf, rx, ry, rw, 1, width, border);
            draw_rect(buf, rx, ry + rh.saturating_sub(1), rw, 1, width, border);
            draw_rect(buf, rx, ry, 1, rh, width, border);
            draw_rect(buf, rx + rw.saturating_sub(1), ry, 1, rh, width, border);
            let ty = ry + ((rh as f32 - 8.0 * sc) / 2.0).round() as usize;
            let shown = fit_text(text, rw.saturating_sub((6.0 * sc).round() as usize) / char_px);
            draw_text(buf, rx + (4.0 * sc).round() as usize, ty, width, &shown, colors.menu_text, sc);
            if selected {
                draw_rect(buf, rx + 2, ry + rh.saturating_sub(3), rw.saturating_sub(4), 1, width, colors.menu_text);
            }
        };

        let radio_y = by + (36.0 * sc).round() as usize;
        draw_text(buffer, bx + (4.0 * sc).round() as usize, label_ty(radio_y, (12.0 * sc).round() as usize), width, "Type", colors.btn_sub_label, sc);
        radio(buffer, ed.signed_opts[0], "Signed", st.edit_signed);
        radio(buffer, ed.signed_opts[1], "Unsign", !st.edit_signed);
        draw_text(buffer, bx + (150.0 * sc).round() as usize, label_ty(radio_y, (12.0 * sc).round() as usize), width, "Size", colors.btn_sub_label, sc);
        radio(buffer, ed.size_opts[0], "1B", st.edit_size == 1);
        radio(buffer, ed.size_opts[1], "2B", st.edit_size == 2);
        radio(buffer, ed.size_opts[2], "4B", st.edit_size == 4);

        let button = |buf: &mut [u32], r: (usize, usize, usize, usize), text: &str, hl: bool| {
            let (rx, ry, rw, rh) = r;
            let hover = point_in_rect(mx, my, rx, ry, rw, rh);
            draw_rect(buf, rx, ry, rw, rh, width, if hl { colors.menu_text } else { colors.btn_border });
            draw_rect(buf, rx + 1, ry + 1, rw.saturating_sub(2), rh.saturating_sub(2), width, if hover { colors.box_bg_hover } else { colors.box_bg_default });
            let label = fit_text(text, rw.saturating_sub((6.0 * sc).round() as usize) / char_px);
            let tw = label.chars().count() * char_px;
            let tx = rx + rw.saturating_sub(tw) / 2;
            let ty = ry + ((rh as f32 - 8.0 * sc) / 2.0).round() as usize;
            draw_text(buf, tx, ty, width, &label, colors.menu_text, sc);
        };
        button(buffer, ed.cancel, "Cancel", false);
        button(buffer, ed.ok, "OK", true);
    }

    let note_color = if st.note.is_empty() { colors.disabled_text } else { RED_TEXT };
    let note_text = if st.note.is_empty() { "Ready" } else { &st.note };
    let max_note = l.win_w.saturating_sub((16.0 * sc).round() as usize) / char_px;
    draw_text(buffer, l.win_x + (8.0 * sc).round() as usize, l.note_y, width, &fit_text(note_text, max_note), note_color, sc);
}

fn parse_addr(s: &str) -> Option<u16> {
    let mut t = s.trim();
    if let Some(rest) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        t = rest;
    }
    if let Some(rest) = t.strip_prefix('$') {
        t = rest;
    }
    if t.is_empty() || t.len() > 4 || !t.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u16::from_str_radix(t, 16).ok()
}

fn clamp_scroll(ms: &mut MenuState) {
    let count = RAM_WATCH_STATE.lock().unwrap().entries.len();
    let max = count.saturating_sub(LIST_ROWS);
    ms.ram_watch_scroll = ms.ram_watch_scroll.min(max);
    if let Some(s) = ms.ram_watch_selected {
        if s >= count {
            ms.ram_watch_selected = if count == 0 { None } else { Some(count - 1) };
        }
    }
}

fn commit_editor(ms: &mut MenuState) {
    let mut st = RAM_WATCH_STATE.lock().unwrap();
    if !st.editor_active {
        return;
    }
    let is_sep = match st.edit_index {
        Some(idx) => st.entries.get(idx).map_or(false, |e| e.is_sep),
        None => false,
    };
    let name_base = st.in_name.trim().to_string();
    if is_sep {
        let name = if name_base.is_empty() { "--------".to_string() } else { name_base };
        match st.edit_index {
            Some(idx) => {
                if let Some(e) = st.entries.get_mut(idx) {
                    e.name = name;
                }
            }
            None => {
                st.entries.push(new_separator(name));
                ms.ram_watch_selected = Some(st.entries.len() - 1);
            }
        }
    } else {
        let addr = match parse_addr(&st.in_addr) {
            Some(a) => a,
            None => {
                st.note = "Invalid address.".to_string();
                return;
            }
        };
        let name = if name_base.is_empty() { format!("{:04X}", addr) } else { name_base };
        let size = st.edit_size;
        let signed = st.edit_signed;
        match st.edit_index {
            Some(idx) => {
                if let Some(e) = st.entries.get_mut(idx) {
                    e.addr = addr;
                    e.name = name;
                    e.size = size;
                    e.signed = signed;
                    e.is_sep = false;
                }
            }
            None => {
                st.entries.push(new_entry(name, addr, size, signed));
                ms.ram_watch_selected = Some(st.entries.len() - 1);
            }
        }
    }
    st.editor_active = false;
    st.edit_index = None;
    st.note.clear();
    drop(st);
    clamp_scroll(ms);
}

fn cancel_editor() {
    let mut st = RAM_WATCH_STATE.lock().unwrap();
    st.editor_active = false;
    st.edit_index = None;
    st.note.clear();
}

pub(crate) fn handle_ram_watch_scroll(ms: &mut MenuState, amount: i32) {
    let count = RAM_WATCH_STATE.lock().unwrap().entries.len();
    let max = count.saturating_sub(LIST_ROWS);
    let cur = ms.ram_watch_scroll as i32 - amount;
    ms.ram_watch_scroll = cur.max(0).min(max as i32).max(0) as usize;
}

pub(crate) fn handle_ram_watch_key(ms: &mut MenuState, _emu: &mut Emulator, keycode: winit::event::VirtualKeyCode) {
    let editing = RAM_WATCH_STATE.lock().unwrap().editor_active;
    match keycode {
        winit::event::VirtualKeyCode::Escape => {
            if editing {
                cancel_editor();
            } else {
                ms.show_ram_watch_window = false;
            }
        }
        winit::event::VirtualKeyCode::Return => {
            if editing {
                commit_editor(ms);
            } else if let Some(idx) = ms.ram_watch_selected {
                let mut st = RAM_WATCH_STATE.lock().unwrap();
                let Some(e) = st.entries.get(idx).cloned() else { return };
                st.editor_active = true;
                st.edit_index = Some(idx);
                st.in_addr = format!("{:04X}", e.addr);
                st.in_name = e.name.clone();
                st.addr_caret = st.in_addr.chars().count();
                st.name_caret = st.in_name.chars().count();
                st.editor_focus = EDIT_FOCUS_ADDR;
                st.edit_size = if e.is_sep { 1 } else { e.size };
                st.edit_signed = e.signed;
                st.note.clear();
            }
        }
        winit::event::VirtualKeyCode::Tab => {
            if editing {
                let mut st = RAM_WATCH_STATE.lock().unwrap();
                st.editor_focus = (st.editor_focus + 1) % 2;
            }
        }
        winit::event::VirtualKeyCode::Delete => {
            if editing {
                let mut st = RAM_WATCH_STATE.lock().unwrap();
                if st.editor_focus == EDIT_FOCUS_ADDR {
                    let caret = st.addr_caret;
                    if caret < st.in_addr.chars().count() {
                        st.in_addr.remove(caret);
                    }
                    st.addr_caret = st.addr_caret.min(st.in_addr.chars().count());
                } else {
                    let caret = st.name_caret;
                    if caret < st.in_name.chars().count() {
                        st.in_name.remove(caret);
                    }
                    st.name_caret = st.name_caret.min(st.in_name.chars().count());
                }
            } else if let Some(idx) = ms.ram_watch_selected {
                let mut st = RAM_WATCH_STATE.lock().unwrap();
                if idx < st.entries.len() {
                    st.entries.remove(idx);
                }
                let count = st.entries.len();
                ms.ram_watch_selected = if count == 0 { None } else { Some(idx.min(count - 1)) };
                drop(st);
                clamp_scroll(ms);
            }
        }
        winit::event::VirtualKeyCode::Back => {
            if editing {
                let mut st = RAM_WATCH_STATE.lock().unwrap();
                if st.editor_focus == EDIT_FOCUS_ADDR {
                    let caret = st.addr_caret;
                    if caret > 0 {
                        st.in_addr.remove(caret - 1);
                    }
                    st.addr_caret = st.addr_caret.saturating_sub(1);
                } else {
                    let caret = st.name_caret;
                    if caret > 0 {
                        st.in_name.remove(caret - 1);
                    }
                    st.name_caret = st.name_caret.saturating_sub(1);
                }
            }
        }
        winit::event::VirtualKeyCode::Left => {
            if editing {
                let mut st = RAM_WATCH_STATE.lock().unwrap();
                if st.editor_focus == EDIT_FOCUS_ADDR {
                    st.addr_caret = st.addr_caret.saturating_sub(1);
                } else {
                    st.name_caret = st.name_caret.saturating_sub(1);
                }
            }
        }
        winit::event::VirtualKeyCode::Right => {
            if editing {
                let mut st = RAM_WATCH_STATE.lock().unwrap();
                if st.editor_focus == EDIT_FOCUS_ADDR {
                    st.addr_caret = (st.addr_caret + 1).min(st.in_addr.chars().count());
                } else {
                    st.name_caret = (st.name_caret + 1).min(st.in_name.chars().count());
                }
            }
        }
        winit::event::VirtualKeyCode::Up | winit::event::VirtualKeyCode::Down => {
            if !editing {
                let count = RAM_WATCH_STATE.lock().unwrap().entries.len();
                if count == 0 {
                    return;
                }
                let sel = match ms.ram_watch_selected {
                    Some(s) => s,
                    None => {
                        ms.ram_watch_selected = Some(0);
                        0
                    }
                };
                let new_sel = if keycode == winit::event::VirtualKeyCode::Up {
                    sel.saturating_sub(1)
                } else {
                    (sel + 1).min(count - 1)
                };
                ms.ram_watch_selected = Some(new_sel);
                if new_sel < ms.ram_watch_scroll {
                    ms.ram_watch_scroll = new_sel;
                } else if new_sel >= ms.ram_watch_scroll + LIST_ROWS {
                    ms.ram_watch_scroll = new_sel + 1 - LIST_ROWS;
                }
            }
        }
        winit::event::VirtualKeyCode::PageUp => {
            if !editing {
                handle_ram_watch_scroll(ms, 10);
            }
        }
        winit::event::VirtualKeyCode::PageDown => {
            if !editing {
                handle_ram_watch_scroll(ms, -10);
            }
        }
        _ => {}
    }
}

pub(crate) fn handle_ram_watch_char(_ms: &mut MenuState, c: char) {
    let mut st = RAM_WATCH_STATE.lock().unwrap();
    if !st.editor_active {
        return;
    }
    if st.editor_focus == EDIT_FOCUS_ADDR {
        if !(c.is_ascii_hexdigit() || c == '$' || c == 'x' || c == 'X') {
            return;
        }
        if st.in_addr.chars().count() >= MAX_ADDR_LEN {
            return;
        }
        let pos = st.addr_caret.min(st.in_addr.len());
        st.in_addr.insert(pos, c);
        st.addr_caret = (st.addr_caret + 1).min(st.in_addr.len());
    } else {
        if c.is_control() {
            return;
        }
        if st.in_name.chars().count() >= MAX_NAME_LEN {
            return;
        }
        let pos = st.name_caret.min(st.in_name.len());
        st.in_name.insert(pos, c);
        st.name_caret = (st.name_caret + 1).min(st.in_name.len());
    }
}

pub(crate) fn handle_ram_watch_click(
    ms: &mut MenuState,
    emu: &mut Emulator,
    button: winit::event::MouseButton,
    mx: usize,
    my: usize,
    width: usize,
    height: usize,
    scale: f32,
) {
    if button != winit::event::MouseButton::Left {
        return;
    }
    let editing = RAM_WATCH_STATE.lock().unwrap().editor_active;
    let l = compute_ram_watch_layout(width, height, scale, editing);

    let (cx, cy, cw, ch) = l.close;
    if point_in_rect(mx, my, cx, cy, cw, ch) {
        ms.show_ram_watch_window = false;
        return;
    }

    for i in 0..5 {
        let (bx, by, bw, bh) = l.toolbar[i];
        if point_in_rect(mx, my, bx, by, bw, bh) {
            match i {
                TOOL_NEWLIST => {
                    let mut st = RAM_WATCH_STATE.lock().unwrap();
                    st.entries.clear();
                    st.editor_active = false;
                    st.note = "List cleared.".to_string();
                    ms.ram_watch_selected = None;
                    ms.ram_watch_scroll = 0;
                }
                TOOL_OPEN | TOOL_APPEND => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("FCEUX RAM Watch", &["wch"])
                        .add_filter("All files", &["*"])
                        .pick_file()
                    {
                        let path_str = path.to_string_lossy().to_string();
                        match load_watches(&path_str, i == TOOL_APPEND) {
                            Ok(n) => {
                                let mut st = RAM_WATCH_STATE.lock().unwrap();
                                st.filename = Some(path_str);
                                st.note = format!("{} {} watches.", if i == TOOL_APPEND { "Appended" } else { "Loaded" }, n);
                                if i == TOOL_OPEN {
                                    ms.ram_watch_selected = None;
                                    ms.ram_watch_scroll = 0;
                                }
                            }
                            Err(e) => {
                                let mut st = RAM_WATCH_STATE.lock().unwrap();
                                st.note = e;
                            }
                        }
                    }
                }
                TOOL_SAVE => {
                    let target = RAM_WATCH_STATE.lock().unwrap().filename.clone();
                    match target {
                        Some(path) => match save_watches(&path) {
                            Ok(_) => {
                                let mut st = RAM_WATCH_STATE.lock().unwrap();
                                st.note = "Saved.".to_string();
                            }
                            Err(e) => {
                                let mut st = RAM_WATCH_STATE.lock().unwrap();
                                st.note = e;
                            }
                        },
                        None => save_watches_as(),
                    }
                }
                TOOL_SAVEAS => save_watches_as(),
                _ => {}
            }
            return;
        }
    }

    let (lx, ly, lw, lh) = l.list;
    if point_in_rect(mx, my, lx, ly, lw, lh) && my >= ly + l.header_h {
        let count = RAM_WATCH_STATE.lock().unwrap().entries.len();
        let row = (my - ly - l.header_h) / l.row_h;
        let item = ms.ram_watch_scroll.min(count.saturating_sub(l.rows_visible)) + row;
        ms.ram_watch_selected = if item < count { Some(item) } else { None };
        return;
    }

    if let Some(ed) = &l.editor {
        if point_in_rect(mx, my, ed.addr_in.0, ed.addr_in.1, ed.addr_in.2, ed.addr_in.3) {
            let mut st = RAM_WATCH_STATE.lock().unwrap();
            st.editor_focus = EDIT_FOCUS_ADDR;
            return;
        }
        if point_in_rect(mx, my, ed.name_in.0, ed.name_in.1, ed.name_in.2, ed.name_in.3) {
            let mut st = RAM_WATCH_STATE.lock().unwrap();
            st.editor_focus = EDIT_FOCUS_NAME;
            return;
        }
        for i in 0..2 {
            let (rx, ry, rw, rh) = ed.signed_opts[i];
            if point_in_rect(mx, my, rx, ry, rw, rh) {
                let mut st = RAM_WATCH_STATE.lock().unwrap();
                st.edit_signed = i == 0;
                return;
            }
        }
        for i in 0..3 {
            let (rx, ry, rw, rh) = ed.size_opts[i];
            if point_in_rect(mx, my, rx, ry, rw, rh) {
                let mut st = RAM_WATCH_STATE.lock().unwrap();
                st.edit_size = [1u8, 2, 4][i];
                return;
            }
        }
        if point_in_rect(mx, my, ed.ok.0, ed.ok.1, ed.ok.2, ed.ok.3) {
            commit_editor(ms);
            return;
        }
        if point_in_rect(mx, my, ed.cancel.0, ed.cancel.1, ed.cancel.2, ed.cancel.3) {
            cancel_editor();
            return;
        }
    }

    let act_ids = [ACT_UP, ACT_NEW, ACT_REMOVE, ACT_SEPARATOR, ACT_DOWN, ACT_EDIT, ACT_DUPLICATE, ACT_CHEAT];
    for i in 0..8 {
        let (bx, by, bw, bh) = l.act[i];
        if point_in_rect(mx, my, bx, by, bw, bh) {
            match act_ids[i] {
                ACT_UP | ACT_DOWN => {
                    let Some(idx) = ms.ram_watch_selected else { return };
                    let delta = if act_ids[i] == ACT_UP { -1i64 } else { 1 };
                    move_entry(ms, idx, delta);
                }
                ACT_NEW => {
                    let mut st = RAM_WATCH_STATE.lock().unwrap();
                    st.editor_active = true;
                    st.edit_index = None;
                    st.in_addr.clear();
                    st.in_name.clear();
                    st.addr_caret = 0;
                    st.name_caret = 0;
                    st.editor_focus = EDIT_FOCUS_ADDR;
                    st.edit_size = 1;
                    st.edit_signed = false;
                    st.note.clear();
                }
                ACT_REMOVE => {
                    let Some(idx) = ms.ram_watch_selected else { return };
                    let mut st = RAM_WATCH_STATE.lock().unwrap();
                    if idx < st.entries.len() {
                        st.entries.remove(idx);
                    }
                    let count = st.entries.len();
                    ms.ram_watch_selected = if count == 0 { None } else { Some(idx.min(count - 1)) };
                    drop(st);
                    clamp_scroll(ms);
                }
                ACT_DUPLICATE => {
                    let Some(idx) = ms.ram_watch_selected else { return };
                    let mut st = RAM_WATCH_STATE.lock().unwrap();
                    let Some(e) = st.entries.get(idx).cloned() else { return };
                    let mut c = e.clone();
                    c.name = format!("{} (copy)", c.name);
                    st.entries.insert(idx + 1, c);
                    ms.ram_watch_selected = Some(idx + 1);
                    st.note.clear();
                }
                ACT_EDIT => {
                    let Some(idx) = ms.ram_watch_selected else { return };
                    let mut st = RAM_WATCH_STATE.lock().unwrap();
                    let Some(e) = st.entries.get(idx).cloned() else { return };
                    st.editor_active = true;
                    st.edit_index = Some(idx);
                    st.in_addr = format!("{:04X}", e.addr);
                    st.in_name = e.name.clone();
                    st.addr_caret = st.in_addr.chars().count();
                    st.name_caret = st.in_name.chars().count();
                    st.editor_focus = EDIT_FOCUS_ADDR;
                    st.edit_size = if e.is_sep { 1 } else { e.size };
                    st.edit_signed = e.signed;
                    st.note.clear();
                }
                ACT_SEPARATOR => {
                    let mut st = RAM_WATCH_STATE.lock().unwrap();
                    let idx = match ms.ram_watch_selected {
                        Some(i) => (i + 1).min(st.entries.len()),
                        None => st.entries.len(),
                    };
                    st.entries.insert(idx, new_separator("--------".to_string()));
                    ms.ram_watch_selected = Some(idx);
                    st.editor_active = true;
                    st.edit_index = Some(idx);
                    st.in_addr.clear();
                    st.in_name = "--------".to_string();
                    st.addr_caret = 0;
                    st.name_caret = st.in_name.chars().count();
                    st.editor_focus = EDIT_FOCUS_NAME;
                    st.note.clear();
                }
                ACT_CHEAT => {
                    let Some(idx) = ms.ram_watch_selected else { return };
                    let entry = {
                        let st = RAM_WATCH_STATE.lock().unwrap();
                        st.entries.get(idx).cloned()
                    };
                    let Some(entry) = entry else { return };
                    if entry.is_sep {
                        return;
                    }
                    for b in 0..entry.size {
                        let addr = entry.addr.wrapping_add(b as u16);
                        let v = emu.debug_peek_mem(addr);
                        emu.cheats.add_cheat(CheatEntry::new(
                            format!("{:04X}", addr),
                            CheatType::Custom,
                            format!("{:04X}:{:02X}", addr, v),
                        ));
                    }
                    let mut st = RAM_WATCH_STATE.lock().unwrap();
                    st.note = format!("Added cheat at {:04X}", entry.addr);
                }
                _ => {}
            }
            return;
        }
    }
}

fn move_entry(ms: &mut MenuState, idx: usize, delta: i64) {
    let mut st = RAM_WATCH_STATE.lock().unwrap();
    let len = st.entries.len();
    if len < 2 || idx >= len {
        return;
    }
    let target = (idx as i64 + delta).clamp(0, len as i64 - 1) as usize;
    if target == idx {
        return;
    }
    st.entries.swap(idx, target);
    ms.ram_watch_selected = Some(target);
}

fn save_watches_as() {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("FCEUX RAM Watch", &["wch"])
        .set_file_name("ramwatch.wch")
        .save_file()
    {
        let path_str = path.to_string_lossy().to_string();
        match save_watches(&path_str) {
            Ok(_) => {
                let mut st = RAM_WATCH_STATE.lock().unwrap();
                st.filename = Some(path_str);
                st.note = "Saved.".to_string();
            }
            Err(e) => {
                let mut st = RAM_WATCH_STATE.lock().unwrap();
                st.note = e;
            }
        }
    }
}

fn escape_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if c == '"' {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn save_watches(path: &str) -> Result<(), String> {
    let st = RAM_WATCH_STATE.lock().unwrap();
    let mut s = String::new();
    s.push('\n');
    s.push_str(&st.entries.len().to_string());
    s.push('\n');
    for (i, e) in st.entries.iter().enumerate() {
        let (size_ch, type_ch, addr) = if e.is_sep {
            ('S', 'S', 0u16)
        } else {
            let size_ch = match e.size {
                1 => 'b',
                2 => 'w',
                _ => 'd',
            };
            let type_ch = if e.signed { 's' } else { 'u' };
            (size_ch, type_ch, e.addr)
        };
        s.push_str(&format!("{:05} {:04X}  {}  {}  0  {}\n", i, addr, size_ch, type_ch, escape_name(&e.name)));
    }
    std::fs::write(path, s).map_err(|e| format!("Failed to save: {}", e))
}

fn cut_comment(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut literal = false;
    for c in line.chars() {
        if literal {
            literal = false;
            out.push(c);
            continue;
        }
        match c {
            '\\' => literal = true,
            '#' => break,
            _ => out.push(c),
        }
    }
    out
}

fn load_watches(path: &str, append: bool) -> Result<usize, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("Failed to load: {}", e))?;
    let mut st = RAM_WATCH_STATE.lock().unwrap();
    if !append {
        st.entries.clear();
        st.editor_active = false;
        st.edit_index = None;
    }
    let mut added = 0usize;
    for raw in text.lines() {
        let line = cut_comment(raw);
        let b = line.as_bytes();
        let mut i = 0;
        while i < b.len() && (b[i] as char).is_whitespace() {
            i += 1;
        }
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        while i < b.len() && (b[i] as char).is_whitespace() {
            i += 1;
        }
        if i + 1 < b.len() && b[i] == b'0' && (b[i + 1] == b'x' || b[i + 1] == b'X') {
            i += 2;
        }
        let mut addr: u32 = 0;
        let mut any = false;
        while i < b.len() && b[i].is_ascii_hexdigit() {
            any = true;
            addr = addr * 16 + (b[i] as char).to_digit(16).unwrap_or(0);
            i += 1;
        }
        if !any {
            continue;
        }
        while i < b.len() && (b[i] as char).is_whitespace() {
            i += 1;
        }
        if i >= b.len() {
            continue;
        }
        let size_ch = (b[i] as char).to_ascii_lowercase();
        i += 1;
        while i < b.len() && (b[i] as char).is_whitespace() {
            i += 1;
        }
        if i >= b.len() {
            continue;
        }
        let type_ch = (b[i] as char).to_ascii_lowercase();
        i += 1;
        if !matches!(type_ch, 'u' | 's' | 'h' | 'b') {
            continue;
        }
        let mut size = 1u8;
        let mut sep = false;
        match size_ch {
            's' => {
                sep = true;
            }
            'b' => size = 1,
            'w' => size = 2,
            'd' => size = 4,
            c if c.is_ascii_digit() => {
                size = (c as u8 - b'0') as u8;
            }
            _ => continue,
        }
        if !sep && size != 1 && size != 2 && size != 4 {
            continue;
        }
        while i < b.len() && (b[i] as char).is_whitespace() {
            i += 1;
        }
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        while i < b.len() && (b[i] as char).is_whitespace() {
            i += 1;
        }
        let name = if i < b.len() && b[i] == b'"' {
            i += 1;
            let mut n = String::new();
            let mut literal = false;
            while i < b.len() {
                let cb = b[i];
                if cb == b'\\' && !literal {
                    literal = true;
                    i += 1;
                    continue;
                }
                if cb == b'"' && !literal {
                    break;
                }
                n.push(cb as char);
                literal = false;
                i += 1;
            }
            n
        } else {
            line[i..].trim_end().to_string()
        };
        let name = name.trim_end().to_string();
        if sep {
            st.entries.push(new_separator(if name.is_empty() { "--------".to_string() } else { name }));
        } else {
            st.entries.push(new_entry(name, (addr & 0xFFFF) as u16, size, type_ch == 's'));
        }
        added += 1;
    }
    Ok(added)
}