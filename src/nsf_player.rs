// the nsf player, ui and functionality!!!

use crate::nsf::NsfInfo;
use font8x8::UnicodeFonts;
use std::collections::VecDeque;

const FFT_SIZE: usize = 4096;
const NUM_BARS: usize = 256;
const MAX_BAR_HEIGHT: f32 = 130.0;
const SPECTRUM_BOTTOM_Y: usize = 190;
const SPECTRUM_TOP_Y: usize = SPECTRUM_BOTTOM_Y - MAX_BAR_HEIGHT as usize;

static FREQ_RANGES: [(f64, f64, f64); 8] = [
    (20.0, 150.0, 0.5),
    (150.0, 400.0, 0.5),
    (400.0, 700.0, 0.75),
    (700.0, 1000.0, 0.75),
    (1000.0, 2000.0, 1.0),
    (2000.0, 4000.0, 1.0),
    (4000.0, 6000.0, 1.25),
    (6000.0, 20000.0, 1.25),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NsfPlayerAction {
    NextTrack,
    PrevTrack,
    SelectTrack(u8),
    TogglePause,
    ToggleRepeat,
    ToggleShuffle,
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NsfTheme {
    pub bg_color: u32,
    pub panel_bg: u32,
    pub grid_fg1: u32,
    pub grid_fg2: u32,
    pub border_color: u32,
    pub text_white: u32,
    pub text_muted: u32,
    pub text_dim: u32,
    pub badge_chip: u32,
    pub bar_color_bottom: (u8, u8, u8),
    pub bar_color_mid: (u8, u8, u8),
    pub bar_color_top: (u8, u8, u8),
    pub peak_color: u32,
}

impl Default for NsfTheme {
    fn default() -> Self {
        Self::from_theme_name("dark")
    }
}

impl NsfTheme {
    pub fn from_theme_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "light" => Self {
                bg_color: 0xFFDCE0E8,
                panel_bg: 0xFFECEFF6,
                grid_fg1: 0xFFBAC3D4,
                grid_fg2: 0xFFCAD3E2,
                border_color: 0xFF0077CC,
                text_white: 0xFF181C26,
                text_muted: 0xFF465064,
                text_dim: 0xFF7A879C,
                badge_chip: 0xFFD04400,
                bar_color_bottom: (0, 119, 204),
                bar_color_mid: (0, 180, 160),
                bar_color_top: (220, 60, 40),
                peak_color: 0xFF004488,
            },
            "classicnes" => Self {
                bg_color: 0xFF161618,
                panel_bg: 0xFF202024,
                grid_fg1: 0xFF3A3A42,
                grid_fg2: 0xFF2A2A30,
                border_color: 0xFFFF3333,
                text_white: 0xFFFFFFFF,
                text_muted: 0xFFD0D0D4,
                text_dim: 0xFF7D7D88,
                badge_chip: 0xFFFF5555,
                bar_color_bottom: (140, 20, 20),
                bar_color_mid: (240, 40, 40),
                bar_color_top: (255, 200, 50),
                peak_color: 0xFFFFFFFF,
            },
            "famicom" => Self {
                bg_color: 0xFF180E10,
                panel_bg: 0xFF241418,
                grid_fg1: 0xFF442229,
                grid_fg2: 0xFF32181D,
                border_color: 0xFFD4A020,
                text_white: 0xFFFFF5E0,
                text_muted: 0xFFDFCA9B,
                text_dim: 0xFF8A6565,
                badge_chip: 0xFFCC2222,
                bar_color_bottom: (120, 20, 30),
                bar_color_mid: (204, 34, 34),
                bar_color_top: (235, 185, 40),
                peak_color: 0xFFFFF5E0,
            },
            "mario" => Self {
                bg_color: 0xFF0E1630,
                panel_bg: 0xFF152044,
                grid_fg1: 0xFF263870,
                grid_fg2: 0xFF1C2A56,
                border_color: 0xFFFFDD00,
                text_white: 0xFFFFFFFF,
                text_muted: 0xFF90BAFF,
                text_dim: 0xFF5875A8,
                badge_chip: 0xFFFF3333,
                bar_color_bottom: (30, 80, 200),
                bar_color_mid: (240, 40, 40),
                bar_color_top: (255, 225, 0),
                peak_color: 0xFFFFFFFF,
            },
            "link" => Self {
                bg_color: 0xFF0B1C0E,
                panel_bg: 0xFF122916,
                grid_fg1: 0xFF224828,
                grid_fg2: 0xFF18351E,
                border_color: 0xFFFFAA00,
                text_white: 0xFFF0FFF0,
                text_muted: 0xFF8CD898,
                text_dim: 0xFF4E7B56,
                badge_chip: 0xFF30C050,
                bar_color_bottom: (20, 100, 40),
                bar_color_mid: (45, 190, 70),
                bar_color_top: (255, 185, 0),
                peak_color: 0xFFFFFFFF,
            },
            "metroid" => Self {
                bg_color: 0xFF140707,
                panel_bg: 0xFF1E0D0E,
                grid_fg1: 0xFF3E1C1C,
                grid_fg2: 0xFF2C1314,
                border_color: 0xFFFF6600,
                text_white: 0xFFFFF0EB,
                text_muted: 0xFFFF9C70,
                text_dim: 0xFF8C4B3D,
                badge_chip: 0xFF00FF88,
                bar_color_bottom: (80, 15, 60),
                bar_color_mid: (230, 50, 15),
                bar_color_top: (255, 130, 0),
                peak_color: 0xFF00FF88,
            },
            "megaman" => Self {
                bg_color: 0xFF060F24,
                panel_bg: 0xFF0B193C,
                grid_fg1: 0xFF163270,
                grid_fg2: 0xFF102452,
                border_color: 0xFF00E5FF,
                text_white: 0xFFFFFFFF,
                text_muted: 0xFF70C8FF,
                text_dim: 0xFF3A6AA6,
                badge_chip: 0xFFFFDD00,
                bar_color_bottom: (0, 60, 180),
                bar_color_mid: (0, 180, 255),
                bar_color_top: (255, 230, 0),
                peak_color: 0xFFFFFFFF,
            },
            _ => Self {
                bg_color: 0xFF10121A,
                panel_bg: 0xFF161922,
                grid_fg1: 0xFF2A2E3D,
                grid_fg2: 0xFF20232E,
                border_color: 0xFF00CCFF,
                text_white: 0xFFFFFFFF,
                text_muted: 0xFFA0AAB8,
                text_dim: 0xFF606875,
                badge_chip: 0xFFFFCC00,
                bar_color_bottom: (0, 204, 255),
                bar_color_mid: (0, 230, 118),
                bar_color_top: (255, 68, 68),
                peak_color: 0xFFFFFFFF,
            },
        }
    }

    pub fn bar_color(&self, col_ratio: f32) -> u32 {
        let (r, g, b) = if col_ratio < 0.5 {
            let t = (col_ratio * 2.0).clamp(0.0, 1.0);
            (
                (self.bar_color_bottom.0 as f32 + t * (self.bar_color_mid.0 as f32 - self.bar_color_bottom.0 as f32)) as u32,
                (self.bar_color_bottom.1 as f32 + t * (self.bar_color_mid.1 as f32 - self.bar_color_bottom.1 as f32)) as u32,
                (self.bar_color_bottom.2 as f32 + t * (self.bar_color_mid.2 as f32 - self.bar_color_bottom.2 as f32)) as u32,
            )
        } else {
            let t = ((col_ratio - 0.5) * 2.0).clamp(0.0, 1.0);
            (
                (self.bar_color_mid.0 as f32 + t * (self.bar_color_top.0 as f32 - self.bar_color_mid.0 as f32)) as u32,
                (self.bar_color_mid.1 as f32 + t * (self.bar_color_top.1 as f32 - self.bar_color_mid.1 as f32)) as u32,
                (self.bar_color_mid.2 as f32 + t * (self.bar_color_top.2 as f32 - self.bar_color_mid.2 as f32)) as u32,
            )
        };
        0xFF000000 | (r << 16) | (g << 8) | b
    }
}

