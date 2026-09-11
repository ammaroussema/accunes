use crate::region::Region;
use std::path::{Path, PathBuf};

const CONFIG_FILE: &str = "accunes.cfg";

fn config_path() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = exe.parent().unwrap_or(&std::path::Path::new("."));
    dir.join(CONFIG_FILE)
}

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .map(|p| p.parent().unwrap_or(Path::new(".")).to_path_buf())
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn save_file_path(rom_path: &str) -> PathBuf {
    let path = Path::new(rom_path);
    let name = path.file_stem().unwrap_or(path.as_os_str());
    PathBuf::from(load_saves_dir()).join(name).with_extension("sav")
}

pub fn turbofile_save_path(rom_path: &str) -> PathBuf {
    let path = Path::new(rom_path);
    let name = path.file_stem().unwrap_or(path.as_os_str());
    PathBuf::from(load_saves_dir()).join(format!("{}.turbofile.sav", name.to_string_lossy()))
}

pub fn battlebox_save_path(rom_path: &str) -> PathBuf {
    let path = Path::new(rom_path);
    let name = path.file_stem().unwrap_or(path.as_os_str());
    PathBuf::from(load_saves_dir()).join(format!("{}.battlebox.sav", name.to_string_lossy()))
}

pub fn state_file_path(rom_path: &str, slot: usize) -> PathBuf {
    let path = Path::new(rom_path);
    let name = path.file_stem().unwrap_or(path.as_os_str());
    PathBuf::from(load_savestates_dir()).join(format!("{}.state{}", name.to_string_lossy(), slot))
}

pub fn load_roms_dir() -> String {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("roms_dir=") {
                return value.trim().to_string();
            }
        }
    }
    String::new()
}

pub fn save_roms_dir(dir: &str) {
    upsert_config("roms_dir", dir);
}

pub fn load_saves_dir() -> String {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("saves_dir=") {
                let v = value.trim().to_string();
                if !v.is_empty() {
                    return v;
                }
            }
        }
    }
    exe_dir().join("saves").to_string_lossy().into_owned()
}

pub fn save_saves_dir(dir: &str) {
    upsert_config("saves_dir", dir);
}

pub fn load_cheats_dir() -> String {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("cheats_dir=") {
                let v = value.trim().to_string();
                if !v.is_empty() {
                    return v;
                }
            }
        }
    }
    let dir = exe_dir().join("cheats");
    let _ = std::fs::create_dir_all(&dir);
    dir.to_string_lossy().into_owned()
}

#[allow(dead_code)]
pub fn save_cheats_dir(dir: &str) {
    upsert_config("cheats_dir", dir);
}

pub fn load_savestates_dir() -> String {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("savestates_dir=") {
                let v = value.trim().to_string();
                if !v.is_empty() {
                    return v;
                }
            }
        }
    }
    exe_dir().join("savestates").to_string_lossy().into_owned()
}

pub fn save_savestates_dir(dir: &str) {
    upsert_config("savestates_dir", dir);
}

pub fn load_screenshots_dir() -> String {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("screenshots_dir=") {
                let v = value.trim().to_string();
                if !v.is_empty() {
                    return v;
                }
            }
        }
    }
    String::new()
}

pub fn save_screenshots_dir(dir: &str) {
    upsert_config("screenshots_dir", dir);
}

pub fn load_region() -> Region {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("region=") {
                match value.trim().to_lowercase().as_str() {
                    "ntsc" => return Region::Ntsc,
                    "pal" => return Region::Pal,
                    "dendy" => return Region::Dendy,
                    _ => return Region::Auto,
                }
            }
        }
    }
    Region::Auto
}

fn upsert_config(key: &str, value: &str) {
    let path = config_path();
    let mut lines = Vec::new();
    let mut found = false;
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(existing_key) = trimmed.split('=').next() {
                if existing_key.trim() == key {
                    lines.push(format!("{}={}", key, value));
                    found = true;
                    continue;
                }
            }
            lines.push(line.to_string());
        }
    }
    if !found {
        lines.push(format!("{}={}", key, value));
    }
    let _ = std::fs::write(&path, lines.join("\n") + "\n");
}

pub fn save_region(region: Region) {
    let name = match region {
        Region::Ntsc => "NTSC",
        Region::Pal => "PAL",
        Region::Dendy => "Dendy",
        Region::Auto => "Auto",
    };
    upsert_config("region", name);
}

pub fn load_pause_on_lost_focus() -> bool {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("pause_on_lost_focus=") {
                let value = trimmed.strip_prefix("pause_on_lost_focus=").unwrap_or("").trim();
                return value.eq_ignore_ascii_case("yes") || value == "1" || value.eq_ignore_ascii_case("true");
            }
        }
    }
    false
}

pub fn save_pause_on_lost_focus(enabled: bool) {
    upsert_config("pause_on_lost_focus", if enabled { "yes" } else { "no" });
}

pub fn load_check_updates_on_startup() -> bool {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("check_updates_on_startup=") {
                let value = trimmed.strip_prefix("check_updates_on_startup=").unwrap_or("").trim();
                return value.eq_ignore_ascii_case("yes") || value == "1" || value.eq_ignore_ascii_case("true");
            }
        }
    }
    false
}

pub fn save_check_updates_on_startup(enabled: bool) {
    upsert_config("check_updates_on_startup", if enabled { "yes" } else { "no" });
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum InitialRam {
    Default,
    Zero,
    AllFF,
    Random,
}

pub fn load_initial_ram() -> InitialRam {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("initial_ram=") {
                match value.trim().to_lowercase().as_str() {
                    "0x00" | "00" | "zero" => return InitialRam::Zero,
                    "0xff" | "ff" | "allff" => return InitialRam::AllFF,
                    "random" => return InitialRam::Random,
                    _ => return InitialRam::Default,
                }
            }
        }
    }
    InitialRam::Default
}

pub fn save_initial_ram(mode: InitialRam) {
    let s = match mode {
        InitialRam::Default => "default",
        InitialRam::Zero => "0x00",
        InitialRam::AllFF => "0xFF",
        InitialRam::Random => "random",
    };
    upsert_config("initial_ram", s);
}

impl InitialRam {
    pub fn next(self) -> Self {
        match self {
            InitialRam::Default => InitialRam::Zero,
            InitialRam::Zero => InitialRam::AllFF,
            InitialRam::AllFF => InitialRam::Random,
            InitialRam::Random => InitialRam::Default,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            InitialRam::Default => "Default",
            InitialRam::Zero => "0x00",
            InitialRam::AllFF => "0xFF",
            InitialRam::Random => "Random",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FpsMode {
    Off,
    Window,
    Overlay,
}

pub fn load_fps_mode() -> FpsMode {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("fps_mode=") {
                match value.trim().to_lowercase().as_str() {
                    "window" => return FpsMode::Window,
                    "overlay" => return FpsMode::Overlay,
                    _ => return FpsMode::Off,
                }
            }
        }
    }
    FpsMode::Off
}

pub fn save_fps_mode(mode: FpsMode) {
    let s = match mode {
        FpsMode::Off => "off",
        FpsMode::Window => "window",
        FpsMode::Overlay => "overlay",
    };
    upsert_config("fps_mode", s);
}

impl FpsMode {
    pub fn next(self) -> Self {
        match self {
            FpsMode::Off => FpsMode::Window,
            FpsMode::Window => FpsMode::Overlay,
            FpsMode::Overlay => FpsMode::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FpsMode::Off => "Off",
            FpsMode::Window => "Window",
            FpsMode::Overlay => "Overlay",
        }
    }
}

pub fn load_confirm_on_exit() -> bool {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("confirm_on_exit=") {
                let v = value.trim().to_lowercase();
                return v == "yes" || v == "1" || v == "true" || v == "on";
            }
        }
    }
    true
}

pub fn save_confirm_on_exit(enabled: bool) {
    upsert_config("confirm_on_exit", if enabled { "on" } else { "off" });
}

pub fn load_auto_save_sram() -> bool {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("auto_save_sram=") {
                let v = value.trim().to_lowercase();
                return v == "yes" || v == "1" || v == "true" || v == "on";
            }
        }
    }
    true
}

pub fn save_auto_save_sram(enabled: bool) {
    upsert_config("auto_save_sram", if enabled { "on" } else { "off" });
}

pub fn load_fds_bios_path() -> String {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("fds_bios_path=") {
                let v = value.trim().to_string();
                if !v.is_empty() {
                    return v;
                }
            }
        }
    }
    "disksys.rom".to_string()
}

pub fn save_fds_bios_path(bios_path: &str) {
    upsert_config("fds_bios_path", bios_path);
}

pub fn load_study_box_bios_path() -> String {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("study_box_bios_path=") {
                let v = value.trim().to_string();
                if !v.is_empty() {
                    return v;
                }
            }
        }
    }
    "StudyBox.bin".to_string()
}

pub fn save_study_box_bios_path(bios_path: &str) {
    upsert_config("study_box_bios_path", bios_path);
}

pub fn load_audio_enabled() -> bool {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("audio_enabled=") {
                let v = value.trim().to_lowercase();
                return v == "yes" || v == "1" || v == "true" || v == "on";
            }
        }
    }
    true
}

pub fn save_audio_enabled(enabled: bool) {
    upsert_config("audio_enabled", if enabled { "yes" } else { "no" });
}

pub fn load_audio_rate() -> u32 {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("audio_rate=") {
                if let Ok(rate) = value.trim().parse::<u32>() {
                    match rate {
                        11025 | 22050 | 32000 | 44100 | 48000 | 96000 => return rate,
                        _ => {}
                    }
                }
            }
        }
    }
    48000
}

pub fn save_audio_rate(rate: u32) {
    upsert_config("audio_rate", &rate.to_string());
}

pub fn load_audio_depth() -> u8 {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("audio_depth=") {
                match value.trim() {
                    "8" => return 8,
                    _ => return 16,
                }
            }
        }
    }
    16
}

pub fn save_audio_depth(depth: u8) {
    upsert_config("audio_depth", if depth == 8 { "8" } else { "16" });
}

pub fn load_swap_duty_cycles() -> bool {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("swap_duty_cycles=") {
                let value = trimmed.strip_prefix("swap_duty_cycles=").unwrap_or("").trim();
                return value.eq_ignore_ascii_case("yes") || value == "1" || value.eq_ignore_ascii_case("true");
            }
        }
    }
    false
}

pub fn save_swap_duty_cycles(enabled: bool) {
    upsert_config("swap_duty_cycles", if enabled { "yes" } else { "no" });
}

pub const CHANNEL_NAMES: &[&str] = &["master", "triangle", "square1", "square2", "noise", "pcm", "expansion"];

pub fn load_channel_volume(channel: &str) -> u8 {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            let key = format!("channel_volume_{}=", channel);
            if let Some(value) = trimmed.strip_prefix(&key) {
                if let Ok(v) = value.trim().parse::<u8>() {
                    return v.min(100);
                }
            }
        }
    }
    100
}

pub fn save_channel_volume(channel: &str, volume: u8) {
    let v = volume.min(100);
    upsert_config(&format!("channel_volume_{}", channel), &v.to_string());
}


fn load_bool_config(key: &str, default: bool) -> bool {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix(&format!("{}=", key)) {
                let v = value.trim().to_lowercase();
                return v == "yes" || v == "1" || v == "true" || v == "on";
            }
        }
    }
    default
}

fn save_bool_config(key: &str, enabled: bool) {
    upsert_config(key, if enabled { "on" } else { "off" });
}

pub fn load_fullscreen() -> bool { load_bool_config("fullscreen", false) }
pub fn save_fullscreen(enabled: bool) { save_bool_config("fullscreen", enabled); }

pub fn load_fullscreen_on_game_load() -> bool { load_bool_config("fullscreen_on_game_load", false) }
pub fn save_fullscreen_on_game_load(enabled: bool) { save_bool_config("fullscreen_on_game_load", enabled); }

pub fn load_hide_mouse_cursor() -> bool { load_bool_config("hide_mouse_cursor", false) }
pub fn save_hide_mouse_cursor(enabled: bool) { save_bool_config("hide_mouse_cursor", enabled); }

pub fn load_crop_overscan() -> bool { load_bool_config("crop_overscan", false) }
pub fn save_crop_overscan(enabled: bool) { save_bool_config("crop_overscan", enabled); }

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AspectRatio {
    Auto,
    Ntsc,
    Pal,
    Standard,
}

pub fn load_aspect_ratio() -> AspectRatio {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("aspect_ratio=") {
                match value.trim().to_lowercase().as_str() {
                    "ntsc" => return AspectRatio::Ntsc,
                    "pal" => return AspectRatio::Pal,
                    "standard" => return AspectRatio::Standard,
                    _ => return AspectRatio::Auto,
                }
            }
        }
    }
    AspectRatio::Auto
}

pub fn save_aspect_ratio(aspect: AspectRatio) {
    let s = match aspect {
        AspectRatio::Auto => "auto",
        AspectRatio::Ntsc => "ntsc",
        AspectRatio::Pal => "pal",
        AspectRatio::Standard => "standard",
    };
    upsert_config("aspect_ratio", s);
}

