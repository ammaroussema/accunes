/*
    this is the cpu and ppu bus in one since they share a lot of interactions etc i guess.
    a lot of weird stuff and edge cases that i won't even pretend to understand i just followed reference code for some of these
*/

use crate::emulator::Emulator;
use std::sync::atomic::Ordering;

pub const PPU_BUS_DECAY_CONSTANT: i32 = 1786830;

impl Emulator {
    /// cpu fetch
    pub fn fetch(&mut self, address: u16) -> u8 {
        self.data_pins_are_not_floating = false;

        if let Some(cart) = self.cart.as_mut() {
            cart.mapper_chip.handle_cpu_read(address);
        }

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
            if let Some(ref cart) = self.cart {
                if let Some(data) = cart.mapper_chip.cpu_ram_override(address) {
                    self.data_bus = data;
                    self.data_pins_are_not_floating = true;
                    return data;
                }
            }
            self.data_bus = self.ram[(address & self.cpu_ram_mask) as usize];
            self.data_pins_are_not_floating = true;        } else if address >= 0x2000 && address < 0x4000 {
            let vt369 = self
                .cart
                .as_ref()
                .map_or(false, |c| c.mapper_chip.onebus_vt369_ppu());
            if vt369 && address >= 0x3000 {
                let ppu_addr = 0x2000 | (address & 0x0FFF);
                let byte = if let Some(cart) = self.cart.as_mut() {
                    let mut mapper = std::mem::replace(&mut cart.mapper_chip, Box::new(crate::mapper::MapperNROM::new(crate::mapper::NromConfig::default())));
                    let (data, _) = mapper.fetch_ppu(
                        &cart.prg_rom,
                        &cart.chr_rom,
                        &cart.prg_ram,
                        &cart.chr_ram,
                        &cart.prg_vram,
                        cart.using_chr_ram,
                        cart.nametable_horizontal_mirroring,
                        cart.alternative_nametable_arrangement,
                        ppu_addr,
                        0,
                        &self.vram,
                    );
                    cart.mapper_chip = mapper;
                    data
                } else {
                    0
                };
                self.data_pins_are_not_floating = true;
                self.data_bus = byte;
            } else {
                let is_onebus = self
                    .cart
                    .as_ref()
                    .map_or(false, |c| crate::mappers::one_bus::is_onebus_mapper(c.memory_mapper));
                if is_onebus && address >= 0x2010 {
                    let cart = self.cart.as_mut().unwrap();
                    let mut mapper = std::mem::replace(&mut cart.mapper_chip, Box::new(crate::mapper::MapperNROM::new(crate::mapper::NromConfig::default())));
                    let result = mapper.fetch_prg(cart, address);
                    let cart = self.cart.as_mut().unwrap();
                    cart.mapper_chip = mapper;
                    self.data_pins_are_not_floating = result.driven;
                    if result.driven {
                        self.data_bus = result.data;
                    }
                } else {
                if self.cart.as_ref().map_or(false, |c| c.mapper_chip.is_um6578()) && address == 0x2008 {
                    self.data_bus = self.um6578_reg2008;
                    self.data_pins_are_not_floating = true;
                    return self.data_bus;
                }
                if self.cart.as_ref().map_or(false, |c| c.mapper_chip.is_um6578())
                    && (0x2040..=0x207F).contains(&address)
                {
                    self.data_bus = self.palette_ram[(address & 0x3F) as usize];
                    self.data_pins_are_not_floating = true;
                    return self.data_bus;
                }
                // ppu registers
                let reg = if self.cart.as_ref().map_or(false, |c| c.mapper_chip.is_um6578()) {
                    if address > 0x2007 {
                        self.data_bus = self.ppu_bus;
                        self.data_pins_are_not_floating = true;
                        return self.data_bus;
                    }
                    address
                } else {
                    address & 0x2007
                };
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
                        if !self.cart.as_ref().map_or(false, |c| c.mapper_chip.is_um6578())
                            && (self.ppu_v & 0x3FFF) >= 0x3F00 {
                            self.this_dot_read_from_palette_ram = true;
                            let is_onebus = self
                                .cart
                                .as_ref()
                                .map_or(false, |c| {
                                    crate::mappers::one_bus::is_onebus_mapper(c.memory_mapper)
                                });
                            let vt369_enhanced = self
                                .cart
                                .as_ref()
                                .map_or(false, |c| c.mapper_chip.onebus_vt369_enhanced_ppu());
                            let pal_addr = if vt369_enhanced {
                                (self.ppu_v & 0x3FF) as usize
                            } else if is_onebus {
                                (self.ppu_v & 0xFF) as usize
                            } else {
                                let mut pal_addr = self.ppu_v & 0x3F1F;
                                if (pal_addr & 3) == 0 { pal_addr &= 0x3F0F; }
                                (pal_addr & 0x1F) as usize
                            };
                            let pal_val = self.palette_ram[pal_addr];
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
            }
        }
        } else if address >= 0x5000 && address < 0x6000 && self.cart.as_ref().map_or(false, |c| c.mapper_chip.is_um6578()) {
            if address < 0x5800 {
                self.data_bus = self.um6578_extra_ram[(address & 0x7FF) as usize];
            } else {
                self.data_bus = 0xFF;
            }
            self.data_pins_are_not_floating = true;
            return self.data_bus;
        } else if self.cart.is_some() {
            if self.cart.as_ref().map_or(false, |c| c.mapper_chip.is_um6578()) && (0x4048..=0x404F).contains(&address) {
                let data = match address {
                    0x4048 => self.um6578_dma_control | if self.um6578_dma_busy != 0 { 0x80 } else { 0 },
                    0x4049 => self.um6578_dma_page,
                    0x404A => (self.um6578_dma_source & 0xFF) as u8,
                    0x404B => ((self.um6578_dma_source >> 8) & 0xFF) as u8,
                    0x404C => (self.um6578_dma_target & 0xFF) as u8,
                    0x404D => ((self.um6578_dma_target >> 8) & 0xFF) as u8,
                    0x404E => (self.um6578_dma_length & 0xFF) as u8,
                    0x404F => ((self.um6578_dma_length >> 8) & 0xFF) as u8,
                    _ => 0,
                };
                self.data_bus = data;
                self.data_pins_are_not_floating = true;
                return data;
            }
            if self.cart.as_ref().map_or(false, |c| c.mapper_chip.is_um6578()) && address == 0x4020 {
                self.data_bus = 0xFF;
                self.data_pins_are_not_floating = true;
                return 0xFF;
            }
            if self.cart.as_ref().map_or(false, |c| c.mapper_chip.is_um6578()) && (0x4200..=0x421F).contains(&address) {
                self.data_bus = 0xFF;
                self.data_pins_are_not_floating = true;
                return 0xFF;
            }
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
            if matches!(cart.memory_mapper, 469) && address == 0x5030 {
                self.irq_level_detector = false;
            }
            if matches!(cart.memory_mapper, 767) && address == 0x50C0 {
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
                // oeka kids tablet
                if reg == 0x17 && self.expansion_type == crate::config::ExpansionType::OekaKidsTablet {
                    let oeka_val = self.expansion_oeka_bits(reg) | (self.data_bus & 0xE0);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = oeka_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // family trainer
                if reg == 0x17 && self.expansion_type.is_family_trainer() {
                    let ft_val = self.expansion_family_trainer_bits() | (self.data_bus & 0xE0);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = ft_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // konami hyper shot
                if reg == 0x17 && self.expansion_type.is_hyper_shot() {
                    let hs_val = self.expansion_hyper_shot_bits() | (self.data_bus & 0xE0);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = hs_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // family basic keyboard
                if reg == 0x17 && self.expansion_type.is_family_basic() {
                    let fb_val = self.expansion_family_basic_bits() | (self.data_bus & 0xE0);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = fb_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // party tap
                if reg == 0x17 && self.expansion_type.is_party_tap() {
                    if self.party_tap_strobe {
                        self.refresh_party_tap_buffer();
                    }
                    let val = if self.party_tap_read_count < 2 {
                        let v = (self.party_tap_buffer & 0x07) << 2;
                        self.party_tap_buffer >>= 3;
                        self.party_tap_read_count += 1;
                        v
                    } else {
                        0x14
                    };
                    let pt_val = val | (self.data_bus & 0xE0);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = pt_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // pachinko controller
                if reg == 0x16 && self.expansion_type.is_pachinko() {
                    if self.pachinko_strobe {
                        self.refresh_pachinko_buffer();
                    }
                    let bit = (self.pachinko_buffer & 1) as u8;
                    let val = bit << 1;
                    self.pachinko_buffer >>= 1;
                    let pach_val = (val & 0x02) | (self.data_bus & 0xFD);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = pach_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // four score: if either port has fourscore, use the extended 4-player controls
                if self.controller1_type == crate::config::ControllerType::FourScore || self.controller2_type == crate::config::ControllerType::FourScore {
                    let w = (reg == 0x17) as usize;
                    let readbit = self.fourscore_readbit[w];
                    let shift = 7 - (readbit & 7);
                    let ret = if readbit >= 8 {
                        (if w == 0 { self.controller_port3.load(Ordering::Relaxed) } else { self.controller_port4.load(Ordering::Relaxed) } >> shift) & 1
                    } else {
                        (if w == 0 { self.controller_port1.load(Ordering::Relaxed) } else { self.controller_port2.load(Ordering::Relaxed) } >> shift) & 1
                    };
                    let mut val = if readbit >= 16 { 0 } else { ret };
                    if readbit == if w == 0 { 19 } else { 18 } { val = 1; }
                    self.fourscore_readbit[w] = self.fourscore_readbit[w].wrapping_add(1);
                    self.apu_controller_ports_strobed = false;
                    let fs_byte = (val as u8) | (self.data_bus & 0xFE) | self.famicom_mic_bit(reg) | self.expansion_zapper_bits(reg) | self.expansion_paddle_bit(reg) | self.expansion_oeka_bits(reg);
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = fs_byte;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // famicom 2-player expansion adapter
                if self.expansion_adapter_type == crate::config::ExpansionAdapterType::TwoPlayer {
                    if self.apu_controller_ports_strobing {
                        self.controller_shift_register1 = self.expansion_adapter_ports[0].load(Ordering::Relaxed);
                        self.controller_shift_register2 = self.expansion_adapter_ports[1].load(Ordering::Relaxed);
                    }
                    let sr = if reg == 0x16 {
                        self.controller_shift_register1
                    } else {
                        self.controller_shift_register2
                    };
                    let bit1 = ((sr >> 7) & 1) << 1;
                    if reg == 0x16 {
                        self.controller1_shift_counter = 2;
                    } else {
                        self.controller2_shift_counter = 2;
                    }
                    self.apu_controller_ports_strobed = false;
                    let val = bit1 | (self.data_bus & 0xFD);
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // famicom 4-player adapter
                if self.expansion_adapter_type == crate::config::ExpansionAdapterType::FourPlayer {
                    let w = (reg - 0x16) as usize;
                    let readbit = self.fourscore_readbit[w];

                    if readbit < 8 {
                        let sr_idx = w; 
                        let bit0 = (self.expansion_adapter_shift_register[sr_idx] >> 7) & 1;
                        self.expansion_adapter_shift_register[sr_idx] = (self.expansion_adapter_shift_register[sr_idx] << 1) | 1;
                        self.fourscore_readbit[w] = readbit.wrapping_add(1);
                        self.apu_controller_ports_strobed = false;
                        let val = (bit0 << 1) | (self.data_bus & 0xFD);
                        if self.do_oam_dma && self.data_pins_are_not_floating {
                            self.internal_bus = self.data_bus;
                            return self.data_bus;
                        }
                        self.data_bus = val;
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    } else if readbit < 16 {
                        let sr_idx = w + 2; 
                        let bit0 = (self.expansion_adapter_shift_register[sr_idx] >> 7) & 1;
                        self.expansion_adapter_shift_register[sr_idx] = (self.expansion_adapter_shift_register[sr_idx] << 1) | 1;
                        self.fourscore_readbit[w] = readbit.wrapping_add(1);
                        self.apu_controller_ports_strobed = false;
                        let val = (bit0 << 1) | (self.data_bus & 0xFD);
                        if self.do_oam_dma && self.data_pins_are_not_floating {
                            self.internal_bus = self.data_bus;
                            return self.data_bus;
                        }
                        self.data_bus = val;
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    } else {
                        let sig_val = if w == 0 { 0x04u8 } else { 0x08u8 };
                        let sig_bit = ((sig_val >> (readbit - 16)) & 1) << 1;
                        self.fourscore_readbit[w] = readbit.wrapping_add(1);
                        self.apu_controller_ports_strobed = false;
                        let val = sig_bit | (self.data_bus & 0xFD);
                        if self.do_oam_dma && self.data_pins_are_not_floating {
                            self.internal_bus = self.data_bus;
                            return self.data_bus;
                        }
                        self.data_bus = val;
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                }
                // arkanoid paddle on port 1 ($4016)
                if reg == 0x16 && self.controller1_type == crate::config::ControllerType::Paddle {
                    let idx = 0usize;
                    let mut paddle_val = 0u8;
                    let px = self.paddle_x.lock().unwrap();
                    let pb = self.paddle_button.lock().unwrap();
                    if self.paddle_readbit[idx] < 8 {
                        let bit = (px[idx] >> (7 - self.paddle_readbit[idx])) & 1;
                        paddle_val |= bit << 4;
                        self.paddle_readbit[idx] += 1;
                    } else {
                        paddle_val |= 1 << 4;
                    }
                    if pb[idx] { paddle_val |= 1 << 3; }
                    drop(px); drop(pb);
                    paddle_val |= self.data_bus & 0xE7;
                    paddle_val |= self.famicom_mic_bit(reg);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = paddle_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // arkanoid paddle on port 2 ($4017)
                if reg == 0x17 && self.controller2_type == crate::config::ControllerType::Paddle {
                    let idx = 1usize;
                    let mut paddle_val = 0u8;
                    let px = self.paddle_x.lock().unwrap();
                    let pb = self.paddle_button.lock().unwrap();
                    if self.paddle_readbit[idx] < 8 {
                        let bit = (px[idx] >> (7 - self.paddle_readbit[idx])) & 1;
                        paddle_val |= bit << 4;
                        self.paddle_readbit[idx] += 1;
                    } else {
                        paddle_val |= 1 << 4;
                    }
                    if pb[idx] { paddle_val |= 1 << 3; }
                    drop(px); drop(pb);
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
                    if self.zapper_trigger.load(Ordering::Relaxed) {
                        zapper_val |= 0x10;
                    }
                    if !self.zapper_check_hit() {
                        zapper_val |= 0x08;
                    }
                    zapper_val |= self.expansion_zapper_bits(reg);
                    zapper_val |= self.expansion_oeka_bits(reg);
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
                    let snes_state_lock = self.snes_state.lock().unwrap();
                    let snes_val = if self.snes_readbit[idx] < 16 {
                        (snes_state_lock[idx] >> self.snes_readbit[idx]) & 1
                    } else {
                        1
                    };
                    drop(snes_state_lock);
                    self.snes_readbit[idx] = self.snes_readbit[idx].wrapping_add(1);
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
                    self.snes_mouse_readbit[idx] = self.snes_mouse_readbit[idx].wrapping_add(1);
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
                if ctype == crate::config::ControllerType::VirtualBoy {
                    let idx = (reg == 0x17) as usize;
                    if self.apu_controller_ports_strobing {
                        self.virtualboy_readbit[idx] = 0;
                        let vb_state_lock = self.virtualboy_state.lock().unwrap();
                        self.virtualboy_state_buffer[idx] = crate::config::build_vb_state(vb_state_lock[idx]);
                        drop(vb_state_lock);
                    }

                    let bit = self.virtualboy_state_buffer[idx] & 1;
                    self.virtualboy_state_buffer[idx] >>= 1;
                    self.virtualboy_state_buffer[idx] |= 0x8000;

                    self.virtualboy_readbit[idx] = self.virtualboy_readbit[idx].wrapping_add(1);
                    self.apu_controller_ports_strobed = false;
                    let vb_byte = (bit as u8) | (self.data_bus & 0xFE) | self.famicom_mic_bit(reg) | self.expansion_zapper_bits(reg) | self.expansion_paddle_bit(reg) | self.expansion_oeka_bits(reg);
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = vb_byte;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }

                if self.apu_controller_ports_strobing {
                    self.controller_shift_register1 = self.controller_port1.load(Ordering::Relaxed);
                    self.controller_shift_register2 = self.controller_port2.load(Ordering::Relaxed);
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
                let pp_d3;
                let pp_d4;
                if is_pp {
                    let count = self.powerpad_shift_count[idx] as usize;
                    pp_d3 = if count >= 8 {
                        0x08
                    } else {
                        (((self.powerpad_shift_data[idx] >> count) & 1) as u8) << 3
                    };
                    pp_d4 = if count >= 4 {
                        0x10
                    } else {
                        (((self.powerpad_shift_data[idx] >> (count + 8)) & 1) as u8) << 4
                    };
                    self.powerpad_shift_count[idx] = self.powerpad_shift_count[idx].wrapping_add(1);
                } else {
                    pp_d3 = 0;
                    pp_d4 = 0;
                }
                let controller_read = (if sr_bit == 0 { 0u8 } else { 1 }) | pp_d3 | pp_d4 | (self.data_bus & 0xE0) | self.expansion_paddle_bit(reg) | self.expansion_zapper_bits(reg) | self.expansion_oeka_bits(reg) | self.famicom_mic_bit(reg);

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

    // expansion port!
    fn expansion_paddle_bit(&mut self, reg: u8) -> u8 {
        if self.expansion_type != crate::config::ExpansionType::ArkanoidPaddle {
            return 0;
        }
        let idx = 2usize;
        let bit = if reg == 0x17 {
            let px = self.paddle_x.lock().unwrap();
            let b = if self.paddle_readbit[idx] < 8 {
                ((px[idx] >> (7 - self.paddle_readbit[idx])) & 1) << 1
            } else {
                2
            };
            drop(px);
            if self.paddle_readbit[idx] < 8 {
                self.paddle_readbit[idx] += 1;
            }
            b
        } else {
            let pb = self.paddle_button.lock().unwrap();
            if pb[idx] { 2 } else { 0 }
        };
        bit
    }

    fn expansion_zapper_bits(&self, reg: u8) -> u8 {
        if self.expansion_type != crate::config::ExpansionType::FamicomZapper {
            return 0;
        }
        if reg != 0x17 {
            return 0;
        }
        let mut val = 0u8;
        if self.expansion_zapper_trigger.load(Ordering::Relaxed) {
            val |= 0x10;
        }
        if !self.zapper_check_hit() {
            val |= 0x08;
        }
        val
    }

    fn expansion_oeka_bits(&self, reg: u8) -> u8 {
        if self.expansion_type != crate::config::ExpansionType::OekaKidsTablet {
            return 0;
        }
        if reg != 0x17 {
            return 0;
        }
        if self.apu_controller_ports_strobing {
            if self.oeka_shift {
                if self.oeka_state_buffer & 0x40000 != 0 {
                    0x00
                } else {
                    0x08
                }
            } else {
                0x04
            }
        } else {
            0x00
        }
    }

    fn expansion_family_trainer_bits(&self) -> u8 {
        if !self.expansion_type.is_family_trainer() {
            return 0;
        }
        let pressed = self.family_trainer_state.lock().unwrap();
        let mut col = [0u8; 4];
        for j in 0..3usize {
            if (self.family_trainer_ignore_rows >> (2 - j)) & 0x01 != 0 {
                continue;
            }
            for i in 0..4usize {
                col[i] |= pressed[j * 4 + i];
            }
        }
        let output = !((col[0] << 4) | (col[1] << 3) | (col[2] << 2) | (col[3] << 1)) & 0x1E;
        output
    }

    fn expansion_hyper_shot_bits(&self) -> u8 {
        if !self.expansion_type.is_hyper_shot() {
            return 0;
        }
        let pressed = self.hyper_shot_state.lock().unwrap();
        let mut output = 0u8;
        if self.hyper_shot_enable_p1 {
            if pressed[1] != 0 { output |= 0x02; }
            if pressed[0] != 0 { output |= 0x04; }
        }
        if self.hyper_shot_enable_p2 {
            if pressed[3] != 0 { output |= 0x08; }
            if pressed[2] != 0 { output |= 0x10; }
        }
        output
    }

    fn expansion_family_basic_bits(&self) -> u8 {
        if !self.expansion_type.is_family_basic() {
            return 0;
        }
        if !self.family_basic_enabled {
            return 0;
        }
        if self.family_basic_row >= 10 {
            return 0;
        }
        const KEY_MATRIX: [u8; 72] = [
            65, 36, 45, 44, 71, 42, 66, 67,
            64, 68, 52, 53, 54, 55, 56, 57,
            63, 14, 11, 10, 50, 51, 15, 26,
            62, 8, 20, 9, 12, 13, 35, 34,
            61, 24, 6, 7, 1, 21, 33, 32,
            60, 19, 17, 3, 5, 2, 31, 30,
            59, 22, 18, 0, 23, 25, 4, 29,
            58, 40, 16, 41, 43, 69, 27, 28,
            70, 46, 49, 48, 47, 37, 38, 39,
        ];
        let pressed = self.family_basic_state.lock().unwrap();
        let row = self.family_basic_row as usize;
        let col = self.family_basic_column as usize;
        if row == 9 {
            return 0;
        }
        let base = row * 8 + if col != 0 { 4 } else { 0 };
        let mut result: u8 = 0;
        for i in 0..4 {
            if pressed[KEY_MATRIX[base + i] as usize] != 0 {
                result |= 0x10;
            }
            result >>= 1;
        }
        ((!result) << 1) & 0x1E
    }

    fn refresh_party_tap_buffer(&mut self) {
        let pressed = self.party_tap_state.lock().unwrap();
        let mut buf = 0u8;
        for i in 0..6 {
            if pressed[i] != 0 {
                buf |= 1 << i;
            }
        }
        self.party_tap_buffer = buf;
        self.party_tap_read_count = 0;
    }

    fn refresh_pachinko_buffer(&mut self) {
        let (press, release) = {
            let s = self.pachinko_state.lock().unwrap();
            (s[0] != 0, s[1] != 0)
        };
        if self.pachinko_analog < 0x63 && press {
            self.pachinko_analog = self.pachinko_analog.wrapping_add(1);
        } else if self.pachinko_analog > 0 && release {
            self.pachinko_analog = self.pachinko_analog.wrapping_sub(1);
        }
        let a = self.pachinko_analog;
        let rev = ((a & 0x01) << 7) | ((a & 0x02) << 5) | ((a & 0x04) << 3) | ((a & 0x08) << 1) | ((a & 0x10) >> 1) | ((a & 0x20) >> 3) | ((a & 0x40) >> 5) | ((a & 0x80) >> 7);
        let p1 = self.controller_port1.load(Ordering::Relaxed);
        let mut ctrl: u8 = 0;
        if p1 & 0x80 != 0 { ctrl |= 1 << 0; }
        if p1 & 0x40 != 0 { ctrl |= 1 << 1; }
        if p1 & 0x10 != 0 { ctrl |= 1 << 2; }
        if p1 & 0x20 != 0 { ctrl |= 1 << 3; }
        if p1 & 0x08 != 0 { ctrl |= 1 << 4; }
        if p1 & 0x04 != 0 { ctrl |= 1 << 5; }
        if p1 & 0x02 != 0 { ctrl |= 1 << 6; }
        if p1 & 0x01 != 0 { ctrl |= 1 << 7; }
        let high = (!rev) as u16;
        self.pachinko_buffer = (ctrl as u16) | (high << 8);
    }

    fn famicom_mic_bit(&self, reg: u8) -> u8 {
        if reg != 0x16 {
            return 0;
        }
        if self.controller2_type != crate::config::ControllerType::FamicomGamepad {
            return 0;
        }
        if self.famicom_mic.load(Ordering::Relaxed) {
            0x04
        } else {
            0
        }
    }

    /// cpu store
    pub fn store(&mut self, input: u8, address: u16) {
        self.data_bus = input;
        if let Some(cart) = self.cart.as_mut() {
            cart.mapper_chip.handle_cpu_write(address, input);
        }
        if address < 0x2000 {
            let overridden = if let Some(ref mut cart) = self.cart {
                cart.mapper_chip.cpu_ram_override_store(address, input)
            } else {
                false
            };
            if !overridden {
                self.ram[(address & self.cpu_ram_mask) as usize] = input;
            }
        } else if address < 0x4000 {
            let vt369 = self
                .cart
                .as_ref()
                .map_or(false, |c| c.mapper_chip.onebus_vt369_ppu());
            if vt369 && address >= 0x3000 {
                let ppu_addr = 0x2000 | (address & 0x0FFF);
                self.store_ppu_data(ppu_addr, input);
            } else {
                let is_onebus = self
                    .cart
                    .as_ref()
                    .map_or(false, |c| crate::mappers::one_bus::is_onebus_mapper(c.memory_mapper));
                if !is_onebus || address < 0x2010 {
                    self.store_ppu_registers(address, input);
                }
            }
        } else if address >= 0x4000 && address <= 0x4015 {
            self.store_apu_registers(address, input);
        } else if address == 0x4016 {
            let prev_strobing = self.apu_controller_ports_strobing;
            self.apu_controller_ports_strobing = (input & 1) != 0;
            if self.expansion_type.is_family_trainer() {
                self.family_trainer_ignore_rows = input & 0x07;
            }
            if self.expansion_type.is_hyper_shot() {
                self.hyper_shot_enable_p2 = (input & 0x02) == 0;
                self.hyper_shot_enable_p1 = (input & 0x04) == 0;
            }
            if self.expansion_type.is_family_basic() {
                let prev_col = self.family_basic_column;
                let col = (input & 0x02) >> 1;
                if col == 0 && prev_col != 0 {
                    self.family_basic_row = (self.family_basic_row + 1) % 10;
                }
                self.family_basic_column = col;
                if (input & 0x01) != 0 {
                    self.family_basic_row = 0;
                }
                self.family_basic_enabled = (input & 0x04) != 0;
            }
            if self.expansion_type.is_party_tap() {
                let new_strobe = (input & 0x01) != 0;
                let prev = self.party_tap_strobe;
                self.party_tap_strobe = new_strobe;
                if prev && !new_strobe {
                    self.refresh_party_tap_buffer();
                }
            }
            if self.expansion_type.is_pachinko() {
                let new_strobe = (input & 0x01) != 0;
                let prev = self.pachinko_strobe;
                self.pachinko_strobe = new_strobe;
                if prev && !new_strobe {
                    self.refresh_pachinko_buffer();
                }
            }
            if self.expansion_type == crate::config::ExpansionType::OekaKidsTablet {
                let new_strobe = (input & 1) != 0;
                let shift = ((input >> 1) & 1) != 0;
                if new_strobe {
                    if !self.oeka_shift && shift {
                        self.oeka_state_buffer <<= 1;
                    }
                    self.oeka_shift = shift;
                } else {
                    let zx = *self.zapper_x.lock().unwrap();
                    let zy = *self.zapper_y.lock().unwrap();
                    let nes_x = (zx * 239.0).round().clamp(0.0, 239.0) as u8;
                    let nes_y = (zy * 255.0).round().clamp(0.0, 255.0) as u8;
                    let touch = 1u32;
                    let click = if self.oeka_click.load(Ordering::Relaxed) { 1u32 } else { 0u32 };
                    self.oeka_state_buffer = ((nes_x as u32) << 10) | ((nes_y as u32) << 2) | (touch << 1) | click;
                }
                self.oeka_strobe = new_strobe;
            }
            if (input & 1) != 0 {
                self.paddle_readbit[0] = 0;
                self.paddle_readbit[1] = 0;
                self.paddle_readbit[2] = 0;
                self.snes_readbit[0] = 0;
                self.snes_readbit[1] = 0;
                self.snes_mouse_readbit[0] = 0;
                self.snes_mouse_readbit[1] = 0;
                self.fourscore_readbit[0] = 0;
                self.fourscore_readbit[1] = 0;
                self.virtualboy_readbit[0] = 0;
                self.virtualboy_readbit[1] = 0;
                if self.expansion_adapter_type == crate::config::ExpansionAdapterType::TwoPlayer {
                    self.controller_shift_register1 = self.expansion_adapter_ports[0].load(Ordering::Relaxed);
                    self.controller_shift_register2 = self.expansion_adapter_ports[1].load(Ordering::Relaxed);
                } else if self.expansion_adapter_type == crate::config::ExpansionAdapterType::FourPlayer {
                    for p in 0..4usize {
                        self.expansion_adapter_shift_register[p] = self.expansion_adapter_ports[p].load(Ordering::Relaxed);
                    }
                }
                {
                    let vb_state_lock = self.virtualboy_state.lock().unwrap();
                    for p in 0..2usize {
                        self.virtualboy_state_buffer[p] = crate::config::build_vb_state(vb_state_lock[p]);
                    }
                }
                // latch accumulated mouse deltas into state
                {
                    let dx_lock = &mut *self.snes_mouse_delta_x.lock().unwrap();
                    let dy_lock = &mut *self.snes_mouse_delta_y.lock().unwrap();
                    let btns = self.snes_mouse_buttons.lock().unwrap();
                    for p in 0..2usize {
                        let dx = dx_lock[p].round() as i16;
                        let dy = dy_lock[p].round() as i16;
                        let cx = dx.clamp(-128, 127);
                        let cy = dy.clamp(-128, 127);
                        self.snes_mouse_state[p] =
                            (if btns[p] & 1 != 0 { 0 } else { 1 })
                            | (if btns[p] & 2 != 0 { 0 } else { 1 } << 1)
                            | (((cx as i16 + 128) as u32) << 8)
                            | (((cy as i16 + 128) as u32) << 16);
                        dx_lock[p] = 0.0;
                        dy_lock[p] = 0.0;
                    }
                }
                // subor mouse: build latch from accumulated deltas (inertia)
                {
                    let sub_btns = self.subor_mouse_buttons.lock().unwrap();
                    let sdx = &mut *self.subor_mouse_dx.lock().unwrap();
                    let sdy = &mut *self.subor_mouse_dy.lock().unwrap();
                    for p in 0..2usize {
                        let mut latch = sub_btns[p] & 0x03;
                        let dx = sdx[p];
                        let dy = sdy[p];
                        if dx > 0 { latch |= 0x08; sdx[p] -= 1; }
                        else if dx < 0 { latch |= 0x0C; sdx[p] += 1; }
                        if dy > 0 { latch |= 0x20; sdy[p] -= 1; }
                        else if dy < 0 { latch |= 0x30; sdy[p] += 1; }
                        self.subor_mouse_latch[p] = latch;
                    }
                }
            } else if prev_strobing {
                self.virtualboy_readbit[0] = 0;
                self.virtualboy_readbit[1] = 0;
                {
                    let vb_state_lock = self.virtualboy_state.lock().unwrap();
                    for p in 0..2usize {
                        self.virtualboy_state_buffer[p] = crate::config::build_vb_state(vb_state_lock[p]);
                    }
                }
            }
            if self.cart.as_ref().is_some_and(|c| matches!(c.memory_mapper, 99 | 604)) {
                let cart = self.cart.as_mut().unwrap();
                let mut mapper = std::mem::replace(&mut cart.mapper_chip, Box::new(crate::mapper::MapperNROM::new(crate::mapper::NromConfig::default())));
                mapper.store_prg(cart, address, input);
                self.cart.as_mut().unwrap().mapper_chip = mapper;
            }
        } else if address == 0x4017 {
            if self.expansion_type.is_family_trainer() {
                self.family_trainer_ignore_rows = input & 0x07;
            }
            if self.expansion_type.is_hyper_shot() {
                self.hyper_shot_enable_p2 = (input & 0x02) == 0;
                self.hyper_shot_enable_p1 = (input & 0x04) == 0;
            }
            if self.expansion_type.is_family_basic() {
                let prev_col = self.family_basic_column;
                let col = (input & 0x02) >> 1;
                if col == 0 && prev_col != 0 {
                    self.family_basic_row = (self.family_basic_row + 1) % 10;
                }
                self.family_basic_column = col;
                if (input & 0x01) != 0 {
                    self.family_basic_row = 0;
                }
                self.family_basic_enabled = (input & 0x04) != 0;
            }
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
            if address >= 0x5000 && address < 0x6000 && self.cart.as_ref().map_or(false, |c| c.mapper_chip.is_um6578()) {
                if address < 0x5800 {
                    self.um6578_extra_ram[(address & 0x7FF) as usize] = input;
                }
                return;
            }
            if address >= 0x5000 && address < 0x6000 {
                let vt369 = self
                    .cart
                    .as_ref()
                    .map_or(false, |c| c.mapper_chip.onebus_vt369_ppu());
                if vt369 {
                    let reg2000_1e = self
                        .cart
                        .as_ref()
                        .map_or(0, |c| c.mapper_chip.vt369_reg2000(0x1E));
                    let mask = if reg2000_1e == 0 { 0x0FF } else { 0x3FF };
                    let pal_addr = ((address & 0x3FF) as usize) & mask;
                    self.palette_ram[pal_addr] = input;
                    return;
                }
            }
            if address == 0x4201 {
                let vt369 = self
                    .cart
                    .as_ref()
                    .map_or(false, |c| c.mapper_chip.onebus_vt369_ppu());
                if vt369 {
                    let start = (input as usize) << 8;
                    let ram_len = self.ram.len();
                    for i in 0..0x200 {
                        let byte = self.ram[(start + i) % ram_len];
                        if i < self.oam.len() {
                            self.oam[i] = byte;
                        }
                        if i < self.vt369_sprite_ram.len() {
                            self.vt369_sprite_ram[i] = byte;
                        }
                    }
                }
            }
            let is_onebus = self
                .cart
                .as_ref()
                .map_or(false, |c| crate::mappers::one_bus::is_onebus_mapper(c.memory_mapper));
            if self.cart.as_ref().map_or(false, |c| c.mapper_chip.is_um6578()) && address == 0x4020 {
                return;
            }
            if self.cart.as_ref().map_or(false, |c| c.mapper_chip.is_um6578()) && (0x4200..=0x421F).contains(&address) {
                return;
            }
            if !is_onebus && address <= 0x402F {
                let apu_addr = 0x4000 + (address & 0x0F);
                self.store_apu_registers(apu_addr, input);
            }
            if self.cart.as_ref().map_or(false, |c| c.mapper_chip.is_um6578()) && address == 0x4016 {
                if self.cart.as_ref().map_or(false, |c| c.memory_mapper == 600) {
                    self.um6578_dma_page = (self.um6578_dma_page & 0x1F) | ((input << 4) & 0xE0);
                }
            }
            if self.cart.as_ref().map_or(false, |c| c.mapper_chip.is_um6578()) && address == 0x4026 {
                if self.cart.as_ref().map_or(false, |c| c.memory_mapper == 601) {
                    self.um6578_dma_page = (self.um6578_dma_page & 0x1F) | ((input >> 2) & 0xE0);
                }
            }
            if self.cart.as_ref().map_or(false, |c| c.mapper_chip.is_um6578()) && (0x4048..=0x404F).contains(&address) {
                match address {
                    0x4048 => {
                        self.um6578_dma_control = input;
                        if input & 0x80 != 0 {
                            self.um6578_dma_control &= !0x80;
                            let ctrl = input;
                            let page = self.um6578_dma_page as usize;
                            let mut src = self.um6578_dma_source;
                            let mut dst = self.um6578_dma_target;
                            let len = self.um6578_dma_length;
                            let mut busy: u32 = 0;
                            for _ in 0..=len {
                                let data = if src & 0x8000 != 0 {
                                    let rom_addr = (src as usize & 0x7FFF) | (page << 15);
                                    let prg_len = self.cart.as_ref().map_or(0, |c| c.prg_rom.len());
                                    if prg_len == 0 { 0u8 } else { self.cart.as_ref().unwrap().prg_rom[rom_addr % prg_len] }
                                } else {
                                    self.um6578_cpu_read(src)
                                };
                                if ctrl & 0x20 != 0 {
                                    self.um6578_cpu_write(dst, data);
                                } else {
                                    self.um6578_write_ppu(dst, data);
                                }
                                src = src.wrapping_add(1);
                                dst = dst.wrapping_add(1);
                                busy = busy.saturating_add(if ctrl & 0x40 != 0 { 1 } else { 2 });
                            }
                            self.um6578_dma_busy = busy;
                        }
                    }
                    0x4049 => self.um6578_dma_page = input & 0x1F,
                    0x404A => self.um6578_dma_source = (self.um6578_dma_source & 0xFF00) | input as u16,
                    0x404B => self.um6578_dma_source = (self.um6578_dma_source & 0x00FF) | ((input as u16) << 8),
                    0x404C => self.um6578_dma_target = (self.um6578_dma_target & 0xFF00) | input as u16,
                    0x404D => self.um6578_dma_target = (self.um6578_dma_target & 0x00FF) | ((input as u16) << 8),
                    0x404E => self.um6578_dma_length = (self.um6578_dma_length & 0xFF00) | input as u16,
                    0x404F => self.um6578_dma_length = (self.um6578_dma_length & 0x00FF) | (((input as u16) << 8) & 0x7F00),
                    _ => {}
                }
                return;
            }
            let cart = self.cart.as_mut().unwrap();
            cart.mapper_cpu_cycle = self.total_cycles as i64;
            let mut mapper = std::mem::replace(&mut cart.mapper_chip, Box::new(crate::mapper::MapperNROM::new(crate::mapper::NromConfig::default())));
            mapper.store_prg(cart, address, input);
            let irq_ack = mapper.take_irq_ack();
            mapper.execute_dma(cart, &mut self.ram, &mut self.vram);
            let cart = self.cart.as_mut().unwrap();
            cart.mapper_chip = mapper;

            if irq_ack {
                self.irq_level_detector = false;
            }

            if cart.memory_mapper == 20 && (address >= 0x4022 && address <= 0x4025) {
                self.irq_level_detector = false;
            }

              if               (address & 0xE001) == 0xE000 && matches!(cart.memory_mapper, 4 | 12 | 37 | 44 | 45 | 47 | 49 | 52 | 64 | 74 | 100 | 114 | 115 | 116 | 118 | 119 | 121 | 123 | 124 | 126 | 131 | 134 | 142 | 165 | 169 | 182 | 187 | 189 | 191 | 192 | 194 | 195 | 196 | 197 | 198 | 199 | 205 | 208 | 215 | 219 | 224 | 238 | 245 | 248 | 249 | 254 | 256 | 259 | 260 | 262 | 263 | 267 | 268 | 269 | 287 | 291 | 292 | 296 | 307 | 313 | 315 | 321 | 322 | 325 | 327 | 333 | 334 | 339 | 344 | 345 | 348 | 351 | 353 | 356 | 359 | 361 | 362 | 364 | 366 | 367 | 368 | 369 | 370 | 372 | 373 | 377 | 383 | 391 | 392 | 393 | 394 | 395 | 422 | 441 | 443 | 444 | 445 | 455 | 456 | 457 | 458 | 460 | 467 | 468 | 472 | 473 | 474 | 475 | 478 | 479 | 480 | 481 | 482 | 483 | 484 | 486 | 490 | 503 | 504 | 505 | 506 | 507 | 508 | 509 | 510 | 511 | 512 | 513 | 516 | 524 | 528 | 531 | 534 | 536 | 537 | 545 | 555 | 566 | 567 | 568 | 569 | 572 | 578 | 593 | 594 | 596 | 597 | 613 | 616 | 618 | 620) {
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
            } else if matches!(cart.memory_mapper, 21 | 22 | 23 | 25 | 85 | 520 | 526 | 529 | 530 | 542 | 544 | 617) && (address & 0xF000) == 0xF000 {
                self.irq_level_detector = false;
            } else if matches!(cart.memory_mapper, 35 | 90 | 209 | 211 | 281 | 282 | 295 | 358 | 359 | 386 | 387 | 388 | 397 | 540) && address >= 0xC000 && address < 0xD000 {
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
                let vt369 = self.cart.as_ref().map_or(false, |c| c.mapper_chip.onebus_vt369_ppu());
                let fast_dma = vt369 && (self.cart.as_ref().map_or(0, |c| c.mapper_chip.vt369_reg4100(0x1C)) & 0x80 != 0);
                if fast_dma {
                    let (middle, length, target) = self.onebus_dma_config();
                    let from_base = ((input as u16) << 8) | (middle as u16);
                    let inc = if self.ppu_control_increment_mode_32 { 32 } else { 1 };
                    let saved_addr_bus = self.address_bus;
                    for i in 0..length {
                        let src_addr = from_base.wrapping_add(i);
                        self.address_bus = src_addr;
                        let val = self.fetch(src_addr);
                        if target == 0x2007 {
                            let taddr = self.vt369_dma_target_addr;
                            if taddr < 0x3C00 {
                                self.store_ppu_data(taddr, val);
                            } else {
                                self.palette_ram[(taddr & 0x3FF) as usize] = val;
                            }
                            self.vt369_dma_target_addr = self.vt369_dma_target_addr.wrapping_add(inc);
                        } else {
                            self.store_ppu_registers(0x2004, val);
                        }
                    }
                    self.address_bus = saved_addr_bus;
                    self.do_oam_dma = false;
                } else {
                    self.do_oam_dma = true;
                    self.first_cycle_of_oam_dma = true;
                    self.dma_address = 0;
                    self.dma_page = input;
                }
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

    fn um6578_cpu_read(&self, addr: u16) -> u8 {
        let a = addr & 0x7FFF;
        if a < 0x2000 {
            let idx = (a & self.cpu_ram_mask) as usize % self.ram.len();
            self.ram[idx]
        } else if (0x5000..0x5800).contains(&a) {
            self.um6578_extra_ram[(a & 0x7FF) as usize]
        } else {
            0xFF
        }
    }

    fn um6578_cpu_write(&mut self, addr: u16, data: u8) {
        let a = addr & 0x7FFF;
        if a < 0x2000 {
            let idx = (a & self.cpu_ram_mask) as usize % self.ram.len();
            self.ram[idx] = data;
        } else if (0x5000..0x5800).contains(&a) {
            self.um6578_extra_ram[(a & 0x7FF) as usize] = data;
        }
    }

    pub fn store_ppu_data(&mut self, address: u16, input: u8) {
        if self.cart.as_ref().map_or(false, |c| c.mapper_chip.is_um6578()) {
            self.um6578_write_ppu(address, input);
            return;
        }
        let address = address & 0x3FFF;
        let vt369_enhanced = self
            .cart
            .as_ref()
            .map_or(false, |c| c.mapper_chip.onebus_vt369_enhanced_ppu());
        let pal_limit = if vt369_enhanced { 0x3C00 } else { 0x3F00 };
        if address < pal_limit {
            if let Some(cart) = self.cart.as_mut() {
                let mut mapper = std::mem::replace(&mut cart.mapper_chip, Box::new(crate::mapper::MapperNROM::new(crate::mapper::NromConfig::default())));
                mapper.store_ppu(cart, address, input, &mut self.vram);
                cart.mapper_chip = mapper;
            }
        } else {
            // palette RAM
            let is_onebus = self
                .cart
                .as_ref()
                .map_or(false, |c| crate::mappers::one_bus::is_onebus_mapper(c.memory_mapper));
            let vt03_ppu = self
                .cart
                .as_ref()
                .map_or(false, |c| c.mapper_chip.onebus_vt03_ppu());
            let mut pal_addr = if vt369_enhanced {
                (address & 0x3FF) as usize
            } else if is_onebus {
                (address & 0xFF) as usize
            } else {
                let mirrored = self.ppu_address_with_mirroring(address);
                (mirrored & 0x1F) as usize
            };
            if vt03_ppu && self.do_oam_dma && !self.is_pal() && !self.is_dendy() {
                pal_addr = pal_addr.wrapping_sub(1) & 0xFF;
            }
            self.palette_ram[pal_addr] = input;

            if vt03_ppu && (pal_addr as u8 & 0x63) == 0 {
                self.palette_ram[pal_addr ^ 0x10] = input;
            }

            // palette mirrors
            if !is_onebus && !vt369_enhanced && (pal_addr & 3) == 0 {
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
    398, 354, 316, 298, 276, 236, 210, 198, 176, 148, 132, 118, 98, 78, 66, 50,
];
