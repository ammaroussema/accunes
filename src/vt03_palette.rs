use std::f32::consts::PI;
use std::sync::OnceLock;

static VT03_PALETTE: OnceLock<[u32; 4096]> = OnceLock::new();

pub fn get_vt03_palette() -> &'static [u32; 4096] {
    VT03_PALETTE.get_or_init(generate_vt03_palette)
}

fn generate_vt03_palette() -> [u32; 4096] {
    let mut palette = [0u32; 4096];
    for i in 0..4096 {
        let mut n_phase = i & 0xF;
        let mut n_luma = (i >> 4) & 0xF;
        let mut n_chroma = (i >> 8) & 0xF;

        let inverted = n_luma < ((n_chroma + 1) >> 1) || n_luma > (15 - (n_chroma >> 1));
        if inverted {
            static ALT_PHASES: [usize; 16] = [13, 7, 8, 9, 10, 11, 12, 1, 2, 3, 4, 5, 6, 0, 14, 15];
            n_phase = ALT_PHASES[n_phase];
            n_chroma = 16 - n_chroma;
            n_luma = (n_luma.wrapping_sub(8)) & 0xF;
        }

        let f_luma = n_luma as f32 / 15.0;
        let f_chroma = n_chroma as f32 / 30.0;
        let phase_offset = 0.0f32;

        let f_phase = (((n_phase as f32 - 2.0) * 30.0) + phase_offset) * PI / 180.0;
        let mut y = f_luma;
        let mut c = f_chroma;

        if n_phase == 0 || n_phase > 12 {
            c = 0.0;
        }
        if n_phase == 0 {
            y += f_chroma;
        }
        if n_phase == 13 {
            y -= f_chroma;
        }
        if n_phase >= 14 {
            y = 0.0;
        }

        let v = f_phase.sin() * c;
        let u = f_phase.cos() * c;
        let mut r = y + 1.1400 * v + 0.0000 * u;
        let mut g = y - 0.5807 * v - 0.3940 * u;
        let mut b = y - 0.0000 * v + 2.0290 * u;

        r = (r - 4.0 / 15.0) / (1.0 - 4.0 / 15.0) * 15.0 / 13.0;
        g = (g - 4.0 / 15.0) / (1.0 - 4.0 / 15.0) * 15.0 / 13.0;
        b = (b - 4.0 / 15.0) / (1.0 - 4.0 / 15.0) * 15.0 / 13.0;

        r = (r - 0.075) / 0.925;
        g = (g - 0.075) / 0.925;
        b = (b - 0.075) / 0.925;

        r = r.clamp(0.0, 1.0);
        g = g.clamp(0.0, 1.0);
        b = b.clamp(0.0, 1.0);

        let rv = (r * 255.0) as u32;
        let gv = (g * 255.0) as u32;
        let bv = (b * 255.0) as u32;

        palette[i] = (rv << 16) | (gv << 8) | bv;
    }
    palette
}
