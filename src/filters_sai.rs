// the 2xsai, super2xsai and supereagle video filters made by retroArch, hans-kristian arntzen and daniel de matteis and ported from mesen!!!
#[inline]
fn fetch(src: &[u32], w: usize, h: usize, x: i32, y: i32) -> u32 {
    let cx = x.clamp(0, w as i32 - 1) as usize;
    let cy = y.clamp(0, h as i32 - 1) as usize;
    src[cy * w + cx]
}

#[inline]
fn interp(a: u32, b: u32) -> u32 {
    ((a & 0xFE_FE_FE_FE) >> 1) + ((b & 0xFE_FE_FE_FE) >> 1) + (a & b & 0x01_01_01_01)
}

#[inline]
fn interp2(a: u32, b: u32, c: u32, d: u32) -> u32 {
    ((a & 0xFC_FC_FC_FC) >> 2)
        + ((b & 0xFC_FC_FC_FC) >> 2)
        + ((c & 0xFC_FC_FC_FC) >> 2)
        + ((d & 0xFC_FC_FC_FC) >> 2)
        + ((((a & 0x03_03_03_03) + (b & 0x03_03_03_03) + (c & 0x03_03_03_03) + (d & 0x03_03_03_03)) >> 2)
            & 0x03_03_03_03)
}

#[inline]
fn result(a: u32, b: u32, c: u32, d: u32) -> i32 {
    (i32::from(a != c || a != d)) - (i32::from(b != c || b != d))
}

// 2xsai
pub fn twoxsai(src: &[u32], width: usize, height: usize) -> Vec<u32> {
    if width == 0 || height == 0 || src.len() < width * height {
        return Vec::new();
    }

    let ow = width * 2;
    let mut out = vec![0u32; ow * height * 2];

    for y in 0..height {
        for x in 0..width {
            let yi = y as i32;
            let xi = x as i32;

            let color_i = fetch(src, width, height, xi - 1, yi - 1);
            let color_e = fetch(src, width, height, xi, yi - 1);
            let color_f = fetch(src, width, height, xi + 1, yi - 1);
            let color_j = fetch(src, width, height, xi + 2, yi - 1);

            let color_g = fetch(src, width, height, xi - 1, yi);
            let color_a = src[y * width + x];
            let color_b = fetch(src, width, height, xi + 1, yi);
            let color_k = fetch(src, width, height, xi + 2, yi);

            let color_h = fetch(src, width, height, xi - 1, yi + 1);
            let color_c = fetch(src, width, height, xi, yi + 1);
            let color_d = fetch(src, width, height, xi + 1, yi + 1);
            let color_l = fetch(src, width, height, xi + 2, yi + 1);

            let color_m = fetch(src, width, height, xi - 1, yi + 2);
            let color_n = fetch(src, width, height, xi, yi + 2);
            let color_o = fetch(src, width, height, xi + 1, yi + 2);

            let (product, product1, product2);

            if color_a == color_d && color_b != color_c {
                if (color_a == color_e && color_b == color_l)
                    || (color_a == color_c && color_a == color_f && color_b != color_e && color_b == color_j)
                {
                    product = color_a;
                } else {
                    product = interp(color_a, color_b);
                }
                if (color_a == color_g && color_c == color_o)
                    || (color_a == color_b && color_a == color_h && color_g != color_c && color_c == color_m)
                {
                    product1 = color_a;
                } else {
                    product1 = interp(color_a, color_c);
                }
                product2 = color_a;
            } else if color_b == color_c && color_a != color_d {
                if (color_b == color_f && color_a == color_h)
                    || (color_b == color_e && color_b == color_d && color_a != color_f && color_a == color_i)
                {
                    product = color_b;
                } else {
                    product = interp(color_a, color_b);
                }
                if (color_c == color_h && color_a == color_f)
                    || (color_c == color_g && color_c == color_d && color_a != color_h && color_a == color_i)
                {
                    product1 = color_c;
                } else {
                    product1 = interp(color_a, color_c);
                }
                product2 = color_b;
            } else if color_a == color_d && color_b == color_c {
                if color_a == color_b {
                    product = color_a;
                    product1 = color_a;
                    product2 = color_a;
                } else {
                    product1 = interp(color_a, color_c);
                    product = interp(color_a, color_b);
                    let mut r = 0i32;
                    r += result(color_a, color_b, color_g, color_e);
                    r += result(color_b, color_a, color_k, color_f);
                    r += result(color_b, color_a, color_h, color_n);
                    r += result(color_a, color_b, color_l, color_o);
                    if r > 0 {
                        product2 = color_a;
                    } else if r < 0 {
                        product2 = color_b;
                    } else {
                        product2 = interp2(color_a, color_b, color_c, color_d);
                    }
                }
            } else {
                product2 = interp2(color_a, color_b, color_c, color_d);
                if color_a == color_c && color_a == color_f && color_b != color_e && color_b == color_j {
                    product = color_a;
                } else if color_b == color_e && color_b == color_d && color_a != color_f && color_a == color_i {
                    product = color_b;
                } else {
                    product = interp(color_a, color_b);
                }
                if color_a == color_b && color_a == color_h && color_g != color_c && color_c == color_m {
                    product1 = color_a;
                } else if color_c == color_g && color_c == color_d && color_a != color_h && color_a == color_i {
                    product1 = color_c;
                } else {
                    product1 = interp(color_a, color_c);
                }
            }

            let base = y * 2 * ow + x * 2;
            out[base] = color_a;
            out[base + 1] = product;
            out[base + ow] = product1;
            out[base + ow + 1] = product2;
        }
    }

    out
}

