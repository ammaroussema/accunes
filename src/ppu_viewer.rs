// ppu pattern tables and palettes viewer
use crate::emulator::Emulator;
use crate::{draw_rect, draw_text, point_in_rect, UiColors, MenuState};

#[allow(dead_code)]
pub(crate) struct PpuViewerLayout {
    pub win_x: usize,
    pub win_y: usize,
    pub win_w: usize,
    pub win_h: usize,
    pub pad_x: usize,
    pub close_x: usize,
    pub close_y: usize,
    pub close_w: usize,
    pub close_h: usize,
    pub sprite16_x: usize,
    pub sprite16_y: usize,
    pub sprite16_w: usize,
    pub sprite16_h: usize,
    pub grid_x: usize,
    pub grid_y: usize,
    pub grid_w: usize,
    pub grid_h: usize,
    pub refresh_x: usize,
    pub refresh_y: usize,
    pub refresh_w: usize,
    pub refresh_h: usize,
    pub step_x: usize,
    pub step_y: usize,
    pub step_w: usize,
    pub step_h: usize,
    pub thdr_y: usize,
    pub thdr_h: usize,
    pub table_y: usize,
    pub table_size: usize,
    pub zoom: usize,
    pub t0_x: usize,
    pub t1_x: usize,
    pub ctrl_y: usize,
    pub ctrl_h: usize,
    pub t0_pal_x: usize,
    pub t0_pal_w: usize,
    pub t1_pal_x: usize,
    pub t1_pal_w: usize,
    pub pal_y: usize,
    pub row0_y: usize,
    pub row1_y: usize,
    pub swatch_w: usize,
    pub swatch_h: usize,
    pub pal_group_gap: usize,
    pub label_w: usize,
    pub status_x: usize,
    pub status_y: usize,
    pub status_w: usize,
    pub status_h: usize,
}