impl AspectRatio {
    pub fn next(self) -> Self {
        match self {
            AspectRatio::Auto => AspectRatio::Ntsc,
            AspectRatio::Ntsc => AspectRatio::Pal,
            AspectRatio::Pal => AspectRatio::Standard,
            AspectRatio::Standard => AspectRatio::Auto,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AspectRatio::Auto => "Auto",
            AspectRatio::Ntsc => "NTSC",
            AspectRatio::Pal => "PAL",
            AspectRatio::Standard => "Standard",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PaletteMode {
    Auto,
    Ntsc,
    Pal,
    Vs,
}

pub fn load_palette_mode() -> PaletteMode {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("palette=") {
                match value.trim().to_lowercase().as_str() {
                    "ntsc" => return PaletteMode::Ntsc,
                    "pal" => return PaletteMode::Pal,
                    "vs" => return PaletteMode::Vs,
                    _ => return PaletteMode::Auto,
                }
            }
        }
    }
    PaletteMode::Auto
}

pub fn load_custom_palette_path(kind: &str) -> Option<String> {
    let key = format!("custom_{}_palette", kind);
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix(&format!("{}=", key)) {
                let v = value.trim().to_string();
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

pub fn save_custom_palette(kind: &str, pal_path: &str) {
    upsert_config(&format!("custom_{}_palette", kind), pal_path);
}

pub fn clear_custom_palette(kind: &str) {
    upsert_config(&format!("custom_{}_palette", kind), "");
}

pub fn save_palette_mode(mode: PaletteMode) {
    let s = match mode {
        PaletteMode::Auto => "auto",
        PaletteMode::Ntsc => "ntsc",
        PaletteMode::Pal => "pal",
        PaletteMode::Vs => "vs",
    };
    upsert_config("palette", s);
}

impl PaletteMode {
    pub fn next(self) -> Self {
        match self {
            PaletteMode::Auto => PaletteMode::Ntsc,
            PaletteMode::Ntsc => PaletteMode::Pal,
            PaletteMode::Pal => PaletteMode::Vs,
            PaletteMode::Vs => PaletteMode::Auto,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PaletteMode::Auto => "Auto",
            PaletteMode::Ntsc => "NTSC",
            PaletteMode::Pal => "PAL",
            PaletteMode::Vs => "VS",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VideoFilter {
    None,
    Scanlines,
    LcdGrid,
    NtscBlargg,
    NtscBisqwit,
    Pal3x,
    Prescale2x,
    Prescale3x,
    Prescale4x,
    Prescale6x,
    Prescale8x,
    Prescale10x,
    Scale2x,
    Scale3x,
    TwoXSaI,
    SuperTwoXSaI,
    SuperEagle,
    Hq2x,
    Hq3x,
    Hq4x,
    Xbrz2x,
    Xbrz3x,
    Xbrz4x,
    Xbrz5x,
    Xbrz6x,
}

pub fn load_video_filter() -> VideoFilter {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("video_filter=") {
                match value.trim().to_lowercase().as_str() {
                    "scanlines" => return VideoFilter::Scanlines,
                    "lcd_grid" => return VideoFilter::LcdGrid,
                    "ntsc_blargg" => return VideoFilter::NtscBlargg,
                    "ntsc_bisqwit" => return VideoFilter::NtscBisqwit,
                    "pal_3x" => return VideoFilter::Pal3x,
                    "prescale_2x" => return VideoFilter::Prescale2x,
                    "prescale_3x" => return VideoFilter::Prescale3x,
                    "prescale_4x" => return VideoFilter::Prescale4x,
                    "prescale_6x" => return VideoFilter::Prescale6x,
                    "prescale_8x" => return VideoFilter::Prescale8x,
                    "prescale_10x" => return VideoFilter::Prescale10x,
                    "scale2x" => return VideoFilter::Scale2x,
                    "scale3x" => return VideoFilter::Scale3x,
                    "2xsai" => return VideoFilter::TwoXSaI,
                    "super2xsai" => return VideoFilter::SuperTwoXSaI,
                    "supereagle" => return VideoFilter::SuperEagle,
                    "hq2x" => return VideoFilter::Hq2x,
                    "hq3x" => return VideoFilter::Hq3x,
                    "hq4x" => return VideoFilter::Hq4x,
                    "xbrz_2x" => return VideoFilter::Xbrz2x,
                    "xbrz_3x" => return VideoFilter::Xbrz3x,
                    "xbrz_4x" => return VideoFilter::Xbrz4x,
                    "xbrz_5x" => return VideoFilter::Xbrz5x,
                    "xbrz_6x" => return VideoFilter::Xbrz6x,
                    _ => return VideoFilter::None,
                }
            }
        }
    }
    VideoFilter::None
}

pub fn save_video_filter(filter: VideoFilter) {
    let s = match filter {
        VideoFilter::None => "none",
        VideoFilter::Scanlines => "scanlines",
        VideoFilter::LcdGrid => "lcd_grid",
        VideoFilter::NtscBlargg => "ntsc_blargg",
        VideoFilter::NtscBisqwit => "ntsc_bisqwit",
        VideoFilter::Pal3x => "pal_3x",
        VideoFilter::Prescale2x => "prescale_2x",
        VideoFilter::Prescale3x => "prescale_3x",
        VideoFilter::Prescale4x => "prescale_4x",
        VideoFilter::Prescale6x => "prescale_6x",
        VideoFilter::Prescale8x => "prescale_8x",
        VideoFilter::Prescale10x => "prescale_10x",
        VideoFilter::Scale2x => "scale2x",
        VideoFilter::Scale3x => "scale3x",
        VideoFilter::TwoXSaI => "2xsai",
        VideoFilter::SuperTwoXSaI => "super2xsai",
        VideoFilter::SuperEagle => "supereagle",
        VideoFilter::Hq2x => "hq2x",
        VideoFilter::Hq3x => "hq3x",
        VideoFilter::Hq4x => "hq4x",
        VideoFilter::Xbrz2x => "xbrz_2x",
        VideoFilter::Xbrz3x => "xbrz_3x",
        VideoFilter::Xbrz4x => "xbrz_4x",
        VideoFilter::Xbrz5x => "xbrz_5x",
        VideoFilter::Xbrz6x => "xbrz_6x",
    };
    upsert_config("video_filter", s);
}

impl VideoFilter {
    pub fn next(self) -> Self {
        match self {
            VideoFilter::None => VideoFilter::Scanlines,
            VideoFilter::Scanlines => VideoFilter::LcdGrid,
            VideoFilter::LcdGrid => VideoFilter::NtscBlargg,
            VideoFilter::NtscBlargg => VideoFilter::NtscBisqwit,
            VideoFilter::NtscBisqwit => VideoFilter::Pal3x,
            VideoFilter::Pal3x => VideoFilter::Prescale2x,
            VideoFilter::Prescale2x => VideoFilter::Prescale3x,
            VideoFilter::Prescale3x => VideoFilter::Prescale4x,
            VideoFilter::Prescale4x => VideoFilter::Prescale6x,
            VideoFilter::Prescale6x => VideoFilter::Prescale8x,
            VideoFilter::Prescale8x => VideoFilter::Prescale10x,
            VideoFilter::Prescale10x => VideoFilter::Scale2x,
            VideoFilter::Scale2x => VideoFilter::Scale3x,
            VideoFilter::Scale3x => VideoFilter::TwoXSaI,
            VideoFilter::TwoXSaI => VideoFilter::SuperTwoXSaI,
            VideoFilter::SuperTwoXSaI => VideoFilter::SuperEagle,
            VideoFilter::SuperEagle => VideoFilter::Hq2x,
            VideoFilter::Hq2x => VideoFilter::Hq3x,
            VideoFilter::Hq3x => VideoFilter::Hq4x,
            VideoFilter::Hq4x => VideoFilter::Xbrz2x,
            VideoFilter::Xbrz2x => VideoFilter::Xbrz3x,
            VideoFilter::Xbrz3x => VideoFilter::Xbrz4x,
            VideoFilter::Xbrz4x => VideoFilter::Xbrz5x,
            VideoFilter::Xbrz5x => VideoFilter::Xbrz6x,
            VideoFilter::Xbrz6x => VideoFilter::None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            VideoFilter::None => "None",
            VideoFilter::Scanlines => "Scanlines",
            VideoFilter::LcdGrid => "LCD Grid",
            VideoFilter::NtscBlargg => "NTSC (Blargg)",
            VideoFilter::NtscBisqwit => "NTSC (Bisqwit)",
            VideoFilter::Pal3x => "PAL 3x",
            VideoFilter::Prescale2x => "Prescale 2x",
            VideoFilter::Prescale3x => "Prescale 3x",
            VideoFilter::Prescale4x => "Prescale 4x",
            VideoFilter::Prescale6x => "Prescale 6x",
            VideoFilter::Prescale8x => "Prescale 8x",
            VideoFilter::Prescale10x => "Prescale 10x",
            VideoFilter::Scale2x => "Scale2x",
            VideoFilter::Scale3x => "Scale3x",
            VideoFilter::TwoXSaI => "2xSaI",
            VideoFilter::SuperTwoXSaI => "Super2xSaI",
            VideoFilter::SuperEagle => "SuperEagle",
            VideoFilter::Hq2x => "HQ2x",
            VideoFilter::Hq3x => "HQ3x",
            VideoFilter::Hq4x => "HQ4x",
            VideoFilter::Xbrz2x => "xBRZ 2x",
            VideoFilter::Xbrz3x => "xBRZ 3x",
            VideoFilter::Xbrz4x => "xBRZ 4x",
            VideoFilter::Xbrz5x => "xBRZ 5x",
            VideoFilter::Xbrz6x => "xBRZ 6x",
        }
    }

    pub fn scale_factor(self) -> Option<u32> {
        match self {
            VideoFilter::Prescale2x | VideoFilter::Scale2x | VideoFilter::TwoXSaI
            | VideoFilter::SuperTwoXSaI | VideoFilter::SuperEagle | VideoFilter::Hq2x
            | VideoFilter::Xbrz2x => Some(2),
            VideoFilter::NtscBlargg | VideoFilter::NtscBisqwit | VideoFilter::Pal3x => None,
            VideoFilter::Prescale3x | VideoFilter::Scale3x | VideoFilter::Hq3x | VideoFilter::Xbrz3x => Some(3),
            VideoFilter::Prescale4x | VideoFilter::Hq4x | VideoFilter::Xbrz4x => Some(4),
            VideoFilter::Prescale6x | VideoFilter::Xbrz6x => Some(6),
            VideoFilter::Prescale8x => Some(8),
            VideoFilter::Prescale10x => Some(10),
            VideoFilter::Xbrz5x => Some(5),
            _ => None,
        }
    }
}

pub fn load_theme() -> String {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("theme=") {
                let v = value.trim().to_lowercase();
                    match v.as_str() {
                        "light" => return "light".to_string(),
                        "classicnes" => return "classicnes".to_string(),
                        "famicom" => return "famicom".to_string(),
                        "mario" => return "mario".to_string(),
                        "link" => return "link".to_string(),
                        "contra" => return "contra".to_string(),
                        "megaman" => return "megaman".to_string(),
                        _ => return "dark".to_string(),
                    }
            }
        }
    }
    "dark".to_string()
}

pub fn save_theme(theme: &str) {
    let val = match theme {
        "light" => "light",
        "classicnes" => "classicnes",
        "famicom" => "famicom",
        "mario" => "mario",
        "link" => "link",
        "contra" => "contra",
        "megaman" => "megaman",
        _ => "dark",
    };
    upsert_config("theme", val);
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ControllerType {
    None,
    Gamepad,
    FamicomGamepad,
    Zapper,
    Paddle,
    PowerPadA,
    PowerPadB,
    SNESPad,
    SNESMouse,
    SuborMouse,
    PS2Mouse,
    YuxingMouse,
    BelsonicMouse,
    MegaBookMouse,
    SudokuExcalibur,
    SudokuExcalibur2,
    FourScore,
    VirtualBoy,
}

impl ControllerType {
    pub fn next(self) -> Self {
        match self {
            ControllerType::None => ControllerType::Gamepad,
            ControllerType::Gamepad => ControllerType::FamicomGamepad,
            ControllerType::FamicomGamepad => ControllerType::Zapper,
            ControllerType::Zapper => ControllerType::Paddle,
            ControllerType::Paddle => ControllerType::PowerPadA,
            ControllerType::PowerPadA => ControllerType::PowerPadB,
            ControllerType::PowerPadB => ControllerType::SNESPad,
            ControllerType::SNESPad => ControllerType::SNESMouse,
            ControllerType::SNESMouse => ControllerType::SuborMouse,
            ControllerType::SuborMouse => ControllerType::PS2Mouse,
            ControllerType::PS2Mouse => ControllerType::YuxingMouse,
            ControllerType::YuxingMouse => ControllerType::BelsonicMouse,
            ControllerType::BelsonicMouse => ControllerType::MegaBookMouse,
            ControllerType::MegaBookMouse => ControllerType::SudokuExcalibur,
            ControllerType::SudokuExcalibur => ControllerType::SudokuExcalibur2,
            ControllerType::SudokuExcalibur2 => ControllerType::FourScore,
            ControllerType::FourScore => ControllerType::VirtualBoy,
            ControllerType::VirtualBoy => ControllerType::None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            ControllerType::None => "None",
            ControllerType::Gamepad => "NES Gamepad",
            ControllerType::FamicomGamepad => "Famicom Gamepad",
            ControllerType::Zapper => "Zapper",
            ControllerType::Paddle => "Paddle",
            ControllerType::PowerPadA => "Power Pad A",
            ControllerType::PowerPadB => "Power Pad B",
            ControllerType::SNESPad => "SNES Pad",
            ControllerType::SNESMouse => "SNES Mouse",
            ControllerType::SuborMouse => "Subor Mouse",
            ControllerType::PS2Mouse => "PS/2 Mouse",
            ControllerType::YuxingMouse => "Yuxing Mouse",
            ControllerType::BelsonicMouse => "Macro Winners Mouse",
            ControllerType::MegaBookMouse => "Mega Book Mouse",
            ControllerType::SudokuExcalibur => "Sudoku",
            ControllerType::SudokuExcalibur2 => "Sudoku",
            ControllerType::FourScore => "Four Score",
            ControllerType::VirtualBoy => "Virtual Boy Gamepad",
        }
    }
    pub fn is_sudoku(self) -> bool {
        matches!(self, ControllerType::SudokuExcalibur | ControllerType::SudokuExcalibur2)
    }
    pub fn is_serial_mouse(self) -> bool {
        matches!(self, ControllerType::PS2Mouse | ControllerType::YuxingMouse | ControllerType::BelsonicMouse | ControllerType::MegaBookMouse)
    }
}

pub fn load_controller_type(key: &str) -> ControllerType {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix(&format!("{}=", key)) {
                return match value.trim().to_lowercase().as_str() {
                    "gamepad" | "nes gamepad" | "nesgamepad" | "nes_gamepad" | "nes controller" => ControllerType::Gamepad,
                    "famicomgamepad" | "famicom gamepad" | "famicom_gamepad" | "famicom controller" | "famicomcontroller" | "famicom" => ControllerType::FamicomGamepad,
                    "zapper" => ControllerType::Zapper,
                    "paddle" => ControllerType::Paddle,
                    "powerpada" | "power pad a" => ControllerType::PowerPadA,
                    "powerpadb" | "power pad b" => ControllerType::PowerPadB,
                    "snespad" | "snes pad" => ControllerType::SNESPad,
                    "snesmouse" | "snes mouse" => ControllerType::SNESMouse,
                    "subormouse" | "subor mouse" => ControllerType::SuborMouse,
                    "ps2mouse" | "ps2 mouse" | "ps/2 mouse" | "ps/2mouse" | "ps2" => ControllerType::PS2Mouse,
                    "yuxingmouse" | "yuxing mouse" => ControllerType::YuxingMouse,
                    "belsonicmouse" | "belsonic mouse" | "macro winners mouse" | "macrowinnersmouse" | "macro winners" | "macrowinners" => ControllerType::BelsonicMouse,
                    "megabookmouse" | "mega book mouse" | "megabook" => ControllerType::MegaBookMouse,
                    "sudokuexcalibur" | "sudoku excalibur" | "sudoku" | "sudokuexcalibur1" | "sudoku excalibur (port 1 only)" | "sudoku1" => ControllerType::SudokuExcalibur,
                    "sudokuexcalibur2" | "sudoku excalibur 2" | "sudoku2" | "sudoku excalibur (port 2 only)" => ControllerType::SudokuExcalibur2,
                    "fourscore" | "four score" => ControllerType::FourScore,
                    "virtualboy" | "virtual boy" | "virtualboygamepad" | "virtual boy gamepad" | "virtualboycontroller" | "virtual boy controller" | "virtualboypad" | "virtual boy pad" => ControllerType::VirtualBoy,
                    _ => ControllerType::None,
                };
            }
        }
    }
    ControllerType::Gamepad
}

pub fn save_controller_type(key: &str, ct: ControllerType) {
    let s = match ct {
        ControllerType::None => "none",
        ControllerType::Gamepad => "gamepad",
        ControllerType::FamicomGamepad => "famicomgamepad",
        ControllerType::Zapper => "zapper",
        ControllerType::Paddle => "paddle",
        ControllerType::PowerPadA => "powerpada",
        ControllerType::PowerPadB => "powerpadb",
        ControllerType::SNESPad => "snespad",
        ControllerType::SNESMouse => "snesmouse",
        ControllerType::SuborMouse => "subormouse",
        ControllerType::PS2Mouse => "ps2mouse",
        ControllerType::YuxingMouse => "yuxingmouse",
        ControllerType::BelsonicMouse => "belsonicmouse",
        ControllerType::MegaBookMouse => "megabookmouse",
        ControllerType::SudokuExcalibur => "sudokuexcalibur",
        ControllerType::SudokuExcalibur2 => "sudokuexcalibur2",
        ControllerType::FourScore => "fourscore",
        ControllerType::VirtualBoy => "virtualboy",
    };
    upsert_config(key, s);
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ExpansionType {
    None,
    ArkanoidPaddle,
    FamicomZapper,
    OekaKidsTablet,
    FamilyTrainerA,
    FamilyTrainerB,
    KonamiHyperShot,
    FamilyBasicKeyboard,
    PartyTap,
    PachinkoController,
    ExcitingBoxing,
    JissenMahjong,
    QuizKing,
    SuborKeyboard,
    Pec586Keyboard,
    Bit79Keyboard,
    KedaKeyboard,
    KingwonKeyboard,
    ZeChengKeyboard,
    BarcodeBattler,
    HoriTrack,
    BandaiHyperShot,
    TurboFile,
    BattleBox,
    TopRider,
    FamiNetSys,
    CityPatrolman,
    Moguraa,
    SharpC1Cassette,
    GoldenNuggetCasino,
    ABLPinball,
    TVPump,
    TrifaceMahjong,
    MahjongGekitou,
}

pub fn load_expansion_type() -> ExpansionType {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("expansion_type=") {
                return match value.trim().to_lowercase().as_str() {
                    "arkanoidpaddle" | "arkanoid paddle" | "paddle" => ExpansionType::ArkanoidPaddle,
                    "famicomzapper" | "famicom zapper" | "famicom_zapper" | "zapper" => ExpansionType::FamicomZapper,
                    "oekakidstablet" | "oeka kids tablet" | "oeka_kids_tablet" | "oeka" | "tablet" => ExpansionType::OekaKidsTablet,
                    "familytrainera" | "family trainer a" | "family_trainer_a" | "familytrainersidea" | "fta" => ExpansionType::FamilyTrainerA,
                    "familytrainerb" | "family trainer b" | "family_trainer_b" | "familytrainersideb" | "ftb" => ExpansionType::FamilyTrainerB,
                    "konamihypershot" | "konami hyper shot" | "konami_hyper_shot" | "hypershot" | "hyper shot" => ExpansionType::KonamiHyperShot,
                    "familybasickeyboard" | "family basic keyboard" | "family_basic_keyboard" | "fbkeyboard" | "familybasic" => ExpansionType::FamilyBasicKeyboard,
                    "partytap" | "party tap" | "party_tap" => ExpansionType::PartyTap,
                    "pachinko" | "pachinko controller" | "pachinko_controller" => ExpansionType::PachinkoController,
                    "punchingbag" | "punching bag" | "punching_bag" | "excitingboxing" | "exciting boxing" | "boxing" => ExpansionType::ExcitingBoxing,
                    "jissenmahjong" | "jissen mahjong" | "jissen_mahjong" | "mahjong" => ExpansionType::JissenMahjong,
                    "quizking" | "quiz king" | "quiz_king" | "quizkingbuzzers" => ExpansionType::QuizKing,
                    "suborkeyboard" | "subor keyboard" | "subor_keyboard" => ExpansionType::SuborKeyboard,
                    "pec586keyboard" | "pec586 keyboard" | "pec586_keyboard" | "pec586" | "dongdapec586keyboard" | "dongda" | "dongdakeyboard" | "dongda keyboard" => ExpansionType::Pec586Keyboard,
                    "bit79keyboard" | "bit79 keyboard" | "bit_79_keyboard" | "bit 79 keyboard" | "bit79" | "bit79kb" => ExpansionType::Bit79Keyboard,
                    "kedakeyboard" | "keda keyboard" | "keda_keyboard" | "keda" => ExpansionType::KedaKeyboard,
                    "kingwonkeyboard" | "kingwon keyboard" | "kingwon_keyboard" | "kingwon" => ExpansionType::KingwonKeyboard,
                    "zechengkeyboard" | "zecheng keyboard" | "zecheng_keyboard" | "zecheng" | "ze cheng keyboard" | "ze-cheng" | "zechenginv" => ExpansionType::ZeChengKeyboard,
                    "barcodebattler" | "barcode battler" | "barcode_battler" => ExpansionType::BarcodeBattler,
                    "horitrack" | "hori track" | "hori_track" => ExpansionType::HoriTrack,
                    "bandaihypershot" | "bandai hyper shot" | "bandai_hyper_shot" => ExpansionType::BandaiHyperShot,
                    "turbofile" | "turbo file" | "turbo_file" => ExpansionType::TurboFile,
                    "battlebox" | "battle box" | "battle_box" => ExpansionType::BattleBox,
                    "toprider" | "top rider" | "top_rider" => ExpansionType::TopRider,
                    "faminetsys" | "famicom network system" | "famicom_network_system" | "faminetsystem" | "fns" => ExpansionType::FamiNetSys,
                    "citypatrolman" | "city patrolman" | "city_patrolman" | "patrolman" | "city patrolman lightgun" => ExpansionType::CityPatrolman,
                    "moguraa" | "pokkun moguraa" | "pokkun_moguraa" | "pokkunmoguraa" | "moguraa mat" => ExpansionType::Moguraa,
                    "sharpc1cassette" | "sharp c1 cassette" | "sharp_c1_cassette" | "sharp c1 cassette interface" | "sharpc1" | "c1cassette" => ExpansionType::SharpC1Cassette,
                    "goldennuggetcasino" | "golden nugget casino" | "golden_nugget_casino" | "majesco casino" | "majescocasino" | "casino" => ExpansionType::GoldenNuggetCasino,
                    "ablpinball" | "abl pinball" | "abl_pinball" => ExpansionType::ABLPinball,
                    "tvpump" | "tv pump" | "tv_pump" | "tv pump (18)" => ExpansionType::TVPump,
                    "trifacemahjong" | "triface mahjong" | "triface_mahjong" | "triface mahjong controller" | "mahjong triface" => ExpansionType::TrifaceMahjong,
                    "mahjonggekitou" | "mahjong gekitou" | "mahjong_gekitou" | "mahjong gekitou densetsu" | "gekitou" | "gekitou mahjong" => ExpansionType::MahjongGekitou,
                    _ => ExpansionType::None,
                };
            }
        }
    }
    ExpansionType::None
}

