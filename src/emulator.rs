// the heart of this nes emulator, handles basically every component interacting with the other and itself

use crate::cartridge::Cartridge;
use crate::config;
use crate::region::{Region, TvSystem};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

#[derive(Clone, Copy, Default)]
pub struct Vt369SpriteEntry {
    pub data: [u8; 16],
    pub start_x: i16,
    pub priority: bool,
    pub sprite0: bool,
}

#[derive(Clone, Copy)]
pub struct Vt369SpriteHiEntry {
    pub data_even: [u8; 64],
    pub data_odd: [u8; 64],
    pub start_x: i16,
    pub mask_x: i16,
    pub sprite0: bool,
}

impl Default for Vt369SpriteHiEntry {
    fn default() -> Self {
        Self {
            data_even: [0u8; 64],
            data_odd: [0u8; 64],
            start_x: 0,
            mask_x: 0,
            sprite0: false,
        }
    }
}

// so of course it'd only make sense for this to start with a massive struct with like thousands of public variable declarations!!

pub struct Emulator {
    pub cart: Option<Cartridge>,

    pub ppu_clock: u8,
    pub cpu_clock: u8,

    pub program_counter: u16,
    pub stack_pointer: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub h: u8,
    pub ignore_h: bool,
    pub data_bus: u8,
    pub internal_bus: u8,
    pub address_bus: u16,
    pub special_bus: u8,
    pub dl: u8,
    pub op_code: u8,
    pub operation_cycle: u8,
    pub temporary_address: u16,
    pub total_cycles: u64,

    pub flag_carry: bool,
    pub flag_zero: bool,
    pub flag_interrupt: bool,
    pub flag_decimal: bool,
    pub flag_overflow: bool,
    pub flag_negative: bool,

    pub cpu_read: bool,
    pub do_brk: bool,
    pub do_nmi: bool,
    pub do_irq: bool,
    pub do_reset: bool,
    pub fix_high_byte: bool,

    pub do_oam_dma: bool,
    pub first_cycle_of_oam_dma: bool,
    pub do_dmc_dma: bool,
    pub dmc_dma_delay: u8,
    pub cannot_run_dmc_dma_right_now: u8,
    pub dma_page: u8,
    pub dma_address: u8,
    pub oam_dma_aligned: bool,
    pub oam_dma_halt: bool,
    pub dmc_dma_halt: bool,
    pub oam_internal_bus: u8,

    pub nmi_pins_signal: bool,
    pub nmi_previous_pins_signal: bool,
    pub irq_level_detector: bool,
    pub nmi_line: bool,
    pub irq_line: bool,

    pub ram: [u8; 0x2000],
    pub cpu_ram_mask: u16,
    pub um6578_extra_ram: [u8; 0x800],
    pub um6578_vram: [[u8; 0x400]; 0xA],
    pub um6578_chr_ram: [u8; 0x8000],
    pub um6578_reg2008: u8,
    pub um6578_color_mask: u8,
    pub um6578_dma_control: u8,
    pub um6578_dma_page: u8,
    pub um6578_dma_source: u16,
    pub um6578_dma_target: u16,
    pub um6578_dma_length: u16,
    pub um6578_dma_busy: u32,
    pub um6578_nt_tile_byte: u8,
    pub um6578_nt_attr_byte: u8,
    pub um6578_bg_palette: u8,
    pub um6578_bg_palette_lo: u8,
    pub vram: [u8; 0x800],
    pub oam: [u8; 0x100],
    pub oam2: [u8; 32],
    pub palette_ram: [u8; 0x400],

    pub ppu_bus: u8,
    pub ppu_bus_decay: [i32; 8],
    pub ppu_oam_address: u8,
    pub ppu_status_vblank: bool,
    pub ppu_status_sprite_zero_hit: bool,
    pub ppu_status_sprite_zero_hit_delayed: bool,
    pub ppu_status_sprite_overflow: bool,
    pub ppu_status_sprite_overflow_delayed: bool,
    pub ppu_status_pending_sprite_zero_hit: bool,
    pub ppu_status_pending_sprite_zero_hit2: bool,
    pub ppu_pending_vblank: bool,
    pub ppu_vset: bool,
    pub ppu_vset_latch1: bool,
    pub ppu_vset_latch2: bool,
    pub ppu_read_2002: bool,

    pub ppu_v: u16,
    pub ppu_t: u16,
    pub ppu_fine_x_scroll: u8,
    pub ppu_addr_latch: bool,
    pub ppu_control_increment_mode_32: bool,
    pub ppu_control_nmi_enabled: bool,
    pub ppu_sprite_x16: bool,
    pub ppu_pattern_select_sprites: bool,
    pub ppu_pattern_select_background: bool,

    pub ppu_scanline: u16,
    pub ppu_dot: u16,
    pub ppu_odd_frame: bool,
    pub ppu_address_bus: u16,
    pub ppu_ale: bool,
    pub ppu_octal_latch: u8,
    pub ppu_read_buffer: u8,
    pub ppu_reset: bool,

    pub ppu_mask_greyscale: bool,
    pub ppu_mask_8px_show_background: bool,
    pub ppu_mask_8px_show_sprites: bool,
    pub ppu_mask_show_background: bool,
    pub ppu_mask_show_sprites: bool,
    pub ppu_mask_emphasize_red: bool,
    pub ppu_mask_emphasize_green: bool,
    pub ppu_mask_emphasize_blue: bool,
    pub ppu_mask_show_background_instant: bool,
    pub ppu_mask_show_sprites_instant: bool,
    pub ppu_mask_show_background_delayed: bool,
    pub ppu_mask_show_sprites_delayed: bool,

    pub ppu_bg_pattern_sr_l: u16,
    pub ppu_bg_pattern_sr_h: u16,
    pub ppu_bg_pattern_sr_l2: u16,
    pub ppu_bg_pattern_sr_h2: u16,
    pub ppu_bg_attr_sr_l: u16,
    pub ppu_bg_attr_sr_h: u16,
    pub ppu_attr_latch_register: u8,
    pub ppu_low_bit_plane: u8,
    pub ppu_high_bit_plane: u8,
    pub ppu_low_bit_plane_hi: u8,
    pub ppu_high_bit_plane_hi: u8,
    pub ppu_attribute: u8,
    pub ppu_onebus_eva: u16,
    pub ppu_onebus_chr: crate::mappers::one_bus::OneBusChrCtx,
    pub vt369_tile_data: [u8; 272],
    pub vt369_tile_data_even: [u8; 272 * 2],
    pub vt369_tile_data_odd: [u8; 272 * 2],
    pub vt369_pat_addr: usize,
    pub vt369_nt_byte: u8,
    pub vt369_sprite_buf: [Vt369SpriteEntry; 64],
    pub vt369_sprite_buf_hi: [Vt369SpriteHiEntry; 64],
    pub vt369_sprite_count: u8,
    pub vt369_sprite_count_hi: u8,
    pub vt369_sprite_ram: [u8; 512],
    pub vt369_spr_addr_high: u8,
    pub vt369_dma_target_addr: u16,

    pub ppu_sprite_sr_l: [u8; 8],
    pub ppu_sprite_sr_h: [u8; 8],
    pub ppu_sprite_sr_l2: [u8; 8],
    pub ppu_sprite_sr_h2: [u8; 8],
    pub ppu_sprite_attribute: [u8; 8],
    pub ppu_sprite_pattern: [u8; 8],
    pub ppu_sprite_x_position: [u8; 8],
    pub ppu_sprite_y_position: [u8; 8],
    pub ppu_sprite_shifter_counter: [u8; 8],
    pub ppu_sprite_pattern_l: u8,
    pub ppu_sprite_pattern_h: u8,
    pub ppu_sprite_pattern_l2: u8,
    pub ppu_sprite_pattern_h2: u8,
    pub ppu_next_scanline_contains_sprite_zero: bool,
    pub ppu_current_scanline_contains_sprite_zero: bool,
    pub ppu_can_detect_sprite_zero_hit: bool,

    pub vs_ppu_variant: u8,

    pub oam2_address: u8,
    pub secondary_oam_full: bool,
    pub oam2_reset_signal: u8,
    pub sprite_evaluation_tick: u8,
    pub oam_address_overflowed_during_sprite_evaluation: bool,
    pub ppu_oam_latch: u8,
    pub ppu_oam_buffer: u8,
    pub ppu_oam_read_latch: u8,
    pub ppu_render_temp: u8,
    pub in_range_check: u16,
    pub nine_objects_on_this_scanline: bool,
    pub ppu_oam_corruption_rendering_disabled_out_of_vblank: bool,
    pub ppu_oam_corruption_rendering_disabled_out_of_vblank_instant: bool,

    pub ppu_v_register_changed_out_of_vblank: bool,
    pub ppu_pending_oam_corruption: bool,
    pub ppu_oam_corruption_index: u8,
    pub ppu_oam_corruption_rendering_enabled_out_of_vblank: bool,
    pub ppu_oam_evaluation_corruption_odd_cycle: bool,
    pub ppu_oam_evaluation_object_in_range: bool,
    pub ppu_oam_evaluation_object_in_x_range: bool,
    pub ppu_palette_corruption_rendering_disabled_out_of_vblank: bool,
    pub oam_corrupted_on_odd_cycle: bool,

    pub ppu_update_2006_delay: u8,
    pub ppu_update_2005_delay: u8,
    pub ppu_update_2001_delay: u8,
    pub ppu_update_2001_oam_corruption_delay: u8,
    pub ppu_update_2001_emphasis_bits_delay: u8,
    pub ppu_update_2005_value: u8,
    pub ppu_update_2001_value: u8,
    pub ppu_update_2006_value: u16,
    pub ppu_update_2006_value_temp: u16,
    pub ppu_was_rendering_before_2001_write: bool,

    pub ppu_2007_read: bool, pub ppu_2007_read_sr: bool,
    pub ppu_2007_read_latches: [bool; 5],
    pub ppu_2007_pd_rb: bool, pub ppu_2007_read_ale: bool,
    pub ppu_2007_read_h0_latch: bool, pub ppu_2007_read_xrb: bool,
    pub ppu_read: bool,
    pub ppu_2007_write: bool, pub ppu_2007_write_sr: bool,
    pub ppu_2007_write_latches: [bool; 5],
    pub ppu_2007_db_par: bool,
    pub ppu_2007_write_ale: bool,
    pub ppu_2007_tstep_latch: bool,
    pub ppu_2007_tstep: bool,
    pub ppu_2007_blnk_latch: bool,
    pub ppu_2007_palette_ram_enable: bool,
    pub ppu_2007_write_data: u8,
    pub ppu_write: bool,
    pub ppu_pattern_address_register_nt: u16,
    pub ppu_pattern_address_register_at: u16,
    pub ppu_pattern_address_register_chr: u16,
    pub ppu_commit_nametable_fetch: bool,
    pub ppu_commit_attribute_fetch: bool,
    pub ppu_commit_pattern_low_fetch: bool,
    pub ppu_commit_pattern_high_fetch: bool,

    pub ppu_a12_prev: bool,
    pub copy_v: bool,
    pub skipped_pre_render_dot_341: bool,

    pub dot_color: u8,
    pub prev_dot_color: u8,
    pub prev_prev_dot_color: u8,
    pub prev_prev_prev_dot_color: u8,
    pub dot_color_rgb: u32,
    pub prev_dot_color_rgb: u32,
    pub prev_prev_dot_color_rgb: u32,
    pub prev_prev_prev_dot_color_rgb: u32,
    pub palette_ram_address: u8,
    pub this_dot_read_from_palette_ram: bool,

    pub apu_alignment: u8,
    pub apu_put_cycle: bool,
    pub apu_status_dmc_interrupt: bool,
    pub apu_status_frame_interrupt: bool,
    pub apu_status_dmc: bool,
    pub apu_status_delayed_dmc: bool,
    pub apu_status_noise: bool,
    pub apu_status_triangle: bool,
    pub apu_status_pulse2: bool,
    pub apu_status_pulse1: bool,
    pub clearing_apu_frame_interrupt: bool,

    pub apu_delayed_dmc_4015: u8,
    pub apu_implicit_abort_dmc_4015: bool,
    pub apu_set_implicit_abort_dmc_4015: bool,

    pub apu_register: [u8; 0x10],
    pub apu_frame_counter_mode: bool,
    pub apu_frame_counter_inhibit_irq: bool,
    pub apu_frame_counter_reset: u8,
    pub apu_framecounter: u16,
    pub apu_quarter_frame_clock: bool,
    pub apu_half_frame_clock: bool,

    pub apu_envelope_start_flag: bool,
    pub apu_envelope_divider_clock: bool,
    pub apu_envelope_decay_level: u8,

    pub apu_length_counter_pulse1: u8,
    pub apu_length_counter_pulse2: u8,
    pub apu_length_counter_triangle: u8,
    pub apu_length_counter_noise: u8,
    pub apu_length_counter_halt_pulse1: bool,
    pub apu_length_counter_halt_pulse2: bool,
    pub apu_length_counter_halt_triangle: bool,
    pub apu_length_counter_halt_noise: bool,
    pub apu_length_counter_reload_pulse1: bool,
    pub apu_length_counter_reload_pulse2: bool,
    pub apu_length_counter_reload_triangle: bool,
    pub apu_length_counter_reload_noise: bool,
    pub apu_length_counter_reload_value_pulse1: u8,
    pub apu_length_counter_reload_value_pulse2: u8,
    pub apu_length_counter_reload_value_triangle: u8,
    pub apu_length_counter_reload_value_noise: u8,

    pub apu_channel_timer_pulse1: u16,
    pub apu_channel_timer_pulse2: u16,
    pub apu_channel_timer_triangle: u16,
    pub apu_channel_timer_noise: u16,
    pub apu_channel_timer_dmc: u16,

    pub apu_dmc_enable_irq: bool,
    pub apu_dmc_loop: bool,
    pub apu_dmc_rate: u16,
    pub apu_dmc_output: u8,
    pub apu_dmc_sample_address: u16,
    pub apu_dmc_sample_length: u16,
    pub apu_dmc_bytes_remaining: u16,
    pub apu_dmc_buffer: u8,
    pub apu_dmc_address_counter: u16,
    pub apu_dmc_shifter: u8,
    pub apu_dmc_shifter_bits_remaining: u8,
    pub dpcm_up: bool,
    pub apu_silent: bool,

        pub audio_buffer: Option<std::sync::Arc<std::sync::Mutex<crate::audio::AudioRingBuffer>>>,
    pub audio_cycles_accumulator: f64,
    pub audio_sample_accumulator: f32,
    pub audio_sample_count: f32,
    pub audio_host_sample_rate: f64,
    pub blip: Option<crate::blip::BlipBuf>,
    pub audio_frame_cycle: u32,
    pub audio_previous_output: i32,

    pub master_volume: f32,
    pub square1_volume: f32,
    pub square2_volume: f32,
    pub triangle_volume: f32,
    pub noise_volume: f32,
    pub pcm_volume: f32,
    pub expansion_volume: f32,
    pub fds_volume: f32,
    pub mmc5_volume: f32,
    pub vrc6_volume: f32,
    pub vrc7_volume: f32,
    pub n163_volume: f32,
    pub sunsoft5b_volume: f32,
    pub audio_enabled: bool,
    pub audio_depth: u8,
    pub swap_duty_cycles: bool,

    pub filter_lp_alpha: f32,
    pub filter_lp_prev_out: f32,
    pub filter_hp1_alpha: f32,
    pub filter_hp1_prev_in: f32,
    pub filter_hp1_prev_out: f32,
    pub filter_hp2_alpha: f32,
    pub filter_hp2_prev_in: f32,
    pub filter_hp2_prev_out: f32,

    pub pulse1_timer: u16,
    pub pulse1_sequencer_step: u8,
    pub pulse1_envelope_divider: u8,
    pub pulse1_envelope_decay_level: u8,
    pub pulse1_envelope_start_flag: bool,
    pub pulse1_sweep_divider: u8,
    pub pulse1_sweep_reload: bool,

    pub pulse2_timer: u16,
    pub pulse2_sequencer_step: u8,
    pub pulse2_envelope_divider: u8,
    pub pulse2_envelope_decay_level: u8,
    pub pulse2_envelope_start_flag: bool,
    pub pulse2_sweep_divider: u8,
    pub pulse2_sweep_reload: bool,

    pub triangle_timer: u16,
    pub triangle_sequencer_step: u8,
    pub triangle_linear_counter: u8,
    pub triangle_linear_counter_reload_flag: bool,

    pub noise_timer: u16,
    pub noise_shift_register: u16,
    pub noise_envelope_divider: u8,
    pub noise_envelope_decay_level: u8,
    pub noise_envelope_start_flag: bool,


    pub apu_controller_ports_strobing: bool,
    pub apu_controller_ports_strobed: bool,
    pub controller_port1: Arc<AtomicU8>,
    pub controller_port2: Arc<AtomicU8>,
    pub controller_shift_register1: u8,
    pub controller_shift_register2: u8,
    pub controller1_shift_counter: u8,
    pub controller2_shift_counter: u8,
    pub data_pins_are_not_floating: bool,

