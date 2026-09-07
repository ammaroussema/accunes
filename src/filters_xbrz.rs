// the xbrz filter, made by zenju and ported from mesen!!!

#[inline(always)]
fn get_red(pix: u32) -> u32 {
    (pix >> 16) & 0xff
}

#[inline(always)]
fn get_green(pix: u32) -> u32 {
    (pix >> 8) & 0xff
}

#[inline(always)]
fn get_blue(pix: u32) -> u32 {
    pix & 0xff
}

#[inline(always)]
fn make_pixel(r: u32, g: u32, b: u32) -> u32 {
    (r << 16) | (g << 8) | b
}

#[inline(always)]
pub fn dist_rgb(pix1: u32, pix2: u32) -> f64 {
    let r_diff = get_red(pix1) as i64 - get_red(pix2) as i64;
    let g_diff = get_green(pix1) as i64 - get_green(pix2) as i64;
    let b_diff = get_blue(pix1) as i64 - get_blue(pix2) as i64;

    let rd = ((r_diff + 255) / 2) * 2 - 255;
    let gd = ((g_diff + 255) / 2) * 2 - 255;
    let bd = ((b_diff + 255) / 2) * 2 - 255;

    let rd = rd as f64;
    let gd = gd as f64;
    let bd = bd as f64;

    const K_B: f64 = 0.0593;
    const K_R: f64 = 0.2627;
    const K_G: f64 = 1.0 - K_B - K_R;

    const SCALE_B: f64 = 0.5 / (1.0 - K_B);
    const SCALE_R: f64 = 0.5 / (1.0 - K_R);

    let y = K_R * rd + K_G * gd + K_B * bd;
    let c_b = SCALE_B * (bd - y);
    let c_r = SCALE_R * (rd - y);

    (y * y + c_b * c_b + c_r * c_r).sqrt()
}

#[inline(always)]
fn gradient_rgb(m: u32, n: u32, pix_front: u32, pix_back: u32) -> u32 {
    let calc = |col_front: u32, col_back: u32| -> u32 { (col_front * m + col_back * (n - m)) / n };
    make_pixel(
        calc(get_red(pix_front), get_red(pix_back)),
        calc(get_green(pix_front), get_green(pix_back)),
        calc(get_blue(pix_front), get_blue(pix_back)),
    )
}

type BlendType = u8;
const BLEND_NONE: BlendType = 0;
const BLEND_NORMAL: BlendType = 1;
const BLEND_DOMINANT: BlendType = 2;

#[derive(Clone, Copy)]
struct BlendResult {
    blend_f: BlendType,
    blend_g: BlendType,
    blend_j: BlendType,
    blend_k: BlendType,
}

#[derive(Clone, Copy)]
struct Kernel4 {
    a: u32,
    b: u32,
    c: u32,
    #[allow(dead_code)]
    d: u32,
    e: u32,
    f: u32,
    g: u32,
    h: u32,
    i: u32,
    j: u32,
    k: u32,
    l: u32,
    #[allow(dead_code)]
    m: u32,
    n: u32,
    o: u32,
    #[allow(dead_code)]
    p: u32,
}

#[derive(Clone, Copy)]
struct Kern {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    e: u32,
    f: u32,
    g: u32,
    h: u32,
    i: u32,
}

