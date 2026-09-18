// nametables viewer!!!
use crate::emulator::Emulator;
use crate::{draw_rect, draw_text, point_in_rect, UiColors, MenuState};
use winit::event::VirtualKeyCode;

const ATTRIBUTE_VIEW_TILE: [u8; 16] = [
    0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F,
    0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF,
];

const IGNORE_PALETTE: [u8; 4] = [0x0F, 0x00, 0x10, 0x20];

pub(crate) struct NametableLayout {
    pub win_x: usize,
    pub win_y: usize,
    pub win_w: usize,
    pub win_h: usize,
    pub close_x: usize,
    pub close_y: usize,
    pub close_w: usize,
    pub close_h: usize,
    pub canvas_x: usize,
    pub canvas_y: usize,
    pub canvas_w: usize,
    pub canvas_h: usize,
    pub tile_size: usize,
    pub scroll_btn: (usize, usize, usize, usize),
    pub grid_btn: (usize, usize, usize, usize),
    pub attr_grid_btn: (usize, usize, usize, usize),
    pub attr_view_btn: (usize, usize, usize, usize),
    pub pal_btn: (usize, usize, usize, usize),
    pub pattern_btn: (usize, usize, usize, usize),
    pub auto_btn: (usize, usize, usize, usize),
    pub step_btn: (usize, usize, usize, usize),
    pub status_x: usize,
    pub status_y: usize,
    pub status_w: usize,
    pub status_h: usize,
}

pub(crate) fn compute_nametable_layout(width: usize, height: usize, scale: f32) -> NametableLayout {
    let sc = scale;
    let pad_x = (8.0 * sc).round() as usize;
    let title_h = (24.0 * sc).round() as usize;
    let close_w = (20.0 * sc).round() as usize;
    let close_h = (20.0 * sc).round() as usize;
    let btn_h = (22.0 * sc).round() as usize;
    let btn_gap = (6.0 * sc).round() as usize;

    let (canvas_w, canvas_h, tile_size) = if width >= 560 && height >= 620 {
        (512, 480, 8)
    } else {
        (256, 240, 4)
    };

    let scroll_w = (98.0 * sc).round() as usize;
    let grid_w = (82.0 * sc).round() as usize;
    let attr_grid_w = (122.0 * sc).round() as usize;
    let attr_view_w = (122.0 * sc).round() as usize;
    let row1_w = scroll_w + grid_w + attr_grid_w + attr_view_w + btn_gap * 3;

    let pal_w = (90.0 * sc).round() as usize;
    let pattern_w = (124.0 * sc).round() as usize;
    let auto_w = (132.0 * sc).round() as usize;
    let step_w = (78.0 * sc).round() as usize;
    let row2_w = pal_w + pattern_w + auto_w + step_w + btn_gap * 3;

    let content_w = canvas_w.max(row1_w).max(row2_w);
    let win_w = (content_w + pad_x * 2).min(width.saturating_sub(16));

    let row1_y_rel = title_h + (4.0 * sc).round() as usize;
    let row2_y_rel = row1_y_rel + btn_h + (4.0 * sc).round() as usize;
    let canvas_y_rel = row2_y_rel + btn_h + (6.0 * sc).round() as usize;
    let status_y_rel = canvas_y_rel + canvas_h + (6.0 * sc).round() as usize;
    let status_h = (44.0 * sc).round() as usize;

    let win_h = status_y_rel + status_h + (8.0 * sc).round() as usize;
    let win_x = (width.saturating_sub(win_w)) / 2;
    let win_y = (height.saturating_sub(win_h)) / 2;

    let close_x = win_x + win_w - close_w - (6.0 * sc).round() as usize;
    let close_y = win_y + (2.0 * sc).round() as usize;

    let mut r1_x = win_x + (win_w.saturating_sub(row1_w)) / 2;
    let scroll_btn = (r1_x, win_y + row1_y_rel, scroll_w, btn_h);
    r1_x += scroll_w + btn_gap;
    let grid_btn = (r1_x, win_y + row1_y_rel, grid_w, btn_h);
    r1_x += grid_w + btn_gap;
    let attr_grid_btn = (r1_x, win_y + row1_y_rel, attr_grid_w, btn_h);
    r1_x += attr_grid_w + btn_gap;
    let attr_view_btn = (r1_x, win_y + row1_y_rel, attr_view_w, btn_h);

    let mut r2_x = win_x + (win_w.saturating_sub(row2_w)) / 2;
    let pal_btn = (r2_x, win_y + row2_y_rel, pal_w, btn_h);
    r2_x += pal_w + btn_gap;
    let pattern_btn = (r2_x, win_y + row2_y_rel, pattern_w, btn_h);
    r2_x += pattern_w + btn_gap;
    let auto_btn = (r2_x, win_y + row2_y_rel, auto_w, btn_h);
    r2_x += auto_w + btn_gap;
    let step_btn = (r2_x, win_y + row2_y_rel, step_w, btn_h);

    let canvas_x = win_x + (win_w.saturating_sub(canvas_w)) / 2;
    let canvas_y = win_y + canvas_y_rel;

    let status_x = win_x + pad_x;
    let status_y = win_y + status_y_rel;
    let status_w = win_w.saturating_sub(pad_x * 2);

    NametableLayout {
        win_x,
        win_y,
        win_w,
        win_h,
        close_x,
        close_y,
        close_w,
        close_h,
        canvas_x,
        canvas_y,
        canvas_w,
        canvas_h,
        tile_size,
        scroll_btn,
        grid_btn,
        attr_grid_btn,
        attr_view_btn,
        pal_btn,
        pattern_btn,
        auto_btn,
        step_btn,
        status_x,
        status_y,
        status_w,
        status_h,
    }
}

