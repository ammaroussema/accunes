// famicom study box system file parser!!!
use crate::wavreader::{decode, WaveData};

#[derive(Clone)]
pub struct PageInfo {
    pub lead_in_offset: u32,
    pub audio_offset: u32,
    pub data: Vec<u8>,
}

#[derive(Clone)]
pub struct StudyBoxTape {
    pub audio: Option<WaveData>,
    pub pages: Vec<PageInfo>,
}

fn read_u32_le(b: &[u8]) -> u32 {
    b[0] as u32
        | ((b[1] as u32) << 8)
        | ((b[2] as u32) << 16)
        | ((b[3] as u32) << 24)
}

pub fn load_tape(data: &[u8]) -> Result<StudyBoxTape, String> {
    if data.len() < 16 {
        return Err("File is too small to parse".to_string());
    }

    let mut pos = 0usize;
    if &data[0..4] != b"STBX" {
        return Err("Invalid studybox file".to_string());
    }
    pos += 4;

    let size = read_u32_le(&data[pos..pos + 4]);
    pos += 4;
    if size != 4 {
        return Err("Unexpected length value".to_string());
    }

    let version = read_u32_le(&data[pos..pos + 4]);
    pos += 4;
    if version != 0x100 {
        return Err(format!("Unsupported version: {version}"));
    }

    let mut prev_audio_offset = 0u32;
    let mut prev_lead_in_offset = 0u32;
    let mut pages = Vec::new();
    let mut audio_file: Vec<u8> = Vec::new();
    let mut audio_found = false;

    while pos + 4 < data.len() {
        let cc = &data[pos..pos + 4];
        pos += 4;

        if cc == b"PAGE" {
            let page_size = read_u32_le(&data[pos..pos + 4]);
            pos += 4;
            let lead_in_offset = read_u32_le(&data[pos..pos + 4]);
            pos += 4;
            let audio_offset = read_u32_le(&data[pos..pos + 4]);
            pos += 4;

            if audio_offset < lead_in_offset {
                return Err("Track lead in must start before the first bit of data".to_string());
            }
            if audio_offset < prev_audio_offset || lead_in_offset < prev_lead_in_offset {
                return Err("PAGE chunks must be in the order found on the audio tape".to_string());
            }
            prev_audio_offset = audio_offset;
            prev_lead_in_offset = lead_in_offset;

            let data_len = (page_size as usize).saturating_sub(8);
            if pos + data_len <= data.len() {
                let page_data = data[pos..pos + data_len].to_vec();
                pages.push(PageInfo {
                    lead_in_offset,
                    audio_offset,
                    data: page_data,
                });
                pos += data_len;
            } else {
                return Err("Invalid size value for PAGE chunk".to_string());
            }
        } else if cc == b"AUDI" {
            let audio_size = read_u32_le(&data[pos..pos + 4]);
            pos += 4;
            let file_type = read_u32_le(&data[pos..pos + 4]);
            pos += 4;
            if file_type == 0 {
                let data_len = (audio_size as usize).saturating_sub(4);
                if pos + data_len <= data.len() {
                    audio_file = data[pos..pos + data_len].to_vec();
                } else {
                    return Err("Invalid size value for AUDI chunk".to_string());
                }
                audio_found = true;
                break;
            } else {
                return Err(format!("Unsupported audio type: {file_type}"));
            }
        } else {
            return Err("Unsupported tag".to_string());
        }
    }

    if pages.is_empty() {
        return Err("No pages found in tape".to_string());
    }

    let audio = if audio_found {
        decode(&audio_file)
    } else {
        None
    };

    Ok(StudyBoxTape { audio, pages })
}
pub fn find_page_index(pages: &[PageInfo], page: u8) -> usize {
    for (i, p) in pages.iter().enumerate() {
        if p.data.len() > 5 && p.data[5] == page {
            return i;
        }
    }
    0
}
