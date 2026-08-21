// the ppu of the nes console is probably for me the most complex component to emulate properly and accurately.
// i hope this is accurate enough!

use crate::emulator::{Emulator, Vt369SpriteEntry, Vt369SpriteHiEntry};

impl Emulator {
    /// ppu cycle
    pub fn emulate_ppu(&mut self) {
        self.copy_v = false;
        if self.ppu_update_2006_delay > 0 {
            self.ppu_update_2006_delay -= 1;
            if self.ppu_update_2006_delay == 0 {
                let temp_prev_v = self.ppu_v;
                self.copy_v = true;
                self.ppu_v = self.ppu_t;
                self.ppu_address_bus = self.ppu_v;
                if (temp_prev_v & 0x3FFF) >= 0x3F00 && (self.ppu_address_bus & 0x3FFF) < 0x3F00 {
                    if self.ppu_scanline < 240 && self.ppu_dot <= 256 {
                        if (temp_prev_v & 0xF) != 0 {
                            self.ppu_v_register_changed_out_of_vblank = true;
                        }
                    }
                }
            }
        }

        if self.ppu_update_2005_delay > 0 {
            self.ppu_update_2005_delay -= 1;
            if self.ppu_update_2005_delay == 0 {
                if !self.ppu_addr_latch {
                    self.ppu_fine_x_scroll = self.ppu_update_2005_value & 7;
                    self.ppu_t = (self.ppu_t & 0b0111111111100000)
                        | ((self.ppu_update_2005_value as u16) >> 3);
                } else {
                    self.ppu_t = (self.ppu_t & 0b0000110000011111)
                        | (((self.ppu_update_2005_value as u16 & 0xF8) << 2)
                            | ((self.ppu_update_2005_value as u16 & 7) << 12));
                }
                self.ppu_addr_latch = !self.ppu_addr_latch;
            }
        }

        self.ppu_dot += 1;
        if self.ppu_dot > 340 {
            self.ppu_dot = 0;
            self.ppu_scanline += 1;
            if self.ppu_scanline >= self.total_scanlines() {
                self.ppu_scanline = 0;
            }
        }

        // vblank timing
        let nmi_scan = self.nmi_scanline();
        if self.ppu_scanline == nmi_scan {
            if self.ppu_dot == 1 {
                self.ppu_reset = false;
                self.frame_advance_reached_vblank = true;
            }
            if self.ppu_dot == 0 {
                self.ppu_pending_vblank = true;
            }
        } else if self.ppu_scanline == self.pre_render_scanline() && self.ppu_dot == 1 {
            self.ppu_status_vblank = false;
            self.ppu_can_detect_sprite_zero_hit = true;
            self.ppu_status_sprite_zero_hit = false;
            self.ppu_status_sprite_overflow = false;
            self.ppu_status_sprite_zero_hit_delayed = false;
        } else if self.ppu_scanline == self.pre_render_scanline().wrapping_sub(1) && self.ppu_dot == 340 {
            self.ppu_odd_frame = !self.ppu_odd_frame;
        }

        self.ppu_vset_latch1 = !self.ppu_vset;
        if self.ppu_vset && !self.ppu_vset_latch2 {
            self.ppu_status_vblank = true;
        }
        if self.ppu_read_2002 {
            self.ppu_read_2002 = false;
            self.ppu_status_vblank = false;
        }

        self.ppu_status_sprite_overflow_delayed = self.ppu_status_sprite_overflow;

        let mapper_scanline = self.mapper_scanline();
        if let Some(cart) = self.cart.as_mut() {
            let rendering_on =
                self.ppu_mask_show_background || self.ppu_mask_show_sprites;
            if cart.mapper_chip.ppu_clock(
                self.ppu_address_bus,
                self.ppu_a12_prev,
                mapper_scanline,
                self.ppu_dot,
                self.ppu_sprite_x16,
                rendering_on,
            ) {
                self.irq_level_detector = true;
            }
        }
        // mapper irq a12 detection
        self.ppu_a12_prev = (self.ppu_address_bus & 0b0001000000000000) != 0;

        // ntsc odd frame skip
        if !self.is_pal() && !self.is_dendy() {
            if self.ppu_odd_frame
                && (self.ppu_mask_show_background || self.ppu_mask_show_sprites)
                && self.ppu_scanline == 0
                && self.ppu_dot == 0
            {
                self.ppu_dot += 1;
                self.skipped_pre_render_dot_341 = true;
            }
            if self.ppu_odd_frame
                && (self.ppu_mask_show_background || self.ppu_mask_show_sprites)
                && self.ppu_scanline == 0
                && self.ppu_dot == 2
            {
                self.skipped_pre_render_dot_341 = false;
            }
        }


        if (self.cpu_clock & 3) != 3 {
            self.ppu_mask_show_background_delayed = self.ppu_mask_show_background;
            self.ppu_mask_show_sprites_delayed = self.ppu_mask_show_sprites;
        }

        self.ppu_data_state_machine();

        // sprite eval logic
        if self.ppu_scanline < 240 || self.ppu_scanline == self.pre_render_scanline() {
            if self.ppu_scanline < 241 || self.ppu_scanline == self.pre_render_scanline() {
                self.ppu_sprite_evaluation();
            }
        }

        self.ppu_mask_show_background_delayed = self.ppu_mask_show_background;
        self.ppu_mask_show_sprites_delayed = self.ppu_mask_show_sprites;

        if !self.ppu_mask_show_background && !self.ppu_mask_show_sprites {
            self.ppu_address_bus = self.ppu_v & 0x3FFF;
        }

        let temp_rendering = self.ppu_mask_show_background || self.ppu_mask_show_sprites;
        if self.ppu_update_2001_delay > 0 {
            self.ppu_update_2001_delay -= 1;
            if self.ppu_update_2001_delay == 0 {
                self.ppu_mask_8px_show_background = (self.ppu_update_2001_value & 0x02) != 0;
                self.ppu_mask_8px_show_sprites = (self.ppu_update_2001_value & 0x04) != 0;
                self.ppu_mask_show_background = (self.ppu_update_2001_value & 0x08) != 0;
                self.ppu_mask_show_sprites = (self.ppu_update_2001_value & 0x10) != 0;
                self.ppu_mask_show_background_instant = self.ppu_mask_show_background;
                self.ppu_mask_show_sprites_instant = self.ppu_mask_show_sprites;

                let temp_rendering_from_input = self.ppu_mask_show_background || self.ppu_mask_show_sprites;
                if temp_rendering && !temp_rendering_from_input {
                    if self.ppu_scanline < 241 || self.ppu_scanline == self.pre_render_scanline() {
                        if (self.ppu_v & 0x3FFF) >= 0x3C00 {
                            self.ppu_palette_corruption_rendering_disabled_out_of_vblank = true;
                        }
                    }
                } else if !temp_rendering && temp_rendering_from_input {
                    if self.ppu_scanline < 241 || self.ppu_scanline == self.pre_render_scanline() {
                        if self.ppu_pending_oam_corruption {
                            let alignment = self.ppu_clock & 3;
                            if alignment == 1 || alignment == 2 {
                                self.ppu_oam_corruption_rendering_enabled_out_of_vblank = true;
                            }
                        }
                    }
                }
            }
        }


        if self.ppu_update_2001_emphasis_bits_delay > 0 {
            self.ppu_update_2001_emphasis_bits_delay -= 1;
            if self.ppu_update_2001_emphasis_bits_delay == 0 {
                self.ppu_mask_greyscale = (self.ppu_update_2001_value & 0x01) != 0;
                if self.is_pal() || self.is_dendy() {
                    self.ppu_mask_emphasize_red = (self.ppu_update_2001_value & 0x40) != 0;
                    self.ppu_mask_emphasize_green = (self.ppu_update_2001_value & 0x20) != 0;
                } else {
                    self.ppu_mask_emphasize_red = (self.ppu_update_2001_value & 0x20) != 0;
                    self.ppu_mask_emphasize_green = (self.ppu_update_2001_value & 0x40) != 0;
                }
                self.ppu_mask_emphasize_blue = (self.ppu_update_2001_value & 0x80) != 0;
            }
        }

        self.prev_prev_prev_dot_color = self.prev_prev_dot_color;
        self.prev_prev_dot_color = self.prev_dot_color;
        self.prev_dot_color = self.dot_color;

        self.prev_prev_prev_dot_color_rgb = self.prev_prev_dot_color_rgb;
        self.prev_prev_dot_color_rgb = self.prev_dot_color_rgb;
        self.prev_dot_color_rgb = self.dot_color_rgb;

        if self.ppu_scanline < 240 || self.ppu_scanline == self.pre_render_scanline() {
            if (self.ppu_dot >= 1 && self.ppu_dot <= 256) || (self.ppu_dot >= 321 && self.ppu_dot <= 336) {
                if self.ppu_mask_show_background_delayed || self.ppu_mask_show_sprites_delayed {
                    self.ppu_render_bg_fetches();
                }
            } else if self.ppu_dot >= 337 || self.ppu_dot == 0 {
                if self.ppu_mask_show_background_delayed || self.ppu_mask_show_sprites_delayed {
                    self.ppu_render_bg_dummy_nt();
                }
            }

            if self.ppu_dot > 0 && self.ppu_dot <= 256 && self.ppu_scanline < 241 {
                self.ppu_render_calculate_pixel();
            }

            if self.ppu_dot > 0 && self.ppu_dot <= 256 {
                self.update_sprite_shift_registers();
            }

            // bg shift register updating
            if self.ppu_scanline < 240 || self.ppu_scanline == self.pre_render_scanline() {
                if (self.ppu_dot >= 1 && self.ppu_dot <= 257) || (self.ppu_dot >= 321 && self.ppu_dot <= 336) {
                    if self.ppu_mask_show_background || self.ppu_mask_show_sprites {
                        self.ppu_update_bg_shift_registers();
                    }
                }
            }

            self.draw_to_screen();
        }

        if self.ppu_update_2001_oam_corruption_delay > 0 {
            self.ppu_update_2001_oam_corruption_delay -= 1;
            if self.ppu_update_2001_oam_corruption_delay == 0 {
                if self.ppu_was_rendering_before_2001_write && (self.ppu_update_2001_value & 0x18) == 0 {
                    if self.ppu_scanline < 240 || self.ppu_scanline == self.pre_render_scanline() {
                        if !self.ppu_pending_oam_corruption {
                            self.ppu_oam_corruption_rendering_disabled_out_of_vblank = true;
                        }
                    }
                }
            }
        }

        // oam buffer logic
        if (self.ppu_mask_show_background || self.ppu_mask_show_sprites) && self.ppu_scanline < 240 {
            if self.ppu_dot == 0 || self.ppu_dot > 320 {
                self.ppu_oam_buffer = self.oam2[0];
            } else if self.ppu_dot <= 64 {
                self.ppu_oam_buffer = 0xFF;
            } else {
                self.ppu_oam_buffer = self.ppu_oam_latch;
            }
        }

        self.ppu_data_state_machine2();

        self.decay_ppu_data_bus();
    }

    // half a ppu cycle for timing sensitive stuff
    pub fn emulate_half_ppu(&mut self) {
        self.ppu_vset_latch2 = !self.ppu_vset_latch1;

        self.ppu_status_sprite_zero_hit_delayed = self.ppu_status_sprite_zero_hit;

        self.ppu_vset = false;
        if self.ppu_pending_vblank {
            self.ppu_pending_vblank = false;
            self.ppu_vset = true;
        }

        if self.ppu_status_pending_sprite_zero_hit2 {
            self.ppu_status_pending_sprite_zero_hit2 = false;
            self.ppu_status_sprite_zero_hit = true;
        }
        if self.ppu_status_pending_sprite_zero_hit {
            self.ppu_status_pending_sprite_zero_hit = false;
            self.ppu_status_pending_sprite_zero_hit2 = true;
        }



        if self.ppu_mask_show_background || self.ppu_mask_show_sprites {
            if self.ppu_scanline < 240 || self.ppu_scanline == self.pre_render_scanline() {
                if self.ppu_dot == 256 {
                    if self.vt369_enhanced_ppu() && (self.ppu_scanline < 240 || self.ppu_scanline == self.pre_render_scanline()) {
                        self.vt369_tile_data.fill(0);
                        self.vt369_tile_data_even.fill(0);
                        self.vt369_tile_data_odd.fill(0);
                        if self.vt369_hires_ppu() {
                            self.vt369_process_sprites_hires();
                        } else {
                            self.vt369_process_sprites_enhanced();
                        }
                    }
                    self.ppu_increment_scroll_y();
                }
                if self.ppu_dot == 257 {
                    self.ppu_reset_x_scroll();
                }
            }
            if self.ppu_scanline == self.pre_render_scanline() && self.ppu_dot >= 280 && self.ppu_dot <= 304 {
                self.ppu_reset_y_scroll();
            }
        }

        self.ppu_data_state_machine_half();

        self.ppu_render_commit_shift_registers_and_bit_planes();
    }

    // ppu data state machine 1
    pub fn ppu_data_state_machine(&mut self) {
        let blnk = (!self.ppu_mask_show_background && !self.ppu_mask_show_sprites)
            || (self.ppu_scanline >= 240 && self.ppu_scanline < self.pre_render_scanline());
        self.ppu_2007_blnk_latch = blnk;
        let h0_dash = ((self.ppu_dot.wrapping_sub(1)) & 1) != 0;

        self.ppu_2007_palette_ram_enable =
            ((self.ppu_address_bus & 0x3F00) == 0x3F00) && self.ppu_2007_blnk_latch;
        self.ppu_2007_read_xrb = self.ppu_2007_read && self.ppu_2007_palette_ram_enable;

        self.ppu_2007_read_latches[4] = !self.ppu_2007_read_latches[3];
        self.ppu_2007_read_latches[2] = !self.ppu_2007_read_latches[1];
        self.ppu_2007_read_latches[0] = self.ppu_2007_read_sr;
        if self.ppu_2007_read {
            self.ppu_2007_read = false;
        }

        self.ppu_2007_pd_rb = self.ppu_2007_read_latches[4] && !self.ppu_2007_read_latches[2];
        self.ppu_2007_read_ale = !self.ppu_2007_read_latches[4] && self.ppu_2007_read_latches[2];
        self.ppu_2007_read_h0_latch = ((self.ppu_dot.wrapping_sub(1)) & 1) != 0;

        self.ppu_read = self.ppu_2007_pd_rb || (!blnk && self.ppu_2007_read_h0_latch);

        self.ppu_2007_write_latches[4] = !self.ppu_2007_write_latches[3];
        self.ppu_2007_write_latches[2] = !self.ppu_2007_write_latches[1];
        self.ppu_2007_write_latches[0] = self.ppu_2007_write_sr;
        if self.ppu_2007_write {
            self.ppu_2007_write = false;
        }
        self.ppu_2007_write_ale = !self.ppu_2007_write_latches[4] && self.ppu_2007_write_latches[2];

        self.ppu_2007_tstep_latch = self.ppu_2007_db_par;

        let b = !blnk && !h0_dash;
        self.ppu_ale = self.ppu_2007_read_ale || self.ppu_2007_write_ale || b;

        if self.ppu_ale && !self.ppu_read {
            self.ppu_octal_latch = self.ppu_address_bus as u8;
        }

        if self.ppu_2007_read_ale || self.ppu_2007_write_ale {
            if !self.ppu_read {
                self.ppu_address_bus = self.ppu_v & 0x3FFF;
                self.ppu_octal_latch = self.ppu_address_bus as u8;
            }
        }
    }

    // ppu data state machine part 2
    pub fn ppu_data_state_machine2(&mut self) {
        if self.ppu_2007_pd_rb {
            self.ppu_read_buffer = self.fetch_ppu();
            if self.ppu_ale {
                self.ppu_octal_latch = self.ppu_address_bus as u8;
            }
        }
    }

