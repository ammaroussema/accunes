// the PAL video filter, by feos, hardwareman, r57shell and ported from fceux!!!

const PAL_PHASES: usize = 108;
const OUTPUT_SCALE: usize = 3;
const FRAME_W: usize = 256 * OUTPUT_SCALE;
const FRAME_H: usize = 240;
const PALETTE_SIZE: usize = 512;

const PHASEX: f64 = 5.0 / 18.0 * 2.0;
const PHASEY: f64 = 1.0 / 6.0 * 2.0;
const PI: f64 = 3.14;

#[inline]
fn round_f64(value: f64) -> i32 {
    (value + 0.5).floor() as i32
}

#[inline]
fn clamp255(v: i32) -> u32 {
    if v < 0 {
        0
    } else if v > 255 {
        255
    } else {
        v as u32
    }
}

#[inline]
fn blend_channels(c1: u32, w1: u32, c2: u32, w2: u32) -> u32 {
    let r = ((c1 >> 16) & 0xFF) * w1 + ((c2 >> 16) & 0xFF) * w2;
    let g = ((c1 >> 8) & 0xFF) * w1 + ((c2 >> 8) & 0xFF) * w2;
    let b = (c1 & 0xFF) * w1 + (c2 & 0xFF) * w2;
    (((r / 100) & 0xFF) << 16) | (((g / 100) & 0xFF) << 8) | ((b / 100) & 0xFF)
}

pub struct PalFilter {
    pal_rgb: Box<[u32; PALETTE_SIZE * PAL_PHASES]>,
    pal_rgb2: Box<[u32; PALETTE_SIZE * PAL_PHASES]>,
    sharp: u32,
    unsharp: u32,
}

impl PalFilter {
    pub fn new(palette: &[u32; PALETTE_SIZE]) -> Self {
        let palnotch = 100i32;
        let palsaturation = 100i32;
        let palsharpness = 0i32;
        let palcontrast = 100i32;
        let palbrightness = 50i32;

        let sat = palsaturation as f64 / 100.0;
        let contrast = palcontrast as f64 / 100.0;
        let bright = (palbrightness - 50) as f64;
        let notch = palnotch as u32;
        let unnotch = (100 - palnotch) as u32;
        let sharp = (50 + palsharpness) as u32;
        let unsharp = (50 - palsharpness) as u32;

        let mut pal_rgb = [0u32; PALETTE_SIZE * PAL_PHASES];
        let mut pal_rgb2 = [0u32; PALETTE_SIZE * PAL_PHASES];
        let mut moire = [0f64; PAL_PHASES];
        let mut mix_r = [0i32; PAL_PHASES];
        let mut mix_g = [0i32; PAL_PHASES];
        let mut mix_b = [0i32; PAL_PHASES];

        for i in 0..PALETTE_SIZE {
            let packed = palette[i];
            let r = (packed >> 16) & 0xFF;
            let g = (packed >> 8) & 0xFF;
            let b = packed & 0xFF;

            let y = 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
            let u = -0.14713 * r as f64 - 0.28886 * g as f64 + 0.436 * b as f64;
            let v = 0.615 * r as f64 - 0.51499 * g as f64 - 0.10001 * b as f64;

            for x in 0..18 {
                for yv in 0..6 {
                    let mut alpha = (x as f64 * PHASEX + yv as f64 * PHASEY) * PI;
                    if yv % 2 == 0 {
                        alpha = -alpha;
                    }
                    moire[x + yv * 18] = y + u * alpha.sin() + v * alpha.cos();
                }
            }

            for j in 0..PAL_PHASES {
                let cr = round_f64(moire[j] * contrast + bright + 1.13983 * v * sat);
                let cg = round_f64(moire[j] * contrast + bright - 0.39465 * u * sat - 0.58060 * v * sat);
                let cb = round_f64(moire[j] * contrast + bright + 2.03211 * u * sat);

                let r = clamp255(cr);
                let g = clamp255(cg);
                let b = clamp255(cb);

                mix_r[j] = r as i32;
                mix_g[j] = g as i32;
                mix_b[j] = b as i32;

                pal_rgb[i * PAL_PHASES + j] = (r << 16) | (g << 8) | b;
            }

            let avg_r = (mix_r[0] + mix_r[1] + mix_r[2] + mix_r[3] + mix_r[4] + mix_r[5]) / 6;
            let avg_g = (mix_g[0] + mix_g[1] + mix_g[2] + mix_g[3] + mix_g[4] + mix_g[5]) / 6;
            let avg_b = (mix_b[0] + mix_b[1] + mix_b[2] + mix_b[3] + mix_b[4] + mix_b[5]) / 6;

            for j in 0..PAL_PHASES {
                let r = ((mix_r[j] as u32 * unnotch + avg_r as u32 * notch) / 100) as i32;
                let g = ((mix_g[j] as u32 * unnotch + avg_g as u32 * notch) / 100) as i32;
                let b = ((mix_b[j] as u32 * unnotch + avg_b as u32 * notch) / 100) as i32;

                pal_rgb2[i * PAL_PHASES + j] = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
            }
        }

        PalFilter {
            pal_rgb: Box::new(pal_rgb),
            pal_rgb2: Box::new(pal_rgb2),
            sharp,
            unsharp,
        }
    }

    pub fn filter_frame(&self, ppu: &[u16]) -> Vec<u32> {
        let mut out = vec![0u32; FRAME_W * FRAME_H];
        let sharp = self.sharp;
        let unsharp = self.unsharp;

        let mut lastindex: u32 = 0;
        let mut lastcolor: u32 = 0;

        for y in 0..FRAME_H {
            let row_off = y * FRAME_W;
            let vphase = 18 * (y % 6);
            let mut out_x = 0usize;

            for x in 0..256 {
                let cur = (ppu[y * 256 + x] & 0x1FF) as usize;
                let nxt = if x + 1 < 256 {
                    (ppu[y * 256 + x + 1] & 0x1FF) as usize
                } else {
                    cur
                };
                let base_idx = cur * PAL_PHASES;

                for xsub in 0..OUTPUT_SCALE {
                    let xabs = x * OUTPUT_SCALE + xsub;
                    let ph = (xabs % 18) + vphase;
                    let moirecolor = self.pal_rgb[base_idx + ph];
                    let notchcolor = self.pal_rgb2[base_idx + ph];

                    let color = if (cur as u32 != nxt as u32 && xsub == 2)
                        || (cur as u32 != lastindex && xsub == 0)
                    {
                        blend_channels(moirecolor, 90, notchcolor, 10)
                    } else if (cur as u32 != nxt as u32 && xsub == 1)
                        || (cur as u32 != lastindex && xsub == 1)
                    {
                        blend_channels(moirecolor, 60, notchcolor, 40)
                    } else if (cur as u32 != nxt as u32 && xsub == 0)
                        || (cur as u32 != lastindex && xsub == 2)
                    {
                        blend_channels(moirecolor, 30, notchcolor, 70)
                    } else {
                        notchcolor
                    };

                    let finalcolor;
                    if color != lastcolor && sharp < 100 {
                        finalcolor = blend_channels(color, sharp, lastcolor, unsharp);
                        lastcolor = blend_channels(lastcolor, sharp, color, unsharp);
                        if out_x > 0 {
                            out[row_off + out_x - 1] = lastcolor;
                        }
                    } else {
                        finalcolor = color;
                    }
                    lastcolor = color;
                    out[row_off + out_x] = finalcolor;
                    out_x += 1;
                }

                lastindex = cur as u32;
            }
        }

        out
    }
}