pub(crate) fn compute_ppu_viewer_layout(width: usize, height: usize, scale: f32) -> PpuViewerLayout {
    let sc = scale;
    let margin_x = (8.0 * sc).round() as usize;
    let win_w = width.saturating_sub(margin_x * 2).min((640.0 * sc).round() as usize);
    let pad_x = (10.0 * sc).round() as usize;
    let mid_gap = (12.0 * sc).round() as usize;
    let avail_table_w = (win_w.saturating_sub(pad_x * 2 + mid_gap)) / 2;
    let zoom = if avail_table_w >= 256 && height >= 520 { 2 } else { 1 };
    let table_size = 128 * zoom;

    let title_h = (24.0 * sc).round() as usize;
    let close_w = (20.0 * sc).round() as usize;
    let close_h = (20.0 * sc).round() as usize;

    let btn_y_rel = title_h + (4.0 * sc).round() as usize;
    let btn_h = (20.0 * sc).round() as usize;
    let btn_gap = (6.0 * sc).round() as usize;

    let sprite16_w = (118.0 * sc).round() as usize;
    let grid_w = (72.0 * sc).round() as usize;
    let refresh_w = (114.0 * sc).round() as usize;
    let step_w = (80.0 * sc).round() as usize;

    let gap_y = (6.0 * sc).round() as usize;
    let thdr_y_rel = btn_y_rel + btn_h + gap_y;
    let thdr_h = (16.0 * sc).round() as usize;

    let table_y_rel = thdr_y_rel + thdr_h + (4.0 * sc).round() as usize;

    let ctrl_y_rel = table_y_rel + table_size + gap_y;
    let ctrl_h = (22.0 * sc).round() as usize;
    let pal_btn_w = (120.0 * sc).round() as usize;

    let pal_y_rel = ctrl_y_rel + ctrl_h + gap_y;
    let pal_hdr_h = (16.0 * sc).round() as usize;
    let swatch_w = (14.0 * sc).round() as usize;
    let swatch_h = (14.0 * sc).round() as usize;
    let pal_group_gap = (6.0 * sc).round() as usize;
    let label_w = (32.0 * sc).round() as usize;
    let row0_y_rel = pal_y_rel + pal_hdr_h + 2;
    let row1_y_rel = row0_y_rel + swatch_h + (4.0 * sc).round() as usize;

    let status_y_rel = row1_y_rel + swatch_h + gap_y;
    let status_h = (36.0 * sc).round() as usize;

    let win_h = status_y_rel + status_h + (8.0 * sc).round() as usize;
    let win_x = (width.saturating_sub(win_w)) / 2;
    let win_y = (height.saturating_sub(win_h)) / 2;

    let close_x = win_x + win_w - close_w - (6.0 * sc).round() as usize;
    let close_y = win_y + (2.0 * sc).round() as usize;

    let mut bx = win_x + pad_x;
    let sprite16_x = bx;
    bx += sprite16_w + btn_gap;
    let grid_x = bx;
    bx += grid_w + btn_gap;
    let refresh_x = bx;
    bx += refresh_w + btn_gap;
    let step_x = bx;

    let btn_y = win_y + btn_y_rel;
    let thdr_y = win_y + thdr_y_rel;
    let table_y = win_y + table_y_rel;
    let t0_x = win_x + pad_x;
    let t1_x = t0_x + table_size + mid_gap;

    let ctrl_y = win_y + ctrl_y_rel;
    let t0_pal_x = t0_x + (table_size.saturating_sub(pal_btn_w)) / 2;
    let t1_pal_x = t1_x + (table_size.saturating_sub(pal_btn_w)) / 2;

    let pal_y = win_y + pal_y_rel;
    let row0_y = win_y + row0_y_rel;
    let row1_y = win_y + row1_y_rel;

    let status_x = win_x + pad_x;
    let status_y = win_y + status_y_rel;
    let status_w = win_w.saturating_sub(pad_x * 2);

    PpuViewerLayout {
        win_x,
        win_y,
        win_w,
        win_h,
        pad_x,
        close_x,
        close_y,
        close_w,
        close_h,
        sprite16_x,
        sprite16_y: btn_y,
        sprite16_w,
        sprite16_h: btn_h,
        grid_x,
        grid_y: btn_y,
        grid_w,
        grid_h: btn_h,
        refresh_x,
        refresh_y: btn_y,
        refresh_w,
        refresh_h: btn_h,
        step_x,
        step_y: btn_y,
        step_w,
        step_h: btn_h,
        thdr_y,
        thdr_h,
        table_y,
        table_size,
        zoom,
        t0_x,
        t1_x,
        ctrl_y,
        ctrl_h,
        t0_pal_x,
        t0_pal_w: pal_btn_w,
        t1_pal_x,
        t1_pal_w: pal_btn_w,
        pal_y,
        row0_y,
        row1_y,
        swatch_w,
        swatch_h,
        pal_group_gap,
        label_w,
        status_x,
        status_y,
        status_w,
        status_h,
    }
}

pub(crate) fn palette_name(pal: usize) -> &'static str {
    match pal {
        0 => "BG 0",
        1 => "BG 1",
        2 => "BG 2",
        3 => "BG 3",
        4 => "SP 0",
        5 => "SP 1",
        6 => "SP 2",
        7 => "SP 3",
        _ => "Gray",
    }
}

pub(crate) fn get_swatch_rect(l: &PpuViewerLayout, is_sprite: bool, group: usize, col: usize) -> (usize, usize, usize, usize) {
    let group_w = l.swatch_w * 4;
    let group_stride = group_w + l.pal_group_gap + l.label_w;
    let x = l.win_x + l.pad_x + group * group_stride + l.label_w + col * l.swatch_w;
    let y = if is_sprite { l.row1_y } else { l.row0_y };
    (x, y, l.swatch_w, l.swatch_h)
}