// super2xsai
pub fn supertwoxsai(src: &[u32], width: usize, height: usize) -> Vec<u32> {
    if width == 0 || height == 0 || src.len() < width * height {
        return Vec::new();
    }

    let ow = width * 2;
    let mut out = vec![0u32; ow * height * 2];

    for y in 0..height {
        for x in 0..width {
            let yi = y as i32;
            let xi = x as i32;

            let color_b0 = fetch(src, width, height, xi - 1, yi - 1);
            let color_b1 = fetch(src, width, height, xi, yi - 1);
            let color_b2 = fetch(src, width, height, xi + 1, yi - 1);
            let color_b3 = fetch(src, width, height, xi + 2, yi - 1);

            let color4 = fetch(src, width, height, xi - 1, yi);
            let color5 = src[y * width + x];
            let color6 = fetch(src, width, height, xi + 1, yi);
            let color_s2 = fetch(src, width, height, xi + 2, yi);

            let color1 = fetch(src, width, height, xi - 1, yi + 1);
            let color2 = fetch(src, width, height, xi, yi + 1);
            let color3 = fetch(src, width, height, xi + 1, yi + 1);
            let color_s1 = fetch(src, width, height, xi + 2, yi + 1);

            let color_a0 = fetch(src, width, height, xi - 1, yi + 2);
            let color_a1 = fetch(src, width, height, xi, yi + 2);
            let color_a2 = fetch(src, width, height, xi + 1, yi + 2);
            let color_a3 = fetch(src, width, height, xi + 2, yi + 2);

            let (product1a, product1b, product2a, product2b);

            if color2 == color6 && color5 != color3 {
                product2b = color2;
                product1b = color2;
            } else if color5 == color3 && color2 != color6 {
                product2b = color5;
                product1b = color5;
            } else if color5 == color3 && color2 == color6 {
                let mut r = 0i32;
                r += result(color6, color5, color1, color_a1);
                r += result(color6, color5, color4, color_b1);
                r += result(color6, color5, color_a2, color_s1);
                r += result(color6, color5, color_b2, color_s2);
                if r > 0 {
                    product2b = color6;
                    product1b = color6;
                } else if r < 0 {
                    product2b = color5;
                    product1b = color5;
                } else {
                    let v = interp(color5, color6);
                    product2b = v;
                    product1b = v;
                }
            } else {
                if color6 == color3 && color3 == color_a1 && color2 != color_a2 && color3 != color_a0 {
                    product2b = interp2(color3, color3, color3, color2);
                } else if color5 == color2 && color2 == color_a2 && color_a1 != color3 && color2 != color_a3 {
                    product2b = interp2(color2, color2, color2, color3);
                } else {
                    product2b = interp(color2, color3);
                }

                if color6 == color3 && color6 == color_b1 && color5 != color_b2 && color6 != color_b0 {
                    product1b = interp2(color6, color6, color6, color5);
                } else if color5 == color2 && color5 == color_b2 && color_b1 != color6 && color5 != color_b3 {
                    product1b = interp2(color6, color5, color5, color5);
                } else {
                    product1b = interp(color5, color6);
                }
            }

if (color5 == color3 && color2 != color6 && color4 == color5 && color5 != color_a2)
                || (color5 == color1 && color6 == color5 && color4 != color2 && color5 != color_a0)
            {
                product2a = interp(color2, color5);
            } else {
                product2a = color2;
            }

            if (color2 == color6 && color5 != color3 && color1 == color2 && color2 != color_b2)
                || (color4 == color2 && color3 == color2 && color1 != color5 && color2 != color_b0)
            {
                product1a = interp(color2, color5);
            } else {
                product1a = color5;
            }

            let base = y * 2 * ow + x * 2;
            out[base] = product1a;
            out[base + 1] = product1b;
            out[base + ow] = product2a;
            out[base + ow + 1] = product2b;
        }
    }

    out
}