    pub zapper_x: Arc<Mutex<f32>>,
    pub zapper_y: Arc<Mutex<f32>>,
    pub zapper_trigger: Arc<AtomicBool>,
    pub zapper_bogo: Arc<AtomicU8>,
    pub expansion_zapper_trigger: Arc<AtomicBool>,
    pub oeka_click: Arc<AtomicBool>,
    pub oeka_strobe: bool,
    pub oeka_shift: bool,
    pub oeka_state_buffer: u32,
    pub family_trainer_state: Arc<Mutex<[u8; 12]>>,
    pub family_trainer_ignore_rows: u8,
    pub hyper_shot_state: Arc<Mutex<[u8; 4]>>,
    pub hyper_shot_enable_p1: bool,
    pub hyper_shot_enable_p2: bool,
    pub bandai_hyper_state: [u32; 2],
    pub bandai_hyper_readbit: [u8; 2],
    pub bandai_hyper_buttons: Arc<Mutex<[u8; 9]>>,
    pub turbo_file_data: [u8; 0x2000],
    pub turbo_file_position: u16,
    pub turbo_file_last_write: u8,
    pub battle_box_data: [u8; 0x200],
    pub battle_box_last_write: u8,
    pub battle_box_address: u8,
    pub battle_box_chip_select: u8,
    pub battle_box_output: u8,
    pub battle_box_write_enabled: bool,
    pub battle_box_input_bit_position: u8,
    pub battle_box_input_data: u16,
    pub battle_box_is_write: bool,
    pub battle_box_is_read: bool,
    pub family_basic_state: Arc<Mutex<[u8; 72]>>,
    pub family_basic_row: u8,
    pub family_basic_column: u8,
    pub family_basic_enabled: bool,
    pub party_tap_state: Arc<Mutex<[u8; 6]>>,
    pub party_tap_buffer: u8,
    pub party_tap_read_count: u8,
    pub party_tap_strobe: bool,
    pub pachinko_state: Arc<Mutex<[u8; 10]>>,
    pub pachinko_analog: u8,
    pub pachinko_buffer: u16,
    pub pachinko_strobe: bool,
    pub punching_bag_state: Arc<Mutex<[u8; 8]>>,
    pub punching_bag_selected_sensors: u8,
    pub jissen_mahjong_state: Arc<Mutex<[u8; 21]>>,
    pub jissen_mahjong_row: u8,
    pub jissen_mahjong_state_buffer: u32,
    pub jissen_mahjong_strobe: bool,
    pub subor_keyboard_state: Arc<Mutex<[u8; 104]>>,
    pub subor_keyboard_row: u8,
    pub subor_keyboard_column: u8,
    pub subor_keyboard_enabled: bool,
    pub pec586_keyboard_state: Arc<Mutex<[u8; 104]>>,
    pub pec586_kspos: u8,
    pub pec586_ksindex: u8,
    pub pec586_kstrobe: u8,
    pub bit79_keyboard_state: Arc<Mutex<[u8; 104]>>,
    pub bit79_keyboard_row: u8,
    pub bit79_keyboard_column: u8,
    pub bit79_keyboard_strobe: u8,
    pub keda_keyboard_state: Arc<Mutex<[u8; 104]>>,
    pub keda_keyboard_row: u8,
    pub keda_keyboard_column: u8,
    pub keda_keyboard_strobe: u8,
    pub kingwon_keyboard_state: Arc<Mutex<[u8; 104]>>,
    pub kingwon_keyboard_row: u8,
    pub kingwon_keyboard_column: u8,
    pub kingwon_keyboard_bits: u8,
    pub zecheng_keyboard_bits1: u8,
    pub zecheng_keyboard_bits2: u8,
    pub zecheng_keyboard_strobe: bool,
    pub zecheng_keyboard_buttons: Arc<Mutex<u8>>,
    pub zecheng_keyboard_keys: Arc<Mutex<u8>>,
    pub zecheng_keyboard_dx: Arc<Mutex<i32>>,
    pub zecheng_keyboard_dy: Arc<Mutex<i32>>,
    pub quiz_king_buttons: Arc<Mutex<[u8; 6]>>,
    pub quiz_king_data_r: u8,
    pub quiz_king_funky_mode: bool,
    pub quiz_king_strobe: bool,
    pub top_rider_buttons: Arc<Mutex<[u8; 8]>>,
    pub top_rider_bs: u32,
    pub top_rider_bss: u32,
    pub top_rider_boop: u32,
    pub fami_net_sys_buttons: Arc<Mutex<[u8; 24]>>,
    pub fami_net_sys_data: u32,
    pub fami_net_sys_readbit: u8,
    pub fami_net_sys_prev_strobe: bool,
    pub city_patrolman_input: Arc<Mutex<[u8; 4]>>,
    pub city_patrolman_strobe: u8,
    pub city_patrolman_time_out: u32,
    pub city_patrolman_shift_reg: u32,
    pub moguraa_buttons: Arc<Mutex<[u8; 12]>>,
    pub moguraa_bits: u16,
    pub moguraa_sel: u8,
    pub sharpc1_tape_state: u8,
    pub sharpc1_tape_data: Vec<u8>,
    pub sharpc1_tape_bits: u32,
    pub sharpc1_tape_position: u32,
    pub sharpc1_tape_level: bool,
    pub sharpc1_tape_file_format: u8,
    pub sharpc1_tape_path: Option<std::path::PathBuf>,
    pub golden_nugget_buttons: Arc<Mutex<[u8; 11]>>,
    pub golden_nugget_strobe: u8,
    pub golden_nugget_shift: u8,
    pub abl_pinball_buttons: Arc<Mutex<[u8; 6]>>,
    pub abl_pinball_wheel: Arc<Mutex<i32>>,
    pub abl_pinball_count: i16,
    pub abl_pinball_plunger: i16,
    pub abl_pinball_plunger_reset_count: i32,
    pub tv_pump_buttons: Arc<Mutex<[u8; 6]>>,
    pub triface_mahjong_buttons: Arc<Mutex<[u8; 22]>>,
    pub triface_mahjong_column: u8,
    pub triface_mahjong_row: u8,
    pub triface_mahjong_keys: [u8; 10],
    pub mahjong_gekitou_buttons: Arc<Mutex<[u8; 22]>>,
    pub mahjong_gekitou_bits: u8,
    pub mahjong_gekitou_bit_ptr: u8,
    pub mahjong_gekitou_strobe: u8,
    pub master_cycle_counter: u64,
    pub barcode_battler_stream: [u8; 200],
    pub barcode_battler_insert_cycle: u64,
    pub barcode_battler_active: bool,
    pub famicom_mic: Arc<AtomicBool>,
    pub paddle_x: Arc<Mutex<[u8; 3]>>,
    pub paddle_button: Arc<Mutex<[bool; 3]>>,
    pub paddle_readbit: [u8; 3],
    pub powerpad_state: Arc<Mutex<[u16; 2]>>,
    pub powerpad_shift_data: [u16; 2],
    pub powerpad_shift_count: [u8; 2],
    pub snes_state: Arc<Mutex<[u16; 2]>>,
    pub snes_readbit: [u8; 2],
    pub snes_mouse_state: [u32; 2],
    pub snes_mouse_readbit: [u8; 2],
    pub snes_mouse_delta_x: Arc<Mutex<[f32; 2]>>,
    pub snes_mouse_delta_y: Arc<Mutex<[f32; 2]>>,
    pub snes_mouse_buttons: Arc<Mutex<[u8; 2]>>,
    pub subor_mouse_buttons: Arc<Mutex<[u8; 2]>>,
    pub subor_mouse_dx: Arc<Mutex<[i32; 2]>>,
    pub subor_mouse_dy: Arc<Mutex<[i32; 2]>>,
    pub subor_mouse_latch: [u8; 2],
    pub hori_track_state: [u32; 2],
    pub hori_track_dx: Arc<Mutex<[f32; 2]>>,
    pub hori_track_dy: Arc<Mutex<[f32; 2]>>,
    pub hori_track_readbit: [u8; 2],

    pub controller_port3: Arc<AtomicU8>,
    pub controller_port4: Arc<AtomicU8>,
    pub fourscore_readbit: [u8; 2],
    pub virtualboy_state: Arc<Mutex<[u16; 2]>>,
    pub virtualboy_state_buffer: [u16; 2],
    pub virtualboy_readbit: [u8; 2],

    pub controller1_type: config::ControllerType,
    pub controller2_type: config::ControllerType,
    pub expansion_type: config::ExpansionType,
    pub expansion_adapter_type: config::ExpansionAdapterType,
    pub expansion_adapter_ports: [Arc<AtomicU8>; 4],
    pub expansion_adapter_shift_register: [u8; 4],

    pub frame_advance_reached_vblank: bool,

    pub screen: Vec<u32>,

    pub region_preference: Region,
    pub resolved_region: Region,

    pub is_um6578_cart: bool,
    pub is_vt32_cart: bool,
    pub is_vt369_ppu_cart: bool,
    pub is_vt369_enhanced_ppu_cart: bool,
    pub is_vs_system_cart: bool,
    pub vt03_4bpp_bg_cart: bool,
    pub vt03_4bpp_sp_cart: bool,
}

impl Emulator {
    pub fn init_ram(ram: &mut [u8; 0x2000], vram: &mut [u8; 0x800], mode: config::InitialRam) {
        match mode {
            config::InitialRam::Default => {
                for i in 0..0x2000usize {
                    let j = i & 0x2;
                    let swap = (i & 0x1F) >= 0x10;
                    if (j < 0x2) != swap {
                        if i < 0x800 {
                            vram[i] = 0xF0;
                        }
                        ram[i] = 0xF0;
                    } else {
                        if i < 0x800 {
                            vram[i] = 0x0F;
                        }
                        ram[i] = 0x0F;
                    }
                }
            }
            config::InitialRam::Zero => {
                for b in ram.iter_mut() { *b = 0; }
                for b in vram.iter_mut() { *b = 0; }
            }
            config::InitialRam::AllFF => {
                for b in ram.iter_mut() { *b = 0xFF; }
                for b in vram.iter_mut() { *b = 0xFF; }
            }
            config::InitialRam::Random => {
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                let mut state = seed as u32;
                for i in 0..0x2000usize {
                    state = state.wrapping_mul(1103515245).wrapping_add(12345);
                    ram[i] = (state >> 16) as u8;
                    if i < 0x800 {
                        state = state.wrapping_mul(1103515245).wrapping_add(12345);
                        vram[i] = (state >> 16) as u8;
                    }
                }
            }
        }
    }