pub(crate) fn render_ppu_viewer_window(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    ms: &MenuState,
    colors: &UiColors,
    emu: &mut Emulator,
    scale: f32,
) {
    let l = compute_ppu_viewer_layout(width, height, scale);
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
    draw_text(buffer, l.win_x + (10.0 * sc).round() as usize, title_ty, width, "PPU Viewer", colors.menu_text, scale);

    let close_hover = point_in_rect(mx, my, l.close_x, l.close_y, l.close_w, l.close_h);
    let close_bg = if close_hover { colors.menu_highlight } else { colors.close_bg };
    draw_rect(buffer, l.close_x, l.close_y, l.close_w, l.close_h, width, close_bg);
    draw_rect(buffer, l.close_x, l.close_y, l.close_w, 1, width, colors.btn_border);
    draw_rect(buffer, l.close_x, l.close_y + l.close_h.saturating_sub(1), l.close_w, 1, width, colors.btn_border);
    draw_rect(buffer, l.close_x, l.close_y, 1, l.close_h, width, colors.btn_border);
    draw_rect(buffer, l.close_x + l.close_w.saturating_sub(1), l.close_y, 1, l.close_h, width, colors.btn_border);
    let close_ty = l.close_y + ((l.close_h as f32 - 8.0 * sc) / 2.0).round() as usize;
    draw_text(buffer, l.close_x + (6.0 * sc).round() as usize, close_ty, width, "X", colors.menu_text, scale);

    let render_btn = |buf: &mut [u32], bx: usize, by: usize, bw: usize, bh: usize, text: &str| {
        let hover = point_in_rect(mx, my, bx, by, bw, bh);
        let bg = if hover {
            colors.box_bg_hover
        } else {
            colors.box_bg_default
        };
        draw_rect(buf, bx, by, bw, bh, width, bg);
        draw_rect(buf, bx, by, bw, 1, width, colors.btn_border);
        draw_rect(buf, bx, by + bh.saturating_sub(1), bw, 1, width, colors.btn_border);
        draw_rect(buf, bx, by, 1, bh, width, colors.btn_border);
        draw_rect(buf, bx + bw.saturating_sub(1), by, 1, bh, width, colors.btn_border);
        let text_w = text.len() * (8.0 * sc).round() as usize;
        let tx = bx + (bw.saturating_sub(text_w)) / 2;
        let ty = by + ((bh as f32 - 8.0 * sc) / 2.0).round() as usize;
        draw_text(buf, tx, ty, width, text, colors.menu_text, scale);
    };

    let s16_text = if ms.ppu_viewer_sprite16 { "Sprites: 8x16" } else { "Sprites: 8x8" };
    render_btn(buffer, l.sprite16_x, l.sprite16_y, l.sprite16_w, l.sprite16_h, s16_text);

    let grid_text = if ms.ppu_viewer_show_grid { "Grid: ON" } else { "Grid: OFF" };
    render_btn(buffer, l.grid_x, l.grid_y, l.grid_w, l.grid_h, grid_text);

    let auto_text = if ms.ppu_viewer_auto_refresh { "Refresh: Live" } else { "Refresh: Freeze" };
    render_btn(buffer, l.refresh_x, l.refresh_y, l.refresh_w, l.refresh_h, auto_text);

    render_btn(buffer, l.step_x, l.step_y, l.step_w, l.step_h, "Refresh");

    let thdr_ty = l.thdr_y + ((l.thdr_h as f32 - 8.0 * sc) / 2.0).round() as usize;
    draw_text(buffer, l.t0_x, thdr_ty, width, "Table 0 ($0000)", colors.menu_text, scale);
    draw_text(buffer, l.t1_x, thdr_ty, width, "Table 1 ($1000)", colors.menu_text, scale);

    let mut table0 = [0u8; 4096];
    let mut table1 = [0u8; 4096];
    emu.debug_peek_chr_tables(&mut table0, &mut table1);
    let pal_ram = emu.debug_peek_palette_ram();

    let render_pattern = |buf: &mut [u32], tx: usize, ty: usize, table_bytes: &[u8; 4096], pal_idx: usize, table_id: usize| {
        let border_color = if ms.ppu_viewer_selected_table == table_id {
            colors.menu_highlight
        } else {
            colors.box_border
        };
        draw_rect(buf, tx.saturating_sub(1), ty.saturating_sub(1), l.table_size + 2, l.table_size + 2, width, border_color);
        for py in 0..128 {
            for px in 0..128 {
                let (tile_idx, in_tile_x, in_tile_y) = if ms.ppu_viewer_sprite16 {
                    let a = (px / 8) * 2;
                    let b = py / 8;
                    let t_x = (a & 0xE) + (b & 1);
                    let t_y = (b & 0xE) + ((a >> 4) & 1);
                    ((t_y << 4) | t_x, px % 8, py % 8)
                } else {
                    let col = px / 8;
                    let row = py / 8;
                    ((row << 4) | col, px % 8, py % 8)
                };

                let chr0 = table_bytes[tile_idx * 16 + in_tile_y];
                let chr1 = table_bytes[tile_idx * 16 + 8 + in_tile_y];
                let bit = 7 - in_tile_x;
                let p = ((chr0 >> bit) & 1) | (((chr1 >> bit) & 1) << 1);

                let nes_color = match pal_idx {
                    0..=3 => {
                        if p == 0 { pal_ram[0] } else { pal_ram[pal_idx * 4 + p as usize] }
                    }
                    4..=7 => {
                        let sp = pal_idx - 4;
                        if p == 0 { pal_ram[0] } else { pal_ram[16 + sp * 4 + p as usize] }
                    }
                    _ => [0x0F, 0x00, 0x10, 0x20][p as usize],
                };

                let rgb = emu.palette_lut[(nes_color & 0x3F) as usize];
                if l.zoom == 1 {
                    let dx = tx + px;
                    let dy = ty + py;
                    if dy < height && dx < width {
                        buf[dy * width + dx] = rgb;
                    }
                } else {
                    let dx = tx + px * 2;
                    let dy = ty + py * 2;
                    if dy + 1 < height && dx + 1 < width {
                        buf[dy * width + dx] = rgb;
                        buf[dy * width + dx + 1] = rgb;
                        buf[(dy + 1) * width + dx] = rgb;
                        buf[(dy + 1) * width + dx + 1] = rgb;
                    }
                }
            }
        }

        if ms.ppu_viewer_show_grid {
            let tile_step = 8 * l.zoom;
            for t in 1..16 {
                let gx = tx + t * tile_step;
                let gy = ty + t * tile_step;
                for y in ty..(ty + l.table_size) {
                    if y < height && gx < width {
                        buf[y * width + gx] = 0x003A3A3A;
                    }
                }
                for x in tx..(tx + l.table_size) {
                    if gy < height && x < width {
                        buf[gy * width + x] = 0x003A3A3A;
                    }
                }
            }
        }

        if let Some((st, scol, srow)) = ms.ppu_viewer_selected_tile {
            if st == table_id {
                let tile_step = 8 * l.zoom;
                let sx = tx + scol * tile_step;
                let sy = ty + srow * tile_step;
                let sw = tile_step;
                let sh = if ms.ppu_viewer_sprite16 { tile_step * 2 } else { tile_step };
                draw_rect(buf, sx, sy, sw, 1, width, 0x00FFFFFF);
                draw_rect(buf, sx, sy + sh.saturating_sub(1), sw, 1, width, 0x00FFFFFF);
                draw_rect(buf, sx, sy, 1, sh, width, 0x00FFFFFF);
                draw_rect(buf, sx + sw.saturating_sub(1), sy, 1, sh, width, 0x00FFFFFF);
            }
        }
    };

    render_pattern(buffer, l.t0_x, l.table_y, &table0, ms.ppu_viewer_pal_table0, 0);
    render_pattern(buffer, l.t1_x, l.table_y, &table1, ms.ppu_viewer_pal_table1, 1);

    let t0_btn_text = format!("Pal: {}", palette_name(ms.ppu_viewer_pal_table0));
    render_btn(buffer, l.t0_pal_x, l.ctrl_y, l.t0_pal_w, l.ctrl_h, &t0_btn_text);

    let t1_btn_text = format!("Pal: {}", palette_name(ms.ppu_viewer_pal_table1));
    render_btn(buffer, l.t1_pal_x, l.ctrl_y, l.t1_pal_w, l.ctrl_h, &t1_btn_text);

    draw_text(buffer, l.win_x + l.pad_x, l.pal_y, width, "Palettes: (Row 1: Background, Row 2: Sprites)", colors.menu_text, scale);

    for g in 0..4 {
        let lx = l.win_x + l.pad_x + g * ((l.swatch_w * 4) + l.pal_group_gap + l.label_w);
        let lbl = format!("BG{}", g);
        let ty0 = l.row0_y + ((l.swatch_h as f32 - 8.0 * sc) / 2.0).round() as usize;
        draw_text(buffer, lx, ty0, width, &lbl, colors.menu_text, scale);

        for c in 0..4 {
            let (sx, sy, sw, sh) = get_swatch_rect(&l, false, g, c);
            let p_idx = if c == 0 { pal_ram[0] } else { pal_ram[g * 4 + c] };
            let rgb = emu.palette_lut[(p_idx & 0x3F) as usize];
            draw_rect(buffer, sx, sy, sw, sh, width, rgb);
            let hover_swatch = point_in_rect(mx, my, sx, sy, sw, sh);
            let border = if hover_swatch { 0x00FFFFFF } else { colors.box_border };
            draw_rect(buffer, sx, sy, sw, 1, width, border);
            draw_rect(buffer, sx, sy + sh.saturating_sub(1), sw, 1, width, border);
            draw_rect(buffer, sx, sy, 1, sh, width, border);
            draw_rect(buffer, sx + sw.saturating_sub(1), sy, 1, sh, width, border);
        }
    }

    for g in 0..4 {
        let lx = l.win_x + l.pad_x + g * ((l.swatch_w * 4) + l.pal_group_gap + l.label_w);
        let lbl = format!("SP{}", g);
        let ty1 = l.row1_y + ((l.swatch_h as f32 - 8.0 * sc) / 2.0).round() as usize;
        draw_text(buffer, lx, ty1, width, &lbl, colors.menu_text, scale);

        for c in 0..4 {
            let (sx, sy, sw, sh) = get_swatch_rect(&l, true, g, c);
            let p_idx = if c == 0 { pal_ram[0] } else { pal_ram[16 + g * 4 + c] };
            let rgb = emu.palette_lut[(p_idx & 0x3F) as usize];
            draw_rect(buffer, sx, sy, sw, sh, width, rgb);
            let hover_swatch = point_in_rect(mx, my, sx, sy, sw, sh);
            let border = if hover_swatch { 0x00FFFFFF } else { colors.box_border };
            draw_rect(buffer, sx, sy, sw, 1, width, border);
            draw_rect(buffer, sx, sy + sh.saturating_sub(1), sw, 1, width, border);
            draw_rect(buffer, sx, sy, 1, sh, width, border);
            draw_rect(buffer, sx + sw.saturating_sub(1), sy, 1, sh, width, border);
        }
    }

    draw_rect(buffer, l.status_x, l.status_y, l.status_w, l.status_h, width, colors.dropdown_bg);
    draw_rect(buffer, l.status_x, l.status_y, l.status_w, 1, width, colors.box_border);
    draw_rect(buffer, l.status_x, l.status_y + l.status_h.saturating_sub(1), l.status_w, 1, width, colors.box_border);
    draw_rect(buffer, l.status_x, l.status_y, 1, l.status_h, width, colors.box_border);
    draw_rect(buffer, l.status_x + l.status_w.saturating_sub(1), l.status_y, 1, l.status_h, width, colors.box_border);

    let status_ty1 = l.status_y + (4.0 * sc).round() as usize;
    let status_ty2 = l.status_y + (18.0 * sc).round() as usize;

    let mut line1 = String::new();
    let mut line2 = String::new();

    let in_t0 = point_in_rect(mx, my, l.t0_x, l.table_y, l.table_size, l.table_size);
    let in_t1 = point_in_rect(mx, my, l.t1_x, l.table_y, l.table_size, l.table_size);

    if in_t0 || in_t1 {
        let (t_id, t_x, t_data, active_pal) = if in_t0 {
            (0, l.t0_x, &table0, ms.ppu_viewer_pal_table0)
        } else {
            (1, l.t1_x, &table1, ms.ppu_viewer_pal_table1)
        };
        let rel_x = (mx - t_x) / l.zoom;
        let rel_y = (my - l.table_y) / l.zoom;
        if rel_x < 128 && rel_y < 128 {
            let (tile_idx, in_tile_x, in_tile_y, is_bot) = if ms.ppu_viewer_sprite16 {
                let a = (rel_x / 8) * 2;
                let b = rel_y / 8;
                let t_x = (a & 0xE) + (b & 1);
                let t_y = (b & 0xE) + ((a >> 4) & 1);
                ((t_y << 4) | t_x, rel_x % 8, rel_y % 8, (b & 1) != 0)
            } else {
                let col = rel_x / 8;
                let row = rel_y / 8;
                ((row << 4) | col, rel_x % 8, rel_y % 8, false)
            };

            let chr0 = t_data[tile_idx * 16 + in_tile_y];
            let chr1 = t_data[tile_idx * 16 + 8 + in_tile_y];
            let bit = 7 - in_tile_x;
            let p = ((chr0 >> bit) & 1) | (((chr1 >> bit) & 1) << 1);

            let nes_color = match active_pal {
                0..=3 => {
                    if p == 0 { pal_ram[0] } else { pal_ram[active_pal * 4 + p as usize] }
                }
                4..=7 => {
                    let sp = active_pal - 4;
                    if p == 0 { pal_ram[0] } else { pal_ram[16 + sp * 4 + p as usize] }
                }
                _ => [0x0F, 0x00, 0x10, 0x20][p as usize],
            };

            let addr = (t_id << 12) | (tile_idx << 4);
            if ms.ppu_viewer_sprite16 {
                line1 = format!("Table {} | Tile: ${:02X} ({}) {} | Addr: ${:04X}", t_id, tile_idx, tile_idx, if is_bot { "Bot" } else { "Top" }, addr);
            } else {
                line1 = format!("Table {} | Tile: ${:02X} ({}) | Addr: ${:04X}", t_id, tile_idx, tile_idx, addr);
            }
            line2 = format!("Pixel: ({}, {}) | Color: {} | Value: ${:02X}", in_tile_x, in_tile_y, p, nes_color);
        }
    } else {
        let mut hovered_pal = None;
        for g in 0..4 {
            for c in 0..4 {
                let (sx, sy, sw, sh) = get_swatch_rect(&l, false, g, c);
                if point_in_rect(mx, my, sx, sy, sw, sh) {
                    let entry = if c == 0 { 0 } else { g * 4 + c };
                    hovered_pal = Some((format!("Palette: BG {} Color {} ($3F{:02X})", g, c, entry), format!("NES Palette Value: ${:02X}", pal_ram[entry])));
                    break;
                }
            }
            if hovered_pal.is_some() { break; }
        }
        if hovered_pal.is_none() {
            for g in 0..4 {
                for c in 0..4 {
                    let (sx, sy, sw, sh) = get_swatch_rect(&l, true, g, c);
                    if point_in_rect(mx, my, sx, sy, sw, sh) {
                        let entry = if c == 0 { 0 } else { 16 + g * 4 + c };
                        hovered_pal = Some((format!("Palette: Sprite {} Color {} ($3F{:02X})", g, c, entry), format!("NES Palette Value: ${:02X}", pal_ram[entry])));
                        break;
                    }
                }
                if hovered_pal.is_some() { break; }
            }
        }

        if let Some((l1, l2)) = hovered_pal {
            line1 = l1;
            line2 = l2;
        } else if let Some((st, scol, srow)) = ms.ppu_viewer_selected_tile {
            let t_idx = (srow << 4) | scol;
            let addr = (st << 12) | (t_idx << 4);
            line1 = format!("Selected Table {} | Tile: ${:02X} ({}) | Addr: ${:04X}", st, t_idx, t_idx, addr);
            line2 = "Right-click table or click Pal button to cycle palette.".to_string();
        } else {
            line1 = "Hover over tile or palette for inspection details.".to_string();
            line2 = "Click tile to select. Right-click table to cycle palette.".to_string();
        }
    }

    draw_text(buffer, l.status_x + (8.0 * sc).round() as usize, status_ty1, width, &line1, colors.menu_text, scale);
    draw_text(buffer, l.status_x + (8.0 * sc).round() as usize, status_ty2, width, &line2, colors.disabled_text, scale);
}