pub struct NsfPlayer {
    pub info: NsfInfo,
    pub current_track: u8,
    pub is_paused: bool,
    pub repeat: bool,
    pub shuffle: bool,

    sample_rate: u32,
    sample_buffer: VecDeque<f32>,
    hann_window: [f32; FFT_SIZE],
    bar_heights: [f32; NUM_BARS],
    peak_heights: [f32; NUM_BARS],

    pub track_frame_counter: u64,
    pub total_frame_counter: u64,
    pub last_audible_frame: u64,
    pub change_track_pending: bool,
    pub current_volume: f32,
    pub theme: NsfTheme,
}

impl NsfPlayer {
    pub fn new(info: NsfInfo) -> Self {
        let starting = info.starting_song.saturating_sub(1);
        let mut hann = [0.0f32; FFT_SIZE];
        let pi2 = 2.0 * std::f64::consts::PI;
        for i in 0..FFT_SIZE {
            hann[i] = (0.5 * (1.0 - (pi2 * i as f64 / (FFT_SIZE - 1) as f64).cos())) as f32;
        }

        Self {
            info,
            current_track: starting,
            is_paused: false,
            repeat: false,
            shuffle: false,
            sample_rate: 48000,
            sample_buffer: VecDeque::with_capacity(FFT_SIZE * 2),
            hann_window: hann,
            bar_heights: [0.0; NUM_BARS],
            peak_heights: [0.0; NUM_BARS],
            track_frame_counter: 0,
            total_frame_counter: 0,
            last_audible_frame: 0,
            change_track_pending: false,
            current_volume: 1.0,
            theme: NsfTheme::default(),
        }
    }