pub fn save_expansion_type(et: ExpansionType) {
    let s = match et {
        ExpansionType::None => "none",
        ExpansionType::ArkanoidPaddle => "arkanoidpaddle",
        ExpansionType::FamicomZapper => "famicomzapper",
        ExpansionType::OekaKidsTablet => "oekakidstablet",
        ExpansionType::FamilyTrainerA => "familytrainera",
        ExpansionType::FamilyTrainerB => "familytrainerb",
        ExpansionType::KonamiHyperShot => "konamihypershot",
        ExpansionType::FamilyBasicKeyboard => "familybasickeyboard",
        ExpansionType::PartyTap => "partytap",
        ExpansionType::PachinkoController => "pachinko",
        ExpansionType::ExcitingBoxing => "punchingbag",
        ExpansionType::JissenMahjong => "jissenmahjong",
        ExpansionType::QuizKing => "quizking",
        ExpansionType::SuborKeyboard => "suborkeyboard",
        ExpansionType::Pec586Keyboard => "pec586keyboard",
        ExpansionType::Bit79Keyboard => "bit79keyboard",
        ExpansionType::KedaKeyboard => "kedakeyboard",
        ExpansionType::KingwonKeyboard => "kingwonkeyboard",
        ExpansionType::ZeChengKeyboard => "zechengkeyboard",
        ExpansionType::BarcodeBattler => "barcodebattler",
        ExpansionType::HoriTrack => "horitrack",
        ExpansionType::BandaiHyperShot => "bandaihypershot",
        ExpansionType::TurboFile => "turbofile",
        ExpansionType::BattleBox => "battlebox",
        ExpansionType::TopRider => "toprider",
        ExpansionType::FamiNetSys => "faminetsys",
        ExpansionType::CityPatrolman => "citypatrolman",
        ExpansionType::Moguraa => "moguraa",
        ExpansionType::SharpC1Cassette => "sharpc1cassette",
        ExpansionType::GoldenNuggetCasino => "goldennuggetcasino",
        ExpansionType::ABLPinball => "ablpinball",
        ExpansionType::TVPump => "tvpump",
        ExpansionType::TrifaceMahjong => "trifacemahjong",
        ExpansionType::MahjongGekitou => "mahjonggekitou",
    };
    upsert_config("expansion_type", s);
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ExpansionAdapterType {
    None,
    TwoPlayer,
    FourPlayer,
    HoriFourPlayer,
}

pub fn load_expansion_adapter_type() -> ExpansionAdapterType {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("expansion_adapter_type=") {
                return match value.trim().to_lowercase().as_str() {
                    "twoplayer" | "2player" | "2-player" | "2 player" => ExpansionAdapterType::TwoPlayer,
                    "fourplayer" | "4player" | "4-player" | "4 player" => ExpansionAdapterType::FourPlayer,
                    "horifourplayer" | "hori4player" | "hori-4player" | "hori 4 player" => ExpansionAdapterType::HoriFourPlayer,
                    _ => ExpansionAdapterType::None,
                };
            }
        }
    }
    ExpansionAdapterType::None
}

