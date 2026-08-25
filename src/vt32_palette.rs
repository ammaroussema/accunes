use std::sync::OnceLock;

static VT32_PALETTE: OnceLock<[u32; 4096]> = OnceLock::new();
static VT32_8BPP_PALETTE: OnceLock<[u32; 4096]> = OnceLock::new();

pub fn get_vt32_palette() -> &'static [u32; 4096] {
    VT32_PALETTE.get_or_init(|| generate_vt32_palette(true))
}
pub fn get_vt32_8bpp_palette() -> &'static [u32; 4096] {
    VT32_8BPP_PALETTE.get_or_init(|| generate_vt32_palette(false))
}

fn generate_vt32_palette(with_blue_cast_removal: bool) -> [u32; 4096] {
    let mut palette = [0u32; 4096];
    for i in 0..4096 {
        let mut r = (i & 0x0F) as u32;
        if with_blue_cast_removal {
            r = r * 0xF / 0xD;
            if r > 0xF { r = 0xF; }
        }
        let g = ((i >> 4) & 0x0F) as u32;
        let b = ((i >> 8) & 0x0F) as u32;
        let r = r * 0xFF / 0x0F;
        let g = g * 0xFF / 0x0F;
        let b = b * 0xFF / 0x0F;
        palette[i] = (r << 16) | (g << 8) | b;
    }
    palette
}