// supereagle
pub fn supereagle(src: &[u32], width: usize, height: usize) -> Vec<u32> {
    if width == 0 || height == 0 || src.len() < width * height {
        return Vec::new();
    }

    let ow = width * 2;
    let mut out = vec![0u32; ow * height * 2];

    for y in 0..height {
        for x in 0..width {
            let yi = y as i32;
            let xi = x as i32;

            let color_b1 = fetch(src, width, height, xi, yi - 1);
            let color_b2 = fetch(src, width, height, xi + 1, yi - 1);

            let color4 = fetch(src, width, height, xi - 1, yi);
            let color5 = src[y * width + x];
            let color6 = fetch(src, width, height, xi + 1, yi);
            let color_s2 = fetch(src, width, height, xi + 2, yi);

            let color1 = fetch(src, width, height, xi - 1, yi + 1);
            let color2 = fetch(src, width, height, xi, yi + 1);
            let color3 = fetch(src, width, height, xi + 1, yi + 1);
            let color_s1 = fetch(src, width, height, xi + 2, yi + 1);

            let color_a1 = fetch(src, width, height, xi, yi + 2);
            let color_a2 = fetch(src, width, height, xi + 1, yi + 2);

            let (product1a, product1b, product2a, product2b);

            if color2 == color6 && color5 != color3 {
                product1b = color2;
                product2a = color2;
                if color1 == color2 || color6 == color_b2 {
                    let v = interp(color2, color5);
                    product1a = interp(color2, v);
                } else {
                    product1a = interp(color5, color6);
                }
                if color6 == color_s2 || color2 == color_a1 {
                    let v = interp(color2, color3);
                    product2b = interp(color2, v);
                } else {
                    product2b = interp(color2, color3);
                }
            } else if color5 == color3 && color2 != color6 {
                product2b = color5;
                product1a = color5;
                if color_b1 == color5 || color3 == color_s1 {
                    let v = interp(color5, color6);
                    product1b = interp(color5, v);
                } else {
                    product1b = interp(color5, color6);
                }
                if color3 == color_a2 || color4 == color5 {
                    let v = interp(color5, color2);
                    product2a = interp(color5, v);
                } else {
                    product2a = interp(color2, color3);
                }
            } else if color5 == color3 && color2 == color6 {
                let mut r = 0i32;
                r += result(color6, color5, color1, color_a1);
                r += result(color6, color5, color4, color_b1);
                r += result(color6, color5, color_a2, color_s1);
                r += result(color6, color5, color_b2, color_s2);
                if r > 0 {
                    product1b = color2;
                    product2a = color2;
                    let v = interp(color5, color6);
                    product1a = v;
                    product2b = v;
                } else if r < 0 {
                    product2b = color5;
                    product1a = color5;
                    let v = interp(color5, color6);
                    product1b = v;
                    product2a = v;
                } else {
                    product2b = color5;
                    product1a = color5;
                    product1b = color2;
                    product2a = color2;
                }
} else {
                let e26 = interp(color2, color6);
                let e53 = interp(color5, color3);
                product1a = interp2(color5, color5, color5, e26);
                product2b = interp2(color3, color3, color3, e26);
                product2a = interp2(color2, color2, color2, e53);
                product1b = interp2(color6, color6, color6, e53);
            }

            let base = y * 2 * ow + x * 2;
            out[base] = product1a;
            out[base + 1] = product1b;
            out[base + ow] = product2a;
            out[base + ow + 1] = product2b;
        }
    }

    out
}
