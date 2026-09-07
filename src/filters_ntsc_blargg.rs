// blargg's NTSC filter, ported from mesen!!!

const BURST_COUNT: usize = 3;
const ALIGNMENT_COUNT: usize = 3;
const RESCALE_IN: usize = 8;
const RESCALE_OUT: usize = 7;
const KERNEL_HALF: usize = 16;
const KERNEL_SIZE: usize = KERNEL_HALF * 2 + 1;
const ENTRY_SIZE: usize = 128;
const BURST_SIZE: usize = ENTRY_SIZE / BURST_COUNT;
const RGB_KERNEL_SIZE: usize = BURST_SIZE / ALIGNMENT_COUNT;
const PALETTE_SIZE: usize = 64 * 8;

const RGB_UNIT: u32 = 1 << 8;
const RGB_OFFSET: f32 = 512.5;
const RGB_BUILDER: u32 = (1 << 21) | (1 << 11) | (1 << 1);
const RGB_BIAS: u32 = RGB_UNIT * 2 * RGB_BUILDER;
const CLAMP_MASK: u32 = RGB_BUILDER * 3 / 2;
const CLAMP_ADD: u32 = RGB_BUILDER * (0x101 as u32);
const LUMA_CUTOFF: f32 = 0.20;
const ARTIFACTS_MID: f32 = 1.0;
const FRINGING_MID: f32 = 1.0;
const ARTIFACTS_MAX: f32 = ARTIFACTS_MID * 1.5;
const FRINGING_MAX: f32 = FRINGING_MID * 2.0;
const PI: f32 = 3.14159265358979323846;
const EXT_DECODER_HUE: f32 = 0.0;
const NES_NTSC_BLACK: usize = 15;

const DEFAULT_DECODER: [f32; 6] = [0.956, 0.621, -0.272, -0.647, -1.105, 1.702];

const LO_LEVELS: [f32; 4] = [-0.12, 0.00, 0.31, 0.72];
const HI_LEVELS: [f32; 4] = [0.40, 0.68, 1.00, 1.00];
const PHASES: [f32; 0x10 + 3] = [
    -1.0, -0.866025, -0.5, 0.0, 0.5, 0.866025,
    1.0, 0.866025, 0.5, 0.0, -0.5, -0.866025,
    -1.0, -0.866025, -0.5, 0.0, 0.5, 0.866025,
    1.0,
];
const TINT_SIN: [f32; 8] = [
    PHASES[0], PHASES[6], PHASES[10], PHASES[8], PHASES[2], PHASES[4], PHASES[0], PHASES[0],
];
const TINT_COS: [f32; 8] = [
    PHASES[3], PHASES[9], PHASES[13], PHASES[11], PHASES[5], PHASES[7], PHASES[3], PHASES[3],
];

const ATTEN_MUL: f32 = 0.79399;
const ATTEN_SUB: f32 = 0.0782838;

const PIXELS: [(usize, f32, [f32; 4]); 3] = [
    (353, 1.0, [1.0, 1.0, 0.6667, 0.0]),
    (22, -1.0, [0.3333, 1.0, 1.0, 0.3333]),
    (154, 1.0, [0.0, 0.6667, 1.0, 1.0]),
];

#[derive(Clone, Copy)]
struct Setup {
    hue: f64,
    saturation: f64,
    contrast: f64,
    brightness: f64,
    sharpness: f64,
    gamma: f64,
    resolution: f64,
    artifacts: f64,
    fringing: f64,
    bleed: f64,
    merge_fields: bool,
}

const DEFAULTS: Setup = Setup {
    hue: 0.0,
    saturation: 0.0,
    contrast: 0.0,
    brightness: 0.0,
    sharpness: 0.0,
    gamma: 0.0,
    resolution: 0.0,
    artifacts: 0.0,
    fringing: 0.0,
    bleed: 0.0,
    merge_fields: false,
};

struct InitT {
    to_rgb: [f32; BURST_COUNT * 6],
    #[allow(dead_code)]
    to_float: [f32; 1],
    contrast: f32,
    brightness: f32,
    artifacts: f32,
    fringing: f32,
    kernel: [f32; RESCALE_OUT * KERNEL_SIZE * 2],
}

