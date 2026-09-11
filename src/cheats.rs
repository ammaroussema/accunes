// nes cheats like game genie, pro action rocky and custom cheats, ported from mesen!!!

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheatType {
    GameGenie,
    ProActionRocky,
    Custom,
}

impl CheatType {
    pub fn name(&self) -> &'static str {
        match self {
            CheatType::GameGenie => "Game Genie",
            CheatType::ProActionRocky => "Pro Action Rocky",
            CheatType::Custom => "Custom (Address:Value[:Compare])",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            CheatType::GameGenie => "Format: 6 or 8 letters (A P Z L G I T Y E O X U K S V N)",
            CheatType::ProActionRocky => "Format: 8 hexadecimal digits (e.g. 01234567)",
            CheatType::Custom => "Format: AAAA:VV or AAAA:VV:CC (hexadecimal)",
        }
    }

    pub fn placeholder(&self) -> &'static str {
        match self {
            CheatType::GameGenie => "e.g. GPOAOU or PASKPLLE",
            CheatType::ProActionRocky => "e.g. 01234567",
            CheatType::Custom => "e.g. 0082:04 or 8000:A9:4C",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodedCheat {
    pub address: u16,
    pub value: u8,
    pub compare: Option<u8>,
}

impl DecodedCheat {
    pub fn display(&self) -> String {
        match self.compare {
            Some(cmp) => format!("${:04X} = ${:02X} (if ${:02X})", self.address, self.value, cmp),
            None => format!("${:04X} = ${:02X}", self.address, self.value),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheatEntry {
    pub description: String,
    pub enabled: bool,
    pub cheat_type: CheatType,
    pub code: String,
}

impl CheatEntry {
    pub fn new(description: String, cheat_type: CheatType, code: String) -> Self {
        Self {
            description,
            enabled: true,
            cheat_type,
            code,
        }
    }

    pub fn decoded_preview(&self) -> String {
        match parse_codes(&self.code, self.cheat_type) {
            Ok(list) => {
                let parts: Vec<String> = list.iter().map(|c| c.display()).collect();
                parts.join("; ")
            }
            Err(e) => format!("Error: {}", e),
        }
    }
}

pub fn decode_game_genie(code: &str) -> Result<DecodedCheat, String> {
    let clean: String = code.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
    let upper = clean.to_ascii_uppercase();
    if upper.len() != 6 && upper.len() != 8 {
        return Err("Game Genie code must be 6 or 8 letters".to_string());
    }

    const GG_LETTERS: &[u8] = b"APZLGITYEOXUKSVN";
    let mut raw: u32 = 0;
    for (i, b) in upper.bytes().enumerate() {
        let pos = GG_LETTERS.iter().position(|&x| x == b)
            .ok_or_else(|| format!("Invalid Game Genie letter '{}' (valid: A P Z L G I T Y E O X U K S V N)", b as char))?;
        raw |= (pos as u32) << (i * 4);
    }

    let decode_value = |raw_val: u32, bit_indexes: &[usize]| -> u32 {
        let mut result = 0;
        for &idx in bit_indexes {
            result <<= 1;
            result |= (raw_val >> idx) & 1;
        }
        result
    };

    let address_bits = [14, 13, 12, 19, 22, 21, 20, 7, 10, 9, 8, 15, 18, 17, 16];
    let mut value_bits = [3, 6, 5, 4, 23, 2, 1, 0];

    let compare = if upper.len() == 8 {
        value_bits[4] = 31;
        let compare_bits = [27, 30, 29, 28, 23, 26, 25, 24];
        Some(decode_value(raw, &compare_bits) as u8)
    } else {
        None
    };

    let address = (decode_value(raw, &address_bits) + 0x8000) as u16;
    let value = decode_value(raw, &value_bits) as u8;

    Ok(DecodedCheat { address, value, compare })
}

pub fn decode_pro_action_rocky(code: &str) -> Result<DecodedCheat, String> {
    let clean: String = code.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
    if clean.len() != 8 {
        return Err("Pro Action Rocky code must be 8 hexadecimal characters".to_string());
    }
    let mut par_code = u32::from_str_radix(&clean, 16)
        .map_err(|_| "Invalid hexadecimal character in Pro Action Rocky code".to_string())?;

    let shift_values = [
        3, 13, 14, 1, 6, 9, 5, 0, 12, 7, 2, 8, 10, 11, 4, 
        19, 21, 23, 22, 20, 17, 16, 18,               
        29, 31, 24, 26, 25, 30, 27, 28                   
    ];

    let mut key: u32 = 0x7E5E_E93A;
    let xor_value: u32 = 0x5C18_4B91;

    par_code >>= 1;

    let mut result: u32 = 0;
    for i in (0..=30).rev() {
        if (((key ^ par_code) >> 30) & 1) != 0 {
            result |= 1u32 << shift_values[i];
            key ^= xor_value;
        }
        par_code = par_code.wrapping_shl(1);
        key = key.wrapping_shl(1);
    }

    let address = ((result & 0x7FFF) + 0x8000) as u16;
    let value = ((result >> 24) & 0xFF) as u8;
    let compare = Some(((result >> 16) & 0xFF) as u8);

    Ok(DecodedCheat { address, value, compare })
}

pub fn decode_custom(code: &str) -> Result<DecodedCheat, String> {
    let clean: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    let parts: Vec<&str> = clean.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return Err("Custom code format: AAAA:VV or AAAA:VV:CC".to_string());
    }
    let address = u16::from_str_radix(parts[0], 16)
        .map_err(|_| format!("Invalid hex address '{}'", parts[0]))?;
    let value = u8::from_str_radix(parts[1], 16)
        .map_err(|_| format!("Invalid hex value '{}'", parts[1]))?;
    let compare = if parts.len() == 3 {
        Some(u8::from_str_radix(parts[2], 16)
            .map_err(|_| format!("Invalid hex compare '{}'", parts[2]))?)
    } else {
        None
    };
    Ok(DecodedCheat { address, value, compare })
}

pub fn detect_cheat_type(code: &str) -> CheatType {
    let clean: String = code.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
    if clean.contains(':') {
        CheatType::Custom
    } else if clean.len() == 8 && clean.chars().all(|c| c.is_ascii_hexdigit()) && !clean.chars().all(|c| "APZLGITYEOXUKSVNapzlgityeoxuksvn".contains(c)) {
        CheatType::ProActionRocky
    } else {
        CheatType::GameGenie
    }
}

pub fn parse_codes(codes_str: &str, cheat_type: CheatType) -> Result<Vec<DecodedCheat>, String> {
    let mut list = Vec::new();
    let raw_items = codes_str.split(|c| c == '\n' || c == '\r' || c == ';' || c == ',');
    for item in raw_items {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        let decoded = match cheat_type {
            CheatType::GameGenie => decode_game_genie(trimmed)?,
            CheatType::ProActionRocky => decode_pro_action_rocky(trimmed)?,
            CheatType::Custom => decode_custom(trimmed)?,
        };
        list.push(decoded);
    }
    if list.is_empty() {
        return Err("No valid code entered".to_string());
    }
    Ok(list)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CheatFileFormat {
    pub disable_all_cheats: bool,
    pub cheats: Vec<CheatEntry>,
}

#[derive(Clone, Debug, Default)]
pub struct CheatManager {
    pub cheats: Vec<CheatEntry>,
    pub disable_all: bool,
    active_by_address: HashMap<u16, Vec<DecodedCheat>>,
    has_active: bool,
    current_rom_path: String,
}

impl CheatManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn rebuild_active(&mut self) {
        self.active_by_address.clear();
        if !self.disable_all {
            for entry in &self.cheats {
                if entry.enabled {
                    if let Ok(decoded_list) = parse_codes(&entry.code, entry.cheat_type) {
                        for decoded in decoded_list {
                            self.active_by_address.entry(decoded.address).or_default().push(decoded);
                        }
                    }
                }
            }
        }
        self.has_active = !self.active_by_address.is_empty();
    }

    #[inline(always)]
    pub fn has_active_cheats(&self) -> bool {
        self.has_active
    }

    #[inline(always)]
    pub fn apply(&self, address: u16, value: u8) -> u8 {
        if !self.has_active {
            return value;
        }
        if let Some(list) = self.active_by_address.get(&address) {
            for cheat in list {
                if cheat.compare.is_none() || cheat.compare == Some(value) {
                    return cheat.value;
                }
            }
        }
        value
    }

    pub fn get_cheat_file_path(rom_path: &str) -> PathBuf {
        let p = Path::new(rom_path);
        let stem = p.file_stem().unwrap_or(p.as_os_str()).to_string_lossy();
        let cheats_dir = crate::config::load_cheats_dir();
        PathBuf::from(cheats_dir).join(format!("{}.json", stem))
    }

    pub fn load_for_rom(&mut self, rom_path: &str) {
        self.current_rom_path = rom_path.to_string();
        self.cheats.clear();
        self.disable_all = false;

        let file_path = Self::get_cheat_file_path(rom_path);
        if file_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                if let Ok(file_data) = serde_json::from_str::<CheatFileFormat>(&content) {
                    self.disable_all = file_data.disable_all_cheats;
                    self.cheats = file_data.cheats;
                }
            }
        }
        self.rebuild_active();
    }

    pub fn save(&self) {
        if self.current_rom_path.is_empty() {
            return;
        }
        let file_path = Self::get_cheat_file_path(&self.current_rom_path);
        if self.cheats.is_empty() && !self.disable_all {
            if file_path.exists() {
                let _ = std::fs::remove_file(&file_path);
            }
            return;
        }

        let file_data = CheatFileFormat {
            disable_all_cheats: self.disable_all,
            cheats: self.cheats.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&file_data) {
            let _ = std::fs::write(&file_path, json);
        }
    }

    pub fn add_cheat(&mut self, cheat: CheatEntry) {
        self.cheats.push(cheat);
        self.rebuild_active();
        self.save();
    }

    pub fn edit_cheat(&mut self, index: usize, cheat: CheatEntry) {
        if index < self.cheats.len() {
            self.cheats[index] = cheat;
            self.rebuild_active();
            self.save();
        }
    }

    pub fn remove_cheat(&mut self, index: usize) {
        if index < self.cheats.len() {
            self.cheats.remove(index);
            self.rebuild_active();
            self.save();
        }
    }

    pub fn toggle_cheat(&mut self, index: usize) {
        if index < self.cheats.len() {
            self.cheats[index].enabled = !self.cheats[index].enabled;
            self.rebuild_active();
            self.save();
        }
    }

    pub fn set_disable_all(&mut self, disabled: bool) {
        self.disable_all = disabled;
        self.rebuild_active();
        self.save();
    }

    pub fn clear_all(&mut self) {
        self.cheats.clear();
        self.rebuild_active();
        self.save();
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DbCheatItem {
    pub desc: String,
    pub code: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DbGameEntry {
    pub name: String,
    #[serde(default)]
    pub sha1: String,
    pub cheats: Vec<DbCheatItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CheatDbRoot {
    pub games: Vec<DbGameEntry>,
}

static CHEAT_DB: OnceLock<Vec<DbGameEntry>> = OnceLock::new();

pub fn get_cheat_database() -> &'static [DbGameEntry] {
    CHEAT_DB.get_or_init(|| {
        const DB_RAW: &str = include_str!("../data/cheats.json");
        let raw = DB_RAW.strip_prefix('\u{feff}').unwrap_or(DB_RAW);
        let root: Result<CheatDbRoot, _> = serde_json::from_str(raw);
        match root {
            Ok(r) => r.games,
            Err(e) => {
                eprintln!("Failed to parse cheats.json: {}", e);
                Vec::new()
            }
        }
    })
}

pub fn find_game_in_db(rom_name: &str) -> Option<&'static DbGameEntry> {
    let db = get_cheat_database();
    if db.is_empty() {
        return None;
    }

    let clean_target = clean_game_name(rom_name);
    if clean_target.is_empty() {
        return None;
    }

    for game in db {
        if clean_game_name(&game.name) == clean_target {
            return Some(game);
        }
    }

    for game in db {
        let clean_db = clean_game_name(&game.name);
        if clean_db.contains(&clean_target) || clean_target.contains(&clean_db) {
            return Some(game);
        }
    }

    None
}

pub fn search_games_in_db(query: &str) -> Vec<&'static DbGameEntry> {
    let db = get_cheat_database();
    let q = query.to_ascii_lowercase();
    let mut matches = Vec::new();
    for game in db {
        if query.is_empty() || game.name.to_ascii_lowercase().contains(&q) {
            matches.push(game);
        }
    }
    matches
}

fn clean_game_name(name: &str) -> String {
    let p = Path::new(name);
    let stem = p.file_stem().unwrap_or(p.as_os_str()).to_string_lossy();
    let mut s = String::new();
    for c in stem.chars() {
        if c == '(' || c == '[' {
            break;
        }
        if c.is_ascii_alphanumeric() {
            s.push(c.to_ascii_lowercase());
        }
    }
    s
}

