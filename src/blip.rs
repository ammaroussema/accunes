// the resampling audio buffer!!!
const TIME_BITS: u32 = 32 + 20;
const TIME_UNIT: u64 = 1u64 << TIME_BITS;

const BASS_SHIFT: u32 = 9; 
const END_FRAME_EXTRA: usize = 2;

const HALF_WIDTH: usize = 8;
const BUF_EXTRA: usize = HALF_WIDTH * 2 + END_FRAME_EXTRA;
const PHASE_BITS: u32 = 5;
const PHASE_COUNT: usize = 1 << PHASE_BITS;
const DELTA_BITS: u32 = 15;
const DELTA_UNIT: i64 = 1 << DELTA_BITS;
const PRE_SHIFT: u32 = 32;
const FRAC_BITS: u32 = TIME_BITS - PRE_SHIFT;

pub const MAX_RATIO: u64 = 1 << 20;

fn arith_shift(n: i64, shift: u32) -> i64 {
    n >> shift
}

const MAX_SAMPLE: i64 = 32767;

fn clamp(n: i64) -> i64 {
    if (n as i16) as i64 != n {
        arith_shift(n, 16) ^ MAX_SAMPLE
    } else {
        n
    }
}

static BL_STEP: [[i16; HALF_WIDTH]; PHASE_COUNT + 1] = [
    [43, -115, 350, -488, 1136, -914, 5861, 21022],
    [44, -118, 348, -473, 1076, -799, 5274, 21001],
    [45, -121, 344, -454, 1011, -677, 4706, 20936],
    [46, -122, 336, -431, 942, -549, 4156, 20829],
    [47, -123, 327, -404, 868, -418, 3629, 20679],
    [47, -122, 316, -375, 792, -285, 3124, 20488],
    [47, -120, 303, -344, 714, -151, 2644, 20256],
    [46, -117, 289, -310, 634, -17, 2188, 19985],
    [46, -114, 273, -275, 553, 117, 1758, 19675],
    [44, -108, 255, -237, 471, 247, 1356, 19327],
    [43, -103, 237, -199, 390, 373, 981, 18944],
    [42, -98, 218, -160, 310, 495, 633, 18527],
    [40, -91, 198, -121, 231, 611, 314, 18078],
    [38, -84, 178, -81, 153, 722, 22, 17599],
    [36, -76, 157, -43, 80, 824, -241, 17092],
    [34, -68, 135, -3, 8, 919, -476, 16558],
    [32, -61, 115, 34, -60, 1006, -683, 16001],
    [29, -52, 94, 70, -123, 1083, -862, 15422],
    [27, -44, 73, 106, -184, 1152, -1015, 14824],
    [25, -36, 53, 139, -239, 1211, -1142, 14210],
    [22, -27, 34, 170, -290, 1261, -1244, 13582],
    [20, -20, 16, 199, -335, 1301, -1322, 12942],
    [18, -12, -3, 226, -375, 1331, -1376, 12293],
    [15, -4, -19, 250, -410, 1351, -1408, 11638],
    [13, 3, -35, 272, -439, 1361, -1419, 10979],
    [11, 9, -49, 292, -464, 1362, -1410, 10319],
    [9, 16, -63, 309, -483, 1354, -1383, 9660],
    [7, 22, -75, 322, -496, 1337, -1339, 9005],
    [6, 26, -85, 333, -504, 1312, -1280, 8355],
    [4, 31, -94, 341, -507, 1278, -1205, 7713],
    [3, 35, -102, 347, -506, 1238, -1119, 7082],
    [1, 40, -110, 350, -499, 1190, -1021, 6464],
    [0, 43, -115, 350, -488, 1136, -914, 5861],
];

pub struct BlipBuf {
    factor: u64,
    offset: u64,
    available: usize,
    size: usize,
    integrator: i64,
    samples: Vec<i32>,
}

impl BlipBuf {
    pub const MAX_FRAME: usize = 4000;

    pub fn new(size: usize) -> BlipBuf {
        let mut m = BlipBuf {
            factor: TIME_UNIT / MAX_RATIO,
            size,
            offset: 0,
            available: 0,
            integrator: 0,
            samples: vec![0i32; size + BUF_EXTRA],
        };
        m.clear();
        m
    }

    pub fn clear(&mut self) {
        self.offset = self.factor / 2;
        self.available = 0;
        self.integrator = 0;
        self.samples.iter_mut().for_each(|s| *s = 0);
    }

    pub fn set_rates(&mut self, clock_rate: f64, sample_rate: f64) {
        let factor = TIME_UNIT as f64 * sample_rate / clock_rate;
        let mut f = factor as u64;
        if (f as f64) < factor {
            f += 1;
        }
        self.factor = f;
    }

    pub fn add_delta(&mut self, clock_time: u32, delta: i32) {
        let fixed = ((clock_time as u64).wrapping_mul(self.factor).wrapping_add(self.offset)) >> PRE_SHIFT;
        let fixed = fixed as u64;
        let out_idx = self.available + ((fixed >> FRAC_BITS) as usize);

        let phase_shift = (FRAC_BITS as i64 - PHASE_BITS as i64) as u32;
        let phase = ((fixed >> phase_shift) & (PHASE_COUNT as u64 - 1)) as usize;
        let pc = PHASE_COUNT - phase;

        let interp = (fixed >> (phase_shift - DELTA_BITS)) & (DELTA_UNIT as u64 - 1);
        let delta2 = ((delta as i64) * (interp as i64)) >> DELTA_BITS;
        let delta1 = (delta as i64) - delta2;

        let out = &mut self.samples[out_idx..out_idx + HALF_WIDTH * 2];
        for k in 0..HALF_WIDTH {
            out[k] = out[k].wrapping_add(
                ((BL_STEP[phase][k] as i64) * delta1 + (BL_STEP[phase + 1][k] as i64) * delta2) as i32,
            );
        }
        for k in 0..HALF_WIDTH {
            let j = HALF_WIDTH - 1 - k;
            out[HALF_WIDTH + k] = out[HALF_WIDTH + k].wrapping_add(
                ((BL_STEP[pc][j] as i64) * delta1 + (BL_STEP[pc - 1][j] as i64) * delta2) as i32,
            );
        }
    }

    pub fn end_frame(&mut self, clock_duration: u32) {
        let off = (clock_duration as u64)
            .wrapping_mul(self.factor)
            .wrapping_add(self.offset);
        self.available += (off >> TIME_BITS) as usize;
        self.offset = off & (TIME_UNIT - 1);
        debug_assert!(self.available <= self.size);
    }

    pub fn samples_available(&self) -> usize {
        self.available
    }

    fn remove_samples(&mut self, count: usize) {
        let remain = self.available + BUF_EXTRA - count;
        self.available -= count;
        for i in 0..remain {
            self.samples[i] = self.samples[i + count];
        }
        for i in remain..remain + count {
            self.samples[i] = 0;
        }
    }

    pub fn read_samples(&mut self, out: &mut [i32], count: usize) -> usize {
        let mut count = count;
        if count > self.available {
            count = self.available;
        }
        if count != 0 {
            let mut sum: i64 = self.integrator;
            for i in 0..count {
                let s = arith_shift(sum, DELTA_BITS);
                sum += self.samples[i] as i64;
                let s = clamp(s);
                out[i] = s as i32;
                sum -= s << (DELTA_BITS - BASS_SHIFT);
            }
            self.integrator = sum;
            self.remove_samples(count);
        }
        count
    }
}