fn rgb_to_yiq(r: f32, g: f32, b: f32, y: &mut f32, i: &mut f32) -> f32 {
    *y = r * 0.299 + g * 0.587 + b * 0.114;
    *i = r * 0.596 - g * 0.275 - b * 0.321;
    r * 0.212 - g * 0.523 + b * 0.311
}

fn yiq_to_rgb_f(y: f32, i: f32, q: f32, to_rgb: &[f32]) -> (f32, f32, f32) {
    let r = y + to_rgb[0] * i + to_rgb[1] * q;
    let g = y + to_rgb[2] * i + to_rgb[3] * q;
    let b = y + to_rgb[4] * i + to_rgb[5] * q;
    (r, g, b)
}

fn yiq_to_rgb_i(y: f32, i: f32, q: f32, to_rgb: &[f32]) -> (i32, i32, i32) {
    let r = (y + to_rgb[0] * i + to_rgb[1] * q) as i32;
    let g = (y + to_rgb[2] * i + to_rgb[3] * q) as i32;
    let b = (y + to_rgb[4] * i + to_rgb[5] * q) as i32;
    (r, g, b)
}

#[inline]
fn pack_rgb(r: i32, g: i32, b: i32) -> u32 {
    (r as u32) << 21 | (g as u32) << 11 | (b as u32) << 1
}

#[inline]
fn pack_rgb_bias(r: i32, g: i32, b: i32) -> u32 {
    pack_rgb(r, g, b).wrapping_sub(RGB_BIAS)
}

fn f32_pow(a: f32, b: f32) -> f32 {
    (a as f64).powf(b as f64) as f32
}
fn f32_cos(x: f32) -> f32 {
    (x as f64).cos() as f32
}

fn init_filters(impl_: &mut InitT, setup: &Setup) {
    let mut kernels = [0f32; KERNEL_SIZE * 2];

    let rolloff: f32 = 1.0 + setup.sharpness as f32 * 0.032;
    let maxh: f32 = 32.0;
    let pow_a_n: f32 = f32_pow(rolloff, maxh);
    let mut to_angle: f32 = setup.resolution as f32 + 1.0;
    to_angle = PI / maxh * LUMA_CUTOFF * (to_angle * to_angle + 1.0);

    kernels[KERNEL_SIZE * 3 / 2] = maxh;
    for i in 0..(KERNEL_HALF * 2 + 1) {
        let x = i as i32 - KERNEL_HALF as i32;
        let angle = x as f32 * to_angle;
        if x != 0 || pow_a_n > 1.056 || pow_a_n < 0.981 {
            let rolloff_cos_a: f32 = rolloff * f32_cos(angle);
            let num: f32 = (1.0 - rolloff_cos_a as f64
                - pow_a_n as f64 * f32_cos(maxh * angle) as f64
                + pow_a_n as f64 * rolloff as f64 * f32_cos((maxh - 1.0) * angle) as f64) as f32;
            let den: f32 = (1.0 - rolloff_cos_a as f64 - rolloff_cos_a as f64 + rolloff as f64 * rolloff as f64) as f32;
            let dsf: f32 = (num as f64 / den as f64) as f32;
            kernels[KERNEL_SIZE * 3 / 2 - KERNEL_HALF + i] = dsf - 0.5;
        }
    }

    let mut sum: f32 = 0.0;
    for i in 0..(KERNEL_HALF * 2 + 1) {
        let x = PI * 2.0 / (KERNEL_HALF * 2) as f32 * i as f32;
        let blackman: f32 = 0.42 - 0.5 * f32_cos(x) + 0.08 * f32_cos(x * 2.0);
        let idx = KERNEL_SIZE * 3 / 2 - KERNEL_HALF + i;
        kernels[idx] *= blackman;
        sum += kernels[idx];
    }
    sum = 1.0 / sum;
    for i in 0..(KERNEL_HALF * 2 + 1) {
        let idx = KERNEL_SIZE * 3 / 2 - KERNEL_HALF + i;
        kernels[idx] *= sum;
    }

    let cutoff_factor: f32 = -0.03125;
    let mut cutoff: f32 = setup.bleed as f32;
    if cutoff < 0.0 {
        cutoff *= cutoff;
        cutoff *= cutoff;
        cutoff *= cutoff;
        cutoff *= -30.0 / 0.65;
    }
    cutoff = cutoff_factor - 0.65 * cutoff_factor * cutoff;
    for i in -(KERNEL_HALF as i32)..=(KERNEL_HALF as i32) {
        let idx = (KERNEL_SIZE / 2) as i32 + i;
        kernels[idx as usize] = ((((i * i) as f32 * cutoff) as f64).exp()) as f32;
    }

    for ph in 0..2 {
        let mut sum: f32 = 0.0;
        let mut x = ph as i32;
        while (x as usize) < KERNEL_SIZE {
            sum += kernels[x as usize];
            x += 2;
        }
        let inv = 1.0 / sum;
        let mut x = ph as i32;
        while (x as usize) < KERNEL_SIZE {
            kernels[x as usize] *= inv;
            x += 2;
        }
    }

    {
        let mut weight: f32 = 1.0;
        let mut out: usize = 0;
        for _ in 0..RESCALE_OUT {
            let mut remain: f32 = 0.0;
            weight -= 1.0 / RESCALE_IN as f32;
            for i in 0..(KERNEL_SIZE * 2) {
                let cur = kernels[i];
                let m = cur * weight;
                impl_.kernel[out] = m + remain;
                remain = cur - m;
                out += 1;
            }
        }
    }
}