    // ppu data state machine for only half a cycle
    pub fn ppu_data_state_machine_half(&mut self) {
        self.ppu_2007_tstep = self.ppu_2007_tstep_latch || self.ppu_2007_pd_rb;
        if self.ppu_2007_tstep {
            if !self.ppu_2007_blnk_latch {
                self.ppu_increment_scroll_y();
            } else {
                self.ppu_v = (self.ppu_v.wrapping_add(
                    if self.ppu_control_increment_mode_32 { 32 } else { 1 }
                )) & 0x7FFF;
                let vt03_ppu = self
                    .cart
                    .as_ref()
                    .map_or(false, |c| c.mapper_chip.onebus_vt03_ppu());
                if vt03_ppu
                    && self.do_oam_dma
                    && !self.is_pal()
                    && !self.is_dendy()
                    && self.ppu_v == 0x4000
                {
                    self.ppu_v = 0x3F00;
                }
            }
        }

        self.ppu_ale = self.ppu_2007_read_ale || self.ppu_2007_write_ale;
        if self.ppu_2007_pd_rb {
            self.ppu_read_buffer = self.fetch_ppu();
            if self.ppu_ale {
                self.ppu_octal_latch = self.ppu_address_bus as u8;
            }
        }

        self.ppu_2007_read_latches[1] = !self.ppu_2007_read_latches[0];
        self.ppu_2007_read_latches[3] = !self.ppu_2007_read_latches[2];
        if !self.ppu_2007_read_latches[3] {
            self.ppu_2007_read_sr = false;
        }

        self.ppu_2007_write_latches[1] = !self.ppu_2007_write_latches[0];
        self.ppu_2007_write_latches[3] = !self.ppu_2007_write_latches[2];
        if !self.ppu_2007_write_latches[3] {
            self.ppu_2007_write_sr = false;
        }

        self.ppu_2007_db_par = self.ppu_2007_write_latches[1] && !self.ppu_2007_write_latches[3];
        self.ppu_write = !self.ppu_2007_palette_ram_enable && self.ppu_2007_db_par;
        if self.ppu_2007_db_par {
            let addr = self.ppu_address_bus;
            let data = self.ppu_2007_write_data;
            self.store_ppu_data(addr, data);
        }
    }

    /// mapper ppu bus interactions
    pub(crate) fn fetch_ppu(&mut self) -> u8 {
        if self.copy_v {
            self.ppu_address_bus = self.ppu_v;
        }

        let addr = (self.ppu_address_bus & 0x7F00) | self.ppu_octal_latch as u16;

        let (data, new_addr_bus) = if let Some(cart) = self.cart.as_mut() {
            cart.mapper_chip.fetch_ppu_with_ctx(
                &cart.prg_rom,
                &cart.chr_rom,
                &cart.prg_ram,
                &cart.chr_ram,
                &cart.prg_vram,
                cart.using_chr_ram,
                cart.nametable_horizontal_mirroring,
                cart.alternative_nametable_arrangement,
                self.ppu_address_bus,
                self.ppu_octal_latch,
                &self.vram,
                self.ppu_onebus_chr,
            )
        } else {
            (0, addr)
        };

        self.ppu_address_bus = new_addr_bus;
        data
    }

    fn onebus_chr_routing_ppu(&self) -> bool {
        self.cart
            .as_ref()
            .map_or(false, |c| c.mapper_chip.onebus_chr_routing_ppu())
    }

    fn onebus_begin_bg_chr_fetch(&mut self) {
        if !self.onebus_chr_routing_ppu() {
            return;
        }
        self.ppu_onebus_chr = crate::mappers::one_bus::OneBusChrCtx {
            eva: self.ppu_onebus_eva,
            is_bg: true,
            is_sprite: false,
            active: true,
        };
    }

    fn onebus_begin_sprite_chr_fetch(&mut self, slot: usize) {
        if !self.onebus_chr_routing_ppu() {
            return;
        }
        let reg2000_10 = self
            .cart
            .as_ref()
            .map_or(0, |c| c.mapper_chip.vt03_reg2000_10());
        let eva = if (reg2000_10 & 0x08) != 0 && slot < 8 {
            ((self.ppu_sprite_attribute[slot] & 3) as u16) << 10
        } else {
            0
        };
        self.ppu_onebus_chr = crate::mappers::one_bus::OneBusChrCtx {
            eva,
            is_bg: false,
            is_sprite: true,
            active: true,
        };
    }

    fn onebus_clear_chr_fetch(&mut self) {
        self.ppu_onebus_chr.active = false;
    }

    // ppu register writes
    pub fn store_ppu_registers(&mut self, addr: u16, input: u8) {
        if self.vt369_enhanced_ppu() && (addr & 0x200F) == 0x2008 {
            self.ppu_bus = input;
            for i in 0..8 {
                self.ppu_bus_decay[i] = 1786830;
            }
            self.vt369_spr_addr_high = input & 1;
            return;
        }
        let reg = addr & 0x2007;
        match reg {
            0x2000 => {
                self.ppu_bus = input;
                for i in 0..8 { self.ppu_bus_decay[i] = 1786830; }
                if self.ppu_reset { return; }
                self.ppu_t = (self.ppu_t & 0b0111001111111111) | (((input & 0x3) as u16) << 10);
                self.emulate_n_master_clock_cycles(2);
                self.ppu_control_nmi_enabled = (input & 0x80) != 0;
                self.ppu_control_increment_mode_32 = (input & 0x4) != 0;
                self.ppu_sprite_x16 = (input & 0x20) != 0;
                self.ppu_pattern_select_sprites = (input & 0x8) != 0;
                self.ppu_pattern_select_background = (input & 0x10) != 0;
            }
            0x2001 => {
                self.ppu_bus = input;
                for i in 0..8 { self.ppu_bus_decay[i] = 1786830; }
                if self.ppu_reset { return; }
                let alignment = self.ppu_clock & 3;
                match alignment {
                    0 => { self.ppu_update_2001_delay = 2; self.ppu_update_2001_emphasis_bits_delay = 2; self.ppu_update_2001_oam_corruption_delay = 2; }
                    1 => { self.ppu_update_2001_delay = 2; self.ppu_update_2001_emphasis_bits_delay = 1; self.ppu_update_2001_oam_corruption_delay = 3; }
                    2 => { self.ppu_update_2001_delay = 3; self.ppu_update_2001_emphasis_bits_delay = 1; self.ppu_update_2001_oam_corruption_delay = 3; }
                    3 | _ => { self.ppu_update_2001_delay = 2; self.ppu_update_2001_emphasis_bits_delay = 2; self.ppu_update_2001_oam_corruption_delay = 2; }
                }
                self.ppu_was_rendering_before_2001_write = self.ppu_mask_show_background || self.ppu_mask_show_sprites;
                self.ppu_mask_show_background_instant = self.ppu_mask_show_background;
                self.ppu_mask_show_sprites_instant = self.ppu_mask_show_sprites;
                if self.is_pal() || self.is_dendy() {
                    self.ppu_mask_emphasize_red = (input & 0x40) != 0;
                    self.ppu_mask_emphasize_green = (input & 0x20) != 0;
                } else {
                    self.ppu_mask_emphasize_red = (input & 0x20) != 0;
                    self.ppu_mask_emphasize_green = (input & 0x40) != 0;
                }
                if self.ppu_update_2001_emphasis_bits_delay == 2 {
                    self.ppu_mask_greyscale = (input & 0x01) != 0;
                    self.ppu_mask_emphasize_blue = (input & 0x80) != 0;
                } else {
                    self.ppu_update_2001_emphasis_bits_delay += 1;
                }
                let temp_rendering = self.ppu_mask_show_background || self.ppu_mask_show_sprites;
                let temp_rendering_from_input = (input & 0x18) != 0;
                if temp_rendering && !temp_rendering_from_input {
                    if self.ppu_scanline < 240 || self.ppu_scanline == self.pre_render_scanline() {
                        self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant = true;
                    }
                }
                if !temp_rendering && temp_rendering_from_input {
                    if self.ppu_pending_oam_corruption {
                        if self.ppu_scanline < 240 || self.ppu_scanline == self.pre_render_scanline() {
                            self.ppu_oam_corruption_rendering_enabled_out_of_vblank = true;
                        }
                    }
                }

                self.ppu_update_2001_value = input;
            }
            0x2002 => {
                self.ppu_bus = input;
                for i in 0..8 { self.ppu_bus_decay[i] = 1786830; }
            }
            0x2003 => {
                self.ppu_bus = input;
                for i in 0..8 { self.ppu_bus_decay[i] = 1786830; }
                self.ppu_oam_address = self.ppu_bus;
            }
            0x2004 => {
                self.ppu_bus = input;
                for i in 0..8 { self.ppu_bus_decay[i] = 1786830; }
                let can_write = ((self.ppu_scanline >= 240 && self.ppu_scanline < self.pre_render_scanline())
                    && (self.ppu_mask_show_background || self.ppu_mask_show_sprites))
                    || (!self.ppu_mask_show_background && !self.ppu_mask_show_sprites);
                if can_write {
                    let mut val = input;
                    if self.vt369_enhanced_ppu() {
                        if (self.vt369_reg2000(0x1D) & 0x01) == 0 {
                            self.vt369_spr_addr_high = 0;
                        }
                        let idx = (self.ppu_oam_address as usize)
                            | ((self.vt369_spr_addr_high as usize) << 8);
                        self.vt369_write_sprite_byte(idx, val);
                        self.ppu_oam_address = self.ppu_oam_address.wrapping_add(1);
                        if self.ppu_oam_address == 0 {
                            self.vt369_spr_addr_high ^= 1;
                        }
                    } else {
                        if (self.ppu_oam_address & 3) == 2 { val &= 0xE3; }
                        self.oam[self.ppu_oam_address as usize] = val;
                        self.ppu_oam_address = self.ppu_oam_address.wrapping_add(1);
                    }
                } else {
                    self.ppu_oam_address = self.ppu_oam_address.wrapping_add(4) & 0xFC;
                }
            }
            0x2005 => {
                self.ppu_bus = input;
                for i in 0..8 { self.ppu_bus_decay[i] = 1786830; }
                if self.ppu_reset { return; }
                let alignment = self.ppu_clock & 3;
                self.ppu_update_2005_delay = match alignment {
                    2 => 2,
                    _ => 1,
                };
                self.ppu_update_2005_value = input;
                if !self.ppu_addr_latch {
                    self.ppu_fine_x_scroll = input & 7;
                    self.ppu_t = (self.ppu_t & 0b0111111111100000) | ((input as u16) >> 3);
                } else {
                    self.ppu_t = (self.ppu_t & 0b0000110000011111)
                        | (((input as u16 & 0xF8) << 2) | ((input as u16 & 7) << 12));
                }
            }
            0x2006 => {
                self.ppu_bus = input;
                for i in 0..8 { self.ppu_bus_decay[i] = 1786830; }
                if self.ppu_reset { return; }
                if !self.ppu_addr_latch {
                    self.ppu_t = (self.ppu_t & 0b000000011111111) | (((input & 0x3F) as u16) << 8);
                } else {
                    self.ppu_t = (self.ppu_t & 0b0111111100000000) | (input as u16);
                    self.ppu_update_2006_value = self.ppu_t;
                    self.ppu_update_2006_value_temp = self.ppu_v;
                    let alignment = self.ppu_clock & 3;
                    self.ppu_update_2006_delay = match alignment {
                        2 => 5,
                        _ => 4,
                    };
                }
                self.ppu_addr_latch = !self.ppu_addr_latch;
            }
            0x2007 => {
                self.ppu_bus = input;
                self.ppu_2007_write_data = self.ppu_bus;
                for i in 0..8 { self.ppu_bus_decay[i] = 1786830; }
                self.emulate_n_master_clock_cycles(7);
                self.ppu_2007_write = true;
                self.ppu_2007_write_sr = true;
            }
            _ => {}
        }
    }

    // ppu scroll
    fn ppu_increment_scroll_y(&mut self) {
        if (self.ppu_v & 0x7000) != 0x7000 {
            self.ppu_v += 0x1000;
        } else {
            self.ppu_v &= !0x7000;
            let mut y = (self.ppu_v & 0x03E0) >> 5;
            if y == 29 {
                y = 0;
                self.ppu_v ^= 0x0800;
            } else if y == 31 {
                y = 0;
            } else {
                y += 1;
            }
            self.ppu_v = (self.ppu_v & !0x03E0) | (y << 5);
        }
    }

    // ppu scroll x reset
    pub fn ppu_reset_x_scroll(&mut self) {
        self.ppu_v &= 0x7BE0;
        self.ppu_v |= self.ppu_t & 0x041F;
    }

    // ppu scroll y reset
    pub fn ppu_reset_y_scroll(&mut self) {
        self.ppu_v = (self.ppu_v & 0x041F) | (self.ppu_t & 0x7BE0);
    }

    fn decay_ppu_data_bus(&mut self) {
        for i in 0..8 {
            if self.ppu_bus_decay[i] > 0 {
                self.ppu_bus_decay[i] -= 1;
                if self.ppu_bus_decay[i] == 0 {
                    self.ppu_bus &= !(1 << i);
                }
            }
        }
    }

    // VT03 4bpp helpers

    fn vt03_4bpp_bg_enabled(&self) -> bool {
        self.cart.as_ref().map_or(false, |c| c.mapper_chip.vt03_4bpp_bg())
    }

    fn vt03_4bpp_sp_enabled(&self) -> bool {
        self.cart.as_ref().map_or(false, |c| c.mapper_chip.vt03_4bpp_sp())
    }

    fn vt369_enhanced_ppu(&self) -> bool {
        self.cart
            .as_ref()
            .map_or(false, |c| c.mapper_chip.onebus_vt369_enhanced_ppu())
    }

    fn vt369_hires_ppu(&self) -> bool {
        self.vt369_enhanced_ppu() && (self.vt369_reg2000(0x1C) & 0x04) != 0
    }

    fn vt369_ppu(&self) -> bool {
        self.cart
            .as_ref()
            .map_or(false, |c| c.mapper_chip.onebus_vt369_ppu())
    }

    fn vt369_reg2000(&self, idx: usize) -> u8 {
        self.cart
            .as_ref()
            .map_or(0, |c| c.mapper_chip.vt369_reg2000(idx))
    }

    fn vt369_reg4100(&self, idx: usize) -> u8 {
        self.cart
            .as_ref()
            .map_or(0, |c| c.mapper_chip.vt369_reg4100(idx))
    }

    fn vt369_mask_vram_addr_for_nt(&self, v: u16) -> u16 {
        if self.vt369_enhanced_ppu() && (self.vt369_reg4100(0x06) & 2) != 0 {
            v & !0x0C00
        } else {
            v
        }
    }

    fn vt369_read_prg(&self, addr: usize) -> u8 {
        self.cart.as_ref().map_or(0, |c| {
            if c.prg_rom.is_empty() {
                0
            } else {
                c.prg_rom[addr % c.prg_rom.len()]
            }
        })
    }

    fn vt369_prg_len(&self) -> usize {
        self.cart.as_ref().map_or(0, |c| c.prg_rom.len())
    }

    fn vt369_fill_tile_row(&mut self, dest: usize) {
        let reg2000_1c = self.vt369_reg2000(0x1C);
        let reg2000_1e = self.vt369_reg2000(0x1E);
        let hires = self.vt369_hires_ppu();

        if hires {
            if dest + 8 > 272 { return; }
            let mut pat_addr = self.vt369_pat_addr << 2;
            if reg2000_1e & 0x01 != 0 {
                pat_addr += self.cart.as_ref().map_or(0, |c| c.mapper_chip.vt369_bg_data());
            } else {
                let raw_bank = self.vt369_reg2000(if self.ppu_pattern_select_background { 0x12 } else { 0x16 });
                let shift1 = if reg2000_1c & 0x08 != 0 { 10 } else { 13 };
                let shift2 = (reg2000_1c & 3) as u32;
                pat_addr += self.cart.as_ref().map_or(0, |c| c.mapper_chip.vt369_relative())
                    + ((raw_bank as usize) << shift1 << shift2);
            }
            let prg_len = self.vt369_prg_len();
            if prg_len == 0 { return; }
            pat_addr &= prg_len - 1;
            if (reg2000_1c & 3) == 2 {
                let dest2 = dest * 2;
                for i in 0..16 {
                    self.vt369_tile_data_even[dest2 + i] = self.vt369_read_prg(pat_addr + i);
                }
                for i in 0..16 {
                    self.vt369_tile_data_odd[dest2 + i] = self.vt369_read_prg(pat_addr + 16 + i);
                }
            }
            return;
        }

        if dest + 8 > self.vt369_tile_data.len() { return; }
        let mut pat_addr = self.vt369_pat_addr;
        if reg2000_1e & 0x01 != 0 {
            pat_addr += self.cart.as_ref().map_or(0, |c| c.mapper_chip.vt369_bg_data());
        } else {
            let raw_bank = self.vt369_reg2000(if self.ppu_pattern_select_background { 0x12 } else { 0x16 });
            let shift1 = if reg2000_1c & 0x08 != 0 { 10 } else { 13 };
            let shift2 = (reg2000_1c & 3) as u32;
            pat_addr += self.cart.as_ref().map_or(0, |c| c.mapper_chip.vt369_relative())
                + ((raw_bank as usize) << shift1 << shift2);
        }
        let prg_len = self.vt369_prg_len();
        if prg_len == 0 { return; }
        pat_addr &= prg_len - 1;
        if (reg2000_1c & 3) == 2 {
            for i in 0..8 {
                self.vt369_tile_data[dest + i] = self.vt369_read_prg(pat_addr + i);
            }
        } else {
            let b0 = self.vt369_read_prg(pat_addr);
            let b1 = self.vt369_read_prg(pat_addr + 1);
            let b2 = self.vt369_read_prg(pat_addr + 2);
            let b3 = self.vt369_read_prg(pat_addr + 3);
            self.vt369_tile_data[dest] = b0 & 0x0F;
            self.vt369_tile_data[dest + 1] = b0 >> 4;
            self.vt369_tile_data[dest + 2] = b1 & 0x0F;
            self.vt369_tile_data[dest + 3] = b1 >> 4;
            self.vt369_tile_data[dest + 4] = b2 & 0x0F;
            self.vt369_tile_data[dest + 5] = b2 >> 4;
            self.vt369_tile_data[dest + 6] = b3 & 0x0F;
            self.vt369_tile_data[dest + 7] = b3 >> 4;
        }
    }

