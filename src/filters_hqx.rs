// the hq2x, hq3x and hq4x video filters by maxim stepen, cameron zemek and ported from mesen!!!
#[inline]
fn wrap_u32(x: f64) -> u32 {
    (x as i64) as u32
}

#[inline]
fn rgb_to_yuv(c: u32) -> u32 {
    let rgb = c & 0x00FF_FFFF;
    let r = (rgb >> 16) as f64;
    let g = ((rgb >> 8) & 0xFF) as f64;
    let b = (rgb & 0xFF) as f64;

    let y = (0.299 * r + 0.587 * g + 0.114 * b) as u32;
    let u = wrap_u32(-0.169 * r - 0.331 * g + 0.5 * b).wrapping_add(128);
    let v = wrap_u32(0.5 * r - 0.419 * g - 0.081 * b).wrapping_add(128);

    (y << 16) | ((u & 0xFF) << 8) | (v & 0xFF)
}

#[inline]
fn yuv_diff(yuv1: u32, yuv2: u32) -> bool {
    let dy = ((yuv1 & 0x00FF_0000) as i32 - (yuv2 & 0x00FF_0000) as i32).abs() > 0x0030_0000;
    let du = ((yuv1 & 0x0000_FF00) as i32 - (yuv2 & 0x0000_FF00) as i32).abs() > 0x0000_0700;
    let dv = ((yuv1 & 0x0000_00FF) as i32 - (yuv2 & 0x0000_00FF) as i32).abs() > 0x0000_0006;
    dy || du || dv
}

#[inline]
fn diff(c1: u32, c2: u32) -> bool {
    yuv_diff(rgb_to_yuv(c1), rgb_to_yuv(c2))
}


const MASK_2: u32 = 0x0000_FF00;
const MASK_13: u32 = 0x00FF_00FF;
const MASK_ALPHA: u32 = 0xFF00_0000;

#[inline]
fn interpolate_2(c1: u32, w1: u32, c2: u32, w2: u32, s: u32) -> u32 {
    if c1 == c2 {
        return c1;
    }
    let a1 = ((c1 & MASK_ALPHA) >> 24).wrapping_mul(w1);
    let a2 = ((c2 & MASK_ALPHA) >> 24).wrapping_mul(w2);
    let a = a1.wrapping_add(a2).wrapping_shl(24 - s) & MASK_ALPHA;
    let m2 = ((c1 & MASK_2).wrapping_mul(w1) + (c2 & MASK_2).wrapping_mul(w2)) >> s & MASK_2;
    let m13 = ((c1 & MASK_13).wrapping_mul(w1) + (c2 & MASK_13).wrapping_mul(w2)) >> s & MASK_13;
    a + m2 + m13
}

#[inline]
fn interpolate_3(c1: u32, w1: u32, c2: u32, w2: u32, c3: u32, w3: u32, s: u32) -> u32 {
    let a1 = ((c1 & MASK_ALPHA) >> 24).wrapping_mul(w1);
    let a2 = ((c2 & MASK_ALPHA) >> 24).wrapping_mul(w2);
    let a3 = ((c3 & MASK_ALPHA) >> 24).wrapping_mul(w3);
    let a = a1
        .wrapping_add(a2)
        .wrapping_add(a3)
        .wrapping_shl(24 - s)
        & MASK_ALPHA;
    let m2 = ((c1 & MASK_2).wrapping_mul(w1)
        + (c2 & MASK_2).wrapping_mul(w2)
        + (c3 & MASK_2).wrapping_mul(w3))
        >> s
        & MASK_2;
    let m13 = ((c1 & MASK_13).wrapping_mul(w1)
        + (c2 & MASK_13).wrapping_mul(w2)
        + (c3 & MASK_13).wrapping_mul(w3))
        >> s
        & MASK_13;
    a + m2 + m13
}

#[inline]
fn interp1(c1: u32, c2: u32) -> u32 {
    interpolate_2(c1, 3, c2, 1, 2)
}
#[inline]
fn interp2(c1: u32, c2: u32, c3: u32) -> u32 {
    interpolate_3(c1, 2, c2, 1, c3, 1, 2)
}
#[inline]
fn interp3(c1: u32, c2: u32) -> u32 {
    interpolate_2(c1, 7, c2, 1, 3)
}
#[inline]
fn interp4(c1: u32, c2: u32, c3: u32) -> u32 {
    interpolate_3(c1, 2, c2, 7, c3, 7, 4)
}
#[inline]
fn interp5(c1: u32, c2: u32) -> u32 {
    interpolate_2(c1, 1, c2, 1, 1)
}
#[inline]
fn interp6(c1: u32, c2: u32, c3: u32) -> u32 {
    interpolate_3(c1, 5, c2, 2, c3, 1, 3)
}
#[inline]
fn interp7(c1: u32, c2: u32, c3: u32) -> u32 {
    interpolate_3(c1, 6, c2, 1, c3, 1, 3)
}
#[inline]
fn interp8(c1: u32, c2: u32) -> u32 {
    interpolate_2(c1, 5, c2, 3, 3)
}
#[inline]
fn interp9(c1: u32, c2: u32, c3: u32) -> u32 {
    interpolate_3(c1, 2, c2, 3, c3, 3, 3)
}
#[inline]
fn interp10(c1: u32, c2: u32, c3: u32) -> u32 {
    interpolate_3(c1, 14, c2, 1, c3, 1, 4)
}

#[inline]
fn fetch_neighborhood(src: &[u32], width: usize, height: usize, x: usize, y: usize) -> [u32; 10] {
    let x = x as i32;
    let y = y as i32;
    let w = width as i32;
    let h = height as i32;

    let get = |cx: i32, cy: i32| -> u32 {
        let cx = cx.clamp(0, w - 1) as usize;
        let cy = cy.clamp(0, h - 1) as usize;
        src[cy * width + cx]
    };

    let mut n = [0u32; 10];
    n[1] = get(x - 1, y - 1);
    n[2] = get(x, y - 1);
    n[3] = get(x + 1, y - 1);
    n[4] = get(x - 1, y);
    n[5] = get(x, y);
    n[6] = get(x + 1, y);
    n[7] = get(x - 1, y + 1);
    n[8] = get(x, y + 1);
    n[9] = get(x + 1, y + 1);
    n
}

#[inline]
fn make_pattern(w: &[u32; 10]) -> u32 {
    let mut pattern = 0u32;
    let mut flag = 1u32;
    let yuv1 = rgb_to_yuv(w[5]);
    for k in 1..=9usize {
        if k == 5 {
            continue;
        }
        if w[k] != w[5] && yuv_diff(yuv1, rgb_to_yuv(w[k])) {
            pattern |= flag;
        }
        flag <<= 1;
    }
    pattern
}

// hq2x

#[inline]
fn p00_0(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn p00_10(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[1])
}
#[inline]
fn p00_11(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[4])
}
#[inline]
fn p00_12(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[2])
}
#[inline]
fn p00_20(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[4], w[2])
}
#[inline]
fn p00_21(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[1], w[2])
}
#[inline]
fn p00_22(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[1], w[4])
}
#[inline]
fn p00_60(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[2], w[4])
}
#[inline]
fn p00_61(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[4], w[2])
}
#[inline]
fn p00_70(w: &[u32; 10]) -> u32 {
    interp7(w[5], w[4], w[2])
}
#[inline]
fn p00_90(w: &[u32; 10]) -> u32 {
    interp9(w[5], w[4], w[2])
}
#[inline]
fn p00_100(w: &[u32; 10]) -> u32 {
    interp10(w[5], w[4], w[2])
}

#[inline]
fn p01_0(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn p01_10(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[3])
}
#[inline]
fn p01_11(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[2])
}
#[inline]
fn p01_12(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[6])
}
#[inline]
fn p01_20(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[2], w[6])
}
#[inline]
fn p01_21(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[3], w[6])
}
#[inline]
fn p01_22(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[3], w[2])
}
#[inline]
fn p01_60(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[6], w[2])
}
#[inline]
fn p01_61(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[2], w[6])
}
#[inline]
fn p01_70(w: &[u32; 10]) -> u32 {
    interp7(w[5], w[2], w[6])
}
#[inline]
fn p01_90(w: &[u32; 10]) -> u32 {
    interp9(w[5], w[2], w[6])
}
#[inline]
fn p01_100(w: &[u32; 10]) -> u32 {
    interp10(w[5], w[2], w[6])
}

#[inline]
fn p10_0(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn p10_10(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[7])
}
#[inline]
fn p10_11(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[8])
}
#[inline]
fn p10_12(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[4])
}
#[inline]
fn p10_20(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[8], w[4])
}
#[inline]
fn p10_21(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[7], w[4])
}
#[inline]
fn p10_22(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[7], w[8])
}
#[inline]
fn p10_60(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[4], w[8])
}
#[inline]
fn p10_61(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[8], w[4])
}
#[inline]
fn p10_70(w: &[u32; 10]) -> u32 {
    interp7(w[5], w[8], w[4])
}
#[inline]
fn p10_90(w: &[u32; 10]) -> u32 {
    interp9(w[5], w[8], w[4])
}
#[inline]
fn p10_100(w: &[u32; 10]) -> u32 {
    interp10(w[5], w[8], w[4])
}

#[inline]
fn p11_0(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn p11_10(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[9])
}
#[inline]
fn p11_11(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[6])
}
#[inline]
fn p11_12(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[8])
}
#[inline]
fn p11_20(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[6], w[8])
}
#[inline]
fn p11_21(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[9], w[8])
}
#[inline]
fn p11_22(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[9], w[6])
}
#[inline]
fn p11_60(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[8], w[6])
}
#[inline]
fn p11_61(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[6], w[8])
}
#[inline]
fn p11_70(w: &[u32; 10]) -> u32 {
    interp7(w[5], w[6], w[8])
}
#[inline]
fn p11_90(w: &[u32; 10]) -> u32 {
    interp9(w[5], w[6], w[8])
}
#[inline]
fn p11_100(w: &[u32; 10]) -> u32 {
    interp10(w[5], w[6], w[8])
}

