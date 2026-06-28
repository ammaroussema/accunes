// the heart of this nes emulator, handles basically every component interacting with the other and itself

use crate::cartridge::Cartridge;
use crate::config;
use crate::region::{Region, TvSystem};

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
    pub total_cycles: i32,

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

    pub ram: [u8; 0x800],
    pub vram: [u8; 0x800],
    pub oam: [u8; 0x100],
    pub oam2: [u8; 32],
    pub palette_ram: [u8; 0x20],

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
    pub ppu_bg_attr_sr_l: u16,
    pub ppu_bg_attr_sr_h: u16,
    pub ppu_attr_latch_register: u8,
    pub ppu_low_bit_plane: u8,
    pub ppu_high_bit_plane: u8,
    pub ppu_attribute: u8,

    pub ppu_sprite_sr_l: [u8; 8],
    pub ppu_sprite_sr_h: [u8; 8],
    pub ppu_sprite_attribute: [u8; 8],
    pub ppu_sprite_pattern: [u8; 8],
    pub ppu_sprite_x_position: [u8; 8],
    pub ppu_sprite_y_position: [u8; 8],
    pub ppu_sprite_shifter_counter: [u8; 8],
    pub ppu_sprite_pattern_l: u8,
    pub ppu_sprite_pattern_h: u8,
    pub ppu_next_scanline_contains_sprite_zero: bool,
    pub ppu_current_scanline_contains_sprite_zero: bool,
    pub ppu_can_detect_sprite_zero_hit: bool,

    pub vs_ppu_variant: u8,

    pub oam2_address: u8,
    pub secondary_oam_full: bool,
    pub sprite_evaluation_tick: u8,
    pub oam_address_overflowed_during_sprite_evaluation: bool,
    pub ppu_oam_latch: u8,
    pub ppu_oam_buffer: u8,
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

    pub audio_buffer: Option<std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<f32>>>>,
    pub audio_cycles_accumulator: f64,
    pub audio_sample_accumulator: f32,
    pub audio_sample_count: f32,
    pub audio_host_sample_rate: f64,

    pub master_volume: f32,
    pub square1_volume: f32,
    pub square2_volume: f32,
    pub triangle_volume: f32,
    pub noise_volume: f32,
    pub pcm_volume: f32,
    pub audio_enabled: bool,
    pub audio_depth: u8,

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
    pub controller_port1: u8,
    pub controller_port2: u8,
    pub controller_shift_register1: u8,
    pub controller_shift_register2: u8,
    pub controller1_shift_counter: u8,
    pub controller2_shift_counter: u8,
    pub data_pins_are_not_floating: bool,

    pub zapper_x: f32,
    pub zapper_y: f32,
    pub zapper_trigger: bool,
    pub zapper_bogo: u8,
    pub paddle_x: [u8; 2],
    pub paddle_button: [bool; 2],
    pub paddle_readbit: [u8; 2],
    pub powerpad_state: [u16; 2],
    pub powerpad_shift_d3: [u8; 2],
    pub powerpad_shift_d4: [u8; 2],
    pub snes_state: [u16; 2],
    pub snes_readbit: [u8; 2],
    pub snes_mouse_state: [u32; 2],
    pub snes_mouse_readbit: [u8; 2],
    pub snes_mouse_delta_x: [f32; 2],
    pub snes_mouse_delta_y: [f32; 2],
    pub snes_mouse_buttons: [u8; 2],
    pub subor_mouse_buttons: [u8; 2],
    pub subor_mouse_dx: [i32; 2],
    pub subor_mouse_dy: [i32; 2],
    pub subor_mouse_latch: [u8; 2],

    pub controller_port3: u8,
    pub controller_port4: u8,
    pub fourscore_readbit: [u8; 2],

    pub controller1_type: config::ControllerType,
    pub controller2_type: config::ControllerType,

    pub frame_advance_reached_vblank: bool,

    pub screen: Vec<u32>,

    pub region_preference: Region,
    pub resolved_region: Region,

}

