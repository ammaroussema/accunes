// this is the nes apu implementation! now techincally the apu is a part of the cpu thingy,
// but logically way different functions so that's why it's here! pretty complex operations hopefully emulated
// accurately enough!

use crate::emulator::Emulator;

const NOISE_PERIOD_LUT_NTSC: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068
];

const NOISE_PERIOD_LUT_PAL: [u16; 16] = [
    4, 7, 15, 30, 59, 89, 119, 149, 188, 236, 353, 472, 708, 944, 1889, 3779
];

impl Emulator {
    /// sweep periods
    pub fn pulse_target_period(&self, pulse_id: usize, current_period: u16, sweep_reg: u8) -> u16 {
        let shift = sweep_reg & 0x7;
        let negate = (sweep_reg & 0x8) != 0;
        let delta = current_period >> shift;
        if negate {
            if pulse_id == 1 {
                current_period.saturating_sub(delta).saturating_sub(1)
            } else {
                current_period.saturating_sub(delta)
            }
        } else {
            current_period.saturating_add(delta)
        }
    }

    /// audio output calculations
    pub fn mix_apu(&self) -> f32 {
        const PULSE_DUTY_TABLE: [[u8; 8]; 4] = [
            [0, 0, 0, 0, 0, 0, 0, 1],
            [0, 0, 0, 0, 0, 0, 1, 1],
            [0, 0, 0, 0, 1, 1, 1, 1],
            [1, 1, 1, 1, 1, 1, 0, 0],
        ];

        const TRIANGLE_TABLE: [u8; 32] = [
            15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15
        ];

        // --- pulse 1 ---
        let p1_current_period = (self.apu_register[2] as u16) | (((self.apu_register[3] & 0x7) as u16) << 8);
        let p1_target_period = self.pulse_target_period(1, p1_current_period, self.apu_register[1]);
        let p1_duty = (self.apu_register[0] >> 6) as usize;
        let p1_active = self.apu_status_pulse1
            && self.apu_length_counter_pulse1 > 0
            && p1_current_period >= 8
            && p1_target_period <= 0x7FF
            && PULSE_DUTY_TABLE[p1_duty][self.pulse1_sequencer_step as usize] == 1;

        let p1_val = if p1_active {
            if (self.apu_register[0] & 0x10) != 0 {
                (self.apu_register[0] & 0xF) as f32
            } else {
                self.pulse1_envelope_decay_level as f32
            }
        } else {
            0.0
        };

        // --- pulse 2 ---
        let p2_current_period = (self.apu_register[6] as u16) | (((self.apu_register[7] & 0x7) as u16) << 8);
        let p2_target_period = self.pulse_target_period(2, p2_current_period, self.apu_register[5]);
        let p2_duty = (self.apu_register[4] >> 6) as usize;
        let p2_active = self.apu_status_pulse2
            && self.apu_length_counter_pulse2 > 0
            && p2_current_period >= 8
            && p2_target_period <= 0x7FF
            && PULSE_DUTY_TABLE[p2_duty][self.pulse2_sequencer_step as usize] == 1;

        let p2_val = if p2_active {
            if (self.apu_register[4] & 0x10) != 0 {
                (self.apu_register[4] & 0xF) as f32
            } else {
                self.pulse2_envelope_decay_level as f32
            }
        } else {
            0.0
        };

        // --- triangle ---
        let tri_val = TRIANGLE_TABLE[self.triangle_sequencer_step as usize] as f32;

        // --- noise ---
        let noise_active = self.apu_status_noise
            && self.apu_length_counter_noise > 0
            && (self.noise_shift_register & 1) == 0;

        let noise_val = if noise_active {
            if (self.apu_register[0xC] & 0x10) != 0 {
                (self.apu_register[0xC] & 0xF) as f32
            } else {
                self.noise_envelope_decay_level as f32
            }
        } else {
            0.0
        };

        // --- dmc ---
        let dmc_val = self.apu_dmc_output as f32;

        // apply per-channel volume multipliers
        let p1_val = p1_val * self.square1_volume;
        let p2_val = p2_val * self.square2_volume;
        let tri_val = tri_val * self.triangle_volume;
        let noise_val = noise_val * self.noise_volume;
        let dmc_val = dmc_val * self.pcm_volume;

        // --- non-linear nes mixer formulas ---
        let pulse_out = if p1_val + p2_val > 0.0 {
            95.88 / ((8128.0 / (p1_val + p2_val)) + 100.0)
        } else {
            0.0
        };

        let tnd_out = if tri_val / 8227.0 + noise_val / 12241.0 + dmc_val / 22638.0 > 0.0 {
            159.79 / ((1.0 / (tri_val / 8227.0 + noise_val / 12241.0 + dmc_val / 22638.0)) + 100.0)
        } else {
            0.0
        };

        // --- external audio from mappers ---
        let ext_val = if let Some(cart) = &self.cart {
            cart.mapper_chip.audio_sample()
        } else {
            0.0
        };

        (pulse_out + tnd_out + (ext_val * 0.1)) * self.master_volume
    }

