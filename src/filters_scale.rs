// the scale2x and scale3x filters by andrea mazzoleni and ported from mesen!!!
fn scale2_border_row(out: &mut [u32], p_prev: &[u32], p_curr: &[u32], p_next: &[u32], width: usize) {
    for w in 1..=width {
        let cur = p_curr[w];
        let left = p_curr[w - 1];
        let right = p_curr[w + 1];
        let up = p_prev[w];
        let down = p_next[w];
        let o = (w - 1) * 2;
        if up != down && left != right {
            out[o] = if left == up { up } else { cur };
            out[o + 1] = if right == up { up } else { cur };
        } else {
            out[o] = cur;
            out[o + 1] = cur;
        }
    }
}

fn scale3_border_row(out: &mut [u32], p_top: &[u32], p_curr: &[u32], p_bottom: &[u32], width: usize) {
    for w in 1..=width {
        let cur = p_curr[w];
        let left = p_curr[w - 1];
        let right = p_curr[w + 1];
        let up = p_top[w];
        let up_l = p_top[w - 1];
        let up_r = p_top[w + 1];
        let down = p_bottom[w];
        let o = (w - 1) * 3;
        if up != down && left != right {
            out[o] = if left == up { up } else { cur };
            out[o + 1] = if (left == up && cur != up_r) || (right == up && cur != up_l) {
                up
            } else {
                cur
            };
            out[o + 2] = if right == up { up } else { cur };
        } else {
            out[o] = cur;
            out[o + 1] = cur;
            out[o + 2] = cur;
        }
    }
}

fn scale3_center_row(out: &mut [u32], p_top: &[u32], p_curr: &[u32], p_bottom: &[u32], width: usize) {
    for w in 1..=width {
        let cur = p_curr[w];
        let left = p_curr[w - 1];
        let right = p_curr[w + 1];
        let up = p_top[w];
        let up_l = p_top[w - 1];
        let up_r = p_top[w + 1];
        let down = p_bottom[w];
        let dn_l = p_bottom[w - 1];
        let dn_r = p_bottom[w + 1];
        let o = (w - 1) * 3;
        if up != down && left != right {
            out[o] = if (left == up && cur != dn_l) || (left == down && cur != up_l) {
                left
            } else {
                cur
            };
            out[o + 1] = cur;
            out[o + 2] = if (right == up && cur != dn_r) || (right == down && cur != up_r) {
                right
            } else {
                cur
            };
        } else {
            out[o] = cur;
            out[o + 1] = cur;
            out[o + 2] = cur;
        }
    }
}

fn pad_row(row: &[u32]) -> Vec<u32> {
    let width = row.len();
    let mut p = Vec::with_capacity(width + 2);
    p.push(row[0]);
    p.extend_from_slice(row);
    p.push(row[width - 1]);
    p
}

/// scale2x
pub fn scale2x(src: &[u32], width: usize, height: usize) -> Vec<u32> {
    if width == 0 || height == 0 || src.len() < width * height {
        return Vec::new();
    }

    let out_width = width * 2;
    let mut out = vec![0u32; out_width * height * 2];

    for y in 0..height {
        let prev_y = if y > 0 { y - 1 } else { 0 };
        let next_y = if y + 1 < height { y + 1 } else { height - 1 };

        let p_prev = pad_row(&src[prev_y * width..(prev_y + 1) * width]);
        let p_curr = pad_row(&src[y * width..(y + 1) * width]);
        let p_next = pad_row(&src[next_y * width..(next_y + 1) * width]);

        let top = &mut out[(y * 2) * out_width..(y * 2 + 1) * out_width];
        scale2_border_row(top, &p_prev, &p_curr, &p_next, width);

        let bottom = &mut out[(y * 2 + 1) * out_width..(y * 2 + 2) * out_width];
        scale2_border_row(bottom, &p_next, &p_curr, &p_prev, width);
    }

    out
}

/// scale3x
pub fn scale3x(src: &[u32], width: usize, height: usize) -> Vec<u32> {
    if width == 0 || height == 0 || src.len() < width * height {
        return Vec::new();
    }

    let out_width = width * 3;
    let mut out = vec![0u32; out_width * height * 3];

    for y in 0..height {
        let prev_y = if y > 0 { y - 1 } else { 0 };
        let next_y = if y + 1 < height { y + 1 } else { height - 1 };

        let p_prev = pad_row(&src[prev_y * width..(prev_y + 1) * width]);
        let p_curr = pad_row(&src[y * width..(y + 1) * width]);
        let p_next = pad_row(&src[next_y * width..(next_y + 1) * width]);

        let top = &mut out[(y * 3) * out_width..(y * 3 + 1) * out_width];
        scale3_border_row(top, &p_prev, &p_curr, &p_next, width);

        let middle = &mut out[(y * 3 + 1) * out_width..(y * 3 + 2) * out_width];
        scale3_center_row(middle, &p_prev, &p_curr, &p_next, width);

        let bottom = &mut out[(y * 3 + 2) * out_width..(y * 3 + 3) * out_width];
        scale3_border_row(bottom, &p_next, &p_curr, &p_prev, width);
    }

    out
}