    pub fn set_sample_rate(&mut self, rate: u32) {
        if rate > 0 {
            self.sample_rate = rate;
        }
    }

    pub fn set_theme(&mut self, theme_name: &str) {
        self.theme = NsfTheme::from_theme_name(theme_name);
    }

    pub fn push_audio_samples(&mut self, samples: &[f32]) {
        for &s in samples {
            self.sample_buffer.push_back(s);
        }
        while self.sample_buffer.len() > FFT_SIZE {
            self.sample_buffer.pop_front();
        }
    }

    pub fn current_position_seconds(&self, fps: f64) -> f64 {
        if fps > 0.0 {
            self.track_frame_counter as f64 / fps
        } else {
            0.0
        }
    }

    pub fn track_duration_seconds(&self) -> Option<f64> {
        let idx = self.current_track as usize;
        if let Some(&ms) = self.info.track_length_ms.get(idx) {
            if ms > 0 {
                let fade = self.info.track_fade_ms.get(idx).copied().unwrap_or(0).max(0);
                return Some((ms + fade) as f64 / 1000.0);
            }
        }
        None
    }

    pub fn track_fade_seconds(&self) -> f64 {
        let idx = self.current_track as usize;
        if let Some(&ms) = self.info.track_fade_ms.get(idx) {
            if ms > 0 {
                return ms as f64 / 1000.0;
            }
        }
        0.0
    }