pub(crate) fn render_nametable_viewer_window(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    ms: &MenuState,
    colors: &UiColors,
    emu: &mut Emulator,
    scale: f32,
) {
    let l = compute_nametable_layout(width, height, scale);
    let sc = scale;
    let (mx, my) = ms.mouse_pos;

    draw_rect(buffer, l.win_x, l.win_y, l.win_w, l.win_h, width, colors.window_bg);
    draw_rect(buffer, l.win_x, l.win_y, l.win_w, 1, width, colors.window_border);
    draw_rect(buffer, l.win_x, l.win_y + l.win_h.saturating_sub(1), l.win_w, 1, width, colors.window_border);
    draw_rect(buffer, l.win_x, l.win_y, 1, l.win_h, width, colors.window_border);
    draw_rect(buffer, l.win_x + l.win_w.saturating_sub(1), l.win_y, 1, l.win_h, width, colors.window_border);

    let title_h = (24.0 * sc).round() as usize;
    draw_rect(buffer, l.win_x + 1, l.win_y + 1, l.win_w.saturating_sub(2), title_h, width, colors.dropdown_bg);
    draw_rect(buffer, l.win_x, l.win_y + title_h + 1, l.win_w, 1, width, colors.window_border);

    let title_ty = l.win_y + ((title_h as f32 - 8.0 * sc) / 2.0).round() as usize;
    draw_text(buffer, l.win_x + (10.0 * sc).round() as usize, title_ty, width, "Nametable Viewer", colors.menu_text, scale);

    let close_hover = point_in_rect(mx, my, l.close_x, l.close_y, l.close_w, l.close_h);
    let close_bg = if close_hover { colors.menu_highlight } else { colors.close_bg };
    draw_rect(buffer, l.close_x, l.close_y, l.close_w, l.close_h, width, close_bg);
    draw_rect(buffer, l.close_x, l.close_y, l.close_w, 1, width, colors.btn_border);
    draw_rect(buffer, l.close_x, l.close_y + l.close_h.saturating_sub(1), l.close_w, 1, width, colors.btn_border);
    draw_rect(buffer, l.close_x, l.close_y, 1, l.close_h, width, colors.btn_border);
    draw_rect(buffer, l.close_x + l.close_w.saturating_sub(1), l.close_y, 1, l.close_h, width, colors.btn_border);
    let close_ty = l.close_y + ((l.close_h as f32 - 8.0 * sc) / 2.0).round() as usize;
    draw_text(buffer, l.close_x + (6.0 * sc).round() as usize, close_ty, width, "X", colors.menu_text, scale);

    let render_btn = |buf: &mut [u32], rect: (usize, usize, usize, usize), text: &str| {
        let (bx, by, bw, bh) = rect;
        let hover = point_in_rect(mx, my, bx, by, bw, bh);
        let bg = if hover { colors.box_bg_hover } else { colors.box_bg_default };
        draw_rect(buf, bx, by, bw, bh, width, bg);
        draw_rect(buf, bx, by, bw, 1, width, colors.btn_border);
        draw_rect(buf, bx, by + bh.saturating_sub(1), bw, 1, width, colors.btn_border);
        draw_rect(buf, bx, by, 1, bh, width, colors.btn_border);
        draw_rect(buf, bx + bw.saturating_sub(1), by, 1, bh, width, colors.btn_border);
        let text_w = (text.len() as f32 * 8.0 * sc).round() as usize;
        let tx = bx + (bw.saturating_sub(text_w)) / 2;
        let font_h = (8.0 * sc).round() as usize;
        let ty = by + (bh.saturating_sub(font_h)) / 2;
        draw_text(buf, tx, ty, width, text, colors.menu_text, scale);
    };

    let scroll_text = if ms.nt_viewer_show_scroll { "Scroll: ON" } else { "Scroll: OFF" };
    render_btn(buffer, l.scroll_btn, scroll_text);

    let grid_text = if ms.nt_viewer_show_grid { "Grid: ON" } else { "Grid: OFF" };
    render_btn(buffer, l.grid_btn, grid_text);

    let attr_grid_text = if ms.nt_viewer_show_attr_grid { "Attr Grid: ON" } else { "Attr Grid: OFF" };
    render_btn(buffer, l.attr_grid_btn, attr_grid_text);

    let attr_view_text = if ms.nt_viewer_attr_view { "Attr View: ON" } else { "Attr View: OFF" };
    render_btn(buffer, l.attr_view_btn, attr_view_text);

    let pal_text = if ms.nt_viewer_ignore_pal { "Pal: Force" } else { "Pal: Real" };
    render_btn(buffer, l.pal_btn, pal_text);

    let pat_text = match ms.nt_viewer_pattern_mode {
        1 => "Pattern: $0000",
        2 => "Pattern: $1000",
        _ => "Pattern: Auto",
    };
    render_btn(buffer, l.pattern_btn, pat_text);

    let auto_text = if ms.nt_viewer_auto_refresh { "Refresh: Live" } else { "Refresh: Freeze" };
    render_btn(buffer, l.auto_btn, auto_text);

    render_btn(buffer, l.step_btn, "Refresh");

    let mut nts = [[0u8; 1024]; 4];
    emu.debug_peek_nametables(&mut nts);

    let mut table0 = [0u8; 4096];
    let mut table1 = [0u8; 4096];
    emu.debug_peek_chr_tables(&mut table0, &mut table1);
    let pal_ram = emu.debug_peek_palette_ram();

    let bg_ptable = if emu.ppu_pattern_select_background { 1 } else { 0 };
    let active_ptable_idx = match ms.nt_viewer_pattern_mode {
        1 => 0,
        2 => 1,
        _ => bg_ptable,
    };
    let chr_table = if active_ptable_idx == 0 { &table0 } else { &table1 };

    draw_rect(buffer, l.canvas_x.saturating_sub(1), l.canvas_y.saturating_sub(1), l.canvas_w + 2, l.canvas_h + 2, width, colors.box_border);

    let half_w = 32 * l.tile_size;
    let half_h = 30 * l.tile_size;

    for nt in 0..4 {
        let nt_col = nt % 2;
        let nt_row = nt / 2;
        let nt_origin_x = l.canvas_x + nt_col * half_w;
        let nt_origin_y = l.canvas_y + nt_row * half_h;

        for ty in 0..30 {
            for tx in 0..32 {
                let nt_addr = ty * 32 + tx;
                let tile_id = nts[nt][nt_addr];
                let attr_addr = 0x3C0 + ((ty / 4) * 8) + (tx / 4);
                let raw_attr = nts[nt][attr_addr];
                let shift = ((ty & 2) << 1) | (tx & 2);
                let pal_idx = (raw_attr >> shift) & 0x03;

                let tile_chr: [u8; 16] = if ms.nt_viewer_attr_view {
                    ATTRIBUTE_VIEW_TILE
                } else {
                    let mut b = [0u8; 16];
                    let o = tile_id as usize * 16;
                    b.copy_from_slice(&chr_table[o..o + 16]);
                    b
                };

                for py in 0..8 {
                    let chr0 = tile_chr[py];
                    let chr1 = tile_chr[py + 8];
                    for px in 0..8 {
                        let bit = 7 - px;
                        let p = ((chr0 >> bit) & 1) | (((chr1 >> bit) & 1) << 1);
                        let nes_color = if ms.nt_viewer_ignore_pal {
                            IGNORE_PALETTE[p as usize]
                        } else if p == 0 {
                            pal_ram[0]
                        } else {
                            pal_ram[pal_idx as usize * 4 + p as usize]
                        };
                        let rgb = emu.palette_lut[(nes_color & 0x3F) as usize];

                        if l.tile_size == 8 {
                            let dx = nt_origin_x + tx * 8 + px;
                            let dy = nt_origin_y + ty * 8 + py;
                            if dy < height && dx < width {
                                buffer[dy * width + dx] = rgb;
                            }
                        } else {
                            let dx = nt_origin_x + tx * 4 + (px / 2);
                            let dy = nt_origin_y + ty * 4 + (py / 2);
                            if dy < height && dx < width {
                                buffer[dy * width + dx] = rgb;
                            }
                        }
                    }
                }
            }
        }
    }

    if ms.nt_viewer_show_grid {
        for t in 1..64 {
            let gx = l.canvas_x + t * l.tile_size;
            if gx < l.canvas_x + l.canvas_w {
                for y in l.canvas_y..(l.canvas_y + l.canvas_h) {
                    if y < height && gx < width {
                        buffer[y * width + gx] = 0x00333333;
                    }
                }
            }
        }
        for t in 1..60 {
            let gy = l.canvas_y + t * l.tile_size;
            if gy < l.canvas_y + l.canvas_h {
                for x in l.canvas_x..(l.canvas_x + l.canvas_w) {
                    if gy < height && x < width {
                        buffer[gy * width + x] = 0x00333333;
                    }
                }
            }
        }
    }

    if ms.nt_viewer_show_attr_grid {
        let attr_step = l.tile_size * 2;
        for t in 1..32 {
            let gx = l.canvas_x + t * attr_step;
            if gx < l.canvas_x + l.canvas_w {
                for y in l.canvas_y..(l.canvas_y + l.canvas_h) {
                    if y < height && gx < width {
                        buffer[y * width + gx] = 0x003A5A8A;
                    }
                }
            }
        }
        for t in 1..30 {
            let gy = l.canvas_y + t * attr_step;
            if gy < l.canvas_y + l.canvas_h {
                for x in l.canvas_x..(l.canvas_x + l.canvas_w) {
                    if gy < height && x < width {
                        buffer[gy * width + x] = 0x003A5A8A;
                    }
                }
            }
        }
    }

    let mid_x = l.canvas_x + half_w;
    let mid_y = l.canvas_y + half_h;
    for y in l.canvas_y..(l.canvas_y + l.canvas_h) {
        if y < height && mid_x < width {
            buffer[y * width + mid_x] = colors.window_border;
        }
    }
    for x in l.canvas_x..(l.canvas_x + l.canvas_w) {
        if mid_y < height && x < width {
            buffer[mid_y * width + x] = colors.window_border;
        }
    }

    let (raw_scroll_x, raw_scroll_y) = emu.debug_get_scroll();
    let scroll_x = raw_scroll_x % 512;
    let scroll_y = raw_scroll_y % 480;

    if ms.nt_viewer_show_scroll {
        let scale_mul = l.tile_size;
        let scale_div = 8;
        let vx = (scroll_x * scale_mul) / scale_div;
        let vy = (scroll_y * scale_mul) / scale_div;
        let vw = (256 * scale_mul) / scale_div;
        let vh = (240 * scale_mul) / scale_div;
        let outline_color = 0x00FF3333;

        let draw_line_h = |buf: &mut [u32], x0: usize, x1: usize, y: usize| {
            if y >= l.canvas_y && y < l.canvas_y + l.canvas_h && y < height {
                for x in x0..=x1 {
                    if x >= l.canvas_x && x < l.canvas_x + l.canvas_w && x < width {
                        buf[y * width + x] = outline_color;
                    }
                }
            }
        };

        let draw_line_v = |buf: &mut [u32], x: usize, y0: usize, y1: usize| {
            if x >= l.canvas_x && x < l.canvas_x + l.canvas_w && x < width {
                for y in y0..=y1 {
                    if y >= l.canvas_y && y < l.canvas_y + l.canvas_h && y < height {
                        buf[y * width + x] = outline_color;
                    }
                }
            }
        };

        let draw_sub_rect = |buf: &mut [u32], rx: usize, ry: usize, rw: usize, rh: usize| {
            if rw == 0 || rh == 0 { return; }
            let x0 = l.canvas_x + rx;
            let x1 = l.canvas_x + rx + rw.saturating_sub(1);
            let y0 = l.canvas_y + ry;
            let y1 = l.canvas_y + ry + rh.saturating_sub(1);
            draw_line_h(buf, x0, x1, y0);
            draw_line_h(buf, x0, x1, y1);
            draw_line_v(buf, x0, y0, y1);
            draw_line_v(buf, x1, y0, y1);
        };

        let w1 = vw.min(l.canvas_w.saturating_sub(vx));
        let h1 = vh.min(l.canvas_h.saturating_sub(vy));
        draw_sub_rect(buffer, vx, vy, w1, h1);

        if vx + vw > l.canvas_w {
            let w2 = (vx + vw) - l.canvas_w;
            draw_sub_rect(buffer, 0, vy, w2, h1);
            if vy + vh > l.canvas_h {
                let h2 = (vy + vh) - l.canvas_h;
                draw_sub_rect(buffer, 0, 0, w2, h2);
            }
        }

        if vy + vh > l.canvas_h {
            let h2 = (vy + vh) - l.canvas_h;
            draw_sub_rect(buffer, vx, 0, w1, h2);
        }
    }

    let mut hover_tile = None;
    if mx >= l.canvas_x && mx < l.canvas_x + l.canvas_w && my >= l.canvas_y && my < l.canvas_y + l.canvas_h {
        let rel_x = mx - l.canvas_x;
        let rel_y = my - l.canvas_y;
        let nt_col = rel_x / half_w;
        let nt_row = rel_y / half_h;
        let nt = nt_row * 2 + nt_col;
        let tx = (rel_x % half_w) / l.tile_size;
        let ty = (rel_y % half_h) / l.tile_size;
        if nt < 4 && tx < 32 && ty < 30 {
            hover_tile = Some((nt, tx, ty));
        }
    }

    let active_tile = hover_tile.or(ms.nt_viewer_selected_tile);

    if let Some((nt, tx, ty)) = active_tile {
        let nt_col = nt % 2;
        let nt_row = nt / 2;
        let hx = l.canvas_x + nt_col * half_w + tx * l.tile_size;
        let hy = l.canvas_y + nt_row * half_h + ty * l.tile_size;
        let hsz = l.tile_size;
        draw_rect(buffer, hx, hy, hsz, 1, width, 0x00FFFFFF);
        draw_rect(buffer, hx, hy + hsz.saturating_sub(1), hsz, 1, width, 0x00FFFFFF);
        draw_rect(buffer, hx, hy, 1, hsz, width, 0x00FFFFFF);
        draw_rect(buffer, hx + hsz.saturating_sub(1), hy, 1, hsz, width, 0x00FFFFFF);
    }

    let lbl_bg = 0xDD1E1E1E;
    let lbl_h = (18.0 * sc).round() as usize;
    let nt_labels = [
        ("NT 0 ($2000)", l.canvas_x + 4, l.canvas_y + 4),
        ("NT 1 ($2400)", l.canvas_x + half_w + 4, l.canvas_y + 4),
        ("NT 2 ($2800)", l.canvas_x + 4, l.canvas_y + half_h + 4),
        ("NT 3 ($2C00)", l.canvas_x + half_w + 4, l.canvas_y + half_h + 4),
    ];
    for (text, lx, ly) in nt_labels {
        let text_w = (text.len() as f32 * 8.0 * sc).round() as usize;
        let lbl_w = text_w + (12.0 * sc).round() as usize;
        draw_rect(buffer, lx, ly, lbl_w, lbl_h, width, lbl_bg);
        draw_rect(buffer, lx, ly, lbl_w, 1, width, colors.box_border);
        draw_rect(buffer, lx, ly + lbl_h.saturating_sub(1), lbl_w, 1, width, colors.box_border);
        draw_rect(buffer, lx, ly, 1, lbl_h, width, colors.box_border);
        draw_rect(buffer, lx + lbl_w.saturating_sub(1), ly, 1, lbl_h, width, colors.box_border);
        let font_h = (8.0 * sc).round() as usize;
        let tx = lx + (lbl_w.saturating_sub(text_w)) / 2;
        let ty = ly + (lbl_h.saturating_sub(font_h)) / 2;
        draw_text(buffer, tx, ty, width, text, colors.menu_text, scale);
    }

    draw_rect(buffer, l.status_x, l.status_y, l.status_w, l.status_h, width, colors.dropdown_bg);
    draw_rect(buffer, l.status_x, l.status_y, l.status_w, 1, width, colors.box_border);
    draw_rect(buffer, l.status_x, l.status_y + l.status_h.saturating_sub(1), l.status_w, 1, width, colors.box_border);
    draw_rect(buffer, l.status_x, l.status_y, 1, l.status_h, width, colors.box_border);
    draw_rect(buffer, l.status_x + l.status_w.saturating_sub(1), l.status_y, 1, l.status_h, width, colors.box_border);

    let status_ty1 = l.status_y + (4.0 * sc).round() as usize;
    let status_ty2 = l.status_y + (17.0 * sc).round() as usize;
    let status_ty3 = l.status_y + (30.0 * sc).round() as usize;
    let text_x = l.status_x + (8.0 * sc).round() as usize;

    let mirroring_desc = emu.debug_get_mirroring_desc();

    if let Some((nt, tx, ty)) = active_tile {
        let nt_base = 0x2000 + nt as u16 * 0x400;
        let tile_addr = nt_base + ty as u16 * 32 + tx as u16;
        let tile_id = nts[nt][ty * 32 + tx];
        let attr_addr = nt_base + 0x3C0 + ((ty as u16 / 4) * 8) + (tx as u16 / 4);
        let raw_attr = nts[nt][0x3C0 + ((ty / 4) * 8) + (tx / 4)];
        let shift = ((ty & 2) << 1) | (tx & 2);
        let pal_idx = (raw_attr >> shift) & 0x03;
        let chr_base = active_ptable_idx as u16 * 0x1000;
        let chr_addr = chr_base + (tile_id as u16 * 16);

        let line1 = format!(
            "NT: {} (${:04X})  Tile: {:02},{:02}  ID: ${:02X} ({})",
            nt, nt_base, tx, ty, tile_id, tile_id
        );
        let line2 = format!(
            "PPU: ${:04X}  CHR: ${:04X}  Attr: ${:04X} (BG{} [${:04X}])",
            tile_addr, chr_addr, attr_addr, pal_idx, 0x3F00 + pal_idx as u16 * 4
        );
        let line3 = format!(
            "Scroll: X={}, Y={}  |  Mirror: {}",
            scroll_x, scroll_y, mirroring_desc
        );

        draw_text(buffer, text_x, status_ty1, width, &line1, colors.menu_text, scale);
        draw_text(buffer, text_x, status_ty2, width, &line2, colors.disabled_text, scale);
        draw_text(buffer, text_x, status_ty3, width, &line3, colors.menu_text, scale);
    } else {
        let line1 = "Hover or click any tile to inspect properties".to_string();
        let line2 = format!("Scroll: X={}, Y={}  |  Mirror: {}", scroll_x, scroll_y, mirroring_desc);
        let line3 = "Keys: S=Scroll  G=Grid  A=Attr  V=View  P=Pal  Arrows".to_string();

        draw_text(buffer, text_x, status_ty1, width, &line1, colors.menu_text, scale);
        draw_text(buffer, text_x, status_ty2, width, &line2, colors.disabled_text, scale);
        draw_text(buffer, text_x, status_ty3, width, &line3, colors.menu_text, scale);
    }
}