fn init(impl_: &mut InitT, setup: &Setup) {
    impl_.brightness = setup.brightness as f32 * (0.5 * RGB_UNIT as f32) + RGB_OFFSET;
    impl_.contrast = setup.contrast as f32 * (0.5 * RGB_UNIT as f32) + RGB_UNIT as f32;

    impl_.artifacts = setup.artifacts as f32;
    if impl_.artifacts > 0.0 {
        impl_.artifacts *= ARTIFACTS_MAX - ARTIFACTS_MID;
    }
    impl_.artifacts = impl_.artifacts * ARTIFACTS_MID + ARTIFACTS_MID;

    impl_.fringing = setup.fringing as f32;
    if impl_.fringing > 0.0 {
        impl_.fringing *= FRINGING_MAX - FRINGING_MID;
    }
    impl_.fringing = impl_.fringing * FRINGING_MID + FRINGING_MID;

    init_filters(impl_, setup);

    {
        let hue: f32 = setup.hue as f32 * PI + PI / 180.0 * EXT_DECODER_HUE;
        let sat: f32 = setup.saturation as f32 + 1.0;
        let mut s: f32 = f32_sin(hue) * sat;
        let mut c: f32 = f32_cos(hue) * sat;
        let mut out: usize = 0;
        let mut n = BURST_COUNT;
        loop {
            for k in 0..3 {
                let i = DEFAULT_DECODER[k * 2];
                let q = DEFAULT_DECODER[k * 2 + 1];
                impl_.to_rgb[out] = i * c - q * s;
                impl_.to_rgb[out + 1] = i * s + q * c;
                out += 2;
            }
            if BURST_COUNT <= 1 {
                break;
            }
            let t = s * -0.5 - c * 0.866025;
            c = s * 0.866025 + c * -0.5;
            s = t;
            n -= 1;
            if n == 0 {
                break;
            }
        }
    }
}

fn f32_sin(x: f32) -> f32 {
    (x as f64).sin() as f32
}

