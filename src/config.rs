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

pub fn turbofile_save_path(rom_path: &str) -> PathBuf {
    let path = Path::new(rom_path);
    let name = path.file_stem().unwrap_or(path.as_os_str());
    exe_dir().join("saves").join(format!("{}.turbofile.sav", name.to_string_lossy()))
}

pub fn battlebox_save_path(rom_path: &str) -> PathBuf {
    let path = Path::new(rom_path);
    let name = path.file_stem().unwrap_or(path.as_os_str());
    exe_dir().join("saves").join(format!("{}.battlebox.sav", name.to_string_lossy()))
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
    FamicomGamepad,
    Zapper,
    Paddle,
    PowerPadA,
    PowerPadB,
    SNESPad,
    SNESMouse,
    SuborMouse,
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
            ControllerType::SuborMouse => ControllerType::FourScore,
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
            ControllerType::FourScore => "Four Score",
            ControllerType::VirtualBoy => "Virtual Boy Gamepad",
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
                    "gamepad" | "nes gamepad" | "nesgamepad" | "nes_gamepad" | "nes controller" => ControllerType::Gamepad,
                    "famicomgamepad" | "famicom gamepad" | "famicom_gamepad" | "famicom controller" | "famicomcontroller" | "famicom" => ControllerType::FamicomGamepad,
                    "zapper" => ControllerType::Zapper,
                    "paddle" => ControllerType::Paddle,
                    "powerpada" | "power pad a" => ControllerType::PowerPadA,
                    "powerpadb" | "power pad b" => ControllerType::PowerPadB,
                    "snespad" | "snes pad" => ControllerType::SNESPad,
                    "snesmouse" | "snes mouse" => ControllerType::SNESMouse,
                    "subormouse" | "subor mouse" => ControllerType::SuborMouse,
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
    SuborKeyboard,
    BarcodeBattler,
    HoriTrack,
    BandaiHyperShot,
    TurboFile,
    BattleBox,
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
                    "suborkeyboard" | "subor keyboard" | "subor_keyboard" => ExpansionType::SuborKeyboard,
                    "barcodebattler" | "barcode battler" | "barcode_battler" => ExpansionType::BarcodeBattler,
                    "horitrack" | "hori track" | "hori_track" => ExpansionType::HoriTrack,
                    "bandaihypershot" | "bandai hyper shot" | "bandai_hyper_shot" => ExpansionType::BandaiHyperShot,
                    "turbofile" | "turbo file" | "turbo_file" => ExpansionType::TurboFile,
                    "battlebox" | "battle box" | "battle_box" => ExpansionType::BattleBox,
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
        ExpansionType::SuborKeyboard => "suborkeyboard",
        ExpansionType::BarcodeBattler => "barcodebattler",
        ExpansionType::HoriTrack => "horitrack",
        ExpansionType::BandaiHyperShot => "bandaihypershot",
        ExpansionType::TurboFile => "turbofile",
        ExpansionType::BattleBox => "battlebox",
    };
    upsert_config("expansion_type", s);
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ExpansionAdapterType {
    None,
    TwoPlayer,
    FourPlayer,
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
    SuborKeyboard,
    BarcodeBattler,
    HoriTrack,
    BandaiHyperShot,
    TurboFile,
    BattleBox,
    TwoPlayerAdapter,
    FourPlayerAdapter,
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
            ExpansionPortType::JissenMahjong => ExpansionPortType::SuborKeyboard,
            ExpansionPortType::SuborKeyboard => ExpansionPortType::BarcodeBattler,
            ExpansionPortType::BarcodeBattler => ExpansionPortType::HoriTrack,
            ExpansionPortType::HoriTrack => ExpansionPortType::BandaiHyperShot,
            ExpansionPortType::BandaiHyperShot => ExpansionPortType::TurboFile,
            ExpansionPortType::TurboFile => ExpansionPortType::BattleBox,
            ExpansionPortType::BattleBox => ExpansionPortType::TwoPlayerAdapter,
            ExpansionPortType::TwoPlayerAdapter => ExpansionPortType::FourPlayerAdapter,
            ExpansionPortType::FourPlayerAdapter => ExpansionPortType::None,
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
            ExpansionPortType::SuborKeyboard => "Subor Keyboard",
            ExpansionPortType::BarcodeBattler => "Barcode Battler",
            ExpansionPortType::HoriTrack => "Hori Track",
            ExpansionPortType::BandaiHyperShot => "Bandai Hyper Shot",
            ExpansionPortType::TurboFile => "Turbo File",
            ExpansionPortType::BattleBox => "Battle Box",
            ExpansionPortType::TwoPlayerAdapter => "2-Player Adapter",
            ExpansionPortType::FourPlayerAdapter => "4-Player Adapter",
        }
    }
    pub fn is_adapter(self) -> bool {
        matches!(self, ExpansionPortType::TwoPlayerAdapter | ExpansionPortType::FourPlayerAdapter)
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
        ExpansionPortType::SuborKeyboard => (ExpansionType::SuborKeyboard, ExpansionAdapterType::None),
        ExpansionPortType::BarcodeBattler => (ExpansionType::BarcodeBattler, ExpansionAdapterType::None),
        ExpansionPortType::HoriTrack => (ExpansionType::HoriTrack, ExpansionAdapterType::None),
        ExpansionPortType::BandaiHyperShot => (ExpansionType::BandaiHyperShot, ExpansionAdapterType::None),
        ExpansionPortType::TurboFile => (ExpansionType::TurboFile, ExpansionAdapterType::None),
        ExpansionPortType::BattleBox => (ExpansionType::BattleBox, ExpansionAdapterType::None),
        ExpansionPortType::TwoPlayerAdapter => (ExpansionType::None, ExpansionAdapterType::TwoPlayer),
        ExpansionPortType::FourPlayerAdapter => (ExpansionType::None, ExpansionAdapterType::FourPlayer),
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
    pub fn is_subor_keyboard(self) -> bool {
        matches!(self, ExpansionType::SuborKeyboard)
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


