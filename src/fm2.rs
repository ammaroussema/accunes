// fceux fm2 movie file support!!!
use crate::movie::{DevicePort, FrameInput, MovieFile, NES_PAD_BUTTONS};
use crate::region::Region;

const FM2_BUTTONS_PER_PAD: usize = 8;

fn crc32_decimal_to_hash(value: &str) -> String {
    let trimmed = value.trim();
    let n: u32 = trimmed.parse().unwrap_or(0);
    format!("crc32:{:08X}", n)
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

// FCEUX native gamepad columns are RLDUTSBA (Right, Left, Down, Up, sTart,
// Select, B, A); our NES_PAD order is R L D U S T B A where index 4 is Select
// and index 5 is Start. Remap FCEUX column -> our button index.
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

fn parse_native_fields(inner: &str, n_pads: usize, line_no: usize) -> Result<Vec<bool>, String> {
    let expected = n_pads * FM2_BUTTONS_PER_PAD;
    let mut fields = inner.split('|');
    let _commands = fields.next().unwrap_or("");
    let mut bits = Vec::new();
    let mut fields_consumed = 0;
    for field in fields {
        if bits.len() >= expected {
            break;
        }
        bits.extend(parse_native_button_field(field));
        fields_consumed += 1;
    }
    if bits.len() != expected {
        return Err(format!(
            "line {}: expected {} native FM2 button fields, found {}",
            line_no, n_pads, fields_consumed
        ));
    }
    Ok(bits)
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
    out.push_str("fourscore 0\n");
    let pad_indices: Vec<usize> = file
        .devices
        .iter()
        .enumerate()
        .filter(|(_, d)| d.kind == "NES_PAD")
        .map(|(i, _)| i)
        .collect();
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
    out.push_str("startsFromSavestate 0\n");
    out.push_str("savestateBinaryInput 0\n");
    for fr in &file.frames {
        out.push('|');
        for (k, &di) in pad_indices.iter().enumerate() {
            if k > 0 {
                out.push(',');
            }
            let port = fr.ports.get(di).cloned().unwrap_or_default();
            for b in 0..FM2_BUTTONS_PER_PAD {
                if b > 0 {
                    out.push(',');
                }
                let on = port.get(b).copied().unwrap_or(false);
                out.push(if on { '1' } else { '0' });
            }
        }
        out.push('|');
        out.push('\n');
    }
    out
}

pub(crate) fn import_from_fm2(text: &str) -> Result<MovieFile, String> {
    let mut f = MovieFile::empty();
    f.emulator = "FM2".to_string();
    // FM2 movies are recorded from power-on unless a savestate is embedded
    // (header key startsFromSavestate 1), which is handled below.
    f.power_on = true;
    f.settings.push(("initial-ram".to_string(), "fceux".to_string()));
    let mut pad_ports: Vec<usize> = Vec::new();
    let mut seen_frames_header = false;
    let mut rom_name_seen = false;
    let mut frames: Vec<Vec<bool>> = Vec::new();
    let mut subtitle_mode = false;
    let mut comment_set = false;
    for (idx, raw) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('|') || line.starts_with('$') || line.starts_with(':') {
            let inner = line.trim_start_matches('|');
            let inner = inner.trim_end_matches('|').trim();
            if inner.is_empty() {
                continue;
            }
            if inner.starts_with("//") {
                continue;
            }
            if pad_ports.is_empty() {
                return Err(format!("line {}: no controllers declared before the first input frame", line_no));
            }
let bits: Vec<bool> = if inner.contains('|') {
                parse_native_fields(inner, pad_ports.len(), line_no)?
            } else if inner.contains(',') {
                let cols: Vec<&str> = inner.split(',').map(str::trim).collect();
                let expected = pad_ports.len() * FM2_BUTTONS_PER_PAD;
                if cols.len() != expected {
                    return Err(format!(
                        "line {}: expected {} frame columns, found {}",
                        line_no, expected, cols.len()
                    ));
                }
                cols.iter().map(|c| *c == "1").collect()
            } else {
                let chars: Vec<char> = inner.chars().collect();
                if chars.iter().all(|c| *c == '0' || *c == '1')
                    && chars.len() == pad_ports.len() * FM2_BUTTONS_PER_PAD
                {
                    chars.iter().map(|c| *c == '1').collect()
                } else {
                    return Err(format!(
                        "line {}: FM2 binary frames are not supported by this import — convert the movie to text frames first",
                        line_no
                    ));
                }
            };
            frames.push(bits);
            seen_frames_header = true;
            subtitle_mode = false;
            continue;
        }
        let sp = line.find(' ');
        let (key, value) = match sp {
            Some(p) => (&line[..p], line[p + 1..].trim_start()),
            None => (line, ""),
        };
        match key {
            "version" => {
                let v: u32 = value
                    .trim()
                    .parse()
                    .map_err(|_| format!("line {}: invalid FM2 version", line_no))?;
                if v < 2 || v > 3 {
                    return Err(format!("line {}: unsupported FM2 version {}", line_no, v));
                }
            }
            "emuVersion" => f.emulator = format!("FM2 ({})", value),
            "rerecordCount" => {
                f.rerecords = value
                    .trim()
                    .parse()
                    .map_err(|_| format!("line {}: invalid rerecordCount", line_no))?;
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
            "romChecksum" => f.rom_hash = crc32_decimal_to_hash(value),
            "port0" | "port1" | "port2" | "port3" => {
                let port_idx = key[4..].parse::<usize>().unwrap_or(0);
                let kind = value.trim().to_string();
                if kind == "gamepad" || kind == "1" {
                    if !pad_ports.contains(&port_idx) {
                        pad_ports.push(port_idx);
                    }
                } else if kind != "0" && !kind.is_empty() {
                    return Err(format!("line {}: unsupported FM2 port {} device '{}'", line_no, port_idx, kind));
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
            "savestateBinaryInput" | "syncSettings" | "unused" | "guid" | "NewPPU" | "FDS"
            | "microphone" | "binary" | "length" | "savestate" | "board" | "tag" => {
                subtitle_mode = false;
            }
            "fourscore" => {
                if value.trim() == "1" {
                    pad_ports = vec![0, 1, 2, 3];
                }
                subtitle_mode = false;
            }
            _ => {
                if !seen_frames_header {
                    return Err(format!("line {}: unknown FM2 key '{}'", line_no, key));
                }
            }
        }
    }
    if pad_ports.is_empty() && frames.is_empty() {
        return Err("FM2 file contains no input frames or controllers".to_string());
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn two_pad_file() -> MovieFile {
        let mut f = MovieFile::with_standard_pads();
        f.devices.truncate(2);
        f.rom_title = "Super Mario Bros (U) [!].nes".to_string();
        f.rom_hash = "crc32:1A2B3C4D".to_string();
        f.region = Region::Ntsc;
        f.rerecords = 7;
        f.author = "tester".to_string();
        f.comment = "a b c".to_string();
        for i in 0..3 {
            let mut port = vec![false; 8];
            if i == 0 {
                port[0] = true;
            }
            if i == 1 {
                port[1] = true;
            }
            f.frames.push(FrameInput {
                ports: vec![port.clone(), vec![false; 8]],
            });
        }
        f
    }

    #[test]
    fn export_produces_fm2_header_and_frames() {
        let file = two_pad_file();
        let out = export_to_fm2(&file);
        assert!(out.starts_with("version 2\n"));
        assert!(out.contains("rerecordCount 7\n"));
        assert!(out.contains("palFlag 0\n"));
        assert!(out.contains("port0 gamepad\n"));
        assert!(out.contains("port1 gamepad\n"));
        assert!(out.contains("port2 0\n"));
        assert!(out.contains("port3 0\n"));
        assert!(out.contains("romChecksum 439041101\n"));
        assert!(out.contains("|1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0|\n"));
        assert!(out.contains("|0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0|\n"));
        assert!(out.contains("|0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0|\n"));
    }

    #[test]
    fn import_parses_fm2_text_frames() {
        let text = "version 2
emuVersion FCEUX 2.2.3
rerecordCount 3
palFlag 0
romFilename test.nes
romName test
romChecksum 0
fourscore 0
port0 gamepad
port1 gamepad
port2 0
port3 0
comment hello
subtitles
author someone
startsFromSavestate 0
savestateBinaryInput 0
|0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0|
|1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0|
";
        let f = import_from_fm2(text).expect("import");
        assert_eq!(f.frames.len(), 2);
        assert_eq!(f.devices.len(), 2);
        assert_eq!(f.rom_title, "test");
        assert_eq!(f.rerecords, 3);
        assert_eq!(f.region, Region::Ntsc);
        assert_eq!(f.frames[1].ports[0][0], true);
        assert_eq!(f.comment, "hello");
    }

    #[test]
    fn import_rejects_binary_frames() {
        let text = "version 3
port0 gamepad
port1 0
port2 0
port3 0
|AAAA|
";
        assert!(import_from_fm2(text).is_err());
    }

    #[test]
    fn import_parses_fceux_native_frames() {
        let text = "version 3
emuVersion 22020
rerecordCount 11351
palFlag 0
romFilename Mario Bros. (World)
romChecksum base64:tRCF2atCZ3Rqntb8YJqY0A==
guid 10E7031A-E40C-7C64-5562-F943B794BFAE
fourscore 0
microphone 0
port0 1
port1 0
port2 0
FDS 0
NewPPU 0
comment author Spikestuff
|0|........|||
|0|R.......|||
|0|....T...|||
|0|.....S..|||
";
        let f = import_from_fm2(text).expect("import");
        assert_eq!(f.frames.len(), 4);
        assert_eq!(f.devices.len(), 1);
        assert_eq!(f.rom_title, "Mario Bros. (World)");
        assert_eq!(f.rerecords, 11351);
        assert_eq!(f.author, "Spikestuff");
        assert_eq!(f.power_on, true);
        assert_eq!(f.frames[1].ports[0][0], true);
        assert_eq!(f.frames[2].ports[0][5], true);
        assert_eq!(f.frames[3].ports[0][4], true);
    }

    #[test]
    fn import_power_on_respects_starts_from_savestate() {
        let text = "version 3
port0 1
port1 0
port2 0
startsFromSavestate 1
|0|........|||
";
        let f = import_from_fm2(text).expect("import");
        assert_eq!(f.power_on, false);
    }

    #[test]
    fn roundtrip_import_export_preserves_input() {
        let file = two_pad_file();
        let out = export_to_fm2(&file);
        let back = import_from_fm2(&out).expect("import roundtrip");
        assert_eq!(back.frames.len(), file.frames.len());
        assert_eq!(back.devices.len(), file.devices.len());
        for (a, b) in file.frames.iter().zip(back.frames.iter()) {
            assert_eq!(a.ports, b.ports);
        }
        assert_eq!(back.rom_title, file.rom_title);
        assert_eq!(back.region, file.region);
    }
}