    // apu emulation every cpu cycle
    pub fn emulate_apu(&mut self) {
        // controller clocking
        if !self.apu_controller_ports_strobing {
            if self.controller1_shift_counter > 0 {
                self.controller1_shift_counter -= 1;
                if self.controller1_shift_counter == 0 {
                    self.controller_shift_register1 <<= 1;
                    self.controller_shift_register1 |= 1;
                    self.powerpad_shift_d3[0] = self.powerpad_shift_d3[0].wrapping_shl(1) | 1;
                    self.powerpad_shift_d4[0] = self.powerpad_shift_d4[0].wrapping_shl(1) | 1;
                }
            }
            if self.controller2_shift_counter > 0 {
                self.controller2_shift_counter -= 1;
                if self.controller2_shift_counter == 0 {
                    self.controller_shift_register2 <<= 1;
                    self.controller_shift_register2 |= 1;
                    self.powerpad_shift_d3[1] = self.powerpad_shift_d3[1].wrapping_shl(1) | 1;
                    self.powerpad_shift_d4[1] = self.powerpad_shift_d4[1].wrapping_shl(1) | 1;
                }
            }
        } else {
            self.controller1_shift_counter = 0;
            self.controller2_shift_counter = 0;
        }

        if !self.apu_put_cycle {
            // get cycle

            // controller strobing
            if self.apu_controller_ports_strobing {
                if !self.apu_controller_ports_strobed {
                    self.apu_controller_ports_strobed = true;
                    // vs system zapper: set controller_port1 for shift register on $4016
                    if self.controller1_type == crate::config::ControllerType::Zapper {
                        if self.cart.as_ref().map(|c| c.is_vs_system).unwrap_or(false) {
                            self.controller_port1 = 0x08
                                | (self.zapper_trigger as u8)
                                | ((self.zapper_check_hit() as u8) << 1);
                        }
                    }
                    self.controller_shift_register1 = self.controller_port1;
                    self.controller_shift_register2 = self.controller_port2;
                    // powerpad: latch d3/d4 shift registers from button state
                    if self.controller1_type == crate::config::ControllerType::PowerPadA || self.controller1_type == crate::config::ControllerType::PowerPadB {
                        let s = self.powerpad_state[0];
                        self.powerpad_shift_d3[0] = (((s >> 1) & 1) << 7 | ((s >> 0) & 1) << 6 | ((s >> 4) & 1) << 5 | ((s >> 8) & 1) << 4 | ((s >> 5) & 1) << 3 | ((s >> 9) & 1) << 2 | ((s >> 10) & 1) << 1 | ((s >> 6) & 1)) as u8;
                        self.powerpad_shift_d4[0] = (((s >> 3) & 1) << 7 | ((s >> 2) & 1) << 6 | ((s >> 11) & 1) << 5 | ((s >> 7) & 1) << 4) as u8;
                    }
                    if self.controller2_type == crate::config::ControllerType::PowerPadA || self.controller2_type == crate::config::ControllerType::PowerPadB {
                        let s = self.powerpad_state[1];
                        self.powerpad_shift_d3[1] = (((s >> 1) & 1) << 7 | ((s >> 0) & 1) << 6 | ((s >> 4) & 1) << 5 | ((s >> 8) & 1) << 4 | ((s >> 5) & 1) << 3 | ((s >> 9) & 1) << 2 | ((s >> 10) & 1) << 1 | ((s >> 6) & 1)) as u8;
                        self.powerpad_shift_d4[1] = (((s >> 3) & 1) << 7 | ((s >> 2) & 1) << 6 | ((s >> 11) & 1) << 5 | ((s >> 7) & 1) << 4) as u8;
                    }
                    if self.controller1_type == crate::config::ControllerType::SNESPad {
                        self.snes_readbit[0] = 0;
                    }
                    if self.controller2_type == crate::config::ControllerType::SNESPad {
                        self.snes_readbit[1] = 0;
                    }
                    if self.controller1_type == crate::config::ControllerType::SNESMouse {
                        self.snes_mouse_readbit[0] = 0;
                    }
                    if self.controller2_type == crate::config::ControllerType::SNESMouse {
                        self.snes_mouse_readbit[1] = 0;
                    }
                }
            } else {
                self.apu_controller_ports_strobed = false;
            }

            // clock channel timers
            self.apu_channel_timer_pulse1 = self.apu_channel_pulse1_sub();
            self.apu_channel_timer_pulse2 = self.apu_channel_pulse2_sub();
            self.apu_channel_timer_noise = self.apu_channel_timer_noise.wrapping_sub(1);

            // clock detailed oscillators on apu get cycles
            // pulse 1
            if self.pulse1_timer == 0 {
                let period = (self.apu_register[2] as u16) | (((self.apu_register[3] & 0x7) as u16) << 8);
                self.pulse1_timer = period;
                self.pulse1_sequencer_step = self.pulse1_sequencer_step.wrapping_sub(1) & 7;
            } else {
                self.pulse1_timer = self.pulse1_timer.saturating_sub(1);
            }

            // pulse 2
            if self.pulse2_timer == 0 {
                let period = (self.apu_register[6] as u16) | (((self.apu_register[7] & 0x7) as u16) << 8);
                self.pulse2_timer = period;
                self.pulse2_sequencer_step = self.pulse2_sequencer_step.wrapping_sub(1) & 7;
            } else {
                self.pulse2_timer = self.pulse2_timer.saturating_sub(1);
            }

            // noise
            if self.noise_timer == 0 {
                let rate_index = (self.apu_register[0xE] & 0xF) as usize;
                self.noise_timer = if self.is_pal() { NOISE_PERIOD_LUT_PAL[rate_index] } else { NOISE_PERIOD_LUT_NTSC[rate_index] };
                let mode = (self.apu_register[0xE] & 0x80) != 0;
                let feedback = (self.noise_shift_register & 1) ^ ((self.noise_shift_register >> (if mode { 6 } else { 1 })) & 1);
                self.noise_shift_register = (self.noise_shift_register >> 1) | (feedback << 14);
            } else {
                self.noise_timer = self.noise_timer.saturating_sub(1);
            }

            // dmc timer (table is in cpu cycles, count is in apu half-cycles)
            self.apu_channel_timer_dmc = self.apu_channel_timer_dmc.wrapping_sub(2);
            if self.apu_channel_timer_dmc == 0 {
                self.apu_channel_timer_dmc = self.apu_dmc_rate;
                if !self.apu_silent {
                    self.dpcm_up = (self.apu_dmc_shifter & 1) == 1;
                    if self.dpcm_up {
                        if self.apu_dmc_output <= 125 { self.apu_dmc_output += 2; }
                    } else {
                        if self.apu_dmc_output >= 2 { self.apu_dmc_output -= 2; }
                    }
                }
                self.apu_dmc_shifter >>= 1;
                self.apu_dmc_shifter_bits_remaining -= 1;
                if self.apu_dmc_shifter_bits_remaining == 0 {
                    self.apu_dmc_shifter_bits_remaining = 8;
                    if self.apu_dmc_bytes_remaining > 0 || self.apu_set_implicit_abort_dmc_4015 {
                        if !self.do_dmc_dma && self.cannot_run_dmc_dma_right_now != 2 {
                            self.do_dmc_dma = true;
                            self.dmc_dma_halt = true;
                        }
                        if self.apu_set_implicit_abort_dmc_4015 {
                            self.apu_implicit_abort_dmc_4015 = true;
                            self.apu_set_implicit_abort_dmc_4015 = false;
                        }
                        self.apu_dmc_shifter = self.apu_dmc_buffer;
                        self.apu_silent = false;
                    } else {
                        self.apu_silent = true;
                    }
                }
            }
            if self.cannot_run_dmc_dma_right_now > 0 {
                self.cannot_run_dmc_dma_right_now = self.cannot_run_dmc_dma_right_now.wrapping_sub(2);
            }
        } else {
            // put cycle
            if self.clearing_apu_frame_interrupt {
                self.clearing_apu_frame_interrupt = false;
                self.apu_status_frame_interrupt = false;
                self.irq_level_detector = false;
            }
            if self.dmc_dma_delay > 0 {
                self.dmc_dma_delay -= 1;
                if self.dmc_dma_delay == 0 && !self.do_dmc_dma {
                    self.do_dmc_dma = true;
                    self.dmc_dma_halt = true;
                    self.apu_dmc_shifter = self.apu_dmc_buffer;
                    self.apu_silent = false;
                }
            }
        }

        // delayed dmc $4015
        if self.apu_delayed_dmc_4015 > 0 {
            self.apu_delayed_dmc_4015 -= 1;
            if self.apu_delayed_dmc_4015 == 0 {
                self.apu_status_dmc = self.apu_status_delayed_dmc;
                if !self.apu_status_dmc {
                    self.apu_dmc_bytes_remaining = 0;
                }
            }
        }

        self.apu_channel_timer_triangle = self.apu_channel_timer_triangle.wrapping_sub(1);

        // clock triangle & noise oscillators (they clock on every cpu cycle)
        // triangle
        if self.triangle_timer == 0 {
            let period = (self.apu_register[0xA] as u16) | (((self.apu_register[0xB] & 0x7) as u16) << 8);
            self.triangle_timer = period;
            if self.apu_length_counter_triangle > 0 && self.triangle_linear_counter > 0 && period >= 2 {
                self.triangle_sequencer_step = (self.triangle_sequencer_step + 1) & 31;
            }
        } else {
            self.triangle_timer = self.triangle_timer.saturating_sub(1);
        }



        // frame counter reset
        if (self.apu_frame_counter_reset & 0x80) == 0 {
            self.apu_frame_counter_reset = self.apu_frame_counter_reset.wrapping_sub(1);
            if (self.apu_frame_counter_reset & 0x80) != 0 {
                self.apu_framecounter = 0;
            }
        }
        self.apu_framecounter = self.apu_framecounter.wrapping_add(1);

        // frame counter sequencer (ntsc vs pal vs dendy values)
        if self.is_dendy() {
            if self.apu_frame_counter_mode {
                // 5-step (Dendy)
                if self.apu_framecounter == 8866 {
                    self.apu_quarter_frame_clock = true;
                } else if self.apu_framecounter == 17732 {
                    self.apu_quarter_frame_clock = true; self.apu_half_frame_clock = true;
                } else if self.apu_framecounter == 26598 {
                    self.apu_quarter_frame_clock = true;
                } else if self.apu_framecounter == 44330 {
                    self.apu_quarter_frame_clock = true; self.apu_half_frame_clock = true;
                } else if self.apu_framecounter == 44331 {
                    self.apu_framecounter = 0;
                }
            } else {
                // 4-step (Dendy)
                if self.apu_framecounter == 8866 {
                    self.apu_quarter_frame_clock = true;
                } else if self.apu_framecounter == 17732 {
                    self.apu_quarter_frame_clock = true; self.apu_half_frame_clock = true;
                } else if self.apu_framecounter == 26598 {
                    self.apu_quarter_frame_clock = true;
                } else if self.apu_framecounter == 35464 {
                    self.apu_status_frame_interrupt = true;
                } else if self.apu_framecounter == 35465 {
                    self.apu_quarter_frame_clock = true;
                    self.apu_status_frame_interrupt = true;
                    self.irq_level_detector |= !self.apu_frame_counter_inhibit_irq;
                    self.apu_half_frame_clock = true;
                } else if self.apu_framecounter == 35466 {
                    self.apu_status_frame_interrupt = !self.apu_frame_counter_inhibit_irq;
                    self.irq_level_detector |= !self.apu_frame_counter_inhibit_irq;
                    self.apu_framecounter = 0;
                }
            }
        } else if self.is_pal() {
            if self.apu_frame_counter_mode {
                // 5-step (PAL)
                if self.apu_framecounter == 8313 {
                    self.apu_quarter_frame_clock = true;
                } else if self.apu_framecounter == 16627 {
                    self.apu_quarter_frame_clock = true; self.apu_half_frame_clock = true;
                } else if self.apu_framecounter == 24939 {
                    self.apu_quarter_frame_clock = true;
                } else if self.apu_framecounter == 41565 {
                    self.apu_quarter_frame_clock = true; self.apu_half_frame_clock = true;
                } else if self.apu_framecounter == 41566 {
                    self.apu_framecounter = 0;
                }
            } else {
                // 4-step (PAL)
                if self.apu_framecounter == 8313 {
                    self.apu_quarter_frame_clock = true;
                } else if self.apu_framecounter == 16627 {
                    self.apu_quarter_frame_clock = true; self.apu_half_frame_clock = true;
                } else if self.apu_framecounter == 24939 {
                    self.apu_quarter_frame_clock = true;
                } else if self.apu_framecounter == 33252 {
                    self.apu_status_frame_interrupt = true;
                } else if self.apu_framecounter == 33253 {
                    self.apu_quarter_frame_clock = true;
                    self.apu_status_frame_interrupt = true;
                    self.irq_level_detector |= !self.apu_frame_counter_inhibit_irq;
                    self.apu_half_frame_clock = true;
                } else if self.apu_framecounter == 33254 {
                    self.apu_status_frame_interrupt = !self.apu_frame_counter_inhibit_irq;
                    self.irq_level_detector |= !self.apu_frame_counter_inhibit_irq;
                    self.apu_framecounter = 0;
                }
            }
        } else {
            // NTSC
            if self.apu_frame_counter_mode {
                // 5-step (NTSC)
                if self.apu_framecounter == 7457 {
                    self.apu_quarter_frame_clock = true;
                } else if self.apu_framecounter == 14913 {
                    self.apu_quarter_frame_clock = true; self.apu_half_frame_clock = true;
                } else if self.apu_framecounter == 22371 {
                    self.apu_quarter_frame_clock = true;
                } else if self.apu_framecounter == 37281 {
                    self.apu_quarter_frame_clock = true; self.apu_half_frame_clock = true;
                } else if self.apu_framecounter == 37282 {
                    self.apu_framecounter = 0;
                }
            } else {
                // 4-step (NTSC)
                if self.apu_framecounter == 7457 {
                    self.apu_quarter_frame_clock = true;
                } else if self.apu_framecounter == 14913 {
                    self.apu_quarter_frame_clock = true; self.apu_half_frame_clock = true;
                } else if self.apu_framecounter == 22371 {
                    self.apu_quarter_frame_clock = true;
                } else if self.apu_framecounter == 29828 {
                    self.apu_status_frame_interrupt = true;
                } else if self.apu_framecounter == 29829 {
                    self.apu_quarter_frame_clock = true;
                    self.apu_status_frame_interrupt = true;
                    self.irq_level_detector |= !self.apu_frame_counter_inhibit_irq;
                    self.apu_half_frame_clock = true;
                } else if self.apu_framecounter == 29830 {
                    self.apu_status_frame_interrupt = !self.apu_frame_counter_inhibit_irq;
                    self.irq_level_detector |= !self.apu_frame_counter_inhibit_irq;
                    self.apu_framecounter = 0;
                }
            }
        }

        // quarter frame
        if self.apu_quarter_frame_clock {
            self.apu_quarter_frame_clock = false;
            if self.apu_envelope_start_flag {
                self.apu_envelope_start_flag = false;
                self.apu_envelope_decay_level = 15;
            } else {
                self.apu_envelope_divider_clock = true;
            }

            // pulse 1 envelope
            let p1_env_param = self.apu_register[0] & 0xF;
            let p1_loop = (self.apu_register[0] & 0x20) != 0;
            if self.pulse1_envelope_start_flag {
                self.pulse1_envelope_start_flag = false;
                self.pulse1_envelope_decay_level = 15;
                self.pulse1_envelope_divider = p1_env_param;
            } else {
                if self.pulse1_envelope_divider == 0 {
                    self.pulse1_envelope_divider = p1_env_param;
                    if self.pulse1_envelope_decay_level > 0 {
                        self.pulse1_envelope_decay_level -= 1;
                    } else if p1_loop {
                        self.pulse1_envelope_decay_level = 15;
                    }
                } else {
                    self.pulse1_envelope_divider = self.pulse1_envelope_divider.saturating_sub(1);
                }
            }

            // pulse 2 envelope
            let p2_env_param = self.apu_register[4] & 0xF;
            let p2_loop = (self.apu_register[4] & 0x20) != 0;
            if self.pulse2_envelope_start_flag {
                self.pulse2_envelope_start_flag = false;
                self.pulse2_envelope_decay_level = 15;
                self.pulse2_envelope_divider = p2_env_param;
            } else {
                if self.pulse2_envelope_divider == 0 {
                    self.pulse2_envelope_divider = p2_env_param;
                    if self.pulse2_envelope_decay_level > 0 {
                        self.pulse2_envelope_decay_level -= 1;
                    } else if p2_loop {
                        self.pulse2_envelope_decay_level = 15;
                    }
                } else {
                    self.pulse2_envelope_divider = self.pulse2_envelope_divider.saturating_sub(1);
                }
            }

            // noise envelope
            let noise_env_param = self.apu_register[0xC] & 0xF;
            let noise_loop = (self.apu_register[0xC] & 0x20) != 0;
            if self.noise_envelope_start_flag {
                self.noise_envelope_start_flag = false;
                self.noise_envelope_decay_level = 15;
                self.noise_envelope_divider = noise_env_param;
            } else {
                if self.noise_envelope_divider == 0 {
                    self.noise_envelope_divider = noise_env_param;
                    if self.noise_envelope_decay_level > 0 {
                        self.noise_envelope_decay_level -= 1;
                    } else if noise_loop {
                        self.noise_envelope_decay_level = 15;
                    }
                } else {
                    self.noise_envelope_divider = self.noise_envelope_divider.saturating_sub(1);
                }
            }

            // triangle linear counter
            let triangle_control = (self.apu_register[8] & 0x80) != 0;
            let triangle_reload_value = self.apu_register[8] & 0x7F;
            if self.triangle_linear_counter_reload_flag {
                self.triangle_linear_counter = triangle_reload_value;
            } else if self.triangle_linear_counter > 0 {
                self.triangle_linear_counter -= 1;
            }
            if !triangle_control {
                self.triangle_linear_counter_reload_flag = false;
            }
        }

        // half frame
        if self.apu_half_frame_clock {
            // length counter reload on half frame
            if self.apu_length_counter_reload_pulse1 && self.apu_length_counter_pulse1 == 0 {
                self.apu_length_counter_pulse1 = self.apu_length_counter_reload_value_pulse1;
            } else { self.apu_length_counter_reload_pulse1 = false; }
            if self.apu_length_counter_reload_pulse2 && self.apu_length_counter_pulse2 == 0 {
                self.apu_length_counter_pulse2 = self.apu_length_counter_reload_value_pulse2;
            } else { self.apu_length_counter_reload_pulse2 = false; }
            if self.apu_length_counter_reload_triangle && self.apu_length_counter_triangle == 0 {
                self.apu_length_counter_triangle = self.apu_length_counter_reload_value_triangle;
            } else { self.apu_length_counter_reload_triangle = false; }
            if self.apu_length_counter_reload_noise && self.apu_length_counter_noise == 0 {
                self.apu_length_counter_noise = self.apu_length_counter_reload_value_noise;
            } else { self.apu_length_counter_reload_noise = false; }

            self.apu_half_frame_clock = false;

            if !self.apu_status_pulse1 { self.apu_length_counter_pulse1 = 0; }
            if !self.apu_status_pulse2 { self.apu_length_counter_pulse2 = 0; }
            if !self.apu_status_triangle { self.apu_length_counter_triangle = 0; }
            if !self.apu_status_noise { self.apu_length_counter_noise = 0; }

            if self.apu_length_counter_pulse1 != 0 && !self.apu_length_counter_halt_pulse1 && !self.apu_length_counter_reload_pulse1 {
                self.apu_length_counter_pulse1 -= 1;
            }
            if self.apu_length_counter_pulse2 != 0 && !self.apu_length_counter_halt_pulse2 && !self.apu_length_counter_reload_pulse2 {
                self.apu_length_counter_pulse2 -= 1;
            }
            if self.apu_length_counter_triangle != 0 && !self.apu_length_counter_halt_triangle && !self.apu_length_counter_reload_triangle {
                self.apu_length_counter_triangle -= 1;
            }
            if self.apu_length_counter_noise != 0 && !self.apu_length_counter_halt_noise && !self.apu_length_counter_reload_noise {
                self.apu_length_counter_noise -= 1;
            }

            // pulse 1 sweep
            let p1_sweep_reg = self.apu_register[1];
            let p1_sweep_enabled = (p1_sweep_reg & 0x80) != 0;
            let p1_sweep_period = (p1_sweep_reg >> 4) & 0x7;
            let p1_sweep_shift = p1_sweep_reg & 0x7;
            let p1_current_period = (self.apu_register[2] as u16) | (((self.apu_register[3] & 0x7) as u16) << 8);
            let p1_target_period = self.pulse_target_period(1, p1_current_period, p1_sweep_reg);

            let p1_sweep_clock = if self.pulse1_sweep_divider == 0 {
                self.pulse1_sweep_divider = p1_sweep_period;
                true
            } else {
                self.pulse1_sweep_divider = self.pulse1_sweep_divider.saturating_sub(1);
                false
            };

            if p1_sweep_clock {
                if p1_sweep_enabled && p1_sweep_shift > 0 && p1_current_period >= 8 && p1_target_period <= 0x7FF {
                    self.apu_register[2] = (p1_target_period & 0xFF) as u8;
                    self.apu_register[3] = (self.apu_register[3] & !0x7) | ((p1_target_period >> 8) & 0x7) as u8;
                    self.apu_channel_timer_pulse1 = (self.apu_channel_timer_pulse1 & !0x700) | (p1_target_period & 0x700);
                }
            }

            if self.pulse1_sweep_reload {
                self.pulse1_sweep_divider = p1_sweep_period;
                self.pulse1_sweep_reload = false;
            }

            // pulse 2 sweep
            let p2_sweep_reg = self.apu_register[5];
            let p2_sweep_enabled = (p2_sweep_reg & 0x80) != 0;
            let p2_sweep_period = (p2_sweep_reg >> 4) & 0x7;
            let p2_sweep_shift = p2_sweep_reg & 0x7;
            let p2_current_period = (self.apu_register[6] as u16) | (((self.apu_register[7] & 0x7) as u16) << 8);
            let p2_target_period = self.pulse_target_period(2, p2_current_period, p2_sweep_reg);

            let p2_sweep_clock = if self.pulse2_sweep_divider == 0 {
                self.pulse2_sweep_divider = p2_sweep_period;
                true
            } else {
                self.pulse2_sweep_divider = self.pulse2_sweep_divider.saturating_sub(1);
                false
            };

            if p2_sweep_clock {
                if p2_sweep_enabled && p2_sweep_shift > 0 && p2_current_period >= 8 && p2_target_period <= 0x7FF {
                    self.apu_register[6] = (p2_target_period & 0xFF) as u8;
                    self.apu_register[7] = (self.apu_register[7] & !0x7) | ((p2_target_period >> 8) & 0x7) as u8;
                    self.apu_channel_timer_pulse2 = (self.apu_channel_timer_pulse2 & !0x700) | (p2_target_period & 0x700);
                }
            }

            if self.pulse2_sweep_reload {
                self.pulse2_sweep_divider = p2_sweep_period;
                self.pulse2_sweep_reload = false;
            }
        } else {
            // non-half-frame reload
            if self.apu_length_counter_reload_pulse1 { self.apu_length_counter_pulse1 = self.apu_length_counter_reload_value_pulse1; }
            if self.apu_length_counter_reload_pulse2 { self.apu_length_counter_pulse2 = self.apu_length_counter_reload_value_pulse2; }
            if self.apu_length_counter_reload_triangle { self.apu_length_counter_triangle = self.apu_length_counter_reload_value_triangle; }
            if self.apu_length_counter_reload_noise { self.apu_length_counter_noise = self.apu_length_counter_reload_value_noise; }
            self.apu_length_counter_reload_pulse1 = false;
            self.apu_length_counter_reload_pulse2 = false;
            self.apu_length_counter_reload_triangle = false;
            self.apu_length_counter_reload_noise = false;
        }

        // update halt flags from registers
        self.apu_length_counter_halt_pulse1 = (self.apu_register[0] & 0x20) != 0;
        self.apu_length_counter_halt_pulse2 = (self.apu_register[4] & 0x20) != 0;
        self.apu_length_counter_halt_triangle = (self.apu_register[8] & 0x80) != 0;
        self.apu_length_counter_halt_noise = (self.apu_register[0xC] & 0x20) != 0;

        // downsample and queue audio sample
        if let Some(ref buffer) = self.audio_buffer {
            // accumulate mix on every cpu cycle for anti-aliasing
            let current_mix = self.mix_apu();
            self.audio_sample_accumulator += current_mix;
            self.audio_sample_count += 1.0;

            self.audio_cycles_accumulator += 1.0;

            let cycles_per_sample = self.cpu_clock() / self.audio_host_sample_rate;
            if self.audio_cycles_accumulator >= cycles_per_sample {
                let avg_sample = self.audio_sample_accumulator / self.audio_sample_count;

                // --- audio filters ---
                // 1. low-pass filter
                let lp_out = self.filter_lp_prev_out + self.filter_lp_alpha * (avg_sample - self.filter_lp_prev_out);
                self.filter_lp_prev_out = lp_out;

                // 2. high-pass filter 1 (440 Hz)
                let hp1_out = self.filter_hp1_prev_out + self.filter_hp1_alpha * (lp_out - self.filter_hp1_prev_in);
                self.filter_hp1_prev_in = lp_out;
                self.filter_hp1_prev_out = hp1_out;

                // 3. high-pass filter 2 (90 Hz)
                let hp2_out = self.filter_hp2_prev_out + self.filter_hp2_alpha * (hp1_out - self.filter_hp2_prev_in);
                self.filter_hp2_prev_in = hp1_out;
                self.filter_hp2_prev_out = hp2_out;

                let mut filtered_sample = hp2_out;

                // soft-clip to prevent harsh crackling from out-of-range samples
                if filtered_sample > 1.0 {
                    filtered_sample = 1.0;
                } else if filtered_sample < -1.0 {
                    filtered_sample = -1.0;
                }

                // apply audio depth
                if self.audio_depth == 8 {
                    filtered_sample = (filtered_sample * 127.0).round() / 127.0;
                }

                if self.audio_enabled {
                    let queue_limit = ((self.audio_host_sample_rate * 0.1) as usize).max(4096);
                    let mut queue = buffer.lock().unwrap();
                    if queue.len() < queue_limit {
                        queue.push_back(filtered_sample);
                    } else {
                        queue.pop_front();
                        queue.push_back(filtered_sample);
                    }
                }
                self.audio_cycles_accumulator -= cycles_per_sample;
                self.audio_sample_accumulator = 0.0;
                self.audio_sample_count = 0.0;
            }
        }
    }

    fn apu_channel_pulse1_sub(&self) -> u16 {
        self.apu_channel_timer_pulse1.wrapping_sub(1)
    }

    fn apu_channel_pulse2_sub(&self) -> u16 {
        self.apu_channel_timer_pulse2.wrapping_sub(1)
    }
}