    pub fn handle_action(&mut self, action: NsfPlayerAction) -> Option<u8> {
        match action {
            NsfPlayerAction::NextTrack => {
                let next = if self.shuffle && self.info.total_songs > 1 {
                    let mut rand = (self.total_frame_counter % (self.info.total_songs as u64 - 1)) as u8;
                    if rand >= self.current_track {
                        rand = (rand + 1) % self.info.total_songs;
                    }
                    rand
                } else {
                    (self.current_track + 1) % self.info.total_songs
                };
                self.select_track(next);
                Some(next)
            }
            NsfPlayerAction::PrevTrack => {
                let prev = if self.current_position_seconds(60.0) < 2.0 {
                    if self.current_track == 0 {
                        self.info.total_songs.saturating_sub(1)
                    } else {
                        self.current_track - 1
                    }
                } else {
                    self.current_track
                };
                self.select_track(prev);
                Some(prev)
            }
            NsfPlayerAction::SelectTrack(t) => {
                let t = t % self.info.total_songs;
                self.select_track(t);
                Some(t)
            }
            NsfPlayerAction::TogglePause => {
                self.is_paused = !self.is_paused;
                None
            }
            NsfPlayerAction::ToggleRepeat => {
                self.repeat = !self.repeat;
                None
            }
            NsfPlayerAction::ToggleShuffle => {
                self.shuffle = !self.shuffle;
                None
            }
        }
    }

    pub fn select_track(&mut self, track: u8) {
        self.current_track = track.clamp(0, self.info.total_songs.saturating_sub(1));
        self.track_frame_counter = 0;
        self.last_audible_frame = self.total_frame_counter;
        self.change_track_pending = false;
        self.current_volume = 1.0;
    }

    pub fn update_frame(&mut self, fps: f64) -> Option<NsfPlayerAction> {
        self.total_frame_counter = self.total_frame_counter.wrapping_add(1);
        if !self.is_paused {
            self.track_frame_counter = self.track_frame_counter.wrapping_add(1);
        }

        let pos = self.current_position_seconds(fps);
        if pos <= 1.0 {
            self.change_track_pending = false;
        }

        if let Some(duration) = self.track_duration_seconds() {
            let fade = self.track_fade_seconds();
            if pos >= duration {
                if !self.change_track_pending {
                    self.change_track_pending = true;
                    return Some(if self.repeat {
                        NsfPlayerAction::SelectTrack(self.current_track)
                    } else {
                        NsfPlayerAction::NextTrack
                    });
                }
            } else if fade > 0.0 && pos >= duration - fade {
                let fade_pos = pos - (duration - fade);
                self.current_volume = (1.0 - (fade_pos / fade)).clamp(0.0, 1.0) as f32;
            } else {
                self.current_volume = 1.0;
            }
        } else {
            self.current_volume = 1.0;
        }

        let mut silent = true;
        if self.sample_buffer.len() >= FFT_SIZE {
            let mut re = [0.0f32; FFT_SIZE];
            let mut im = [0.0f32; FFT_SIZE];

            for i in 0..FFT_SIZE {
                re[i] = self.sample_buffer[i] * self.hann_window[i];
                im[i] = 0.0;
            }

            fft_radix2(&mut re, &mut im);

            let bin_hz = self.sample_rate as f64 / FFT_SIZE as f64;
            let half = FFT_SIZE / 2;

            for i in 0..8 {
                let (f_start, f_end, weight) = FREQ_RANGES[i];
                let f_range = f_end - f_start;
                for j in 0..32 {
                    let col = i * 32 + j;
                    let start_f = f_start + f_range * (j as f64) / 32.0;
                    let end_f = f_start + f_range * ((j + 1) as f64) / 32.0;

                    let b_start = ((start_f / bin_hz) as usize).clamp(1, half - 1);
                    let b_end = ((end_f / bin_hz) as usize).clamp(b_start, half - 1);

                    let mut sum_amp = 0.0f64;
                    for b in b_start..=b_end {
                        let mag = ((re[b] * re[b] + im[b] * im[b]) as f64).sqrt();
                        sum_amp += mag;
                    }
                    let count = (b_end - b_start + 1) as f64;
                    let avg = (sum_amp / count) * weight * 1.8;
                    let target_h = (avg as f32).min(MAX_BAR_HEIGHT);

                    if target_h >= 1.5 {
                        silent = false;
                    }

                    if target_h >= self.bar_heights[col] {
                        self.bar_heights[col] = target_h;
                    } else {
                        self.bar_heights[col] = (self.bar_heights[col] - 2.5).max(target_h).max(0.0);
                    }

                    if target_h >= self.peak_heights[col] {
                        self.peak_heights[col] = target_h;
                    } else {
                        self.peak_heights[col] = (self.peak_heights[col] - 0.7).max(0.0);
                    }
                }
            }
        } else {
            for col in 0..NUM_BARS {
                self.bar_heights[col] = (self.bar_heights[col] - 3.0).max(0.0);
                self.peak_heights[col] = (self.peak_heights[col] - 1.0).max(0.0);
            }
        }

        if !silent {
            self.last_audible_frame = self.total_frame_counter;
        } else {
            let silence_frames = self.total_frame_counter.saturating_sub(self.last_audible_frame);
            let silence_secs = silence_frames as f64 / fps.max(1.0);
            if pos > 2.0 && silence_secs >= 3.5 && !self.change_track_pending {
                self.change_track_pending = true;
                return Some(if self.repeat {
                    NsfPlayerAction::SelectTrack(self.current_track)
                } else {
                    NsfPlayerAction::NextTrack
                });
            }
        }

        None
    }