pub fn save_expansion_adapter_type(at: ExpansionAdapterType) {
    let s = match at {
        ExpansionAdapterType::None => "none",
        ExpansionAdapterType::TwoPlayer => "twoplayer",
        ExpansionAdapterType::FourPlayer => "fourplayer",
        ExpansionAdapterType::HoriFourPlayer => "horifourplayer",
    };
    upsert_config("expansion_adapter_type", s);
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ExpansionPortType {
    None,
    ArkanoidPaddle,
    FamicomZapper,
    OekaKidsTablet,
    FamilyTrainerA,
    FamilyTrainerB,
    KonamiHyperShot,
    FamilyBasicKeyboard,
    PartyTap,
    PachinkoController,
    ExcitingBoxing,
    JissenMahjong,
    QuizKing,
    SuborKeyboard,
    Pec586Keyboard,
    Bit79Keyboard,
    KedaKeyboard,
    KingwonKeyboard,
    ZeChengKeyboard,
    BarcodeBattler,
    HoriTrack,
    BandaiHyperShot,
    TurboFile,
    BattleBox,
    TopRider,
    FamiNetSys,
    CityPatrolman,
    Moguraa,
    SharpC1Cassette,
    GoldenNuggetCasino,
    ABLPinball,
    TVPump,
    TrifaceMahjong,
    MahjongGekitou,
    TwoPlayerAdapter,
    FourPlayerAdapter,
    HoriFourPlayerAdapter,
}

impl ExpansionPortType {
    pub fn next(self) -> Self {
        match self {
            ExpansionPortType::None => ExpansionPortType::FamicomZapper,
            ExpansionPortType::FamicomZapper => ExpansionPortType::ArkanoidPaddle,
            ExpansionPortType::ArkanoidPaddle => ExpansionPortType::OekaKidsTablet,
            ExpansionPortType::OekaKidsTablet => ExpansionPortType::FamilyTrainerA,
            ExpansionPortType::FamilyTrainerA => ExpansionPortType::FamilyTrainerB,
            ExpansionPortType::FamilyTrainerB => ExpansionPortType::KonamiHyperShot,
            ExpansionPortType::KonamiHyperShot => ExpansionPortType::FamilyBasicKeyboard,
            ExpansionPortType::FamilyBasicKeyboard => ExpansionPortType::PartyTap,
            ExpansionPortType::PartyTap => ExpansionPortType::PachinkoController,
            ExpansionPortType::PachinkoController => ExpansionPortType::ExcitingBoxing,
            ExpansionPortType::ExcitingBoxing => ExpansionPortType::JissenMahjong,
            ExpansionPortType::JissenMahjong => ExpansionPortType::QuizKing,
            ExpansionPortType::QuizKing => ExpansionPortType::SuborKeyboard,
            ExpansionPortType::SuborKeyboard => ExpansionPortType::Pec586Keyboard,
            ExpansionPortType::Pec586Keyboard => ExpansionPortType::Bit79Keyboard,
            ExpansionPortType::Bit79Keyboard => ExpansionPortType::KedaKeyboard,
            ExpansionPortType::KedaKeyboard => ExpansionPortType::KingwonKeyboard,
            ExpansionPortType::KingwonKeyboard => ExpansionPortType::ZeChengKeyboard,
            ExpansionPortType::ZeChengKeyboard => ExpansionPortType::BarcodeBattler,
            ExpansionPortType::BarcodeBattler => ExpansionPortType::HoriTrack,
            ExpansionPortType::HoriTrack => ExpansionPortType::BandaiHyperShot,
            ExpansionPortType::BandaiHyperShot => ExpansionPortType::TurboFile,
            ExpansionPortType::TurboFile => ExpansionPortType::BattleBox,
            ExpansionPortType::BattleBox => ExpansionPortType::TopRider,
            ExpansionPortType::TopRider => ExpansionPortType::FamiNetSys,
            ExpansionPortType::FamiNetSys => ExpansionPortType::CityPatrolman,
            ExpansionPortType::CityPatrolman => ExpansionPortType::Moguraa,
            ExpansionPortType::Moguraa => ExpansionPortType::SharpC1Cassette,
            ExpansionPortType::SharpC1Cassette => ExpansionPortType::GoldenNuggetCasino,
            ExpansionPortType::GoldenNuggetCasino => ExpansionPortType::ABLPinball,
            ExpansionPortType::ABLPinball => ExpansionPortType::TVPump,
            ExpansionPortType::TVPump => ExpansionPortType::TrifaceMahjong,
            ExpansionPortType::TrifaceMahjong => ExpansionPortType::MahjongGekitou,
            ExpansionPortType::MahjongGekitou => ExpansionPortType::TwoPlayerAdapter,
            ExpansionPortType::TwoPlayerAdapter => ExpansionPortType::FourPlayerAdapter,
            ExpansionPortType::FourPlayerAdapter => ExpansionPortType::HoriFourPlayerAdapter,
            ExpansionPortType::HoriFourPlayerAdapter => ExpansionPortType::None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            ExpansionPortType::None => "None",
            ExpansionPortType::ArkanoidPaddle => "Paddle",
            ExpansionPortType::FamicomZapper => "Zapper",
            ExpansionPortType::OekaKidsTablet => "Oeka Kids Tablet",
            ExpansionPortType::FamilyTrainerA => "Family Trainer A",
            ExpansionPortType::FamilyTrainerB => "Family Trainer B",
            ExpansionPortType::KonamiHyperShot => "Hyper Shot",
            ExpansionPortType::FamilyBasicKeyboard => "Family Basic",
            ExpansionPortType::PartyTap => "Party Tap",
            ExpansionPortType::PachinkoController => "Pachinko",
            ExpansionPortType::ExcitingBoxing => "Punching Bag",
            ExpansionPortType::JissenMahjong => "Jissen Mahjong",
            ExpansionPortType::QuizKing => "Quiz King Buzzers",
            ExpansionPortType::SuborKeyboard => "Subor Keyboard",
            ExpansionPortType::Pec586Keyboard => "Dongda PEC-586",
            ExpansionPortType::Bit79Keyboard => "Bit 79 Keyboard",
            ExpansionPortType::KedaKeyboard => "Keda Keyboard",
            ExpansionPortType::KingwonKeyboard => "Kingwon Keyboard",
            ExpansionPortType::ZeChengKeyboard => "Ze Cheng Keyboard",
            ExpansionPortType::BarcodeBattler => "Barcode Battler",
            ExpansionPortType::HoriTrack => "Hori Track",
            ExpansionPortType::BandaiHyperShot => "Bandai Hyper Shot",
            ExpansionPortType::TurboFile => "Turbo File",
            ExpansionPortType::BattleBox => "Battle Box",
            ExpansionPortType::TopRider => "Top Rider",
            ExpansionPortType::FamiNetSys => "Famicom Network",
            ExpansionPortType::CityPatrolman => "City Patrolman",
            ExpansionPortType::Moguraa => "Pokkun Moguraa",
            ExpansionPortType::SharpC1Cassette => "Sharp C1 Cassette",
            ExpansionPortType::GoldenNuggetCasino => "Majesco Casino",
            ExpansionPortType::ABLPinball => "ABL Pinball",
            ExpansionPortType::TVPump => "TV Pump",
            ExpansionPortType::TrifaceMahjong => "Triface Mahjong",
            ExpansionPortType::MahjongGekitou => "Mahjong Gekitou",
            ExpansionPortType::TwoPlayerAdapter => "2-Player Adapter",
            ExpansionPortType::FourPlayerAdapter => "4-Player Adapter",
            ExpansionPortType::HoriFourPlayerAdapter => "Hori 4-Player",
        }
    }
    pub fn is_adapter(self) -> bool {
        matches!(self, ExpansionPortType::TwoPlayerAdapter | ExpansionPortType::FourPlayerAdapter | ExpansionPortType::HoriFourPlayerAdapter)
    }
}

pub fn save_expansion_port_type(et: ExpansionPortType) -> (ExpansionType, ExpansionAdapterType) {
    let (dev, adap) = match et {
        ExpansionPortType::None => (ExpansionType::None, ExpansionAdapterType::None),
        ExpansionPortType::ArkanoidPaddle => (ExpansionType::ArkanoidPaddle, ExpansionAdapterType::None),
        ExpansionPortType::FamicomZapper => (ExpansionType::FamicomZapper, ExpansionAdapterType::None),
        ExpansionPortType::OekaKidsTablet => (ExpansionType::OekaKidsTablet, ExpansionAdapterType::None),
        ExpansionPortType::FamilyTrainerA => (ExpansionType::FamilyTrainerA, ExpansionAdapterType::None),
        ExpansionPortType::FamilyTrainerB => (ExpansionType::FamilyTrainerB, ExpansionAdapterType::None),
        ExpansionPortType::KonamiHyperShot => (ExpansionType::KonamiHyperShot, ExpansionAdapterType::None),
        ExpansionPortType::FamilyBasicKeyboard => (ExpansionType::FamilyBasicKeyboard, ExpansionAdapterType::None),
        ExpansionPortType::PartyTap => (ExpansionType::PartyTap, ExpansionAdapterType::None),
        ExpansionPortType::PachinkoController => (ExpansionType::PachinkoController, ExpansionAdapterType::None),
        ExpansionPortType::ExcitingBoxing => (ExpansionType::ExcitingBoxing, ExpansionAdapterType::None),
        ExpansionPortType::JissenMahjong => (ExpansionType::JissenMahjong, ExpansionAdapterType::None),
        ExpansionPortType::QuizKing => (ExpansionType::QuizKing, ExpansionAdapterType::None),
        ExpansionPortType::SuborKeyboard => (ExpansionType::SuborKeyboard, ExpansionAdapterType::None),
        ExpansionPortType::Pec586Keyboard => (ExpansionType::Pec586Keyboard, ExpansionAdapterType::None),
        ExpansionPortType::Bit79Keyboard => (ExpansionType::Bit79Keyboard, ExpansionAdapterType::None),
        ExpansionPortType::KedaKeyboard => (ExpansionType::KedaKeyboard, ExpansionAdapterType::None),
        ExpansionPortType::KingwonKeyboard => (ExpansionType::KingwonKeyboard, ExpansionAdapterType::None),
        ExpansionPortType::ZeChengKeyboard => (ExpansionType::ZeChengKeyboard, ExpansionAdapterType::None),
        ExpansionPortType::BarcodeBattler => (ExpansionType::BarcodeBattler, ExpansionAdapterType::None),
        ExpansionPortType::HoriTrack => (ExpansionType::HoriTrack, ExpansionAdapterType::None),
        ExpansionPortType::BandaiHyperShot => (ExpansionType::BandaiHyperShot, ExpansionAdapterType::None),
        ExpansionPortType::TurboFile => (ExpansionType::TurboFile, ExpansionAdapterType::None),
        ExpansionPortType::BattleBox => (ExpansionType::BattleBox, ExpansionAdapterType::None),
        ExpansionPortType::TopRider => (ExpansionType::TopRider, ExpansionAdapterType::None),
        ExpansionPortType::FamiNetSys => (ExpansionType::FamiNetSys, ExpansionAdapterType::None),
        ExpansionPortType::CityPatrolman => (ExpansionType::CityPatrolman, ExpansionAdapterType::None),
        ExpansionPortType::Moguraa => (ExpansionType::Moguraa, ExpansionAdapterType::None),
        ExpansionPortType::SharpC1Cassette => (ExpansionType::SharpC1Cassette, ExpansionAdapterType::None),
        ExpansionPortType::GoldenNuggetCasino => (ExpansionType::GoldenNuggetCasino, ExpansionAdapterType::None),
        ExpansionPortType::ABLPinball => (ExpansionType::ABLPinball, ExpansionAdapterType::None),
        ExpansionPortType::TVPump => (ExpansionType::TVPump, ExpansionAdapterType::None),
        ExpansionPortType::TrifaceMahjong => (ExpansionType::TrifaceMahjong, ExpansionAdapterType::None),
        ExpansionPortType::MahjongGekitou => (ExpansionType::MahjongGekitou, ExpansionAdapterType::None),
        ExpansionPortType::TwoPlayerAdapter => (ExpansionType::None, ExpansionAdapterType::TwoPlayer),
        ExpansionPortType::FourPlayerAdapter => (ExpansionType::None, ExpansionAdapterType::FourPlayer),
        ExpansionPortType::HoriFourPlayerAdapter => (ExpansionType::None, ExpansionAdapterType::HoriFourPlayer),
    };
    save_expansion_type(dev);
    save_expansion_adapter_type(adap);
    (dev, adap)
}

pub fn load_expansion_adapter_bindings(port: usize) -> [String; GAMEPAD_BUTTON_COUNT] {
    let prefix = match port {
        0 => "expansion1",
        1 => "expansion2",
        2 => "expansion3",
        3 => "expansion4",
        _ => "expansion1",
    };
    load_bindings(prefix)
}

pub fn save_expansion_adapter_binding(port: usize, button: usize, key: &str) {
    let prefix = match port {
        0 => "expansion1",
        1 => "expansion2",
        2 => "expansion3",
        3 => "expansion4",
        _ => "expansion1",
    };
    save_binding(prefix, button, key);
}

pub fn clear_expansion_adapter_bindings(port: usize) {
    let prefix = match port {
        0 => "expansion1",
        1 => "expansion2",
        2 => "expansion3",
        3 => "expansion4",
        _ => "expansion1",
    };
    clear_bindings(prefix);
}

pub fn reset_expansion_adapter_bindings(port: usize) {
    let prefix = match port {
        0 => "expansion1",
        1 => "expansion2",
        2 => "expansion3",
        3 => "expansion4",
        _ => "expansion1",
    };
    reset_bindings(prefix);
}

pub fn load_allow_opposing_dpad() -> bool {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("allow_opposing_dpad=") {
                let v = value.trim().to_lowercase();
                return v == "yes" || v == "1" || v == "true" || v == "on";
            }
        }
    }
    true
}