pub(crate) fn handle_nametable_viewer_click(
    ms: &mut MenuState,
    button: winit::event::MouseButton,
    mx: usize,
    my: usize,
    width: usize,
    height: usize,
    scale: f32,
) -> bool {
    let l = compute_nametable_layout(width, height, scale);

    if !point_in_rect(mx, my, l.win_x, l.win_y, l.win_w, l.win_h) {
        return false;
    }

    if button == winit::event::MouseButton::Left {
        if point_in_rect(mx, my, l.close_x, l.close_y, l.close_w, l.close_h) {
            ms.show_nametable_viewer_window = false;
            return true;
        }

        let (bx, by, bw, bh) = l.scroll_btn;
        if point_in_rect(mx, my, bx, by, bw, bh) {
            ms.nt_viewer_show_scroll = !ms.nt_viewer_show_scroll;
            return true;
        }

        let (bx, by, bw, bh) = l.grid_btn;
        if point_in_rect(mx, my, bx, by, bw, bh) {
            ms.nt_viewer_show_grid = !ms.nt_viewer_show_grid;
            return true;
        }

        let (bx, by, bw, bh) = l.attr_grid_btn;
        if point_in_rect(mx, my, bx, by, bw, bh) {
            ms.nt_viewer_show_attr_grid = !ms.nt_viewer_show_attr_grid;
            return true;
        }

        let (bx, by, bw, bh) = l.attr_view_btn;
        if point_in_rect(mx, my, bx, by, bw, bh) {
            ms.nt_viewer_attr_view = !ms.nt_viewer_attr_view;
            return true;
        }

        let (bx, by, bw, bh) = l.pal_btn;
        if point_in_rect(mx, my, bx, by, bw, bh) {
            ms.nt_viewer_ignore_pal = !ms.nt_viewer_ignore_pal;
            return true;
        }

        let (bx, by, bw, bh) = l.pattern_btn;
        if point_in_rect(mx, my, bx, by, bw, bh) {
            ms.nt_viewer_pattern_mode = (ms.nt_viewer_pattern_mode + 1) % 3;
            return true;
        }

        let (bx, by, bw, bh) = l.auto_btn;
        if point_in_rect(mx, my, bx, by, bw, bh) {
            ms.nt_viewer_auto_refresh = !ms.nt_viewer_auto_refresh;
            return true;
        }

        let (bx, by, bw, bh) = l.step_btn;
        if point_in_rect(mx, my, bx, by, bw, bh) {
            return true;
        }

        if mx >= l.canvas_x && mx < l.canvas_x + l.canvas_w && my >= l.canvas_y && my < l.canvas_y + l.canvas_h {
            let rel_x = mx - l.canvas_x;
            let rel_y = my - l.canvas_y;
            let half_w = 32 * l.tile_size;
            let half_h = 30 * l.tile_size;
            let nt_col = rel_x / half_w;
            let nt_row = rel_y / half_h;
            let nt = nt_row * 2 + nt_col;
            let tx = (rel_x % half_w) / l.tile_size;
            let ty = (rel_y % half_h) / l.tile_size;
            if nt < 4 && tx < 32 && ty < 30 {
                ms.nt_viewer_selected_tile = Some((nt, tx, ty));
            }
            return true;
        }
    }

    true
}

