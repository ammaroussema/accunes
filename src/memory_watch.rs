// memory watching tool!!!
use std::sync::Mutex;

use crate::emulator::Emulator;
use crate::{draw_rect, draw_text, point_in_rect, MenuState, UiColors};

const FROZEN_TEXT: u32 = 0xFF4488CC;

const SLOTS_PER_COL: usize = 12;
const NUM_SLOTS: usize = 24;
const MONITORS: usize = 4;
const MAX_ADDR_LEN: usize = 5;
const MAX_LABEL_LEN: usize = 64;

const FOCUS_ADDR: usize = 0;
const FOCUS_NAME: usize = 1;

const TOOL_NEW: usize = 0;
const TOOL_OPEN: usize = 1;
const TOOL_SAVE: usize = 2;
const TOOL_SAVEAS: usize = 3;
const TOOL_COLLAPSE: usize = 4;

const MONITOR_LINK: [usize; MONITORS] = [0, 1, 12, 13];

const FORMULA_LABELS: [&str; 6] = ["> than", "> by 1", "< than", "< by 1", "equal", "!equal"];

#[derive(Clone)]
struct Slot {
    addr_text: String,
    label: String,
    valid: bool,
    hex: bool,
    twobytes: bool,
    addr: u16,
}

impl Slot {
    const fn empty() -> Self {
        Slot {
            addr_text: String::new(),
            label: String::new(),
            valid: false,
            hex: false,
            twobytes: false,
            addr: 0,
        }
    }
}

#[derive(Clone)]
struct Monitor {
    formula: u8,
    count: u32,
    started: bool,
    mon_addr: u16,
    last: u8,
}

struct MemoryWatchState {
    slots: [Slot; NUM_SLOTS],
    monitors: [Monitor; MONITORS],
    collapsed: bool,
    dirty: bool,
    last_file: Option<String>,
    focus: Option<(usize, usize)>,
    caret: usize,
}

static MEMORY_WATCH_STATE: Mutex<MemoryWatchState> = Mutex::new(initial_state());

pub(crate) fn open_window() {
}

fn parse_slot(slot: &mut Slot) {
    slot.valid = false;
    slot.hex = false;
    slot.twobytes = false;
    let mut rest: &str = slot.addr_text.trim();
    if let Some(r) = rest.strip_prefix('!') {
        slot.twobytes = true;
        rest = r;
    } else if let Some(r) = rest.strip_prefix('x') {
        slot.hex = true;
        rest = r;
    } else if let Some(r) = rest.strip_prefix('X') {
        slot.hex = true;
        slot.twobytes = true;
        rest = r;
    }
    let mut addr: u32 = 0;
    let mut any = false;
    for c in rest.chars() {
        if c.is_ascii_hexdigit() {
            any = true;
            addr = addr * 16 + c.to_digit(16).unwrap_or(0);
        } else {
            break;
        }
    }
    if any && addr <= 0xFFFF {
        slot.valid = true;
        slot.addr = addr as u16;
    }
}

fn formula_holds(f: u8, now: u8, prev: u8) -> bool {
    match f {
        1 => now.wrapping_sub(prev) == 1,
        2 => now < prev,
        3 => prev.wrapping_sub(now) == 1,
        4 => now == prev,
        5 => now != prev,
        _ => now > prev,
    }
}

