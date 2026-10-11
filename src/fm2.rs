use crate::movie::{DevicePort, FrameInput, MovieFile, NES_PAD_BUTTONS};
use crate::region::Region;

const FM2_BUTTONS_PER_PAD: usize = 8;

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut table = [0xFFu8; 256];
    for (i, &c) in CHARS.iter().enumerate() {
        table[c as usize] = i as u8;
    }
    let clean: Vec<u8> = input.bytes().filter(|&b| b != b'=' && !b.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity((clean.len() * 3) / 4);
    for chunk in clean.chunks(4) {
        let mut val = 0u32;
        for &b in chunk {
            let v = table[b as usize];
            if v == 0xFF {
                return None;
            }
            val = (val << 6) | (v as u32);
        }
        match chunk.len() {
            4 => {
                out.push((val >> 16) as u8);
                out.push((val >> 8) as u8);
                out.push(val as u8);
            }
            3 => {
                out.push((val >> 10) as u8);
                out.push((val >> 2) as u8);
            }
            2 => {
                out.push((val >> 4) as u8);
            }
            _ => return None,
        }
    }
    Some(out)
}

fn parse_fm2_checksum(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed == "0"
        || trimmed.eq_ignore_ascii_case("unknown")
        || trimmed.eq_ignore_ascii_case("none")
        || trimmed.chars().all(|c| c == '0')
    {
        return String::new();
    }

    if let Some(rest) = trimmed.strip_prefix("base64:") {
        if let Some(bytes) = base64_decode(rest) {
            if bytes.iter().all(|&b| b == 0) {
                return String::new();
            }
            if bytes.len() == 16 {
                let mut hex_str = String::with_capacity(32);
                for b in bytes {
                    hex_str.push_str(&format!("{:02x}", b));
                }
                return format!("md5:{}", hex_str);
            } else if bytes.len() == 4 {
                let n = u32::from_le_bytes(bytes.try_into().unwrap());
                if n == 0 {
                    return String::new();
                }
                return format!("crc32:{:08X}", n);
            }
        }
    }

    let hex_candidate = if let Some(rest) = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")) {
        rest.trim()
    } else {
        trimmed
    };

    if hex_candidate.len() == 32 && hex_candidate.chars().all(|c| c.is_ascii_hexdigit()) {
        if hex_candidate.chars().all(|c| c == '0') {
            return String::new();
        }
        return format!("md5:{}", hex_candidate.to_ascii_lowercase());
    }

    if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
        if let Ok(n) = u32::from_str_radix(hex_candidate, 16) {
            if n == 0 {
                return String::new();
            }
            return format!("crc32:{:08X}", n);
        }
    }

    if let Ok(n) = trimmed.parse::<i64>() {
        let u = n as u32;
        if u == 0 {
            return String::new();
        }
        return format!("crc32:{:08X}", u);
    }

    if hex_candidate.len() == 8 && hex_candidate.chars().all(|c| c.is_ascii_hexdigit()) {
        if let Ok(n) = u32::from_str_radix(hex_candidate, 16) {
            if n == 0 {
                return String::new();
            }
            return format!("crc32:{:08X}", n);
        }
    }

    String::new()
}

fn crc32_hash_to_decimal(hash: &str) -> u32 {
    match hash.strip_prefix("crc32:") {
        Some(h) => u32::from_str_radix(h.trim(), 16).unwrap_or(0),
        None => 0,
    }
}

fn sanitize_single_line(text: &str) -> String {
    text.replace('\n', " ").replace('\r', " ")
}

const FM2_COL_TO_PAD: [usize; 8] = [0, 1, 2, 3, 5, 4, 6, 7];

fn parse_native_button_field(field: &str) -> Vec<bool> {
    let chars: Vec<char> = field.chars().collect();
    let mut bits = vec![false; FM2_BUTTONS_PER_PAD];
    for col in 0..FM2_BUTTONS_PER_PAD {
        let c = chars.get(col).copied().unwrap_or('.');
        if c != '.' && c != ' ' {
            bits[FM2_COL_TO_PAD[col]] = true;
        }
    }
    bits
}

