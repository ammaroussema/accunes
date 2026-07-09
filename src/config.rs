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
    exe_dir().join("saves").join(name).with_extension("sav")
}

pub fn state_file_path(rom_path: &str, slot: usize) -> PathBuf {
    let path = Path::new(rom_path);
    let name = path.file_stem().unwrap_or(path.as_os_str());
    exe_dir().join("savestates").join(format!("{}.state{}", name.to_string_lossy(), slot))
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

pub const CHANNEL_NAMES: &[&str] = &["master", "triangle", "square1", "square2", "noise", "pcm"];

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
    Zapper,
    Paddle,
    PowerPadA,
    PowerPadB,
    SNESPad,
    SNESMouse,
    SuborMouse,
    FourScore,
}

impl ControllerType {
    pub fn next(self, allow_paddle: bool) -> Self {
        match self {
            ControllerType::None => ControllerType::Gamepad,
            ControllerType::Gamepad => ControllerType::Zapper,
            ControllerType::Zapper => if allow_paddle { ControllerType::Paddle } else { ControllerType::PowerPadA },
            ControllerType::Paddle => ControllerType::PowerPadA,
            ControllerType::PowerPadA => ControllerType::PowerPadB,
            ControllerType::PowerPadB => ControllerType::SNESPad,
            ControllerType::SNESPad => ControllerType::SNESMouse,
            ControllerType::SNESMouse => ControllerType::SuborMouse,
            ControllerType::SuborMouse => ControllerType::FourScore,
            ControllerType::FourScore => ControllerType::None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            ControllerType::None => "None",
            ControllerType::Gamepad => "Gamepad",
            ControllerType::Zapper => "Zapper",
            ControllerType::Paddle => "Paddle",
            ControllerType::PowerPadA => "Power Pad A",
            ControllerType::PowerPadB => "Power Pad B",
            ControllerType::SNESPad => "SNES Pad",
            ControllerType::SNESMouse => "SNES Mouse",
            ControllerType::SuborMouse => "Subor Mouse",
            ControllerType::FourScore => "Four Score",
        }
    }
}

pub fn load_controller_type(key: &str) -> ControllerType {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix(&format!("{}=", key)) {
                return match value.trim().to_lowercase().as_str() {
                    "gamepad" => ControllerType::Gamepad,
                    "zapper" => ControllerType::Zapper,
                    "paddle" => ControllerType::Paddle,
                    "powerpada" | "power pad a" => ControllerType::PowerPadA,
                    "powerpadb" | "power pad b" => ControllerType::PowerPadB,
                    "snespad" | "snes pad" => ControllerType::SNESPad,
                    "snesmouse" | "snes mouse" => ControllerType::SNESMouse,
                    "subormouse" | "subor mouse" => ControllerType::SuborMouse,
                    "fourscore" | "four score" => ControllerType::FourScore,
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
        ControllerType::Zapper => "zapper",
        ControllerType::Paddle => "paddle",
        ControllerType::PowerPadA => "powerpada",
        ControllerType::PowerPadB => "powerpadb",
        ControllerType::SNESPad => "snespad",
        ControllerType::SNESMouse => "snesmouse",
        ControllerType::SuborMouse => "subormouse",
        ControllerType::FourScore => "fourscore",
    };
    upsert_config(key, s);
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

pub const SNES_BUTTON_COUNT: usize = 12;
pub const SNES_BUTTONS: &[&str] = &["B","Y","Select","Start","Up","Down","Left","Right","A","X","L","R"];
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
