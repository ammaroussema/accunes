// accunes .acm/.acmb movie system!!!
#![allow(dead_code)]
use crate::region::Region;

pub(crate) const ACM_MAGIC: &str = "ACMN";
pub(crate) const ACM_BINARY_MAGIC: &str = "ACMB";
pub(crate) const ACM_FORMAT_VERSION: u32 = 1;

pub(crate) const NES_PAD_BUTTONS: [char; 8] = ['R', 'L', 'D', 'U', 'S', 'T', 'B', 'A'];

pub(crate) const NES_PAD_ATOMIC_BITS: [u8; 8] = [0, 1, 2, 3, 5, 4, 6, 7];

pub(crate) fn nes_pad_vec_to_byte(buttons: &[bool]) -> u8 {
    let mut b = 0u8;
    for (i, &bit) in NES_PAD_ATOMIC_BITS.iter().enumerate() {
        if i < buttons.len() && buttons[i] {
            b |= 1 << bit;
        }
    }
    b
}

pub(crate) fn nes_pad_byte_to_vec(byte: u8) -> Vec<bool> {
    NES_PAD_ATOMIC_BITS.iter().map(|&bit| byte & (1 << bit) != 0).collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MovieMode {
    None,
    Recording,
    Playing,
    RecordingFromPlayback,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum AcmEventKind {
    SoftReset,
    HardReset,
    Power,
    FdsInsert,
    FdsEject,
    FdsSwapSide,
    VsInsertCoin1,
    VsInsertCoin2,
    VsService,
}

impl AcmEventKind {
    fn name(self) -> &'static str {
        match self {
            AcmEventKind::SoftReset => "SOFT_RESET",
            AcmEventKind::HardReset => "HARD_RESET",
            AcmEventKind::Power => "POWER",
            AcmEventKind::FdsInsert => "FDS_INSERT",
            AcmEventKind::FdsEject => "FDS_EJECT",
            AcmEventKind::FdsSwapSide => "FDS_SWAP",
            AcmEventKind::VsInsertCoin1 => "VS_COIN1",
            AcmEventKind::VsInsertCoin2 => "VS_COIN2",
            AcmEventKind::VsService => "VS_SERVICE",
        }
    }

    fn from_name(s: &str) -> Option<AcmEventKind> {
        match s {
            "SOFT_RESET" => Some(AcmEventKind::SoftReset),
            "HARD_RESET" => Some(AcmEventKind::HardReset),
            "POWER" => Some(AcmEventKind::Power),
            "FDS_INSERT" => Some(AcmEventKind::FdsInsert),
            "FDS_EJECT" => Some(AcmEventKind::FdsEject),
            "FDS_SWAP" => Some(AcmEventKind::FdsSwapSide),
            "VS_COIN1" => Some(AcmEventKind::VsInsertCoin1),
            "VS_COIN2" => Some(AcmEventKind::VsInsertCoin2),
            "VS_SERVICE" => Some(AcmEventKind::VsService),
            _ => None,
        }
    }
}

pub(crate) fn region_to_token(r: Region) -> &'static str {
    match r {
        Region::Auto => "AUTO",
        Region::Ntsc => "NTSC",
        Region::Pal => "PAL",
        Region::Dendy => "DENDY",
    }
}