fn parse_fm2_cmd_byte(cmd_val: u32, frame: u64, events: &mut Vec<crate::movie::AcmEvent>) {
    if cmd_val == 0 {
        return;
    }
    if cmd_val & 1 != 0 {
        events.push(crate::movie::AcmEvent { frame, kind: crate::movie::AcmEventKind::SoftReset });
    }
    if cmd_val & 2 != 0 {
        events.push(crate::movie::AcmEvent { frame, kind: crate::movie::AcmEventKind::Power });
    }
    if cmd_val & 4 != 0 {
        events.push(crate::movie::AcmEvent { frame, kind: crate::movie::AcmEventKind::FdsInsert });
    }
    if cmd_val & 8 != 0 {
        events.push(crate::movie::AcmEvent { frame, kind: crate::movie::AcmEventKind::FdsSwapSide });
    }
    if cmd_val & 16 != 0 {
        events.push(crate::movie::AcmEvent { frame, kind: crate::movie::AcmEventKind::VsInsertCoin1 });
    }
    if cmd_val & 32 != 0 {
        events.push(crate::movie::AcmEvent { frame, kind: crate::movie::AcmEventKind::VsInsertCoin2 });
    }
    if cmd_val & 64 != 0 {
        events.push(crate::movie::AcmEvent { frame, kind: crate::movie::AcmEventKind::VsService });
    }
}

fn parse_fm2_commands(cmd_str: &str, frame: u64, events: &mut Vec<crate::movie::AcmEvent>) {
    let s = cmd_str.trim();
    if s.is_empty() || s == "0" {
        return;
    }
    if let Ok(cmd_val) = s.parse::<u32>() {
        parse_fm2_cmd_byte(cmd_val, frame, events);
    } else {
        if s.contains('1') || s.contains('R') || s.contains('r') {
            events.push(crate::movie::AcmEvent { frame, kind: crate::movie::AcmEventKind::SoftReset });
        }
        if s.contains('2') || s.contains('P') || s.contains('p') {
            events.push(crate::movie::AcmEvent { frame, kind: crate::movie::AcmEventKind::Power });
        }
        if s.contains('4') || s.contains('I') || s.contains('i') {
            events.push(crate::movie::AcmEvent { frame, kind: crate::movie::AcmEventKind::FdsInsert });
        }
        if s.contains('8') || s.contains('S') || s.contains('s') {
            events.push(crate::movie::AcmEvent { frame, kind: crate::movie::AcmEventKind::FdsSwapSide });
        }
    }
}