pub(crate) fn handle_ppu_viewer_click(
    ms: &mut MenuState,
    button: winit::event::MouseButton,
    mx: usize,
    my: usize,
    width: usize,
    height: usize,
    scale: f32,
) {
    let l = compute_ppu_viewer_layout(width, height, scale);

    if button == winit::event::MouseButton::Left {
        if point_in_rect(mx, my, l.close_x, l.close_y, l.close_w, l.close_h) {
            ms.show_ppu_viewer_window = false;
            return;
        }

        if point_in_rect(mx, my, l.sprite16_x, l.sprite16_y, l.sprite16_w, l.sprite16_h) {
            ms.ppu_viewer_sprite16 = !ms.ppu_viewer_sprite16;
            return;
        }

        if point_in_rect(mx, my, l.grid_x, l.grid_y, l.grid_w, l.grid_h) {
            ms.ppu_viewer_show_grid = !ms.ppu_viewer_show_grid;
            return;
        }

        if point_in_rect(mx, my, l.refresh_x, l.refresh_y, l.refresh_w, l.refresh_h) {
            ms.ppu_viewer_auto_refresh = !ms.ppu_viewer_auto_refresh;
            return;
        }

        if point_in_rect(mx, my, l.step_x, l.step_y, l.step_w, l.step_h) {
            return;
        }

        if point_in_rect(mx, my, l.t0_pal_x, l.ctrl_y, l.t0_pal_w, l.ctrl_h) {
            ms.ppu_viewer_pal_table0 = (ms.ppu_viewer_pal_table0 + 1) % 9;
            ms.ppu_viewer_selected_table = 0;
            return;
        }

        if point_in_rect(mx, my, l.t1_pal_x, l.ctrl_y, l.t1_pal_w, l.ctrl_h) {
            ms.ppu_viewer_pal_table1 = (ms.ppu_viewer_pal_table1 + 1) % 9;
            ms.ppu_viewer_selected_table = 1;
            return;
        }

        if point_in_rect(mx, my, l.t0_x, l.table_y, l.table_size, l.table_size) {
            let rel_x = (mx - l.t0_x) / l.zoom;
            let rel_y = (my - l.table_y) / l.zoom;
            let col = (rel_x / 8).min(15);
            let row = (rel_y / 8).min(15);
            ms.ppu_viewer_selected_tile = Some((0, col, row));
            ms.ppu_viewer_selected_table = 0;
            return;
        }

        if point_in_rect(mx, my, l.t1_x, l.table_y, l.table_size, l.table_size) {
            let rel_x = (mx - l.t1_x) / l.zoom;
            let rel_y = (my - l.table_y) / l.zoom;
            let col = (rel_x / 8).min(15);
            let row = (rel_y / 8).min(15);
            ms.ppu_viewer_selected_tile = Some((1, col, row));
            ms.ppu_viewer_selected_table = 1;
            return;
        }

        for g in 0..4 {
            for c in 0..4 {
                let (sx, sy, sw, sh) = get_swatch_rect(&l, false, g, c);
                if point_in_rect(mx, my, sx, sy, sw, sh) {
                    if ms.ppu_viewer_selected_table == 0 {
                        ms.ppu_viewer_pal_table0 = g;
                    } else {
                        ms.ppu_viewer_pal_table1 = g;
                    }
                    return;
                }
            }
        }

        for g in 0..4 {
            for c in 0..4 {
                let (sx, sy, sw, sh) = get_swatch_rect(&l, true, g, c);
                if point_in_rect(mx, my, sx, sy, sw, sh) {
                    if ms.ppu_viewer_selected_table == 0 {
                        ms.ppu_viewer_pal_table0 = 4 + g;
                    } else {
                        ms.ppu_viewer_pal_table1 = 4 + g;
                    }
                    return;
                }
            }
        }
    } else if button == winit::event::MouseButton::Right {
        if point_in_rect(mx, my, l.t0_x, l.table_y, l.table_size, l.table_size) {
            ms.ppu_viewer_pal_table0 = (ms.ppu_viewer_pal_table0 + 1) % 9;
            ms.ppu_viewer_selected_table = 0;
            return;
        }

        if point_in_rect(mx, my, l.t1_x, l.table_y, l.table_size, l.table_size) {
            ms.ppu_viewer_pal_table1 = (ms.ppu_viewer_pal_table1 + 1) % 9;
            ms.ppu_viewer_selected_table = 1;
            return;
        }
    }
}

pub(crate) fn handle_ppu_viewer_key(ms: &mut MenuState, keycode: winit::event::VirtualKeyCode) {
    match keycode {
        winit::event::VirtualKeyCode::Escape => {
            ms.show_ppu_viewer_window = false;
        }
        winit::event::VirtualKeyCode::G => {
            ms.ppu_viewer_show_grid = !ms.ppu_viewer_show_grid;
        }
        winit::event::VirtualKeyCode::S => {
            ms.ppu_viewer_sprite16 = !ms.ppu_viewer_sprite16;
        }
        winit::event::VirtualKeyCode::P => {
            if ms.ppu_viewer_selected_table == 0 {
                ms.ppu_viewer_pal_table0 = (ms.ppu_viewer_pal_table0 + 1) % 9;
            } else {
                ms.ppu_viewer_pal_table1 = (ms.ppu_viewer_pal_table1 + 1) % 9;
            }
        }
        winit::event::VirtualKeyCode::Space | winit::event::VirtualKeyCode::F5 => {
            ms.ppu_viewer_auto_refresh = !ms.ppu_viewer_auto_refresh;
        }
        _ => {}
    }
}