    pub fn new() -> Self {
        let mut ram = [0u8; 0x2000];
        let mut vram = [0u8; 0x800];

        Self::init_ram(&mut ram, &mut vram, config::InitialRam::Default);
        let oam2 = [0xFFu8; 32];

        let mut palette_ram = [0u8; 0x400];
        let pal_init: [u8; 0x20] = [
            0x00,0x00,0x28,0x00,0x00,0x08,0x00,0x00,
            0x00,0x01,0x01,0x20,0x00,0x08,0x00,0x02,
            0x00,0x00,0x00,0x00,0x00,0x02,0x21,0x00,
            0x00,0x00,0x00,0x00,0x00,0x10,0x00,0x00,
        ];
        palette_ram[..0x20].copy_from_slice(&pal_init);

        Emulator {
            cart: None,
            ppu_clock: 0, cpu_clock: 0,
            program_counter: 0xFFFF, stack_pointer: 0x00,
            a: 0, x: 0, y: 0, h: 0, ignore_h: false,
            data_bus: 0, internal_bus: 0, address_bus: 0, special_bus: 0, dl: 0,
            op_code: 0, operation_cycle: 0, temporary_address: 0,
            total_cycles: 0,
            flag_carry: false, flag_zero: false, flag_interrupt: true,
            flag_decimal: false, flag_overflow: false, flag_negative: false,
            cpu_read: false, do_brk: false, do_nmi: false, do_irq: false,
            do_reset: true, fix_high_byte: false,
            do_oam_dma: false, first_cycle_of_oam_dma: false,
            do_dmc_dma: false, dmc_dma_delay: 0, cannot_run_dmc_dma_right_now: 0,
            dma_page: 0, dma_address: 0,
            oam_dma_aligned: false, oam_dma_halt: false, dmc_dma_halt: false,
            oam_internal_bus: 0,
            nmi_pins_signal: false, nmi_previous_pins_signal: false,
            irq_level_detector: false, nmi_line: false, irq_line: false,
            ram, cpu_ram_mask: 0x7FF, um6578_extra_ram: [0u8; 0x800], um6578_vram: [[0u8; 0x400]; 0xA], um6578_chr_ram: [0u8; 0x8000], um6578_reg2008: 0, um6578_color_mask: 0x3, um6578_dma_control: 0, um6578_dma_page: 0, um6578_dma_source: 0, um6578_dma_target: 0, um6578_dma_length: 0, um6578_dma_busy: 0, um6578_nt_tile_byte: 0, um6578_nt_attr_byte: 0, um6578_bg_palette: 0, um6578_bg_palette_lo: 0, vram, oam: [0u8; 0x100], oam2, palette_ram,
            ppu_bus: 0, ppu_bus_decay: [0i32; 8], ppu_oam_address: 0,
            ppu_status_vblank: false, ppu_status_sprite_zero_hit: false,
            ppu_status_sprite_zero_hit_delayed: false,
            ppu_status_sprite_overflow: false, ppu_status_sprite_overflow_delayed: false,
            ppu_status_pending_sprite_zero_hit: false,
            ppu_status_pending_sprite_zero_hit2: false,
            ppu_pending_vblank: false,
            ppu_vset: false, ppu_vset_latch1: false, ppu_vset_latch2: false,
            ppu_read_2002: false,
            ppu_v: 0, ppu_t: 0, ppu_fine_x_scroll: 0, ppu_addr_latch: false,
            ppu_control_increment_mode_32: false, ppu_control_nmi_enabled: false,
            ppu_sprite_x16: false,
            ppu_pattern_select_sprites: false, ppu_pattern_select_background: false,
            ppu_scanline: 0, ppu_dot: 7,
            ppu_odd_frame: true,
            ppu_address_bus: 0, ppu_ale: false, ppu_octal_latch: 0,
            ppu_read_buffer: 0, ppu_reset: false,
            ppu_mask_greyscale: false,
            ppu_mask_8px_show_background: false, ppu_mask_8px_show_sprites: false,
            ppu_mask_show_background: false, ppu_mask_show_sprites: false,
            ppu_mask_emphasize_red: false, ppu_mask_emphasize_green: false,
            ppu_mask_emphasize_blue: false,
            ppu_mask_show_background_instant: false, ppu_mask_show_sprites_instant: false,
            ppu_mask_show_background_delayed: false, ppu_mask_show_sprites_delayed: false,
            ppu_bg_pattern_sr_l: 0, ppu_bg_pattern_sr_h: 0,
            ppu_bg_pattern_sr_l2: 0, ppu_bg_pattern_sr_h2: 0,
            ppu_bg_attr_sr_l: 0, ppu_bg_attr_sr_h: 0,
            ppu_attr_latch_register: 0,
            ppu_low_bit_plane: 0, ppu_high_bit_plane: 0,
            ppu_low_bit_plane_hi: 0, ppu_high_bit_plane_hi: 0,
            ppu_attribute: 0,
            ppu_onebus_eva: 0,
            ppu_onebus_chr: crate::mappers::one_bus::OneBusChrCtx::default(),
            vt369_tile_data: [0; 272],
            vt369_tile_data_even: [0; 272 * 2],
            vt369_tile_data_odd: [0; 272 * 2],
            vt369_pat_addr: 0,
            vt369_nt_byte: 0,
            vt369_sprite_buf: [Vt369SpriteEntry::default(); 64],
            vt369_sprite_buf_hi: [Vt369SpriteHiEntry::default(); 64],
            vt369_sprite_count: 0,
            vt369_sprite_count_hi: 0,
            vt369_sprite_ram: [0; 512],
            vt369_spr_addr_high: 0,
            vt369_dma_target_addr: 0,
            ppu_sprite_sr_l: [0; 8], ppu_sprite_sr_h: [0; 8],
            ppu_sprite_sr_l2: [0; 8], ppu_sprite_sr_h2: [0; 8],
            ppu_sprite_attribute: [0; 8], ppu_sprite_pattern: [0; 8],
            ppu_sprite_x_position: [0; 8], ppu_sprite_y_position: [0; 8],
            ppu_sprite_shifter_counter: [0; 8],
            vs_ppu_variant: 3,
            ppu_sprite_pattern_l: 0, ppu_sprite_pattern_h: 0,
            ppu_sprite_pattern_l2: 0, ppu_sprite_pattern_h2: 0,
            ppu_next_scanline_contains_sprite_zero: false,
            ppu_current_scanline_contains_sprite_zero: false,
            ppu_can_detect_sprite_zero_hit: false,
            oam2_address: 0, secondary_oam_full: false, oam2_reset_signal: 0,
            sprite_evaluation_tick: 0,
            oam_address_overflowed_during_sprite_evaluation: false,
            ppu_oam_latch: 0, ppu_oam_buffer: 0, ppu_oam_read_latch: 0, ppu_render_temp: 0,
            in_range_check: 0, nine_objects_on_this_scanline: false,
            ppu_oam_corruption_rendering_disabled_out_of_vblank: false,
            ppu_oam_corruption_rendering_disabled_out_of_vblank_instant: false,
            ppu_v_register_changed_out_of_vblank: false,
            ppu_pending_oam_corruption: false, ppu_oam_corruption_index: 0,
            ppu_oam_corruption_rendering_enabled_out_of_vblank: false,
            ppu_oam_evaluation_corruption_odd_cycle: false,
            ppu_oam_evaluation_object_in_range: false,
            ppu_oam_evaluation_object_in_x_range: false,
            ppu_palette_corruption_rendering_disabled_out_of_vblank: false,
            oam_corrupted_on_odd_cycle: false,
            ppu_update_2006_delay: 0, ppu_update_2005_delay: 0,
            ppu_update_2001_delay: 0, ppu_update_2001_oam_corruption_delay: 0,
            ppu_update_2001_emphasis_bits_delay: 0,
            ppu_update_2005_value: 0, ppu_update_2001_value: 0,
            ppu_update_2006_value: 0, ppu_update_2006_value_temp: 0,
            ppu_was_rendering_before_2001_write: false,
            ppu_2007_read: false, ppu_2007_read_sr: false,
            ppu_2007_read_latches: [false; 5],
            ppu_2007_pd_rb: false, ppu_2007_read_ale: false,
            ppu_2007_read_h0_latch: false, ppu_2007_read_xrb: false,
            ppu_read: false,
            ppu_2007_write: false, ppu_2007_write_sr: false,
            ppu_2007_write_latches: [false; 5],
            ppu_2007_db_par: false, ppu_2007_write_ale: false,
            ppu_2007_tstep_latch: false, ppu_2007_tstep: false,
            ppu_2007_blnk_latch: false, ppu_2007_palette_ram_enable: false,
            ppu_2007_write_data: 0, ppu_write: false,
            ppu_pattern_address_register_nt: 0,
            ppu_pattern_address_register_at: 0, ppu_pattern_address_register_chr: 0,
            ppu_commit_nametable_fetch: false, ppu_commit_attribute_fetch: false,
            ppu_commit_pattern_low_fetch: false, ppu_commit_pattern_high_fetch: false,
            ppu_a12_prev: false, copy_v: false, skipped_pre_render_dot_341: false,
            dot_color: 0, prev_dot_color: 0, prev_prev_dot_color: 0,
            prev_prev_prev_dot_color: 0,
            dot_color_rgb: 0, prev_dot_color_rgb: 0, prev_prev_dot_color_rgb: 0,
            prev_prev_prev_dot_color_rgb: 0, palette_ram_address: 0,
            this_dot_read_from_palette_ram: false,
            apu_alignment: 0,
            apu_put_cycle: true,
            apu_status_dmc_interrupt: false, apu_status_frame_interrupt: false,
            apu_status_dmc: false, apu_status_delayed_dmc: false,
            apu_status_noise: false, apu_status_triangle: false,
            apu_status_pulse2: false, apu_status_pulse1: false,
            clearing_apu_frame_interrupt: false,
            apu_delayed_dmc_4015: 0,
            apu_implicit_abort_dmc_4015: false,
            apu_set_implicit_abort_dmc_4015: false,
            apu_register: [0u8; 0x10],
            apu_frame_counter_mode: false, apu_frame_counter_inhibit_irq: false,
            apu_frame_counter_reset: 0xFF, apu_framecounter: 0,
            apu_quarter_frame_clock: false, apu_half_frame_clock: false,
            apu_envelope_start_flag: false, apu_envelope_divider_clock: false,
            apu_envelope_decay_level: 0,
            apu_length_counter_pulse1: 0, apu_length_counter_pulse2: 0,
            apu_length_counter_triangle: 0, apu_length_counter_noise: 0,
            apu_length_counter_halt_pulse1: false, apu_length_counter_halt_pulse2: false,
            apu_length_counter_halt_triangle: false, apu_length_counter_halt_noise: false,
            apu_length_counter_reload_pulse1: false, apu_length_counter_reload_pulse2: false,
            apu_length_counter_reload_triangle: false, apu_length_counter_reload_noise: false,
            apu_length_counter_reload_value_pulse1: 0,
            apu_length_counter_reload_value_pulse2: 0,
            apu_length_counter_reload_value_triangle: 0,
            apu_length_counter_reload_value_noise: 0,
            apu_channel_timer_pulse1: 0, apu_channel_timer_pulse2: 0,
            apu_channel_timer_triangle: 0, apu_channel_timer_noise: 0,
            apu_channel_timer_dmc: 1022,
            apu_dmc_enable_irq: false, apu_dmc_loop: false,
            apu_dmc_rate: 428, apu_dmc_output: 0,
            apu_dmc_sample_address: 0xC000, apu_dmc_sample_length: 1,
            apu_dmc_bytes_remaining: 0, apu_dmc_buffer: 0,
            apu_dmc_address_counter: 0xC000, apu_dmc_shifter: 0,
            apu_dmc_shifter_bits_remaining: 8, dpcm_up: false, apu_silent: true,
            audio_buffer: None,
            audio_cycles_accumulator: 0.0,
            audio_sample_accumulator: 0.0,
            audio_sample_count: 0.0,
            audio_host_sample_rate: 44100.0,
            blip: None,
            audio_frame_cycle: 0,
            audio_previous_output: 0,
            master_volume: 1.0,
            square1_volume: 1.0,
            square2_volume: 1.0,
            triangle_volume: 1.0,
            noise_volume: 1.0,
            pcm_volume: 1.0,
            expansion_volume: 1.0,
            fds_volume: 1.0,
            mmc5_volume: 1.0,
            vrc6_volume: 1.0,
            vrc7_volume: 1.0,
            n163_volume: 1.0,
            sunsoft5b_volume: 1.0,
            audio_enabled: true,
            audio_depth: 16,
            swap_duty_cycles: false,
            filter_lp_alpha: 0.815686,
            filter_lp_prev_out: 0.0,
            filter_hp1_alpha: 0.996039,
            filter_hp1_prev_in: 0.0,
            filter_hp1_prev_out: 0.0,
            filter_hp2_alpha: 0.999835,
            filter_hp2_prev_in: 0.0,
            filter_hp2_prev_out: 0.0,
            pulse1_timer: 0,
            pulse1_sequencer_step: 0,
            pulse1_envelope_divider: 0,
            pulse1_envelope_decay_level: 0,
            pulse1_envelope_start_flag: false,
            pulse1_sweep_divider: 0,
            pulse1_sweep_reload: false,
            pulse2_timer: 0,
            pulse2_sequencer_step: 0,
            pulse2_envelope_divider: 0,
            pulse2_envelope_decay_level: 0,
            pulse2_envelope_start_flag: false,
            pulse2_sweep_divider: 0,
            pulse2_sweep_reload: false,
            triangle_timer: 0,
            triangle_sequencer_step: 0,
            triangle_linear_counter: 0,
            triangle_linear_counter_reload_flag: false,
            noise_timer: 0,
            noise_shift_register: 1,
            noise_envelope_divider: 0,
            noise_envelope_decay_level: 0,
            noise_envelope_start_flag: false,

            apu_controller_ports_strobing: false, apu_controller_ports_strobed: false,
            controller_port1: Arc::new(AtomicU8::new(0)), controller_port2: Arc::new(AtomicU8::new(0)),
            controller_shift_register1: 0, controller_shift_register2: 0,
            controller1_shift_counter: 0, controller2_shift_counter: 0,
            data_pins_are_not_floating: false,
            zapper_x: Arc::new(Mutex::new(0.0)), zapper_y: Arc::new(Mutex::new(0.0)),
            zapper_trigger: Arc::new(AtomicBool::new(false)), zapper_bogo: Arc::new(AtomicU8::new(0)),
            expansion_zapper_trigger: Arc::new(AtomicBool::new(false)),
            oeka_click: Arc::new(AtomicBool::new(false)),
            oeka_strobe: false, oeka_shift: false, oeka_state_buffer: 0,
            family_trainer_state: Arc::new(Mutex::new([0; 12])), family_trainer_ignore_rows: 0,
            hyper_shot_state: Arc::new(Mutex::new([0; 4])), hyper_shot_enable_p1: true, hyper_shot_enable_p2: true,
            bandai_hyper_state: [0; 2], bandai_hyper_readbit: [0; 2],
            bandai_hyper_buttons: Arc::new(Mutex::new([0; 9])),
            turbo_file_data: [0; 0x2000], turbo_file_position: 0, turbo_file_last_write: 0,
            battle_box_data: [0; 0x200], battle_box_last_write: 0, battle_box_address: 0, battle_box_chip_select: 0, battle_box_output: 0, battle_box_write_enabled: false, battle_box_input_bit_position: 0, battle_box_input_data: 0, battle_box_is_write: false, battle_box_is_read: false,
            family_basic_state: Arc::new(Mutex::new([0; 72])), family_basic_row: 0, family_basic_column: 0, family_basic_enabled: false,
            party_tap_state: Arc::new(Mutex::new([0; 6])), party_tap_buffer: 0, party_tap_read_count: 0, party_tap_strobe: false,
            pachinko_state: Arc::new(Mutex::new([0; 10])), pachinko_analog: 0, pachinko_buffer: 0, pachinko_strobe: false,
            punching_bag_state: Arc::new(Mutex::new([0; 8])), punching_bag_selected_sensors: 0,
            jissen_mahjong_state: Arc::new(Mutex::new([0; 21])), jissen_mahjong_row: 0, jissen_mahjong_state_buffer: 0, jissen_mahjong_strobe: false,
            subor_keyboard_state: Arc::new(Mutex::new([0; 104])), subor_keyboard_row: 0, subor_keyboard_column: 0, subor_keyboard_enabled: false,
            pec586_keyboard_state: Arc::new(Mutex::new([0; 104])), pec586_kspos: 0, pec586_ksindex: 0, pec586_kstrobe: 0,
            bit79_keyboard_state: Arc::new(Mutex::new([0; 104])), bit79_keyboard_row: 0, bit79_keyboard_column: 0, bit79_keyboard_strobe: 0,
            keda_keyboard_state: Arc::new(Mutex::new([0; 104])), keda_keyboard_row: 0, keda_keyboard_column: 0, keda_keyboard_strobe: 0,
            kingwon_keyboard_state: Arc::new(Mutex::new([0; 104])), kingwon_keyboard_row: 0, kingwon_keyboard_column: 0, kingwon_keyboard_bits: 0,
            zecheng_keyboard_bits1: 0, zecheng_keyboard_bits2: 0, zecheng_keyboard_strobe: false,
            zecheng_keyboard_buttons: Arc::new(Mutex::new(0)), zecheng_keyboard_keys: Arc::new(Mutex::new(0)), zecheng_keyboard_dx: Arc::new(Mutex::new(0)), zecheng_keyboard_dy: Arc::new(Mutex::new(0)),
            quiz_king_buttons: Arc::new(Mutex::new([0; 6])), quiz_king_data_r: 0, quiz_king_funky_mode: false, quiz_king_strobe: false,
            top_rider_buttons: Arc::new(Mutex::new([0; 8])), top_rider_bs: 0, top_rider_bss: 0, top_rider_boop: 0,
            fami_net_sys_buttons: Arc::new(Mutex::new([0; 24])), fami_net_sys_data: 0, fami_net_sys_readbit: 0, fami_net_sys_prev_strobe: false,
            city_patrolman_input: Arc::new(Mutex::new([0; 4])), city_patrolman_strobe: 0, city_patrolman_time_out: 0, city_patrolman_shift_reg: 0,
            moguraa_buttons: Arc::new(Mutex::new([0; 12])), moguraa_bits: 0, moguraa_sel: 0,
            sharpc1_tape_state: 0, sharpc1_tape_data: Vec::new(), sharpc1_tape_bits: 0, sharpc1_tape_position: 0, sharpc1_tape_level: false, sharpc1_tape_file_format: 1, sharpc1_tape_path: None,
            golden_nugget_buttons: Arc::new(Mutex::new([0; 11])), golden_nugget_strobe: 0, golden_nugget_shift: 0,
            abl_pinball_buttons: Arc::new(Mutex::new([0; 6])), abl_pinball_wheel: Arc::new(Mutex::new(0)), abl_pinball_count: -1, abl_pinball_plunger: 1, abl_pinball_plunger_reset_count: 0,
            tv_pump_buttons: Arc::new(Mutex::new([0; 6])),
            triface_mahjong_buttons: Arc::new(Mutex::new([0; 22])), triface_mahjong_column: 0, triface_mahjong_row: 0, triface_mahjong_keys: [0; 10],
            mahjong_gekitou_buttons: Arc::new(Mutex::new([0; 22])), mahjong_gekitou_bits: 0, mahjong_gekitou_bit_ptr: 0, mahjong_gekitou_strobe: 0,
            master_cycle_counter: 0, barcode_battler_stream: [0; 200], barcode_battler_insert_cycle: 0, barcode_battler_active: false,
            famicom_mic: Arc::new(AtomicBool::new(false)),
            paddle_x: Arc::new(Mutex::new([0; 3])), paddle_button: Arc::new(Mutex::new([false; 3])), paddle_readbit: [0; 3],
            powerpad_state: Arc::new(Mutex::new([0; 2])), powerpad_shift_data: [0; 2], powerpad_shift_count: [0; 2],
            snes_state: Arc::new(Mutex::new([0; 2])), snes_readbit: [0; 2],
            snes_mouse_state: [0; 2], snes_mouse_readbit: [0; 2],
            snes_mouse_delta_x: Arc::new(Mutex::new([0.0; 2])), snes_mouse_delta_y: Arc::new(Mutex::new([0.0; 2])),
            snes_mouse_buttons: Arc::new(Mutex::new([0; 2])),
            subor_mouse_buttons: Arc::new(Mutex::new([0; 2])),
            subor_mouse_dx: Arc::new(Mutex::new([0; 2])),
            subor_mouse_dy: Arc::new(Mutex::new([0; 2])),
            subor_mouse_latch: [0; 2],
            hori_track_state: [0; 2], hori_track_dx: Arc::new(Mutex::new([0.0; 2])), hori_track_dy: Arc::new(Mutex::new([0.0; 2])), hori_track_readbit: [0; 2],
            controller_port3: Arc::new(AtomicU8::new(0)),
            controller_port4: Arc::new(AtomicU8::new(0)),
            fourscore_readbit: [0; 2],
            virtualboy_state: Arc::new(Mutex::new([0u16; 2])),
            virtualboy_state_buffer: [0u16; 2],
            virtualboy_readbit: [0; 2],
            controller1_type: config::ControllerType::None,
            controller2_type: config::ControllerType::None,
            expansion_type: config::ExpansionType::None,
            expansion_adapter_type: config::ExpansionAdapterType::None,
            expansion_adapter_ports: [Arc::new(AtomicU8::new(0)), Arc::new(AtomicU8::new(0)), Arc::new(AtomicU8::new(0)), Arc::new(AtomicU8::new(0))],
            expansion_adapter_shift_register: [0; 4],
            frame_advance_reached_vblank: false,
            screen: vec![0u32; 256 * 240],
            region_preference: Region::Auto,
            resolved_region: Region::Ntsc,
            is_um6578_cart: false,
            is_vt32_cart: false,
            is_vt369_ppu_cart: false,
            is_vt369_enhanced_ppu_cart: false,
            is_vs_system_cart: false,
            vt03_4bpp_bg_cart: false,
            vt03_4bpp_sp_cart: false,
        }
    }

    pub fn load_cartridge(&mut self, cart: Cartridge) {
        self.resolved_region = self.compute_region(&cart.tv_system, &cart.name);
        let cpu_clock = self.cpu_clock();
        self.is_um6578_cart = cart.mapper_chip.is_um6578();
        self.is_vt32_cart = cart.mapper_chip.is_vt32();
        self.is_vt369_ppu_cart = cart.mapper_chip.onebus_vt369_ppu();
        self.is_vt369_enhanced_ppu_cart = cart.mapper_chip.onebus_vt369_enhanced_ppu();
        self.is_vs_system_cart = cart.is_vs_system;
        self.vt03_4bpp_bg_cart = cart.mapper_chip.vt03_4bpp_bg();
        self.vt03_4bpp_sp_cart = cart.mapper_chip.vt03_4bpp_sp();
        self.cpu_ram_mask = if self.is_um6578_cart {
            0x1FFF
        } else if cart.mapper_chip.onebus_cpu_ram_4k() {
            0xFFF
        } else {
            0x7FF
        };
        if self.is_um6578_cart {
            self.apu_frame_counter_inhibit_irq = true;
            if cart.memory_mapper == 601 {
                self.um6578_dma_page = 0x20;
            }
        }
        let old = self.cart.replace(cart);
        if let Some(old_cart) = old {
            std::thread::spawn(move || drop(old_cart));
        }
        if let Some(ref mut cart) = self.cart {
            cart.mapper_chip.set_cpu_clock(cpu_clock);
        }
        self.load_turbo_file();
        self.load_battle_box();
        let host_rate = self.audio_host_sample_rate as u32;
        if let Some(ref mut cart) = self.cart {
            cart.mapper_chip.set_audio_sample_rate(host_rate);
        }
        self.reset_audio();
    }
    pub fn clear_cart(&mut self) {
        if let Some(old) = self.cart.take() {
            std::thread::spawn(move || drop(old));
        }
        self.is_um6578_cart = false;
        self.is_vt32_cart = false;
        self.is_vt369_ppu_cart = false;
        self.is_vt369_enhanced_ppu_cart = false;
        self.is_vs_system_cart = false;
        self.vt03_4bpp_bg_cart = false;
        self.vt03_4bpp_sp_cart = false;
        self.reset_audio();
    }

    pub fn set_region_preference(&mut self, region: Region) {
        self.region_preference = region;
        if let Some(ref cart) = self.cart {
            self.resolved_region = self.compute_region(&cart.tv_system, &cart.name);
        }
        let cpu_clock = self.cpu_clock();
        if let Some(ref mut cart) = self.cart {
            cart.mapper_chip.set_cpu_clock(cpu_clock);
        }
        self.reset();
    }

    fn compute_region(&self, tv_system: &TvSystem, filename: &str) -> Region {
        match self.region_preference {
            Region::Ntsc => Region::Ntsc,
            Region::Pal => Region::Pal,
            Region::Dendy => Region::Dendy,
            Region::Auto => {
                match tv_system {
                    TvSystem::Ntsc | TvSystem::Dual => Region::Ntsc,
                    TvSystem::Pal => Region::Pal,
                    TvSystem::Dendy => Region::Dendy,
                    TvSystem::Unknown => {
                        match TvSystem::from_filename(filename) {
                            TvSystem::Pal => Region::Pal,
                            TvSystem::Dendy => Region::Dendy,
                            _ => Region::Ntsc,
                        }
                    }
                }
            }
        }
    }

    pub fn is_pal(&self) -> bool {
        self.resolved_region == Region::Pal
    }

    pub fn is_dendy(&self) -> bool {
        self.resolved_region == Region::Dendy
    }

    pub fn cpu_clock(&self) -> f64 {
        if self.is_pal() {
            1_662_607.0
        } else if self.is_dendy() {
            1_773_448.0
        } else {
            1_789_773.0
        }
    }

    pub fn total_scanlines(&self) -> u16 {
        if self.is_pal() || self.is_dendy() { 312 } else { 262 }
    }

    pub fn pre_render_scanline(&self) -> u16 {
        self.total_scanlines() - 1
    }

    pub fn nmi_scanline(&self) -> u16 {
        if self.is_dendy() { 291 } else { 241 }
    }

    pub fn mapper_scanline(&self) -> u16 {
        if (self.is_pal() || self.is_dendy()) && self.ppu_scanline == self.pre_render_scanline() {
            261
        } else {
            self.ppu_scanline
        }
    }

    pub fn power_cycle(&mut self, mode: config::InitialRam) {
        Self::init_ram(&mut self.ram, &mut self.vram, mode);
        self.oam2 = [0xFFu8; 32];
        self.reset();
        if let Some(ref mut cart) = self.cart {
            let saved_dip = cart.mapper_chip.get_dip_switches();
            cart.mapper_chip.reset_power_cycle();
            cart.mapper_chip.set_dip_switches(saved_dip);
        }
    }