pub(crate) fn on_frame(emu: &mut Emulator) {
    let mut st = MEMORY_WATCH_STATE.lock().unwrap();
    for x in 0..MONITORS {
        let si = MONITOR_LINK[x];
        let slot = &st.slots[si];
        if !slot.valid {
            st.monitors[x].started = false;
            continue;
        }
        let addr = slot.addr;
        let cur = emu.debug_peek_mem(addr);
        let m = &mut st.monitors[x];
        if !m.started || m.mon_addr != addr {
            m.started = true;
            m.mon_addr = addr;
            m.last = cur;
        } else {
            let prev = m.last;
            if formula_holds(m.formula, cur, prev) {
                m.count = m.count.saturating_add(1);
            }
            m.last = cur;
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CellRects {
    addr: (usize, usize, usize, usize),
    name: (usize, usize, usize, usize),
    value_x: usize,
    value_y: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct MonitorRects {
    box_pos: (usize, usize, usize, usize),
    addr: [(usize, usize); 2],
    formula: [(usize, usize, usize, usize); 2],
    count: [(usize, usize); 2],
    reset: [(usize, usize, usize, usize); 2],
    header_y: usize,
}

pub(crate) struct MemoryWatchLayout {
    pub win_x: usize,
    pub win_y: usize,
    pub win_w: usize,
    pub win_h: usize,
    pub title_h: usize,
    pub close: (usize, usize, usize, usize),
    pub toolbar: [(usize, usize, usize, usize); 5],
    pub cells: [[CellRects; SLOTS_PER_COL]; 2],
    pub monitors: [MonitorRects; 2],
    pub groups: [(usize, usize, usize, usize); 2],
    pub group_w: usize,
    pub group_gap: usize,
    pub header_y: usize,
    pub col_addr_x: usize,
    pub col_name_x: usize,
    pub col_value_x: usize,
    pub mon_caption_y: usize,
}

const fn initial_state() -> MemoryWatchState {
    MemoryWatchState {
        slots: [
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
            Slot::empty(),
        ],
        monitors: [
            Monitor {
                formula: 0,
                count: 0,
                started: false,
                mon_addr: 0,
                last: 0,
            },
            Monitor {
                formula: 0,
                count: 0,
                started: false,
                mon_addr: 0,
                last: 0,
            },
            Monitor {
                formula: 0,
                count: 0,
                started: false,
                mon_addr: 0,
                last: 0,
            },
            Monitor {
                formula: 0,
                count: 0,
                started: false,
                mon_addr: 0,
                last: 0,
            },
        ],
        collapsed: false,
        dirty: false,
        last_file: None,
        focus: None,
        caret: 0,
    }
}

fn memory_watch_fit_factor(scale: f32, width: usize, height: usize) -> f32 {
    let cap_w = width.saturating_sub(16) as f32;
    let cap_h = height.saturating_sub(36) as f32;
    let measure = |fit: f32| -> (f32, f32) {
        let s = scale * fit;
        let r = |v: f32| -> f32 { (v * s * 2.0).round() / 2.0 };
        let w = r(12.0) + r(200.0) + r(14.0) + r(200.0) + r(12.0);
        let h = r(26.0)
            + r(6.0)
            + r(18.0)
            + r(6.0)
            + r(264.0)
            + r(4.0)
            + r(90.0)
            + r(12.0);
        (w, h)
    };
    let est = ((cap_w / (438.0 * scale)).min(cap_h / (426.0 * scale))).min(1.0).max(0.2);
    let (w0, h0) = measure(est);
    let f1 = est * (cap_w / w0.max(1.0)).min(cap_h / h0.max(1.0));
    let f1 = f1.min(1.0).max(0.2);
    let (w1, h1) = measure(f1);
    let f2 = f1 * (cap_w / w1.max(1.0)).min(cap_h / h1.max(1.0));
    f2.min(f1 * 1.02).min(1.0).max(0.2) * 0.96
}

pub(crate) fn compute_memory_watch_layout(width: usize, height: usize, scale: f32, collapsed: bool) -> MemoryWatchLayout {
    let fit = memory_watch_fit_factor(scale, width, height);
    let sc = scale * fit;
    let u = |v: f32| -> usize { (v * sc).round() as usize };

    let pad = u(12.0);
    let title_h = u(26.0);

    let bar_y = title_h + u(6.0);
    let mut toolbar = [(0usize, 0usize, 0usize, 0usize); 5];
    for i in 0..5 {
        toolbar[i] = (pad + i * (u(76.0) + u(4.0)), bar_y, u(76.0), u(18.0));
    }

    let box_w = u(200.0);
    let box_gap = u(14.0);
    let box_y = bar_y + u(18.0) + u(6.0);
    let box_h = u(264.0);

    let row0_y = box_y + u(22.0);
    let row_h = u(19.0);
    let cell_w_addr = u(54.0);
    let cell_w_name = u(88.0);
    let cell_h = u(15.0);
    let addr_x = u(4.0);
    let name_x = u(66.0);
    let col_value_x = u(158.0);
    let header_y = box_y + u(8.0);

    let win_w = pad
        + (box_w + box_gap)
        + if collapsed { 0 } else { box_w }
        + pad;
    let toolbar_w = pad + 5 * u(76.0) + 4 * u(4.0) + pad;
    let win_w = if collapsed { toolbar_w.max(win_w) } else { win_w };
    let mon_y = box_y + box_h + u(4.0);
    let mon_h = u(90.0);
    let win_h = mon_y + mon_h + pad;
    let mon_caption_y = mon_y + u(4.0);
    let mh_y = mon_y + u(20.0);
    let mrow0 = mon_y + u(34.0);
    let mrow1 = mon_y + u(58.0);
    let mon_addr_x = u(8.0);
    let mon_formula_x = u(48.0);
    let mon_formula_w = u(64.0);
    let mon_count_x = u(124.0);
    let mon_reset_x = u(166.0);
    let mon_reset_w = u(14.0);
    let mon_cell_h = u(15.0);
    let mon_box_w = win_w.saturating_sub(pad + pad);
    let win_x = (width.saturating_sub(win_w)) / 2;
    let win_y = (height.saturating_sub(win_h)) / 2;

    let mut cells = [[CellRects {
        addr: (0, 0, 0, 0),
        name: (0, 0, 0, 0),
        value_x: 0,
        value_y: 0,
    }; SLOTS_PER_COL]; 2];
    for col in 0..2 {
        let gx = pad + col * (box_w + box_gap);
        for r in 0..SLOTS_PER_COL {
            let ry = row0_y + r * row_h;
            cells[col][r] = CellRects {
                addr: (gx + addr_x, ry, cell_w_addr, cell_h),
                name: (gx + name_x, ry, cell_w_name, cell_h),
                value_x: gx + col_value_x,
                value_y: ry + ((cell_h as f32 - 8.0 * sc) / 2.0).round() as usize,
            };
        }
    }

    let mon_cell = |rows: [usize; 2]| -> MonitorRects {
        let gx = 0usize;
        MonitorRects {
            box_pos: (gx, mon_y, mon_box_w, mon_h),
            addr: [(mon_addr_x, rows[0]), (mon_addr_x, rows[1])],
            formula: [
                (mon_formula_x, rows[0], mon_formula_w, mon_cell_h),
                (mon_formula_x, rows[1], mon_formula_w, mon_cell_h),
            ],
            count: [(mon_count_x, rows[0]), (mon_count_x, rows[1])],
            reset: [
                (mon_reset_x, rows[0], mon_reset_w, mon_cell_h),
                (mon_reset_x, rows[1], mon_reset_w, mon_cell_h),
            ],
            header_y: mh_y,
        }
    };
    let raw_monitors = [mon_cell([mrow0, mrow1]), mon_cell([mrow0, mrow1])];

    let close_w = u(20.0);
    let close_h = u(20.0);
    let close_x = win_x + win_w.saturating_sub(close_w + u(6.0));
    let close_y = win_y + u(2.0);

    let mut groups = [(0usize, 0usize, 0usize, 0usize); 2];
    for col in 0..2 {
        let gx = pad + col * (box_w + box_gap);
        groups[col] = (win_x + gx, win_y + box_y, box_w, box_h);
    }

    let mut m_toolbar = [(0usize, 0usize, 0usize, 0usize); 5];
    for i in 0..5 {
        let (bx, by, bw, bh) = toolbar[i];
        m_toolbar[i] = (win_x + bx, win_y + by, bw, bh);
    }
    let mut m_cells = [[CellRects {
        addr: (0, 0, 0, 0),
        name: (0, 0, 0, 0),
        value_x: 0,
        value_y: 0,
    }; SLOTS_PER_COL]; 2];
    for col in 0..2 {
        for r in 0..SLOTS_PER_COL {
            let c = cells[col][r];
            m_cells[col][r] = CellRects {
                addr: (win_x + c.addr.0, win_y + c.addr.1, c.addr.2, c.addr.3),
                name: (win_x + c.name.0, win_y + c.name.1, c.name.2, c.name.3),
                value_x: win_x + c.value_x,
                value_y: win_y + c.value_y,
            };
        }
    }
    let mut m_monitors = [
        MonitorRects {
            box_pos: (0, 0, 0, 0),
            addr: [(0, 0); 2],
            formula: [(0, 0, 0, 0); 2],
            count: [(0, 0); 2],
            reset: [(0, 0, 0, 0); 2],
            header_y: 0,
        },
        MonitorRects {
            box_pos: (0, 0, 0, 0),
            addr: [(0, 0); 2],
            formula: [(0, 0, 0, 0); 2],
            count: [(0, 0); 2],
            reset: [(0, 0, 0, 0); 2],
            header_y: 0,
        },
    ];
    for col in 0..2 {
        let m0 = raw_monitors[col];
        let hx = pad + col * (box_w + box_gap);
        let mbox = m0.box_pos;
        m_monitors[col] = MonitorRects {
            box_pos: (win_x + pad, win_y + mbox.1, mbox.2, mbox.3),
            addr: [
                (win_x + hx + m0.addr[0].0, win_y + m0.addr[0].1),
                (win_x + hx + m0.addr[1].0, win_y + m0.addr[1].1),
            ],
            formula: [
                (win_x + hx + m0.formula[0].0, win_y + m0.formula[0].1, m0.formula[0].2, m0.formula[0].3),
                (win_x + hx + m0.formula[1].0, win_y + m0.formula[1].1, m0.formula[1].2, m0.formula[1].3),
            ],
            count: [
                (win_x + hx + m0.count[0].0, win_y + m0.count[0].1),
                (win_x + hx + m0.count[1].0, win_y + m0.count[1].1),
            ],
            reset: [
                (win_x + hx + m0.reset[0].0, win_y + m0.reset[0].1, m0.reset[0].2, m0.reset[0].3),
                (win_x + hx + m0.reset[1].0, win_y + m0.reset[1].1, m0.reset[1].2, m0.reset[1].3),
            ],
            header_y: win_y + m0.header_y,
        };
    }

    MemoryWatchLayout {
        win_x,
        win_y,
        win_w,
        win_h,
        title_h,
        close: (close_x, close_y, close_w, close_h),
        toolbar: m_toolbar,
        cells: m_cells,
        monitors: m_monitors,
        groups,
group_w: box_w,
        group_gap: box_gap,
        header_y: win_y + header_y,
        col_addr_x: win_x + pad + addr_x,
        col_name_x: win_x + pad + name_x,
        col_value_x: win_x + pad + col_value_x,
        mon_caption_y: win_y + mon_caption_y,
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

pub(crate) fn render_memory_watch_window(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    ms: &MenuState,
    colors: &UiColors,
    emu: &mut Emulator,
    scale: f32,
) {
    let st = MEMORY_WATCH_STATE.lock().unwrap();
    let l = compute_memory_watch_layout(width, height, scale, st.collapsed);
    let sc = scale * memory_watch_fit_factor(scale, width, height);
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
    draw_text(buffer, l.win_x + (10.0 * sc).round() as usize, title_ty, width, "Memory Watch", colors.menu_text, sc);

    let (cx, cy, cw, ch) = l.close;
    let close_hover = point_in_rect(mx, my, cx, cy, cw, ch);
    draw_rect(buffer, cx, cy, cw, ch, width, if close_hover { colors.menu_highlight } else { colors.close_bg });
    let close_ty = cy + ((ch as f32 - 8.0 * sc) / 2.0).round() as usize;
    draw_text(buffer, cx + (6.0 * sc).round() as usize, close_ty, width, "X", colors.menu_text, sc);

    let toolbar_labels = ["New", "Open", "Save", "Save As", if st.collapsed { ">>" } else { "<<" }];
    let toolbar_enabled = [true, true, st.last_file.is_some(), true, true];
    for i in 0..5 {
        let (bx, by, bw, bh) = l.toolbar[i];
        let enabled = toolbar_enabled[i];
        let hover = enabled && point_in_rect(mx, my, bx, by, bw, bh);
        draw_rect(buffer, bx, by, bw, bh, width, if enabled { colors.btn_border } else { colors.disabled_btn_bg });
        draw_rect(buffer, bx + 1, by + 1, bw.saturating_sub(2), bh.saturating_sub(2), width, if hover { colors.box_bg_hover } else if enabled { colors.box_bg_default } else { colors.disabled_btn_bg });
        let label = fit_text(toolbar_labels[i], bw.saturating_sub((6.0 * sc).round() as usize) / char_px.max(1));
        let tw = label.chars().count() * char_px;
        let tx = bx + bw.saturating_sub(tw) / 2;
        let ty = by + ((bh as f32 - 8.0 * sc) / 2.0).round() as usize;
        draw_text(buffer, tx, ty, width, &label, if enabled { colors.menu_text } else { colors.disabled_text }, sc);
    }

    let visible_cols: usize = if st.collapsed { 1 } else { 2 };

    for col in 0..visible_cols {
        let (box_x, box_y, box_w, box_h) = l.groups[col];
        draw_rect(buffer, box_x, box_y, box_w, box_h, width, colors.box_border);

        let header_ty = l.header_y;
        let col_off = col * (l.group_w + l.group_gap);
        draw_text(buffer, l.col_addr_x + col_off, header_ty, width, "Address", colors.btn_sub_label, sc);
        draw_text(buffer, l.col_name_x + col_off, header_ty, width, "Name", colors.btn_sub_label, sc);
        draw_text(buffer, l.col_value_x + col_off, header_ty, width, "Value", colors.btn_sub_label, sc);

        for r in 0..SLOTS_PER_COL {
            let cell = l.cells[col][r];
            let si = col * SLOTS_PER_COL + r;
            let slot = &st.slots[si];
            let focused = st.focus == Some((si, FOCUS_ADDR));

            let (ax, ay, aw, ah) = cell.addr;
            let a_hover = point_in_rect(mx, my, ax, ay, aw, ah);
            draw_rect(buffer, ax, ay, aw, ah, width, colors.btn_border);
            draw_rect(buffer, ax + 1, ay + 1, aw.saturating_sub(2), ah.saturating_sub(2), width, if focused { colors.box_bg_hover } else if a_hover { colors.menu_bg } else { colors.box_bg_default });
            let a_ty = ay + ((ah as f32 - 8.0 * sc) / 2.0).round() as usize;
            let shown = fit_text(&slot.addr_text, aw.saturating_sub((6.0 * sc).round() as usize) / char_px.max(1));
            draw_text(buffer, ax + (4.0 * sc).round() as usize, a_ty, width, &shown, colors.menu_text, sc);
            if focused {
                let caret_x = ax + (4.0 * sc).round() as usize + st.caret * char_px;
                draw_rect(buffer, caret_x.min(ax + aw.saturating_sub(3)), ay + (3.0 * sc).round() as usize, (2.0 * sc).round() as usize, ah.saturating_sub((6.0 * sc).round() as usize), width, colors.menu_text);
            }

            let (nx, ny, nw, nh) = cell.name;
            let n_hover = point_in_rect(mx, my, nx, ny, nw, nh);
            let name_focused = st.focus == Some((si, FOCUS_NAME));
            draw_rect(buffer, nx, ny, nw, nh, width, colors.btn_border);
            draw_rect(buffer, nx + 1, ny + 1, nw.saturating_sub(2), nh.saturating_sub(2), width, if name_focused { colors.box_bg_hover } else if n_hover { colors.menu_bg } else { colors.box_bg_default });
            let n_ty = ny + ((nh as f32 - 8.0 * sc) / 2.0).round() as usize;
            let nshown = fit_text(&slot.label, nw.saturating_sub((6.0 * sc).round() as usize) / char_px.max(1));
            draw_text(buffer, nx + (4.0 * sc).round() as usize, n_ty, width, &nshown, colors.menu_text, sc);
            if name_focused {
                let caret_x = nx + (4.0 * sc).round() as usize + st.caret * char_px;
                draw_rect(buffer, caret_x.min(nx + nw.saturating_sub(3)), ny + (3.0 * sc).round() as usize, (2.0 * sc).round() as usize, nh.saturating_sub((6.0 * sc).round() as usize), width, colors.menu_text);
            }

            if slot.valid {
                let lo = emu.debug_peek_mem(slot.addr);
                let frozen = emu.cheats.apply(slot.addr, lo) != lo;
                let v = if slot.twobytes {
                    let hi = emu.debug_peek_mem(slot.addr.wrapping_add(1));
                    (lo as u32) | ((hi as u32) << 8)
                } else {
                    lo as u32
                };
                let text = if slot.hex {
                    if slot.twobytes {
                        format!("{:04X}", v as u16)
                    } else {
                        format!("{:02X}", v as u8)
                    }
                } else if slot.twobytes {
                    (v as u16).to_string()
                } else {
                    (v as u8).to_string()
                };
                let color = if frozen { FROZEN_TEXT } else { colors.menu_text };
                draw_text(buffer, cell.value_x, cell.value_y, width, &fit_text(&text, 5), color, sc);
            } else {
                draw_text(buffer, cell.value_x, cell.value_y, width, "-", colors.disabled_text, sc);
            }
        }
    }

    for col in 0..visible_cols {
        let m = l.monitors[col];
        if col == 0 {
            let (gpx, gpy, gpw, gph) = m.box_pos;
            draw_rect(buffer, gpx, gpy, gpw, gph, width, colors.box_border);
            draw_text(buffer, gpx + (4.0 * sc).round() as usize, l.mon_caption_y, width, "Memory Change Monitoring:", colors.btn_sub_label, sc);
        }
        let mh_y = m.header_y;
        draw_text(buffer, m.addr[0].0, mh_y, width, "Addr", colors.btn_sub_label, sc);
        draw_text(buffer, m.formula[0].0, mh_y, width, "Formula", colors.btn_sub_label, sc);
        draw_text(buffer, m.count[0].0, mh_y, width, "Count", colors.btn_sub_label, sc);
        draw_text(buffer, m.reset[0].0, mh_y, width, "R", colors.btn_sub_label, sc);

        let mbase = if col == 0 { 0 } else { MONITORS / 2 };
        let links = [MONITOR_LINK[mbase], MONITOR_LINK[mbase + 1]];
        for row in 0..2 {
            let si = links[row];
            let slot = &st.slots[si];
            let slot_text = slot.addr_text.trim().to_string();

            let (fbx, fby, fbw, fbh) = m.formula[row];
            let row_text_y = fby + ((fbh as f32 - 8.0 * sc) / 2.0).round() as usize;
            draw_text(buffer, m.addr[row].0, row_text_y, width, &fit_text(&slot_text, 4), colors.menu_text, sc);

            let f_hover = point_in_rect(mx, my, fbx, fby, fbw, fbh);
            draw_rect(buffer, fbx, fby, fbw, fbh, width, colors.btn_border);
            draw_rect(buffer, fbx + 1, fby + 1, fbw.saturating_sub(2), fbh.saturating_sub(2), width, if f_hover { colors.box_bg_hover } else { colors.box_bg_default });
            let ftext = FORMULA_LABELS[st.monitors[mbase + row].formula as usize % FORMULA_LABELS.len()];
            draw_text(buffer, fbx + (3.0 * sc).round() as usize, row_text_y, width, ftext, colors.menu_text, sc);

            let count_text = st.monitors[mbase + row].count.to_string();
            draw_text(buffer, m.count[row].0, row_text_y, width, &count_text, colors.menu_text, sc);

            let (rbx, rby, rbw, rbh) = m.reset[row];
            let r_hover = point_in_rect(mx, my, rbx, rby, rbw, rbh);
            draw_rect(buffer, rbx, rby, rbw, rbh, width, colors.btn_border);
            draw_rect(buffer, rbx + 1, rby + 1, rbw.saturating_sub(2), rbh.saturating_sub(2), width, if r_hover { colors.box_bg_hover } else { colors.box_bg_default });
            let r_ty = rby + ((rbh as f32 - 8.0 * sc) / 2.0).round() as usize;
            let r_x = rbx + (rbw.saturating_sub(char_px)) / 2;
            draw_text(buffer, r_x, r_ty, width, "R", colors.menu_text, sc);
        }
    }
}

fn clamp_caret(st: &mut MemoryWatchState) {
    if let Some((si, field)) = st.focus {
        let len = if field == FOCUS_ADDR { st.slots[si].addr_text.chars().count() } else { st.slots[si].label.chars().count() };
        st.caret = st.caret.min(len);
    }
}

fn set_caret_to_end(st: &mut MemoryWatchState) {
    if let Some((si, field)) = st.focus {
        st.caret = if field == FOCUS_ADDR { st.slots[si].addr_text.chars().count() } else { st.slots[si].label.chars().count() };
    }
}

pub(crate) fn handle_memory_watch_click(
    ms: &mut MenuState,
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
    let mut st = MEMORY_WATCH_STATE.lock().unwrap();
    let l = compute_memory_watch_layout(width, height, scale, st.collapsed);

    let (cx, cy, cw, ch) = l.close;
    if point_in_rect(mx, my, cx, cy, cw, ch) {
        ms.show_memory_watch_window = false;
        return;
    }

    for i in 0..5 {
        let (bx, by, bw, bh) = l.toolbar[i];
        if point_in_rect(mx, my, bx, by, bw, bh) {
            drop(st);
            match i {
                TOOL_NEW => new_list(ms),
                TOOL_OPEN => open_list(ms),
                TOOL_SAVE => save_current(ms),
                TOOL_SAVEAS => save_as(ms),
                TOOL_COLLAPSE => {
                    let mut st = MEMORY_WATCH_STATE.lock().unwrap();
                    st.collapsed = !st.collapsed;
                }
                _ => {}
            }
            return;
        }
    }

    let visible_cols: usize = if st.collapsed { 1 } else { 2 };
    for col in 0..visible_cols {
        for r in 0..SLOTS_PER_COL {
            let cell = l.cells[col][r];
            let si = col * SLOTS_PER_COL + r;
            if point_in_rect(mx, my, cell.addr.0, cell.addr.1, cell.addr.2, cell.addr.3) {
                st.focus = Some((si, FOCUS_ADDR));
                set_caret_to_end(&mut st);
                st.dirty = true;
                return;
            }
            if point_in_rect(mx, my, cell.name.0, cell.name.1, cell.name.2, cell.name.3) {
                st.focus = Some((si, FOCUS_NAME));
                set_caret_to_end(&mut st);
                st.dirty = true;
                return;
            }
        }
    }

    for col in 0..visible_cols {
        let m = l.monitors[col];
        let mbase = if col == 0 { 0 } else { MONITORS / 2 };
        if !point_in_rect(mx, my, m.box_pos.0, m.box_pos.1, m.box_pos.2, m.box_pos.3) {
            continue;
        }
        for row in 0..2 {
            let fb = m.formula[row];
            if point_in_rect(mx, my, fb.0, fb.1, fb.2, fb.3) {
                let mon = &mut st.monitors[mbase + row];
                mon.formula = ((mon.formula as usize + 1) % FORMULA_LABELS.len()) as u8;
                return;
            }
            let rb = m.reset[row];
            if point_in_rect(mx, my, rb.0, rb.1, rb.2, rb.3) {
                st.monitors[mbase + row].count = 0;
                return;
            }
        }
    }

    if !point_in_rect(mx, my, l.win_x, l.win_y, l.win_w, l.win_h) || point_in_rect(mx, my, l.win_x + 1, l.win_y + 1, l.win_w.saturating_sub(2), l.title_h) {
        st.focus = None;
    }
}

fn new_list(ms: &mut MenuState) {
    let mut st = MEMORY_WATCH_STATE.lock().unwrap();
    for s in st.slots.iter_mut() {
        s.addr_text.clear();
        s.label.clear();
        s.valid = false;
        s.hex = false;
        s.twobytes = false;
        s.addr = 0;
    }
    st.focus = None;
    st.caret = 0;
    st.dirty = false;
    st.last_file = None;
    let _ = ms;
}

fn open_list(ms: &mut MenuState) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Memory watch list", &["txt"])
        .set_file_name("memorywatch.txt")
        .pick_file()
    {
        let path_str = path.to_string_lossy().to_string();
        match load_list(&path_str) {
            Ok(_) => {
                let mut st = MEMORY_WATCH_STATE.lock().unwrap();
                st.last_file = Some(path_str);
                st.dirty = false;
                st.focus = None;
            }
            Err(e) => {
                ms.show_error = true;
                ms.error_message = e;
            }
        }
    }
}

fn save_current(ms: &mut MenuState) {
    let path = {
        let st = MEMORY_WATCH_STATE.lock().unwrap();
        st.last_file.clone()
    };
    match path {
        Some(p) => match save_list(&p) {
            Ok(_) => {
                let mut st = MEMORY_WATCH_STATE.lock().unwrap();
                st.dirty = false;
            }
            Err(e) => {
                ms.show_error = true;
                ms.error_message = e;
            }
        },
        None => save_as(ms),
    }
}

fn save_as(ms: &mut MenuState) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Memory watch list", &["txt"])
        .set_file_name("memorywatch.txt")
        .save_file()
    {
        let path_str = path.to_string_lossy().to_string();
        match save_list(&path_str) {
            Ok(_) => {
                let mut st = MEMORY_WATCH_STATE.lock().unwrap();
                st.last_file = Some(path_str);
                st.dirty = false;
            }
            Err(e) => {
                ms.show_error = true;
                ms.error_message = e;
            }
        }
    }
}

fn save_list(path: &str) -> Result<(), String> {
    let st = MEMORY_WATCH_STATE.lock().unwrap();
    let mut s = String::new();
    for slot in st.slots.iter() {
        let a = if slot.addr_text.is_empty() {
            "|".to_string()
        } else {
            slot.addr_text.replace(' ', "|")
        };
        let n = if slot.label.is_empty() {
            "|".to_string()
        } else {
            slot.label.replace(' ', "|")
        };
        s.push_str(&a);
        s.push(' ');
        s.push_str(&n);
        s.push('\n');
    }
    std::fs::write(path, s).map_err(|e| format!("Failed to save: {}", e))
}

fn load_list(path: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("Failed to load: {}", e))?;
    let mut st = MEMORY_WATCH_STATE.lock().unwrap();
    for s in st.slots.iter_mut() {
        s.addr_text.clear();
        s.label.clear();
        s.addr = 0;
        s.hex = false;
        s.twobytes = false;
        s.valid = false;
    }
    for (i, line) in text.lines().enumerate() {
        if i >= NUM_SLOTS {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, ' ');
        let a = parts.next().unwrap_or("|").replace('|', " ");
        let n = parts.next().unwrap_or("|").replace('|', " ");
        let slot = &mut st.slots[i];
        slot.addr_text = a.trim().to_string();
        slot.label = n.trim().to_string();
        parse_slot(slot);
    }
    Ok(())
}

fn char_byte_pos(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map(|(b, _)| b).unwrap_or(s.len())
}

fn key_move_caret(st: &mut MemoryWatchState, delta: i64) {
    if let Some((si, field)) = st.focus {
        let len = if field == FOCUS_ADDR { st.slots[si].addr_text.chars().count() } else { st.slots[si].label.chars().count() };
        st.caret = ((st.caret as i64 + delta).max(0).min(len as i64)) as usize;
    }
}

fn tab_next(st: &mut MemoryWatchState) {
    let next = match st.focus {
        Some((si, field)) if field == FOCUS_ADDR => Some((si, FOCUS_NAME)),
        Some((si, field)) if field == FOCUS_NAME => {
            if si + 1 < NUM_SLOTS {
                Some((si + 1, FOCUS_ADDR))
            } else {
                None
            }
        }
        Some(_) => None,
        None => Some((0, FOCUS_ADDR)),
    };
    st.focus = next;
    set_caret_to_end(st);
}

pub(crate) fn handle_memory_watch_key(_ms: &mut MenuState, keycode: winit::event::VirtualKeyCode) {
    let mut st = MEMORY_WATCH_STATE.lock().unwrap();
    match keycode {
        winit::event::VirtualKeyCode::Tab => {
            tab_next(&mut st);
            st.dirty = true;
        }
        winit::event::VirtualKeyCode::Left => {
            if st.focus.is_some() {
                key_move_caret(&mut st, -1);
            }
        }
        winit::event::VirtualKeyCode::Right => {
            if st.focus.is_some() {
                key_move_caret(&mut st, 1);
            }
        }
        winit::event::VirtualKeyCode::Home => st.caret = 0,
        winit::event::VirtualKeyCode::End => set_caret_to_end(&mut st),
        winit::event::VirtualKeyCode::Back => {
            if let Some((si, field)) = st.focus {
                let cur = st.caret;
                if cur > 0 {
                    if field == FOCUS_ADDR {
                        let bi = char_byte_pos(&st.slots[si].addr_text, cur - 1);
                        let ch = st.slots[si].addr_text[bi..].chars().next().unwrap();
                        st.slots[si].addr_text.replace_range(bi..bi + ch.len_utf8(), "");
                        parse_slot(&mut st.slots[si]);
                    } else {
                        let bi = char_byte_pos(&st.slots[si].label, cur - 1);
                        let ch = st.slots[si].label[bi..].chars().next().unwrap();
                        st.slots[si].label.replace_range(bi..bi + ch.len_utf8(), "");
                    }
                    st.caret = cur - 1;
                    st.dirty = true;
                }
            }
        }
        winit::event::VirtualKeyCode::Delete => {
            if let Some((si, field)) = st.focus {
                let cur = st.caret;
                if field == FOCUS_ADDR {
                    if cur < st.slots[si].addr_text.chars().count() {
                        let bi = char_byte_pos(&st.slots[si].addr_text, cur);
                        let ch = st.slots[si].addr_text[bi..].chars().next().unwrap();
                        st.slots[si].addr_text.replace_range(bi..bi + ch.len_utf8(), "");
                        parse_slot(&mut st.slots[si]);
                        st.dirty = true;
                    }
                } else if cur < st.slots[si].label.chars().count() {
                    let bi = char_byte_pos(&st.slots[si].label, cur);
                    let ch = st.slots[si].label[bi..].chars().next().unwrap();
                    st.slots[si].label.replace_range(bi..bi + ch.len_utf8(), "");
                    st.dirty = true;
                }
            }
        }
        winit::event::VirtualKeyCode::Return => {
            st.focus = None;
        }
        winit::event::VirtualKeyCode::Escape => {
            st.focus = None;
        }
        _ => {}
    }
    clamp_caret(&mut st);
}

pub(crate) fn handle_memory_watch_char(_ms: &mut MenuState, c: char) {
    let mut st = MEMORY_WATCH_STATE.lock().unwrap();
    let Some((si, field)) = st.focus else {
        return;
    };
    if field == FOCUS_ADDR {
        if !(c.is_ascii_hexdigit() || c == '!' || c == 'x' || c == 'X') {
            return;
        }
        if st.slots[si].addr_text.chars().count() >= MAX_ADDR_LEN {
            return;
        }
        let pos = char_byte_pos(&st.slots[si].addr_text, st.caret);
        st.slots[si].addr_text.insert(pos, c);
        st.caret = (st.caret + 1).min(st.slots[si].addr_text.chars().count());
        parse_slot(&mut st.slots[si]);
        st.dirty = true;
    } else {
        if c.is_control() {
            return;
        }
        if st.slots[si].label.chars().count() >= MAX_LABEL_LEN {
            return;
        }
        let pos = char_byte_pos(&st.slots[si].label, st.caret);
        st.slots[si].label.insert(pos, c);
        st.caret = (st.caret + 1).min(st.slots[si].label.chars().count());
        st.dirty = true;
    }
}