pub fn save_allow_opposing_dpad(enabled: bool) {
    upsert_config("allow_opposing_dpad", if enabled { "on" } else { "off" });
}

pub fn load_auto_detect_game_controller() -> bool {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("auto_detect_game_controller=") {
                let v = value.trim().to_lowercase();
                return v == "yes" || v == "1" || v == "true" || v == "on";
            }
        }
    }
    true
}

pub fn save_auto_detect_game_controller(enabled: bool) {
    upsert_config("auto_detect_game_controller", if enabled { "on" } else { "off" });
}

pub const GAMEPAD_BUTTONS: &[&str] = &["A","B","TurboA","TurboB","Select","Start","Up","Down","Left","Right"];
pub const GAMEPAD_BUTTON_COUNT: usize = 10;

pub fn load_bindings(prefix: &str) -> [String; GAMEPAD_BUTTON_COUNT] {
    let defaults: [&str; GAMEPAD_BUTTON_COUNT] = match prefix {
        "controller1" => ["X","C","W","V","Space","Return","Up","Down","Left","Right"],
        "controller3" => ["Numpad1","Numpad2","Numpad3","Numpad4","Numpad5","Numpad6","Numpad8","Numpad2","Numpad4","Numpad6"],
        "controller4" => ["Numpad7","Numpad8","Numpad9","Numpad0","NumpadAdd","NumpadEnter","I","K","J","L"],
        _             => ["Y","U","T","G","F","H","I","K","J","L"],
    };
    let mut b = [(); GAMEPAD_BUTTON_COUNT].map(|_| String::new());
    for (i, d) in defaults.iter().enumerate() { b[i] = d.to_string(); }
    if let Ok(content) = std::fs::read_to_string(&config_path()) {
        for line in content.lines() {
            let t = line.trim();
            for (i, btn) in GAMEPAD_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}=", prefix, btn);
                if let Some(v) = t.strip_prefix(&key) { b[i] = v.trim().to_string(); }
            }
        }
    }
    b
}

pub fn save_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, GAMEPAD_BUTTONS[button]), key);
}

pub fn clear_bindings(prefix: &str) {
    for btn in GAMEPAD_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_bindings(prefix: &str) {
    let defaults: [&str; GAMEPAD_BUTTON_COUNT] = match prefix {
        "controller1" => ["X","C","W","V","Space","Return","Up","Down","Left","Right"],
        "controller3" => ["Numpad1","Numpad2","Numpad3","Numpad4","Numpad5","Numpad6","Numpad8","Numpad2","Numpad4","Numpad6"],
        "controller4" => ["Numpad7","Numpad8","Numpad9","Numpad0","NumpadAdd","NumpadEnter","I","K","J","L"],
        _             => ["Y","U","T","G","F","H","I","K","J","L"],
    };
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, GAMEPAD_BUTTONS[i]), val);
    }
}

pub fn load_zapper_trigger() -> String {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("controller2_zapper_trigger=") {
                return value.trim().to_string();
            }
        }
    }
    "MouseLeft".to_string()
}

pub fn save_zapper_trigger(key: &str) {
    upsert_config("controller2_zapper_trigger", key);
}

pub fn load_expansion_zapper_trigger() -> String {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("expansion_zapper_trigger=") {
                return value.trim().to_string();
            }
        }
    }
    "MouseLeft".to_string()
}

pub fn save_expansion_zapper_trigger(key: &str) {
    upsert_config("expansion_zapper_trigger", key);
}

pub fn load_expansion_oeka_click() -> String {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("expansion_oeka_click=") {
                return value.trim().to_string();
            }
        }
    }
    "MouseLeft".to_string()
}

pub fn save_expansion_oeka_click(key: &str) {
    upsert_config("expansion_oeka_click", key);
}

pub fn load_famicom_mic() -> String {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("famicom_mic=") {
                return value.trim().to_string();
            }
            if let Some(value) = trimmed.strip_prefix("controller2_famicom_mic=") {
                return value.trim().to_string();
            }
        }
    }
    "M".to_string()
}

pub fn save_famicom_mic(key: &str) {
    upsert_config("famicom_mic", key);
}

pub fn load_paddle_button(prefix: &str) -> String {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix(&format!("{}_paddle_button=", prefix)) {
                return value.trim().to_string();
            }
        }
    }
    "MouseLeft".to_string()
}

pub fn save_paddle_button(prefix: &str, key: &str) {
    upsert_config(&format!("{}_paddle_button", prefix), key);
}

pub const POWERPAD_BUTTON_COUNT: usize = 12;
pub const POWERPAD_BUTTONS: &[&str] = &["P0","P1","P2","P3","P4","P5","P6","P7","P8","P9","P10","P11"];
pub const POWERPAD_LABELS: &[&str] = &["1","2","3","4","5","6","7","8","9","10","11","12"];

pub fn save_powerpad_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, POWERPAD_BUTTONS[button]), key);
}

pub fn clear_powerpad_bindings(prefix: &str) {
    for btn in POWERPAD_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_powerpad_bindings(prefix: &str) {
    let defaults: [&str; POWERPAD_BUTTON_COUNT] = ["1","2","3","4","5","6","7","8","9","0","Minus","Equals"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, POWERPAD_BUTTONS[i]), val);
    }
}

pub fn load_powerpad_bindings(prefix: &str) -> [String; POWERPAD_BUTTON_COUNT] {
    let mut b = [(); POWERPAD_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in POWERPAD_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; POWERPAD_BUTTON_COUNT] = ["1","2","3","4","5","6","7","8","9","0","Minus","Equals"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

impl ExpansionType {
    pub fn is_family_trainer(self) -> bool {
        matches!(self, ExpansionType::FamilyTrainerA | ExpansionType::FamilyTrainerB)
    }
    pub fn is_hyper_shot(self) -> bool {
        matches!(self, ExpansionType::KonamiHyperShot)
    }
    pub fn is_family_basic(self) -> bool {
        matches!(self, ExpansionType::FamilyBasicKeyboard)
    }
    pub fn is_party_tap(self) -> bool {
        matches!(self, ExpansionType::PartyTap)
    }
    pub fn is_pachinko(self) -> bool {
        matches!(self, ExpansionType::PachinkoController)
    }
    pub fn is_exciting_boxing(self) -> bool {
        matches!(self, ExpansionType::ExcitingBoxing)
    }
    pub fn is_jissen_mahjong(self) -> bool {
        matches!(self, ExpansionType::JissenMahjong)
    }
    pub fn is_quiz_king(self) -> bool {
        matches!(self, ExpansionType::QuizKing)
    }
    pub fn is_subor_keyboard(self) -> bool {
        matches!(self, ExpansionType::SuborKeyboard)
    }
    pub fn is_pec586_keyboard(self) -> bool {
        matches!(self, ExpansionType::Pec586Keyboard)
    }
    pub fn is_bit79_keyboard(self) -> bool {
        matches!(self, ExpansionType::Bit79Keyboard)
    }
    pub fn is_keda_keyboard(self) -> bool {
        matches!(self, ExpansionType::KedaKeyboard)
    }
    pub fn is_kingwon_keyboard(self) -> bool {
        matches!(self, ExpansionType::KingwonKeyboard)
    }
    pub fn is_zecheng_keyboard(self) -> bool {
        matches!(self, ExpansionType::ZeChengKeyboard)
    }
    pub fn is_barcode_battler(self) -> bool {
        matches!(self, ExpansionType::BarcodeBattler)
    }
    pub fn is_bandai_hyper_shot(self) -> bool {
        matches!(self, ExpansionType::BandaiHyperShot)
    }
    pub fn is_turbo_file(self) -> bool {
        matches!(self, ExpansionType::TurboFile)
    }
    pub fn is_battle_box(self) -> bool {
        matches!(self, ExpansionType::BattleBox)
    }
    pub fn is_top_rider(self) -> bool {
        matches!(self, ExpansionType::TopRider)
    }
    pub fn is_fami_net_sys(self) -> bool {
        matches!(self, ExpansionType::FamiNetSys)
    }
    pub fn is_city_patrolman(self) -> bool {
        matches!(self, ExpansionType::CityPatrolman)
    }
    pub fn is_moguraa(self) -> bool {
        matches!(self, ExpansionType::Moguraa)
    }
    pub fn is_sharp_c1_cassette(self) -> bool {
        matches!(self, ExpansionType::SharpC1Cassette)
    }
    pub fn is_golden_nugget_casino(self) -> bool {
        matches!(self, ExpansionType::GoldenNuggetCasino)
    }
    pub fn is_abl_pinball(self) -> bool {
        matches!(self, ExpansionType::ABLPinball)
    }
    pub fn is_tv_pump(self) -> bool {
        matches!(self, ExpansionType::TVPump)
    }
    pub fn is_triface_mahjong(self) -> bool {
        matches!(self, ExpansionType::TrifaceMahjong)
    }
    pub fn is_mahjong_gekitou(self) -> bool {
        matches!(self, ExpansionType::MahjongGekitou)
    }
}

pub const HYPER_SHOT_BUTTON_COUNT: usize = 4;
pub const HYPER_SHOT_BUTTONS: &[&str] = &["P1R", "P1J", "P2R", "P2J"];
pub const HYPER_SHOT_LABELS: &[&str] = &["P1 Run", "P1 Jump", "P2 Run", "P2 Jump"];

pub const BANDAI_HYPER_SHOT_BUTTON_COUNT: usize = 9;
pub const BANDAI_HYPER_SHOT_BUTTONS: &[&str] = &["A", "B", "Sel", "Sta", "U", "D", "L", "R", "Fire"];
pub const BANDAI_HYPER_SHOT_LABELS: &[&str] = &["A", "B", "Select", "Start", "Up", "Down", "Left", "Right", "Fire"];

pub fn save_hyper_shot_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, HYPER_SHOT_BUTTONS[button]), key);
}

pub fn clear_hyper_shot_bindings(prefix: &str) {
    for btn in HYPER_SHOT_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_hyper_shot_bindings(prefix: &str) {
    let defaults: [&str; HYPER_SHOT_BUTTON_COUNT] = ["Z", "X", "N", "M"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, HYPER_SHOT_BUTTONS[i]), val);
    }
}

pub fn load_hyper_shot_bindings(prefix: &str) -> [String; HYPER_SHOT_BUTTON_COUNT] {
    let mut b = [(); HYPER_SHOT_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in HYPER_SHOT_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; HYPER_SHOT_BUTTON_COUNT] = ["Z", "X", "N", "M"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub fn save_bandai_hyper_shot_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, BANDAI_HYPER_SHOT_BUTTONS[button]), key);
}