    fn vt369_compute_pat_addr(&mut self) {
        let reg2000_1c = self.vt369_reg2000(0x1C);
        let tile = self.vt369_nt_byte as usize;
        let fine_bits = ((self.ppu_v >> 11) & 0x0E) as usize;
        if reg2000_1c & 0x08 != 0 {
            self.vt369_pat_addr = ((tile << 4) | fine_bits) << (reg2000_1c & 3);
        } else {
            let attr = self.ppu_attribute as usize;
            self.vt369_pat_addr =
                ((attr << 12) | (tile << 4) | fine_bits) << (reg2000_1c & 3);
        }
    }

    fn vt369_enhanced_bg_tick(&mut self) {
        let dot = self.ppu_dot;
        if (dot % 8 == 5 && dot <= 253) || dot == 325 || dot == 333 {
            self.vt369_compute_pat_addr();
            let dest = if dot == 325 || dot == 333 {
                dot - 325
            } else {
                dot + 11
            };
            self.vt369_fill_tile_row(dest as usize);
        }
    }

    fn vt369_get_pal_index(&self, tc: u16) -> u32 {
        if self.vt369_enhanced_ppu() {
            let lo = self.palette_ram[((tc << 1) & 0x3FF) as usize] as u16;
            let hi = self.palette_ram[(((tc << 1) | 1) & 0x3FF) as usize] as u16;
            let rgb555 = (hi << 8) | lo;
            let r = (((rgb555 >> 10) & 0x1F) as u32) * 255 / 31;
            let g = (((rgb555 >> 5) & 0x1F) as u32) * 255 / 31;
            let b = ((rgb555 & 0x1F) as u32) * 255 / 31;
            (r << 16) | (g << 8) | b
        } else {
            let tc = tc as u8;
            let tc = if (tc & 0x63) == 0 { 0 } else { tc };
            if (self.vt369_reg2000(0x10) & 0x80) != 0 {
                let hi = self.palette_ram[(tc | 0x80) as usize] as u16;
                let lo = self.palette_ram[(tc | 0x00) as usize] as u16;
                let rgb555 = (hi << 8) | lo;
                let r = (((rgb555 >> 10) & 0x1F) as u32) * 255 / 31;
                let g = (((rgb555 >> 5) & 0x1F) as u32) * 255 / 31;
                let b = ((rgb555 & 0x1F) as u32) * 255 / 31;
                (r << 16) | (g << 8) | b
            } else {
                let pal_idx = (self.palette_ram[(tc & 0x1F) as usize] & 0x3F) as usize;
                NES_PALETTE[pal_idx]
            }
        }
    }

    fn vt369_sprite_byte(&self, idx: usize) -> u8 {
        self.vt369_sprite_ram
            .get(idx)
            .copied()
            .unwrap_or(0)
    }

    fn vt369_write_sprite_byte(&mut self, idx: usize, val: u8) {
        if idx < self.vt369_sprite_ram.len() {
            self.vt369_sprite_ram[idx] = val;
        }
        if idx < self.oam.len() {
            self.oam[idx] = val;
        }
    }

    fn vt369_process_sprites_enhanced(&mut self) {
        self.vt369_sprite_count = 0;
        if !self.vt369_enhanced_ppu() {
            return;
        }
        let reg2000_1c = self.vt369_reg2000(0x1C);
        let reg2000_1d = self.vt369_reg2000(0x1D);
        let reg2000_1e = self.vt369_reg2000(0x1E);
        let reg2000_10 = self.vt369_reg2000(0x10);
        let max_sprite = if reg2000_1d & 0x01 != 0 { 127 } else { 63 };
        let use_new_oam = reg2000_1e & 0x04 != 0;
        let prg_len = self.vt369_prg_len();
        if prg_len == 0 {
            return;
        }

        for sprite_num in (0..=max_sprite).rev() {
            let (sprite_x, sprite_y, mut sprite_tile, palette, flip_x, flip_y, priority) =
                if !use_new_oam {
                    let base = (sprite_num as usize) << 2;
                    if base + 3 >= self.oam.len() {
                        continue;
                    }
                    let oam = &self.oam[base..base + 4];
                    let mut sx = oam[3] as i16;
                    let mut sy = oam[0] as i16;
                    let st = (oam[1] as u16) | (((oam[2] as u16) << 6) & 0x700);
                    let (pal, fx, fy, pri) = if reg2000_1d & 0x08 != 0 {
                        let neg_x = (oam[2] & 0x40) != 0;
                        let neg_y = (oam[2] & 0x80) != 0;
                        if neg_x { sx -= 256; }
                        if neg_y { sy -= 256; }
                        let p = (oam[2] << 4 & 0x30) | (oam[2] << 1 & 0x40);
                        (p, false, false, false)
                    } else {
                        let p = oam[2] << 4 & 0x30;
                        let fx = (oam[2] & 0x40) != 0;
                        let fy = (oam[2] & 0x80) != 0;
                        let pri = (oam[2] & 0x20) != 0;
                        (p, fx, fy, pri)
                    };
                    (sx, sy, st, pal, fx, fy, pri)
                } else {
                    let sn = sprite_num as usize;
                    if sn >= 128 {
                        continue;
                    }
                    let mut sy = self.vt369_sprite_byte(sn) as i16;
                    let mut sx = self.vt369_sprite_byte(0x180 | sn) as i16;
                    let st = (self.vt369_sprite_byte(0x080 | sn) as u16)
                        | (((self.vt369_sprite_byte(0x100 | sn) as u16) << 6) & 0x700);
                    let attr = self.vt369_sprite_byte(0x100 | sn);
                    let (pal, fx, fy, pri) = if reg2000_1d & 0x08 != 0 {
                        let neg_x = (attr & 0x40) != 0;
                        let neg_y = (attr & 0x80) != 0;
                        if neg_x { sx -= 256; }
                        if neg_y { sy -= 256; }
                        let p = (attr << 4 & 0x30) | (attr << 1 & 0x40);
                        (p, false, false, false)
                    } else {
                        let p = attr << 4 & 0x30;
                        let fx = (attr & 0x40) != 0;
                        let fy = (attr & 0x80) != 0;
                        let pri = (attr & 0x20) != 0;
                        (p, fx, fy, pri)
                    };
                    (sx, sy, st, pal, fx, fy, pri)
                };

            if reg2000_1c & 0x80 != 0 {
                sprite_tile &= 0xFF;
            }
            if reg2000_1e == 0x0F && reg2000_1d == 0x0B && (reg2000_10 & 0x10) != 0 {
                sprite_tile = (sprite_tile & 0x3F) | ((sprite_tile >> 2) & !0x3F);
            }

            let right_table = if reg2000_1e & 0x01 == 0 {
                if self.ppu_sprite_x16 {
                    (sprite_tile & 1) != 0
                } else {
                    self.ppu_pattern_select_sprites
                }
            } else {
                false
            };

            if self.ppu_sprite_x16 {
                sprite_tile &= !1;
            }

            let sprite_height = if reg2000_1d & 0x04 != 0 || self.ppu_sprite_x16 {
                15
            } else {
                7
            };
            let cur_sl = if self.ppu_scanline == self.pre_render_scanline() {
                -1
            } else {
                self.ppu_scanline as i16
            };
            let mut sprite_sl = cur_sl - sprite_y;
            if sprite_sl < 0 || sprite_sl > sprite_height {
                continue;
            }
            if flip_y {
                sprite_sl = sprite_height - sprite_sl;
            }

            let mut pattern_addr = (sprite_tile as usize) << 6;
            if reg2000_1d & 0x04 != 0 {
                pattern_addr <<= 1;
            }
            pattern_addr += (sprite_sl as usize) << 3;
            if reg2000_1d & 0x02 == 0 && reg2000_1d & 0x04 == 0 && reg2000_1c & 0x20 == 0 {
                pattern_addr >>= 1;
            }

            if reg2000_1e & 0x01 != 0 {
                pattern_addr += self
                    .cart
                    .as_ref()
                    .map_or(0, |c| c.mapper_chip.vt369_spr_data());
            } else {
                let bank_reg = if right_table {
                    self.vt369_reg2000(0x12)
                } else {
                    self.vt369_reg2000(0x16)
                };
                let shift1 = if reg2000_1c & 0x80 != 0 { 10 } else { 13 };
                let shift2 = (reg2000_1c >> 4) & 3;
                pattern_addr += self.cart.as_ref().map_or(0, |c| c.mapper_chip.vt369_relative())
                    + ((bank_reg as usize) << shift1 << shift2);
            }
            pattern_addr &= prg_len - 1;

            let count = self.vt369_sprite_count as usize;
            if count >= self.vt369_sprite_buf.len() {
                break;
            }
            let mut entry = Vt369SpriteEntry {
                start_x: sprite_x,
                priority,
                sprite0: sprite_num == 0,
                ..Default::default()
            };

            if reg2000_1d & 0x02 != 0 {
                if flip_x {
                    entry.data[15] = palette | (self.vt369_read_prg(pattern_addr + 0) & 0x0F);
                    entry.data[14] = palette | (self.vt369_read_prg(pattern_addr + 0) >> 4);
                    entry.data[13] = palette | (self.vt369_read_prg(pattern_addr + 1) & 0x0F);
                    entry.data[12] = palette | (self.vt369_read_prg(pattern_addr + 1) >> 4);
                    entry.data[11] = palette | (self.vt369_read_prg(pattern_addr + 2) & 0x0F);
                    entry.data[10] = palette | (self.vt369_read_prg(pattern_addr + 2) >> 4);
                    entry.data[9] = palette | (self.vt369_read_prg(pattern_addr + 3) & 0x0F);
                    entry.data[8] = palette | (self.vt369_read_prg(pattern_addr + 3) >> 4);
                    entry.data[7] = palette | (self.vt369_read_prg(pattern_addr + 4) & 0x0F);
                    entry.data[6] = palette | (self.vt369_read_prg(pattern_addr + 4) >> 4);
                    entry.data[5] = palette | (self.vt369_read_prg(pattern_addr + 5) & 0x0F);
                    entry.data[4] = palette | (self.vt369_read_prg(pattern_addr + 5) >> 4);
                    entry.data[3] = palette | (self.vt369_read_prg(pattern_addr + 6) & 0x0F);
                    entry.data[2] = palette | (self.vt369_read_prg(pattern_addr + 6) >> 4);
                    entry.data[1] = palette | (self.vt369_read_prg(pattern_addr + 7) & 0x0F);
                    entry.data[0] = palette | (self.vt369_read_prg(pattern_addr + 7) >> 4);
                } else {
                    entry.data[0] = palette | (self.vt369_read_prg(pattern_addr + 0) & 0x0F);
                    entry.data[1] = palette | (self.vt369_read_prg(pattern_addr + 0) >> 4);
                    entry.data[2] = palette | (self.vt369_read_prg(pattern_addr + 1) & 0x0F);
                    entry.data[3] = palette | (self.vt369_read_prg(pattern_addr + 1) >> 4);
                    entry.data[4] = palette | (self.vt369_read_prg(pattern_addr + 2) & 0x0F);
                    entry.data[5] = palette | (self.vt369_read_prg(pattern_addr + 2) >> 4);
                    entry.data[6] = palette | (self.vt369_read_prg(pattern_addr + 3) & 0x0F);
                    entry.data[7] = palette | (self.vt369_read_prg(pattern_addr + 3) >> 4);
                    entry.data[8] = palette | (self.vt369_read_prg(pattern_addr + 4) & 0x0F);
                    entry.data[9] = palette | (self.vt369_read_prg(pattern_addr + 4) >> 4);
                    entry.data[10] = palette | (self.vt369_read_prg(pattern_addr + 5) & 0x0F);
                    entry.data[11] = palette | (self.vt369_read_prg(pattern_addr + 5) >> 4);
                    entry.data[12] = palette | (self.vt369_read_prg(pattern_addr + 6) & 0x0F);
                    entry.data[13] = palette | (self.vt369_read_prg(pattern_addr + 6) >> 4);
                    entry.data[14] = palette | (self.vt369_read_prg(pattern_addr + 7) & 0x0F);
                    entry.data[15] = palette | (self.vt369_read_prg(pattern_addr + 7) >> 4);
                }
                for d in &mut entry.data {
                    if *d == palette {
                        *d = 0;
                    }
                }
            } else if reg2000_1d & 0x02 == 0
                && reg2000_1d & 0x04 == 0
                && reg2000_1c & 0x20 == 0
            {
                if flip_x {
                    entry.data[7] = palette | (self.vt369_read_prg(pattern_addr) & 0x0F);
                    entry.data[6] = palette | (self.vt369_read_prg(pattern_addr) >> 4);
                    entry.data[5] = palette | (self.vt369_read_prg(pattern_addr + 1) & 0x0F);
                    entry.data[4] = palette | (self.vt369_read_prg(pattern_addr + 1) >> 4);
                    entry.data[3] = palette | (self.vt369_read_prg(pattern_addr + 2) & 0x0F);
                    entry.data[2] = palette | (self.vt369_read_prg(pattern_addr + 2) >> 4);
                    entry.data[1] = palette | (self.vt369_read_prg(pattern_addr + 3) & 0x0F);
                    entry.data[0] = palette | (self.vt369_read_prg(pattern_addr + 3) >> 4);
                } else {
                    entry.data[0] = palette | (self.vt369_read_prg(pattern_addr) & 0x0F);
                    entry.data[1] = palette | (self.vt369_read_prg(pattern_addr) >> 4);
                    entry.data[2] = palette | (self.vt369_read_prg(pattern_addr + 1) & 0x0F);
                    entry.data[3] = palette | (self.vt369_read_prg(pattern_addr + 1) >> 4);
                    entry.data[4] = palette | (self.vt369_read_prg(pattern_addr + 2) & 0x0F);
                    entry.data[5] = palette | (self.vt369_read_prg(pattern_addr + 2) >> 4);
                    entry.data[6] = palette | (self.vt369_read_prg(pattern_addr + 3) & 0x0F);
                    entry.data[7] = palette | (self.vt369_read_prg(pattern_addr + 3) >> 4);
                }
                for d in &mut entry.data[..8] {
                    if *d == palette {
                        *d = 0;
                    }
                }
            } else {
                if flip_x {
                    for i in 0..8 {
                        entry.data[i] = self.vt369_read_prg(pattern_addr + (7 - i));
                    }
                } else {
                    for i in 0..8 {
                        entry.data[i] = self.vt369_read_prg(pattern_addr + i);
                    }
                }
            }

            self.vt369_sprite_buf[count] = entry;
            self.vt369_sprite_count += 1;
        }
    }

