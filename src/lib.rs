// reorganized into libraries for web support!!!

pub const APP_VERSION: &str = "1.7.8";

pub mod apu;
pub mod audio;
pub mod blip;
pub mod bus;
pub mod cartridge;
pub mod cheats;
pub mod config;
pub mod cpu;
pub mod crc;
pub mod emulator;
pub mod filters_hqx;
pub mod filters_ntsc_bisqwit;
pub mod filters_ntsc_blargg;
pub mod filters_pal;
pub mod filters_sai;
pub mod filters_scale;
pub mod filters_xbrz;
pub mod fm2;
pub mod mapper;
pub mod mappers;
pub mod movie;
pub mod nesdb;
pub mod nsf;
pub mod nsf_player;
pub mod ppu;
pub mod ps2_device_port;
pub mod ra;
pub mod region;
pub mod status_viewer {
    pub use crate::crc::md5_hex;
}
pub mod studybox;
pub mod vt03_palette;
pub mod vt32_palette;
pub mod wavreader;
pub mod wxn;

#[cfg(target_arch = "wasm32")]
pub mod wasm;