impl Emulator {
    pub fn init_ram(ram: &mut [u8; 0x800], vram: &mut [u8; 0x800], mode: config::InitialRam) {
        match mode {
            config::InitialRam::Default => {
                for i in 0..0x800usize {
                    let j = i & 0x2;
                    let swap = (i & 0x1F) >= 0x10;
                    if (j < 0x2) != swap {
                        vram[i] = 0xF0;
                        ram[i] = 0xF0;
                    } else {
                        vram[i] = 0x0F;
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
                for i in 0..0x800usize {
                    state = state.wrapping_mul(1103515245).wrapping_add(12345);
                    ram[i] = (state >> 16) as u8;
                    state = state.wrapping_mul(1103515245).wrapping_add(12345);
                    vram[i] = (state >> 16) as u8;
                }
            }
        }
    }

    pub fn new() -> Self {
        let mut ram = [0u8; 0x800];
        let mut vram = [0u8; 0x800];

        Self::init_ram(&mut ram, &mut vram, config::InitialRam::Default);
        let oam2 = [0xFFu8; 32];

        let mut palette_ram = [0u8; 0x20];
        let pal_init: [u8; 0x20] = [
            0x00,0x00,0x28,0x00,0x00,0x08,0x00,0x00,
            0x00,0x01,0x01,0x20,0x00,0x08,0x00,0x02,
            0x00,0x00,0x00,0x00,0x00,0x02,0x21,0x00,
            0x00,0x00,0x00,0x00,0x00,0x10,0x00,0x00,
        ];
        palette_ram.copy_from_slice(&pal_init);

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
            ram, vram, oam: [0u8; 0x100], oam2, palette_ram,
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
            ppu_bg_attr_sr_l: 0, ppu_bg_attr_sr_h: 0,
            ppu_attr_latch_register: 0,
            ppu_low_bit_plane: 0, ppu_high_bit_plane: 0, ppu_attribute: 0,
            ppu_sprite_sr_l: [0; 8], ppu_sprite_sr_h: [0; 8],
            ppu_sprite_attribute: [0; 8], ppu_sprite_pattern: [0; 8],
            ppu_sprite_x_position: [0; 8], ppu_sprite_y_position: [0; 8],
            ppu_sprite_shifter_counter: [0; 8],
            vs_ppu_variant: 3,
            ppu_sprite_pattern_l: 0, ppu_sprite_pattern_h: 0,
            ppu_next_scanline_contains_sprite_zero: false,
            ppu_current_scanline_contains_sprite_zero: false,
            ppu_can_detect_sprite_zero_hit: false,
            oam2_address: 0, secondary_oam_full: false,
            sprite_evaluation_tick: 0,
            oam_address_overflowed_during_sprite_evaluation: false,
            ppu_oam_latch: 0, ppu_oam_buffer: 0, ppu_render_temp: 0,
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
            prev_prev_prev_dot_color: 0, palette_ram_address: 0,
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
            master_volume: 1.0,
            square1_volume: 1.0,
            square2_volume: 1.0,
            triangle_volume: 1.0,
            noise_volume: 1.0,
            pcm_volume: 1.0,
            audio_enabled: true,
            audio_depth: 16,
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
            controller_port1: 0, controller_port2: 0,
            controller_shift_register1: 0, controller_shift_register2: 0,
            controller1_shift_counter: 0, controller2_shift_counter: 0,
            data_pins_are_not_floating: false,
            zapper_x: 0.0, zapper_y: 0.0, zapper_trigger: false, zapper_bogo: 0,
            paddle_x: [0; 2], paddle_button: [false; 2], paddle_readbit: [0; 2],
            powerpad_state: [0; 2], powerpad_shift_d3: [0xFF; 2], powerpad_shift_d4: [0xFF; 2],
            snes_state: [0; 2], snes_readbit: [0; 2],
            snes_mouse_state: [0; 2], snes_mouse_readbit: [0; 2],
            snes_mouse_delta_x: [0.0; 2], snes_mouse_delta_y: [0.0; 2],
            snes_mouse_buttons: [0; 2],
            subor_mouse_buttons: [0; 2],
            subor_mouse_dx: [0; 2],
            subor_mouse_dy: [0; 2],
            subor_mouse_latch: [0; 2],
            controller_port3: 0,
            controller_port4: 0,
            fourscore_readbit: [0; 2],
            controller1_type: config::ControllerType::None,
            controller2_type: config::ControllerType::None,
            frame_advance_reached_vblank: false,
            screen: vec![0u32; 256 * 240],
            region_preference: Region::Auto,
            resolved_region: Region::Ntsc,
        }
    }

    pub fn load_cartridge(&mut self, cart: Cartridge) {
        self.resolved_region = self.compute_region(&cart.tv_system, &cart.name);
        let cpu_clock = self.cpu_clock();
        self.cart = Some(cart);
        if let Some(ref mut cart) = self.cart {
            cart.mapper_chip.set_cpu_clock(cpu_clock);
        }
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
    }

    fn compute_region(&self, tv_system: &TvSystem, filename: &str) -> Region {
        match self.region_preference {
            Region::Ntsc => Region::Ntsc,
            Region::Pal => Region::Pal,
            Region::Dendy => Region::Dendy,
            Region::Auto => {
                match tv_system {
                    TvSystem::Ntsc => Region::Ntsc,
                    TvSystem::Pal | TvSystem::Dual => Region::Pal,
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
        self.zapper_bogo = 0;

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
    }

    pub fn save_prg_ram(&self) {
        if let Some(cart) = &self.cart {
            if !cart.has_battery {
                return;
            }
            let sav_path = crate::config::save_file_path(&cart.name);
            if let Some(parent) = sav_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let data = {
                let mapper = &cart.mapper_chip;
                if let Some(save) = mapper.battery_save_data(cart) {
                    Some(save)
                } else if !cart.prg_ram.is_empty() {
                    Some(cart.prg_ram.clone())
                } else {
                    None
                }
            };
            if let Some(data) = data {
                if let Err(e) = std::fs::write(&sav_path, &data) {
                    eprintln!("Failed to save SRAM to {:?}: {}", sav_path, e);
                } else {
                    println!("Saved SRAM to {:?}", sav_path);
                }
            }
        }
    }

    pub fn set_audio_output(
        &mut self,
        buffer: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<f32>>>,
        sample_rate: f64,
    ) {
        self.audio_buffer = Some(buffer);
        self.audio_host_sample_rate = sample_rate;

        let sr = sample_rate as f32;
        let dt = 1.0 / sr;

        self.filter_lp_alpha = dt / ((1.0 / (2.0 * std::f32::consts::PI * 14000.0)) + dt);

        self.filter_hp1_alpha = (1.0 / (2.0 * std::f32::consts::PI * 440.0)) / ((1.0 / (2.0 * std::f32::consts::PI * 440.0)) + dt);

        self.filter_hp2_alpha = (1.0 / (2.0 * std::f32::consts::PI * 90.0)) / ((1.0 / (2.0 * std::f32::consts::PI * 90.0)) + dt);
    }

    pub fn change_disk(&mut self) {
        if let Some(ref mut cart) = self.cart {
            cart.mapper_chip.change_disk();
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
                || cart.memory_mapper == 45
                || cart.memory_mapper == 59
                || cart.memory_mapper == 83
                || cart.memory_mapper == 90
                || cart.memory_mapper == 124
                || cart.memory_mapper == 264
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


    // run one frame!
    pub fn core_frame_advance(&mut self) {
        if self.zapper_bogo > 0 {
            self.zapper_bogo -= 1;
        }
        self.frame_advance_reached_vblank = false;
        while !self.frame_advance_reached_vblank {
            self.emulator_core();
        }

        while self.ppu_scanline != 0 {
            self.emulator_core();
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
        if self.cpu_clock == 12 {
            self.cpu_clock = 0;
            if let Some(cart) = self.cart.as_mut() {
                if cart.mapper_chip.cpu_clock(1) {
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
        if self.cpu_clock == 16 {
            self.cpu_clock = 0;
            if let Some(cart) = self.cart.as_mut() {
                if cart.mapper_chip.cpu_clock(1) {
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
        if self.cpu_clock == 15 {
            self.cpu_clock = 0;
            if let Some(cart) = self.cart.as_mut() {
                if cart.mapper_chip.cpu_clock(1) {
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
        if self.zapper_bogo > 0 {
            return false;
        }
        let sx = (self.zapper_x * 255.0).round() as usize;
        let sy = (self.zapper_y * 239.0).round() as usize;
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
        out.extend_from_slice(&self.ppu_bg_attr_sr_l.to_le_bytes());
        out.extend_from_slice(&self.ppu_bg_attr_sr_h.to_le_bytes());
        out.push(self.ppu_attr_latch_register);
        out.push(self.ppu_low_bit_plane);
        out.push(self.ppu_high_bit_plane);
        out.push(self.ppu_attribute);
        out.extend_from_slice(&self.ppu_sprite_sr_l);
        out.extend_from_slice(&self.ppu_sprite_sr_h);
        out.extend_from_slice(&self.ppu_sprite_attribute);
        out.extend_from_slice(&self.ppu_sprite_pattern);
        out.extend_from_slice(&self.ppu_sprite_x_position);
        out.extend_from_slice(&self.ppu_sprite_y_position);
        out.extend_from_slice(&self.ppu_sprite_shifter_counter);
        out.push(self.ppu_sprite_pattern_l);
        out.push(self.ppu_sprite_pattern_h);
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
        out.push(self.controller_port1);
        out.push(self.controller_port2);
        out.push(self.controller_shift_register1);
        out.push(self.controller_shift_register2);
        out.push(self.controller1_shift_counter);
        out.push(self.controller2_shift_counter);
        out.push(if self.data_pins_are_not_floating { 1 } else { 0 });
        out.push(if self.frame_advance_reached_vblank { 1 } else { 0 });
        if let Some(cart) = &self.cart {
            let mapper_state = cart.mapper_chip.save_mapper_registers(cart);
            out.extend_from_slice(&(mapper_state.len() as u32).to_le_bytes());
            out.extend_from_slice(&mapper_state);
        } else {
            out.extend_from_slice(&0u32.to_le_bytes());
        }
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
        self.total_cycles = i32::from_le_bytes([read_u8()?, read_u8()?, read_u8()?, read_u8()?]);
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
        for i in 0..self.ram.len() { self.ram[i] = read_u8()?; }
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
        self.ppu_bg_attr_sr_l = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_bg_attr_sr_h = u16::from_le_bytes([read_u8()?, read_u8()?]);
        self.ppu_attr_latch_register = read_u8()?;
        self.ppu_low_bit_plane = read_u8()?;
        self.ppu_high_bit_plane = read_u8()?;
        self.ppu_attribute = read_u8()?;
        for i in 0..self.ppu_sprite_sr_l.len() { self.ppu_sprite_sr_l[i] = read_u8()?; }
        for i in 0..self.ppu_sprite_sr_h.len() { self.ppu_sprite_sr_h[i] = read_u8()?; }
        for i in 0..self.ppu_sprite_attribute.len() { self.ppu_sprite_attribute[i] = read_u8()?; }
        for i in 0..self.ppu_sprite_pattern.len() { self.ppu_sprite_pattern[i] = read_u8()?; }
        for i in 0..self.ppu_sprite_x_position.len() { self.ppu_sprite_x_position[i] = read_u8()?; }
        for i in 0..self.ppu_sprite_y_position.len() { self.ppu_sprite_y_position[i] = read_u8()?; }
        for i in 0..self.ppu_sprite_shifter_counter.len() { self.ppu_sprite_shifter_counter[i] = read_u8()?; }
        self.ppu_sprite_pattern_l = read_u8()?;
        self.ppu_sprite_pattern_h = read_u8()?;
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
        self.controller_port1 = read_u8()?;
        self.controller_port2 = read_u8()?;
        self.controller_shift_register1 = read_u8()?;
        self.controller_shift_register2 = read_u8()?;
        self.controller1_shift_counter = read_u8()?;
        self.controller2_shift_counter = read_u8()?;
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
                    crate::mapper::create_mapper(0, 0, &[0; 16], &[], 0, false, false, "").unwrap(),
                );
                real_mapper.load_mapper_registers(cart, &mapper_state, 0);
                cart.mapper_chip = real_mapper;
            }
        }
        Ok(())
    }

}