pub fn clear_bandai_hyper_shot_bindings(prefix: &str) {
    for btn in BANDAI_HYPER_SHOT_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_bandai_hyper_shot_bindings(prefix: &str) {
    let defaults: [&str; BANDAI_HYPER_SHOT_BUTTON_COUNT] = ["Left", "Down", "Right", "Up", "J", "K", "Space", "Enter", "F"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, BANDAI_HYPER_SHOT_BUTTONS[i]), val);
    }
}

pub fn load_bandai_hyper_shot_bindings(prefix: &str) -> [String; BANDAI_HYPER_SHOT_BUTTON_COUNT] {
    let mut b = [(); BANDAI_HYPER_SHOT_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in BANDAI_HYPER_SHOT_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; BANDAI_HYPER_SHOT_BUTTON_COUNT] = ["Left", "Down", "Right", "Up", "J", "K", "Space", "Enter", "F"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub const FAMILY_BASIC_BUTTON_COUNT: usize = 72;
pub const FAMILY_BASIC_BUTTONS: &[&str] = &[
    "A","B","C","D","E","F","G","H","I","J","K","L","M","N","O","P","Q","R","S","T","U","V","W","X","Y","Z",
    "N0","N1","N2","N3","N4","N5","N6","N7","N8","N9",
    "Return","Space","Del","Ins","Esc","Ctrl","RSHIFT","LSHIFT","RBracket","LBracket","Up","Down","Left","Right",
    "Dot","Comma","Colon","SemiColon","Under","Slash","Minus","Caret","F1","F2","F3","F4","F5","F6","F7","F8","Yen","Stop","At","Grph","ClrHome","Kana",
];
pub const FAMILY_BASIC_LABELS: &[&str] = &[
    "A","B","C","D","E","F","G","H","I","J","K","L","M","N","O","P","Q","R","S","T","U","V","W","X","Y","Z",
    "0","1","2","3","4","5","6","7","8","9",
    "Return","Space","Del","Ins","Esc","Ctrl","R Shift","L Shift","]","[","Up","Down","Left","Right",
    ".",",",":",";","_","/","-","^","F1","F2","F3","F4","F5","F6","F7","F8","Yen","Stop","@","Graph","Home","Kana",
];

pub fn save_family_basic_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, FAMILY_BASIC_BUTTONS[button]), key);
}

pub fn clear_family_basic_bindings(prefix: &str) {
    for btn in FAMILY_BASIC_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_family_basic_bindings(prefix: &str) {
    let defaults: [&str; FAMILY_BASIC_BUTTON_COUNT] = [
        "A","B","C","D","E","F","G","H","I","J","K","L","M","N","O","P","Q","R","S","T","U","V","W","X","Y","Z",
        "0","1","2","3","4","5","6","7","8","9",
        "Enter","Space","Delete","Insert","Escape","LeftControl","RightShift","LeftShift","RightBracket","LeftBracket","Up","Down","Left","Right",
        ".",",",":",";","_","/","-","^","F1","F2","F3","F4","F5","F6","F7","F8","Backslash",".","At","LeftAlt","Home","Kana",
    ];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, FAMILY_BASIC_BUTTONS[i]), val);
    }
}

pub fn load_family_basic_bindings(prefix: &str) -> [String; FAMILY_BASIC_BUTTON_COUNT] {
    let mut b = [(); FAMILY_BASIC_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in FAMILY_BASIC_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; FAMILY_BASIC_BUTTON_COUNT] = [
        "A","B","C","D","E","F","G","H","I","J","K","L","M","N","O","P","Q","R","S","T","U","V","W","X","Y","Z",
        "0","1","2","3","4","5","6","7","8","9",
        "Enter","Space","Delete","Insert","Escape","LeftControl","RightShift","LeftShift","RightBracket","LeftBracket","Up","Down","Left","Right",
        ".",",",":",";","_","/","-","^","F1","F2","F3","F4","F5","F6","F7","F8","Backslash",".","At","LeftAlt","Home","Kana",
    ];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub const PARTY_TAP_BUTTON_COUNT: usize = 6;
pub const PARTY_TAP_BUTTONS: &[&str] = &["B1","B2","B3","B4","B5","B6"];
pub const PARTY_TAP_LABELS: &[&str] = &["1","2","3","4","5","6"];

pub fn save_party_tap_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, PARTY_TAP_BUTTONS[button]), key);
}

pub fn clear_party_tap_bindings(prefix: &str) {
    for btn in PARTY_TAP_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_party_tap_bindings(prefix: &str) {
    let defaults: [&str; PARTY_TAP_BUTTON_COUNT] = ["1","2","3","4","5","6"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, PARTY_TAP_BUTTONS[i]), val);
    }
}

pub fn load_party_tap_bindings(prefix: &str) -> [String; PARTY_TAP_BUTTON_COUNT] {
    let mut b = [(); PARTY_TAP_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in PARTY_TAP_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; PARTY_TAP_BUTTON_COUNT] = ["1","2","3","4","5","6"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub const PACHINKO_BUTTON_COUNT: usize = 2;
pub const PACHINKO_BUTTONS: &[&str] = &["Press","Release"];
pub const PACHINKO_LABELS: &[&str] = &["Press","Release"];

pub fn save_pachinko_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, PACHINKO_BUTTONS[button]), key);
}

pub fn clear_pachinko_bindings(prefix: &str) {
    for btn in PACHINKO_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_pachinko_bindings(prefix: &str) {
    let defaults: [&str; PACHINKO_BUTTON_COUNT] = ["C","V"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, PACHINKO_BUTTONS[i]), val);
    }
}

pub fn load_pachinko_bindings(prefix: &str) -> [String; PACHINKO_BUTTON_COUNT] {
    let mut b = [(); PACHINKO_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in PACHINKO_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; PACHINKO_BUTTON_COUNT] = ["C","V"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub const EXCITING_BOXING_BUTTON_COUNT: usize = 8;
pub const EXCITING_BOXING_BUTTONS: &[&str] = &["LeftHook", "MoveRight", "MoveLeft", "RightHook", "LeftJab", "HitBody", "RightJab", "Straight"];
pub const EXCITING_BOXING_LABELS: &[&str] = &["Left Hook", "Move Right", "Move Left", "Right Hook", "Left Jab", "Hit Body", "Right Jab", "Straight"];

pub fn save_exciting_boxing_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, EXCITING_BOXING_BUTTONS[button]), key);
}

pub fn clear_exciting_boxing_bindings(prefix: &str) {
    for btn in EXCITING_BOXING_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_exciting_boxing_bindings(prefix: &str) {
    let defaults: [&str; EXCITING_BOXING_BUTTON_COUNT] = ["S", "D", "A", "F", "Q", "W", "E", "R"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, EXCITING_BOXING_BUTTONS[i]), val);
    }
}

pub fn load_exciting_boxing_bindings(prefix: &str) -> [String; EXCITING_BOXING_BUTTON_COUNT] {
    let mut b = [(); EXCITING_BOXING_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in EXCITING_BOXING_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; EXCITING_BOXING_BUTTON_COUNT] = ["S", "D", "A", "F", "Q", "W", "E", "R"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub const JISSEN_MAHJONG_BUTTON_COUNT: usize = 21;
pub const JISSEN_MAHJONG_BUTTONS: &[&str] = &["A","B","C","D","E","F","G","H","I","J","K","L","M","N","Select","Start","Kan","Pon","Chii","Riichi","Ron"];
pub const JISSEN_MAHJONG_LABELS: &[&str] = &["A","B","C","D","E","F","G","H","I","J","K","L","M","N","Select","Start","Kan","Pon","Chii","Riichi","Ron"];

pub fn save_jissen_mahjong_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, JISSEN_MAHJONG_BUTTONS[button]), key);
}

pub fn clear_jissen_mahjong_bindings(prefix: &str) {
    for btn in JISSEN_MAHJONG_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_jissen_mahjong_bindings(prefix: &str) {
    let defaults: [&str; JISSEN_MAHJONG_BUTTON_COUNT] = ["1","2","3","4","5","6","7","8","9","0","Q","W","E","R","T","Y","U","I","O","P","A"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, JISSEN_MAHJONG_BUTTONS[i]), val);
    }
}

pub fn load_jissen_mahjong_bindings(prefix: &str) -> [String; JISSEN_MAHJONG_BUTTON_COUNT] {
    let mut b = [(); JISSEN_MAHJONG_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in JISSEN_MAHJONG_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; JISSEN_MAHJONG_BUTTON_COUNT] = ["1","2","3","4","5","6","7","8","9","0","Q","W","E","R","T","Y","U","I","O","P","A"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub const CITY_PATROLMAN_BUTTON_COUNT: usize = 2;
pub const CITY_PATROLMAN_BUTTONS: &[&str] = &["Trigger", "Reload"];
pub const CITY_PATROLMAN_LABELS: &[&str] = &["Trigger", "Reload"];

pub fn save_city_patrolman_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, CITY_PATROLMAN_BUTTONS[button]), key);
}

pub fn clear_city_patrolman_bindings(prefix: &str) {
    for btn in CITY_PATROLMAN_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_city_patrolman_bindings(prefix: &str) {
    let defaults: [&str; CITY_PATROLMAN_BUTTON_COUNT] = ["MouseLeft", "R"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, CITY_PATROLMAN_BUTTONS[i]), val);
    }
}

pub fn load_city_patrolman_bindings(prefix: &str) -> [String; CITY_PATROLMAN_BUTTON_COUNT] {
    let mut b = [(); CITY_PATROLMAN_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in CITY_PATROLMAN_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; CITY_PATROLMAN_BUTTON_COUNT] = ["MouseLeft", "R"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub const MOGURAA_BUTTON_COUNT: usize = 12;
pub const MOGURAA_BUTTONS: &[&str] = &["B1","B2","B3","B4","B5","B6","B7","B8","B9","B10","B11","B12"];
pub const MOGURAA_LABELS: &[&str] = &["1","2","3","4","5","6","7","8","9","10","11","12"];

pub fn save_moguraa_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, MOGURAA_BUTTONS[button]), key);
}

pub fn clear_moguraa_bindings(prefix: &str) {
    for btn in MOGURAA_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_moguraa_bindings(prefix: &str) {
    let defaults: [&str; MOGURAA_BUTTON_COUNT] = ["1","2","3","4","5","6","7","8","9","0","Q","W"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, MOGURAA_BUTTONS[i]), val);
    }
}

pub fn load_moguraa_bindings(prefix: &str) -> [String; MOGURAA_BUTTON_COUNT] {
    let mut b = [(); MOGURAA_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in MOGURAA_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; MOGURAA_BUTTON_COUNT] = ["1","2","3","4","5","6","7","8","9","0","Q","W"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub const GOLDEN_NUGGET_CASINO_BUTTON_COUNT: usize = 11;
pub const GOLDEN_NUGGET_CASINO_BUTTONS: &[&str] = &["Start","A","B","P4","P5","P6","P7","Up","Down","Left","Right"];
pub const GOLDEN_NUGGET_CASINO_LABELS: &[&str] = &["Start","A","B","P4","P5","P6","P7","Up","Down","Left","Right"];

pub fn save_golden_nugget_casino_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, GOLDEN_NUGGET_CASINO_BUTTONS[button]), key);
}

pub fn clear_golden_nugget_casino_bindings(prefix: &str) {
    for btn in GOLDEN_NUGGET_CASINO_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_golden_nugget_casino_bindings(prefix: &str) {
    let defaults: [&str; GOLDEN_NUGGET_CASINO_BUTTON_COUNT] = ["Space","X","Z","S","D","F","G","Up","Down","Left","Right"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, GOLDEN_NUGGET_CASINO_BUTTONS[i]), val);
    }
}

pub fn load_golden_nugget_casino_bindings(prefix: &str) -> [String; GOLDEN_NUGGET_CASINO_BUTTON_COUNT] {
    let mut b = [(); GOLDEN_NUGGET_CASINO_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in GOLDEN_NUGGET_CASINO_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; GOLDEN_NUGGET_CASINO_BUTTON_COUNT] = ["Space","X","Z","S","D","F","G","Up","Down","Left","Right"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub const ABL_PINBALL_BUTTON_COUNT: usize = 6;
pub const ABL_PINBALL_BUTTONS: &[&str] = &["Select","Start","Left","Right","Nudge","Plunger"];
pub const ABL_PINBALL_LABELS: &[&str] = &["Select","Start","Left","Right","Nudge","Plunger (Wheel)"];

pub fn save_abl_pinball_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, ABL_PINBALL_BUTTONS[button]), key);
}

pub fn clear_abl_pinball_bindings(prefix: &str) {
    for btn in ABL_PINBALL_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_abl_pinball_bindings(prefix: &str) {
    let defaults: [&str; ABL_PINBALL_BUTTON_COUNT] = ["Space","Return","Left","Right","N",""];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, ABL_PINBALL_BUTTONS[i]), val);
    }
}