    pub fn apply_apu_alignment(&mut self) {
        match self.apu_alignment & 3 {
            1 => { self.apu_channel_timer_dmc = 1022; self.apu_put_cycle = false; }
            2 => { self.apu_channel_timer_dmc = 1020; self.apu_put_cycle = true; }
            3 => { self.apu_channel_timer_dmc = 1020; self.apu_put_cycle = false; }
            _ => { self.apu_channel_timer_dmc = 1022; self.apu_put_cycle = true; }
        }
    }

    pub fn reset(&mut self) {
        self.flag_interrupt = true;
        self.apu_dmc_output &= 1;

        self.apu_status_dmc_interrupt = false;
        self.apu_status_frame_interrupt = false;
        self.apu_status_delayed_dmc = false;
        self.apu_status_dmc = false;
        self.apu_status_noise = false;
        self.apu_status_triangle = false;
        self.apu_status_pulse2 = false;
        self.apu_status_pulse1 = false;
        self.apu_dmc_bytes_remaining = 0;
        self.apu_length_counter_noise = 0;
        self.apu_length_counter_triangle = 0;
        self.apu_length_counter_pulse2 = 0;
        self.apu_length_counter_pulse1 = 0;
        self.apu_framecounter = 0;

        self.ppu_control_nmi_enabled = false;
        self.ppu_control_increment_mode_32 = false;
        self.ppu_sprite_x16 = false;
        self.ppu_pattern_select_sprites = false;
        self.ppu_pattern_select_background = false;
        self.ppu_t = 0;

        self.ppu_mask_greyscale = false;
        self.ppu_mask_emphasize_red = false;
        self.ppu_mask_emphasize_green = false;
        self.ppu_mask_emphasize_blue = false;
        self.ppu_mask_8px_show_background = false;
        self.ppu_mask_8px_show_sprites = false;
        self.ppu_mask_show_background = false;
        self.ppu_mask_show_sprites = false;

        self.ppu_update_2005_delay = 0;
        self.ppu_fine_x_scroll = 0;

        self.ppu_read_buffer = 0;
        self.ppu_odd_frame = false;

        self.ppu_dot = 0;
        self.ppu_scanline = 0;

        self.do_dmc_dma = false;
        self.do_oam_dma = false;
        self.operation_cycle = 0;
        self.zapper_bogo.store(0, Ordering::Relaxed);

        self.apply_apu_alignment();

        self.ppu_clock = 0;
        self.cpu_clock = 0;
        self.do_reset = true;
        self.ppu_reset = false;

        if let Some(ref mut cart) = self.cart {
            if (cart.memory_mapper == 6 || cart.memory_mapper == 17) && !cart.trainer.is_empty() {
                crate::mappers::ffe::install_trainer(&cart.trainer, &mut cart.prg_ram);
            }
                        let saved_dip = cart.mapper_chip.get_dip_switches();
            cart.mapper_chip.reset();
            cart.mapper_chip.set_dip_switches(saved_dip);
        }
        self.reset_audio();
    }

    pub fn save_prg_ram(&self) {
        if let Some(cart) = &self.cart {
            if !cart.has_battery {
                return;
            }
            let sav_path = crate::config::save_file_path(&cart.name);
            let data_opt = {
                let mapper = &cart.mapper_chip;
                if let Some(save) = mapper.battery_save_data(cart) {
                    Some(save)
                } else if !cart.prg_ram.is_empty() {
                    Some(cart.prg_ram.clone())
                } else {
                    None
                }
            };
            if let Some(data) = data_opt {
                std::thread::spawn(move || {
                    if let Some(parent) = sav_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Err(e) = std::fs::write(&sav_path, &data) {
                        eprintln!("Failed to save SRAM to {:?}: {}", sav_path, e);
                    } else {
                        println!("Saved SRAM to {:?}", sav_path);
                    }
                });
            }
        }
    }

    /// ascii turbo file: persist the 0x2000-byte SRAM to a per-rom .turbofile.sav
    pub fn save_turbo_file(&self) {
        if !self.expansion_type.is_turbo_file() {
            return;
        }
        if let Some(cart) = &self.cart {
            let sav_path = crate::config::turbofile_save_path(&cart.name);
            let data = self.turbo_file_data.to_vec();
            std::thread::spawn(move || {
                if let Some(parent) = sav_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(e) = std::fs::write(&sav_path, &data) {
                    eprintln!("Failed to save Turbo File SRAM to {:?}: {}", sav_path, e);
                } else {
                    println!("Saved Turbo File SRAM to {:?}", sav_path);
                }
            });
        }
    }

    /// ascii turbo file: load SRAM from the per-rom .turbofile.sav
    pub fn load_turbo_file(&mut self) {
        self.turbo_file_data = [0; 0x2000];
        self.turbo_file_position = 0;
        self.turbo_file_last_write = 0;
        if !self.expansion_type.is_turbo_file() {
            return;
        }
        if let Some(cart) = &self.cart {
            let sav_path = crate::config::turbofile_save_path(&cart.name);
            if let Ok(data) = std::fs::read(&sav_path) {
                if data.len() >= 0x2000 {
                    self.turbo_file_data[..0x2000].copy_from_slice(&data[..0x2000]);
                }
            }
        }
    }

    /// battle box: persist the 0x200-byte storage to a per-rom .battlebox.sav
    pub fn save_battle_box(&self) {
        if !self.expansion_type.is_battle_box() {
            return;
        }
        if let Some(cart) = &self.cart {
            let sav_path = crate::config::battlebox_save_path(&cart.name);
            let data = self.battle_box_data.to_vec();
            std::thread::spawn(move || {
                if let Some(parent) = sav_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(e) = std::fs::write(&sav_path, &data) {
                    eprintln!("Failed to save Battle Box SRAM to {:?}: {}", sav_path, e);
                } else {
                    println!("Saved Battle Box SRAM to {:?}", sav_path);
                }
            });
        }
    }

    /// battle box: load storage from the per-rom .battlebox.sav
    pub fn load_battle_box(&mut self) {
        self.battle_box_data = [0; 0x200];
        self.battle_box_last_write = 0;
        self.battle_box_address = 0;
        self.battle_box_chip_select = 0;
        self.battle_box_output = 0;
        self.battle_box_write_enabled = false;
        self.battle_box_input_bit_position = 0;
        self.battle_box_input_data = 0;
        self.battle_box_is_write = false;
        self.battle_box_is_read = false;
        if !self.expansion_type.is_battle_box() {
            return;
        }
        if let Some(cart) = &self.cart {
            let sav_path = crate::config::battlebox_save_path(&cart.name);
            if let Ok(data) = std::fs::read(&sav_path) {
                if data.len() >= 0x200 {
                    self.battle_box_data[..0x200].copy_from_slice(&data[..0x200]);
                }
            }
        }
    }

    pub fn expansion_tick(&mut self) {
        if self.expansion_type.is_city_patrolman() {
            self.city_patrolman_time_out = self.city_patrolman_time_out.wrapping_add(1);
        }
        if self.expansion_type.is_abl_pinball() {
            self.abl_pinball_plunger_reset_count += 1;
            if self.abl_pinball_plunger_reset_count >= 1789773 / 4 {
                self.abl_pinball_plunger_reset_count = 0;
                if self.abl_pinball_plunger > 0 {
                    self.abl_pinball_plunger -= 1;
                }
            }
        }
        if self.expansion_type.is_sharp_c1_cassette() {
            self.tape_cpu_cycle();
        }
    }

    pub fn tape_play(&mut self, path: std::path::PathBuf, file_format: u8) {
        self.tape_stop();
        if let Ok(data) = std::fs::read(&path) {
            self.sharpc1_tape_data = data;
            self.sharpc1_tape_bits = (self.sharpc1_tape_data.len() as u32).wrapping_mul(8);
            self.sharpc1_tape_position = 0;
            self.sharpc1_tape_state = 1;
            self.sharpc1_tape_file_format = file_format;
            self.sharpc1_tape_path = Some(path);
        }
    }

    pub fn tape_record(&mut self, path: std::path::PathBuf, file_format: u8) {
        self.tape_stop();
        self.sharpc1_tape_data.clear();
        self.sharpc1_tape_state = 2;
        self.sharpc1_tape_file_format = file_format;
        self.sharpc1_tape_path = Some(path);
    }

    pub fn tape_stop(&mut self) {
        if self.sharpc1_tape_state == 2 {
            if let Some(path) = self.sharpc1_tape_path.clone() {
                let mut out: Vec<u8> = Vec::new();
                if self.sharpc1_tape_file_format == 2 {
                    let mut count: u32 = 0;
                    for &byte in self.sharpc1_tape_data.iter() {
                        for bit in 0..8 {
                            let level = if byte & (0x80 >> bit) != 0 { 0.50f32 } else { -0.50f32 };
                            count += 176;
                            if count >= 39375 {
                                count -= 39375;
                                let sample = ((level * 127.0) - 128.0) as i16 as u8;
                                out.push(sample);
                            }
                        }
                    }
                    let mut wav = Vec::with_capacity(44 + out.len());
                    wav.extend_from_slice(b"RIFF");
                    wav.extend_from_slice(&((36 + out.len() as u32).to_le_bytes()));
                    wav.extend_from_slice(b"WAVEfmt ");
                    wav.extend_from_slice(&20u32.to_le_bytes());
                    wav.extend_from_slice(&1u16.to_le_bytes());
                    wav.extend_from_slice(&1u16.to_le_bytes());
                    wav.extend_from_slice(&8000u32.to_le_bytes());
                    wav.extend_from_slice(&8000u32.to_le_bytes());
                    wav.extend_from_slice(&1u16.to_le_bytes());
                    wav.extend_from_slice(&8u16.to_le_bytes());
                    wav.extend_from_slice(b"data");
                    wav.extend_from_slice(&(out.len() as u32).to_le_bytes());
                    wav.extend_from_slice(&out);
                    let _ = std::fs::write(&path, &wav);
                } else {
                    out = self.sharpc1_tape_data.clone();
                    let _ = std::fs::write(&path, &out);
                }
            }
        }
        self.sharpc1_tape_data.clear();
        self.sharpc1_tape_bits = 0;
        self.sharpc1_tape_position = 0;
        self.sharpc1_tape_level = false;
        self.sharpc1_tape_state = 0;
        self.sharpc1_tape_path = None;
    }

    pub fn tape_cpu_cycle(&mut self) {
        match self.sharpc1_tape_state {
            2 => {
                if (self.sharpc1_tape_bits & 7) == 0 {
                    self.sharpc1_tape_data.push(0);
                }
                let byte_idx = (self.sharpc1_tape_position >> 3) as usize;
                let bit_mask = 0x80 >> (self.sharpc1_tape_position & 7);
                if byte_idx < self.sharpc1_tape_data.len() {
                    if self.sharpc1_tape_level {
                        self.sharpc1_tape_data[byte_idx] |= bit_mask;
                    } else {
                        self.sharpc1_tape_data[byte_idx] &= !bit_mask;
                    }
                }
                self.sharpc1_tape_position = self.sharpc1_tape_position.wrapping_add(1);
                self.sharpc1_tape_bits = self.sharpc1_tape_bits.wrapping_add(1);
            }
            1 => {
                if self.sharpc1_tape_position < self.sharpc1_tape_bits {
                    let byte_idx = (self.sharpc1_tape_position >> 3) as usize;
                    self.sharpc1_tape_level = if byte_idx < self.sharpc1_tape_data.len() {
                        (self.sharpc1_tape_data[byte_idx] & (0x80 >> (self.sharpc1_tape_position & 7))) != 0
                    } else {
                        false
                    };
                } else {
                    self.tape_stop();
                }
                self.sharpc1_tape_position = self.sharpc1_tape_position.wrapping_add(1);
            }
            _ => {}
        }
    }

    pub fn tape_output(&mut self, level: bool) {
        if self.sharpc1_tape_state == 2 {
            self.store_apu_registers(0x4011, if level { 1 } else { 0 });
        }
        self.sharpc1_tape_level = level;
    }

    pub fn tape_input(&mut self) -> bool {
        if self.sharpc1_tape_state == 1 {
            self.store_apu_registers(0x4011, if self.sharpc1_tape_level { 1 } else { 0 });
        }
        self.sharpc1_tape_level
    }

    pub fn set_audio_output(
        &mut self,
                buffer: std::sync::Arc<std::sync::Mutex<crate::audio::AudioRingBuffer>>,
        sample_rate: f64,
    ) {
        self.audio_buffer = Some(buffer);
        self.audio_host_sample_rate = sample_rate;

        let sr = sample_rate as f32;
        let dt = 1.0 / sr;
        
        let mut blip = crate::blip::BlipBuf::new(crate::blip::BlipBuf::MAX_FRAME);
        blip.set_rates(self.cpu_clock(), sample_rate);
        self.blip = Some(blip);

        self.filter_lp_alpha = dt / ((1.0 / (2.0 * std::f32::consts::PI * 14000.0)) + dt);

        self.filter_hp1_alpha = (1.0 / (2.0 * std::f32::consts::PI * 440.0)) / ((1.0 / (2.0 * std::f32::consts::PI * 440.0)) + dt);

                self.filter_hp2_alpha = (1.0 / (2.0 * std::f32::consts::PI * 90.0)) / ((1.0 / (2.0 * std::f32::consts::PI * 90.0)) + dt);

        self.audio_cycles_accumulator = 0.0;
        self.audio_sample_accumulator = 0.0;
        self.audio_sample_count = 0.0;
        self.audio_frame_cycle = 0;
        self.audio_previous_output = 0;
        if let Some(ref mut blip) = self.blip {
            blip.clear();
        }
        self.reset_audio();
    }

    pub fn reset_audio(&mut self) {
        if let Some(ref buffer) = self.audio_buffer {
            let prime =
                ((self.audio_host_sample_rate * 0.03) as usize).max(256);
            if let Ok(mut ring) = buffer.lock() {
                ring.clear_and_reset();
                ring.fill_silence(prime);
            }
        }
        self.audio_frame_cycle = 0;
        self.audio_previous_output = 0;
        self.filter_lp_prev_out = 0.0;
        self.filter_hp1_prev_in = 0.0;
        self.filter_hp1_prev_out = 0.0;
        self.filter_hp2_prev_in = 0.0;
        self.filter_hp2_prev_out = 0.0;
        let cpu_clock = self.cpu_clock();
        let host_rate = self.audio_host_sample_rate;
        if let Some(ref mut blip) = self.blip {
            blip.set_rates(cpu_clock, host_rate);
            blip.clear();
        }
    }

    pub fn change_disk(&mut self) {
        if let Some(ref mut cart) = self.cart {
            cart.mapper_chip.change_disk();
        }
    }

    pub fn disk_inserted(&self) -> bool {
        if let Some(ref cart) = self.cart {
            cart.mapper_chip.disk_inserted()
        } else {
            false
        }
    }

    pub fn eject_disk(&mut self) {
        if let Some(ref mut cart) = self.cart {
            cart.mapper_chip.eject_disk();
        }
    }

    pub fn insert_disk(&mut self) {
        if let Some(ref mut cart) = self.cart {
            cart.mapper_chip.insert_disk();
        }
    }

    pub fn insert_coin(&mut self, coin: u8) {
        if let Some(ref mut cart) = self.cart {
            cart.mapper_chip.insert_coin(coin);
        }
    }

    pub fn service_button(&mut self) {
        if let Some(ref mut cart) = self.cart {
            cart.mapper_chip.service_button();
        }
    }

    pub fn get_dip_switches(&self) -> u8 {
        if let Some(ref cart) = self.cart {
            cart.mapper_chip.get_dip_switches()
        } else {
            0
        }
    }

    pub fn set_dip_switches(&mut self, value: u8) {
        if let Some(ref mut cart) = self.cart {
            cart.mapper_chip.set_dip_switches(value);
        }
    }

    pub fn set_barcode(&mut self, rcode: &[u8]) -> bool {
        if self.expansion_type.is_barcode_battler() {
            self.set_barcode_battler(rcode);
            return true;
        }
        if let Some(ref mut cart) = self.cart {
            cart.mapper_chip.set_barcode(rcode)
        } else {
            false
        }
    }

    pub fn set_barcode_battler(&mut self, rcode: &[u8]) {
        let barcode_text = std::str::from_utf8(rcode).unwrap_or("");
        let mut stream = [0u8; 200];
        let mut text = String::from(barcode_text);
        text += "EPOCH\r\n";
        text.insert_str(0, &(0..20usize.saturating_sub(text.len())).map(|_| ' ').collect::<String>());
        let mut pos = 0usize;
        for i in 0..20.min(text.len()) {
            let ch = text.as_bytes()[i];
            stream[pos] = 1;
            pos += 1;
            for j in 0..8u8 {
                stream[pos] = !(((ch >> j) & 0x01) as u8);
                pos += 1;
            }
            stream[pos] = 0;
            pos += 1;
        }
        self.barcode_battler_stream = stream;
        self.barcode_battler_insert_cycle = self.master_cycle_counter;
        self.barcode_battler_active = true;
    }

    #[allow(dead_code)]
    pub fn get_vs_ppu_variant(&self) -> u8 {
        self.vs_ppu_variant
    }

    pub fn set_vs_ppu_variant(&mut self, variant: u8) {
        self.vs_ppu_variant = variant;
    }

    pub fn has_dip_switches(&self) -> bool {
        if let Some(ref cart) = self.cart {
            cart.is_vs_system
                || matches!(
                    cart.memory_mapper,
                    45 | 59 | 67 | 68 | 75 | 83 | 90 | 99 | 124 | 151 | 176 | 206 | 242 | 260 | 262 | 264
                        | 270 | 286 | 287 | 288 | 289 | 296 | 319 | 332 | 334 | 338 | 344 | 351 | 357
                        | 360 | 367 | 370 | 380 | 390 | 401 | 411 | 412 | 414 | 421 | 428 | 432 | 435
                        | 443 | 444 | 445 | 447 | 449 | 458 | 460 | 472 | 482 | 488 | 490 | 494 | 555
                        | 568 | 624
                )
        } else {
            false
        }
    }

    pub fn prg_rom_crc32(&self) -> u32 {
        if let Some(ref cart) = self.cart {
            cart.prg_rom_crc32
        } else {
            0
        }
    }