pub(crate) fn export_to_fm2(file: &MovieFile) -> String {
    let mut out = String::new();
    out.push_str("version 2\n");
    out.push_str(&format!("emuVersion {}\n", sanitize_single_line(&file.emulator)));
    out.push_str(&format!("rerecordCount {}\n", file.rerecords));
    let pal = match file.region {
        Region::Pal | Region::Dendy => 1,
        _ => 0,
    };
    out.push_str(&format!("palFlag {}\n", pal));
    out.push_str(&format!("romFilename {}\n", sanitize_single_line(&file.rom_title)));
    out.push_str(&format!("romName {}\n", sanitize_single_line(&file.rom_title)));
    out.push_str(&format!("romChecksum {}\n", crc32_hash_to_decimal(&file.rom_hash)));
    out.push_str(&format!("guid {}\n", sanitize_single_line(&file.guid)));
    let pad_indices: Vec<usize> = file
        .devices
        .iter()
        .enumerate()
        .filter(|(_, d)| d.kind == "NES_PAD")
        .map(|(i, _)| i)
        .collect();
    let fourscore = if pad_indices.len() > 2 { 1 } else { 0 };
    out.push_str(&format!("fourscore {}\n", fourscore));
    for i in 0..4 {
        let slot = format!("port{}", i + 1);
        let present = file.devices.iter().any(|d| d.slot == slot && d.kind == "NES_PAD");
        out.push_str(&format!("port{} {}\n", i, if present { "gamepad" } else { "0" }));
    }
    out.push_str(&format!("comment {}\n", sanitize_single_line(&file.comment)));
    out.push_str("subtitles\n");
    for s in &file.subtitles {
        out.push_str(&format!(
            "comment //{}|{}|{}|16777215\n",
            sanitize_single_line(&s.text),
            s.start,
            s.end.saturating_sub(s.start)
        ));
    }
    out.push_str(&format!("author {}\n", sanitize_single_line(&file.author)));
    out.push_str(&format!("startsFromSavestate {}\n", if file.power_on { 0 } else { 1 }));
    out.push_str("savestateBinaryInput 0\n");

    let num_ports = if fourscore == 1 { 4 } else { 2 };
    for (i, fr) in file.frames.iter().enumerate() {
        let mut cmd_val = 0u32;
        for ev in file.events.iter().filter(|e| e.frame == i as u64) {
            match ev.kind {
                crate::movie::AcmEventKind::SoftReset => cmd_val |= 1,
                crate::movie::AcmEventKind::HardReset | crate::movie::AcmEventKind::Power => cmd_val |= 2,
                crate::movie::AcmEventKind::FdsInsert => cmd_val |= 4,
                crate::movie::AcmEventKind::FdsSwapSide => cmd_val |= 8,
                crate::movie::AcmEventKind::FdsEject => cmd_val |= 4,
                crate::movie::AcmEventKind::VsInsertCoin1 => cmd_val |= 16,
                crate::movie::AcmEventKind::VsInsertCoin2 => cmd_val |= 32,
                crate::movie::AcmEventKind::VsService => cmd_val |= 64,
            }
        }
        out.push('|');
        out.push_str(&cmd_val.to_string());
        for p in 0..num_ports {
            out.push('|');
            let slot = format!("port{}", p + 1);
            if let Some(dev_idx) = file.devices.iter().position(|d| d.slot == slot && d.kind == "NES_PAD") {
                let port = fr.ports.get(dev_idx).cloned().unwrap_or_default();
                out.push(if port.get(0).copied().unwrap_or(false) { 'R' } else { '.' });
                out.push(if port.get(1).copied().unwrap_or(false) { 'L' } else { '.' });
                out.push(if port.get(2).copied().unwrap_or(false) { 'D' } else { '.' });
                out.push(if port.get(3).copied().unwrap_or(false) { 'U' } else { '.' });
                out.push(if port.get(5).copied().unwrap_or(false) { 'T' } else { '.' });
                out.push(if port.get(4).copied().unwrap_or(false) { 'S' } else { '.' });
                out.push(if port.get(6).copied().unwrap_or(false) { 'B' } else { '.' });
                out.push(if port.get(7).copied().unwrap_or(false) { 'A' } else { '.' });
            }
        }
        out.push('|');
        out.push('\n');
    }
    out
}

