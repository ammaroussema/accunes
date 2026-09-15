// famicom study box system wave file reader!!!

#[derive(Clone)]
pub struct WaveData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

fn read_u16(b: &[u8]) -> u16 {
    b[0] as u16 | ((b[1] as u16) << 8)
}

fn read_u32(b: &[u8]) -> u32 {
    b[0] as u32
        | ((b[1] as u32) << 8)
        | ((b[2] as u32) << 16)
        | ((b[3] as u32) << 24)
}

fn read_i32(b: &[u8]) -> i32 {
    read_u32(b) as i32
}

pub fn decode(data: &[u8]) -> Option<WaveData> {
    if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return None;
    }

    let mut pos = 12usize;
    let mut audio_format: Option<u16> = None;
    let mut channels: Option<u16> = None;
    let mut sample_rate: Option<u32> = None;
    let mut bits_per_sample: Option<u16> = None;
    let mut pcm_data: Option<&[u8]> = None;

    while pos + 8 <= data.len() {
        let chunk_id = &data[pos..pos + 4];
        let size = read_u32(&data[pos + 4..pos + 8]) as usize;
        let chunk_start = pos + 8;
        if chunk_start + size > data.len() {
            break;
        }

        if chunk_id == b"fmt " {
            if size >= 16 {
                let fmt = &data[chunk_start..chunk_start + size];
                audio_format = Some(read_u16(&fmt[0..2]));
                channels = Some(read_u16(&fmt[2..4]));
                sample_rate = Some(read_u32(&fmt[4..8]));
                bits_per_sample = Some(read_u16(&fmt[14..16]));
            }
        } else if chunk_id == b"data" {
            pcm_data = Some(&data[chunk_start..chunk_start + size]);
        }

        pos = chunk_start + size + (size & 1);
    }

    let fmt = audio_format?;
    let channels = channels?;
    let sample_rate = sample_rate?;
    let bits = bits_per_sample?;
    let pcm = pcm_data?;

    if channels == 0 || sample_rate == 0 || bits == 0 {
        return None;
    }

    let bytes_per_frame = (bits as usize).div_ceil(8) * channels as usize;
    if bytes_per_frame == 0 {
        return None;
    }

    let frames = pcm.len() / bytes_per_frame;
    let mut samples = Vec::with_capacity(frames);

    let mut idx = 0usize;
    for _ in 0..frames {
        let mut acc = 0.0f64;
        for ch in 0..channels {
            let base = idx + ch as usize * (bits as usize).div_ceil(8);
            let b = &pcm[base..base + (bits as usize).div_ceil(8)];
            let v = match fmt {
                1 => match bits {
                    8 => (b[0] as f64 - 128.0) / 128.0,
                    16 => i16::from_le_bytes([b[0], b[1]]) as f64 / 32768.0,
                    24 => {
                        let val = (b[0] as i32)
                            | ((b[1] as i32) << 8)
                            | ((b[2] as i32) << 16);
                        let val = (val << 8) >> 8;
                        val as f64 / 8388608.0
                    }
                    32 => read_i32(&[b[0], b[1], b[2], b[3]]) as f64 / 2147483648.0,
                    _ => 0.0,
                },
                3 => match bits {
                    32 => {
                        let raw = read_u32(&[b[0], b[1], b[2], b[3]]);
                        f32::from_bits(raw) as f64
                    }
                    64 => {
                        let lo = read_u32(&[b[0], b[1], b[2], b[3]]) as u64;
                        let hi = read_u32(&[b[4], b[5], b[6], b[7]]) as u64;
                        f64::from_bits(lo | (hi << 32))
                    }
                    _ => 0.0,
                },
                _ => 0.0,
            };
            acc += v;
        }
        acc /= channels as f64;
        if acc > 1.0 {
            acc = 1.0;
        } else if acc < -1.0 {
            acc = -1.0;
        }
        samples.push(acc as f32);
        idx += bytes_per_frame;
    }

    Some(WaveData {
        samples,
        sample_rate,
    })
}