fn gen_kernel(impl_: &InitT, mut y: f32, mut i: f32, mut q: f32, out: &mut [u32; ENTRY_SIZE]) {
    let mut to_rgb_idx: usize = 0;
    let mut burst_remain = BURST_COUNT;
    y -= RGB_OFFSET;
    let mut oi: usize = 0;
    loop {
        for pixel in PIXELS.iter() {
            let (offset, negate, kernel) = *pixel;
            let yy = y * impl_.fringing * negate;
            let ic0 = (i + yy) * kernel[0];
            let qc1 = (q + yy) * kernel[1];
            let ic2 = (i - yy) * kernel[2];
            let qc3 = (q - yy) * kernel[3];

            let factor = impl_.artifacts * negate;
            let ii = i * factor;
            let yc0 = (y + ii) * kernel[0];
            let yc2 = (y - ii) * kernel[2];

            let qq = q * factor;
            let yc1 = (y + qq) * kernel[1];
            let yc3 = (y - qq) * kernel[3];

            let mut kpos = offset;
            let tr = &impl_.to_rgb[to_rgb_idx..to_rgb_idx + 6];
            for _ in 0..RGB_KERNEL_SIZE {
                let k = &impl_.kernel;
                let ii = k[kpos] * ic0 + k[kpos + 2] * ic2;
                let qq = k[kpos + 1] * qc1 + k[kpos + 3] * qc3;
                let yy = k[kpos + KERNEL_SIZE] * yc0
                    + k[kpos + KERNEL_SIZE + 1] * yc1
                    + k[kpos + KERNEL_SIZE + 2] * yc2
                    + k[kpos + KERNEL_SIZE + 3] * yc3
                    + RGB_OFFSET;
                if kpos < KERNEL_SIZE * 2 * (RESCALE_OUT - 1) {
                    kpos += KERNEL_SIZE * 2 - 1;
                } else {
                    kpos -= KERNEL_SIZE * 2 * (RESCALE_OUT - 1) + 2;
                }
                let (r, g, b) = yiq_to_rgb_i(yy, ii, qq, tr);
                out[oi] = pack_rgb_bias(r, g, b);
                oi += 1;
            }
        }
        if BURST_COUNT <= 1 {
            break;
        }
        to_rgb_idx += 6;
        let t = i * -0.5 - q * -0.866025;
        q = i * -0.866025 + q * -0.5;
        i = t;
        burst_remain -= 1;
        if burst_remain == 0 {
            break;
        }
    }
}

fn merge_kernel_fields(io: &mut [u32; ENTRY_SIZE]) {
    for n in 0..BURST_SIZE {
        let p0 = io[BURST_SIZE * 0 + n].wrapping_add(RGB_BIAS);
        let p1 = io[BURST_SIZE * 1 + n].wrapping_add(RGB_BIAS);
        let p2 = io[BURST_SIZE * 2 + n].wrapping_add(RGB_BIAS);
        io[BURST_SIZE * 0 + n] =
            (p0.wrapping_add(p1).wrapping_sub((p0 ^ p1) & RGB_BUILDER) >> 1).wrapping_sub(RGB_BIAS);
        io[BURST_SIZE * 1 + n] =
            (p1.wrapping_add(p2).wrapping_sub((p1 ^ p2) & RGB_BUILDER) >> 1).wrapping_sub(RGB_BIAS);
        io[BURST_SIZE * 2 + n] =
            (p2.wrapping_add(p0).wrapping_sub((p2 ^ p0) & RGB_BUILDER) >> 1).wrapping_sub(RGB_BIAS);
    }
}

fn correct_errors(color: u32, out: &mut [u32; ENTRY_SIZE]) {
    let mut base: usize = 0;
    for _ in 0..BURST_COUNT {
        for i in 0..(RGB_KERNEL_SIZE / 2) {
            let error = color
                .wrapping_sub(out[base + i])
                .wrapping_sub(out[base + (i + 12) % 14 + 14])
                .wrapping_sub(out[base + (i + 10) % 14 + 28])
                .wrapping_sub(out[base + i + 7])
                .wrapping_sub(out[base + i + 5 + 14])
                .wrapping_sub(out[base + i + 3 + 28]);
            let a = i + 3 + 28;
            let b = i + 5 + 14;
            let c = i + 7;
            let mut fourth = error.wrapping_add(2 * RGB_BUILDER) >> 2;
            fourth &= (RGB_BIAS >> 1).wrapping_sub(RGB_BUILDER);
            fourth = fourth.wrapping_sub(RGB_BIAS >> 2);
            out[base + a] = out[base + a].wrapping_add(fourth);
            out[base + b] = out[base + b].wrapping_add(fourth);
            out[base + c] = out[base + c].wrapping_add(fourth);
            out[base + i] = out[base + i].wrapping_add(error.wrapping_sub(fourth.wrapping_mul(3)));
        }
        base += ALIGNMENT_COUNT * RGB_KERNEL_SIZE;
    }
}

pub struct NtscBlargg {
    table: Box<[[u32; ENTRY_SIZE]; PALETTE_SIZE]>,
}