pub(crate) fn region_from_token(s: &str) -> Option<Region> {
    match s {
        "AUTO" => Some(Region::Auto),
        "NTSC" => Some(Region::Ntsc),
        "PAL" => Some(Region::Pal),
        "DENDY" => Some(Region::Dendy),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AcmEvent {
    pub frame: u64,
    pub kind: AcmEventKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DevicePort {
    pub slot: String,
    pub kind: String,
    pub buttons: Vec<char>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub(crate) struct FrameInput {
    pub ports: Vec<Vec<bool>>,
}

impl FrameInput {
    pub(crate) fn empty(nports: usize) -> FrameInput {
        FrameInput { ports: vec![Vec::new(); nports] }
    }

    pub(crate) fn empty_for(devices: &[DevicePort]) -> FrameInput {
        FrameInput { ports: devices.iter().map(|d| vec![false; d.buttons.len()]).collect() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Chapter {
    pub frame: u64,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Subtitle {
    pub start: u64,
    pub end: u64,
    pub text: String,
}

pub(crate) fn guid_from_bytes(b: &[u8; 16]) -> String {
    format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        b[0], b[1], b[2], b[3],
        b[4], b[5],
        b[6], b[7],
        b[8], b[9],
        b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

pub(crate) fn guid_to_bytes(guid: &str) -> [u8; 16] {
    let hex: String = guid.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hex.len() == 32 {
        let mut b = [0u8; 16];
        for i in 0..16 {
            if let Ok(v) = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16) {
                b[i] = v;
            }
        }
        return b;
    }
    let mut b = [0u8; 16];
    for (i, c) in guid.bytes().enumerate() {
        b[i % 16] ^= c;
    }
    b
}

pub(crate) fn generate_guid() -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    #[cfg(not(target_arch = "wasm32"))]
    let nanos = {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos()
    };
    #[cfg(target_arch = "wasm32")]
    let nanos = (js_sys::Date::now() * 1_000_000.0) as u128;

    let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    #[cfg(not(target_arch = "wasm32"))]
    let pid = std::process::id() as u64;
    #[cfg(target_arch = "wasm32")]
    let pid = 1u64;
    let mut s0 = (nanos as u64) ^ 0x517cc1b727220a95;
    let mut s1 = ((nanos >> 64) as u64) ^ count.wrapping_mul(0x9e3779b97f4a7c15) ^ pid;
    s0 ^= s0 << 13; s0 ^= s0 >> 7; s0 ^= s0 << 17;
    s1 ^= s1 << 13; s1 ^= s1 >> 7; s1 ^= s1 << 17;
    let b0 = s0.to_le_bytes();
    let b1 = s1.to_le_bytes();
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&b0);
    bytes[8..].copy_from_slice(&b1);
    bytes[6] = (bytes[6] & 0x0F) | 0x40;
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    guid_from_bytes(&bytes)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MovieFile {
    pub format: u32,
    pub emulator: String,
    pub rom_title: String,
    pub rom_hash: String,
    pub region: Region,
    pub rerecords: u32,
    pub author: String,
    pub comment: String,
    pub power_on: bool,
    pub guid: String,
    pub settings: Vec<(String, String)>,
    pub devices: Vec<DevicePort>,
    pub frames: Vec<FrameInput>,
    pub events: Vec<AcmEvent>,
    pub chapters: Vec<Chapter>,
    pub subtitles: Vec<Subtitle>,
}

impl MovieFile {
    pub(crate) fn empty() -> MovieFile {
        MovieFile {
            format: ACM_FORMAT_VERSION,
            emulator: format!("AccuNES {}", crate::APP_VERSION),
            rom_title: String::new(),
            rom_hash: String::new(),
            region: Region::Auto,
            rerecords: 0,
            author: String::new(),
            comment: String::new(),
            power_on: false,
            guid: generate_guid(),
            settings: Vec::new(),
            devices: Vec::new(),
            frames: Vec::new(),
            events: Vec::new(),
            chapters: Vec::new(),
            subtitles: Vec::new(),
        }
    }

    pub(crate) fn standard_devices() -> Vec<DevicePort> {
        (1..=4)
            .map(|p| DevicePort {
                slot: format!("port{}", p),
                kind: "NES_PAD".to_string(),
                buttons: NES_PAD_BUTTONS.to_vec(),
            })
            .collect()
    }

    pub(crate) fn with_standard_pads() -> MovieFile {
        let mut f = MovieFile::empty();
        f.devices = MovieFile::standard_devices();
        f
    }

    pub(crate) fn frame_is_empty(&self, idx: usize) -> bool {
        match self.frames.get(idx) {
            None => true,
            Some(fr) => fr.ports.iter().all(|b| b.iter().all(|v| !v)),
        }
    }

    pub(crate) fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(ACM_MAGIC);
        out.push('\n');
        out.push_str(&format!("[format] {}\n", self.format));
        out.push_str(&format!("[emulator] {}\n", escape(&self.emulator)));
        out.push_str(&format!("[romTitle] {}\n", escape(&self.rom_title)));
        out.push_str(&format!("[romHash] {}\n", escape(&self.rom_hash)));
        out.push_str(&format!("[region] {}\n", region_to_token(self.region)));
        out.push_str(&format!("[rerecords] {}\n", self.rerecords));
        out.push_str(&format!("[author] {}\n", escape(&self.author)));
        out.push_str(&format!("[comment] {}\n", escape(&self.comment)));
        out.push_str(&format!("[powerOn] {}\n", if self.power_on { "1" } else { "0" }));
        out.push_str(&format!("[guid] {}\n", self.guid));
        for (k, v) in &self.settings {
            out.push_str(&format!("[setting] {}={}\n", escape(k), escape(v)));
        }
        out.push_str("[devices]\n");
        for d in &self.devices {
            let mut line = format!("{} {} ", d.slot, d.kind);
            for b in &d.buttons {
                line.push(*b);
                line.push(' ');
            }
            out.push_str(line.trim_end());
            out.push('\n');
        }
        out.push_str("[frames]\n");
        for (i, fr) in self.frames.iter().enumerate() {
            if self.frame_is_empty(i) {
                continue;
            }
            out.push('|');
            out.push_str(&format!("{:x}", i));
            for (pi, d) in self.devices.iter().enumerate() {
                out.push('|');
                out.push_str(&encode_field(fr, d, pi));
            }
            out.push('\n');
        }
        out.push_str("[events]\n");
        for e in &self.events {
            out.push_str(&format!("F{:x} {}\n", e.frame, e.kind.name()));
        }
        if !self.chapters.is_empty() {
            out.push_str("[chapters]\n");
            for c in &self.chapters {
                out.push_str(&format!("F{:x} {}\n", c.frame, escape(&c.name)));
            }
        }
        if !self.subtitles.is_empty() {
            out.push_str("[subtitles]\n");
            for s in &self.subtitles {
                out.push_str(&format!("F{:x}-F{:x} {}\n", s.start, s.end, escape(&s.text)));
            }
        }
        out
    }

    pub(crate) fn parse_text(text: &str) -> Result<MovieFile, String> {
        let mut f = MovieFile::empty();
        let mut section = Section::None;
        let mut last_frame: Option<u64> = None;
        let mut magic_seen = false;
        for (idx, raw) in text.lines().enumerate() {
            let line_no = idx + 1;
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') {
                let end = line
                    .find(']')
                    .ok_or_else(|| format!("line {}: unterminated section tag", line_no))?;
                let key = &line[1..end];
                let rest = line[end + 1..].trim_start();
                match key {
                    "format" => {
                        let n: u32 = rest.parse().map_err(|_| format!("line {}: invalid format version", line_no))?;
                        if n == 0 {
                            return Err(format!("line {}: invalid format version", line_no));
                        }
                        if n > ACM_FORMAT_VERSION {
                            return Err(format!(
                                "line {}: unsupported .acm format {} (this build supports up to {})",
                                line_no, n, ACM_FORMAT_VERSION
                            ));
                        }
                        f.format = n;
                    }
                    "emulator" => f.emulator = unescape(rest),
                    "romTitle" => f.rom_title = unescape(rest),
                    "romHash" => f.rom_hash = unescape(rest),
                    "region" => {
                        f.region = region_from_token(rest)
                            .ok_or_else(|| format!("line {}: unknown region '{}'", line_no, rest))?;
                    }
                    "rerecords" => {
                        f.rerecords = rest
                            .parse()
                            .map_err(|_| format!("line {}: invalid rerecords", line_no))?;
                    }
                    "author" => f.author = unescape(rest),
                    "comment" => f.comment = unescape(rest),
                    "powerOn" => f.power_on = parse_bool(rest, line_no)?,
                    "guid" => f.guid = rest.trim().to_string(),
                    "setting" => {
                        f.settings.push(split_setting(rest, line_no)?);
                    }
                    "devices" => section = Section::Devices,
                    "frames" => section = Section::Frames,
                    "events" => section = Section::Events,
                    "chapters" => section = Section::Chapters,
                    "subtitles" => section = Section::Subtitles,
                    _ => return Err(format!("line {}: unknown section '{}'", line_no, key)),
                }
                continue;
            }
            match section {
                Section::None => {
                    if line == ACM_MAGIC {
                        magic_seen = true;
                        continue;
                    }
                    return Err(format!("line {}: content before any section", line_no));
                }
                Section::Devices => {
                    f.devices.push(parse_device(line, line_no)?);
                }
                Section::Frames => {
                    let fields: Vec<&str> = line.split('|').collect();
                    if fields.len() != f.devices.len() + 2 {
                        return Err(format!(
                            "line {}: expected {} frame fields, found {}",
                            line_no,
                            f.devices.len(),
                            fields.len().saturating_sub(2)
                        ));
                    }
                    let frame = u64::from_str_radix(fields[1], 16)
                        .map_err(|_| format!("line {}: invalid frame number '{}'", line_no, fields[1]))?;
                    if let Some(last) = last_frame {
                        if frame <= last {
                            return Err(format!("line {}: frame numbers must increase", line_no));
                        }
                    }
                    let np = f.devices.len();
                    if last_frame.is_none() {
                        for _ in 0..frame {
                            f.frames.push(FrameInput::empty_for(&f.devices));
                        }
                    } else if let Some(mut last) = last_frame {
                        while last + 1 < frame {
                            f.frames.push(FrameInput::empty_for(&f.devices));
                            last += 1;
                        }
                    }
                    let mut ports = Vec::with_capacity(np);
                    for (pi, d) in f.devices.iter().enumerate() {
                        ports.push(decode_field(fields[pi + 2], d, line_no)?);
                    }
                    f.frames.push(FrameInput { ports });
                    last_frame = Some(frame);
                }
                Section::Events => {
                    let (frame, rest) = split_tagged(line, line_no)?;
                    let kind = AcmEventKind::from_name(rest)
                        .ok_or_else(|| format!("line {}: unknown event '{}'", line_no, rest))?;
                    f.events.push(AcmEvent { frame, kind });
                }
                Section::Chapters => {
                    let (frame, name) = split_tagged(line, line_no)?;
                    f.chapters.push(Chapter { frame, name: unescape(name) });
                }
                Section::Subtitles => {
                    let rest_line = line.strip_prefix('F').unwrap_or(line);
                    let dash = rest_line
                        .find('-')
                        .ok_or_else(|| format!("line {}: malformed subtitle range", line_no))?;
                    let start = u64::from_str_radix(&rest_line[..dash], 16)
                        .map_err(|_| format!("line {}: invalid subtitle start", line_no))?;
                    let tail = &rest_line[dash + 1..];
                    let sp = tail
                        .find(' ')
                        .ok_or_else(|| format!("line {}: malformed subtitle range", line_no))?;
                    let end_token = &tail[..sp];
                    let end_str = end_token.strip_prefix('F').unwrap_or(end_token);
                    let end = u64::from_str_radix(end_str, 16)
                        .map_err(|_| format!("line {}: invalid subtitle end", line_no))?;
                    if end < start {
                        return Err(format!("line {}: subtitle end before start", line_no));
                    }
                    f.subtitles.push(Subtitle { start, end, text: unescape(tail[sp + 1..].trim_start()) });
                }
            }
        }
        if !magic_seen {
            return Err("missing magic 'ACMN'".to_string());
        }
        if f.format == 0 {
            return Err("missing [format] header".to_string());
        }
        f.sort_by_frame();
        Ok(f)
    }

    fn sort_by_frame(&mut self) {
        self.events.sort_by_key(|e| e.frame);
        self.chapters.sort_by_key(|c| c.frame);
        self.subtitles.sort_by_key(|s| s.start);
    }

    pub(crate) fn to_binary(&self) -> Vec<u8> {
        let mut w = BinWriter::new();
        w.raw(ACM_BINARY_MAGIC.as_bytes());
        w.u32(self.format);
        w.u8(region_index(self.region));
        w.u32(self.rerecords);
        w.u8(if self.power_on { 1 } else { 0 });
        w.str(&self.emulator);
        w.str(&self.rom_title);
        w.str(&self.rom_hash);
        w.str(&self.author);
        w.str(&self.comment);
        let mut settings = self.settings.clone();
        if !settings.iter().any(|(k, _)| k == "guid") {
            settings.push(("guid".to_string(), self.guid.clone()));
        }
        w.u32(settings.len() as u32);
        for (k, v) in &settings {
            w.str(k);
            w.str(v);
        }
        w.u32(self.devices.len() as u32);
        for d in &self.devices {
            w.str(&d.slot);
            w.str(&d.kind);
            w.u32(d.buttons.len() as u32);
            for &b in &d.buttons {
                w.u8(b as u8);
            }
        }
        let n = self.frames.len();
        w.u32(n as u32);
        let mut i = 0usize;
        let mut prev = FrameInput::empty(self.devices.len());
        while i < n {
            let base = &self.frames[i];
            let delta = calc_frame_delta(&prev, base, &self.devices);
            if delta.iter().all(|&b| b == 0) {
                let mut count = 1u32;
                while (i + count as usize) < n {
                    let d2 = calc_frame_delta(base, &self.frames[i + count as usize], &self.devices);
                    if d2.iter().all(|&b| b == 0) {
                        count += 1;
                    } else {
                        break;
                    }
                }
                w.u8(0);
                w.u32(count);
                prev = base.clone();
                i += count as usize;
            } else {
                w.u8(1);
                w.raw(&delta);
                prev = base.clone();
                i += 1;
            }
        }
        w.u32(self.events.len() as u32);
        for e in &self.events {
            w.u64(e.frame);
            w.u8(kind_index(e.kind));
        }
        w.u32(self.chapters.len() as u32);
        for c in &self.chapters {
            w.u64(c.frame);
            w.str(&c.name);
        }
        w.u32(self.subtitles.len() as u32);
        for s in &self.subtitles {
            w.u64(s.start);
            w.u64(s.end);
            w.str(&s.text);
        }
        w.finish()
    }

    pub(crate) fn parse_binary(data: &[u8]) -> Result<MovieFile, String> {
        let mut r = BinReader::new(data);
        if r.bytes(4)? != ACM_BINARY_MAGIC.as_bytes() {
            return Err("not a binary .acm movie (bad magic)".to_string());
        }
        let mut f = MovieFile::empty();
        f.format = r.u32()?;
        if f.format == 0 || f.format > ACM_FORMAT_VERSION {
            return Err(format!("unsupported binary .acm format {}", f.format));
        }
        f.region = region_at_index(r.u8()? as usize)
            .ok_or_else(|| "binary movie: invalid region".to_string())?;
        f.rerecords = r.u32()?;
        f.power_on = match r.u8()? {
            0 => false,
            1 => true,
            _ => return Err("binary movie: invalid powerOn".to_string()),
        };
        f.emulator = r.str()?;
        f.rom_title = r.str()?;
        f.rom_hash = r.str()?;
        f.author = r.str()?;
        f.comment = r.str()?;
        let ns = r.u32()? as usize;
        for _ in 0..ns {
            let k = r.str()?;
            let v = r.str()?;
            f.settings.push((k, v));
        }
        if let Some((_, g)) = f.settings.iter().find(|(k, _)| k == "guid") {
            f.guid = g.clone();
        }
        let nd = r.u32()? as usize;
        for _ in 0..nd {
            let slot = r.str()?;
            let kind = r.str()?;
            let nb = r.u32()? as usize;
            let mut buttons = Vec::with_capacity(nb);
            let mut seen = Vec::with_capacity(nb);
            for _ in 0..nb {
                let c = r.u8()?;
                if !c.is_ascii_alphabetic() {
                    return Err("binary movie: invalid button".to_string());
                }
                if seen.contains(&c) {
                    return Err("binary movie: duplicate button".to_string());
                }
                seen.push(c);
                buttons.push(c as char);
            }
            if buttons.is_empty() {
                return Err("binary movie: device has no buttons".to_string());
            }
            f.devices.push(DevicePort { slot, kind, buttons });
        }
        let nfr = r.u32()? as usize;
        let mut prev = FrameInput::empty(f.devices.len());
        let mut pushed = 0usize;
        while pushed < nfr {
            match r.u8()? {
                0 => {
                    let count = r.u32()? as usize;
                    if count == 0 {
                        return Err("binary movie: zero-length run".to_string());
                    }
                    if pushed + count > nfr {
                        return Err("binary movie: run overruns declared frame count".to_string());
                    }
                    for _ in 0..count {
                        f.frames.push(prev.clone());
                        pushed += 1;
                    }
                }
                1 => {
                    let w: usize = f.devices.iter().map(|d| (d.buttons.len() + 7) / 8).sum();
                    let delta = r.bytes(w)?.to_vec();
                    let cur = apply_frame_delta(&prev, &delta, &f.devices);
                    f.frames.push(cur.clone());
                    prev = cur;
                    pushed += 1;
                }
                _ => return Err("binary movie: unknown frame tag".to_string()),
            }
        }
        let ne = r.u32()? as usize;
        for _ in 0..ne {
            let frame = r.u64()?;
            let kind = kind_from_index(r.u8()?)
                .ok_or_else(|| "binary movie: unknown event".to_string())?;
            f.events.push(AcmEvent { frame, kind });
        }
        let nc = r.u32()? as usize;
        for _ in 0..nc {
            let frame = r.u64()?;
            let name = r.str()?;
            f.chapters.push(Chapter { frame, name });
        }
        let nsub = r.u32()? as usize;
        for _ in 0..nsub {
            let start = r.u64()?;
            let end = r.u64()?;
            if end < start {
                return Err("binary movie: subtitle end before start".to_string());
            }
            let text = r.str()?;
            f.subtitles.push(Subtitle { start, end, text });
        }
        if r.remaining() != 0 {
            return Err("binary movie: trailing data".to_string());
        }
        f.sort_by_frame();
        Ok(f)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Section {
    None,
    Devices,
    Frames,
    Events,
    Chapters,
    Subtitles,
}

fn parse_bool(s: &str, line_no: usize) -> Result<bool, String> {
    match s {
        "1" => Ok(true),
        "0" => Ok(false),
        _ => Err(format!("line {}: expected 1 or 0, found '{}'", line_no, s)),
    }
}

fn split_setting(s: &str, line_no: usize) -> Result<(String, String), String> {
    match s.find('=') {
        Some(i) => Ok((s[..i].to_string(), s[i + 1..].to_string())),
        None => Err(format!("line {}: malformed setting, expected key=value", line_no)),
    }
}

fn parse_device(line: &str, line_no: usize) -> Result<DevicePort, String> {
    let mut it = line.split_whitespace();
    let slot = match it.next() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return Err(format!("line {}: device missing slot", line_no)),
    };
    let kind = match it.next() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return Err(format!("line {}: device missing kind", line_no)),
    };
    let mut buttons: Vec<char> = Vec::new();
    for b in it {
        if b.len() != 1 {
            return Err(format!("line {}: invalid button name '{}'", line_no, b));
        }
        let c = b.chars().next().unwrap();
        if !c.is_ascii_alphabetic() {
            return Err(format!("line {}: invalid button name '{}'", line_no, b));
        }
        if buttons.contains(&c) {
            return Err(format!("line {}: duplicate button '{}'", line_no, c));
        }
        buttons.push(c);
    }
    if buttons.is_empty() {
        return Err(format!("line {}: device '{}' has no buttons", line_no, slot));
    }
    Ok(DevicePort { slot, kind, buttons })
}

fn decode_field(field: &str, dev: &DevicePort, idx: usize) -> Result<Vec<bool>, String> {
    if field.len() != dev.buttons.len() {
        return Err(format!(
            "device slot '{}': field {} has {} chars but device has {} buttons",
            dev.slot,
            idx,
            field.len(),
            dev.buttons.len()
        ));
    }
    let mut v = Vec::with_capacity(field.len());
    for (i, c) in field.chars().enumerate() {
        if c == '.' {
            v.push(false);
        } else if c == dev.buttons[i] {
            v.push(true);
        } else {
            return Err(format!(
                "device slot '{}': unexpected button char '{}' at field position {}",
                dev.slot, c, i
            ));
        }
    }
    Ok(v)
}

fn encode_field(inp: &FrameInput, dev: &DevicePort, idx: usize) -> String {
    let active = inp.ports.get(idx).map(|v| v.as_slice()).unwrap_or(&[]);
    let mut s = String::with_capacity(dev.buttons.len());
    for (j, letter) in dev.buttons.iter().enumerate() {
        if active.get(j).copied().unwrap_or(false) {
            s.push(*letter);
        } else {
            s.push('.');
        }
    }
    s
}

fn pack_buttons(bits: &[bool]) -> Vec<u8> {
    let nbytes = (bits.len() + 7) / 8;
    let mut out = vec![0u8; nbytes];
    for (i, &b) in bits.iter().enumerate() {
        if b {
            out[i / 8] |= 1 << (i % 8);
        }
    }
    out
}

fn unpack_buttons(bytes: &[u8], count: usize) -> Vec<bool> {
    let mut v = Vec::with_capacity(count);
    for i in 0..count {
        v.push(bytes.get(i / 8).copied().unwrap_or(0) & (1 << (i % 8)) != 0);
    }
    v
}

fn delta_len(devices: &[DevicePort]) -> usize {
    devices.iter().map(|d| (d.buttons.len() + 7) / 8).sum()
}

fn calc_frame_delta(prev: &FrameInput, cur: &FrameInput, devices: &[DevicePort]) -> Vec<u8> {
    let mut out = Vec::with_capacity(delta_len(devices));
    for (pi, d) in devices.iter().enumerate() {
        let want = (d.buttons.len() + 7) / 8;
        let p = pack_buttons(prev.ports.get(pi).map(Vec::as_slice).unwrap_or(&[]));
        let c = pack_buttons(cur.ports.get(pi).map(Vec::as_slice).unwrap_or(&[]));
        for i in 0..want {
            out.push(c.get(i).copied().unwrap_or(0) ^ p.get(i).copied().unwrap_or(0));
        }
    }
    out
}

fn apply_frame_delta(prev: &FrameInput, delta: &[u8], devices: &[DevicePort]) -> FrameInput {
    let mut ports = Vec::with_capacity(devices.len());
    let mut off = 0usize;
    for d in devices {
        let want = (d.buttons.len() + 7) / 8;
        let p = pack_buttons(prev.ports.get(ports.len()).map(Vec::as_slice).unwrap_or(&[]));
        let mut bytes = Vec::with_capacity(want);
        for i in 0..want {
            bytes.push(p.get(i).copied().unwrap_or(0) ^ delta.get(off + i).copied().unwrap_or(0));
        }
        off += want;
        ports.push(unpack_buttons(&bytes, d.buttons.len()));
    }
    FrameInput { ports }
}

fn region_index(r: Region) -> u8 {
    match r {
        Region::Auto => 0,
        Region::Ntsc => 1,
        Region::Pal => 2,
        Region::Dendy => 3,
    }
}

fn region_at_index(i: usize) -> Option<Region> {
    match i {
        0 => Some(Region::Auto),
        1 => Some(Region::Ntsc),
        2 => Some(Region::Pal),
        3 => Some(Region::Dendy),
        _ => None,
    }
}

fn kind_index(k: AcmEventKind) -> u8 {
    match k {
        AcmEventKind::SoftReset => 0,
        AcmEventKind::HardReset => 1,
        AcmEventKind::Power => 2,
        AcmEventKind::FdsInsert => 3,
        AcmEventKind::FdsEject => 4,
        AcmEventKind::FdsSwapSide => 5,
        AcmEventKind::VsInsertCoin1 => 6,
        AcmEventKind::VsInsertCoin2 => 7,
        AcmEventKind::VsService => 8,
    }
}

fn kind_from_index(i: u8) -> Option<AcmEventKind> {
    match i {
        0 => Some(AcmEventKind::SoftReset),
        1 => Some(AcmEventKind::HardReset),
        2 => Some(AcmEventKind::Power),
        3 => Some(AcmEventKind::FdsInsert),
        4 => Some(AcmEventKind::FdsEject),
        5 => Some(AcmEventKind::FdsSwapSide),
        6 => Some(AcmEventKind::VsInsertCoin1),
        7 => Some(AcmEventKind::VsInsertCoin2),
        8 => Some(AcmEventKind::VsService),
        _ => None,
    }
}

struct BinWriter {
    out: Vec<u8>,
}

impl BinWriter {
    fn new() -> BinWriter {
        BinWriter { out: Vec::new() }
    }
    fn u8(&mut self, v: u8) {
        self.out.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }
    fn str(&mut self, s: &str) {
        let b = s.as_bytes();
        self.u32(b.len() as u32);
        self.out.extend_from_slice(b);
    }
    fn raw(&mut self, b: &[u8]) {
        self.out.extend_from_slice(b);
    }
    fn finish(self) -> Vec<u8> {
        self.out
    }
}

struct BinReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BinReader<'a> {
    fn new(data: &'a [u8]) -> BinReader<'a> {
        BinReader { data, pos: 0 }
    }
    fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }
    fn u8(&mut self) -> Result<u8, String> {
        match self.data.get(self.pos) {
            Some(&v) => {
                self.pos += 1;
                Ok(v)
            }
            None => Err("binary movie: unexpected end of data".to_string()),
        }
    }
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], String> {
        if n > self.remaining() {
            return Err("binary movie: unexpected end of data".to_string());
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32, String> {
        let b: [u8; 4] = self.bytes(4)?.try_into().unwrap();
        Ok(u32::from_le_bytes(b))
    }
    fn u64(&mut self) -> Result<u64, String> {
        let b: [u8; 8] = self.bytes(8)?.try_into().unwrap();
        Ok(u64::from_le_bytes(b))
    }
    fn str(&mut self) -> Result<String, String> {
        let len = self.u32()? as usize;
        let raw = self.bytes(len)?;
        std::str::from_utf8(raw)
            .map(str::to_string)
            .map_err(|_| "binary movie: invalid UTF-8 string".to_string())
    }
}

fn split_tagged(line: &str, line_no: usize) -> Result<(u64, &str), String> {
    if !line.starts_with('F') {
        return Err(format!("line {}: expected frame tag starting with 'F'", line_no));
    }
    let rest = &line[1..];
    let sp = rest
        .find(' ')
        .ok_or_else(|| format!("line {}: malformed frame tag", line_no))?;
    let frame = u64::from_str_radix(&rest[..sp], 16)
        .map_err(|_| format!("line {}: invalid frame number '{}'", line_no, &rest[..sp]))?;
    Ok((frame, rest[sp + 1..].trim_start()))
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\x5c").replace('\n', "\\n")
}

fn unescape(s: &str) -> String {
    s.replace("\\n", "\n").replace("\\x5c", "\\")
}

pub(crate) static LIVE_INPUT_LOCK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) struct MovieEngine {
    pub mode: MovieMode,
    pub read_only: bool,
    pub file: MovieFile,
    pub cursor: u64,
    pub dirty: bool,
    pub last_play_path: Option<String>,
    pub current_path: Option<String>,
}

impl MovieEngine {
    pub(crate) fn new() -> MovieEngine {
        MovieEngine {
            mode: MovieMode::None,
            read_only: false,
            file: MovieFile::with_standard_pads(),
            cursor: 0,
            dirty: false,
            last_play_path: None,
            current_path: None,
        }
    }

    pub(crate) fn active(&self) -> bool {
        matches!(self.mode, MovieMode::Recording | MovieMode::Playing | MovieMode::RecordingFromPlayback)
    }

    pub(crate) fn is_providing_input(&self) -> bool {
        matches!(self.mode, MovieMode::Playing | MovieMode::RecordingFromPlayback)
    }

    pub(crate) fn has_movie(&self) -> bool {
        self.mode != MovieMode::None || !self.file.frames.is_empty() || !self.file.events.is_empty()
    }

    pub(crate) fn has_unsaved_content(&self) -> bool {
        self.dirty && (!self.file.frames.is_empty() || !self.file.events.is_empty())
    }

    pub(crate) fn save_to_disk(&mut self, path: &str, binary: bool) -> Result<(), String> {
        let bytes = if binary { self.file.to_binary() } else { self.file.to_text().into_bytes() };
        std::fs::write(path, bytes).map_err(|e| e.to_string())?;
        self.current_path = Some(path.to_string());
        self.dirty = false;
        Ok(())
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub(crate) fn recorded_frames(&self) -> u64 {
        self.file.frames.len() as u64
    }

    pub(crate) fn begin_recording(&mut self) {
        self.mode = MovieMode::Recording;
        self.dirty = true;
    }

    pub(crate) fn begin_playing(&mut self) {
        self.mode = MovieMode::Playing;
    }

    pub(crate) fn start_new_recording(&mut self) {
        self.file = MovieFile::with_standard_pads();
        self.cursor = 0;
        self.dirty = true;
        self.mode = MovieMode::Recording;
        self.current_path = None;
    }

    pub(crate) fn load_playback_from_disk(&mut self, path: &str) -> Result<(), String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let lower = path.to_lowercase();
        let file = if lower.ends_with(".fm2") || lower.ends_with(".fm3") {
            crate::fm2::import_from_fm(&bytes)?
        } else if bytes.starts_with(ACM_BINARY_MAGIC.as_bytes()) {
            MovieFile::parse_binary(&bytes)?
        } else if let Ok(text) = std::str::from_utf8(&bytes) {
            if text.starts_with("version 2") || text.starts_with("version 3") {
                crate::fm2::import_from_fm(&bytes)?
            } else {
                MovieFile::parse_text(text)?
            }
        } else {
            MovieFile::parse_binary(&bytes)?
        };
        self.file = file;
        self.cursor = 0;
        self.dirty = false;
        self.mode = MovieMode::Playing;
        self.read_only = true;
        self.last_play_path = Some(path.to_string());
        self.current_path = Some(path.to_string());
        Ok(())
    }

    pub(crate) fn adopt_import(&mut self, file: MovieFile) {
        self.file = file;
        self.cursor = 0;
        self.dirty = false;
        self.mode = MovieMode::Playing;
        self.read_only = true;
        self.current_path = None;
    }

    pub(crate) fn describe(&self) -> String {
        let mode_str = match self.mode {
            MovieMode::None => "Inactive".to_string(),
            MovieMode::Recording => "Recording".to_string(),
            MovieMode::Playing => "Playback".to_string(),
            MovieMode::RecordingFromPlayback => "Recording from playback".to_string(),
        };
        format!(
            "Mode: {}\nRead-only: {}\nFrames: {}\nCursor: {}\nRerecords: {}\nPower cycle on start: {}",
            mode_str,
            if self.read_only { "yes" } else { "no" },
            self.recorded_frames(),
            self.cursor,
            self.file.rerecords,
            if self.file.power_on { "yes" } else { "no" },
        )
    }

    pub(crate) fn to_recording_from_playback(&mut self) {
        if self.mode == MovieMode::Playing {
            self.mode = MovieMode::RecordingFromPlayback;
            self.file.rerecords += 1;
            self.dirty = true;
        }
    }

    pub(crate) fn stop(&mut self) {
        self.mode = MovieMode::None;
    }

    pub(crate) fn set_read_only(&mut self, ro: bool) {
        self.read_only = ro;
    }

    pub(crate) fn status_ribbon(&self) -> Option<String> {
        let label = match self.mode {
            MovieMode::None => return None,
            MovieMode::Recording => "REC",
            MovieMode::Playing => "PLAY",
            MovieMode::RecordingFromPlayback => "REC+",
        };
        Some(format!("{} {:06} / {:06}", label, self.cursor, self.file.frames.len()))
    }

    pub(crate) fn seek_frame(&mut self, frame: u64) {
        self.cursor = frame.min(self.file.frames.len() as u64);
    }

    pub(crate) fn seek_start(&mut self) {
        self.cursor = 0;
    }

    pub(crate) fn seek_end(&mut self) {
        self.cursor = self.file.frames.len() as u64;
    }

    pub(crate) fn backtrack_prev_frame(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub(crate) fn backtrack_next_frame(&mut self) {
        self.cursor = (self.cursor + 1).min(self.file.frames.len() as u64);
    }

    pub(crate) fn backtrack_prev_input_change(&mut self) {
        let n = self.file.frames.len();
        if n < 2 {
            self.cursor = 0;
            return;
        }
        for i in (1..self.cursor as usize).rev() {
            if self.file.frames[i] != self.file.frames[i - 1] {
                self.cursor = i as u64;
                return;
            }
        }
        self.cursor = 0;
    }

    pub(crate) fn backtrack_next_input_change(&mut self) {
        let n = self.file.frames.len();
        if n < 2 {
            self.cursor = n as u64;
            return;
        }
        for i in self.cursor as usize + 1..n {
            if self.file.frames[i] != self.file.frames[i - 1] {
                self.cursor = i as u64;
                return;
            }
        }
        self.cursor = n as u64;
    }

    pub(crate) fn re_record_from_here(&mut self) {
        let f = self.cursor as usize;
        if f < self.file.frames.len() {
            self.file.frames.truncate(f);
        }
        self.file.rerecords += 1;
        self.dirty = true;
        self.mode = MovieMode::Recording;
    }

    pub(crate) fn record_event(&mut self, kind: AcmEventKind) {
        let in_recording_tail = match self.mode {
            MovieMode::Recording => true,
            MovieMode::RecordingFromPlayback => self.cursor >= self.file.frames.len() as u64,
            _ => false,
        };
        if in_recording_tail {
            self.file.events.push(AcmEvent { frame: self.cursor, kind });
            self.dirty = true;
        }
    }

    pub(crate) fn events_at(&self, frame: u64) -> Vec<AcmEvent> {
        self.file.events.iter().filter(|e| e.frame == frame).cloned().collect()
    }

    pub(crate) fn input_at(&self, frame: u64) -> FrameInput {
        match self.file.frames.get(frame as usize) {
            Some(f) => f.clone(),
            None => FrameInput::empty(self.file.devices.len()),
        }
    }
}

pub(crate) fn movie_initial_ram(file: &MovieFile) -> crate::config::InitialRam {
    for (k, v) in &file.settings {
        if k == "initial-ram" {
            if let Some(mode) = crate::config::InitialRam::from_token(v) {
                return mode;
            }
        }
    }
    crate::config::load_initial_ram()
}

fn normalize_title(s: &str) -> String {
    let mut t = s.trim().to_ascii_lowercase();
    for ext in &[".nes", ".fds", ".unf", ".unif", ".nsf", ".fm2", ".fm3", ".acm", ".acmb"] {
        if t.ends_with(ext) {
            t.truncate(t.len() - ext.len());
            break;
        }
    }
    if let Some(pos) = t.rfind(|c| c == '/' || c == '\\') {
        t = t[pos + 1..].to_string();
    }
    t
}

pub(crate) fn titles_match(a: &str, b: &str) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    let na = normalize_title(a);
    let nb = normalize_title(b);
    if na == nb {
        return true;
    }
    if !na.is_empty() && !nb.is_empty() && (na.starts_with(&nb) || nb.starts_with(&na)) {
        return true;
    }
    false
}

pub(crate) fn rom_mismatch_warning(
    recorded_title: &str,
    recorded_hash: &str,
    cur_name: &str,
    candidate_hashes: &[&str],
    display_hash: &str,
) -> Option<String> {
    let is_zero_hash = recorded_hash.is_empty()
        || recorded_hash == "0"
        || recorded_hash == "crc32:00000000"
        || recorded_hash.strip_prefix("crc32:").map(|h| h.chars().all(|c| c == '0')).unwrap_or(false)
        || recorded_hash.strip_prefix("md5:").map(|h| h.chars().all(|c| c == '0')).unwrap_or(false);

    let can_check = (!is_zero_hash && !recorded_hash.is_empty()) || !recorded_title.is_empty();
    if !can_check {
        return None;
    }

    let hash_matches = !is_zero_hash && candidate_hashes.iter().any(|&h| h.eq_ignore_ascii_case(recorded_hash));
    let title_matches = !recorded_title.is_empty() && titles_match(recorded_title, cur_name);

    if hash_matches || title_matches {
        return None;
    }

    let show_rec_hash = if is_zero_hash || recorded_hash.is_empty() {
        "(unknown)".to_string()
    } else {
        recorded_hash.to_string()
    };

    Some(format!(
        "WARNING: This movie was recorded for a different ROM.\n\nMovie ROM: {}\nMovie hash: {}\nLoaded ROM: {}\nLoaded hash: {}",
        if recorded_title.is_empty() { "(unknown)" } else { recorded_title },
        show_rec_hash,
        cur_name,
        display_hash,
    ))
}
