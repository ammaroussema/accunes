/*
    this is the cpu and ppu bus in one since they share a lot of interactions etc i guess.
    a lot of weird stuff and edge cases that i won't even pretend to understand i just followed reference code for some of these
*/

use crate::emulator::Emulator;

pub const PPU_BUS_DECAY_CONSTANT: i32 = 1786830;

impl Emulator {
    /// cpu fetch
    pub fn fetch(&mut self, address: u16) -> u8 {
        self.data_pins_are_not_floating = false;

        if address >= 0x8000 {
            // rom — go through mapper
            if self.cart.is_some() {
                let cart = self.cart.as_mut().unwrap();
                let mut mapper = std::mem::replace(&mut cart.mapper_chip, Box::new(crate::mapper::MapperNROM::new(crate::mapper::NromConfig::default())));
                let result = mapper.fetch_prg(cart, address);
                let cart = self.cart.as_mut().unwrap();
                cart.mapper_chip = mapper;
                self.data_pins_are_not_floating = result.driven;
                if result.driven {
                    self.data_bus = result.data;
                }
            }
        } else if address < 0x2000 {
            self.data_bus = self.ram[(address & 0x7FF) as usize];
            self.data_pins_are_not_floating = true;
        } else if address >= 0x2000 && address < 0x4000 {
            // ppu registers
            let reg = address & 0x2007;
            match reg {
                0x2000 => { self.data_bus = self.ppu_bus; }
                0x2001 => { self.data_bus = self.ppu_bus; }
                0x2002 => {
                    self.data_bus = if self.ppu_status_vblank { 0x80 } else { 0 };
                    self.ppu_read_2002 = true;
                    self.emulate_until_end_of_read();
                    self.data_bus |= ((if self.ppu_status_sprite_zero_hit_delayed { 0x40u8 } else { 0 })
                        | (if self.ppu_status_sprite_overflow_delayed { 0x20 } else { 0 }))
                        & 0xE0;
                    self.data_bus |= self.ppu_bus & 0x1F;
                    self.ppu_addr_latch = false;
                    self.ppu_bus = self.data_bus;
                    for i in 5..8 { self.ppu_bus_decay[i] = PPU_BUS_DECAY_CONSTANT; }
                }
                0x2003 => { self.data_bus = self.ppu_bus; }
                0x2004 => {
                    self.emulate_until_end_of_read();
                    self.data_bus = self.read_oam();
                    self.ppu_bus = self.data_bus;
                    for i in 0..8 { self.ppu_bus_decay[i] = PPU_BUS_DECAY_CONSTANT; }
                }
                0x2005 => { self.data_bus = self.ppu_bus; }
                0x2006 => { self.data_bus = self.ppu_bus; }
                0x2007 => {
                    if (self.ppu_v & 0x3FFF) >= 0x3F00 {
                        self.this_dot_read_from_palette_ram = true;
                        let mut pal_addr = self.ppu_v & 0x3F1F;
                        if (pal_addr & 3) == 0 { pal_addr &= 0x3F0F; }
                        let pal_val = self.palette_ram[(pal_addr & 0x1F) as usize];
                        let mask = if self.ppu_mask_greyscale { 0x30 } else { 0x3F };
                        self.data_bus = (pal_val & mask) | (self.ppu_bus & 0xC0);
                    } else {
                        self.data_bus = self.ppu_read_buffer;
                    }
                    self.ppu_bus = self.data_bus;
                    for i in 0..8 { self.ppu_bus_decay[i] = PPU_BUS_DECAY_CONSTANT; }
                    self.emulate_until_end_of_read();
                    self.ppu_2007_read_sr = true;
                    self.ppu_2007_read = true;
                }
                _ => {}
            }
            self.data_pins_are_not_floating = true;
        } else if self.cart.is_some() {
            // $4000-$401F: apu/io registers, and mapper space
            let cart = self.cart.as_mut().unwrap();
            let mut mapper = std::mem::replace(&mut cart.mapper_chip, Box::new(crate::mapper::MapperNROM::new(crate::mapper::NromConfig::default())));
            let result = mapper.fetch_prg(cart, address);
            let cart = self.cart.as_mut().unwrap();
            cart.mapper_chip = mapper;
            self.data_pins_are_not_floating = result.driven;
            if result.driven {
                self.data_bus = result.data;
            }
            if cart.memory_mapper == 20 && (address == 0x4030 || address == 0x4031 || address == 0x4032) {
                self.irq_level_detector = false;
            }
            if matches!(cart.memory_mapper, 5) && address == 0x5204 {
                self.irq_level_detector = false;
            }
            if matches!(cart.memory_mapper, 303 | 304) && address == 0x4030 {
                self.irq_level_detector = false;
            }
        }

        // apu register reads ($4015, $4016, $4017)
        if self.address_bus >= 0x4000 && self.address_bus <= 0x401F {
            let reg = (address & 0x1F) as u8;
            if reg == 0x15 {
                self.internal_bus &= 0x20;
                self.internal_bus |= if self.apu_status_dmc_interrupt { 0x80 } else { 0 };
                self.internal_bus |= if self.apu_status_frame_interrupt { 0x40 } else { 0 };
                self.internal_bus |= if self.apu_dmc_bytes_remaining != 0 && self.apu_status_delayed_dmc { 0x10 } else { 0 };
                self.internal_bus |= if self.apu_length_counter_noise != 0 { 0x08 } else { 0 };
                self.internal_bus |= if self.apu_length_counter_triangle != 0 { 0x04 } else { 0 };
                self.internal_bus |= if self.apu_length_counter_pulse2 != 0 { 0x02 } else { 0 };
                self.internal_bus |= if self.apu_length_counter_pulse1 != 0 { 0x01 } else { 0 };
                self.clearing_apu_frame_interrupt = true;
                return self.internal_bus; // $4015 read does not affect the external data bus
            } else if reg == 0x16 || reg == 0x17 {
                // four score: if either port has fourscore, use the extended 4-player controls
                if self.controller1_type == crate::config::ControllerType::FourScore || self.controller2_type == crate::config::ControllerType::FourScore {
                    let w = (reg == 0x17) as usize;
                    let readbit = self.fourscore_readbit[w];
                    let shift = 7 - (readbit & 7);
                    let ret = if readbit >= 8 {
                        (if w == 0 { self.controller_port3 } else { self.controller_port4 } >> shift) & 1
                    } else {
                        (if w == 0 { self.controller_port1 } else { self.controller_port2 } >> shift) & 1
                    };
                    let mut val = if readbit >= 16 { 0 } else { ret };
                    if readbit == if w == 0 { 19 } else { 18 } { val = 1; }
                    self.fourscore_readbit[w] += 1;
                    self.apu_controller_ports_strobed = false;
                    let fs_byte = (val as u8) | (self.data_bus & 0xFE);
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = fs_byte;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // arkanoid paddle on port 2 ($4017)
                if reg == 0x17 && self.controller2_type == crate::config::ControllerType::Paddle {
                    let idx = 1usize;
                    let mut paddle_val = 0u8;
                    if self.paddle_readbit[idx] < 8 {
                        let bit = (self.paddle_x[idx] >> (7 - self.paddle_readbit[idx])) & 1;
                        paddle_val |= bit << 4;
                        self.paddle_readbit[idx] += 1;
                    } else {
                        paddle_val |= 1 << 4;
                    }
                    if self.paddle_button[idx] {
                        paddle_val |= 1 << 3;
                    }
                    paddle_val |= self.data_bus & 0xE7;
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = paddle_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // zapper on port 2: bypass shift register mechanism
                if reg == 0x17 && self.controller2_type == crate::config::ControllerType::Zapper {
                    let mut zapper_val = 0u8;
                    if self.zapper_trigger {
                        zapper_val |= 0x10;
                    }
                    if !self.zapper_check_hit() {
                        zapper_val |= 0x08;
                    }
                    let zapper_read = zapper_val | (self.data_bus & 0xE0);

                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = zapper_read;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }

                // snes pad bypass (16-bit shift register on d0)
                let ctype = if reg == 0x16 { self.controller1_type } else { self.controller2_type };
                if ctype == crate::config::ControllerType::SNESPad {
                    let idx = (reg == 0x17) as usize;
                    let snes_val = if self.snes_readbit[idx] < 16 {
                        (self.snes_state[idx] >> self.snes_readbit[idx]) & 1
                    } else {
                        1
                    };
                    self.snes_readbit[idx] += 1;
                    self.apu_controller_ports_strobed = false;
                    let snes_byte = (snes_val as u8) | (self.data_bus & 0xFE);
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = snes_byte;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                if ctype == crate::config::ControllerType::SNESMouse {
                    let idx = (reg == 0x17) as usize;
                    let mouse_val = if self.snes_mouse_readbit[idx] < 32 {
                        (self.snes_mouse_state[idx] >> self.snes_mouse_readbit[idx]) & 1
                    } else {
                        1
                    };
                    self.snes_mouse_readbit[idx] += 1;
                    self.apu_controller_ports_strobed = false;
                    let mouse_byte = (mouse_val as u8) | (self.data_bus & 0xFE);
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = mouse_byte;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                if ctype == crate::config::ControllerType::SuborMouse {
                    let idx = (reg == 0x17) as usize;
                    let subor_val = self.subor_mouse_latch[idx] & 1;
                    self.subor_mouse_latch[idx] = (self.subor_mouse_latch[idx] >> 1) | 0x80;
                    self.apu_controller_ports_strobed = false;
                    let subor_byte = subor_val | (self.data_bus & 0xFE);
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = subor_byte;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }

                if self.apu_controller_ports_strobing {
                    self.controller_shift_register1 = self.controller_port1;
                    self.controller_shift_register2 = self.controller_port2;
                }

                let sr_bit = if reg == 0x16 {
                    let bit = self.controller_shift_register1 & 0x80;
                    self.controller1_shift_counter = 2;
                    bit
                } else {
                    let bit = self.controller_shift_register2 & 0x80;
                    self.controller2_shift_counter = 2;
                    bit
                };

                self.apu_controller_ports_strobed = false;
                let idx = (reg == 0x17) as usize;
                let is_pp = ctype == crate::config::ControllerType::PowerPadA || ctype == crate::config::ControllerType::PowerPadB;
                let pp_d3 = if is_pp { (self.powerpad_shift_d3[idx] & 0x80) >> 3 } else { 0 };
                let pp_d4 = if is_pp { (self.powerpad_shift_d4[idx] & 0x80) >> 2 } else { 0 };
                let controller_read = (if sr_bit == 0 { 0u8 } else { 1 }) | pp_d3 | pp_d4 | (self.data_bus & 0xE0);

                // vs system quirks: let mapper adjust controller read if applicable
                let adjusted = if let Some(cart) = self.cart.as_ref() {
                    if cart.is_vs_system {
                        cart.mapper_chip.adjust_controller_read(address, controller_read)
                    } else {
                        controller_read
                    }
                } else {
                    controller_read
                };

                if self.do_oam_dma && self.data_pins_are_not_floating {
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                self.data_bus = adjusted;
                self.internal_bus = self.data_bus;
                return self.data_bus;
            }
        }

        self.internal_bus = self.data_bus;
        self.data_bus
    }

    /// cpu store
    pub fn store(&mut self, input: u8, address: u16) {
        self.data_bus = input;
        if let Some(cart) = self.cart.as_mut() {
            cart.mapper_chip.handle_cpu_write(address, input);
        }
        if address < 0x2000 {
            self.ram[(address & 0x7FF) as usize] = input;
        } else if address < 0x4000 {
            self.store_ppu_registers(address, input);
        } else if address >= 0x4000 && address <= 0x4015 {
            self.store_apu_registers(address, input);
        } else if address == 0x4016 {
            self.apu_controller_ports_strobing = (input & 1) != 0;
            if (input & 1) != 0 {
                self.paddle_readbit[0] = 0;
                self.paddle_readbit[1] = 0;
                self.snes_readbit[0] = 0;
                self.snes_readbit[1] = 0;
                self.snes_mouse_readbit[0] = 0;
                self.snes_mouse_readbit[1] = 0;
                self.fourscore_readbit[0] = 0;
                self.fourscore_readbit[1] = 0;
                // latch accumulated mouse deltas into state
                for p in 0..2usize {
                    let dx = self.snes_mouse_delta_x[p].round() as i16;
                    let dy = self.snes_mouse_delta_y[p].round() as i16;
                    let cx = dx.clamp(-128, 127);
                    let cy = dy.clamp(-128, 127);
                    self.snes_mouse_state[p] =
                        (if self.snes_mouse_buttons[p] & 1 != 0 { 0 } else { 1 })
                        | (if self.snes_mouse_buttons[p] & 2 != 0 { 0 } else { 1 } << 1)
                        | (((cx as i16 + 128) as u32) << 8)
                        | (((cy as i16 + 128) as u32) << 16);
                    self.snes_mouse_delta_x[p] = 0.0;
                    self.snes_mouse_delta_y[p] = 0.0;
                }
                // subor mouse: build latch from accumulated deltas (inertia)
                for p in 0..2usize {
                    let mut latch = self.subor_mouse_buttons[p] & 0x03;
                    let dx = self.subor_mouse_dx[p];
                    let dy = self.subor_mouse_dy[p];
                    if dx > 0 { latch |= 0x08; self.subor_mouse_dx[p] -= 1; }
                    else if dx < 0 { latch |= 0x0C; self.subor_mouse_dx[p] += 1; }
                    if dy > 0 { latch |= 0x20; self.subor_mouse_dy[p] -= 1; }
                    else if dy < 0 { latch |= 0x30; self.subor_mouse_dy[p] += 1; }
                    self.subor_mouse_latch[p] = latch;
                }
            }
            if self.cart.as_ref().is_some_and(|c| c.memory_mapper == 99) {
                let cart = self.cart.as_mut().unwrap();
                let mut mapper = std::mem::replace(&mut cart.mapper_chip, Box::new(crate::mapper::MapperNROM::new(crate::mapper::NromConfig::default())));
                mapper.store_prg(cart, address, input);
                self.cart.as_mut().unwrap().mapper_chip = mapper;
            }
        } else if address == 0x4017 {
            self.apu_frame_counter_mode = (input & 0x80) != 0;
            self.apu_frame_counter_inhibit_irq = (input & 0x40) != 0;
            if self.apu_frame_counter_mode {
                self.apu_half_frame_clock = true;
                self.apu_quarter_frame_clock = true;
            }
            if self.apu_frame_counter_inhibit_irq {
                self.apu_status_frame_interrupt = false;
                self.irq_level_detector = false;
            }
            self.apu_frame_counter_reset = if self.apu_put_cycle { 3 } else { 4 };
        } else if address >= 0x4020 && self.cart.is_some() {
            let cart = self.cart.as_mut().unwrap();
            cart.mapper_cpu_cycle = self.total_cycles as i64;
            let mut mapper = std::mem::replace(&mut cart.mapper_chip, Box::new(crate::mapper::MapperNROM::new(crate::mapper::NromConfig::default())));
            mapper.store_prg(cart, address, input);
            let irq_ack = mapper.take_irq_ack();
            let cart = self.cart.as_mut().unwrap();
            cart.mapper_chip = mapper;

            if irq_ack {
                self.irq_level_detector = false;
            }

            if cart.memory_mapper == 20 && (address >= 0x4022 && address <= 0x4025) {
                self.irq_level_detector = false;
            }

              if (address & 0xE001) == 0xE000 && matches!(cart.memory_mapper, 4 | 12 | 37 | 44 | 45 | 47 | 49 | 52 | 64 | 74 | 100 | 114 | 115 | 116 | 118 | 119 | 121 | 123 | 126 | 131 | 134 | 142 | 165 | 169 | 182 | 187 | 189 | 191 | 192 | 194 | 195 | 196 | 197 | 198 | 199 | 205 | 208 | 215 | 219 | 224 | 238 | 245 | 248 | 249 | 254 | 256 | 259 | 260 | 262 | 263 | 267 | 268 | 269 | 287 | 291 | 292 | 296 | 307 | 422 | 455 | 531 | 534) {
                self.irq_level_detector = false;
            } else if cart.memory_mapper == 5 && address == 0x5204 {
                self.irq_level_detector = false;
            } else if cart.memory_mapper == 298 {
                let decoded = (address & 0xF003) | ((address & 0x000C) >> 2);
                match decoded & 0xF003 {
                    0xF001 | 0xF003 => {
                        self.irq_level_detector = false;
                    }
                    _ => {}
                }
            } else if (cart.memory_mapper == 6 || cart.memory_mapper == 17)
                && (address >= 0x4501 && address <= 0x4503)
            {
                self.irq_level_detector = false;
            } else if cart.memory_mapper == 65 && (address == 0x9003 || address == 0x9004) {
                self.irq_level_detector = false;
            } else if cart.memory_mapper == 67 && (address & 0xF800) == 0xD800 {
                self.irq_level_detector = false;
            } else if cart.memory_mapper == 73 && (address & 0xF000 == 0xC000 || address & 0xF000 == 0xD000) {
                self.irq_level_detector = false;
            } else if (cart.memory_mapper == 82 || cart.memory_mapper == 552) && address == 0x7EFF {
                self.irq_level_detector = false;
            } else if (cart.memory_mapper == 83 || cart.memory_mapper == 264) && address >= 0x8000 {
                let addr = if cart.memory_mapper == 264 {
                    (address >> 2 & 0x3FC0) | (address & 0x003F)
                } else {
                    address
                };
                let reg = (addr >> 8) & 3;
                let index = addr & 0x1F;
                if reg == 2 && index & 1 == 0 {
                    self.irq_level_detector = false;
                }
            } else if cart.memory_mapper == 85 && address >= 0xF000 {
                self.irq_level_detector = false;
            } else if matches!(cart.memory_mapper, 35 | 90 | 209 | 211 | 281 | 282 | 295 | 358 | 386 | 387 | 388 | 397) && address >= 0xC000 && address < 0xD000 {
                self.irq_level_detector = false;
            } else if cart.memory_mapper == 91 && address >= 0x7000 && address < 0x8000 {
                let reg = if cart.sub_mapper == 1 { address & 7 } else { address & 3 };
                if reg == 2 {
                    self.irq_level_detector = false;
                }
            } else if matches!(cart.memory_mapper, 102 | 284) && address >= 0x8000 && address < 0xC000 && (address & 0xF) == 0x9 {
                self.irq_level_detector = false;
            } else if cart.memory_mapper == 117 && (address & 0xE000) == 0xC000 {
                self.irq_level_detector = false;
            }
        }

        self.data_bus = input;
    }

    /// store to apu registers $4000-$4015
    fn store_apu_registers(&mut self, address: u16, input: u8) {
        match address {
            0x4001 => {
                self.apu_register[1] = input;
                self.pulse1_sweep_reload = true;
            }
            0x4003 => {
                self.apu_register[3] = input;
                if self.apu_status_pulse1 {
                    self.apu_length_counter_reload_value_pulse1 = APU_LENGTH_COUNTER_LUT[(input >> 3) as usize];
                    self.apu_length_counter_reload_pulse1 = true;
                }
                self.apu_channel_timer_pulse1 |= ((input & 0x7) as u16) << 8;
                self.pulse1_envelope_start_flag = true;
                self.pulse1_sequencer_step = 0;
            }
            0x4005 => {
                self.apu_register[5] = input;
                self.pulse2_sweep_reload = true;
            }
            0x4007 => {
                self.apu_register[7] = input;
                if self.apu_status_pulse2 {
                    self.apu_length_counter_reload_value_pulse2 = APU_LENGTH_COUNTER_LUT[(input >> 3) as usize];
                    self.apu_length_counter_reload_pulse2 = true;
                }
                self.apu_channel_timer_pulse2 |= ((input & 0x7) as u16) << 8;
                self.pulse2_envelope_start_flag = true;
                self.pulse2_sequencer_step = 0;
            }
            0x400B => {
                self.apu_register[0xB] = input;
                if self.apu_status_triangle {
                    self.apu_length_counter_reload_value_triangle = APU_LENGTH_COUNTER_LUT[(input >> 3) as usize];
                    self.apu_length_counter_reload_triangle = true;
                }
                self.apu_channel_timer_triangle |= ((input & 0x7) as u16) << 8;
                self.triangle_linear_counter_reload_flag = true;
            }
            0x400F => {
                self.apu_register[0xF] = input;
                if self.apu_status_noise {
                    self.apu_length_counter_reload_value_noise = APU_LENGTH_COUNTER_LUT[(input >> 3) as usize];
                    self.apu_length_counter_reload_noise = true;
                }
                self.noise_envelope_start_flag = true;
            }

            0x4010 => {
                self.apu_dmc_enable_irq = (input & 0x80) != 0;
                self.apu_dmc_loop = (input & 0x40) != 0;
                self.apu_dmc_rate = if self.is_pal() { APU_DMC_RATE_LUT_PAL } else { APU_DMC_RATE_LUT_NTSC }[(input & 0xF) as usize];
                if !self.apu_dmc_enable_irq {
                    self.apu_status_dmc_interrupt = false;
                    self.irq_level_detector = false;
                }
            }
            0x4011 => { self.apu_dmc_output = input & 0x7F; }
            0x4012 => { self.apu_dmc_sample_address = 0xC000 | ((input as u16) << 6); }
            0x4013 => { self.apu_dmc_sample_length = ((input as u16) << 4) | 1; }
            0x4014 => {
                self.do_oam_dma = true;
                self.first_cycle_of_oam_dma = true;
                self.dma_address = 0;
                self.dma_page = input;
            }
            0x4015 => {
                self.apu_status_delayed_dmc = (input & 0x10) != 0;
                self.apu_status_noise = (input & 0x08) != 0;
                self.apu_status_triangle = (input & 0x04) != 0;
                self.apu_status_pulse2 = (input & 0x02) != 0;
                self.apu_status_pulse1 = (input & 0x01) != 0;

                self.apu_delayed_dmc_4015 = if self.apu_put_cycle { 3 } else { 4 };

                if self.apu_status_delayed_dmc && self.apu_dmc_bytes_remaining == 0 {
                    self.start_dmc_sample();
                    if self.apu_silent {
                        self.dmc_dma_delay = 2;
                    }
                }

                if !self.apu_status_noise { self.apu_length_counter_noise = 0; }
                if !self.apu_status_triangle { self.apu_length_counter_triangle = 0; }
                if !self.apu_status_pulse2 { self.apu_length_counter_pulse2 = 0; }
                if !self.apu_status_pulse1 { self.apu_length_counter_pulse1 = 0; }
                self.apu_status_dmc_interrupt = false;
                self.irq_level_detector = false;

                // dma explicit abort
                if !self.apu_status_delayed_dmc
                    && ((self.apu_channel_timer_dmc == 2 && !self.apu_put_cycle)
                        || (self.apu_channel_timer_dmc == self.apu_dmc_rate && self.apu_put_cycle))
                {
                    self.apu_delayed_dmc_4015 = if self.apu_put_cycle { 5 } else { 6 };
                }

                // dma implicit abort
                if self.apu_status_delayed_dmc
                    && ((self.apu_channel_timer_dmc == 10 && !self.apu_put_cycle)
                        || (self.apu_channel_timer_dmc == 8 && self.apu_put_cycle))
                {
                    self.apu_set_implicit_abort_dmc_4015 = true;
                }
            }
            _ => {
                self.apu_register[(address & 0xFF) as usize] = input;
            }
        }
    }

    fn read_oam(&self) -> u8 {
        if (self.ppu_mask_show_background || self.ppu_mask_show_sprites) && self.ppu_scanline < 240 {
            self.ppu_oam_buffer
        } else {
            self.oam[self.ppu_oam_address as usize]
        }
    }

    pub fn emulate_until_end_of_read(&mut self) {
        for _ in 0..7 {
            self.emulator_core();
        }
    }

    pub fn emulate_n_master_clock_cycles(&mut self, n: usize) {
        for _ in 0..n {
            self.emulator_core();
        }
    }

    /// ppu nametable address mirroring
    pub fn ppu_address_with_mirroring(&self, mut address: u16) -> u16 {
        if address < 0x2000 { return address; }
        if address >= 0x3F00 {
            address &= 0x3F1F;
            if (address & 3) == 0 { address &= 0x3F0F; }
            return address;
        }
        address &= 0x2FFF;
        if let Some(cart) = self.cart.as_ref() {
            cart.mapper_chip.mirror_nametable(cart, address)
        } else {
            address
        }
    }

    pub fn store_ppu_data(&mut self, address: u16, input: u8) {
        let address = address & 0x3FFF;
        if address < 0x3F00 {
            if let Some(cart) = self.cart.as_mut() {
                let mut mapper = std::mem::replace(&mut cart.mapper_chip, Box::new(crate::mapper::MapperNROM::new(crate::mapper::NromConfig::default())));
                mapper.store_ppu(cart, address, input, &mut self.vram);
                cart.mapper_chip = mapper;
            }
        } else {
            // palette RAM
            let mirrored = self.ppu_address_with_mirroring(address);
            let pal_addr = (mirrored & 0x1F) as usize;
            self.palette_ram[pal_addr] = input;
            
            // palette mirrors
            if (pal_addr & 0x03) == 0 {
                self.palette_ram[pal_addr ^ 0x10] = input;
            }
        }
    }
}

pub static APU_LENGTH_COUNTER_LUT: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14,
    12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
];

pub static APU_DMC_RATE_LUT_NTSC: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

pub static APU_DMC_RATE_LUT_PAL: [u16; 16] = [
    398, 354, 316, 298, 266, 236, 210, 199, 177, 149, 132, 119, 99, 78, 67, 50,
];