    fn vt369_process_sprites_hires(&mut self) {
        self.vt369_sprite_count_hi = 0;
        if !self.vt369_hires_ppu() {
            return;
        }
        let reg2000_1d = self.vt369_reg2000(0x1D);
        let max_sprite = if reg2000_1d & 0x01 != 0 { 127 } else { 63 };
        let prg_len = self.vt369_prg_len();
        if prg_len == 0 {
            return;
        }

        for sprite_num in (0..=max_sprite).rev() {
            let sn = sprite_num as usize;
            let (sprite_x, sprite_y, sprite_tile, palette, flip_x, flip_y, bit8pp, size_x, size_y) = {
                    let base = sn << 3;
                    if base + 7 >= self.oam.len() { continue; }
                    let oam = &self.oam[base..base + 8];
                    let oam_y = oam[0] as i16 - 1;
                    let oam_x = oam[7] as i16;
                    let oam_tile = (oam[1] as u16) | ((oam[2] as u16) << 8);
                    let oam_cfg = oam[3];
                    let oam_attr = oam[6];
                    let size_x = 4_usize << ((oam_cfg >> 4) & 3);
                    let size_y = 4_usize << ((oam_cfg >> 2) & 3);
                    let bit8pp = (oam_cfg & 0x80) != 0;
                    let pal = (oam_attr << 4) & 0xF0;
                    let fx = (oam_attr & 0x40) != 0;
                    let fy = (oam_attr & 0x80) != 0;
                    (oam_x, oam_y, oam_tile as usize, pal, fx, fy, bit8pp, size_x, size_y)
                };

            let mut sprite_sl = self.ppu_scanline as i16 - sprite_y;
            if sprite_sl < 0 || sprite_sl > (size_y as i16 - 1) {
                continue;
            }
            if flip_y {
                sprite_sl = (size_y as i16 - 1) - sprite_sl;
            }

            let count = self.vt369_sprite_count_hi as usize;
            if count >= self.vt369_sprite_buf_hi.len() {
                break;
            }
            let mut entry = Vt369SpriteHiEntry {
                start_x: sprite_x,
                mask_x: !(size_x as i16 - 1),
                sprite0: sprite_num == 0,
                ..Default::default()
            };

            if bit8pp {
                let mut pattern_addr = sprite_tile * size_x * size_y * 2 * 2
                    + (sprite_sl as usize) * size_x * 2 * 2;
                pattern_addr += self.cart.as_ref().map_or(0, |c| c.mapper_chip.vt369_spr_data());
                pattern_addr &= prg_len - 1;
                for i in 0..size_x * 2 {
                    entry.data_even[i] = self.vt369_read_prg(pattern_addr + i);
                }
                pattern_addr += size_x * 2;
                for i in 0..size_x * 2 {
                    entry.data_odd[i] = self.vt369_read_prg(pattern_addr + i);
                }
            } else {
                let mut pattern_addr = sprite_tile * size_x * size_y * 2
                    + (sprite_sl as usize) * size_x * 2;
                pattern_addr += self.cart.as_ref().map_or(0, |c| c.mapper_chip.vt369_spr_data());
                pattern_addr &= prg_len - 1;
                for i in 0..size_x {
                    let b = self.vt369_read_prg(pattern_addr + i);
                    entry.data_even[i * 2] = palette | (b & 0x0F);
                    entry.data_even[i * 2 + 1] = palette | (b >> 4);
                }
                pattern_addr += size_x;
                for i in 0..size_x {
                    let b = self.vt369_read_prg(pattern_addr + i);
                    entry.data_odd[i * 2] = palette | (b & 0x0F);
                    entry.data_odd[i * 2 + 1] = palette | (b >> 4);
                }
                for i in 0..size_x * 2 {
                    if entry.data_even[i] == palette { entry.data_even[i] = 0; }
                    if entry.data_odd[i] == palette { entry.data_odd[i] = 0; }
                }
            }
            if flip_x {
                for i in 0..size_x {
                    entry.data_even.swap(i, size_x * 2 - 1 - i);
                    entry.data_odd.swap(i, size_x * 2 - 1 - i);
                }
            }
            if flip_y {
                for i in 0..size_x * 2 {
                    let tmp = entry.data_even[i];
                    entry.data_even[i] = entry.data_odd[i];
                    entry.data_odd[i] = tmp;
                }
            }

            self.vt369_sprite_buf_hi[count] = entry;
            self.vt369_sprite_count_hi += 1;
        }
    }

    // background tile fetching

    pub(crate) fn ppu_render_bg_fetches(&mut self) {
        let cycle_tick = (self.ppu_dot.wrapping_add(7)) & 7;


        match cycle_tick {
            0 => {
                let masked_v = self.vt369_mask_vram_addr_for_nt(self.ppu_v);
                self.ppu_pattern_address_register_nt = 0x2000 | (masked_v & 0x0FFF);
                self.ppu_address_bus = self.ppu_pattern_address_register_nt;
            }
            1 => {
                self.ppu_address_bus = (self.ppu_pattern_address_register_nt & 0xFF00) | self.ppu_octal_latch as u16;
                self.ppu_render_temp = self.fetch_ppu();
                self.ppu_commit_nametable_fetch = true;
            }
            2 => {
                self.ppu_pattern_address_register_at = 0x23C0 | (self.ppu_v & 0x0C00) | ((self.ppu_v >> 4) & 0x38) | ((self.ppu_v >> 2) & 0x07);
                self.ppu_address_bus = self.ppu_pattern_address_register_at;
            }
            3 => {
                self.ppu_address_bus = (self.ppu_pattern_address_register_at & 0xFF00) | self.ppu_octal_latch as u16;
                self.ppu_render_temp = self.fetch_ppu();
                self.ppu_commit_attribute_fetch = true;
            }
            4 => {
                self.ppu_check_par();
                self.ppu_pattern_address_register_chr &= 0b1111111110111;
                self.ppu_address_bus = self.ppu_pattern_address_register_chr;
            }
            5 => {
                let base_addr = (self.ppu_pattern_address_register_chr & 0xFF00) | self.ppu_octal_latch as u16;
                self.ppu_address_bus = base_addr;
                self.onebus_begin_bg_chr_fetch();
                self.ppu_render_temp = self.fetch_ppu();
                self.onebus_clear_chr_fetch();
                self.ppu_commit_pattern_low_fetch = true;
                if self.vt03_4bpp_bg_enabled() {
                    self.ppu_address_bus = base_addr | 0x4000;
                    self.onebus_begin_bg_chr_fetch();
                    self.ppu_low_bit_plane_hi = self.fetch_ppu();
                    self.onebus_clear_chr_fetch();
                }
            }
            6 => {
                self.ppu_check_par();
                self.ppu_pattern_address_register_chr |= 8;
                self.ppu_address_bus = self.ppu_pattern_address_register_chr;
            }
            7 => {
                let base_addr = (self.ppu_pattern_address_register_chr & 0xFF00) | self.ppu_octal_latch as u16;
                self.ppu_address_bus = base_addr;
                self.onebus_begin_bg_chr_fetch();
                self.ppu_render_temp = self.fetch_ppu();
                self.onebus_clear_chr_fetch();
                self.ppu_commit_pattern_high_fetch = true;
                if self.vt03_4bpp_bg_enabled() {
                    self.ppu_address_bus = base_addr | 0x4000;
                    self.onebus_begin_bg_chr_fetch();
                    self.ppu_high_bit_plane_hi = self.fetch_ppu();
                    self.onebus_clear_chr_fetch();
                }
            }
            _ => {}
        }

        if self.vt369_enhanced_ppu() {
            self.vt369_enhanced_bg_tick();
        }

        if self.ppu_ale && !self.ppu_read {
            self.ppu_octal_latch = self.ppu_address_bus as u8;
        }
    }

    fn ppu_render_bg_dummy_nt(&mut self) {
        if self.ppu_read {
            self.ppu_octal_latch = self.ppu_address_bus as u8;
        }

        if self.ppu_dot == 0 {
            self.ppu_check_par();
            self.ppu_pattern_address_register_chr &= 0b1111111110111;
            if self.ppu_scanline != self.pre_render_scanline() {
                self.ppu_address_bus = self.ppu_pattern_address_register_chr;
            }
        } else {
            let cycle_tick = self.ppu_dot.wrapping_sub(337);
            match cycle_tick {
                0 => {
                    self.ppu_address_bus = 0x2000 + (self.ppu_v & 0x0FFF);
                }
                1 => {
                    self.ppu_address_bus = 0x2000 + (self.ppu_v & 0x0FFF);
                    self.ppu_render_temp = self.fetch_ppu();
                    self.ppu_commit_nametable_fetch = true;
                }
                2 => {
                    self.ppu_address_bus = 0x2000 + (self.ppu_v & 0x0FFF);
                }
                3 => {
                    self.ppu_render_temp = self.fetch_ppu();
                }
                _ => {}
            }
        }

        if self.ppu_ale && !self.ppu_read {
            self.ppu_octal_latch = self.ppu_address_bus as u8;
        }
    }

    pub fn ppu_render_commit_shift_registers_and_bit_planes(&mut self) {
        if self.ppu_commit_nametable_fetch {
            self.ppu_commit_nametable_fetch = false;
            self.ppu_pattern_address_register_chr &= 0b1000000001111;
            if self.ppu_dot < 256 || self.ppu_dot > 320 {
                self.ppu_pattern_address_register_chr |= (self.ppu_address_bus & 0xFF) << 4;
                if self.vt369_enhanced_ppu() {
                    self.vt369_nt_byte = (self.ppu_address_bus & 0xFF) as u8;
                }
            } else {
                let idx = ((self.oam2_address & 0x1C) + 1) as usize;
                self.ppu_pattern_address_register_chr |= (self.oam2[idx] as u16) << 4;
            }
        }
        if self.ppu_commit_attribute_fetch {
            self.ppu_commit_attribute_fetch = false;
            self.ppu_attribute = self.ppu_render_temp;
            if (self.ppu_v & 3) >= 2 {
                self.ppu_attribute >>= 2;
            }
            if (((self.ppu_v & 0b0000001111100000) >> 5) & 3) >= 2 {
                self.ppu_attribute >>= 4;
            }
            self.ppu_attribute &= 3;
            if self.onebus_chr_routing_ppu() {
                let reg2000_10 = self
                    .cart
                    .as_ref()
                    .map_or(0, |c| c.mapper_chip.vt03_reg2000_10());
                self.ppu_onebus_eva = if (reg2000_10 & 0x10) != 0 {
                    (self.ppu_attribute as u16) << 10
                } else {
                    0
                };
            }
        }
        if self.ppu_commit_pattern_low_fetch {
            self.ppu_commit_pattern_low_fetch = false;
            self.ppu_low_bit_plane = self.ppu_render_temp;
        }
        if self.ppu_commit_pattern_high_fetch {
            self.ppu_commit_pattern_high_fetch = false;
            self.ppu_high_bit_plane = self.ppu_render_temp;
            self.ppu_load_shift_registers();
            self.ppu_increment_scroll_x();
        }
    }

    fn ppu_update_bg_shift_registers(&mut self) {
        self.ppu_bg_pattern_sr_l <<= 1;
        self.ppu_bg_pattern_sr_h = (self.ppu_bg_pattern_sr_h << 1) | 1;
        if self.vt03_4bpp_bg_enabled() {
            self.ppu_bg_pattern_sr_l2 <<= 1;
            self.ppu_bg_pattern_sr_h2 = (self.ppu_bg_pattern_sr_h2 << 1) | 1;
        }
        self.ppu_bg_attr_sr_l = (self.ppu_bg_attr_sr_l << 1) | (self.ppu_attr_latch_register & 1) as u16;
        self.ppu_bg_attr_sr_h = (self.ppu_bg_attr_sr_h << 1) | ((self.ppu_attr_latch_register & 2) >> 1) as u16;
    }

    fn ppu_load_shift_registers(&mut self) {
        self.ppu_bg_pattern_sr_l =
            (self.ppu_bg_pattern_sr_l & 0xFF00) | self.ppu_low_bit_plane as u16;
        self.ppu_bg_pattern_sr_h =
            (self.ppu_bg_pattern_sr_h & 0xFF00) | self.ppu_high_bit_plane as u16;
        if self.vt03_4bpp_bg_enabled() {
            self.ppu_bg_pattern_sr_l2 =
                (self.ppu_bg_pattern_sr_l2 & 0xFF00) | self.ppu_low_bit_plane_hi as u16;
            self.ppu_bg_pattern_sr_h2 =
                (self.ppu_bg_pattern_sr_h2 & 0xFF00) | self.ppu_high_bit_plane_hi as u16;
        }
        self.ppu_attr_latch_register = self.ppu_attribute;
    }

    fn ppu_increment_scroll_x(&mut self) {
        if (self.ppu_v & 0x001F) == 31 {
            self.ppu_v &= 0xFFE0;
            self.ppu_v ^= 0x0400;
        } else {
            self.ppu_v += 1;
        }
    }

    pub(crate) fn ppu_check_par(&mut self) {
        if self.ppu_dot < 256 || self.ppu_dot > 320 {
            self.ppu_pattern_address_register_chr &= 0b0111111111000;
            self.ppu_pattern_address_register_chr |= if self.ppu_pattern_select_background { 0b1000000000000 } else { 0 };
            self.ppu_pattern_address_register_chr |= (self.ppu_v & 0b0111000000000000) >> 12;
        } else {
            if !self.ppu_sprite_x16 {
                let flipy = (self.oam2[((self.oam2_address & 0x1C) + 2) as usize] & 0x80) != 0;
                self.ppu_pattern_address_register_chr &= 0b0111111111000;
                self.ppu_pattern_address_register_chr |= if self.ppu_pattern_select_sprites { 0b1000000000000 } else { 0 };
                self.ppu_pattern_address_register_chr |= if flipy { 7 - (self.in_range_check & 0x7) } else { self.in_range_check & 0x7 };
            } else {
                let flipy = (self.oam2[((self.oam2_address & 0x1C) + 2) as usize] & 0x80) != 0;
                self.ppu_pattern_address_register_chr &= 0b0111111101000;
                self.ppu_pattern_address_register_chr |= if (self.oam2[((self.oam2_address & 0x1C) + 1) as usize] & 1) != 0 { 0b1000000000000 } else { 0 };
                self.ppu_pattern_address_register_chr |= if flipy { 7 - (self.in_range_check & 0x7) } else { self.in_range_check & 0x7 };
                self.ppu_pattern_address_register_chr |= ((self.in_range_check & 0x08) ^ if flipy { 8 } else { 0 }) << 1;
            }
        }
    }

    pub fn flip_byte(b: u8) -> u8 {
        let mut b = b;
        b = ((b & 0xF0) >> 4) | ((b & 0x0F) << 4);
        b = ((b & 0xCC) >> 2) | ((b & 0x33) << 2);
        b = ((b & 0xAA) >> 1) | ((b & 0x55) << 1);
        b
    }

    fn ppu_render_calculate_pixel_vt369_enhanced(&mut self) {
        let hires = self.vt369_hires_ppu();
        let mut displayed_tc: u16 = 0;
        let show_bg = self.ppu_mask_show_background
            && (self.ppu_dot > 8 || self.ppu_mask_8px_show_background);
        let show_sp = self.ppu_mask_show_sprites
            && (self.ppu_dot > 8 || self.ppu_mask_8px_show_sprites);
        let dot = self.ppu_dot.saturating_sub(1);

        if show_bg {
            if hires {
                let idx = dot as usize + self.ppu_fine_x_scroll as usize;
                let idx2 = idx * 2;
                if idx2 + 1 < self.vt369_tile_data_even.len() {
                    displayed_tc = self.vt369_tile_data_even[idx2] as u16;
                }
            } else {
                let idx = dot as usize + self.ppu_fine_x_scroll as usize;
                if idx < self.vt369_tile_data.len() {
                    displayed_tc = self.vt369_tile_data[idx] as u16;
                }
            }
        }

        if show_sp {
            if hires {
                for i in 0..self.vt369_sprite_count_hi as usize {
                    let sprite = &self.vt369_sprite_buf_hi[i];
                    let sprite_pixel = dot as i16 - sprite.start_x;
                    if (sprite_pixel & sprite.mask_x) != 0 {
                        continue;
                    }
                    let idx = sprite_pixel as usize * 2;
                    if idx + 1 >= sprite.data_even.len() {
                        continue;
                    }
                    let sp_data = sprite.data_even[idx];
                    if sp_data == 0 {
                        continue;
                    }
                    if sprite.sprite0 && dot < 255 {
                        self.ppu_status_sprite_zero_hit = true;
                    }
                    if !(displayed_tc != 0 && false) {
                        displayed_tc = sp_data as u16 | 0x100;
                    }
                }
            } else {
                let sp_mask = if (self.vt369_reg2000(0x1D) & 0x02) != 0 {
                    15
                } else {
                    7
                };
                for i in 0..self.vt369_sprite_count as usize {
                    let sprite = &self.vt369_sprite_buf[i];
                    let sprite_pixel = dot as i16 - sprite.start_x;
                    if sprite_pixel < 0 || sprite_pixel > sp_mask {
                        continue;
                    }
                    let sp_data = sprite.data[sprite_pixel as usize];
                    if sp_data == 0 {
                        continue;
                    }
                    if sprite.sprite0 && dot < 255 {
                        self.ppu_status_sprite_zero_hit = true;
                    }
                    if !(displayed_tc != 0 && sprite.priority) {
                        displayed_tc = sp_data as u16 | 0x100;
                    }
                }
            }
        }

        if (self.ppu_mask_show_background || self.ppu_mask_show_sprites)
            && self.ppu_scanline < 240
        {
            let rgb = self.vt369_get_pal_index(displayed_tc);
            self.dot_color_rgb = rgb;
            self.palette_ram_address = (displayed_tc & 0xFF) as u8;
        } else if (self.ppu_v & 0x3F1F) >= 0x3F00 {
            self.palette_ram_address = (self.ppu_v & 0x1F) as u8;
            if (self.palette_ram_address & 3) == 0 {
                self.palette_ram_address &= 0x0F;
            }
            let pal_idx = (self.palette_ram[self.palette_ram_address as usize & 0x1F] & 0x3F) as usize;
            self.dot_color_rgb = NES_PALETTE[pal_idx];
        } else {
            self.palette_ram_address = 0;
            self.dot_color_rgb = NES_PALETTE[0];
        }
    }