fn pre_process_corners(ker: &Kernel4, cfg_dominant: f64) -> BlendResult {
    let mut result = BlendResult {
        blend_f: BLEND_NONE,
        blend_g: BLEND_NONE,
        blend_j: BLEND_NONE,
        blend_k: BLEND_NONE,
    };

    if (ker.f == ker.g && ker.j == ker.k) || (ker.f == ker.j && ker.g == ker.k) {
        return result;
    }

    const WEIGHT: f64 = 4.0;
    let jg = dist_rgb(ker.i, ker.f)
        + dist_rgb(ker.f, ker.c)
        + dist_rgb(ker.n, ker.k)
        + dist_rgb(ker.k, ker.h)
        + WEIGHT * dist_rgb(ker.j, ker.g);
    let fk = dist_rgb(ker.e, ker.j)
        + dist_rgb(ker.j, ker.o)
        + dist_rgb(ker.b, ker.g)
        + dist_rgb(ker.g, ker.l)
        + WEIGHT * dist_rgb(ker.f, ker.k);

    if jg < fk {
        let dominant_gradient = cfg_dominant * jg < fk;
        if ker.f != ker.g && ker.f != ker.j {
            result.blend_f = if dominant_gradient { BLEND_DOMINANT } else { BLEND_NORMAL };
        }
        if ker.k != ker.j && ker.k != ker.g {
            result.blend_k = if dominant_gradient { BLEND_DOMINANT } else { BLEND_NORMAL };
        }
    } else if fk < jg {
        let dominant_gradient = cfg_dominant * fk < jg;
        if ker.j != ker.f && ker.j != ker.k {
            result.blend_j = if dominant_gradient { BLEND_DOMINANT } else { BLEND_NORMAL };
        }
        if ker.g != ker.f && ker.g != ker.k {
            result.blend_g = if dominant_gradient { BLEND_DOMINANT } else { BLEND_NORMAL };
        }
    }
    result
}

#[inline(always)]
fn get_top_r(b: u8) -> BlendType {
    0x3 & (b >> 2)
}
#[inline(always)]
fn get_bottom_r(b: u8) -> BlendType {
    0x3 & (b >> 4)
}
#[inline(always)]
fn get_bottom_l(b: u8) -> BlendType {
    0x3 & (b >> 6)
}

#[inline(always)]
fn rotate_blend_info(rot: u32, b: u8) -> u8 {
    match rot {
        90 => ((b << 2) | (b >> 6)) & 0xff,
        180 => ((b << 4) | (b >> 4)) & 0xff,
        270 => ((b << 6) | (b >> 2)) & 0xff,
        _ => b,
    }
}

#[inline(always)]
fn rotate_kern(rot: u32, k: Kern) -> Kern {
    match rot {
        90 => Kern {
            a: k.a,
            b: k.d,
            c: k.a,
            d: k.h,
            e: k.e,
            f: k.b,
            g: k.i,
            h: k.f,
            i: k.c,
        },
        180 => Kern {
            a: k.a,
            b: k.h,
            c: k.g,
            d: k.f,
            e: k.e,
            f: k.d,
            g: k.c,
            h: k.b,
            i: k.a,
        },
        270 => Kern {
            a: k.a,
            b: k.f,
            c: k.i,
            d: k.b,
            e: k.e,
            f: k.h,
            g: k.a,
            h: k.d,
            i: k.g,
        },
        _ => k,
    }
}

#[inline(always)]
fn phys_pos(n: u32, rot: u32, i_logical: u32, j_logical: u32) -> (u32, u32) {
    let nm1 = n - 1;
    match rot {
        90 => (nm1 - j_logical, i_logical),
        180 => (nm1 - i_logical, nm1 - j_logical),
        270 => (j_logical, nm1 - i_logical),
        _ => (i_logical, j_logical),
    }
}

#[inline(always)]
fn grad(
    out: &mut [u32],
    out_width: u32,
    block_x: u32,
    block_y: u32,
    n: u32,
    rot: u32,
    i_logical: u32,
    j_logical: u32,
    m: u32,
    div: u32,
    col: u32,
) {
    let (io, jo) = phys_pos(n, rot, i_logical, j_logical);
    let idx = ((block_y + io) * out_width + (block_x + jo)) as usize;
    out[idx] = gradient_rgb(m, div, col, out[idx]);
}

