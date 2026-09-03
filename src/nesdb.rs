// input auto-detection database! based on mesen so shoutouts to them!!!

use std::collections::HashMap;

use crate::config::{ControllerType, ExpansionAdapterType, ExpansionType};

const INPUT_FOUR_SCORE: u8 = 2;
const INPUT_FOUR_PLAYER_ADAPTER: u8 = 3;
const INPUT_ZAPPER: u8 = 8;
const INPUT_TWO_ZAPPERS: u8 = 9;
const INPUT_BANDAI_HYPER_SHOT: u8 = 10;
const INPUT_POWER_PAD_A: u8 = 11;
const INPUT_POWER_PAD_B: u8 = 12;
const INPUT_FAMILY_TRAINER_A: u8 = 13;
const INPUT_FAMILY_TRAINER_B: u8 = 14;
const INPUT_ARKANOID_NES: u8 = 15;
const INPUT_ARKANOID_FAMICOM: u8 = 16;
const INPUT_DOUBLE_ARKANOID: u8 = 17;
const INPUT_KONAMI_HYPER_SHOT: u8 = 18;
const INPUT_PACHINKO: u8 = 19;
const INPUT_EXCITING_BOXING: u8 = 20;
const INPUT_JISSEN_MAHJONG: u8 = 21;
const INPUT_PARTY_TAP: u8 = 22;
const INPUT_OEKA_KIDS_TABLET: u8 = 23;
const INPUT_BARCODE_BATTLER: u8 = 24;
const INPUT_TURBO_FILE: u8 = 27;
const INPUT_BATTLE_BOX: u8 = 28;
const INPUT_FAMILY_BASIC_KEYBOARD: u8 = 29;
const INPUT_SUBOR_KEYBOARD: u8 = 32;
const INPUT_SUBOR_KEYBOARD_MOUSE_1: u8 = 33;
const INPUT_SUBOR_KEYBOARD_MOUSE_2: u8 = 34;
const INPUT_SNES_MOUSE: u8 = 41;
const INPUT_SNES_CONTROLLERS: u8 = 43;

pub struct DetectedInput {
    pub port1: ControllerType,
    pub port2: ControllerType,
    pub expansion: ExpansionType,
    pub adapter: ExpansionAdapterType,
}

struct DbEntry {
    input_type: u8,
    is_famicom: bool,
}

fn parse_db() -> HashMap<u32, DbEntry> {
    let mut db = HashMap::new();
    for line in include_str!("../data/controllerdb.txt").lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split(',');
        let crc = match u32::from_str_radix(it.next().unwrap_or(""), 16) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let input_type = match it.next().unwrap_or("").parse::<u8>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let is_famicom = it.next().unwrap_or("") == "1";
        db.insert(crc, DbEntry { input_type, is_famicom });
    }
    db
}

pub fn detect(prg_chr_crc32: u32) -> Option<DetectedInput> {
    static DB: std::sync::OnceLock<HashMap<u32, DbEntry>> = std::sync::OnceLock::new();
    let db = DB.get_or_init(parse_db);
    let entry = db.get(&prg_chr_crc32)?;
    let is_famicom = entry.is_famicom;
    match entry.input_type {
        INPUT_FOUR_SCORE => Some(DetectedInput {
            port1: ControllerType::FourScore,
            port2: ControllerType::Gamepad,
            expansion: ExpansionType::None,
            adapter: ExpansionAdapterType::FourPlayer,
        }),
        INPUT_FOUR_PLAYER_ADAPTER => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::Gamepad,
            expansion: ExpansionType::None,
            adapter: ExpansionAdapterType::TwoPlayer,
        }),
        INPUT_ZAPPER => {
            if is_famicom {
                Some(DetectedInput {
                    port1: ControllerType::Gamepad,
                    port2: ControllerType::Gamepad,
                    expansion: ExpansionType::FamicomZapper,
                    adapter: ExpansionAdapterType::None,
                })
            } else {
                Some(DetectedInput {
                    port1: ControllerType::Gamepad,
                    port2: ControllerType::Zapper,
                    expansion: ExpansionType::None,
                    adapter: ExpansionAdapterType::None,
                })
            }
        }
        INPUT_TWO_ZAPPERS => Some(DetectedInput {
            port1: ControllerType::Zapper,
            port2: ControllerType::Zapper,
            expansion: ExpansionType::None,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_BANDAI_HYPER_SHOT => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::Gamepad,
            expansion: ExpansionType::BandaiHyperShot,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_POWER_PAD_A => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::PowerPadA,
            expansion: ExpansionType::None,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_POWER_PAD_B => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::PowerPadB,
            expansion: ExpansionType::None,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_FAMILY_TRAINER_A => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::Gamepad,
            expansion: ExpansionType::FamilyTrainerA,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_FAMILY_TRAINER_B => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::Gamepad,
            expansion: ExpansionType::FamilyTrainerB,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_ARKANOID_NES => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::Paddle,
            expansion: ExpansionType::None,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_ARKANOID_FAMICOM | INPUT_DOUBLE_ARKANOID => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::Gamepad,
            expansion: ExpansionType::ArkanoidPaddle,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_KONAMI_HYPER_SHOT => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::Gamepad,
            expansion: ExpansionType::KonamiHyperShot,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_PACHINKO => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::Gamepad,
            expansion: ExpansionType::PachinkoController,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_EXCITING_BOXING => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::Gamepad,
            expansion: ExpansionType::ExcitingBoxing,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_JISSEN_MAHJONG => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::Gamepad,
            expansion: ExpansionType::JissenMahjong,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_PARTY_TAP => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::Gamepad,
            expansion: ExpansionType::PartyTap,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_OEKA_KIDS_TABLET => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::Gamepad,
            expansion: ExpansionType::OekaKidsTablet,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_BARCODE_BATTLER => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::Gamepad,
            expansion: ExpansionType::BarcodeBattler,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_TURBO_FILE => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::Gamepad,
            expansion: ExpansionType::TurboFile,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_BATTLE_BOX => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::Gamepad,
            expansion: ExpansionType::BattleBox,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_FAMILY_BASIC_KEYBOARD => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::Gamepad,
            expansion: ExpansionType::FamilyBasicKeyboard,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_SUBOR_KEYBOARD => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::Gamepad,
            expansion: ExpansionType::SuborKeyboard,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_SUBOR_KEYBOARD_MOUSE_1 | INPUT_SUBOR_KEYBOARD_MOUSE_2 => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::SuborMouse,
            expansion: ExpansionType::SuborKeyboard,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_SNES_MOUSE => Some(DetectedInput {
            port1: ControllerType::Gamepad,
            port2: ControllerType::SNESMouse,
            expansion: ExpansionType::None,
            adapter: ExpansionAdapterType::None,
        }),
        INPUT_SNES_CONTROLLERS => Some(DetectedInput {
            port1: ControllerType::SNESPad,
            port2: ControllerType::SNESPad,
            expansion: ExpansionType::None,
            adapter: ExpansionAdapterType::None,
        }),
        _ => None,
    }
}