pub(crate) fn handle_nametable_viewer_key(ms: &mut MenuState, keycode: VirtualKeyCode) -> bool {
    match keycode {
        VirtualKeyCode::Escape => {
            ms.show_nametable_viewer_window = false;
            true
        }
        VirtualKeyCode::S => {
            ms.nt_viewer_show_scroll = !ms.nt_viewer_show_scroll;
            true
        }
        VirtualKeyCode::G | VirtualKeyCode::T => {
            ms.nt_viewer_show_grid = !ms.nt_viewer_show_grid;
            true
        }
        VirtualKeyCode::A => {
            ms.nt_viewer_show_attr_grid = !ms.nt_viewer_show_attr_grid;
            true
        }
        VirtualKeyCode::V => {
            ms.nt_viewer_attr_view = !ms.nt_viewer_attr_view;
            true
        }
        VirtualKeyCode::P => {
            ms.nt_viewer_ignore_pal = !ms.nt_viewer_ignore_pal;
            true
        }
        VirtualKeyCode::Space => {
            true
        }
        VirtualKeyCode::Up => {
            let (mut nt, tx, mut ty) = ms.nt_viewer_selected_tile.unwrap_or((0, 0, 0));
            if ty == 0 {
                if nt >= 2 {
                    nt -= 2;
                } else {
                    nt += 2;
                }
                ty = 29;
            } else {
                ty -= 1;
            }
            ms.nt_viewer_selected_tile = Some((nt, tx, ty));
            true
        }
        VirtualKeyCode::Down => {
            let (mut nt, tx, mut ty) = ms.nt_viewer_selected_tile.unwrap_or((0, 0, 0));
            if ty >= 29 {
                if nt >= 2 {
                    nt -= 2;
                } else {
                    nt += 2;
                }
                ty = 0;
            } else {
                ty += 1;
            }
            ms.nt_viewer_selected_tile = Some((nt, tx, ty));
            true
        }
        VirtualKeyCode::Left => {
            let (mut nt, mut tx, ty) = ms.nt_viewer_selected_tile.unwrap_or((0, 0, 0));
            if tx == 0 {
                if (nt % 2) == 1 {
                    nt -= 1;
                } else {
                    nt += 1;
                }
                tx = 31;
            } else {
                tx -= 1;
            }
            ms.nt_viewer_selected_tile = Some((nt, tx, ty));
            true
        }
        VirtualKeyCode::Right => {
            let (mut nt, mut tx, ty) = ms.nt_viewer_selected_tile.unwrap_or((0, 0, 0));
            if tx >= 31 {
                if (nt % 2) == 1 {
                    nt -= 1;
                } else {
                    nt += 1;
                }
                tx = 0;
            } else {
                tx += 1;
            }
            ms.nt_viewer_selected_tile = Some((nt, tx, ty));
            true
        }
        _ => false,
    }
}
