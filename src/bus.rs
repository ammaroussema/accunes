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
                } else {
                    self.data_bus = self.ram[(address & self.cpu_ram_mask) as usize];
                    self.data_pins_are_not_floating = true;
                }
            } else {
                self.data_bus = self.ram[(address & self.cpu_ram_mask) as usize];
                self.data_pins_are_not_floating = true;
            }        } else if address >= 0x2000 && address < 0x4000 {
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
            // $410E/$410F/$412C for abl, tv pump and golden nugget casino
            if matches!(address, 0x410E | 0x410F | 0x412C)
                && (self.expansion_type.is_abl_pinball()
                    || self.expansion_type.is_golden_nugget_casino()
                    || self.expansion_type.is_tv_pump())
            {
                let is_onebus = self
                    .cart
                    .as_ref()
                    .map_or(false, |c| {
                        crate::mappers::one_bus::is_onebus_mapper(c.memory_mapper)
                    });
                if is_onebus {
                    let reg = self
                        .cart
                        .as_ref()
                        .map_or(0, |c| c.mapper_chip.vt369_reg4100(0x0D));
                    let val = match address {
                        0x410E => {
                            let low = if (!reg & 0x01) != 0 && (reg & 0x02) != 0 {
                                self.onebus_read_iop(0) & 0x0F
                            } else {
                                0x0F
                            };
                            let high = if (!reg & 0x04) != 0 && (reg & 0x08) != 0 {
                                self.onebus_read_iop(1) & 0x0F
                            } else {
                                0x0F
                            };
                            low | (high << 4)
                        }
                        0x410F => {
                            let low = if (!reg & 0x10) != 0 && (reg & 0x20) != 0 {
                                self.onebus_read_iop(2) & 0x0F
                            } else {
                                0x0F
                            };
                            let high = self.onebus_read_iop(3) & 0x0F;
                            low | (high << 4)
                        }
                        0x412C => self.onebus_read_iop(4),
                        _ => unreachable!(),
                    };
                    self.data_bus = val;
                    self.data_pins_are_not_floating = true;
                    return self.data_bus;
                }
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
            if cart.mapper_chip.is_study_box() && address == 0x4200 {
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
                // famicom network controller
                if reg == 0x16 && self.expansion_type.is_fami_net_sys() {
                    let fns_val = self.fami_net_sys_bits() | (self.data_bus & 0xFD);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = fns_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // city patrolman light gun
                if reg == 0x16 && self.expansion_type.is_city_patrolman() {
                    let cp_val = self.city_patrolman_read1() | (self.data_bus & 0xFC);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = cp_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // pokkun moguraa
                if reg == 0x17 && self.expansion_type.is_moguraa() {
                    let mog_val = self.moguraa_read() | (self.data_bus & 0xE1);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = mog_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // golden nugget casino
                if reg == 0x16 && self.expansion_type.is_golden_nugget_casino() {
                    let gnc_val = self.golden_nugget_read1() | (self.data_bus & 0xFE);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = gnc_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // abl pinball
                if reg == 0x16 && self.expansion_type.is_abl_pinball() {
                    let abl_val = self.abl_pinball_read1() | (self.data_bus & 0xF8);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = abl_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // abl pinball
                if reg == 0x17 && self.expansion_type.is_abl_pinball() {
                    let abl_val = self.abl_pinball_read2() | (self.data_bus & 0xF8);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = abl_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // sharp c1 cassette
                if reg == 0x17 && self.expansion_type.is_sharp_c1_cassette() {
                    let sc_val = if self.tape_input() { 0 } else { 4 } | (self.data_bus & 0xFB);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = sc_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // triface mahjong
                if reg == 0x17 && self.expansion_type.is_triface_mahjong() {
                    self.triface_mahjong_build_keys();
                    let tf_val = self.triface_mahjong_read() | (self.data_bus & 0xE1);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = tf_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // mahjong gekitou 
                if reg == 0x16 && self.expansion_type.is_mahjong_gekitou() {
                    let mg_val = self.mahjong_gekitou_read1() | (self.data_bus & 0xFE);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = mg_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
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
                // bandai hyper shot
                if reg == 0x17 && self.expansion_type.is_bandai_hyper_shot() {
                    let mut bh_val = self.data_bus & 0xE0;
                    if !self.zapper_check_hit() {
                        bh_val |= 0x08;
                    }
                    let bh = self.bandai_hyper_buttons.lock().unwrap();
                    if bh[8] != 0 {
                        bh_val |= 0x10;
                    }
                    drop(bh);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = bh_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // ascii turbo file
                if reg == 0x17 && self.expansion_type.is_turbo_file() {
                    let pos = self.turbo_file_position as usize;
                    let bit = ((self.turbo_file_data[pos / 8] >> (pos % 8)) & 0x01) << 2;
                    let tf_val = bit | (self.data_bus & 0xFB);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = tf_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // battle box
                if reg == 0x17 && self.expansion_type.is_battle_box() {
                    if self.battle_box_last_write & 0x01 != 0 {
                        self.battle_box_chip_select ^= 0x01;
                        self.battle_box_input_data = 0;
                        self.battle_box_input_bit_position = 0;
                    }
                    self.battle_box_output ^= 0x01;
                    let mut read_bit = 0u8;
                    if self.battle_box_is_read {
                        let addr = if self.battle_box_chip_select != 0 { 0x80 } else { 0 } | self.battle_box_address;
                        let word = u16::from_le_bytes([self.battle_box_data[(addr as usize) * 2], self.battle_box_data[(addr as usize) * 2 + 1]]);
                        read_bit = (((word >> self.battle_box_input_bit_position) & 0x01) as u8) << 3;
                    }
                    let write_bit = self.battle_box_output << 4;
                    let bb_val = read_bit | write_bit | (self.data_bus & 0xC7);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = bb_val;
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
                // exciting boxing (punching bag) controller
                if reg == 0x17 && self.expansion_type.is_exciting_boxing() {
                    let eb_val = self.expansion_exciting_boxing_bits() | (self.data_bus & 0xE0);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = eb_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // jissen mahjong controller
                if reg == 0x17 && self.expansion_type.is_jissen_mahjong() {
                    let bit = self.jissen_mahjong_read();
                    let jm_val = (bit << 1) | (self.data_bus & 0xFD);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = jm_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // subor keyboard
                if reg == 0x17 && self.expansion_type.is_subor_keyboard() {
                    let sk_val = self.subor_keyboard_bits() | (self.data_bus & 0xE1);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = sk_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // PEC586 keyboard (dongda keyboard)
                if reg == 0x17 && self.expansion_type.is_pec586_keyboard() {
                    let pk_bit = self.pec586_keyboard_bit();
                    let mut pk_val = pk_bit | (self.data_bus & 0xFD);
                    pk_val = (pk_val & !0x40) | ((pk_val << 5) & 0x40);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = pk_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // bit-79 keyboard
                if reg == 0x16 && self.expansion_type.is_bit79_keyboard() {
                    let b79_bit = self.bit79_keyboard_bit();
                    let b79_val = b79_bit | (self.data_bus & 0xFD);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = b79_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // keda keyboard
                if reg == 0x17 && self.expansion_type.is_keda_keyboard() {
                    let kd_bit = self.keda_keyboard_bit();
                    let mut kd_val = kd_bit | (self.data_bus & 0xFD);
                    kd_val = (kd_val & !0x40) | ((kd_val << 5) & 0x40);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = kd_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // kingwon keyboard
                if reg == 0x17 && self.expansion_type.is_kingwon_keyboard() {
                    let kw_bit = self.kingwon_keyboard_bit();
                    let kw_val = kw_bit | (self.data_bus & 0xFD);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = kw_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // ze cheng keyboard/mouse
                if (reg == 0x16 || reg == 0x17) && self.expansion_type.is_zecheng_keyboard() {
                    let idx = (reg == 0x17) as usize;
                    let bits = if idx == 0 { self.zecheng_keyboard_bits1 } else { self.zecheng_keyboard_bits2 };
                    let bit = ((bits & 0x80) >> 7) as u8;
                    if idx == 0 {
                        self.zecheng_keyboard_bits1 <<= 1;
                    } else {
                        self.zecheng_keyboard_bits2 <<= 1;
                    }
                    let zc_val = bit | (self.data_bus & 0xFE);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = zc_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // quiz king buzzers
                if reg == 0x17 && self.expansion_type.is_quiz_king() {
                    let qk_val = self.quiz_king_bits() | (self.data_bus & 0xE3);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = qk_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // top rider
                if reg == 0x17 && self.expansion_type.is_top_rider() {
                    let tr_val = self.top_rider_bits() | (self.data_bus & 0xE7);
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = tr_val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // barcode battler
                if reg == 0x17 && self.expansion_type.is_barcode_battler() {
                    let bb_val = if self.barcode_battler_active {
                        let elapsed = self.master_cycle_counter.saturating_sub(self.barcode_battler_insert_cycle);
                        let cycles_per_bit = 21_477_272 / 1200;
                        let stream_pos = (elapsed / cycles_per_bit) as usize;
                        if stream_pos < 200 {
                            (self.barcode_battler_stream[stream_pos] << 2) | (self.data_bus & 0xE0)
                        } else {
                            self.data_bus & 0xE0
                        }
                    } else {
                        self.data_bus & 0xE0
                    };
                    self.apu_controller_ports_strobed = false;
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = bb_val;
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
                    let std_sr = if w == 0 { self.controller_shift_register1 } else { self.controller_shift_register2 };
                    let p_bit = ((std_sr >> 7) & 1) as u8;
                    if w == 0 {
                        self.controller1_shift_counter = 2;
                    } else {
                        self.controller2_shift_counter = 2;
                    }
                    let readbit = self.fourscore_readbit[w];
                    let exp_bit = if readbit < 8 {
                        ((self.expansion_adapter_shift_register[w] >> 7) & 1) as u8
                    } else {
                        1
                    };
                    self.expansion_adapter_shift_register[w] = (self.expansion_adapter_shift_register[w] << 1) | 1;
                    self.fourscore_readbit[w] = readbit.wrapping_add(1);
                    self.apu_controller_ports_strobed = false;
                    let val = p_bit | (exp_bit << 1) | (self.data_bus & 0xFC);
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                // hori 4-player adapter
                if self.expansion_adapter_type == crate::config::ExpansionAdapterType::HoriFourPlayer {
                    let w = (reg - 0x16) as usize;
                    let std_sr = if w == 0 { self.controller_shift_register1 } else { self.controller_shift_register2 };
                    let p_bit = ((std_sr >> 7) & 1) as u8;
                    if w == 0 {
                        self.controller1_shift_counter = 2;
                    } else {
                        self.controller2_shift_counter = 2;
                    }
                    let readbit = self.fourscore_readbit[w];
                    let exp_bit = if readbit < 8 {
                        ((self.expansion_adapter_shift_register[w] >> 7) & 1) as u8
                    } else if readbit < 16 {
                        ((self.expansion_adapter_shift_register[w + 2] >> 7) & 1) as u8
                    } else if readbit == if w == 0 { 18 } else { 19 } {
                        1
                    } else if readbit >= 24 {
                        1
                    } else {
                        0
                    };
                    if readbit < 16 {
                        let sr_idx = w + if readbit < 8 { 0 } else { 2 };
                        self.expansion_adapter_shift_register[sr_idx] = (self.expansion_adapter_shift_register[sr_idx] << 1) | 1;
                    }
                    if readbit < 24 {
                        self.fourscore_readbit[w] = readbit.wrapping_add(1);
                    }
                    self.apu_controller_ports_strobed = false;
                    let val = p_bit | (exp_bit << 1) | (self.data_bus & 0xFC);
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = val;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
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
                if ctype == crate::config::ControllerType::PS2Mouse {
                    let idx = (reg == 0x17) as usize;
                    let ps2_val = self.ps2_mouse_read(idx);
                    self.apu_controller_ports_strobed = false;
                    let ps2_byte = ps2_val | (self.data_bus & 0xFE);
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = ps2_byte;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                if ctype == crate::config::ControllerType::YuxingMouse {
                    let idx = (reg == 0x17) as usize;
                    let yx_val = self.yuxing_mouse_read(idx);
                    self.apu_controller_ports_strobed = false;
                    let yx_byte = yx_val | (self.data_bus & 0xFE);
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = yx_byte;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                if ctype == crate::config::ControllerType::BelsonicMouse {
                    let idx = (reg == 0x17) as usize;
                    let bs_val = self.belsonic_mouse_read(idx);
                    self.apu_controller_ports_strobed = false;
                    let bs_byte = bs_val | (self.data_bus & 0xFE);
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = bs_byte;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                if ctype == crate::config::ControllerType::MegaBookMouse {
                    let idx = (reg == 0x17) as usize;
                    let mb_val = self.megabook_mouse_read(idx);
                    self.apu_controller_ports_strobed = false;
                    let mb_byte = mb_val | (self.data_bus & 0xFE);
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = mb_byte;
                    self.internal_bus = self.data_bus;
                    return self.data_bus;
                }
                if self.expansion_type == crate::config::ExpansionType::HoriTrack {
                    let idx = (reg == 0x17) as usize;
                    let val = (self.hori_track_state[idx] & 0x01) as u8;
                    self.hori_track_state[idx] >>= 1;
                    self.apu_controller_ports_strobed = false;
                    let ht_byte = (val << 1) | (self.data_bus & 0xFD);
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = ht_byte;
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
                // bandai hyper shot
                if reg == 0x16 && self.expansion_type.is_bandai_hyper_shot() {
                    let idx = 0usize;
                    let val = (self.bandai_hyper_state[idx] & 0x01) as u8;
                    self.bandai_hyper_state[idx] >>= 1;
                    self.apu_controller_ports_strobed = false;
                    let bh_byte = (val << 1) | (self.data_bus & 0xFD);
                    if self.do_oam_dma && self.data_pins_are_not_floating {
                        self.internal_bus = self.data_bus;
                        return self.data_bus;
                    }
                    self.data_bus = bh_byte;
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

        if self.cheats.has_active_cheats() {
            self.data_bus = self.cheats.apply(address, self.data_bus);
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

    fn expansion_exciting_boxing_bits(&self) -> u8 {
        if !self.expansion_type.is_exciting_boxing() {
            return 0;
        }
        let pressed = self.punching_bag_state.lock().unwrap();
        let mut output: u8 = 0;
        if self.punching_bag_selected_sensors == 0 {
            if pressed[0] == 0 { output |= 0x02; }
            if pressed[1] == 0 { output |= 0x04; }
            if pressed[2] == 0 { output |= 0x08; }
            if pressed[3] == 0 { output |= 0x10; }
        } else {
            if pressed[4] == 0 { output |= 0x02; }
            if pressed[5] == 0 { output |= 0x04; }
            if pressed[6] == 0 { output |= 0x08; }
            if pressed[7] == 0 { output |= 0x10; }
        }
        output
    }

    fn jissen_mahjong_read(&mut self) -> u8 {
        if self.jissen_mahjong_strobe {
            self.refresh_jissen_mahjong_buffer();
        }
        let bit = (self.jissen_mahjong_state_buffer & 0x01) as u8;
        self.jissen_mahjong_state_buffer >>= 1;
        bit
    }

    fn refresh_jissen_mahjong_buffer(&mut self) {
        let pressed = self.jissen_mahjong_state.lock().unwrap();
        let mut buf: u32 = 0;
        match self.jissen_mahjong_row {
            1 => {
                if pressed[13] != 0 { buf |= 0x04; }
                if pressed[12] != 0 { buf |= 0x08; }
                if pressed[11] != 0 { buf |= 0x10; }
                if pressed[10] != 0 { buf |= 0x20; }
                if pressed[9] != 0 { buf |= 0x40; }
                if pressed[8] != 0 { buf |= 0x80; }
            }
            2 => {
                if pressed[7] != 0 { buf |= 0x01; }
                if pressed[6] != 0 { buf |= 0x02; }
                if pressed[5] != 0 { buf |= 0x04; }
                if pressed[4] != 0 { buf |= 0x08; }
                if pressed[3] != 0 { buf |= 0x10; }
                if pressed[2] != 0 { buf |= 0x20; }
                if pressed[1] != 0 { buf |= 0x40; }
                if pressed[0] != 0 { buf |= 0x80; }
            }
            3 => {
                if pressed[20] != 0 { buf |= 0x02; }
                if pressed[19] != 0 { buf |= 0x04; }
                if pressed[18] != 0 { buf |= 0x08; }
                if pressed[17] != 0 { buf |= 0x10; }
                if pressed[16] != 0 { buf |= 0x20; }
                if pressed[15] != 0 { buf |= 0x40; }
                if pressed[14] != 0 { buf |= 0x80; }
            }
            _ => {}
        }
        self.jissen_mahjong_state_buffer = buf;
    }

    fn subor_keyboard_bits(&self) -> u8 {
        if !self.expansion_type.is_subor_keyboard() {
            return 0;
        }
        if !self.subor_keyboard_enabled {
            return 0x1E;
        }
        let pressed = self.subor_keyboard_state.lock().unwrap();
        let mut result: u8 = 0;
        let base_index = (self.subor_keyboard_row as usize) * 8 + if self.subor_keyboard_column != 0 { 4 } else { 0 };
        for i in 0..4usize {
            let idx = crate::config::SUBOR_KEYBOARD_MATRIX[base_index + i] as usize;
            if pressed[idx] != 0 {
                result |= 1 << i;
            }
        }
        if self.subor_keyboard_row == 9 && self.subor_keyboard_column != 0 {
            result |= 0x01;
        }
        ((!result) << 1) & 0x1E
    }

    /// dongda PEC-586 keyboard
    fn pec586_keyboard_bit(&mut self) -> u8 {
        if !self.expansion_type.is_pec586_keyboard() {
            return 0;
        }
        let pressed = self.pec586_keyboard_state.lock().unwrap();
        let kspos = (self.pec586_kspos % 13) as usize;
        let ksindex = (7 - (self.pec586_ksindex & 7)) as usize;
        let idx = crate::config::PEC586_KEYBOARD_MATRIX[kspos * 8 + ksindex] as usize;
        self.pec586_ksindex = (self.pec586_ksindex + 1) & 7;
        if idx < pressed.len() && pressed[idx] != 0 {
            0x02
        } else {
            0
        }
    }

    /// bit-79 keyboard
    fn bit79_keyboard_bit(&mut self) -> u8 {
        if !self.expansion_type.is_bit79_keyboard() {
            return 0;
        }
        let pressed = self.bit79_keyboard_state.lock().unwrap();
        let row = (self.bit79_keyboard_row % 10) as usize;
        let col = (7 - (self.bit79_keyboard_column & 7)) as usize;
        let idx = crate::config::BIT79_KEYBOARD_MATRIX[row * 8 + col] as usize;
        self.bit79_keyboard_column = (self.bit79_keyboard_column + 1) & 7;
        if idx < pressed.len() && pressed[idx] != 0 {
            0x02
        } else {
            0
        }
    }

    /// keda keyboard
    fn keda_keyboard_bit(&mut self) -> u8 {
        if !self.expansion_type.is_keda_keyboard() {
            return 0;
        }
        let pressed = self.keda_keyboard_state.lock().unwrap();
        let row = (self.keda_keyboard_row % 11) as usize;
        let col = (7 - (self.keda_keyboard_column & 7)) as usize;
        let idx = crate::config::KEDA_KEYBOARD_MATRIX[row * 8 + col] as usize;
        self.keda_keyboard_column = (self.keda_keyboard_column + 1) & 7;
        if idx < pressed.len() && pressed[idx] != 0 {
            0x02
        } else {
            0
        }
    }

    /// kingwon keyboard
    fn kingwon_keyboard_bit(&mut self) -> u8 {
        if !self.expansion_type.is_kingwon_keyboard() {
            return 0;
        }
        let pressed = self.kingwon_keyboard_state.lock().unwrap();
        let row = (self.kingwon_keyboard_row % 13) as usize;
        let col = (self.kingwon_keyboard_column & 7) as usize;
        let idx = crate::config::KINGWON_KEYBOARD_MATRIX[row * 8 + col] as usize;
        self.kingwon_keyboard_column = (self.kingwon_keyboard_column + 1) & 7;
        if idx < pressed.len() && pressed[idx] != 0 {
            0
        } else {
            0x02
        }
    }

    /// bit-79 keyboard 
    fn bit79_keyboard_write(&mut self, input: u8) {
        let prev = self.bit79_keyboard_strobe;
        if (prev & 0x02) == 0 && (input & 0x02) != 0 {
            self.bit79_keyboard_row = 0;
        }
        if (prev & 0x01) != 0 && (input & 0x01) == 0 {
            self.bit79_keyboard_column = 0;
        }
        if (prev & 0x04) != 0 && (input & 0x04) == 0 {
            self.bit79_keyboard_row = (self.bit79_keyboard_row + 1) % 10;
        }
        self.bit79_keyboard_strobe = input;
    }

    /// keda keyboard
    fn keda_keyboard_write(&mut self, input: u8) {
        let prev = self.keda_keyboard_strobe;
        if (prev & 0x02) == 0 && (input & 0x02) != 0 {
            self.keda_keyboard_row = 0;
        }
        if (prev & 0x01) != 0 && (input & 0x01) == 0 {
            self.keda_keyboard_column = 0;
        }
        if (prev & 0x04) != 0 && (input & 0x04) == 0 {
            self.keda_keyboard_row = (self.keda_keyboard_row + 1) % 11;
        }
        self.keda_keyboard_strobe = input;
    }

    /// kingwon keyboard
    fn kingwon_keyboard_write(&mut self, input: u8) {
        if input & 0x04 != 0 {
            let prev = self.kingwon_keyboard_bits;
            if prev & 0x02 != 0 && input & 0x02 == 0 {
                self.kingwon_keyboard_row = 0;
                self.kingwon_keyboard_column = 0;
            } else if prev & 0x01 != 0 && input & 0x01 == 0 {
                self.kingwon_keyboard_row = (self.kingwon_keyboard_row + 1) % 13;
                self.kingwon_keyboard_column = 0;
            }
            self.kingwon_keyboard_bits = input;
        }
    }

    /// ze cheng keyboard/mouse
    fn zecheng_keyboard_write(&mut self, input: u8) {
        let strobe = (input & 0x01) != 0;
        if self.zecheng_keyboard_strobe && !strobe {
            self.zecheng_keyboard_bits1 = 0;
            self.zecheng_keyboard_bits2 = 0;
            let btns = *self.zecheng_keyboard_buttons.lock().unwrap();
            let dx = *self.zecheng_keyboard_dx.lock().unwrap();
            let dy = *self.zecheng_keyboard_dy.lock().unwrap();
            if btns != 0 || dx != 0 || dy != 0 {
                self.zecheng_keyboard_bits2 = 0x40
                    | (if btns & 0x01 != 0 { 0x04 } else { 0 })
                    | (if btns & 0x02 != 0 { 0x08 } else { 0 });
                let mut delta_x = dx;
                let mut delta_y = dy;
                *self.zecheng_keyboard_dx.lock().unwrap() = 0;
                *self.zecheng_keyboard_dy.lock().unwrap() = 0;
                if delta_x < 0 {
                    self.zecheng_keyboard_bits2 |= 0x01;
                    delta_x = -delta_x;
                }
                if delta_y < 0 {
                    self.zecheng_keyboard_bits2 |= 0x02;
                    delta_y = -delta_y;
                }
                let mag_x = (32 - (delta_x as u32).leading_zeros()) as u8;
                let mag_y = (32 - (delta_y as u32).leading_zeros()) as u8;
                self.zecheng_keyboard_bits1 = mag_y | (mag_x << 4);
            } else {
                self.zecheng_keyboard_bits1 = 0;
                self.zecheng_keyboard_bits2 = 0x80;
                let keys = *self.zecheng_keyboard_keys.lock().unwrap();
                if keys & 0x02 != 0 {
                    self.zecheng_keyboard_bits1 = 65; 
                }
                if keys & 0x01 != 0 {
                    self.zecheng_keyboard_bits1 = 1;
                }
            }
        }
        self.zecheng_keyboard_strobe = strobe;
    }

    /// quiz king buzzers
    fn quiz_king_bits(&mut self) -> u8 {
        if !self.expansion_type.is_quiz_king() {
            return 0;
        }
        let out = (self.quiz_king_data_r & 0x07) << 2;
        self.quiz_king_data_r >>= 3;
        self.quiz_king_data_r |= if self.quiz_king_funky_mode { 0x28 } else { 0x38 };
        out
    }

    fn quiz_king_latch(&self) -> u8 {
        let pressed = self.quiz_king_buttons.lock().unwrap();
        let mut v = 0u8;
        for i in 0..6usize {
            if pressed[i] != 0 {
                v |= 1 << i;
            }
        }
        v
    }

    /// top rider
    fn top_rider_bits(&mut self) -> u8 {
        if !self.expansion_type.is_top_rider() {
            return 0;
        }
        let mut out = 0u8;
        out |= ((self.top_rider_bs & 1) << 3) as u8;
        out |= ((self.top_rider_boop & 1) << 4) as u8;
        self.top_rider_bs >>= 1;
        self.top_rider_boop >>= 1;
        out
    }

    fn reload_top_rider(&mut self) {
        let buttons = self.top_rider_buttons.lock().unwrap();
        let mut b = 0u8;
        for i in 0..8usize {
            if buttons[i] != 0 {
                b |= 1 << i;
            }
        }
        let mut bss = b as u32;
        bss |= bss << 8;
        bss |= bss << 8;
        self.top_rider_bss = bss;
        self.top_rider_bs = bss;
        self.top_rider_boop = 0;
    }

    /// famicom network controller
    fn fami_net_sys_bits(&mut self) -> u8 {
        if !self.expansion_type.is_fami_net_sys() {
            return 0;
        }
        if self.fami_net_sys_readbit < 24 {
            let bit = ((self.fami_net_sys_data >> self.fami_net_sys_readbit) & 1) as u8;
            self.fami_net_sys_readbit += 1;
            bit << 1
        } else {
            2
        }
    }

    fn pack_fami_net_sys_data(&mut self) {
        if !self.expansion_type.is_fami_net_sys() {
            return;
        }
        let buttons = self.fami_net_sys_buttons.lock().unwrap();
        let mut data = 0u32;
        for i in 0..24usize {
            if buttons[i] != 0 {
                data |= 1 << i;
            }
        }
        self.fami_net_sys_data = data & 0x00BFFFFF;
    }

    /// city patrolman light gun
    fn city_patrolman_read1(&mut self) -> u8 {
        let mut result = 0u8;
        if self.city_patrolman_strobe & 0x02 != 0 {
            result = (self.city_patrolman_shift_reg & 0x03) as u8;
            self.city_patrolman_shift_reg >>= 2;
        }
        result
    }

    fn city_patrolman_write(&mut self, val: u8) {
        self.city_patrolman_time_out = self.city_patrolman_time_out.wrapping_add(1);
        if self.city_patrolman_time_out >= 8192 {
            self.city_patrolman_time_out = 0;
            let input = self.city_patrolman_input.lock().unwrap();
            let pos_x = input[0] as u32;
            let pos_y = input[1] as u32;
            let trigger = input[2] != 0;
            let reload = input[3] != 0;
            drop(input);
            let mut sr: u32 = 0;
            if trigger {
                sr |= 0x10000;
            }
            if reload {
                sr |= 0x20000;
            }
            if pos_x != 0xFF && pos_y != 0xFF {
                sr |= 0x40000;
            }
            for i in 0..8 {
                sr |= ((pos_x >> i) & 1) << (2 * i);
                sr |= ((pos_y >> i) & 1) << (2 * i + 1);
            }
            self.city_patrolman_shift_reg = sr;
        }
        self.city_patrolman_strobe = val & 0x02;
    }

    /// pokkun moguraa
    fn moguraa_read(&mut self) -> u8 {
        let mut result = 0u8;
        if self.moguraa_sel & 0x01 != 0 {
            result = ((self.moguraa_bits >> 8) & 0x0F) as u8;
        } else if self.moguraa_sel & 0x02 != 0 {
            result = ((self.moguraa_bits >> 4) & 0x0F) as u8;
        } else if self.moguraa_sel & 0x04 != 0 {
            result = (self.moguraa_bits & 0x0F) as u8;
        }
        (result ^ 0x0F) << 1
    }

    fn moguraa_write(&mut self, val: u8) {
        self.moguraa_bits = self.moguraa_new_bits();
        self.moguraa_sel = (!val) & 0x07;
    }

    fn moguraa_new_bits(&self) -> u16 {
        let buttons = self.moguraa_buttons.lock().unwrap();
        let mut bits = 0u16;
        for i in 0..12usize {
            if buttons[i] != 0 {
                bits |= 1 << i;
            }
        }
        bits
    }

    /// golden nugget casino
    fn golden_nugget_read1(&mut self) -> u8 {
        let result = (self.golden_nugget_shift & 0x01) as u8;
        self.golden_nugget_shift >>= 1;
        result
    }

    fn golden_nugget_write(&mut self, val: u8) {
        if self.golden_nugget_strobe & 0x01 != 0 && val & 0x01 == 0 {
            let buttons = self.golden_nugget_buttons.lock().unwrap();
            let mut shift = 0u8;
            if buttons[1] != 0 { shift |= 0x01; }
            if buttons[2] != 0 { shift |= 0x02; }
            if buttons[0] != 0 { shift |= 0x08; }
            if buttons[7] != 0 { shift |= 0x10; }
            if buttons[8] != 0 { shift |= 0x20; }
            if buttons[9] != 0 { shift |= 0x40; }
            if buttons[10] != 0 { shift |= 0x80; }
            drop(buttons);
            self.golden_nugget_shift = shift;
        }
        self.golden_nugget_strobe = val & 0x01;
    }

    /// abl pinball
    fn abl_pinball_apply_wheel(&mut self) {
        let mut wheel = self.abl_pinball_wheel.lock().unwrap();
        let delta = *wheel;
        *wheel = 0;
        drop(wheel);
        let d = (delta / 5) as i16;
        if d != 0 {
            self.abl_pinball_plunger = (self.abl_pinball_plunger + d).clamp(1, 20);
        }
    }

    fn abl_pinball_read1(&mut self) -> u8 {
        self.abl_pinball_apply_wheel();
        if self.abl_pinball_plunger == 0 {
            self.abl_pinball_plunger = 1;
        }
        let mut result: i16 = if self.abl_pinball_count < 0 {
            1
        } else if self.abl_pinball_count < self.abl_pinball_plunger {
            0
        } else {
            1
        };
        result |= ((self.abl_pinball_buttons_byte() >> 3) & 0x02) as i16;
        self.abl_pinball_count += 1;
        if self.abl_pinball_count >= self.abl_pinball_plunger {
            self.abl_pinball_count = -1;
        }
        result as u8
    }

    fn abl_pinball_read2(&mut self) -> u8 {
        self.abl_pinball_apply_wheel();
        self.abl_pinball_buttons_byte() & 0x07
    }

    fn abl_pinball_buttons_byte(&self) -> u8 {
        let buttons = self.abl_pinball_buttons.lock().unwrap();
        let mut b = 0u8;
        if buttons[0] != 0 { b |= 0x10; }
        if buttons[1] != 0 { b |= 0x01; }
        if buttons[2] != 0 { b |= 0x02; }
        if buttons[3] != 0 { b |= 0x04; }
        if buttons[4] != 0 { b |= 0x08; }
        b
    }

    /// tv pump
    fn tv_pump_buttons_byte(&mut self) -> u8 {
        let buttons = self.tv_pump_buttons.lock().unwrap();
        let mut b = 0u8;
        for i in 0..6usize {
            if buttons[i] != 0 {
                b |= 1 << i;
            }
        }
        b
    }

    /// triface mahjong
    fn triface_mahjong_read(&mut self) -> u8 {
        let mut result = 0u8;
        if self.triface_mahjong_row < 10 {
            if self.triface_mahjong_column != 0 {
                result = (self.triface_mahjong_keys[self.triface_mahjong_row as usize] & 0xF0) >> 3;
            } else {
                result = (self.triface_mahjong_keys[self.triface_mahjong_row as usize] & 0x0F) << 1;
            }
            result ^= 0x1E;
        }
        result
    }

    fn triface_mahjong_write(&mut self, val: u8) {
        let reset_kb = (val & 0x01) != 0;
        let sel_column = (val & 0x02) != 0;
        let select_key = (val & 0x04) != 0;
        if select_key {
            if self.triface_mahjong_column != 0 && !sel_column {
                self.triface_mahjong_row = self.triface_mahjong_row.wrapping_add(1);
            }
            self.triface_mahjong_column = if sel_column { 1 } else { 0 };
            if reset_kb {
                self.triface_mahjong_row = 0;
            }
        }
    }

    fn triface_mahjong_build_keys(&mut self) {
        const MAP: [(u8, u8, u8, u8); 22] = [
            (0, 0x04, 0, 0x04), (0, 0x02, 0, 0x02), (8, 0x02, 8, 0x02),
            (3, 0x02, 3, 0x02), (6, 0x80, 6, 0x80), (6, 0x04, 6, 0x04),
            (3, 0x04, 3, 0x04), (6, 0x08, 6, 0x08), (8, 0x04, 8, 0x04),
            (3, 0x20, 3, 0x20), (7, 0x20, 7, 0x20), (5, 0x01, 5, 0x01),
            (8, 0x20, 8, 0x20), (1, 0x04, 1, 0x04), (8, 0x01, 8, 0x01),
            (1, 0x04, 1, 0x04), (5, 0x20, 5, 0x20), (8, 0x80, 8, 0x80),
            (0, 0x08, 0, 0x08), (1, 0x02, 1, 0x02), (0, 0x20, 0, 0x20),
            (5, 0x20, 8, 0x04),
        ];
        let pressed = self.triface_mahjong_buttons.lock().unwrap();
        let mut rows = [0u8; 10];
        for (i, m) in MAP.iter().enumerate() {
            if pressed[i] != 0 {
                rows[m.0 as usize] |= m.1;
                rows[m.2 as usize] |= m.3;
            }
        }
        drop(pressed);
        self.triface_mahjong_keys = rows;
    }

    /// mahjong gekitou
    fn mahjong_gekitou_read1(&mut self) -> u8 {
        let result: u8;
        if self.mahjong_gekitou_strobe != 0 {
            self.mahjong_gekitou_bits = self.mahjong_gekitou_new_bits();
            self.mahjong_gekitou_bit_ptr = 0;
            result = self.mahjong_gekitou_bits & 1;
        } else {
            result = if self.mahjong_gekitou_bit_ptr < 8 {
                (self.mahjong_gekitou_bits >> (7 - self.mahjong_gekitou_bit_ptr)) & 1
            } else {
                1
            };
            if self.mahjong_gekitou_bit_ptr < 8 {
                self.mahjong_gekitou_bit_ptr = self.mahjong_gekitou_bit_ptr.wrapping_add(1);
            }
        }
        result
    }

    fn mahjong_gekitou_write(&mut self, val: u8) {
        if self.mahjong_gekitou_strobe != 0 || val & 0x01 != 0 {
            self.mahjong_gekitou_strobe = val & 0x01;
            self.mahjong_gekitou_bits = self.mahjong_gekitou_new_bits();
            self.mahjong_gekitou_bit_ptr = 0;
        }
    }

    fn mahjong_gekitou_new_bits(&self) -> u8 {
        const MAP: [u8; 22] = [
            0x80, 0x40, 0x82, 0x88, 0x81, 0x84, 0xA0, 0x90,
            0x42, 0x48, 0x41, 0x44, 0x60, 0x50, 0x02, 0x50,
            0x08, 0x01, 0x04, 0x20, 0x10, 0x62,
        ];
        let pressed = self.mahjong_gekitou_buttons.lock().unwrap();
        let mut bits = 0u8;
        for (i, p) in pressed.iter().enumerate() {
            if *p != 0 {
                bits |= MAP[i];
            }
        }
        bits
    }

    /// onebus read for expansion port
    fn onebus_read_iop(&mut self, port: u8) -> u8 {
        match port {
            3 => {
                if self.expansion_type.is_abl_pinball() {
                    self.abl_pinball_buttons_byte() & 0x08
                } else if self.expansion_type.is_golden_nugget_casino() {
                    let buttons = self.golden_nugget_buttons.lock().unwrap();
                    let mut b = 0u8;
                    if buttons[3] != 0 { b |= 0x01; }
                    if buttons[6] != 0 { b |= 0x02; }
                    if buttons[5] != 0 { b |= 0x04; }
                    if buttons[4] != 0 { b |= 0x08; }
                    !b
                } else {
                    0
                }
            }
            4 => {
                if self.expansion_type.is_tv_pump() {
                    !self.tv_pump_buttons_byte()
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    /// famicom microphone
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

    /// ascii turbo file
    fn turbo_file_write(&mut self, input: u8) {
        if input & 0x02 == 0 {
            self.turbo_file_position = 0;
        }
        if (input & 0x04 == 0) && (self.turbo_file_last_write & 0x04 != 0) {
            let pos = self.turbo_file_position as usize;
            let bit = pos % 8;
            self.turbo_file_data[pos / 8] &= !(1 << bit);
            self.turbo_file_data[pos / 8] |= (input & 0x01) << bit;
            self.turbo_file_position = (self.turbo_file_position + 1) & 0xFFFF;
        }
        self.turbo_file_last_write = input;
    }

    /// battle box
    fn battle_box_write(&mut self, input: u8) {
        if input & 0x01 != 0 && self.battle_box_last_write & 0x01 == 0 {
            self.battle_box_input_data &= !(1 << self.battle_box_input_bit_position);
            self.battle_box_input_data |= (self.battle_box_output as u16) << self.battle_box_input_bit_position;
            self.battle_box_input_bit_position += 1;
            if self.battle_box_input_bit_position > 15 {
                if self.battle_box_is_write {
                    let addr = if self.battle_box_chip_select != 0 { 0x80 } else { 0 } | self.battle_box_address;
                    let base = (addr as usize) * 2;
                    self.battle_box_data[base..base + 2].copy_from_slice(&self.battle_box_input_data.to_le_bytes());
                    self.battle_box_is_write = false;
                } else {
                    self.battle_box_is_read = false;
                    let address = (self.battle_box_input_data & 0x7F) as u8;
                    let cmd = (((self.battle_box_input_data & 0x7F00) >> 8) as u8) ^ 0x7F;
                    match cmd {
                        0x01 => {
                            self.battle_box_address = address;
                            self.battle_box_is_read = true;
                        }
                        0x06 => {
                            if self.battle_box_write_enabled {
                                self.battle_box_address = address;
                                self.battle_box_is_write = true;
                            }
                        }
                        0x0C => {
                            if self.battle_box_write_enabled {
                                self.battle_box_data = [0; 0x200];
                            }
                        }
                        0x0D => {}
                        0x09 => { self.battle_box_write_enabled = true; }
                        0x0B => { self.battle_box_write_enabled = false; }
                        _ => {}
                    }
                }
                self.battle_box_input_bit_position = 0;
            }
        }
        self.battle_box_last_write = input;
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
            if self.expansion_type.is_bit79_keyboard() {
                self.bit79_keyboard_write(input);
            }
            if self.expansion_type.is_keda_keyboard() {
                self.keda_keyboard_write(input);
            }
            if self.expansion_type.is_kingwon_keyboard() {
                self.kingwon_keyboard_write(input);
            }
            if self.expansion_type.is_zecheng_keyboard() {
                self.zecheng_keyboard_write(input);
            }
            if self.expansion_type.is_pec586_keyboard() {
                let prev = self.pec586_kstrobe;
                if (prev & 0x02) == 0 && (input & 0x02) != 0 {
                    self.pec586_kspos = 0;
                }
                if (prev & 0x01) != 0 && (input & 0x01) == 0 {
                    self.pec586_ksindex = 0;
                }
                if (prev & 0x04) != 0 && (input & 0x04) == 0 {
                    self.pec586_kspos = (self.pec586_kspos + 1) % 13;
                }
                self.pec586_kstrobe = input;
            }
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
            if self.expansion_type.is_turbo_file() {
                self.turbo_file_write(input);
            }
            if self.expansion_type.is_battle_box() {
                self.battle_box_write(input);
            }
            if self.expansion_type.is_city_patrolman() {
                self.city_patrolman_write(input);
            }
            if self.expansion_type.is_moguraa() {
                self.moguraa_write(input);
            }
            if self.expansion_type.is_golden_nugget_casino() {
                self.golden_nugget_write(input);
            }
            if self.expansion_type.is_sharp_c1_cassette() {
                self.tape_output((input & 0x04) != 0);
            }
            if self.expansion_type.is_triface_mahjong() {
                self.triface_mahjong_write(input);
            }
            if self.expansion_type.is_mahjong_gekitou() {
                self.mahjong_gekitou_write(input);
            }
            if self.controller1_type == crate::config::ControllerType::PS2Mouse {
                self.ps2_mouse_write(0, input);
            }
            if self.controller2_type == crate::config::ControllerType::PS2Mouse {
                self.ps2_mouse_write(1, input);
            }
            if self.controller1_type == crate::config::ControllerType::YuxingMouse {
                self.yuxing_mouse_write(0, input);
            }
            if self.controller2_type == crate::config::ControllerType::YuxingMouse {
                self.yuxing_mouse_write(1, input);
            }
            if self.controller1_type == crate::config::ControllerType::BelsonicMouse {
                self.belsonic_mouse_write(0, input);
            }
            if self.controller2_type == crate::config::ControllerType::BelsonicMouse {
                self.belsonic_mouse_write(1, input);
            }
            if self.controller1_type == crate::config::ControllerType::MegaBookMouse {
                self.megabook_mouse_write(0, input);
            }
            if self.controller2_type == crate::config::ControllerType::MegaBookMouse {
                self.megabook_mouse_write(1, input);
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
                if self.expansion_type.is_subor_keyboard() {
                    self.subor_keyboard_row = 0;
                    self.subor_keyboard_column = 0;
                }
                if self.expansion_adapter_type == crate::config::ExpansionAdapterType::TwoPlayer {
                    self.controller_shift_register1 = self.expansion_adapter_ports[0].load(Ordering::Relaxed);
                    self.controller_shift_register2 = self.expansion_adapter_ports[1].load(Ordering::Relaxed);
                } else if self.expansion_adapter_type == crate::config::ExpansionAdapterType::FourPlayer || self.expansion_adapter_type == crate::config::ExpansionAdapterType::HoriFourPlayer {
                    for p in 0..4usize {
                        self.expansion_adapter_shift_register[p] = self.expansion_adapter_ports[p].load(Ordering::Relaxed);
                    }
                    self.controller_shift_register1 = self.controller_port1.load(Ordering::Relaxed);
                    self.controller_shift_register2 = self.controller_port2.load(Ordering::Relaxed);
                }
                {
                    let vb_state_lock = self.virtualboy_state.lock().unwrap();
                    for p in 0..2usize {
                        self.virtualboy_state_buffer[p] = crate::config::build_vb_state(vb_state_lock[p]);
                    }
                }
                // latch accumulated mouse deltas into state
                self.fold_mouse_deltas();
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
                // hori track
                if self.expansion_type == crate::config::ExpansionType::HoriTrack {
                    let hdx = &mut *self.hori_track_dx.lock().unwrap();
                    let hdy = &mut *self.hori_track_dy.lock().unwrap();
                    for p in 0..2usize {
                        let btns = if p == 0 { self.controller_port1.load(Ordering::Relaxed) } else { self.controller_port2.load(Ordering::Relaxed) };
                        let raw_dx = hdx[p].round() as i32;
                        let raw_dy = hdy[p].round() as i32;
                        let clamped_dx = raw_dx.max(-8).min(7);
                        let clamped_dy = raw_dy.max(-8).min(7);
                        hdx[p] -= clamped_dx as f32;
                        hdy[p] -= clamped_dy as f32;
                        let bit_rev_dx = ((clamped_dx & 0x08) >> 3) | ((clamped_dx & 0x04) >> 1) | ((clamped_dx & 0x02) << 1) | ((clamped_dx & 0x01) << 3);
                        let bit_rev_dy = ((clamped_dy & 0x08) >> 3) | ((clamped_dy & 0x04) >> 1) | ((clamped_dy & 0x02) << 1) | ((clamped_dy & 0x01) << 3);
                        let byte1 = ((!bit_rev_dy) & 0x0F) | (((!bit_rev_dx) & 0x0F) << 4);
                        self.hori_track_state[p] = (btns as u32) | ((byte1 as u32) << 8) | (0x09 << 16);
                    }
                }
                // bandai hyper shot
                if self.expansion_type.is_bandai_hyper_shot() {
                    let bh = self.bandai_hyper_buttons.lock().unwrap();
                    let mut byte = 0u32;
                    for i in 0..8usize {
                        if bh[i] != 0 {
                            byte |= 1 << i;
                        }
                    }
                    drop(bh);
                    self.bandai_hyper_state[0] = byte;
                    self.bandai_hyper_readbit[0] = 0;
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
            if self.expansion_type.is_bit79_keyboard() {
                self.bit79_keyboard_write(input);
            }
            if self.expansion_type.is_keda_keyboard() {
                self.keda_keyboard_write(input);
            }
            if self.expansion_type.is_kingwon_keyboard() {
                self.kingwon_keyboard_write(input);
            }
            if self.expansion_type.is_zecheng_keyboard() {
                self.zecheng_keyboard_write(input);
            }
            if self.expansion_type.is_turbo_file() {
                self.turbo_file_write(input);
            }
            if self.expansion_type.is_battle_box() {
                self.battle_box_write(input);
            }
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
            if self.expansion_type.is_exciting_boxing() {
                self.punching_bag_selected_sensors = (input & 0x02) >> 1;
            }
            if self.expansion_type.is_jissen_mahjong() {
                self.jissen_mahjong_row = (input & 0x06) >> 1;
                let prev_strobe = self.jissen_mahjong_strobe;
                self.jissen_mahjong_strobe = (input & 0x01) != 0;
                if prev_strobe && !self.jissen_mahjong_strobe {
                    self.refresh_jissen_mahjong_buffer();
                }
            }
            if self.expansion_type.is_subor_keyboard() {
                let prev_column = self.subor_keyboard_column;
                self.subor_keyboard_column = (input & 0x02) >> 1;
                self.subor_keyboard_enabled = (input & 0x04) != 0;
                if self.subor_keyboard_enabled {
                    if self.subor_keyboard_column == 0 && prev_column != 0 {
                        self.subor_keyboard_row = (self.subor_keyboard_row + 1) % 13;
                    }
                }
            }
            if self.expansion_type.is_pec586_keyboard() {
                let prev = self.pec586_kstrobe;
                if (prev & 0x02) == 0 && (input & 0x02) != 0 {
                    self.pec586_kspos = 0;
                }
                if (prev & 0x01) != 0 && (input & 0x01) == 0 {
                    self.pec586_ksindex = 0;
                }
                if (prev & 0x04) != 0 && (input & 0x04) == 0 {
                    self.pec586_kspos = (self.pec586_kspos + 1) % 13;
                }
                self.pec586_kstrobe = input;
            }
            if self.expansion_type.is_quiz_king() {
                self.quiz_king_funky_mode = (input & 0x04) != 0;
                let new_strobe = (input & 0x01) != 0;
                if self.quiz_king_strobe && !new_strobe {
                    self.quiz_king_data_r = self.quiz_king_latch();
                }
                self.quiz_king_strobe = new_strobe;
            }
            if self.expansion_type.is_top_rider() {
                self.reload_top_rider();
            }
            if self.expansion_type.is_fami_net_sys() {
                let new_strobe = (input & 0x01) != 0;
                if self.fami_net_sys_prev_strobe && !new_strobe {
                    self.pack_fami_net_sys_data();
                    self.fami_net_sys_readbit = 0;
                }
                self.fami_net_sys_prev_strobe = new_strobe;
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

            if cart.mapper_chip.is_study_box() && (address == 0x4200 || address == 0x4202) {
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
    pub(crate) fn store_apu_registers(&mut self, address: u16, input: u8) {
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
            self.ppu_oam_read_latch
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
