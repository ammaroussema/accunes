// NES Sound Format (NSF) and Extended NES Sound Format (NSFe) file parser!!!

#[derive(Clone, Debug)]
pub struct NsfInfo {
    pub version: u8,
    pub total_songs: u8,
    pub starting_song: u8,
    pub load_address: u16,
    pub init_address: u16,
    pub play_address: u16,
    pub song_name: String,
    pub artist_name: String,
    pub copyright_holder: String,
    pub ripper_name: String,
    pub play_speed_ntsc: u16,
    pub play_speed_pal: u16,
    pub bank_setup: [u8; 8],
    pub flags: u8, 
    pub sound_chips: u8,
    pub track_names: Vec<String>,
    pub track_length_ms: Vec<i32>,
    pub track_fade_ms: Vec<i32>,
    pub is_nsfe: bool,
}

impl Default for NsfInfo {
    fn default() -> Self {
        Self {
            version: 1,
            total_songs: 1,
            starting_song: 1,
            load_address: 0x8000,
            init_address: 0x8000,
            play_address: 0x8000,
            song_name: String::new(),
            artist_name: String::new(),
            copyright_holder: String::new(),
            ripper_name: String::new(),
            play_speed_ntsc: 16639,
            play_speed_pal: 19997,
            bank_setup: [0; 8],
            flags: 0,
            sound_chips: 0,
            track_names: Vec::new(),
            track_length_ms: Vec::new(),
            track_fade_ms: Vec::new(),
            is_nsfe: false,
        }
    }
}

impl NsfInfo {
    pub fn is_pal(&self) -> bool {
        (self.flags & 0x01) != 0 && (self.flags & 0x02) == 0
    }

    pub fn has_vrc6(&self) -> bool { (self.sound_chips & 0x01) != 0 }
    pub fn has_vrc7(&self) -> bool { (self.sound_chips & 0x02) != 0 }
    pub fn has_fds(&self) -> bool { (self.sound_chips & 0x04) != 0 }
    pub fn has_mmc5(&self) -> bool { (self.sound_chips & 0x08) != 0 }
    pub fn has_namco163(&self) -> bool { (self.sound_chips & 0x10) != 0 }
    pub fn has_sunsoft5b(&self) -> bool { (self.sound_chips & 0x20) != 0 }

    pub fn sound_chip_names(&self) -> Vec<&'static str> {
        let mut chips = Vec::new();
        if self.has_vrc6() { chips.push("VRC6"); }
        if self.has_vrc7() { chips.push("VRC7"); }
        if self.has_fds() { chips.push("FDS"); }
        if self.has_mmc5() { chips.push("MMC5"); }
        if self.has_namco163() { chips.push("N163"); }
        if self.has_sunsoft5b() { chips.push("5B"); }
        chips
    }
}

pub struct NsfRom {
    pub info: NsfInfo,
    pub prg_rom: Vec<u8>,
}

fn read_u16_le(data: &[u8], offset: usize) -> u16 {
    (data[offset] as u16) | ((data[offset + 1] as u16) << 8)
}

fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    (data[offset] as u32)
        | ((data[offset + 1] as u32) << 8)
        | ((data[offset + 2] as u32) << 16)
        | ((data[offset + 3] as u32) << 24)
}

fn read_i32_le(data: &[u8], offset: usize) -> i32 {
    read_u32_le(data, offset) as i32
}

fn read_null_terminated_string(bytes: &[u8]) -> String {
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..len]).trim().to_string()
}

pub fn parse_nsf(bytes: &[u8]) -> Result<NsfRom, String> {
    if bytes.len() < 0x80 {
        return Err("NSF file is smaller than 128-byte header".to_string());
    }

    if &bytes[0..5] != b"NESM\x1a" {
        return Err("Invalid NSF header magic (expected NESM\\x1A)".to_string());
    }

    let version = bytes[0x05];
    let total_songs = bytes[0x06].max(1);
    let starting_song = bytes[0x07].clamp(1, total_songs);
    let load_address = read_u16_le(bytes, 0x08);
    let init_address = read_u16_le(bytes, 0x0A);
    let play_address = read_u16_le(bytes, 0x0C);

    let song_name = read_null_terminated_string(&bytes[0x0E..0x2E]);
    let artist_name = read_null_terminated_string(&bytes[0x2E..0x4E]);
    let copyright_holder = read_null_terminated_string(&bytes[0x4E..0x6E]);

    let mut play_speed_ntsc = read_u16_le(bytes, 0x6E);
    if play_speed_ntsc == 0 || play_speed_ntsc == 16666 || play_speed_ntsc == 16667 {
        play_speed_ntsc = 16639;
    }

    let mut bank_setup = [0u8; 8];
    bank_setup.copy_from_slice(&bytes[0x70..0x78]);

    let mut play_speed_pal = read_u16_le(bytes, 0x78);
    if play_speed_pal == 0 || play_speed_pal == 20000 {
        play_speed_pal = 19997;
    }

    let flags = bytes[0x7A];
    let sound_chips = bytes[0x7B];

    let pad_start = (load_address % 4096) as usize;
    let rom_payload = &bytes[0x80..];
    let mut prg_rom = Vec::with_capacity(pad_start + rom_payload.len() + 4096);
    prg_rom.resize(pad_start, 0);
    prg_rom.extend_from_slice(rom_payload);

    let rem = prg_rom.len() % 4096;
    if rem != 0 {
        prg_rom.resize(prg_rom.len() + (4096 - rem), 0);
    }

    let info = NsfInfo {
        version,
        total_songs,
        starting_song,
        load_address,
        init_address,
        play_address,
        song_name,
        artist_name,
        copyright_holder,
        ripper_name: String::new(),
        play_speed_ntsc,
        play_speed_pal,
        bank_setup,
        flags,
        sound_chips,
        track_names: Vec::new(),
        track_length_ms: vec![-1; total_songs as usize],
        track_fade_ms: vec![-1; total_songs as usize],
        is_nsfe: false,
    };

    Ok(NsfRom { info, prg_rom })
}