    pub fn memory_mapper(&self) -> u16 {
        if let Some(ref cart) = self.cart {
            cart.memory_mapper
        } else {
            0
        }
    }


    // run one frame!
    pub fn core_frame_advance(&mut self) {
        let mut bogo = self.zapper_bogo.load(Ordering::Relaxed);
        while bogo > 0 {
            match self.zapper_bogo.compare_exchange_weak(bogo, bogo - 1, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(v) => bogo = v,
            }
        }
        self.frame_advance_reached_vblank = false;
        if self.is_pal() {
            while !self.frame_advance_reached_vblank {
                self.emulator_core_pal();
            }
            while self.ppu_scanline != 0 {
                self.emulator_core_pal();
            }
        } else if self.is_dendy() {
            while !self.frame_advance_reached_vblank {
                self.emulator_core_dendy();
            }
            while self.ppu_scanline != 0 {
                self.emulator_core_dendy();
            }
        } else {
            while !self.frame_advance_reached_vblank {
                self.emulator_core_ntsc();
            }
            while self.ppu_scanline != 0 {
                self.emulator_core_ntsc();
            }
        }
    }

    pub fn emulator_core(&mut self) {
        if self.is_pal() {
            self.emulator_core_pal();
        } else if self.is_dendy() {
            self.emulator_core_dendy();
        } else {
            self.emulator_core_ntsc();
        }
    }

    // ntsc core logic
    fn emulator_core_ntsc(&mut self) {
        self.master_cycle_counter += 1;
        if self.cpu_clock == 12 {
            self.cpu_clock = 0;
            if let Some(cart) = self.cart.as_mut() {
                let irq = cart.mapper_chip.cpu_clock(1);
                if cart.mapper_chip.cpu_clock_irq_level() {
                    self.irq_level_detector = irq;
                } else if irq {
                    self.irq_level_detector = true;
                }
            }
            self.cpu_tick();
            self.total_cycles += 1;
        }

        if self.cpu_clock == 4 {
            self.nmi_line |= self.ppu_control_nmi_enabled && self.ppu_status_vblank;
            if self.operation_cycle == 0 && !(self.ppu_control_nmi_enabled && self.ppu_status_vblank) {
                self.nmi_line = false;
            }
        }

        if self.cpu_clock == 7 {
            self.irq_line = self.irq_level_detector;
            if self.apu_status_frame_interrupt && !self.apu_frame_counter_inhibit_irq {
                self.irq_level_detector = true;
            }
            if let Some(cart) = self.cart.as_mut() {
                if cart.mapper_chip.cpu_clock_rise(self.ppu_address_bus) {
                    self.irq_level_detector = true;
                }
            }
        }

        if self.ppu_clock == 4 {
            self.ppu_clock = 0;
            self.emulate_ppu();
        }

        if self.ppu_clock == 2 {
            self.emulate_half_ppu();
        }

        if self.cpu_clock == 0 {
            self.emulate_apu();
            self.apu_put_cycle = !self.apu_put_cycle;
        }

        self.ppu_clock += 1;
        self.cpu_clock += 1;
    }

    // pal core logic
    fn emulator_core_pal(&mut self) {
        self.master_cycle_counter += 1;
        if self.cpu_clock == 16 {
            self.cpu_clock = 0;
            if let Some(cart) = self.cart.as_mut() {
                let irq = cart.mapper_chip.cpu_clock(1);
                if cart.mapper_chip.cpu_clock_irq_level() {
                    self.irq_level_detector = irq;
                } else if irq {
                    self.irq_level_detector = true;
                }
            }
            self.cpu_tick();
            self.total_cycles += 1;
        }

        if self.cpu_clock == 5 {
            self.nmi_line |= self.ppu_control_nmi_enabled && self.ppu_status_vblank;
            if self.operation_cycle == 0 && !(self.ppu_control_nmi_enabled && self.ppu_status_vblank) {
                self.nmi_line = false;
            }
        }

        if self.cpu_clock == 9 {
            self.irq_line = self.irq_level_detector;
            if self.apu_status_frame_interrupt && !self.apu_frame_counter_inhibit_irq {
                self.irq_level_detector = true;
            }
            if let Some(cart) = self.cart.as_mut() {
                if cart.mapper_chip.cpu_clock_rise(self.ppu_address_bus) {
                    self.irq_level_detector = true;
                }
            }
        }

        if self.ppu_clock == 5 {
            self.ppu_clock = 0;
            self.emulate_ppu();
        }

        if self.ppu_clock == 2 {
            self.emulate_half_ppu();
        }

        if self.cpu_clock == 0 {
            self.emulate_apu();
            self.apu_put_cycle = !self.apu_put_cycle;
        }

        self.ppu_clock += 1;
        self.cpu_clock += 1;
    }

    // dendy core logic
    fn emulator_core_dendy(&mut self) {
        self.master_cycle_counter += 1;
        if self.cpu_clock == 15 {
            self.cpu_clock = 0;
            if let Some(cart) = self.cart.as_mut() {
                let irq = cart.mapper_chip.cpu_clock(1);
                if cart.mapper_chip.cpu_clock_irq_level() {
                    self.irq_level_detector = irq;
                } else if irq {
                    self.irq_level_detector = true;
                }
            }
            self.cpu_tick();
            self.total_cycles += 1;
        }

        if self.cpu_clock == 5 {
            self.nmi_line |= self.ppu_control_nmi_enabled && self.ppu_status_vblank;
            if self.operation_cycle == 0 && !(self.ppu_control_nmi_enabled && self.ppu_status_vblank) {
                self.nmi_line = false;
            }
        }

        if self.cpu_clock == 9 {
            self.irq_line = self.irq_level_detector;
            if self.apu_status_frame_interrupt && !self.apu_frame_counter_inhibit_irq {
                self.irq_level_detector = true;
            }
            if let Some(cart) = self.cart.as_mut() {
                if cart.mapper_chip.cpu_clock_rise(self.ppu_address_bus) {
                    self.irq_level_detector = true;
                }
            }
        }

        if self.ppu_clock == 5 {
            self.ppu_clock = 0;
            self.emulate_ppu();
        }

        if self.ppu_clock == 2 {
            self.emulate_half_ppu();
        }

        if self.cpu_clock == 0 {
            self.emulate_apu();
            self.apu_put_cycle = !self.apu_put_cycle;
        }

        self.ppu_clock += 1;
        self.cpu_clock += 1;
    }

    // zapper light hit detection
    pub fn zapper_check_hit(&self) -> bool {
        if self.zapper_bogo.load(Ordering::Relaxed) > 0 {
            return false;
        }
        let zx = *self.zapper_x.lock().unwrap();
        let zy = *self.zapper_y.lock().unwrap();
        let sx = (zx * 255.0).round() as usize;
        let sy = (zy * 239.0).round() as usize;
        let sx = sx.min(255);
        let sy = sy.min(239);
        let pixel = self.screen[sy * 256 + sx];
        let r = (pixel >> 16) & 0xFF;
        let g = (pixel >> 8) & 0xFF;
        let b = pixel & 0xFF;
        (r as u32 + g as u32 + b as u32) >= 300
    }