pub fn load_abl_pinball_bindings(prefix: &str) -> [String; ABL_PINBALL_BUTTON_COUNT] {
    let mut b = [(); ABL_PINBALL_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in ABL_PINBALL_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; ABL_PINBALL_BUTTON_COUNT] = ["Space","Return","Left","Right","N",""];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub const TV_PUMP_BUTTON_COUNT: usize = 6;
pub const TV_PUMP_BUTTONS: &[&str] = &["B1","B2","B3","B4","B5","B6"];
pub const TV_PUMP_LABELS: &[&str] = &["1","2","3","4","5","6"];

pub fn save_tv_pump_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, TV_PUMP_BUTTONS[button]), key);
}

pub fn clear_tv_pump_bindings(prefix: &str) {
    for btn in TV_PUMP_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_tv_pump_bindings(prefix: &str) {
    let defaults: [&str; TV_PUMP_BUTTON_COUNT] = ["1","2","3","4","5","6"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, TV_PUMP_BUTTONS[i]), val);
    }
}

pub fn load_tv_pump_bindings(prefix: &str) -> [String; TV_PUMP_BUTTON_COUNT] {
    let mut b = [(); TV_PUMP_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in TV_PUMP_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; TV_PUMP_BUTTON_COUNT] = ["1","2","3","4","5","6"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub const MAHJONG_2X22_BUTTON_COUNT: usize = 22;
pub const MAHJONG_2X22_BUTTONS: &[&str] = &["A","B","C","D","E","F","G","H","I","J","K","L","M","N","Select","Start","Kan","Pon","Chii","Riichi","Ron","Test"];
pub const MAHJONG_2X22_LABELS: &[&str] = &["A","B","C","D","E","F","G","H","I","J","K","L","M","N","Select","Start","Kan","Pon","Chii","Riichi","Ron","Test"];

pub fn save_triface_mahjong_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, MAHJONG_2X22_BUTTONS[button]), key);
}

pub fn clear_triface_mahjong_bindings(prefix: &str) {
    for btn in MAHJONG_2X22_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_triface_mahjong_bindings(prefix: &str) {
    let defaults: [&str; MAHJONG_2X22_BUTTON_COUNT] = ["1","2","3","4","5","6","7","8","9","0","Q","W","E","R","T","Y","U","I","O","P","A","S"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, MAHJONG_2X22_BUTTONS[i]), val);
    }
}

pub fn load_triface_mahjong_bindings(prefix: &str) -> [String; MAHJONG_2X22_BUTTON_COUNT] {
    let mut b = [(); MAHJONG_2X22_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in MAHJONG_2X22_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; MAHJONG_2X22_BUTTON_COUNT] = ["1","2","3","4","5","6","7","8","9","0","Q","W","E","R","T","Y","U","I","O","P","A","S"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub fn save_mahjong_gekitou_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, MAHJONG_2X22_BUTTONS[button]), key);
}

pub fn clear_mahjong_gekitou_bindings(prefix: &str) {
    for btn in MAHJONG_2X22_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_mahjong_gekitou_bindings(prefix: &str) {
    let defaults: [&str; MAHJONG_2X22_BUTTON_COUNT] = ["1","2","3","4","5","6","7","8","9","0","Q","W","E","R","T","Y","U","I","O","P","A","S"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, MAHJONG_2X22_BUTTONS[i]), val);
    }
}

pub fn load_mahjong_gekitou_bindings(prefix: &str) -> [String; MAHJONG_2X22_BUTTON_COUNT] {
    let mut b = [(); MAHJONG_2X22_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in MAHJONG_2X22_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; MAHJONG_2X22_BUTTON_COUNT] = ["1","2","3","4","5","6","7","8","9","0","Q","W","E","R","T","Y","U","I","O","P","A","S"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub const SUBOR_KEYBOARD_MATRIX: [u8; 104] = [
    30, 6, 5, 2, 37, 4, 31, 21,
    28, 3, 18, 86, 36, 22, 29, 23,
    88, 82, 91, 95, 43, 90, 89, 87,
    35, 8, 11, 65, 40, 14, 26, 66,
    75, 85, 92, 94, 42, 74, 70, 93,
    16, 76, 25, 83, 84, 0, 27, 78,
    33, 24, 10, 12, 39, 20, 34, 9,
    72, 67, 68, 69, 41, 15, 71, 79,
    19, 7, 13, 81, 38, 17, 32, 1,
    54, 58, 52, 56, 99, 96, 97, 98,
    80, 52, 55, 46, 47, 49, 50, 56,
    63, 60, 61, 57, 45, 53, 62, 64,
    73, 54, 77, 81, 44, 51, 59, 48,
];

pub const SUBOR_KEYBOARD_BUTTON_COUNT: usize = 99;
pub const SUBOR_KEYBOARD_BUTTONS: &[&str] = &[
    "A","B","C","D","E","F","G","H","I","J","K","L","M","N","O","P","Q","R","S","T","U","V","W","X","Y","Z",
    "0","1","2","3","4","5","6","7","8","9",
    "F1","F2","F3","F4","F5","F6","F7","F8","F9","F10","F11","F12",
    "Np0","Np1","Np2","Np3","Np4","Np5","Np6","Np7","Np8","Np9",
    "NpEnt","NpDot","Np+","Np*","Np/","Np-","NumLk",
    ",",".",";","'","/","\\","=","-","`","[","]",
    "Caps","Pause","Ctrl","Shift","Alt","Space","Bksp","Tab","Esc","Enter","End","Home","Ins","Del","PgUp","PgDn",
    "Up","Down","Left","Right","--","--","--",
];
pub const SUBOR_KEYBOARD_LABELS: &[&str] = SUBOR_KEYBOARD_BUTTONS;

pub const PEC586_KEYBOARD_MATRIX: [u8; 104] = [
    84, 81, 80, 78, 79, 73, 83, 76, 
    41, 42, 40, 39, 43, 37, 36, 38, 
    71, 48, 66,  0, 85, 27, 16, 25,
    99, 51, 54, 18, 57, 28, 22, 23,
    69, 50, 53,  3, 56, 29,  4,  2, 
    77, 49, 52,  5, 55, 30, 17, 21,
    82, 70, 99,  6, 75, 31, 19,  1, 
    35, 66, 11, 10, 14, 34,  8, 65, 
    26, 69, 67,  9, 15, 33, 20, 12,
    72, 72, 68,  7, 74, 32, 24, 13,
    46, 47, 45, 99, 72, 44, 99, 99,
    92, 95, 93, 62, 94, 61, 63, 60,
    88, 49, 87, 90, 89, 86, 91, 64,
];

pub const BIT79_KEYBOARD_MATRIX: [u8; 80] = [
    99, 81, 99, 92, 94, 99, 99, 99,
    65, 66, 99, 99, 99, 69, 99, 79, 
    12, 13, 23, 25, 79,  1, 21,  2, 
    10, 11, 99, 99, 99, 67, 68, 85, 
     9,  7, 18,  0, 78,  6,  5,  3,
    20, 24, 22, 16, 83, 19, 17,  4, 
     8, 14, 99, 99, 99, 15, 74, 75, 
    33, 32, 28, 27, 73, 31, 30, 29, 
    34, 35, 99, 99, 99, 99, 72, 71, 
    82, 70, 99, 99, 99, 93, 99, 95,
];

pub const KEDA_KEYBOARD_MATRIX: [u8; 88] = [
    73, 83, 76, 79, 27, 16,  0, 25,
    28, 22, 18, 23, 29,  4,  3,  2,
    30, 17,  5, 21, 31, 19,  6,  1,
    32, 24,  7, 13, 33, 20,  9, 12,
    34,  8, 10, 65, 35, 14, 11, 66,
    26, 15, 67, 69, 72, 74, 68, 78,
    71, 75, 85, 82, 61, 87, 94, 86,
    99, 92, 99, 93, 84, 99, 95, 79,
    99, 99, 81, 88, 58, 99, 60, 63,
    36, 37, 38, 39, 40, 41, 42, 43,
    99, 99, 47, 46, 99, 61, 99, 99,
];

pub const KINGWON_KEYBOARD_MATRIX: [u8; 104] = [
    25,  0, 16, 27, 79, 76, 83, 73,
     2,  3,  4, 29, 23, 18, 22, 28,
     1,  6, 19, 31, 21,  5, 17, 30,
    12,  9, 20, 33, 13,  7, 24, 32,
    66, 11, 14, 35, 65, 10,  8, 34,
    70, 68, 74, 72, 69, 67, 15, 26,
    86, 94, 87, 99, 82, 85, 75, 71,
    91, 95, 90, 84, 93, 99, 92, 99,
    89, 60, 61, 58, 88, 81, 80, 78,
    43, 42, 41, 40, 39, 38, 37, 36,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
];

pub const ZECHENG_KEYBOARD_BUTTON_COUNT: usize = 4;
pub const ZECHENG_KEYBOARD_BUTTONS: &[&str] = &["Q", "S", "LMB", "RMB"];
pub const ZECHENG_KEYBOARD_LABELS: &[&str] = &["Q", "S", "Left", "Right"];

pub fn load_zecheng_keyboard_bindings(prefix: &str) -> [String; ZECHENG_KEYBOARD_BUTTON_COUNT] {
    let mut b = [(); ZECHENG_KEYBOARD_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            if let Some(eq) = t.find('=') {
                let k = t[..eq].trim();
                let v = t[eq + 1..].trim();
                for (i, btn) in ZECHENG_KEYBOARD_BUTTONS.iter().enumerate() {
                    if k.eq_ignore_ascii_case(&format!("{}_{}", prefix, btn)) {
                        b[i] = v.to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; ZECHENG_KEYBOARD_BUTTON_COUNT] = ["Q", "S", "LButton", "RButton"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub fn save_zecheng_keyboard_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, ZECHENG_KEYBOARD_BUTTONS[button]), key);
}

pub fn clear_zecheng_keyboard_bindings(prefix: &str) {
    for btn in ZECHENG_KEYBOARD_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_zecheng_keyboard_bindings(prefix: &str) {
    let defaults: [&str; ZECHENG_KEYBOARD_BUTTON_COUNT] = ["Q", "S", "LButton", "RButton"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, ZECHENG_KEYBOARD_BUTTONS[i]), val);
    }
}

pub const QUIZ_KING_BUTTON_COUNT: usize = 6;
pub const QUIZ_KING_BUTTONS: &[&str] = &["P1", "P2", "P3", "P4", "P5", "P6"];
pub const QUIZ_KING_LABELS: &[&str] = &["P1", "P2", "P3", "P4", "P5", "P6"];

pub fn save_quiz_king_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, QUIZ_KING_BUTTONS[button]), key);
}

pub fn clear_quiz_king_bindings(prefix: &str) {
    for btn in QUIZ_KING_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_quiz_king_bindings(prefix: &str) {
    let defaults: [&str; QUIZ_KING_BUTTON_COUNT] = ["Q", "W", "E", "R", "T", "Y"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, QUIZ_KING_BUTTONS[i]), val);
    }
}

pub fn load_quiz_king_bindings(prefix: &str) -> [String; QUIZ_KING_BUTTON_COUNT] {
    let mut b = [(); QUIZ_KING_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in QUIZ_KING_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; QUIZ_KING_BUTTON_COUNT] = ["Q", "W", "E", "R", "T", "Y"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub const TOP_RIDER_BUTTON_COUNT: usize = 8;
pub const TOP_RIDER_BUTTONS: &[&str] = &["A", "B", "C", "D", "E", "F", "G", "H"];
pub const TOP_RIDER_LABELS: &[&str] = &["A", "B", "C", "D", "E", "F", "G", "H"];

pub fn save_top_rider_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, TOP_RIDER_BUTTONS[button]), key);
}

pub fn clear_top_rider_bindings(prefix: &str) {
    for btn in TOP_RIDER_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_top_rider_bindings(prefix: &str) {
    let defaults: [&str; TOP_RIDER_BUTTON_COUNT] = ["Q", "W", "E", "R", "T", "Y", "U", "I"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, TOP_RIDER_BUTTONS[i]), val);
    }
}

pub fn load_top_rider_bindings(prefix: &str) -> [String; TOP_RIDER_BUTTON_COUNT] {
    let mut b = [(); TOP_RIDER_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in TOP_RIDER_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; TOP_RIDER_BUTTON_COUNT] = ["Q", "W", "E", "R", "T", "Y", "U", "I"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub const FAMI_NET_SYS_BUTTON_COUNT: usize = 24;
pub const FAMI_NET_SYS_BUTTONS: &[&str] = &[
    "V", "C", "X", "Z", "Up", "Down", "Left", "Right",
    "0", "1", "2", "3", "4", "5", "6", "7",
    "8", "9", "Asterisk", "Plus", "Delete", "Minus", "Esc", "BS",
];
pub const FAMI_NET_SYS_LABELS: &[&str] = &[
    "V", "C", "X", "Z", "Up", "Down", "Left", "Right",
    "0", "1", "2", "3", "4", "5", "6", "7",
    "8", "9", "*", "+", "Del", "-", "Esc", "BS",
];

pub fn save_fami_net_sys_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, FAMI_NET_SYS_BUTTONS[button]), key);
}