    fn ppu_render_calculate_pixel(&mut self) {
        if self.vt369_enhanced_ppu() {
            self.ppu_render_calculate_pixel_vt369_enhanced();
            return;
        }

        let mut palette: u8 = 0;
        let mut color: u8 = 0;

        let bg_color_mask: u8 = if self.vt03_4bpp_bg_enabled() { 0x0F } else { 0x03 };
        let sp_color_mask: u8 = if self.vt03_4bpp_sp_enabled() { 0x0F } else { 0x03 };

        if self.ppu_mask_show_background
            && (self.ppu_dot > 8 || self.ppu_mask_8px_show_background)
        {
            let col0 = ((self.ppu_bg_pattern_sr_l >> (15 - self.ppu_fine_x_scroll as u16)) & 1) as u8;
            let col1 = ((self.ppu_bg_pattern_sr_h >> (15 - self.ppu_fine_x_scroll as u16)) & 1) as u8;
            color = (col1 << 1) | col0;
            if self.vt03_4bpp_bg_enabled() {
                let col2 = ((self.ppu_bg_pattern_sr_l2 >> (15 - self.ppu_fine_x_scroll as u16)) & 1) as u8;
                let col3 = ((self.ppu_bg_pattern_sr_h2 >> (15 - self.ppu_fine_x_scroll as u16)) & 1) as u8;
                color |= (col2 << 2) | (col3 << 3);
            }

            let pal0 = ((self.ppu_bg_attr_sr_l >> (7 - self.ppu_fine_x_scroll as u16)) & 1) as u8;
            let pal1 = ((self.ppu_bg_attr_sr_h >> (7 - self.ppu_fine_x_scroll as u16)) & 1) as u8;
            palette = (pal1 << 1) | pal0;

            if (color & bg_color_mask) == 0 && palette != 0 {
                palette = 0;
            }
        }

        let mut sprite_color: u8 = 0;
        let mut sprite_palette: u8 = 0;
        let mut sprite_priority = false;
        let mut sprite_index: usize = 8;

        if self.ppu_mask_show_sprites
            && (self.ppu_dot > 8 || self.ppu_mask_8px_show_sprites)
        {
            for i in 0..8 {
                if self.ppu_sprite_shifter_counter[i] == 0 || self.skipped_pre_render_dot_341 {
                    let sl = (self.ppu_sprite_sr_l[i] & 0x80) != 0;
                    let sh = (self.ppu_sprite_sr_h[i] & 0x80) != 0;
                    let mut sc = (if sh { 2u8 } else { 0 }) | (if sl { 1u8 } else { 0 });
                    if self.vt03_4bpp_sp_enabled() {
                        let sl2 = (self.ppu_sprite_sr_l2[i] & 0x80) != 0;
                        let sh2 = (self.ppu_sprite_sr_h2[i] & 0x80) != 0;
                        sc |= (if sl2 { 4u8 } else { 0 }) | (if sh2 { 8u8 } else { 0 });
                    }
                    sprite_color = sc;
                    sprite_palette = (self.ppu_sprite_attribute[i] & 0x03) | 0x04;
                    sprite_priority = (self.ppu_sprite_attribute[i] >> 5) & 1 == 0;
                    sprite_index = i;
                } else {
                    continue;
                }

                if (sprite_color & sp_color_mask) != 0 {
                    break;
                }
            }

            if self.ppu_can_detect_sprite_zero_hit
                && sprite_index == 0
                && self.ppu_current_scanline_contains_sprite_zero
                && self.ppu_mask_show_background
                && self.ppu_mask_show_sprites
                && (color & bg_color_mask) != 0
                && (sprite_color & sp_color_mask) != 0
                && (self.ppu_mask_8px_show_sprites || self.ppu_dot > 8)
                && self.ppu_dot < 256
            {
                if self.ppu_dot == 256 {
                } else {
                    self.ppu_status_pending_sprite_zero_hit = true;
                    self.ppu_can_detect_sprite_zero_hit = false;
                }
            }

            if (color & bg_color_mask) == 0 && (sprite_color & sp_color_mask) != 0 {
                color = sprite_color;
                palette = sprite_palette;
            } else if (sprite_color & sp_color_mask) != 0 && sprite_priority {
                color = sprite_color;
                palette = sprite_palette;
            }
        }

        let reg2000_10 = self.cart.as_ref().map_or(0, |c| c.mapper_chip.vt03_reg2000_10());
        let is_vt03 = self.vt03_4bpp_bg_enabled() || self.vt03_4bpp_sp_enabled() || (reg2000_10 & 0x80) != 0;

        if (self.ppu_mask_show_background || self.ppu_mask_show_sprites)
            && self.ppu_scanline < 240
        {
            if is_vt03 {
                let bkexten = (reg2000_10 & 0x10) != 0;
                let is_sprite = (palette & 4) != 0;
                let c0 = color & 1;
                let c1 = (color >> 1) & 1;
                let c2 = (color >> 2) & 1;
                let c3 = (color >> 3) & 1;
                let attr_bits = if !is_sprite && bkexten { 0 } else { palette & 3 };
                let sprite_flag = if is_sprite { 0x10 } else { 0 };
                let tc = (c3 << 6) | (c2 << 5) | sprite_flag | (attr_bits << 2) | (c1 << 1) | c0;
                self.palette_ram_address = if (tc & 0x63) == 0 { 0 } else { tc };
            } else {
                self.palette_ram_address = (palette << 2) | color;
            }
        } else {
            if (self.ppu_v & 0x3F1F) >= 0x3F00 {
                self.palette_ram_address = (self.ppu_v & 0x1F) as u8;
                if (self.palette_ram_address & 3) == 0 {
                    self.palette_ram_address &= 0x0F;
                }
            } else {
                self.palette_ram_address = 0;
            }
        }

        if self.ppu_palette_corruption_rendering_disabled_out_of_vblank || self.ppu_v_register_changed_out_of_vblank {
            self.ppu_v_register_changed_out_of_vblank = false;
            self.ppu_palette_corruption_rendering_disabled_out_of_vblank = false;
            self.corrupt_palettes(color, palette);
        }

        self.dot_color = self.palette_ram[self.palette_ram_address as usize & 0x1F] & 0x3F;

        let is_vt369 = self.vt369_ppu();
        let is_vt03 = self.vt03_4bpp_bg_enabled() || self.vt03_4bpp_sp_enabled() || (reg2000_10 & 0x80) != 0;

        if is_vt369 {
            let tc = self.palette_ram_address as u16;
            self.dot_color_rgb = self.vt369_get_pal_index(tc);
        } else if is_vt03 {
            let tc = self.palette_ram_address as usize & 0x7F;
            if (reg2000_10 & 0x80) != 0 {
                let tc_masked = if (tc & 0x63) == 0 { 0 } else { tc };
                let hi = self.palette_ram[tc_masked | 0x80] as usize;
                let lo = self.palette_ram[tc_masked | 0x00] as usize;
                let vt_color = ((hi << 6) + lo) & 0xFFF;
                self.dot_color_rgb = crate::vt03_palette::get_vt03_palette()[vt_color];
            } else {
                let pal_idx = (self.palette_ram[tc] & 0x3F) as usize;
                self.dot_color_rgb = NES_PALETTE[pal_idx];
            }
        } else {
            let pal_idx = (self.palette_ram[self.palette_ram_address as usize & 0x1F] & 0x3F) as usize;
            self.dot_color_rgb = NES_PALETTE[pal_idx];
        }
    }

    fn draw_to_screen(&mut self) {
        if self.ppu_dot > 3 && self.ppu_dot <= 259 && self.ppu_scanline < 241 {
            let mut chosen_color = self.prev_prev_prev_dot_color as usize;
            if self.ppu_mask_greyscale {
                chosen_color &= 0x30;
            }
            let mut emphasis: usize = 0;
            if self.ppu_mask_emphasize_red { emphasis |= 0x40; }
            if self.ppu_mask_emphasize_green { emphasis |= 0x20; }
            if self.ppu_mask_emphasize_blue { emphasis |= 0x100; }

            let mut odd_offset: usize = 0;
            if !self.is_pal() && !self.is_dendy() && self.ppu_scanline == 0 && self.ppu_odd_frame
                && (self.ppu_mask_show_background || self.ppu_mask_show_sprites)
            {
                odd_offset = 1;
            }

            if odd_offset == 1 && self.ppu_dot == 4 {
            } else {
                let x = (self.ppu_dot as usize) - 4 - odd_offset;
                let y = self.ppu_scanline as usize;
                if x < 256 && y < 240 {
                    let reg2000_10 = self.cart.as_ref().map_or(0, |c| c.mapper_chip.vt03_reg2000_10());
                    let is_vt369 = self.vt369_ppu();
                    let is_vt03 = self.vt03_4bpp_bg_enabled() || self.vt03_4bpp_sp_enabled() || (reg2000_10 & 0x80) != 0;
                    if is_vt369 || is_vt03 || self.vt369_enhanced_ppu() {
                        self.screen[y * 256 + x] = self.prev_prev_prev_dot_color_rgb;
                    } else {
                        let pal_idx = (chosen_color | emphasis) % NES_PALETTE.len();
                        let is_vs = self.cart.as_ref().map(|c| c.is_vs_system).unwrap_or(false);
                        self.screen[y * 256 + x] = if is_vs {
                            match self.vs_ppu_variant {
                                0 => VS_RP2C04_0001_PALETTE[pal_idx],
                                1 => VS_RP2C04_0002_PALETTE[pal_idx],
                                2 => VS_RP2C04_0003_PALETTE[pal_idx],
                                4 => NES_PALETTE[pal_idx],
                                _ => VS_RP2C04_0004_PALETTE[pal_idx],
                            }
                        } else {
                            NES_PALETTE[pal_idx]
                        };
                    }
                }
            }
        }
    }

    fn update_sprite_shift_registers(&mut self) {
        if self.ppu_dot >= 1 && self.ppu_dot <= 256 {
            for i in 0..8 {
                if self.ppu_sprite_shifter_counter[i] > 0 && !self.skipped_pre_render_dot_341 {
                    self.ppu_sprite_shifter_counter[i] -= 1;
                } else if self.ppu_mask_show_sprites || self.ppu_mask_show_background {
                    self.ppu_sprite_sr_l[i] <<= 1;
                    self.ppu_sprite_sr_h[i] <<= 1;
                    if self.vt03_4bpp_sp_enabled() {
                        self.ppu_sprite_sr_l2[i] <<= 1;
                        self.ppu_sprite_sr_h2[i] <<= 1;
                    }
                }
            }
        }
    }

    // oam corruption
    pub fn corrupt_oam(&mut self) {
        if self.ppu_oam_corruption_index == 0x20 {
            self.ppu_oam_corruption_index = 0;
        }
        let base_idx = self.ppu_oam_corruption_index as usize * 8;
        for i in 0..8 {
            self.oam[base_idx + i] = self.oam[i];
        }
        self.oam2[self.ppu_oam_corruption_index as usize] = self.oam2[0];
    }