pub fn parse_nsfe(bytes: &[u8]) -> Result<NsfRom, String> {
    if bytes.len() < 8 {
        return Err("NSFe file is too small".to_string());
    }

    if &bytes[0..4] != b"NSFE" {
        return Err("Invalid NSFe header magic (expected NSFE)".to_string());
    }

    let mut info = NsfInfo {
        is_nsfe: true,
        ..Default::default()
    };

    let mut prg_rom = Vec::new();
    let mut offset = 4;
    let file_len = bytes.len();

    let mut saw_nend = false;

    while offset + 8 <= file_len {
        let chunk_len = read_u32_le(bytes, offset) as usize;
        let fourcc = &bytes[offset + 4..offset + 8];
        offset += 8;

        if offset + chunk_len > file_len {
            return Err("NSFe chunk exceeds file length".to_string());
        }

        let chunk_data = &bytes[offset..offset + chunk_len];
        offset += chunk_len;

        match fourcc {
            b"INFO" => {
                if chunk_len < 8 {
                    return Err("NSFe INFO chunk too small".to_string());
                }
                info.load_address = read_u16_le(chunk_data, 0);
                info.init_address = read_u16_le(chunk_data, 2);
                info.play_address = read_u16_le(chunk_data, 4);
                info.flags = chunk_data[6];
                if chunk_len > 7 {
                    info.sound_chips = chunk_data[7];
                }
                if chunk_len > 8 {
                    info.total_songs = chunk_data[8].max(1);
                }
                if chunk_len > 9 {
                    info.starting_song = (chunk_data[9] + 1).clamp(1, info.total_songs);
                }
                info.play_speed_ntsc = 16639;
                info.play_speed_pal = 19997;
            }
            b"DATA" => {
                let pad_start = (info.load_address % 4096) as usize;
                prg_rom.clear();
                prg_rom.resize(pad_start, 0);
                prg_rom.extend_from_slice(chunk_data);
                let rem = prg_rom.len() % 4096;
                if rem != 0 {
                    prg_rom.resize(prg_rom.len() + (4096 - rem), 0);
                }
            }
            b"BANK" => {
                let count = chunk_len.min(8);
                info.bank_setup[..count].copy_from_slice(&chunk_data[..count]);
            }
            b"time" => {
                let mut pos = 0;
                while pos + 4 <= chunk_len {
                    info.track_length_ms.push(read_i32_le(chunk_data, pos));
                    pos += 4;
                }
            }
            b"fade" => {
                let mut pos = 0;
                while pos + 4 <= chunk_len {
                    info.track_fade_ms.push(read_i32_le(chunk_data, pos));
                    pos += 4;
                }
            }
            b"tlbl" => {
                let mut start = 0;
                for (i, &b) in chunk_data.iter().enumerate() {
                    if b == 0 {
                        let s = String::from_utf8_lossy(&chunk_data[start..i]).trim().to_string();
                        info.track_names.push(s);
                        start = i + 1;
                    }
                }
                if start < chunk_len {
                    let s = String::from_utf8_lossy(&chunk_data[start..chunk_len]).trim().to_string();
                    if !s.is_empty() {
                        info.track_names.push(s);
                    }
                }
            }
            b"auth" => {
                let mut strings = Vec::new();
                let mut start = 0;
                for (i, &b) in chunk_data.iter().enumerate() {
                    if b == 0 {
                        let s = String::from_utf8_lossy(&chunk_data[start..i]).trim().to_string();
                        strings.push(s);
                        start = i + 1;
                    }
                }
                if start < chunk_len {
                    let s = String::from_utf8_lossy(&chunk_data[start..chunk_len]).trim().to_string();
                    if !s.is_empty() {
                        strings.push(s);
                    }
                }
                if let Some(s) = strings.get(0) { info.song_name = s.clone(); }
                if let Some(s) = strings.get(1) { info.artist_name = s.clone(); }
                if let Some(s) = strings.get(2) { info.copyright_holder = s.clone(); }
                if let Some(s) = strings.get(3) { info.ripper_name = s.clone(); }
            }
            b"NEND" => {
                saw_nend = true;
                break;
            }
            _ => {
                if fourcc[0].is_ascii_uppercase() {
                    return Err(format!(
                        "Unsupported mandatory NSFe chunk: {}",
                        String::from_utf8_lossy(fourcc)
                    ));
                }
            }
        }
    }

    if !saw_nend && prg_rom.is_empty() {
        return Err("NSFe missing required DATA or NEND chunk".to_string());
    }

    while info.track_length_ms.len() < info.total_songs as usize {
        info.track_length_ms.push(-1);
    }
    while info.track_fade_ms.len() < info.total_songs as usize {
        info.track_fade_ms.push(-1);
    }

    Ok(NsfRom { info, prg_rom })
}