pub fn clear_fami_net_sys_bindings(prefix: &str) {
    for btn in FAMI_NET_SYS_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_fami_net_sys_bindings(prefix: &str) {
    let defaults: [&str; FAMI_NET_SYS_BUTTON_COUNT] = [
        "V", "C", "X", "Z", "Up", "Down", "Left", "Right",
        "Key0", "Key1", "Key2", "Key3", "Key4", "Key5", "Key6", "Key7",
        "Key8", "Key9", "Asterisk", "NumpadAdd", "Delete", "Minus", "Escape", "Back",
    ];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, FAMI_NET_SYS_BUTTONS[i]), val);
    }
}

pub fn load_fami_net_sys_bindings(prefix: &str) -> [String; FAMI_NET_SYS_BUTTON_COUNT] {
    let mut b = [(); FAMI_NET_SYS_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in FAMI_NET_SYS_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; FAMI_NET_SYS_BUTTON_COUNT] = [
        "V", "C", "X", "Z", "Up", "Down", "Left", "Right",
        "Key0", "Key1", "Key2", "Key3", "Key4", "Key5", "Key6", "Key7",
        "Key8", "Key9", "Asterisk", "NumpadAdd", "Delete", "Minus", "Escape", "Back",
    ];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub fn save_subor_keyboard_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, SUBOR_KEYBOARD_BUTTONS[button]), key);
}

pub fn clear_subor_keyboard_bindings(prefix: &str) {
    for btn in SUBOR_KEYBOARD_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn reset_subor_keyboard_bindings(prefix: &str) {
    let defaults: [&str; SUBOR_KEYBOARD_BUTTON_COUNT] = [
        "A","B","C","D","E","F","G","H","I","J","K","L","M","N","O","P","Q","R","S","T","U","V","W","X","Y","Z",
        "Key0","Key1","Key2","Key3","Key4","Key5","Key6","Key7","Key8","Key9",
        "F1","F2","F3","F4","F5","F6","F7","F8","F9","F10","F11","F12",
        "Numpad0","Numpad1","Numpad2","Numpad3","Numpad4","Numpad5","Numpad6","Numpad7","Numpad8","Numpad9",
        "NumpadEnter","NumpadDecimal","NumpadAdd","NumpadMultiply","NumpadDivide","NumpadSubtract","Numlock",
        "Comma","Period","Semicolon","Apostrophe","Slash","Backslash","Equals","Minus","Grave","LBracket","RBracket",
        "Capital","Pause","LControl","LShift","LAlt","Space","Back","Tab","Escape","Return","End","Home","Insert","Delete","PageUp","PageDown",
        "Up","Down","Left","Right","","","",
    ];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, SUBOR_KEYBOARD_BUTTONS[i]), val);
    }
}

pub fn load_subor_keyboard_bindings(prefix: &str) -> [String; SUBOR_KEYBOARD_BUTTON_COUNT] {
    let mut b = [(); SUBOR_KEYBOARD_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in SUBOR_KEYBOARD_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; SUBOR_KEYBOARD_BUTTON_COUNT] = [
        "A","B","C","D","E","F","G","H","I","J","K","L","M","N","O","P","Q","R","S","T","U","V","W","X","Y","Z",
        "Key0","Key1","Key2","Key3","Key4","Key5","Key6","Key7","Key8","Key9",
        "F1","F2","F3","F4","F5","F6","F7","F8","F9","F10","F11","F12",
        "Numpad0","Numpad1","Numpad2","Numpad3","Numpad4","Numpad5","Numpad6","Numpad7","Numpad8","Numpad9",
        "NumpadEnter","NumpadDecimal","NumpadAdd","NumpadMultiply","NumpadDivide","NumpadSubtract","Numlock",
        "Comma","Period","Semicolon","Apostrophe","Slash","Backslash","Equals","Minus","Grave","LBracket","RBracket",
        "Capital","Pause","LControl","LShift","LAlt","Space","Back","Tab","Escape","Return","End","Home","Insert","Delete","PageUp","PageDown",
        "Up","Down","Left","Right","","","",
    ];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub const SNES_BUTTON_COUNT: usize = 12;pub const SNES_BUTTONS: &[&str] = &["B","Y","Select","Start","Up","Down","Left","Right","A","X","L","R"];
pub const SNES_LABELS: &[&str] = &["B","Y","Select","Start","Up","Down","Left","Right","A","X","L","R"];

pub fn load_snes_bindings(prefix: &str) -> [String; SNES_BUTTON_COUNT] {
    let mut b = [(); SNES_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in SNES_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; SNES_BUTTON_COUNT] = ["C","X","Space","Return","Up","Down","Left","Right","D","S","Q","W"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub fn save_snes_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, SNES_BUTTONS[button]), key);
}

pub fn reset_snes_bindings(prefix: &str) {
    let defaults: [&str; SNES_BUTTON_COUNT] = ["C","X","Space","Return","Up","Down","Left","Right","D","S","Q","W"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, SNES_BUTTONS[i]), val);
    }
}

pub fn clear_snes_bindings(prefix: &str) {
    for btn in SNES_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub const SNES_MOUSE_BUTTON_COUNT: usize = 2;
pub const SNES_MOUSE_BUTTONS: &[&str] = &["Left","Right"];
pub const SNES_MOUSE_LABELS: &[&str] = &["Left","Right"];

pub fn load_snes_mouse_bindings(prefix: &str) -> [String; SNES_MOUSE_BUTTON_COUNT] {
    let mut b = [(); SNES_MOUSE_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in SNES_MOUSE_BUTTONS.iter().enumerate() {
                let key = format!("{}_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    let defaults: [&str; SNES_MOUSE_BUTTON_COUNT] = ["MouseLeft","MouseRight"];
    for (i, val) in defaults.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub fn save_snes_mouse_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_{}", prefix, SNES_MOUSE_BUTTONS[button]), key);
}

pub fn reset_snes_mouse_bindings(prefix: &str) {
    let defaults: [&str; SNES_MOUSE_BUTTON_COUNT] = ["MouseLeft","MouseRight"];
    for (i, val) in defaults.iter().enumerate() {
        upsert_config(&format!("{}_{}", prefix, SNES_MOUSE_BUTTONS[i]), val);
    }
}

pub fn clear_snes_mouse_bindings(prefix: &str) {
    for btn in SNES_MOUSE_BUTTONS {
        upsert_config(&format!("{}_{}", prefix, btn), "");
    }
}

pub fn load_subor_mouse_bindings(prefix: &str) -> [String; SNES_MOUSE_BUTTON_COUNT] {
    load_snes_mouse_bindings(prefix)
}

pub fn save_subor_mouse_binding(prefix: &str, button: usize, key: &str) {
    save_snes_mouse_binding(prefix, button, key);
}

pub fn reset_subor_mouse_bindings(prefix: &str) {
    reset_snes_mouse_bindings(prefix);
}

pub fn clear_subor_mouse_bindings(prefix: &str) {
    clear_snes_mouse_bindings(prefix);
}

pub const VB_BUTTON_COUNT: usize = 14;
pub const VB_BUTTONS: &[&str] = &["RDown","RLeft","Select","Start","LUp","LDown","LLeft","LRight","RRight","RUp","L","R","B","A"];
pub const VB_DISPLAY_ORDER: &[usize] = &[13, 12, 2, 3, 10, 11, 4, 5, 6, 7, 9, 0, 1, 8];
pub const VB_LABELS: &[&str] = &["A","B","Select","Start","L","R","L-Up","L-Down","L-Left","L-Right","R-Up","R-Down","R-Left","R-Right"];

pub fn vb_serial_from_menu(menu_idx: usize) -> usize {
    VB_DISPLAY_ORDER[menu_idx]
}

pub fn filter_vb_opposing(state: &mut u16) {
    if (*state & (1 << 4)) != 0 && (*state & (1 << 5)) != 0 {
        *state &= !((1 << 4) | (1 << 5));
    }
    if (*state & (1 << 6)) != 0 && (*state & (1 << 7)) != 0 {
        *state &= !((1 << 6) | (1 << 7));
    }
    if (*state & (1 << 9)) != 0 && (*state & (1 << 0)) != 0 {
        *state &= !((1 << 9) | (1 << 0));
    }
    if (*state & (1 << 1)) != 0 && (*state & (1 << 8)) != 0 {
        *state &= !((1 << 1) | (1 << 8));
    }
}

pub fn build_vb_state(raw_state: u16) -> u16 {
    let mut s = raw_state;
    filter_vb_opposing(&mut s);
    s | (1 << 14)
}

const VB_DEFAULT_BINDINGS: [&str; VB_BUTTON_COUNT] = ["K","J","Space","Return","Up","Down","Left","Right","L","I","Q","W","X","Z"];

pub fn load_vb_bindings(prefix: &str) -> [String; VB_BUTTON_COUNT] {
    let mut b = [(); VB_BUTTON_COUNT].map(|_| String::new());
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            for (i, btn) in VB_BUTTONS.iter().enumerate() {
                let key = format!("{}_vb_{}", prefix, btn);
                if let Some(v) = trimmed.strip_prefix(&key) {
                    if let Some(value) = v.strip_prefix('=') {
                        b[i] = value.trim().to_string();
                    }
                }
            }
        }
    }
    for (i, val) in VB_DEFAULT_BINDINGS.iter().enumerate() {
        if b[i].is_empty() {
            b[i] = val.to_string();
        }
    }
    b
}

pub fn save_vb_binding(prefix: &str, button: usize, key: &str) {
    upsert_config(&format!("{}_vb_{}", prefix, VB_BUTTONS[button]), key);
}

pub fn reset_vb_bindings(prefix: &str) {
    for (i, val) in VB_DEFAULT_BINDINGS.iter().enumerate() {
        upsert_config(&format!("{}_vb_{}", prefix, VB_BUTTONS[i]), val);
    }
}

pub fn clear_vb_bindings(prefix: &str) {
    for btn in VB_BUTTONS {
        upsert_config(&format!("{}_vb_{}", prefix, btn), "");
    }
}

pub const HOTKEY_COUNT: usize = 17;

pub const HOTKEY_LABELS: [&str; HOTKEY_COUNT] = [
    "Open ROM",
    "Close ROM",
    "Recent ROMs (1-8)",
    "Quick Save",
    "Quick Load",
    "Save State (1-9)",
    "Load State (1-9)",
    "Exit",
    "Pause/Resume",
    "DIP Switches",
    "Insert Coin 1/2",
    "Service Button",
    "Insert/Eject Disk",
    "Swap Disk",
    "Input Barcode",
    "Reset",
    "Power Cycle",
];

pub const HOTKEY_CONFIG_KEYS: [&str; HOTKEY_COUNT] = [
    "hotkey_open_rom",
    "hotkey_close_rom",
    "hotkey_recent_roms",
    "hotkey_quick_save",
    "hotkey_quick_load",
    "hotkey_save_state",
    "hotkey_load_state",
    "hotkey_exit",
    "hotkey_pause",
    "hotkey_dip_switches",
    "hotkey_insert_coin",
    "hotkey_service_button",
    "hotkey_insert_eject_disk",
    "hotkey_swap_disk",
    "hotkey_input_barcode",
    "hotkey_reset",
    "hotkey_power_cycle",
];

pub const DEFAULT_HOTKEYS: [&str; HOTKEY_COUNT] = [
    "Ctrl + O",
    "Ctrl + C",
    "Ctrl + O + (1-8)",
    "Ctrl + S",
    "Ctrl + L",
    "Ctrl + S + (1-9)",
    "Ctrl + L + (1-9)",
    "Ctrl + Esc",
    "Ctrl + P",
    "Ctrl + D",
    "Ctrl + I + (1-2)",
    "Ctrl + U",
    "Ctrl + E",
    "Ctrl + Y",
    "Ctrl + B",
    "Ctrl + R",
    "Ctrl + Shift + R",
];

pub fn load_hotkeys() -> [String; HOTKEY_COUNT] {
    let mut b = [(); HOTKEY_COUNT].map(|_| String::new());
    for (i, d) in DEFAULT_HOTKEYS.iter().enumerate() {
        b[i] = d.to_string();
    }
    if let Ok(content) = std::fs::read_to_string(&config_path()) {
        for line in content.lines() {
            let t = line.trim();
            for (i, key) in HOTKEY_CONFIG_KEYS.iter().enumerate() {
                let prefix = format!("{}=", key);
                if let Some(v) = t.strip_prefix(&prefix) {
                    let val = v.trim();
                    if !val.is_empty() {
                        b[i] = val.to_string();
                    }
                }
            }
        }
    }
    b
}

pub fn save_hotkey(index: usize, binding: &str) {
    if index < HOTKEY_COUNT {
        upsert_config(HOTKEY_CONFIG_KEYS[index], binding);
    }
}

pub fn reset_hotkeys() {
    for (i, val) in DEFAULT_HOTKEYS.iter().enumerate() {
        upsert_config(HOTKEY_CONFIG_KEYS[i], val);
    }
}