pub(crate) fn export_to_fm3(file: &MovieFile) -> Vec<u8> {
    let mut text = String::new();
    text.push_str("version 3\n");
    text.push_str(&format!("emuVersion {}\n", sanitize_single_line(&file.emulator)));
    text.push_str(&format!("rerecordCount {}\n", file.rerecords));
    let pal = match file.region {
        Region::Pal | Region::Dendy => 1,
        _ => 0,
    };
    text.push_str(&format!("palFlag {}\n", pal));
    text.push_str(&format!("romFilename {}\n", sanitize_single_line(&file.rom_title)));
    text.push_str(&format!("romChecksum {}\n", crc32_hash_to_decimal(&file.rom_hash)));
    text.push_str(&format!("guid {}\n", sanitize_single_line(&file.guid)));
    let pad_indices: Vec<usize> = file
        .devices
        .iter()
        .enumerate()
        .filter(|(_, d)| d.kind == "NES_PAD")
        .map(|(i, _)| i)
        .collect();
    let fourscore = if pad_indices.len() > 2 { 1 } else { 0 };
    text.push_str(&format!("fourscore {}\n", fourscore));
    text.push_str("microphone 0\n");
    for i in 0..4 {
        let slot = format!("port{}", i + 1);
        let present = file.devices.iter().any(|d| d.slot == slot && d.kind == "NES_PAD");
        text.push_str(&format!("port{} {}\n", i, if present { "1" } else { "0" }));
    }
    text.push_str("FDS 0\nNewPPU 0\nRAMInitOption 0\nRAMInitSeed 0\n");
    if !file.comment.is_empty() {
        text.push_str(&format!("comment {}\n", sanitize_single_line(&file.comment)));
    }
    if !file.author.is_empty() {
        text.push_str(&format!("comment author {}\n", sanitize_single_line(&file.author)));
    }
    for s in &file.subtitles {
        text.push_str(&format!(
            "subtitle //{}|{}|{}\n",
            sanitize_single_line(&s.text),
            s.start,
            s.end.saturating_sub(s.start)
        ));
    }
    text.push_str(&format!("length {}\n", file.frames.len()));

    let num_ports = if fourscore == 1 { 4 } else { 2 };
    for (i, fr) in file.frames.iter().enumerate() {
        let mut cmd_val = 0u32;
        for ev in file.events.iter().filter(|e| e.frame == i as u64) {
            match ev.kind {
                crate::movie::AcmEventKind::SoftReset => cmd_val |= 1,
                crate::movie::AcmEventKind::HardReset | crate::movie::AcmEventKind::Power => cmd_val |= 2,
                crate::movie::AcmEventKind::FdsInsert => cmd_val |= 4,
                crate::movie::AcmEventKind::FdsSwapSide => cmd_val |= 8,
                crate::movie::AcmEventKind::FdsEject => cmd_val |= 16,
                crate::movie::AcmEventKind::VsInsertCoin1 => cmd_val |= 32,
                crate::movie::AcmEventKind::VsInsertCoin2 => cmd_val |= 64,
                crate::movie::AcmEventKind::VsService => cmd_val |= 128,
            }
        }
        text.push('|');
        text.push_str(&cmd_val.to_string());
        for p in 0..num_ports {
            text.push('|');
            let slot = format!("port{}", p + 1);
            if let Some(dev_idx) = file.devices.iter().position(|d| d.slot == slot && d.kind == "NES_PAD") {
                let port = fr.ports.get(dev_idx).cloned().unwrap_or_default();
                text.push(if port.get(0).copied().unwrap_or(false) { 'R' } else { '.' });
                text.push(if port.get(1).copied().unwrap_or(false) { 'L' } else { '.' });
                text.push(if port.get(2).copied().unwrap_or(false) { 'D' } else { '.' });
                text.push(if port.get(3).copied().unwrap_or(false) { 'U' } else { '.' });
                text.push(if port.get(5).copied().unwrap_or(false) { 'T' } else { '.' });
                text.push(if port.get(4).copied().unwrap_or(false) { 'S' } else { '.' });
                text.push(if port.get(6).copied().unwrap_or(false) { 'B' } else { '.' });
                text.push(if port.get(7).copied().unwrap_or(false) { 'A' } else { '.' });
            }
        }
        text.push('|');
        text.push('\n');
    }

    let mut out = text.into_bytes();
    let has_markers = !file.chapters.is_empty();
    let saved_stuff_map: u32 = if has_markers { 1 } else { 0 };
    out.extend_from_slice(&3u32.to_le_bytes());
    out.extend_from_slice(&saved_stuff_map.to_le_bytes());
    out.extend_from_slice(&6u32.to_le_bytes());

    let pointer_table_pos = out.len();
    out.extend_from_slice(&[0u8; 24]);

    let markers_offset = out.len() as u32;
    if has_markers {
        out.extend_from_slice(b"MARKERS\0");
        let mut markers_array = vec![0i32; file.frames.len()];
        let mut notes = vec!["Power on".to_string()];
        for chap in &file.chapters {
            if (chap.frame as usize) < markers_array.len() {
                notes.push(chap.name.clone());
                markers_array[chap.frame as usize] = (notes.len() - 1) as i32;
            }
        }
        out.extend_from_slice(&(markers_array.len() as u32).to_le_bytes());
        let mut raw_arr = Vec::with_capacity(markers_array.len() * 4);
        for v in markers_array {
            raw_arr.extend_from_slice(&v.to_le_bytes());
        }
        let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&raw_arr, 6);
        out.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        out.extend_from_slice(&compressed);
        out.extend_from_slice(&(notes.len() as u32).to_le_bytes());
        for note in &notes {
            let nb = note.as_bytes();
            let len = (nb.len() + 1) as u32;
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(nb);
            out.push(0);
        }
    } else {
        out.extend_from_slice(b"MARKERX\0");
    }

    let bookmarks_offset = out.len() as u32;
    out.extend_from_slice(b"BOOKMARKX\0");

    let greenzone_offset = out.len() as u32;
    out.extend_from_slice(b"GREENZONX\0");

    let history_offset = out.len() as u32;
    out.extend_from_slice(b"HISTORX\0");

    let pianoroll_offset = out.len() as u32;
    out.extend_from_slice(b"PIANO_ROLX\0");

    let selection_offset = out.len() as u32;
    out.extend_from_slice(b"SELECTIOX\0");

    out[pointer_table_pos..pointer_table_pos + 4].copy_from_slice(&markers_offset.to_le_bytes());
    out[pointer_table_pos + 4..pointer_table_pos + 8].copy_from_slice(&bookmarks_offset.to_le_bytes());
    out[pointer_table_pos + 8..pointer_table_pos + 12].copy_from_slice(&greenzone_offset.to_le_bytes());
    out[pointer_table_pos + 12..pointer_table_pos + 16].copy_from_slice(&history_offset.to_le_bytes());
    out[pointer_table_pos + 16..pointer_table_pos + 20].copy_from_slice(&pianoroll_offset.to_le_bytes());
    out[pointer_table_pos + 20..pointer_table_pos + 24].copy_from_slice(&selection_offset.to_le_bytes());

    out
}