    pub fn draw_hud(&self, buffer: &mut [u32], fps: f64) {
        if buffer.len() < 256 * 240 {
            return;
        }

        let theme = self.theme;

        for p in buffer.iter_mut().take(256 * 240) {
            *p = theme.bg_color;
        }

        draw_rect_fast(buffer, 0, 0, 256, 2, theme.border_color);

        let y_top = 5;
        let format_badge = if self.info.is_nsfe { "[NSFe]" } else { "[NSF]" };
        draw_text_fast(buffer, 8, y_top, format_badge, theme.border_color);

        let track_badge = format!("TRK {:02}/{:02}", self.current_track + 1, self.info.total_songs);
        let tb_x = 256 - track_badge.len() * 8 - 8;
        draw_text_fast(buffer, tb_x, y_top, &track_badge, theme.text_white);

        let mut bx = 54;
        for chip in self.info.sound_chip_names() {
            let badge = format!("[{}]", chip);
            if bx + badge.len() * 8 > tb_x.saturating_sub(4) {
                break;
            }
            draw_text_fast(buffer, bx, y_top, &badge, theme.badge_chip);
            bx += badge.len() * 8 + 4;
        }

        let label_x = 8;
        let val_x = 74;
        let max_val_len = (256 - val_x - 8) / 8;

        let mut y = 17;
        let title = if !self.info.song_name.is_empty() {
            &self.info.song_name
        } else {
            "Unknown Title"
        };
        draw_text_fast(buffer, label_x, y, "Game:", theme.text_dim);
        draw_text_fast(buffer, val_x, y, &truncate_str(title, max_val_len), theme.text_white);

        y += 10;
        let artist = if !self.info.artist_name.is_empty() {
            &self.info.artist_name
        } else {
            "Unknown Artist"
        };
        draw_text_fast(buffer, label_x, y, "Artist:", theme.text_dim);
        draw_text_fast(buffer, val_x, y, &truncate_str(artist, max_val_len), theme.text_muted);

        y += 10;
        let mut comment = self.info.copyright_holder.clone();
        if !self.info.ripper_name.is_empty() {
            if !comment.is_empty() { comment.push(' '); }
            comment.push_str(&self.info.ripper_name);
        }
        if !comment.is_empty() {
            draw_text_fast(buffer, label_x, y, "Extra:", theme.text_dim);
            draw_text_fast(buffer, val_x, y, &truncate_str(&comment, max_val_len), theme.text_dim);
        }

        let spec_top = SPECTRUM_TOP_Y;
        let spec_bottom = SPECTRUM_BOTTOM_Y;
        let spec_h = spec_bottom - spec_top;

        fill_rect(buffer, 0, spec_top, 256, spec_h, theme.panel_bg);
        draw_rect_fast(buffer, 0, spec_top - 1, 256, 1, theme.grid_fg1);
        draw_rect_fast(buffer, 0, spec_bottom + 1, 256, 1, theme.grid_fg1);

        for i in 1..8 {
            let x = i * 32;
            draw_line_v(buffer, x, spec_top, spec_bottom, theme.grid_fg1);
        }
        for i in 0..8 {
            let x = i * 32 + 16;
            draw_line_v(buffer, x, spec_top, spec_bottom, theme.grid_fg2);
        }

        draw_text_fast(buffer, 2, spec_top + 4, "20Hz", theme.grid_fg1);
        draw_text_fast(buffer, 24, spec_top + 18, "150", theme.grid_fg2);
        draw_text_fast(buffer, 54, spec_top + 4, "400", theme.grid_fg1);
        draw_text_fast(buffer, 86, spec_top + 18, "700", theme.grid_fg2);
        draw_text_fast(buffer, 118, spec_top + 4, "1k", theme.grid_fg1);
        draw_text_fast(buffer, 150, spec_top + 18, "2k", theme.grid_fg2);
        draw_text_fast(buffer, 182, spec_top + 4, "4k", theme.grid_fg1);
        draw_text_fast(buffer, 214, spec_top + 18, "6k", theme.grid_fg2);
        draw_text_fast(buffer, 230, spec_top + 4, "20k", theme.grid_fg1);

        for x in 0..NUM_BARS {
            let h = self.bar_heights[x].round() as usize;
            if h > 0 {
                let bar_top = spec_bottom.saturating_sub(h);
                let col_ratio = h as f32 / MAX_BAR_HEIGHT;
                let bar_color = theme.bar_color(col_ratio);

                for py in bar_top..spec_bottom {
                    buffer[py * 256 + x] = bar_color;
                }
            }

            let peak_h = self.peak_heights[x].round() as usize;
            if peak_h > 1 {
                let py = spec_bottom.saturating_sub(peak_h);
                if py >= spec_top && py < spec_bottom {
                    buffer[py * 256 + x] = theme.peak_color;
                }
            }
        }

        let pos_sec = self.current_position_seconds(fps);
        let duration_opt = self.track_duration_seconds();

        let bar_x = 12;
        let bar_w = 232;
        let bar_y = 196;
        fill_rect(buffer, bar_x, bar_y, bar_w, 4, theme.grid_fg2);
        draw_rect_fast(buffer, bar_x - 1, bar_y - 1, bar_w + 2, 6, theme.grid_fg1);

        if let Some(dur) = duration_opt {
            let ratio = (pos_sec / dur).clamp(0.0, 1.0) as f32;
            let fill_w = (bar_w as f32 * ratio).round() as usize;
            fill_rect(buffer, bar_x, bar_y, fill_w, 4, theme.border_color);
        } else {
            let pulse_x = bar_x + ((self.track_frame_counter % 120) as f32 / 120.0 * (bar_w - 24) as f32) as usize;
            fill_rect(buffer, pulse_x, bar_y, 24, 4, theme.border_color);
        }

        let cur_time_str = format_time(pos_sec);
        let time_str = if let Some(dur) = duration_opt {
            format!("{} / {}", cur_time_str, format_time(dur))
        } else {
            cur_time_str
        };

        let time_w = time_str.len() * 8;
        let time_x = 256 - time_w - 10;

        let track_name = if let Some(name) = self.info.track_names.get(self.current_track as usize) {
            if !name.is_empty() { name } else { &self.info.song_name }
        } else {
            &self.info.song_name
        };

        draw_text_fast(buffer, 12, 206, "Name:", theme.text_dim);
        let name_val_x = 12 + 5 * 8 + 4;
        let max_name_len = time_x.saturating_sub(name_val_x + 6) / 8;
        let t_name_display = truncate_str(track_name, max_name_len);
        draw_text_fast(buffer, name_val_x, 206, &t_name_display, theme.text_white);

        draw_text_fast(buffer, time_x, 206, &time_str, theme.border_color);

        let bot_y = 222;
        let state_str = if self.is_paused { "[PAUSED]" } else { "[PLAY]" };
        let state_color = if self.is_paused { 0xFFFF4444 } else { 0xFF00E676 };
        draw_text_fast(buffer, 10, bot_y, state_str, state_color);

        if self.repeat {
            draw_text_fast(buffer, 76, bot_y, "[REP]", theme.border_color);
        } else {
            draw_text_fast(buffer, 76, bot_y, "[---]", theme.grid_fg1);
        }

        if self.shuffle {
            draw_text_fast(buffer, 122, bot_y, "[SHF]", theme.border_color);
        } else {
            draw_text_fast(buffer, 122, bot_y, "[---]", theme.grid_fg1);
        }

        draw_text_fast(buffer, 168, bot_y, "<PRV NXT>", theme.text_muted);
    }
}