    // palette corruption
    pub fn corrupt_palettes(&mut self, color: u8, _palette: u8) {
        if (self.cpu_clock & 3) != 2 {
            return;
        }

        let v = self.ppu_v as usize;
        let ram = self.palette_ram;
        let mut c = ram;

        match color {
            0 => {
                let v_low = v & 0xF;
                let v_c = v & 0xC;
                c[v_low] = (ram[0] & ram[v_c]) | (ram[0] & ram[v_low]) | (ram[v_c] & ram[v_low]);
            }
            1 => {
                match v & 0xF {
                    0 => {
                        c[0x0] = (ram[0x1] & ram[0xD]) | ram[0x0];
                        c[0x4] = ram[0x5];
                        c[0x8] = ram[0x9];
                        c[0xC] = ram[0xD];
                    }
                    1 => {}
                    2 => {
                        c[0x2] = (ram[0x2] | ram[0xD]) & ram[0x3];
                        c[0x3] = (ram[0x1] | ram[0x2]) & ram[0x3];
                        c[0x6] = (ram[0x6] | ram[0x5]) & ram[0x7];
                        c[0xA] = (ram[0xA] | ram[0x9]) & ram[0xB];
                        c[0xE] = ram[0xD];
                        c[0xF] = ram[0xD];
                    }
                    3 => {
                        c[0x3] &= ram[0x1] | ram[0xD];
                        c[0xF] = ram[0xD];
                    }
                    4 => {
                        c[0x0] = ram[0x1];
                        c[0x4] = (ram[0x5] & ram[0xD]) | ram[0x4];
                        c[0x8] = ram[0x9];
                        c[0xC] = ram[0xD];
                    }
                    5 => {}
                    6 => {
                        c[0x2] = (ram[0x2] | ram[0x1]) & ram[0x3];
                        c[0x6] = (ram[0x6] | ram[0x7]) & ram[0xD];
                        c[0x7] = (ram[0x7] | ram[0x6]) & ram[0x5];
                        c[0xA] = (ram[0xA] | ram[0x9]) & ram[0xB];
                        c[0xE] = ram[0xD];
                        c[0xF] = ram[0xD];
                    }
                    7 => {
                        c[0x7] &= ram[0x5] | ram[0xD];
                        c[0xF] = ram[0xD];
                    }
                    8 => {
                        c[0x0] = ram[0x1];
                        c[0x4] = ram[0x5];
                        c[0x8] = (ram[0x9] & ram[0xD]) | ram[0x8];
                        c[0xC] = ram[0xD];
                    }
                    9 => {}
                    0xA => {
                        c[0x2] = (ram[0x2] | ram[0x1]) & ram[0x3];
                        c[0x6] = (ram[0x6] | ram[0xD]) & ram[0x7];
                        c[0xA] = (ram[0xB] | ram[0xD]) & ram[0xA];
                        c[0xB] = (ram[0x9] | ram[0xA]) & ram[0xB];
                        c[0xE] = ram[0xD];
                        c[0xF] = ram[0xD];
                    }
                    0xB => {
                        c[0xB] &= ram[0x9] | ram[0xD];
                        c[0xF] = ram[0xD];
                    }
                    0xC => {
                        c[0x0] = ram[0x1];
                        c[0x4] = ram[0x5];
                        c[0x8] = ram[0x9];
                        c[0xC] = ram[0xD];
                    }
                    0xD => {}
                    0xE => {
                        c[0x2] = (ram[0x2] | ram[0x1]) & ram[0x3];
                        c[0x6] = (ram[0x6] | ram[0xD]) & ram[0x7];
                        c[0xA] = (ram[0xA] | ram[0x9]) & ram[0xB];
                        c[0xE] = ram[0xD];
                        c[0xF] = ram[0xD];
                    }
                    0xF => {
                        c[0xF] = ram[0xD];
                    }
                    _ => {}
                }
            }
            2 => {
                match v & 0xF {
                    0 => {
                        c[0x0] = ram[0x0] | (ram[0x2] & ram[0xE]);
                        c[0x4] = ram[0x6];
                        c[0x8] = ram[0xA];
                        c[0xC] = ram[0xE];
                    }
                    1 => {
                        c[0x1] = (ram[0x2] | ram[0x1] | ram[0xE]) & (ram[0x3] | ram[0xE]);
                        c[0x3] = (ram[0x2] | ram[0xE] | 0x3C) & ram[0x3];
                        c[0x5] = (ram[0x6] | ram[0x7]) & ram[0x5];
                        c[0x9] = (ram[0xA] | ram[0xB]) & ram[0x9];
                        c[0xD] = ram[0xE];
                        c[0xF] = ram[0xE];
                    }
                    2 => {}
                    3 => {
                        c[0x3] &= ram[0x2] | ram[0xE];
                        c[0xF] = ram[0xE];
                    }
                    4 => {
                        c[0x0] = ram[0x2];
                        c[0x4] = ram[0x4] | (ram[0x6] & ram[0xE]);
                        c[0x8] = ram[0xA];
                        c[0xC] = ram[0xE];
                    }
                    5 => {
                        c[0x1] = (ram[0x2] | ram[0x1]) & ram[0x3];
                        c[0x5] = (ram[0xE] | ram[0x6]) & ram[0x5];
                        c[0x7] = (ram[0xE] | ram[0x6]) & ram[0x7];
                        c[0xD] = ram[0xE];
                        c[0xF] = ram[0xE];
                    }
                    6 => {}
                    7 => {
                        c[0x7] &= ram[0x6] | ram[0xE];
                    }
                    8 => {
                        c[0x0] = ram[0x2];
                        c[0x4] = ram[0x6];
                        c[0x8] = ram[0x8] | (ram[0xA] & ram[0xE]);
                        c[0xC] = ram[0xE];
                    }
                    9 => {
                        c[0x1] = (ram[0x2] | ram[0x1]) & ram[0x3];
                        c[0x5] = (ram[0x6] | ram[0x5]) & ram[0x7];
                        c[0x9] = (ram[0xE] | ram[0xA] | 0x01) & ram[0x9];
                        c[0xB] = (ram[0xE] | ram[0xA] | 0x31) & ram[0xB];
                        c[0xD] = ram[0xE];
                        c[0xF] = ram[0xE];
                    }
                    0xA => {}
                    0xB => {
                        c[0xB] &= ram[0xA] | ram[0xE];
                        c[0xF] = ram[0xE];
                    }
                    0xC => {
                        c[0x0] = ram[0x2];
                        c[0x4] = ram[0x6];
                        c[0x8] = ram[0xA];
                        c[0xC] = ram[0xE];
                    }
                    0xD => {
                        c[0x1] = (ram[0x2] | ram[0x1]) & ram[0x3];
                        c[0x5] = (ram[0x6] | ram[0x5]) & ram[0x7];
                        c[0x9] = (ram[0xA] | ram[0x9]) & ram[0xB];
                        c[0xD] = ram[0xE];
                        c[0xF] = ram[0xE];
                    }
                    0xE => {}
                    0xF => {
                        c[0xF] = ram[0xE];
                    }
                    _ => {}
                }
            }
            3 => {
                match v & 0xF {
                    0 => {
                        c[0x0] = ram[0x3] | (ram[0xF] & ram[0x0]);
                        c[0x4] &= ram[0x7];
                        c[0x8] &= ram[0x9] | ram[0xA] | ram[0xB] | ram[0xF] | 0x22;
                        c[0xC] = ram[0xF];
                    }
                    1 => {
                        c[0x1] = (ram[0x1] | ram[0xF]) & ram[0x3];
                        c[0x5] = ram[0x7];
                        c[0x9] = ram[0xB];
                        c[0xD] = ram[0xF];
                    }
                    2 => {
                        c[0x2] = (ram[0x3] | ram[0xF]) & ram[0x3];
                        c[0x6] = ram[0x7];
                        c[0xA] = ram[0xB];
                        c[0xE] = ram[0xF];
                    }
                    3 => {}
                    4 => {
                        c[0x0] &= ((ram[0xF] ^ 0xFF)) | ram[0x1] | ram[0x2] | ram[0x3] | 0x7;
                        c[0x4] &= ram[0x7] | ram[0xF];
                        c[0x8] &= ram[0xB] | ram[0xF] | (ram[0xC] ^ 0xFF);
                        c[0xC] = (ram[0x7] & ram[0xF]) | ram[0xC];
                    }
                    5 => {
                        c[0x1] = ram[0x3];
                        c[0x5] = (ram[0x5] | ram[0xF]) & ram[0x7];
                        c[0x9] = ram[0xB];
                        c[0xD] = ram[0xF];
                    }
                    6 => {
                        c[0x2] = ram[0x3];
                        c[0x6] = (ram[0x6] | ram[0xF]) & ram[0x7];
                        c[0xA] = ram[0xB];
                        c[0xE] = ram[0xF];
                    }
                    7 => {}
                    8 => {
                        c[0x0] &= ((ram[0xF] ^ 0xFF)) | ram[0x1] | ram[0x2] | ram[0x3] | 0x23;
                        c[0x4] = ram[0x7];
                        c[0x8] &= ram[0xB] | ram[0xF] | (ram[0xC] ^ 0xFF);
                        c[0xC] = (ram[0xB] & ram[0xF]) | ram[0xC];
                    }
                    9 => {
                        c[0x1] = ram[0x3];
                        c[0x5] = ram[0x7];
                        c[0x9] = (ram[0x9] | ram[0xF]) & ram[0xB];
                        c[0xD] = ram[0xF];
                    }
                    0xA => {
                        c[0x2] = ram[0x3];
                        c[0x6] = ram[0x7];
                        c[0xA] = (ram[0xA] | ram[0xF]) & ram[0xB];
                        c[0xE] = ram[0xF];
                    }
                    0xB => {}
                    0xC => {
                        c[0x0] &= ((ram[0xF] ^ 0xFF)) | ram[0x1] | ram[0x2] | ram[0x3] | 0x37;
                        c[0x4] = ram[0x7];
                        c[0x8] &= ram[0xB] | 0x2F;
                        c[0xC] = ram[0xF];
                    }
                    0xD => {
                        c[0x1] = ram[0x3];
                        c[0x5] = ram[0x7];
                        c[0x9] = ram[0xB];
                        c[0xD] = ram[0xF];
                    }
                    0xE => {
                        c[0x2] = ram[0x3];
                        c[0x6] = ram[0x7];
                        c[0xA] = ram[0xB];
                        c[0xE] = ram[0xF];
                    }
                    0xF => {}
                    _ => {}
                }
            }
            _ => {}
        }

        self.palette_ram = c;
    }
}

impl Emulator {

    // the sprite evaluation
    pub fn ppu_sprite_evaluation(&mut self) {
        if self.ppu_mask_show_background_instant || self.ppu_mask_show_sprites_instant {
            if self.ppu_pending_oam_corruption {
                self.ppu_pending_oam_corruption = false;
                if !self.ppu_oam_corruption_rendering_enabled_out_of_vblank {
                    self.corrupt_oam();
                }
                self.ppu_oam_corruption_rendering_enabled_out_of_vblank = false;
            }
        }

        let pre_render = self.ppu_scanline == self.pre_render_scanline();

        if self.ppu_dot <= 64 {
            if (self.ppu_dot & 1) == 1 {
                if self.ppu_mask_show_background_delayed || self.ppu_mask_show_sprites_delayed {
                    if pre_render {
                        self.ppu_oam_latch = self.oam2[self.oam2_address as usize];
                    } else {
                        self.ppu_oam_latch = 0xFF;
                    }
                    if self.ppu_dot == 1 {
                        self.oam2_address = 0;
                        self.secondary_oam_full = false;
                        self.sprite_evaluation_tick = 0;
                        self.oam_address_overflowed_during_sprite_evaluation = false;
                    }
                    if self.ppu_oam_corruption_rendering_disabled_out_of_vblank
                        || self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant
                    {
                        self.ppu_oam_corruption_rendering_disabled_out_of_vblank = false;
                        self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant = false;
                        self.ppu_pending_oam_corruption = true;
                        self.ppu_oam_corruption_index = self.oam2_address;
                    }
                }
            } else {
                if self.ppu_dot > 0 {
                    if self.ppu_mask_show_background_delayed || self.ppu_mask_show_sprites_delayed {
                        if !pre_render {
                            self.oam2[self.oam2_address as usize] = self.ppu_oam_latch;
                        }
                        if self.ppu_oam_corruption_rendering_disabled_out_of_vblank {
                            self.ppu_oam_corruption_rendering_disabled_out_of_vblank = false;
                            self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant = false;
                            self.ppu_pending_oam_corruption = true;
                            self.ppu_oam_corruption_index = self.oam2_address;
                        }
                        self.oam2_address = self.oam2_address.wrapping_add(1) & 0x1F;
                        if self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant && self.ppu_dot == 64 {
                            self.ppu_oam_corruption_rendering_disabled_out_of_vblank = false;
                            self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant = false;
                            self.ppu_pending_oam_corruption = true;
                        }
                    } else {
                        if self.ppu_oam_corruption_rendering_disabled_out_of_vblank
                            || self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant
                        {
                            self.ppu_oam_corruption_rendering_disabled_out_of_vblank = false;
                            self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant = false;
                            self.ppu_pending_oam_corruption = true;
                            self.ppu_oam_corruption_index = self.oam2_address;
                        }
                    }
                } else {
                    self.oam2_address = self.oam2_address.wrapping_add(1) & 0x1F;
                }
            }
        }
        else if self.ppu_dot >= 65 && self.ppu_dot <= 256 {
            if self.ppu_dot == 65 {
                self.oam2_address = 0;
                self.nine_objects_on_this_scanline = false;
            }
            if self.ppu_mask_show_background_instant
                || self.ppu_mask_show_sprites_instant
                || self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant
            {
                if (self.ppu_dot & 1) == 1 {
                    let _prev = self.ppu_oam_latch;
                    self.ppu_oam_latch = self.oam[self.ppu_oam_address as usize];
                    if (self.ppu_oam_address & 3) == 2 {
                        self.ppu_oam_latch &= 0xE7;
                    }
                    if self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant {
                        self.ppu_oam_evaluation_corruption_odd_cycle = false;
                        self.ppu_oam_corruption_rendering_disabled_out_of_vblank = false;
                        if !pre_render {
                            self.ppu_oam_address = self.ppu_oam_address.wrapping_add(1);
                        }
                        self.oam_corrupted_on_odd_cycle = true;
                    }
                } else {
                    if !self.oam_address_overflowed_during_sprite_evaluation {
                        let pre_inc_val = self.ppu_oam_address;
                        if !self.secondary_oam_full && !pre_render {
                            self.oam2[self.oam2_address as usize] = self.ppu_oam_latch;
                        }
                        let oam2_read = self.oam2[self.oam2_address as usize];

                        if self.sprite_evaluation_tick == 0 {
                            self.ppu_oam_evaluation_object_in_x_range = false;
                            self.in_range_check = ((self.ppu_scanline & 0xFF) as u16).wrapping_sub(self.ppu_oam_latch as u16);
                            let sprite_h: u16 = if self.ppu_sprite_x16 { 16 } else { 8 };

                            if !self.nine_objects_on_this_scanline && !pre_render && self.in_range_check < sprite_h {
                                self.ppu_oam_evaluation_object_in_range = true;
                                if !self.secondary_oam_full {
                                    if !self.oam_corrupted_on_odd_cycle && !pre_render {
                                        self.ppu_oam_address = self.ppu_oam_address.wrapping_add(1);
                                    }
                                    if !self.oam_corrupted_on_odd_cycle {
                                        self.oam2_address = self.oam2_address.wrapping_add(1);
                                    }
                                    if !self.secondary_oam_full {
                                        self.oam2_address &= 0x1F;
                                        if self.oam2_address == 0 {
                                            self.secondary_oam_full = true;
                                        }
                                    }
                                    if self.ppu_dot == 66 {
                                        self.ppu_next_scanline_contains_sprite_zero = true;
                                    }
                                } else {
                                    self.nine_objects_on_this_scanline = true;
                                    self.ppu_oam_address = self.ppu_oam_address.wrapping_add(1);
                                    if !self.ppu_status_sprite_overflow {
                                        self.ppu_status_sprite_overflow = true;
                                    }
                                }
                                if !pre_render {
                                    self.sprite_evaluation_tick += 1;
                                }
                            } else {
                                if self.ppu_dot == 66 {
                                    self.ppu_next_scanline_contains_sprite_zero = false;
                                }
                                self.ppu_oam_evaluation_object_in_range = false;
                                if !self.oam_corrupted_on_odd_cycle && !pre_render {
                                    if self.secondary_oam_full && !self.nine_objects_on_this_scanline {
                                        if (self.ppu_oam_address & 0x3) == 3 {
                                            self.ppu_oam_address = self.ppu_oam_address.wrapping_add(1);
                                        } else {
                                            self.ppu_oam_address = self.ppu_oam_address.wrapping_add(4);
                                            self.ppu_oam_address = self.ppu_oam_address.wrapping_add(1);
                                        }
                                    } else {
                                        self.ppu_oam_address = self.ppu_oam_address.wrapping_add(4);
                                        self.ppu_oam_address &= 0xFC;
                                    }
                                }
                            }
                        } else {
                            if self.sprite_evaluation_tick == 3 {
                                self.ppu_oam_evaluation_object_in_range = false;
                                let sprite_h: u16 = if self.ppu_sprite_x16 { 16 } else { 8 };
                                let diff = (self.ppu_scanline as u16).wrapping_sub(self.ppu_oam_latch as u16);
                                if diff < sprite_h {
                                    self.ppu_oam_evaluation_object_in_x_range = true;
                                    if !self.secondary_oam_full {
                                        if !self.oam_corrupted_on_odd_cycle && !pre_render {
                                            self.ppu_oam_address = self.ppu_oam_address.wrapping_add(1);
                                        }
                                    } else {
                                        if !self.oam_corrupted_on_odd_cycle && !pre_render {
                                            self.ppu_oam_address = self.ppu_oam_address.wrapping_add(4);
                                        }
                                    }
                                } else {
                                    self.ppu_oam_evaluation_object_in_x_range = false;
                                    if !self.secondary_oam_full {
                                        if !self.oam_corrupted_on_odd_cycle && !pre_render {
                                            self.ppu_oam_address = self.ppu_oam_address.wrapping_add(1);
                                            self.ppu_oam_address &= 0xFC;
                                        }
                                    } else {
                                        self.ppu_oam_address = self.ppu_oam_address.wrapping_add(1);
                                        self.ppu_oam_address &= 0xFC;
                                    }
                                }
                            } else {
                                if !self.oam_corrupted_on_odd_cycle && !pre_render {
                                    self.ppu_oam_address = self.ppu_oam_address.wrapping_add(1);
                                }
                            }
                            self.sprite_evaluation_tick += 1;
                            self.sprite_evaluation_tick &= 3;
                            if !self.secondary_oam_full && !pre_render {
                                self.oam2_address = self.oam2_address.wrapping_add(1);
                                self.oam2_address &= 0x1F;
                                if self.oam2_address == 0 {
                                    self.secondary_oam_full = true;
                                }
                            }
                        }
                        self.oam_corrupted_on_odd_cycle = false;
                        if self.ppu_oam_address < pre_inc_val && self.ppu_oam_address < 4 {
                            self.oam_address_overflowed_during_sprite_evaluation = true;
                        }
                        self.ppu_oam_latch = oam2_read;
                    } else {
                        if !self.oam_corrupted_on_odd_cycle && !pre_render {
                            self.ppu_oam_address = self.ppu_oam_address.wrapping_add(4);
                            self.ppu_oam_address &= 0xFC;
                        }
                        self.ppu_oam_latch = self.oam2[self.oam2_address as usize];
                    }
                    if self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant
                        && !self.ppu_oam_evaluation_corruption_odd_cycle
                    {
                        self.ppu_oam_corruption_rendering_disabled_out_of_vblank = false;
                        self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant = false;
                        self.ppu_pending_oam_corruption = true;
                        if (self.oam2_address & 3) != 0 && !self.oam_address_overflowed_during_sprite_evaluation && !pre_render {
                            self.oam2_address &= 0xFC;
                            self.oam2_address = self.oam2_address.wrapping_add(4);
                        }
                        self.ppu_oam_corruption_index = self.oam2_address;
                    }
                    self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant = false;
                }
            }
        }
        else if self.ppu_dot >= 257 && self.ppu_dot <= 320 {
            self.ppu_current_scanline_contains_sprite_zero = self.ppu_next_scanline_contains_sprite_zero;
            if self.ppu_mask_show_background_delayed || self.ppu_mask_show_sprites_delayed {
                self.ppu_oam_address = 0;
            }
            if self.ppu_dot == 257 {
                self.oam2_address = 0;
                self.sprite_evaluation_tick = 0;
            }

            if self.ppu_oam_corruption_rendering_disabled_out_of_vblank
                && (self.ppu_clock == 0 || self.ppu_clock == 3)
            {
                self.ppu_oam_corruption_rendering_disabled_out_of_vblank = false;
                self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant = false;
                self.ppu_pending_oam_corruption = true;
                self.ppu_oam_corruption_index = self.oam2_address;
            }

            if self.ppu_read {
                self.ppu_octal_latch = self.ppu_address_bus as u8;
            }

            let tick = self.sprite_evaluation_tick;
            match tick {
                0 => {
                    if self.onebus_chr_routing_ppu() {
                        self.ppu_onebus_eva = 0;
                    }
                    if self.ppu_mask_show_background_delayed || self.ppu_mask_show_sprites_delayed {
                        self.ppu_oam_latch = self.oam2[self.oam2_address as usize];
                        let slot = (self.oam2_address / 4) as usize;
                        if slot < 8 { self.ppu_sprite_y_position[slot] = self.ppu_oam_latch; }
                        let masked_v = self.vt369_mask_vram_addr_for_nt(self.ppu_v);
                        self.ppu_pattern_address_register_nt = 0x2000 + (masked_v & 0x0FFF);
                        self.ppu_address_bus = self.ppu_pattern_address_register_nt;
                        self.in_range_check = ((self.ppu_scanline & 0xFF) as u16).wrapping_sub(self.ppu_oam_latch as u16);
                    }
                    self.oam2_address = self.oam2_address.wrapping_add(1);
                }
                1 => {
                    if self.ppu_mask_show_background_delayed || self.ppu_mask_show_sprites_delayed {
                        self.ppu_oam_latch = self.oam2[self.oam2_address as usize];
                        let slot = (self.oam2_address / 4) as usize;
                        if slot < 8 { self.ppu_sprite_pattern[slot] = self.ppu_oam_latch; }
                        self.ppu_render_bg_fetches(); // dummy NT fetch
                    }
                    self.oam2_address = self.oam2_address.wrapping_add(1);
                }
                2 => {
                    if self.ppu_mask_show_background_delayed || self.ppu_mask_show_sprites_delayed {
                        self.ppu_oam_latch = self.oam2[self.oam2_address as usize];
                        let slot = (self.oam2_address / 4) as usize;
                        if slot < 8 { self.ppu_sprite_attribute[slot] = self.ppu_oam_latch; }
                        let masked_v = self.vt369_mask_vram_addr_for_nt(self.ppu_v);
                        self.ppu_pattern_address_register_nt = 0x2000 + (masked_v & 0x0FFF);
                        self.ppu_address_bus = self.ppu_pattern_address_register_nt;
                    }
                    self.oam2_address = self.oam2_address.wrapping_add(1);
                }
                3 => {
                    if self.ppu_mask_show_background_delayed || self.ppu_mask_show_sprites_delayed {
                        self.ppu_oam_latch = self.oam2[self.oam2_address as usize];
                        let slot = (self.oam2_address / 4) as usize;
                        if slot < 8 {
                            self.ppu_sprite_x_position[slot] = self.ppu_oam_latch;
                            self.ppu_sprite_shifter_counter[slot] = self.ppu_oam_latch;
                        }
                        self.ppu_render_bg_fetches();
                    }
                }
                4 => {
                    if self.ppu_mask_show_background_delayed || self.ppu_mask_show_sprites_delayed {
                        self.ppu_oam_latch = self.oam2[self.oam2_address as usize];
                        let slot = (self.oam2_address / 4) as usize;
                        if slot < 8 { self.ppu_sprite_x_position[slot] = self.ppu_oam_latch; }
                        self.ppu_sprite_get_address((self.oam2_address / 4) as usize);
                        self.ppu_check_par();
                        self.ppu_pattern_address_register_chr &= 0b1111111110111;
                        self.ppu_address_bus = self.ppu_pattern_address_register_chr;
                    }
                }
                5 => {
                    if self.ppu_mask_show_background_delayed || self.ppu_mask_show_sprites_delayed {
                        self.ppu_oam_latch = self.oam2[self.oam2_address as usize];
                        let slot = (self.oam2_address / 4) as usize;
                        if slot < 8 { self.ppu_sprite_x_position[slot] = self.ppu_oam_latch; }
                        self.ppu_sprite_get_address((self.oam2_address / 4) as usize);
                        let base_addr = (self.ppu_address_bus & 0xFF00) | self.ppu_octal_latch as u16;
                        self.ppu_address_bus = base_addr;
                        self.onebus_begin_sprite_chr_fetch(slot);
                        self.ppu_sprite_pattern_l = self.fetch_ppu();
                        self.onebus_clear_chr_fetch();
                        if slot < 8 && ((self.ppu_sprite_attribute[slot] >> 6) & 1) == 1 {
                            self.ppu_sprite_pattern_l = Self::flip_byte(self.ppu_sprite_pattern_l);
                        }
                        if slot < 8 { self.ppu_sprite_sr_l[slot] = self.ppu_sprite_pattern_l; }
                        let sprite_h: u16 = if self.ppu_sprite_x16 { 16 } else { 8 };
                        if !(self.in_range_check < sprite_h) {
                            if slot < 8 { self.ppu_sprite_sr_l[slot] = 0; }
                        }
                        if self.vt03_4bpp_sp_enabled() {
                            self.ppu_address_bus = base_addr | 0x4000;
                            self.onebus_begin_sprite_chr_fetch(slot);
                            self.ppu_sprite_pattern_l2 = self.fetch_ppu();
                            self.onebus_clear_chr_fetch();
                            if slot < 8 && ((self.ppu_sprite_attribute[slot] >> 6) & 1) == 1 {
                                self.ppu_sprite_pattern_l2 = Self::flip_byte(self.ppu_sprite_pattern_l2);
                            }
                            if slot < 8 { self.ppu_sprite_sr_l2[slot] = self.ppu_sprite_pattern_l2; }
                            if !(self.in_range_check < sprite_h) {
                                if slot < 8 { self.ppu_sprite_sr_l2[slot] = 0; }
                            }
                        }
                    }
                }
                6 => {
                    if self.ppu_mask_show_background_delayed || self.ppu_mask_show_sprites_delayed {
                        self.ppu_oam_latch = self.oam2[self.oam2_address as usize];
                        let slot = (self.oam2_address / 4) as usize;
                        if slot < 8 { self.ppu_sprite_x_position[slot] = self.ppu_oam_latch; }
                        self.ppu_sprite_get_address((self.oam2_address / 4) as usize);
                        self.ppu_check_par();
                        self.ppu_pattern_address_register_chr |= 8;
                        self.ppu_address_bus = self.ppu_pattern_address_register_chr;
                    }
                }
                7 => {
                    if self.ppu_mask_show_background_delayed || self.ppu_mask_show_sprites_delayed {
                        self.ppu_oam_latch = self.oam2[self.oam2_address as usize];
                        let slot = (self.oam2_address / 4) as usize;
                        if slot < 8 { self.ppu_sprite_x_position[slot] = self.ppu_oam_latch; }
                        self.ppu_sprite_get_address((self.oam2_address / 4) as usize);
                        let base_addr = (self.ppu_address_bus & 0xFF00) | self.ppu_octal_latch as u16;
                        self.ppu_address_bus = base_addr;
                        self.onebus_begin_sprite_chr_fetch(slot);
                        self.ppu_sprite_pattern_h = self.fetch_ppu();
                        self.onebus_clear_chr_fetch();
                        if slot < 8 && ((self.ppu_sprite_attribute[slot] >> 6) & 1) == 1 {
                            self.ppu_sprite_pattern_h = Self::flip_byte(self.ppu_sprite_pattern_h);
                        }
                        if slot < 8 { self.ppu_sprite_sr_h[slot] = self.ppu_sprite_pattern_h; }
                        let sprite_h: u16 = if self.ppu_sprite_x16 { 16 } else { 8 };
                        if !(self.in_range_check < sprite_h) {
                            if slot < 8 { self.ppu_sprite_sr_h[slot] = 0; }
                        }
                        if self.vt03_4bpp_sp_enabled() {
                            self.ppu_address_bus = base_addr | 0x4000;
                            self.onebus_begin_sprite_chr_fetch(slot);
                            self.ppu_sprite_pattern_h2 = self.fetch_ppu();
                            self.onebus_clear_chr_fetch();
                            if slot < 8 && ((self.ppu_sprite_attribute[slot] >> 6) & 1) == 1 {
                                self.ppu_sprite_pattern_h2 = Self::flip_byte(self.ppu_sprite_pattern_h2);
                            }
                            if slot < 8 { self.ppu_sprite_sr_h2[slot] = self.ppu_sprite_pattern_h2; }
                            if !(self.in_range_check < sprite_h) {
                                if slot < 8 { self.ppu_sprite_sr_h2[slot] = 0; }
                            }
                        }
                    }
                    self.oam2_address = self.oam2_address.wrapping_add(1);
                }
                _ => {}
            }
            if self.ppu_ale && !self.ppu_read {
                self.ppu_octal_latch = self.ppu_address_bus as u8;
            }
            self.oam2_address &= 0x1F;
            self.sprite_evaluation_tick = (self.sprite_evaluation_tick + 1) & 7;

            if self.ppu_oam_corruption_rendering_disabled_out_of_vblank
                && (self.ppu_clock == 1 || self.ppu_clock == 2)
            {
                self.ppu_oam_corruption_rendering_disabled_out_of_vblank = false;
                self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant = false;
                self.ppu_pending_oam_corruption = true;
                self.ppu_oam_corruption_index = self.oam2_address;
            }
        }
        else {
            if self.ppu_dot == 339 {
                if !self.ppu_mask_show_background && !self.ppu_mask_show_sprites {
                    for i in 0..8 {
                        self.ppu_sprite_shifter_counter[i] = 0;
                    }
                }
            }
            if self.ppu_oam_corruption_rendering_disabled_out_of_vblank
                || self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant
            {
                self.ppu_oam_corruption_rendering_disabled_out_of_vblank = false;
                self.ppu_oam_corruption_rendering_disabled_out_of_vblank_instant = false;
                self.ppu_pending_oam_corruption = true;
                self.ppu_oam_corruption_index = self.oam2_address;
            }
        }
    }

