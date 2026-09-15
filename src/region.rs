#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Region {
    Auto,
    Ntsc,
    Pal,
    Dendy,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TvSystem {
    Ntsc,
    Pal,
    Dendy,
    Dual,
    Unknown,
}

impl TvSystem {
    pub fn from_ines_header(rom: &[u8], is_nes20: bool) -> Self {
        if is_nes20 && rom.len() > 12 {
            match rom[12] & 0x03 {
                0 => TvSystem::Ntsc,
                1 => TvSystem::Pal,
                2 => TvSystem::Dual,
                3 => TvSystem::Dendy,
                _ => TvSystem::Unknown,
            }
        } else {
            if rom.len() > 7 && (rom[7] & 1) != 0 {
                TvSystem::Pal
            } else {
                TvSystem::Unknown
            }
        }
    }

    pub fn from_filename(path: &str) -> Self {
        let lower = path.to_lowercase();
        if lower.contains("(e)")
            || lower.contains("(europe)")
            || lower.contains("(germany)")
            || lower.contains("(france)")
            || lower.contains("(spain)")
            || lower.contains("(italy)")
            || lower.contains("(australia)")
            || lower.contains("(sweden)")
            || lower.contains("(euro)")
        {
            TvSystem::Pal
        } else if lower.contains("(dendy)")
            || lower.contains("(russia)")
        {
            TvSystem::Dendy
        } else if lower.contains("(u)")
            || lower.contains("(usa)")
            || lower.contains("(japan)")
            || lower.contains("(j)")
            || lower.contains("(world)")
        {
            TvSystem::Ntsc
        } else {
            TvSystem::Unknown
        }
    }
}


