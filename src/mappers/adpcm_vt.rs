const VT1682_INDEX_STEP: [u8; 4] = [0, 0, 3, 5];
const VT1682_INDEX_TABLE: [u8; 26] = [
    0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 20, 20, 20, 20,
];
const VT1682_STEP_TABLE: [[i8; 21]; 4] = [
    [0, 1, 1, 1, 1, 1, 2, 2, 2, 3, 3, 4, 5, 5, 6, 7, 8, 10, 11, 13, 15],
    [1, 3, 3, 3, 4, 4, 6, 6, 7, 9, 10, 12, 15, 16, 19, 22, 25, 30, 34, 40, 46],
    [3, 5, 5, 6, 7, 8, 10, 11, 13, 16, 18, 21, 25, 28, 32, 38, 43, 51, 58, 68, 78],
    [4, 7, 7, 8, 10, 11, 14, 15, 18, 22, 25, 29, 35, 39, 45, 53, 60, 71, 81, 95, 109],
];

pub fn vt1682_decode_adpcm(code: u8, output: &mut u8, index: &mut u8) {
    let idx = (*index as usize).min(20);
    let step = VT1682_STEP_TABLE[(code & 3) as usize][idx] as i16;
    let sign = if (code & 4) != 0 { -1 } else { 1 };
    let predictor = (*output as i8 as i16) + step * sign;
    let clamped = predictor.clamp(-128, 127) as i8;
    *output = clamped as u8;
    let next_idx = (*index as usize) + (VT1682_INDEX_STEP[(code & 3) as usize] as usize);
    *index = VT1682_INDEX_TABLE[next_idx.min(25)];
}

const VT369_STEP_TABLE: [[i16; 16]; 16] = [
    [0, 14, 28, 42, 56, 70, 84, 97, -111, -97, -84, -70, -56, -42, -28, -14],
    [0, 13, 26, 39, 52, 65, 78, 91, -104, -91, -78, -65, -52, -39, -26, -13],
    [0, 11, 21, 32, 43, 54, 64, 75, -86, -75, -64, -54, -43, -32, -21, -11],
    [0, 9, 18, 27, 35, 44, 53, 62, -71, -62, -53, -44, -35, -27, -18, -9],
    [0, 7, 13, 20, 27, 34, 40, 47, -54, -47, -40, -34, -27, -20, -13, -7],
    [0, 6, 11, 17, 22, 28, 33, 39, -44, -39, -33, -28, -22, -17, -11, -6],
    [0, 5, 9, 14, 18, 23, 27, 32, -36, -32, -27, -23, -18, -14, -9, -5],
    [0, 4, 8, 11, 15, 19, 23, 26, -30, -26, -23, -19, -15, -11, -8, -4],
    [0, 3, 6, 9, 12, 15, 17, 20, -23, -20, -17, -15, -12, -9, -6, -3],
    [0, 2, 5, 7, 10, 12, 14, 17, -19, -17, -14, -12, -10, -7, -5, -2],
    [0, 2, 4, 6, 8, 10, 12, 14, -16, -14, -12, -10, -8, -6, -4, -2],
    [0, 2, 3, 5, 6, 8, 10, 11, -13, -11, -10, -8, -6, -5, -3, -2],
    [0, 1, 2, 4, 5, 6, 7, 9, -10, -9, -7, -6, -5, -4, -2, -1],
    [0, 1, 2, 3, 4, 5, 6, 7, -8, -7, -6, -5, -4, -3, -2, -1],
    [0, 1, 2, 3, 3, 4, 5, 6, -7, -6, -5, -4, -3, -3, -2, -1],
    [0, 1, 1, 2, 3, 4, 4, 5, -6, -5, -4, -4, -3, -2, -1, -1],
];

pub fn vt369_decode_adpcm(adpcm_data: &mut [u8]) -> i32 {
    let lead = adpcm_data[0];
    let frame = adpcm_data[1];
    let volume = adpcm_data[2];
    let last_output = adpcm_data[3] as i8;
    let cur_output = adpcm_data[4] as i8;
    let mut position = adpcm_data[5] as usize;

    position %= 48;
    let nibble = (frame >> if (position & 1) != 0 { 4 } else { 0 }) & 0x0F;
    let mut index = lead as i32;
    if position >= 24 && (lead & 0x40) != 0 {
        index -= 1;
    }
    if position >= 24 && (lead & 0x80) != 0 {
        index += 2;
    }
    let step = VT369_STEP_TABLE[(index as usize) & 0x0F][nibble as usize];
    let output: i8 = match (lead >> 4) & 3 {
        0 => step as i8,
        1 => (step + (cur_output as i16)) as i8,
        2 => (step + (cur_output as i16) * 2 - (last_output as i16)) as i8,
        3 => (step + (cur_output as i16) - ((last_output as i16) >> 1)) as i8,
        _ => 0,
    };
    adpcm_data[3] = cur_output as u8;
    adpcm_data[4] = output as u8;
    adpcm_data[5] = (position + 1) as u8;
    (output as i32) * ((volume & 0x7F) as i32)
}