#[allow(dead_code)]
pub(crate) fn import_from_fm2(text: &str) -> Result<MovieFile, String> {
    import_from_fm(text.as_bytes())
}

pub(crate) fn import_from_fm(bytes: &[u8]) -> Result<MovieFile, String> {
    let mut f = MovieFile::empty();
    f.emulator = "FM2".to_string();
    f.power_on = true;
    f.settings.push(("initial-ram".to_string(), "fceux".to_string()));
    let mut pad_ports: Vec<usize> = Vec::new();
    let mut rom_name_seen = false;
    let mut subtitle_mode = false;
    let mut comment_set = false;
    let mut binary_flag = false;
    let mut load_frame_count: Option<usize> = None;
    let mut fourscore = false;

    let mut idx = 0;
    while idx < bytes.len() {
        if binary_flag && bytes[idx] == b'|' {
            idx += 1;
            break;
        }
        let line_end = bytes[idx..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|p| idx + p)
            .unwrap_or(bytes.len());
        let line_bytes = &bytes[idx..line_end];
        let trimmed_bytes = {
            let mut s = 0;
            while s < line_bytes.len() && (line_bytes[s] == b' ' || line_bytes[s] == b'\t' || line_bytes[s] == b'\r') {
                s += 1;
            }
            let mut e = line_bytes.len();
            while e > s && (line_bytes[e - 1] == b' ' || line_bytes[e - 1] == b'\t' || line_bytes[e - 1] == b'\r') {
                e -= 1;
            }
            &line_bytes[s..e]
        };
        if trimmed_bytes.is_empty() {
            idx = if line_end < bytes.len() { line_end + 1 } else { line_end };
            continue;
        }
        if !binary_flag && (trimmed_bytes.starts_with(b"|") || trimmed_bytes.starts_with(b"$") || trimmed_bytes.starts_with(b":")) {
            break;
        }
        idx = if line_end < bytes.len() { line_end + 1 } else { line_end };
        let line = String::from_utf8_lossy(trimmed_bytes);
        let sp = line.find(' ');
        let (key, value) = match sp {
            Some(p) => (&line[..p], line[p + 1..].trim_start()),
            None => (line.as_ref(), ""),
        };
        match key {
            "version" => {
                let v: u32 = value.trim().parse().map_err(|_| "invalid FM version".to_string())?;
                if v < 2 || v > 3 {
                    return Err(format!("unsupported FM version {}", v));
                }
            }
            "emuVersion" => f.emulator = format!("FM2 ({})", value),
            "rerecordCount" => {
                f.rerecords = value.trim().parse().map_err(|_| "invalid rerecordCount".to_string())?;
            }
            "palFlag" => {
                f.region = if value.trim() == "1" { Region::Pal } else { Region::Ntsc };
            }
            "romName" => {
                f.rom_title = value.to_string();
                rom_name_seen = true;
            }
            "romFilename" => {
                if !rom_name_seen && f.rom_title.is_empty() {
                    f.rom_title = value.to_string();
                }
            }
            "romChecksum" => f.rom_hash = parse_fm2_checksum(value),
            "port0" | "port1" | "port2" | "port3" => {
                let port_idx = key[4..].parse::<usize>().unwrap_or(0);
                let kind = value.trim();
                if kind == "gamepad" || kind == "1" {
                    if !pad_ports.contains(&port_idx) {
                        pad_ports.push(port_idx);
                    }
                } else if kind != "0" && !kind.is_empty() {
                    return Err(format!("unsupported port {} device '{}'", port_idx, kind));
                }
            }
            "binary" => {
                if value.trim() == "1" {
                    binary_flag = true;
                }
            }
            "length" => {
                if let Ok(n) = value.trim().parse::<usize>() {
                    load_frame_count = Some(n);
                }
            }
            "comment" => {
                let v = value.trim();
                if subtitle_mode && v.starts_with("//") {
                    let content = &v[2..];
                    let parts: Vec<&str> = content.split('|').collect();
                    let text = parts.first().map(|s| s.to_string()).unwrap_or_default();
                    let start = parts.get(1).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                    let len = parts.get(2).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                    f.subtitles.push(crate::movie::Subtitle {
                        start,
                        end: start.saturating_add(len),
                        text,
                    });
                } else if !comment_set && !subtitle_mode {
                    if let Some(a) = v.strip_prefix("author ") {
                        f.author = a.to_string();
                    }
                    f.comment = v.to_string();
                    comment_set = true;
                }
            }
            "subtitles" => {
                subtitle_mode = true;
            }
            "author" => {
                subtitle_mode = false;
                f.author = value.to_string();
            }
            "startsFromSavestate" => {
                f.power_on = value.trim() != "1";
                subtitle_mode = false;
            }
            "guid" => {
                f.guid = value.trim().to_string();
                subtitle_mode = false;
            }
            "fourscore" => {
                if value.trim() == "1" {
                    fourscore = true;
                    pad_ports = vec![0, 1, 2, 3];
                }
                subtitle_mode = false;
            }
            "savestateBinaryInput" | "syncSettings" | "unused" | "NewPPU" | "FDS"
            | "microphone" | "savestate" | "board" | "tag" | "RAMInitOption" | "RAMInitSeed" => {
                subtitle_mode = false;
            }
            _ => {}
        }
    }

    if fourscore {
        pad_ports = vec![0, 1, 2, 3];
    } else if pad_ports.is_empty() {
        pad_ports = vec![0, 1];
    }

    let mut frames: Vec<Vec<bool>> = Vec::new();
    if binary_flag {
        let record_size = 1 + pad_ports.len();
        let target_frames = match load_frame_count {
            Some(n) => n,
            None => (bytes.len().saturating_sub(idx)) / record_size,
        };
        for frame_idx in 0..target_frames {
            if idx + record_size > bytes.len() {
                break;
            }
            let cmd_byte = bytes[idx];
            idx += 1;
            parse_fm2_cmd_byte(cmd_byte as u32, frame_idx as u64, &mut f.events);
            let mut port_bits = Vec::with_capacity(pad_ports.len() * FM2_BUTTONS_PER_PAD);
            for _ in 0..pad_ports.len() {
                let joy_byte = bytes[idx];
                idx += 1;
                port_bits.push((joy_byte & 0x80) != 0);
                port_bits.push((joy_byte & 0x40) != 0);
                port_bits.push((joy_byte & 0x20) != 0);
                port_bits.push((joy_byte & 0x10) != 0);
                port_bits.push((joy_byte & 0x04) != 0);
                port_bits.push((joy_byte & 0x08) != 0);
                port_bits.push((joy_byte & 0x02) != 0);
                port_bits.push((joy_byte & 0x01) != 0);
            }
            frames.push(port_bits);
        }
    } else {
        while idx < bytes.len() {
            if let Some(target) = load_frame_count {
                if frames.len() >= target {
                    break;
                }
            }
            let line_end = bytes[idx..]
                .iter()
                .position(|&b| b == b'\n')
                .map(|p| idx + p)
                .unwrap_or(bytes.len());
            let line_bytes = &bytes[idx..line_end];
            let trimmed_bytes = {
                let mut s = 0;
                while s < line_bytes.len() && (line_bytes[s] == b' ' || line_bytes[s] == b'\t' || line_bytes[s] == b'\r') {
                    s += 1;
                }
                let mut e = line_bytes.len();
                while e > s && (line_bytes[e - 1] == b' ' || line_bytes[e - 1] == b'\t' || line_bytes[e - 1] == b'\r') {
                    e -= 1;
                }
                &line_bytes[s..e]
            };
            if !trimmed_bytes.starts_with(b"|") && !trimmed_bytes.starts_with(b"$") && !trimmed_bytes.starts_with(b":") {
                if !trimmed_bytes.is_empty() {
                    break;
                }
                idx = if line_end < bytes.len() { line_end + 1 } else { line_end };
                continue;
            }
            idx = if line_end < bytes.len() { line_end + 1 } else { line_end };
            let raw_line = String::from_utf8_lossy(trimmed_bytes);
            let line = raw_line.trim();
            let slice = if line.starts_with('|') { &line[1..] } else { line };
            let slice = if slice.ends_with('|') { &slice[..slice.len() - 1] } else { slice };
            if slice.is_empty() || slice.starts_with("//") {
                continue;
            }
            let bits: Vec<bool> = if slice.contains('|') {
                let fields: Vec<&str> = slice.split('|').collect();
                let cmd_str = fields[0];
                let current_frame = frames.len() as u64;
                parse_fm2_commands(cmd_str, current_frame, &mut f.events);
                let mut bits = Vec::with_capacity(pad_ports.len() * FM2_BUTTONS_PER_PAD);
                for (p_order, &p_idx) in pad_ports.iter().enumerate() {
                    let field = if fields.len() > p_idx + 1 {
                        fields[p_idx + 1]
                    } else if fields.len() > p_order + 1 {
                        fields[p_order + 1]
                    } else {
                        ""
                    };
                    bits.extend(parse_native_button_field(field));
                }
                bits
            } else if slice.contains(',') {
                let cols: Vec<&str> = slice.split(',').map(str::trim).collect();
                cols.iter().map(|c| *c == "1").collect()
            } else {
                let chars: Vec<char> = slice.chars().collect();
                chars.iter().map(|c| *c == '1').collect()
            };
            frames.push(bits);
        }
    }

    while idx < bytes.len() && (bytes[idx] == b'\r' || bytes[idx] == b'\n' || bytes[idx] == b' ' || bytes[idx] == b'\t') {
        idx += 1;
    }

    if idx + 12 <= bytes.len() {
        let project_ver = u32::from_le_bytes(bytes[idx..idx + 4].try_into().unwrap());
        if project_ver >= 1 && project_ver <= 10 {
            f.emulator = if f.emulator.is_empty() {
                "FM3".to_string()
            } else {
                format!("FM3 ({})", f.emulator)
            };
            let num_ptrs = u32::from_le_bytes(bytes[idx + 8..idx + 12].try_into().unwrap()) as usize;
            if num_ptrs >= 1 && num_ptrs <= 32 && idx + 12 + num_ptrs * 4 <= bytes.len() {
                let mut ptrs = Vec::with_capacity(num_ptrs);
                for i in 0..num_ptrs {
                    let off = idx + 12 + i * 4;
                    ptrs.push(u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap()) as usize);
                }
                if let Some(&m_off) = ptrs.get(0) {
                    if m_off > 0 && m_off + 8 <= bytes.len() && &bytes[m_off..m_off + 7] == b"MARKERS" {
                        let mut mo = m_off + 8;
                        if mo + 8 <= bytes.len() {
                            let size = u32::from_le_bytes(bytes[mo..mo + 4].try_into().unwrap()) as usize;
                            let comprlen = u32::from_le_bytes(bytes[mo + 4..mo + 8].try_into().unwrap()) as usize;
                            mo += 8;
                            if mo + comprlen <= bytes.len() {
                                let compr = &bytes[mo..mo + comprlen];
                                mo += comprlen;
                                if let Ok(decomp) = miniz_oxide::inflate::decompress_to_vec_zlib(compr) {
                                    let mut markers_arr = Vec::with_capacity(size);
                                    for chk in decomp.chunks_exact(4) {
                                        markers_arr.push(i32::from_le_bytes(chk.try_into().unwrap()));
                                    }
                                    if mo + 4 <= bytes.len() {
                                        let num_notes = u32::from_le_bytes(bytes[mo..mo + 4].try_into().unwrap()) as usize;
                                        mo += 4;
                                        let mut notes = Vec::with_capacity(num_notes);
                                        for _ in 0..num_notes {
                                            if mo + 4 > bytes.len() {
                                                break;
                                            }
                                            let nlen = u32::from_le_bytes(bytes[mo..mo + 4].try_into().unwrap()) as usize;
                                            mo += 4;
                                            if mo + nlen > bytes.len() {
                                                break;
                                            }
                                            let note_bytes = &bytes[mo..mo + nlen];
                                            mo += nlen;
                                            let text = String::from_utf8_lossy(note_bytes).trim_end_matches('\0').to_string();
                                            notes.push(text);
                                        }
                                        for (f_idx, &n_id) in markers_arr.iter().enumerate() {
                                            if n_id > 0 && (n_id as usize) < notes.len() {
                                                let name = &notes[n_id as usize];
                                                if !name.is_empty() {
                                                    f.chapters.push(crate::movie::Chapter {
                                                        frame: f_idx as u64,
                                                        name: name.clone(),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(&b_off) = ptrs.get(1) {
                    if b_off > 0 && b_off + 10 <= bytes.len() && &bytes[b_off..b_off + 9] == b"BOOKMARKS" {
                        let mut bo = b_off + 10;
                        for _ in 0..10 {
                            if bo >= bytes.len() {
                                break;
                            }
                            let not_empty = bytes[bo] != 0;
                            bo += 1;
                            if not_empty {
                                if bo + 4 <= bytes.len() {
                                    let key_frame = u32::from_le_bytes(bytes[bo..bo + 4].try_into().unwrap()) as u64;
                                    bo += 24;
                                    let desc = if bo < bytes.len() {
                                        let dlen = bytes[bo] as usize;
                                        bo += 1;
                                        if bo + dlen <= bytes.len() {
                                            String::from_utf8_lossy(&bytes[bo..bo + dlen]).trim_end_matches('\0').to_string()
                                        } else {
                                            String::new()
                                        }
                                    } else {
                                        String::new()
                                    };
                                    if !f.chapters.iter().any(|c| c.frame == key_frame) {
                                        let label = if desc.is_empty() {
                                            format!("Bookmark @{}", key_frame)
                                        } else {
                                            format!("Bookmark: {}", desc)
                                        };
                                        f.chapters.push(crate::movie::Chapter { frame: key_frame, name: label });
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    f.chapters.sort_by_key(|c| c.frame);
    f.chapters.dedup_by(|a, b| a.frame == b.frame && a.name == b.name);

    let devices: Vec<DevicePort> = pad_ports
        .iter()
        .map(|&p| DevicePort {
            slot: format!("port{}", p + 1),
            kind: "NES_PAD".to_string(),
            buttons: NES_PAD_BUTTONS.to_vec(),
        })
        .collect();
    f.devices = devices;

    for bits in frames {
        let mut ports = Vec::with_capacity(pad_ports.len());
        for chunk in bits.chunks_exact(FM2_BUTTONS_PER_PAD) {
            ports.push(chunk.to_vec());
        }
        f.frames.push(FrameInput { ports });
    }

    Ok(f)
}