    fn ppu_sprite_get_address(&mut self, slot: usize) {
        if slot >= 8 { return; }
        let scanline = (self.ppu_scanline & 0xFF) as u16;
        let y_pos = self.ppu_sprite_y_position[slot] as u16;
        let pattern = self.ppu_sprite_pattern[slot];
        let attr = self.ppu_sprite_attribute[slot];
        let flip_y = (attr >> 7) & 1 == 1;

        let address: u16;
        if !self.ppu_sprite_x16 {
            let base = if self.ppu_pattern_select_sprites { 0x1000u16 } else { 0 };
            let tile_offset = (pattern as u16) << 4;
            if !flip_y {
                address = base.wrapping_add(tile_offset).wrapping_add(scanline.wrapping_sub(y_pos));
            } else {
                address = base.wrapping_add(tile_offset).wrapping_add((7u16.wrapping_sub(scanline.wrapping_sub(y_pos))) & 7);
            }
        } else {
            let base = if (pattern & 1) == 1 { 0x1000u16 } else { 0 };
            let tile_base = ((pattern & 0xFE) as u16) << 4;
            let diff = scanline.wrapping_sub(y_pos);
            if !flip_y {
                if diff < 8 {
                    address = base.wrapping_add(tile_base).wrapping_add(diff);
                } else {
                    address = base.wrapping_add(tile_base).wrapping_add(16).wrapping_add(diff & 7);
                }
            } else {
                if diff < 8 {
                    address = base.wrapping_add(tile_base).wrapping_add(16).wrapping_add(7).wrapping_sub(diff & 7);
                } else {
                    address = base.wrapping_add(tile_base).wrapping_add(7).wrapping_sub(diff & 7);
                }
            }
        }
        self.ppu_address_bus = address & 0x3FFF;
    }
}

const fn vs_palette_from_rgb333(data: &[(u8, u8, u8); 64]) -> [u32; 512] {
    let mut pal = [0u32; 512];
    let mut i = 0;
    while i < 512 {
        let (r, g, b) = data[i & 0x3F];
        let emph_block = i >> 6;
        let er = if (emph_block & 1) != 0 { 0xFF } else { r };
        let eg = if (emph_block & 2) != 0 { 0xFF } else { g };
        let eb = if (emph_block & 4) != 0 { 0xFF } else { b };
        pal[i] = 0xFF000000 | ((er as u32) << 16) | ((eg as u32) << 8) | (eb as u32);
        i += 1;
    }
    pal
}

const VS_RP2C04_0001_PALETTE: [u32; 512] = vs_palette_from_rgb333(&[
    (0xFF, 0xC7, 0xDB), (0x41, 0x8A, 0xFF), (0xDB, 0x28, 0x00), (0x5D, 0x96, 0xFF), (0x00, 0x82, 0x8A), (0x00, 0x45, 0x00), (0x00, 0x00, 0x00), (0xE7, 0x00, 0x59),
    (0xFF, 0xFF, 0xFF), (0x75, 0x75, 0x75), (0xFF, 0x9A, 0x38), (0xAA, 0x00, 0x10), (0x8E, 0x00, 0x75), (0xFF, 0x9A, 0x38), (0x41, 0x2C, 0x00), (0xFF, 0xFF, 0xFF),
    (0x3C, 0xBE, 0xFF), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0x82, 0xD3, 0x10), (0x9E, 0xFF, 0xF3), (0xC7, 0xD7, 0xFF), (0xFF, 0xBE, 0xB2), (0x20, 0x38, 0xEF),
    (0x00, 0x00, 0x00), (0x59, 0xFB, 0x9A), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0xFF, 0xFF, 0xFF), (0xBE, 0xBE, 0xBE), (0xF7, 0x79, 0xFF), (0x24, 0x18, 0x8E),
    (0x00, 0x00, 0x00), (0xAA, 0xE7, 0xFF), (0x00, 0x00, 0x00), (0x4D, 0xDF, 0x49), (0x00, 0xEB, 0xDB), (0x18, 0x3C, 0x5D), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00),
    (0x00, 0x00, 0x00), (0x7D, 0x08, 0x00), (0xFF, 0xC7, 0xFF), (0xA6, 0x00, 0x00), (0x82, 0x00, 0xF3), (0x00, 0x00, 0xAA), (0xFF, 0x75, 0x61), (0x00, 0x00, 0x00),
    (0x00, 0x00, 0x00), (0x00, 0x96, 0x00), (0xBE, 0xBE, 0xBE), (0x00, 0x51, 0x00), (0xE3, 0xFF, 0xA2), (0x00, 0x00, 0x00), (0xFF, 0xDB, 0xAA), (0xCB, 0x4D, 0x0C),
    (0x00, 0x00, 0x00), (0x00, 0x71, 0xEF), (0x00, 0x45, 0x00), (0x00, 0x00, 0x00), (0xE3, 0xFF, 0xA2), (0xFF, 0x75, 0xB6), (0x8A, 0x71, 0x00), (0x00, 0x00, 0x00),
]);

const VS_RP2C04_0002_PALETTE: [u32; 512] = vs_palette_from_rgb333(&[
    (0x00, 0x00, 0x00), (0xFF, 0x9A, 0x38), (0x8A, 0x71, 0x00), (0x00, 0x00, 0x00), (0xAA, 0xF3, 0xBE), (0xFF, 0x75, 0xB6), (0x00, 0x00, 0x00), (0xAA, 0xE7, 0xFF),
    (0xDB, 0x28, 0x00), (0x82, 0x00, 0xF3), (0xFF, 0xE7, 0xA2), (0xFF, 0xC7, 0xFF), (0xFF, 0xFF, 0xFF), (0x41, 0x8A, 0xFF), (0x00, 0x00, 0x00), (0x00, 0x3C, 0x14),
    (0x00, 0x00, 0x00), (0x3C, 0xBE, 0xFF), (0xA6, 0x00, 0x00), (0x00, 0x00, 0x00), (0x00, 0x92, 0x38), (0x82, 0xD3, 0x10), (0x00, 0x00, 0x00), (0x5D, 0x96, 0xFF),
    (0x00, 0x00, 0x00), (0xF7, 0x79, 0xFF), (0x00, 0x00, 0x00), (0x59, 0xFB, 0x9A), (0x00, 0x00, 0x00), (0x41, 0x2C, 0x00), (0x00, 0x00, 0x00), (0x45, 0x00, 0x9E),
    (0x00, 0x00, 0x00), (0xFF, 0xBE, 0xB2), (0xFF, 0x75, 0x61), (0xD7, 0xCB, 0xFF), (0x00, 0x71, 0xEF), (0x00, 0x00, 0x00), (0xBE, 0xBE, 0xBE), (0x00, 0x00, 0xAA),
    (0xBE, 0x00, 0xBE), (0x00, 0x00, 0x00), (0x75, 0x75, 0x75), (0x00, 0x45, 0x00), (0x20, 0x38, 0xEF), (0x00, 0x00, 0x00), (0xFF, 0xDB, 0xAA), (0xFF, 0xFF, 0xFF),
    (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0x4D, 0xDF, 0x49), (0xCB, 0x4D, 0x0C), (0x18, 0x3C, 0x5D), (0x24, 0x18, 0x8E), (0xE7, 0x00, 0x59), (0x00, 0x96, 0x00),
    (0x00, 0x00, 0x00), (0x00, 0xEB, 0xDB), (0x7D, 0x08, 0x00), (0xFF, 0xDB, 0xAA), (0x00, 0x00, 0x00), (0xAA, 0x00, 0x10), (0x00, 0x51, 0x00), (0x75, 0x75, 0x75),
]);