fn fft_radix2(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0;
    for i in 0..n - 1 {
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
        let mut k = n / 2;
        while k <= j {
            j -= k;
            k /= 2;
        }
        j += k;
    }

    let mut l = 2;
    while l <= n {
        let half = l / 2;
        let angle = -2.0 * std::f64::consts::PI / l as f64;
        let w_step_re = angle.cos() as f32;
        let w_step_im = angle.sin() as f32;

        let mut i = 0;
        while i < n {
            let mut w_re = 1.0f32;
            let mut w_im = 0.0f32;
            for m in 0..half {
                let u_re = re[i + m];
                let u_im = im[i + m];
                let v_re = re[i + m + half] * w_re - im[i + m + half] * w_im;
                let v_im = re[i + m + half] * w_im + im[i + m + half] * w_re;

                re[i + m] = u_re + v_re;
                im[i + m] = u_im + v_im;
                re[i + m + half] = u_re - v_re;
                im[i + m + half] = u_im - v_im;

                let next_w_re = w_re * w_step_re - w_im * w_step_im;
                let next_w_im = w_re * w_step_im + w_im * w_step_re;
                w_re = next_w_re;
                w_im = next_w_im;
            }
            i += l;
        }
        l *= 2;
    }
}

fn format_time(seconds: f64) -> String {
    let s = seconds.max(0.0) as u32;
    let mins = s / 60;
    let secs = s % 60;
    format!("{:02}:{:02}", mins, secs)
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else if max_chars <= 2 {
        s.chars().take(max_chars).collect()
    } else {
        let mut out: String = s.chars().take(max_chars.saturating_sub(2)).collect();
        out.push_str("..");
        out
    }
}