fn nes_ntsc_init_into(table: &mut [[u32; ENTRY_SIZE]; PALETTE_SIZE], setup: &Setup, palette: &[u32; PALETTE_SIZE]) {
    let mut impl_ = InitT {
        to_rgb: [0.0; BURST_COUNT * 6],
        to_float: [0.0; 1],
        contrast: 0.0,
        brightness: 0.0,
        artifacts: 0.0,
        fringing: 0.0,
        kernel: [0.0; RESCALE_OUT * KERNEL_SIZE * 2],
    };
    init(&mut impl_, setup);

    #[allow(unused_mut)]
    let mut gamma: f32 = setup.gamma as f32 * -0.5;
    let gamma_factor: f32 = f32_pow(gamma.abs(), 0.73);
    let gamma_factor = if gamma < 0.0 { -gamma_factor } else { gamma_factor };

    let merge_fields = setup.merge_fields;

    #[allow(unused_assignments)]
    for entry in 0..PALETTE_SIZE {
        let level = (entry >> 4) & 0x03;
        let mut lo = LO_LEVELS[level];
        let mut hi = HI_LEVELS[level];

        let color = entry & 0x0F;
        if color == 0 {
            lo = hi;
        }
        if color == 0x0D {
            hi = lo;
        }
        if color > 0x0D {
            hi = 0.0;
            lo = 0.0;
        }

        let sat = (hi - lo) * 0.5;
        let mut i = PHASES[color] * sat;
        let mut q = PHASES[color + 3] * sat;
        let mut y = (hi + lo) * 0.5;

        {
            let tint = (entry >> 6) & 7;
            if tint != 0 && color <= 0x0D {
                if tint == 7 {
                    y = y * (ATTEN_MUL * 1.13) - (ATTEN_SUB * 1.13);
                } else {
                    let tint_color = tint;
                    let mut sat = hi * (0.5 - ATTEN_MUL * 0.5) + ATTEN_SUB * 0.5;
                    y -= sat * 0.5;
                    if tint >= 3 && tint != 4 {
                        sat *= 0.6;
                        y -= sat;
                    }
                    i += TINT_SIN[tint_color] * sat;
                    q += TINT_COS[tint_color] * sat;
                }
            }
        }

        {
            let rgb = palette[entry];
            let r = ((rgb >> 16) & 0xFF) as f32 * (1.0 / 0xFF as f32);
            let g = ((rgb >> 8) & 0xFF) as f32 * (1.0 / 0xFF as f32);
            let b = (rgb & 0xFF) as f32 * (1.0 / 0xFF as f32);
            q = rgb_to_yiq(r, g, b, &mut y, &mut i);
        }

        y *= setup.contrast as f32 * 0.5 + 1.0;
        y += setup.brightness as f32 * 0.5 - 0.5 / 256.0;

        {
            let (mut r, mut g, mut b) = yiq_to_rgb_f(y, i, q, &DEFAULT_DECODER);
            r = (r * gamma_factor - gamma_factor) * r + r;
            g = (g * gamma_factor - gamma_factor) * g + g;
            b = (b * gamma_factor - gamma_factor) * b + b;
            q = rgb_to_yiq(r, g, b, &mut y, &mut i);
        }

        i *= RGB_UNIT as f32;
        q *= RGB_UNIT as f32;
        y *= RGB_UNIT as f32;
        y += RGB_OFFSET;

        let (r, g, b) = yiq_to_rgb_i(y, i, q, &impl_.to_rgb);
        let b_clamped = if b < 0x3E0 { b } else { 0x3E0 };
        let rgb: u32 = pack_rgb(r, g, b_clamped);

        gen_kernel(&impl_, y, i, q, &mut table[entry]);
        if merge_fields {
            merge_kernel_fields(&mut table[entry]);
        }
        correct_errors(rgb, &mut table[entry]);
    }
}

impl NtscBlargg {
    pub fn new(palette: &[u32; PALETTE_SIZE]) -> Self {
        let mut table = Box::new([[0u32; ENTRY_SIZE]; PALETTE_SIZE]);
        nes_ntsc_init_into(&mut table, &DEFAULTS, palette);
        NtscBlargg { table }
    }