const VS_RP2C04_0003_PALETTE: [u32; 512] = vs_palette_from_rgb333(&[
    (0x45, 0x00, 0x9E), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0x75, 0x75, 0x75), (0x00, 0xAA, 0x00), (0xFF, 0xFF, 0xFF), (0xAA, 0xE7, 0xFF), (0x00, 0x45, 0x00),
    (0x24, 0x18, 0x8E), (0x00, 0x00, 0x00), (0xFF, 0xBE, 0xB2), (0x41, 0x2C, 0x00), (0xE7, 0x00, 0x59), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0xFF, 0xFF, 0xFF),
    (0x5D, 0x96, 0xFF), (0x00, 0x82, 0x8A), (0x00, 0x00, 0x00), (0x20, 0x38, 0xEF), (0x00, 0x96, 0x00), (0x8A, 0x71, 0x00), (0xCB, 0x4D, 0x0C), (0x00, 0x92, 0x38),
    (0x75, 0x75, 0x75), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0x00, 0x00, 0xAA), (0xDB, 0x28, 0x00), (0xA6, 0x00, 0x00), (0x00, 0x00, 0x00), (0xFF, 0xC7, 0xDB),
    (0x41, 0x8A, 0xFF), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0xFF, 0xDB, 0xAA), (0x00, 0x00, 0x00), (0xFF, 0x9A, 0x38), (0xFF, 0x75, 0x61), (0xFF, 0xFF, 0xFF),
    (0x82, 0xD3, 0x10), (0x00, 0x00, 0x00), (0x3C, 0xBE, 0xFF), (0xF7, 0x79, 0xFF), (0x00, 0x71, 0xEF), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00),
    (0x00, 0xEB, 0xDB), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0x7D, 0x08, 0x00), (0x4D, 0xDF, 0x49), (0xF3, 0xBE, 0x3C), (0x00, 0x00, 0x00),
    (0x00, 0x51, 0x00), (0x00, 0x00, 0x00), (0xC7, 0xD7, 0xFF), (0xFF, 0xDB, 0xAA), (0x82, 0x00, 0xF3), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0x18, 0x3C, 0x5D),
]);

const VS_RP2C04_0004_PALETTE: [u32; 512] = vs_palette_from_rgb333(&[
    (0x8A, 0x71, 0x00), (0x00, 0x00, 0x00), (0x00, 0x82, 0x8A), (0xF3, 0xBE, 0x3C), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0x24, 0x18, 0x8E), (0xCB, 0x4D, 0x0C),
    (0xBE, 0xBE, 0xBE), (0x00, 0x00, 0x00), (0x4D, 0xDF, 0x49), (0x00, 0x00, 0x00), (0xFF, 0xBE, 0xB2), (0xFF, 0xDB, 0xAA), (0x00, 0xAA, 0x00), (0x00, 0x00, 0x00),
    (0xFF, 0x75, 0xB6), (0x00, 0x00, 0x00), (0x20, 0x38, 0xEF), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0xFF, 0x75, 0x61),
    (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0x5D, 0x96, 0xFF), (0x00, 0x96, 0x00), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0xAA, 0xF3, 0xBE), (0x3C, 0xBE, 0xFF),
    (0xAA, 0x00, 0x10), (0x00, 0x51, 0x00), (0x7D, 0x08, 0x00), (0x00, 0x00, 0xAA), (0x82, 0x00, 0xF3), (0x00, 0x00, 0x00), (0x75, 0x75, 0x75), (0xE7, 0x00, 0x59),
    (0x18, 0x3C, 0x5D), (0x00, 0x00, 0x00), (0x00, 0x71, 0xEF), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0xFF, 0xE7, 0xA2), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00),
    (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0x41, 0x2C, 0x00), (0xDB, 0x28, 0x00), (0x00, 0x00, 0x00), (0x00, 0x00, 0x00), (0xFF, 0xFF, 0xFF), (0x9E, 0xFF, 0xF3),
    (0x00, 0x00, 0x00), (0xFF, 0x9A, 0x38), (0x00, 0x00, 0x00), (0xAA, 0xE7, 0xFF), (0x82, 0xD3, 0x10), (0x00, 0x00, 0x00), (0xFF, 0xFF, 0xFF), (0x00, 0x45, 0x00),
]);



const NES_PALETTE: [u32; 512] = [
    0xFF666666, 0xFF002A88, 0xFF1412A7, 0xFF3B00A4, 0xFF5C007E, 0xFF6E0040, 0xFF6C0600, 0xFF561D00,
    0xFF333500, 0xFF0B4800, 0xFF005200, 0xFF004F08, 0xFF00404D, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFFADADAD, 0xFF155FD9, 0xFF4240FF, 0xFF7527FE, 0xFFA01ACC, 0xFFB71E7B, 0xFFB53120, 0xFF994E00,
    0xFF6B6D00, 0xFF388700, 0xFF0C9300, 0xFF008F32, 0xFF007C8D, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFFFFFEFF, 0xFF64B0FF, 0xFF9290FF, 0xFFC676FF, 0xFFF36AFF, 0xFFFE6ECC, 0xFFFE8170, 0xFFEA9E22,
    0xFFBCBE00, 0xFF88D800, 0xFF5CE430, 0xFF45E082, 0xFF48CDDE, 0xFF4F4F4F, 0xFF000000, 0xFF000000,
    0xFFFFFEFF, 0xFFC0DFFF, 0xFFD3D2FF, 0xFFE8C8FF, 0xFFFBC2FF, 0xFFFEC4EA, 0xFFFECCC5, 0xFFF7D8A5,
    0xFFE4E594, 0xFFCFEF96, 0xFFBDF4AB, 0xFFB3F3CC, 0xFFB5EBF2, 0xFFB8B8B8, 0xFF000000, 0xFF000000,
    0xFF7B5E48, 0xFF002478, 0xFF140F93, 0xFF350090, 0xFF50006E, 0xFF5E0038, 0xFF5D0400, 0xFF4B1800,
    0xFF2D2C00, 0xFF0B3D00, 0xFF004600, 0xFF004307, 0xFF003542, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFFC49B7C, 0xFF1554BD, 0xFF3D39E3, 0xFF6B22E0, 0xFF8E17B4, 0xFFA01A6C, 0xFF9E2B1C, 0xFF854400,
    0xFF5E5F00, 0xFF387600, 0xFF118000, 0xFF007C2B, 0xFF006B7C, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFFFFE5C6, 0xFF5C9EE3, 0xFF8480FF, 0xFFB268FF, 0xFFD95FFF, 0xFFE662B4, 0xFFE57261, 0xFFD38C1E,
    0xFFABA800, 0xFF7CBD00, 0xFF54C728, 0xFF40C472, 0xFF42B4C4, 0xFF494945, 0xFF000000, 0xFF000000,
    0xFFFFE5C6, 0xFFAFC7E3, 0xFFBFBCFF, 0xFFD1B4FF, 0xFFE1AFFF, 0xFFE6B1D2, 0xFFE7B7B0, 0xFFDEC195,
    0xFFCFCC86, 0xFFBDD489, 0xFFAFD89A, 0xFFA7D7B7, 0xFFA8D0D9, 0xFFAAAAAA, 0xFF000000, 0xFF000000,
    0xFF4B624D, 0xFF00277A, 0xFF0C1395, 0xFF2B0092, 0xFF480070, 0xFF56003A, 0xFF550500, 0xFF441A00,
    0xFF2A2E00, 0xFF0A3F00, 0xFF004800, 0xFF004508, 0xFF003743, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFF899E80, 0xFF1157BF, 0xFF363BE4, 0xFF6024E1, 0xFF8019B6, 0xFF8F1C6E, 0xFF8D2D1D, 0xFF754600,
    0xFF536100, 0xFF317800, 0xFF0F8200, 0xFF007E2C, 0xFF006D7D, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFFC8E8CA, 0xFF56A0E5, 0xFF7C82FF, 0xFFA76BFF, 0xFFCB61FF, 0xFFD564B6, 0xFFD57463, 0xFFC38E20,
    0xFF9FAA00, 0xFF74BF00, 0xFF50C82A, 0xFF3EC574, 0xFF40B6C6, 0xFF454B47, 0xFF000000, 0xFF000000,
    0xFFC8E8CA, 0xFFA5C9E5, 0xFFB3BEFF, 0xFFC3B6FF, 0xFFD1B2FF, 0xFFD6B3D4, 0xFFD6B9B2, 0xFFCEC397,
    0xFFC1CE88, 0xFFB0D68B, 0xFFA4DA9C, 0xFF9DD9B9, 0xFF9ED2DB, 0xFF9EACAC, 0xFF000000, 0xFF000000,
    0xFF434634, 0xFF001E6B, 0xFF0B0F85, 0xFF270083, 0xFF420064, 0xFF4F0032, 0xFF4E0400, 0xFF3F1500,
    0xFF282700, 0xFF093600, 0xFF003E00, 0xFF003C06, 0xFF002F39, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFF75866D, 0xFF0E4AA8, 0xFF3132CA, 0xFF581EC8, 0xFF7415A0, 0xFF821860, 0xFF802716, 0xFF6B3C00,
    0xFF4D5300, 0xFF2E6700, 0xFF106F00, 0xFF006C25, 0xFF005D6B, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFFB1C8AD, 0xFF4B8ACB, 0xFF6E70E9, 0xFF955CE8, 0xFFB554C2, 0xFFBE567F, 0xFFBE6435, 0xFFAF7B16,
    0xFF8F9400, 0xFF6AA600, 0xFF49AE23, 0xFF3AAC62, 0xFF3B9FAF, 0xFF3E4138, 0xFF000000, 0xFF000000,
    0xFFB1C8AD, 0xFF94ADCB, 0xFFA0A4E9, 0xFFAE9DE8, 0xFFBB99C2, 0xFFBF9BBD, 0xFFC0A09E, 0xFFB9A886,
    0xFFADB17A, 0xFF9EB87C, 0xFF93BB8B, 0xFF8DBBA4, 0xFF8EB5C3, 0xFF8E9696, 0xFF000000, 0xFF000000,
    0xFF5F5F5F, 0xFF00247C, 0xFF110E97, 0xFF340093, 0xFF520072, 0xFF63003A, 0xFF610500, 0xFF4C1900,
    0xFF2D2F00, 0xFF0A4000, 0xFF004900, 0xFF004607, 0xFF003844, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFF9F9F9F, 0xFF1357C6, 0xFF3B3AE8, 0xFF6A23E6, 0xFF9217BA, 0xFFA51B70, 0xFFA32C1D, 0xFF8A4700,
    0xFF616200, 0xFF327A00, 0xFF0B8500, 0xFF00822D, 0xFF007080, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFFEBEBEB, 0xFF5CA0E9, 0xFF8682FF, 0xFFB56CFF, 0xFFDE61FF, 0xFFE864BA, 0xFFE87667, 0xFFD6901F,
    0xFFACAD00, 0xFF7CC500, 0xFF54CF2B, 0xFF3FCC76, 0xFF42BAC9, 0xFF484848, 0xFF000000, 0xFF000000,
    0xFFEBEBEB, 0xFFAFBEE9, 0xFFC2C1FF, 0xFFD6B5FF, 0xFFE7B1FF, 0xFFEB92D7, 0xFFEBB8B3, 0xFFE4C597,
    0xFFD2D388, 0xFFBEDC8B, 0xFFAFDA9C, 0xFFA8D9BC, 0xFFA9D2DB, 0xFFA8A8A8, 0xFF000000, 0xFF000000,
    0xFF5A4941, 0xFF001F6C, 0xFF110D83, 0xFF2F0080, 0xFF480063, 0xFF540032, 0xFF530300, 0xFF421400,
    0xFF282700, 0xFF093600, 0xFF003F00, 0xFF003C06, 0xFF002F3A, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFFB48D71, 0xFF124CA8, 0xFF3634CD, 0xFF611FCB, 0xFF8215A3, 0xFF911862, 0xFF8F2718, 0xFF783D00,
    0xFF555600, 0xFF326B00, 0xFF0F7400, 0xFF007127, 0xFF006171, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFFEBD2B6, 0xFF5490CE, 0xFF7974FF, 0xFFA25EFF, 0xFFC657FF, 0xFFD059A3, 0xFFD06858, 0xFFC1801B,
    0xFF9D9A00, 0xFF71AD00, 0xFF4CB524, 0xFF3AB265, 0xFF3CA5B3, 0xFF42423F, 0xFF000000, 0xFF000000,
    0xFFEBD2B6, 0xFFA1B6CE, 0xFFB1ADFF, 0xFFC2A3FF, 0xFFD19FFF, 0xFFD5A1C0, 0xFFD6A5A0, 0xFFCFB088,
    0xFFBDBA7A, 0xFFADC37D, 0xFFA1C68A, 0xFF99C5A4, 0xFF9AC0C6, 0xFF9A9A9A, 0xFF000000, 0xFF000000,
    0xFF374D45, 0xFF00226E, 0xFF0A0F85, 0xFF270083, 0xFF400065, 0xFF4D0033, 0xFF4C0400, 0xFF3D1500,
    0xFF252900, 0xFF093800, 0xFF004100, 0xFF003E07, 0xFF00313B, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFF799175, 0xFF0F4FC8, 0xFF3035CD, 0xFF5820CB, 0xFF7518A3, 0xFF821B63, 0xFF802919, 0xFF6A3F00,
    0xFF4B5600, 0xFF2C6D00, 0xFF0D7700, 0xFF007228, 0xFF006272, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFFB1D5AD, 0xFF4D93CE, 0xFF7177FF, 0xFF9962FF, 0xFFBA5AFF, 0xFFC45CB6, 0xFFC46B5A, 0xFFB3841C,
    0xFF929C00, 0xFF6AA900, 0xFF49B626, 0xFF37B367, 0xFF39A5B5, 0xFF3E4340, 0xFF000000, 0xFF000000,
    0xFFB1D5AD, 0xFF94B5CE, 0xFFA1ADFF, 0xFFB1A5FF, 0xFFBF9FFF, 0xFFC5A1C1, 0xFFC5A6A1, 0xFFBDAD89,
    0xFFB2BD7E, 0xFFA2C780, 0xFF96CA8B, 0xFF8FCA9E, 0xFF90C2C8, 0xFF909C9C, 0xFF000000, 0xFF000000,
    0xFF31342E, 0xFF001A60, 0xFF0A0B75, 0xFF220073, 0xFF3A0059, 0xFF46002D, 0xFF450300, 0xFF381200,
    0xFF232300, 0xFF083100, 0xFF003800, 0xFF003606, 0xFF002A33, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFF677764, 0xFF0D4396, 0xFF2C2CB5, 0xFF4F1BB3, 0xFF681290, 0xFF751556, 0xFF732314, 0xFF603600,
    0xFF454B00, 0xFF295D00, 0xFF0E6400, 0xFF006121, 0xFF005461, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFF9EBA9B, 0xFF437DB6, 0xFF6265D1, 0xFF8553D0, 0xFFA14BB0, 0xFFAB4E72, 0xFFAB592F, 0xFF9E6F14,
    0xFF818600, 0xFF5F9600, 0xFF419D1F, 0xFF349C58, 0xFF358F9D, 0xFF373A33, 0xFF000000, 0xFF000000,
    0xFF9EBA9B, 0xFF849BB6, 0xFF8E8ED1, 0xFF9C86D0, 0xFFA781B0, 0xFFAB83A9, 0xFFAB888C, 0xFFA49779,
    0xFF9B9F6D, 0xFF8EA56F, 0xFF83A97C, 0xFF7EA993, 0xFF7FA3AD, 0xFF7F8787, 0xFF000000, 0xFF000000,
];