fn fill_rect(buffer: &mut [u32], x: usize, y: usize, w: usize, h: usize, color: u32) {
    for py in y..(y + h).min(240) {
        let row = py * 256;
        for px in x..(x + w).min(256) {
            buffer[row + px] = color;
        }
    }
}

fn draw_rect_fast(buffer: &mut [u32], x: usize, y: usize, w: usize, h: usize, color: u32) {
    fill_rect(buffer, x, y, w, h, color);
}

fn draw_line_v(buffer: &mut [u32], x: usize, y1: usize, y2: usize, color: u32) {
    if x < 256 {
        for y in y1..=y2.min(239) {
            buffer[y * 256 + x] = color;
        }
    }
}

fn draw_text_fast(buffer: &mut [u32], x: usize, y: usize, text: &str, color: u32) {
    for (i, c) in text.chars().enumerate() {
        let cx = x + i * 8;
        if cx + 8 > 256 || y + 8 > 240 {
            continue;
        }
        if let Some(glyph) = font8x8::BASIC_FONTS.get(c) {
            for gy in 0..8 {
                let row = glyph[gy];
                if row == 0 { continue; }
                let py = y + gy;
                let row_offset = py * 256;
                for gx in 0..8 {
                    if (row >> gx) & 1 != 0 {
                        buffer[row_offset + cx + gx] = color;
                    }
                }
            }
        }
    }
}