pub fn hq2x(src: &[u32], width: usize, height: usize) -> Vec<u32> {
    let out_w = width * 2;
    let mut dst = vec![0u32; out_w * height * 2];
    let mut idx = 0usize;
    let p1 = out_w;
    let p2 = out_w + 1;

    for _j in 0..height {
        for i in 0..width {
            let w = fetch_neighborhood(src, width, height, i, _j);
            match make_pattern(&w) {
                0 | 1 | 4 | 32 | 128 | 5 | 132 | 160 | 33 | 129 | 36 | 133 | 164 | 161 | 37 | 165 => {
                    dst[idx] = p00_20(&w);
                    dst[idx + 1] = p01_20(&w);
                    dst[idx + p1] = p10_20(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                2 | 34 | 130 | 162 => {
                    dst[idx] = p00_22(&w);
                    dst[idx + 1] = p01_21(&w);
                    dst[idx + p1] = p10_20(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                16 | 17 | 48 | 49 => {
                    dst[idx] = p00_20(&w);
                    dst[idx + 1] = p01_22(&w);
                    dst[idx + p1] = p10_20(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                64 | 65 | 68 | 69 => {
                    dst[idx] = p00_20(&w);
                    dst[idx + 1] = p01_20(&w);
                    dst[idx + p1] = p10_21(&w);
                    dst[idx + p2] = p11_22(&w);
                }
                8 | 12 | 136 | 140 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_20(&w);
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                3 | 35 | 131 | 163 => {
                    dst[idx] = p00_11(&w);
                    dst[idx + 1] = p01_21(&w);
                    dst[idx + p1] = p10_20(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                6 | 38 | 134 | 166 => {
                    dst[idx] = p00_22(&w);
                    dst[idx + 1] = p01_12(&w);
                    dst[idx + p1] = p10_20(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                20 | 21 | 52 | 53 => {
                    dst[idx] = p00_20(&w);
                    dst[idx + 1] = p01_11(&w);
                    dst[idx + p1] = p10_20(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                144 | 145 | 176 | 177 => {
                    dst[idx] = p00_20(&w);
                    dst[idx + 1] = p01_22(&w);
                    dst[idx + p1] = p10_20(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                192 | 193 | 196 | 197 => {
                    dst[idx] = p00_20(&w);
                    dst[idx + 1] = p01_20(&w);
                    dst[idx + p1] = p10_21(&w);
                    dst[idx + p2] = p11_11(&w);
                }
                96 | 97 | 100 | 101 => {
                    dst[idx] = p00_20(&w);
                    dst[idx + 1] = p01_20(&w);
                    dst[idx + p1] = p10_12(&w);
                    dst[idx + p2] = p11_22(&w);
                }
                40 | 44 | 168 | 172 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_20(&w);
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                9 | 13 | 137 | 141 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_20(&w);
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                18 | 50 => {
                    dst[idx] = p00_22(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_10(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    dst[idx + p1] = p10_20(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                80 | 81 => {
                    dst[idx] = p00_20(&w);
                    dst[idx + 1] = p01_22(&w);
                    dst[idx + p1] = p10_21(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_10(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                72 | 76 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_20(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_10(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    dst[idx + p2] = p11_22(&w);
                }
                10 | 138 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_10(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    dst[idx + 1] = p01_21(&w);
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                66 => {
                    dst[idx] = p00_22(&w);
                    dst[idx + 1] = p01_21(&w);
                    dst[idx + p1] = p10_21(&w);
                    dst[idx + p2] = p11_22(&w);
                }
                24 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_22(&w);
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                7 | 39 | 135 => {
                    dst[idx] = p00_11(&w);
                    dst[idx + 1] = p01_12(&w);
                    dst[idx + p1] = p10_20(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                148 | 149 | 180 => {
                    dst[idx] = p00_20(&w);
                    dst[idx + 1] = p01_11(&w);
                    dst[idx + p1] = p10_20(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                224 | 228 | 225 => {
                    dst[idx] = p00_20(&w);
                    dst[idx + 1] = p01_20(&w);
                    dst[idx + p1] = p10_12(&w);
                    dst[idx + p2] = p11_11(&w);
                }
                41 | 169 | 45 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_20(&w);
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                22 | 54 => {
                    dst[idx] = p00_22(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    dst[idx + p1] = p10_20(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                208 | 209 => {
                    dst[idx] = p00_20(&w);
                    dst[idx + 1] = p01_22(&w);
                    dst[idx + p1] = p10_21(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                104 | 108 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_20(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    dst[idx + p2] = p11_22(&w);
                }
                11 | 139 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    dst[idx + 1] = p01_21(&w);
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                19 | 51 => {
                    if diff(w[2], w[6]) {
                        dst[idx] = p00_11(&w);
                        dst[idx + 1] = p01_10(&w);
                    } else {
                        dst[idx] = p00_60(&w);
                        dst[idx + 1] = p01_90(&w);
                    }
                    dst[idx + p1] = p10_20(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                146 | 178 => {
                    dst[idx] = p00_22(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_10(&w);
                        dst[idx + p2] = p11_12(&w);
                    } else {
                        dst[idx + 1] = p01_90(&w);
                        dst[idx + p2] = p11_61(&w);
                    }
                    dst[idx + p1] = p10_20(&w);
                }
                84 | 85 => {
                    dst[idx] = p00_20(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + 1] = p01_11(&w);
                        dst[idx + p2] = p11_10(&w);
                    } else {
                        dst[idx + 1] = p01_60(&w);
                        dst[idx + p2] = p11_90(&w);
                    }
                    dst[idx + p1] = p10_21(&w);
                }
                112 | 113 => {
                    dst[idx] = p00_20(&w);
                    dst[idx + 1] = p01_22(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p1] = p10_12(&w);
                        dst[idx + p2] = p11_10(&w);
                    } else {
                        dst[idx + p1] = p10_61(&w);
                        dst[idx + p2] = p11_90(&w);
                    }
                }
                200 | 204 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_20(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_10(&w);
                        dst[idx + p2] = p11_11(&w);
                    } else {
                        dst[idx + p1] = p10_90(&w);
                        dst[idx + p2] = p11_60(&w);
                    }
                }
                73 | 77 => {
                    if diff(w[8], w[4]) {
                        dst[idx] = p00_12(&w);
                        dst[idx + p1] = p10_10(&w);
                    } else {
                        dst[idx] = p00_61(&w);
                        dst[idx + p1] = p10_90(&w);
                    }
                    dst[idx + 1] = p01_20(&w);
                    dst[idx + p2] = p11_22(&w);
                }
                42 | 170 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_10(&w);
                        dst[idx + p1] = p10_11(&w);
                    } else {
                        dst[idx] = p00_90(&w);
                        dst[idx + p1] = p10_60(&w);
                    }
                    dst[idx + 1] = p01_21(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                14 | 142 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_10(&w);
                        dst[idx + 1] = p01_12(&w);
                    } else {
                        dst[idx] = p00_90(&w);
                        dst[idx + 1] = p01_61(&w);
                    }
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                67 => {
                    dst[idx] = p00_11(&w);
                    dst[idx + 1] = p01_21(&w);
                    dst[idx + p1] = p10_21(&w);
                    dst[idx + p2] = p11_22(&w);
                }
                70 => {
                    dst[idx] = p00_22(&w);
                    dst[idx + 1] = p01_12(&w);
                    dst[idx + p1] = p10_21(&w);
                    dst[idx + p2] = p11_22(&w);
                }
                28 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_11(&w);
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                152 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_22(&w);
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                194 => {
                    dst[idx] = p00_22(&w);
                    dst[idx + 1] = p01_21(&w);
                    dst[idx + p1] = p10_21(&w);
                    dst[idx + p2] = p11_11(&w);
                }
                98 => {
                    dst[idx] = p00_22(&w);
                    dst[idx + 1] = p01_21(&w);
                    dst[idx + p1] = p10_12(&w);
                    dst[idx + p2] = p11_22(&w);
                }
                56 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_22(&w);
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                25 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_22(&w);
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                26 | 31 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                82 | 214 => {
                    dst[idx] = p00_22(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    dst[idx + p1] = p10_21(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                88 | 248 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_22(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                74 | 107 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    dst[idx + 1] = p01_21(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    dst[idx + p2] = p11_22(&w);
                }
                27 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    dst[idx + 1] = p01_10(&w);
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                86 => {
                    dst[idx] = p00_22(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    dst[idx + p1] = p10_21(&w);
                    dst[idx + p2] = p11_10(&w);
                }
                216 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_22(&w);
                    dst[idx + p1] = p10_10(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                106 => {
                    dst[idx] = p00_10(&w);
                    dst[idx + 1] = p01_21(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    dst[idx + p2] = p11_22(&w);
                }
                30 => {
                    dst[idx] = p00_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                210 => {
                    dst[idx] = p00_22(&w);
                    dst[idx + 1] = p01_10(&w);
                    dst[idx + p1] = p10_21(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                120 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_22(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    dst[idx + p2] = p11_10(&w);
                }
                75 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    dst[idx + 1] = p01_21(&w);
                    dst[idx + p1] = p10_10(&w);
                    dst[idx + p2] = p11_22(&w);
                }
                29 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_11(&w);
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                198 => {
                    dst[idx] = p00_22(&w);
                    dst[idx + 1] = p01_12(&w);
                    dst[idx + p1] = p10_21(&w);
                    dst[idx + p2] = p11_11(&w);
                }
                184 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_22(&w);
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                99 => {
                    dst[idx] = p00_11(&w);
                    dst[idx + 1] = p01_21(&w);
                    dst[idx + p1] = p10_12(&w);
                    dst[idx + p2] = p11_22(&w);
                }
                57 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_22(&w);
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                71 => {
                    dst[idx] = p00_11(&w);
                    dst[idx + 1] = p01_12(&w);
                    dst[idx + p1] = p10_21(&w);
                    dst[idx + p2] = p11_22(&w);
                }
                156 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_11(&w);
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                226 => {
                    dst[idx] = p00_22(&w);
                    dst[idx + 1] = p01_21(&w);
                    dst[idx + p1] = p10_12(&w);
                    dst[idx + p2] = p11_11(&w);
                }
                60 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_11(&w);
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                195 => {
                    dst[idx] = p00_11(&w);
                    dst[idx + 1] = p01_21(&w);
                    dst[idx + p1] = p10_21(&w);
                    dst[idx + p2] = p11_11(&w);
                }
                102 => {
                    dst[idx] = p00_22(&w);
                    dst[idx + 1] = p01_12(&w);
                    dst[idx + p1] = p10_12(&w);
                    dst[idx + p2] = p11_22(&w);
                }
                153 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_22(&w);
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                58 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_10(&w);
                    } else {
                        dst[idx] = p00_70(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_10(&w);
                    } else {
                        dst[idx + 1] = p01_70(&w);
                    }
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                83 => {
                    dst[idx] = p00_11(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_10(&w);
                    } else {
                        dst[idx + 1] = p01_70(&w);
                    }
                    dst[idx + p1] = p10_21(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_10(&w);
                    } else {
                        dst[idx + p2] = p11_70(&w);
                    }
                }
                92 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_11(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_10(&w);
                    } else {
                        dst[idx + p1] = p10_70(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_10(&w);
                    } else {
                        dst[idx + p2] = p11_70(&w);
                    }
                }
                202 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_10(&w);
                    } else {
                        dst[idx] = p00_70(&w);
                    }
                    dst[idx + 1] = p01_21(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_10(&w);
                    } else {
                        dst[idx + p1] = p10_70(&w);
                    }
                    dst[idx + p2] = p11_11(&w);
                }
                78 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_10(&w);
                    } else {
                        dst[idx] = p00_70(&w);
                    }
                    dst[idx + 1] = p01_12(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_10(&w);
                    } else {
                        dst[idx + p1] = p10_70(&w);
                    }
                    dst[idx + p2] = p11_22(&w);
                }
                154 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_10(&w);
                    } else {
                        dst[idx] = p00_70(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_10(&w);
                    } else {
                        dst[idx + 1] = p01_70(&w);
                    }
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                114 => {
                    dst[idx] = p00_22(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_10(&w);
                    } else {
                        dst[idx + 1] = p01_70(&w);
                    }
                    dst[idx + p1] = p10_12(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_10(&w);
                    } else {
                        dst[idx + p2] = p11_70(&w);
                    }
                }
                89 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_22(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_10(&w);
                    } else {
                        dst[idx + p1] = p10_70(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_10(&w);
                    } else {
                        dst[idx + p2] = p11_70(&w);
                    }
                }
                90 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_10(&w);
                    } else {
                        dst[idx] = p00_70(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_10(&w);
                    } else {
                        dst[idx + 1] = p01_70(&w);
                    }
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_10(&w);
                    } else {
                        dst[idx + p1] = p10_70(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_10(&w);
                    } else {
                        dst[idx + p2] = p11_70(&w);
                    }
                }
                55 | 23 => {
                    if diff(w[2], w[6]) {
                        dst[idx] = p00_11(&w);
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx] = p00_60(&w);
                        dst[idx + 1] = p01_90(&w);
                    }
                    dst[idx + p1] = p10_20(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                182 | 150 => {
                    dst[idx] = p00_22(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                        dst[idx + p2] = p11_12(&w);
                    } else {
                        dst[idx + 1] = p01_90(&w);
                        dst[idx + p2] = p11_61(&w);
                    }
                    dst[idx + p1] = p10_20(&w);
                }
                213 | 212 => {
                    dst[idx] = p00_20(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + 1] = p01_11(&w);
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + 1] = p01_60(&w);
                        dst[idx + p2] = p11_90(&w);
                    }
                    dst[idx + p1] = p10_21(&w);
                }
                241 | 240 => {
                    dst[idx] = p00_20(&w);
                    dst[idx + 1] = p01_22(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p1] = p10_12(&w);
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p1] = p10_61(&w);
                        dst[idx + p2] = p11_90(&w);
                    }
                }
                236 | 232 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_20(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                        dst[idx + p2] = p11_11(&w);
                    } else {
                        dst[idx + p1] = p10_90(&w);
                        dst[idx + p2] = p11_60(&w);
                    }
                }
                109 | 105 => {
                    if diff(w[8], w[4]) {
                        dst[idx] = p00_12(&w);
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx] = p00_61(&w);
                        dst[idx + p1] = p10_90(&w);
                    }
                    dst[idx + 1] = p01_20(&w);
                    dst[idx + p2] = p11_22(&w);
                }
                171 | 43 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                        dst[idx + p1] = p10_11(&w);
                    } else {
                        dst[idx] = p00_90(&w);
                        dst[idx + p1] = p10_60(&w);
                    }
                    dst[idx + 1] = p01_21(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                143 | 15 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                        dst[idx + 1] = p01_12(&w);
                    } else {
                        dst[idx] = p00_90(&w);
                        dst[idx + 1] = p01_61(&w);
                    }
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                124 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_11(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    dst[idx + p2] = p11_10(&w);
                }
                203 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    dst[idx + 1] = p01_21(&w);
                    dst[idx + p1] = p10_10(&w);
                    dst[idx + p2] = p11_11(&w);
                }
                62 => {
                    dst[idx] = p00_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                211 => {
                    dst[idx] = p00_11(&w);
                    dst[idx + 1] = p01_10(&w);
                    dst[idx + p1] = p10_21(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                118 => {
                    dst[idx] = p00_22(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    dst[idx + p1] = p10_12(&w);
                    dst[idx + p2] = p11_10(&w);
                }
                217 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_22(&w);
                    dst[idx + p1] = p10_10(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                110 => {
                    dst[idx] = p00_10(&w);
                    dst[idx + 1] = p01_12(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    dst[idx + p2] = p11_22(&w);
                }
                155 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    dst[idx + 1] = p01_10(&w);
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                188 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_11(&w);
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                185 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_22(&w);
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                61 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_11(&w);
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                157 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_11(&w);
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                103 => {
                    dst[idx] = p00_11(&w);
                    dst[idx + 1] = p01_12(&w);
                    dst[idx + p1] = p10_12(&w);
                    dst[idx + p2] = p11_22(&w);
                }
                227 => {
                    dst[idx] = p00_11(&w);
                    dst[idx + 1] = p01_21(&w);
                    dst[idx + p1] = p10_12(&w);
                    dst[idx + p2] = p11_11(&w);
                }
                230 => {
                    dst[idx] = p00_22(&w);
                    dst[idx + 1] = p01_12(&w);
                    dst[idx + p1] = p10_12(&w);
                    dst[idx + p2] = p11_11(&w);
                }
                199 => {
                    dst[idx] = p00_11(&w);
                    dst[idx + 1] = p01_12(&w);
                    dst[idx + p1] = p10_21(&w);
                    dst[idx + p2] = p11_11(&w);
                }
                220 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_11(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_10(&w);
                    } else {
                        dst[idx + p1] = p10_70(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                158 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_10(&w);
                    } else {
                        dst[idx] = p00_70(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                234 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_10(&w);
                    } else {
                        dst[idx] = p00_70(&w);
                    }
                    dst[idx + 1] = p01_21(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    dst[idx + p2] = p11_11(&w);
                }
                242 => {
                    dst[idx] = p00_22(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_10(&w);
                    } else {
                        dst[idx + 1] = p01_70(&w);
                    }
                    dst[idx + p1] = p10_12(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                59 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_10(&w);
                    } else {
                        dst[idx + 1] = p01_70(&w);
                    }
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                121 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_22(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_10(&w);
                    } else {
                        dst[idx + p2] = p11_70(&w);
                    }
                }
                87 => {
                    dst[idx] = p00_11(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    dst[idx + p1] = p10_21(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_10(&w);
                    } else {
                        dst[idx + p2] = p11_70(&w);
                    }
                }
                79 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    dst[idx + 1] = p01_12(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_10(&w);
                    } else {
                        dst[idx + p1] = p10_70(&w);
                    }
                    dst[idx + p2] = p11_22(&w);
                }
                122 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_10(&w);
                    } else {
                        dst[idx] = p00_70(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_10(&w);
                    } else {
                        dst[idx + 1] = p01_70(&w);
                    }
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_10(&w);
                    } else {
                        dst[idx + p2] = p11_70(&w);
                    }
                }
                94 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_10(&w);
                    } else {
                        dst[idx] = p00_70(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_10(&w);
                    } else {
                        dst[idx + p1] = p10_70(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_10(&w);
                    } else {
                        dst[idx + p2] = p11_70(&w);
                    }
                }
                218 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_10(&w);
                    } else {
                        dst[idx] = p00_70(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_10(&w);
                    } else {
                        dst[idx + 1] = p01_70(&w);
                    }
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_10(&w);
                    } else {
                        dst[idx + p1] = p10_70(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                91 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_10(&w);
                    } else {
                        dst[idx + 1] = p01_70(&w);
                    }
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_10(&w);
                    } else {
                        dst[idx + p1] = p10_70(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_10(&w);
                    } else {
                        dst[idx + p2] = p11_70(&w);
                    }
                }
                229 => {
                    dst[idx] = p00_20(&w);
                    dst[idx + 1] = p01_20(&w);
                    dst[idx + p1] = p10_12(&w);
                    dst[idx + p2] = p11_11(&w);
                }
                167 => {
                    dst[idx] = p00_11(&w);
                    dst[idx + 1] = p01_12(&w);
                    dst[idx + p1] = p10_20(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                173 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_20(&w);
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                181 => {
                    dst[idx] = p00_20(&w);
                    dst[idx + 1] = p01_11(&w);
                    dst[idx + p1] = p10_20(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                186 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_10(&w);
                    } else {
                        dst[idx] = p00_70(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_10(&w);
                    } else {
                        dst[idx + 1] = p01_70(&w);
                    }
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                115 => {
                    dst[idx] = p00_11(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_10(&w);
                    } else {
                        dst[idx + 1] = p01_70(&w);
                    }
                    dst[idx + p1] = p10_12(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_10(&w);
                    } else {
                        dst[idx + p2] = p11_70(&w);
                    }
                }
                93 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_11(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_10(&w);
                    } else {
                        dst[idx + p1] = p10_70(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_10(&w);
                    } else {
                        dst[idx + p2] = p11_70(&w);
                    }
                }
                206 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_10(&w);
                    } else {
                        dst[idx] = p00_70(&w);
                    }
                    dst[idx + 1] = p01_12(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_10(&w);
                    } else {
                        dst[idx + p1] = p10_70(&w);
                    }
                    dst[idx + p2] = p11_11(&w);
                }
                205 | 201 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_20(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_10(&w);
                    } else {
                        dst[idx + p1] = p10_70(&w);
                    }
                    dst[idx + p2] = p11_11(&w);
                }
                174 | 46 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_10(&w);
                    } else {
                        dst[idx] = p00_70(&w);
                    }
                    dst[idx + 1] = p01_12(&w);
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                179 | 147 => {
                    dst[idx] = p00_11(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_10(&w);
                    } else {
                        dst[idx + 1] = p01_70(&w);
                    }
                    dst[idx + p1] = p10_20(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                117 | 116 => {
                    dst[idx] = p00_20(&w);
                    dst[idx + 1] = p01_11(&w);
                    dst[idx + p1] = p10_12(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_10(&w);
                    } else {
                        dst[idx + p2] = p11_70(&w);
                    }
                }
                189 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_11(&w);
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                231 => {
                    dst[idx] = p00_11(&w);
                    dst[idx + 1] = p01_12(&w);
                    dst[idx + p1] = p10_12(&w);
                    dst[idx + p2] = p11_11(&w);
                }
                126 => {
                    dst[idx] = p00_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    dst[idx + p2] = p11_10(&w);
                }
                219 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    dst[idx + 1] = p01_10(&w);
                    dst[idx + p1] = p10_10(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                125 => {
                    if diff(w[8], w[4]) {
                        dst[idx] = p00_12(&w);
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx] = p00_61(&w);
                        dst[idx + p1] = p10_90(&w);
                    }
                    dst[idx + 1] = p01_11(&w);
                    dst[idx + p2] = p11_10(&w);
                }
                221 => {
                    dst[idx] = p00_12(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + 1] = p01_11(&w);
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + 1] = p01_60(&w);
                        dst[idx + p2] = p11_90(&w);
                    }
                    dst[idx + p1] = p10_10(&w);
                }
                207 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                        dst[idx + 1] = p01_12(&w);
                    } else {
                        dst[idx] = p00_90(&w);
                        dst[idx + 1] = p01_61(&w);
                    }
                    dst[idx + p1] = p10_10(&w);
                    dst[idx + p2] = p11_11(&w);
                }
                238 => {
                    dst[idx] = p00_10(&w);
                    dst[idx + 1] = p01_12(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                        dst[idx + p2] = p11_11(&w);
                    } else {
                        dst[idx + p1] = p10_90(&w);
                        dst[idx + p2] = p11_60(&w);
                    }
                }
                190 => {
                    dst[idx] = p00_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                        dst[idx + p2] = p11_12(&w);
                    } else {
                        dst[idx + 1] = p01_90(&w);
                        dst[idx + p2] = p11_61(&w);
                    }
                    dst[idx + p1] = p10_11(&w);
                }
                187 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                        dst[idx + p1] = p10_11(&w);
                    } else {
                        dst[idx] = p00_90(&w);
                        dst[idx + p1] = p10_60(&w);
                    }
                    dst[idx + 1] = p01_10(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                243 => {
                    dst[idx] = p00_11(&w);
                    dst[idx + 1] = p01_10(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p1] = p10_12(&w);
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p1] = p10_61(&w);
                        dst[idx + p2] = p11_90(&w);
                    }
                }
                119 => {
                    if diff(w[2], w[6]) {
                        dst[idx] = p00_11(&w);
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx] = p00_60(&w);
                        dst[idx + 1] = p01_90(&w);
                    }
                    dst[idx + p1] = p10_12(&w);
                    dst[idx + p2] = p11_10(&w);
                }
                237 | 233 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_20(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_100(&w);
                    }
                    dst[idx + p2] = p11_11(&w);
                }
                175 | 47 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_100(&w);
                    }
                    dst[idx + 1] = p01_12(&w);
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_20(&w);
                }
                183 | 151 => {
                    dst[idx] = p00_11(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_100(&w);
                    }
                    dst[idx + p1] = p10_20(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                245 | 244 => {
                    dst[idx] = p00_20(&w);
                    dst[idx + 1] = p01_11(&w);
                    dst[idx + p1] = p10_12(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_100(&w);
                    }
                }
                250 => {
                    dst[idx] = p00_10(&w);
                    dst[idx + 1] = p01_10(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                123 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    dst[idx + 1] = p01_10(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    dst[idx + p2] = p11_10(&w);
                }
                95 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    dst[idx + p1] = p10_10(&w);
                    dst[idx + p2] = p11_10(&w);
                }
                222 => {
                    dst[idx] = p00_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    dst[idx + p1] = p10_10(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                252 => {
                    dst[idx] = p00_21(&w);
                    dst[idx + 1] = p01_11(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_100(&w);
                    }
                }
                249 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_22(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_100(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                235 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    dst[idx + 1] = p01_21(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_100(&w);
                    }
                    dst[idx + p2] = p11_11(&w);
                }
                111 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_100(&w);
                    }
                    dst[idx + 1] = p01_12(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    dst[idx + p2] = p11_22(&w);
                }
                63 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_100(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_21(&w);
                }
                159 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_100(&w);
                    }
                    dst[idx + p1] = p10_22(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                215 => {
                    dst[idx] = p00_11(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_100(&w);
                    }
                    dst[idx + p1] = p10_21(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                246 => {
                    dst[idx] = p00_22(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    dst[idx + p1] = p10_12(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_100(&w);
                    }
                }
                254 => {
                    dst[idx] = p00_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_100(&w);
                    }
                }
                253 => {
                    dst[idx] = p00_12(&w);
                    dst[idx + 1] = p01_11(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_100(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_100(&w);
                    }
                }
                251 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    dst[idx + 1] = p01_10(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_100(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                239 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_100(&w);
                    }
                    dst[idx + 1] = p01_12(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_100(&w);
                    }
                    dst[idx + p2] = p11_11(&w);
                }
                127 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_100(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_20(&w);
                    }
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_20(&w);
                    }
                    dst[idx + p2] = p11_10(&w);
                }
                191 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_100(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_100(&w);
                    }
                    dst[idx + p1] = p10_11(&w);
                    dst[idx + p2] = p11_12(&w);
                }
                223 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_20(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_100(&w);
                    }
                    dst[idx + p1] = p10_10(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_20(&w);
                    }
                }
                247 => {
                    dst[idx] = p00_11(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_100(&w);
                    }
                    dst[idx + p1] = p10_12(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_100(&w);
                    }
                }
                255 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_0(&w);
                    } else {
                        dst[idx] = p00_100(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_0(&w);
                    } else {
                        dst[idx + 1] = p01_100(&w);
                    }
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_0(&w);
                    } else {
                        dst[idx + p1] = p10_100(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2] = p11_0(&w);
                    } else {
                        dst[idx + p2] = p11_100(&w);
                    }
                }
                _ => {}
            }
            idx += 2;
        }
        idx += out_w;
    }
    dst
}

// ---------------------------------------------------------------------------
// HQ3x
// ---------------------------------------------------------------------------

// PIXEL helpers. Each returns the colour for one of the nine 3x3 block
// positions, mirroring the reference macros (00=top-left .. 22=bottom-right,
// PIXEL11 is always the centre colour w[5]).

#[inline]
fn p00_1m(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[1])
}
#[inline]
fn p00_1u(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[2])
}
#[inline]
fn p00_1l(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[4])
}
#[inline]
fn p00_2(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[4], w[2])
}
#[inline]
fn p00_4(w: &[u32; 10]) -> u32 {
    interp4(w[5], w[4], w[2])
}
#[inline]
fn p00_5(w: &[u32; 10]) -> u32 {
    interp5(w[4], w[2])
}
#[inline]
fn p00_c(w: &[u32; 10]) -> u32 {
    w[5]
}

#[inline]
fn p01_1(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[2])
}
#[inline]
fn p01_3(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[2])
}
#[inline]
fn p01_6(w: &[u32; 10]) -> u32 {
    interp1(w[2], w[5])
}
#[inline]
fn p01_c(w: &[u32; 10]) -> u32 {
    w[5]
}

#[inline]
fn p02_1m(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[3])
}
#[inline]
fn p02_1u(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[2])
}
#[inline]
fn p02_1r(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[6])
}
#[inline]
fn p02_2(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[2], w[6])
}
#[inline]
fn p02_4(w: &[u32; 10]) -> u32 {
    interp4(w[5], w[2], w[6])
}
#[inline]
fn p02_5(w: &[u32; 10]) -> u32 {
    interp5(w[2], w[6])
}
#[inline]
fn p02_c(w: &[u32; 10]) -> u32 {
    w[5]
}

#[inline]
fn p10_1(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[4])
}
#[inline]
fn p10_3(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[4])
}
#[inline]
fn p10_6(w: &[u32; 10]) -> u32 {
    interp1(w[4], w[5])
}
#[inline]
fn p10_c(w: &[u32; 10]) -> u32 {
    w[5]
}

#[inline]
fn p11(w: &[u32; 10]) -> u32 {
    w[5]
}

#[inline]
fn p12_1(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[6])
}
#[inline]
fn p12_3(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[6])
}
#[inline]
fn p12_6(w: &[u32; 10]) -> u32 {
    interp1(w[6], w[5])
}
#[inline]
fn p12_c(w: &[u32; 10]) -> u32 {
    w[5]
}

#[inline]
fn p20_1m(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[7])
}
#[inline]
fn p20_1d(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[8])
}
#[inline]
fn p20_1l(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[4])
}
#[inline]
fn p20_2(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[8], w[4])
}
#[inline]
fn p20_4(w: &[u32; 10]) -> u32 {
    interp4(w[5], w[8], w[4])
}
#[inline]
fn p20_5(w: &[u32; 10]) -> u32 {
    interp5(w[8], w[4])
}
#[inline]
fn p20_c(w: &[u32; 10]) -> u32 {
    w[5]
}

#[inline]
fn p21_1(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[8])
}
#[inline]
fn p21_3(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[8])
}
#[inline]
fn p21_6(w: &[u32; 10]) -> u32 {
    interp1(w[8], w[5])
}
#[inline]
fn p21_c(w: &[u32; 10]) -> u32 {
    w[5]
}

#[inline]
fn p22_1m(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[9])
}
#[inline]
fn p22_1d(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[8])
}
#[inline]
fn p22_1r(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[6])
}
#[inline]
fn p22_2(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[6], w[8])
}
#[inline]
fn p22_4(w: &[u32; 10]) -> u32 {
    interp4(w[5], w[6], w[8])
}
#[inline]
fn p22_5(w: &[u32; 10]) -> u32 {
    interp5(w[6], w[8])
}
#[inline]
fn p22_c(w: &[u32; 10]) -> u32 {
    w[5]
}

/// Scale a row-major 0xAARRGGBB image by 3x using the HQ3x algorithm.
pub fn hq3x(src: &[u32], width: usize, height: usize) -> Vec<u32> {
    let out_w = width * 3;
    let mut dst = vec![0u32; out_w * height * 3];
    let mut idx = 0usize;
    let p1 = out_w;
    let p2 = out_w * 2;

    for _j in 0..height {
        for i in 0..width {
            let w = fetch_neighborhood(src, width, height, i, _j);
            match make_pattern(&w) {
                0 | 1 | 4 | 32 | 128 | 5 | 132 | 160 | 33 | 129 | 36 | 133 | 164 | 161 | 37 | 165 => {
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                2 | 34 | 130 | 162 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                16 | 17 | 48 | 49 => {
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                64 | 65 | 68 | 69 => {
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                8 | 12 | 136 | 140 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                3 | 35 | 131 | 163 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                6 | 38 | 134 | 166 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                20 | 21 | 52 | 53 => {
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                144 | 145 | 176 | 177 => {
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                192 | 193 | 196 | 197 => {
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                96 | 97 | 100 | 101 => {
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                40 | 44 | 168 | 172 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                9 | 13 | 137 | 141 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                18 | 50 => {
                    dst[idx] = p00_1m(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_1m(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + 2] = p02_4(&w);
                        dst[idx + p1 + 2] = p12_3(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                80 | 81 => {
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1m(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_1m(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_3(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                72 | 76 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_1m(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx + p1] = p10_3(&w);
                        dst[idx + p2] = p20_4(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                    }
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                10 | 138 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_1m(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + p1] = p10_3(&w);
                    }
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                66 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                24 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                7 | 39 | 135 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                148 | 149 | 180 => {
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                224 | 228 | 225 => {
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                41 | 169 | 45 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                22 | 54 => {
                    dst[idx] = p00_1m(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + 2] = p02_4(&w);
                        dst[idx + p1 + 2] = p12_3(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                208 | 209 => {
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1m(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_3(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                104 | 108 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx + p1] = p10_3(&w);
                        dst[idx + p2] = p20_4(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                    }
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                11 | 139 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + p1] = p10_3(&w);
                    }
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                19 | 51 => {
                    if diff(w[2], w[6]) {
                        dst[idx] = p00_1l(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_1m(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                        dst[idx + 1] = p01_6(&w);
                        dst[idx + 2] = p02_5(&w);
                        dst[idx + p1 + 2] = p12_1(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                146 | 178 => {
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_1m(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 2] = p22_1d(&w);
                    } else {
                        dst[idx + 1] = p01_1(&w);
                        dst[idx + 2] = p02_5(&w);
                        dst[idx + p1 + 2] = p12_6(&w);
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                    dst[idx] = p00_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                }
                84 | 85 => {
                    if diff(w[6], w[8]) {
                        dst[idx + 2] = p02_1u(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_1m(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                        dst[idx + p1 + 2] = p12_6(&w);
                        dst[idx + p2 + 1] = p21_1(&w);
                        dst[idx + p2 + 2] = p22_5(&w);
                    }
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1m(&w);
                }
                112 | 113 => {
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2] = p20_1l(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_1m(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_1(&w);
                        dst[idx + p2] = p20_2(&w);
                        dst[idx + p2 + 1] = p21_6(&w);
                        dst[idx + p2 + 2] = p22_5(&w);
                    }
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                }
                200 | 204 => {
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_1m(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_1r(&w);
                    } else {
                        dst[idx + p1] = p10_1(&w);
                        dst[idx + p2] = p20_5(&w);
                        dst[idx + p2 + 1] = p21_6(&w);
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                }
                73 | 77 => {
                    if diff(w[8], w[4]) {
                        dst[idx] = p00_1u(&w);
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_1m(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                        dst[idx + p1] = p10_6(&w);
                        dst[idx + p2] = p20_5(&w);
                        dst[idx + p2 + 1] = p21_1(&w);
                    }
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                42 | 170 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_1m(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_1d(&w);
                    } else {
                        dst[idx] = p00_5(&w);
                        dst[idx + 1] = p01_1(&w);
                        dst[idx + p1] = p10_6(&w);
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                14 | 142 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_1m(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_1r(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_5(&w);
                        dst[idx + 1] = p01_6(&w);
                        dst[idx + 2] = p02_2(&w);
                        dst[idx + p1] = p10_1(&w);
                    }
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                67 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                70 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                28 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                152 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                194 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                98 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                56 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                25 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                26 | 31 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + p1] = p10_3(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx + 2] = p02_4(&w);
                        dst[idx + p1 + 2] = p12_3(&w);
                    }
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                82 | 214 => {
                    dst[idx] = p00_1m(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                    } else {
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + 2] = p02_4(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1m(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p2 + 1] = p21_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                88 | 248 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                    } else {
                        dst[idx + p1] = p10_3(&w);
                        dst[idx + p2] = p20_4(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                74 | 107 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + 1] = p01_3(&w);
                    }
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx + p2] = p20_4(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                    }
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                27 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + p1] = p10_3(&w);
                    }
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                86 => {
                    dst[idx] = p00_1m(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + 2] = p02_4(&w);
                        dst[idx + p1 + 2] = p12_3(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                216 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1m(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_3(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                106 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx + p1] = p10_3(&w);
                        dst[idx + p2] = p20_4(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                    }
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                30 => {
                    dst[idx] = p00_1m(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + 2] = p02_4(&w);
                        dst[idx + p1 + 2] = p12_3(&w);
                    }
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                210 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1m(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_3(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                120 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx + p1] = p10_3(&w);
                        dst[idx + p2] = p20_4(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                    }
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                75 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + p1] = p10_3(&w);
                    }
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                29 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                198 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                184 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                99 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                57 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                71 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                156 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                226 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                60 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                195 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                102 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                153 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                58 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_1m(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_1m(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                83 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_1m(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_1m(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                92 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_1m(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_1m(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                202 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_1m(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_1m(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                78 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_1m(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_1m(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                154 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_1m(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_1m(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                114 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_1m(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_1m(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                89 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_1m(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_1m(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                90 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_1m(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_1m(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_1m(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_1m(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                55 | 23 => {
                    if diff(w[2], w[6]) {
                        dst[idx] = p00_1l(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                        dst[idx + 1] = p01_6(&w);
                        dst[idx + 2] = p02_5(&w);
                        dst[idx + p1 + 2] = p12_1(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                182 | 150 => {
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 2] = p22_1d(&w);
                    } else {
                        dst[idx + 1] = p01_1(&w);
                        dst[idx + 2] = p02_5(&w);
                        dst[idx + p1 + 2] = p12_6(&w);
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                    dst[idx] = p00_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                }
                213 | 212 => {
                    if diff(w[6], w[8]) {
                        dst[idx + 2] = p02_1u(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                        dst[idx + p1 + 2] = p12_6(&w);
                        dst[idx + p2 + 1] = p21_1(&w);
                        dst[idx + p2 + 2] = p22_5(&w);
                    }
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1m(&w);
                }
                241 | 240 => {
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2] = p20_1l(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_1(&w);
                        dst[idx + p2] = p20_2(&w);
                        dst[idx + p2 + 1] = p21_6(&w);
                        dst[idx + p2 + 2] = p22_5(&w);
                    }
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                }
                236 | 232 => {
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_1r(&w);
                    } else {
                        dst[idx + p1] = p10_1(&w);
                        dst[idx + p2] = p20_5(&w);
                        dst[idx + p2 + 1] = p21_6(&w);
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                }
                109 | 105 => {
                    if diff(w[8], w[4]) {
                        dst[idx] = p00_1u(&w);
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                        dst[idx + p1] = p10_6(&w);
                        dst[idx + p2] = p20_5(&w);
                        dst[idx + p2 + 1] = p21_1(&w);
                    }
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                171 | 43 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_1d(&w);
                    } else {
                        dst[idx] = p00_5(&w);
                        dst[idx + 1] = p01_1(&w);
                        dst[idx + p1] = p10_6(&w);
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                143 | 15 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_1r(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_5(&w);
                        dst[idx + 1] = p01_6(&w);
                        dst[idx + 2] = p02_2(&w);
                        dst[idx + p1] = p10_1(&w);
                    }
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                124 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx + p1] = p10_3(&w);
                        dst[idx + p2] = p20_4(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                    }
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                203 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + p1] = p10_3(&w);
                    }
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                62 => {
                    dst[idx] = p00_1m(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + 2] = p02_4(&w);
                        dst[idx + p1 + 2] = p12_3(&w);
                    }
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                211 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1m(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_3(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                118 => {
                    dst[idx] = p00_1m(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + 2] = p02_4(&w);
                        dst[idx + p1 + 2] = p12_3(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                217 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1m(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_3(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                110 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx + p1] = p10_3(&w);
                        dst[idx + p2] = p20_4(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                    }
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                155 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + p1] = p10_3(&w);
                    }
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                188 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                185 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                61 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                157 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                103 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                227 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                230 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                199 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                220 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_1m(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_3(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                158 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_1m(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + 2] = p02_4(&w);
                        dst[idx + p1 + 2] = p12_3(&w);
                    }
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                234 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_1m(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx + p1] = p10_3(&w);
                        dst[idx + p2] = p20_4(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                    }
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                242 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_1m(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1l(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_3(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                59 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + p1] = p10_3(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_1m(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                121 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx + p1] = p10_3(&w);
                        dst[idx + p2] = p20_4(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_1m(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                87 => {
                    dst[idx] = p00_1l(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + 2] = p02_4(&w);
                        dst[idx + p1 + 2] = p12_3(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_1m(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                79 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + p1] = p10_3(&w);
                    }
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_1m(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                122 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_1m(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_1m(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx + p1] = p10_3(&w);
                        dst[idx + p2] = p20_4(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_1m(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                94 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_1m(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + 2] = p02_4(&w);
                        dst[idx + p1 + 2] = p12_3(&w);
                    }
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_1m(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_1m(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                218 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_1m(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_1m(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_1m(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_3(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                91 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + p1] = p10_3(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_1m(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_1m(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_1m(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                229 => {
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                167 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                173 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                181 => {
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                186 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_1m(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_1m(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                115 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_1m(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_1m(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                93 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_1m(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_1m(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                206 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_1m(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_1m(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                205 | 201 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_1m(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                174 | 46 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_1m(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                179 | 147 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_1m(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                117 | 116 => {
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_1m(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                189 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                231 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                126 => {
                    dst[idx] = p00_1m(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + 2] = p02_4(&w);
                        dst[idx + p1 + 2] = p12_3(&w);
                    }
                    dst[idx + p1 + 1] = p11(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx + p1] = p10_3(&w);
                        dst[idx + p2] = p20_4(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                    }
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                219 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + p1] = p10_3(&w);
                    }
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1m(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_3(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                125 => {
                    if diff(w[8], w[4]) {
                        dst[idx] = p00_1u(&w);
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                        dst[idx + p1] = p10_6(&w);
                        dst[idx + p2] = p20_5(&w);
                        dst[idx + p2 + 1] = p21_1(&w);
                    }
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                221 => {
                    if diff(w[6], w[8]) {
                        dst[idx + 2] = p02_1u(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                        dst[idx + p1 + 2] = p12_6(&w);
                        dst[idx + p2 + 1] = p21_1(&w);
                        dst[idx + p2 + 2] = p22_5(&w);
                    }
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1m(&w);
                }
                207 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_1r(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_5(&w);
                        dst[idx + 1] = p01_6(&w);
                        dst[idx + 2] = p02_2(&w);
                        dst[idx + p1] = p10_1(&w);
                    }
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                238 => {
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_1r(&w);
                    } else {
                        dst[idx + p1] = p10_1(&w);
                        dst[idx + p2] = p20_5(&w);
                        dst[idx + p2 + 1] = p21_6(&w);
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                }
                190 => {
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 2] = p22_1d(&w);
                    } else {
                        dst[idx + 1] = p01_1(&w);
                        dst[idx + 2] = p02_5(&w);
                        dst[idx + p1 + 2] = p12_6(&w);
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                    dst[idx] = p00_1m(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                }
                187 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_1d(&w);
                    } else {
                        dst[idx] = p00_5(&w);
                        dst[idx + 1] = p01_1(&w);
                        dst[idx + p1] = p10_6(&w);
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                243 => {
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2] = p20_1l(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_1(&w);
                        dst[idx + p2] = p20_2(&w);
                        dst[idx + p2 + 1] = p21_6(&w);
                        dst[idx + p2 + 2] = p22_5(&w);
                    }
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                }
                119 => {
                    if diff(w[2], w[6]) {
                        dst[idx] = p00_1l(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                        dst[idx + 1] = p01_6(&w);
                        dst[idx + 2] = p02_5(&w);
                        dst[idx + p1 + 2] = p12_1(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                237 | 233 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_2(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_c(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                175 | 47 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_2(&w);
                }
                183 | 151 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_c(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_2(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                245 | 244 => {
                    dst[idx] = p00_2(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                250 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                    } else {
                        dst[idx + p1] = p10_3(&w);
                        dst[idx + p2] = p20_4(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                123 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + 1] = p01_3(&w);
                    }
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx + p2] = p20_4(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                    }
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                95 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + p1] = p10_3(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx + 2] = p02_4(&w);
                        dst[idx + p1 + 2] = p12_3(&w);
                    }
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                222 => {
                    dst[idx] = p00_1m(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                    } else {
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + 2] = p02_4(&w);
                    }
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1m(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p2 + 1] = p21_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                252 => {
                    dst[idx] = p00_1m(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                    } else {
                        dst[idx + p1] = p10_3(&w);
                        dst[idx + p2] = p20_4(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                249 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_c(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                235 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + 1] = p01_3(&w);
                    }
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_c(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                111 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx + p2] = p20_4(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                    }
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                63 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx + 2] = p02_4(&w);
                        dst[idx + p1 + 2] = p12_3(&w);
                    }
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                159 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + p1] = p10_3(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_c(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1m(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                215 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_c(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1m(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p2 + 1] = p21_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                246 => {
                    dst[idx] = p00_1m(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                    } else {
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + 2] = p02_4(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                254 => {
                    dst[idx] = p00_1m(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                    } else {
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + 2] = p02_4(&w);
                    }
                    dst[idx + p1 + 1] = p11(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                    } else {
                        dst[idx + p1] = p10_3(&w);
                        dst[idx + p2] = p20_4(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_3(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                253 => {
                    dst[idx] = p00_1u(&w);
                    dst[idx + 1] = p01_1(&w);
                    dst[idx + 2] = p02_1u(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_c(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                251 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + 1] = p01_3(&w);
                    }
                    dst[idx + 2] = p02_1m(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p1] = p10_c(&w);
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx + p1] = p10_3(&w);
                        dst[idx + p2] = p20_2(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p1 + 2] = p12_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p1 + 2] = p12_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                239 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    dst[idx + 2] = p02_1r(&w);
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_1(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_c(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    dst[idx + p2 + 2] = p22_1r(&w);
                }
                127 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + p1] = p10_3(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx + 2] = p02_4(&w);
                        dst[idx + p1 + 2] = p12_3(&w);
                    }
                    dst[idx + p1 + 1] = p11(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_c(&w);
                        dst[idx + p2 + 1] = p21_c(&w);
                    } else {
                        dst[idx + p2] = p20_4(&w);
                        dst[idx + p2 + 1] = p21_3(&w);
                    }
                    dst[idx + p2 + 2] = p22_1m(&w);
                }
                191 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_c(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1d(&w);
                    dst[idx + p2 + 1] = p21_1(&w);
                    dst[idx + p2 + 2] = p22_1d(&w);
                }
                223 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                        dst[idx + p1] = p10_c(&w);
                    } else {
                        dst[idx] = p00_4(&w);
                        dst[idx + p1] = p10_3(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 1] = p01_c(&w);
                        dst[idx + 2] = p02_c(&w);
                        dst[idx + p1 + 2] = p12_c(&w);
                    } else {
                        dst[idx + 1] = p01_3(&w);
                        dst[idx + 2] = p02_2(&w);
                        dst[idx + p1 + 2] = p12_3(&w);
                    }
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p2] = p20_1m(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 1] = p21_c(&w);
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p2 + 1] = p21_3(&w);
                        dst[idx + p2 + 2] = p22_4(&w);
                    }
                }
                247 => {
                    dst[idx] = p00_1l(&w);
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_c(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1] = p10_1(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    dst[idx + p2] = p20_1l(&w);
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                255 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = p00_c(&w);
                    } else {
                        dst[idx] = p00_2(&w);
                    }
                    dst[idx + 1] = p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = p02_c(&w);
                    } else {
                        dst[idx + 2] = p02_2(&w);
                    }
                    dst[idx + p1] = p10_c(&w);
                    dst[idx + p1 + 1] = p11(&w);
                    dst[idx + p1 + 2] = p12_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = p20_c(&w);
                    } else {
                        dst[idx + p2] = p20_2(&w);
                    }
                    dst[idx + p2 + 1] = p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = p22_c(&w);
                    } else {
                        dst[idx + p2 + 2] = p22_2(&w);
                    }
                }
                _ => {}
            }
            idx += 3;
        }
        idx += p2;
    }
    dst
}

// hq4x

#[inline]
fn q4_p00_c(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn q4_p00_11(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[4])
}
#[inline]
fn q4_p00_12(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[2])
}
#[inline]
fn q4_p00_20(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[2], w[4])
}
#[inline]
fn q4_p00_50(w: &[u32; 10]) -> u32 {
    interp5(w[2], w[4])
}
#[inline]
fn q4_p00_80(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[1])
}
#[inline]
fn q4_p00_81(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[4])
}
#[inline]
fn q4_p00_82(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[2])
}

#[inline]
fn q4_p01_c(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn q4_p01_10(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[1])
}
#[inline]
fn q4_p01_12(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[2])
}
#[inline]
fn q4_p01_14(w: &[u32; 10]) -> u32 {
    interp1(w[2], w[5])
}
#[inline]
fn q4_p01_21(w: &[u32; 10]) -> u32 {
    interp2(w[2], w[5], w[4])
}
#[inline]
fn q4_p01_31(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[4])
}
#[inline]
fn q4_p01_50(w: &[u32; 10]) -> u32 {
    interp5(w[2], w[5])
}
#[inline]
fn q4_p01_60(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[2], w[4])
}
#[inline]
fn q4_p01_61(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[2], w[1])
}
#[inline]
fn q4_p01_82(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[2])
}
#[inline]
fn q4_p01_83(w: &[u32; 10]) -> u32 {
    interp8(w[2], w[4])
}

#[inline]
fn q4_p02_c(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn q4_p02_10(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[3])
}
#[inline]
fn q4_p02_11(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[2])
}
#[inline]
fn q4_p02_13(w: &[u32; 10]) -> u32 {
    interp1(w[2], w[5])
}
#[inline]
fn q4_p02_21(w: &[u32; 10]) -> u32 {
    interp2(w[2], w[5], w[6])
}
#[inline]
fn q4_p02_32(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[6])
}
#[inline]
fn q4_p02_50(w: &[u32; 10]) -> u32 {
    interp5(w[2], w[5])
}
#[inline]
fn q4_p02_60(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[2], w[6])
}
#[inline]
fn q4_p02_61(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[2], w[3])
}
#[inline]
fn q4_p02_81(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[2])
}
#[inline]
fn q4_p02_83(w: &[u32; 10]) -> u32 {
    interp8(w[2], w[6])
}

#[inline]
fn q4_p03_c(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn q4_p03_11(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[2])
}
#[inline]
fn q4_p03_12(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[6])
}
#[inline]
fn q4_p03_20(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[2], w[6])
}
#[inline]
fn q4_p03_50(w: &[u32; 10]) -> u32 {
    interp5(w[2], w[6])
}
#[inline]
fn q4_p03_80(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[3])
}
#[inline]
fn q4_p03_81(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[2])
}
#[inline]
fn q4_p03_82(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[6])
}

#[inline]
fn q4_p10_c(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn q4_p10_10(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[1])
}
#[inline]
fn q4_p10_11(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[4])
}
#[inline]
fn q4_p10_13(w: &[u32; 10]) -> u32 {
    interp1(w[4], w[5])
}
#[inline]
fn q4_p10_21(w: &[u32; 10]) -> u32 {
    interp2(w[4], w[5], w[2])
}
#[inline]
fn q4_p10_32(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[2])
}
#[inline]
fn q4_p10_50(w: &[u32; 10]) -> u32 {
    interp5(w[4], w[5])
}
#[inline]
fn q4_p10_60(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[4], w[2])
}
#[inline]
fn q4_p10_61(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[4], w[1])
}
#[inline]
fn q4_p10_81(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[4])
}
#[inline]
fn q4_p10_83(w: &[u32; 10]) -> u32 {
    interp8(w[4], w[2])
}

#[inline]
fn q4_p11_c(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn q4_p11_30(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[1])
}
#[inline]
fn q4_p11_31(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[4])
}
#[inline]
fn q4_p11_32(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[2])
}
#[inline]
fn q4_p11_70(w: &[u32; 10]) -> u32 {
    interp7(w[5], w[4], w[2])
}

#[inline]
fn q4_p12_c(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn q4_p12_30(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[3])
}
#[inline]
fn q4_p12_31(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[2])
}
#[inline]
fn q4_p12_32(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[6])
}
#[inline]
fn q4_p12_70(w: &[u32; 10]) -> u32 {
    interp7(w[5], w[6], w[2])
}

#[inline]
fn q4_p13_c(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn q4_p13_10(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[3])
}
#[inline]
fn q4_p13_12(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[6])
}
#[inline]
fn q4_p13_14(w: &[u32; 10]) -> u32 {
    interp1(w[6], w[5])
}
#[inline]
fn q4_p13_21(w: &[u32; 10]) -> u32 {
    interp2(w[6], w[5], w[2])
}
#[inline]
fn q4_p13_31(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[2])
}
#[inline]
fn q4_p13_50(w: &[u32; 10]) -> u32 {
    interp5(w[6], w[5])
}
#[inline]
fn q4_p13_60(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[6], w[2])
}
#[inline]
fn q4_p13_61(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[6], w[3])
}
#[inline]
fn q4_p13_82(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[6])
}
#[inline]
fn q4_p13_83(w: &[u32; 10]) -> u32 {
    interp8(w[6], w[2])
}

#[inline]
fn q4_p20_c(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn q4_p20_10(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[7])
}
#[inline]
fn q4_p20_12(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[4])
}
#[inline]
fn q4_p20_14(w: &[u32; 10]) -> u32 {
    interp1(w[4], w[5])
}
#[inline]
fn q4_p20_21(w: &[u32; 10]) -> u32 {
    interp2(w[4], w[5], w[8])
}
#[inline]
fn q4_p20_31(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[8])
}
#[inline]
fn q4_p20_50(w: &[u32; 10]) -> u32 {
    interp5(w[4], w[5])
}
#[inline]
fn q4_p20_60(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[4], w[8])
}
#[inline]
fn q4_p20_61(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[4], w[7])
}
#[inline]
fn q4_p20_82(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[4])
}
#[inline]
fn q4_p20_83(w: &[u32; 10]) -> u32 {
    interp8(w[4], w[8])
}

#[inline]
fn q4_p21_c(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn q4_p21_30(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[7])
}
#[inline]
fn q4_p21_31(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[8])
}
#[inline]
fn q4_p21_32(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[4])
}
#[inline]
fn q4_p21_70(w: &[u32; 10]) -> u32 {
    interp7(w[5], w[4], w[8])
}

#[inline]
fn q4_p22_c(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn q4_p22_30(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[9])
}
#[inline]
fn q4_p22_31(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[6])
}
#[inline]
fn q4_p22_32(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[8])
}
#[inline]
fn q4_p22_70(w: &[u32; 10]) -> u32 {
    interp7(w[5], w[6], w[8])
}

#[inline]
fn q4_p23_c(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn q4_p23_10(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[9])
}
#[inline]
fn q4_p23_11(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[6])
}
#[inline]
fn q4_p23_13(w: &[u32; 10]) -> u32 {
    interp1(w[6], w[5])
}
#[inline]
fn q4_p23_21(w: &[u32; 10]) -> u32 {
    interp2(w[6], w[5], w[8])
}
#[inline]
fn q4_p23_32(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[8])
}
#[inline]
fn q4_p23_50(w: &[u32; 10]) -> u32 {
    interp5(w[6], w[5])
}
#[inline]
fn q4_p23_60(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[6], w[8])
}
#[inline]
fn q4_p23_61(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[6], w[9])
}
#[inline]
fn q4_p23_81(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[6])
}
#[inline]
fn q4_p23_83(w: &[u32; 10]) -> u32 {
    interp8(w[6], w[8])
}

#[inline]
fn q4_p30_c(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn q4_p30_11(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[8])
}
#[inline]
fn q4_p30_12(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[4])
}
#[inline]
fn q4_p30_20(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[8], w[4])
}
#[inline]
fn q4_p30_50(w: &[u32; 10]) -> u32 {
    interp5(w[8], w[4])
}
#[inline]
fn q4_p30_80(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[7])
}
#[inline]
fn q4_p30_81(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[8])
}
#[inline]
fn q4_p30_82(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[4])
}

#[inline]
fn q4_p31_c(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn q4_p31_10(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[7])
}
#[inline]
fn q4_p31_11(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[8])
}
#[inline]
fn q4_p31_13(w: &[u32; 10]) -> u32 {
    interp1(w[8], w[5])
}
#[inline]
fn q4_p31_21(w: &[u32; 10]) -> u32 {
    interp2(w[8], w[5], w[4])
}
#[inline]
fn q4_p31_32(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[4])
}
#[inline]
fn q4_p31_50(w: &[u32; 10]) -> u32 {
    interp5(w[8], w[5])
}
#[inline]
fn q4_p31_60(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[8], w[4])
}
#[inline]
fn q4_p31_61(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[8], w[7])
}
#[inline]
fn q4_p31_81(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[8])
}
#[inline]
fn q4_p31_83(w: &[u32; 10]) -> u32 {
    interp8(w[8], w[4])
}

#[inline]
fn q4_p32_c(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn q4_p32_10(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[9])
}
#[inline]
fn q4_p32_12(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[8])
}
#[inline]
fn q4_p32_14(w: &[u32; 10]) -> u32 {
    interp1(w[8], w[5])
}
#[inline]
fn q4_p32_21(w: &[u32; 10]) -> u32 {
    interp2(w[8], w[5], w[6])
}
#[inline]
fn q4_p32_31(w: &[u32; 10]) -> u32 {
    interp3(w[5], w[6])
}
#[inline]
fn q4_p32_50(w: &[u32; 10]) -> u32 {
    interp5(w[8], w[5])
}
#[inline]
fn q4_p32_60(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[8], w[6])
}
#[inline]
fn q4_p32_61(w: &[u32; 10]) -> u32 {
    interp6(w[5], w[8], w[9])
}
#[inline]
fn q4_p32_82(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[8])
}
#[inline]
fn q4_p32_83(w: &[u32; 10]) -> u32 {
    interp8(w[8], w[6])
}

#[inline]
fn q4_p33_c(w: &[u32; 10]) -> u32 {
    w[5]
}
#[inline]
fn q4_p33_11(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[6])
}
#[inline]
fn q4_p33_12(w: &[u32; 10]) -> u32 {
    interp1(w[5], w[8])
}
#[inline]
fn q4_p33_20(w: &[u32; 10]) -> u32 {
    interp2(w[5], w[8], w[6])
}
#[inline]
fn q4_p33_50(w: &[u32; 10]) -> u32 {
    interp5(w[8], w[6])
}
#[inline]
fn q4_p33_80(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[9])
}
#[inline]
fn q4_p33_81(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[6])
}
#[inline]
fn q4_p33_82(w: &[u32; 10]) -> u32 {
    interp8(w[5], w[8])
}

pub fn hq4x(src: &[u32], width: usize, height: usize) -> Vec<u32> {
    let out_w = width * 4;
    let mut dst = vec![0u32; out_w * height * 4];
    let mut idx = 0usize;
    let p1 = out_w;
    let p2 = out_w * 2;
    let p3 = out_w * 3;

    for _j in 0..height {
        for i in 0..width {
            let w = fetch_neighborhood(src, width, height, i, _j);
            match make_pattern(&w) {
                0 | 1 | 4 | 32 | 128 | 5 | 132 | 160 | 33 | 129 | 36 | 133 | 164 | 161 | 37 | 165 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                2 | 34 | 130 | 162 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                16 | 17 | 48 | 49 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                64 | 65 | 68 | 69 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                8 | 12 | 136 | 140 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                3 | 35 | 131 | 163 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                6 | 38 | 134 | 166 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                20 | 21 | 52 | 53 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                144 | 145 | 176 | 177 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                192 | 193 | 196 | 197 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                96 | 97 | 100 | 101 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                40 | 44 | 168 | 172 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                9 | 13 | 137 | 141 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                18 | 50 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_10(&w);
                        dst[idx + 3] = q4_p03_80(&w);
                        dst[idx + p1 + 2] = q4_p12_30(&w);
                        dst[idx + p1 + 3] = q4_p13_10(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                80 | 81 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = q4_p22_30(&w);
                        dst[idx + p2 + 3] = q4_p23_10(&w);
                        dst[idx + p3 + 2] = q4_p32_10(&w);
                        dst[idx + p3 + 3] = q4_p33_80(&w);
                    } else {
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                }
                72 | 76 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_10(&w);
                        dst[idx + p2 + 1] = q4_p21_30(&w);
                        dst[idx + p3] = q4_p30_80(&w);
                        dst[idx + p3 + 1] = q4_p31_10(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                10 | 138 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_80(&w);
                        dst[idx + 1] = q4_p01_10(&w);
                        dst[idx + p1] = q4_p10_10(&w);
                        dst[idx + p1 + 1] = q4_p11_30(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                    }
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                66 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                24 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                7 | 39 | 135 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                148 | 149 | 180 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                224 | 228 | 225 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                41 | 169 | 45 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                22 | 54 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                208 | 209 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                }
                104 | 108 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                11 | 139 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                    }
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                19 | 51 => {
                    if diff(w[2], w[6]) {
                        dst[idx] = q4_p00_81(&w);
                        dst[idx + 1] = q4_p01_31(&w);
                        dst[idx + 2] = q4_p02_10(&w);
                        dst[idx + 3] = q4_p03_80(&w);
                        dst[idx + p1 + 2] = q4_p12_30(&w);
                        dst[idx + p1 + 3] = q4_p13_10(&w);
                    } else {
                        dst[idx] = q4_p00_12(&w);
                        dst[idx + 1] = q4_p01_14(&w);
                        dst[idx + 2] = q4_p02_83(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 2] = q4_p12_70(&w);
                        dst[idx + p1 + 3] = q4_p13_21(&w);
                    }
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                146 | 178 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_10(&w);
                        dst[idx + 3] = q4_p03_80(&w);
                        dst[idx + p1 + 2] = q4_p12_30(&w);
                        dst[idx + p1 + 3] = q4_p13_10(&w);
                        dst[idx + p2 + 3] = q4_p23_32(&w);
                        dst[idx + p3 + 3] = q4_p33_82(&w);
                    } else {
                        dst[idx + 2] = q4_p02_21(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 2] = q4_p12_70(&w);
                        dst[idx + p1 + 3] = q4_p13_83(&w);
                        dst[idx + p2 + 3] = q4_p23_13(&w);
                        dst[idx + p3 + 3] = q4_p33_11(&w);
                    }
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                }
                84 | 85 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + 3] = q4_p03_81(&w);
                        dst[idx + p1 + 3] = q4_p13_31(&w);
                        dst[idx + p2 + 2] = q4_p22_30(&w);
                        dst[idx + p2 + 3] = q4_p23_10(&w);
                        dst[idx + p3 + 2] = q4_p32_10(&w);
                        dst[idx + p3 + 3] = q4_p33_80(&w);
                    } else {
                        dst[idx + 3] = q4_p03_12(&w);
                        dst[idx + p1 + 3] = q4_p13_14(&w);
                        dst[idx + p2 + 2] = q4_p22_70(&w);
                        dst[idx + p2 + 3] = q4_p23_83(&w);
                        dst[idx + p3 + 2] = q4_p32_21(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                }
                112 | 113 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = q4_p22_30(&w);
                        dst[idx + p2 + 3] = q4_p23_10(&w);
                        dst[idx + p3] = q4_p30_82(&w);
                        dst[idx + p3 + 1] = q4_p31_32(&w);
                        dst[idx + p3 + 2] = q4_p32_10(&w);
                        dst[idx + p3 + 3] = q4_p33_80(&w);
                    } else {
                        dst[idx + p2 + 2] = q4_p22_70(&w);
                        dst[idx + p2 + 3] = q4_p23_21(&w);
                        dst[idx + p3] = q4_p30_11(&w);
                        dst[idx + p3 + 1] = q4_p31_13(&w);
                        dst[idx + p3 + 2] = q4_p32_83(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                }
                200 | 204 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_10(&w);
                        dst[idx + p2 + 1] = q4_p21_30(&w);
                        dst[idx + p3] = q4_p30_80(&w);
                        dst[idx + p3 + 1] = q4_p31_10(&w);
                        dst[idx + p3 + 2] = q4_p32_31(&w);
                        dst[idx + p3 + 3] = q4_p33_81(&w);
                    } else {
                        dst[idx + p2] = q4_p20_21(&w);
                        dst[idx + p2 + 1] = q4_p21_70(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_83(&w);
                        dst[idx + p3 + 2] = q4_p32_14(&w);
                        dst[idx + p3 + 3] = q4_p33_12(&w);
                    }
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                }
                73 | 77 => {
                    if diff(w[8], w[4]) {
                        dst[idx] = q4_p00_82(&w);
                        dst[idx + p1] = q4_p10_32(&w);
                        dst[idx + p2] = q4_p20_10(&w);
                        dst[idx + p2 + 1] = q4_p21_30(&w);
                        dst[idx + p3] = q4_p30_80(&w);
                        dst[idx + p3 + 1] = q4_p31_10(&w);
                    } else {
                        dst[idx] = q4_p00_11(&w);
                        dst[idx + p1] = q4_p10_13(&w);
                        dst[idx + p2] = q4_p20_83(&w);
                        dst[idx + p2 + 1] = q4_p21_70(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_21(&w);
                    }
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                42 | 170 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_80(&w);
                        dst[idx + 1] = q4_p01_10(&w);
                        dst[idx + p1] = q4_p10_10(&w);
                        dst[idx + p1 + 1] = q4_p11_30(&w);
                        dst[idx + p2] = q4_p20_31(&w);
                        dst[idx + p3] = q4_p30_81(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_21(&w);
                        dst[idx + p1] = q4_p10_83(&w);
                        dst[idx + p1 + 1] = q4_p11_70(&w);
                        dst[idx + p2] = q4_p20_14(&w);
                        dst[idx + p3] = q4_p30_12(&w);
                    }
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                14 | 142 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_80(&w);
                        dst[idx + 1] = q4_p01_10(&w);
                        dst[idx + 2] = q4_p02_32(&w);
                        dst[idx + 3] = q4_p03_82(&w);
                        dst[idx + p1] = q4_p10_10(&w);
                        dst[idx + p1 + 1] = q4_p11_30(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_83(&w);
                        dst[idx + 2] = q4_p02_13(&w);
                        dst[idx + 3] = q4_p03_11(&w);
                        dst[idx + p1] = q4_p10_21(&w);
                        dst[idx + p1 + 1] = q4_p11_70(&w);
                    }
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                67 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                70 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                28 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                152 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                194 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                98 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                56 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                25 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                26 | 31 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                82 | 214 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                }
                88 | 248 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                }
                74 | 107 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                    }
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                27 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                    }
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                86 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                216 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                }
                106 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                30 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                210 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                }
                120 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                75 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                    }
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                29 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                198 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                184 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                99 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                57 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                71 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                156 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                226 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                60 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                195 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                102 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                153 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                58 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_80(&w);
                        dst[idx + 1] = q4_p01_10(&w);
                        dst[idx + p1] = q4_p10_10(&w);
                        dst[idx + p1 + 1] = q4_p11_30(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                        dst[idx + 1] = q4_p01_12(&w);
                        dst[idx + p1] = q4_p10_11(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_10(&w);
                        dst[idx + 3] = q4_p03_80(&w);
                        dst[idx + p1 + 2] = q4_p12_30(&w);
                        dst[idx + p1 + 3] = q4_p13_10(&w);
                    } else {
                        dst[idx + 2] = q4_p02_11(&w);
                        dst[idx + 3] = q4_p03_20(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_12(&w);
                    }
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                83 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_10(&w);
                        dst[idx + 3] = q4_p03_80(&w);
                        dst[idx + p1 + 2] = q4_p12_30(&w);
                        dst[idx + p1 + 3] = q4_p13_10(&w);
                    } else {
                        dst[idx + 2] = q4_p02_11(&w);
                        dst[idx + 3] = q4_p03_20(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_12(&w);
                    }
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = q4_p22_30(&w);
                        dst[idx + p2 + 3] = q4_p23_10(&w);
                        dst[idx + p3 + 2] = q4_p32_10(&w);
                        dst[idx + p3 + 3] = q4_p33_80(&w);
                    } else {
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_11(&w);
                        dst[idx + p3 + 2] = q4_p32_12(&w);
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                }
                92 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_10(&w);
                        dst[idx + p2 + 1] = q4_p21_30(&w);
                        dst[idx + p3] = q4_p30_80(&w);
                        dst[idx + p3 + 1] = q4_p31_10(&w);
                    } else {
                        dst[idx + p2] = q4_p20_12(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_20(&w);
                        dst[idx + p3 + 1] = q4_p31_11(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = q4_p22_30(&w);
                        dst[idx + p2 + 3] = q4_p23_10(&w);
                        dst[idx + p3 + 2] = q4_p32_10(&w);
                        dst[idx + p3 + 3] = q4_p33_80(&w);
                    } else {
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_11(&w);
                        dst[idx + p3 + 2] = q4_p32_12(&w);
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                }
                202 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_80(&w);
                        dst[idx + 1] = q4_p01_10(&w);
                        dst[idx + p1] = q4_p10_10(&w);
                        dst[idx + p1 + 1] = q4_p11_30(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                        dst[idx + 1] = q4_p01_12(&w);
                        dst[idx + p1] = q4_p10_11(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                    }
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_10(&w);
                        dst[idx + p2 + 1] = q4_p21_30(&w);
                        dst[idx + p3] = q4_p30_80(&w);
                        dst[idx + p3 + 1] = q4_p31_10(&w);
                    } else {
                        dst[idx + p2] = q4_p20_12(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_20(&w);
                        dst[idx + p3 + 1] = q4_p31_11(&w);
                    }
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                78 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_80(&w);
                        dst[idx + 1] = q4_p01_10(&w);
                        dst[idx + p1] = q4_p10_10(&w);
                        dst[idx + p1 + 1] = q4_p11_30(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                        dst[idx + 1] = q4_p01_12(&w);
                        dst[idx + p1] = q4_p10_11(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                    }
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_10(&w);
                        dst[idx + p2 + 1] = q4_p21_30(&w);
                        dst[idx + p3] = q4_p30_80(&w);
                        dst[idx + p3 + 1] = q4_p31_10(&w);
                    } else {
                        dst[idx + p2] = q4_p20_12(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_20(&w);
                        dst[idx + p3 + 1] = q4_p31_11(&w);
                    }
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                154 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_80(&w);
                        dst[idx + 1] = q4_p01_10(&w);
                        dst[idx + p1] = q4_p10_10(&w);
                        dst[idx + p1 + 1] = q4_p11_30(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                        dst[idx + 1] = q4_p01_12(&w);
                        dst[idx + p1] = q4_p10_11(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_10(&w);
                        dst[idx + 3] = q4_p03_80(&w);
                        dst[idx + p1 + 2] = q4_p12_30(&w);
                        dst[idx + p1 + 3] = q4_p13_10(&w);
                    } else {
                        dst[idx + 2] = q4_p02_11(&w);
                        dst[idx + 3] = q4_p03_20(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_12(&w);
                    }
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                114 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_10(&w);
                        dst[idx + 3] = q4_p03_80(&w);
                        dst[idx + p1 + 2] = q4_p12_30(&w);
                        dst[idx + p1 + 3] = q4_p13_10(&w);
                    } else {
                        dst[idx + 2] = q4_p02_11(&w);
                        dst[idx + 3] = q4_p03_20(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_12(&w);
                    }
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = q4_p22_30(&w);
                        dst[idx + p2 + 3] = q4_p23_10(&w);
                        dst[idx + p3 + 2] = q4_p32_10(&w);
                        dst[idx + p3 + 3] = q4_p33_80(&w);
                    } else {
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_11(&w);
                        dst[idx + p3 + 2] = q4_p32_12(&w);
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                }
                89 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_10(&w);
                        dst[idx + p2 + 1] = q4_p21_30(&w);
                        dst[idx + p3] = q4_p30_80(&w);
                        dst[idx + p3 + 1] = q4_p31_10(&w);
                    } else {
                        dst[idx + p2] = q4_p20_12(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_20(&w);
                        dst[idx + p3 + 1] = q4_p31_11(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = q4_p22_30(&w);
                        dst[idx + p2 + 3] = q4_p23_10(&w);
                        dst[idx + p3 + 2] = q4_p32_10(&w);
                        dst[idx + p3 + 3] = q4_p33_80(&w);
                    } else {
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_11(&w);
                        dst[idx + p3 + 2] = q4_p32_12(&w);
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                }
                90 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_80(&w);
                        dst[idx + 1] = q4_p01_10(&w);
                        dst[idx + p1] = q4_p10_10(&w);
                        dst[idx + p1 + 1] = q4_p11_30(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                        dst[idx + 1] = q4_p01_12(&w);
                        dst[idx + p1] = q4_p10_11(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_10(&w);
                        dst[idx + 3] = q4_p03_80(&w);
                        dst[idx + p1 + 2] = q4_p12_30(&w);
                        dst[idx + p1 + 3] = q4_p13_10(&w);
                    } else {
                        dst[idx + 2] = q4_p02_11(&w);
                        dst[idx + 3] = q4_p03_20(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_12(&w);
                    }
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_10(&w);
                        dst[idx + p2 + 1] = q4_p21_30(&w);
                        dst[idx + p3] = q4_p30_80(&w);
                        dst[idx + p3 + 1] = q4_p31_10(&w);
                    } else {
                        dst[idx + p2] = q4_p20_12(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_20(&w);
                        dst[idx + p3 + 1] = q4_p31_11(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = q4_p22_30(&w);
                        dst[idx + p2 + 3] = q4_p23_10(&w);
                        dst[idx + p3 + 2] = q4_p32_10(&w);
                        dst[idx + p3 + 3] = q4_p33_80(&w);
                    } else {
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_11(&w);
                        dst[idx + p3 + 2] = q4_p32_12(&w);
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                }
                55 | 23 => {
                    if diff(w[2], w[6]) {
                        dst[idx] = q4_p00_81(&w);
                        dst[idx + 1] = q4_p01_31(&w);
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx] = q4_p00_12(&w);
                        dst[idx + 1] = q4_p01_14(&w);
                        dst[idx + 2] = q4_p02_83(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 2] = q4_p12_70(&w);
                        dst[idx + p1 + 3] = q4_p13_21(&w);
                    }
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                182 | 150 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                        dst[idx + p2 + 3] = q4_p23_32(&w);
                        dst[idx + p3 + 3] = q4_p33_82(&w);
                    } else {
                        dst[idx + 2] = q4_p02_21(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 2] = q4_p12_70(&w);
                        dst[idx + p1 + 3] = q4_p13_83(&w);
                        dst[idx + p2 + 3] = q4_p23_13(&w);
                        dst[idx + p3 + 3] = q4_p33_11(&w);
                    }
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                }
                213 | 212 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + 3] = q4_p03_81(&w);
                        dst[idx + p1 + 3] = q4_p13_31(&w);
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + 3] = q4_p03_12(&w);
                        dst[idx + p1 + 3] = q4_p13_14(&w);
                        dst[idx + p2 + 2] = q4_p22_70(&w);
                        dst[idx + p2 + 3] = q4_p23_83(&w);
                        dst[idx + p3 + 2] = q4_p32_21(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                }
                241 | 240 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3] = q4_p30_82(&w);
                        dst[idx + p3 + 1] = q4_p31_32(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 2] = q4_p22_70(&w);
                        dst[idx + p2 + 3] = q4_p23_21(&w);
                        dst[idx + p3] = q4_p30_11(&w);
                        dst[idx + p3 + 1] = q4_p31_13(&w);
                        dst[idx + p3 + 2] = q4_p32_83(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                }
                236 | 232 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                        dst[idx + p3 + 2] = q4_p32_31(&w);
                        dst[idx + p3 + 3] = q4_p33_81(&w);
                    } else {
                        dst[idx + p2] = q4_p20_21(&w);
                        dst[idx + p2 + 1] = q4_p21_70(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_83(&w);
                        dst[idx + p3 + 2] = q4_p32_14(&w);
                        dst[idx + p3 + 3] = q4_p33_12(&w);
                    }
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                }
                109 | 105 => {
                    if diff(w[8], w[4]) {
                        dst[idx] = q4_p00_82(&w);
                        dst[idx + p1] = q4_p10_32(&w);
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx] = q4_p00_11(&w);
                        dst[idx + p1] = q4_p10_13(&w);
                        dst[idx + p2] = q4_p20_83(&w);
                        dst[idx + p2 + 1] = q4_p21_70(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_21(&w);
                    }
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                171 | 43 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                        dst[idx + p2] = q4_p20_31(&w);
                        dst[idx + p3] = q4_p30_81(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_21(&w);
                        dst[idx + p1] = q4_p10_83(&w);
                        dst[idx + p1 + 1] = q4_p11_70(&w);
                        dst[idx + p2] = q4_p20_14(&w);
                        dst[idx + p3] = q4_p30_12(&w);
                    }
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                143 | 15 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + 2] = q4_p02_32(&w);
                        dst[idx + 3] = q4_p03_82(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_83(&w);
                        dst[idx + 2] = q4_p02_13(&w);
                        dst[idx + 3] = q4_p03_11(&w);
                        dst[idx + p1] = q4_p10_21(&w);
                        dst[idx + p1 + 1] = q4_p11_70(&w);
                    }
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                124 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                203 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                    }
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                62 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                211 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                }
                118 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                217 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                }
                110 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                155 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                    }
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                188 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                185 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                61 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                157 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                103 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                227 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                230 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                199 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                220 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_10(&w);
                        dst[idx + p2 + 1] = q4_p21_30(&w);
                        dst[idx + p3] = q4_p30_80(&w);
                        dst[idx + p3 + 1] = q4_p31_10(&w);
                    } else {
                        dst[idx + p2] = q4_p20_12(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_20(&w);
                        dst[idx + p3 + 1] = q4_p31_11(&w);
                    }
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                }
                158 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_80(&w);
                        dst[idx + 1] = q4_p01_10(&w);
                        dst[idx + p1] = q4_p10_10(&w);
                        dst[idx + p1 + 1] = q4_p11_30(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                        dst[idx + 1] = q4_p01_12(&w);
                        dst[idx + p1] = q4_p10_11(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                234 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_80(&w);
                        dst[idx + 1] = q4_p01_10(&w);
                        dst[idx + p1] = q4_p10_10(&w);
                        dst[idx + p1 + 1] = q4_p11_30(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                        dst[idx + 1] = q4_p01_12(&w);
                        dst[idx + p1] = q4_p10_11(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                    }
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                242 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_10(&w);
                        dst[idx + 3] = q4_p03_80(&w);
                        dst[idx + p1 + 2] = q4_p12_30(&w);
                        dst[idx + p1 + 3] = q4_p13_10(&w);
                    } else {
                        dst[idx + 2] = q4_p02_11(&w);
                        dst[idx + 3] = q4_p03_20(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_12(&w);
                    }
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                }
                59 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_10(&w);
                        dst[idx + 3] = q4_p03_80(&w);
                        dst[idx + p1 + 2] = q4_p12_30(&w);
                        dst[idx + p1 + 3] = q4_p13_10(&w);
                    } else {
                        dst[idx + 2] = q4_p02_11(&w);
                        dst[idx + 3] = q4_p03_20(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_12(&w);
                    }
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                121 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = q4_p22_30(&w);
                        dst[idx + p2 + 3] = q4_p23_10(&w);
                        dst[idx + p3 + 2] = q4_p32_10(&w);
                        dst[idx + p3 + 3] = q4_p33_80(&w);
                    } else {
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_11(&w);
                        dst[idx + p3 + 2] = q4_p32_12(&w);
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                }
                87 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = q4_p22_30(&w);
                        dst[idx + p2 + 3] = q4_p23_10(&w);
                        dst[idx + p3 + 2] = q4_p32_10(&w);
                        dst[idx + p3 + 3] = q4_p33_80(&w);
                    } else {
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_11(&w);
                        dst[idx + p3 + 2] = q4_p32_12(&w);
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                }
                79 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                    }
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_10(&w);
                        dst[idx + p2 + 1] = q4_p21_30(&w);
                        dst[idx + p3] = q4_p30_80(&w);
                        dst[idx + p3 + 1] = q4_p31_10(&w);
                    } else {
                        dst[idx + p2] = q4_p20_12(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_20(&w);
                        dst[idx + p3 + 1] = q4_p31_11(&w);
                    }
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                122 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_80(&w);
                        dst[idx + 1] = q4_p01_10(&w);
                        dst[idx + p1] = q4_p10_10(&w);
                        dst[idx + p1 + 1] = q4_p11_30(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                        dst[idx + 1] = q4_p01_12(&w);
                        dst[idx + p1] = q4_p10_11(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_10(&w);
                        dst[idx + 3] = q4_p03_80(&w);
                        dst[idx + p1 + 2] = q4_p12_30(&w);
                        dst[idx + p1 + 3] = q4_p13_10(&w);
                    } else {
                        dst[idx + 2] = q4_p02_11(&w);
                        dst[idx + 3] = q4_p03_20(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_12(&w);
                    }
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = q4_p22_30(&w);
                        dst[idx + p2 + 3] = q4_p23_10(&w);
                        dst[idx + p3 + 2] = q4_p32_10(&w);
                        dst[idx + p3 + 3] = q4_p33_80(&w);
                    } else {
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_11(&w);
                        dst[idx + p3 + 2] = q4_p32_12(&w);
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                }
                94 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_80(&w);
                        dst[idx + 1] = q4_p01_10(&w);
                        dst[idx + p1] = q4_p10_10(&w);
                        dst[idx + p1 + 1] = q4_p11_30(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                        dst[idx + 1] = q4_p01_12(&w);
                        dst[idx + p1] = q4_p10_11(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_10(&w);
                        dst[idx + p2 + 1] = q4_p21_30(&w);
                        dst[idx + p3] = q4_p30_80(&w);
                        dst[idx + p3 + 1] = q4_p31_10(&w);
                    } else {
                        dst[idx + p2] = q4_p20_12(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_20(&w);
                        dst[idx + p3 + 1] = q4_p31_11(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = q4_p22_30(&w);
                        dst[idx + p2 + 3] = q4_p23_10(&w);
                        dst[idx + p3 + 2] = q4_p32_10(&w);
                        dst[idx + p3 + 3] = q4_p33_80(&w);
                    } else {
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_11(&w);
                        dst[idx + p3 + 2] = q4_p32_12(&w);
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                }
                218 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_80(&w);
                        dst[idx + 1] = q4_p01_10(&w);
                        dst[idx + p1] = q4_p10_10(&w);
                        dst[idx + p1 + 1] = q4_p11_30(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                        dst[idx + 1] = q4_p01_12(&w);
                        dst[idx + p1] = q4_p10_11(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_10(&w);
                        dst[idx + 3] = q4_p03_80(&w);
                        dst[idx + p1 + 2] = q4_p12_30(&w);
                        dst[idx + p1 + 3] = q4_p13_10(&w);
                    } else {
                        dst[idx + 2] = q4_p02_11(&w);
                        dst[idx + 3] = q4_p03_20(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_12(&w);
                    }
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_10(&w);
                        dst[idx + p2 + 1] = q4_p21_30(&w);
                        dst[idx + p3] = q4_p30_80(&w);
                        dst[idx + p3 + 1] = q4_p31_10(&w);
                    } else {
                        dst[idx + p2] = q4_p20_12(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_20(&w);
                        dst[idx + p3 + 1] = q4_p31_11(&w);
                    }
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                }
                91 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_10(&w);
                        dst[idx + 3] = q4_p03_80(&w);
                        dst[idx + p1 + 2] = q4_p12_30(&w);
                        dst[idx + p1 + 3] = q4_p13_10(&w);
                    } else {
                        dst[idx + 2] = q4_p02_11(&w);
                        dst[idx + 3] = q4_p03_20(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_12(&w);
                    }
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_10(&w);
                        dst[idx + p2 + 1] = q4_p21_30(&w);
                        dst[idx + p3] = q4_p30_80(&w);
                        dst[idx + p3 + 1] = q4_p31_10(&w);
                    } else {
                        dst[idx + p2] = q4_p20_12(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_20(&w);
                        dst[idx + p3 + 1] = q4_p31_11(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = q4_p22_30(&w);
                        dst[idx + p2 + 3] = q4_p23_10(&w);
                        dst[idx + p3 + 2] = q4_p32_10(&w);
                        dst[idx + p3 + 3] = q4_p33_80(&w);
                    } else {
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_11(&w);
                        dst[idx + p3 + 2] = q4_p32_12(&w);
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                }
                229 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                167 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                173 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                181 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                186 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_80(&w);
                        dst[idx + 1] = q4_p01_10(&w);
                        dst[idx + p1] = q4_p10_10(&w);
                        dst[idx + p1 + 1] = q4_p11_30(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                        dst[idx + 1] = q4_p01_12(&w);
                        dst[idx + p1] = q4_p10_11(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_10(&w);
                        dst[idx + 3] = q4_p03_80(&w);
                        dst[idx + p1 + 2] = q4_p12_30(&w);
                        dst[idx + p1 + 3] = q4_p13_10(&w);
                    } else {
                        dst[idx + 2] = q4_p02_11(&w);
                        dst[idx + 3] = q4_p03_20(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_12(&w);
                    }
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                115 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_10(&w);
                        dst[idx + 3] = q4_p03_80(&w);
                        dst[idx + p1 + 2] = q4_p12_30(&w);
                        dst[idx + p1 + 3] = q4_p13_10(&w);
                    } else {
                        dst[idx + 2] = q4_p02_11(&w);
                        dst[idx + 3] = q4_p03_20(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_12(&w);
                    }
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = q4_p22_30(&w);
                        dst[idx + p2 + 3] = q4_p23_10(&w);
                        dst[idx + p3 + 2] = q4_p32_10(&w);
                        dst[idx + p3 + 3] = q4_p33_80(&w);
                    } else {
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_11(&w);
                        dst[idx + p3 + 2] = q4_p32_12(&w);
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                }
                93 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_10(&w);
                        dst[idx + p2 + 1] = q4_p21_30(&w);
                        dst[idx + p3] = q4_p30_80(&w);
                        dst[idx + p3 + 1] = q4_p31_10(&w);
                    } else {
                        dst[idx + p2] = q4_p20_12(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_20(&w);
                        dst[idx + p3 + 1] = q4_p31_11(&w);
                    }
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = q4_p22_30(&w);
                        dst[idx + p2 + 3] = q4_p23_10(&w);
                        dst[idx + p3 + 2] = q4_p32_10(&w);
                        dst[idx + p3 + 3] = q4_p33_80(&w);
                    } else {
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_11(&w);
                        dst[idx + p3 + 2] = q4_p32_12(&w);
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                }
                206 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_80(&w);
                        dst[idx + 1] = q4_p01_10(&w);
                        dst[idx + p1] = q4_p10_10(&w);
                        dst[idx + p1 + 1] = q4_p11_30(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                        dst[idx + 1] = q4_p01_12(&w);
                        dst[idx + p1] = q4_p10_11(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                    }
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_10(&w);
                        dst[idx + p2 + 1] = q4_p21_30(&w);
                        dst[idx + p3] = q4_p30_80(&w);
                        dst[idx + p3 + 1] = q4_p31_10(&w);
                    } else {
                        dst[idx + p2] = q4_p20_12(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_20(&w);
                        dst[idx + p3 + 1] = q4_p31_11(&w);
                    }
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                205 | 201 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_10(&w);
                        dst[idx + p2 + 1] = q4_p21_30(&w);
                        dst[idx + p3] = q4_p30_80(&w);
                        dst[idx + p3 + 1] = q4_p31_10(&w);
                    } else {
                        dst[idx + p2] = q4_p20_12(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_20(&w);
                        dst[idx + p3 + 1] = q4_p31_11(&w);
                    }
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                174 | 46 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_80(&w);
                        dst[idx + 1] = q4_p01_10(&w);
                        dst[idx + p1] = q4_p10_10(&w);
                        dst[idx + p1 + 1] = q4_p11_30(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                        dst[idx + 1] = q4_p01_12(&w);
                        dst[idx + p1] = q4_p10_11(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                    }
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                179 | 147 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_10(&w);
                        dst[idx + 3] = q4_p03_80(&w);
                        dst[idx + p1 + 2] = q4_p12_30(&w);
                        dst[idx + p1 + 3] = q4_p13_10(&w);
                    } else {
                        dst[idx + 2] = q4_p02_11(&w);
                        dst[idx + 3] = q4_p03_20(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_12(&w);
                    }
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                117 | 116 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = q4_p22_30(&w);
                        dst[idx + p2 + 3] = q4_p23_10(&w);
                        dst[idx + p3 + 2] = q4_p32_10(&w);
                        dst[idx + p3 + 3] = q4_p33_80(&w);
                    } else {
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_11(&w);
                        dst[idx + p3 + 2] = q4_p32_12(&w);
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                }
                189 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                231 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                126 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                219 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                    }
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                }
                125 => {
                    if diff(w[8], w[4]) {
                        dst[idx] = q4_p00_82(&w);
                        dst[idx + p1] = q4_p10_32(&w);
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx] = q4_p00_11(&w);
                        dst[idx + p1] = q4_p10_13(&w);
                        dst[idx + p2] = q4_p20_83(&w);
                        dst[idx + p2 + 1] = q4_p21_70(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_21(&w);
                    }
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                221 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + 3] = q4_p03_81(&w);
                        dst[idx + p1 + 3] = q4_p13_31(&w);
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + 3] = q4_p03_12(&w);
                        dst[idx + p1 + 3] = q4_p13_14(&w);
                        dst[idx + p2 + 2] = q4_p22_70(&w);
                        dst[idx + p2 + 3] = q4_p23_83(&w);
                        dst[idx + p3 + 2] = q4_p32_21(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                }
                207 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + 2] = q4_p02_32(&w);
                        dst[idx + 3] = q4_p03_82(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_83(&w);
                        dst[idx + 2] = q4_p02_13(&w);
                        dst[idx + 3] = q4_p03_11(&w);
                        dst[idx + p1] = q4_p10_21(&w);
                        dst[idx + p1 + 1] = q4_p11_70(&w);
                    }
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                238 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p2 + 1] = q4_p21_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                        dst[idx + p3 + 2] = q4_p32_31(&w);
                        dst[idx + p3 + 3] = q4_p33_81(&w);
                    } else {
                        dst[idx + p2] = q4_p20_21(&w);
                        dst[idx + p2 + 1] = q4_p21_70(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_83(&w);
                        dst[idx + p3 + 2] = q4_p32_14(&w);
                        dst[idx + p3 + 3] = q4_p33_12(&w);
                    }
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                }
                190 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                        dst[idx + p2 + 3] = q4_p23_32(&w);
                        dst[idx + p3 + 3] = q4_p33_82(&w);
                    } else {
                        dst[idx + 2] = q4_p02_21(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 2] = q4_p12_70(&w);
                        dst[idx + p1 + 3] = q4_p13_83(&w);
                        dst[idx + p2 + 3] = q4_p23_13(&w);
                        dst[idx + p3 + 3] = q4_p33_11(&w);
                    }
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                }
                187 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                        dst[idx + p1 + 1] = q4_p11_c(&w);
                        dst[idx + p2] = q4_p20_31(&w);
                        dst[idx + p3] = q4_p30_81(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_21(&w);
                        dst[idx + p1] = q4_p10_83(&w);
                        dst[idx + p1 + 1] = q4_p11_70(&w);
                        dst[idx + p2] = q4_p20_14(&w);
                        dst[idx + p3] = q4_p30_12(&w);
                    }
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                243 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 2] = q4_p22_c(&w);
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3] = q4_p30_82(&w);
                        dst[idx + p3 + 1] = q4_p31_32(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 2] = q4_p22_70(&w);
                        dst[idx + p2 + 3] = q4_p23_21(&w);
                        dst[idx + p3] = q4_p30_11(&w);
                        dst[idx + p3 + 1] = q4_p31_13(&w);
                        dst[idx + p3 + 2] = q4_p32_83(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                }
                119 => {
                    if diff(w[2], w[6]) {
                        dst[idx] = q4_p00_81(&w);
                        dst[idx + 1] = q4_p01_31(&w);
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 2] = q4_p12_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx] = q4_p00_12(&w);
                        dst[idx + 1] = q4_p01_14(&w);
                        dst[idx + 2] = q4_p02_83(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 2] = q4_p12_70(&w);
                        dst[idx + p1 + 3] = q4_p13_21(&w);
                    }
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                237 | 233 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_60(&w);
                    dst[idx + 3] = q4_p03_20(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_70(&w);
                    dst[idx + p1 + 3] = q4_p13_60(&w);
                    dst[idx + p2] = q4_p20_c(&w);
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p3] = q4_p30_c(&w);
                    } else {
                        dst[idx + p3] = q4_p30_20(&w);
                    }
                    dst[idx + p3 + 1] = q4_p31_c(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                175 | 47 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                    }
                    dst[idx + 1] = q4_p01_c(&w);
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1] = q4_p10_c(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_70(&w);
                    dst[idx + p2 + 3] = q4_p23_60(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_60(&w);
                    dst[idx + p3 + 3] = q4_p33_20(&w);
                }
                183 | 151 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    dst[idx + 2] = q4_p02_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 3] = q4_p03_c(&w);
                    } else {
                        dst[idx + 3] = q4_p03_20(&w);
                    }
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p1 + 3] = q4_p13_c(&w);
                    dst[idx + p2] = q4_p20_60(&w);
                    dst[idx + p2 + 1] = q4_p21_70(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_20(&w);
                    dst[idx + p3 + 1] = q4_p31_60(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                245 | 244 => {
                    dst[idx] = q4_p00_20(&w);
                    dst[idx + 1] = q4_p01_60(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_60(&w);
                    dst[idx + p1 + 1] = q4_p11_70(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    dst[idx + p2 + 3] = q4_p23_c(&w);
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                    dst[idx + p3 + 2] = q4_p32_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                }
                250 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                }
                123 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                    }
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                95 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                    }
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                222 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                }
                252 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_61(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    dst[idx + p2 + 3] = q4_p23_c(&w);
                    dst[idx + p3 + 2] = q4_p32_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                }
                249 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_61(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_c(&w);
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                    if diff(w[8], w[4]) {
                        dst[idx + p3] = q4_p30_c(&w);
                    } else {
                        dst[idx + p3] = q4_p30_20(&w);
                    }
                    dst[idx + p3 + 1] = q4_p31_c(&w);
                }
                235 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                    }
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_61(&w);
                    dst[idx + p2] = q4_p20_c(&w);
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p3] = q4_p30_c(&w);
                    } else {
                        dst[idx + p3] = q4_p30_20(&w);
                    }
                    dst[idx + p3 + 1] = q4_p31_c(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                111 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                    }
                    dst[idx + 1] = q4_p01_c(&w);
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1] = q4_p10_c(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_61(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                63 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                    }
                    dst[idx + 1] = q4_p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1] = q4_p10_c(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_61(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                159 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                    }
                    dst[idx + 2] = q4_p02_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 3] = q4_p03_c(&w);
                    } else {
                        dst[idx + 3] = q4_p03_20(&w);
                    }
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p1 + 3] = q4_p13_c(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_61(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                215 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    dst[idx + 2] = q4_p02_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 3] = q4_p03_c(&w);
                    } else {
                        dst[idx + 3] = q4_p03_20(&w);
                    }
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p1 + 3] = q4_p13_c(&w);
                    dst[idx + p2] = q4_p20_61(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                }
                246 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1] = q4_p10_61(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    dst[idx + p2 + 3] = q4_p23_c(&w);
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                    dst[idx + p3 + 2] = q4_p32_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                }
                254 => {
                    dst[idx] = q4_p00_80(&w);
                    dst[idx + 1] = q4_p01_10(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1] = q4_p10_10(&w);
                    dst[idx + p1 + 1] = q4_p11_30(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    dst[idx + p2 + 3] = q4_p23_c(&w);
                    dst[idx + p3 + 2] = q4_p32_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                }
                253 => {
                    dst[idx] = q4_p00_82(&w);
                    dst[idx + 1] = q4_p01_82(&w);
                    dst[idx + 2] = q4_p02_81(&w);
                    dst[idx + 3] = q4_p03_81(&w);
                    dst[idx + p1] = q4_p10_32(&w);
                    dst[idx + p1 + 1] = q4_p11_32(&w);
                    dst[idx + p1 + 2] = q4_p12_31(&w);
                    dst[idx + p1 + 3] = q4_p13_31(&w);
                    dst[idx + p2] = q4_p20_c(&w);
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    dst[idx + p2 + 3] = q4_p23_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p3] = q4_p30_c(&w);
                    } else {
                        dst[idx + p3] = q4_p30_20(&w);
                    }
                    dst[idx + p3 + 1] = q4_p31_c(&w);
                    dst[idx + p3 + 2] = q4_p32_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                }
                251 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                    }
                    dst[idx + 2] = q4_p02_10(&w);
                    dst[idx + 3] = q4_p03_80(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_30(&w);
                    dst[idx + p1 + 3] = q4_p13_10(&w);
                    dst[idx + p2] = q4_p20_c(&w);
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                    if diff(w[8], w[4]) {
                        dst[idx + p3] = q4_p30_c(&w);
                    } else {
                        dst[idx + p3] = q4_p30_20(&w);
                    }
                    dst[idx + p3 + 1] = q4_p31_c(&w);
                }
                239 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                    }
                    dst[idx + 1] = q4_p01_c(&w);
                    dst[idx + 2] = q4_p02_32(&w);
                    dst[idx + 3] = q4_p03_82(&w);
                    dst[idx + p1] = q4_p10_c(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_32(&w);
                    dst[idx + p1 + 3] = q4_p13_82(&w);
                    dst[idx + p2] = q4_p20_c(&w);
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_31(&w);
                    dst[idx + p2 + 3] = q4_p23_81(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p3] = q4_p30_c(&w);
                    } else {
                        dst[idx + p3] = q4_p30_20(&w);
                    }
                    dst[idx + p3 + 1] = q4_p31_c(&w);
                    dst[idx + p3 + 2] = q4_p32_31(&w);
                    dst[idx + p3 + 3] = q4_p33_81(&w);
                }
                127 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                    }
                    dst[idx + 1] = q4_p01_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 2] = q4_p02_c(&w);
                        dst[idx + 3] = q4_p03_c(&w);
                        dst[idx + p1 + 3] = q4_p13_c(&w);
                    } else {
                        dst[idx + 2] = q4_p02_50(&w);
                        dst[idx + 3] = q4_p03_50(&w);
                        dst[idx + p1 + 3] = q4_p13_50(&w);
                    }
                    dst[idx + p1] = q4_p10_c(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p2] = q4_p20_c(&w);
                        dst[idx + p3] = q4_p30_c(&w);
                        dst[idx + p3 + 1] = q4_p31_c(&w);
                    } else {
                        dst[idx + p2] = q4_p20_50(&w);
                        dst[idx + p3] = q4_p30_50(&w);
                        dst[idx + p3 + 1] = q4_p31_50(&w);
                    }
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_30(&w);
                    dst[idx + p2 + 3] = q4_p23_10(&w);
                    dst[idx + p3 + 2] = q4_p32_10(&w);
                    dst[idx + p3 + 3] = q4_p33_80(&w);
                }
                191 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                    }
                    dst[idx + 1] = q4_p01_c(&w);
                    dst[idx + 2] = q4_p02_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 3] = q4_p03_c(&w);
                    } else {
                        dst[idx + 3] = q4_p03_20(&w);
                    }
                    dst[idx + p1] = q4_p10_c(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p1 + 3] = q4_p13_c(&w);
                    dst[idx + p2] = q4_p20_31(&w);
                    dst[idx + p2 + 1] = q4_p21_31(&w);
                    dst[idx + p2 + 2] = q4_p22_32(&w);
                    dst[idx + p2 + 3] = q4_p23_32(&w);
                    dst[idx + p3] = q4_p30_81(&w);
                    dst[idx + p3 + 1] = q4_p31_81(&w);
                    dst[idx + p3 + 2] = q4_p32_82(&w);
                    dst[idx + p3 + 3] = q4_p33_82(&w);
                }
                223 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                        dst[idx + 1] = q4_p01_c(&w);
                        dst[idx + p1] = q4_p10_c(&w);
                    } else {
                        dst[idx] = q4_p00_50(&w);
                        dst[idx + 1] = q4_p01_50(&w);
                        dst[idx + p1] = q4_p10_50(&w);
                    }
                    dst[idx + 2] = q4_p02_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 3] = q4_p03_c(&w);
                    } else {
                        dst[idx + 3] = q4_p03_20(&w);
                    }
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p1 + 3] = q4_p13_c(&w);
                    dst[idx + p2] = q4_p20_10(&w);
                    dst[idx + p2 + 1] = q4_p21_30(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p2 + 3] = q4_p23_c(&w);
                        dst[idx + p3 + 2] = q4_p32_c(&w);
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p2 + 3] = q4_p23_50(&w);
                        dst[idx + p3 + 2] = q4_p32_50(&w);
                        dst[idx + p3 + 3] = q4_p33_50(&w);
                    }
                    dst[idx + p3] = q4_p30_80(&w);
                    dst[idx + p3 + 1] = q4_p31_10(&w);
                }
                247 => {
                    dst[idx] = q4_p00_81(&w);
                    dst[idx + 1] = q4_p01_31(&w);
                    dst[idx + 2] = q4_p02_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 3] = q4_p03_c(&w);
                    } else {
                        dst[idx + 3] = q4_p03_20(&w);
                    }
                    dst[idx + p1] = q4_p10_81(&w);
                    dst[idx + p1 + 1] = q4_p11_31(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p1 + 3] = q4_p13_c(&w);
                    dst[idx + p2] = q4_p20_82(&w);
                    dst[idx + p2 + 1] = q4_p21_32(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    dst[idx + p2 + 3] = q4_p23_c(&w);
                    dst[idx + p3] = q4_p30_82(&w);
                    dst[idx + p3 + 1] = q4_p31_32(&w);
                    dst[idx + p3 + 2] = q4_p32_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                }
                255 => {
                    if diff(w[4], w[2]) {
                        dst[idx] = q4_p00_c(&w);
                    } else {
                        dst[idx] = q4_p00_20(&w);
                    }
                    dst[idx + 1] = q4_p01_c(&w);
                    dst[idx + 2] = q4_p02_c(&w);
                    if diff(w[2], w[6]) {
                        dst[idx + 3] = q4_p03_c(&w);
                    } else {
                        dst[idx + 3] = q4_p03_20(&w);
                    }
                    dst[idx + p1] = q4_p10_c(&w);
                    dst[idx + p1 + 1] = q4_p11_c(&w);
                    dst[idx + p1 + 2] = q4_p12_c(&w);
                    dst[idx + p1 + 3] = q4_p13_c(&w);
                    dst[idx + p2] = q4_p20_c(&w);
                    dst[idx + p2 + 1] = q4_p21_c(&w);
                    dst[idx + p2 + 2] = q4_p22_c(&w);
                    dst[idx + p2 + 3] = q4_p23_c(&w);
                    if diff(w[8], w[4]) {
                        dst[idx + p3] = q4_p30_c(&w);
                    } else {
                        dst[idx + p3] = q4_p30_20(&w);
                    }
                    dst[idx + p3 + 1] = q4_p31_c(&w);
                    dst[idx + p3 + 2] = q4_p32_c(&w);
                    if diff(w[6], w[8]) {
                        dst[idx + p3 + 3] = q4_p33_c(&w);
                    } else {
                        dst[idx + p3 + 3] = q4_p33_20(&w);
                    }
                }
                _ => {}
            }
            idx += 4;
        }
        idx += p3;
    }
    dst
}