#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// the main interface of how the emulator works!! the ui, the main module declarations, and interactions among other stuff is
// handled here!!

mod audio;
mod blip;
mod cartridge;
mod crc;
mod mappers;
mod mapper;
mod wxn;
mod studybox;
mod wavreader;
mod emulator;
mod cpu;
mod ppu;
mod apu;
mod bus;
mod config;
mod nesdb;
mod region;
mod vt03_palette;
mod vt32_palette;

use region::Region;
use emulator::Emulator;
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};
use std::rc::Rc;
use std::cell::RefCell;
use audio::AudioRingBuffer;
use std::thread;
use std::time::{Duration, Instant};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use winit::event::{Event as WinitEvent, WindowEvent};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::window::{WindowBuilder, Icon};
use softbuffer::{Context, Surface};
use font8x8::UnicodeFonts;
use gilrs::Gilrs;

#[cfg(windows)]
#[link(name = "winmm")]
extern "system" {
    fn timeBeginPeriod(u_period: u32) -> u32;
    fn timeEndPeriod(u_period: u32) -> u32;
}

pub struct TimerGuard(#[allow(dead_code)] u32);

impl TimerGuard {
    pub fn new(period_ms: u32) -> Self {
        #[cfg(windows)]
        unsafe {
            timeBeginPeriod(period_ms);
        }
        TimerGuard(period_ms)
    }
}

impl Drop for TimerGuard {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            timeEndPeriod(self.0);
        }
    }
}

const NES_WIDTH: u32 = 256;
const NES_HEIGHT: u32 = 240;
const SCALE: u32 = 3;
const BASE_MENU_HEIGHT: usize = 32;

fn mouse_button_str(button: &winit::event::MouseButton) -> String {
    match button {
        winit::event::MouseButton::Left => "MouseLeft".to_string(),
        winit::event::MouseButton::Right => "MouseRight".to_string(),
        winit::event::MouseButton::Middle => "MouseMiddle".to_string(),
        winit::event::MouseButton::Other(n) => format!("MouseOther({})", n),
    }
}



#[derive(Clone, Copy, PartialEq)]
enum Menu {
    File,    Nes,
    Options,
    Help,
}

#[derive(Clone, Copy, PartialEq)]
enum FileMenuItem {
    Open,
    Close,
    Recent,
    QuickSave,
    QuickLoad,
    SaveState,
    LoadState,
    Exit,
}

#[derive(Clone, Copy, PartialEq)]
enum NesMenuItem {
    Pause,
    DipSwitches,
    InsertCoin1,
    InsertCoin2,
    ServiceButton,
    InsertEjectDisk,
    SwapDisk,
    InputBarcode,
    Reset,
    PowerCycle,
}

#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum RegionMenuItem {
    Ntsc,
    Pal,
    Dendy,
    Auto,
}

#[derive(Clone)]
enum UpdateCheckState {
    Idle,
    Checking,
    UpToDate,
    Available { version: String, date: String, url: String },
    Error(String),
}

// dip switch stuff! this is parsed from dip.cfg which is actually from nintendulatornrs. credit where credit is due!

#[derive(Clone)]
struct DipChoice {
    name:  String,
    value: u32,
}

#[derive(Clone)]
struct DipSetting {
    name:         String,
    mask:         u32,
    default_val:  u32,
    choices:      Vec<DipChoice>,
}

#[derive(Clone)]
struct DipGame {
    settings: Vec<DipSetting>,
}

fn supergm3_dip_game() -> DipGame {
    DipGame {
        settings: vec![
            DipSetting {
                name: "Play Time per Credit".into(),
                mask: 0x03,
                default_val: 0x00,
                choices: vec![
                    DipChoice { name: "3 min".into(),  value: 0x00 },
                    DipChoice { name: "5 min".into(),  value: 0x02 },
                    DipChoice { name: "8 min".into(),  value: 0x01 },
                    DipChoice { name: "10 min".into(), value: 0x03 },
                ],
            },
            DipSetting {
                name: "Coinage".into(),
                mask: 0x0c,
                default_val: 0x00,
                choices: vec![
                    DipChoice { name: "3 Coins / 1 Credit".into(), value: 0x0c },
                    DipChoice { name: "2 Coins / 1 Credit".into(), value: 0x08 },
                    DipChoice { name: "1 Coin  / 1 Credit".into(),  value: 0x00 },
                    DipChoice { name: "1 Coin  / 2 Credits".into(), value: 0x04 },
                ],
            },
            DipSetting {
                name: "Enable 2 Players".into(),
                mask: 0x10,
                default_val: 0x00,
                choices: vec![
                    DipChoice { name: "Off".into(), value: 0x00 },
                    DipChoice { name: "On".into(),  value: 0x10 },
                ],
            },
        ],
    }
}

fn parse_dip_value(s: &str) -> Option<u32> {
    let s = s.split(';').next().unwrap_or("").trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<u32>().ok()
    }
}

fn load_dip_game(target_crc: u32, mapper_id: u16) -> Option<DipGame> {
    static DIP_TEXT_CACHE: OnceLock<Option<String>> = OnceLock::new();
    let text = DIP_TEXT_CACHE.get_or_init(|| {
        let cfg_path = (|| {
            let exe = std::env::current_exe().ok()?;
            let mut dir = exe.parent()?.to_path_buf();
            loop {
                let candidate = dir.join("dip.cfg");
                if candidate.exists() {
                    return Some(candidate);
                }
                if !dir.pop() {
                    break;
                }
            }
            None
        })()
        .unwrap_or_else(|| std::path::PathBuf::from("dip.cfg"));
        let raw = std::fs::read(&cfg_path).ok()?;
        let s: String = if raw.len() >= 2 && raw[0] == 0xFF && raw[1] == 0xFE {
            let u16s: Vec<u16> = raw[2..]
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect();
            String::from_utf16_lossy(&u16s).to_owned()
        } else if raw.len() >= 3 && raw[0] == 0xEF && raw[1] == 0xBB && raw[2] == 0xBF {
            String::from_utf8_lossy(&raw[3..]).into_owned()
        } else {
            String::from_utf8_lossy(&raw).into_owned()
        };
        Some(s)
    });
    let text = text.as_ref()?;

    let mut found_game  = false;
    let mut crc_matches = false;
    let mut game        = DipGame { settings: Vec::new() };
    let mut cur_setting: Option<DipSetting> = None;

    for raw_line in text.lines() {
        let line: &str = raw_line.trim_end();
        if line.trim_start().starts_with(';') || line.trim().is_empty() {
            continue;
        }

        let tokens: Vec<&str> = line.split('\t').map(str::trim).filter(|t| !t.is_empty()).collect();
        if tokens.is_empty() { continue; }

        match tokens[0] {
            "game" => {
                // save the previous setting of the previous game
                if let Some(s) = cur_setting.take() {
                    game.settings.push(s);
                }
                if found_game && crc_matches {
                    return Some(game);
                }
                // start a new game record
                found_game  = true;
                crc_matches = false;
                game        = DipGame { settings: Vec::new() };
            }
            "crc" => {
                if tokens.len() >= 2 {
                    if let Some(crc) = parse_dip_value(tokens[1]) {
                        crc_matches = crc == target_crc;
                    }
                }
            }
            "setting" => {
                if let Some(s) = cur_setting.take() {
                    game.settings.push(s);
                }
                let mut mask = 0u32;
                let mut name = String::new();
                let mut default_val = 0u32;
                let mut i = 1usize;
                while i < tokens.len() {
                    match tokens[i] {
                        "mask" if i + 1 < tokens.len() => {
                            mask = parse_dip_value(tokens[i + 1]).unwrap_or(0);
                            i += 2;
                        }
                        "default" if i + 1 < tokens.len() => {
                            default_val = parse_dip_value(tokens[i + 1]).unwrap_or(0);
                            i += 2;
                        }
                        "name" if i + 1 < tokens.len() => {
                            name = tokens[i + 1].trim_matches('"').to_string();
                            i += 2;
                        }
                        _ => { i += 1; }
                    }
                }
                cur_setting = Some(DipSetting { name, mask, default_val, choices: Vec::new() });
            }
            "choice" => {
                let mut value = 0u32;
                let mut name  = String::new();
                let mut i = 1usize;
                while i < tokens.len() {
                    match tokens[i] {
                        "value" if i + 1 < tokens.len() => {
                            value = parse_dip_value(tokens[i + 1]).unwrap_or(0);
                            i += 2;
                        }
                        "name" if i + 1 < tokens.len() => {
                            name = tokens[i + 1].trim_matches('"').to_string();
                            i += 2;
                        }
                        _ => { i += 1; }
                    }
                }
                if let Some(ref mut s) = cur_setting {
                    s.choices.push(DipChoice { name, value });
                }
            }
            _ => {}
        }
    }

    if let Some(s) = cur_setting.take() {
        game.settings.push(s);
    }
    if found_game && crc_matches {
        return Some(game);
    }

    if mapper_id == 124 {
        return Some(supergm3_dip_game());
    }

    None
}

// determine which vs system variant the game might be using!
fn compute_vs_ppu_variant(game: &DipGame, dip_val: u8, crc: u32) -> u8 {
    for setting in &game.settings {
        let name_lower = setting.name.to_lowercase();
        if name_lower.contains("ppu type") || name_lower.contains("ppu") {
            let active_val = (dip_val as u32) & setting.mask;
            if let Some(choice) = setting.choices.iter().find(|c| c.value == active_val) {
                let cn = choice.name.to_lowercase();
                if cn.contains("0001") { return 0; }
                if cn.contains("0002") { return 1; }
                if cn.contains("0003") { return 2; }
                if cn.contains("0004") { return 3; }
                if cn.contains("2c03") || cn.contains("rp2c03") || cn.contains("2c05") { return 4; }
                if cn.contains("rp2c02") { return 3; }
            }
        }
    }
    // crc method for similar results!
    match crc {
        // RP2C04-0001
        0xFF5135A3 | // Hogan's Alley
        0xD99A2087 | // Vs. Gradius
        0x17AE56BE | // Vs. Freedom Force
        0xE2C0A2BE | // Vs. Platoon
        0xCA85E56D | // Vs. Mighty Bomb Jack
        0xEC461DB9 | // Vs. Pinball
        0x44691677 | // Vs. Baseball
        0x381E5E08 | // Vs. ?? (unknown, but PPU fixed)
        0xAE8063EF => 0, // Vs. Mach Rider (Japanese)
        // RP2C04-0002
        0xFFBEF374 | // Vs. Castlevania
        0x12B36F73 | // Vs. Wrecking Crew
        0x70901B25 | // Vs. Slalom
        0x0B65A917 | // Vs. Mach Rider
        0x99FB3B3B | // Raid on Bungeling Bay
        0xCC2C4B5D => 1, // Vs. Stroke & Match Golf
        // RP2C04-0003
        0x9213A19E | // Vs. Balloon Fight
        0x46914E3E | // Vs. Soccer
        0xD5D7EAC4 | // Vs. Dr. Mario
        0x1E438D52 | // Vs. The Goonies
        0xE4407DB3 | // Vs. Excitebike (EB4-3 E)
        0xCBE85490 | // Vs. Excitebike (EB4-3 X)
        0x29155E0C => 2, // Vs. Excitebike (EB4-4 A)
        // RP2C04-0004
        0x8B60CC58 | // Vs. Super Mario Bros.
        0x43A357EF | // Ice Climber (IC4-4 X)
        0xD4EB5923 | // Ice Climber (IC4-4 B-1)
        0x07138C06 => 3, // Clu Clu Land
        // RP2C03
        0xED588F00 | // Vs. Duck Hunt (RC2C03B/RP2C03B)
        0xFE446787 | // Vs. Gumshoe (RC2C05-03, same base palette)
        0xD46B8C5F => 4, // Vs. Tennis (RC2C03B/RP2C03B)
        _ => 3,
    }
}

// ─────────────────────────────────────────────────────────────────────────────

struct MenuState {
    hovered_menu: Option<Menu>,
    active_menu: Option<Menu>,
    hovered_file_item: Option<FileMenuItem>,
    hovered_nes_item: Option<NesMenuItem>,
    hovered_region_item: Option<RegionMenuItem>,
    show_recent_submenu: bool,
    show_save_state_submenu: bool,
    show_load_state_submenu: bool,
    show_region_submenu: bool,
    current_region: Region,
    show_general_settings: bool,
    show_audio_settings: bool,
    show_video_settings: bool,
    show_input_settings: bool,
    show_controller1_settings: bool,
    show_controller2_settings: bool,
    show_expansion_settings: bool,
    adapter_pair: Option<usize>,
    rebind_controller: Option<u8>,
    rebind_button: Option<usize>,
    hovered_ctrl_button: Option<usize>,
    hovered_expansion_button: Option<usize>,
    hovered_recent_index: Option<usize>,
    hovered_region_index: Option<usize>,
    hovered_options_index: Option<usize>,
    hovered_help_index: Option<usize>,
    hovered_save_slot: Option<usize>,
    hovered_load_slot: Option<usize>,
    show_about: bool,
    show_updates: bool,
    show_error: bool,
    error_message: String,
    show_dip_switches: bool,
    show_barcode_input: bool,
    barcode_input: String,
    barcode_caret: usize,
    barcode_sel_anchor: Option<usize>,
    barcode_dragging: bool,
    about_icon_data: Option<Vec<u8>>,
    about_icon_size: (u32, u32),
    dip_hovered_bit: Option<u8>,
    dip_definition: Option<DipGame>,
    show_confirm_exit_dialog: bool,
    mouse_pos: (usize, usize),
    menu_height: usize,
    scale: f32,
    dragging_audio_slider: Option<usize>,
    screen_dest_x: usize,
    screen_dest_y: usize,
    screen_dest_w: usize,
    screen_dest_h: usize,
    theme: String,
}

impl MenuState {
    fn new() -> Self {
        Self {
            hovered_menu: None,
            active_menu: None,
            hovered_file_item: None,
            hovered_nes_item: None,
            hovered_region_item: None,
            show_recent_submenu: false,
            show_save_state_submenu: false,
            show_load_state_submenu: false,
            show_region_submenu: false,
            current_region: config::load_region(),
            show_general_settings: false,
            show_audio_settings: false,
            show_video_settings: false,
            show_input_settings: false,
            show_controller1_settings: false,
            show_controller2_settings: false,
            show_expansion_settings: false,
            adapter_pair: None,
            rebind_controller: None,
            rebind_button: None,
            hovered_ctrl_button: None,
            hovered_expansion_button: None,
            hovered_recent_index: None,
            hovered_region_index: None,
            hovered_options_index: None,
            hovered_help_index: None,
            hovered_save_slot: None,
            hovered_load_slot: None,
            show_about: false,
            show_updates: false,
            show_error: false,
            error_message: String::new(),
            show_dip_switches: false,
            show_barcode_input: false,
            barcode_input: String::new(),
            barcode_caret: 0,
            barcode_sel_anchor: None,
            barcode_dragging: false,
            about_icon_data: None,
            about_icon_size: (0, 0),
            dip_hovered_bit: None,
            dip_definition: None,
            show_confirm_exit_dialog: false,
            mouse_pos: (0, 0),
            menu_height: BASE_MENU_HEIGHT,
            scale: 1.0,
            dragging_audio_slider: None,
            screen_dest_x: 0,
            screen_dest_y: 0,
            screen_dest_w: 0,
            screen_dest_h: 0,
            theme: config::load_theme(),
        }
    }
}

#[derive(Clone, Copy)]
struct UiColors {
    menu_text: u32,
    disabled_text: u32,
    menu_highlight: u32,
    global_bg: u32,
    menu_bg: u32,
    dropdown_bg: u32,
    window_bg: u32,
    window_border: u32,
    close_bg: u32,
    box_border: u32,
    box_bg_hover: u32,
    box_bg_default: u32,
    slider_track: u32,
    slider_fill: u32,
    rebind_border: u32,
    rebind_bg: u32,
    btn_sub_label: u32,
    disabled_btn_bg: u32,
    btn_border: u32,
    dip_on_fill: u32,
}

const DARK_COLORS: UiColors = UiColors {
    menu_text: 0xFFFFFFFF,
    disabled_text: 0x88888888,
    menu_highlight: 0xFF505050,
    global_bg: 0xFF353535,
    menu_bg: 0xFF2D2D2D,
    dropdown_bg: 0xFF3D3D3D,
    window_bg: 0xFF4D4D4D,
    window_border: 0xFF6D6D6D,
    close_bg: 0xFF808080,
    box_border: 0xFF7D7D7D,
    box_bg_hover: 0xFF5D5D5D,
    box_bg_default: 0xFF2D2D2D,
    slider_track: 0xFF222222,
    slider_fill: 0xFF4488CC,
    rebind_border: 0xFFFFAA00,
    rebind_bg: 0xFF4D3D00,
    btn_sub_label: 0xFFAAAAAA,
    disabled_btn_bg: 0xFF252525,
    btn_border: 0xFF555555,
    dip_on_fill: 0xFF00FF00,
};

const LIGHT_COLORS: UiColors = UiColors {
    menu_text: 0xFF000000,
    disabled_text: 0xFF999999,
    menu_highlight: 0xFFD0D0D0,
    global_bg: 0xFFCCCCCC,
    menu_bg: 0xFFE0E0E0,
    dropdown_bg: 0xFFF0F0F0,
    window_bg: 0xFFF8F8F8,
    window_border: 0xFFAAAAAA,
    close_bg: 0xFFCCCCCC,
    box_border: 0xFFAAAAAA,
    box_bg_hover: 0xFFE0E0E0,
    box_bg_default: 0xFFFFFFFF,
    slider_track: 0xFFD0D0D0,
    slider_fill: 0xFF4488CC,
    rebind_border: 0xFFFFAA00,
    rebind_bg: 0xFFFFF0CC,
    btn_sub_label: 0xFF666666,
    disabled_btn_bg: 0xFFE8E8E8,
    btn_border: 0xFF999999,
    dip_on_fill: 0xFF00FF00,
};

const CLASSIC_NES_COLORS: UiColors = UiColors {
    menu_text: 0xFFFFFFFF,
    disabled_text: 0x88888888,
    menu_highlight: 0xFF505050,
    global_bg: 0xFF2A2A2A,
    menu_bg: 0xFF4A1A1A,
    dropdown_bg: 0xFF333333,
    window_bg: 0xFF3D3D3D,
    window_border: 0xFF555555,
    close_bg: 0xFFE02020,
    box_border: 0xFF555555,
    box_bg_hover: 0xFF4A4A4A,
    box_bg_default: 0xFF222222,
    slider_track: 0xFF333333,
    slider_fill: 0xFFE02020,
    rebind_border: 0xFFE02020,
    rebind_bg: 0xFF4D1A1A,
    btn_sub_label: 0xFFAAAAAA,
    disabled_btn_bg: 0xFF1E1E1E,
    btn_border: 0xFF444444,
    dip_on_fill: 0xFFE02020,
};

const FAMICOM_COLORS: UiColors = UiColors {
    menu_text: 0xFF000000,
    disabled_text: 0xFF999999,
    menu_highlight: 0xFFE8DCC0,
    global_bg: 0xFFD8D0C0,
    menu_bg: 0xFFF0E8D0,
    dropdown_bg: 0xFFF5F0E0,
    window_bg: 0xFFF8F4E8,
    window_border: 0xFFCC2222,
    close_bg: 0xFFCC2222,
    box_border: 0xFFCC2222,
    box_bg_hover: 0xFFE8DCC0,
    box_bg_default: 0xFFFFFBF0,
    slider_track: 0xFFE0D8C0,
    slider_fill: 0xFFD4A020,
    rebind_border: 0xFFD4A020,
    rebind_bg: 0xFFF5EDD0,
    btn_sub_label: 0xFF665544,
    disabled_btn_bg: 0xFFF0E8D8,
    btn_border: 0xFFCC2222,
    dip_on_fill: 0xFF4488CC,
};

const MARIO_COLORS: UiColors = UiColors {
    menu_text: 0xFFFFFFFF,
    disabled_text: 0x88888888,
    menu_highlight: 0xFFCC3333,
    global_bg: 0xFF1A2A5C,
    menu_bg: 0xFFCC2020,
    dropdown_bg: 0xFF2A3A6C,
    window_bg: 0xFF3A4A7C,
    window_border: 0xFFFFDD00,
    close_bg: 0xFFFFDD00,
    box_border: 0xFFFFDD00,
    box_bg_hover: 0xFF4A5A8C,
    box_bg_default: 0xFF1A2A5C,
    slider_track: 0xFF2A3A6C,
    slider_fill: 0xFFFFDD00,
    rebind_border: 0xFFFFDD00,
    rebind_bg: 0xFF5C3A1A,
    btn_sub_label: 0xFFCCCCCC,
    disabled_btn_bg: 0xFF2A2A4A,
    btn_border: 0xFFFFDD00,
    dip_on_fill: 0xFFFFDD00,
};

const LINK_COLORS: UiColors = UiColors {
    menu_text: 0xFFFFFFFF,
    disabled_text: 0x88888888,
    menu_highlight: 0xFF3A8A3A,
    global_bg: 0xFF1A3A1A,
    menu_bg: 0xFF2D6B2D,
    dropdown_bg: 0xFF2A4A2A,
    window_bg: 0xFF3A5A3A,
    window_border: 0xFFFFAA00,
    close_bg: 0xFFFFAA00,
    box_border: 0xFFFFAA00,
    box_bg_hover: 0xFF4A6A4A,
    box_bg_default: 0xFF1A3A1A,
    slider_track: 0xFF2A4A2A,
    slider_fill: 0xFFFFAA00,
    rebind_border: 0xFFFFAA00,
    rebind_bg: 0xFF3A2A1A,
    btn_sub_label: 0xFFAAAAAA,
    disabled_btn_bg: 0xFF243A24,
    btn_border: 0xFFFFAA00,
    dip_on_fill: 0xFFFFAA00,
};

const METROID_COLORS: UiColors = UiColors {
    menu_text: 0xFFFFFFFF,
    disabled_text: 0x88888888,
    menu_highlight: 0xFFFF4400,
    global_bg: 0xFF1A0A0A,
    menu_bg: 0xFFFF4400,
    dropdown_bg: 0xFF2A1A1A,
    window_bg: 0xFF3A2A2A,
    window_border: 0xFFFF6600,
    close_bg: 0xFFCC0000,
    box_border: 0xFFFF6600,
    box_bg_hover: 0xFF4A2A2A,
    box_bg_default: 0xFF1A0A0A,
    slider_track: 0xFF2A1A1A,
    slider_fill: 0xFFFF6600,
    rebind_border: 0xFFFF4400,
    rebind_bg: 0xFF3A1A0A,
    btn_sub_label: 0xFFAAAAAA,
    disabled_btn_bg: 0xFF221414,
    btn_border: 0xFFFF6600,
    dip_on_fill: 0xFFFF4400,
};

const MEGAMAN_COLORS: UiColors = UiColors {
    menu_text: 0xFFFFFFFF,
    disabled_text: 0x88888888,
    menu_highlight: 0xFF00CCFF,
    global_bg: 0xFF0A1A3A,
    menu_bg: 0xFF0055CC,
    dropdown_bg: 0xFF1A2A4A,
    window_bg: 0xFF2A3A5A,
    window_border: 0xFF00CCFF,
    close_bg: 0xFF00CCFF,
    box_border: 0xFF00CCFF,
    box_bg_hover: 0xFF3A4A6A,
    box_bg_default: 0xFF0A1A3A,
    slider_track: 0xFF1A2A4A,
    slider_fill: 0xFF00CCFF,
    rebind_border: 0xFF00CCFF,
    rebind_bg: 0xFF0A2A4A,
    btn_sub_label: 0xFFAAAAAA,
    disabled_btn_bg: 0xFF121A2A,
    btn_border: 0xFF00CCFF,
    dip_on_fill: 0xFF00CCFF,
};

const APP_VERSION: &str = "1.6.4";

fn strip_version_prefix(s: &str) -> &str {
    s.trim_start_matches(|c: char| c.is_ascii_alphabetic())
}

fn version_compare(a: &str, b: &str) -> std::cmp::Ordering {
    let a = strip_version_prefix(a);
    let b = strip_version_prefix(b);
    let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
    let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
    for i in 0..std::cmp::max(a_parts.len(), b_parts.len()) {
        let a_val = a_parts.get(i).copied().unwrap_or(0);
        let b_val = b_parts.get(i).copied().unwrap_or(0);
        if a_val > b_val { return std::cmp::Ordering::Greater; }
        if a_val < b_val { return std::cmp::Ordering::Less; }
    }
    std::cmp::Ordering::Equal
}

fn draw_char(buffer: &mut [u32], x: usize, y: usize, width: usize, c: char, color: u32, scale: f32) {
    if width == 0 { return; }
    let glyph = font8x8::BASIC_FONTS.get(c);
    if let Some(glyph) = glyph {
        let size = (8.0 * scale).round() as usize;
        if size == 0 { return; }

        let limit = size.min(128);
        let buffer_height = buffer.len() / width;

        for dy in 0..limit {
            let py = y + dy;
            if py >= buffer_height { continue; }
            let row_offset = py * width;

            let y_s = dy * 8 * 256 / size;
            let y_e = (dy + 1) * 8 * 256 / size;

            for dx in 0..limit {
                let px = x + dx;
                if px >= width { continue; }

                let x_s = dx * 8 * 256 / size;
                let x_e = (dx + 1) * 8 * 256 / size;

                let total = (x_e - x_s) as u64 * (y_e - y_s) as u64;
                let mut covered = 0u64;

                let gy0 = y_s / 256;
                let gy1 = ((y_e + 255) / 256).min(8);
                for gy in gy0..gy1 {
                    let row = glyph[gy];
                    if row == 0 { continue; }

                    let gy_pix = gy * 256;
                    let oy = (y_e.min(gy_pix + 256)).saturating_sub(y_s.max(gy_pix)) as u64;

                    let gx0 = x_s / 256;
                    let gx1 = ((x_e + 255) / 256).min(8);
                    for gx in gx0..gx1 {
                        if (row >> gx) & 1 == 0 { continue; }
                        let gx_pix = gx * 256;
                        let ox = (x_e.min(gx_pix + 256)).saturating_sub(x_s.max(gx_pix)) as u64;
                        covered += ox * oy;
                    }
                }

                let t = ((covered * 255 / total) as u32).min(255);
                if t > 0 {
                    let bg = buffer[row_offset + px];
                    let r = ((color >> 16) & 0xFF) * t / 255 + ((bg >> 16) & 0xFF) * (255 - t) / 255;
                    let g = ((color >> 8) & 0xFF) * t / 255 + ((bg >> 8) & 0xFF) * (255 - t) / 255;
                    let b = (color & 0xFF) * t / 255 + ((bg & 0xFF) * (255 - t) / 255);
                    buffer[row_offset + px] = 0xFF000000 | (r << 16) | (g << 8) | b;
                }
            }
        }
    }
}

fn draw_text(buffer: &mut [u32], x: usize, y: usize, width: usize, text: &str, color: u32, scale: f32) {
    let char_w = 8.0 * scale;
    for (i, c) in text.chars().enumerate() {
        let cx = x + (i as f32 * char_w).round() as usize;
        draw_char(buffer, cx, y, width, c, color, scale);
    }
}

fn truncate_text(text: &str, max_w: usize, scale: f32) -> String {
    let char_w = (8.0 * scale).round() as usize;
    if char_w == 0 { return String::new(); }
    let max_chars = max_w / char_w;
    let mut out: String = text.chars().take(max_chars).collect();
    if out.chars().count() < text.chars().count() {
        while out.chars().count() > 0 && out.chars().last() == Some('…') {
            out.pop();
        }
        if out.chars().count() > 0 {
            out.pop();
        }
        out.push('…');
    }
    out
}

fn draw_text_wrapped(
    buffer: &mut [u32],
    x: usize,
    y: usize,
    max_w: usize,
    buf_width: usize,
    text: &str,
    color: u32,
    scale: f32,
) -> usize {
    let char_w = (8.0 * scale).round() as usize;
    let line_h = (10.0 * scale).round() as usize;
    if char_w == 0 { return 1; }
    let max_chars = (max_w / char_w).max(1);

    let words: Vec<&str> = text.split(' ').collect();
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for word in &words {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current, word)
        };
        if candidate.chars().count() <= max_chars {
            current = candidate;
        } else {
            if !current.is_empty() {
                lines.push(current.clone());
            }
            let mut remaining = *word;
            while remaining.chars().count() > max_chars {
                let (chunk, rest) = remaining.split_at(
                    remaining.char_indices().nth(max_chars).map(|(i, _)| i).unwrap_or(remaining.len())
                );
                lines.push(chunk.to_string());
                remaining = rest;
            }
            current = remaining.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }

    for (li, line) in lines.iter().enumerate() {
        draw_text(buffer, x, y + li * line_h, buf_width, line, color, scale);
    }
    lines.len()
}

fn measure_wrapped_height(text: &str, max_w: usize, scale: f32) -> usize {
    let char_w = (8.0 * scale).round() as usize;
    let line_h = (10.0 * scale).round() as usize;
    if char_w == 0 { return line_h; }
    let max_chars = (max_w / char_w).max(1);

    let words: Vec<&str> = text.split(' ').collect();
    let mut lines: usize = 0;
    let mut current_len: usize = 0;

    for word in &words {
        let word_len = word.chars().count();
        let candidate_len = if current_len == 0 { word_len } else { current_len + 1 + word_len };
        if candidate_len <= max_chars {
            current_len = candidate_len;
        } else {
            if current_len > 0 { lines += 1; }
            let mut rem = word_len;
            while rem > max_chars { lines += 1; rem = rem.saturating_sub(max_chars); }
            current_len = rem;
        }
    }
    if current_len > 0 { lines += 1; }
    if lines == 0 { lines = 1; }
    lines * line_h
}

fn draw_rect(buffer: &mut [u32], x: usize, y: usize, w: usize, h: usize, width: usize, color: u32) {
    for py in y..y + h {
        for px in x..x + w {
            if px < width && py < buffer.len() / width {
                buffer[py * width + px] = color;
            }
        }
    }
}

fn draw_image_rgba(buffer: &mut [u32], x: usize, y: usize, width: usize, rgba_data: &[u8], img_w: u32, img_h: u32, scale: f32) {
    let scaled_w = (img_w as f32 * scale).round() as usize;
    let scaled_h = (img_h as f32 * scale).round() as usize;
    
    for sy in 0..scaled_h {
        for sx in 0..scaled_w {
            let src_x = (sx as f32 / scale).round() as usize;
            let src_y = (sy as f32 / scale).round() as usize;
            
            if src_x < img_w as usize && src_y < img_h as usize {
                let src_idx = (src_y * img_w as usize + src_x) * 4;
                if src_idx + 3 < rgba_data.len() {
                    let r = rgba_data[src_idx] as u32;
                    let g = rgba_data[src_idx + 1] as u32;
                    let b = rgba_data[src_idx + 2] as u32;
                    let a = rgba_data[src_idx + 3] as u32;
                    
                    let px = x + sx;
                    let py = y + sy;
                    
                    if px < width && py < buffer.len() / width {
                        if a > 128 {
                            buffer[py * width + px] = (a << 24) | (r << 16) | (g << 8) | b;
                        }
                    }
                }
            }
        }
    }
}

fn point_in_rect(px: usize, py: usize, x: usize, y: usize, w: usize, h: usize) -> bool {
    px >= x && px < x + w && py >= y && py < y + h
}

fn barcode_sel_bounds(anchor: Option<usize>, caret: usize) -> (usize, usize) {
    match anchor {
        Some(a) if a != caret => (a.min(caret), a.max(caret)),
        _ => (caret, caret),
    }
}

fn barcode_insert_digit(ms: &Rc<RefCell<MenuState>>, c: char) {
    let mut ms = ms.borrow_mut();
    let (a, b) = barcode_sel_bounds(ms.barcode_sel_anchor, ms.barcode_caret);
    if a != b {
        ms.barcode_input.drain(a..b);
        ms.barcode_caret = a;
        ms.barcode_sel_anchor = None;
    }
    if ms.barcode_input.len() < 13 {
        let caret = ms.barcode_caret;
        ms.barcode_input.insert(caret, c);
        ms.barcode_caret = caret.saturating_add(1);
    }
}

fn barcode_backspace(ms: &Rc<RefCell<MenuState>>) {
    let mut ms = ms.borrow_mut();
    let (a, b) = barcode_sel_bounds(ms.barcode_sel_anchor, ms.barcode_caret);
    if a != b {
        ms.barcode_input.drain(a..b);
        ms.barcode_caret = a;
        ms.barcode_sel_anchor = None;
    } else if ms.barcode_caret > 0 {
        ms.barcode_caret -= 1;
        let caret = ms.barcode_caret;
        ms.barcode_input.remove(caret);
    }
}

fn barcode_delete(ms: &Rc<RefCell<MenuState>>) {
    let mut ms = ms.borrow_mut();
    let (a, b) = barcode_sel_bounds(ms.barcode_sel_anchor, ms.barcode_caret);
    if a != b {
        ms.barcode_input.drain(a..b);
        ms.barcode_caret = a;
        ms.barcode_sel_anchor = None;
    } else if ms.barcode_caret < ms.barcode_input.len() {
        let caret = ms.barcode_caret;
        ms.barcode_input.remove(caret);
    }
}

fn barcode_move_caret(ms: &Rc<RefCell<MenuState>>, delta: isize, _shift: bool) {
    let mut ms = ms.borrow_mut();
    let new_pos = if delta < 0 {
        ms.barcode_caret.saturating_sub(1)
    } else {
        (ms.barcode_caret + 1).min(ms.barcode_input.len())
    };
    ms.barcode_caret = new_pos;
    ms.barcode_sel_anchor = None;
}

fn barcode_home_end(ms: &Rc<RefCell<MenuState>>, home: bool) {
    let mut ms = ms.borrow_mut();
    ms.barcode_caret = if home { 0 } else { ms.barcode_input.len() };
    ms.barcode_sel_anchor = None;
}

fn barcode_select_all(ms: &Rc<RefCell<MenuState>>) {
    let mut ms = ms.borrow_mut();
    let len = ms.barcode_input.len();
    if len > 0 {
        ms.barcode_sel_anchor = Some(0);
        ms.barcode_caret = len;
    }
}

fn barcode_paste(ms: &Rc<RefCell<MenuState>>) {
    let text = arboard::Clipboard::new()
        .ok()
        .and_then(|mut cb| cb.get_text().ok())
        .unwrap_or_default();
    if text.is_empty() {
        return;
    }
    let mut ms = ms.borrow_mut();
    let (a, b) = barcode_sel_bounds(ms.barcode_sel_anchor, ms.barcode_caret);
    if a != b {
        ms.barcode_input.drain(a..b);
        ms.barcode_caret = a;
        ms.barcode_sel_anchor = None;
    }
    let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    let space = 13usize.saturating_sub(ms.barcode_input.len());
    let take = digits.len().min(space);
    if take > 0 {
        let caret = ms.barcode_caret;
        ms.barcode_input.insert_str(caret, &digits[..take]);
        ms.barcode_caret = caret + take;
    }
}

fn barcode_caret_from_x(mx: usize, field_x: usize, sc: f32, len: usize) -> usize {
    let pad = (8.0 * sc).round() as usize;
    let char_w = (8.0 * sc).round() as usize;
    let off = mx.saturating_sub(field_x + pad);
    let idx = if char_w > 0 { off / char_w } else { 0 };
    idx.min(len)
}

fn calculate_item_positions(items: &[&str], dropdown_x: usize, dropdown_y: usize, dropdown_w: usize, scale: f32) -> Vec<(usize, usize, usize, usize)> {
    let pad_x = (8.0 * scale).round() as usize;
    let pad_y = (4.0 * scale).round() as usize;
    let text_max_w = dropdown_w.saturating_sub(pad_x * 2);
    
    let mut positions = Vec::new();
    let mut cur_y = dropdown_y;
    
    for item in items {
        let item_h = measure_wrapped_height(item, text_max_w, scale) + pad_y * 2;
        positions.push((dropdown_x, cur_y, dropdown_w, item_h));
        cur_y += item_h;
    }
    
    positions
}

fn calculate_submenu_positions(count: usize, submenu_x: usize, anchor_y: usize, submenu_w: usize, item_h: usize) -> Vec<(usize, usize, usize, usize)> {
    let mut positions = Vec::new();
    
    for i in 0..count {
        let item_y = anchor_y + i * item_h;
        positions.push((submenu_x, item_y, submenu_w, item_h));
    }
    
    positions
}

fn active_audio_channels(_rate: u32) -> &'static [(usize, &'static str)] {
    const ALL: &[(usize, &str)] = &[
        (0, "Master:"),
        (1, "Triangle:"),
        (2, "Square 1:"),
        (3, "Square 2:"),
        (4, "Noise:"),
        (5, "PCM:"),
    ];
    ALL
}



#[allow(dead_code)]
enum EmuCommand {
    LoadCartridge(cartridge::Cartridge),
    Reset,
    PowerCycle(config::InitialRam),
    InsertCoin(u8),
    ServiceButton,
    ChangeDisk,
    EjectDisk,
    InsertDisk,
    SavePrgRam,
    ClearCart,
    SetDipSwitches(u8),
    SetVsPpuVariant(u8),
    SetBarcode(Vec<u8>),
    SetRegionPreference(Region),
    SetController1Type(config::ControllerType),
    SetController2Type(config::ControllerType),
    Exit,
}

fn init_audio(device: &cpal::Device, phase_inc: Arc<Mutex<f64>>, audio_buffer: Arc<Mutex<AudioRingBuffer>>) -> (u32, cpal::Stream) {
    let supported = device.default_output_config().expect("Failed to get default output config");
    let channels = supported.channels() as usize;
    let sample_rate = supported.sample_rate().0;

    let stream = device.build_output_stream(
        &supported.into(),
        make_audio_callback(audio_buffer.clone(), channels, phase_inc),
        |err| eprintln!("Audio error: {}", err),
        None,
    ).expect("Failed to build output stream");

    stream.play().expect("Failed to play audio stream");
    println!("Audio output initialized at {} Hz", sample_rate);
    (sample_rate, stream)
}

fn make_audio_callback(buffer: Arc<Mutex<AudioRingBuffer>>, channels: usize, phase_inc: Arc<Mutex<f64>>) -> impl FnMut(&mut [f32], &cpal::OutputCallbackInfo) {
    const RAMP_FRAMES: f32 = audio::UNDERRUN_RAMP_FRAMES;
    let mut read_phase = 0.0f64;
    let mut last_output = 0.0f32;
    let mut ramp = 1.0f32;
    let mut scratch: Vec<f32> = Vec::with_capacity(4096);

    move |data, _| {
        let p_inc = *phase_inc.lock().unwrap();
        let frames = data.len() / channels;

        let (buf_len, copy_len) = {
            let ring = buffer.lock().unwrap();
                        let need = ((read_phase + p_inc * frames as f64).floor() as usize) + 2;
            let copy_len = need.min(ring.len());
            scratch.clear();
            if scratch.capacity() < copy_len {
                scratch.reserve(copy_len - scratch.len());
            }
            for i in 0..copy_len {
                scratch.push(ring.sample_at(i));
            }
            (ring.len(), copy_len)
        };

        if copy_len == 0 {
            for frame in data.chunks_mut(channels) {
                ramp = (ramp - 1.0 / RAMP_FRAMES).max(0.0);
                let s = last_output * ramp;
                for out in frame.iter_mut() {
                    *out = s;
                }
            }
            return;
        }

        let mut pos = read_phase;
        for frame in data.chunks_mut(channels) {
            let fi = pos.floor() as isize;
            if fi >= copy_len as isize {
                ramp = (ramp - 1.0 / RAMP_FRAMES).max(0.0);
                let s = last_output * ramp;
                for out in frame.iter_mut() {
                    *out = s;
                }
                continue;
            }

            let frac = (pos - fi as f64) as f32;
            let raw = if frac <= f32::EPSILON {
                scratch[fi as usize]
            } else {
                let i0 = (fi - 1).max(0) as usize;
                let i1 = fi as usize;
                let i2 = ((fi + 1).min(copy_len as isize - 1)) as usize;
                let i3 = ((fi + 2).min(copy_len as isize - 1)) as usize;
                let (y0, y1, y2, y3) = (scratch[i0], scratch[i1], scratch[i2], scratch[i3]);
                let c0 = y1;
                let c1 = 0.5 * (y2 - y0);
                let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
                let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
                ((c3 * frac + c2) * frac + c1) * frac + c0
            };
            last_output = raw;
            ramp = (ramp + 1.0 / RAMP_FRAMES).min(1.0);
            let out = raw * ramp;
            for o in frame.iter_mut() {
                *o = out;
            }
            pos += p_inc;
        }

        let consumed = pos.floor() as usize;
        let mut actual = consumed.min(buf_len);
        {
            let mut ring = buffer.lock().unwrap();
            let avail = ring.len();
            if avail < actual {
                actual = avail;
            }
            ring.consume(actual);
        }
        read_phase = pos - actual as f64;
    }
}

fn main() {
    let window_width = NES_WIDTH * SCALE;
    let window_height = NES_HEIGHT * SCALE;

    let icon_data = std::fs::read("accunesicon.ico").expect("Failed to read icon file");
    let icon_image = image::load_from_memory(&icon_data).expect("Failed to decode icon image");
    let icon_rgba = icon_image.to_rgba8();
    let (width, height) = icon_rgba.dimensions();
    let icon = Icon::from_rgba(icon_rgba.to_vec(), width, height).expect("Failed to create icon");

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("AccuNES 1.6.4")
        .with_inner_size(winit::dpi::PhysicalSize::new(window_width, window_height))
        .with_window_icon(Some(icon))
        .build(&event_loop)
        .expect("Failed to create window");

    let context = unsafe { Context::new(&window) }.expect("Failed to create softbuffer context");
    let surface = Rc::new(RefCell::new(unsafe { Surface::new(&context, &window) }.expect("Failed to create softbuffer surface")));

    let emu = Arc::new(Mutex::new(Emulator::new()));
    let controller_port1 = Arc::new(AtomicU8::new(0));
    let controller_port2 = Arc::new(AtomicU8::new(0));
    let controller_port3 = Arc::new(AtomicU8::new(0));
    let controller_port4 = Arc::new(AtomicU8::new(0));
    let expansion_adapter_port1 = Arc::new(AtomicU8::new(0));
    let expansion_adapter_port2 = Arc::new(AtomicU8::new(0));
    let expansion_adapter_port3 = Arc::new(AtomicU8::new(0));
    let expansion_adapter_port4 = Arc::new(AtomicU8::new(0));
    let zapper_x = Arc::new(Mutex::new(0.0f32));
    let zapper_y = Arc::new(Mutex::new(0.0f32));
    let zapper_trigger = Arc::new(AtomicBool::new(false));
    let expansion_zapper_trigger = Arc::new(AtomicBool::new(false));
    let oeka_click = Arc::new(AtomicBool::new(false));
    let family_trainer_state = Arc::new(Mutex::new([0u8; 12]));
    let hyper_shot_state = Arc::new(Mutex::new([0u8; 4]));
    let bandai_hyper_buttons = Arc::new(Mutex::new([0u8; 9]));
    let family_basic_state = Arc::new(Mutex::new([0u8; 72]));
    let party_tap_state = Arc::new(Mutex::new([0u8; 6]));
    let pachinko_state = Arc::new(Mutex::new([0u8; 10]));
    let punching_bag_state = Arc::new(Mutex::new([0u8; 8]));
    let jissen_mahjong_state = Arc::new(Mutex::new([0u8; 21]));
    let subor_keyboard_state = Arc::new(Mutex::new([0u8; 104]));
    let famicom_mic = Arc::new(AtomicBool::new(false));
    let zapper_bogo = Arc::new(AtomicU8::new(0));
    let paddle_x = Arc::new(Mutex::new([0u8; 3]));
    let paddle_button = Arc::new(Mutex::new([false; 3]));
    let powerpad_state = Arc::new(Mutex::new([0u16; 2]));
    let snes_state = Arc::new(Mutex::new([0u16; 2]));
    let virtualboy_state = Arc::new(Mutex::new([0u16; 2]));
    let snes_mouse_delta_x = Arc::new(Mutex::new([0.0f32; 2]));
    let snes_mouse_delta_y = Arc::new(Mutex::new([0.0f32; 2]));
    let snes_mouse_buttons = Arc::new(Mutex::new([0u8; 2]));
    let subor_mouse_buttons = Arc::new(Mutex::new([0u8; 2]));
    let subor_mouse_dx = Arc::new(Mutex::new([0i32; 2]));
    let subor_mouse_dy = Arc::new(Mutex::new([0i32; 2]));
    let hori_track_dx = Arc::new(Mutex::new([0.0f32; 2]));
    let hori_track_dy = Arc::new(Mutex::new([0.0f32; 2]));
    {
        let mut e = emu.lock().unwrap();
        e.controller_port1 = controller_port1.clone();
        e.controller_port2 = controller_port2.clone();
        e.controller_port3 = controller_port3.clone();
        e.controller_port4 = controller_port4.clone();
        e.expansion_adapter_ports = [expansion_adapter_port1.clone(), expansion_adapter_port2.clone(), expansion_adapter_port3.clone(), expansion_adapter_port4.clone()];
        e.zapper_x = zapper_x.clone();
        e.zapper_y = zapper_y.clone();
        e.zapper_trigger = zapper_trigger.clone();
        e.expansion_zapper_trigger = expansion_zapper_trigger.clone();
        e.oeka_click = oeka_click.clone();
        e.family_trainer_state = family_trainer_state.clone();
        e.hyper_shot_state = hyper_shot_state.clone();
        e.bandai_hyper_buttons = bandai_hyper_buttons.clone();
        e.family_basic_state = family_basic_state.clone();
        e.party_tap_state = party_tap_state.clone();
        e.pachinko_state = pachinko_state.clone();
        e.punching_bag_state = punching_bag_state.clone();
        e.jissen_mahjong_state = jissen_mahjong_state.clone();
        e.subor_keyboard_state = subor_keyboard_state.clone();
        e.famicom_mic = famicom_mic.clone();
        e.zapper_bogo = zapper_bogo.clone();
        e.paddle_x = paddle_x.clone();
        e.paddle_button = paddle_button.clone();
        e.powerpad_state = powerpad_state.clone();
        e.snes_state = snes_state.clone();
        e.virtualboy_state = virtualboy_state.clone();
        e.snes_mouse_delta_x = snes_mouse_delta_x.clone();
        e.snes_mouse_delta_y = snes_mouse_delta_y.clone();
        e.snes_mouse_buttons = snes_mouse_buttons.clone();
        e.subor_mouse_buttons = subor_mouse_buttons.clone();
        e.subor_mouse_dx = subor_mouse_dx.clone();
        e.subor_mouse_dy = subor_mouse_dy.clone();
        e.hori_track_dx = hori_track_dx.clone();
        e.hori_track_dy = hori_track_dy.clone();
        e.region_preference = config::load_region();
    }
    let cpal_device = cpal::default_host().default_output_device().expect("Failed to get default output device");
    let configured_rate = config::load_audio_rate();
    let audio_rate = Rc::new(RefCell::new(configured_rate));
    let audio_device_rate = Arc::new(Mutex::new(0u32));
    let phase_inc = Arc::new(Mutex::new(1.0));
                    let audio_buffer = Arc::new(Mutex::new(AudioRingBuffer::new(configured_rate as usize)));
    let (device_rate, _audio_stream) = init_audio(&cpal_device, phase_inc.clone(), audio_buffer.clone());
    *audio_device_rate.lock().unwrap() = device_rate;
    *phase_inc.lock().unwrap() = configured_rate as f64 / device_rate as f64;
    {
        let mut e = emu.lock().unwrap();
        e.set_audio_output(audio_buffer.clone(), configured_rate as f64);
    }

    let audio_enabled = Rc::new(RefCell::new(config::load_audio_enabled()));
    let audio_depth = Rc::new(RefCell::new(config::load_audio_depth()));
    let channel_volumes: Rc<RefCell<[u8; 6]>> = Rc::new(RefCell::new([
        config::load_channel_volume("master"),
        config::load_channel_volume("triangle"),
        config::load_channel_volume("square1"),
        config::load_channel_volume("square2"),
        config::load_channel_volume("noise"),
        config::load_channel_volume("pcm"),
    ]));
    {
        let mut e = emu.lock().unwrap();
        e.audio_enabled = *audio_enabled.borrow();
        e.audio_depth = *audio_depth.borrow();
        let vols = channel_volumes.borrow();
        e.master_volume = vols[0] as f32 / 100.0;
        e.triangle_volume = vols[1] as f32 / 100.0;
        e.square1_volume = vols[2] as f32 / 100.0;
        e.square2_volume = vols[3] as f32 / 100.0;
        e.noise_volume = vols[4] as f32 / 100.0;
        e.pcm_volume = vols[5] as f32 / 100.0;
    }

    let fullscreen = Rc::new(RefCell::new(config::load_fullscreen()));
    let fullscreen_on_game_load = Rc::new(RefCell::new(config::load_fullscreen_on_game_load()));
    let hide_mouse_cursor = Rc::new(RefCell::new(config::load_hide_mouse_cursor()));
    let crop_overscan = Rc::new(RefCell::new(config::load_crop_overscan()));
    window.set_cursor_visible(!config::load_hide_mouse_cursor());
    if config::load_fullscreen() {
        window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
    }

    let rom_loaded = Rc::new(RefCell::new(false));
    let current_rom = Rc::new(RefCell::new(Option::<String>::None));
    let paused = Arc::new(AtomicBool::new(false));
    let barcode_ctrl = Arc::new(AtomicBool::new(false));
    let pause_on_lost_focus = Rc::new(RefCell::new(config::load_pause_on_lost_focus()));
    let initial_ram = Rc::new(RefCell::new(config::load_initial_ram()));
    let fps_mode = Rc::new(RefCell::new(config::load_fps_mode()));
    let confirm_on_exit = Rc::new(RefCell::new(config::load_confirm_on_exit()));
    let auto_save_sram = Rc::new(RefCell::new(config::load_auto_save_sram()));
    let check_updates_on_startup = Rc::new(RefCell::new(config::load_check_updates_on_startup()));
    let controller1_type = Rc::new(RefCell::new(config::load_controller_type("controller1_type")));
    let controller2_type = Rc::new(RefCell::new(config::load_controller_type("controller2_type")));
    let expansion_type = Rc::new(RefCell::new(config::load_expansion_type()));
    let expansion_adapter_type = Rc::new(RefCell::new(config::load_expansion_adapter_type()));
    emu.lock().unwrap().controller1_type = *controller1_type.borrow();
    emu.lock().unwrap().controller2_type = *controller2_type.borrow();
    emu.lock().unwrap().expansion_type = *expansion_type.borrow();
    emu.lock().unwrap().expansion_adapter_type = *expansion_adapter_type.borrow();
    let allow_opposing_dpad = Rc::new(RefCell::new(config::load_allow_opposing_dpad()));
    let auto_detect_game_controller = Rc::new(RefCell::new(config::load_auto_detect_game_controller()));
    let controller1_bindings = Rc::new(RefCell::new(config::load_bindings("controller1")));
    let controller2_bindings = Rc::new(RefCell::new(config::load_bindings("controller2")));
    let zapper_trigger_binding = Rc::new(RefCell::new(config::load_zapper_trigger()));
    let expansion_zapper_trigger_binding = Rc::new(RefCell::new(config::load_expansion_zapper_trigger()));
    let expansion_oeka_click_binding = Rc::new(RefCell::new(config::load_expansion_oeka_click()));
    let family_trainer_bindings = Rc::new(RefCell::new(config::load_powerpad_bindings("familytrainer")));
    let hyper_shot_bindings = Rc::new(RefCell::new(config::load_hyper_shot_bindings("hypershot")));
    let bandai_hyper_bindings = Rc::new(RefCell::new(config::load_bandai_hyper_shot_bindings("bandaihypershot")));
    let family_basic_bindings = Rc::new(RefCell::new(config::load_family_basic_bindings("familybasic")));
    let party_tap_bindings = Rc::new(RefCell::new(config::load_party_tap_bindings("partytap")));
    let pachinko_bindings = Rc::new(RefCell::new(config::load_pachinko_bindings("pachinko")));
    let punching_bag_bindings = Rc::new(RefCell::new(config::load_exciting_boxing_bindings("punchingbag")));
    let jissen_mahjong_bindings = Rc::new(RefCell::new(config::load_jissen_mahjong_bindings("jissenmahjong")));
    let subor_keyboard_bindings = Rc::new(RefCell::new(config::load_subor_keyboard_bindings("suborkeyboard")));
    let famicom_mic_binding = Rc::new(RefCell::new(config::load_famicom_mic()));

    let paddle1_button_binding = Rc::new(RefCell::new(config::load_paddle_button("controller1")));
    let paddle2_button_binding = Rc::new(RefCell::new(config::load_paddle_button("controller2")));
    let expansion_paddle_button_binding = Rc::new(RefCell::new(config::load_paddle_button("expansion")));
    let powerpad1_bindings = Rc::new(RefCell::new(config::load_powerpad_bindings("controller1")));
    let powerpad2_bindings = Rc::new(RefCell::new(config::load_powerpad_bindings("controller2")));
    let snes1_bindings = Rc::new(RefCell::new(config::load_snes_bindings("controller1")));
    let snes2_bindings = Rc::new(RefCell::new(config::load_snes_bindings("controller2")));
    let snes_mouse1_bindings = Rc::new(RefCell::new(config::load_snes_mouse_bindings("controller1")));
    let snes_mouse2_bindings = Rc::new(RefCell::new(config::load_snes_mouse_bindings("controller2")));
    let subor_mouse1_bindings = Rc::new(RefCell::new(config::load_subor_mouse_bindings("controller1")));
    let subor_mouse2_bindings = Rc::new(RefCell::new(config::load_subor_mouse_bindings("controller2")));
    let vb1_bindings = Rc::new(RefCell::new(config::load_vb_bindings("controller1")));
    let vb2_bindings = Rc::new(RefCell::new(config::load_vb_bindings("controller2")));
    let controller3_bindings = Rc::new(RefCell::new(config::load_bindings("controller3")));
    let controller4_bindings = Rc::new(RefCell::new(config::load_bindings("controller4")));
    let expansion1_bindings = Rc::new(RefCell::new(config::load_expansion_adapter_bindings(0)));
    let expansion2_bindings = Rc::new(RefCell::new(config::load_expansion_adapter_bindings(1)));
    let expansion3_bindings = Rc::new(RefCell::new(config::load_expansion_adapter_bindings(2)));
    let expansion4_bindings = Rc::new(RefCell::new(config::load_expansion_adapter_bindings(3)));
    let last_mouse_x = Rc::new(RefCell::new(0.0f64));
    let last_mouse_y = Rc::new(RefCell::new(0.0f64));
    let menu_state = Rc::new(RefCell::new(MenuState::new()));
    
    let icon_data = std::fs::read("accunesicon.ico").expect("Failed to read icon file");
    let icon_image = image::load_from_memory(&icon_data).expect("Failed to decode icon image");
    let icon_rgba = icon_image.to_rgba8();
    let (icon_w, icon_h) = icon_rgba.dimensions();
    {
        let mut ms = menu_state.borrow_mut();
        ms.about_icon_data = Some(icon_rgba.to_vec());
        ms.about_icon_size = (icon_w, icon_h);
    }
    
    let mut gilrs = match Gilrs::new() {
        Ok(gilrs) => Some(gilrs),
        Err(e) => {
            eprintln!("Failed to initialize gamepad support: {}", e);
            menu_state.borrow_mut().show_error = true;
            menu_state.borrow_mut().error_message =
                format!("Failed to initialize gamepad support: {}", e);
            None
        }
    };
    
    let recent_roms = Rc::new(RefCell::new(Vec::<String>::new()));
    let quick_save_slot = Rc::new(RefCell::new(Option::<Vec<u8>>::None));
    let update_check_state = Arc::new(Mutex::new(UpdateCheckState::Idle));
    let auto_check_started = Arc::new(AtomicBool::new(false));

    if *check_updates_on_startup.borrow() {
        auto_check_started.store(true, Ordering::Relaxed);
        let update_state = update_check_state.clone();
        let auto_started = auto_check_started.clone();
        std::thread::spawn(move || {
            {
                let mut state = update_state.lock().unwrap();
                *state = UpdateCheckState::Checking;
            }
            let tls = match ureq::native_tls::TlsConnector::new() {
                Ok(c) => c,
                Err(_) => {
                    let mut state = update_state.lock().unwrap();
                    *state = UpdateCheckState::Idle;
                    auto_started.store(false, Ordering::Relaxed);
                    return;
                }
            };
            let agent = ureq::AgentBuilder::new()
                .tls_connector(std::sync::Arc::new(tls))
                .build();
            match agent
                .get("https://api.github.com/repos/ammaroussema/accunes/releases/latest")
                .set("User-Agent", "AccuNES")
                .call()
            {
                Ok(response) => {
                    match response.into_json::<serde_json::Value>() {
                        Ok(json) => {
                            let tag = json["tag_name"].as_str().unwrap_or("unknown").to_string();
                            let raw_date = json["published_at"].as_str().unwrap_or("").to_string();
                            let date = if raw_date.len() >= 10 { raw_date[..10].to_string() } else { raw_date };
                            let url = json["html_url"].as_str().unwrap_or("https://github.com/ammaroussema/accunes/releases/latest").to_string();
                            let mut state = update_state.lock().unwrap();
                            let tag_clean = tag.trim_start_matches('v');
                            if version_compare(tag_clean, APP_VERSION) == std::cmp::Ordering::Greater {
                                *state = UpdateCheckState::Available { version: tag, date, url };
                            } else {
                                *state = UpdateCheckState::UpToDate;
                                auto_started.store(false, Ordering::Relaxed);
                            }
                        }
                        Err(_e) => {
                            let mut state = update_state.lock().unwrap();
                            *state = UpdateCheckState::Idle;
                            auto_started.store(false, Ordering::Relaxed);
                        }
                    }
                }
                Err(_e) => {
                    let mut state = update_state.lock().unwrap();
                    *state = UpdateCheckState::Idle;
                    auto_started.store(false, Ordering::Relaxed);
                }
            }
        });
    }

    if let Ok(roms) = std::fs::read_to_string(".recent_roms") {
        *recent_roms.borrow_mut() = roms.lines().take(8).map(|s| s.to_string()).collect();
    } else {
        let _ = std::fs::write(".recent_roms", "");
    }

    let _frame_count = Rc::new(RefCell::new(0));
    let fps_update_time = Rc::new(RefCell::new(std::time::Instant::now()));
    let current_fps = Arc::new(AtomicU32::new(0));
    let emu_frame_count = Arc::new(AtomicU32::new(0));

    let window = Arc::new(window);
    let surface_clone = surface.clone();
    let menu_state_clone = menu_state.clone();
    let recent_roms_clone = recent_roms.clone();
    let quick_save_slot_clone = quick_save_slot.clone();


    let emu_clone = emu.clone();
    let rom_loaded_clone = rom_loaded.clone();
    let current_rom_clone = current_rom.clone();
    let paused_clone = paused.clone();
    let barcode_ctrl_clone = barcode_ctrl.clone();
    let pause_on_lost_focus_clone = pause_on_lost_focus.clone();
    let initial_ram_clone = initial_ram.clone();
    let fps_mode_clone = fps_mode.clone();
    let confirm_on_exit_clone = confirm_on_exit.clone();
    let auto_save_sram_clone = auto_save_sram.clone();
    let check_updates_on_startup_clone = check_updates_on_startup.clone();
    let controller1_type_clone = controller1_type.clone();
    let controller2_type_clone = controller2_type.clone();
    let expansion_type_clone = expansion_type.clone();
    let allow_opposing_dpad_clone = allow_opposing_dpad.clone();
    let auto_detect_game_controller_clone = auto_detect_game_controller.clone();
    let controller1_bindings_clone = controller1_bindings.clone();
    let controller2_bindings_clone = controller2_bindings.clone();
    let controller_port1_clone = controller_port1.clone();
    let controller_port2_clone = controller_port2.clone();
    let controller_port3_clone = controller_port3.clone();
    let controller_port4_clone = controller_port4.clone();
    let expansion_adapter_port1_clone = expansion_adapter_port1.clone();
    let expansion_adapter_port2_clone = expansion_adapter_port2.clone();
    let expansion_adapter_port3_clone = expansion_adapter_port3.clone();
    let expansion_adapter_port4_clone = expansion_adapter_port4.clone();
    let zapper_x_clone = zapper_x.clone();
    let zapper_y_clone = zapper_y.clone();
    let zapper_trigger_clone = zapper_trigger.clone();
    let expansion_zapper_trigger_clone = expansion_zapper_trigger.clone();
    let oeka_click_clone = oeka_click.clone();
    let family_trainer_state_clone = family_trainer_state.clone();
    let hyper_shot_state_clone = hyper_shot_state.clone();
    let bandai_hyper_buttons_clone = bandai_hyper_buttons.clone();
    let family_basic_state_clone = family_basic_state.clone();
    let party_tap_state_clone = party_tap_state.clone();
    let pachinko_state_clone = pachinko_state.clone();
    let punching_bag_state_clone = punching_bag_state.clone();
    let jissen_mahjong_state_clone = jissen_mahjong_state.clone();
    let subor_keyboard_state_clone = subor_keyboard_state.clone();
    let famicom_mic_clone = famicom_mic.clone();
    let zapper_bogo_clone = zapper_bogo.clone();
    let paddle_x_clone = paddle_x.clone();
    let paddle_button_clone = paddle_button.clone();
    let powerpad_state_clone = powerpad_state.clone();
    let snes_state_clone = snes_state.clone();
    let snes_mouse_delta_x_clone = snes_mouse_delta_x.clone();
    let snes_mouse_delta_y_clone = snes_mouse_delta_y.clone();
    let snes_mouse_buttons_clone = snes_mouse_buttons.clone();
    let subor_mouse_buttons_clone = subor_mouse_buttons.clone();
    let subor_mouse_dx_clone = subor_mouse_dx.clone();
    let subor_mouse_dy_clone = subor_mouse_dy.clone();
    let hori_track_dx_clone = hori_track_dx.clone();
    let hori_track_dy_clone = hori_track_dy.clone();
    let zapper_trigger_binding_clone = zapper_trigger_binding.clone();
    let expansion_zapper_trigger_binding_clone = expansion_zapper_trigger_binding.clone();
    let expansion_oeka_click_binding_clone = expansion_oeka_click_binding.clone();
    let family_trainer_bindings_clone = family_trainer_bindings.clone();
    let hyper_shot_bindings_clone = hyper_shot_bindings.clone();
    let bandai_hyper_bindings_clone = bandai_hyper_bindings.clone();
    let family_basic_bindings_clone = family_basic_bindings.clone();
    let party_tap_bindings_clone = party_tap_bindings.clone();
    let pachinko_bindings_clone = pachinko_bindings.clone();
    let punching_bag_bindings_clone = punching_bag_bindings.clone();
    let jissen_mahjong_bindings_clone = jissen_mahjong_bindings.clone();
    let subor_keyboard_bindings_clone = subor_keyboard_bindings.clone();
    let famicom_mic_binding_clone = famicom_mic_binding.clone();
    let paddle1_button_binding_clone = paddle1_button_binding.clone();
    let paddle2_button_binding_clone = paddle2_button_binding.clone();
    let expansion_paddle_button_binding_clone = expansion_paddle_button_binding.clone();
    let powerpad1_bindings_clone = powerpad1_bindings.clone();
    let powerpad2_bindings_clone = powerpad2_bindings.clone();
    let snes1_bindings_clone = snes1_bindings.clone();
    let snes2_bindings_clone = snes2_bindings.clone();
    let snes_mouse1_bindings_clone = snes_mouse1_bindings.clone();
    let snes_mouse2_bindings_clone = snes_mouse2_bindings.clone();
    let subor_mouse1_bindings_clone = subor_mouse1_bindings.clone();
    let subor_mouse2_bindings_clone = subor_mouse2_bindings.clone();
    let vb1_bindings_clone = vb1_bindings.clone();
    let vb2_bindings_clone = vb2_bindings.clone();
    let virtualboy_state_clone = virtualboy_state.clone();
    let controller3_bindings_clone = controller3_bindings.clone();
    let controller4_bindings_clone = controller4_bindings.clone();
    let expansion1_bindings_clone = expansion1_bindings.clone();
    let expansion2_bindings_clone = expansion2_bindings.clone();
    let expansion3_bindings_clone = expansion3_bindings.clone();
    let expansion4_bindings_clone = expansion4_bindings.clone();
    let expansion_adapter_type_clone = expansion_adapter_type.clone();
    let auto_apply_input = {
        let c1_type = controller1_type.clone();
        let c2_type = controller2_type.clone();
        let exp_type = expansion_type.clone();
        let exp_adapter = expansion_adapter_type.clone();
        let adc = auto_detect_game_controller.clone();
        let ecu = emu.clone();
        move |cart: &cartridge::Cartridge| {
            if !*adc.borrow() {
                return;
            }
            let Some(det) = crate::nesdb::detect(cart.prg_chr_crc32) else {
                return;
            };
            *c1_type.borrow_mut() = det.port1;
            config::save_controller_type("controller1_type", det.port1);
            *c2_type.borrow_mut() = det.port2;
            config::save_controller_type("controller2_type", det.port2);
            *exp_type.borrow_mut() = det.expansion;
            config::save_expansion_type(det.expansion);
            *exp_adapter.borrow_mut() = det.adapter;
            config::save_expansion_adapter_type(det.adapter);
            if let Ok(mut e) = ecu.lock() {
                e.controller1_type = det.port1;
                e.controller2_type = det.port2;
                e.expansion_type = det.expansion;
                e.expansion_adapter_type = det.adapter;
            }
            println!("Auto-detected controller setup for {:?}", cart.name);
        }
    };
    let last_mouse_x_clone = last_mouse_x.clone();
    let last_mouse_y_clone = last_mouse_y.clone();
    let fps_update_time_clone = fps_update_time.clone();
    let current_fps_clone = current_fps.clone();
    let audio_enabled_clone = audio_enabled.clone();
    let audio_depth_clone = audio_depth.clone();
    let audio_rate_clone = audio_rate.clone();
    let audio_device_rate_clone = audio_device_rate.clone();
    let phase_inc_clone = phase_inc.clone();
    let audio_buffer_clone2 = audio_buffer.clone();
    let channel_volumes_clone = channel_volumes.clone();

    let fullscreen_clone = fullscreen.clone();
    let fullscreen_on_game_load_clone = fullscreen_on_game_load.clone();
    let hide_mouse_cursor_clone = hide_mouse_cursor.clone();
    let crop_overscan_clone = crop_overscan.clone();

    let screen_buffer = Arc::new(Mutex::new(vec![0u32; (NES_WIDTH * NES_HEIGHT) as usize]));
    let (cmd_tx, cmd_rx) = mpsc::channel::<EmuCommand>();
    let exit_flag = Arc::new(AtomicBool::new(false));
    let rom_loaded_flag = Arc::new(AtomicBool::new(false));
    let disk_inserted_flag = Arc::new(AtomicBool::new(false));

    let screen_buffer_clone = screen_buffer.clone();
    let rom_loaded_flag_clone = rom_loaded_flag.clone();
    let disk_inserted_clone = disk_inserted_flag.clone();

    let emu_thread = emu.clone();
    let screen_out = screen_buffer.clone();
    let exit_for_thread = exit_flag.clone();
    let emu_frame_count_thread = emu_frame_count.clone();
    let current_fps_thread = current_fps.clone();
    let rom_loaded_for_thread = rom_loaded_flag.clone();
    let paused_thread = paused.clone();
    let disk_inserted_flag_thread = disk_inserted_flag.clone();

    let emu_frame_count_ui = emu_frame_count.clone();

    thread::spawn(move || {
        let _timer_guard = TimerGuard::new(1);
        let target_ntsc = Duration::from_secs_f64(1.0 / 60.0);
        let target_pal = Duration::from_secs_f64(1.0 / 50.0);
        let target_dendy = Duration::from_secs_f64(1.0 / 50.0);
        let mut fps_count_local: u32 = 0;
        let mut fps_time_local = Instant::now();
        let mut next_deadline = Instant::now();

        loop {
            if exit_for_thread.load(Ordering::Relaxed) {
                break;
            }
            if !rom_loaded_for_thread.load(Ordering::Relaxed) || paused_thread.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(2));
                next_deadline = Instant::now();
                continue;
            }

            let target: Duration;
            {
                let mut e = emu_thread.lock().unwrap();
                target = if e.is_pal() {
                    target_pal
                } else if e.is_dendy() {
                    target_dendy
                } else {
                    target_ntsc
                };

                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        EmuCommand::Exit => { return; }
                        EmuCommand::LoadCartridge(cart) => {
                            e.save_turbo_file();
                            e.save_battle_box();
                            e.load_cartridge(cart);
                            disk_inserted_flag_thread.store(e.disk_inserted(), Ordering::Relaxed);
                        }
                        EmuCommand::Reset => { e.reset(); }
                        EmuCommand::PowerCycle(mode) => { e.power_cycle(mode); }
                        EmuCommand::InsertCoin(n) => { e.insert_coin(n); }
                        EmuCommand::ServiceButton => { e.service_button(); }
                        EmuCommand::ChangeDisk => {
                            e.change_disk();
                            disk_inserted_flag_thread.store(e.disk_inserted(), Ordering::Relaxed);
                        }
                        EmuCommand::EjectDisk => {
                            e.eject_disk();
                            disk_inserted_flag_thread.store(e.disk_inserted(), Ordering::Relaxed);
                        }
                        EmuCommand::InsertDisk => {
                            e.insert_disk();
                            disk_inserted_flag_thread.store(e.disk_inserted(), Ordering::Relaxed);
                        }
                        EmuCommand::SavePrgRam => { e.save_prg_ram(); e.save_turbo_file(); e.save_battle_box(); }
                        EmuCommand::ClearCart => {
                            e.save_turbo_file();
                            e.save_battle_box();
                            e.clear_cart();
                            disk_inserted_flag_thread.store(false, Ordering::Relaxed);
                        }
                        EmuCommand::SetDipSwitches(val) => { e.set_dip_switches(val); }
                        EmuCommand::SetVsPpuVariant(v) => { e.set_vs_ppu_variant(v); }
                        EmuCommand::SetBarcode(rcode) => { e.set_barcode(&rcode); }
                        EmuCommand::SetRegionPreference(r) => { e.set_region_preference(r); }
                        EmuCommand::SetController1Type(t) => { e.controller1_type = t; }
                        EmuCommand::SetController2Type(t) => { e.controller2_type = t; }
                    }
                }

                e.core_frame_advance();
                disk_inserted_flag_thread.store(e.disk_inserted(), Ordering::Relaxed);
                emu_frame_count_thread.fetch_add(1, Ordering::Relaxed);

                let mut screen = screen_out.lock().unwrap();
                screen.copy_from_slice(&e.screen);
            }
            fps_count_local += 1;
            let elapsed_fps = fps_time_local.elapsed();
            if elapsed_fps >= Duration::from_secs(1) {
                let calculated_fps = (fps_count_local as f64 / elapsed_fps.as_secs_f64()).round() as u32;
                current_fps_thread.store(calculated_fps, Ordering::Relaxed);
                fps_count_local = 0;
                fps_time_local = Instant::now();
            }

            next_deadline += target;
            let now = Instant::now();
            if now >= next_deadline {
                if now > next_deadline + target * 3 {
                    next_deadline = now;
                }
            } else {
                let remaining = next_deadline - now;
                if remaining > Duration::from_micros(2000) {
                    thread::sleep(remaining - Duration::from_micros(1500));
                }
                while Instant::now() < next_deadline {
                    std::hint::spin_loop();
                }
            }
        }
    });

    let mut last_rendered_frame: u32 = u32::MAX;
    let mut last_rendered_mouse: (usize, usize) = (usize::MAX, usize::MAX);
    let mut last_menu_active: bool = false;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(4));

        const GP_DEFAULTS: [&str; 10] = ["South", "East", "West", "North", "Select", "Start", "DPadUp", "DPadDown", "DPadLeft", "DPadRight"];
        while let Some(gilrs_event) = gilrs.as_mut().and_then(|g| g.next_event()) {
            let (pressed, btn_name) = match &gilrs_event.event {
                gilrs::EventType::ButtonPressed(b, _) | gilrs::EventType::ButtonRepeated(b, _) => (true, format!("{:?}", b)),
                gilrs::EventType::ButtonReleased(b, _) => (false, format!("{:?}", b)),
                _ => continue,
            };

            let rebound = {
                let ms = menu_state_clone.borrow();
                if ms.rebind_controller.is_some() && ms.rebind_button.is_some() && pressed {
                    let ctrl = ms.rebind_controller.unwrap();
                    let b = ms.rebind_button.unwrap();
                    drop(ms);
                    let mut ms_mut = menu_state_clone.borrow_mut();
                    ms_mut.rebind_controller = None;
                    ms_mut.rebind_button = None;
                    let prefix = if ctrl == 1 { "controller1" } else { "controller2" };
                    if *expansion_adapter_type_clone.borrow() != config::ExpansionAdapterType::None
                        && ms_mut.show_expansion_settings {
                        let pair = ms_mut.adapter_pair.unwrap_or(0);
                        let pl = pair * 2 + (ctrl as usize).checked_sub(1).unwrap_or(0);
                        let port_bindings = match pl {
                            0 => Some(&mut *expansion1_bindings_clone.borrow_mut()),
                            1 => Some(&mut *expansion2_bindings_clone.borrow_mut()),
                            2 => Some(&mut *expansion3_bindings_clone.borrow_mut()),
                            _ => Some(&mut *expansion4_bindings_clone.borrow_mut()),
                        };
                        if let Some(bindings) = port_bindings {
                            bindings[b] = btn_name.clone();
                            config::save_expansion_adapter_binding(pl, b, &btn_name);
                        }
                    } else if ctrl == 1 {
                        let c1t = *controller1_type_clone.borrow();
                        if c1t == config::ControllerType::PowerPadA || c1t == config::ControllerType::PowerPadB {
                            powerpad1_bindings_clone.borrow_mut()[b] = btn_name.clone();
                            config::save_powerpad_binding(prefix, b, &btn_name);
                        } else if c1t == config::ControllerType::SNESPad {
                            snes1_bindings_clone.borrow_mut()[b] = btn_name.clone();
                            config::save_snes_binding(prefix, b, &btn_name);
                        } else if c1t == config::ControllerType::SNESMouse {
                            snes_mouse1_bindings_clone.borrow_mut()[b] = btn_name.clone();
                            config::save_snes_mouse_binding(prefix, b, &btn_name);
                        } else if c1t == config::ControllerType::SuborMouse {
                            subor_mouse1_bindings_clone.borrow_mut()[b] = btn_name.clone();
                            config::save_subor_mouse_binding(prefix, b, &btn_name);
                        } else if c1t == config::ControllerType::VirtualBoy {
                            let s = config::vb_serial_from_menu(b);
                            vb1_bindings_clone.borrow_mut()[s] = btn_name.clone();
                            config::save_vb_binding(prefix, s, &btn_name);
                        } else if c1t == config::ControllerType::Paddle {
                            *paddle1_button_binding_clone.borrow_mut() = btn_name.clone();
                            config::save_paddle_button("controller1", &btn_name);
                        } else {
                            controller1_bindings_clone.borrow_mut()[b] = btn_name.clone();
                            config::save_binding(prefix, b, &btn_name);
                        }
                    } else if *controller2_type_clone.borrow() == config::ControllerType::Zapper {
                        *zapper_trigger_binding_clone.borrow_mut() = btn_name.clone();
                        config::save_zapper_trigger(&btn_name);
                    } else if *controller2_type_clone.borrow() == config::ControllerType::Paddle {
                        *paddle2_button_binding_clone.borrow_mut() = btn_name.clone();
                        config::save_paddle_button("controller2", &btn_name);
                    } else if *controller2_type_clone.borrow() == config::ControllerType::PowerPadA || *controller2_type_clone.borrow() == config::ControllerType::PowerPadB {
                        powerpad2_bindings_clone.borrow_mut()[b] = btn_name.clone();
                        config::save_powerpad_binding(prefix, b, &btn_name);
                    } else if *controller2_type_clone.borrow() == config::ControllerType::SNESPad {
                        snes2_bindings_clone.borrow_mut()[b] = btn_name.clone();
                        config::save_snes_binding(prefix, b, &btn_name);
                    } else if *controller2_type_clone.borrow() == config::ControllerType::SNESMouse {
                        snes_mouse2_bindings_clone.borrow_mut()[b] = btn_name.clone();
                        config::save_snes_mouse_binding(prefix, b, &btn_name);
                    } else if *controller2_type_clone.borrow() == config::ControllerType::SuborMouse {
                        subor_mouse2_bindings_clone.borrow_mut()[b] = btn_name.clone();
                        config::save_subor_mouse_binding(prefix, b, &btn_name);
                    } else if *controller2_type_clone.borrow() == config::ControllerType::VirtualBoy {
                        let s = config::vb_serial_from_menu(b);
                        vb2_bindings_clone.borrow_mut()[s] = btn_name.clone();
                        config::save_vb_binding(prefix, s, &btn_name);
                    } else if ctrl == 2 {
                        controller2_bindings_clone.borrow_mut()[b] = btn_name.clone();
                        config::save_binding(prefix, b, &btn_name);
                    } else if ctrl == 3 {
                        controller3_bindings_clone.borrow_mut()[b] = btn_name.clone();
                        config::save_binding("controller3", b, &btn_name);
                    } else if ctrl == 4 {
                        controller4_bindings_clone.borrow_mut()[b] = btn_name.clone();
                        config::save_binding("controller4", b, &btn_name);
                    }
                    true
                } else { false }
            };
            if rebound { continue; }
            const BIT_MASKS: [u8; 10] = [0x80, 0x40, 0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01];
            // controller 1
            let mut matched1 = false;
            if *controller1_type_clone.borrow() != config::ControllerType::None {
                let b1 = controller1_bindings_clone.borrow();
                for (i, s) in b1.iter().enumerate() {
                    if s == &btn_name {
                        let mask = BIT_MASKS[i];
                        if pressed { controller_port1_clone.fetch_or(mask, Ordering::Relaxed); } else { controller_port1_clone.fetch_and(!mask, Ordering::Relaxed); }
                        if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                            if controller_port1_clone.load(Ordering::Relaxed) & 0x03 == 0x03 { controller_port1_clone.fetch_and(!mask, Ordering::Relaxed); }
                            if controller_port1_clone.load(Ordering::Relaxed) & 0x0C == 0x0C { controller_port1_clone.fetch_and(!mask, Ordering::Relaxed); }
                        }
                        matched1 = true;
                        break;
                    }
                }
            }
            if !matched1 && *controller1_type_clone.borrow() != config::ControllerType::None {
                for (i, &def) in GP_DEFAULTS.iter().enumerate() {
                    if def == btn_name {
                        let mask = BIT_MASKS[i];
                        if pressed { controller_port1_clone.fetch_or(mask, Ordering::Relaxed); } else { controller_port1_clone.fetch_and(!mask, Ordering::Relaxed); }
                        if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                            if controller_port1_clone.load(Ordering::Relaxed) & 0x03 == 0x03 { controller_port1_clone.fetch_and(!mask, Ordering::Relaxed); }
                            if controller_port1_clone.load(Ordering::Relaxed) & 0x0C == 0x0C { controller_port1_clone.fetch_and(!mask, Ordering::Relaxed); }
                        }
                        break;
                    }
                }
            }
            // controller 2
            let mut matched2 = false;
            if *controller2_type_clone.borrow() == config::ControllerType::Gamepad || *controller2_type_clone.borrow() == config::ControllerType::FamicomGamepad || *controller1_type_clone.borrow() == config::ControllerType::FourScore {
                let b2 = controller2_bindings_clone.borrow();
                for (i, s) in b2.iter().enumerate() {
                    if s == &btn_name {
                        let mask = BIT_MASKS[i];
                        if pressed { controller_port2_clone.fetch_or(mask, Ordering::Relaxed); } else { controller_port2_clone.fetch_and(!mask, Ordering::Relaxed); }
                        if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                            if controller_port2_clone.load(Ordering::Relaxed) & 0x03 == 0x03 { controller_port2_clone.fetch_and(!mask, Ordering::Relaxed); }
                            if controller_port2_clone.load(Ordering::Relaxed) & 0x0C == 0x0C { controller_port2_clone.fetch_and(!mask, Ordering::Relaxed); }
                        }
                        matched2 = true;
                        break;
                    }
                }
            }
            if !matched2 && (*controller2_type_clone.borrow() == config::ControllerType::Gamepad || *controller2_type_clone.borrow() == config::ControllerType::FamicomGamepad || *controller1_type_clone.borrow() == config::ControllerType::FourScore) {
                for (i, &def) in GP_DEFAULTS.iter().enumerate() {
                    if def == btn_name {
                        let mask = BIT_MASKS[i];
                        if pressed { controller_port2_clone.fetch_or(mask, Ordering::Relaxed); } else { controller_port2_clone.fetch_and(!mask, Ordering::Relaxed); }
                        if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                            if controller_port2_clone.load(Ordering::Relaxed) & 0x03 == 0x03 { controller_port2_clone.fetch_and(!mask, Ordering::Relaxed); }
                            if controller_port2_clone.load(Ordering::Relaxed) & 0x0C == 0x0C { controller_port2_clone.fetch_and(!mask, Ordering::Relaxed); }
                        }
                        break;
                    }
                }
            }
            // famicom microphone
            if *controller2_type_clone.borrow() == config::ControllerType::FamicomGamepad {
                let mb = famicom_mic_binding_clone.borrow();
                if &*mb == &btn_name {
                    if pressed { famicom_mic_clone.store(true, Ordering::Relaxed); } else { famicom_mic_clone.store(false, Ordering::Relaxed); }
                }
            }
            // controller 3 and 4 (four score)
            if *controller1_type_clone.borrow() == config::ControllerType::FourScore || *controller2_type_clone.borrow() == config::ControllerType::FourScore {
                let b3 = controller3_bindings_clone.borrow();
                for (i, s) in b3.iter().enumerate() {
                    if s == &btn_name {
                        let mask = BIT_MASKS[i];
                        if pressed { controller_port3_clone.fetch_or(mask, Ordering::Relaxed); } else { controller_port3_clone.fetch_and(!mask, Ordering::Relaxed); }
                        if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                            if controller_port3_clone.load(Ordering::Relaxed) & 0x03 == 0x03 { controller_port3_clone.fetch_and(!mask, Ordering::Relaxed); }
                            if controller_port3_clone.load(Ordering::Relaxed) & 0x0C == 0x0C { controller_port3_clone.fetch_and(!mask, Ordering::Relaxed); }
                        }
                        break;
                    }
                }
                for (i, &def) in GP_DEFAULTS.iter().enumerate() {
                    if def == btn_name {
                        let mask = BIT_MASKS[i];
                        if pressed { controller_port3_clone.fetch_or(mask, Ordering::Relaxed); } else { controller_port3_clone.fetch_and(!mask, Ordering::Relaxed); }
                        if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                            if controller_port3_clone.load(Ordering::Relaxed) & 0x03 == 0x03 { controller_port3_clone.fetch_and(!mask, Ordering::Relaxed); }
                            if controller_port3_clone.load(Ordering::Relaxed) & 0x0C == 0x0C { controller_port3_clone.fetch_and(!mask, Ordering::Relaxed); }
                        }
                        break;
                    }
                }
                let b4 = controller4_bindings_clone.borrow();
                for (i, s) in b4.iter().enumerate() {
                    if s == &btn_name {
                        let mask = BIT_MASKS[i];
                        if pressed { controller_port4_clone.fetch_or(mask, Ordering::Relaxed); } else { controller_port4_clone.fetch_and(!mask, Ordering::Relaxed); }
                        if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                            if controller_port4_clone.load(Ordering::Relaxed) & 0x03 == 0x03 { controller_port4_clone.fetch_and(!mask, Ordering::Relaxed); }
                            if controller_port4_clone.load(Ordering::Relaxed) & 0x0C == 0x0C { controller_port4_clone.fetch_and(!mask, Ordering::Relaxed); }
                        }
                        break;
                    }
                }
                for (i, &def) in GP_DEFAULTS.iter().enumerate() {
                    if def == btn_name {
                        let mask = BIT_MASKS[i];
                        if pressed { controller_port4_clone.fetch_or(mask, Ordering::Relaxed); } else { controller_port4_clone.fetch_and(!mask, Ordering::Relaxed); }
                        if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                            if controller_port4_clone.load(Ordering::Relaxed) & 0x03 == 0x03 { controller_port4_clone.fetch_and(!mask, Ordering::Relaxed); }
                            if controller_port4_clone.load(Ordering::Relaxed) & 0x0C == 0x0C { controller_port4_clone.fetch_and(!mask, Ordering::Relaxed); }
                        }
                        break;
                    }
                }
            }
            // expansion adapters
            if *expansion_adapter_type_clone.borrow() != config::ExpansionAdapterType::None {
                let exp_bindings = [
                    expansion1_bindings_clone.borrow(),
                    expansion2_bindings_clone.borrow(),
                    expansion3_bindings_clone.borrow(),
                    expansion4_bindings_clone.borrow(),
                ];
                let exp_ports = [
                    &expansion_adapter_port1_clone,
                    &expansion_adapter_port2_clone,
                    &expansion_adapter_port3_clone,
                    &expansion_adapter_port4_clone,
                ];
                for (pi, bindings) in exp_bindings.iter().enumerate() {
                    for (i, s) in bindings.iter().enumerate() {
                        if s == &btn_name {
                            let mask = BIT_MASKS[i];
                            let port = exp_ports[pi];
                            if pressed { port.fetch_or(mask, Ordering::Relaxed); } else { port.fetch_and(!mask, Ordering::Relaxed); }
                            if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                                if port.load(Ordering::Relaxed) & 0x03 == 0x03 { port.fetch_and(!mask, Ordering::Relaxed); }
                                if port.load(Ordering::Relaxed) & 0x0C == 0x0C { port.fetch_and(!mask, Ordering::Relaxed); }
                            }
                            break;
                        }
                    }
                }
            }
            // zapper trigger (also vs zapper trigger)
            {
                let zt = zapper_trigger_binding_clone.borrow();
                if *zt == btn_name {
                    if pressed {
                        zapper_trigger_clone.store(true, Ordering::Relaxed);
                        zapper_bogo_clone.store(3, Ordering::Relaxed);
                    } else {
                        zapper_trigger_clone.store(false, Ordering::Relaxed);
                    }
                }
            }
            // expansion zapper trigger
            {
                let ezt = expansion_zapper_trigger_binding_clone.borrow();
                if *ezt == btn_name {
                    if pressed {
                        expansion_zapper_trigger_clone.store(true, Ordering::Relaxed);
                        zapper_bogo_clone.store(3, Ordering::Relaxed);
                    } else {
                        expansion_zapper_trigger_clone.store(false, Ordering::Relaxed);
                    }
                }
            }
            // oeka kids tablet click
            {
                let okt = expansion_oeka_click_binding_clone.borrow();
                if *okt == btn_name {
                    oeka_click_clone.store(pressed, Ordering::Relaxed);
                }
            }
            // family trainer buttons
            if expansion_type_clone.borrow().is_family_trainer() {
                let ft = family_trainer_bindings_clone.borrow();
                let side_b = *expansion_type_clone.borrow() == config::ExpansionType::FamilyTrainerB;
                let mut ft_lock = family_trainer_state_clone.lock().unwrap();
                for (i, s) in ft.iter().enumerate() {
                    if s == &btn_name {
                        let idx = if side_b { (i / 4) * 4 + (3 - (i % 4)) } else { i };
                        ft_lock[idx] = if pressed { 1 } else { 0 };
                    }
                }
            }
            // konami hyper shot buttons
            if expansion_type_clone.borrow().is_hyper_shot() {
                let hs = hyper_shot_bindings_clone.borrow();
                let mut hs_lock = hyper_shot_state_clone.lock().unwrap();
                for (i, s) in hs.iter().enumerate() {
                    if s == &btn_name {
                        hs_lock[i] = if pressed { 1 } else { 0 };
                    }
                }
            }
            // bandai hyper shot buttons
            if expansion_type_clone.borrow().is_bandai_hyper_shot() {
                let bh = bandai_hyper_bindings_clone.borrow();
                let mut bh_lock = bandai_hyper_buttons_clone.lock().unwrap();
                for (i, s) in bh.iter().enumerate() {
                    if s == &btn_name {
                        bh_lock[i] = if pressed { 1 } else { 0 };
                    }
                }
            }
            // family basic keyboard buttons
            if expansion_type_clone.borrow().is_family_basic() {
                let fb = family_basic_bindings_clone.borrow();
                let mut fb_lock = family_basic_state_clone.lock().unwrap();
                for (i, s) in fb.iter().enumerate() {
                    if s == &btn_name {
                        fb_lock[i] = if pressed { 1 } else { 0 };
                    }
                }
            }
            // party tap buttons
            if expansion_type_clone.borrow().is_party_tap() {
                let pt = party_tap_bindings_clone.borrow();
                let mut pt_lock = party_tap_state_clone.lock().unwrap();
                for (i, s) in pt.iter().enumerate() {
                    if s == &btn_name {
                        pt_lock[i] = if pressed { 1 } else { 0 };
                    }
                }
            }
            // pachinko controller buttons
            if expansion_type_clone.borrow().is_pachinko() {
                let pc = pachinko_bindings_clone.borrow();
                let mut pc_lock = pachinko_state_clone.lock().unwrap();
                for (i, s) in pc.iter().enumerate() {
                    if s == &btn_name {
                        pc_lock[i] = if pressed { 1 } else { 0 };
                    }
                }
            }
            // exciting boxing (punching bag) buttons
            if expansion_type_clone.borrow().is_exciting_boxing() {
                let eb = punching_bag_bindings_clone.borrow();
                let mut eb_lock = punching_bag_state_clone.lock().unwrap();
                for (i, s) in eb.iter().enumerate() {
                    if s == &btn_name {
                        eb_lock[i] = if pressed { 1 } else { 0 };
                    }
                }
            }
            // jissen mahjong buttons
            if expansion_type_clone.borrow().is_jissen_mahjong() {
                let jm = jissen_mahjong_bindings_clone.borrow();
                let mut jm_lock = jissen_mahjong_state_clone.lock().unwrap();
                for (i, s) in jm.iter().enumerate() {
                    if s == &btn_name {
                        jm_lock[i] = if pressed { 1 } else { 0 };
                    }
                }
            }
            if expansion_type_clone.borrow().is_subor_keyboard() {
                let sk = subor_keyboard_bindings_clone.borrow();
                let mut sk_lock = subor_keyboard_state_clone.lock().unwrap();
                for (i, s) in sk.iter().enumerate() {
                    if s == &btn_name {
                        sk_lock[i] = if pressed { 1 } else { 0 };
                    }
                }
            }
            // paddle buttons
            {
                let pb = paddle1_button_binding_clone.borrow();
                if *pb == btn_name {
                    paddle_button_clone.lock().unwrap()[0] = pressed;
                }
            }
            {
                let pb = paddle2_button_binding_clone.borrow();
                if *pb == btn_name {
                    paddle_button_clone.lock().unwrap()[1] = pressed;
                }
            }
            {
                let pb = expansion_paddle_button_binding_clone.borrow();
                if *pb == btn_name {
                    paddle_button_clone.lock().unwrap()[2] = pressed;
                }
            }
            // powerpad buttons
            {
                let pp1 = powerpad1_bindings_clone.borrow();
                for (i, s) in pp1.iter().enumerate() {
                    if s == &btn_name {
                        let mut pp = powerpad_state_clone.lock().unwrap();
                        if pressed { pp[0] |= 1 << i; } else { pp[0] &= !(1 << i); }
                    }
                }
            }
            {
                let pp2 = powerpad2_bindings_clone.borrow();
                for (i, s) in pp2.iter().enumerate() {
                    if s == &btn_name {
                        let mut pp = powerpad_state_clone.lock().unwrap();
                        if pressed { pp[1] |= 1 << i; } else { pp[1] &= !(1 << i); }
                    }
                }
            }
            // snes pad buttons
            {
                let s1 = snes1_bindings_clone.borrow();
                for (i, s) in s1.iter().enumerate() {
                    if s == &btn_name {
                        let mut ss = snes_state_clone.lock().unwrap();
                        if pressed { ss[0] |= 1 << i; } else { ss[0] &= !(1 << i); }
                    }
                }
            }
            {
                let s2 = snes2_bindings_clone.borrow();
                for (i, s) in s2.iter().enumerate() {
                    if s == &btn_name {
                        let mut ss = snes_state_clone.lock().unwrap();
                        if pressed { ss[1] |= 1 << i; } else { ss[1] &= !(1 << i); }
                    }
                }
            }
            // virtual boy pad buttons
            {
                let v1 = vb1_bindings_clone.borrow();
                for (i, s) in v1.iter().enumerate() {
                    if s == &btn_name {
                        let mut vs = virtualboy_state_clone.lock().unwrap();
                        if pressed { vs[0] |= 1 << i; } else { vs[0] &= !(1 << i); }
                    }
                }
            }
            {
                let v2 = vb2_bindings_clone.borrow();
                for (i, s) in v2.iter().enumerate() {
                    if s == &btn_name {
                        let mut vs = virtualboy_state_clone.lock().unwrap();
                        if pressed { vs[1] |= 1 << i; } else { vs[1] &= !(1 << i); }
                    }
                }
            }
            // snes mouse buttons
            {
                let m1 = snes_mouse1_bindings_clone.borrow();
                for (i, s) in m1.iter().enumerate() {
                    if s == &btn_name {
                        let mut mb = snes_mouse_buttons_clone.lock().unwrap();
                        if pressed { mb[0] |= 1 << i; } else { mb[0] &= !(1 << i); }
                    }
                }
            }
            {
                let m2 = snes_mouse2_bindings_clone.borrow();
                for (i, s) in m2.iter().enumerate() {
                    if s == &btn_name {
                        let mut mb = snes_mouse_buttons_clone.lock().unwrap();
                        if pressed { mb[1] |= 1 << i; } else { mb[1] &= !(1 << i); }
                    }
                }
            }
            // subor mouse buttons
            {
                let m1 = subor_mouse1_bindings_clone.borrow();
                for (i, s) in m1.iter().enumerate() {
                    if s == &btn_name {
                        let mut sb = subor_mouse_buttons_clone.lock().unwrap();
                        if pressed { sb[0] |= 1 << i; } else { sb[0] &= !(1 << i); }
                    }
                }
            }
            {
                let m2 = subor_mouse2_bindings_clone.borrow();
                for (i, s) in m2.iter().enumerate() {
                    if s == &btn_name {
                        let mut sb = subor_mouse_buttons_clone.lock().unwrap();
                        if pressed { sb[1] |= 1 << i; } else { sb[1] &= !(1 << i); }
                    }
                }
            }
        }

        match event {
            WinitEvent::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if *confirm_on_exit_clone.borrow() && *rom_loaded_clone.borrow() {
                    menu_state_clone.borrow_mut().show_confirm_exit_dialog = true;
                    paused_clone.store(true, Ordering::Relaxed);
                    window.request_redraw();
                } else {
                    if *auto_save_sram_clone.borrow() && *rom_loaded_clone.borrow() {
                        emu_clone.lock().unwrap().save_prg_ram();
                        emu_clone.lock().unwrap().save_turbo_file();
                        emu_clone.lock().unwrap().save_battle_box();
                    }
                    *control_flow = ControlFlow::Exit;
                }
            }
            WinitEvent::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {

            }
            WinitEvent::WindowEvent {
                event: WindowEvent::Focused(gained),
                ..
            } => {
                if *pause_on_lost_focus_clone.borrow() && *rom_loaded_clone.borrow() {
                    paused_clone.store(!gained, Ordering::Relaxed);
                }
            }
            WinitEvent::WindowEvent {
                event: WindowEvent::KeyboardInput {
                    input: winit::event::KeyboardInput {
                        virtual_keycode: Some(keycode),
                        state,
                        ..
                    },
                    ..
                },
                ..
            } => {
                let pressed = state == winit::event::ElementState::Pressed;
                match keycode {
                    winit::event::VirtualKeyCode::LControl
                    | winit::event::VirtualKeyCode::RControl => {
                        barcode_ctrl_clone.store(pressed, Ordering::Relaxed);
                    }
                    _ => {}
                }
                let key_str = format!("{:?}", keycode);
                let typing_barcode = {
                    let ms = menu_state_clone.borrow();
                    ms.show_barcode_input
                };
                if typing_barcode {
                    if pressed {
                        let ctrl = barcode_ctrl_clone.load(Ordering::Relaxed);
                        match keycode {
                            winit::event::VirtualKeyCode::Key0 => barcode_insert_digit(&menu_state_clone, '0'),
                            winit::event::VirtualKeyCode::Key1 => barcode_insert_digit(&menu_state_clone, '1'),
                            winit::event::VirtualKeyCode::Key2 => barcode_insert_digit(&menu_state_clone, '2'),
                            winit::event::VirtualKeyCode::Key3 => barcode_insert_digit(&menu_state_clone, '3'),
                            winit::event::VirtualKeyCode::Key4 => barcode_insert_digit(&menu_state_clone, '4'),
                            winit::event::VirtualKeyCode::Key5 => barcode_insert_digit(&menu_state_clone, '5'),
                            winit::event::VirtualKeyCode::Key6 => barcode_insert_digit(&menu_state_clone, '6'),
                            winit::event::VirtualKeyCode::Key7 => barcode_insert_digit(&menu_state_clone, '7'),
                            winit::event::VirtualKeyCode::Key8 => barcode_insert_digit(&menu_state_clone, '8'),
                            winit::event::VirtualKeyCode::Key9 => barcode_insert_digit(&menu_state_clone, '9'),
                            winit::event::VirtualKeyCode::Numpad0 => barcode_insert_digit(&menu_state_clone, '0'),
                            winit::event::VirtualKeyCode::Numpad1 => barcode_insert_digit(&menu_state_clone, '1'),
                            winit::event::VirtualKeyCode::Numpad2 => barcode_insert_digit(&menu_state_clone, '2'),
                            winit::event::VirtualKeyCode::Numpad3 => barcode_insert_digit(&menu_state_clone, '3'),
                            winit::event::VirtualKeyCode::Numpad4 => barcode_insert_digit(&menu_state_clone, '4'),
                            winit::event::VirtualKeyCode::Numpad5 => barcode_insert_digit(&menu_state_clone, '5'),
                            winit::event::VirtualKeyCode::Numpad6 => barcode_insert_digit(&menu_state_clone, '6'),
                            winit::event::VirtualKeyCode::Numpad7 => barcode_insert_digit(&menu_state_clone, '7'),
                            winit::event::VirtualKeyCode::Numpad8 => barcode_insert_digit(&menu_state_clone, '8'),
                            winit::event::VirtualKeyCode::Numpad9 => barcode_insert_digit(&menu_state_clone, '9'),
                            winit::event::VirtualKeyCode::Back => barcode_backspace(&menu_state_clone),
                            winit::event::VirtualKeyCode::Delete => barcode_delete(&menu_state_clone),
                            winit::event::VirtualKeyCode::Left => barcode_move_caret(&menu_state_clone, -1, false),
                            winit::event::VirtualKeyCode::Right => barcode_move_caret(&menu_state_clone, 1, false),
                            winit::event::VirtualKeyCode::Home => barcode_home_end(&menu_state_clone, true),
                            winit::event::VirtualKeyCode::End => barcode_home_end(&menu_state_clone, false),
                            winit::event::VirtualKeyCode::A => {
                                if ctrl { barcode_select_all(&menu_state_clone); }
                            }
                            winit::event::VirtualKeyCode::V => {
                                if ctrl { barcode_paste(&menu_state_clone); }
                            }
                            winit::event::VirtualKeyCode::Return => {
                                let rc: Vec<u8> = menu_state_clone.borrow().barcode_input.bytes().collect();
                                let _ = cmd_tx.send(EmuCommand::SetBarcode(rc));
                                barcode_ctrl_clone.store(false, Ordering::Relaxed);
                                menu_state_clone.borrow_mut().show_barcode_input = false;
                                paused_clone.store(false, Ordering::Relaxed);
                            }
                            winit::event::VirtualKeyCode::Escape => {
                                barcode_ctrl_clone.store(false, Ordering::Relaxed);
                                menu_state_clone.borrow_mut().show_barcode_input = false;
                                paused_clone.store(false, Ordering::Relaxed);
                            }
                            _ => {}
                        }
                    }
                } else {
                let rebound = {
                    let ms = menu_state_clone.borrow();
                    if ms.rebind_controller.is_some() && ms.rebind_button.is_some() && pressed {
                        let ctrl = ms.rebind_controller.unwrap();
                        let btn = ms.rebind_button.unwrap();
                        drop(ms);
                        let mut ms_mut = menu_state_clone.borrow_mut();
                        ms_mut.rebind_controller = None;
                        ms_mut.rebind_button = None;
                        if ctrl == 0 {
                            if *expansion_type_clone.borrow() == config::ExpansionType::ArkanoidPaddle {
                                *expansion_paddle_button_binding_clone.borrow_mut() = key_str.clone();
                                config::save_paddle_button("expansion", &key_str);
                            } else if *expansion_type_clone.borrow() == config::ExpansionType::FamicomZapper {
                                *expansion_zapper_trigger_binding_clone.borrow_mut() = key_str.clone();
                                config::save_expansion_zapper_trigger(&key_str);
                            } else if *expansion_type_clone.borrow() == config::ExpansionType::OekaKidsTablet {
                                *expansion_oeka_click_binding_clone.borrow_mut() = key_str.clone();
                                config::save_expansion_oeka_click(&key_str);
                            } else if expansion_type_clone.borrow().is_family_trainer() {
                                family_trainer_bindings_clone.borrow_mut()[btn] = key_str.clone();
                                config::save_powerpad_binding("familytrainer", btn, &key_str);
                            } else if expansion_type_clone.borrow().is_hyper_shot() {
                                hyper_shot_bindings_clone.borrow_mut()[btn] = key_str.clone();
                                config::save_hyper_shot_binding("hypershot", btn, &key_str);
                            } else if expansion_type_clone.borrow().is_bandai_hyper_shot() {
                                bandai_hyper_bindings_clone.borrow_mut()[btn] = key_str.clone();
                                config::save_bandai_hyper_shot_binding("bandaihypershot", btn, &key_str);
                            } else if expansion_type_clone.borrow().is_family_basic() {
                                family_basic_bindings_clone.borrow_mut()[btn] = key_str.clone();
                                config::save_family_basic_binding("familybasic", btn, &key_str);
                            } else if expansion_type_clone.borrow().is_party_tap() {
                                party_tap_bindings_clone.borrow_mut()[btn] = key_str.clone();
                                config::save_party_tap_binding("partytap", btn, &key_str);
                            } else if expansion_type_clone.borrow().is_pachinko() {
                                pachinko_bindings_clone.borrow_mut()[btn] = key_str.clone();
                                config::save_pachinko_binding("pachinko", btn, &key_str);
                            } else if expansion_type_clone.borrow().is_exciting_boxing() {
                                punching_bag_bindings_clone.borrow_mut()[btn] = key_str.clone();
                                config::save_exciting_boxing_binding("punchingbag", btn, &key_str);
                            } else if expansion_type_clone.borrow().is_jissen_mahjong() {
                                jissen_mahjong_bindings_clone.borrow_mut()[btn] = key_str.clone();
                                config::save_jissen_mahjong_binding("jissenmahjong", btn, &key_str);
                            } else if expansion_type_clone.borrow().is_subor_keyboard() {
                                subor_keyboard_bindings_clone.borrow_mut()[btn] = key_str.clone();
                                config::save_subor_keyboard_binding("suborkeyboard", btn, &key_str);
                            }
                        } else if *expansion_adapter_type_clone.borrow() != config::ExpansionAdapterType::None
                            && ms_mut.show_expansion_settings {
                            let pair = ms_mut.adapter_pair.unwrap_or(0);
                            let pl = pair * 2 + (ctrl as usize).checked_sub(1).unwrap_or(0);
                            let port_bindings = match pl {
                                0 => Some(&mut *expansion1_bindings_clone.borrow_mut()),
                                1 => Some(&mut *expansion2_bindings_clone.borrow_mut()),
                                2 => Some(&mut *expansion3_bindings_clone.borrow_mut()),
                                _ => Some(&mut *expansion4_bindings_clone.borrow_mut()),
                            };
                            if let Some(bindings) = port_bindings {
                                bindings[btn] = key_str.clone();
                                config::save_expansion_adapter_binding(pl, btn, &key_str);
                            }
                        } else {
                        let prefix = if ctrl == 1 { "controller1" } else { "controller2" };
                        if ctrl == 1 {
                            let c1t = *controller1_type_clone.borrow();
                            if c1t == config::ControllerType::PowerPadA || c1t == config::ControllerType::PowerPadB {
                                powerpad1_bindings_clone.borrow_mut()[btn] = key_str.clone();
                                config::save_powerpad_binding(prefix, btn, &key_str);
                            } else if c1t == config::ControllerType::SNESPad {
                                snes1_bindings_clone.borrow_mut()[btn] = key_str.clone();
                                config::save_snes_binding(prefix, btn, &key_str);
                            } else if c1t == config::ControllerType::SNESMouse {
                                snes_mouse1_bindings_clone.borrow_mut()[btn] = key_str.clone();
                                config::save_snes_mouse_binding(prefix, btn, &key_str);
                            } else if c1t == config::ControllerType::SuborMouse {
                                subor_mouse1_bindings_clone.borrow_mut()[btn] = key_str.clone();
                                config::save_subor_mouse_binding(prefix, btn, &key_str);
                            } else if c1t == config::ControllerType::VirtualBoy {
                                let s = config::vb_serial_from_menu(btn);
                                vb1_bindings_clone.borrow_mut()[s] = key_str.clone();
                                config::save_vb_binding(prefix, s, &key_str);
                            } else if c1t == config::ControllerType::Paddle {
                                *paddle1_button_binding_clone.borrow_mut() = key_str.clone();
                                config::save_paddle_button("controller1", &key_str);
                            } else {
                                controller1_bindings_clone.borrow_mut()[btn] = key_str.clone();
                                config::save_binding(prefix, btn, &key_str);
                            }
                        } else if *controller2_type_clone.borrow() == config::ControllerType::Zapper {
                            *zapper_trigger_binding_clone.borrow_mut() = key_str.clone();
                            config::save_zapper_trigger(&key_str);
                        } else if *controller2_type_clone.borrow() == config::ControllerType::Paddle {
                            *paddle2_button_binding_clone.borrow_mut() = key_str.clone();
                            config::save_paddle_button("controller2", &key_str);
                        } else if *controller2_type_clone.borrow() == config::ControllerType::PowerPadA || *controller2_type_clone.borrow() == config::ControllerType::PowerPadB {
                            powerpad2_bindings_clone.borrow_mut()[btn] = key_str.clone();
                            config::save_powerpad_binding(prefix, btn, &key_str);
                        } else if *controller2_type_clone.borrow() == config::ControllerType::SNESPad {
                            snes2_bindings_clone.borrow_mut()[btn] = key_str.clone();
                            config::save_snes_binding(prefix, btn, &key_str);
                        } else if *controller2_type_clone.borrow() == config::ControllerType::SNESMouse {
                            snes_mouse2_bindings_clone.borrow_mut()[btn] = key_str.clone();
                            config::save_snes_mouse_binding(prefix, btn, &key_str);
                        } else if *controller2_type_clone.borrow() == config::ControllerType::SuborMouse {
                            subor_mouse2_bindings_clone.borrow_mut()[btn] = key_str.clone();
                            config::save_subor_mouse_binding(prefix, btn, &key_str);
                        } else if *controller2_type_clone.borrow() == config::ControllerType::VirtualBoy {
                            let s = config::vb_serial_from_menu(btn);
                            vb2_bindings_clone.borrow_mut()[s] = key_str.clone();
                            config::save_vb_binding(prefix, s, &key_str);
                        } else if *controller2_type_clone.borrow() == config::ControllerType::FamicomGamepad && btn == 10 {
                            *famicom_mic_binding_clone.borrow_mut() = key_str.clone();
                            config::save_famicom_mic(&key_str);
                        } else if ctrl == 2 {
                            controller2_bindings_clone.borrow_mut()[btn] = key_str.clone();
                            config::save_binding(prefix, btn, &key_str);
                        } else if ctrl == 3 {
                            controller3_bindings_clone.borrow_mut()[btn] = key_str.clone();
                            config::save_binding("controller3", btn, &key_str);
                        } else if ctrl == 4 {
                            controller4_bindings_clone.borrow_mut()[btn] = key_str.clone();
                            config::save_binding("controller4", btn, &key_str);
                        }
                        }
                        true
                    } else {
                        false
                    }
                };
                if !rebound {
                const BIT_MASKS: [u8; 10] = [0x80, 0x40, 0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01];
                // controller 1
                if *controller1_type_clone.borrow() != config::ControllerType::None {
                    let b1 = controller1_bindings_clone.borrow();
                    let mut any_match = false;
                    for (i, s) in b1.iter().enumerate() {
                        if s == &key_str {
                            any_match = true;
                            let mask = BIT_MASKS[i];
                            if pressed { controller_port1_clone.fetch_or(mask, Ordering::Relaxed); } else { controller_port1_clone.fetch_and(!mask, Ordering::Relaxed); }
                            if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                                if controller_port1_clone.load(Ordering::Relaxed) & 0x03 == 0x03 { controller_port1_clone.fetch_and(!mask, Ordering::Relaxed); }
                                if controller_port1_clone.load(Ordering::Relaxed) & 0x0C == 0x0C { controller_port1_clone.fetch_and(!mask, Ordering::Relaxed); }
                            }
                            break;
                        }
                    }
                    if any_match {  }
                }
                // controller 2
                if *controller2_type_clone.borrow() == config::ControllerType::Gamepad || *controller2_type_clone.borrow() == config::ControllerType::FamicomGamepad || *controller1_type_clone.borrow() == config::ControllerType::FourScore {
                    let b2 = controller2_bindings_clone.borrow();
                    for (i, s) in b2.iter().enumerate() {
                        if s == &key_str {
                            let mask = BIT_MASKS[i];
                            if pressed { controller_port2_clone.fetch_or(mask, Ordering::Relaxed); } else { controller_port2_clone.fetch_and(!mask, Ordering::Relaxed); }
                            if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                                if controller_port2_clone.load(Ordering::Relaxed) & 0x03 == 0x03 { controller_port2_clone.fetch_and(!mask, Ordering::Relaxed); }
                                if controller_port2_clone.load(Ordering::Relaxed) & 0x0C == 0x0C { controller_port2_clone.fetch_and(!mask, Ordering::Relaxed); }
                            }
                            break;
                        }
                    }
                }
                // famicom microphone
                if *controller2_type_clone.borrow() == config::ControllerType::FamicomGamepad {
                    let mb = famicom_mic_binding_clone.borrow();
                    if &*mb == &key_str {
                        if pressed { famicom_mic_clone.store(true, Ordering::Relaxed); } else { famicom_mic_clone.store(false, Ordering::Relaxed); }
                    }
                }
                // controller 3 and 4 (four score)
                if *controller1_type_clone.borrow() == config::ControllerType::FourScore || *controller2_type_clone.borrow() == config::ControllerType::FourScore {
                    let b3 = controller3_bindings_clone.borrow();
                    for (i, s) in b3.iter().enumerate() {
                        if s == &key_str {
                            let mask = BIT_MASKS[i];
                            if pressed { controller_port3_clone.fetch_or(mask, Ordering::Relaxed); } else { controller_port3_clone.fetch_and(!mask, Ordering::Relaxed); }
                            if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                                if controller_port3_clone.load(Ordering::Relaxed) & 0x03 == 0x03 { controller_port3_clone.fetch_and(!mask, Ordering::Relaxed); }
                                if controller_port3_clone.load(Ordering::Relaxed) & 0x0C == 0x0C { controller_port3_clone.fetch_and(!mask, Ordering::Relaxed); }
                            }
                            break;
                        }
                    }
                    let b4 = controller4_bindings_clone.borrow();
                    for (i, s) in b4.iter().enumerate() {
                        if s == &key_str {
                            let mask = BIT_MASKS[i];
                            if pressed { controller_port4_clone.fetch_or(mask, Ordering::Relaxed); } else { controller_port4_clone.fetch_and(!mask, Ordering::Relaxed); }
                            if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                                if controller_port4_clone.load(Ordering::Relaxed) & 0x03 == 0x03 { controller_port4_clone.fetch_and(!mask, Ordering::Relaxed); }
                                if controller_port4_clone.load(Ordering::Relaxed) & 0x0C == 0x0C { controller_port4_clone.fetch_and(!mask, Ordering::Relaxed); }
                            }
                            break;
                        }
                    }
                }
                // expansion adapters
                if *expansion_adapter_type_clone.borrow() != config::ExpansionAdapterType::None {
                    let exp_bindings = [
                        expansion1_bindings_clone.borrow(),
                        expansion2_bindings_clone.borrow(),
                        expansion3_bindings_clone.borrow(),
                        expansion4_bindings_clone.borrow(),
                    ];
                    let exp_ports = [
                        &expansion_adapter_port1_clone,
                        &expansion_adapter_port2_clone,
                        &expansion_adapter_port3_clone,
                        &expansion_adapter_port4_clone,
                    ];
                    for (pi, bindings) in exp_bindings.iter().enumerate() {
                        for (i, s) in bindings.iter().enumerate() {
                            if s == &key_str {
                                let mask = BIT_MASKS[i];
                                let port = exp_ports[pi];
                                if pressed { port.fetch_or(mask, Ordering::Relaxed); } else { port.fetch_and(!mask, Ordering::Relaxed); }
                                if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                                    if port.load(Ordering::Relaxed) & 0x03 == 0x03 { port.fetch_and(!mask, Ordering::Relaxed); }
                                    if port.load(Ordering::Relaxed) & 0x0C == 0x0C { port.fetch_and(!mask, Ordering::Relaxed); }
                                }
                                break;
                            }
                        }
                    }
                }
                // zapper trigger (also vs zapper trigger)
                {
                    let zt = zapper_trigger_binding_clone.borrow();
                    if *zt == key_str {
                        if pressed {
                            zapper_trigger_clone.store(true, Ordering::Relaxed);
                            zapper_bogo_clone.store(3, Ordering::Relaxed);
                        } else {
                            zapper_trigger_clone.store(false, Ordering::Relaxed);
                        }
                    }
                }
                // expansion zapper trigger
                {
                    let ezt = expansion_zapper_trigger_binding_clone.borrow();
                    if *ezt == key_str {
                        if pressed {
                            expansion_zapper_trigger_clone.store(true, Ordering::Relaxed);
                            zapper_bogo_clone.store(3, Ordering::Relaxed);
                        } else {
                            expansion_zapper_trigger_clone.store(false, Ordering::Relaxed);
                        }
                    }
                }
                // oeka kids tablet click
                {
                    let okt = expansion_oeka_click_binding_clone.borrow();
                    if *okt == key_str {
                        oeka_click_clone.store(pressed, Ordering::Relaxed);
                    }
                }
                // family trainer buttons
                if expansion_type_clone.borrow().is_family_trainer() {
                    let ft = family_trainer_bindings_clone.borrow();
                    let side_b = *expansion_type_clone.borrow() == config::ExpansionType::FamilyTrainerB;
                    let mut ft_lock = family_trainer_state_clone.lock().unwrap();
                    for (i, s) in ft.iter().enumerate() {
                        if s == &key_str {
                            let idx = if side_b { (i / 4) * 4 + (3 - (i % 4)) } else { i };
                            ft_lock[idx] = if pressed { 1 } else { 0 };
                        }
                    }
                }
                // konami hyper shot buttons
                if expansion_type_clone.borrow().is_hyper_shot() {
                    let hs = hyper_shot_bindings_clone.borrow();
                    let mut hs_lock = hyper_shot_state_clone.lock().unwrap();
                    for (i, s) in hs.iter().enumerate() {
                        if s == &key_str {
                            hs_lock[i] = if pressed { 1 } else { 0 };
                        }
                    }
                }
                // bandai hyper shot buttons
                if expansion_type_clone.borrow().is_bandai_hyper_shot() {
                    let bh = bandai_hyper_bindings_clone.borrow();
                    let mut bh_lock = bandai_hyper_buttons_clone.lock().unwrap();
                    for (i, s) in bh.iter().enumerate() {
                        if s == &key_str {
                            bh_lock[i] = if pressed { 1 } else { 0 };
                        }
                    }
                }
                // family basic keyboard buttons
                if expansion_type_clone.borrow().is_family_basic() {
                    let fb = family_basic_bindings_clone.borrow();
                    let mut fb_lock = family_basic_state_clone.lock().unwrap();
                    for (i, s) in fb.iter().enumerate() {
                        if s == &key_str {
                            fb_lock[i] = if pressed { 1 } else { 0 };
                        }
                    }
                }
                // party tap buttons
                if expansion_type_clone.borrow().is_party_tap() {
                    let pt = party_tap_bindings_clone.borrow();
                    let mut pt_lock = party_tap_state_clone.lock().unwrap();
                    for (i, s) in pt.iter().enumerate() {
                        if s == &key_str {
                            pt_lock[i] = if pressed { 1 } else { 0 };
                        }
                    }
                }
                // pachinko controller buttons
                if expansion_type_clone.borrow().is_pachinko() {
                    let pc = pachinko_bindings_clone.borrow();
                    let mut pc_lock = pachinko_state_clone.lock().unwrap();
                    for (i, s) in pc.iter().enumerate() {
                        if s == &key_str {
                            pc_lock[i] = if pressed { 1 } else { 0 };
                        }
                    }
                }
                // exciting boxing (punching bag) buttons
                if expansion_type_clone.borrow().is_exciting_boxing() {
                    let eb = punching_bag_bindings_clone.borrow();
                    let mut eb_lock = punching_bag_state_clone.lock().unwrap();
                    for (i, s) in eb.iter().enumerate() {
                        if s == &key_str {
                            eb_lock[i] = if pressed { 1 } else { 0 };
                        }
                    }
                }
                // jissen mahjong buttons
                if expansion_type_clone.borrow().is_jissen_mahjong() {
                    let jm = jissen_mahjong_bindings_clone.borrow();
                    let mut jm_lock = jissen_mahjong_state_clone.lock().unwrap();
                    for (i, s) in jm.iter().enumerate() {
                        if s == &key_str {
                            jm_lock[i] = if pressed { 1 } else { 0 };
                        }
                    }
                }
                // subor keyboard
                if expansion_type_clone.borrow().is_subor_keyboard() {
                    let sk = subor_keyboard_bindings_clone.borrow();
                    let mut sk_lock = subor_keyboard_state_clone.lock().unwrap();
                    for (i, s) in sk.iter().enumerate() {
                        if s == &key_str {
                            sk_lock[i] = if pressed { 1 } else { 0 };
                        }
                    }
                }
                // paddle button
                {
                    let pb = paddle1_button_binding_clone.borrow();
                    if *pb == key_str {
                        paddle_button_clone.lock().unwrap()[0] = pressed;
                    }
                }
                {
                    let pb = paddle2_button_binding_clone.borrow();
                    if *pb == key_str {
                        paddle_button_clone.lock().unwrap()[1] = pressed;
                    }
                }
                {
                    let pb = expansion_paddle_button_binding_clone.borrow();
                    if *pb == key_str {
                        paddle_button_clone.lock().unwrap()[2] = pressed;
                    }
                }
                // powerpad buttons
                {
                    let pp1 = powerpad1_bindings_clone.borrow();
                    for (i, s) in pp1.iter().enumerate() {
                        if s == &key_str {
                            let mut pp = powerpad_state_clone.lock().unwrap();
                            if pressed { pp[0] |= 1 << i; } else { pp[0] &= !(1 << i); }
                        }
                    }
                }
                {
                    let pp2 = powerpad2_bindings_clone.borrow();
                    for (i, s) in pp2.iter().enumerate() {
                        if s == &key_str {
                            let mut pp = powerpad_state_clone.lock().unwrap();
                            if pressed { pp[1] |= 1 << i; } else { pp[1] &= !(1 << i); }
                        }
                    }
                }
                // snes pad buttons
                {
                    let s1 = snes1_bindings_clone.borrow();
                    for (i, s) in s1.iter().enumerate() {
                        if s == &key_str {
                            let mut ss = snes_state_clone.lock().unwrap();
                            if pressed { ss[0] |= 1 << i; } else { ss[0] &= !(1 << i); }
                        }
                    }
                }
                {
                    let s2 = snes2_bindings_clone.borrow();
                    for (i, s) in s2.iter().enumerate() {
                        if s == &key_str {
                            let mut ss = snes_state_clone.lock().unwrap();
                            if pressed { ss[1] |= 1 << i; } else { ss[1] &= !(1 << i); }
                        }
                    }
                }
                // virtual boy pad buttons
                {
                    let v1 = vb1_bindings_clone.borrow();
                    for (i, s) in v1.iter().enumerate() {
                        if s == &key_str {
                            let mut vs = virtualboy_state_clone.lock().unwrap();
                            if pressed { vs[0] |= 1 << i; } else { vs[0] &= !(1 << i); }
                        }
                    }
                }
                {
                    let v2 = vb2_bindings_clone.borrow();
                    for (i, s) in v2.iter().enumerate() {
                        if s == &key_str {
                            let mut vs = virtualboy_state_clone.lock().unwrap();
                            if pressed { vs[1] |= 1 << i; } else { vs[1] &= !(1 << i); }
                        }
                    }
                }
                // snes mouse buttons
                {
                    let m1 = snes_mouse1_bindings_clone.borrow();
                    for (i, s) in m1.iter().enumerate() {
                        if s == &key_str {
                            let mut mb = snes_mouse_buttons_clone.lock().unwrap();
                            if pressed { mb[0] |= 1 << i; } else { mb[0] &= !(1 << i); }
                        }
                    }
                }
                {
                    let m2 = snes_mouse2_bindings_clone.borrow();
                    for (i, s) in m2.iter().enumerate() {
                        if s == &key_str {
                            let mut mb = snes_mouse_buttons_clone.lock().unwrap();
                            if pressed { mb[1] |= 1 << i; } else { mb[1] &= !(1 << i); }
                        }
                    }
                }
                // subor mouse buttons
                {
                    let m1 = subor_mouse1_bindings_clone.borrow();
                    for (i, s) in m1.iter().enumerate() {
                        if s == &key_str {
                            let mut sb = subor_mouse_buttons_clone.lock().unwrap();
                            if pressed { sb[0] |= 1 << i; } else { sb[0] &= !(1 << i); }
                        }
                    }
                }
                {
                    let m2 = subor_mouse2_bindings_clone.borrow();
                    for (i, s) in m2.iter().enumerate() {
                        if s == &key_str {
                            let mut sb = subor_mouse_buttons_clone.lock().unwrap();
                            if pressed { sb[1] |= 1 << i; } else { sb[1] &= !(1 << i); }
                        }
                    }
                }
                if pressed {
                    match keycode {
                        winit::event::VirtualKeyCode::P => {
                            paused_clone.store(!paused_clone.load(Ordering::Relaxed), Ordering::Relaxed);
                        }
                        winit::event::VirtualKeyCode::R => {
                            if *rom_loaded_clone.borrow() {
                                let _ = cmd_tx.send(EmuCommand::Reset);
                            }
                        }
                        _ => {}
                    }
                }
                }
                }
            }
            WinitEvent::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                let (mx, my) = (position.x as usize, position.y as usize);
                let mut ms = menu_state_clone.borrow_mut();
                ms.mouse_pos = (mx, my);
                ms.hovered_ctrl_button = None;
                ms.hovered_expansion_button = None;
                let sc = ms.scale;
                let ws = window.inner_size();
                let width = ws.width as usize;
                let height = ws.height as usize;
                if ms.show_barcode_input && ms.barcode_dragging {
                    let dlg_w = (320.0 * sc).round() as usize;
                    let dlg_h = (160.0 * sc).round() as usize;
                    let dlg_x = (width.saturating_sub(dlg_w)) / 2;
                    let dlg_y = (height.saturating_sub(dlg_h)) / 2;
                    let margin = (12.0 * sc).round() as usize;
                    let title_h = (26.0 * sc).round() as usize;
                    let field_x = dlg_x + margin;
                    let field_y = dlg_y + margin + title_h;
                    let field_h = (30.0 * sc).round() as usize;
                    if my >= field_y && my < field_y + field_h {
                        let caret = barcode_caret_from_x(mx, field_x, sc, ms.barcode_input.len());
                        ms.barcode_caret = caret;
                    }
                }
                if ms.show_controller1_settings {
                    let cw = (440.0 * sc).round() as usize;
                    let title_h = (30.0 * sc).round() as usize;
                    let btn_w = (90.0 * sc).round() as usize;
                    let btn_h = (26.0 * sc).round() as usize;
                    let gap_y = (8.0 * sc).round() as usize;
                    let gap_x = (cw.saturating_sub(2 * btn_w)) / 3;
                    let c1t = *controller1_type_clone.borrow();
                    let c2t = *controller2_type_clone.borrow();
                    let is_fs = c1t == config::ControllerType::FourScore || c2t == config::ControllerType::FourScore;
                    let fs_block_h = 4 * (btn_h + gap_y) + btn_h;
                    let tmp_g0 = title_h + (10.0 * sc).round() as usize;
                    let ch = if is_fs {
                        let fs_ch = tmp_g0 + 2 * (fs_block_h + btn_h / 2) + (40.0 * sc).round() as usize;
                        fs_ch.max(260)
                    } else {
                        (260.0 * sc).round() as usize
                    };
                    let cx = (width.saturating_sub(cw)) / 2;
                    let cy = (height.saturating_sub(ch)) / 2;
                    let grid_y0 = cy + tmp_g0;
                            if c1t == config::ControllerType::Zapper || c1t == config::ControllerType::Paddle {
                        let total_w = 2 * btn_w + gap_x;
                        let trig_bx = cx + (cw - total_w) / 2;
                        if point_in_rect(mx, my, trig_bx, grid_y0, total_w, btn_h) {
                            ms.hovered_ctrl_button = Some(0);
                        }
                    } else if c1t == config::ControllerType::SNESMouse || c1t == config::ControllerType::SuborMouse {
                        let per_w = btn_w;
                        let half_gap = (cw.saturating_sub(2 * per_w)) / 3;
                        for i in 0..config::SNES_MOUSE_BUTTON_COUNT {
                            let bx = cx + half_gap + i * (per_w + half_gap);
                            if point_in_rect(mx, my, bx, grid_y0, per_w, btn_h) {
                                ms.hovered_ctrl_button = Some(i);
                                break;
                            }
                        }
                    } else if c1t == config::ControllerType::PowerPadA || c1t == config::ControllerType::PowerPadB {
                        let cols = 4;
                        let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                        for i in 0..config::POWERPAD_BUTTON_COUNT {
                            let row = i / cols;
                            let col = i % cols;
                            let bx = cx + gap_x + col * (btn_w + gap_x);
                            let by = grid_y0 + row * (btn_h + gap_y);
                            if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                ms.hovered_ctrl_button = Some(i);
                                break;
                            }
                        }
                    } else if c1t == config::ControllerType::SNESPad {
                        let cols = 4;
                        let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                        for i in 0..config::SNES_BUTTON_COUNT {
                            let row = i / cols;
                            let col = i % cols;
                            let bx = cx + gap_x + col * (btn_w + gap_x);
                            let by = grid_y0 + row * (btn_h + gap_y);
                            if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                ms.hovered_ctrl_button = Some(i);
                                break;
                            }
                        }
                    } else if c1t == config::ControllerType::VirtualBoy {
                        let cols = 4;
                        let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                        for k in 0..config::VB_BUTTON_COUNT {
                            let row = k / cols;
                            let col = k % cols;
                            let bx = cx + gap_x + col * (btn_w + gap_x);
                            let by = grid_y0 + row * (btn_h + gap_y);
                            if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                ms.hovered_ctrl_button = Some(k);
                                break;
                            }
                        }
                    } else if c1t == config::ControllerType::FourScore || *controller2_type_clone.borrow() == config::ControllerType::FourScore {
                        let grid_h = 4 * (btn_h + gap_y) + btn_h;
                        for player in 0..2usize {
                            let yoff = grid_y0 + player * (grid_h + btn_h / 2);
                            let btn_y0 = yoff + (btn_h * 3 / 4) as usize;
                            for i in 0..config::GAMEPAD_BUTTON_COUNT {
                                let row = i / 2;
                                let by = btn_y0 + row * (btn_h + gap_y);
                                let bx = if i % 2 == 0 { cx + gap_x } else { cx + gap_x * 2 + btn_w };
                                if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                    ms.hovered_ctrl_button = Some(i * 2 + player);
                                    break;
                                }
                            }
                            if ms.hovered_ctrl_button.is_some() { break; }
                        }
                    } else {
                        for i in 0..config::GAMEPAD_BUTTON_COUNT {
                            let row = i / 2;
                            let by = grid_y0 + row * (btn_h + gap_y);
                            let bx = if i % 2 == 0 { cx + gap_x } else { cx + gap_x * 2 + btn_w };
                            if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                ms.hovered_ctrl_button = Some(i);
                                break;
                            }
                        }
                    }
                } else if ms.show_controller2_settings {
                    let cw = (440.0 * sc).round() as usize;
                    let title_h = (30.0 * sc).round() as usize;
                    let btn_w = (90.0 * sc).round() as usize;
                    let btn_h = (26.0 * sc).round() as usize;
                    let gap_y = (8.0 * sc).round() as usize;
                    let gap_x = (cw.saturating_sub(2 * btn_w)) / 3;
                    let c2t = *controller2_type_clone.borrow();
                    let is_fs = c2t == config::ControllerType::FourScore;
                    let fs_block_h = 4 * (btn_h + gap_y) + btn_h;
                    let tmp_g0 = title_h + (10.0 * sc).round() as usize;
                    let ch = if is_fs {
                        let fs_ch = tmp_g0 + 2 * (fs_block_h + btn_h / 2) + (40.0 * sc).round() as usize;
                        fs_ch.max(260)
                    } else {
                        (260.0 * sc).round() as usize
                    };
                    let cx = (width.saturating_sub(cw)) / 2;
                    let cy = (height.saturating_sub(ch)) / 2;
                    let grid_y0 = cy + tmp_g0;
                    if c2t == config::ControllerType::Zapper || c2t == config::ControllerType::Paddle {
                        let total_w = 2 * btn_w + gap_x;
                        let trig_bx = cx + (cw - total_w) / 2;
                        if point_in_rect(mx, my, trig_bx, grid_y0, total_w, btn_h) {
                            ms.hovered_ctrl_button = Some(0);
                        }
                    } else if c2t == config::ControllerType::SNESMouse || c2t == config::ControllerType::SuborMouse {
                        let per_w = btn_w;
                        let half_gap = (cw.saturating_sub(2 * per_w)) / 3;
                        for i in 0..config::SNES_MOUSE_BUTTON_COUNT {
                            let bx = cx + half_gap + i * (per_w + half_gap);
                            if point_in_rect(mx, my, bx, grid_y0, per_w, btn_h) {
                                ms.hovered_ctrl_button = Some(i);
                                break;
                            }
                        }
                    } else if c2t == config::ControllerType::PowerPadA || c2t == config::ControllerType::PowerPadB {
                        let cols = 4;
                        let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                        for i in 0..config::POWERPAD_BUTTON_COUNT {
                            let row = i / cols;
                            let col = i % cols;
                            let bx = cx + gap_x + col * (btn_w + gap_x);
                            let by = grid_y0 + row * (btn_h + gap_y);
                            if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                ms.hovered_ctrl_button = Some(i);
                                break;
                            }
                        }
                    } else if c2t == config::ControllerType::SNESPad {
                        let cols = 4;
                        let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                        for i in 0..config::SNES_BUTTON_COUNT {
                            let row = i / cols;
                            let col = i % cols;
                            let bx = cx + gap_x + col * (btn_w + gap_x);
                            let by = grid_y0 + row * (btn_h + gap_y);
                            if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                ms.hovered_ctrl_button = Some(i);
                                break;
                            }
                        }
                    } else if c2t == config::ControllerType::VirtualBoy {
                        let cols = 4;
                        let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                        for k in 0..config::VB_BUTTON_COUNT {
                            let row = k / cols;
                            let col = k % cols;
                            let bx = cx + gap_x + col * (btn_w + gap_x);
                            let by = grid_y0 + row * (btn_h + gap_y);
                            if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                ms.hovered_ctrl_button = Some(k);
                                break;
                            }
                        }
                    } else if c2t == config::ControllerType::FourScore {
                        let block_h = 4 * (btn_h + gap_y) + btn_h;
                        for player in 0..2usize {
                            let yoff = grid_y0 + player * (block_h + btn_h / 2);
                            let btn_y0 = yoff + (btn_h * 3 / 4) as usize;
                            for i in 0..config::GAMEPAD_BUTTON_COUNT {
                                let row = i / 2;
                                let by = btn_y0 + row * (btn_h + gap_y);
                                let bx = if i % 2 == 0 { cx + gap_x } else { cx + gap_x * 2 + btn_w };
                                if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                    ms.hovered_ctrl_button = Some(i * 4 + (player + 2));
                                    break;
                                }
                            }
                            if ms.hovered_ctrl_button.is_some() { break; }
                        }
                    } else {
                        for i in 0..config::GAMEPAD_BUTTON_COUNT {
                            let row = i / 2;
                            let by = grid_y0 + row * (btn_h + gap_y);
                            let bx = if i % 2 == 0 { cx + gap_x } else { cx + gap_x * 2 + btn_w };
                            if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                ms.hovered_ctrl_button = Some(i);
                                break;
                            }
                        }
                        if c2t == config::ControllerType::FamicomGamepad && ms.hovered_ctrl_button.is_none() {
                            let mic_row = 5;
                            let mic_y = grid_y0 + mic_row * (btn_h + gap_y);
                            let mic_x = cx + (cw - btn_w) / 2;
                            if point_in_rect(mx, my, mic_x, mic_y, btn_w, btn_h) {
                                ms.hovered_ctrl_button = Some(10);
                            }
                        }
                    }
                    } else if ms.show_expansion_settings {
                        let is_adapter = *expansion_adapter_type_clone.borrow() != config::ExpansionAdapterType::None;
                        if is_adapter {
                            let pair = ms.adapter_pair.unwrap_or(0);
                            let player_offset = pair * 2;
                            let sw = (440.0 * sc).round() as usize;
                            let btn_w = (90.0 * sc).round() as usize;
                            let btn_h = (26.0 * sc).round() as usize;
                            let gap_y = (8.0 * sc).round() as usize;
                            let gap_x = (sw.saturating_sub(2 * btn_w)) / 3;
                            let grid_h = 4 * (btn_h + gap_y) + btn_h;
                            let tmp_g0 = (30.0 * sc).round() as usize + (10.0 * sc).round() as usize;
                            let sh = (tmp_g0 + 2 * (grid_h + btn_h / 2) + (40.0 * sc).round() as usize).max(260);
                            let sx = (width.saturating_sub(sw)) / 2;
                            let sy = (height.saturating_sub(sh)) / 2;
                            let grid_y0 = sy + tmp_g0;
                            'outer2: for local in 0..2 {
                                let yoff = grid_y0 + local * (grid_h + btn_h / 2);
                                let btn_y0 = yoff + (btn_h * 3 / 4) as usize;
                                for i in 0..config::GAMEPAD_BUTTON_COUNT {
                                    let row = i / 2;
                                    let by = btn_y0 + row * (btn_h + gap_y);
                                    let bx = if i % 2 == 0 { sx + gap_x } else { sx + gap_x * 2 + btn_w };
                                    if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                        ms.hovered_expansion_button = Some(local * config::GAMEPAD_BUTTON_COUNT + i);
                                        break 'outer2;
                                    }
                                }
                            }
                            let _ = player_offset;
                        } else {
                            let sw = (440.0 * sc).round() as usize;
                            let title_h = (30.0 * sc).round() as usize;
                            let btn_w = (90.0 * sc).round() as usize;
                            let btn_h = (26.0 * sc).round() as usize;
                            let gap_x = (sw.saturating_sub(2 * btn_w)) / 3;
                            let is_family_trainer = expansion_type_clone.borrow().is_family_trainer();
                            let is_hyper_shot = expansion_type_clone.borrow().is_hyper_shot();
                            let is_family_basic = expansion_type_clone.borrow().is_family_basic();
                            let is_party_tap = expansion_type_clone.borrow().is_party_tap();
                            let is_pachinko = expansion_type_clone.borrow().is_pachinko();
                            let is_exciting_boxing = expansion_type_clone.borrow().is_exciting_boxing();
                            let is_jissen_mahjong = expansion_type_clone.borrow().is_jissen_mahjong();
                            let is_subor_keyboard = expansion_type_clone.borrow().is_subor_keyboard();
                            let is_bandai_hyper_shot = expansion_type_clone.borrow().is_bandai_hyper_shot();
                            let sh = if is_family_trainer {
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let act_btn_h = (24.0 * sc).round() as usize;
                                (title_h + (10.0 * sc).round() as usize + btn_h / 2 + 3 * mbtn_h + 2 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h + (20.0 * sc).round() as usize) as usize
                            } else if is_hyper_shot {
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let act_btn_h = (24.0 * sc).round() as usize;
                                (title_h + (10.0 * sc).round() as usize + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h + (20.0 * sc).round() as usize) as usize
                            } else if is_bandai_hyper_shot {
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let act_btn_h = (24.0 * sc).round() as usize;
                                (title_h + (10.0 * sc).round() as usize + btn_h / 2 + 3 * mbtn_h + 2 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h + (20.0 * sc).round() as usize) as usize
                            } else if is_family_basic {
                                let mbtn_h = (20.0 * sc).round() as usize;
                                let mrow_gap = (4.0 * sc).round() as usize;
                                let act_btn_h = (24.0 * sc).round() as usize;
                                (title_h + (10.0 * sc).round() as usize + btn_h / 2 + 9 * mbtn_h + 8 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h + (20.0 * sc).round() as usize) as usize
                            } else if is_party_tap {
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let act_btn_h = (24.0 * sc).round() as usize;
                                (title_h + (10.0 * sc).round() as usize + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h + (20.0 * sc).round() as usize) as usize
                            } else if is_pachinko {
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let act_btn_h = (24.0 * sc).round() as usize;
                                (title_h + (10.0 * sc).round() as usize + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h + (20.0 * sc).round() as usize) as usize
                            } else if is_exciting_boxing {
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let act_btn_h = (24.0 * sc).round() as usize;
                                (title_h + (10.0 * sc).round() as usize + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h + (20.0 * sc).round() as usize) as usize
                            } else if is_jissen_mahjong {
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let act_btn_h = (24.0 * sc).round() as usize;
                                (title_h + (10.0 * sc).round() as usize + btn_h / 2 + 7 * mbtn_h + 6 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h + (20.0 * sc).round() as usize) as usize
                            } else if is_subor_keyboard {
                                let mbtn_h = (20.0 * sc).round() as usize;
                                let mrow_gap = (4.0 * sc).round() as usize;
                                let act_btn_h = (24.0 * sc).round() as usize;
                                (title_h + (10.0 * sc).round() as usize + btn_h / 2 + 13 * mbtn_h + 12 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h + (20.0 * sc).round() as usize) as usize
                            } else {
                                (150.0 * sc).round() as usize
                            };
                            let sx = (width.saturating_sub(sw)) / 2;
                            let sy = (height.saturating_sub(sh)) / 2;
                            let grid_y0 = sy + title_h + (10.0 * sc).round() as usize;
                            if is_family_trainer {
                                let btn_gap = (6.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(5 * btn_gap)) / 4;
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                'outer3: for r in 0..3usize {
                                    for c in 0..4usize {
                                        let i = r * 4 + c;
                                        let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                        let by = m_y0 + r * (mbtn_h + mrow_gap);
                                        if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                            ms.hovered_expansion_button = Some(i);
                                            break 'outer3;
                                        }
                                    }
                                }
                            } else if is_hyper_shot {
                                let btn_gap = (6.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(3 * btn_gap)) / 2;
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                'outer3: for r in 0..2usize {
                                    for c in 0..2usize {
                                        let i = r * 2 + c;
                                        let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                        let by = m_y0 + r * (mbtn_h + mrow_gap);
                                        if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                            ms.hovered_expansion_button = Some(i);
                                            break 'outer3;
                                        }
                                    }
                                }
                            } else if is_bandai_hyper_shot {
                                let btn_gap = (6.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(5 * btn_gap)) / 4;
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                'outer3: for r in 0..3usize {
                                    for c in 0..4usize {
                                        let i = r * 4 + c;
                                        if i >= config::BANDAI_HYPER_SHOT_BUTTON_COUNT { continue; }
                                        let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                        let by = m_y0 + r * (mbtn_h + mrow_gap);
                                        if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                            ms.hovered_expansion_button = Some(i);
                                            break 'outer3;
                                        }
                                    }
                                }
                            } else if is_family_basic {
                                let btn_gap = (4.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(9 * btn_gap)) / 8;
                                let mbtn_h = (20.0 * sc).round() as usize;
                                let mrow_gap = (4.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                'outer3: for r in 0..9usize {
                                    for c in 0..8usize {
                                        let i = r * 8 + c;
                                        if i >= config::FAMILY_BASIC_BUTTON_COUNT { continue; }
                                        let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                        let by = m_y0 + r * (mbtn_h + mrow_gap);
                                        if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                            ms.hovered_expansion_button = Some(i);
                                            break 'outer3;
                                        }
                                    }
                                }
                            } else if is_party_tap {
                                let btn_gap = (6.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(4 * btn_gap)) / 3;
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                'outer3: for r in 0..2usize {
                                    for c in 0..3usize {
                                        let i = r * 3 + c;
                                        let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                        let by = m_y0 + r * (mbtn_h + mrow_gap);
                                        if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                            ms.hovered_expansion_button = Some(i);
                                            break 'outer3;
                                        }
                                    }
                                }
                            } else if is_pachinko {
                                let btn_gap = (6.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(3 * btn_gap)) / 2;
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                for i in 0..config::PACHINKO_BUTTON_COUNT {
                                    let bx = sx + btn_gap + i * (mcol_w + btn_gap);
                                    let by = m_y0;
                                    if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                        ms.hovered_expansion_button = Some(i);
                                        break;
                                    }
                                }
                            } else if is_exciting_boxing {
                                let btn_gap = (6.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(5 * btn_gap)) / 4;
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                'outer3: for r in 0..2usize {
                                    for c in 0..4usize {
                                        let i = r * 4 + c;
                                        let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                        let by = m_y0 + r * (mbtn_h + mrow_gap);
                                        if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                            ms.hovered_expansion_button = Some(i);
                                            break 'outer3;
                                        }
                                    }
                                }
                            } else if is_jissen_mahjong {
                                let btn_gap = (6.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(4 * btn_gap)) / 3;
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                'outer3: for r in 0..7usize {
                                    for c in 0..3usize {
                                        let i = r * 3 + c;
                                        let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                        let by = m_y0 + r * (mbtn_h + mrow_gap);
                                        if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                            ms.hovered_expansion_button = Some(i);
                                            break 'outer3;
                                        }
                                    }
                                }
                            } else if is_subor_keyboard {
                                let btn_gap = (4.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(9 * btn_gap)) / 8;
                                let mbtn_h = (20.0 * sc).round() as usize;
                                let mrow_gap = (4.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                'outer3: for r in 0..13usize {
                                    for c in 0..8usize {
                                        let i = r * 8 + c;
                                        if i >= config::SUBOR_KEYBOARD_BUTTON_COUNT { continue; }
                                        let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                        let by = m_y0 + r * (mbtn_h + mrow_gap);
                                        if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                            ms.hovered_expansion_button = Some(i);
                                            break 'outer3;
                                        }
                                    }
                                }
                            } else {
                                let bind_total = 2 * btn_w + gap_x;
                                let bind_bx = sx + (sw.saturating_sub(bind_total)) / 2;
                                if point_in_rect(mx, my, bind_bx, grid_y0, bind_total, btn_h) {
                                    ms.hovered_expansion_button = Some(0);
                                }
                            }
                        }
                    }

                // audio slider
                let drag_chan = ms.dragging_audio_slider;
                drop(ms);
                if let Some(drag_chan) = drag_chan {
                    let channels = active_audio_channels(0);
                    if let Some(ch_row) = channels.iter().position(|&(idx, _)| idx == drag_chan) {
                        let aw = (400.0 * sc).round() as usize;
                        let gap = (8.0 * sc).round() as usize;
                        let title_h = (30.0 * sc).round() as usize;
                        let row_h = (22.0 * sc).round() as usize;
                        let slider_w = (120.0 * sc).round() as usize;
                        let border_thickness = (2.0 * sc).round() as usize;
                        let content_rows = 3 + channels.len();
                        let ah = title_h + gap + content_rows * (row_h + gap) - gap + border_thickness * 2;
                        let ax = (width.saturating_sub(aw)) / 2;
                        let ay = (height.saturating_sub(ah)) / 2;
                        let slider_x = ax + aw - (15.0 * sc).round() as usize - slider_w;
                        let slider_start = ay + title_h + gap + 4 * (row_h + gap);
                        let drag_row_y = slider_start + ch_row * (row_h + gap);
                        if my >= drag_row_y && my < drag_row_y + row_h {
                            let rel_x = mx.saturating_sub(slider_x);
                            let pct = ((rel_x as f32 / slider_w as f32) * 100.0).round().min(100.0).max(0.0) as u8;
                            let mut vols = channel_volumes_clone.borrow_mut();
                            vols[drag_chan] = pct;
                            config::save_channel_volume(config::CHANNEL_NAMES[drag_chan], pct);
                            let vol_f32 = pct as f32 / 100.0;
                            let mut e = emu_clone.lock().unwrap();
                            match drag_chan {
                                0 => e.master_volume = vol_f32,
                                1 => e.triangle_volume = vol_f32,
                                2 => e.square1_volume = vol_f32,
                                3 => e.square2_volume = vol_f32,
                                4 => e.noise_volume = vol_f32,
                                _ => e.pcm_volume = vol_f32,
                            }
                        }
                    }
                }

                // update zapper position for light gun
                let (sd_x, sd_y, sd_w, sd_h) = {
                    let msr = menu_state_clone.borrow();
                    (msr.screen_dest_x, msr.screen_dest_y, msr.screen_dest_w, msr.screen_dest_h)
                };
                if sd_w > 0 && mx >= sd_x && mx < sd_x + sd_w && my >= sd_y && my < sd_y + sd_h {
                    let rel_x = mx - sd_x;
                    let rel_y = my - sd_y;
                    let nes_x = (rel_x * 256) / sd_w;
                    let nes_y = (rel_y * 240) / sd_h;
                    *zapper_x_clone.lock().unwrap() = nes_x as f32 / 255.0;
                    *zapper_y_clone.lock().unwrap() = nes_y as f32 / 239.0;
                    // paddle position from mouse x
                    let raw = 98u16 + (nes_x as u16 * 144 / 240);
                    let px = raw.min(242) as u8;
                    {
                        let mut pxv = paddle_x_clone.lock().unwrap();
                        pxv[0] = !px;
                        pxv[1] = !px;
                        pxv[2] = !px;
                    }
                }
                // snes mouse delta tracking (track even outside NES screen)
                let lx = *last_mouse_x_clone.borrow();
                let ly = *last_mouse_y_clone.borrow();
                if lx != 0.0 || ly != 0.0 {
                    let dx = position.x - lx;
                    let dy = position.y - ly;
                    if dx.abs() < 500.0 && dy.abs() < 500.0 {
                        {
                            let mut sdx = snes_mouse_delta_x_clone.lock().unwrap();
                            let mut sdy = snes_mouse_delta_y_clone.lock().unwrap();
                            sdx[0] += (dx as f32) / 4.0;
                            sdy[0] += (dy as f32) / 4.0;
                            sdx[1] += (dx as f32) / 4.0;
                            sdy[1] += (dy as f32) / 4.0;
                        }
                        // subor mouse: raw deltas
                        let dxi = dx as i32;
                        let dyi = dy as i32;
                        {
                            let mut sdx = subor_mouse_dx_clone.lock().unwrap();
                            let mut sdy = subor_mouse_dy_clone.lock().unwrap();
                            sdx[0] = sdx[0].saturating_add(dxi).clamp(-32, 32);
                            sdy[0] = sdy[0].saturating_add(dyi).clamp(-32, 32);
                            sdx[1] = sdx[1].saturating_add(dxi).clamp(-32, 32);
                            sdy[1] = sdy[1].saturating_add(dyi).clamp(-32, 32);
                        }
                        // hori track: raw deltas
                        {
                            let mut hdx = hori_track_dx_clone.lock().unwrap();
                            let mut hdy = hori_track_dy_clone.lock().unwrap();
                            hdx[0] += dx as f32;
                            hdy[0] += dy as f32;
                            hdx[1] += dx as f32;
                            hdy[1] += dy as f32;
                        }
                    }
                }
                *last_mouse_x_clone.borrow_mut() = position.x;
                *last_mouse_y_clone.borrow_mut() = position.y;
            }
            WinitEvent::WindowEvent {
                event: WindowEvent::CursorLeft { .. },
                ..
            } => {
                let mut ms = menu_state_clone.borrow_mut();
                ms.hovered_menu = None;
                ms.hovered_file_item = None;
                ms.hovered_nes_item = None;
                ms.hovered_help_index = None;
                ms.hovered_region_item = None;
                ms.hovered_region_index = None;
                ms.hovered_recent_index = None;
                ms.hovered_options_index = None;
                ms.hovered_save_slot = None;
                ms.hovered_load_slot = None;
                ms.hovered_ctrl_button = None;
                ms.dragging_audio_slider = None;
            }
            WinitEvent::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                let pressed = state == winit::event::ElementState::Pressed;
                let btn_str = mouse_button_str(&button);
                if pressed {
                    let rebound = {
                        let ms = menu_state_clone.borrow();
                        if ms.rebind_controller.is_some() && ms.rebind_button.is_some() {
                            let ctrl = ms.rebind_controller.unwrap();
                            let b = ms.rebind_button.unwrap();
                            drop(ms);
                            let mut ms_mut = menu_state_clone.borrow_mut();
                            ms_mut.rebind_controller = None;
                            ms_mut.rebind_button = None;
                            if ctrl == 0 {
                                if *expansion_type_clone.borrow() == config::ExpansionType::ArkanoidPaddle {
                                    *expansion_paddle_button_binding_clone.borrow_mut() = btn_str.clone();
                                    config::save_paddle_button("expansion", &btn_str);
                                } else if *expansion_type_clone.borrow() == config::ExpansionType::FamicomZapper {
                                    *expansion_zapper_trigger_binding_clone.borrow_mut() = btn_str.clone();
                                    config::save_expansion_zapper_trigger(&btn_str);
                                } else if *expansion_type_clone.borrow() == config::ExpansionType::OekaKidsTablet {
                                    *expansion_oeka_click_binding_clone.borrow_mut() = btn_str.clone();
                                    config::save_expansion_oeka_click(&btn_str);
                                } else if expansion_type_clone.borrow().is_family_trainer() {
                                    family_trainer_bindings_clone.borrow_mut()[b] = btn_str.clone();
                                    config::save_powerpad_binding("familytrainer", b, &btn_str);
                                } else if expansion_type_clone.borrow().is_hyper_shot() {
                                    hyper_shot_bindings_clone.borrow_mut()[b] = btn_str.clone();
                                    config::save_hyper_shot_binding("hypershot", b, &btn_str);
                                } else if expansion_type_clone.borrow().is_bandai_hyper_shot() {
                                    bandai_hyper_bindings_clone.borrow_mut()[b] = btn_str.clone();
                                    config::save_bandai_hyper_shot_binding("bandaihypershot", b, &btn_str);
                                } else if expansion_type_clone.borrow().is_family_basic() {
                                    family_basic_bindings_clone.borrow_mut()[b] = btn_str.clone();
                                    config::save_family_basic_binding("familybasic", b, &btn_str);
                                } else if expansion_type_clone.borrow().is_party_tap() {
                                    party_tap_bindings_clone.borrow_mut()[b] = btn_str.clone();
                                    config::save_party_tap_binding("partytap", b, &btn_str);
                                } else if expansion_type_clone.borrow().is_pachinko() {
                                    pachinko_bindings_clone.borrow_mut()[b] = btn_str.clone();
                                    config::save_pachinko_binding("pachinko", b, &btn_str);
                                } else if expansion_type_clone.borrow().is_exciting_boxing() {
                                    punching_bag_bindings_clone.borrow_mut()[b] = btn_str.clone();
                                    config::save_exciting_boxing_binding("punchingbag", b, &btn_str);
                                } else if expansion_type_clone.borrow().is_jissen_mahjong() {
                                    jissen_mahjong_bindings_clone.borrow_mut()[b] = btn_str.clone();
                                    config::save_jissen_mahjong_binding("jissenmahjong", b, &btn_str);
                                } else if expansion_type_clone.borrow().is_subor_keyboard() {
                                    subor_keyboard_bindings_clone.borrow_mut()[b] = btn_str.clone();
                                    config::save_subor_keyboard_binding("suborkeyboard", b, &btn_str);
                                }
                            } else if *expansion_adapter_type_clone.borrow() != config::ExpansionAdapterType::None
                                && ms_mut.show_expansion_settings {
                                let pair = ms_mut.adapter_pair.unwrap_or(0);
                                let pl = pair * 2 + (ctrl as usize).checked_sub(1).unwrap_or(0);
                                let port_bindings = match pl {
                                    0 => Some(&mut *expansion1_bindings_clone.borrow_mut()),
                                    1 => Some(&mut *expansion2_bindings_clone.borrow_mut()),
                                    2 => Some(&mut *expansion3_bindings_clone.borrow_mut()),
                                    _ => Some(&mut *expansion4_bindings_clone.borrow_mut()),
                                };
                                if let Some(bindings) = port_bindings {
                                    bindings[b] = btn_str.clone();
                                    config::save_expansion_adapter_binding(pl, b, &btn_str);
                                }
                            } else {
                            let prefix = if ctrl == 1 { "controller1" } else { "controller2" };
                            let bs = btn_str.clone();
                            if ctrl == 1 {
                                let c1t = *controller1_type_clone.borrow();
                                if c1t == config::ControllerType::PowerPadA || c1t == config::ControllerType::PowerPadB {
                                    powerpad1_bindings_clone.borrow_mut()[b] = bs;
                                    config::save_powerpad_binding(prefix, b, &btn_str);
                                } else if c1t == config::ControllerType::SNESPad {
                                    snes1_bindings_clone.borrow_mut()[b] = bs;
                                    config::save_snes_binding(prefix, b, &btn_str);
                            } else if c1t == config::ControllerType::SNESMouse {
                                snes_mouse1_bindings_clone.borrow_mut()[b] = bs;
                                config::save_snes_mouse_binding(prefix, b, &btn_str);
                            } else if c1t == config::ControllerType::SuborMouse {
                                subor_mouse1_bindings_clone.borrow_mut()[b] = bs;
                                config::save_subor_mouse_binding(prefix, b, &btn_str);
                            } else if c1t == config::ControllerType::VirtualBoy {
                                let s = config::vb_serial_from_menu(b);
                                vb1_bindings_clone.borrow_mut()[s] = bs;
                                config::save_vb_binding(prefix, s, &btn_str);
                            } else if c1t == config::ControllerType::Paddle {
                                *paddle1_button_binding_clone.borrow_mut() = bs;
                                config::save_paddle_button("controller1", &btn_str);
                            } else {
                                controller1_bindings_clone.borrow_mut()[b] = bs;
                                config::save_binding(prefix, b, &btn_str);
                            }
                        } else if *controller2_type_clone.borrow() == config::ControllerType::Zapper {
                                *zapper_trigger_binding_clone.borrow_mut() = bs;
                                config::save_zapper_trigger(&btn_str);
                            } else if *controller2_type_clone.borrow() == config::ControllerType::Paddle {
                                *paddle2_button_binding_clone.borrow_mut() = bs;
                                config::save_paddle_button("controller2", &btn_str);
                            } else if *controller2_type_clone.borrow() == config::ControllerType::PowerPadA || *controller2_type_clone.borrow() == config::ControllerType::PowerPadB {
                                powerpad2_bindings_clone.borrow_mut()[b] = bs;
                                config::save_powerpad_binding(prefix, b, &btn_str);
                            } else if *controller2_type_clone.borrow() == config::ControllerType::SNESPad {
                                snes2_bindings_clone.borrow_mut()[b] = bs;
                                config::save_snes_binding(prefix, b, &btn_str);
                            } else if *controller2_type_clone.borrow() == config::ControllerType::SNESMouse {
                                snes_mouse2_bindings_clone.borrow_mut()[b] = bs;
                                config::save_snes_mouse_binding(prefix, b, &btn_str);
                            } else if *controller2_type_clone.borrow() == config::ControllerType::SuborMouse {
                                subor_mouse2_bindings_clone.borrow_mut()[b] = bs;
                                config::save_subor_mouse_binding(prefix, b, &btn_str);
                            } else if *controller2_type_clone.borrow() == config::ControllerType::VirtualBoy {
                                let s = config::vb_serial_from_menu(b);
                                vb2_bindings_clone.borrow_mut()[s] = bs;
                                config::save_vb_binding(prefix, s, &btn_str);
                            } else if *controller2_type_clone.borrow() == config::ControllerType::FamicomGamepad && b == 10 {
                                *famicom_mic_binding_clone.borrow_mut() = bs;
                                config::save_famicom_mic(&btn_str);
                            } else if ctrl == 2 {
                                controller2_bindings_clone.borrow_mut()[b] = bs;
                                config::save_binding(prefix, b, &btn_str);
                            } else if ctrl == 3 {
                                controller3_bindings_clone.borrow_mut()[b] = bs;
                                config::save_binding("controller3", b, &btn_str);
                            } else if ctrl == 4 {
                                controller4_bindings_clone.borrow_mut()[b] = bs;
                                config::save_binding("controller4", b, &btn_str);
                            }
                            }
                            true
                        } else {
                            false
                        }
                    };
                    if rebound { return; }
                }
                let is_modal_open = {
                    let ms = menu_state_clone.borrow();
                    ms.show_dip_switches || ms.show_general_settings || ms.show_audio_settings
                        || ms.show_video_settings || ms.show_input_settings
                        || ms.show_controller1_settings || ms.show_controller2_settings
                        || ms.show_expansion_settings
                        || ms.show_about || ms.show_error || ms.show_confirm_exit_dialog
                        || ms.show_barcode_input
                };
                if !is_modal_open {
                    const BIT_MASKS: [u8; 10] = [0x80, 0x40, 0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01];
                    if *controller1_type_clone.borrow() != config::ControllerType::None {
                        let b1 = controller1_bindings_clone.borrow();
                        for (i, s) in b1.iter().enumerate() {
                            if s == &btn_str {
                                let mask = BIT_MASKS[i];
                                if pressed { controller_port1_clone.fetch_or(mask, Ordering::Relaxed); } else { controller_port1_clone.fetch_and(!mask, Ordering::Relaxed); }
                                if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                                    if controller_port1_clone.load(Ordering::Relaxed) & 0x03 == 0x03 { controller_port1_clone.fetch_and(!mask, Ordering::Relaxed); }
                                    if controller_port1_clone.load(Ordering::Relaxed) & 0x0C == 0x0C { controller_port1_clone.fetch_and(!mask, Ordering::Relaxed); }
                                }
                                break;
                            }
                        }
                    }
                // controller 3 and 4 (four score)
                if *controller1_type_clone.borrow() == config::ControllerType::FourScore || *controller2_type_clone.borrow() == config::ControllerType::FourScore {
                    let b3 = controller3_bindings_clone.borrow();
                    for (i, s) in b3.iter().enumerate() {
                        if s == &btn_str {
                            let mask = BIT_MASKS[i];
                            if pressed { controller_port3_clone.fetch_or(mask, Ordering::Relaxed); } else { controller_port3_clone.fetch_and(!mask, Ordering::Relaxed); }
                            if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                                if controller_port3_clone.load(Ordering::Relaxed) & 0x03 == 0x03 { controller_port3_clone.fetch_and(!mask, Ordering::Relaxed); }
                                if controller_port3_clone.load(Ordering::Relaxed) & 0x0C == 0x0C { controller_port3_clone.fetch_and(!mask, Ordering::Relaxed); }
                            }
                            break;
                        }
                    }
                    let b4 = controller4_bindings_clone.borrow();
                    for (i, s) in b4.iter().enumerate() {
                        if s == &btn_str {
                            let mask = BIT_MASKS[i];
                            if pressed { controller_port4_clone.fetch_or(mask, Ordering::Relaxed); } else { controller_port4_clone.fetch_and(!mask, Ordering::Relaxed); }
                            if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                                if controller_port4_clone.load(Ordering::Relaxed) & 0x03 == 0x03 { controller_port4_clone.fetch_and(!mask, Ordering::Relaxed); }
                                if controller_port4_clone.load(Ordering::Relaxed) & 0x0C == 0x0C { controller_port4_clone.fetch_and(!mask, Ordering::Relaxed); }
                            }
                            break;
                        }
                    }
                }
                // expansion adapters
                if *expansion_adapter_type_clone.borrow() != config::ExpansionAdapterType::None {
                    let exp_bindings = [
                        expansion1_bindings_clone.borrow(),
                        expansion2_bindings_clone.borrow(),
                        expansion3_bindings_clone.borrow(),
                        expansion4_bindings_clone.borrow(),
                    ];
                    let exp_ports = [
                        &expansion_adapter_port1_clone,
                        &expansion_adapter_port2_clone,
                        &expansion_adapter_port3_clone,
                        &expansion_adapter_port4_clone,
                    ];
                    for (pi, bindings) in exp_bindings.iter().enumerate() {
                        for (i, s) in bindings.iter().enumerate() {
                            if s == &btn_str {
                                let mask = BIT_MASKS[i];
                                let port = exp_ports[pi];
                                if pressed { port.fetch_or(mask, Ordering::Relaxed); } else { port.fetch_and(!mask, Ordering::Relaxed); }
                                if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                                    if port.load(Ordering::Relaxed) & 0x03 == 0x03 { port.fetch_and(!mask, Ordering::Relaxed); }
                                    if port.load(Ordering::Relaxed) & 0x0C == 0x0C { port.fetch_and(!mask, Ordering::Relaxed); }
                                }
                                break;
                            }
                        }
                    }
                }
                // zapper trigger (also vs zapper trigger)
                    {
                        let zt = zapper_trigger_binding_clone.borrow();
                        if *zt == btn_str {
                            if pressed {
                                zapper_trigger_clone.store(true, Ordering::Relaxed);
                                zapper_bogo_clone.store(3, Ordering::Relaxed);
                            } else {
                                zapper_trigger_clone.store(false, Ordering::Relaxed);
                            }
                        }
                    }
                    // expansion zapper trigger
                    {
                        let ezt = expansion_zapper_trigger_binding_clone.borrow();
                        if *ezt == btn_str {
                            if pressed {
                                expansion_zapper_trigger_clone.store(true, Ordering::Relaxed);
                                zapper_bogo_clone.store(3, Ordering::Relaxed);
                            } else {
                                expansion_zapper_trigger_clone.store(false, Ordering::Relaxed);
                            }
                        }
                    }
                    // oeka kids tablet click
                    {
                        let okt = expansion_oeka_click_binding_clone.borrow();
                        if *okt == btn_str {
                            oeka_click_clone.store(pressed, Ordering::Relaxed);
                        }
                    }
                    // family trainer buttons
                    if expansion_type_clone.borrow().is_family_trainer() {
                        let ft = family_trainer_bindings_clone.borrow();
                        let side_b = *expansion_type_clone.borrow() == config::ExpansionType::FamilyTrainerB;
                        let mut ft_lock = family_trainer_state_clone.lock().unwrap();
                        for (i, s) in ft.iter().enumerate() {
                            if s == &btn_str {
                                let idx = if side_b { (i / 4) * 4 + (3 - (i % 4)) } else { i };
                                ft_lock[idx] = if pressed { 1 } else { 0 };
                            }
                        }
                    }
                    // konami hyper shot buttons
                    if expansion_type_clone.borrow().is_hyper_shot() {
                        let hs = hyper_shot_bindings_clone.borrow();
                        let mut hs_lock = hyper_shot_state_clone.lock().unwrap();
                        for (i, s) in hs.iter().enumerate() {
                            if s == &btn_str {
                                hs_lock[i] = if pressed { 1 } else { 0 };
                            }
                        }
                    }
                    // bandai hyper shot buttons
                    if expansion_type_clone.borrow().is_bandai_hyper_shot() {
                        let bh = bandai_hyper_bindings_clone.borrow();
                        let mut bh_lock = bandai_hyper_buttons_clone.lock().unwrap();
                        for (i, s) in bh.iter().enumerate() {
                            if s == &btn_str {
                                bh_lock[i] = if pressed { 1 } else { 0 };
                            }
                        }
                    }
                    // family basic keyboard buttons
                    if expansion_type_clone.borrow().is_family_basic() {
                        let fb = family_basic_bindings_clone.borrow();
                        let mut fb_lock = family_basic_state_clone.lock().unwrap();
                        for (i, s) in fb.iter().enumerate() {
                            if s == &btn_str {
                                fb_lock[i] = if pressed { 1 } else { 0 };
                            }
                        }
                    }
                    // party tap buttons
                    if expansion_type_clone.borrow().is_party_tap() {
                        let pt = party_tap_bindings_clone.borrow();
                        let mut pt_lock = party_tap_state_clone.lock().unwrap();
                        for (i, s) in pt.iter().enumerate() {
                            if s == &btn_str {
                                pt_lock[i] = if pressed { 1 } else { 0 };
                            }
                        }
                    }
                    // pachinko controller buttons
                    if expansion_type_clone.borrow().is_pachinko() {
                        let pc = pachinko_bindings_clone.borrow();
                        let mut pc_lock = pachinko_state_clone.lock().unwrap();
                        for (i, s) in pc.iter().enumerate() {
                            if s == &btn_str {
                                pc_lock[i] = if pressed { 1 } else { 0 };
                            }
                        }
                    }
                    // exciting boxing (punching bag) buttons
                    if expansion_type_clone.borrow().is_exciting_boxing() {
                        let eb = punching_bag_bindings_clone.borrow();
                        let mut eb_lock = punching_bag_state_clone.lock().unwrap();
                        for (i, s) in eb.iter().enumerate() {
                            if s == &btn_str {
                                eb_lock[i] = if pressed { 1 } else { 0 };
                            }
                        }
                    }
                    // jissen mahjong buttons
                    if expansion_type_clone.borrow().is_jissen_mahjong() {
                        let jm = jissen_mahjong_bindings_clone.borrow();
                        let mut jm_lock = jissen_mahjong_state_clone.lock().unwrap();
                        for (i, s) in jm.iter().enumerate() {
                            if s == &btn_str {
                                jm_lock[i] = if pressed { 1 } else { 0 };
                            }
                        }
                    }
                    // subor keyboard buttons
                    if expansion_type_clone.borrow().is_subor_keyboard() {
                        let sk = subor_keyboard_bindings_clone.borrow();
                        let mut sk_lock = subor_keyboard_state_clone.lock().unwrap();
                        for (i, s) in sk.iter().enumerate() {
                            if s == &btn_str {
                                sk_lock[i] = if pressed { 1 } else { 0 };
                            }
                        }
                    }
                    // paddle button
                    {
                        let pb = paddle1_button_binding_clone.borrow();
                        if *pb == btn_str {
                            paddle_button_clone.lock().unwrap()[0] = pressed;
                        }
                    }
                    {
                        let pb = paddle2_button_binding_clone.borrow();
                        if *pb == btn_str {
                            paddle_button_clone.lock().unwrap()[1] = pressed;
                        }
                    }
                    {
                        let pb = expansion_paddle_button_binding_clone.borrow();
                        if *pb == btn_str {
                            paddle_button_clone.lock().unwrap()[2] = pressed;
                        }
                    }
                    // powerpad buttons
                    {
                        let pp1 = powerpad1_bindings_clone.borrow();
                        for (i, s) in pp1.iter().enumerate() {
                            if s == &btn_str {
                                let mut pp = powerpad_state_clone.lock().unwrap();
                                if pressed { pp[0] |= 1 << i; } else { pp[0] &= !(1 << i); }
                            }
                        }
                    }
                    {
                        let pp2 = powerpad2_bindings_clone.borrow();
                        for (i, s) in pp2.iter().enumerate() {
                            if s == &btn_str {
                                let mut pp = powerpad_state_clone.lock().unwrap();
                                if pressed { pp[1] |= 1 << i; } else { pp[1] &= !(1 << i); }
                            }
                        }
                    }
                    // snes pad buttons
                    {
                        let s1 = snes1_bindings_clone.borrow();
                        for (i, s) in s1.iter().enumerate() {
                            if s == &btn_str {
                                let mut ss = snes_state_clone.lock().unwrap();
                                if pressed { ss[0] |= 1 << i; } else { ss[0] &= !(1 << i); }
                            }
                        }
                    }
                    {
                        let s2 = snes2_bindings_clone.borrow();
                        for (i, s) in s2.iter().enumerate() {
                            if s == &btn_str {
                                let mut ss = snes_state_clone.lock().unwrap();
                                if pressed { ss[1] |= 1 << i; } else { ss[1] &= !(1 << i); }
                            }
                        }
                    }
                    // virtual boy pad buttons
                    {
                        let v1 = vb1_bindings_clone.borrow();
                        for (i, s) in v1.iter().enumerate() {
                            if s == &btn_str {
                                let mut vs = virtualboy_state_clone.lock().unwrap();
                                if pressed { vs[0] |= 1 << i; } else { vs[0] &= !(1 << i); }
                            }
                        }
                    }
                    {
                        let v2 = vb2_bindings_clone.borrow();
                        for (i, s) in v2.iter().enumerate() {
                            if s == &btn_str {
                                let mut vs = virtualboy_state_clone.lock().unwrap();
                                if pressed { vs[1] |= 1 << i; } else { vs[1] &= !(1 << i); }
                            }
                        }
                    }
                    // snes mouse buttons
                    {
                        let m1 = snes_mouse1_bindings_clone.borrow();
                        for (i, s) in m1.iter().enumerate() {
                            if s == &btn_str {
                                let mut mb = snes_mouse_buttons_clone.lock().unwrap();
                                if pressed { mb[0] |= 1 << i; } else { mb[0] &= !(1 << i); }
                            }
                        }
                    }
                    {
                        let m2 = snes_mouse2_bindings_clone.borrow();
                        for (i, s) in m2.iter().enumerate() {
                            if s == &btn_str {
                                let mut mb = snes_mouse_buttons_clone.lock().unwrap();
                                if pressed { mb[1] |= 1 << i; } else { mb[1] &= !(1 << i); }
                            }
                        }
                    }
                    // subor mouse buttons
                    {
                        let m1 = subor_mouse1_bindings_clone.borrow();
                        for (i, s) in m1.iter().enumerate() {
                            if s == &btn_str {
                                let mut sb = subor_mouse_buttons_clone.lock().unwrap();
                                if pressed { sb[0] |= 1 << i; } else { sb[0] &= !(1 << i); }
                            }
                        }
                    }
                    {
                        let m2 = subor_mouse2_bindings_clone.borrow();
                        for (i, s) in m2.iter().enumerate() {
                            if s == &btn_str {
                                let mut sb = subor_mouse_buttons_clone.lock().unwrap();
                                if pressed { sb[1] |= 1 << i; } else { sb[1] &= !(1 << i); }
                            }
                        }
                    }
                    // controller 2 gamepad
                    if *controller2_type_clone.borrow() == config::ControllerType::Gamepad || *controller2_type_clone.borrow() == config::ControllerType::FamicomGamepad || *controller1_type_clone.borrow() == config::ControllerType::FourScore {
                        let b2 = controller2_bindings_clone.borrow();
                        for (i, s) in b2.iter().enumerate() {
                            if s == &btn_str {
                                let mask = BIT_MASKS[i];
                                if pressed { controller_port2_clone.fetch_or(mask, Ordering::Relaxed); } else { controller_port2_clone.fetch_and(!mask, Ordering::Relaxed); }
                                if !*allow_opposing_dpad_clone.borrow() && pressed && (mask & 0x0F) != 0 {
                                    if controller_port2_clone.load(Ordering::Relaxed) & 0x03 == 0x03 { controller_port2_clone.fetch_and(!mask, Ordering::Relaxed); }
                                    if controller_port2_clone.load(Ordering::Relaxed) & 0x0C == 0x0C { controller_port2_clone.fetch_and(!mask, Ordering::Relaxed); }
                                }
                                break;
                            }
                        }
                    }
                    // famicom microphone
                    if *controller2_type_clone.borrow() == config::ControllerType::FamicomGamepad {
                        let mb = famicom_mic_binding_clone.borrow();
                        if &*mb == &btn_str {
                            if pressed { famicom_mic_clone.store(true, Ordering::Relaxed); } else { famicom_mic_clone.store(false, Ordering::Relaxed); }
                        }
                    }
                }
                if pressed && button == winit::event::MouseButton::Left {
                    let window_size = window.inner_size();
                    let width = window_size.width as usize;
                    let height = window_size.height as usize;
                    let ms = menu_state_clone.borrow();
                    let (mx, my) = ms.mouse_pos;
                    
                    if ms.show_dip_switches {
                        let sc = ms.scale;
                        let is_custom = ms.dip_definition.is_some();
                        let row_h_base = 35.0 * sc;
                        let line_h = 10.0 * sc;
                        let (dialog_w, dialog_h, choice_x_offset, lines_per_row) = if let Some(ref game) = ms.dip_definition {
                            let max_name_len = game.settings.iter().map(|s| s.name.len()).max().unwrap_or(0);
                            let max_choice_len = game.settings.iter().flat_map(|s| s.choices.iter().map(|c| c.name.len())).max().unwrap_or(0);
                            let label_w = (max_name_len as f32) * 8.0 * sc;
                            let label_area = (label_w + (25.0 * sc)).max(225.0 * sc);
                            let choice_w = (max_choice_len as f32) * 8.0 * sc + (20.0 * sc);
                            let choice_area = choice_w.max(120.0 * sc);
                            let natural_w = (label_area + choice_area + (15.0 * sc)).max(320.0 * sc);
                            let max_dialog_w = (width as f32 - (80.0 * sc)).max(320.0 * sc);
                            let (final_w, final_label_area, lines) = if natural_w > max_dialog_w {
                                let cap_w = max_dialog_w;
                                let choice_area_f = choice_area.max(120.0 * sc);
                                let label_area_f = (cap_w - choice_area_f - (15.0 * sc)).max(180.0 * sc);
                                let avail_label_w = label_area_f - (15.0 * sc) - (5.0 * sc);
                                let cpl = (avail_label_w / (8.0 * sc)).floor() as usize;
                                let cpl = cpl.max(16);
                                let l: Vec<usize> = game.settings.iter().map(|s| ((s.name.len() + cpl - 1) / cpl).max(1)).collect();
                                (cap_w.round() as usize, label_area_f.round() as usize, l)
                            } else {
                                let l = vec![1; game.settings.len()];
                                (natural_w.round() as usize, label_area.round() as usize, l)
                            };
                            let total_extra: f32 = lines.iter().map(|&n| (n as f32 - 1.0) * line_h).sum();
                            let h = (50.0 * sc + row_h_base * game.settings.len() as f32 + total_extra).max(120.0 * sc).round() as usize;
                            (final_w, h, final_label_area, lines)
                        } else {
                            let w = (320.0 * sc).round() as usize;
                            let h = (260.0 * sc).round() as usize;
                            (w, h, 0, Vec::new())
                        };
                        
                        let dialog_x = (width.saturating_sub(dialog_w)) / 2;
                        let dialog_y = (height.saturating_sub(dialog_h)) / 2;

                        let close_w = (20.0 * sc).round() as usize;
                        let close_h = (20.0 * sc).round() as usize;
                        let close_x = dialog_x + dialog_w - close_w - (10.0 * sc).round() as usize;
                        let close_y = dialog_y + (5.0 * sc).round() as usize;

                        if point_in_rect(mx, my, close_x, close_y, close_w, close_h) {
                            drop(ms);
                            let mut ms_mut = menu_state_clone.borrow_mut();
                            ms_mut.show_dip_switches = false;
                            paused_clone.store(false, Ordering::Relaxed);
                        } else if is_custom {
                            let choice_w = dialog_w - choice_x_offset - (15.0 * sc).round() as usize;
                            let choice_h = (24.0 * sc).round() as usize;
                            let choice_x = dialog_x + choice_x_offset;

                            let mut clicked_setting_idx = None;
                            if let Some(ref game) = ms.dip_definition {
                                for i in 0..game.settings.len() {
                                    let row_extra: f32 = (0..i).map(|j| (lines_per_row.get(j).copied().unwrap_or(1) as f32 - 1.0) * line_h).sum();
                                    let row_y = dialog_y + (45.0 * sc + i as f32 * row_h_base + row_extra).round() as usize;
                                    if point_in_rect(mx, my, choice_x, row_y, choice_w, choice_h) {
                                        clicked_setting_idx = Some(i);
                                        break;
                                    }
                                }
                            }
                            let clicked_setting = clicked_setting_idx.and_then(|idx| {
                                ms.dip_definition.as_ref().map(|g| g.settings[idx].clone())
                            });
                            let game_clone = ms.dip_definition.clone();
                            drop(ms);
                            if let Some(setting) = clicked_setting {
                                if !setting.choices.is_empty() {
                                    let mut emu = emu_clone.lock().unwrap();
                                    let mut val = emu.get_dip_switches() as u32;
                                    let current_val = val & setting.mask;
                                    let current_pos = setting.choices.iter().position(|c| c.value == current_val).unwrap_or(0);
                                    let next_pos = (current_pos + 1) % setting.choices.len();
                                    val = (val & !setting.mask) | setting.choices[next_pos].value;
                                    emu.set_dip_switches(val as u8);
                                    if let Some(ref game) = game_clone {
                                        let crc = emu.prg_rom_crc32();
                                        let variant = compute_vs_ppu_variant(game, emu.get_dip_switches(), crc);
                                        emu.set_vs_ppu_variant(variant);
                                    }
                                }
                            }
                        } else {
                            let row_start_y = dialog_y + (45.0 * sc).round() as usize;
                            let row_h = (25.0 * sc).round() as usize;
                            let cb_w = (16.0 * sc).round() as usize;
                            let cb_h = (16.0 * sc).round() as usize;
                            let cb_x = dialog_x + dialog_w - cb_w - (25.0 * sc).round() as usize;

                            let mut clicked_bit = None;
                            for bit_idx in 0..8 {
                                let cb_y = row_start_y + bit_idx * row_h;
                                if point_in_rect(mx, my, cb_x, cb_y, cb_w, cb_h) {
                                    clicked_bit = Some(bit_idx);
                                    break;
                                }
                            }
                            drop(ms);
                            if let Some(bit) = clicked_bit {
                                let mut emu = emu_clone.lock().unwrap();
                                let mut val = emu.get_dip_switches();
                                val ^= 1 << bit;
                                emu.set_dip_switches(val);
                            }
                        }
                    } else if ms.show_confirm_exit_dialog {
                        let sc = ms.scale;
                        let dlg_w = (300.0 * sc).round() as usize;
                        let dlg_h = (100.0 * sc).round() as usize;
                        let dlg_x = (width.saturating_sub(dlg_w)) / 2;
                        let dlg_y = (height.saturating_sub(dlg_h)) / 2;
                        let btn_w = (60.0 * sc).round() as usize;
                        let btn_h = (24.0 * sc).round() as usize;
                        let btn_y = dlg_y + dlg_h - btn_h - (10.0 * sc).round() as usize;
                        let yes_x = dlg_x + (30.0 * sc).round() as usize;
                        let no_x = dlg_x + dlg_w - btn_w - (30.0 * sc).round() as usize;
                        if point_in_rect(mx, my, yes_x, btn_y, btn_w, btn_h) {
                            drop(ms);
                            let emu = emu_clone.lock().unwrap();
                            if *auto_save_sram_clone.borrow() {
                                emu.save_prg_ram();
                                emu.save_turbo_file();
                                emu.save_battle_box();
                            }
                            *control_flow = ControlFlow::Exit;
                        } else if point_in_rect(mx, my, no_x, btn_y, btn_w, btn_h) {
                            drop(ms);
                            menu_state_clone.borrow_mut().show_confirm_exit_dialog = false;
                            paused_clone.store(false, Ordering::Relaxed);
                        }
                    } else if ms.show_barcode_input {
                        let sc = ms.scale;
                        let dlg_w = (320.0 * sc).round() as usize;
                        let dlg_h = (160.0 * sc).round() as usize;
                        let dlg_x = (width.saturating_sub(dlg_w)) / 2;
                        let dlg_y = (height.saturating_sub(dlg_h)) / 2;
                        let margin = (12.0 * sc).round() as usize;
                        let title_h = (26.0 * sc).round() as usize;
                        let field_x = dlg_x + margin;
                        let field_w = dlg_w - margin * 2;
                        let field_y = dlg_y + margin + title_h;
                        let field_h = (30.0 * sc).round() as usize;
                        let btn_w = (60.0 * sc).round() as usize;
                        let btn_h = (24.0 * sc).round() as usize;
                        let btn_y = dlg_y + dlg_h - btn_h - margin;
                        let ok_x = dlg_x + margin;
                        let clear_x = ok_x + btn_w + (8.0 * sc).round() as usize;
                        let close_x = dlg_x + dlg_w - btn_w - margin;
                        if point_in_rect(mx, my, field_x, field_y, field_w, field_h) {
                            drop(ms);
                            let mut ms_mut = menu_state_clone.borrow_mut();
                            let caret = barcode_caret_from_x(mx, field_x, sc, ms_mut.barcode_input.len());
                            ms_mut.barcode_caret = caret;
                            ms_mut.barcode_sel_anchor = Some(caret);
                            ms_mut.barcode_dragging = true;
                        } else if point_in_rect(mx, my, ok_x, btn_y, btn_w, btn_h) {
                            drop(ms);
                            let rc: Vec<u8> = menu_state_clone.borrow().barcode_input.bytes().collect();
                            let _ = cmd_tx.send(EmuCommand::SetBarcode(rc));
                            barcode_ctrl_clone.store(false, Ordering::Relaxed);
                            menu_state_clone.borrow_mut().show_barcode_input = false;
                            paused_clone.store(false, Ordering::Relaxed);
                        } else if point_in_rect(mx, my, clear_x, btn_y, btn_w, btn_h) {
                            drop(ms);
                            let mut ms_mut = menu_state_clone.borrow_mut();
                            ms_mut.barcode_input.clear();
                            ms_mut.barcode_caret = 0;
                            ms_mut.barcode_sel_anchor = None;
                        } else if point_in_rect(mx, my, close_x, btn_y, btn_w, btn_h) {
                            drop(ms);
                            barcode_ctrl_clone.store(false, Ordering::Relaxed);
                            menu_state_clone.borrow_mut().show_barcode_input = false;
                            paused_clone.store(false, Ordering::Relaxed);
                        }
                    } else if ms.show_about {
                        let about_w = (300.0 * ms.scale).round() as usize;
                        let about_h = (220.0 * ms.scale).round() as usize;
                        let about_x = (width.saturating_sub(about_w)) / 2;
                        let about_y = (height.saturating_sub(about_h)) / 2;
                        let close_w = (20.0 * ms.scale).round() as usize;
                        let close_h = (20.0 * ms.scale).round() as usize;
                        let close_x = about_x + about_w - close_w - (10.0 * ms.scale).round() as usize;
                        let close_y = about_y + (10.0 * ms.scale).round() as usize;
                        if point_in_rect(mx, my, close_x, close_y, close_w, close_h) {
                            drop(ms);
                            menu_state_clone.borrow_mut().show_about = false;
                            paused_clone.store(false, Ordering::Relaxed);
                        }
                    } else if ms.show_updates {
                        let update_w = (360.0 * ms.scale).round() as usize;
                        let update_h = (180.0 * ms.scale).round() as usize;
                        let update_x = (width.saturating_sub(update_w)) / 2;
                        let update_y = (height.saturating_sub(update_h)) / 2;
                        let title_h = (30.0 * ms.scale).round() as usize;
                        let close_w = (20.0 * ms.scale).round() as usize;
                        let close_h = (20.0 * ms.scale).round() as usize;
                        let close_right_margin = (3.0 * ms.scale).round() as usize;
                        let close_x = update_x + update_w - close_w - close_right_margin;
                        let close_y = update_y + (title_h - close_h) / 2 - (1.0 * ms.scale).round() as usize;
                        if point_in_rect(mx, my, close_x, close_y, close_w, close_h) {
                            drop(ms);
                            menu_state_clone.borrow_mut().show_updates = false;
                            paused_clone.store(false, Ordering::Relaxed);
                        } else {
                            let update_state = update_check_state.lock().unwrap();
                            if let UpdateCheckState::Available { url, .. } = &*update_state {
                                let btn_w = (160.0 * ms.scale).round() as usize;
                                let btn_h = (28.0 * ms.scale).round() as usize;
                                let btn_x = update_x + (update_w.saturating_sub(btn_w)) / 2;
                                let btn_y = update_y + update_h - btn_h - (15.0 * ms.scale).round() as usize;
                                if point_in_rect(mx, my, btn_x, btn_y, btn_w, btn_h) {
                                    let url = url.clone();
                                    drop(update_state);
                                    drop(ms);
                                    let _ = std::process::Command::new("cmd")
                                        .args(&["/c", "start", "", &url])
                                        .spawn();
                                    menu_state_clone.borrow_mut().show_updates = false;
                                    paused_clone.store(false, Ordering::Relaxed);
                                }
                            }
                        }
                    } else if ms.show_general_settings {
                        let general_w = (400.0 * ms.scale).round() as usize;
                        let general_h = (280.0 * ms.scale).round() as usize;
                        let general_x = (width.saturating_sub(general_w)) / 2;
                        let general_y = (height.saturating_sub(general_h)) / 2;
                        let close_w = (20.0 * ms.scale).round() as usize;
                        let close_h = (20.0 * ms.scale).round() as usize;
                        let close_x = general_x + general_w - close_w - (10.0 * ms.scale).round() as usize;
                        let close_y = general_y + (10.0 * ms.scale).round() as usize;
                        if point_in_rect(mx, my, close_x, close_y, close_w, close_h) {
                            drop(ms);
                            menu_state_clone.borrow_mut().show_general_settings = false;
                            paused_clone.store(false, Ordering::Relaxed);
                        } else {
                            let sc = ms.scale;
                            let title_h = (30.0 * sc).round() as usize;
                            let row_h = (22.0 * sc).round() as usize;
                            let box_w = (100.0 * sc).round() as usize;
                            let row_y = general_y + title_h + (15.0 * sc).round() as usize;
                            let box_x = general_x + general_w - (15.0 * sc).round() as usize - box_w;

                            if point_in_rect(mx, my, box_x, row_y, box_w, row_h) {
                                let new_val = !*pause_on_lost_focus_clone.borrow();
                                *pause_on_lost_focus_clone.borrow_mut() = new_val;
                                config::save_pause_on_lost_focus(new_val);
                            }
                            let row2_y = row_y + row_h + (8.0 * sc).round() as usize;
                            if point_in_rect(mx, my, box_x, row2_y, box_w, row_h) {
                                let mut mode = initial_ram_clone.borrow_mut();
                                *mode = mode.next();
                                config::save_initial_ram(*mode);
                            }
                            let row3_y = row2_y + row_h + (8.0 * sc).round() as usize;
                            if point_in_rect(mx, my, box_x, row3_y, box_w, row_h) {
                                let mut mode = fps_mode_clone.borrow_mut();
                                *mode = mode.next();
                                config::save_fps_mode(*mode);
                            }
                            let row4_y = row3_y + row_h + (8.0 * sc).round() as usize;
                            if point_in_rect(mx, my, box_x, row4_y, box_w, row_h) {
                                let new_val = !*confirm_on_exit_clone.borrow();
                                *confirm_on_exit_clone.borrow_mut() = new_val;
                                config::save_confirm_on_exit(new_val);
                            }
                            let row5_y = row4_y + row_h + (8.0 * sc).round() as usize;
                            if point_in_rect(mx, my, box_x, row5_y, box_w, row_h) {
                                let new_val = !*auto_save_sram_clone.borrow();
                                *auto_save_sram_clone.borrow_mut() = new_val;
                                config::save_auto_save_sram(new_val);
                            }
                            let row6_y = row5_y + row_h + (8.0 * sc).round() as usize;
                            if point_in_rect(mx, my, box_x, row6_y, box_w, row_h) {
                                let new_val = !*check_updates_on_startup_clone.borrow();
                                *check_updates_on_startup_clone.borrow_mut() = new_val;
                                config::save_check_updates_on_startup(new_val);
                            }
                            let row7_y = row6_y + row_h + (8.0 * sc).round() as usize;
                            if point_in_rect(mx, my, box_x, row7_y, box_w, row_h) {
                                drop(ms);
                                let cur = menu_state_clone.borrow().theme.clone();
                                let new_theme = match cur.as_str() {
                                    "dark" => "light",
                                    "light" => "classicnes",
                                    "classicnes" => "famicom",
                                    "famicom" => "mario",
                                    "mario" => "link",
                                    "link" => "metroid",
                                    "metroid" => "megaman",
                                    "megaman" => "dark",
                                    _ => "dark",
                                }.to_string();
                                menu_state_clone.borrow_mut().theme = new_theme.clone();
                                config::save_theme(&new_theme);
                            }
                        }
                    } else if ms.show_audio_settings {
                        let sc = ms.scale;
                        let aw = (400.0 * sc).round() as usize;
                        let gap = (8.0 * sc).round() as usize;
                        let title_h = (30.0 * sc).round() as usize;
                        let row_h = (22.0 * sc).round() as usize;
                        let border_thickness = (2.0 * sc).round() as usize;
                        let rate_val = *audio_rate_clone.borrow();
                        let channels = active_audio_channels(rate_val);
                        let content_rows = 3 + channels.len();
                        let ah = title_h + gap + content_rows * (row_h + gap) - gap + border_thickness * 2;
                        let ax = (width.saturating_sub(aw)) / 2;
                        let ay = (height.saturating_sub(ah)) / 2;
                        let close_w = (20.0 * sc).round() as usize;
                        let close_h = (20.0 * sc).round() as usize;
                        let close_x = ax + aw - close_w - (10.0 * sc).round() as usize;
                        let close_y = ay + (5.0 * sc).round() as usize;
                        if point_in_rect(mx, my, close_x, close_y, close_w, close_h) {
                            drop(ms);
                            menu_state_clone.borrow_mut().show_audio_settings = false;
                            paused_clone.store(false, Ordering::Relaxed);
                        } else {
                            let sc = ms.scale;
                            let title_h = (30.0 * sc).round() as usize;
                            let row_h = (22.0 * sc).round() as usize;
                            let gap = (8.0 * sc).round() as usize;
                            let box_w = (80.0 * sc).round() as usize;
                            let slider_w = (120.0 * sc).round() as usize;
                            let mut row_y = ay + title_h + gap;
                            let box_x = ax + aw - (15.0 * sc).round() as usize - box_w;
                            let slider_x = ax + aw - (15.0 * sc).round() as usize - slider_w;
                            let channels = active_audio_channels(*audio_rate_clone.borrow());

                            if point_in_rect(mx, my, box_x, row_y, box_w, row_h) {
                                let new_val = !*audio_enabled_clone.borrow();
                                *audio_enabled_clone.borrow_mut() = new_val;
                                config::save_audio_enabled(new_val);
                                emu_clone.lock().unwrap().audio_enabled = new_val;
                            }

                            row_y += row_h + gap;
                            if point_in_rect(mx, my, box_x, row_y, box_w, row_h) {
                                let mut depth = audio_depth_clone.borrow_mut();
                                *depth = if *depth == 8 { 16 } else { 8 };
                                config::save_audio_depth(*depth);
                                emu_clone.lock().unwrap().audio_depth = *depth;
                            }

                            row_y += row_h + gap;
                            if point_in_rect(mx, my, box_x, row_y, box_w, row_h) {
                                const RATES: &[u32] = &[11025, 22050, 32000, 44100, 48000, 96000];
                                let current = *audio_rate_clone.borrow();
                                let pos = RATES.iter().position(|r| *r == current).unwrap_or(0);
                                let new_rate = RATES[(pos + 1) % RATES.len()];
                                *audio_rate_clone.borrow_mut() = new_rate;
                                config::save_audio_rate(new_rate);
                                                                let dev_rate = *audio_device_rate_clone.lock().unwrap();
                                *phase_inc_clone.lock().unwrap() = new_rate as f64 / dev_rate as f64;
                                emu_clone.lock().unwrap().set_audio_output(audio_buffer_clone2.clone(), new_rate as f64);
                            }

                            drop(ms);
                            for &(chan_idx, _) in channels {
                                row_y += row_h + gap;
                                if point_in_rect(mx, my, slider_x, row_y, slider_w, row_h) {
                                    let click_x = mx.saturating_sub(slider_x);
                                    let pct = ((click_x as f32 / slider_w as f32) * 100.0).round().min(100.0).max(0.0) as u8;
                                    let mut vols = channel_volumes_clone.borrow_mut();
                                    vols[chan_idx] = pct;
                                    config::save_channel_volume(config::CHANNEL_NAMES[chan_idx], pct);
                                    let vol_f32 = pct as f32 / 100.0;
                                    let mut e = emu_clone.lock().unwrap();
                                    match chan_idx {
                                        0 => e.master_volume = vol_f32,
                                        1 => e.triangle_volume = vol_f32,
                                        2 => e.square1_volume = vol_f32,
                                        3 => e.square2_volume = vol_f32,
                                        4 => e.noise_volume = vol_f32,
                                        _ => e.pcm_volume = vol_f32,
                                    }
                                    menu_state_clone.borrow_mut().dragging_audio_slider = Some(chan_idx);
                                }
                            }
                        }
                    } else if ms.show_controller1_settings {
                        let sc = ms.scale;
                        let c1_w = (440.0 * sc).round() as usize;
                        let title_h = (30.0 * sc).round() as usize;
                        let btn_w = (90.0 * sc).round() as usize;
                        let btn_h = (26.0 * sc).round() as usize;
                        let gap_y = (8.0 * sc).round() as usize;
                        let c1t = *controller1_type_clone.borrow();
                        let c2t = *controller2_type_clone.borrow();
                        let fs1 = c1t == config::ControllerType::FourScore || c2t == config::ControllerType::FourScore;
                        let grid_h = 4 * (btn_h + gap_y) + btn_h;
                        let tmp_g0 = title_h + (10.0 * sc).round() as usize;
                        let c1_h = if fs1 {
                            let fs_ch = tmp_g0 + 2 * (grid_h + btn_h / 2) + (40.0 * sc).round() as usize;
                            fs_ch.max(260)
                        } else {
                            (260.0 * sc).round() as usize
                        };
                        let c1_x = (width.saturating_sub(c1_w)) / 2;
                        let c1_y = (height.saturating_sub(c1_h)) / 2;
                        let _title_h = title_h;
                        let close_w = (20.0 * sc).round() as usize;
                        let close_h = (20.0 * sc).round() as usize;
                        let close_x = c1_x + c1_w - close_w - (10.0 * sc).round() as usize;
                        let close_y = c1_y + (5.0 * sc).round() as usize;
                        if point_in_rect(mx, my, close_x, close_y, close_w, close_h) {
                            drop(ms);
                            menu_state_clone.borrow_mut().show_controller1_settings = false;
                            paused_clone.store(false, Ordering::Relaxed);
                        } else {
                            let cw = c1_w;
                            let cx = c1_x;
                            let cy = c1_y;
                            let gap_x = (cw.saturating_sub(2 * btn_w)) / 3;
                            let grid_y0 = cy + tmp_g0;
                            let mut clicked_rebind = None;
                            let c1t = *controller1_type_clone.borrow();
                            let is_single1 = c1t == config::ControllerType::Zapper || c1t == config::ControllerType::Paddle;
                            let is_snesmouse1 = c1t == config::ControllerType::SNESMouse || c1t == config::ControllerType::SuborMouse;
                            let is_pp1 = c1t == config::ControllerType::PowerPadA || c1t == config::ControllerType::PowerPadB;
                            let is_snes1 = c1t == config::ControllerType::SNESPad;
                            let is_vb1 = c1t == config::ControllerType::VirtualBoy;
                            if is_single1 {
                                let total_w = 2 * btn_w + gap_x;
                                let trig_bx = cx + (cw - total_w) / 2;
                                if point_in_rect(mx, my, trig_bx, grid_y0, total_w, btn_h) {
                                    clicked_rebind = Some(0);
                                }
                            } else if is_snesmouse1 {
                                let per_w = btn_w;
                                let half_gap = (cw.saturating_sub(2 * per_w)) / 3;
                                for i in 0..config::SNES_MOUSE_BUTTON_COUNT {
                                    let bx = cx + half_gap + i * (per_w + half_gap);
                                    if point_in_rect(mx, my, bx, grid_y0, per_w, btn_h) {
                                        clicked_rebind = Some(i);
                                        break;
                                    }
                                }
                            } else if is_pp1 {
                                let cols = 4;
                                let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                                for i in 0..config::POWERPAD_BUTTON_COUNT {
                                    let row = i / cols;
                                    let col = i % cols;
                                    let bx = cx + gap_x + col * (btn_w + gap_x);
                                    let by = grid_y0 + row * (btn_h + gap_y);
                                    if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                        clicked_rebind = Some(i);
                                        break;
                                    }
                                }
                            } else if is_snes1 {
                                let cols = 4;
                                let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                                for i in 0..config::SNES_BUTTON_COUNT {
                                    let row = i / cols;
                                    let col = i % cols;
                                    let bx = cx + gap_x + col * (btn_w + gap_x);
                                    let by = grid_y0 + row * (btn_h + gap_y);
                                    if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                        clicked_rebind = Some(i);
                                        break;
                                    }
                                }
                            } else if is_vb1 {
                                let cols = 4;
                                let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                                for i in 0..config::VB_BUTTON_COUNT {
                                    let row = i / cols;
                                    let col = i % cols;
                                    let bx = cx + gap_x + col * (btn_w + gap_x);
                                    let by = grid_y0 + row * (btn_h + gap_y);
                                    if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                        clicked_rebind = Some(i);
                                        break;
                                    }
                                }
                            } else if fs1 {
                                let grid_h = 4 * (btn_h + gap_y) + btn_h;
                                for player in 0..2usize {
                                    let yoff = grid_y0 + player * (grid_h + btn_h / 2);
                                    let btn_y0 = yoff + (btn_h * 3 / 4) as usize;
                                    for i in 0..config::GAMEPAD_BUTTON_COUNT {
                                        let row = i / 2;
                                        let by = btn_y0 + row * (btn_h + gap_y);
                                        let bx = if i % 2 == 0 { cx + gap_x } else { cx + gap_x * 2 + btn_w };
                                        if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                            clicked_rebind = Some(i * 2 + player);
                                            break;
                                        }
                                    }
                                    if clicked_rebind.is_some() { break; }
                                }
                            } else {
                                for i in 0..config::GAMEPAD_BUTTON_COUNT {
                                    let row = i / 2;
                                    let by = grid_y0 + row * (btn_h + gap_y);
                                    let bx = if i % 2 == 0 { cx + gap_x } else { cx + gap_x * 2 + btn_w };
                                    if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                        clicked_rebind = Some(i);
                                        break;
                                    }
                                }
                            }
                            let act_btn_w = (70.0 * sc).round() as usize;
                            let act_btn_h = (24.0 * sc).round() as usize;
                            let grid_h = 4 * (btn_h + gap_y) + btn_h;
                            let last_row_bottom = if is_single1 || is_snesmouse1 { grid_y0 + btn_h } else if is_pp1 || is_snes1 { grid_y0 + 2 * (btn_h + gap_y) + btn_h } else if is_vb1 { grid_y0 + 3 * (btn_h + gap_y) + btn_h } else if fs1 { grid_y0 + 2 * (grid_h + btn_h / 2) } else { grid_y0 + 4 * (btn_h + gap_y) + btn_h };
                            let act_y = last_row_bottom + (10.0 * sc).round() as usize;
                            let act_gap = (10.0 * sc).round() as usize;
                            let act_total = 2 * act_btn_w + act_gap;
                            let act_x0 = cx + (cw - act_total) / 2;
                            let clicked_clear = point_in_rect(mx, my, act_x0, act_y, act_btn_w, act_btn_h);
                            let clicked_reset = !clicked_clear && point_in_rect(mx, my, act_x0 + act_btn_w + act_gap, act_y, act_btn_w, act_btn_h);
                            if let Some(i) = clicked_rebind {
                                drop(ms);
                                let mut ms_mut = menu_state_clone.borrow_mut();
                                if fs1 {
                                    let btn = i / 2;
                                    let player = i % 2;
                                    ms_mut.rebind_controller = Some(player as u8 + 1);
                                    ms_mut.rebind_button = Some(btn);
                                } else {
                                    ms_mut.rebind_controller = Some(1);
                                    ms_mut.rebind_button = Some(i);
                                }
                            } else if clicked_clear {
                                drop(ms);
                                if c1t == config::ControllerType::Zapper {
                                    config::save_zapper_trigger("");
                                    *zapper_trigger_binding_clone.borrow_mut() = String::new();
                                } else if c1t == config::ControllerType::Paddle {
                                    config::save_paddle_button("controller1", "");
                                    *paddle1_button_binding_clone.borrow_mut() = String::new();
                                } else if c1t == config::ControllerType::SNESMouse {
                                    config::clear_snes_mouse_bindings("controller1");
                                    *snes_mouse1_bindings_clone.borrow_mut() = config::load_snes_mouse_bindings("controller1");
                                } else if c1t == config::ControllerType::SuborMouse {
                                    config::clear_subor_mouse_bindings("controller1");
                                    *subor_mouse1_bindings_clone.borrow_mut() = config::load_subor_mouse_bindings("controller1");
                                } else if is_pp1 {
                                    config::clear_powerpad_bindings("controller1");
                                    *powerpad1_bindings_clone.borrow_mut() = config::load_powerpad_bindings("controller1");
                                } else if is_snes1 {
                                    config::clear_snes_bindings("controller1");
                                    *snes1_bindings_clone.borrow_mut() = config::load_snes_bindings("controller1");
                                } else if is_vb1 {
                                    config::clear_vb_bindings("controller1");
                                    *vb1_bindings_clone.borrow_mut() = config::load_vb_bindings("controller1");
                                } else if fs1 {
                                    for pfx in &["controller1", "controller2", "controller3", "controller4"] {
                                        config::clear_bindings(pfx);
                                    }
                                    *controller1_bindings_clone.borrow_mut() = config::load_bindings("controller1");
                                    *controller2_bindings_clone.borrow_mut() = config::load_bindings("controller2");
                                    *controller3_bindings_clone.borrow_mut() = config::load_bindings("controller3");
                                    *controller4_bindings_clone.borrow_mut() = config::load_bindings("controller4");
                                } else {
                                    config::clear_bindings("controller1");
                                    *controller1_bindings_clone.borrow_mut() = config::load_bindings("controller1");
                                }
                            } else if clicked_reset {
                                drop(ms);
                                if c1t == config::ControllerType::Zapper {
                                    config::save_zapper_trigger("MouseLeft");
                                    *zapper_trigger_binding_clone.borrow_mut() = "MouseLeft".to_string();
                                } else if c1t == config::ControllerType::Paddle {
                                    config::save_paddle_button("controller1", "MouseLeft");
                                    *paddle1_button_binding_clone.borrow_mut() = "MouseLeft".to_string();
                                } else if c1t == config::ControllerType::SNESMouse {
                                    config::reset_snes_mouse_bindings("controller1");
                                    *snes_mouse1_bindings_clone.borrow_mut() = config::load_snes_mouse_bindings("controller1");
                                } else if c1t == config::ControllerType::SuborMouse {
                                    config::reset_subor_mouse_bindings("controller1");
                                    *subor_mouse1_bindings_clone.borrow_mut() = config::load_subor_mouse_bindings("controller1");
                                } else if is_pp1 {
                                    config::reset_powerpad_bindings("controller1");
                                    *powerpad1_bindings_clone.borrow_mut() = config::load_powerpad_bindings("controller1");
                                } else if is_snes1 {
                                    config::reset_snes_bindings("controller1");
                                    *snes1_bindings_clone.borrow_mut() = config::load_snes_bindings("controller1");
                                } else if is_vb1 {
                                    config::reset_vb_bindings("controller1");
                                    *vb1_bindings_clone.borrow_mut() = config::load_vb_bindings("controller1");
                                } else if fs1 {
                                    for pfx in &["controller1", "controller2", "controller3", "controller4"] {
                                        config::reset_bindings(pfx);
                                    }
                                    *controller1_bindings_clone.borrow_mut() = config::load_bindings("controller1");
                                    *controller2_bindings_clone.borrow_mut() = config::load_bindings("controller2");
                                    *controller3_bindings_clone.borrow_mut() = config::load_bindings("controller3");
                                    *controller4_bindings_clone.borrow_mut() = config::load_bindings("controller4");
                                } else {
                                    config::reset_bindings("controller1");
                                    *controller1_bindings_clone.borrow_mut() = config::load_bindings("controller1");
                                }
                            }
                        }
                    } else if ms.show_expansion_settings {
                        let sc = ms.scale;
                        let adapter_type = *expansion_adapter_type_clone.borrow();
                        let is_adapter = adapter_type != config::ExpansionAdapterType::None;
                        let pair = ms.adapter_pair.unwrap_or(0);
                        let player_offset = pair * 2;
                        let dialog_sub_ports = 2;
                        let sw = (440.0 * sc).round() as usize;
                        let btn_w = (90.0 * sc).round() as usize;
                        let btn_h = (26.0 * sc).round() as usize;
                        let gap_y = (8.0 * sc).round() as usize;
                        let gap_x = (sw.saturating_sub(2 * btn_w)) / 3;
                        let grid_h = 4 * (btn_h + gap_y) + btn_h;
                        let tmp_g0 = (30.0 * sc).round() as usize + (10.0 * sc).round() as usize;
                        let sh = if is_adapter {
                            let ch = tmp_g0 + dialog_sub_ports * (grid_h + btn_h / 2) + (40.0 * sc).round() as usize;
                            ch.max(260)
                        } else if expansion_type_clone.borrow().is_family_trainer() {
                            let mbtn_h = (30.0 * sc).round() as usize;
                            let mrow_gap = (6.0 * sc).round() as usize;
                            let act_btn_h2 = (24.0 * sc).round() as usize;
                            tmp_g0 + btn_h / 2 + 3 * mbtn_h + 2 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h2 + (20.0 * sc).round() as usize
                        } else if expansion_type_clone.borrow().is_hyper_shot() {
                            let mbtn_h = (30.0 * sc).round() as usize;
                            let mrow_gap = (6.0 * sc).round() as usize;
                            let act_btn_h2 = (24.0 * sc).round() as usize;
                            tmp_g0 + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h2 + (20.0 * sc).round() as usize
                        } else if expansion_type_clone.borrow().is_bandai_hyper_shot() {
                            let mbtn_h = (30.0 * sc).round() as usize;
                            let mrow_gap = (6.0 * sc).round() as usize;
                            let act_btn_h2 = (24.0 * sc).round() as usize;
                            tmp_g0 + btn_h / 2 + 3 * mbtn_h + 2 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h2 + (20.0 * sc).round() as usize
                        } else if expansion_type_clone.borrow().is_family_basic() {
                            let mbtn_h = (20.0 * sc).round() as usize;
                            let mrow_gap = (4.0 * sc).round() as usize;
                            let act_btn_h2 = (24.0 * sc).round() as usize;
                            tmp_g0 + btn_h / 2 + 9 * mbtn_h + 8 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h2 + (20.0 * sc).round() as usize
                        } else if expansion_type_clone.borrow().is_party_tap() {
                            let mbtn_h = (30.0 * sc).round() as usize;
                            let mrow_gap = (6.0 * sc).round() as usize;
                            let act_btn_h2 = (24.0 * sc).round() as usize;
                            tmp_g0 + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h2 + (20.0 * sc).round() as usize
                        } else if expansion_type_clone.borrow().is_pachinko() {
                            let mbtn_h = (30.0 * sc).round() as usize;
                            let mrow_gap = (6.0 * sc).round() as usize;
                            let act_btn_h2 = (24.0 * sc).round() as usize;
                            tmp_g0 + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h2 + (20.0 * sc).round() as usize
                        } else if expansion_type_clone.borrow().is_exciting_boxing() {
                            let mbtn_h = (30.0 * sc).round() as usize;
                            let mrow_gap = (6.0 * sc).round() as usize;
                            let act_btn_h2 = (24.0 * sc).round() as usize;
                            tmp_g0 + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h2 + (20.0 * sc).round() as usize
                        } else if expansion_type_clone.borrow().is_jissen_mahjong() {
                            let mbtn_h = (30.0 * sc).round() as usize;
                            let mrow_gap = (6.0 * sc).round() as usize;
                            let act_btn_h2 = (24.0 * sc).round() as usize;
                            tmp_g0 + btn_h / 2 + 7 * mbtn_h + 6 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h2 + (20.0 * sc).round() as usize
                        } else if expansion_type_clone.borrow().is_subor_keyboard() {
                            let mbtn_h = (20.0 * sc).round() as usize;
                            let mrow_gap = (4.0 * sc).round() as usize;
                            let act_btn_h2 = (24.0 * sc).round() as usize;
                            tmp_g0 + btn_h / 2 + 13 * mbtn_h + 12 * mrow_gap + btn_h + (10.0 * sc).round() as usize + act_btn_h2 + (20.0 * sc).round() as usize
                        } else {
                            (150.0 * sc).round() as usize
                        };
                        let title_h = (30.0 * sc).round() as usize;
                        let sx = (width.saturating_sub(sw)) / 2;
                        let sy = (height.saturating_sub(sh)) / 2;
                        let close_w = (20.0 * sc).round() as usize;
                        let close_h = (20.0 * sc).round() as usize;
                        let close_x = sx + sw - close_w - (10.0 * sc).round() as usize;
                        let close_y = sy + (5.0 * sc).round() as usize;
                        if point_in_rect(mx, my, close_x, close_y, close_w, close_h) {
                            drop(ms);
                            menu_state_clone.borrow_mut().show_expansion_settings = false;
                            paused_clone.store(false, Ordering::Relaxed);
                        } else if is_adapter {
                            let grid_y0 = sy + tmp_g0;
                            let act_btn_w = (70.0 * sc).round() as usize;
                            let act_btn_h = (24.0 * sc).round() as usize;
                            let act_gap = (10.0 * sc).round() as usize;
                            let act_total = 2 * act_btn_w + act_gap;
                            let act_x0 = sx + (sw.saturating_sub(act_total)) / 2;
                            let act_y = grid_y0 + dialog_sub_ports * (grid_h + btn_h / 2) + (10.0 * sc).round() as usize;
                            let clicked_clear = point_in_rect(mx, my, act_x0, act_y, act_btn_w, act_btn_h);
                            let clicked_reset = !clicked_clear && point_in_rect(mx, my, act_x0 + act_btn_w + act_gap, act_y, act_btn_w, act_btn_h);
                            let mut clicked_pair: Option<(usize, usize)> = None;
                            'outer: for local in 0..dialog_sub_ports {
                                let yoff = grid_y0 + local * (grid_h + btn_h / 2);
                                let btn_y0 = yoff + (btn_h * 3 / 4) as usize;
                                for i in 0..config::GAMEPAD_BUTTON_COUNT {
                                    let row = i / 2;
                                    let by = btn_y0 + row * (btn_h + gap_y);
                                    let bx = if i % 2 == 0 { sx + gap_x } else { sx + gap_x * 2 + btn_w };
                                    if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                        clicked_pair = Some((local, i));
                                        break 'outer;
                                    }
                                }
                            }
                            if let Some((local, btn)) = clicked_pair {
                                drop(ms);
                                let mut ms_mut = menu_state_clone.borrow_mut();
                                ms_mut.rebind_controller = Some(local as u8 + 1);
                                ms_mut.rebind_button = Some(btn);
                            } else if clicked_clear {
                                drop(ms);
                                for p in player_offset..(player_offset + dialog_sub_ports) {
                                    config::clear_expansion_adapter_bindings(p);
                                }
                                let mut e1 = expansion1_bindings_clone.borrow_mut();
                                let mut e2 = expansion2_bindings_clone.borrow_mut();
                                let mut e3 = expansion3_bindings_clone.borrow_mut();
                                let mut e4 = expansion4_bindings_clone.borrow_mut();
                                *e1 = config::load_expansion_adapter_bindings(0);
                                *e2 = config::load_expansion_adapter_bindings(1);
                                *e3 = config::load_expansion_adapter_bindings(2);
                                *e4 = config::load_expansion_adapter_bindings(3);
                            } else if clicked_reset {
                                drop(ms);
                                for p in player_offset..(player_offset + dialog_sub_ports) {
                                    config::reset_expansion_adapter_bindings(p);
                                }
                                let mut e1 = expansion1_bindings_clone.borrow_mut();
                                let mut e2 = expansion2_bindings_clone.borrow_mut();
                                let mut e3 = expansion3_bindings_clone.borrow_mut();
                                let mut e4 = expansion4_bindings_clone.borrow_mut();
                                *e1 = config::load_expansion_adapter_bindings(0);
                                *e2 = config::load_expansion_adapter_bindings(1);
                                *e3 = config::load_expansion_adapter_bindings(2);
                                *e4 = config::load_expansion_adapter_bindings(3);
                            }
                        } else {
                            let grid_y0 = sy + title_h + (10.0 * sc).round() as usize;
                            let is_family_trainer = expansion_type_clone.borrow().is_family_trainer();
                            let is_hyper_shot = expansion_type_clone.borrow().is_hyper_shot();
                            let is_family_basic = expansion_type_clone.borrow().is_family_basic();
                            let is_party_tap = expansion_type_clone.borrow().is_party_tap();
                            let is_pachinko = expansion_type_clone.borrow().is_pachinko();
                            let is_exciting_boxing = expansion_type_clone.borrow().is_exciting_boxing();
                            let is_jissen_mahjong = expansion_type_clone.borrow().is_jissen_mahjong();
                            let is_subor_keyboard = expansion_type_clone.borrow().is_subor_keyboard();
                            let is_bandai_hyper_shot = expansion_type_clone.borrow().is_bandai_hyper_shot();
                            let mut clicked_grid_btn: Option<usize> = None;
                            let bind_total = 2 * btn_w + gap_x;
                            let bind_bx = sx + (sw.saturating_sub(bind_total)) / 2;
                            let clicked_bind = if is_family_trainer {
                                let btn_gap = (6.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(5 * btn_gap)) / 4;
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                let mut found = false;
                                'outer: for r in 0..3usize {
                                    for c in 0..4usize {
                                        let i = r * 4 + c;
                                        let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                        let by = m_y0 + r * (mbtn_h + mrow_gap);
                                        if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                            clicked_grid_btn = Some(i);
                                            found = true;
                                            break 'outer;
                                        }
                                    }
                                }
                                found
                            } else if is_hyper_shot {
                                let btn_gap = (6.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(3 * btn_gap)) / 2;
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                let mut found = false;
                                'outer: for r in 0..2usize {
                                    for c in 0..2usize {
                                        let i = r * 2 + c;
                                        let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                        let by = m_y0 + r * (mbtn_h + mrow_gap);
                                        if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                            clicked_grid_btn = Some(i);
                                            found = true;
                                            break 'outer;
                                        }
                                    }
                                }
                                found
                            } else if is_bandai_hyper_shot {
                                let btn_gap = (6.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(5 * btn_gap)) / 4;
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                let mut found = false;
                                'outer: for r in 0..3usize {
                                    for c in 0..4usize {
                                        let i = r * 4 + c;
                                        if i >= config::BANDAI_HYPER_SHOT_BUTTON_COUNT { continue; }
                                        let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                        let by = m_y0 + r * (mbtn_h + mrow_gap);
                                        if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                            clicked_grid_btn = Some(i);
                                            found = true;
                                            break 'outer;
                                        }
                                    }
                                }
                                found
                            } else if is_family_basic {
                                let btn_gap = (4.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(9 * btn_gap)) / 8;
                                let mbtn_h = (20.0 * sc).round() as usize;
                                let mrow_gap = (4.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                let mut found = false;
                                'outer: for r in 0..9usize {
                                    for c in 0..8usize {
                                        let i = r * 8 + c;
                                        if i >= config::FAMILY_BASIC_BUTTON_COUNT { continue; }
                                        let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                        let by = m_y0 + r * (mbtn_h + mrow_gap);
                                        if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                            clicked_grid_btn = Some(i);
                                            found = true;
                                            break 'outer;
                                        }
                                    }
                                }
                                found
                            } else if is_party_tap {
                                let btn_gap = (6.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(4 * btn_gap)) / 3;
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                let mut found = false;
                                'outer: for r in 0..2usize {
                                    for c in 0..3usize {
                                        let i = r * 3 + c;
                                        let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                        let by = m_y0 + r * (mbtn_h + mrow_gap);
                                        if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                            clicked_grid_btn = Some(i);
                                            found = true;
                                            break 'outer;
                                        }
                                    }
                                }
                                found
                            } else if is_pachinko {
                                let btn_gap = (6.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(3 * btn_gap)) / 2;
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                let mut found = false;
                                for i in 0..config::PACHINKO_BUTTON_COUNT {
                                    let bx = sx + btn_gap + i * (mcol_w + btn_gap);
                                    let by = m_y0;
                                    if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                        clicked_grid_btn = Some(i);
                                        found = true;
                                        break;
                                    }
                                }
                                found
                            } else if is_exciting_boxing {
                                let btn_gap = (6.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(5 * btn_gap)) / 4;
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                let mut found = false;
                                'outer: for r in 0..2usize {
                                    for c in 0..4usize {
                                        let i = r * 4 + c;
                                        let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                        let by = m_y0 + r * (mbtn_h + mrow_gap);
                                        if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                            clicked_grid_btn = Some(i);
                                            found = true;
                                            break 'outer;
                                        }
                                    }
                                }
                                found
                            } else if is_jissen_mahjong {
                                let btn_gap = (6.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(4 * btn_gap)) / 3;
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                let mut found = false;
                                'outer: for r in 0..7usize {
                                    for c in 0..3usize {
                                        let i = r * 3 + c;
                                        let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                        let by = m_y0 + r * (mbtn_h + mrow_gap);
                                        if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                            clicked_grid_btn = Some(i);
                                            found = true;
                                            break 'outer;
                                        }
                                    }
                                }
                                found
                            } else if is_subor_keyboard {
                                let btn_gap = (4.0 * sc).round() as usize;
                                let mcol_w = (sw.saturating_sub(9 * btn_gap)) / 8;
                                let mbtn_h = (20.0 * sc).round() as usize;
                                let mrow_gap = (4.0 * sc).round() as usize;
                                let m_y0 = grid_y0 + (btn_h / 2);
                                let mut found = false;
                                'outer: for r in 0..13usize {
                                    for c in 0..8usize {
                                        let i = r * 8 + c;
                                        if i >= config::SUBOR_KEYBOARD_BUTTON_COUNT { continue; }
                                        let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                        let by = m_y0 + r * (mbtn_h + mrow_gap);
                                        if point_in_rect(mx, my, bx, by, mcol_w, mbtn_h) {
                                            clicked_grid_btn = Some(i);
                                            found = true;
                                            break 'outer;
                                        }
                                    }
                                }
                                found
                            } else {
                                point_in_rect(mx, my, bind_bx, grid_y0, bind_total, btn_h)
                            };
                            let act_btn_w = (70.0 * sc).round() as usize;
                            let act_btn_h = (24.0 * sc).round() as usize;
                            let act_gap = (10.0 * sc).round() as usize;
                            let act_total = 2 * act_btn_w + act_gap;
                            let act_x0 = sx + (sw.saturating_sub(act_total)) / 2;
                            let act_y = if is_family_trainer {
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                grid_y0 + btn_h / 2 + 3 * mbtn_h + 2 * mrow_gap + (10.0 * sc).round() as usize
                            } else if is_hyper_shot {
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                grid_y0 + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + (10.0 * sc).round() as usize
                            } else if is_bandai_hyper_shot {
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                grid_y0 + btn_h / 2 + 3 * mbtn_h + 2 * mrow_gap + (10.0 * sc).round() as usize
                            } else if is_family_basic {
                                let mbtn_h = (20.0 * sc).round() as usize;
                                let mrow_gap = (4.0 * sc).round() as usize;
                                grid_y0 + btn_h / 2 + 9 * mbtn_h + 8 * mrow_gap + (10.0 * sc).round() as usize
                            } else if is_party_tap {
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                grid_y0 + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + (10.0 * sc).round() as usize
                            } else if is_pachinko {
                                let mbtn_h = (30.0 * sc).round() as usize;
                                grid_y0 + btn_h / 2 + mbtn_h + (10.0 * sc).round() as usize
                            } else if is_exciting_boxing {
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                grid_y0 + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + (10.0 * sc).round() as usize
                            } else if is_jissen_mahjong {
                                let mbtn_h = (30.0 * sc).round() as usize;
                                let mrow_gap = (6.0 * sc).round() as usize;
                                grid_y0 + btn_h / 2 + 7 * mbtn_h + 6 * mrow_gap + (10.0 * sc).round() as usize
                            } else if is_subor_keyboard {
                                let mbtn_h = (20.0 * sc).round() as usize;
                                let mrow_gap = (4.0 * sc).round() as usize;
                                grid_y0 + btn_h / 2 + 13 * mbtn_h + 12 * mrow_gap + (10.0 * sc).round() as usize
                            } else {
                                grid_y0 + btn_h + (10.0 * sc).round() as usize
                            };
                            let clicked_clear = point_in_rect(mx, my, act_x0, act_y, act_btn_w, act_btn_h);
                            let clicked_reset = !clicked_clear && point_in_rect(mx, my, act_x0 + act_btn_w + act_gap, act_y, act_btn_w, act_btn_h);
                            if clicked_bind {
                                drop(ms);
                                let mut ms_mut = menu_state_clone.borrow_mut();
                                ms_mut.rebind_controller = Some(0);
                                ms_mut.rebind_button = if is_family_trainer || is_hyper_shot || is_family_basic || is_party_tap || is_pachinko || is_exciting_boxing || is_jissen_mahjong || is_subor_keyboard || is_bandai_hyper_shot { Some(clicked_grid_btn.unwrap_or(0)) } else { Some(0) };
                            } else if clicked_clear {
                                drop(ms);
                                if expansion_type_clone.borrow().is_family_trainer() {
                                    config::clear_powerpad_bindings("familytrainer");
                                    *family_trainer_bindings_clone.borrow_mut() = config::load_powerpad_bindings("familytrainer");
                                } else if expansion_type_clone.borrow().is_hyper_shot() {
                                    config::clear_hyper_shot_bindings("hypershot");
                                    *hyper_shot_bindings_clone.borrow_mut() = config::load_hyper_shot_bindings("hypershot");
                                } else if expansion_type_clone.borrow().is_bandai_hyper_shot() {
                                    config::clear_bandai_hyper_shot_bindings("bandaihypershot");
                                    *bandai_hyper_bindings_clone.borrow_mut() = config::load_bandai_hyper_shot_bindings("bandaihypershot");
                                } else if expansion_type_clone.borrow().is_family_basic() {
                                    config::clear_family_basic_bindings("familybasic");
                                    *family_basic_bindings_clone.borrow_mut() = config::load_family_basic_bindings("familybasic");
                                } else if expansion_type_clone.borrow().is_party_tap() {
                                    config::clear_party_tap_bindings("partytap");
                                    *party_tap_bindings_clone.borrow_mut() = config::load_party_tap_bindings("partytap");
                                } else if expansion_type_clone.borrow().is_pachinko() {
                                    config::clear_pachinko_bindings("pachinko");
                                    *pachinko_bindings_clone.borrow_mut() = config::load_pachinko_bindings("pachinko");
                                } else if expansion_type_clone.borrow().is_exciting_boxing() {
                                    config::clear_exciting_boxing_bindings("punchingbag");
                                    *punching_bag_bindings_clone.borrow_mut() = config::load_exciting_boxing_bindings("punchingbag");
                                } else if expansion_type_clone.borrow().is_jissen_mahjong() {
                                    config::clear_jissen_mahjong_bindings("jissenmahjong");
                                    *jissen_mahjong_bindings_clone.borrow_mut() = config::load_jissen_mahjong_bindings("jissenmahjong");
                                } else if expansion_type_clone.borrow().is_subor_keyboard() {
                                    config::clear_subor_keyboard_bindings("suborkeyboard");
                                    *subor_keyboard_bindings_clone.borrow_mut() = config::load_subor_keyboard_bindings("suborkeyboard");
                                } else if *expansion_type_clone.borrow() == config::ExpansionType::FamicomZapper {
                                    config::save_expansion_zapper_trigger("");
                                    *expansion_zapper_trigger_binding_clone.borrow_mut() = String::new();
                                } else if *expansion_type_clone.borrow() == config::ExpansionType::OekaKidsTablet {
                                    config::save_expansion_oeka_click("");
                                    *expansion_oeka_click_binding_clone.borrow_mut() = String::new();
                                } else {
                                    config::save_paddle_button("expansion", "");
                                    *expansion_paddle_button_binding_clone.borrow_mut() = String::new();
                                }
                            } else if clicked_reset {
                                drop(ms);
                                if expansion_type_clone.borrow().is_family_trainer() {
                                    config::reset_powerpad_bindings("familytrainer");
                                    *family_trainer_bindings_clone.borrow_mut() = config::load_powerpad_bindings("familytrainer");
                                } else if expansion_type_clone.borrow().is_hyper_shot() {
                                    config::reset_hyper_shot_bindings("hypershot");
                                    *hyper_shot_bindings_clone.borrow_mut() = config::load_hyper_shot_bindings("hypershot");
                                } else if expansion_type_clone.borrow().is_bandai_hyper_shot() {
                                    config::reset_bandai_hyper_shot_bindings("bandaihypershot");
                                    *bandai_hyper_bindings_clone.borrow_mut() = config::load_bandai_hyper_shot_bindings("bandaihypershot");
                                } else if expansion_type_clone.borrow().is_family_basic() {
                                    config::reset_family_basic_bindings("familybasic");
                                    *family_basic_bindings_clone.borrow_mut() = config::load_family_basic_bindings("familybasic");
                                } else if expansion_type_clone.borrow().is_party_tap() {
                                    config::reset_party_tap_bindings("partytap");
                                    *party_tap_bindings_clone.borrow_mut() = config::load_party_tap_bindings("partytap");
                                } else if expansion_type_clone.borrow().is_pachinko() {
                                    config::reset_pachinko_bindings("pachinko");
                                    *pachinko_bindings_clone.borrow_mut() = config::load_pachinko_bindings("pachinko");
                                } else if expansion_type_clone.borrow().is_exciting_boxing() {
                                    config::reset_exciting_boxing_bindings("punchingbag");
                                    *punching_bag_bindings_clone.borrow_mut() = config::load_exciting_boxing_bindings("punchingbag");
                                } else if expansion_type_clone.borrow().is_jissen_mahjong() {
                                    config::reset_jissen_mahjong_bindings("jissenmahjong");
                                    *jissen_mahjong_bindings_clone.borrow_mut() = config::load_jissen_mahjong_bindings("jissenmahjong");
                                } else if expansion_type_clone.borrow().is_subor_keyboard() {
                                    config::reset_subor_keyboard_bindings("suborkeyboard");
                                    *subor_keyboard_bindings_clone.borrow_mut() = config::load_subor_keyboard_bindings("suborkeyboard");
                                } else if *expansion_type_clone.borrow() == config::ExpansionType::FamicomZapper {
                                    config::save_expansion_zapper_trigger("MouseLeft");
                                    *expansion_zapper_trigger_binding_clone.borrow_mut() = "MouseLeft".to_string();
                                } else if *expansion_type_clone.borrow() == config::ExpansionType::OekaKidsTablet {
                                    config::save_expansion_oeka_click("MouseLeft");
                                    *expansion_oeka_click_binding_clone.borrow_mut() = "MouseLeft".to_string();
                                } else {
                                    config::save_paddle_button("expansion", "MouseLeft");
                                    *expansion_paddle_button_binding_clone.borrow_mut() = "MouseLeft".to_string();
                                }
                            }
                        }
                    } else if ms.show_controller2_settings {
                        let sc = ms.scale;
                        let c2_w = (440.0 * sc).round() as usize;
                        let title_h = (30.0 * sc).round() as usize;
                        let btn_w = (90.0 * sc).round() as usize;
                        let btn_h = (26.0 * sc).round() as usize;
                        let gap_y = (8.0 * sc).round() as usize;
                        let c2t_fs = *controller2_type_clone.borrow() == config::ControllerType::FourScore;
                        let grid_h = 4 * (btn_h + gap_y) + btn_h;
                        let tmp_g0 = title_h + (10.0 * sc).round() as usize;
                        let c2_h = if c2t_fs {
                            let fs_ch = tmp_g0 + 2 * (grid_h + btn_h / 2) + (40.0 * sc).round() as usize;
                            fs_ch.max(260)
                        } else if *controller2_type_clone.borrow() == config::ControllerType::FamicomGamepad {
                            (295.0 * sc).round() as usize
                        } else {
                            (260.0 * sc).round() as usize
                        };
                        let c2_x = (width.saturating_sub(c2_w)) / 2;
                        let c2_y = (height.saturating_sub(c2_h)) / 2;
                        let _title_h = title_h;
                        let close_w = (20.0 * sc).round() as usize;
                        let close_h = (20.0 * sc).round() as usize;
                        let close_x = c2_x + c2_w - close_w - (10.0 * sc).round() as usize;
                        let close_y = c2_y + (5.0 * sc).round() as usize;
                        if point_in_rect(mx, my, close_x, close_y, close_w, close_h) {
                            drop(ms);
                            menu_state_clone.borrow_mut().show_controller2_settings = false;
                            paused_clone.store(false, Ordering::Relaxed);
                        } else {
                            let cw = c2_w;
                            let cx = c2_x;
                            let cy = c2_y;
                            let gap_x = (cw.saturating_sub(2 * btn_w)) / 3;
                            let grid_y0 = cy + tmp_g0;
                            let mut clicked_rebind = None;
                            let c2t = *controller2_type_clone.borrow();
                            let is_single2 = c2t == config::ControllerType::Zapper || c2t == config::ControllerType::Paddle;
                            let is_snesmouse2 = c2t == config::ControllerType::SNESMouse || c2t == config::ControllerType::SuborMouse;
                            let is_pp2 = c2t == config::ControllerType::PowerPadA || c2t == config::ControllerType::PowerPadB;
                            let is_snes2 = c2t == config::ControllerType::SNESPad;
                            let is_vb2 = c2t == config::ControllerType::VirtualBoy;
                            if is_single2 {
                                let total_w = 2 * btn_w + gap_x;
                                let trig_bx = cx + (cw - total_w) / 2;
                                if point_in_rect(mx, my, trig_bx, grid_y0, total_w, btn_h) {
                                    clicked_rebind = Some(0);
                                }
                            } else if is_snesmouse2 {
                                let per_w = btn_w;
                                let half_gap = (cw.saturating_sub(2 * per_w)) / 3;
                                for i in 0..config::SNES_MOUSE_BUTTON_COUNT {
                                    let bx = cx + half_gap + i * (per_w + half_gap);
                                    if point_in_rect(mx, my, bx, grid_y0, per_w, btn_h) {
                                        clicked_rebind = Some(i);
                                        break;
                                    }
                                }
                            } else if is_pp2 {
                                let cols = 4;
                                let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                                for i in 0..config::POWERPAD_BUTTON_COUNT {
                                    let row = i / cols;
                                    let col = i % cols;
                                    let bx = cx + gap_x + col * (btn_w + gap_x);
                                    let by = grid_y0 + row * (btn_h + gap_y);
                                    if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                        clicked_rebind = Some(i);
                                        break;
                                    }
                                }
                            } else if is_snes2 {
                                let cols = 4;
                                let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                                for i in 0..config::SNES_BUTTON_COUNT {
                                    let row = i / cols;
                                    let col = i % cols;
                                    let bx = cx + gap_x + col * (btn_w + gap_x);
                                    let by = grid_y0 + row * (btn_h + gap_y);
                                    if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                        clicked_rebind = Some(i);
                                        break;
                                    }
                                }
                            } else if is_vb2 {
                                let cols = 4;
                                let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                                for i in 0..config::VB_BUTTON_COUNT {
                                    let row = i / cols;
                                    let col = i % cols;
                                    let bx = cx + gap_x + col * (btn_w + gap_x);
                                    let by = grid_y0 + row * (btn_h + gap_y);
                                    if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                        clicked_rebind = Some(i);
                                        break;
                                    }
                                }
                            } else if c2t == config::ControllerType::FourScore {
                                let block_h = 4 * (btn_h + gap_y) + btn_h;
                                for player in 0..2usize {
                                    let yoff = grid_y0 + player * (block_h + btn_h / 2);
                                    let btn_y0 = yoff + (btn_h * 3 / 4) as usize;
                                    for i in 0..config::GAMEPAD_BUTTON_COUNT {
                                        let row = i / 2;
                                        let by = btn_y0 + row * (btn_h + gap_y);
                                        let bx = if i % 2 == 0 { cx + gap_x } else { cx + gap_x * 2 + btn_w };
                                        if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                            clicked_rebind = Some(i * 4 + (player + 2));
                                            break;
                                        }
                                    }
                                    if clicked_rebind.is_some() { break; }
                                }
                            } else {
                                for i in 0..config::GAMEPAD_BUTTON_COUNT {
                                    let row = i / 2;
                                    let by = grid_y0 + row * (btn_h + gap_y);
                                    let bx = if i % 2 == 0 { cx + gap_x } else { cx + gap_x * 2 + btn_w };
                                    if point_in_rect(mx, my, bx, by, btn_w, btn_h) {
                                        clicked_rebind = Some(i);
                                        break;
                                    }
                                }
                                if c2t == config::ControllerType::FamicomGamepad && clicked_rebind.is_none() {
                                    let mic_row = 5;
                                    let mic_y = grid_y0 + mic_row * (btn_h + gap_y);
                                    let mic_x = cx + (cw - btn_w) / 2;
                                    if point_in_rect(mx, my, mic_x, mic_y, btn_w, btn_h) {
                                        clicked_rebind = Some(10);
                                    }
                                }
                            }
                            let act_btn_w = (70.0 * sc).round() as usize;
                            let act_btn_h = (24.0 * sc).round() as usize;
                            let last_row_bottom = if is_single2 || is_snesmouse2 { grid_y0 + btn_h } else if is_pp2 || is_snes2 { grid_y0 + 2 * (btn_h + gap_y) + btn_h } else if is_vb2 { grid_y0 + 3 * (btn_h + gap_y) + btn_h } else if c2t == config::ControllerType::FamicomGamepad { grid_y0 + 5 * (btn_h + gap_y) + btn_h } else { grid_y0 + 4 * (btn_h + gap_y) + btn_h };
                            let act_y = last_row_bottom + (10.0 * sc).round() as usize;
                            let act_gap = (10.0 * sc).round() as usize;
                            let act_total = 2 * act_btn_w + act_gap;
                            let act_x0 = cx + (cw - act_total) / 2;
                            let clicked_clear = point_in_rect(mx, my, act_x0, act_y, act_btn_w, act_btn_h);
                            let clicked_reset = !clicked_clear && point_in_rect(mx, my, act_x0 + act_btn_w + act_gap, act_y, act_btn_w, act_btn_h);
                            if let Some(i) = clicked_rebind {
                                drop(ms);
                                let mut ms_mut = menu_state_clone.borrow_mut();
                                if c2t == config::ControllerType::FourScore {
                                    let btn = i / 4;
                                    let player = i % 4;
                                    ms_mut.rebind_controller = Some(player as u8 + 1);
                                    ms_mut.rebind_button = Some(btn);
                                } else {
                                    ms_mut.rebind_controller = Some(2);
                                    ms_mut.rebind_button = Some(i);
                                }
                            } else if clicked_clear {
                                drop(ms);
                                if c2t == config::ControllerType::Zapper {
                                    config::save_zapper_trigger("");
                                    *zapper_trigger_binding_clone.borrow_mut() = String::new();
                                } else if c2t == config::ControllerType::Paddle {
                                    config::save_paddle_button("controller2", "");
                                    *paddle2_button_binding_clone.borrow_mut() = String::new();
                                } else if c2t == config::ControllerType::SNESMouse {
                                    config::clear_snes_mouse_bindings("controller2");
                                    *snes_mouse2_bindings_clone.borrow_mut() = config::load_snes_mouse_bindings("controller2");
                                } else if c2t == config::ControllerType::SuborMouse {
                                    config::clear_subor_mouse_bindings("controller2");
                                    *subor_mouse2_bindings_clone.borrow_mut() = config::load_subor_mouse_bindings("controller2");
                                } else if is_pp2 {
                                    config::clear_powerpad_bindings("controller2");
                                    *powerpad2_bindings_clone.borrow_mut() = config::load_powerpad_bindings("controller2");
                                } else if is_snes2 {
                                    config::clear_snes_bindings("controller2");
                                    *snes2_bindings_clone.borrow_mut() = config::load_snes_bindings("controller2");
                                } else if is_vb2 {
                                    config::clear_vb_bindings("controller2");
                                    *vb2_bindings_clone.borrow_mut() = config::load_vb_bindings("controller2");
                                } else if c2t == config::ControllerType::FourScore {
                                    for pfx in &["controller1", "controller2", "controller3", "controller4"] {
                                        config::clear_bindings(pfx);
                                    }
                                    *controller1_bindings_clone.borrow_mut() = config::load_bindings("controller1");
                                    *controller2_bindings_clone.borrow_mut() = config::load_bindings("controller2");
                                    *controller3_bindings_clone.borrow_mut() = config::load_bindings("controller3");
                                    *controller4_bindings_clone.borrow_mut() = config::load_bindings("controller4");
                                } else {
                                    config::clear_bindings("controller2");
                                    *controller2_bindings_clone.borrow_mut() = config::load_bindings("controller2");
                                }
                            } else if clicked_reset {
                                drop(ms);
                                if c2t == config::ControllerType::Zapper {
                                    config::save_zapper_trigger("MouseLeft");
                                    *zapper_trigger_binding_clone.borrow_mut() = "MouseLeft".to_string();
                                } else if c2t == config::ControllerType::Paddle {
                                    config::save_paddle_button("controller2", "MouseLeft");
                                    *paddle2_button_binding_clone.borrow_mut() = "MouseLeft".to_string();
                                } else if c2t == config::ControllerType::SNESMouse {
                                    config::reset_snes_mouse_bindings("controller2");
                                    *snes_mouse2_bindings_clone.borrow_mut() = config::load_snes_mouse_bindings("controller2");
                                } else if c2t == config::ControllerType::SuborMouse {
                                    config::reset_subor_mouse_bindings("controller2");
                                    *subor_mouse2_bindings_clone.borrow_mut() = config::load_subor_mouse_bindings("controller2");
                                } else if is_pp2 {
                                    config::reset_powerpad_bindings("controller2");
                                    *powerpad2_bindings_clone.borrow_mut() = config::load_powerpad_bindings("controller2");
                                } else if is_snes2 {
                                    config::reset_snes_bindings("controller2");
                                    *snes2_bindings_clone.borrow_mut() = config::load_snes_bindings("controller2");
                                } else if is_vb2 {
                                    config::reset_vb_bindings("controller2");
                                    *vb2_bindings_clone.borrow_mut() = config::load_vb_bindings("controller2");
                                } else if c2t == config::ControllerType::FourScore {
                                    for pfx in &["controller1", "controller2", "controller3", "controller4"] {
                                        config::reset_bindings(pfx);
                                    }
                                    *controller1_bindings_clone.borrow_mut() = config::load_bindings("controller1");
                                    *controller2_bindings_clone.borrow_mut() = config::load_bindings("controller2");
                                    *controller3_bindings_clone.borrow_mut() = config::load_bindings("controller3");
                                    *controller4_bindings_clone.borrow_mut() = config::load_bindings("controller4");
                                } else {
                                    config::reset_bindings("controller2");
                                    *controller2_bindings_clone.borrow_mut() = config::load_bindings("controller2");
                                }
                            }
                        }
                    } else if ms.show_video_settings {
                        let sc = ms.scale;
                        let vw = (400.0 * sc).round() as usize;
                        let gap = (8.0 * sc).round() as usize;
                        let title_h = (30.0 * sc).round() as usize;
                        let row_h = (22.0 * sc).round() as usize;
                        let border_thickness = (2.0 * sc).round() as usize;
                        let video_items = 4;
                        let vh = title_h + gap + video_items * (row_h + gap) - gap + border_thickness * 2;
                        let vx = (width.saturating_sub(vw)) / 2;
                        let vy = (height.saturating_sub(vh)) / 2;
                        let close_w = (20.0 * sc).round() as usize;
                        let close_h = (20.0 * sc).round() as usize;
                        let close_x = vx + vw - close_w - (10.0 * sc).round() as usize;
                        let close_y = vy + (5.0 * sc).round() as usize;
                        if point_in_rect(mx, my, close_x, close_y, close_w, close_h) {
                            drop(ms);
                            menu_state_clone.borrow_mut().show_video_settings = false;
                            paused_clone.store(false, Ordering::Relaxed);
                        } else {
                            let box_w = (80.0 * sc).round() as usize;
                            let mut row_y = vy + title_h + gap;
                            let box_x = vx + vw - (15.0 * sc).round() as usize - box_w;

                            if point_in_rect(mx, my, box_x, row_y, box_w, row_h) {
                                let new_val = !*fullscreen_clone.borrow();
                                *fullscreen_clone.borrow_mut() = new_val;
                                config::save_fullscreen(new_val);
                                if new_val {
                                    window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                                } else {
                                    window.set_fullscreen(None);
                                }
                            }
                            row_y += row_h + gap;
                            if point_in_rect(mx, my, box_x, row_y, box_w, row_h) {
                                let new_val = !*fullscreen_on_game_load_clone.borrow();
                                *fullscreen_on_game_load_clone.borrow_mut() = new_val;
                                config::save_fullscreen_on_game_load(new_val);
                            }
                            row_y += row_h + gap;
                            if point_in_rect(mx, my, box_x, row_y, box_w, row_h) {
                                let new_val = !*hide_mouse_cursor_clone.borrow();
                                *hide_mouse_cursor_clone.borrow_mut() = new_val;
                                config::save_hide_mouse_cursor(new_val);
                                window.set_cursor_visible(!new_val);
                            }
                            row_y += row_h + gap;
                            if point_in_rect(mx, my, box_x, row_y, box_w, row_h) {
                                let new_val = !*crop_overscan_clone.borrow();
                                *crop_overscan_clone.borrow_mut() = new_val;
                                config::save_crop_overscan(new_val);
                            }
                        }
                    } else if ms.show_input_settings {
                        let sc = ms.scale;
                    let input_w = (480.0 * sc).round() as usize;
                    let input_h_4p = (410.0 * sc).round() as usize;
                    let input_h = if *expansion_adapter_type_clone.borrow() == config::ExpansionAdapterType::FourPlayer { input_h_4p } else { (380.0 * sc).round() as usize };
                        let input_x = (width.saturating_sub(input_w)) / 2;
                        let input_y = (height.saturating_sub(input_h)) / 2;
                        let title_h = (30.0 * sc).round() as usize;
                        let close_w = (20.0 * sc).round() as usize;
                        let close_h = (20.0 * sc).round() as usize;
                        let close_x = input_x + input_w - close_w - (10.0 * sc).round() as usize;
                        let close_y = input_y + (5.0 * sc).round() as usize;
                        if point_in_rect(mx, my, close_x, close_y, close_w, close_h) {
                            drop(ms);
                            menu_state_clone.borrow_mut().show_input_settings = false;
                            paused_clone.store(false, Ordering::Relaxed);
                        } else {
                            let row_h = (22.0 * sc).round() as usize;
                            let box_w = (165.0 * sc).round() as usize;
                            let configure_h = (24.0 * sc).round() as usize;
                            let col_gap = (10.0 * sc).round() as usize;
                            let col_w = (input_w - (10.0 * sc).round() as usize * 2 - col_gap) / 2;
                            let col1_x = input_x + (10.0 * sc).round() as usize;
                            let col2_x = col1_x + col_w + col_gap;
                            let content_y = input_y + title_h + (10.0 * sc).round() as usize;
                            let cfg_y = content_y + row_h + (5.0 * sc).round() as usize;
                            let type_label_w = ((5 * 8) as f32 * sc).round() as usize;
                            let label_gap = (6.0 * sc).round() as usize;
                            let c1_cfg_x = col1_x + type_label_w + label_gap;
                            let c2_cfg_x = col2_x + type_label_w + label_gap;
                            let type_y = cfg_y + configure_h + (8.0 * sc).round() as usize;
                            let t1_box_x = c1_cfg_x;
                            let t2_box_x = c2_cfg_x;
                            if point_in_rect(mx, my, c1_cfg_x, cfg_y, box_w, configure_h) {
                                let c1t = *controller1_type_clone.borrow();
                                if c1t != config::ControllerType::None {
                                    drop(ms);
                                    menu_state_clone.borrow_mut().show_controller1_settings = true;
                                }
                            } else if point_in_rect(mx, my, c2_cfg_x, cfg_y, box_w, configure_h) {
                                let c2t = *controller2_type_clone.borrow();
                                if c2t != config::ControllerType::None {
                                    drop(ms);
                                    menu_state_clone.borrow_mut().show_controller2_settings = true;
                                }
                            } else if point_in_rect(mx, my, t1_box_x, type_y, box_w, row_h) {
                                let mut ct = controller1_type_clone.borrow_mut();
                                let prev = *ct;
                                *ct = ct.next();
                                if *ct == config::ControllerType::FourScore {
                                    *controller2_type_clone.borrow_mut() = config::ControllerType::FourScore;
                                    config::save_controller_type("controller2_type", config::ControllerType::FourScore);
                                    emu_clone.lock().unwrap().controller2_type = config::ControllerType::FourScore;
                                } else if prev == config::ControllerType::FourScore {
                                    *controller2_type_clone.borrow_mut() = config::ControllerType::None;
                                    config::save_controller_type("controller2_type", config::ControllerType::None);
                                    emu_clone.lock().unwrap().controller2_type = config::ControllerType::None;
                                }
                                config::save_controller_type("controller1_type", *ct);
                                emu_clone.lock().unwrap().controller1_type = *ct;
                            } else if point_in_rect(mx, my, t2_box_x, type_y, box_w, row_h) {
                                let mut ct = controller2_type_clone.borrow_mut();
                                let prev = *ct;
                                *ct = ct.next();
                                if *ct == config::ControllerType::FourScore {
                                    *controller1_type_clone.borrow_mut() = config::ControllerType::FourScore;
                                    config::save_controller_type("controller1_type", config::ControllerType::FourScore);
                                    emu_clone.lock().unwrap().controller1_type = config::ControllerType::FourScore;
                                } else if prev == config::ControllerType::FourScore {
                                    *controller1_type_clone.borrow_mut() = config::ControllerType::None;
                                    config::save_controller_type("controller1_type", config::ControllerType::None);
                                    emu_clone.lock().unwrap().controller1_type = config::ControllerType::None;
                                }
                                config::save_controller_type("controller2_type", *ct);
                                emu_clone.lock().unwrap().controller2_type = *ct;
                            } else {
                                let exp_title_y = type_y + row_h + (12.0 * sc).round() as usize;
                                let exp_cfg_y = exp_title_y + row_h + (5.0 * sc).round() as usize;
                                let exp_cfg_x = input_x + input_w.saturating_sub(box_w) / 2;
                                let exp_port_type = {
                                    let dt = *expansion_type_clone.borrow();
                                    let at = *expansion_adapter_type_clone.borrow();
                                    match at {
                                        config::ExpansionAdapterType::TwoPlayer => config::ExpansionPortType::TwoPlayerAdapter,
                                        config::ExpansionAdapterType::FourPlayer => config::ExpansionPortType::FourPlayerAdapter,
                                         config::ExpansionAdapterType::None => match dt {
                                             config::ExpansionType::ArkanoidPaddle => config::ExpansionPortType::ArkanoidPaddle,
                                             config::ExpansionType::FamicomZapper => config::ExpansionPortType::FamicomZapper,
                                             config::ExpansionType::OekaKidsTablet => config::ExpansionPortType::OekaKidsTablet,
                                             config::ExpansionType::FamilyTrainerA => config::ExpansionPortType::FamilyTrainerA,
                                             config::ExpansionType::FamilyTrainerB => config::ExpansionPortType::FamilyTrainerB,
                                             config::ExpansionType::KonamiHyperShot => config::ExpansionPortType::KonamiHyperShot,
                                             config::ExpansionType::FamilyBasicKeyboard => config::ExpansionPortType::FamilyBasicKeyboard,
                                             config::ExpansionType::PartyTap => config::ExpansionPortType::PartyTap,
                                             config::ExpansionType::PachinkoController => config::ExpansionPortType::PachinkoController,
                                             config::ExpansionType::ExcitingBoxing => config::ExpansionPortType::ExcitingBoxing,
                                             config::ExpansionType::JissenMahjong => config::ExpansionPortType::JissenMahjong,
                                             config::ExpansionType::SuborKeyboard => config::ExpansionPortType::SuborKeyboard,
                                             config::ExpansionType::BarcodeBattler => config::ExpansionPortType::BarcodeBattler,
                                             config::ExpansionType::HoriTrack => config::ExpansionPortType::HoriTrack,
                                             config::ExpansionType::BandaiHyperShot => config::ExpansionPortType::BandaiHyperShot,
                                             config::ExpansionType::TurboFile => config::ExpansionPortType::TurboFile,
                                             config::ExpansionType::BattleBox => config::ExpansionPortType::BattleBox,
                                             config::ExpansionType::None => config::ExpansionPortType::None,
                                         },
                                    }
                                };
                                let is_4p = exp_port_type == config::ExpansionPortType::FourPlayerAdapter;
                                let exp_type_y = if is_4p {
                                    let cfg34_y = exp_cfg_y + configure_h + (4.0 * sc).round() as usize;
                                    cfg34_y + configure_h + (8.0 * sc).round() as usize
                                } else {
                                    exp_cfg_y + configure_h + (8.0 * sc).round() as usize
                                };
                                let exp_type_box_x = exp_cfg_x;
                                let dpad_y = exp_type_y + row_h + (15.0 * sc).round() as usize;
                                let dpad_box_x = c2_cfg_x;
                                if is_4p && point_in_rect(mx, my, exp_cfg_x, exp_cfg_y, box_w, configure_h) {
                                    drop(ms);
                                    menu_state_clone.borrow_mut().adapter_pair = Some(0);
                                    menu_state_clone.borrow_mut().show_expansion_settings = true;
                                } else if is_4p && point_in_rect(mx, my, exp_cfg_x, exp_cfg_y + configure_h + (4.0 * sc).round() as usize, box_w, configure_h) {
                                    drop(ms);
                                    menu_state_clone.borrow_mut().adapter_pair = Some(1);
                                    menu_state_clone.borrow_mut().show_expansion_settings = true;
                                } else if !is_4p && point_in_rect(mx, my, exp_cfg_x, exp_cfg_y, box_w, configure_h) {
                                    if exp_port_type != config::ExpansionPortType::None && exp_port_type != config::ExpansionPortType::BarcodeBattler && exp_port_type != config::ExpansionPortType::TurboFile && exp_port_type != config::ExpansionPortType::BattleBox {
                                        drop(ms);
                                        menu_state_clone.borrow_mut().show_expansion_settings = true;
                                    }
                                } else if point_in_rect(mx, my, exp_type_box_x, exp_type_y, box_w, row_h) {
                                    let cur = exp_port_type;
                                    let nxt = cur.next();
                                    let (dev, adap) = config::save_expansion_port_type(nxt);
                                    *expansion_type_clone.borrow_mut() = dev;
                                    *expansion_adapter_type_clone.borrow_mut() = adap;
                                    emu_clone.lock().unwrap().expansion_type = dev;
                                    emu_clone.lock().unwrap().expansion_adapter_type = adap;
                                    if nxt.is_adapter() {
                                        let mut c2t = controller2_type_clone.borrow_mut();
                                        if *c2t != config::ControllerType::None {
                                            *c2t = config::ControllerType::None;
                                            config::save_controller_type("controller2_type", config::ControllerType::None);
                                            emu_clone.lock().unwrap().controller2_type = config::ControllerType::None;
                                        }
                                    } else if cur.is_adapter() {
                                        let mut c2t = controller2_type_clone.borrow_mut();
                                        *c2t = config::ControllerType::Gamepad;
                                        config::save_controller_type("controller2_type", config::ControllerType::Gamepad);
                                        emu_clone.lock().unwrap().controller2_type = config::ControllerType::Gamepad;
                                    }
                                } else if point_in_rect(mx, my, dpad_box_x, dpad_y, box_w, row_h) {
                                    let new_val = !*allow_opposing_dpad_clone.borrow();
                                    *allow_opposing_dpad_clone.borrow_mut() = new_val;
                                    config::save_allow_opposing_dpad(new_val);
                                } else if point_in_rect(mx, my, col1_x, dpad_y + row_h + (10.0 * sc).round() as usize, (20.0 * sc).round() as usize, (20.0 * sc).round() as usize) {
                                    let new_val = !*auto_detect_game_controller_clone.borrow();
                                    *auto_detect_game_controller_clone.borrow_mut() = new_val;
                                    config::save_auto_detect_game_controller(new_val);
                                }
                            }
                        }
                    } else if ms.show_error {
                        let error_w = (400.0 * ms.scale).round() as usize;
                        let error_h = (100.0 * ms.scale).round() as usize;
                        let error_x = (width.saturating_sub(error_w)) / 2;
                        let error_y = (height.saturating_sub(error_h)) / 2;
                        let close_w = (20.0 * ms.scale).round() as usize;
                        let close_h = (20.0 * ms.scale).round() as usize;
                        let close_x = error_x + error_w - close_w - (10.0 * ms.scale).round() as usize;
                        let close_y = error_y + (10.0 * ms.scale).round() as usize;
                        if point_in_rect(mx, my, close_x, close_y, close_w, close_h) {
                            drop(ms);
                            menu_state_clone.borrow_mut().show_error = false;
                        }
                    } else if my < ms.menu_height {
                        let menu_names = [("FILE", Menu::File), ("NES", Menu::Nes), ("OPTIONS", Menu::Options), ("HELP", Menu::Help)];
                        let item_w = width / 4;
                        
                        for (i, (_, menu)) in menu_names.iter().enumerate() {
                            let current_x = i * item_w;
                            if point_in_rect(mx, my, current_x, 0, item_w, ms.menu_height) {
                                drop(ms);
                                let mut ms_mut = menu_state_clone.borrow_mut();
                                if ms_mut.active_menu == Some(*menu) {
                                    ms_mut.active_menu = None;
                                } else {
                                    ms_mut.active_menu = Some(*menu);
                                    ms_mut.show_recent_submenu = false;
                                    ms_mut.show_region_submenu = false;
                                }
                                break;
                            }
                        }
                    } else if let Some(active) = ms.active_menu {
                        drop(ms);
                        let mut ms_mut = menu_state_clone.borrow_mut();
                        let item_w = width / 4;
                        let dropdown_x = match active {
                            Menu::File => 0,
                            Menu::Nes => item_w,
                            Menu::Options => 2 * item_w,
                            Menu::Help => 3 * item_w,
                        };
                        match active {
                            Menu::File => {
                                let dropdown_w = item_w;
                                let dropdown_y = ms_mut.menu_height;
                                let sc = ms_mut.scale;
                                let submenu_x = dropdown_x + dropdown_w;
                                let submenu_w = (150.0 * sc).round() as usize;
                                let recent_w = (200.0 * sc).round() as usize;
                                let slot_h = (16.0 * sc).round() as usize;

                                let file_items = ["Open", "Close", "Recent", "Quick Save", "Quick Load", "Save State", "Load State", "Exit"];
                                let file_positions = calculate_item_positions(&file_items, dropdown_x, dropdown_y, dropdown_w, sc);
                                
                                let recent_anchor_y = dropdown_y + file_positions[0].3 + file_positions[1].3;
                                let save_anchor_y = dropdown_y + file_positions[0].3 + file_positions[1].3 + file_positions[2].3 + file_positions[3].3 + file_positions[4].3;
                                let load_anchor_y = dropdown_y + file_positions[0].3 + file_positions[1].3 + file_positions[2].3 + file_positions[3].3 + file_positions[4].3 + file_positions[5].3;

                                if ms_mut.show_recent_submenu {
                                    let roms = recent_roms_clone.borrow();
                                    let recent_positions = calculate_submenu_positions(roms.len(), submenu_x, recent_anchor_y, recent_w, slot_h);
                                    for (i, (x, y, w, h)) in recent_positions.iter().enumerate() {
                                        if point_in_rect(mx, my, *x, *y, *w, *h) {
                                            let path_str = roms[i].clone();
                                            match cartridge::Cartridge::from_file(&path_str) {
                                                     Ok(cart) => {
                                                    let crc = cart.prg_rom_crc32;
                                                    let mapper_id = cart.memory_mapper;
                                                    let dip_needed = load_dip_game(crc, mapper_id);
                                                    auto_apply_input(&cart);
                                                    let _ = cmd_tx.send(EmuCommand::LoadCartridge(cart));
                                                    let _ = cmd_tx.send(EmuCommand::PowerCycle(*initial_ram_clone.borrow()));
                                                    *rom_loaded_clone.borrow_mut() = true;
                                                    rom_loaded_flag_clone.store(true, Ordering::Relaxed);
                                                    *current_rom_clone.borrow_mut() = Some(path_str.clone());
                                                     if let Some(game) = dip_needed {
                                                         let dip_val = 0u8;
                                                         let new_val = game.settings.iter().fold(dip_val as u32, |val, s| (val & !s.mask) | s.default_val);
                                                         let variant = compute_vs_ppu_variant(&game, new_val as u8, crc);
                                                         let _ = cmd_tx.send(EmuCommand::SetDipSwitches(new_val as u8));
                                                         let _ = cmd_tx.send(EmuCommand::SetVsPpuVariant(variant));
                                                         ms_mut.dip_definition = Some(game);
                                                         ms_mut.show_dip_switches = true;
                                                         paused_clone.store(true, Ordering::Relaxed);
                                                     }
                                                     
                                                     if *fullscreen_on_game_load_clone.borrow() {
                                                        window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                                                    }
                                                    window.set_cursor_visible(!*hide_mouse_cursor_clone.borrow());
                                                    
                                                    drop(roms);
                                                    let mut roms_mut = recent_roms_clone.borrow_mut();
                                                    if let Some(pos) = roms_mut.iter().position(|r| r == &path_str) {
                                                        roms_mut.remove(pos);
                                                    }
                                                    roms_mut.insert(0, path_str);
                                                    roms_mut.truncate(8);
                                                    let _ = std::fs::write(".recent_roms", roms_mut.join("\n"));
                                                }
                                                Err(e) => {
                                                    ms_mut.show_error = true;
                                                    ms_mut.error_message = e;
                                                    drop(roms);
                                                }
                                            }
                                            ms_mut.show_recent_submenu = false;
                                            ms_mut.active_menu = None;
                                            break;
                                        }
                                    }
                                }
                                if ms_mut.show_save_state_submenu {
                                    let save_positions = calculate_submenu_positions(9, submenu_x, save_anchor_y, submenu_w, slot_h);
                                    for (slot, (x, y, w, h)) in save_positions.iter().enumerate() {
                                        if point_in_rect(mx, my, *x, *y, *w, *h) {
                                            if *rom_loaded_clone.borrow() {
                                                if let Some(ref rom_path) = *current_rom_clone.borrow() {
                                                    let state_data = emu_clone.lock().unwrap().save_state_to_bytes();
                                                    let state_path = config::state_file_path(rom_path, slot + 1);
                                                    if let Some(parent) = state_path.parent() {
                                                        let _ = std::fs::create_dir_all(parent);
                                                    }
                                                    if let Err(e) = std::fs::write(&state_path, &state_data) {
                                                        eprintln!("Failed to save state to {}: {}", state_path.display(), e);
                                                    } else {
                                                        println!("Saved state to {}", state_path.display());
                                                    }
                                                }
                                            }
                                            ms_mut.show_save_state_submenu = false;
                                            ms_mut.active_menu = None;
                                            break;
                                        }
                                    }
                                }
                                if ms_mut.show_load_state_submenu {
                                    let load_positions = calculate_submenu_positions(9, submenu_x, load_anchor_y, submenu_w, slot_h);
                                    for (slot, (x, y, w, h)) in load_positions.iter().enumerate() {
                                        if point_in_rect(mx, my, *x, *y, *w, *h) {
                                            if *rom_loaded_clone.borrow() {
                                                if let Some(ref rom_path) = *current_rom_clone.borrow() {
                                                    let state_path = config::state_file_path(rom_path, slot + 1);
                                                    if state_path.exists() {
                                                         match std::fs::read(&state_path) {
                                                             Ok(state_data) => {
                                                                 if let Err(e) = emu_clone.lock().unwrap().load_state_from_bytes(&state_data) {
                                                                     eprintln!("Failed to load state: {}", e);
                                                                 } else {
                                                                     println!("Loaded state from {}", state_path.display());
                                                                 }
                                                             }
                                                             Err(e) => {
                                                                 eprintln!("Failed to read state from {}: {}", state_path.display(), e);
                                                             }
                                                         }
                                                    }
                                                }
                                            }
                                            ms_mut.show_load_state_submenu = false;
                                            ms_mut.active_menu = None;
                                            break;
                                        }
                                    }
                                }
                                let file_menu_items = [
                                    FileMenuItem::Open,
                                    FileMenuItem::Close,
                                    FileMenuItem::Recent,
                                    FileMenuItem::QuickSave,
                                    FileMenuItem::QuickLoad,
                                    FileMenuItem::SaveState,
                                    FileMenuItem::LoadState,
                                    FileMenuItem::Exit,
                                ];
                                for (i, (x, y, w, h)) in file_positions.iter().enumerate() {
                                    if point_in_rect(mx, my, *x, *y, *w, *h) {
                                        match file_menu_items[i] {
                                            FileMenuItem::Open => {
                                                if let Some(path) = rfd::FileDialog::new()
                                                    .add_filter("All ROMs", &["nes", "unif", "unf", "fds", "qd", "wxn", "studybox", "study"])
                                                    .add_filter("NES ROMs", &["nes", "unif", "unf"])
                                                    .add_filter("FDS / QD ROMs", &["fds", "qd"])
                                                    .add_filter("Waixing ROMs", &["wxn"])
                                                    .add_filter("Study Box Tapes", &["studybox", "study"])
                                                    .pick_file() {
                                                    let path_str = path.to_string_lossy().to_string();
                                                    match cartridge::Cartridge::from_file(&path_str) {
                                                        Ok(cart) => {
                                                    let crc = cart.prg_rom_crc32;
                                                    let mapper_id = cart.memory_mapper;
                                                    let dip_needed = load_dip_game(crc, mapper_id);
                                                    auto_apply_input(&cart);
                                                    let _ = cmd_tx.send(EmuCommand::LoadCartridge(cart));
                                                    let _ = cmd_tx.send(EmuCommand::PowerCycle(*initial_ram_clone.borrow()));
                                                    *rom_loaded_clone.borrow_mut() = true;
                                                    rom_loaded_flag_clone.store(true, Ordering::Relaxed);
                                                    *current_rom_clone.borrow_mut() = Some(path_str.clone());
                                                             if let Some(game) = dip_needed {
                                                                 let dip_val = 0u8;
                                                                 let new_val = game.settings.iter().fold(dip_val as u32, |val, s| (val & !s.mask) | s.default_val);
                                                                 let variant = compute_vs_ppu_variant(&game, new_val as u8, crc);
                                                                 let _ = cmd_tx.send(EmuCommand::SetDipSwitches(new_val as u8));
                                                                 let _ = cmd_tx.send(EmuCommand::SetVsPpuVariant(variant));
                                                                 ms_mut.dip_definition = Some(game);
                                                                 ms_mut.show_dip_switches = true;
                                                                 paused_clone.store(true, Ordering::Relaxed);
                                                             }

                                                            if *fullscreen_on_game_load_clone.borrow() {
                                                                window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                                                            }
                                                            window.set_cursor_visible(!*hide_mouse_cursor_clone.borrow());
                                                            
                                                            let mut roms = recent_roms_clone.borrow_mut();
                                                            if let Some(pos) = roms.iter().position(|r| r == &path_str) {
                                                                roms.remove(pos);
                                                            }
                                                            roms.insert(0, path_str);
                                                            roms.truncate(8);
                                                            let _ = std::fs::write(".recent_roms", roms.join("\n"));
                                                        }
                                                        Err(e) => {
                                                            ms_mut.show_error = true;
                                                            ms_mut.error_message = e;
                                                        }
                                                    }
                                                }
                                            }
                                            FileMenuItem::Close => {
                                                if *auto_save_sram_clone.borrow() {
                                                    let _ = cmd_tx.send(EmuCommand::SavePrgRam);
                                                }
                                                let _ = cmd_tx.send(EmuCommand::ClearCart);
                                                *rom_loaded_clone.borrow_mut() = false;
                                                rom_loaded_flag_clone.store(false, Ordering::Relaxed);
                                                *current_rom_clone.borrow_mut() = None;
                                            }
                                            FileMenuItem::Exit => {
                                                if *confirm_on_exit_clone.borrow() && *rom_loaded_clone.borrow() {
                                                    ms_mut.show_confirm_exit_dialog = true;
                                                    paused_clone.store(true, Ordering::Relaxed);
                                                    window.request_redraw();
                                                } else {
                                                    if *auto_save_sram_clone.borrow() && *rom_loaded_clone.borrow() {
                                                        let _ = cmd_tx.send(EmuCommand::SavePrgRam);
                                                    }
                                                    *control_flow = ControlFlow::Exit;
                                                }
                                            }
                                            FileMenuItem::Recent => {
                                                ms_mut.show_recent_submenu = !ms_mut.show_recent_submenu;
                                                ms_mut.show_save_state_submenu = false;
                                                ms_mut.show_load_state_submenu = false;
                                            }
                                            FileMenuItem::QuickSave => {
                                                if *rom_loaded_clone.borrow() {
                                                    let state_data = emu_clone.lock().unwrap().save_state_to_bytes();
                                                    *quick_save_slot_clone.borrow_mut() = Some(state_data);
                                                    println!("Quick saved successfully");
                                                }
                                            }
                                            FileMenuItem::QuickLoad => {
                                                if *rom_loaded_clone.borrow() {
                                                    if let Some(ref state_data) = *quick_save_slot_clone.borrow() {
                                                        if let Err(e) = emu_clone.lock().unwrap().load_state_from_bytes(state_data) {
                                                            eprintln!("Failed to quick load: {}", e);
                                                        } else {
                                                            println!("Quick loaded successfully");
                                                        }
                                                    }
                                                }
                                            }
                                            FileMenuItem::SaveState => {
                                                ms_mut.show_save_state_submenu = !ms_mut.show_save_state_submenu;
                                                ms_mut.show_recent_submenu = false;
                                                ms_mut.show_load_state_submenu = false;
                                            }
                                            FileMenuItem::LoadState => {
                                                ms_mut.show_load_state_submenu = !ms_mut.show_load_state_submenu;
                                                ms_mut.show_recent_submenu = false;
                                                ms_mut.show_save_state_submenu = false;
                                            }
                                        }
                                        if file_menu_items[i] != FileMenuItem::Recent && file_menu_items[i] != FileMenuItem::SaveState && file_menu_items[i] != FileMenuItem::LoadState {
                                            ms_mut.active_menu = None;
                                        }
                                        break;
                                    }
                                }
                            }
                            Menu::Nes => {
                                let dropdown_w = item_w;
                                let dropdown_y = ms_mut.menu_height;
                                let sc = ms_mut.scale;
                                let pause_text = if paused_clone.load(Ordering::Relaxed) { "Resume" } else { "Pause" };
                                let disk_label = if *rom_loaded_clone.borrow() && disk_inserted_clone.load(Ordering::Relaxed) {
                                    "Eject Disk"
                                } else {
                                    "Insert Disk"
                                };
                                let nes_items = [pause_text, "DIP Switches", "Insert Coin 1", "Insert Coin 2", "Service Button", disk_label, "Swap Disk", "Input Barcode", "Reset", "Power Cycle"];
                                let nes_positions = calculate_item_positions(&nes_items, dropdown_x, dropdown_y, dropdown_w, sc);
                                let nes_menu_items = [NesMenuItem::Pause, NesMenuItem::DipSwitches, NesMenuItem::InsertCoin1, NesMenuItem::InsertCoin2, NesMenuItem::ServiceButton, NesMenuItem::InsertEjectDisk, NesMenuItem::SwapDisk, NesMenuItem::InputBarcode, NesMenuItem::Reset, NesMenuItem::PowerCycle];
                                
                                for (i, (x, y, w, h)) in nes_positions.iter().enumerate() {
                                    if point_in_rect(mx, my, *x, *y, *w, *h) {
                                        match nes_menu_items[i] {
                                            NesMenuItem::Pause => {
                                                paused_clone.store(!paused_clone.load(Ordering::Relaxed), Ordering::Relaxed);
                                            }
                                            NesMenuItem::DipSwitches => {
                                                   if *rom_loaded_clone.borrow() {
                                                       let dip_info = emu_clone.lock().ok().map(|e| (e.has_dip_switches(), e.memory_mapper(), e.prg_rom_crc32(), e.get_dip_switches()));
                                                       if let Some((has_dip, mapper_id, crc, dip_val)) = dip_info {
                                                           if has_dip {
                                                               let game = load_dip_game(crc, mapper_id);
                                                               if let Some(ref g) = game {
                                                                   let variant = compute_vs_ppu_variant(g, dip_val, crc);
                                                                   let _ = cmd_tx.send(EmuCommand::SetVsPpuVariant(variant));
                                                               }
                                                          ms_mut.dip_definition = game;
                                                           ms_mut.show_dip_switches = true;
                                                           paused_clone.store(true, Ordering::Relaxed);
                                                       }
                                                   }
                                               }
                                             }
                                            NesMenuItem::InsertCoin1 => {
                                                if *rom_loaded_clone.borrow() {
                                                    let _ = cmd_tx.send(EmuCommand::InsertCoin(0));
                                                }
                                            }
                                            NesMenuItem::InsertCoin2 => {
                                                if *rom_loaded_clone.borrow() {
                                                    let _ = cmd_tx.send(EmuCommand::InsertCoin(1));
                                                }
                                            }
                                            NesMenuItem::ServiceButton => {
                                                if *rom_loaded_clone.borrow() {
                                                    let _ = cmd_tx.send(EmuCommand::ServiceButton);
                                                }
                                            }
                                            NesMenuItem::InsertEjectDisk => {
                                                if *rom_loaded_clone.borrow() {
                                                    if disk_inserted_clone.load(Ordering::Relaxed) {
                                                        let _ = cmd_tx.send(EmuCommand::EjectDisk);
                                                        disk_inserted_clone.store(false, Ordering::Relaxed);
                                                    } else {
                                                        let _ = cmd_tx.send(EmuCommand::InsertDisk);
                                                        disk_inserted_clone.store(true, Ordering::Relaxed);
                                                    }
                                                }
                                            }
                                            NesMenuItem::SwapDisk => {
                                                if *rom_loaded_clone.borrow() {
                                                    let _ = cmd_tx.send(EmuCommand::ChangeDisk);
                                                }
                                            }
                                            NesMenuItem::InputBarcode => {
                                                if *rom_loaded_clone.borrow() {
                                                    barcode_ctrl_clone.store(false, Ordering::Relaxed);
                                                    ms_mut.barcode_input.clear();
                                                    ms_mut.barcode_caret = 0;
                                                    ms_mut.barcode_sel_anchor = None;
                                                    ms_mut.barcode_dragging = false;
                                                    ms_mut.show_barcode_input = true;
                                                    paused_clone.store(true, Ordering::Relaxed);
                                                }
                                            }
                                            NesMenuItem::Reset => {
                                                if *rom_loaded_clone.borrow() {
                                                    let _ = cmd_tx.send(EmuCommand::Reset);
                                                }
                                            }
                                            NesMenuItem::PowerCycle => {
                                                if *rom_loaded_clone.borrow() {
                                                    let _ = cmd_tx.send(EmuCommand::PowerCycle(*initial_ram_clone.borrow()));
                                                }
                                            }
                                        }
                                        ms_mut.active_menu = None;
                                        break;
                                    }
                                }
                            }
                            Menu::Help => {
                                let dropdown_w = item_w;
                                let dropdown_y = ms_mut.menu_height;
                                let sc = ms_mut.scale;
                                let help_items = ["Check for Updates", "About"];
                                let help_positions = calculate_item_positions(&help_items, dropdown_x, dropdown_y, dropdown_w, sc);
                                
                                if let Some((x, y, w, h)) = help_positions.get(0) {
                                    if point_in_rect(mx, my, *x, *y, *w, *h) {
                                        ms_mut.show_updates = true;
                                        ms_mut.active_menu = None;
                                        paused_clone.store(true, Ordering::Relaxed);
                                        let update_state = update_check_state.clone();
                                        thread::spawn(move || {
                                            {
                                                let mut state = update_state.lock().unwrap();
                                                *state = UpdateCheckState::Checking;
                                            }
                                            let tls = match ureq::native_tls::TlsConnector::new() {
                                                Ok(c) => c,
                                                Err(e) => {
                                                    let mut state = update_state.lock().unwrap();
                                                    *state = UpdateCheckState::Error(format!("TLS init failed: {}", e));
                                                    return;
                                                }
                                            };
                                            let agent = ureq::AgentBuilder::new()
                                                .tls_connector(std::sync::Arc::new(tls))
                                                .build();
                                            match agent
                                                .get("https://api.github.com/repos/ammaroussema/accunes/releases/latest")
                                                .set("User-Agent", "AccuNES")
                                                .call()
                                            {
                                                Ok(response) => {
                                                    match response.into_json::<serde_json::Value>() {
                                                        Ok(json) => {
                                                            let tag = json["tag_name"].as_str().unwrap_or("unknown").to_string();
                                                            let raw_date = json["published_at"].as_str().unwrap_or("").to_string();
                                                            let date = if raw_date.len() >= 10 { raw_date[..10].to_string() } else { raw_date };
                                                            let url = json["html_url"].as_str().unwrap_or("https://github.com/ammaroussema/accunes/releases/latest").to_string();
                                                            let mut state = update_state.lock().unwrap();
                                                            let tag_clean = tag.trim_start_matches('v');
                                                            if version_compare(tag_clean, APP_VERSION) == std::cmp::Ordering::Greater {
                                                                *state = UpdateCheckState::Available { version: tag, date, url };
                                                            } else {
                                                                *state = UpdateCheckState::UpToDate;
                                                            }
                                                        }
                                                        Err(e) => {
                                                            let mut state = update_state.lock().unwrap();
                                                            *state = UpdateCheckState::Error(format!("Parse error: {}", e));
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    let mut state = update_state.lock().unwrap();
                                                    *state = UpdateCheckState::Error(format!("Request failed: {}", e));
                                                }
                                            }
                                        });
                                    }
                                }
                                if let Some((x, y, w, h)) = help_positions.get(1) {
                                    if point_in_rect(mx, my, *x, *y, *w, *h) {
                                        ms_mut.show_about = true;
                                        ms_mut.active_menu = None;
                                        paused_clone.store(true, Ordering::Relaxed);
                                    }
                                }
                            }
                            Menu::Options => {
                                let dropdown_w = item_w;
                                let dropdown_y = ms_mut.menu_height;
                                let sc = ms_mut.scale;
                                let submenu_x = dropdown_x + dropdown_w;
                                let submenu_w = (150.0 * sc).round() as usize;
                                let submenu_item_h = (16.0 * sc).round() as usize;

                                let options_items = ["General", "Input", "Audio", "Video", "Region", "Set FDS BIOS", "Set Study Box BIOS"];
                                let options_positions = calculate_item_positions(&options_items, dropdown_x, dropdown_y, dropdown_w, sc);

                                if ms_mut.show_region_submenu {
                                    let region_anchor_y = dropdown_y;
                                    let region_positions = calculate_submenu_positions(4, submenu_x, region_anchor_y, submenu_w, submenu_item_h);
                                    let mut clicked = false;
                                    for (i, (x, y, w, h)) in region_positions.iter().enumerate() {
                                        if point_in_rect(mx, my, *x, *y, *w, *h) {
                                            match i {
                                                0 => {
                                                    let _ = cmd_tx.send(EmuCommand::SetRegionPreference(Region::Ntsc));
                                                    config::save_region(Region::Ntsc);
                                                    ms_mut.current_region = Region::Ntsc;
                                                }
                                                1 => {
                                                    let _ = cmd_tx.send(EmuCommand::SetRegionPreference(Region::Pal));
                                                    config::save_region(Region::Pal);
                                                    ms_mut.current_region = Region::Pal;
                                                }
                                                2 => {
                                                    let _ = cmd_tx.send(EmuCommand::SetRegionPreference(Region::Dendy));
                                                    config::save_region(Region::Dendy);
                                                    ms_mut.current_region = Region::Dendy;
                                                }
                                                _ => {
                                                    let _ = cmd_tx.send(EmuCommand::SetRegionPreference(Region::Auto));
                                                    config::save_region(Region::Auto);
                                                    ms_mut.current_region = Region::Auto;
                                                }
                                            }
                                            ms_mut.show_region_submenu = false;
                                            ms_mut.active_menu = None;
                                            clicked = true;
                                            break;
                                        }
                                    }
                                    if !clicked {
                                        if let Some((x, y, w, h)) = options_positions.get(4) {
                                            if point_in_rect(mx, my, *x, *y, *w, *h) {
                                                ms_mut.show_region_submenu = false;
                                            }
                                        }
                                    }
                                } else {
                                    if let Some((x, y, w, h)) = options_positions.get(0) {
                                        if point_in_rect(mx, my, *x, *y, *w, *h) {
                                            ms_mut.show_general_settings = true;
                                            ms_mut.active_menu = None;
                                            paused_clone.store(true, Ordering::Relaxed);
                                        }
                                    }
                                    if let Some((x, y, w, h)) = options_positions.get(1) {
                                        if point_in_rect(mx, my, *x, *y, *w, *h) {
                                            ms_mut.show_input_settings = true;
                                            ms_mut.active_menu = None;
                                            paused_clone.store(true, Ordering::Relaxed);
                                        }
                                    }
                                    if let Some((x, y, w, h)) = options_positions.get(2) {
                                        if point_in_rect(mx, my, *x, *y, *w, *h) {
                                            ms_mut.show_audio_settings = true;
                                            ms_mut.active_menu = None;
                                            paused_clone.store(true, Ordering::Relaxed);
                                        }
                                    }
                                    if let Some((x, y, w, h)) = options_positions.get(3) {
                                        if point_in_rect(mx, my, *x, *y, *w, *h) {
                                            ms_mut.show_video_settings = true;
                                            ms_mut.active_menu = None;
                                            paused_clone.store(true, Ordering::Relaxed);
                                        }
                                    }
                                    if let Some((x, y, w, h)) = options_positions.get(4) {
                                        if point_in_rect(mx, my, *x, *y, *w, *h) {
                                            ms_mut.show_region_submenu = true;
                                        }
                                    }
                                    if let Some((x, y, w, h)) = options_positions.get(5) {
                                        if point_in_rect(mx, my, *x, *y, *w, *h) {
                                            if let Some(path) = rfd::FileDialog::new()
                                                .add_filter("FDS BIOS", &["rom", "bin"])
                                                .pick_file() {
                                                let path_str = path.to_string_lossy().to_string();
                                                config::save_fds_bios_path(&path_str);
                                            }
                                            ms_mut.active_menu = None;
                                        }
                                    }
                                    if let Some((x, y, w, h)) = options_positions.get(6) {
                                        if point_in_rect(mx, my, *x, *y, *w, *h) {
                                            if let Some(path) = rfd::FileDialog::new()
                                                .add_filter("Study Box BIOS", &["rom", "bin"])
                                                .pick_file() {
                                                let path_str = path.to_string_lossy().to_string();
                                                config::save_study_box_bios_path(&path_str);
                                            }
                                            ms_mut.active_menu = None;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if !pressed && button == winit::event::MouseButton::Left {
                    let mut ms = menu_state_clone.borrow_mut();
                    ms.dragging_audio_slider = None;
                    ms.barcode_dragging = false;
                }
            }

            WinitEvent::MainEventsCleared => {
                let (mx, my) = menu_state_clone.borrow().mouse_pos;
                let mut ms_mut = menu_state_clone.borrow_mut();
                let window_size = window.inner_size();
                let width = window_size.width as usize;
                let height = window_size.height as usize;
                let item_w = width / 4;
                
                if ms_mut.show_dip_switches {
                    let sc = ms_mut.scale;
                    ms_mut.dip_hovered_bit = None;
                    
                    if let Some(ref game) = ms_mut.dip_definition {
                        let max_name_len = game.settings.iter().map(|s| s.name.len()).max().unwrap_or(0);
                        let max_choice_len = game.settings.iter().flat_map(|s| s.choices.iter().map(|c| c.name.len())).max().unwrap_or(0);
                        let label_w = (max_name_len as f32) * 8.0 * sc;
                        let label_area = (label_w + (25.0 * sc)).max(225.0 * sc);
                        let choice_w_tmp = (max_choice_len as f32) * 8.0 * sc + (20.0 * sc);
                        let choice_area = choice_w_tmp.max(120.0 * sc);
                        let natural_w = (label_area + choice_area + (15.0 * sc)).max(320.0 * sc);
                        let max_dialog_w = (width as f32 - (80.0 * sc)).max(320.0 * sc);
                        let (final_w, final_label_area, lines) = if natural_w > max_dialog_w {
                            let cap_w = max_dialog_w;
                            let choice_area_f = choice_area.max(120.0 * sc);
                            let label_area_f = (cap_w - choice_area_f - (15.0 * sc)).max(180.0 * sc);
                            let avail_label_w = label_area_f - (15.0 * sc) - (5.0 * sc);
                            let cpl = (avail_label_w / (8.0 * sc)).floor() as usize;
                            let cpl = cpl.max(16);
                            let l: Vec<usize> = game.settings.iter().map(|s| ((s.name.len() + cpl - 1) / cpl).max(1)).collect();
                            (cap_w.round() as usize, label_area_f.round() as usize, l)
                        } else {
                            let l = vec![1; game.settings.len()];
                            (natural_w.round() as usize, label_area.round() as usize, l)
                        };
                        let row_h_base = 35.0 * sc;
                        let line_h = 10.0 * sc;
                        let total_extra: f32 = lines.iter().map(|&n| (n as f32 - 1.0) * line_h).sum();
                        let h = (50.0 * sc + row_h_base * game.settings.len() as f32 + total_extra).max(120.0 * sc).round() as usize;
                        
                        let dialog_x = (width.saturating_sub(final_w)) / 2;
                        let dialog_y = (height.saturating_sub(h)) / 2;
                        
                        let choice_w = final_w - final_label_area - (15.0 * sc).round() as usize;
                        let choice_h = (24.0 * sc).round() as usize;
                        let choice_x = dialog_x + final_label_area;
                        
                        for i in 0..game.settings.len() {
                            let row_extra: f32 = (0..i).map(|j| (lines.get(j).copied().unwrap_or(1) as f32 - 1.0) * line_h).sum();
                            let row_y = dialog_y + (45.0 * sc + i as f32 * row_h_base + row_extra).round() as usize;
                            if point_in_rect(mx, my, choice_x, row_y, choice_w, choice_h) {
                                ms_mut.dip_hovered_bit = Some(i as u8);
                                break;
                            }
                        }
                    } else {
                        let dialog_w = (320.0 * sc).round() as usize;
                        let dialog_h = (260.0 * sc).round() as usize;
                        let dialog_x = (width.saturating_sub(dialog_w)) / 2;
                        let dialog_y = (height.saturating_sub(dialog_h)) / 2;
                        let row_start_y = dialog_y + (45.0 * sc).round() as usize;
                        let row_h = (25.0 * sc).round() as usize;
                        let cb_w = (16.0 * sc).round() as usize;
                        let cb_h = (16.0 * sc).round() as usize;
                        let cb_x = dialog_x + dialog_w - cb_w - (25.0 * sc).round() as usize;

                        for bit_idx in 0..8 {
                            let cb_y = row_start_y + bit_idx * row_h;
                            if point_in_rect(mx, my, cb_x, cb_y, cb_w, cb_h) {
                                ms_mut.dip_hovered_bit = Some(bit_idx as u8);
                                break;
                            }
                        }
                    }
                    ms_mut.hovered_menu = None;
                    ms_mut.hovered_file_item = None;
                    ms_mut.hovered_nes_item = None;
                    ms_mut.hovered_help_index = None;
                    ms_mut.hovered_recent_index = None;
                } else if my < ms_mut.menu_height {
                    let menu_names = [("FILE", Menu::File), ("NES", Menu::Nes), ("OPTIONS", Menu::Options), ("HELP", Menu::Help)];
                    
                    ms_mut.hovered_menu = None;
                    for (i, (_, menu)) in menu_names.iter().enumerate() {
                        let current_x = i * item_w;
                        if point_in_rect(mx, my, current_x, 0, item_w, ms_mut.menu_height) {
                            ms_mut.hovered_menu = Some(*menu);
                            break;
                        }
                    }
                } else if let Some(active) = ms_mut.active_menu {
                    let dropdown_y = ms_mut.menu_height;
                    let dropdown_x = match active {
                        Menu::File => 0,
                        Menu::Nes => item_w,
                        Menu::Options => 2 * item_w,
                        Menu::Help => 3 * item_w,
                    };
                    match active {
                        Menu::File => {
                            let sc = ms_mut.scale;
                            let dropdown_w = item_w;
                            let submenu_x = dropdown_x + dropdown_w;
                            let submenu_w = (150.0 * sc).round() as usize;
                            let recent_w = (200.0 * sc).round() as usize;
                            let slot_h = (16.0 * sc).round() as usize;

                            let file_items = ["Open", "Close", "Recent", "Quick Save", "Quick Load", "Save State", "Load State", "Exit"];
                            let file_positions = calculate_item_positions(&file_items, dropdown_x, dropdown_y, dropdown_w, sc);
                            
                            let recent_anchor_y = dropdown_y + file_positions[0].3 + file_positions[1].3;
                            let save_anchor_y = dropdown_y + file_positions[0].3 + file_positions[1].3 + file_positions[2].3 + file_positions[3].3 + file_positions[4].3;
                            let load_anchor_y = dropdown_y + file_positions[0].3 + file_positions[1].3 + file_positions[2].3 + file_positions[3].3 + file_positions[4].3 + file_positions[5].3;

                            let file_menu_items = [
                                FileMenuItem::Open,
                                FileMenuItem::Close,
                                FileMenuItem::Recent,
                                FileMenuItem::QuickSave,
                                FileMenuItem::QuickLoad,
                                FileMenuItem::SaveState,
                                FileMenuItem::LoadState,
                                FileMenuItem::Exit,
                            ];

                            if ms_mut.show_recent_submenu {
                                let roms = recent_roms_clone.borrow();
                                ms_mut.hovered_file_item = None;
                                ms_mut.hovered_recent_index = None;
                                ms_mut.hovered_save_slot = None;
                                ms_mut.hovered_load_slot = None;
                                let recent_positions = calculate_submenu_positions(roms.len(), submenu_x, recent_anchor_y, recent_w, slot_h);
                                for (i, (x, y, w, h)) in recent_positions.iter().enumerate() {
                                    if point_in_rect(mx, my, *x, *y, *w, *h) {
                                        ms_mut.hovered_recent_index = Some(i);
                                        break;
                                    }
                                }
                            } else if ms_mut.show_save_state_submenu {
                                ms_mut.hovered_file_item = None;
                                ms_mut.hovered_recent_index = None;
                                ms_mut.hovered_save_slot = None;
                                ms_mut.hovered_load_slot = None;
                                let save_positions = calculate_submenu_positions(9, submenu_x, save_anchor_y, submenu_w, slot_h);
                                for (slot, (x, y, w, h)) in save_positions.iter().enumerate() {
                                    if point_in_rect(mx, my, *x, *y, *w, *h) {
                                        ms_mut.hovered_save_slot = Some(slot + 1);
                                        break;
                                    }
                                }
                            } else if ms_mut.show_load_state_submenu {
                                ms_mut.hovered_file_item = None;
                                ms_mut.hovered_recent_index = None;
                                ms_mut.hovered_save_slot = None;
                                ms_mut.hovered_load_slot = None;
                                let load_positions = calculate_submenu_positions(9, submenu_x, load_anchor_y, submenu_w, slot_h);
                                for (slot, (x, y, w, h)) in load_positions.iter().enumerate() {
                                    if point_in_rect(mx, my, *x, *y, *w, *h) {
                                        ms_mut.hovered_load_slot = Some(slot + 1);
                                        break;
                                    }
                                }
                            } else {
                                ms_mut.hovered_recent_index = None;
                                ms_mut.hovered_save_slot = None;
                                ms_mut.hovered_load_slot = None;
                                ms_mut.hovered_file_item = None;
                                for (i, (x, y, w, h)) in file_positions.iter().enumerate() {
                                    if point_in_rect(mx, my, *x, *y, *w, *h) {
                                        ms_mut.hovered_file_item = Some(file_menu_items[i]);
                                        break;
                                    }
                                }
                            }
                        }
                        Menu::Nes => {
                            let sc = ms_mut.scale;
                            let dropdown_w = item_w;
                            let pause_text = if paused_clone.load(Ordering::Relaxed) { "Resume" } else { "Pause" };
                            let disk_label = if *rom_loaded_clone.borrow() && disk_inserted_clone.load(Ordering::Relaxed) {
                                "Eject Disk"
                            } else {
                                "Insert Disk"
                            };
                            let nes_items = [pause_text, "DIP Switches", "Insert Coin 1", "Insert Coin 2", "Service Button", disk_label, "Swap Disk", "Input Barcode", "Reset", "Power Cycle"];
                            let nes_positions = calculate_item_positions(&nes_items, dropdown_x, dropdown_y, dropdown_w, sc);
                            let nes_menu_items = [NesMenuItem::Pause, NesMenuItem::DipSwitches, NesMenuItem::InsertCoin1, NesMenuItem::InsertCoin2, NesMenuItem::ServiceButton, NesMenuItem::InsertEjectDisk, NesMenuItem::SwapDisk, NesMenuItem::InputBarcode, NesMenuItem::Reset, NesMenuItem::PowerCycle];
                            
                            ms_mut.hovered_nes_item = None;
                            for (i, (x, y, w, h)) in nes_positions.iter().enumerate() {
                                if point_in_rect(mx, my, *x, *y, *w, *h) {
                                    ms_mut.hovered_nes_item = Some(nes_menu_items[i]);
                                    break;
                                }
                            }
                        }
                        Menu::Help => {
                            let sc = ms_mut.scale;
                            let dropdown_w = item_w;
                            let help_items = ["Check for Updates", "About"];
                            let help_positions = calculate_item_positions(&help_items, dropdown_x, dropdown_y, dropdown_w, sc);

                            ms_mut.hovered_help_index = None;
                            for (i, (x, y, w, h)) in help_positions.iter().enumerate() {
                                if point_in_rect(mx, my, *x, *y, *w, *h) {
                                    ms_mut.hovered_help_index = Some(i);
                                    break;
                                }
                            }
                        }
                        Menu::Options => {
                            let sc = ms_mut.scale;
                            let dropdown_w = item_w;
                            let submenu_x = dropdown_x + dropdown_w;
                            let submenu_w = (150.0 * sc).round() as usize;
                            let submenu_item_h = (16.0 * sc).round() as usize;

                            let options_items = ["General", "Input", "Audio", "Video", "Region", "Set FDS BIOS", "Set Study Box BIOS"];
                            let options_positions = calculate_item_positions(&options_items, dropdown_x, dropdown_y, dropdown_w, sc);

                            ms_mut.hovered_options_index = None;
                            for (i, (x, y, w, h)) in options_positions.iter().enumerate() {
                                if point_in_rect(mx, my, *x, *y, *w, *h) {
                                    ms_mut.hovered_options_index = Some(i);
                                    break;
                                }
                            }

                            if ms_mut.show_region_submenu {
                                ms_mut.hovered_region_item = None;
                                ms_mut.hovered_region_index = None;
                                let region_anchor_y = dropdown_y;
                                let region_positions = calculate_submenu_positions(4, submenu_x, region_anchor_y, submenu_w, submenu_item_h);
                                for (i, (x, y, w, h)) in region_positions.iter().enumerate() {
                                    if point_in_rect(mx, my, *x, *y, *w, *h) {
                                        ms_mut.hovered_region_index = Some(i);
                                        break;
                                    }
                                }
                            } else {
                                ms_mut.hovered_region_index = None;
                                ms_mut.hovered_region_item = None;
                            }
                        }
                    }
                } else {
                    ms_mut.hovered_menu = None;
                    ms_mut.hovered_file_item = None;
                    ms_mut.hovered_nes_item = None;
                    ms_mut.hovered_help_index = None;
                    ms_mut.hovered_region_item = None;
                    ms_mut.hovered_region_index = None;
                    ms_mut.hovered_options_index = None;
                    ms_mut.hovered_recent_index = None;
                }
                
                let cur_frame = emu_frame_count_ui.load(Ordering::Relaxed);
                let mouse_changed = ms_mut.mouse_pos != last_rendered_mouse;
                let menu_active = ms_mut.hovered_menu.is_some()
                    || ms_mut.active_menu.is_some()
                    || ms_mut.show_about
                    || ms_mut.show_updates
                    || ms_mut.show_dip_switches
                    || ms_mut.show_general_settings
                    || ms_mut.show_audio_settings
                    || ms_mut.show_video_settings
                    || ms_mut.show_input_settings
                    || ms_mut.show_controller1_settings
                    || ms_mut.show_controller2_settings
                    || ms_mut.show_expansion_settings
                    || ms_mut.show_confirm_exit_dialog
                    || ms_mut.show_barcode_input
                    || ms_mut.show_error
                    || ms_mut.rebind_button.is_some();
                let menu_changed = menu_active != last_menu_active;
                if cur_frame != last_rendered_frame || mouse_changed || menu_changed || menu_active {
                    last_rendered_frame = cur_frame;
                    last_rendered_mouse = ms_mut.mouse_pos;
                    last_menu_active = menu_active;
                    window.request_redraw();
                }
            }

            WinitEvent::RedrawRequested(_) => {

                let fps_elapsed = fps_update_time_clone.borrow().elapsed();
                if fps_elapsed.as_secs() >= 1 {
                    let fps = current_fps_clone.load(Ordering::Relaxed);
                    *fps_update_time_clone.borrow_mut() = std::time::Instant::now();
                    let base_title = if let Some(ref rom_path) = *current_rom_clone.borrow() {
                        let mut filename = std::path::Path::new(rom_path)
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| rom_path.clone());
                        
                        let lower = filename.to_lowercase();
                        if lower.ends_with(".nes") || lower.ends_with(".unif") || lower.ends_with(".unf") || lower.ends_with(".wxn") {
                            filename.truncate(filename.len() - 4);
                        } else if lower.ends_with(".fds") || lower.ends_with(".qd") || lower.ends_with(".studybox") || lower.ends_with(".study") {
                            filename.truncate(filename.len() - 4);
                        }
                        format!("AccuNES 1.6.4: {}", filename)
                    } else {
                        "AccuNES 1.6.4".to_string()
                    };
                    let title = if *fps_mode_clone.borrow() == config::FpsMode::Window {
                        format!("{} - {} FPS", base_title, fps)
                    } else {
                        base_title
                    };
                    window.set_title(&title);
                }

                let window_size = window.inner_size();
                let width = window_size.width as usize;
                let height = window_size.height as usize;
                
                let scale = (height as f32 / 480.0).max(0.525);
                let menu_height = (BASE_MENU_HEIGHT as f32 * scale).round() as usize;
                
                menu_state_clone.borrow_mut().scale = scale;
                menu_state_clone.borrow_mut().menu_height = menu_height;

                if auto_check_started.load(Ordering::Relaxed) {
                    let state = update_check_state.lock().unwrap();
                    if !matches!(*state, UpdateCheckState::Checking) {
                        drop(state);
                        auto_check_started.store(false, Ordering::Relaxed);
                        let mut state = update_check_state.lock().unwrap();
                        match &*state {
                            UpdateCheckState::Available { .. } => {
                                let mut ms = menu_state_clone.borrow_mut();
                                ms.show_updates = true;
                                paused_clone.store(true, Ordering::Relaxed);
                            }
                            _ => {
                                *state = UpdateCheckState::Idle;
                            }
                        }
                    }
                }

                let mut surface_ref = surface_clone.borrow_mut();
                surface_ref.resize(
                    std::num::NonZeroU32::new(width.max(1) as u32).unwrap(),
                    std::num::NonZeroU32::new(height.max(1) as u32).unwrap()
                ).expect("Failed to resize surface");
                let mut buffer = surface_ref.buffer_mut().expect("Failed to get buffer");

                let theme = menu_state_clone.borrow().theme.clone();
                let colors = match theme.as_str() {
                    "light" => LIGHT_COLORS,
                    "classicnes" => CLASSIC_NES_COLORS,
                    "famicom" => FAMICOM_COLORS,
                    "mario" => MARIO_COLORS,
                    "link" => LINK_COLORS,
                    "metroid" => METROID_COLORS,
                    "megaman" => MEGAMAN_COLORS,
                    _ => DARK_COLORS,
                };

                buffer.fill(colors.global_bg);

                if *rom_loaded_clone.borrow() && height > menu_height && width > 0 {
                    let screen_data = screen_buffer_clone.lock().unwrap();
                    let nes_width = NES_WIDTH as usize;
                    let nes_height = NES_HEIGHT as usize;

                    let available_height = height - menu_height;
                    let nes_aspect = nes_width as f32 / nes_height as f32;
                    let screen_aspect = width as f32 / available_height as f32;

                    let (dest_w, dest_h, dest_x, dest_y) = if screen_aspect > nes_aspect {
                        let scale_h = available_height;
                        let scale_w = ((available_height as f32) * nes_aspect).round() as usize;
                        let offset_x = width.saturating_sub(scale_w) / 2;
                        (scale_w, scale_h, offset_x, menu_height)
                    } else {
                        let scale_w = width;
                        let scale_h = ((width as f32) / nes_aspect).round() as usize;
                        let offset_y = menu_height + available_height.saturating_sub(scale_h) / 2;
                        (scale_w, scale_h, 0, offset_y)
                    };
                    menu_state_clone.borrow_mut().screen_dest_x = dest_x;
                    menu_state_clone.borrow_mut().screen_dest_y = dest_y;
                    menu_state_clone.borrow_mut().screen_dest_w = dest_w;
                    menu_state_clone.borrow_mut().screen_dest_h = dest_h;

                    let crop_enabled = *crop_overscan_clone.borrow();
                    let crop = if crop_enabled { 8usize } else { 0 };
                    let usable_w = nes_width - 2 * crop;
                    let usable_h = nes_height - 2 * crop;
                    let mut x_mapping = Vec::with_capacity(dest_w);
                    for dx in 0..dest_w {
                        if crop_enabled {
                            x_mapping.push(crop + (dx * usable_w) / dest_w);
                        } else {
                            x_mapping.push((dx * nes_width) / dest_w);
                        }
                    }

                    for dy in 0..dest_h {
                        let sy = if crop_enabled {
                            crop + (dy * usable_h) / dest_h
                        } else {
                            (dy * nes_height) / dest_h
                        };
                        if sy >= nes_height { continue; }
                        let screen_y = dest_y + dy;
                        if screen_y >= height { continue; }
                        
                        let dest_row_offset = screen_y * width;
                        let src_row_offset = sy * nes_width;
                        let dest_start = dest_row_offset + dest_x;

                        if dest_start + dest_w <= buffer.len() {
                            for dx in 0..dest_w {
                                let sx = x_mapping[dx];
                                let pixel = screen_data[src_row_offset + sx];
                                buffer[dest_start + dx] = 0xFF000000u32 | (pixel & 0x00FFFFFFu32);
                            }
                        }
                    }
                }

                let menu_text = colors.menu_text;
                let menu_highlight = colors.menu_highlight;
                draw_rect(&mut buffer, 0, 0, width, menu_height, width, colors.menu_bg);
                
                let menu_names = [("FILE", Menu::File), ("NES", Menu::Nes), ("OPTIONS", Menu::Options), ("HELP", Menu::Help)];
                let item_w = width / 4;
                for (i, (name, menu)) in menu_names.iter().enumerate() {
                    let current_x = i * item_w;
                    let ms = menu_state_clone.borrow();
                    if ms.hovered_menu == Some(*menu) || ms.active_menu == Some(*menu) {
                        draw_rect(&mut buffer, current_x, 0, item_w, menu_height, width, menu_highlight);
                    }
                    drop(ms);
                    
                    let text_w = name.len() as f32 * 8.0 * scale;
                    let text_x = current_x + ((item_w as f32 - text_w) / 2.0).round() as usize;
                    let text_y = ((menu_height as f32 - 8.0 * scale) / 2.0).round() as usize;
                    draw_text(&mut buffer, text_x, text_y, width, name, menu_text, scale);
                }
                
                let ms = menu_state_clone.borrow();
                if let Some(active) = ms.active_menu {
                    let dropdown_bg = colors.dropdown_bg;
                    let dropdown_y = menu_height;
                    let dropdown_x = match active {
                        Menu::File => 0,
                        Menu::Nes => item_w,
                        Menu::Options => 2 * item_w,
                        Menu::Help => 3 * item_w,
                    };
                    match active {
                        Menu::File => {
                            let items: &[(&str, FileMenuItem)] = &[
                                ("Open", FileMenuItem::Open),
                                ("Close", FileMenuItem::Close),
                                ("Recent", FileMenuItem::Recent),
                                ("Quick Save", FileMenuItem::QuickSave),
                                ("Quick Load", FileMenuItem::QuickLoad),
                                ("Save State", FileMenuItem::SaveState),
                                ("Load State", FileMenuItem::LoadState),
                                ("Exit", FileMenuItem::Exit),
                            ];
                            let dropdown_w = item_w;
                            let pad_x = (8.0 * scale).round() as usize;
                            let pad_y = (4.0 * scale).round() as usize;
                            let arrow_w = (16.0 * scale).round() as usize;
                            let has_arrow = |item: &FileMenuItem| matches!(item, FileMenuItem::Recent | FileMenuItem::SaveState | FileMenuItem::LoadState);
                            let text_max_w = |item: &FileMenuItem| if has_arrow(item) { dropdown_w.saturating_sub(pad_x + arrow_w) } else { dropdown_w.saturating_sub(pad_x * 2) };
                            let item_heights: Vec<usize> = items.iter().map(|(name, item)| {
                                let h = measure_wrapped_height(name, text_max_w(item), scale);
                                h + pad_y * 2
                            }).collect();
                            let dropdown_h: usize = item_heights.iter().sum();
                            draw_rect(&mut buffer, dropdown_x, dropdown_y, dropdown_w, dropdown_h, width, dropdown_bg);
                            let mut item_y = dropdown_y;
                            for ((name, item), &ih) in items.iter().zip(item_heights.iter()) {
                                if ms.hovered_file_item == Some(*item) {
                                    draw_rect(&mut buffer, dropdown_x, item_y, dropdown_w, ih, width, menu_highlight);
                                }
                                let enabled = match item {
                                    FileMenuItem::QuickLoad => quick_save_slot_clone.borrow().is_some() && *rom_loaded_clone.borrow(),
                                    FileMenuItem::QuickSave | FileMenuItem::SaveState | FileMenuItem::LoadState => *rom_loaded_clone.borrow(),
                                    FileMenuItem::Close => *rom_loaded_clone.borrow(),
                                    _ => true,
                                };
                                let text_color = if enabled { menu_text } else { colors.disabled_text };
                                draw_text_wrapped(&mut buffer, dropdown_x + pad_x, item_y + pad_y, text_max_w(item), width, name, text_color, scale);
                                if has_arrow(item) {
                                    let arrow_x = (dropdown_x + dropdown_w).saturating_sub(arrow_w);
                                    draw_text(&mut buffer, arrow_x, item_y + pad_y, width, ">", text_color, scale);
                                }
                                item_y += ih;
                            }

                            if ms.show_recent_submenu {
                                let recent_x = dropdown_x + dropdown_w;
                                let recent_anchor_y = dropdown_y + item_heights[0..2].iter().sum::<usize>();
                                let roms = recent_roms_clone.borrow();
                                let recent_item_h = (16.0 * scale).round() as usize;
                                let recent_h = roms.len() * recent_item_h;
                                let recent_w = (200.0 * scale).round() as usize;
                                draw_rect(&mut buffer, recent_x, recent_anchor_y, recent_w, recent_h, width, dropdown_bg);
                                for (i, rom_path) in roms.iter().enumerate() {
                                    let iy = recent_anchor_y + i * recent_item_h;
                                    if ms.hovered_recent_index == Some(i) {
                                        draw_rect(&mut buffer, recent_x, iy, recent_w, recent_item_h, width, menu_highlight);
                                    }
                                    let filename = std::path::Path::new(rom_path)
                                        .file_name()
                                        .map(|s| s.to_string_lossy().to_string())
                                        .unwrap_or_default();
                                    let mut cut_idx = filename.len();
                                    let lower_filename = filename.to_lowercase();
                                    for ext in &[".nes", ".fds", ".qd", ".unif", ".unf", ".wxn", ".studybox", ".study"] {
                                        if let Some(idx) = lower_filename.find(ext) { cut_idx = cut_idx.min(idx); }
                                    }
                                    for ch in &['(', '['] {
                                        if let Some(idx) = filename.find(*ch) { cut_idx = cut_idx.min(idx); }
                                    }
                                    let base_name = filename[..cut_idx].trim();
                                    let display_name: String = base_name.chars().take(20).collect();
                                    draw_text_wrapped(&mut buffer, recent_x + pad_x, iy + pad_y, recent_w.saturating_sub(pad_x * 2), width, &display_name, menu_text, scale);
                                }
                            }

                            if ms.show_save_state_submenu {
                                let save_x = dropdown_x + dropdown_w;
                                let save_anchor_y = dropdown_y + item_heights[0..5].iter().sum::<usize>();
                                let submenu_w = (150.0 * scale).round() as usize;
                                let slot_h = (16.0 * scale).round() as usize;
                                let save_h = 9 * slot_h;
                                draw_rect(&mut buffer, save_x, save_anchor_y, submenu_w, save_h, width, dropdown_bg);
                                for slot in 1..=9usize {
                                    let iy = save_anchor_y + (slot - 1) * slot_h;
                                    if ms.hovered_save_slot == Some(slot) {
                                        draw_rect(&mut buffer, save_x, iy, submenu_w, slot_h, width, menu_highlight);
                                    }
                                    let slot_name = format!("Slot {}", slot);
                                    draw_text(&mut buffer, save_x + pad_x, iy + pad_y, width, &slot_name, menu_text, scale);
                                }
                            }

                            if ms.show_load_state_submenu {
                                let load_x = dropdown_x + dropdown_w;
                                let load_anchor_y = dropdown_y + item_heights[0..6].iter().sum::<usize>();
                                let submenu_w = (150.0 * scale).round() as usize;
                                let slot_h = (16.0 * scale).round() as usize;
                                let load_h = 9 * slot_h;
                                draw_rect(&mut buffer, load_x, load_anchor_y, submenu_w, load_h, width, dropdown_bg);
                                let current_rom_opt = current_rom_clone.borrow();
                                for slot in 1..=9usize {
                                    let iy = load_anchor_y + (slot - 1) * slot_h;
                                    if ms.hovered_load_slot == Some(slot) {
                                        draw_rect(&mut buffer, load_x, iy, submenu_w, slot_h, width, menu_highlight);
                                    }
                                    let exists = if let Some(ref rom_path) = *current_rom_opt {
                                        config::state_file_path(rom_path, slot).exists()
                                    } else { false };
                                    let slot_name = if exists { format!("Slot {} (Used)", slot) } else { format!("Slot {} (Empty)", slot) };
                                    let text_color = if exists { menu_text } else { colors.disabled_text };
                                    draw_text(&mut buffer, load_x + pad_x, iy + pad_y, width, &slot_name, text_color, scale);
                                }
                            }
                        }
                        Menu::Nes => {
                            let pause_text = if paused_clone.load(Ordering::Relaxed) { "Resume" } else { "Pause" };
                            let disk_label = if disk_inserted_clone.load(Ordering::Relaxed) {
                                "Eject Disk"
                            } else {
                                "Insert Disk"
                            };
                            let items: &[(&str, NesMenuItem)] = &[
                                (pause_text, NesMenuItem::Pause),
                                ("DIP Switches", NesMenuItem::DipSwitches),
                                ("Insert Coin 1", NesMenuItem::InsertCoin1),
                                ("Insert Coin 2", NesMenuItem::InsertCoin2),
                                ("Service Button", NesMenuItem::ServiceButton),
    (disk_label, NesMenuItem::InsertEjectDisk),
    ("Swap Disk", NesMenuItem::SwapDisk),
    ("Input Barcode", NesMenuItem::InputBarcode),
    ("Reset", NesMenuItem::Reset),
    ("Power Cycle", NesMenuItem::PowerCycle),
];
                            let dropdown_w = item_w;
                            let pad_x = (8.0 * scale).round() as usize;
                            let pad_y = (4.0 * scale).round() as usize;
                            let text_max_w = dropdown_w.saturating_sub(pad_x * 2);
                            let item_heights: Vec<usize> = items.iter().map(|(name, _)| {
                                measure_wrapped_height(name, text_max_w, scale) + pad_y * 2
                            }).collect();
                            let dropdown_h: usize = item_heights.iter().sum();
                            draw_rect(&mut buffer, dropdown_x, dropdown_y, dropdown_w, dropdown_h, width, dropdown_bg);
                            let mut item_y = dropdown_y;
                            for ((name, item), &ih) in items.iter().zip(item_heights.iter()) {
                                if ms.hovered_nes_item == Some(*item) {
                                    draw_rect(&mut buffer, dropdown_x, item_y, dropdown_w, ih, width, menu_highlight);
                                }
                                let enabled = match item {
                                    NesMenuItem::SwapDisk | NesMenuItem::InsertEjectDisk => {
                                        if let Some(ref rom_path) = *current_rom_clone.borrow() {
                                            rom_path.to_lowercase().ends_with(".fds")
                                        } else {
                                            false
                                        }
                                    }
                                    NesMenuItem::DipSwitches => {
                                        *rom_loaded_clone.borrow()
                                    }
                                    NesMenuItem::InputBarcode => {
                                        *rom_loaded_clone.borrow()
                                    }
                                    _ => *rom_loaded_clone.borrow(),
                                };
                                let text_color = if enabled { menu_text } else { colors.disabled_text };
                                draw_text_wrapped(&mut buffer, dropdown_x + pad_x, item_y + pad_y, text_max_w, width, name, text_color, scale);
                                item_y += ih;
                            }
                        }
                        Menu::Help => {
                            let dropdown_w = item_w;
                            let pad_x = (8.0 * scale).round() as usize;
                            let pad_y = (4.0 * scale).round() as usize;
                            let help_items = ["Check for Updates", "About"];
                            let text_max_w = dropdown_w.saturating_sub(pad_x * 2);
                            let item_heights: Vec<usize> = help_items.iter().map(|name| {
                                measure_wrapped_height(name, text_max_w, scale) + pad_y * 2
                            }).collect();
                            let dropdown_h: usize = item_heights.iter().sum();
                            draw_rect(&mut buffer, dropdown_x, dropdown_y, dropdown_w, dropdown_h, width, dropdown_bg);
                            let mut item_y = dropdown_y;
                            for (i, (name, &ih)) in help_items.iter().zip(item_heights.iter()).enumerate() {
                                if ms.hovered_help_index == Some(i) {
                                    draw_rect(&mut buffer, dropdown_x, item_y, dropdown_w, ih, width, menu_highlight);
                                }
                                draw_text_wrapped(&mut buffer, dropdown_x + pad_x, item_y + pad_y, text_max_w, width, name, menu_text, scale);
                                item_y += ih;
                            }
                        }
                        Menu::Options => {
                            let dropdown_w = item_w;
                            let pad_x = (8.0 * scale).round() as usize;
                            let pad_y = (4.0 * scale).round() as usize;
                            let options_items = ["General", "Input", "Audio", "Video", "Region", "Set FDS BIOS", "Set Study Box BIOS"];
                            let text_max_w = dropdown_w.saturating_sub(pad_x * 2);
                            let item_heights: Vec<usize> = options_items.iter().map(|name| {
                                measure_wrapped_height(name, text_max_w, scale) + pad_y * 2
                            }).collect();
                            let dropdown_h: usize = item_heights.iter().sum();
                            draw_rect(&mut buffer, dropdown_x, dropdown_y, dropdown_w, dropdown_h, width, dropdown_bg);
                            let mut item_y = dropdown_y;
                            for (i, (name, &ih)) in options_items.iter().zip(item_heights.iter()).enumerate() {
                                if ms.hovered_options_index == Some(i) {
                                    draw_rect(&mut buffer, dropdown_x, item_y, dropdown_w, ih, width, menu_highlight);
                                }
                                draw_text_wrapped(&mut buffer, dropdown_x + pad_x, item_y + pad_y, text_max_w, width, name, menu_text, scale);
                                if i == 4 {
                                    let arrow_x = dropdown_x + dropdown_w - pad_x - (8.0 * scale).round() as usize;
                                    draw_text(&mut buffer, arrow_x, item_y + pad_y, width, ">", menu_text, scale);
                                }
                                item_y += ih;
                            }

                            if ms.show_region_submenu {
                                let submenu_x = dropdown_x + dropdown_w;
                                let submenu_item_h = (16.0 * scale).round() as usize;
                                let submenu_w = (150.0 * scale).round() as usize;
                                let region_anchor_y = dropdown_y;
                                let region_items = ["NTSC", "PAL", "Dendy", "Auto"];
                                let region_h = region_items.len() * submenu_item_h;
                                draw_rect(&mut buffer, submenu_x, region_anchor_y, submenu_w, region_h, width, dropdown_bg);
                                let region_pref = ms.current_region;
                                let indicator_gap = (10.0 * scale).round() as usize;
                                let name_x = submenu_x + pad_x + indicator_gap;
                                let name_max_w = submenu_w.saturating_sub(name_x - submenu_x + pad_x);
                                for (i, name) in region_items.iter().enumerate() {
                                    let iy = region_anchor_y + i * submenu_item_h;
                                    if ms.hovered_region_index == Some(i) {
                                        draw_rect(&mut buffer, submenu_x, iy, submenu_w, submenu_item_h, width, menu_highlight);
                                    }
                                    let is_selected = match (region_pref, i) {
                                        (Region::Ntsc, 0) => true,
                                        (Region::Pal, 1) => true,
                                        (Region::Dendy, 2) => true,
                                        (Region::Auto, 3) => true,
                                        _ => false,
                                    };
                                    draw_text_wrapped(&mut buffer, name_x, iy + pad_y, name_max_w, width, name, menu_text, scale);
                                    if is_selected {
                                        draw_text(&mut buffer, submenu_x + pad_x, iy + pad_y, width, ">", menu_text, scale);
                                    }
                                }
                            }
                        }
                    }
                }
                
                if ms.show_about {
                    let about_w = (300.0 * scale).round() as usize;
                    let about_h = (220.0 * scale).round() as usize;
                    let about_x = (width.saturating_sub(about_w)) / 2;
                    let about_y = (height.saturating_sub(about_h)) / 2;
                    let window_bg = colors.window_bg;
                    let window_border = colors.window_border;
                    
                    draw_rect(&mut buffer, about_x, about_y, about_w, about_h, width, window_bg);
                    draw_rect(&mut buffer, about_x, about_y, about_w, (2.0 * scale).round() as usize, width, window_border);
                    draw_rect(&mut buffer, about_x, about_y, (2.0 * scale).round() as usize, about_h, width, window_border);
                    draw_rect(&mut buffer, about_x + about_w - (2.0 * scale).round() as usize, about_y, (2.0 * scale).round() as usize, about_h, width, window_border);
                    draw_rect(&mut buffer, about_x, about_y + about_h - (2.0 * scale).round() as usize, about_w, (2.0 * scale).round() as usize, width, window_border);
                    
                    if let (Some(icon_data), (icon_w, icon_h)) = (&ms.about_icon_data, ms.about_icon_size) {
                        let icon_scale = scale * 0.5;
                        let icon_display_w = (icon_w as f32 * icon_scale).round() as usize;
                        let icon_x = about_x + (about_w.saturating_sub(icon_display_w)) / 2;
                        let icon_y = about_y + (20.0 * scale).round() as usize;
                        draw_image_rgba(&mut buffer, icon_x, icon_y, width, icon_data, icon_w, icon_h, icon_scale);
                    }
                    
                    let lines = [
                        "AccuNES",
                        "Accurate NES/Famicom Emulator",
                        "Created by: Oussema Ammar",
                        "Version: 1.6.4",
                    ];
                    let line_spacing = (20.0 * scale).round() as usize;
                    let icon_offset = if ms.about_icon_data.is_some() { (50.0 * scale).round() as usize } else { 0 };
                    let text_block_h = lines.len().saturating_sub(1) * line_spacing + (8.0 * scale).round() as usize;
                    let start_y = about_y + about_h.saturating_sub(text_block_h + icon_offset) / 2 + icon_offset;
                    
                    for (i, text) in lines.iter().enumerate() {
                        let text_w = text.len() as f32 * 8.0 * scale;
                        let text_x = about_x + ((about_w as f32 - text_w) / 2.0).round() as usize;
                        let text_y = start_y + i * line_spacing;
                        draw_text(&mut buffer, text_x, text_y, width, text, menu_text, scale);
                    }
                    
                    let close_w = (20.0 * scale).round() as usize;
                    let close_h = (20.0 * scale).round() as usize;
                    let close_x = about_x + about_w - close_w - (10.0 * scale).round() as usize;
                    let close_y = about_y + (10.0 * scale).round() as usize;
                    draw_rect(&mut buffer, close_x, close_y, close_w, close_h, width, colors.close_bg);
                    draw_text(&mut buffer, close_x + (6.0 * scale).round() as usize, close_y + (6.0 * scale).round() as usize, width, "X", colors.menu_text, scale);
                }
                
                if ms.show_updates {
                    let update_w = (360.0 * scale).round() as usize;
                    let update_h = (180.0 * scale).round() as usize;
                    let update_x = (width.saturating_sub(update_w)) / 2;
                    let update_y = (height.saturating_sub(update_h)) / 2;
                    let window_bg = colors.window_bg;
                    let window_border = colors.window_border;
                    let title_bg = colors.dropdown_bg;

                    draw_rect(&mut buffer, update_x, update_y, update_w, update_h, width, window_bg);
                    let title_h = (30.0 * scale).round() as usize;
                    draw_rect(&mut buffer, update_x, update_y, update_w, title_h, width, title_bg);
                    draw_text(&mut buffer, update_x + (10.0 * scale).round() as usize, update_y + (8.0 * scale).round() as usize, width, "Check for Updates", menu_text, scale);

                    let close_w = (20.0 * scale).round() as usize;
                    let close_h = (20.0 * scale).round() as usize;
                    let close_right_margin = (3.0 * scale).round() as usize;
                    let close_x = update_x + update_w - close_w - close_right_margin;
                    let close_y = update_y + (title_h - close_h) / 2 - (1.0 * scale).round() as usize;
                    draw_rect(&mut buffer, close_x, close_y, close_w, close_h, width, colors.close_bg);
                    draw_text(&mut buffer, close_x + (6.0 * scale).round() as usize, close_y + (6.0 * scale).round() as usize, width, "X", colors.menu_text, scale);

                    let border = (2.0 * scale).round() as usize;
                    draw_rect(&mut buffer, update_x, update_y, update_w, border, width, window_border);
                    draw_rect(&mut buffer, update_x, update_y, border, update_h, width, window_border);
                    draw_rect(&mut buffer, update_x + update_w - border, update_y, border, update_h, width, window_border);
                    draw_rect(&mut buffer, update_x, update_y + update_h - border, update_w, border, width, window_border);

                    let content_x = update_x + (15.0 * scale).round() as usize;
                    let content_y = update_y + title_h + (15.0 * scale).round() as usize;
                    let content_max_w = update_w.saturating_sub((30.0 * scale).round() as usize);

                    let update_state = update_check_state.lock().unwrap();
                    match &*update_state {
                        UpdateCheckState::Checking => {
                            draw_text_wrapped(&mut buffer, content_x, content_y, content_max_w, width, "Checking for updates...", menu_text, scale);
                        }
                        UpdateCheckState::UpToDate => {
                            draw_text_wrapped(&mut buffer, content_x, content_y, content_max_w, width, "You have the latest version.", menu_text, scale);
                        }
                        UpdateCheckState::Available { version, date, .. } => {
                            let lines = [
                                format!("New version available: {}", version),
                                format!("Release date: {}", date),
                            ];
                            let mut line_y = content_y;
                            for line in &lines {
                                draw_text_wrapped(&mut buffer, content_x, line_y, content_max_w, width, line, menu_text, scale);
                                line_y += (20.0 * scale).round() as usize;
                            }
                            let btn_w = (160.0 * scale).round() as usize;
                            let btn_h = (28.0 * scale).round() as usize;
                            let btn_x = update_x + (update_w.saturating_sub(btn_w)) / 2;
                            let btn_y = update_y + update_h - btn_h - (15.0 * scale).round() as usize;
                            draw_rect(&mut buffer, btn_x, btn_y, btn_w, btn_h, width, colors.close_bg);
                            let btn_text = "Download Update";
                            let btn_text_w = btn_text.len() as f32 * 8.0 * scale;
                            let btn_text_x = btn_x + ((btn_w as f32 - btn_text_w) / 2.0).round() as usize;
                            let btn_text_y = btn_y + ((btn_h as f32 - 8.0 * scale) / 2.0).round() as usize;
                            draw_text(&mut buffer, btn_text_x, btn_text_y, width, btn_text, colors.menu_text, scale);
                        }
                        UpdateCheckState::Error(msg) => {
                            draw_text_wrapped(&mut buffer, content_x, content_y, content_max_w, width, msg, colors.disabled_text, scale);
                        }
                        UpdateCheckState::Idle => {
                            draw_text_wrapped(&mut buffer, content_x, content_y, content_max_w, width, "Click Check for Updates to begin.", menu_text, scale);
                        }
                    }
                    drop(update_state);
                }
                
                if ms.show_general_settings {
                    let general_w = (400.0 * scale).round() as usize;
                    let general_h = (280.0 * scale).round() as usize;
                    let general_x = (width.saturating_sub(general_w)) / 2;
                    let general_y = (height.saturating_sub(general_h)) / 2;
                    let window_bg = colors.window_bg;
                    let window_border = colors.window_border;
                    let title_bg = colors.dropdown_bg;

                    draw_rect(&mut buffer, general_x, general_y, general_w, general_h, width, window_bg);

                    let title_h = (30.0 * scale).round() as usize;
                    draw_rect(&mut buffer, general_x, general_y, general_w, title_h, width, title_bg);
                    draw_text(&mut buffer, general_x + (10.0 * scale).round() as usize, general_y + (8.0 * scale).round() as usize, width, "General Settings", menu_text, scale);

                    let border_thickness = (2.0 * scale).round() as usize;
                    draw_rect(&mut buffer, general_x, general_y, general_w, border_thickness, width, window_border);
                    draw_rect(&mut buffer, general_x, general_y, border_thickness, general_h, width, window_border);
                    draw_rect(&mut buffer, general_x + general_w - border_thickness, general_y, border_thickness, general_h, width, window_border);
                    draw_rect(&mut buffer, general_x, general_y + general_h - border_thickness, general_w, border_thickness, width, window_border);

                    let row_h = (22.0 * scale).round() as usize;
                    let box_w = (100.0 * scale).round() as usize;
                    let label_x = general_x + (15.0 * scale).round() as usize;
                    let (mouse_x, mouse_y) = ms.mouse_pos;
                    let mut row_y = general_y + title_h + (15.0 * scale).round() as usize;

                    draw_text(&mut buffer, label_x, row_y + (6.0 * scale).round() as usize, width, "Pause on lost focus:", menu_text, scale);
                    let box_x = general_x + general_w - (15.0 * scale).round() as usize - box_w;
                    let box_y = row_y;
                    let val = if *pause_on_lost_focus_clone.borrow() { "On" } else { "Off" };
                    let hovered = point_in_rect(mouse_x, mouse_y, box_x, row_y, box_w, row_h);
                    let bg = if hovered { colors.box_bg_hover } else { colors.box_bg_default };
                    draw_rect(&mut buffer, box_x, box_y, box_w, row_h, width, colors.box_border);
                    draw_rect(&mut buffer, box_x + 1, box_y + 1, box_w - 2, row_h - 2, width, bg);
                    let vw = val.len() as f32 * 8.0 * scale;
                    draw_text(&mut buffer, box_x + ((box_w as f32 - vw) / 2.0).round() as usize, box_y + (6.0 * scale).round() as usize, width, val, menu_text, scale);

                    row_y += row_h + (8.0 * scale).round() as usize;
                    draw_text(&mut buffer, label_x, row_y + (6.0 * scale).round() as usize, width, "Initial RAM value:", menu_text, scale);
                    let box_x = general_x + general_w - (15.0 * scale).round() as usize - box_w;
                    let box_y = row_y;
                    let ram_mode = *initial_ram_clone.borrow();
                    let val = ram_mode.label();
                    let hovered = point_in_rect(mouse_x, mouse_y, box_x, box_y, box_w, row_h);
                    let bg = if hovered { colors.box_bg_hover } else { colors.box_bg_default };
                    draw_rect(&mut buffer, box_x, box_y, box_w, row_h, width, colors.box_border);
                    draw_rect(&mut buffer, box_x + 1, box_y + 1, box_w - 2, row_h - 2, width, bg);
                    let vw = val.len() as f32 * 8.0 * scale;
                    draw_text(&mut buffer, box_x + ((box_w as f32 - vw) / 2.0).round() as usize, box_y + (6.0 * scale).round() as usize, width, val, menu_text, scale);

                    row_y += row_h + (8.0 * scale).round() as usize;
                    draw_text(&mut buffer, label_x, row_y + (6.0 * scale).round() as usize, width, "Show FPS:", menu_text, scale);
                    let box_x = general_x + general_w - (15.0 * scale).round() as usize - box_w;
                    let box_y = row_y;
                    let fps_val = *fps_mode_clone.borrow();
                    let val = fps_val.label();
                    let hovered = point_in_rect(mouse_x, mouse_y, box_x, box_y, box_w, row_h);
                    let bg = if hovered { colors.box_bg_hover } else { colors.box_bg_default };
                    draw_rect(&mut buffer, box_x, box_y, box_w, row_h, width, colors.box_border);
                    draw_rect(&mut buffer, box_x + 1, box_y + 1, box_w - 2, row_h - 2, width, bg);
                    let vw = val.len() as f32 * 8.0 * scale;
                    draw_text(&mut buffer, box_x + ((box_w as f32 - vw) / 2.0).round() as usize, box_y + (6.0 * scale).round() as usize, width, val, menu_text, scale);

                    row_y += row_h + (8.0 * scale).round() as usize;
                    draw_text(&mut buffer, label_x, row_y + (6.0 * scale).round() as usize, width, "Confirm on exit:", menu_text, scale);
                    let box_x = general_x + general_w - (15.0 * scale).round() as usize - box_w;
                    let box_y = row_y;
                    let val = if *confirm_on_exit_clone.borrow() { "On" } else { "Off" };
                    let hovered = point_in_rect(mouse_x, mouse_y, box_x, box_y, box_w, row_h);
                    let bg = if hovered { colors.box_bg_hover } else { colors.box_bg_default };
                    draw_rect(&mut buffer, box_x, box_y, box_w, row_h, width, colors.box_border);
                    draw_rect(&mut buffer, box_x + 1, box_y + 1, box_w - 2, row_h - 2, width, bg);
                    let vw = val.len() as f32 * 8.0 * scale;
                    draw_text(&mut buffer, box_x + ((box_w as f32 - vw) / 2.0).round() as usize, box_y + (6.0 * scale).round() as usize, width, val, menu_text, scale);

                    row_y += row_h + (8.0 * scale).round() as usize;
                    draw_text(&mut buffer, label_x, row_y + (6.0 * scale).round() as usize, width, "Auto-save SRAM:", menu_text, scale);
                    let box_x = general_x + general_w - (15.0 * scale).round() as usize - box_w;
                    let box_y = row_y;
                    let val = if *auto_save_sram_clone.borrow() { "On" } else { "Off" };
                    let hovered = point_in_rect(mouse_x, mouse_y, box_x, box_y, box_w, row_h);
                    let bg = if hovered { colors.box_bg_hover } else { colors.box_bg_default };
                    draw_rect(&mut buffer, box_x, box_y, box_w, row_h, width, colors.box_border);
                    draw_rect(&mut buffer, box_x + 1, box_y + 1, box_w - 2, row_h - 2, width, bg);
                    let vw = val.len() as f32 * 8.0 * scale;
                    draw_text(&mut buffer, box_x + ((box_w as f32 - vw) / 2.0).round() as usize, box_y + (6.0 * scale).round() as usize, width, val, menu_text, scale);

                    row_y += row_h + (8.0 * scale).round() as usize;
                    draw_text(&mut buffer, label_x, row_y + (6.0 * scale).round() as usize, width, "Check for updates on startup:", menu_text, scale);
                    let box_x = general_x + general_w - (15.0 * scale).round() as usize - box_w;
                    let box_y = row_y;
                    let val = if *check_updates_on_startup_clone.borrow() { "On" } else { "Off" };
                    let hovered = point_in_rect(mouse_x, mouse_y, box_x, box_y, box_w, row_h);
                    let bg = if hovered { colors.box_bg_hover } else { colors.box_bg_default };
                    draw_rect(&mut buffer, box_x, box_y, box_w, row_h, width, colors.box_border);
                    draw_rect(&mut buffer, box_x + 1, box_y + 1, box_w - 2, row_h - 2, width, bg);
                    let vw = val.len() as f32 * 8.0 * scale;
                    draw_text(&mut buffer, box_x + ((box_w as f32 - vw) / 2.0).round() as usize, box_y + (6.0 * scale).round() as usize, width, val, menu_text, scale);

                    row_y += row_h + (8.0 * scale).round() as usize;
                    draw_text(&mut buffer, label_x, row_y + (6.0 * scale).round() as usize, width, "Theme:", menu_text, scale);
                    let box_x = general_x + general_w - (15.0 * scale).round() as usize - box_w;
                    let box_y = row_y;
                    let theme_str = match ms.theme.as_str() {
                        "light" => "Light",
                        "classicnes" => "Classic NES",
                        "famicom" => "Famicom",
                        "mario" => "Mario",
                        "link" => "Link",
                        "metroid" => "Metroid",
                        "megaman" => "Mega Man",
                        _ => "Dark",
                    };
                    let hovered = point_in_rect(mouse_x, mouse_y, box_x, box_y, box_w, row_h);
                    let bg = if hovered { colors.box_bg_hover } else { colors.box_bg_default };
                    draw_rect(&mut buffer, box_x, box_y, box_w, row_h, width, colors.box_border);
                    draw_rect(&mut buffer, box_x + 1, box_y + 1, box_w - 2, row_h - 2, width, bg);
                    let vw = theme_str.len() as f32 * 8.0 * scale;
                    draw_text(&mut buffer, box_x + ((box_w as f32 - vw) / 2.0).round() as usize, box_y + (6.0 * scale).round() as usize, width, theme_str, menu_text, scale);

                    let close_w = (20.0 * scale).round() as usize;
                    let close_h = (20.0 * scale).round() as usize;
                    let close_x = general_x + general_w - close_w - (10.0 * scale).round() as usize;
                    let close_y = general_y + (5.0 * scale).round() as usize;
                    draw_rect(&mut buffer, close_x, close_y, close_w, close_h, width, colors.close_bg);
                    draw_text(&mut buffer, close_x + (6.0 * scale).round() as usize, close_y + (6.0 * scale).round() as usize, width, "X", colors.menu_text, scale);
                }

                if ms.show_audio_settings {
                    let aw = (400.0 * scale).round() as usize;
                    let row_h = (22.0 * scale).round() as usize;
                    let gap = (8.0 * scale).round() as usize;
                    let title_h = (30.0 * scale).round() as usize;
                    let border_thickness = (2.0 * scale).round() as usize;
                    let channels = active_audio_channels(*audio_rate_clone.borrow());
                    let content_rows = 3 + channels.len();
                    let inner_h = title_h + gap + content_rows * (row_h + gap) - gap + border_thickness * 2;
                    let ah = inner_h;
                    let ax = (width.saturating_sub(aw)) / 2;
                    let ay = (height.saturating_sub(ah)) / 2;
                    let window_bg = colors.window_bg;
                    let window_border = colors.window_border;
                    let title_bg = colors.dropdown_bg;
                    let (mouse_x, mouse_y) = ms.mouse_pos;

                    draw_rect(&mut buffer, ax, ay, aw, ah, width, window_bg);

                    draw_rect(&mut buffer, ax, ay, aw, title_h, width, title_bg);
                    draw_text(&mut buffer, ax + (10.0 * scale).round() as usize, ay + (8.0 * scale).round() as usize, width, "Audio Settings", menu_text, scale);

                    draw_rect(&mut buffer, ax, ay, aw, border_thickness, width, window_border);
                    draw_rect(&mut buffer, ax, ay, border_thickness, ah, width, window_border);
                    draw_rect(&mut buffer, ax + aw - border_thickness, ay, border_thickness, ah, width, window_border);
                    draw_rect(&mut buffer, ax, ay + ah - border_thickness, aw, border_thickness, width, window_border);

                    let close_w = (20.0 * scale).round() as usize;
                    let close_h = (20.0 * scale).round() as usize;
                    let close_x = ax + aw - close_w - (10.0 * scale).round() as usize;
                    let close_y = ay + (5.0 * scale).round() as usize;
                    draw_rect(&mut buffer, close_x, close_y, close_w, close_h, width, colors.close_bg);
                    draw_text(&mut buffer, close_x + (6.0 * scale).round() as usize, close_y + (6.0 * scale).round() as usize, width, "X", colors.menu_text, scale);

                    let label_x = ax + (15.0 * scale).round() as usize;
                    let mut row_y = ay + title_h + gap;

                    draw_text(&mut buffer, label_x, row_y + (6.0 * scale).round() as usize, width, "Enable Audio:", menu_text, scale);
                    let box_w = (80.0 * scale).round() as usize;
                    let box_x = ax + aw - (15.0 * scale).round() as usize - box_w;
                    let val = if *audio_enabled_clone.borrow() { "ON" } else { "OFF" };
                    let hovered = point_in_rect(mouse_x, mouse_y, box_x, row_y, box_w, row_h);
                    let bg = if hovered { colors.box_bg_hover } else { colors.box_bg_default };
                    draw_rect(&mut buffer, box_x, row_y, box_w, row_h, width, colors.box_border);
                    draw_rect(&mut buffer, box_x + 1, row_y + 1, box_w - 2, row_h - 2, width, bg);
                    let vw = val.len() as f32 * 8.0 * scale;
                    draw_text(&mut buffer, box_x + ((box_w as f32 - vw) / 2.0).round() as usize, row_y + (6.0 * scale).round() as usize, width, val, menu_text, scale);

                    row_y += row_h + gap;
                    draw_text(&mut buffer, label_x, row_y + (6.0 * scale).round() as usize, width, "Depth:", menu_text, scale);
                    let depth_val = *audio_depth_clone.borrow();
                    let depth_str = if depth_val == 8 { "8-bit" } else { "16-bit" };
                    let hovered = point_in_rect(mouse_x, mouse_y, box_x, row_y, box_w, row_h);
                    let bg = if hovered { colors.box_bg_hover } else { colors.box_bg_default };
                    draw_rect(&mut buffer, box_x, row_y, box_w, row_h, width, colors.box_border);
                    draw_rect(&mut buffer, box_x + 1, row_y + 1, box_w - 2, row_h - 2, width, bg);
                    let vw = depth_str.len() as f32 * 8.0 * scale;
                    draw_text(&mut buffer, box_x + ((box_w as f32 - vw) / 2.0).round() as usize, row_y + (6.0 * scale).round() as usize, width, depth_str, menu_text, scale);

                    row_y += row_h + gap;
                    draw_text(&mut buffer, label_x, row_y + (6.0 * scale).round() as usize, width, "Sample Rate:", menu_text, scale);
                    let rate_val = *audio_rate_clone.borrow();
                    let rate_str = format!("{} Hz", rate_val);
                    let hovered = point_in_rect(mouse_x, mouse_y, box_x, row_y, box_w, row_h);
                    let bg = if hovered { colors.box_bg_hover } else { colors.box_bg_default };
                    draw_rect(&mut buffer, box_x, row_y, box_w, row_h, width, colors.box_border);
                    draw_rect(&mut buffer, box_x + 1, row_y + 1, box_w - 2, row_h - 2, width, bg);
                    let vw = rate_str.len() as f32 * 8.0 * scale;
                    draw_text(&mut buffer, box_x + ((box_w as f32 - vw) / 2.0).round() as usize, row_y + (6.0 * scale).round() as usize, width, &rate_str, menu_text, scale);

                    let slider_w = (120.0 * scale).round() as usize;
                    let slider_h = (14.0 * scale).round() as usize;
                    let slider_x = ax + aw - (15.0 * scale).round() as usize - slider_w;
                    let slider_track_color = colors.slider_track;
                    let slider_fill_color = colors.slider_fill;
                    let vols = channel_volumes_clone.borrow();

                    for &(chan_idx, label) in channels {
                        row_y += row_h + gap;
                        draw_text(&mut buffer, label_x, row_y + (4.0 * scale).round() as usize, width, label, menu_text, scale);
                        let pct = format!("{}%", vols[chan_idx]);
                        let pct_w = (pct.len() as f32 * 8.0 * scale).round() as usize;
                        let pct_x = slider_x - (6.0 * scale).round() as usize - pct_w;
                        draw_text(&mut buffer, pct_x, row_y + (4.0 * scale).round() as usize, width, &pct, menu_text, scale);
                        let sy = row_y + ((row_h as f32 - slider_h as f32) / 2.0).round() as usize;
                        draw_rect(&mut buffer, slider_x, sy, slider_w, slider_h, width, slider_track_color);
                        let fill_w = ((vols[chan_idx] as f32 / 100.0) * slider_w as f32).round() as usize;
                        if fill_w > 0 {
                            draw_rect(&mut buffer, slider_x, sy, fill_w, slider_h, width, slider_fill_color);
                        }
                    }
                }

                if ms.show_video_settings {
                    let vw = (400.0 * scale).round() as usize;
                    let row_h = (22.0 * scale).round() as usize;
                    let gap = (8.0 * scale).round() as usize;
                    let title_h = (30.0 * scale).round() as usize;
                    let border_thickness = (2.0 * scale).round() as usize;
                    let video_items = 4;
                    let inner_h = title_h + gap + video_items * (row_h + gap) - gap + border_thickness * 2;
                    let vh = inner_h;
                    let vx = (width.saturating_sub(vw)) / 2;
                    let vy = (height.saturating_sub(vh)) / 2;
                    let window_bg = colors.window_bg;
                    let window_border = colors.window_border;
                    let title_bg = colors.dropdown_bg;
                    let (mouse_x, mouse_y) = ms.mouse_pos;

                    draw_rect(&mut buffer, vx, vy, vw, vh, width, window_bg);

                    draw_rect(&mut buffer, vx, vy, vw, title_h, width, title_bg);
                    draw_text(&mut buffer, vx + (10.0 * scale).round() as usize, vy + (8.0 * scale).round() as usize, width, "Video Settings", menu_text, scale);

                    draw_rect(&mut buffer, vx, vy, vw, border_thickness, width, window_border);
                    draw_rect(&mut buffer, vx, vy, border_thickness, vh, width, window_border);
                    draw_rect(&mut buffer, vx + vw - border_thickness, vy, border_thickness, vh, width, window_border);
                    draw_rect(&mut buffer, vx, vy + vh - border_thickness, vw, border_thickness, width, window_border);

                    let close_w = (20.0 * scale).round() as usize;
                    let close_h = (20.0 * scale).round() as usize;
                    let close_x = vx + vw - close_w - (10.0 * scale).round() as usize;
                    let close_y = vy + (5.0 * scale).round() as usize;
                    draw_rect(&mut buffer, close_x, close_y, close_w, close_h, width, colors.close_bg);
                    draw_text(&mut buffer, close_x + (6.0 * scale).round() as usize, close_y + (6.0 * scale).round() as usize, width, "X", colors.menu_text, scale);

                    let label_x = vx + (15.0 * scale).round() as usize;
                    let box_w = (80.0 * scale).round() as usize;
                    let box_x = vx + vw - (15.0 * scale).round() as usize - box_w;
                    let mut row_y = vy + title_h + gap;
                    let video_labels = [
                        ("Fullscreen", fullscreen_clone.borrow()),
                        ("Fullscreen on Game Load", fullscreen_on_game_load_clone.borrow()),
                        ("Hide Mouse Cursor", hide_mouse_cursor_clone.borrow()),
                        ("Crop Overscan", crop_overscan_clone.borrow()),
                    ];

                    for (label, val_ref) in video_labels.iter() {
                        draw_text(&mut buffer, label_x, row_y + (6.0 * scale).round() as usize, width, label, menu_text, scale);
                        let val = if **val_ref { "ON" } else { "OFF" };
                        let hovered = point_in_rect(mouse_x, mouse_y, box_x, row_y, box_w, row_h);
                        let bg = if hovered { colors.box_bg_hover } else { colors.box_bg_default };
                        draw_rect(&mut buffer, box_x, row_y, box_w, row_h, width, colors.box_border);
                        draw_rect(&mut buffer, box_x + 1, row_y + 1, box_w - 2, row_h - 2, width, bg);
                        let vw_text = val.len() as f32 * 8.0 * scale;
                        draw_text(&mut buffer, box_x + ((box_w as f32 - vw_text) / 2.0).round() as usize, row_y + (6.0 * scale).round() as usize, width, val, menu_text, scale);
                        row_y += row_h + gap;
                    }
                }

                if ms.show_confirm_exit_dialog {
                    let dlg_w = (300.0 * scale).round() as usize;
                    let dlg_h = (100.0 * scale).round() as usize;
                    let dlg_x = (width.saturating_sub(dlg_w)) / 2;
                    let dlg_y = (height.saturating_sub(dlg_h)) / 2;
                    let window_bg = colors.window_bg;
                    draw_rect(&mut buffer, dlg_x, dlg_y, dlg_w, dlg_h, width, window_bg);
                    draw_rect(&mut buffer, dlg_x, dlg_y, dlg_w, (2.0 * scale).round() as usize, width, colors.window_border);
                    draw_rect(&mut buffer, dlg_x, dlg_y, (2.0 * scale).round() as usize, dlg_h, width, colors.window_border);
                    draw_rect(&mut buffer, dlg_x + dlg_w - (2.0 * scale).round() as usize, dlg_y, (2.0 * scale).round() as usize, dlg_h, width, colors.window_border);
                    draw_rect(&mut buffer, dlg_x, dlg_y + dlg_h - (2.0 * scale).round() as usize, dlg_w - 0, (2.0 * scale).round() as usize, width, colors.window_border);
                    draw_text(&mut buffer, dlg_x + (15.0 * scale).round() as usize, dlg_y + (15.0 * scale).round() as usize, width, "Exit emulator?", menu_text, scale);
                    let btn_w = (60.0 * scale).round() as usize;
                    let btn_h = (24.0 * scale).round() as usize;
                    let btn_y = dlg_y + dlg_h - btn_h - (10.0 * scale).round() as usize;
                    let (mx, my) = ms.mouse_pos;
                    let yes_x = dlg_x + (30.0 * scale).round() as usize;
                    let yes_hovered = point_in_rect(mx, my, yes_x, btn_y, btn_w, btn_h);
                    draw_rect(&mut buffer, yes_x, btn_y, btn_w, btn_h, width, colors.box_border);
                    draw_rect(&mut buffer, yes_x + 1, btn_y + 1, btn_w - 2, btn_h - 2, width, if yes_hovered { colors.box_bg_hover } else { colors.box_bg_default });
                    draw_text(&mut buffer, yes_x + (22.0 * scale).round() as usize, btn_y + (7.0 * scale).round() as usize, width, "Yes", menu_text, scale);
                    let no_x = dlg_x + dlg_w - btn_w - (30.0 * scale).round() as usize;
                    let no_hovered = point_in_rect(mx, my, no_x, btn_y, btn_w, btn_h);
                    draw_rect(&mut buffer, no_x, btn_y, btn_w, btn_h, width, colors.box_border);
                    draw_rect(&mut buffer, no_x + 1, btn_y + 1, btn_w - 2, btn_h - 2, width, if no_hovered { colors.box_bg_hover } else { colors.box_bg_default });
                    draw_text(&mut buffer, no_x + (22.0 * scale).round() as usize, btn_y + (7.0 * scale).round() as usize, width, "No", menu_text, scale);
                }

                if ms.show_barcode_input {
                    let sc = scale;
                    let dlg_w = (320.0 * sc).round() as usize;
                    let dlg_h = (160.0 * sc).round() as usize;
                    let dlg_x = (width.saturating_sub(dlg_w)) / 2;
                    let dlg_y = (height.saturating_sub(dlg_h)) / 2;
                    let window_bg = colors.window_bg;
                    draw_rect(&mut buffer, dlg_x, dlg_y, dlg_w, dlg_h, width, window_bg);
                    draw_rect(&mut buffer, dlg_x, dlg_y, dlg_w, (2.0 * sc).round() as usize, width, colors.window_border);
                    draw_rect(&mut buffer, dlg_x, dlg_y, (2.0 * sc).round() as usize, dlg_h, width, colors.window_border);
                    draw_rect(&mut buffer, dlg_x + dlg_w - (2.0 * sc).round() as usize, dlg_y, (2.0 * sc).round() as usize, dlg_h, width, colors.window_border);
                    draw_rect(&mut buffer, dlg_x, dlg_y + dlg_h - (2.0 * sc).round() as usize, dlg_w, (2.0 * sc).round() as usize, width, colors.window_border);
                    let title_h = (26.0 * sc).round() as usize;
                    draw_rect(&mut buffer, dlg_x, dlg_y, dlg_w, title_h, width, colors.dropdown_bg);
                    draw_text(&mut buffer, dlg_x + (10.0 * sc).round() as usize, dlg_y + (7.0 * sc).round() as usize, width, "Input Barcode", menu_text, scale);
                    let margin = (12.0 * sc).round() as usize;
                    let field_x = dlg_x + margin;
                    let field_w = dlg_w - margin * 2;
                    let field_y = dlg_y + margin + title_h;
                    let field_h = (30.0 * sc).round() as usize;
                    draw_rect(&mut buffer, field_x, field_y, field_w, field_h, width, colors.box_border);
                    draw_rect(&mut buffer, field_x + 1, field_y + 1, field_w - 2, field_h - 2, width, colors.box_bg_default);
                    let field_text = ms.barcode_input.clone();
                    let char_w = (8.0 * sc).round() as usize;
                    let text_x = field_x + (8.0 * sc).round() as usize;
                    let text_y = field_y + (7.0 * sc).round() as usize;
                    if field_text.is_empty() {
                        draw_text(&mut buffer, text_x, text_y, width, "(type digits, then OK)", colors.disabled_text, scale);
                    } else {
                        let (sa, sb) = barcode_sel_bounds(ms.barcode_sel_anchor, ms.barcode_caret);
                        if sa != sb {
                            let sel_x = text_x + sa * char_w;
                            let sel_w = (sb - sa) * char_w;
                            draw_rect(&mut buffer, sel_x, text_y, sel_w, (8.0 * sc).round() as usize, width, colors.box_bg_hover);
                        }
                        draw_text(&mut buffer, text_x, text_y, width, &field_text, menu_text, scale);
                    }
                    if ms.barcode_caret <= field_text.len() {
                        let caret_x = text_x + ms.barcode_caret * char_w;
                        draw_rect(&mut buffer, caret_x, text_y, (1.0 * sc).round() as usize, (8.0 * sc).round() as usize, width, menu_text);
                    }
                    let btn_w = (60.0 * sc).round() as usize;
                    let btn_h = (24.0 * sc).round() as usize;
                    let btn_y = dlg_y + dlg_h - btn_h - margin;
                    let text_cry = btn_y + ((btn_h as f32 - 8.0 * sc) / 2.0).round() as usize;
                    let ok_x = dlg_x + margin;
                    let clear_x = ok_x + btn_w + (8.0 * sc).round() as usize;
                    let close_x = dlg_x + dlg_w - btn_w - margin;
                    let (mx, my) = ms.mouse_pos;
                    let ok_hovered = point_in_rect(mx, my, ok_x, btn_y, btn_w, btn_h);
                    draw_rect(&mut buffer, ok_x, btn_y, btn_w, btn_h, width, colors.box_border);
                    draw_rect(&mut buffer, ok_x + 1, btn_y + 1, btn_w - 2, btn_h - 2, width, if ok_hovered { colors.box_bg_hover } else { colors.box_bg_default });
                    draw_text(&mut buffer, ok_x + ((btn_w as f32 - 16.0 * sc) / 2.0).round() as usize, text_cry, width, "OK", menu_text, scale);
                    let clear_hovered = point_in_rect(mx, my, clear_x, btn_y, btn_w, btn_h);
                    draw_rect(&mut buffer, clear_x, btn_y, btn_w, btn_h, width, colors.box_border);
                    draw_rect(&mut buffer, clear_x + 1, btn_y + 1, btn_w - 2, btn_h - 2, width, if clear_hovered { colors.box_bg_hover } else { colors.box_bg_default });
                    draw_text(&mut buffer, clear_x + ((btn_w as f32 - 40.0 * sc) / 2.0).round() as usize, text_cry, width, "Clear", menu_text, scale);
                    let close_hovered = point_in_rect(mx, my, close_x, btn_y, btn_w, btn_h);
                    draw_rect(&mut buffer, close_x, btn_y, btn_w, btn_h, width, colors.box_border);
                    draw_rect(&mut buffer, close_x + 1, btn_y + 1, btn_w - 2, btn_h - 2, width, if close_hovered { colors.box_bg_hover } else { colors.box_bg_default });
                    draw_text(&mut buffer, close_x + ((btn_w as f32 - 48.0 * sc) / 2.0).round() as usize, text_cry, width, "Cancel", menu_text, scale);
                }

                if ms.show_input_settings {
                    let input_w = (480.0 * scale).round() as usize;
                    let input_h_base = (380.0 * scale).round() as usize;
                    let input_h_4p = (410.0 * scale).round() as usize;
                    let input_h = {
                            let dt = *expansion_type_clone.borrow();
                            let at = *expansion_adapter_type_clone.borrow();
                            let ept = match at {
                                config::ExpansionAdapterType::TwoPlayer => config::ExpansionPortType::TwoPlayerAdapter,
                                config::ExpansionAdapterType::FourPlayer => config::ExpansionPortType::FourPlayerAdapter,
                                config::ExpansionAdapterType::None => match dt {
                                    config::ExpansionType::ArkanoidPaddle => config::ExpansionPortType::ArkanoidPaddle,
                                    config::ExpansionType::FamicomZapper => config::ExpansionPortType::FamicomZapper,
                                    config::ExpansionType::OekaKidsTablet => config::ExpansionPortType::OekaKidsTablet,
                                    config::ExpansionType::FamilyTrainerA => config::ExpansionPortType::FamilyTrainerA,
                                    config::ExpansionType::FamilyTrainerB => config::ExpansionPortType::FamilyTrainerB,
                                    config::ExpansionType::KonamiHyperShot => config::ExpansionPortType::KonamiHyperShot,
                                    config::ExpansionType::FamilyBasicKeyboard => config::ExpansionPortType::FamilyBasicKeyboard,
                                    config::ExpansionType::PartyTap => config::ExpansionPortType::PartyTap,
                                    config::ExpansionType::PachinkoController => config::ExpansionPortType::PachinkoController,
                                             config::ExpansionType::ExcitingBoxing => config::ExpansionPortType::ExcitingBoxing,
                                             config::ExpansionType::JissenMahjong => config::ExpansionPortType::JissenMahjong,
                                             config::ExpansionType::SuborKeyboard => config::ExpansionPortType::SuborKeyboard,
                                             config::ExpansionType::BarcodeBattler => config::ExpansionPortType::BarcodeBattler,
                                             config::ExpansionType::HoriTrack => config::ExpansionPortType::HoriTrack,
                                             config::ExpansionType::BandaiHyperShot => config::ExpansionPortType::BandaiHyperShot,
                                             config::ExpansionType::TurboFile => config::ExpansionPortType::TurboFile,
                                             config::ExpansionType::BattleBox => config::ExpansionPortType::BattleBox,
                                             config::ExpansionType::None => config::ExpansionPortType::None,
                                 },
                             };
                             if ept == config::ExpansionPortType::FourPlayerAdapter { input_h_4p } else { input_h_base }
                        };
                    let input_x = (width.saturating_sub(input_w)) / 2;
                    let input_y = (height.saturating_sub(input_h)) / 2;
                    let title_h = (30.0 * scale).round() as usize;
                    let border_thickness = (2.0 * scale).round() as usize;
                    let window_bg = colors.window_bg;
                    let title_bg = colors.dropdown_bg;
                    let window_border = colors.window_border;
                    let row_h = (22.0 * scale).round() as usize;
                    let box_w = (165.0 * scale).round() as usize;
                    let col_gap = (10.0 * scale).round() as usize;
                    let configure_h = (24.0 * scale).round() as usize;
                    let col_w = (input_w - (10.0 * scale).round() as usize * 2 - col_gap) / 2;
                    let (mouse_x, mouse_y) = ms.mouse_pos;
                    draw_rect(&mut buffer, input_x, input_y, input_w, input_h, width, window_bg);
                    draw_rect(&mut buffer, input_x, input_y, input_w, title_h, width, title_bg);
                    draw_text(&mut buffer, input_x + (10.0 * scale).round() as usize, input_y + (8.0 * scale).round() as usize, width, "Input Options", menu_text, scale);
                    draw_rect(&mut buffer, input_x, input_y, input_w, border_thickness, width, window_border);
                    draw_rect(&mut buffer, input_x, input_y, border_thickness, input_h, width, window_border);
                    draw_rect(&mut buffer, input_x + input_w - border_thickness, input_y, border_thickness, input_h, width, window_border);
                    draw_rect(&mut buffer, input_x, input_y + input_h - border_thickness, input_w, border_thickness, width, window_border);
                    let close_w = (20.0 * scale).round() as usize;
                    let close_h = (20.0 * scale).round() as usize;
                    let close_x = input_x + input_w - close_w - (10.0 * scale).round() as usize;
                    let close_y = input_y + (5.0 * scale).round() as usize;
                    draw_rect(&mut buffer, close_x, close_y, close_w, close_h, width, colors.close_bg);
                    draw_text(&mut buffer, close_x + (6.0 * scale).round() as usize, close_y + (6.0 * scale).round() as usize, width, "X", colors.menu_text, scale);
                    let content_y = input_y + title_h + (10.0 * scale).round() as usize;
                    let col1_x = input_x + (10.0 * scale).round() as usize;
                    let col2_x = col1_x + col_w + col_gap;
                    draw_text(&mut buffer, col1_x, content_y, width, "Controller 1", menu_text, scale);
                    draw_text(&mut buffer, col2_x, content_y, width, "Controller 2", menu_text, scale);
                    let cfg_y = content_y + row_h + (5.0 * scale).round() as usize;
                    let type_label_w = ((5 * 8) as f32 * scale).round() as usize;
                    let label_gap = (6.0 * scale).round() as usize;
                    let btn_x_offset = col1_x + type_label_w + label_gap;
                    let btn2_x_offset = col2_x + type_label_w + label_gap;
                    let c1_cfg_x = btn_x_offset;
                    let c1_type = *controller1_type_clone.borrow();
                    let c2_type = *controller2_type_clone.borrow();
                    let c1_cfg_enabled = c1_type != config::ControllerType::None;
                    let c2_cfg_enabled = c2_type != config::ControllerType::None;
                    let disabled_text = colors.disabled_text;
                    let c1_cfg_hovered = c1_cfg_enabled && point_in_rect(mouse_x, mouse_y, c1_cfg_x, cfg_y, box_w, configure_h);
                    let c1_cfg_bg = if !c1_cfg_enabled { colors.disabled_btn_bg } else if c1_cfg_hovered { colors.box_bg_hover } else { colors.dropdown_bg };
                    draw_rect(&mut buffer, c1_cfg_x, cfg_y, box_w, configure_h, width, colors.btn_border);
                    draw_rect(&mut buffer, c1_cfg_x + 1, cfg_y + 1, box_w - 2, configure_h - 2, width, c1_cfg_bg);
                    let cfg_label = "Configure";
                    let cfg_vw = cfg_label.len() as f32 * 8.0 * scale;
                    let cfg_color = if c1_cfg_enabled { menu_text } else { disabled_text };
                    draw_text(&mut buffer, c1_cfg_x + ((box_w as f32 - cfg_vw) / 2.0).round() as usize, cfg_y + (7.0 * scale).round() as usize, width, cfg_label, cfg_color, scale);
                    let c2_cfg_x = btn2_x_offset;
                    let c2_cfg_hovered = c2_cfg_enabled && point_in_rect(mouse_x, mouse_y, c2_cfg_x, cfg_y, box_w, configure_h);
                    let c2_cfg_bg = if !c2_cfg_enabled { colors.disabled_btn_bg } else if c2_cfg_hovered { colors.box_bg_hover } else { colors.dropdown_bg };
                    draw_rect(&mut buffer, c2_cfg_x, cfg_y, box_w, configure_h, width, colors.btn_border);
                    draw_rect(&mut buffer, c2_cfg_x + 1, cfg_y + 1, box_w - 2, configure_h - 2, width, c2_cfg_bg);
                    let cfg_color2 = if c2_cfg_enabled { menu_text } else { disabled_text };
                    draw_text(&mut buffer, c2_cfg_x + ((box_w as f32 - cfg_vw) / 2.0).round() as usize, cfg_y + (7.0 * scale).round() as usize, width, cfg_label, cfg_color2, scale);
                    let type_y = cfg_y + configure_h + (8.0 * scale).round() as usize;
                    let t1_box_x = btn_x_offset;
                    draw_text(&mut buffer, col1_x, type_y + (6.0 * scale).round() as usize, width, "Type:", menu_text, scale);
                    let c1_type = *controller1_type_clone.borrow();
                    let t1_val = c1_type.label();
                    let t1_hovered = point_in_rect(mouse_x, mouse_y, t1_box_x, type_y, box_w, row_h);
                    let t1_bg = if t1_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                    draw_rect(&mut buffer, t1_box_x, type_y, box_w, row_h, width, colors.box_border);
                    draw_rect(&mut buffer, t1_box_x + 1, type_y + 1, box_w - 2, row_h - 2, width, t1_bg);
                    let t1_vw = t1_val.len() as f32 * 8.0 * scale;
                    draw_text(&mut buffer, t1_box_x + ((box_w as f32 - t1_vw) / 2.0).round() as usize, type_y + (6.0 * scale).round() as usize, width, t1_val, menu_text, scale);
                    let t2_box_x = col2_x + type_label_w + label_gap;
                    draw_text(&mut buffer, col2_x, type_y + (6.0 * scale).round() as usize, width, "Type:", menu_text, scale);
                    let c2_type = *controller2_type_clone.borrow();
                    let t2_val = c2_type.label();
                    let t2_hovered = point_in_rect(mouse_x, mouse_y, t2_box_x, type_y, box_w, row_h);
                    let t2_bg = if t2_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                    draw_rect(&mut buffer, t2_box_x, type_y, box_w, row_h, width, colors.box_border);
                    draw_rect(&mut buffer, t2_box_x + 1, type_y + 1, box_w - 2, row_h - 2, width, t2_bg);
                    let t2_vw = t2_val.len() as f32 * 8.0 * scale;
                    draw_text(&mut buffer, t2_box_x + ((box_w as f32 - t2_vw) / 2.0).round() as usize, type_y + (6.0 * scale).round() as usize, width, t2_val, menu_text, scale);
                    let exp_title_y = type_y + row_h + (12.0 * scale).round() as usize;
                    let exp_title_vw = ("Famicom Expansion Port".len() as f32 * 8.0 * scale).round() as usize;
                    let exp_title_x = input_x + input_w.saturating_sub(exp_title_vw) / 2;
                    draw_text(&mut buffer, exp_title_x, exp_title_y, width, "Famicom Expansion Port", menu_text, scale);
                    let exp_cfg_y = exp_title_y + row_h + (5.0 * scale).round() as usize;
                    let exp_cfg_x = input_x + input_w.saturating_sub(box_w) / 2;
                    let exp_port_type = {
                        let dt = *expansion_type_clone.borrow();
                        let at = *expansion_adapter_type_clone.borrow();
                        match at {
                            config::ExpansionAdapterType::TwoPlayer => config::ExpansionPortType::TwoPlayerAdapter,
                            config::ExpansionAdapterType::FourPlayer => config::ExpansionPortType::FourPlayerAdapter,
                            config::ExpansionAdapterType::None => match dt {
                                config::ExpansionType::ArkanoidPaddle => config::ExpansionPortType::ArkanoidPaddle,
                                config::ExpansionType::FamicomZapper => config::ExpansionPortType::FamicomZapper,
                                config::ExpansionType::OekaKidsTablet => config::ExpansionPortType::OekaKidsTablet,
                                config::ExpansionType::FamilyTrainerA => config::ExpansionPortType::FamilyTrainerA,
                                config::ExpansionType::FamilyTrainerB => config::ExpansionPortType::FamilyTrainerB,
                                config::ExpansionType::KonamiHyperShot => config::ExpansionPortType::KonamiHyperShot,
                                config::ExpansionType::FamilyBasicKeyboard => config::ExpansionPortType::FamilyBasicKeyboard,
                                config::ExpansionType::PartyTap => config::ExpansionPortType::PartyTap,
                                config::ExpansionType::PachinkoController => config::ExpansionPortType::PachinkoController,
                                             config::ExpansionType::ExcitingBoxing => config::ExpansionPortType::ExcitingBoxing,
                                             config::ExpansionType::JissenMahjong => config::ExpansionPortType::JissenMahjong,
                                             config::ExpansionType::SuborKeyboard => config::ExpansionPortType::SuborKeyboard,
                                             config::ExpansionType::BarcodeBattler => config::ExpansionPortType::BarcodeBattler,
                                             config::ExpansionType::HoriTrack => config::ExpansionPortType::HoriTrack,
                                             config::ExpansionType::BandaiHyperShot => config::ExpansionPortType::BandaiHyperShot,
                                             config::ExpansionType::TurboFile => config::ExpansionPortType::TurboFile,
                                             config::ExpansionType::BattleBox => config::ExpansionPortType::BattleBox,
                                 config::ExpansionType::None => config::ExpansionPortType::None,
                            },
                        }
                    };
                    let exp_type_enabled = exp_port_type != config::ExpansionPortType::None && exp_port_type != config::ExpansionPortType::BarcodeBattler && exp_port_type != config::ExpansionPortType::TurboFile && exp_port_type != config::ExpansionPortType::BattleBox;
                    let is_4p = exp_port_type == config::ExpansionPortType::FourPlayerAdapter;
                    let cfg_vw2 = cfg_label.len() as f32 * 8.0 * scale;
                    let exp_type_y = if is_4p {
                        let cfg12_label = "Configure 1/2";
                        let cfg12_vw = cfg12_label.len() as f32 * 8.0 * scale;
                        let cfg12_hovered = point_in_rect(mouse_x, mouse_y, exp_cfg_x, exp_cfg_y, box_w, configure_h);
                        let cfg12_bg = if cfg12_hovered { colors.box_bg_hover } else { colors.dropdown_bg };
                        draw_rect(&mut buffer, exp_cfg_x, exp_cfg_y, box_w, configure_h, width, colors.btn_border);
                        draw_rect(&mut buffer, exp_cfg_x + 1, exp_cfg_y + 1, box_w - 2, configure_h - 2, width, cfg12_bg);
                        draw_text(&mut buffer, exp_cfg_x + ((box_w as f32 - cfg12_vw) / 2.0).round() as usize, exp_cfg_y + (7.0 * scale).round() as usize, width, cfg12_label, menu_text, scale);
                        let cfg34_label = "Configure 3/4";
                        let cfg34_vw = cfg34_label.len() as f32 * 8.0 * scale;
                        let cfg34_y = exp_cfg_y + configure_h + (4.0 * scale).round() as usize;
                        let cfg34_hovered = point_in_rect(mouse_x, mouse_y, exp_cfg_x, cfg34_y, box_w, configure_h);
                        let cfg34_bg = if cfg34_hovered { colors.box_bg_hover } else { colors.dropdown_bg };
                        draw_rect(&mut buffer, exp_cfg_x, cfg34_y, box_w, configure_h, width, colors.btn_border);
                        draw_rect(&mut buffer, exp_cfg_x + 1, cfg34_y + 1, box_w - 2, configure_h - 2, width, cfg34_bg);
                        draw_text(&mut buffer, exp_cfg_x + ((box_w as f32 - cfg34_vw) / 2.0).round() as usize, cfg34_y + (7.0 * scale).round() as usize, width, cfg34_label, menu_text, scale);
                        cfg34_y + configure_h + (8.0 * scale).round() as usize
                    } else {
                        let exp_cfg_hovered = exp_type_enabled && point_in_rect(mouse_x, mouse_y, exp_cfg_x, exp_cfg_y, box_w, configure_h);
                        let exp_cfg_bg = if !exp_type_enabled { colors.disabled_btn_bg } else if exp_cfg_hovered { colors.box_bg_hover } else { colors.dropdown_bg };
                        draw_rect(&mut buffer, exp_cfg_x, exp_cfg_y, box_w, configure_h, width, colors.btn_border);
                        draw_rect(&mut buffer, exp_cfg_x + 1, exp_cfg_y + 1, box_w - 2, configure_h - 2, width, exp_cfg_bg);
                        let exp_cfg_color = if exp_type_enabled { menu_text } else { colors.disabled_text };
                        draw_text(&mut buffer, exp_cfg_x + ((box_w as f32 - cfg_vw2) / 2.0).round() as usize, exp_cfg_y + (7.0 * scale).round() as usize, width, cfg_label, exp_cfg_color, scale);
                        exp_cfg_y + configure_h + (8.0 * scale).round() as usize
                    };
                    let exp_group_x = exp_cfg_x - type_label_w - label_gap;
                    draw_text(&mut buffer, exp_group_x, exp_type_y + (6.0 * scale).round() as usize, width, "Type:", menu_text, scale);
                    let exp_type_box_x = exp_cfg_x;
                    let exp_val = exp_port_type.label();
                    let exp_type_hovered = point_in_rect(mouse_x, mouse_y, exp_type_box_x, exp_type_y, box_w, row_h);
                    let exp_type_bg = if exp_type_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                    draw_rect(&mut buffer, exp_type_box_x, exp_type_y, box_w, row_h, width, colors.box_border);
                    draw_rect(&mut buffer, exp_type_box_x + 1, exp_type_y + 1, box_w - 2, row_h - 2, width, exp_type_bg);
                    let exp_vw = exp_val.len() as f32 * 8.0 * scale;
                    draw_text(&mut buffer, exp_type_box_x + ((box_w as f32 - exp_vw) / 2.0).round() as usize, exp_type_y + (6.0 * scale).round() as usize, width, exp_val, menu_text, scale);
                    let dpad_y = exp_type_y + row_h + (15.0 * scale).round() as usize;
                    draw_text(&mut buffer, col1_x, dpad_y + (6.0 * scale).round() as usize, width, "Allow L+R/U+D:", menu_text, scale);
                    let dpad_box_x = btn2_x_offset;
                    let dpad_val = if *allow_opposing_dpad_clone.borrow() { "On" } else { "Off" };
                    let dpad_hovered = point_in_rect(mouse_x, mouse_y, dpad_box_x, dpad_y, box_w, row_h);
                    let dpad_bg = if dpad_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                    draw_rect(&mut buffer, dpad_box_x, dpad_y, box_w, row_h, width, colors.box_border);
                    draw_rect(&mut buffer, dpad_box_x + 1, dpad_y + 1, box_w - 2, row_h - 2, width, dpad_bg);
                    let dpad_vw = dpad_val.len() as f32 * 8.0 * scale;
                    draw_text(&mut buffer, dpad_box_x + ((box_w as f32 - dpad_vw) / 2.0).round() as usize, dpad_y + (6.0 * scale).round() as usize, width, dpad_val, menu_text, scale);
                    let auto_y = dpad_y + row_h + (10.0 * scale).round() as usize;
                    let cb = (20.0 * scale).round() as usize;
                    let cb_x = col1_x;
                    let auto_val = *auto_detect_game_controller_clone.borrow();
                    let cb_hovered = point_in_rect(mouse_x, mouse_y, cb_x, auto_y, cb, cb);
                    let cb_bg = if cb_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                    draw_rect(&mut buffer, cb_x, auto_y, cb, cb, width, colors.box_border);
                    draw_rect(&mut buffer, cb_x + 1, auto_y + 1, cb.saturating_sub(2), cb.saturating_sub(2), width, cb_bg);
                    if auto_val {
                        draw_text(&mut buffer, cb_x + (2.0 * scale).round() as usize, auto_y + ((cb as f32 - 8.0 * scale) / 2.0).round() as usize, width, "X", menu_text, scale);
                    }
                    draw_text(&mut buffer, cb_x + cb + (8.0 * scale).round() as usize, auto_y + ((cb as f32 - 8.0 * scale) / 2.0).round() as usize, width, "Auto-detect game controller", menu_text, scale);
                }
                
                if ms.show_controller1_settings {
                    let (mouse_x, mouse_y) = ms.mouse_pos;
                    let cw = (440.0 * scale).round() as usize;
                    let title_h = (30.0 * scale).round() as usize;
                    let bth = (2.0 * scale).round() as usize;
                    let btn_w = (90.0 * scale).round() as usize;
                    let btn_h = (26.0 * scale).round() as usize;
                    let gap_y = (8.0 * scale).round() as usize;
                    let gap_x = (cw.saturating_sub(2 * btn_w)) / 3;
                    let c1t = *controller1_type_clone.borrow();
                    let c2t = *controller2_type_clone.borrow();
                    let c1t_fs = c1t == config::ControllerType::FourScore || c2t == config::ControllerType::FourScore;
                    let fs_block_h = 4 * (btn_h + gap_y) + btn_h;
                    let tmp_g0 = title_h + (10.0 * scale).round() as usize;
                    let ch = if c1t_fs {
                        let fs_ch = tmp_g0 + 2 * (fs_block_h + btn_h / 2) + (40.0 * scale).round() as usize;
                        fs_ch.max(260)
                    } else {
                        (260.0 * scale).round() as usize
                    };
                    let cx = (width.saturating_sub(cw)) / 2;
                    let cy = (height.saturating_sub(ch)) / 2;
                    let grid_y0 = cy + tmp_g0;
                    let wbg = colors.window_bg;
                    let tbg = colors.dropdown_bg;
                    let wbrd = colors.window_border;
                    draw_rect(&mut buffer, cx, cy, cw, ch, width, wbg);
                    draw_rect(&mut buffer, cx, cy, cw, title_h, width, tbg);
                    draw_text(&mut buffer, cx + (10.0 * scale).round() as usize, cy + (8.0 * scale).round() as usize, width, "Controller 1 Settings", menu_text, scale);
                    draw_rect(&mut buffer, cx, cy, cw, bth, width, wbrd);
                    draw_rect(&mut buffer, cx, cy, bth, ch, width, wbrd);
                    draw_rect(&mut buffer, cx + cw - bth, cy, bth, ch, width, wbrd);
                    draw_rect(&mut buffer, cx, cy + ch - bth, cw, bth, width, wbrd);
                    let close_w = (20.0 * scale).round() as usize;
                    let close_h = (20.0 * scale).round() as usize;
                    let close_x = cx + cw - close_w - (10.0 * scale).round() as usize;
                    let close_y = cy + (5.0 * scale).round() as usize;
                    draw_rect(&mut buffer, close_x, close_y, close_w, close_h, width, colors.close_bg);
                    draw_text(&mut buffer, close_x + (6.0 * scale).round() as usize, close_y + (6.0 * scale).round() as usize, width, "X", colors.menu_text, scale);
                    let labels = ["A", "B", "Turbo A", "Turbo B", "Select", "Start", "Up", "Down", "Left", "Right"];
                    if c1t == config::ControllerType::Zapper || c1t == config::ControllerType::Paddle {
                        let total_w = 2 * btn_w + gap_x;
                        let trig_bx = cx + (cw - total_w) / 2;
                        let is_hovered = ms.hovered_ctrl_button == Some(0);
                        let is_rebinding = ms.rebind_controller == Some(1) && ms.rebind_button == Some(0);
                        let (lbl, binding) = if c1t == config::ControllerType::Zapper {
                            ("Trigger", zapper_trigger_binding_clone.borrow().clone())
                        } else {
                            ("Button", paddle1_button_binding_clone.borrow().clone())
                        };
                        let txt = if is_rebinding { "?".to_string() } else { binding };
                        let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                        let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                        draw_rect(&mut buffer, trig_bx, grid_y0, total_w, btn_h, width, border);
                        draw_rect(&mut buffer, trig_bx + 1, grid_y0 + 1, total_w - 2, btn_h - 2, width, bg);
                        let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                        draw_text(&mut buffer, trig_bx + ((total_w as f32 - lbl_vw) / 2.0).round() as usize, grid_y0 + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                        let key_vw = txt.len() as f32 * 8.0 * scale;
                        draw_text(&mut buffer, trig_bx + ((total_w as f32 - key_vw) / 2.0).round() as usize, grid_y0 + (14.0 * scale).round() as usize, width, &txt, menu_text, scale);
                    } else if c1t == config::ControllerType::SNESMouse || c1t == config::ControllerType::SuborMouse {
                        let mouse_bindings: Vec<String> = if c1t == config::ControllerType::SNESMouse {
                            snes_mouse1_bindings_clone.borrow().to_vec()
                        } else {
                            subor_mouse1_bindings_clone.borrow().to_vec()
                        };
                        let rebinding = ms.rebind_controller == Some(1);
                        let per_w = btn_w;
                        let half_gap = (cw.saturating_sub(2 * per_w)) / 3;
                        for i in 0..config::SNES_MOUSE_BUTTON_COUNT {
                            let bx = cx + half_gap + i * (per_w + half_gap);
                            let is_hovered = ms.hovered_ctrl_button == Some(i);
                            let is_rebinding_btn = rebinding && ms.rebind_button == Some(i);
                            let txt = if is_rebinding_btn { "?" } else { &mouse_bindings[i] };
                            let border = if is_rebinding_btn { colors.rebind_border } else { colors.box_border };
                            let bg = if is_rebinding_btn { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                            draw_rect(&mut buffer, bx, grid_y0, per_w, btn_h, width, border);
                            draw_rect(&mut buffer, bx + 1, grid_y0 + 1, per_w - 2, btn_h - 2, width, bg);
                            let lbl = config::SNES_MOUSE_LABELS[i];
                            let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((per_w as f32 - lbl_vw) / 2.0).round() as usize, grid_y0 + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                            let key_vw = txt.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((per_w as f32 - key_vw) / 2.0).round() as usize, grid_y0 + (14.0 * scale).round() as usize, width, txt, menu_text, scale);
                        }
                    } else if c1t == config::ControllerType::PowerPadA || c1t == config::ControllerType::PowerPadB {
                        let pp_bindings = powerpad1_bindings_clone.borrow();
                        let rebinding = ms.rebind_controller == Some(1);
                        let cols = 4;
                        let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                        for i in 0..config::POWERPAD_BUTTON_COUNT {
                            let row = i / cols;
                            let col = i % cols;
                            let bx = cx + gap_x + col * (btn_w + gap_x);
                            let by = grid_y0 + row * (btn_h + gap_y);
                            let is_hovered = ms.hovered_ctrl_button == Some(i);
                            let is_rebinding = rebinding && ms.rebind_button == Some(i);
                            let txt = if is_rebinding { "?" } else { &pp_bindings[i] };
                            let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                            let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                            draw_rect(&mut buffer, bx, by, btn_w, btn_h, width, border);
                            draw_rect(&mut buffer, bx + 1, by + 1, btn_w - 2, btn_h - 2, width, bg);
                            let lbl = config::POWERPAD_LABELS[i];
                            let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((btn_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                            let key_vw = txt.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((btn_w as f32 - key_vw) / 2.0).round() as usize, by + (12.0 * scale).round() as usize, width, txt, menu_text, scale);
                        }
                    } else if c1t == config::ControllerType::SNESPad {
                        let snes_bindings = snes1_bindings_clone.borrow();
                        let rebinding = ms.rebind_controller == Some(1);
                        let cols = 4;
                        let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                        for i in 0..config::SNES_BUTTON_COUNT {
                            let row = i / cols;
                            let col = i % cols;
                            let bx = cx + gap_x + col * (btn_w + gap_x);
                            let by = grid_y0 + row * (btn_h + gap_y);
                            let is_hovered = ms.hovered_ctrl_button == Some(i);
                            let is_rebinding = rebinding && ms.rebind_button == Some(i);
                            let txt = if is_rebinding { "?" } else { &snes_bindings[i] };
                            let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                            let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                            draw_rect(&mut buffer, bx, by, btn_w, btn_h, width, border);
                            draw_rect(&mut buffer, bx + 1, by + 1, btn_w - 2, btn_h - 2, width, bg);
                            let lbl = config::SNES_LABELS[i];
                            let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((btn_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                            let key_vw = txt.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((btn_w as f32 - key_vw) / 2.0).round() as usize, by + (12.0 * scale).round() as usize, width, txt, menu_text, scale);
                        }
                    } else if c1t == config::ControllerType::VirtualBoy {
                        let vb_bindings = vb1_bindings_clone.borrow();
                        let rebinding = ms.rebind_controller == Some(1);
                        let cols = 4;
                        let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                        for k in 0..config::VB_BUTTON_COUNT {
                            let row = k / cols;
                            let col = k % cols;
                            let bx = cx + gap_x + col * (btn_w + gap_x);
                            let by = grid_y0 + row * (btn_h + gap_y);
                            let is_hovered = ms.hovered_ctrl_button == Some(k);
                            let is_rebinding = rebinding && ms.rebind_button == Some(k);
                            let txt = if is_rebinding { "?" } else { &vb_bindings[config::VB_DISPLAY_ORDER[k]] };
                            let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                            let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                            draw_rect(&mut buffer, bx, by, btn_w, btn_h, width, border);
                            draw_rect(&mut buffer, bx + 1, by + 1, btn_w - 2, btn_h - 2, width, bg);
                            let lbl = config::VB_LABELS[k];
                            let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((btn_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                            let key_vw = txt.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((btn_w as f32 - key_vw) / 2.0).round() as usize, by + (12.0 * scale).round() as usize, width, txt, menu_text, scale);
                        }
                    } else if c1t == config::ControllerType::FourScore || *controller2_type_clone.borrow() == config::ControllerType::FourScore {
                        let grid_h = 4 * (btn_h + gap_y) + btn_h;
                        let player_cfgs = [(1, &*controller1_bindings_clone.borrow(), "P1"), (2, &*controller2_bindings_clone.borrow(), "P2")];
                        for (pi, (pnum, bindings, plbl)) in player_cfgs.iter().enumerate() {
                            let yoff = grid_y0 + pi * (grid_h + btn_h / 2);
                            draw_text(&mut buffer, cx + (4.0 * scale).round() as usize, yoff + (2.0 * scale).round() as usize, width, plbl, colors.btn_sub_label, scale);
                            let btn_y0 = yoff + (btn_h * 3 / 4) as usize;
                            let rebinding = ms.rebind_controller == Some(*pnum);
                            let rebind_btn = ms.rebind_button;
                            for i in 0..config::GAMEPAD_BUTTON_COUNT {
                                let row = i / 2;
                                let by = btn_y0 + row * (btn_h + gap_y);
                                let bx = if i % 2 == 0 { cx + gap_x } else { cx + gap_x * 2 + btn_w };
                                let btn_idx = i * 2 + pi;
                                let is_hovered = ms.hovered_ctrl_button == Some(btn_idx);
                                let is_rebinding = rebinding && rebind_btn == Some(i);
                                let txt = if is_rebinding { "?" } else { &bindings[i] };
                                let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                                let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                                draw_rect(&mut buffer, bx, by, btn_w, btn_h, width, border);
                                draw_rect(&mut buffer, bx + 1, by + 1, btn_w - 2, btn_h - 2, width, bg);
                                let lbl = labels[i];
                                let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                                draw_text(&mut buffer, bx + ((btn_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                                let key_txt = txt;
                                let key_vw = key_txt.chars().count() as f32 * 8.0 * scale;
                                draw_text(&mut buffer, bx + ((btn_w as f32 - key_vw) / 2.0).round() as usize, by + (12.0 * scale).round() as usize, width, key_txt, menu_text, scale);
                            }
                        }
                    } else {
                        let bindings = controller1_bindings_clone.borrow();
                        let rebinding = ms.rebind_controller == Some(1);
                        let rebind_btn = ms.rebind_button;
                        for i in 0..config::GAMEPAD_BUTTON_COUNT {
                            let row = i / 2;
                            let by = grid_y0 + row * (btn_h + gap_y);
                            let bx = if i % 2 == 0 { cx + gap_x } else { cx + gap_x * 2 + btn_w };
                            let is_hovered = ms.hovered_ctrl_button == Some(i);
                            let is_rebinding = rebinding && rebind_btn == Some(i);
                            let txt = if is_rebinding { "?" } else { &bindings[i] };
                            let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                            let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                            draw_rect(&mut buffer, bx, by, btn_w, btn_h, width, border);
                            draw_rect(&mut buffer, bx + 1, by + 1, btn_w - 2, btn_h - 2, width, bg);
                            let lbl = labels[i];
                            let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((btn_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                            let key_txt = txt;
                            let key_vw = key_txt.chars().count() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((btn_w as f32 - key_vw) / 2.0).round() as usize, by + (12.0 * scale).round() as usize, width, key_txt, menu_text, scale);
                        }
                    }
                    let act_btn_w = (70.0 * scale).round() as usize;
                    let act_btn_h = (24.0 * scale).round() as usize;
                    let grid_h = 4 * (btn_h + gap_y) + btn_h;
                    let last_row_bottom = if c1t == config::ControllerType::Zapper || c1t == config::ControllerType::SNESMouse || c1t == config::ControllerType::SuborMouse { grid_y0 + btn_h } else if c1t == config::ControllerType::PowerPadA || c1t == config::ControllerType::PowerPadB || c1t == config::ControllerType::SNESPad { grid_y0 + 2 * (btn_h + gap_y) + btn_h } else if c1t == config::ControllerType::VirtualBoy { grid_y0 + 3 * (btn_h + gap_y) + btn_h } else if c1t == config::ControllerType::FourScore || *controller2_type_clone.borrow() == config::ControllerType::FourScore { grid_y0 + 2 * (grid_h + btn_h / 2) } else { grid_y0 + 4 * (btn_h + gap_y) + btn_h };
                    let act_y = last_row_bottom + (10.0 * scale).round() as usize;
                    let act_gap = (10.0 * scale).round() as usize;
                    let act_total = 2 * act_btn_w + act_gap;
                    let act_x0 = cx + (cw - act_total) / 2;
                    for (j, label) in ["Clear", "Reset"].iter().enumerate() {
                        let ax = act_x0 + j * (act_btn_w + act_gap);
                        let ah = point_in_rect(mouse_x, mouse_y, ax, act_y, act_btn_w, act_btn_h);
                        let abg = if ah { colors.box_bg_hover } else { colors.dropdown_bg };
                        draw_rect(&mut buffer, ax, act_y, act_btn_w, act_btn_h, width, colors.btn_border);
                        draw_rect(&mut buffer, ax + 1, act_y + 1, act_btn_w - 2, act_btn_h - 2, width, abg);
                        let tw = label.len() as f32 * 8.0 * scale;
                        draw_text(&mut buffer, ax + ((act_btn_w as f32 - tw) / 2.0).round() as usize, act_y + (7.0 * scale).round() as usize, width, label, menu_text, scale);
                    }
                }
                
                if ms.show_controller2_settings {
                    let (mouse_x, mouse_y) = ms.mouse_pos;
                    let cw = (440.0 * scale).round() as usize;
                    let title_h = (30.0 * scale).round() as usize;
                    let bth = (2.0 * scale).round() as usize;
                    let btn_w = (90.0 * scale).round() as usize;
                    let btn_h = (26.0 * scale).round() as usize;
                    let gap_y = (8.0 * scale).round() as usize;
                    let gap_x = (cw.saturating_sub(2 * btn_w)) / 3;
                    let c2t = *controller2_type_clone.borrow();
                    let fs_block_h = 4 * (btn_h + gap_y) + btn_h;
                    let tmp_g0 = title_h + (10.0 * scale).round() as usize;
                    let ch = if c2t == config::ControllerType::FourScore {
                        let fs_ch = tmp_g0 + 2 * (fs_block_h + btn_h / 2) + (40.0 * scale).round() as usize;
                        fs_ch.max(260)
                    } else if c2t == config::ControllerType::FamicomGamepad {
                        (295.0 * scale).round() as usize
                    } else {
                        (260.0 * scale).round() as usize
                    };
                    let cx = (width.saturating_sub(cw)) / 2;
                    let cy = (height.saturating_sub(ch)) / 2;
                    let grid_y0 = cy + tmp_g0;
                    let wbg = colors.window_bg;
                    let tbg = colors.dropdown_bg;
                    let wbrd = colors.window_border;
                    draw_rect(&mut buffer, cx, cy, cw, ch, width, wbg);
                    draw_rect(&mut buffer, cx, cy, cw, title_h, width, tbg);
                    draw_text(&mut buffer, cx + (10.0 * scale).round() as usize, cy + (8.0 * scale).round() as usize, width, "Controller 2 Settings", menu_text, scale);
                    draw_rect(&mut buffer, cx, cy, cw, bth, width, wbrd);
                    draw_rect(&mut buffer, cx, cy, bth, ch, width, wbrd);
                    draw_rect(&mut buffer, cx + cw - bth, cy, bth, ch, width, wbrd);
                    draw_rect(&mut buffer, cx, cy + ch - bth, cw, bth, width, wbrd);
                    let close_w = (20.0 * scale).round() as usize;
                    let close_h = (20.0 * scale).round() as usize;
                    let close_x = cx + cw - close_w - (10.0 * scale).round() as usize;
                    let close_y = cy + (5.0 * scale).round() as usize;
                    draw_rect(&mut buffer, close_x, close_y, close_w, close_h, width, colors.close_bg);
                    draw_text(&mut buffer, close_x + (6.0 * scale).round() as usize, close_y + (6.0 * scale).round() as usize, width, "X", colors.menu_text, scale);
                    let labels = ["A", "B", "Turbo A", "Turbo B", "Select", "Start", "Up", "Down", "Left", "Right"];
                    if c2t == config::ControllerType::Zapper || c2t == config::ControllerType::Paddle {
                        let total_w = 2 * btn_w + gap_x;
                        let trig_bx = cx + (cw - total_w) / 2;
                        let is_hovered = ms.hovered_ctrl_button == Some(0);
                        let is_rebinding = ms.rebind_controller == Some(2) && ms.rebind_button == Some(0);
                        let (lbl, txt) = if c2t == config::ControllerType::Zapper {
                            let zt = zapper_trigger_binding_clone.borrow();
                            ("Trigger", if is_rebinding { "?".to_string() } else { zt.clone() })
                        } else {
                            let pb = paddle2_button_binding_clone.borrow();
                            ("Button", if is_rebinding { "?".to_string() } else { pb.clone() })
                        };
                        let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                        let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                        draw_rect(&mut buffer, trig_bx, grid_y0, total_w, btn_h, width, border);
                        draw_rect(&mut buffer, trig_bx + 1, grid_y0 + 1, total_w - 2, btn_h - 2, width, bg);
                        let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                        draw_text(&mut buffer, trig_bx + ((total_w as f32 - lbl_vw) / 2.0).round() as usize, grid_y0 + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                        let key_vw = txt.len() as f32 * 8.0 * scale;
                        draw_text(&mut buffer, trig_bx + ((total_w as f32 - key_vw) / 2.0).round() as usize, grid_y0 + (14.0 * scale).round() as usize, width, &txt, menu_text, scale);
                    } else if c2t == config::ControllerType::SNESMouse || c2t == config::ControllerType::SuborMouse {
                        let mouse_bindings: Vec<String> = if c2t == config::ControllerType::SNESMouse {
                            snes_mouse2_bindings_clone.borrow().to_vec()
                        } else {
                            subor_mouse2_bindings_clone.borrow().to_vec()
                        };
                        let rebinding = ms.rebind_controller == Some(2);
                        let per_w = btn_w;
                        let half_gap = (cw.saturating_sub(2 * per_w)) / 3;
                        for i in 0..config::SNES_MOUSE_BUTTON_COUNT {
                            let bx = cx + half_gap + i * (per_w + half_gap);
                            let is_hovered = ms.hovered_ctrl_button == Some(i);
                            let is_rebinding_btn = rebinding && ms.rebind_button == Some(i);
                            let txt = if is_rebinding_btn { "?" } else { &mouse_bindings[i] };
                            let border = if is_rebinding_btn { colors.rebind_border } else { colors.box_border };
                            let bg = if is_rebinding_btn { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                            draw_rect(&mut buffer, bx, grid_y0, per_w, btn_h, width, border);
                            draw_rect(&mut buffer, bx + 1, grid_y0 + 1, per_w - 2, btn_h - 2, width, bg);
                            let lbl = config::SNES_MOUSE_LABELS[i];
                            let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((per_w as f32 - lbl_vw) / 2.0).round() as usize, grid_y0 + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                            let key_vw = txt.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((per_w as f32 - key_vw) / 2.0).round() as usize, grid_y0 + (14.0 * scale).round() as usize, width, txt, menu_text, scale);
                        }
                    } else if c2t == config::ControllerType::PowerPadA || c2t == config::ControllerType::PowerPadB {
                        let pp_bindings = powerpad2_bindings_clone.borrow();
                        let rebinding = ms.rebind_controller == Some(2);
                        let cols = 4;
                        let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                        for i in 0..config::POWERPAD_BUTTON_COUNT {
                            let row = i / cols;
                            let col = i % cols;
                            let bx = cx + gap_x + col * (btn_w + gap_x);
                            let by = grid_y0 + row * (btn_h + gap_y);
                            let is_hovered = ms.hovered_ctrl_button == Some(i);
                            let is_rebinding = rebinding && ms.rebind_button == Some(i);
                            let txt = if is_rebinding { "?" } else { &pp_bindings[i] };
                            let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                            let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                            draw_rect(&mut buffer, bx, by, btn_w, btn_h, width, border);
                            draw_rect(&mut buffer, bx + 1, by + 1, btn_w - 2, btn_h - 2, width, bg);
                            let lbl = config::POWERPAD_LABELS[i];
                            let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((btn_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                            let key_vw = txt.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((btn_w as f32 - key_vw) / 2.0).round() as usize, by + (12.0 * scale).round() as usize, width, txt, menu_text, scale);
                        }
                    } else if c2t == config::ControllerType::SNESPad {
                        let snes_bindings = snes2_bindings_clone.borrow();
                        let rebinding = ms.rebind_controller == Some(2);
                        let cols = 4;
                        let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                        for i in 0..config::SNES_BUTTON_COUNT {
                            let row = i / cols;
                            let col = i % cols;
                            let bx = cx + gap_x + col * (btn_w + gap_x);
                            let by = grid_y0 + row * (btn_h + gap_y);
                            let is_hovered = ms.hovered_ctrl_button == Some(i);
                            let is_rebinding = rebinding && ms.rebind_button == Some(i);
                            let txt = if is_rebinding { "?" } else { &snes_bindings[i] };
                            let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                            let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                            draw_rect(&mut buffer, bx, by, btn_w, btn_h, width, border);
                            draw_rect(&mut buffer, bx + 1, by + 1, btn_w - 2, btn_h - 2, width, bg);
                            let lbl = config::SNES_LABELS[i];
                            let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((btn_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                            let key_vw = txt.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((btn_w as f32 - key_vw) / 2.0).round() as usize, by + (12.0 * scale).round() as usize, width, txt, menu_text, scale);
                        }
                    } else if c2t == config::ControllerType::VirtualBoy {
                        let vb_bindings = vb2_bindings_clone.borrow();
                        let rebinding = ms.rebind_controller == Some(2);
                        let cols = 4;
                        let gap_x = (cw.saturating_sub(cols * btn_w)) / (cols + 1);
                        for k in 0..config::VB_BUTTON_COUNT {
                            let row = k / cols;
                            let col = k % cols;
                            let bx = cx + gap_x + col * (btn_w + gap_x);
                            let by = grid_y0 + row * (btn_h + gap_y);
                            let is_hovered = ms.hovered_ctrl_button == Some(k);
                            let is_rebinding = rebinding && ms.rebind_button == Some(k);
                            let txt = if is_rebinding { "?" } else { &vb_bindings[config::VB_DISPLAY_ORDER[k]] };
                            let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                            let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                            draw_rect(&mut buffer, bx, by, btn_w, btn_h, width, border);
                            draw_rect(&mut buffer, bx + 1, by + 1, btn_w - 2, btn_h - 2, width, bg);
                            let lbl = config::VB_LABELS[k];
                            let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((btn_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                            let key_vw = txt.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((btn_w as f32 - key_vw) / 2.0).round() as usize, by + (12.0 * scale).round() as usize, width, txt, menu_text, scale);
                        }
                    } else if c2t == config::ControllerType::FourScore {
                        let bindings_arr = [
                            controller3_bindings_clone.borrow(),
                            controller4_bindings_clone.borrow(),
                        ];
                        let rebinding = ms.rebind_controller;
                        let rebind_btn = ms.rebind_button;
                        for player in 0..2usize {
                            let yoff = grid_y0 + player * (fs_block_h + btn_h / 2);
                            let plbl = ["P3", "P4"][player];
                            draw_text(&mut buffer, cx + (4.0 * scale).round() as usize, yoff + (2.0 * scale).round() as usize, width, plbl, colors.btn_sub_label, scale);
                            let btn_y0 = yoff + (btn_h * 3 / 4) as usize;
                            for i in 0..config::GAMEPAD_BUTTON_COUNT {
                                let row = i / 2;
                                let by = btn_y0 + row * (btn_h + gap_y);
                                let bx = if i % 2 == 0 { cx + gap_x } else { cx + gap_x * 2 + btn_w };
                                let btn_idx = i * 4 + (player + 2);
                                let is_hovered = ms.hovered_ctrl_button == Some(btn_idx);
                                let is_rebinding = rebinding == Some(player as u8 + 3) && rebind_btn == Some(i);
                                let txt = if is_rebinding { "?" } else { &bindings_arr[player][i] };
                                let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                                let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                                draw_rect(&mut buffer, bx, by, btn_w, btn_h, width, border);
                                draw_rect(&mut buffer, bx + 1, by + 1, btn_w - 2, btn_h - 2, width, bg);
                                let lbl = labels[i];
                                let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                                draw_text(&mut buffer, bx + ((btn_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                                let key_txt = txt;
                                let key_vw = key_txt.chars().count() as f32 * 8.0 * scale;
                                draw_text(&mut buffer, bx + ((btn_w as f32 - key_vw) / 2.0).round() as usize, by + (12.0 * scale).round() as usize, width, key_txt, menu_text, scale);
                            }
                        }
                    } else {
                        let bindings = controller2_bindings_clone.borrow();
                        let rebinding = ms.rebind_controller == Some(2);
                        let rebind_btn = ms.rebind_button;
                        for i in 0..config::GAMEPAD_BUTTON_COUNT {
                            let row = i / 2;
                            let by = grid_y0 + row * (btn_h + gap_y);
                            let bx = if i % 2 == 0 { cx + gap_x } else { cx + gap_x * 2 + btn_w };
                            let is_hovered = ms.hovered_ctrl_button == Some(i);
                            let is_rebinding = rebinding && rebind_btn == Some(i);
                            let txt = if is_rebinding { "?" } else { &bindings[i] };
                            let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                            let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                            draw_rect(&mut buffer, bx, by, btn_w, btn_h, width, border);
                            draw_rect(&mut buffer, bx + 1, by + 1, btn_w - 2, btn_h - 2, width, bg);
                            let lbl = labels[i];
                            let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((btn_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                            let key_txt = txt;
                            let key_vw = key_txt.chars().count() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, bx + ((btn_w as f32 - key_vw) / 2.0).round() as usize, by + (12.0 * scale).round() as usize, width, key_txt, menu_text, scale);
                        }
                        if c2t == config::ControllerType::FamicomGamepad {
                            let mic_row = 5;
                            let mic_y = grid_y0 + mic_row * (btn_h + gap_y);
                            let mic_x = cx + (cw - btn_w) / 2;
                            let is_hovered = ms.hovered_ctrl_button == Some(10);
                            let is_rebinding = rebinding && rebind_btn == Some(10);
                            let mic_txt = if is_rebinding { "?".to_string() } else { famicom_mic_binding_clone.borrow().clone() };
                            let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                            let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                            draw_rect(&mut buffer, mic_x, mic_y, btn_w, btn_h, width, border);
                            draw_rect(&mut buffer, mic_x + 1, mic_y + 1, btn_w - 2, btn_h - 2, width, bg);
                            let lbl = "Mic";
                            let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, mic_x + ((btn_w as f32 - lbl_vw) / 2.0).round() as usize, mic_y + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                            let key_vw = mic_txt.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, mic_x + ((btn_w as f32 - key_vw) / 2.0).round() as usize, mic_y + (12.0 * scale).round() as usize, width, &mic_txt, menu_text, scale);
                        }
                    }
                    let act_btn_w = (70.0 * scale).round() as usize;
                            let act_btn_h = (24.0 * scale).round() as usize;
                            let last_row_bottom = if c2t == config::ControllerType::Zapper || c2t == config::ControllerType::Paddle || c2t == config::ControllerType::SNESMouse || c2t == config::ControllerType::SuborMouse { grid_y0 + btn_h } else if c2t == config::ControllerType::PowerPadA || c2t == config::ControllerType::PowerPadB || c2t == config::ControllerType::SNESPad { grid_y0 + 2 * (btn_h + gap_y) + btn_h } else if c2t == config::ControllerType::VirtualBoy { grid_y0 + 3 * (btn_h + gap_y) + btn_h } else if c2t == config::ControllerType::FourScore { grid_y0 + 2 * (fs_block_h + btn_h / 2) } else if c2t == config::ControllerType::FamicomGamepad { grid_y0 + 5 * (btn_h + gap_y) + btn_h } else { grid_y0 + 4 * (btn_h + gap_y) + btn_h };
                    let act_y = last_row_bottom + (10.0 * scale).round() as usize;
                    let act_gap = (10.0 * scale).round() as usize;
                    let act_total = 2 * act_btn_w + act_gap;
                    let act_x0 = cx + (cw - act_total) / 2;
                    for (j, label) in ["Clear", "Reset"].iter().enumerate() {
                        let ax = act_x0 + j * (act_btn_w + act_gap);
                        let ah = point_in_rect(mouse_x, mouse_y, ax, act_y, act_btn_w, act_btn_h);
                        let abg = if ah { colors.box_bg_hover } else { colors.dropdown_bg };
                        draw_rect(&mut buffer, ax, act_y, act_btn_w, act_btn_h, width, colors.btn_border);
                        draw_rect(&mut buffer, ax + 1, act_y + 1, act_btn_w - 2, act_btn_h - 2, width, abg);
                        let tw = label.len() as f32 * 8.0 * scale;
                        draw_text(&mut buffer, ax + ((act_btn_w as f32 - tw) / 2.0).round() as usize, act_y + (7.0 * scale).round() as usize, width, label, menu_text, scale);
                    }
                }
                
                if ms.show_expansion_settings {
                    let (mouse_x, mouse_y) = ms.mouse_pos;
                    let adapter_type = *expansion_adapter_type_clone.borrow();
                    let is_adapter = adapter_type != config::ExpansionAdapterType::None;
                    let pair = ms.adapter_pair.unwrap_or(0);
                    let player_offset = pair * 2;
                    let dialog_sub_ports = 2;
                    let sw = (440.0 * scale).round() as usize;
                    let title_h = (30.0 * scale).round() as usize;
                    let bth = (2.0 * scale).round() as usize;
                    let btn_w = (90.0 * scale).round() as usize;
                    let btn_h = (26.0 * scale).round() as usize;
                    let gap_y = (8.0 * scale).round() as usize;
                    let gap_x = (sw.saturating_sub(2 * btn_w)) / 3;
                    let grid_h = 4 * (btn_h + gap_y) + btn_h;
                    let tmp_g0 = title_h + (10.0 * scale).round() as usize;
                    let sh = if is_adapter {
                        let ch = tmp_g0 + dialog_sub_ports * (grid_h + btn_h / 2) + (40.0 * scale).round() as usize;
                        ch.max(260)
                    } else if expansion_type_clone.borrow().is_family_trainer() {
                        let mbtn_h = (30.0 * scale).round() as usize;
                        let mrow_gap = (6.0 * scale).round() as usize;
                        let act_btn_h = (24.0 * scale).round() as usize;
                        (tmp_g0 + btn_h / 2 + 3 * mbtn_h + 2 * mrow_gap + btn_h + (10.0 * scale).round() as usize + act_btn_h + (20.0 * scale).round() as usize) as usize
                    } else if expansion_type_clone.borrow().is_hyper_shot() {
                        let mbtn_h = (30.0 * scale).round() as usize;
                        let mrow_gap = (6.0 * scale).round() as usize;
                        let act_btn_h = (24.0 * scale).round() as usize;
                        (tmp_g0 + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + btn_h + (10.0 * scale).round() as usize + act_btn_h + (20.0 * scale).round() as usize) as usize
                    } else if expansion_type_clone.borrow().is_bandai_hyper_shot() {
                        let mbtn_h = (30.0 * scale).round() as usize;
                        let mrow_gap = (6.0 * scale).round() as usize;
                        let act_btn_h = (24.0 * scale).round() as usize;
                        (tmp_g0 + btn_h / 2 + 3 * mbtn_h + 2 * mrow_gap + btn_h + (10.0 * scale).round() as usize + act_btn_h + (20.0 * scale).round() as usize) as usize
                    } else if expansion_type_clone.borrow().is_family_basic() {
                        let mbtn_h = (20.0 * scale).round() as usize;
                        let mrow_gap = (4.0 * scale).round() as usize;
                        let act_btn_h = (24.0 * scale).round() as usize;
                        (tmp_g0 + btn_h / 2 + 9 * mbtn_h + 8 * mrow_gap + btn_h + (10.0 * scale).round() as usize + act_btn_h + (20.0 * scale).round() as usize) as usize
                    } else if expansion_type_clone.borrow().is_party_tap() {
                        let mbtn_h = (30.0 * scale).round() as usize;
                        let mrow_gap = (6.0 * scale).round() as usize;
                        let act_btn_h = (24.0 * scale).round() as usize;
                        (tmp_g0 + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + btn_h + (10.0 * scale).round() as usize + act_btn_h + (20.0 * scale).round() as usize) as usize
                    } else if expansion_type_clone.borrow().is_pachinko() {
                        let mbtn_h = (30.0 * scale).round() as usize;
                        let mrow_gap = (6.0 * scale).round() as usize;
                        let act_btn_h = (24.0 * scale).round() as usize;
                        (tmp_g0 + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + btn_h + (10.0 * scale).round() as usize + act_btn_h + (20.0 * scale).round() as usize) as usize
                    } else if expansion_type_clone.borrow().is_exciting_boxing() {
                        let mbtn_h = (30.0 * scale).round() as usize;
                        let mrow_gap = (6.0 * scale).round() as usize;
                        let act_btn_h = (24.0 * scale).round() as usize;
                        (tmp_g0 + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + btn_h + (10.0 * scale).round() as usize + act_btn_h + (20.0 * scale).round() as usize) as usize
                    } else if expansion_type_clone.borrow().is_jissen_mahjong() {
                        let mbtn_h = (30.0 * scale).round() as usize;
                        let mrow_gap = (6.0 * scale).round() as usize;
                        let act_btn_h = (24.0 * scale).round() as usize;
                        (tmp_g0 + btn_h / 2 + 7 * mbtn_h + 6 * mrow_gap + btn_h + (10.0 * scale).round() as usize + act_btn_h + (20.0 * scale).round() as usize) as usize
                    } else if expansion_type_clone.borrow().is_subor_keyboard() {
                        let mbtn_h = (20.0 * scale).round() as usize;
                        let mrow_gap = (4.0 * scale).round() as usize;
                        let act_btn_h = (24.0 * scale).round() as usize;
                        (tmp_g0 + btn_h / 2 + 13 * mbtn_h + 12 * mrow_gap + btn_h + (10.0 * scale).round() as usize + act_btn_h + (20.0 * scale).round() as usize) as usize
                    } else {
                        (150.0 * scale).round() as usize
                    };
                    let sx = (width.saturating_sub(sw)) / 2;
                    let sy = (height.saturating_sub(sh)) / 2;
                    let grid_y0 = sy + tmp_g0;
                    let wbg = colors.window_bg;
                    let tbg = colors.dropdown_bg;
                    let wbrd = colors.window_border;
                    draw_rect(&mut buffer, sx, sy, sw, sh, width, wbg);
                    draw_rect(&mut buffer, sx, sy, sw, title_h, width, tbg);
                    let dialog_title = if is_adapter { format!("Expansion Port Settings - P{}/P{}", player_offset + 1, player_offset + 2) } else { "Famicom Expansion Port Settings".to_string() };
                    draw_text(&mut buffer, sx + (10.0 * scale).round() as usize, sy + (8.0 * scale).round() as usize, width, &dialog_title, menu_text, scale);
                    draw_rect(&mut buffer, sx, sy, sw, bth, width, wbrd);
                    draw_rect(&mut buffer, sx, sy, bth, sh, width, wbrd);
                    draw_rect(&mut buffer, sx + sw - bth, sy, bth, sh, width, wbrd);
                    draw_rect(&mut buffer, sx, sy + sh - bth, sw, bth, width, wbrd);
                    let close_w = (20.0 * scale).round() as usize;
                    let close_h = (20.0 * scale).round() as usize;
                    let close_x = sx + sw - close_w - (10.0 * scale).round() as usize;
                    let close_y = sy + (5.0 * scale).round() as usize;
                    draw_rect(&mut buffer, close_x, close_y, close_w, close_h, width, colors.close_bg);
                    draw_text(&mut buffer, close_x + (6.0 * scale).round() as usize, close_y + (6.0 * scale).round() as usize, width, "X", colors.menu_text, scale);
                    if is_adapter {
                        let bindings_arr = [
                            expansion1_bindings_clone.borrow(),
                            expansion2_bindings_clone.borrow(),
                            expansion3_bindings_clone.borrow(),
                            expansion4_bindings_clone.borrow(),
                        ];
                        let labels = ["A", "B", "Turbo A", "Turbo B", "Select", "Start", "Up", "Down", "Left", "Right"];
                        for local in 0..dialog_sub_ports {
                            let player = player_offset + local;
                            let yoff = grid_y0 + local * (grid_h + btn_h / 2);
                            let plbl = format!("P{}", player + 1);
                            draw_text(&mut buffer, sx + (4.0 * scale).round() as usize, yoff + (2.0 * scale).round() as usize, width, &plbl, colors.btn_sub_label, scale);
                            let btn_y0 = yoff + (btn_h * 3 / 4) as usize;
                            let rebinding = ms.rebind_controller == Some(local as u8 + 1);
                            for i in 0..config::GAMEPAD_BUTTON_COUNT {
                                let row = i / 2;
                                let by = btn_y0 + row * (btn_h + gap_y);
                                let bx = if i % 2 == 0 { sx + gap_x } else { sx + gap_x * 2 + btn_w };
                                let is_rebinding = rebinding && ms.rebind_button == Some(i);
                                let txt = if is_rebinding { "?".to_string() } else { bindings_arr[player][i].clone() };
                                let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                                let is_hovered = ms.hovered_expansion_button == Some(local * config::GAMEPAD_BUTTON_COUNT + i);
                                let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                                draw_rect(&mut buffer, bx, by, btn_w, btn_h, width, border);
                                draw_rect(&mut buffer, bx + 1, by + 1, btn_w - 2, btn_h - 2, width, bg);
                                let lbl = labels[i];
                                let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                                draw_text(&mut buffer, bx + ((btn_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                                let key_txt = truncate_text(&txt, btn_w, scale);
                                let key_vw = key_txt.chars().count() as f32 * 8.0 * scale;
                                draw_text(&mut buffer, bx + ((btn_w as f32 - key_vw) / 2.0).round() as usize, by + (12.0 * scale).round() as usize, width, &key_txt, menu_text, scale);
                            }
                        }
                        let act_btn_w = (70.0 * scale).round() as usize;
                        let act_btn_h = (24.0 * scale).round() as usize;
                        let act_gap = (10.0 * scale).round() as usize;
                        let act_total = 2 * act_btn_w + act_gap;
                        let act_x0 = sx + (sw.saturating_sub(act_total)) / 2;
                        let act_y = grid_y0 + dialog_sub_ports * (grid_h + btn_h / 2) + (10.0 * scale).round() as usize;
                        for (j, label) in ["Clear", "Reset"].iter().enumerate() {
                            let ax = act_x0 + j * (act_btn_w + act_gap);
                            let ah = point_in_rect(mouse_x, mouse_y, ax, act_y, act_btn_w, act_btn_h);
                            let abg = if ah { colors.box_bg_hover } else { colors.dropdown_bg };
                            draw_rect(&mut buffer, ax, act_y, act_btn_w, act_btn_h, width, colors.btn_border);
                            draw_rect(&mut buffer, ax + 1, act_y + 1, act_btn_w - 2, act_btn_h - 2, width, abg);
                            let tw = label.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, ax + ((act_btn_w as f32 - tw) / 2.0).round() as usize, act_y + (7.0 * scale).round() as usize, width, label, menu_text, scale);
                        }
                    } else {
                        let is_famicom_zapper = *expansion_type_clone.borrow() == config::ExpansionType::FamicomZapper;
                        let is_oeka = *expansion_type_clone.borrow() == config::ExpansionType::OekaKidsTablet;
                        let is_family_trainer = expansion_type_clone.borrow().is_family_trainer();
                        let is_hyper_shot = expansion_type_clone.borrow().is_hyper_shot();
                        let is_family_basic = expansion_type_clone.borrow().is_family_basic();
                        let is_party_tap = expansion_type_clone.borrow().is_party_tap();
                        let is_pachinko = expansion_type_clone.borrow().is_pachinko();
                        let is_exciting_boxing = expansion_type_clone.borrow().is_exciting_boxing();
                        let is_jissen_mahjong = expansion_type_clone.borrow().is_jissen_mahjong();
                        let is_subor_keyboard = expansion_type_clone.borrow().is_subor_keyboard();
                        let is_bandai_hyper_shot = expansion_type_clone.borrow().is_bandai_hyper_shot();
                        if is_family_trainer {
                            let ft = family_trainer_bindings_clone.borrow();
                            let btn_gap = (6.0 * scale).round() as usize;
                            let mcol_w = (sw.saturating_sub(5 * btn_gap)) / 4;
                            let mbtn_h = (30.0 * scale).round() as usize;
                            let mrow_gap = (6.0 * scale).round() as usize;
                            let m_y0 = grid_y0 + ((btn_h as f32 / 2.0).round() as usize);
                            for r in 0..3usize {
                                for c in 0..4usize {
                                    let i = r * 4 + c;
                                    let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                    let by = m_y0 + r * (mbtn_h + mrow_gap);
                                    let is_rebinding = ms.rebind_controller == Some(0) && ms.rebind_button == Some(i);
                                    let txt = if is_rebinding { "?".to_string() } else { ft[i].clone() };
                                    let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                                    let is_hovered = ms.hovered_expansion_button == Some(i);
                                    let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                                    draw_rect(&mut buffer, bx, by, mcol_w, mbtn_h, width, border);
                                    draw_rect(&mut buffer, bx + 1, by + 1, mcol_w - 2, mbtn_h - 2, width, bg);
                                    let lbl = config::POWERPAD_LABELS[i];
                                    let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                                    let key_txt = truncate_text(&txt, mcol_w, scale);
                                    let key_vw = key_txt.chars().count() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - key_vw) / 2.0).round() as usize, by + (14.0 * scale).round() as usize, width, &key_txt, menu_text, scale);
                                }
                            }
                            } else if is_hyper_shot {
                            let hs = hyper_shot_bindings_clone.borrow();
                            let btn_gap = (6.0 * scale).round() as usize;
                            let mcol_w = (sw.saturating_sub(3 * btn_gap)) / 2;
                            let mbtn_h = (30.0 * scale).round() as usize;
                            let mrow_gap = (6.0 * scale).round() as usize;
                            let m_y0 = grid_y0 + ((btn_h as f32 / 2.0).round() as usize);
                            for r in 0..2usize {
                                for c in 0..2usize {
                                    let i = r * 2 + c;
                                    let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                    let by = m_y0 + r * (mbtn_h + mrow_gap);
                                    let is_rebinding = ms.rebind_controller == Some(0) && ms.rebind_button == Some(i);
                                    let txt = if is_rebinding { "?".to_string() } else { hs[i].clone() };
                                    let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                                    let is_hovered = ms.hovered_expansion_button == Some(i);
                                    let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                                    draw_rect(&mut buffer, bx, by, mcol_w, mbtn_h, width, border);
                                    draw_rect(&mut buffer, bx + 1, by + 1, mcol_w - 2, mbtn_h - 2, width, bg);
                                    let lbl = config::HYPER_SHOT_LABELS[i];
                                    let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                                    let key_txt = truncate_text(&txt, mcol_w, scale);
                                    let key_vw = key_txt.chars().count() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - key_vw) / 2.0).round() as usize, by + (14.0 * scale).round() as usize, width, &key_txt, menu_text, scale);
                                }
                            }
                        } else if is_bandai_hyper_shot {
                            let bh = bandai_hyper_bindings_clone.borrow();
                            let btn_gap = (6.0 * scale).round() as usize;
                            let mcol_w = (sw.saturating_sub(5 * btn_gap)) / 4;
                            let mbtn_h = (30.0 * scale).round() as usize;
                            let mrow_gap = (6.0 * scale).round() as usize;
                            let m_y0 = grid_y0 + ((btn_h as f32 / 2.0).round() as usize);
                            for r in 0..3usize {
                                for c in 0..4usize {
                                    let i = r * 4 + c;
                                    if i >= config::BANDAI_HYPER_SHOT_BUTTON_COUNT { continue; }
                                    let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                    let by = m_y0 + r * (mbtn_h + mrow_gap);
                                    let is_rebinding = ms.rebind_controller == Some(0) && ms.rebind_button == Some(i);
                                    let txt = if is_rebinding { "?".to_string() } else { bh[i].clone() };
                                    let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                                    let is_hovered = ms.hovered_expansion_button == Some(i);
                                    let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                                    draw_rect(&mut buffer, bx, by, mcol_w, mbtn_h, width, border);
                                    draw_rect(&mut buffer, bx + 1, by + 1, mcol_w - 2, mbtn_h - 2, width, bg);
                                    let lbl = config::BANDAI_HYPER_SHOT_LABELS[i];
                                    let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                                    let key_txt = truncate_text(&txt, mcol_w, scale);
                                    let key_vw = key_txt.chars().count() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - key_vw) / 2.0).round() as usize, by + (14.0 * scale).round() as usize, width, &key_txt, menu_text, scale);
                                }
                            }
                        } else if is_family_basic {
                            let fb = family_basic_bindings_clone.borrow();
                            let btn_gap = (4.0 * scale).round() as usize;
                            let mcol_w = (sw.saturating_sub(9 * btn_gap)) / 8;
                            let mbtn_h = (20.0 * scale).round() as usize;
                            let mrow_gap = (4.0 * scale).round() as usize;
                            let m_y0 = grid_y0 + ((btn_h as f32 / 2.0).round() as usize);
                            for r in 0..9usize {
                                for c in 0..8usize {
                                    let i = r * 8 + c;
                                    if i >= config::FAMILY_BASIC_BUTTON_COUNT { continue; }
                                    let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                    let by = m_y0 + r * (mbtn_h + mrow_gap);
                                    let is_rebinding = ms.rebind_controller == Some(0) && ms.rebind_button == Some(i);
                                    let txt = if is_rebinding { "?".to_string() } else { fb[i].clone() };
                                    let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                                    let is_hovered = ms.hovered_expansion_button == Some(i);
                                    let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                                    draw_rect(&mut buffer, bx, by, mcol_w, mbtn_h, width, border);
                                    draw_rect(&mut buffer, bx + 1, by + 1, mcol_w - 2, mbtn_h - 2, width, bg);
                                    let lbl = config::FAMILY_BASIC_LABELS[i];
                                    let max_chars = (mcol_w / (8.0 * scale).round() as usize).max(1);
                                    let lbl_disp = if lbl.len() > max_chars { &lbl[..max_chars] } else { lbl };
                                    let lbl_vw = lbl_disp.len() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - lbl_vw) / 2.0).round() as usize, by + (1.0 * scale).round() as usize, width, lbl_disp, colors.btn_sub_label, scale);
                                    let max_key = (mcol_w / (8.0 * scale).round() as usize).max(1);
                                    let txt_disp = if txt.len() > max_key { &txt[..max_key] } else { &txt[..] };
                                    let key_vw = txt_disp.len() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - key_vw) / 2.0).round() as usize, by + (10.0 * scale).round() as usize, width, txt_disp, menu_text, scale);
                                }
                            }
                        } else if is_party_tap {
                            let pt = party_tap_bindings_clone.borrow();
                            let btn_gap = (6.0 * scale).round() as usize;
                            let mcol_w = (sw.saturating_sub(4 * btn_gap)) / 3;
                            let mbtn_h = (30.0 * scale).round() as usize;
                            let mrow_gap = (6.0 * scale).round() as usize;
                            let m_y0 = grid_y0 + ((btn_h as f32 / 2.0).round() as usize);
                            for r in 0..2usize {
                                for c in 0..3usize {
                                    let i = r * 3 + c;
                                    let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                    let by = m_y0 + r * (mbtn_h + mrow_gap);
                                    let is_rebinding = ms.rebind_controller == Some(0) && ms.rebind_button == Some(i);
                                    let txt = if is_rebinding { "?".to_string() } else { pt[i].clone() };
                                    let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                                    let is_hovered = ms.hovered_expansion_button == Some(i);
                                    let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                                    draw_rect(&mut buffer, bx, by, mcol_w, mbtn_h, width, border);
                                    draw_rect(&mut buffer, bx + 1, by + 1, mcol_w - 2, mbtn_h - 2, width, bg);
                                    let lbl = config::PARTY_TAP_LABELS[i];
                                    let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                                    let key_txt = truncate_text(&txt, mcol_w, scale);
                                    let key_vw = key_txt.chars().count() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - key_vw) / 2.0).round() as usize, by + (14.0 * scale).round() as usize, width, &key_txt, menu_text, scale);
                                }
                            }
                        } else if is_pachinko {
                            let pc = pachinko_bindings_clone.borrow();
                            let btn_gap = (6.0 * scale).round() as usize;
                            let mcol_w = (sw.saturating_sub(3 * btn_gap)) / 2;
                            let mbtn_h = (30.0 * scale).round() as usize;
                            let m_y0 = grid_y0 + ((btn_h as f32 / 2.0).round() as usize);
                            for i in 0..config::PACHINKO_BUTTON_COUNT {
                                let bx = sx + btn_gap + i * (mcol_w + btn_gap);
                                let by = m_y0;
                                let is_rebinding = ms.rebind_controller == Some(0) && ms.rebind_button == Some(i);
                                let txt = if is_rebinding { "?".to_string() } else { pc[i].clone() };
                                let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                                let is_hovered = ms.hovered_expansion_button == Some(i);
                                let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                                draw_rect(&mut buffer, bx, by, mcol_w, mbtn_h, width, border);
                                draw_rect(&mut buffer, bx + 1, by + 1, mcol_w - 2, mbtn_h - 2, width, bg);
                                let lbl = config::PACHINKO_LABELS[i];
                                let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                                    let key_txt = truncate_text(&txt, mcol_w, scale);
                                    let key_vw = key_txt.chars().count() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - key_vw) / 2.0).round() as usize, by + (14.0 * scale).round() as usize, width, &key_txt, menu_text, scale);
                                }
                        } else if is_exciting_boxing {
                            let eb = punching_bag_bindings_clone.borrow();
                            let btn_gap = (6.0 * scale).round() as usize;
                            let mcol_w = (sw.saturating_sub(5 * btn_gap)) / 4;
                            let mbtn_h = (30.0 * scale).round() as usize;
                            let mrow_gap = (6.0 * scale).round() as usize;
                            let m_y0 = grid_y0 + ((btn_h as f32 / 2.0).round() as usize);
                            for r in 0..2usize {
                                for c in 0..4usize {
                                    let i = r * 4 + c;
                                    let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                    let by = m_y0 + r * (mbtn_h + mrow_gap);
                                    let is_rebinding = ms.rebind_controller == Some(0) && ms.rebind_button == Some(i);
                                    let txt = if is_rebinding { "?".to_string() } else { eb[i].clone() };
                                    let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                                    let is_hovered = ms.hovered_expansion_button == Some(i);
                                    let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                                    draw_rect(&mut buffer, bx, by, mcol_w, mbtn_h, width, border);
                                    draw_rect(&mut buffer, bx + 1, by + 1, mcol_w - 2, mbtn_h - 2, width, bg);
                                    let lbl = config::EXCITING_BOXING_LABELS[i];
                                    let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                                    let key_txt = truncate_text(&txt, mcol_w, scale);
                                    let key_vw = key_txt.chars().count() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - key_vw) / 2.0).round() as usize, by + (14.0 * scale).round() as usize, width, &key_txt, menu_text, scale);
                                }
                            }
                        } else if is_jissen_mahjong {
                            let jm = jissen_mahjong_bindings_clone.borrow();
                            let btn_gap = (6.0 * scale).round() as usize;
                            let mcol_w = (sw.saturating_sub(4 * btn_gap)) / 3;
                            let mbtn_h = (30.0 * scale).round() as usize;
                            let mrow_gap = (6.0 * scale).round() as usize;
                            let m_y0 = grid_y0 + ((btn_h as f32 / 2.0).round() as usize);
                            for r in 0..7usize {
                                for c in 0..3usize {
                                    let i = r * 3 + c;
                                    let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                    let by = m_y0 + r * (mbtn_h + mrow_gap);
                                    let is_rebinding = ms.rebind_controller == Some(0) && ms.rebind_button == Some(i);
                                    let txt = if is_rebinding { "?".to_string() } else { jm[i].clone() };
                                    let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                                    let is_hovered = ms.hovered_expansion_button == Some(i);
                                    let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                                    draw_rect(&mut buffer, bx, by, mcol_w, mbtn_h, width, border);
                                    draw_rect(&mut buffer, bx + 1, by + 1, mcol_w - 2, mbtn_h - 2, width, bg);
                                    let lbl = config::JISSEN_MAHJONG_LABELS[i];
                                    let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - lbl_vw) / 2.0).round() as usize, by + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                                    let key_txt = truncate_text(&txt, mcol_w, scale);
                                    let key_vw = key_txt.chars().count() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - key_vw) / 2.0).round() as usize, by + (14.0 * scale).round() as usize, width, &key_txt, menu_text, scale);
                                }
                            }
                        } else if is_subor_keyboard {
                            let sk = subor_keyboard_bindings_clone.borrow();
                            let btn_gap = (4.0 * scale).round() as usize;
                            let mcol_w = (sw.saturating_sub(9 * btn_gap)) / 8;
                            let mbtn_h = (20.0 * scale).round() as usize;
                            let mrow_gap = (4.0 * scale).round() as usize;
                            let m_y0 = grid_y0 + ((btn_h as f32 / 2.0).round() as usize);
                            for r in 0..13usize {
                                for c in 0..8usize {
                                    let i = r * 8 + c;
                                    if i >= config::SUBOR_KEYBOARD_BUTTON_COUNT { continue; }
                                    let bx = sx + btn_gap + c * (mcol_w + btn_gap);
                                    let by = m_y0 + r * (mbtn_h + mrow_gap);
                                    let is_rebinding = ms.rebind_controller == Some(0) && ms.rebind_button == Some(i);
                                    let txt = if is_rebinding { "?".to_string() } else { sk[i].clone() };
                                    let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                                    let is_hovered = ms.hovered_expansion_button == Some(i);
                                    let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                                    draw_rect(&mut buffer, bx, by, mcol_w, mbtn_h, width, border);
                                    draw_rect(&mut buffer, bx + 1, by + 1, mcol_w - 2, mbtn_h - 2, width, bg);
                                    let lbl = config::SUBOR_KEYBOARD_LABELS[i];
                                    let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - lbl_vw) / 2.0).round() as usize, by + (1.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                                    let key_txt = truncate_text(&txt, mcol_w, scale);
                                    let key_vw = key_txt.chars().count() as f32 * 8.0 * scale;
                                    draw_text(&mut buffer, bx + ((mcol_w as f32 - key_vw) / 2.0).round() as usize, by + (10.0 * scale).round() as usize, width, &key_txt, menu_text, scale);
                                }
                            }
                        } else {
                        let bind_total = 2 * btn_w + gap_x;
                        let bind_bx = sx + (sw.saturating_sub(bind_total)) / 2;
                        let lbl = if is_famicom_zapper { "Trigger" } else if is_oeka { "Click" } else { "Button" };
                        let binding_txt = if is_oeka {
                            expansion_oeka_click_binding_clone.borrow().clone()
                        } else if is_famicom_zapper {
                            expansion_zapper_trigger_binding_clone.borrow().clone()
                        } else {
                            expansion_paddle_button_binding_clone.borrow().clone()
                        };
                        let is_hovered = ms.hovered_expansion_button == Some(0);
                        let is_rebinding = ms.rebind_controller == Some(0) && ms.rebind_button == Some(0);
                        let txt = if is_rebinding { "?".to_string() } else { binding_txt };
                        let border = if is_rebinding { colors.rebind_border } else { colors.box_border };
                        let bg = if is_rebinding { colors.rebind_bg } else if is_hovered { colors.box_bg_hover } else { colors.box_bg_default };
                        draw_rect(&mut buffer, bind_bx, grid_y0, bind_total, btn_h, width, border);
                        draw_rect(&mut buffer, bind_bx + 1, grid_y0 + 1, bind_total - 2, btn_h - 2, width, bg);
                        let lbl_vw = lbl.len() as f32 * 8.0 * scale;
                        draw_text(&mut buffer, bind_bx + ((bind_total as f32 - lbl_vw) / 2.0).round() as usize, grid_y0 + (2.0 * scale).round() as usize, width, lbl, colors.btn_sub_label, scale);
                        let key_vw = txt.len() as f32 * 8.0 * scale;
                        draw_text(&mut buffer, bind_bx + ((bind_total as f32 - key_vw) / 2.0).round() as usize, grid_y0 + (14.0 * scale).round() as usize, width, &txt, menu_text, scale);
                        }
                        let act_btn_w = (70.0 * scale).round() as usize;
                        let act_btn_h = (24.0 * scale).round() as usize;
                        let act_gap = (10.0 * scale).round() as usize;
                        let act_total = 2 * act_btn_w + act_gap;
                        let act_x0 = sx + (sw.saturating_sub(act_total)) / 2;
                        let is_ft_act = expansion_type_clone.borrow().is_family_trainer();
                        let is_hs_act = expansion_type_clone.borrow().is_hyper_shot();
                        let is_fb_act = expansion_type_clone.borrow().is_family_basic();
                        let is_pt_act = expansion_type_clone.borrow().is_party_tap();
                        let is_pach_act = expansion_type_clone.borrow().is_pachinko();
                        let is_eb_act = expansion_type_clone.borrow().is_exciting_boxing();
                        let is_jm_act = expansion_type_clone.borrow().is_jissen_mahjong();
                        let is_sk_act = expansion_type_clone.borrow().is_subor_keyboard();
                        let is_bh_act = expansion_type_clone.borrow().is_bandai_hyper_shot();
                        let act_y = if is_ft_act {
                            let mbtn_h = (30.0 * scale).round() as usize;
                            let mrow_gap = (6.0 * scale).round() as usize;
                            grid_y0 + btn_h / 2 + 3 * mbtn_h + 2 * mrow_gap + (10.0 * scale).round() as usize
                        } else if is_hs_act {
                            let mbtn_h = (30.0 * scale).round() as usize;
                            let mrow_gap = (6.0 * scale).round() as usize;
                            grid_y0 + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + (10.0 * scale).round() as usize
                        } else if is_bh_act {
                            let mbtn_h = (30.0 * scale).round() as usize;
                            let mrow_gap = (6.0 * scale).round() as usize;
                            grid_y0 + btn_h / 2 + 3 * mbtn_h + 2 * mrow_gap + (10.0 * scale).round() as usize
                        } else if is_fb_act {
                            let mbtn_h = (20.0 * scale).round() as usize;
                            let mrow_gap = (4.0 * scale).round() as usize;
                            grid_y0 + btn_h / 2 + 9 * mbtn_h + 8 * mrow_gap + (10.0 * scale).round() as usize
                        } else if is_pt_act {
                            let mbtn_h = (30.0 * scale).round() as usize;
                            let mrow_gap = (6.0 * scale).round() as usize;
                            grid_y0 + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + (10.0 * scale).round() as usize
                        } else if is_pach_act {
                            let mbtn_h = (30.0 * scale).round() as usize;
                            grid_y0 + btn_h / 2 + mbtn_h + (10.0 * scale).round() as usize
                        } else if is_eb_act {
                            let mbtn_h = (30.0 * scale).round() as usize;
                            let mrow_gap = (6.0 * scale).round() as usize;
                            grid_y0 + btn_h / 2 + 2 * mbtn_h + 1 * mrow_gap + (10.0 * scale).round() as usize
                        } else if is_jm_act {
                            let mbtn_h = (30.0 * scale).round() as usize;
                            let mrow_gap = (6.0 * scale).round() as usize;
                            grid_y0 + btn_h / 2 + 7 * mbtn_h + 6 * mrow_gap + (10.0 * scale).round() as usize
                        } else if is_sk_act {
                            let mbtn_h = (20.0 * scale).round() as usize;
                            let mrow_gap = (4.0 * scale).round() as usize;
                            grid_y0 + btn_h / 2 + 13 * mbtn_h + 12 * mrow_gap + (10.0 * scale).round() as usize
                        } else {
                            grid_y0 + btn_h + (10.0 * scale).round() as usize
                        };
                        for (j, label) in ["Clear", "Reset"].iter().enumerate() {
                            let ax = act_x0 + j * (act_btn_w + act_gap);
                            let ah = point_in_rect(mouse_x, mouse_y, ax, act_y, act_btn_w, act_btn_h);
                            let abg = if ah { colors.box_bg_hover } else { colors.dropdown_bg };
                            draw_rect(&mut buffer, ax, act_y, act_btn_w, act_btn_h, width, colors.btn_border);
                            draw_rect(&mut buffer, ax + 1, act_y + 1, act_btn_w - 2, act_btn_h - 2, width, abg);
                            let tw = label.len() as f32 * 8.0 * scale;
                            draw_text(&mut buffer, ax + ((act_btn_w as f32 - tw) / 2.0).round() as usize, act_y + (7.0 * scale).round() as usize, width, label, menu_text, scale);
                        }
                    }
                }
                
                if ms.show_error {
                    let error_w = (400.0 * scale).round() as usize;
                    let error_h = (100.0 * scale).round() as usize;
                    let error_x = (width.saturating_sub(error_w)) / 2;
                    let error_y = (height.saturating_sub(error_h)) / 2;
                    let window_bg = colors.window_bg;
                    let window_border = colors.window_border;
                    
                    draw_rect(&mut buffer, error_x, error_y, error_w, error_h, width, window_bg);
                    draw_rect(&mut buffer, error_x, error_y, error_w, (2.0 * scale).round() as usize, width, window_border);
                    draw_rect(&mut buffer, error_x, error_y, (2.0 * scale).round() as usize, error_h, width, window_border);
                    draw_rect(&mut buffer, error_x + error_w - (2.0 * scale).round() as usize, error_y, (2.0 * scale).round() as usize, error_h, width, window_border);
                    draw_rect(&mut buffer, error_x, error_y + error_h - (2.0 * scale).round() as usize, error_w, (2.0 * scale).round() as usize, width, window_border);
                    
                    let text_max_w = error_w.saturating_sub((20.0 * scale).round() as usize);
                    let line_h = (10.0 * scale).round() as usize;
                    let text_h = measure_wrapped_height(&ms.error_message, text_max_w, scale);
                    let start_y = error_y + error_h.saturating_sub(text_h) / 2;
                    
                    let char_w = (8.0 * scale).round() as usize;
                    let max_chars = (text_max_w / char_w).max(1);
                    let words: Vec<&str> = ms.error_message.split(' ').collect();
                    let mut lines: Vec<String> = Vec::new();
                    let mut current = String::new();
                    for word in &words {
                        let candidate = if current.is_empty() { word.to_string() } else { format!("{} {}", current, word) };
                        if candidate.chars().count() <= max_chars {
                            current = candidate;
                        } else {
                            if !current.is_empty() { lines.push(current.clone()); }
                            let mut remaining = *word;
                            while remaining.chars().count() > max_chars {
                                let split_at = remaining.char_indices().nth(max_chars).map(|(i, _)| i).unwrap_or(remaining.len());
                                let (chunk, rest) = remaining.split_at(split_at);
                                lines.push(chunk.to_string());
                                remaining = rest;
                            }
                            current = remaining.to_string();
                        }
                    }
                    if !current.is_empty() { lines.push(current); }
                    if lines.is_empty() { lines.push(String::new()); }
                    
                    for (i, line) in lines.iter().enumerate() {
                        let line_w = line.chars().count() as f32 * 8.0 * scale;
                        let line_x = error_x + ((error_w as f32 - line_w) / 2.0).round() as usize;
                        draw_text(&mut buffer, line_x, start_y + i * line_h, width, line, menu_text, scale);
                    }
                    
                    let close_w = (20.0 * scale).round() as usize;
                    let close_h = (20.0 * scale).round() as usize;
                    let close_x = error_x + error_w - close_w - (10.0 * scale).round() as usize;
                    let close_y = error_y + (10.0 * scale).round() as usize;
                    draw_rect(&mut buffer, close_x, close_y, close_w, close_h, width, colors.close_bg);
                    draw_text(&mut buffer, close_x + (6.0 * scale).round() as usize, close_y + (6.0 * scale).round() as usize, width, "X", colors.menu_text, scale);
                }

                if ms.show_dip_switches {
                    let _is_custom = ms.dip_definition.is_some();
                    let row_h_base = 35.0 * scale;
                    let line_h = 10.0 * scale;
                    let (dialog_w, dialog_h, choice_x_offset, lines_per_row) = if let Some(ref game) = ms.dip_definition {
                        let max_name_len = game.settings.iter().map(|s| s.name.len()).max().unwrap_or(0);
                        let max_choice_len = game.settings.iter().flat_map(|s| s.choices.iter().map(|c| c.name.len())).max().unwrap_or(0);
                        let label_w = (max_name_len as f32) * 8.0 * scale;
                        let label_area = (label_w + (25.0 * scale)).max(225.0 * scale);
                        let choice_w = (max_choice_len as f32) * 8.0 * scale + (20.0 * scale);
                        let choice_area = choice_w.max(120.0 * scale);
                        let natural_w = (label_area + choice_area + (15.0 * scale)).max(320.0 * scale);
                        let max_dialog_w = (width as f32 - (80.0 * scale)).max(320.0 * scale);
                        let (final_w, final_label_area, lines) = if natural_w > max_dialog_w {
                            let cap_w = max_dialog_w;
                            let choice_area_f = choice_area.max(120.0 * scale);
                            let label_area_f = (cap_w - choice_area_f - (15.0 * scale)).max(180.0 * scale);
                            let avail_label_w = label_area_f - (15.0 * scale) - (5.0 * scale);
                            let cpl = (avail_label_w / (8.0 * scale)).floor() as usize;
                            let cpl = cpl.max(16);
                            let l: Vec<usize> = game.settings.iter().map(|s| ((s.name.len() + cpl - 1) / cpl).max(1)).collect();
                            (cap_w.round() as usize, label_area_f.round() as usize, l)
                        } else {
                            let l = vec![1; game.settings.len()];
                            (natural_w.round() as usize, label_area.round() as usize, l)
                        };
                        let total_extra: f32 = lines.iter().map(|&n| (n as f32 - 1.0) * line_h).sum();
                        let h = (50.0 * scale + row_h_base * game.settings.len() as f32 + total_extra).max(120.0 * scale).round() as usize;
                        (final_w, h, final_label_area, lines)
                    } else {
                        let w = (320.0 * scale).round() as usize;
                        let h = (260.0 * scale).round() as usize;
                        (w, h, 0, Vec::new())
                    };
                    
                    let dialog_x = (width.saturating_sub(dialog_w)) / 2;
                    let dialog_y = (height.saturating_sub(dialog_h)) / 2;
                    let window_bg = colors.window_bg;
                    let window_border = colors.window_border;
                    let title_bg = colors.dropdown_bg;
                    
                    draw_rect(&mut buffer, dialog_x, dialog_y, dialog_w, dialog_h, width, window_bg);
                    
                    let title_h = (30.0 * scale).round() as usize;
                    draw_rect(&mut buffer, dialog_x, dialog_y, dialog_w, title_h, width, title_bg);
                    draw_text(&mut buffer, dialog_x + (10.0 * scale).round() as usize, dialog_y + (8.0 * scale).round() as usize, width, "DIP Switches", menu_text, scale);
                    
                    let border_thickness = (2.0 * scale).round() as usize;
                    draw_rect(&mut buffer, dialog_x, dialog_y, dialog_w, border_thickness, width, window_border);
                    draw_rect(&mut buffer, dialog_x, dialog_y, border_thickness, dialog_h, width, window_border);
                    draw_rect(&mut buffer, dialog_x + dialog_w - border_thickness, dialog_y, border_thickness, dialog_h, width, window_border);
                    draw_rect(&mut buffer, dialog_x, dialog_y + dialog_h - border_thickness, dialog_w, border_thickness, width, window_border);

                    let close_w = (20.0 * scale).round() as usize;
                    let close_h = (20.0 * scale).round() as usize;
                    let close_x = dialog_x + dialog_w - close_w - (10.0 * scale).round() as usize;
                    let close_y = dialog_y + (5.0 * scale).round() as usize;
                    draw_rect(&mut buffer, close_x, close_y, close_w, close_h, width, colors.close_bg);
                    draw_text(&mut buffer, close_x + (6.0 * scale).round() as usize, close_y + (6.0 * scale).round() as usize, width, "X", colors.menu_text, scale);

                    let dip_val = emu_clone.lock().unwrap().get_dip_switches();

                    if let Some(ref game) = ms.dip_definition {
                        let choice_w = dialog_w - choice_x_offset - (15.0 * scale).round() as usize;
                        let choice_h = (24.0 * scale).round() as usize;
                        let choice_x = dialog_x + choice_x_offset;
                        
                        for i in 0..game.settings.len() {
                            let setting = &game.settings[i];
                            let num_lines = lines_per_row.get(i).copied().unwrap_or(1);
                            let row_extra: f32 = (0..i).map(|j| (lines_per_row.get(j).copied().unwrap_or(1) as f32 - 1.0) * line_h).sum();
                            let row_y = dialog_y + (45.0 * scale + i as f32 * row_h_base + row_extra).round() as usize;
                            let row_h_actual = row_h_base + (num_lines as f32 - 1.0) * line_h;
                            
                            if num_lines > 1 {
                                let avail_label_w = choice_x_offset - (15.0 * scale).round() as usize - (5.0 * scale).round() as usize;
                                let cpl = (avail_label_w as f32 / (8.0 * scale)).floor() as usize;
                                for line in 0..num_lines {
                                    let start = line * cpl;
                                    let end = (start + cpl).min(setting.name.len());
                                    let text_line = &setting.name[start..end];
                                    let line_y = row_y + (8.0 * scale).round() as usize + (line as f32 * line_h).round() as usize;
                                    draw_text(&mut buffer, dialog_x + (15.0 * scale).round() as usize, line_y, width, text_line, menu_text, scale);
                                }
                            } else {
                                draw_text(&mut buffer, dialog_x + (15.0 * scale).round() as usize, row_y + (8.0 * scale).round() as usize, width, &setting.name, menu_text, scale);
                            }
                            
                            let active_val = dip_val as u32 & setting.mask;
                            let choice_name = setting.choices.iter().find(|c| c.value == active_val).map(|c| c.name.as_str()).unwrap_or_else(|| setting.choices.first().map_or("Unknown", |c| c.name.as_str()));
                            
                            let max_choice_chars = if choice_w > (8.0 * scale) as usize { ((choice_w as f32 - (4.0 * scale)) / (8.0 * scale)).floor() as usize } else { 3 };
                            let display_choice = if choice_name.len() > max_choice_chars && max_choice_chars > 3 {
                                let mut s = String::with_capacity(max_choice_chars);
                                s.push_str(&choice_name[..max_choice_chars - 3]);
                                s.push_str("...");
                                s
                            } else {
                                choice_name.to_string()
                            };
                            
                            let box_bg = if ms.dip_hovered_bit == Some(i as u8) { colors.box_bg_hover } else { colors.box_bg_default };
                            let choice_y = row_y + ((row_h_actual - choice_h as f32) / 2.0).round() as usize;
                            draw_rect(&mut buffer, choice_x, choice_y, choice_w, choice_h, width, colors.box_border);
                            let inner_pad = (1.0 * scale).round() as usize;
                            draw_rect(&mut buffer, choice_x + inner_pad, choice_y + inner_pad, choice_w - inner_pad * 2, choice_h - inner_pad * 2, width, box_bg);
                            
                            let text_len = display_choice.len() as f32 * 8.0 * scale;
                            let text_x = choice_x + ((choice_w as f32 - text_len) / 2.0).round() as usize;
                            let text_y = choice_y + (8.0 * scale).round() as usize;
                            draw_text(&mut buffer, text_x, text_y, width, &display_choice, colors.menu_text, scale);
                        }
                    } else {
                        let row_start_y = dialog_y + (45.0 * scale).round() as usize;
                        let row_h = (25.0 * scale).round() as usize;
                        let cb_w = (16.0 * scale).round() as usize;
                        let cb_h = (16.0 * scale).round() as usize;
                        let cb_x = dialog_x + dialog_w - cb_w - (25.0 * scale).round() as usize;

                        for bit_idx in 0..8 {
                            let row_y = row_start_y + bit_idx * row_h;
                            let switch_num = bit_idx + 1;
                            let is_on = (dip_val & (1 << bit_idx)) != 0;
                            let label = format!("Switch {} (Bit {}):", switch_num, bit_idx);
                            
                            draw_text(&mut buffer, dialog_x + (20.0 * scale).round() as usize, row_y + (4.0 * scale).round() as usize, width, &label, menu_text, scale);

                            let cb_y = row_y;
                            let cb_bg = if ms.dip_hovered_bit == Some(bit_idx as u8) { colors.box_bg_hover } else { colors.box_bg_default };
                            draw_rect(&mut buffer, cb_x, cb_y, cb_w, cb_h, width, colors.box_border);
                            let cb_inner_padding = (2.0 * scale).round() as usize;
                            draw_rect(&mut buffer, cb_x + cb_inner_padding, cb_y + cb_inner_padding, cb_w - cb_inner_padding * 2, cb_h - cb_inner_padding * 2, width, cb_bg);

                            if is_on {
                                let fill_color = colors.dip_on_fill;
                                draw_rect(&mut buffer, cb_x + cb_inner_padding * 2, cb_y + cb_inner_padding * 2, cb_w - cb_inner_padding * 4, cb_h - cb_inner_padding * 4, width, fill_color);
                            }
                        }
                    }
                }
                
                if *fps_mode_clone.borrow() == config::FpsMode::Overlay {
                    let fps_text = format!("{} FPS", current_fps_clone.load(Ordering::Relaxed));
                    let overlay_x = (10.0 * scale).round() as usize;
                    let overlay_y = menu_height + (10.0 * scale).round() as usize;
                    draw_text(&mut buffer, overlay_x, overlay_y, width, &fps_text, colors.menu_text, scale);
                }
                
                drop(ms);

                if *hide_mouse_cursor_clone.borrow() {
                    let ms_state = menu_state_clone.borrow();
                    let ui_visible = ms_state.active_menu.is_some()
                        || ms_state.hovered_menu.is_some()
                        || ms_state.show_recent_submenu
                        || ms_state.show_save_state_submenu
                        || ms_state.show_load_state_submenu
                        || ms_state.show_region_submenu
                        || ms_state.show_general_settings
                        || ms_state.show_audio_settings
                        || ms_state.show_video_settings
                        || ms_state.show_input_settings
                        || ms_state.show_controller1_settings
                        || ms_state.show_controller2_settings
                        || ms_state.show_expansion_settings
                        || ms_state.show_about
                        || ms_state.show_error
                        || ms_state.show_dip_switches
                        || ms_state.show_confirm_exit_dialog
                        || ms_state.show_barcode_input;
                    let emu_active = *rom_loaded_clone.borrow() && !paused_clone.load(Ordering::Relaxed);
                    let oeka_active = *expansion_type_clone.borrow() == config::ExpansionType::OekaKidsTablet;
                    drop(ms_state);
                    window.set_cursor_visible(ui_visible || !emu_active || oeka_active);
                }

                buffer.present().expect("Failed to present buffer");
            }
            _ => {}
        }
    });
}

