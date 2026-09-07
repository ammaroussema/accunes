// bisqwit’s NTSC filter, ported from mesen!!!
const SIGNALS_PER_PIXEL: usize = 8;
const LINE_WIDTH: usize = 256;
const SIGNAL_WIDTH: usize = LINE_WIDTH * SIGNALS_PER_PIXEL;
const RES_DIVIDER: i32 = 4;
const OUTPUT_SCALE: usize = (8 / RES_DIVIDER) as usize;
const FRAME_W: usize = 256 * OUTPUT_SCALE;
const FRAME_H: usize = 240 * OUTPUT_SCALE;

const BITMASK_LUT: [u16; 12] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x100, 0x200, 0x400, 0x800];

const EMPHASIS_LUT: [u16; 8] = [
    0,
    0b000000111111,
    0b001111110000,
    0b001111111111,
    0b111100000011,
    0b111100111111,
    0b111111110011,
    0b111111111111,
];

const SIGNAL_LUMA_LOW: [[f64; 4]; 2] = [[0.228, 0.312, 0.552, 0.880], [0.192, 0.256, 0.448, 0.712]];
const SIGNAL_LUMA_HIGH: [[f64; 4]; 2] = [[0.616, 0.840, 1.100, 1.100], [0.500, 0.676, 0.896, 0.896]];

pub struct NtscBisqwit {
    signal_low: [i8; 0x80],
    signal_high: [i8; 0x80],
    sinetable: [i8; 27],
    y_width: i32,
    y: i32,
    ir: i32,
    ig: i32,
    ib: i32,
    qr: i32,
    qg: i32,
    qb: i32,
    brightness: i32,
}

#[inline]
fn clamp255(v: i32) -> i32 {
    if v < 0 {
        0
    } else if v > 255 {
        255
    } else {
        v
    }
}

impl NtscBisqwit {
    pub fn new() -> Self {
        let signal_blank = 0.312;
        let signal_white = 1.100;
        let mut signal_low = [0i8; 0x80];
        let mut signal_high = [0i8; 0x80];
        for h in 0..=1 {
            for i in 0..=0x3F {
                let mut m = SIGNAL_LUMA_LOW[h][i / 0x10];
                let mut q = SIGNAL_LUMA_HIGH[h][i / 0x10];
                if (i & 0x0F) == 0x0D {
                    q = m;
                } else if (i & 0x0F) == 0 {
                    m = q;
                } else if (i & 0x0F) >= 0x0E {
                    m = signal_blank;
                    q = signal_blank;
                }
                let idx = (if h == 1 { 0x40 } else { 0 }) | i;
                signal_low[idx] = (((m - signal_blank) / (signal_white - signal_blank)) * 100.0).floor() as i8;
                signal_high[idx] = (((q - signal_blank) / (signal_white - signal_blank)) * 100.0).floor() as i8;
            }
        }

        let pi = std::f64::consts::PI;
        let mut sinetable = [0i8; 27];
        for i in 0..27 {
            sinetable[i] = (8.0 * (i as f64 * 2.0 * pi / 12.0).sin()) as i8;
        }

        let contrast = ((0.0 + 1.0) * (0.0 + 1.0) * 167941.0) as i32;
        let saturation = ((0.0 + 1.0) * (0.0 + 1.0) * 144044.0) as i32;
        let brightness = 0i32;

        let y_width = 12i32 + 24i32;

        let y = contrast / y_width;
        let ir = (contrast as f64 * 1.994681e-6 * saturation as f64 / y_width as f64) as i32;
        let qr = (contrast as f64 * 9.915742e-7 * saturation as f64 / y_width as f64) as i32;
        let ig = (contrast as f64 * 9.151351e-8 * saturation as f64 / y_width as f64) as i32;
        let qg = (contrast as f64 * -6.334805e-7 * saturation as f64 / y_width as f64) as i32;
        let ib = (contrast as f64 * -1.012984e-6 * saturation as f64 / y_width as f64) as i32;
        let qb = (contrast as f64 * 1.667217e-6 * saturation as f64 / y_width as f64) as i32;

        NtscBisqwit {
            signal_low,
            signal_high,
            sinetable,
            y_width,
            y,
            ir,
            ig,
            ib,
            qr,
            qg,
            qb,
            brightness,
        }
    }