    pub fn save_state_to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(self.ppu_clock);
        out.push(self.cpu_clock);
        out.extend_from_slice(&self.program_counter.to_le_bytes());
        out.push(self.stack_pointer);
        out.push(self.a);
        out.push(self.x);
        out.push(self.y);
        out.push(self.h);
        out.push(if self.ignore_h { 1 } else { 0 });
        out.push(self.data_bus);
        out.push(self.internal_bus);
        out.extend_from_slice(&self.address_bus.to_le_bytes());
        out.push(self.special_bus);
        out.push(self.dl);
        out.push(self.op_code);
        out.push(self.operation_cycle);
        out.extend_from_slice(&self.temporary_address.to_le_bytes());
        out.extend_from_slice(&self.total_cycles.to_le_bytes());
        out.push(if self.flag_carry { 1 } else { 0 });
        out.push(if self.flag_zero { 1 } else { 0 });
        out.push(if self.flag_interrupt { 1 } else { 0 });
        out.push(if self.flag_decimal { 1 } else { 0 });
        out.push(if self.flag_overflow { 1 } else { 0 });
        out.push(if self.flag_negative { 1 } else { 0 });
        out.push(if self.cpu_read { 1 } else { 0 });
        out.push(if self.do_brk { 1 } else { 0 });
        out.push(if self.do_nmi { 1 } else { 0 });
        out.push(if self.do_irq { 1 } else { 0 });
        out.push(if self.do_reset { 1 } else { 0 });
        out.push(if self.fix_high_byte { 1 } else { 0 });
        out.push(if self.do_oam_dma { 1 } else { 0 });
        out.push(if self.first_cycle_of_oam_dma { 1 } else { 0 });
        out.push(if self.do_dmc_dma { 1 } else { 0 });
        out.push(self.dmc_dma_delay);
        out.push(self.cannot_run_dmc_dma_right_now);
        out.push(self.dma_page);
        out.push(self.dma_address);
        out.push(if self.oam_dma_aligned { 1 } else { 0 });
        out.push(if self.oam_dma_halt { 1 } else { 0 });
        out.push(if self.dmc_dma_halt { 1 } else { 0 });
        out.push(self.oam_internal_bus);
        out.push(if self.nmi_pins_signal { 1 } else { 0 });
        out.push(if self.nmi_previous_pins_signal { 1 } else { 0 });
        out.push(if self.irq_level_detector { 1 } else { 0 });
        out.push(if self.nmi_line { 1 } else { 0 });
        out.push(if self.irq_line { 1 } else { 0 });
        out.extend_from_slice(&self.cpu_ram_mask.to_le_bytes());
        out.extend_from_slice(&self.ram);
        out.extend_from_slice(&self.vram);
        out.extend_from_slice(&self.oam);
        out.extend_from_slice(&self.oam2);
        out.extend_from_slice(&self.palette_ram);
        out.push(self.ppu_bus);
        for v in &self.ppu_bus_decay { out.extend_from_slice(&v.to_le_bytes()); }
        out.push(self.ppu_oam_address);
        out.push(if self.ppu_status_vblank { 1 } else { 0 });
        out.push(if self.ppu_status_sprite_zero_hit { 1 } else { 0 });
        out.push(if self.ppu_status_sprite_zero_hit_delayed { 1 } else { 0 });
        out.push(if self.ppu_status_sprite_overflow { 1 } else { 0 });
        out.push(if self.ppu_status_sprite_overflow_delayed { 1 } else { 0 });
        out.push(if self.ppu_status_pending_sprite_zero_hit { 1 } else { 0 });
        out.push(if self.ppu_status_pending_sprite_zero_hit2 { 1 } else { 0 });
        out.push(if self.ppu_pending_vblank { 1 } else { 0 });
        out.push(if self.ppu_vset { 1 } else { 0 });
        out.push(if self.ppu_vset_latch1 { 1 } else { 0 });
        out.push(if self.ppu_vset_latch2 { 1 } else { 0 });
        out.push(if self.ppu_read_2002 { 1 } else { 0 });
        out.extend_from_slice(&self.ppu_v.to_le_bytes());
        out.extend_from_slice(&self.ppu_t.to_le_bytes());
        out.push(self.ppu_fine_x_scroll);
        out.push(if self.ppu_addr_latch { 1 } else { 0 });
        out.push(if self.ppu_control_increment_mode_32 { 1 } else { 0 });
        out.push(if self.ppu_control_nmi_enabled { 1 } else { 0 });
        out.push(if self.ppu_sprite_x16 { 1 } else { 0 });
        out.push(if self.ppu_pattern_select_sprites { 1 } else { 0 });
        out.push(if self.ppu_pattern_select_background { 1 } else { 0 });
        out.extend_from_slice(&self.ppu_scanline.to_le_bytes());
        out.extend_from_slice(&self.ppu_dot.to_le_bytes());
        out.push(if self.ppu_odd_frame { 1 } else { 0 });
        out.extend_from_slice(&self.ppu_address_bus.to_le_bytes());
        out.push(if self.ppu_ale { 1 } else { 0 });
        out.push(self.ppu_octal_latch);
        out.push(self.ppu_read_buffer);
        out.push(if self.ppu_reset { 1 } else { 0 });
        out.push(if self.ppu_mask_greyscale { 1 } else { 0 });
        out.push(if self.ppu_mask_8px_show_background { 1 } else { 0 });
        out.push(if self.ppu_mask_8px_show_sprites { 1 } else { 0 });
        out.push(if self.ppu_mask_show_background { 1 } else { 0 });
        out.push(if self.ppu_mask_show_sprites { 1 } else { 0 });
        out.push(if self.ppu_mask_emphasize_red { 1 } else { 0 });
        out.push(if self.ppu_mask_emphasize_green { 1 } else { 0 });
        out.push(if self.ppu_mask_emphasize_blue { 1 } else { 0 });
        out.push(if self.ppu_mask_show_background_instant { 1 } else { 0 });
        out.push(if self.ppu_mask_show_sprites_instant { 1 } else { 0 });
        out.push(if self.ppu_mask_show_background_delayed { 1 } else { 0 });
        out.push(if self.ppu_mask_show_sprites_delayed { 1 } else { 0 });
        out.extend_from_slice(&self.ppu_bg_pattern_sr_l.to_le_bytes());
        out.extend_from_slice(&self.ppu_bg_pattern_sr_h.to_le_bytes());
        out.extend_from_slice(&self.ppu_bg_pattern_sr_l2.to_le_bytes());
        out.extend_from_slice(&self.ppu_bg_pattern_sr_h2.to_le_bytes());
        out.extend_from_slice(&self.ppu_bg_attr_sr_l.to_le_bytes());
        out.extend_from_slice(&self.ppu_bg_attr_sr_h.to_le_bytes());
        out.push(self.ppu_attr_latch_register);
        out.push(self.ppu_low_bit_plane);
        out.push(self.ppu_high_bit_plane);
        out.push(self.ppu_low_bit_plane_hi);
        out.push(self.ppu_high_bit_plane_hi);
        out.push(self.ppu_attribute);
        out.extend_from_slice(&self.ppu_sprite_sr_l);
        out.extend_from_slice(&self.ppu_sprite_sr_h);
        out.extend_from_slice(&self.ppu_sprite_sr_l2);
        out.extend_from_slice(&self.ppu_sprite_sr_h2);
        out.extend_from_slice(&self.ppu_sprite_attribute);
        out.extend_from_slice(&self.ppu_sprite_pattern);
        out.extend_from_slice(&self.ppu_sprite_x_position);
        out.extend_from_slice(&self.ppu_sprite_y_position);
        out.extend_from_slice(&self.ppu_sprite_shifter_counter);
        out.push(self.ppu_sprite_pattern_l);
        out.push(self.ppu_sprite_pattern_h);
        out.push(self.ppu_sprite_pattern_l2);
        out.push(self.ppu_sprite_pattern_h2);
        out.push(if self.ppu_next_scanline_contains_sprite_zero { 1 } else { 0 });
        out.push(if self.ppu_current_scanline_contains_sprite_zero { 1 } else { 0 });
        out.push(if self.ppu_can_detect_sprite_zero_hit { 1 } else { 0 });
        out.push(self.oam2_address);
        out.push(if self.secondary_oam_full { 1 } else { 0 });
        out.push(self.sprite_evaluation_tick);
        out.push(if self.oam_address_overflowed_during_sprite_evaluation { 1 } else { 0 });
        out.push(self.ppu_oam_latch);
        out.push(self.ppu_oam_buffer);
        out.push(self.ppu_render_temp);
        out.extend_from_slice(&self.in_range_check.to_le_bytes());
        out.push(if self.nine_objects_on_this_scanline { 1 } else { 0 });
        out.push(if self.ppu_oam_corruption_rendering_disabled_out_of_vblank { 1 } else { 0 });
        out.push(if self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant { 1 } else { 0 });
        out.push(if self.ppu_v_register_changed_out_of_vblank { 1 } else { 0 });
        out.push(if self.ppu_pending_oam_corruption { 1 } else { 0 });
        out.push(self.ppu_oam_corruption_index);
        out.push(if self.ppu_oam_corruption_rendering_enabled_out_of_vblank { 1 } else { 0 });
        out.push(if self.ppu_oam_evaluation_corruption_odd_cycle { 1 } else { 0 });
        out.push(if self.ppu_oam_evaluation_object_in_range { 1 } else { 0 });
        out.push(if self.ppu_oam_evaluation_object_in_x_range { 1 } else { 0 });
        out.push(if self.ppu_palette_corruption_rendering_disabled_out_of_vblank { 1 } else { 0 });
        out.push(if self.oam_corrupted_on_odd_cycle { 1 } else { 0 });
        out.push(self.ppu_update_2006_delay);
        out.push(self.ppu_update_2005_delay);
        out.push(self.ppu_update_2001_delay);
        out.push(self.ppu_update_2001_oam_corruption_delay);
        out.push(self.ppu_update_2001_emphasis_bits_delay);
        out.push(self.ppu_update_2005_value);
        out.push(self.ppu_update_2001_value);
        out.extend_from_slice(&self.ppu_update_2006_value.to_le_bytes());
        out.extend_from_slice(&self.ppu_update_2006_value_temp.to_le_bytes());
        out.push(if self.ppu_was_rendering_before_2001_write { 1 } else { 0 });
        out.push(if self.ppu_2007_read { 1 } else { 0 });
        out.push(if self.ppu_2007_read_sr { 1 } else { 0 });
        for v in &self.ppu_2007_read_latches { out.push(if *v { 1 } else { 0 }); }
        out.push(if self.ppu_2007_pd_rb { 1 } else { 0 });
        out.push(if self.ppu_2007_read_ale { 1 } else { 0 });
        out.push(if self.ppu_2007_read_h0_latch { 1 } else { 0 });
        out.push(if self.ppu_2007_read_xrb { 1 } else { 0 });
        out.push(if self.ppu_read { 1 } else { 0 });
        out.push(if self.ppu_2007_write { 1 } else { 0 });
        out.push(if self.ppu_2007_write_sr { 1 } else { 0 });
        for v in &self.ppu_2007_write_latches { out.push(if *v { 1 } else { 0 }); }
        out.push(if self.ppu_2007_db_par { 1 } else { 0 });
        out.push(if self.ppu_2007_write_ale { 1 } else { 0 });
        out.push(if self.ppu_2007_tstep_latch { 1 } else { 0 });
        out.push(if self.ppu_2007_tstep { 1 } else { 0 });
        out.push(if self.ppu_2007_blnk_latch { 1 } else { 0 });
        out.push(if self.ppu_2007_palette_ram_enable { 1 } else { 0 });
        out.push(self.ppu_2007_write_data);
        out.push(if self.ppu_write { 1 } else { 0 });
        out.extend_from_slice(&self.ppu_pattern_address_register_nt.to_le_bytes());
        out.extend_from_slice(&self.ppu_pattern_address_register_at.to_le_bytes());
        out.extend_from_slice(&self.ppu_pattern_address_register_chr.to_le_bytes());
        out.push(if self.ppu_commit_nametable_fetch { 1 } else { 0 });
        out.push(if self.ppu_commit_attribute_fetch { 1 } else { 0 });
        out.push(if self.ppu_commit_pattern_low_fetch { 1 } else { 0 });
        out.push(if self.ppu_commit_pattern_high_fetch { 1 } else { 0 });
        out.push(if self.ppu_a12_prev { 1 } else { 0 });
        out.push(if self.copy_v { 1 } else { 0 });
        out.push(if self.skipped_pre_render_dot_341 { 1 } else { 0 });
        out.push(self.dot_color);
        out.push(self.prev_dot_color);
        out.push(self.prev_prev_dot_color);
        out.push(self.prev_prev_prev_dot_color);
        out.push(self.palette_ram_address);
        out.push(if self.this_dot_read_from_palette_ram { 1 } else { 0 });
        out.push(if self.apu_put_cycle { 1 } else { 0 });
        out.push(self.apu_alignment);
        out.push(if self.apu_status_dmc_interrupt { 1 } else { 0 });
        out.push(if self.apu_status_frame_interrupt { 1 } else { 0 });
        out.push(if self.apu_status_dmc { 1 } else { 0 });
        out.push(if self.apu_status_delayed_dmc { 1 } else { 0 });
        out.push(if self.apu_status_noise { 1 } else { 0 });
        out.push(if self.apu_status_triangle { 1 } else { 0 });
        out.push(if self.apu_status_pulse2 { 1 } else { 0 });
        out.push(if self.apu_status_pulse1 { 1 } else { 0 });
        out.push(if self.clearing_apu_frame_interrupt { 1 } else { 0 });
        out.push(self.apu_delayed_dmc_4015);
        out.push(if self.apu_implicit_abort_dmc_4015 { 1 } else { 0 });
        out.push(if self.apu_set_implicit_abort_dmc_4015 { 1 } else { 0 });
        out.extend_from_slice(&self.apu_register);
        out.push(if self.apu_frame_counter_mode { 1 } else { 0 });
        out.push(if self.apu_frame_counter_inhibit_irq { 1 } else { 0 });
        out.push(self.apu_frame_counter_reset);
        out.extend_from_slice(&self.apu_framecounter.to_le_bytes());
        out.push(if self.apu_quarter_frame_clock { 1 } else { 0 });
        out.push(if self.apu_half_frame_clock { 1 } else { 0 });
        out.push(if self.apu_envelope_start_flag { 1 } else { 0 });
        out.push(if self.apu_envelope_divider_clock { 1 } else { 0 });
        out.push(self.apu_envelope_decay_level);
        out.push(self.apu_length_counter_pulse1);
        out.push(self.apu_length_counter_pulse2);
        out.push(self.apu_length_counter_triangle);
        out.push(self.apu_length_counter_noise);
        out.push(if self.apu_length_counter_halt_pulse1 { 1 } else { 0 });
        out.push(if self.apu_length_counter_halt_pulse2 { 1 } else { 0 });
        out.push(if self.apu_length_counter_halt_triangle { 1 } else { 0 });
        out.push(if self.apu_length_counter_halt_noise { 1 } else { 0 });
        out.push(if self.apu_length_counter_reload_pulse1 { 1 } else { 0 });
        out.push(if self.apu_length_counter_reload_pulse2 { 1 } else { 0 });
        out.push(if self.apu_length_counter_reload_triangle { 1 } else { 0 });
        out.push(if self.apu_length_counter_reload_noise { 1 } else { 0 });
        out.push(self.apu_length_counter_reload_value_pulse1);
        out.push(self.apu_length_counter_reload_value_pulse2);
        out.push(self.apu_length_counter_reload_value_triangle);
        out.push(self.apu_length_counter_reload_value_noise);
        out.extend_from_slice(&self.apu_channel_timer_pulse1.to_le_bytes());
        out.extend_from_slice(&self.apu_channel_timer_pulse2.to_le_bytes());
        out.extend_from_slice(&self.apu_channel_timer_triangle.to_le_bytes());
        out.extend_from_slice(&self.apu_channel_timer_noise.to_le_bytes());
        out.extend_from_slice(&self.apu_channel_timer_dmc.to_le_bytes());
        out.push(if self.apu_dmc_enable_irq { 1 } else { 0 });
        out.push(if self.apu_dmc_loop { 1 } else { 0 });
        out.extend_from_slice(&self.apu_dmc_rate.to_le_bytes());
        out.push(self.apu_dmc_output);
        out.extend_from_slice(&self.apu_dmc_sample_address.to_le_bytes());
        out.extend_from_slice(&self.apu_dmc_sample_length.to_le_bytes());
        out.extend_from_slice(&self.apu_dmc_bytes_remaining.to_le_bytes());
        out.push(self.apu_dmc_buffer);
        out.extend_from_slice(&self.apu_dmc_address_counter.to_le_bytes());
        out.push(self.apu_dmc_shifter);
        out.push(self.apu_dmc_shifter_bits_remaining);
        out.push(if self.dpcm_up { 1 } else { 0 });
        out.push(if self.apu_silent { 1 } else { 0 });
        out.extend_from_slice(&self.audio_cycles_accumulator.to_le_bytes());
        out.extend_from_slice(&self.audio_sample_accumulator.to_le_bytes());
        out.extend_from_slice(&self.audio_sample_count.to_le_bytes());
        out.extend_from_slice(&self.audio_host_sample_rate.to_le_bytes());
        out.extend_from_slice(&self.audio_frame_cycle.to_le_bytes());
        out.extend_from_slice(&self.audio_previous_output.to_le_bytes());
        out.extend_from_slice(&self.filter_lp_alpha.to_le_bytes());
        out.extend_from_slice(&self.filter_lp_prev_out.to_le_bytes());
        out.extend_from_slice(&self.filter_hp1_alpha.to_le_bytes());
        out.extend_from_slice(&self.filter_hp1_prev_in.to_le_bytes());
        out.extend_from_slice(&self.filter_hp1_prev_out.to_le_bytes());
        out.extend_from_slice(&self.filter_hp2_alpha.to_le_bytes());
        out.extend_from_slice(&self.filter_hp2_prev_in.to_le_bytes());
        out.extend_from_slice(&self.filter_hp2_prev_out.to_le_bytes());
        out.extend_from_slice(&self.pulse1_timer.to_le_bytes());
        out.push(self.pulse1_sequencer_step);
        out.push(self.pulse1_envelope_divider);
        out.push(self.pulse1_envelope_decay_level);
        out.push(if self.pulse1_envelope_start_flag { 1 } else { 0 });
        out.push(self.pulse1_sweep_divider);
        out.push(if self.pulse1_sweep_reload { 1 } else { 0 });
        out.extend_from_slice(&self.pulse2_timer.to_le_bytes());
        out.push(self.pulse2_sequencer_step);
        out.push(self.pulse2_envelope_divider);
        out.push(self.pulse2_envelope_decay_level);
        out.push(if self.pulse2_envelope_start_flag { 1 } else { 0 });
        out.push(self.pulse2_sweep_divider);
        out.push(if self.pulse2_sweep_reload { 1 } else { 0 });
        out.extend_from_slice(&self.triangle_timer.to_le_bytes());
        out.push(self.triangle_sequencer_step);
        out.push(self.triangle_linear_counter);
        out.push(if self.triangle_linear_counter_reload_flag { 1 } else { 0 });
        out.extend_from_slice(&self.noise_timer.to_le_bytes());
        out.extend_from_slice(&self.noise_shift_register.to_le_bytes());
        out.push(self.noise_envelope_divider);
        out.push(self.noise_envelope_decay_level);
        out.push(if self.noise_envelope_start_flag { 1 } else { 0 });
        out.push(if self.apu_controller_ports_strobing { 1 } else { 0 });
        out.push(if self.apu_controller_ports_strobed { 1 } else { 0 });
        out.push(self.controller_port1.load(Ordering::Relaxed));
        out.push(self.controller_port2.load(Ordering::Relaxed));
        out.push(self.controller_shift_register1);
        out.push(self.controller_shift_register2);
        out.push(self.controller1_shift_counter);
        out.push(self.controller2_shift_counter);
        out.extend_from_slice(&self.powerpad_shift_data[0].to_le_bytes());
        out.extend_from_slice(&self.powerpad_shift_data[1].to_le_bytes());
        out.push(self.powerpad_shift_count[0]);
        out.push(self.powerpad_shift_count[1]);
        out.push(self.paddle_readbit[0]);
        out.push(self.paddle_readbit[1]);
        out.push(self.paddle_readbit[2]);
        out.push(self.snes_readbit[0]);
        out.push(self.snes_readbit[1]);
        out.push(self.snes_mouse_readbit[0]);
        out.push(self.snes_mouse_readbit[1]);
        out.push(self.subor_mouse_latch[0]);
        out.push(self.subor_mouse_latch[1]);
        out.extend_from_slice(&self.hori_track_state[0].to_le_bytes());
        out.extend_from_slice(&self.hori_track_state[1].to_le_bytes());
        out.push(self.hori_track_readbit[0]);
        out.push(self.hori_track_readbit[1]);
        out.push(self.fourscore_readbit[0]);
        out.push(self.fourscore_readbit[1]);
        out.push(self.expansion_adapter_ports[0].load(Ordering::Relaxed));
        out.push(self.expansion_adapter_ports[1].load(Ordering::Relaxed));
        out.push(self.expansion_adapter_ports[2].load(Ordering::Relaxed));
        out.push(self.expansion_adapter_ports[3].load(Ordering::Relaxed));
        out.push(self.expansion_adapter_shift_register[0]);
        out.push(self.expansion_adapter_shift_register[1]);
        out.push(self.expansion_adapter_shift_register[2]);
        out.push(self.expansion_adapter_shift_register[3]);
        out.push(if self.oeka_strobe { 1 } else { 0 });
        out.push(if self.oeka_shift { 1 } else { 0 });
        out.extend_from_slice(&self.oeka_state_buffer.to_le_bytes());
        out.push(self.family_trainer_ignore_rows);
        out.extend_from_slice(&self.family_trainer_state.lock().unwrap()[..]);
        out.extend_from_slice(&self.hyper_shot_state.lock().unwrap()[..]);
        out.push(if self.hyper_shot_enable_p1 { 1 } else { 0 });
        out.push(if self.hyper_shot_enable_p2 { 1 } else { 0 });
        out.extend_from_slice(&self.bandai_hyper_state[0].to_le_bytes());
        out.extend_from_slice(&self.bandai_hyper_state[1].to_le_bytes());
        out.push(self.bandai_hyper_readbit[0]);
        out.push(self.bandai_hyper_readbit[1]);
        out.extend_from_slice(&self.turbo_file_data);
        out.extend_from_slice(&self.turbo_file_position.to_le_bytes());
        out.push(self.turbo_file_last_write);
        out.extend_from_slice(&self.battle_box_data);
        out.push(self.battle_box_last_write);
        out.push(self.battle_box_address);
        out.push(self.battle_box_chip_select);
        out.push(self.battle_box_output);
        out.push(if self.battle_box_write_enabled { 1 } else { 0 });
        out.push(self.battle_box_input_bit_position);
        out.extend_from_slice(&self.battle_box_input_data.to_le_bytes());
        out.push(if self.battle_box_is_write { 1 } else { 0 });
        out.push(if self.battle_box_is_read { 1 } else { 0 });
        out.extend_from_slice(&self.family_basic_state.lock().unwrap()[..]);
        out.push(self.family_basic_row);
        out.push(self.family_basic_column);
        out.push(if self.family_basic_enabled { 1 } else { 0 });
        out.extend_from_slice(&self.party_tap_state.lock().unwrap()[..]);
        out.push(self.party_tap_buffer);
        out.push(self.party_tap_read_count);
        out.push(if self.party_tap_strobe { 1 } else { 0 });
        out.extend_from_slice(&self.pachinko_state.lock().unwrap()[..]);
        out.push(self.pachinko_analog);
        out.extend_from_slice(&self.pachinko_buffer.to_le_bytes());
        out.push(if self.pachinko_strobe { 1 } else { 0 });
        out.extend_from_slice(&self.punching_bag_state.lock().unwrap()[..]);
        out.push(self.punching_bag_selected_sensors);
        out.extend_from_slice(&self.jissen_mahjong_state.lock().unwrap()[..]);
        out.push(self.jissen_mahjong_row);
        out.extend_from_slice(&self.jissen_mahjong_state_buffer.to_le_bytes());
        out.push(if self.jissen_mahjong_strobe { 1 } else { 0 });
        out.extend_from_slice(&self.subor_keyboard_state.lock().unwrap()[..]);
        out.push(self.subor_keyboard_row);
        out.push(self.subor_keyboard_column);
        out.push(if self.subor_keyboard_enabled { 1 } else { 0 });
        out.extend_from_slice(&self.master_cycle_counter.to_le_bytes());
        out.extend_from_slice(&self.barcode_battler_stream);
        out.extend_from_slice(&self.barcode_battler_insert_cycle.to_le_bytes());
        out.push(if self.barcode_battler_active { 1 } else { 0 });
        out.push(self.virtualboy_readbit[0]);
        out.push(self.virtualboy_readbit[1]);
        out.extend_from_slice(&self.virtualboy_state_buffer[0].to_le_bytes());
        out.extend_from_slice(&self.virtualboy_state_buffer[1].to_le_bytes());
        out.push(if self.data_pins_are_not_floating { 1 } else { 0 });
        out.push(if self.frame_advance_reached_vblank { 1 } else { 0 });
        if let Some(cart) = &self.cart {
            let mapper_state = cart.mapper_chip.save_mapper_registers(cart);
            out.extend_from_slice(&(mapper_state.len() as u32).to_le_bytes());
            out.extend_from_slice(&mapper_state);
        } else {
            out.extend_from_slice(&0u32.to_le_bytes());
        }
        out.extend_from_slice(&self.um6578_extra_ram);
        for bank in &self.um6578_vram { out.extend_from_slice(bank); }
        out.push(self.um6578_reg2008);
        out.push(self.um6578_color_mask);
        out.push(self.um6578_dma_control);
        out.push(self.um6578_dma_page);
        out.extend_from_slice(&self.um6578_dma_source.to_le_bytes());
        out.extend_from_slice(&self.um6578_dma_target.to_le_bytes());
        out.extend_from_slice(&self.um6578_dma_length.to_le_bytes());
        out.extend_from_slice(&self.um6578_dma_busy.to_le_bytes());
        out.push(self.um6578_nt_tile_byte);
        out.push(self.um6578_nt_attr_byte);
        out.push(self.um6578_bg_palette);
        out.push(self.um6578_bg_palette_lo);
        out.extend_from_slice(&self.um6578_chr_ram);
        out.extend_from_slice(&self.pec586_keyboard_state.lock().unwrap()[..]);
        out.push(self.pec586_kspos);
        out.push(self.pec586_ksindex);
        out.push(self.pec586_kstrobe);
        out.extend_from_slice(&self.bit79_keyboard_state.lock().unwrap()[..]);
        out.push(self.bit79_keyboard_row);
        out.push(self.bit79_keyboard_column);
        out.push(self.bit79_keyboard_strobe);
        out.extend_from_slice(&self.keda_keyboard_state.lock().unwrap()[..]);
        out.push(self.keda_keyboard_row);
        out.push(self.keda_keyboard_column);
        out.push(self.keda_keyboard_strobe);
        out.extend_from_slice(&self.kingwon_keyboard_state.lock().unwrap()[..]);
        out.push(self.kingwon_keyboard_row);
        out.push(self.kingwon_keyboard_column);
        out.push(self.kingwon_keyboard_bits);
        out.push(self.zecheng_keyboard_bits1);
        out.push(self.zecheng_keyboard_bits2);
        out.push(self.zecheng_keyboard_strobe as u8);
        out.push(*self.zecheng_keyboard_buttons.lock().unwrap());
        out.push(*self.zecheng_keyboard_keys.lock().unwrap());
        out.extend_from_slice(&self.quiz_king_buttons.lock().unwrap()[..]);
        out.push(self.quiz_king_data_r);
        out.push(if self.quiz_king_funky_mode { 1 } else { 0 });
        out.push(if self.quiz_king_strobe { 1 } else { 0 });
        out.extend_from_slice(&self.top_rider_buttons.lock().unwrap()[..]);
        out.extend_from_slice(&self.top_rider_bs.to_le_bytes());
        out.extend_from_slice(&self.top_rider_bss.to_le_bytes());
        out.extend_from_slice(&self.top_rider_boop.to_le_bytes());
        out.extend_from_slice(&self.fami_net_sys_buttons.lock().unwrap()[..]);
        out.extend_from_slice(&self.fami_net_sys_data.to_le_bytes());
        out.push(self.fami_net_sys_readbit);
        out.push(if self.fami_net_sys_prev_strobe { 1 } else { 0 });
        out.extend_from_slice(&self.city_patrolman_input.lock().unwrap()[..]);
        out.push(self.city_patrolman_strobe);
        out.extend_from_slice(&self.city_patrolman_time_out.to_le_bytes());
        out.extend_from_slice(&self.city_patrolman_shift_reg.to_le_bytes());
        out.extend_from_slice(&self.moguraa_buttons.lock().unwrap()[..]);
        out.extend_from_slice(&self.moguraa_bits.to_le_bytes());
        out.push(self.moguraa_sel);
        out.extend_from_slice(&self.golden_nugget_buttons.lock().unwrap()[..]);
        out.push(self.golden_nugget_strobe);
        out.push(self.golden_nugget_shift);
        out.extend_from_slice(&self.abl_pinball_buttons.lock().unwrap()[..]);
        out.extend_from_slice(&self.abl_pinball_count.to_le_bytes());
        out.extend_from_slice(&self.abl_pinball_plunger.to_le_bytes());
        out.extend_from_slice(&self.abl_pinball_plunger_reset_count.to_le_bytes());
        out.extend_from_slice(&self.tv_pump_buttons.lock().unwrap()[..]);
        out.extend_from_slice(&self.triface_mahjong_buttons.lock().unwrap()[..]);
        out.push(self.triface_mahjong_column);
        out.push(self.triface_mahjong_row);
        out.extend_from_slice(&self.triface_mahjong_keys);
        out.extend_from_slice(&self.mahjong_gekitou_buttons.lock().unwrap()[..]);
        out.push(self.mahjong_gekitou_bits);
        out.push(self.mahjong_gekitou_bit_ptr);
        out.push(self.mahjong_gekitou_strobe);
        out.push(self.ppu_oam_read_latch);
        out.push(self.oam2_reset_signal);
        out
    }

    pub fn load_state_from_bytes(&mut self, data: &[u8]) -> Result<(), String> {
        let mut p = 0;
        let mut read_u8 = || -> Result<u8, String> { if p < data.len() { let v = data[p]; p+=1; Ok(v) } else { Err("EOF".to_string()) } };
        self.ppu_clock = read_u8()?;
        self.cpu_clock = read_u8()?;
        self.program_counter = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.stack_pointer = read_u8()?;
        self.a = read_u8()?;
        self.x = read_u8()?;
        self.y = read_u8()?;
        self.h = read_u8()?;
        self.ignore_h = read_u8()? != 0;
        self.data_bus = read_u8()?;
        self.internal_bus = read_u8()?;
        self.address_bus = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.special_bus = read_u8()?;
        self.dl = read_u8()?;
        self.op_code = read_u8()?;
        self.operation_cycle = read_u8()?;
        self.temporary_address = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.total_cycles = u64::from_le_bytes([
            read_u8()?, read_u8()?, read_u8()?, read_u8()?,
            read_u8()?, read_u8()?, read_u8()?, read_u8()?,
        ]);
        self.flag_carry = read_u8()? != 0;
        self.flag_zero = read_u8()? != 0;
        self.flag_interrupt = read_u8()? != 0;
        self.flag_decimal = read_u8()? != 0;
        self.flag_overflow = read_u8()? != 0;
        self.flag_negative = read_u8()? != 0;
        self.cpu_read = read_u8()? != 0;
        self.do_brk = read_u8()? != 0;
        self.do_nmi = read_u8()? != 0;
        self.do_irq = read_u8()? != 0;
        self.do_reset = read_u8()? != 0;
        self.fix_high_byte = read_u8()? != 0;
        self.do_oam_dma = read_u8()? != 0;
        self.first_cycle_of_oam_dma = read_u8()? != 0;
        self.do_dmc_dma = read_u8()? != 0;
        self.dmc_dma_delay = read_u8()?;
        self.cannot_run_dmc_dma_right_now = read_u8()?;
        self.dma_page = read_u8()?;
        self.dma_address = read_u8()?;
        self.oam_dma_aligned = read_u8()? != 0;
        self.oam_dma_halt = read_u8()? != 0;
        self.dmc_dma_halt = read_u8()? != 0;
        self.oam_internal_bus = read_u8()?;
        self.nmi_pins_signal = read_u8()? != 0;
        self.nmi_previous_pins_signal = read_u8()? != 0;
        self.irq_level_detector = read_u8()? != 0;
        self.nmi_line = read_u8()? != 0;
        self.irq_line = read_u8()? != 0;
        self.cpu_ram_mask = u16::from_le_bytes([read_u8()?, read_u8()?]);
        let is_new_save = data.len() > 5000;
        if is_new_save {
            for i in 0..self.ram.len() { self.ram[i] = read_u8()?; }
        } else {
            for i in 0..0x1000 { self.ram[i] = read_u8()?; }
            for i in 0x1000..self.ram.len() { self.ram[i] = 0; }
        }
        for i in 0..self.vram.len() { self.vram[i] = read_u8()?; }
        for i in 0..self.oam.len() { self.oam[i] = read_u8()?; }
        for i in 0..self.oam2.len() { self.oam2[i] = read_u8()?; }
        for i in 0..self.palette_ram.len() { self.palette_ram[i] = read_u8()?; }
        self.ppu_bus = read_u8()?;
        for i in 0..self.ppu_bus_decay.len() { self.ppu_bus_decay[i] = i32::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?]); }
        self.ppu_oam_address = read_u8()?;
        self.ppu_status_vblank = read_u8()? != 0;
        self.ppu_status_sprite_zero_hit = read_u8()? != 0;
        self.ppu_status_sprite_zero_hit_delayed = read_u8()? != 0;
        self.ppu_status_sprite_overflow = read_u8()? != 0;
        self.ppu_status_sprite_overflow_delayed = read_u8()? != 0;
        self.ppu_status_pending_sprite_zero_hit = read_u8()? != 0;
        self.ppu_status_pending_sprite_zero_hit2 = read_u8()? != 0;
        self.ppu_pending_vblank = read_u8()? != 0;
        self.ppu_vset = read_u8()? != 0;
        self.ppu_vset_latch1 = read_u8()? != 0;
        self.ppu_vset_latch2 = read_u8()? != 0;
        self.ppu_read_2002 = read_u8()? != 0;
        self.ppu_v = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_t = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_fine_x_scroll = read_u8()?;
        self.ppu_addr_latch = read_u8()? != 0;
        self.ppu_control_increment_mode_32 = read_u8()? != 0;
        self.ppu_control_nmi_enabled = read_u8()? != 0;
        self.ppu_sprite_x16 = read_u8()? != 0;
        self.ppu_pattern_select_sprites = read_u8()? != 0;
        self.ppu_pattern_select_background = read_u8()? != 0;
        self.ppu_scanline = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_dot = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_odd_frame = read_u8()? != 0;
        self.ppu_address_bus = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_ale = read_u8()? != 0;
        self.ppu_octal_latch = read_u8()?;
        self.ppu_read_buffer = read_u8()?;
        self.ppu_reset = read_u8()? != 0;
        self.ppu_mask_greyscale = read_u8()? != 0;
        self.ppu_mask_8px_show_background = read_u8()? != 0;
        self.ppu_mask_8px_show_sprites = read_u8()? != 0;
        self.ppu_mask_show_background = read_u8()? != 0;
        self.ppu_mask_show_sprites = read_u8()? != 0;
        self.ppu_mask_emphasize_red = read_u8()? != 0;
        self.ppu_mask_emphasize_green = read_u8()? != 0;
        self.ppu_mask_emphasize_blue = read_u8()? != 0;
        self.ppu_mask_show_background_instant = read_u8()? != 0;
        self.ppu_mask_show_sprites_instant = read_u8()? != 0;
        self.ppu_mask_show_background_delayed = read_u8()? != 0;
        self.ppu_mask_show_sprites_delayed = read_u8()? != 0;
        self.ppu_bg_pattern_sr_l = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_bg_pattern_sr_h = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_bg_pattern_sr_l2 = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_bg_pattern_sr_h2 = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_bg_attr_sr_l = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_bg_attr_sr_h = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_attr_latch_register = read_u8()?;
        self.ppu_low_bit_plane = read_u8()?;
        self.ppu_high_bit_plane = read_u8()?;
        self.ppu_low_bit_plane_hi = read_u8()?;
        self.ppu_high_bit_plane_hi = read_u8()?;
        self.ppu_attribute = read_u8()?;
        for i in 0..self.ppu_sprite_sr_l.len() { self.ppu_sprite_sr_l[i] = read_u8()?; }
        for i in 0..self.ppu_sprite_sr_h.len() { self.ppu_sprite_sr_h[i] = read_u8()?; }
        for i in 0..self.ppu_sprite_sr_l2.len() { self.ppu_sprite_sr_l2[i] = read_u8()?; }
        for i in 0..self.ppu_sprite_sr_h2.len() { self.ppu_sprite_sr_h2[i] = read_u8()?; }
        for i in 0..self.ppu_sprite_attribute.len() { self.ppu_sprite_attribute[i] = read_u8()?; }
        for i in 0..self.ppu_sprite_pattern.len() { self.ppu_sprite_pattern[i] = read_u8()?; }
        for i in 0..self.ppu_sprite_x_position.len() { self.ppu_sprite_x_position[i] = read_u8()?; }
        for i in 0..self.ppu_sprite_y_position.len() { self.ppu_sprite_y_position[i] = read_u8()?; }
        for i in 0..self.ppu_sprite_shifter_counter.len() { self.ppu_sprite_shifter_counter[i] = read_u8()?; }
        self.ppu_sprite_pattern_l = read_u8()?;
        self.ppu_sprite_pattern_h = read_u8()?;
        self.ppu_sprite_pattern_l2 = read_u8()?;
        self.ppu_sprite_pattern_h2 = read_u8()?;
        self.ppu_next_scanline_contains_sprite_zero = read_u8()? != 0;
        self.ppu_current_scanline_contains_sprite_zero = read_u8()? != 0;
        self.ppu_can_detect_sprite_zero_hit = read_u8()? != 0;
        self.oam2_address = read_u8()?;
        self.secondary_oam_full = read_u8()? != 0;
        self.sprite_evaluation_tick = read_u8()?;
        self.oam_address_overflowed_during_sprite_evaluation = read_u8()? != 0;
        self.ppu_oam_latch = read_u8()?;
        self.ppu_oam_buffer = read_u8()?;
        self.ppu_render_temp = read_u8()?;
        self.in_range_check = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.nine_objects_on_this_scanline = read_u8()? != 0;
        self.ppu_oam_corruption_rendering_disabled_out_of_vblank = read_u8()? != 0;
        self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant = read_u8()? != 0;
        self.ppu_v_register_changed_out_of_vblank = read_u8()? != 0;
        self.ppu_pending_oam_corruption = read_u8()? != 0;
        self.ppu_oam_corruption_index = read_u8()?;
        self.ppu_oam_corruption_rendering_enabled_out_of_vblank = read_u8()? != 0;
        self.ppu_oam_evaluation_corruption_odd_cycle = read_u8()? != 0;
        self.ppu_oam_evaluation_object_in_range = read_u8()? != 0;
        self.ppu_oam_evaluation_object_in_x_range = read_u8()? != 0;
        self.ppu_palette_corruption_rendering_disabled_out_of_vblank = read_u8()? != 0;
        self.oam_corrupted_on_odd_cycle = read_u8()? != 0;
        self.ppu_update_2006_delay = read_u8()?;
        self.ppu_update_2005_delay = read_u8()?;
        self.ppu_update_2001_delay = read_u8()?;
        self.ppu_update_2001_oam_corruption_delay = read_u8()?;
        self.ppu_update_2001_emphasis_bits_delay = read_u8()?;
        self.ppu_update_2005_value = read_u8()?;
        self.ppu_update_2001_value = read_u8()?;
        self.ppu_update_2006_value = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_update_2006_value_temp = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_was_rendering_before_2001_write = read_u8()? != 0;
        self.ppu_2007_read = read_u8()? != 0;
        self.ppu_2007_read_sr = read_u8()? != 0;
        for i in 0..self.ppu_2007_read_latches.len() { self.ppu_2007_read_latches[i] = read_u8()? != 0; }
        self.ppu_2007_pd_rb = read_u8()? != 0;
        self.ppu_2007_read_ale = read_u8()? != 0;
        self.ppu_2007_read_h0_latch = read_u8()? != 0;
        self.ppu_2007_read_xrb = read_u8()? != 0;
        self.ppu_read = read_u8()? != 0;
        self.ppu_2007_write = read_u8()? != 0;
        self.ppu_2007_write_sr = read_u8()? != 0;
        for i in 0..self.ppu_2007_write_latches.len() { self.ppu_2007_write_latches[i] = read_u8()? != 0; }
        self.ppu_2007_db_par = read_u8()? != 0;
        self.ppu_2007_write_ale = read_u8()? != 0;
        self.ppu_2007_tstep_latch = read_u8()? != 0;
        self.ppu_2007_tstep = read_u8()? != 0;
        self.ppu_2007_blnk_latch = read_u8()? != 0;
        self.ppu_2007_palette_ram_enable = read_u8()? != 0;
        self.ppu_2007_write_data = read_u8()?;
        self.ppu_write = read_u8()? != 0;
        self.ppu_pattern_address_register_nt = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_pattern_address_register_at = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_pattern_address_register_chr = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_commit_nametable_fetch = read_u8()? != 0;
        self.ppu_commit_attribute_fetch = read_u8()? != 0;
        self.ppu_commit_pattern_low_fetch = read_u8()? != 0;
        self.ppu_commit_pattern_high_fetch = read_u8()? != 0;
        self.ppu_a12_prev = read_u8()? != 0;
        self.copy_v = read_u8()? != 0;
        self.skipped_pre_render_dot_341 = read_u8()? != 0;
        self.dot_color = read_u8()?;
        self.prev_dot_color = read_u8()?;
        self.prev_prev_dot_color = read_u8()?;
        self.prev_prev_prev_dot_color = read_u8()?;
        self.palette_ram_address = read_u8()?;
        self.this_dot_read_from_palette_ram = read_u8()? != 0;
        self.apu_put_cycle = read_u8()? != 0;
        self.apu_alignment = read_u8()?;
        self.apu_status_dmc_interrupt = read_u8()? != 0;
        self.apu_status_frame_interrupt = read_u8()? != 0;
        self.apu_status_dmc = read_u8()? != 0;
        self.apu_status_delayed_dmc = read_u8()? != 0;
        self.apu_status_noise = read_u8()? != 0;
        self.apu_status_triangle = read_u8()? != 0;
        self.apu_status_pulse2 = read_u8()? != 0;
        self.apu_status_pulse1 = read_u8()? != 0;
        self.clearing_apu_frame_interrupt = read_u8()? != 0;
        self.apu_delayed_dmc_4015 = read_u8()?;
        self.apu_implicit_abort_dmc_4015 = read_u8()? != 0;
        self.apu_set_implicit_abort_dmc_4015 = read_u8()? != 0;
        for i in 0..self.apu_register.len() { self.apu_register[i] = read_u8()?; }
        self.apu_frame_counter_mode = read_u8()? != 0;
        self.apu_frame_counter_inhibit_irq = read_u8()? != 0;
        self.apu_frame_counter_reset = read_u8()?;
        self.apu_framecounter = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.apu_quarter_frame_clock = read_u8()? != 0;
        self.apu_half_frame_clock = read_u8()? != 0;
        self.apu_envelope_start_flag = read_u8()? != 0;
        self.apu_envelope_divider_clock = read_u8()? != 0;
        self.apu_envelope_decay_level = read_u8()?;
        self.apu_length_counter_pulse1 = read_u8()?;
        self.apu_length_counter_pulse2 = read_u8()?;
        self.apu_length_counter_triangle = read_u8()?;
        self.apu_length_counter_noise = read_u8()?;
        self.apu_length_counter_halt_pulse1 = read_u8()? != 0;
        self.apu_length_counter_halt_pulse2 = read_u8()? != 0;
        self.apu_length_counter_halt_triangle = read_u8()? != 0;
        self.apu_length_counter_halt_noise = read_u8()? != 0;
        self.apu_length_counter_reload_pulse1 = read_u8()? != 0;
        self.apu_length_counter_reload_pulse2 = read_u8()? != 0;
        self.apu_length_counter_reload_triangle = read_u8()? != 0;
        self.apu_length_counter_reload_noise = read_u8()? != 0;
        self.apu_length_counter_reload_value_pulse1 = read_u8()?;
        self.apu_length_counter_reload_value_pulse2 = read_u8()?;
        self.apu_length_counter_reload_value_triangle = read_u8()?;
        self.apu_length_counter_reload_value_noise = read_u8()?;
        self.apu_channel_timer_pulse1 = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.apu_channel_timer_pulse2 = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.apu_channel_timer_triangle = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.apu_channel_timer_noise = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.apu_channel_timer_dmc = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.apu_dmc_enable_irq = read_u8()? != 0;
        self.apu_dmc_loop = read_u8()? != 0;
        self.apu_dmc_rate = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.apu_dmc_output = read_u8()?;
        self.apu_dmc_sample_address = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.apu_dmc_sample_length = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.apu_dmc_bytes_remaining = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.apu_dmc_buffer = read_u8()?;
        self.apu_dmc_address_counter = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.apu_dmc_shifter = read_u8()?;
        self.apu_dmc_shifter_bits_remaining = read_u8()?;
        self.dpcm_up = read_u8()? != 0;
        self.apu_silent = read_u8()? != 0;
        self.audio_cycles_accumulator = f64::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?, read_u8()?, read_u8()?, read_u8()?, read_u8()?]);
        self.audio_sample_accumulator = f32::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?]);
        self.audio_sample_count = f32::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?]);
        self.audio_host_sample_rate = f64::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?, read_u8()?, read_u8()?, read_u8()?, read_u8()?]);
        self.audio_frame_cycle = u32::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?]);
        self.audio_previous_output = i32::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?]);
        let mut blip = crate::blip::BlipBuf::new(crate::blip::BlipBuf::MAX_FRAME);
        blip.set_rates(self.cpu_clock(), self.audio_host_sample_rate);
        self.blip = Some(blip);
        self.filter_lp_alpha = f32::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?]);
        self.filter_lp_prev_out = f32::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?]);
        self.filter_hp1_alpha = f32::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?]);
        self.filter_hp1_prev_in = f32::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?]);
        self.filter_hp1_prev_out = f32::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?]);
        self.filter_hp2_alpha = f32::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?]);
        self.filter_hp2_prev_in = f32::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?]);
        self.filter_hp2_prev_out = f32::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?]);
        self.pulse1_timer = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.pulse1_sequencer_step = read_u8()?;
        self.pulse1_envelope_divider = read_u8()?;
        self.pulse1_envelope_decay_level = read_u8()?;
        self.pulse1_envelope_start_flag = read_u8()? != 0;
        self.pulse1_sweep_divider = read_u8()?;
        self.pulse1_sweep_reload = read_u8()? != 0;
        self.pulse2_timer = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.pulse2_sequencer_step = read_u8()?;
        self.pulse2_envelope_divider = read_u8()?;
        self.pulse2_envelope_decay_level = read_u8()?;
        self.pulse2_envelope_start_flag = read_u8()? != 0;
        self.pulse2_sweep_divider = read_u8()?;
        self.pulse2_sweep_reload = read_u8()? != 0;
        self.triangle_timer = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.triangle_sequencer_step = read_u8()?;
        self.triangle_linear_counter = read_u8()?;
        self.triangle_linear_counter_reload_flag = read_u8()? != 0;
        self.noise_timer = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.noise_shift_register = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.noise_envelope_divider = read_u8()?;
        self.noise_envelope_decay_level = read_u8()?;
        self.noise_envelope_start_flag = read_u8()? != 0;
        self.apu_controller_ports_strobing = read_u8()? != 0;
        self.apu_controller_ports_strobed = read_u8()? != 0;
        self.controller_port1.store(read_u8()?, Ordering::Relaxed);
        self.controller_port2.store(read_u8()?, Ordering::Relaxed);
        self.controller_shift_register1 = read_u8()?;
        self.controller_shift_register2 = read_u8()?;
        self.controller1_shift_counter = read_u8()?;
        self.controller2_shift_counter = read_u8()?;
        self.powerpad_shift_data[0] = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.powerpad_shift_data[1] = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.powerpad_shift_count[0] = read_u8()?;
        self.powerpad_shift_count[1] = read_u8()?;
        self.paddle_readbit[0] = read_u8()?;
        self.paddle_readbit[1] = read_u8()?;
        self.paddle_readbit[2] = read_u8()?;
        self.snes_readbit[0] = read_u8()?;
        self.snes_readbit[1] = read_u8()?;
        self.snes_mouse_readbit[0] = read_u8()?;
        self.snes_mouse_readbit[1] = read_u8()?;
        self.subor_mouse_latch[0] = read_u8()?;
        self.subor_mouse_latch[1] = read_u8()?;
        let mut ht0 = [0u8; 4]; for b in ht0.iter_mut() { *b = read_u8()?; }
        let mut ht1 = [0u8; 4]; for b in ht1.iter_mut() { *b = read_u8()?; }
        self.hori_track_state[0] = u32::from_le_bytes(ht0);
        self.hori_track_state[1] = u32::from_le_bytes(ht1);
        self.hori_track_readbit[0] = read_u8()?;
        self.hori_track_readbit[1] = read_u8()?;
        self.fourscore_readbit[0] = read_u8()?;
        self.fourscore_readbit[1] = read_u8()?;
        self.expansion_adapter_ports[0].store(read_u8()?, Ordering::Relaxed);
        self.expansion_adapter_ports[1].store(read_u8()?, Ordering::Relaxed);
        self.expansion_adapter_ports[2].store(read_u8()?, Ordering::Relaxed);
        self.expansion_adapter_ports[3].store(read_u8()?, Ordering::Relaxed);
        self.expansion_adapter_shift_register[0] = read_u8()?;
        self.expansion_adapter_shift_register[1] = read_u8()?;
        self.expansion_adapter_shift_register[2] = read_u8()?;
        self.expansion_adapter_shift_register[3] = read_u8()?;
        self.oeka_strobe = read_u8()? != 0;
        self.oeka_shift = read_u8()? != 0;
        self.oeka_state_buffer = u32::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?]);
        self.family_trainer_ignore_rows = read_u8()?;
        let mut ft_state = self.family_trainer_state.lock().unwrap();
        for slot in ft_state.iter_mut() {
            *slot = read_u8()?;
        }
        drop(ft_state);
        let mut hs_state = self.hyper_shot_state.lock().unwrap();
        for slot in hs_state.iter_mut() {
            *slot = read_u8()?;
        }
        drop(hs_state);
        self.hyper_shot_enable_p1 = read_u8()? != 0;
        self.hyper_shot_enable_p2 = read_u8()? != 0;
        let mut bh0 = [0u8; 4]; for b in bh0.iter_mut() { *b = read_u8()?; }
        let mut bh1 = [0u8; 4]; for b in bh1.iter_mut() { *b = read_u8()?; }
        self.bandai_hyper_state[0] = u32::from_le_bytes(bh0);
        self.bandai_hyper_state[1] = u32::from_le_bytes(bh1);
        self.bandai_hyper_readbit[0] = read_u8()?;
        self.bandai_hyper_readbit[1] = read_u8()?;
        for slot in self.turbo_file_data.iter_mut() {
            *slot = read_u8()?;
        }
        self.turbo_file_position = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.turbo_file_last_write = read_u8()?;
        for slot in self.battle_box_data.iter_mut() {
            *slot = read_u8()?;
        }
        self.battle_box_last_write = read_u8()?;
        self.battle_box_address = read_u8()?;
        self.battle_box_chip_select = read_u8()?;
        self.battle_box_output = read_u8()?;
        self.battle_box_write_enabled = read_u8()? != 0;
        self.battle_box_input_bit_position = read_u8()?;
        self.battle_box_input_data = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.battle_box_is_write = read_u8()? != 0;
        self.battle_box_is_read = read_u8()? != 0;
        let mut fb_state = self.family_basic_state.lock().unwrap();
        for slot in fb_state.iter_mut() {
            *slot = read_u8()?;
        }
        drop(fb_state);
        self.family_basic_row = read_u8()?;
        self.family_basic_column = read_u8()?;
        self.family_basic_enabled = read_u8()? != 0;
        let mut pt_state = self.party_tap_state.lock().unwrap();
        for slot in pt_state.iter_mut() {
            *slot = read_u8()?;
        }
        drop(pt_state);
        self.party_tap_buffer = read_u8()?;
        self.party_tap_read_count = read_u8()?;
        self.party_tap_strobe = read_u8()? != 0;
        let mut pc_state = self.pachinko_state.lock().unwrap();
        for slot in pc_state.iter_mut() {
            *slot = read_u8()?;
        }
        drop(pc_state);
        self.pachinko_analog = read_u8()?;
        self.pachinko_buffer = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.pachinko_strobe = read_u8()? != 0;
        let mut pb_state = self.punching_bag_state.lock().unwrap();
        for slot in pb_state.iter_mut() {
            *slot = read_u8()?;
        }
        drop(pb_state);
        self.punching_bag_selected_sensors = read_u8()?;
        let mut jm_state = self.jissen_mahjong_state.lock().unwrap();
        for slot in jm_state.iter_mut() {
            *slot = read_u8()?;
        }
        drop(jm_state);
        self.jissen_mahjong_row = read_u8()?;
        self.jissen_mahjong_state_buffer = u32::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?]);
        self.jissen_mahjong_strobe = read_u8()? != 0;
        let mut sk_state = self.subor_keyboard_state.lock().unwrap();
        for slot in sk_state.iter_mut() {
            *slot = read_u8()?;
        }
        drop(sk_state);
        self.subor_keyboard_row = read_u8()?;
        self.subor_keyboard_column = read_u8()?;
        self.subor_keyboard_enabled = read_u8()? != 0;
        let mut mcc_bytes = [0u8; 8];
        for b in mcc_bytes.iter_mut() { *b = read_u8()?; }
        self.master_cycle_counter = u64::from_le_bytes(mcc_bytes);
        for slot in self.barcode_battler_stream.iter_mut() { *slot = read_u8()?; }
        let mut bbc_bytes = [0u8; 8];
        for b in bbc_bytes.iter_mut() { *b = read_u8()?; }
        self.barcode_battler_insert_cycle = u64::from_le_bytes(bbc_bytes);
        self.barcode_battler_active = read_u8()? != 0;
        self.virtualboy_readbit[0] = read_u8()?;
        self.virtualboy_readbit[1] = read_u8()?;
        self.virtualboy_state_buffer[0] = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.virtualboy_state_buffer[1] = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.data_pins_are_not_floating = read_u8()? != 0;
        self.frame_advance_reached_vblank = read_u8()? != 0;
        let mapper_len = u32::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?]) as usize;
        if mapper_len > 0 {
            let mut mapper_state = vec![0u8; mapper_len];
            for i in 0..mapper_len {
                mapper_state[i] = read_u8()?;
            }
            if let Some(cart) = &mut self.cart {
                let mut real_mapper = std::mem::replace(
                    &mut cart.mapper_chip,
                    crate::mapper::create_mapper(0, 0, &[0; 16], &[], 0, false, false, "", &[]).unwrap(),
                );
                real_mapper.load_mapper_registers(cart, &mapper_state, 0);
                cart.mapper_chip = real_mapper;
            }
        }
        if p < data.len() {
            for i in 0..self.um6578_extra_ram.len() {
                if p < data.len() { self.um6578_extra_ram[i] = data[p]; p+=1; }
            }
            for bank in &mut self.um6578_vram {
                for i in 0..bank.len() { if p < data.len() { bank[i] = data[p]; p+=1; } }
            }
            if p < data.len() { self.um6578_reg2008 = data[p]; p+=1; }
            if p < data.len() { self.um6578_color_mask = data[p]; p+=1; }
            if p < data.len() { self.um6578_dma_control = data[p]; p+=1; }
            if p < data.len() { self.um6578_dma_page = data[p]; p+=1; }
            if p + 1 < data.len() { self.um6578_dma_source = u16::from_le_bytes([data[p], data[p+1]]); p+=2; }
            if p + 1 < data.len() { self.um6578_dma_target = u16::from_le_bytes([data[p], data[p+1]]); p+=2; }
            if p + 1 < data.len() { self.um6578_dma_length = u16::from_le_bytes([data[p], data[p+1]]); p+=2; }
            if p + 3 < data.len() { self.um6578_dma_busy = u32::from_le_bytes([data[p], data[p+1], data[p+2], data[p+3]]); p+=4; }
        if p < data.len() { self.um6578_nt_tile_byte = data[p]; p+=1; }
        if p < data.len() { self.um6578_nt_attr_byte = data[p]; p+=1; }
        if p < data.len() { self.um6578_bg_palette = data[p]; p+=1; }
        if p < data.len() { self.um6578_bg_palette_lo = data[p]; p+=1; }
            for i in 0..self.um6578_chr_ram.len() {
                if p < data.len() { self.um6578_chr_ram[i] = data[p]; p+=1; }
            }
        }
        {
            let mut pk_state = self.pec586_keyboard_state.lock().unwrap();
            for slot in pk_state.iter_mut() {
                if p >= data.len() { break; }
                *slot = data[p]; p+=1;
            }
        }
        if p < data.len() { self.pec586_kspos = data[p]; p+=1; }
        if p < data.len() { self.pec586_ksindex = data[p]; p+=1; }
        if p < data.len() { self.pec586_kstrobe = data[p]; p+=1; }
        {
            let mut b79_state = self.bit79_keyboard_state.lock().unwrap();
            for slot in b79_state.iter_mut() {
                if p >= data.len() { break; }
                *slot = data[p]; p+=1;
            }
        }
        if p < data.len() { self.bit79_keyboard_row = data[p]; p+=1; }
        if p < data.len() { self.bit79_keyboard_column = data[p]; p+=1; }
        if p < data.len() { self.bit79_keyboard_strobe = data[p]; p+=1; }
        {
            let mut kd_state = self.keda_keyboard_state.lock().unwrap();
            for slot in kd_state.iter_mut() {
                if p >= data.len() { break; }
                *slot = data[p]; p+=1;
            }
        }
        if p < data.len() { self.keda_keyboard_row = data[p]; p+=1; }
        if p < data.len() { self.keda_keyboard_column = data[p]; p+=1; }
        if p < data.len() { self.keda_keyboard_strobe = data[p]; p+=1; }
        {
            let mut kw_state = self.kingwon_keyboard_state.lock().unwrap();
            for slot in kw_state.iter_mut() {
                if p >= data.len() { break; }
                *slot = data[p]; p+=1;
            }
        }
        if p < data.len() { self.kingwon_keyboard_row = data[p]; p+=1; }
        if p < data.len() { self.kingwon_keyboard_column = data[p]; p+=1; }
        if p < data.len() { self.kingwon_keyboard_bits = data[p]; p+=1; }
        if p < data.len() { self.zecheng_keyboard_bits1 = data[p]; p+=1; }
        if p < data.len() { self.zecheng_keyboard_bits2 = data[p]; p+=1; }
        if p < data.len() { self.zecheng_keyboard_strobe = data[p] != 0; p+=1; }
        if p < data.len() {
            *self.zecheng_keyboard_buttons.lock().unwrap() = data[p];
            p+=1;
        }
        if p < data.len() {
            *self.zecheng_keyboard_keys.lock().unwrap() = data[p];
            p+=1;
        }
        {
            let mut qk_buttons = self.quiz_king_buttons.lock().unwrap();
            for slot in qk_buttons.iter_mut() {
                if p >= data.len() { break; }
                *slot = data[p]; p+=1;
            }
        }
        if p < data.len() { self.quiz_king_data_r = data[p]; p+=1; }
        if p < data.len() { self.quiz_king_funky_mode = data[p] != 0; p+=1; }
        if p < data.len() { self.quiz_king_strobe = data[p] != 0; p+=1; }
        {
            let mut tr_buttons = self.top_rider_buttons.lock().unwrap();
            for slot in tr_buttons.iter_mut() {
                if p >= data.len() { break; }
                *slot = data[p]; p+=1;
            }
        }
        let mut read_u32 = || -> Result<u32, String> {
            if p + 3 < data.len() {
                let v = u32::from_le_bytes([data[p], data[p+1], data[p+2], data[p+3]]);
                p += 4;
                Ok(v)
            } else {
                Ok(0)
            }
        };
        self.top_rider_bs = read_u32()?;
        self.top_rider_bss = read_u32()?;
        self.top_rider_boop = read_u32()?;
        {
            let mut fns_buttons = self.fami_net_sys_buttons.lock().unwrap();
            for slot in fns_buttons.iter_mut() {
                if p >= data.len() { break; }
                *slot = data[p]; p+=1;
            }
        }
        let mut fns_data_bytes = [0u8; 4];
        for b in fns_data_bytes.iter_mut() {
            if p < data.len() { *b = data[p]; p+=1; }
        }
        self.fami_net_sys_data = u32::from_le_bytes(fns_data_bytes);
        if p < data.len() { self.fami_net_sys_readbit = data[p]; p+=1; }
        if p < data.len() { self.fami_net_sys_prev_strobe = data[p] != 0; }
        {
            let mut cp_input = self.city_patrolman_input.lock().unwrap();
            for slot in cp_input.iter_mut() {
                if p >= data.len() { break; }
                *slot = data[p]; p+=1;
            }
        }
        if p < data.len() { self.city_patrolman_strobe = data[p]; p+=1; }
        if p + 3 < data.len() {
            self.city_patrolman_time_out = u32::from_le_bytes([data[p], data[p+1], data[p+2], data[p+3]]);
            p += 4;
            self.city_patrolman_shift_reg = u32::from_le_bytes([data[p], data[p+1], data[p+2], data[p+3]]);
            p += 4;
        }
        {
            let mut mog_buttons = self.moguraa_buttons.lock().unwrap();
            for slot in mog_buttons.iter_mut() {
                if p >= data.len() { break; }
                *slot = data[p]; p+=1;
            }
        }
        if p + 1 < data.len() {
            self.moguraa_bits = u16::from_le_bytes([data[p], data[p+1]]);
            p += 2;
        }
        if p < data.len() { self.moguraa_sel = data[p]; p+=1; }
        {
            let mut gnc_buttons = self.golden_nugget_buttons.lock().unwrap();
            for slot in gnc_buttons.iter_mut() {
                if p >= data.len() { break; }
                *slot = data[p]; p+=1;
            }
        }
        if p < data.len() { self.golden_nugget_strobe = data[p]; p+=1; }
        if p < data.len() { self.golden_nugget_shift = data[p]; p+=1; }
        {
            let mut abl_buttons = self.abl_pinball_buttons.lock().unwrap();
            for slot in abl_buttons.iter_mut() {
                if p >= data.len() { break; }
                *slot = data[p]; p+=1;
            }
        }
        if p + 1 < data.len() {
            self.abl_pinball_count = i16::from_le_bytes([data[p], data[p+1]]);
            p += 2;
        }
        if p + 1 < data.len() {
            self.abl_pinball_plunger = i16::from_le_bytes([data[p], data[p+1]]);
            p += 2;
        }
        if p + 3 < data.len() {
            self.abl_pinball_plunger_reset_count = i32::from_le_bytes([data[p], data[p+1], data[p+2], data[p+3]]);
            p += 4;
        }
        {
            let mut tv_buttons = self.tv_pump_buttons.lock().unwrap();
            for slot in tv_buttons.iter_mut() {
                if p >= data.len() { break; }
                *slot = data[p]; p+=1;
            }
        }
        {
            let mut tr_buttons = self.triface_mahjong_buttons.lock().unwrap();
            for slot in tr_buttons.iter_mut() {
                if p >= data.len() { break; }
                *slot = data[p]; p+=1;
            }
        }
        if p < data.len() { self.triface_mahjong_column = data[p]; p+=1; }
        if p < data.len() { self.triface_mahjong_row = data[p]; p+=1; }
        for slot in self.triface_mahjong_keys.iter_mut() {
            if p >= data.len() { break; }
            *slot = data[p]; p+=1;
        }
        {
            let mut mg_buttons = self.mahjong_gekitou_buttons.lock().unwrap();
            for slot in mg_buttons.iter_mut() {
                if p >= data.len() { break; }
                *slot = data[p]; p+=1;
            }
        }
        if p < data.len() { self.mahjong_gekitou_bits = data[p]; p+=1; }
        if p < data.len() { self.mahjong_gekitou_bit_ptr = data[p]; p+=1; }
        if p < data.len() { self.mahjong_gekitou_strobe = data[p]; }
        if p < data.len() { self.ppu_oam_read_latch = data[p]; }
        if p < data.len() { self.oam2_reset_signal = data[p]; }
        Ok(())
    }
}