#[inline(always)]
fn set_col(
    out: &mut [u32],
    out_width: u32,
    block_x: u32,
    block_y: u32,
    n: u32,
    rot: u32,
    i_logical: u32,
    j_logical: u32,
    col: u32,
) {
    let (io, jo) = phys_pos(n, rot, i_logical, j_logical);
    let idx = ((block_y + io) * out_width + (block_x + jo)) as usize;
    out[idx] = col;
}

macro_rules! g {
    ($out:ident, $ow:ident, $bx:ident, $by:ident, $n:ident, $rot:ident, $i:literal, $j:literal, $m:literal, $d:literal, $col:ident) => {
        grad($out, $ow, $bx, $by, $n, $rot, $i, $j, $m, $d, $col)
    };
}
macro_rules! s {
    ($out:ident, $ow:ident, $bx:ident, $by:ident, $n:ident, $rot:ident, $i:literal, $j:literal, $col:ident) => {
        set_col($out, $ow, $bx, $by, $n, $rot, $i, $j, $col)
    };
}

#[allow(clippy::too_many_arguments)]
fn blend_line_shallow(
    n: u32,
    out: &mut [u32],
    out_width: u32,
    block_x: u32,
    block_y: u32,
    rot: u32,
    col: u32,
) {
    match n {
        2 => {
            g!(out, out_width, block_x, block_y, n, rot, 1, 0, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 1, 1, 3, 4, col);
        }
        3 => {
            g!(out, out_width, block_x, block_y, n, rot, 2, 0, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 1, 2, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 2, 1, 3, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 2, 2, col);
        }
        4 => {
            g!(out, out_width, block_x, block_y, n, rot, 3, 0, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 2, 2, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 1, 3, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 2, 3, 3, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 3, 2, col);
            s!(out, out_width, block_x, block_y, n, rot, 3, 3, col);
        }
        5 => {
            g!(out, out_width, block_x, block_y, n, rot, 4, 0, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 2, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 2, 4, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 4, 1, 3, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 3, 3, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 4, 2, col);
            s!(out, out_width, block_x, block_y, n, rot, 4, 3, col);
            s!(out, out_width, block_x, block_y, n, rot, 4, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 3, 4, col);
        }
        _ => {
            g!(out, out_width, block_x, block_y, n, rot, 5, 0, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 4, 2, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 4, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 5, 1, 3, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 4, 3, 3, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 5, 3, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 5, 2, col);
            s!(out, out_width, block_x, block_y, n, rot, 5, 3, col);
            s!(out, out_width, block_x, block_y, n, rot, 5, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 5, 5, col);
            s!(out, out_width, block_x, block_y, n, rot, 4, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 4, 5, col);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn blend_line_steep(
    n: u32,
    out: &mut [u32],
    out_width: u32,
    block_x: u32,
    block_y: u32,
    rot: u32,
    col: u32,
) {
    match n {
        2 => {
            g!(out, out_width, block_x, block_y, n, rot, 0, 1, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 1, 1, 3, 4, col);
        }
        3 => {
            g!(out, out_width, block_x, block_y, n, rot, 0, 2, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 2, 1, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 1, 2, 3, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 2, 2, col);
        }
        4 => {
            g!(out, out_width, block_x, block_y, n, rot, 0, 3, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 2, 2, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 1, 3, 3, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 2, 3, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 2, 3, col);
            s!(out, out_width, block_x, block_y, n, rot, 3, 3, col);
        }
        5 => {
            g!(out, out_width, block_x, block_y, n, rot, 0, 4, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 2, 3, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 4, 2, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 1, 4, 3, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 3, 3, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 2, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 3, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 4, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 4, 3, col);
        }
        _ => {
            g!(out, out_width, block_x, block_y, n, rot, 0, 5, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 2, 4, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 4, 3, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 1, 5, 3, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 4, 3, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 5, 3, 3, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 2, 5, col);
            s!(out, out_width, block_x, block_y, n, rot, 3, 5, col);
            s!(out, out_width, block_x, block_y, n, rot, 4, 5, col);
            s!(out, out_width, block_x, block_y, n, rot, 5, 5, col);
            s!(out, out_width, block_x, block_y, n, rot, 4, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 5, 4, col);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn blend_line_steep_and_shallow(
    n: u32,
    out: &mut [u32],
    out_width: u32,
    block_x: u32,
    block_y: u32,
    rot: u32,
    col: u32,
) {
    match n {
        2 => {
            g!(out, out_width, block_x, block_y, n, rot, 1, 0, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 0, 1, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 1, 1, 5, 6, col);
        }
        3 => {
            g!(out, out_width, block_x, block_y, n, rot, 2, 0, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 0, 2, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 2, 1, 3, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 1, 2, 3, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 2, 2, col);
        }
        4 => {
            g!(out, out_width, block_x, block_y, n, rot, 3, 1, 3, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 1, 3, 3, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 0, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 0, 3, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 2, 2, 1, 3, col);
            s!(out, out_width, block_x, block_y, n, rot, 3, 3, col);
            s!(out, out_width, block_x, block_y, n, rot, 3, 2, col);
            s!(out, out_width, block_x, block_y, n, rot, 2, 3, col);
        }
        5 => {
            g!(out, out_width, block_x, block_y, n, rot, 0, 4, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 2, 3, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 1, 4, 3, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 4, 0, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 2, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 4, 1, 3, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 3, 2, 3, col);
            s!(out, out_width, block_x, block_y, n, rot, 2, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 3, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 4, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 4, 2, col);
            s!(out, out_width, block_x, block_y, n, rot, 4, 3, col);
        }
        _ => {
            g!(out, out_width, block_x, block_y, n, rot, 0, 5, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 2, 4, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 1, 5, 3, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 4, 3, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 5, 0, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 4, 2, 1, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 5, 1, 3, 4, col);
            g!(out, out_width, block_x, block_y, n, rot, 4, 3, 3, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 2, 5, col);
            s!(out, out_width, block_x, block_y, n, rot, 3, 5, col);
            s!(out, out_width, block_x, block_y, n, rot, 4, 5, col);
            s!(out, out_width, block_x, block_y, n, rot, 5, 5, col);
            s!(out, out_width, block_x, block_y, n, rot, 4, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 5, 4, col);
            s!(out, out_width, block_x, block_y, n, rot, 5, 2, col);
            s!(out, out_width, block_x, block_y, n, rot, 5, 3, col);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn blend_line_diagonal(
    n: u32,
    out: &mut [u32],
    out_width: u32,
    block_x: u32,
    block_y: u32,
    rot: u32,
    col: u32,
) {
    match n {
        2 => {
            g!(out, out_width, block_x, block_y, n, rot, 1, 1, 1, 2, col);
        }
        3 => {
            g!(out, out_width, block_x, block_y, n, rot, 1, 2, 1, 8, col);
            g!(out, out_width, block_x, block_y, n, rot, 2, 1, 1, 8, col);
            g!(out, out_width, block_x, block_y, n, rot, 2, 2, 7, 8, col);
        }
        4 => {
            g!(out, out_width, block_x, block_y, n, rot, 3, 2, 1, 2, col);
            g!(out, out_width, block_x, block_y, n, rot, 2, 3, 1, 2, col);
            s!(out, out_width, block_x, block_y, n, rot, 3, 3, col);
        }
        5 => {
            g!(out, out_width, block_x, block_y, n, rot, 4, 2, 1, 8, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 3, 1, 8, col);
            g!(out, out_width, block_x, block_y, n, rot, 2, 4, 1, 8, col);
            g!(out, out_width, block_x, block_y, n, rot, 4, 3, 7, 8, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 4, 7, 8, col);
            s!(out, out_width, block_x, block_y, n, rot, 4, 4, col);
        }
        _ => {
            g!(out, out_width, block_x, block_y, n, rot, 5, 3, 1, 2, col);
            g!(out, out_width, block_x, block_y, n, rot, 4, 4, 1, 2, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 5, 1, 2, col);
            s!(out, out_width, block_x, block_y, n, rot, 4, 5, col);
            s!(out, out_width, block_x, block_y, n, rot, 5, 5, col);
            s!(out, out_width, block_x, block_y, n, rot, 5, 4, col);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn blend_corner(
    n: u32,
    out: &mut [u32],
    out_width: u32,
    block_x: u32,
    block_y: u32,
    rot: u32,
    col: u32,
) {
    match n {
        2 => {
            g!(out, out_width, block_x, block_y, n, rot, 1, 1, 21, 100, col);
        }
        3 => {
            g!(out, out_width, block_x, block_y, n, rot, 2, 2, 45, 100, col);
        }
        4 => {
            g!(out, out_width, block_x, block_y, n, rot, 3, 3, 68, 100, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 2, 9, 100, col);
            g!(out, out_width, block_x, block_y, n, rot, 2, 3, 9, 100, col);
        }
        5 => {
            g!(out, out_width, block_x, block_y, n, rot, 4, 4, 86, 100, col);
            g!(out, out_width, block_x, block_y, n, rot, 4, 3, 23, 100, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 4, 23, 100, col);
        }
        _ => {
            g!(out, out_width, block_x, block_y, n, rot, 5, 5, 97, 100, col);
            g!(out, out_width, block_x, block_y, n, rot, 4, 5, 42, 100, col);
            g!(out, out_width, block_x, block_y, n, rot, 5, 4, 42, 100, col);
            g!(out, out_width, block_x, block_y, n, rot, 5, 3, 6, 100, col);
            g!(out, out_width, block_x, block_y, n, rot, 3, 5, 6, 100, col);
        }
    }
}

const CFG_EQUAL_COLOR_TOLERANCE: f64 = 30.0;
const CFG_DOMINANT_DIRECTION_THRESHOLD: f64 = 3.6;
const CFG_STEEP_DIRECTION_THRESHOLD: f64 = 2.2;

fn blend_pixel(
    n: u32,
    ker3: &Kern,
    out: &mut [u32],
    out_width: u32,
    block_x: u32,
    block_y: u32,
    rot: u32,
    blend_info: u8,
) {
    let blend = rotate_blend_info(rot, blend_info);

    if get_bottom_r(blend) < BLEND_NORMAL {
        return;
    }

    let k = rotate_kern(rot, *ker3);

    let eq = |p1: u32, p2: u32| dist_rgb(p1, p2) < CFG_EQUAL_COLOR_TOLERANCE;
    let dist = |p1: u32, p2: u32| dist_rgb(p1, p2);

    let do_line_blend = if get_bottom_r(blend) >= BLEND_DOMINANT {
        true
    } else if get_top_r(blend) != BLEND_NONE && !eq(k.e, k.g) {
        false
    } else if get_bottom_l(blend) != BLEND_NONE && !eq(k.e, k.c) {
        false
    } else if !eq(k.e, k.i) && eq(k.g, k.h) && eq(k.h, k.i) && eq(k.i, k.f) && eq(k.f, k.c) {
        false
    } else {
        true
    };

    let px = if dist(k.e, k.f) <= dist(k.e, k.h) { k.f } else { k.h };

    if do_line_blend {
        let fg = dist(k.f, k.g);
        let hc = dist(k.h, k.c);

        let have_shallow = CFG_STEEP_DIRECTION_THRESHOLD * fg <= hc && k.e != k.g && k.d != k.g;
        let have_steep = CFG_STEEP_DIRECTION_THRESHOLD * hc <= fg && k.e != k.c && k.b != k.c;

        if have_shallow && have_steep {
            blend_line_steep_and_shallow(n, out, out_width, block_x, block_y, rot, px);
        } else if have_shallow {
            blend_line_shallow(n, out, out_width, block_x, block_y, rot, px);
        } else if have_steep {
            blend_line_steep(n, out, out_width, block_x, block_y, rot, px);
        } else {
            blend_line_diagonal(n, out, out_width, block_x, block_y, rot, px);
        }
    } else {
        blend_corner(n, out, out_width, block_x, block_y, rot, px);
    }
}

fn scale_image(src: &[u32], width: usize, height: usize, n: u32) -> Vec<u32> {
    let out_width = width * n as usize;
    let out_height = height * n as usize;
    let mut out = vec![0u32; out_width * out_height];

    let mut pre_proc = vec![0u8; width];

    for y in 0..height {
        let s_m1 = y.saturating_sub(1);
        let s_0 = y;
        let s_p1 = (y + 1).min(height - 1);
        let s_p2 = (y + 2).min(height - 1);

        let mut blend_xy1: u8 = 0; 

        for x in 0..width {
            let x_m1 = x.saturating_sub(1);
            let x_p1 = (x + 1).min(width - 1);
            let x_p2 = (x + 2).min(width - 1);

            let ker4 = Kernel4 {
                a: src[s_m1 * width + x_m1],
                b: src[s_m1 * width + x],
                c: src[s_m1 * width + x_p1],
                d: src[s_m1 * width + x_p2],
                e: src[s_0 * width + x_m1],
                f: src[s_0 * width + x],
                g: src[s_0 * width + x_p1],
                h: src[s_0 * width + x_p2],
                i: src[s_p1 * width + x_m1],
                j: src[s_p1 * width + x],
                k: src[s_p1 * width + x_p1],
                l: src[s_p1 * width + x_p2],
                m: src[s_p2 * width + x_m1],
                n: src[s_p2 * width + x],
                o: src[s_p2 * width + x_p1],
                p: src[s_p2 * width + x_p2],
            };

            let mut blend_xy: u8;
            {
                let res = pre_process_corners(&ker4, CFG_DOMINANT_DIRECTION_THRESHOLD);

                blend_xy = pre_proc[x];
                blend_xy |= res.blend_f << 4;

                blend_xy1 |= res.blend_j << 2;
                pre_proc[x] = blend_xy1;

                blend_xy1 = 0;
                blend_xy1 |= res.blend_k;

                if x + 1 < width {
                    pre_proc[x + 1] |= res.blend_g << 6;
                }
            }

            let block_x = (x as u32) * n;
            let block_y = (y as u32) * n;
            for by in 0..n {
                for bx in 0..n {
                    let idx = ((block_y + by) as usize) * out_width + (block_x + bx) as usize;
                    out[idx] = ker4.f;
                }
            }

            if blend_xy != 0 {
                let ker3 = Kern {
                    a: ker4.a,
                    b: ker4.b,
                    c: ker4.c,
                    d: ker4.e,
                    e: ker4.f,
                    f: ker4.g,
                    g: ker4.i,
                    h: ker4.j,
                    i: ker4.k,
                };

                blend_pixel(n, &ker3, &mut out, out_width as u32, block_x, block_y, 0, blend_xy);
                blend_pixel(
                    n, &ker3, &mut out, out_width as u32, block_x, block_y, 90, blend_xy,
                );
                blend_pixel(
                    n, &ker3, &mut out, out_width as u32, block_x, block_y, 180, blend_xy,
                );
                blend_pixel(
                    n, &ker3, &mut out, out_width as u32, block_x, block_y, 270, blend_xy,
                );
            }
        }
    }

    out
}

pub fn xbrz(factor: u32, src: &[u32], width: usize, height: usize) -> Vec<u32> {
    match factor {
        2 => scale_image(src, width, height, 2),
        3 => scale_image(src, width, height, 3),
        4 => scale_image(src, width, height, 4),
        5 => scale_image(src, width, height, 5),
        6 => scale_image(src, width, height, 6),
        _ => Vec::new(),
    }
}