    pub fn filter_frame(&self, ppu: &[u16], video_phase: u32) -> Vec<u32> {
        const BASE_W: usize = 256;
        const BASE_H: usize = 240;
        const FULL_W: usize = ((BASE_W - 1) / 3 + 1) * 7;
        const OUT_W: usize = FULL_W;
        const OUT_H: usize = BASE_H * 2;

        let mut full = vec![0u32; FULL_W * BASE_H];
        let chunk_count = (BASE_W - 1) / 3;

        for y in 0..BASE_H {
            let line_in = &ppu[y * BASE_W..(y + 1) * BASE_W];
            let b = (video_phase as usize + y) % BURST_COUNT;
            let row = &mut full[y * FULL_W..(y + 1) * FULL_W];

            let entry = |e: usize| &self.table[e % PALETTE_SIZE][b * BURST_SIZE..b * BURST_SIZE + BURST_SIZE];

            let mut k0 = entry(NES_NTSC_BLACK);
            let mut k1 = entry(NES_NTSC_BLACK);
            let mut k2 = entry(line_in[0] as usize);
            let mut kx0;
            let mut kx1 = k0;
            let mut kx2 = k0;
            let mut li = 1;
            let mut lo = 0;

            for _ in 0..chunk_count {
                kx0 = k0;
                k0 = entry(line_in[li] as usize);
                row[lo] = rgb_out(k0, k1, k2, kx0, kx1, kx2, 0);
                row[lo + 1] = rgb_out(k0, k1, k2, kx0, kx1, kx2, 1);

                kx1 = k1;
                k1 = entry(line_in[li + 1] as usize);
                row[lo + 2] = rgb_out(k0, k1, k2, kx0, kx1, kx2, 2);
                row[lo + 3] = rgb_out(k0, k1, k2, kx0, kx1, kx2, 3);

                kx2 = k2;
                k2 = entry(line_in[li + 2] as usize);
                row[lo + 4] = rgb_out(k0, k1, k2, kx0, kx1, kx2, 4);
                row[lo + 5] = rgb_out(k0, k1, k2, kx0, kx1, kx2, 5);
                row[lo + 6] = rgb_out(k0, k1, k2, kx0, kx1, kx2, 6);

                li += 3;
                lo += 7;
            }

            kx0 = k0;
            k0 = entry(NES_NTSC_BLACK);
            row[lo] = rgb_out(k0, k1, k2, kx0, kx1, kx2, 0);
            row[lo + 1] = rgb_out(k0, k1, k2, kx0, kx1, kx2, 1);

            kx1 = k1;
            k1 = entry(NES_NTSC_BLACK);
            row[lo + 2] = rgb_out(k0, k1, k2, kx0, kx1, kx2, 2);
            row[lo + 3] = rgb_out(k0, k1, k2, kx0, kx1, kx2, 3);

            kx2 = k2;
            k2 = entry(NES_NTSC_BLACK);
            row[lo + 4] = rgb_out(k0, k1, k2, kx0, kx1, kx2, 4);
            row[lo + 5] = rgb_out(k0, k1, k2, kx0, kx1, kx2, 5);
            row[lo + 6] = rgb_out(k0, k1, k2, kx0, kx1, kx2, 6);
        }

        let mut out = vec![0u32; OUT_W * OUT_H];
        for r in 0..OUT_H / 2 {
            let src_start = r * FULL_W;
            let src = &full[src_start..src_start + OUT_W];
            out[r * 2 * OUT_W..(r * 2 + 1) * OUT_W].copy_from_slice(src);
            out[(r * 2 + 1) * OUT_W..(r * 2 + 2) * OUT_W].copy_from_slice(src);
        }
        out
    }
}

#[inline]
fn rgb_out(k0: &[u32], k1: &[u32], k2: &[u32], kx0: &[u32], kx1: &[u32], kx2: &[u32], x: usize) -> u32 {
    let mut io = k0[x]
        .wrapping_add(k1[(x + 12) % 7 + 14])
        .wrapping_add(k2[(x + 10) % 7 + 28])
        .wrapping_add(kx0[(x + 7) % 14])
        .wrapping_add(kx1[(x + 5) % 7 + 21])
        .wrapping_add(kx2[(x + 3) % 7 + 35]);
    let sub = (io >> 9) & CLAMP_MASK;
    let mut clamp = CLAMP_ADD.wrapping_sub(sub);
    io |= clamp;
    clamp = clamp.wrapping_sub(sub);
    io &= clamp;
    0xFF000000 | (io >> 5 & 0xFF0000) | (io >> 3 & 0xFF00) | (io >> 1 & 0xFF)
}