    fn generate_ntsc_signal(&self, ntsc_signal: &mut [i8; SIGNAL_WIDTH], phase: &mut i32, row: usize, ppu: &[u16]) {
        for x in 0..256 {
            let ppu_data = ppu[(row << 8) | x];
            let pixel_color = ppu_data & 0x3F;
            let emphasis = (ppu_data >> 6) as usize;
            let hue = (ppu_data & 0x0F) as usize;

            let mut emphasis_wave: u32 = 0;
            if emphasis != 0 {
                let e = EMPHASIS_LUT[emphasis % 8] as u32;
                let sh = hue % 12;
                emphasis_wave = ((e >> sh) | (e << (12 - sh))) & 0xFFFF;
            }

            let mut phase_bitmask = BITMASK_LUT[((*phase - hue as i32).abs()) as usize % 12];
            for j in 0..SIGNALS_PER_PIXEL {
                phase_bitmask = phase_bitmask.wrapping_shl(1);

                let color = (pixel_color | if phase_bitmask & emphasis_wave as u16 != 0 { 0x40 } else { 0 }) as usize;
                let mut voltage = self.signal_high[color];

                if phase_bitmask >= (1 << 12) {
                    phase_bitmask = 1;
                } else if phase_bitmask >= (1 << 6) {
                    voltage = self.signal_low[color];
                }
                ntsc_signal[(x << 3) | j] = voltage;
            }

            *phase += SIGNALS_PER_PIXEL as i32;
        }
        *phase += (341 - 256) * SIGNALS_PER_PIXEL as i32;
    }

    fn ntsc_decode_line(&self, width: usize, signal: &[i8], target: &mut [u32], phase0: usize) {
        const PAD: usize = 36;
        let mut pbuf = [0i8; SIGNAL_WIDTH + PAD * 2];
        pbuf[PAD..PAD + width].copy_from_slice(signal);

        let mut ysum = self.brightness;
        let mut isum = 0i32;
        let mut qsum = 0i32;

        let mut out = 0usize;
        let mut cind = 0usize;
        let mut s = -(self.y_width / 2);
        while s < width as i32 {
            let hi = (s + PAD as i32 + self.y_width / 2) as usize;
            let lo = hi - self.y_width as usize;
            let inc = pbuf[hi] as i32 - pbuf[lo] as i32;
            ysum += inc;
            let cv = self.sinetable[cind + phase0] as i32;
            let sv = self.sinetable[cind + 3 + phase0] as i32;
            isum += cv * inc;
            qsum += sv * inc;
            cind += 1;
            if cind == 12 {
                cind = 0;
            }
            if s >= 0 && s & 3 == 0 {
                let r = clamp255((ysum * self.y + isum * self.ir + qsum * self.qr) / 65536);
                let g = clamp255((ysum * self.y + isum * self.ig + qsum * self.qg) / 65536);
                let b = clamp255((ysum * self.y + isum * self.ib + qsum * self.qb) / 65536);
                target[out] =
                    0xFF000000u32 | (((r as u32) & 0xFF) << 16) | (((g as u32) & 0xFF) << 8) | ((b as u32) & 0xFF);
                out += 1;
            }
            s += 1;
        }
    }

    pub fn filter_frame(&self, ppu: &[u16], video_phase: u32) -> Vec<u32> {
        let mut out = vec![0u32; FRAME_W * FRAME_H];
        let mut phase = video_phase as i32 * 4;
        let mut row_signal = [0i8; SIGNAL_WIDTH];

        let start_row = 0usize;
        let end_row = 239usize;
        for y in start_row..=end_row {
            let start_cycle = phase % 12;
            self.generate_ntsc_signal(&mut row_signal, &mut phase, y, ppu);
            let phase0 = ((start_cycle + 7) % 12) as usize;
            let row_offset = y * 2 * FRAME_W;
            self.ntsc_decode_line(SIGNAL_WIDTH, &row_signal, &mut out[row_offset..row_offset + FRAME_W], phase0);
            out.copy_within(row_offset..row_offset + FRAME_W, row_offset + FRAME_W);
        }
        out
    }
}