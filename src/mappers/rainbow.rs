use crate::cartridge::Cartridge;
use crate::mapper::{mirror_h_or_v, FetchResult, Mapper};
use crate::mappers::flash_s29::FlashS29;
use crate::mappers::rainbow_audio::RainbowAudio;

#[derive(Clone, Copy, Debug, Default)]
pub struct NtControl {
    pub attr_ext_mode: bool,
    pub bg_ext_mode: bool,
    pub fpga_ram_src: u8,
    pub fill_mode: bool,
    pub source: u8,
}

impl NtControl {
    pub fn to_byte(&self) -> u8 {
        (if self.attr_ext_mode { 1 } else { 0 })
            | (if self.bg_ext_mode { 2 } else { 0 })
            | ((self.fpga_ram_src & 0x03) << 2)
            | (if self.fill_mode { 0x20 } else { 0 })
            | ((self.source & 0x03) << 6)
    }

    pub fn from_byte(&mut self, value: u8) {
        self.attr_ext_mode = (value & 0x01) != 0;
        self.bg_ext_mode = (value & 0x02) != 0;
        self.fpga_ram_src = (value & 0x0C) >> 2;
        self.fill_mode = (value & 0x20) != 0;
        self.source = (value & 0xC0) >> 6;
    }
}

pub struct MapperRainbow {
    pub audio: RainbowAudio,
    pub prg_flash: FlashS29,
    pub chr_flash: FlashS29,

    org_prg_rom: Vec<u8>,
    org_chr_rom: Vec<u8>,
    chr_flash_data: Vec<u8>,

    high_banks: [u16; 8],
    low_banks: [u16; 2],
    chr_banks: [u16; 16],

    fpga_ram_bank: u8,

    high_mode: u8,
    low_mode: u8,
    chr_mode: u8,
    chr_source: u8,
    window_enabled: bool,
    sprite_ext_mode: bool,
    bg_ext_mode_offset: u8,

    nt_banks: [u8; 4],
    nt_control: [NtControl; 4],

    fill_mode_tile_index: u8,
    fill_mode_attr_index: u8,

    window_control: NtControl,
    window_bank: u8,
    window_x1: u8,
    window_x2: u8,
    window_y1: u8,
    window_y2: u8,
    window_scroll_x: u8,
    window_scroll_y: u8,
    in_window: bool,

    sl_irq_enabled: bool,
    sl_irq_pending: bool,
    sl_irq_scanline: u8,
    sl_irq_offset: u8,
    irq_active: bool,

    last_ppu_read_addr: u16,
    scanline_counter: i16,
    ppu_idle_counter: u8,
    nt_read_counter: u8,
    ppu_read_counter: u8,

    in_frame: bool,
    in_hblank: bool,
    jitter_counter: u8,

    cpu_irq_counter: u16,
    cpu_irq_reload_value: u16,
    cpu_irq_enabled: bool,
    cpu_irq_pending: bool,
    cpu_irq_enable_after_ack: bool,
    cpu_irq_ack_on_4011: bool,
    cpu_parity: bool,

    fpga_ram_addr: u16,
    fpga_ram_inc: u8,

    nmi_vector_enabled: bool,
    irq_vector_enabled: bool,
    nmi_vector_addr: u16,
    irq_vector_addr: u16,

    override_tile_fetch: bool,
    ext_data: u8,
    nt_fetch_counter: u8,

    sprite_ext_data: [u8; 64],
    oam_pos_y: [u8; 64],
    oam_mappings: [u8; 8],
    sprite_ext_bank: u8,
    large_sprites: bool,
    oam_addr: u8,

    oam_ext_update_page: u8,
    oam_slow_update_page: u8,
    oam_sprite_limit: u8,
    oam_code: [u8; 0x506],
    oam_code_locked: bool,

    esp_enabled: bool,
    wifi_irq_enabled: bool,
    wifi_irq_pending: bool,
    data_sent: bool,
    data_received: bool,
    data_ready: bool,
    send_src_addr: u8,
    recv_dst_addr: u8,

    mapper_ram: [u8; 0x2000],
    #[allow(dead_code)]
    has_battery: bool,
}

impl MapperRainbow {
    pub fn new(_rom: &[u8], prg_size: usize, chr_size: usize, has_battery: bool) -> Self {
        let prg_flash = FlashS29::new(prg_size);
        let chr_flash_size = chr_size.max(0x200000);
        let chr_flash = FlashS29::new(chr_flash_size);
        let chr_flash_data = vec![0xFFu8; chr_flash_size];

        let mut mapper = Self {
            audio: RainbowAudio::new(),
            prg_flash,
            chr_flash,
            org_prg_rom: Vec::new(),
            org_chr_rom: Vec::new(),
            chr_flash_data,
            high_banks: [0; 8],
            low_banks: [0; 2],
            chr_banks: [0; 16],
            fpga_ram_bank: 0,
            high_mode: 0,
            low_mode: 0,
            chr_mode: 0,
            chr_source: 0,
            window_enabled: false,
            sprite_ext_mode: false,
            bg_ext_mode_offset: 0,
            nt_banks: [0; 4],
            nt_control: [NtControl::default(); 4],
            fill_mode_tile_index: 0,
            fill_mode_attr_index: 0,
            window_control: NtControl::default(),
            window_bank: 0,
            window_x1: 0,
            window_x2: 0,
            window_y1: 0,
            window_y2: 0,
            window_scroll_x: 0,
            window_scroll_y: 0,
            in_window: false,
            sl_irq_enabled: false,
            sl_irq_pending: false,
            sl_irq_scanline: 0,
            sl_irq_offset: 0,
            irq_active: false,
            last_ppu_read_addr: 0,
            scanline_counter: -1,
            ppu_idle_counter: 0,
            nt_read_counter: 0,
            ppu_read_counter: 0,
            in_frame: false,
            in_hblank: false,
            jitter_counter: 0,
            cpu_irq_counter: 0,
            cpu_irq_reload_value: 0,
            cpu_irq_enabled: false,
            cpu_irq_pending: false,
            cpu_irq_enable_after_ack: false,
            cpu_irq_ack_on_4011: false,
            cpu_parity: false,
            fpga_ram_addr: 0,
            fpga_ram_inc: 0,
            nmi_vector_enabled: false,
            irq_vector_enabled: false,
            nmi_vector_addr: 0,
            irq_vector_addr: 0,
            override_tile_fetch: false,
            ext_data: 0,
            nt_fetch_counter: 0,
            sprite_ext_data: [0; 64],
            oam_pos_y: [0; 64],
            oam_mappings: [0; 8],
            sprite_ext_bank: 0,
            large_sprites: false,
            oam_addr: 0,
            oam_ext_update_page: 0,
            oam_slow_update_page: 0,
            oam_sprite_limit: 0x3F,
            oam_code: [0; 0x506],
            oam_code_locked: false,
            esp_enabled: false,
            wifi_irq_enabled: false,
            wifi_irq_pending: false,
            data_sent: false,
            data_received: false,
            data_ready: false,
            send_src_addr: 0,
            recv_dst_addr: 0,
            mapper_ram: [0; 0x2000],
            has_battery,
        };

        mapper.reset_registers();
        mapper
    }

    pub fn for_ines(
        _header: &[u8],
        _submapper: u8,
        prg_rom: &[u8],
        chr_rom: &[u8],
        has_battery: bool,
    ) -> Self {
        let mut mapper = Self::new(prg_rom, prg_rom.len(), chr_rom.len(), has_battery);
        mapper.org_prg_rom = prg_rom.to_vec();
        mapper.org_chr_rom = chr_rom.to_vec();
        let copy_len = chr_rom.len().min(mapper.chr_flash_data.len());
        if copy_len > 0 {
            mapper.chr_flash_data[..copy_len].copy_from_slice(&chr_rom[..copy_len]);
        }
        mapper
    }

    fn reset_registers(&mut self) {
        self.write_internal_reg(0x4100, 0x00);
        self.write_internal_reg(0x4108, 0x00);
        self.write_internal_reg(0x4118, 0x00);
        self.write_internal_reg(0x4120, 0x00);
        self.write_internal_reg(0x4130, 0x00);
        self.write_internal_reg(0x4140, 0x00);
        self.write_internal_reg(0x4126, 0x00);
        self.write_internal_reg(0x4127, 0x00);
        self.write_internal_reg(0x4128, 0x01);
        self.write_internal_reg(0x4129, 0x01);
        self.write_internal_reg(0x412E, 0x00);
        self.write_internal_reg(0x412A, 0x00);
        self.write_internal_reg(0x412B, 0x00);
        self.write_internal_reg(0x412C, 0x00);
        self.write_internal_reg(0x412D, 0x00);
        self.write_internal_reg(0x412F, 0x80);
        self.write_internal_reg(0x4241, 0x07);
        self.write_internal_reg(0x4242, 0x06);
        self.write_internal_reg(0x4152, 0x00);
        self.write_internal_reg(0x4153, 0x87);
        self.write_internal_reg(0x415A, 0x00);
        self.write_internal_reg(0x4190, 0x00);
        self.write_internal_reg(0x41A9, 0x00);
        self.write_internal_reg(0x41AA, 0x0F);
    }

    fn update_irq_status(&mut self) {
        let active = (self.cpu_irq_enabled && self.cpu_irq_pending)
            || (self.sl_irq_enabled && self.sl_irq_pending);
        if active {
            if !self.irq_active {
                self.jitter_counter = 0;
            }
            self.irq_active = true;
        } else {
            self.irq_active = false;
        }
    }

    fn ack_cpu_irq(&mut self) {
        self.cpu_irq_enabled = self.cpu_irq_enable_after_ack;
        self.cpu_irq_pending = false;
        self.update_irq_status();
    }

    fn generate_ext_update(&mut self) {
        if self.oam_code_locked {
            return;
        }
        self.oam_code_locked = true;

        let mut i = 2;
        for spr in 0..=self.oam_sprite_limit as usize {
            self.oam_code[i] = 0xA9; i += 1;
            self.oam_code[i] =
                self.mapper_ram[0x1800 + (self.oam_ext_update_page as usize * 0x100) + spr * 4];
            i += 1;

            self.oam_code[i] = 0x8D; i += 1;
            self.oam_code[i] = spr as u8; i += 1;
            self.oam_code[i] = 0x42; i += 1;
        }
        self.oam_code[i] = 0x60;
    }

    fn generate_oam_slow_update(&mut self) {
        if self.oam_code_locked {
            return;
        }
        self.oam_code_locked = true;

        let mut i = 0;
        self.oam_code[i] = 0xA9; i += 1;
        self.oam_code[i] = 0x00; i += 1;

        self.oam_code[i] = 0x8D; i += 1;
        self.oam_code[i] = 0x03; i += 1;
        self.oam_code[i] = 0x20; i += 1;

        for j in 0..(self.oam_sprite_limit as usize + 1) * 4 {
            self.oam_code[i] = 0xA9; i += 1;
            self.oam_code[i] = self.mapper_ram[0x1800 + (self.oam_slow_update_page as usize * 0x100) + j]; i += 1;

            self.oam_code[i] = 0x8D; i += 1; 
            self.oam_code[i] = 0x04; i += 1;
            self.oam_code[i] = 0x20; i += 1;
        }
        self.oam_code[i] = 0x60;
    }

    fn update_in_window_flag(&mut self) {
        let scanline = if self.nt_fetch_counter >= 41 {
            (self.scanline_counter + 1) as u8
        } else {
            self.scanline_counter as u8
        };

        let y_match = if self.window_y1 >= self.window_y2 {
            scanline <= self.window_y2 || scanline > self.window_y1
        } else {
            scanline >= self.window_y1 && scanline <= self.window_y2
        };

        let column = ((self.nt_fetch_counter + 1) % 42) as u8;
        let x_match = if self.window_x1 >= self.window_x2 {
            column <= self.window_x2 || column > self.window_x1
        } else {
            column >= self.window_x1 && column <= self.window_x2
        };

        self.in_window = x_match && y_match;
    }

    fn process_sprite_eval(&mut self) {
        let mut sprite_count = 0;
        let scanline = self.scanline_counter;
        let height = if self.large_sprites { 16 } else { 8 };

        self.oam_mappings.fill(0);

        for i in 0..64u8 {
            let y = self.oam_pos_y[i as usize] as i16;
            if scanline >= y && scanline < y + height {
                self.oam_mappings[sprite_count] = i;
                sprite_count += 1;
                if sprite_count >= 8 {
                    break;
                }
            }
        }
    }

    fn detect_scanline_start(&mut self, addr: u16) {
        if addr >= 0x2000 && addr <= 0x2FFF {
            if self.last_ppu_read_addr == addr {
                self.nt_read_counter += 1;
                if self.nt_read_counter >= 2 {
                    if !self.in_frame {
                        self.in_frame = true;
                        self.scanline_counter = 0;
                    } else {
                        self.scanline_counter += 1;
                    }

                    self.process_sprite_eval();

                    self.nt_fetch_counter = 0;
                    self.ppu_read_counter = 0;
                    self.nt_read_counter = 0;
                    self.in_hblank = false;
                }
            } else {
                self.nt_read_counter = 0;
            }
        } else {
            self.nt_read_counter = 0;
        }
    }

    fn read_chr_direct(&self, addr: usize, chr_rom: &[u8], chr_ram: &[u8]) -> u8 {
        match self.chr_source {
            0 => {
                if !self.chr_flash_data.is_empty() {
                    let len = if !chr_rom.is_empty() {
                        chr_rom.len().min(self.chr_flash_data.len())
                    } else {
                        self.chr_flash_data.len()
                    };
                    self.chr_flash_data[addr % len]
                } else if !chr_rom.is_empty() {
                    chr_rom[addr % chr_rom.len()]
                } else {
                    0
                }
            }
            1 => {
                if !chr_ram.is_empty() {
                    chr_ram[addr % chr_ram.len()]
                } else {
                    0
                }
            }
            _ => self.mapper_ram[addr & 0x1FFF],
        }
    }

    fn internal_read_vram(
        &self,
        addr: u16,
        chr_rom: &[u8],
        chr_ram: &[u8],
        vram: &[u8],
        using_chr_ram: bool,
    ) -> u8 {
        if addr < 0x2000 {
            if self.chr_source == 2 {
                self.mapper_ram[addr as usize & 0x0FFF]
            } else if self.chr_source == 3 {
                vram[(addr as usize) & 0x07FF]
            } else {
                let chr_mode = (self.chr_mode as usize).min(7);
                let chr_bank_size = (0x2000 >> chr_mode) as usize;
                let chr_bank_count = 1 << chr_mode;
                let slot = (addr as usize / chr_bank_size) % chr_bank_count;
                let offset = addr as usize % chr_bank_size;
                let abs_addr = (self.chr_banks[slot] as usize * chr_bank_size) + offset;
                self.read_chr_direct(abs_addr, chr_rom, chr_ram)
            }
        } else if addr < 0x3F00 {
            let quadrant = ((addr >> 10) & 3) as usize;
            let ctrl = self.nt_control[quadrant];
            self.read_nt_data(
                ctrl.source,
                self.nt_banks[quadrant],
                addr,
                chr_rom,
                chr_ram,
                vram,
                using_chr_ram,
            )
        } else {
            0
        }
    }

    fn read_nt_data(
        &self,
        source: u8,
        bank: u8,
        addr: u16,
        chr_rom: &[u8],
        chr_ram: &[u8],
        vram: &[u8],
        _using_chr_ram: bool,
    ) -> u8 {
        let offset = (addr & 0x3FF) as usize;
        match source {
            0 => {
                let vram_offset = ((bank as usize * 0x400) + offset) & 0x7FF;
                vram[vram_offset]
            }
            1 => {
                if !chr_ram.is_empty() {
                    let chr_offset = ((bank as usize * 0x400) + offset) & (chr_ram.len() - 1);
                    chr_ram[chr_offset]
                } else {
                    0
                }
            }
            2 => {
                let fpga_offset = (((bank as usize & 0x03) * 0x400) + offset) & 0x1FFF;
                self.mapper_ram[fpga_offset]
            }
            3 => {
                if !self.chr_flash_data.is_empty() {
                    let len = if !chr_rom.is_empty() {
                        chr_rom.len().min(self.chr_flash_data.len())
                    } else {
                        self.chr_flash_data.len()
                    };
                    let rom_offset = ((bank as usize * 0x400) + offset) % len;
                    self.chr_flash_data[rom_offset]
                } else if !chr_rom.is_empty() {
                    let rom_offset = ((bank as usize * 0x400) + offset) % chr_rom.len();
                    chr_rom[rom_offset]
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    fn write_internal_reg(&mut self, addr: u16, value: u8) {
        match addr {
            0x4100 => {
                self.high_mode = value & 0x07;
                self.low_mode = (value & 0x80) >> 7;
            }
            0x4115 => {
                self.fpga_ram_bank = value & 0x01;
            }
            0x4120 => {
                self.chr_mode = value & 0x07;
                self.window_enabled = (value & 0x10) != 0;
                self.sprite_ext_mode = (value & 0x20) != 0;
                self.chr_source = (value & 0xC0) >> 6;
            }
            0x4121 => {
                self.bg_ext_mode_offset = value & 0x1F;
            }
            0x4124 => {
                self.fill_mode_tile_index = value;
            }
            0x4125 => {
                self.fill_mode_attr_index = value & 0x03;
            }
            0x412E => {
                self.window_bank = value;
            }
            0x412F => {
                self.window_control.attr_ext_mode = (value & 0x01) != 0;
                self.window_control.bg_ext_mode = (value & 0x02) != 0;
                self.window_control.fpga_ram_src = (value & 0x0C) >> 2;
                self.window_control.fill_mode = (value & 0x20) != 0;
                self.window_control.source = 2;
            }
            0x4150 => {
                self.sl_irq_scanline = value;
            }
            0x4151 => {
                self.sl_irq_enabled = true;
                self.update_irq_status();
            }
            0x4152 => {
                self.sl_irq_enabled = false;
                self.sl_irq_pending = false;
                self.update_irq_status();
            }
            0x4153 => {
                self.sl_irq_offset = value.clamp(1, 170);
            }
            0x4158 => {
                self.cpu_irq_reload_value = (self.cpu_irq_reload_value & 0x00FF) | ((value as u16) << 8);
            }
            0x4159 => {
                self.cpu_irq_reload_value = (self.cpu_irq_reload_value & 0xFF00) | (value as u16);
            }
            0x415A => {
                self.cpu_irq_enabled = (value & 0x01) != 0;
                self.cpu_irq_enable_after_ack = (value & 0x02) != 0;
                self.cpu_irq_ack_on_4011 = (value & 0x04) != 0;
                if self.cpu_irq_enabled {
                    self.cpu_irq_counter = self.cpu_irq_reload_value;
                }
                self.update_irq_status();
            }
            0x415B => {
                self.ack_cpu_irq();
            }
            0x4157 => {
                self.cpu_parity = false;
            }
            0x415C => {
                self.fpga_ram_addr = (self.fpga_ram_addr & 0x00FF) | (((value as u16) & 0x1F) << 8);
            }
            0x415D => {
                self.fpga_ram_addr = (self.fpga_ram_addr & 0xFF00) | (value as u16);
            }
            0x415E => {
                self.fpga_ram_inc = value;
            }
            0x415F => {
                self.mapper_ram[self.fpga_ram_addr as usize & 0x1FFF] = value;
                self.fpga_ram_addr = (self.fpga_ram_addr.wrapping_add(self.fpga_ram_inc as u16)) & 0x1FFF;
            }
            0x416B => {
                self.nmi_vector_enabled = (value & 0x01) != 0;
                self.irq_vector_enabled = (value & 0x02) != 0;
            }
            0x416C => {
                self.nmi_vector_addr = (self.nmi_vector_addr & 0x00FF) | ((value as u16) << 8);
            }
            0x416D => {
                self.nmi_vector_addr = (self.nmi_vector_addr & 0xFF00) | (value as u16);
            }
            0x416E => {
                self.irq_vector_addr = (self.irq_vector_addr & 0x00FF) | ((value as u16) << 8);
            }
            0x416F => {
                self.irq_vector_addr = (self.irq_vector_addr & 0xFF00) | (value as u16);
            }
            0x4170 => {
                self.window_x1 = value & 0x1F;
            }
            0x4171 => {
                self.window_x2 = value & 0x1F;
            }
            0x4172 => {
                self.window_y1 = value;
            }
            0x4173 => {
                self.window_y2 = value;
            }
            0x4174 => {
                self.window_scroll_x = value & 0x1F;
            }
            0x4175 => {
                self.window_scroll_y = value;
            }
            0x4190 => {
                self.esp_enabled = (value & 0x01) != 0;
                self.wifi_irq_enabled = (value & 0x02) != 0;
            }
            0x4191 => {
                self.data_received = false;
            }
            0x4192 => {
                self.data_sent = false;
            }
            0x4193 => {
                self.recv_dst_addr = value & 0x07;
            }
            0x4194 => {
                self.send_src_addr = value & 0x07;
            }
            0x4240 => {
                self.sprite_ext_bank = value & 0x07;
            }
            0x4241 => {
                self.oam_slow_update_page = value & 0x07;
            }
            0x4242 => {
                self.oam_ext_update_page = value & 0x07;
            }
            0x4243 => {
                self.oam_sprite_limit = value & 0x3F;
            }
            0x4106..=0x4107 => {
                let idx = (addr - 0x4106) as usize;
                self.low_banks[idx] = (self.low_banks[idx] & 0x00FF) | (((value & 0xCF) as u16) << 8);
            }
            0x4116..=0x4117 => {
                let idx = (addr - 0x4116) as usize;
                self.low_banks[idx] = (self.low_banks[idx] & 0xFF00) | (value as u16);
            }
            0x4108..=0x410F => {
                let idx = (addr - 0x4108) as usize;
                self.high_banks[idx] = (self.high_banks[idx] & 0x00FF) | (((value & 0x87) as u16) << 8);
            }
            0x4118..=0x411F => {
                let idx = (addr - 0x4118) as usize;
                self.high_banks[idx] = (self.high_banks[idx] & 0xFF00) | (value as u16);
            }
            0x4130..=0x413F => {
                let idx = (addr - 0x4130) as usize;
                self.chr_banks[idx] = (self.chr_banks[idx] & 0x00FF) | (((value & 0x1F) as u16) << 8);
            }
            0x4140..=0x414F => {
                let idx = (addr - 0x4140) as usize;
                self.chr_banks[idx] = (self.chr_banks[idx] & 0xFF00) | (value as u16);
            }
            0x4126..=0x4129 => {
                self.nt_banks[(addr - 0x4126) as usize] = value;
            }
            0x412A..=0x412D => {
                self.nt_control[(addr - 0x412A) as usize].from_byte(value);
            }
            0x41A0..=0x41AA => {
                self.audio.write_register(addr, value);
            }
            0x4200..=0x423F => {
                self.sprite_ext_data[(addr - 0x4200) as usize] = value;
            }
            _ => {}
        }
    }

    fn resolve_high_bank(&self, address: u16) -> (u16, usize, usize) {
        match self.high_mode {
            0 => (self.high_banks[0], 0x8000, (address - 0x8000) as usize),
            1 => {
                if address < 0xC000 {
                    (self.high_banks[0], 0x4000, (address - 0x8000) as usize)
                } else {
                    (self.high_banks[4], 0x4000, (address - 0xC000) as usize)
                }
            }
            2 => {
                if address < 0xC000 {
                    (self.high_banks[0], 0x4000, (address - 0x8000) as usize)
                } else if address < 0xE000 {
                    (self.high_banks[4], 0x2000, (address - 0xC000) as usize)
                } else {
                    (self.high_banks[6], 0x2000, (address - 0xE000) as usize)
                }
            }
            3 => {
                if address < 0xA000 {
                    (self.high_banks[0], 0x2000, (address - 0x8000) as usize)
                } else if address < 0xC000 {
                    (self.high_banks[2], 0x2000, (address - 0xA000) as usize)
                } else if address < 0xE000 {
                    (self.high_banks[4], 0x2000, (address - 0xC000) as usize)
                } else {
                    (self.high_banks[6], 0x2000, (address - 0xE000) as usize)
                }
            }
            _ => {
                let slot = ((address - 0x8000) / 0x1000) as usize;
                (self.high_banks[slot.min(7)], 0x1000, (address & 0x0FFF) as usize)
            }
        }
    }
}

impl Mapper for MapperRainbow {
    fn reset(&mut self) {
        self.reset_registers();
    }

    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult {
        if address == 0x4011 {
            if self.audio.output_to_4011 {
                return FetchResult {
                    data: self.audio.get_last_output() << 1,
                    driven: true,
                };
            }
            return FetchResult { data: 0, driven: false };
        }

        match address {
            0x4100 => {
                return FetchResult {
                    data: self.high_mode | (self.low_mode << 7),
                    driven: true,
                };
            }
            0x4120 => {
                let data = self.chr_mode
                    | (if self.window_enabled { 0x10 } else { 0 })
                    | (if self.sprite_ext_mode { 0x20 } else { 0 })
                    | (self.chr_source << 6);
                return FetchResult { data, driven: true };
            }
            0x412A..=0x412D => {
                let data = self.nt_control[(address - 0x412A) as usize].to_byte();
                return FetchResult { data, driven: true };
            }
            0x412F => {
                return FetchResult {
                    data: self.window_control.to_byte(),
                    driven: true,
                };
            }
            0x4150 => {
                let val = if self.in_frame && self.scanline_counter >= 0 {
                    self.scanline_counter as u8
                } else {
                    0
                };
                return FetchResult { data: val, driven: true };
            }
            0x4151 => {
                self.sl_irq_pending = false;
                self.update_irq_status();
                let status = (if self.in_hblank { 0x80 } else { 0 })
                    | (if self.in_frame { 0x40 } else { 0 });
                return FetchResult { data: status, driven: true };
            }
            0x4154 => {
                return FetchResult { data: self.jitter_counter, driven: true };
            }
            0x4157 => {
                let val = if self.cpu_parity { 0x80 } else { 0 };
                return FetchResult { data: val, driven: true };
            }
            0x415F => {
                let value = self.mapper_ram[self.fpga_ram_addr as usize & 0x1FFF];
                self.fpga_ram_addr = (self.fpga_ram_addr.wrapping_add(self.fpga_ram_inc as u16)) & 0x1FFF;
                return FetchResult { data: value, driven: true };
            }
            0x4160 => {
                return FetchResult { data: 0x21, driven: true };
            }
            0x4161 => {
                let status = (if self.sl_irq_pending { 0x80 } else { 0 })
                    | (if self.cpu_irq_pending { 0x40 } else { 0 })
                    | (if self.wifi_irq_pending { 1 } else { 0 });
                return FetchResult { data: status, driven: true };
            }
            0x4190 => {
                let status = (if self.esp_enabled { 1 } else { 0 })
                    | (if self.wifi_irq_enabled { 2 } else { 0 });
                return FetchResult { data: status, driven: true };
            }
            0x4191 => {
                let status = (if self.data_ready { 0x40 } else { 0 })
                    | (if self.data_received { 0x80 } else { 0 });
                return FetchResult { data: status, driven: true };
            }
            0x4192 => {
                let status = if self.data_sent { 0x80 } else { 0 };
                return FetchResult { data: status, driven: true };
            }
            0x4280 => {
                self.generate_oam_slow_update();
            }
            0x4282 => {
                self.generate_ext_update();
            }
            _ => {}
        }

        if address >= 0x4280 && address < 0x4800 {
            if address >= 0x4286 {
                self.oam_code_locked = false;
            }
            let idx = (address - 0x4280) as usize;
            let byte = if idx < self.oam_code.len() { self.oam_code[idx] } else { 0 };
            return FetchResult { data: byte, driven: true };
        }

        if address >= 0x4800 && address < 0x5000 {
            let offset = 0x1800 + (address - 0x4800) as usize;
            return FetchResult { data: self.mapper_ram[offset & 0x1FFF], driven: true };
        }

        if address >= 0x5000 && address < 0x6000 {
            let offset = (self.fpga_ram_bank as usize * 0x1000) + (address - 0x5000) as usize;
            return FetchResult { data: self.mapper_ram[offset & 0x1FFF], driven: true };
        }

        if address >= 0x6000 && address < 0x8000 {
            let (bank_reg, bank_size, offset_in_bank) = if self.low_mode != 0 {
                if address < 0x7000 {
                    (self.low_banks[0], 0x1000, (address - 0x6000) as usize)
                } else {
                    (self.low_banks[1], 0x1000, (address - 0x7000) as usize)
                }
            } else {
                (self.low_banks[0], 0x2000, (address - 0x6000) as usize)
            };

            let source = (bank_reg & 0xC000) >> 14;
            match source {
                0 | 1 => {
                    let abs_addr = ((bank_reg & 0x7FFF) as usize * bank_size) + offset_in_bank;
                    if self.prg_flash.is_software_id_mode() {
                        let val = self.prg_flash.read(abs_addr as u32).unwrap_or_else(|| {
                            if !cart.prg_rom.is_empty() {
                                cart.prg_rom[abs_addr % cart.prg_rom.len()]
                            } else {
                                0
                            }
                        });
                        return FetchResult { data: val, driven: true };
                    } else {
                        let val = if !cart.prg_rom.is_empty() {
                            cart.prg_rom[abs_addr % cart.prg_rom.len()]
                        } else {
                            0
                        };
                        return FetchResult { data: val, driven: true };
                    }
                }
                2 => {
                    let abs_addr = ((bank_reg & 0x3FFF) as usize * bank_size) + offset_in_bank;
                    let val = if !cart.prg_ram.is_empty() {
                        cart.prg_ram[abs_addr % cart.prg_ram.len()]
                    } else {
                        0
                    };
                    return FetchResult { data: val, driven: true };
                }
                3 => {
                    let abs_addr = ((bank_reg & 0x3FFF) as usize * bank_size) + offset_in_bank;
                    return FetchResult { data: self.mapper_ram[abs_addr & 0x1FFF], driven: true };
                }
                _ => {}
            }
        }

        if address >= 0x8000 {
            if address == 0xFFFA || address == 0xFFFB {
                self.in_frame = false;
                self.last_ppu_read_addr = 0;
                self.scanline_counter = 0;
                self.sl_irq_pending = false;
                self.update_irq_status();

                if self.nmi_vector_enabled {
                    let val = if (address & 1) != 0 {
                        (self.nmi_vector_addr >> 8) as u8
                    } else {
                        (self.nmi_vector_addr & 0xFF) as u8
                    };
                    return FetchResult { data: val, driven: true };
                }
            } else if (address == 0xFFFE || address == 0xFFFF) && self.irq_vector_enabled {
                let val = if (address & 1) != 0 {
                    (self.irq_vector_addr >> 8) as u8
                } else {
                    (self.irq_vector_addr & 0xFF) as u8
                };
                return FetchResult { data: val, driven: true };
            }

            let (reg, size, offset) = self.resolve_high_bank(address);
            if (reg & 0x8000) != 0 {
                let abs_addr = ((reg & 0x7FFF) as usize * size) + offset;
                let val = if !cart.prg_ram.is_empty() {
                    cart.prg_ram[abs_addr % cart.prg_ram.len()]
                } else {
                    0
                };
                return FetchResult { data: val, driven: true };
            } else {
                let abs_addr = ((reg & 0x7FFF) as usize * size) + offset;
                if self.prg_flash.is_software_id_mode() {
                    let val = self.prg_flash.read(abs_addr as u32).unwrap_or_else(|| {
                        if !cart.prg_rom.is_empty() {
                            cart.prg_rom[abs_addr % cart.prg_rom.len()]
                        } else {
                            0
                        }
                    });
                    return FetchResult { data: val, driven: true };
                } else {
                    let val = if !cart.prg_rom.is_empty() {
                        cart.prg_rom[abs_addr % cart.prg_rom.len()]
                    } else {
                        0
                    };
                    return FetchResult { data: val, driven: true };
                }
            }
        }

        FetchResult { data: 0, driven: false }
    }

    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8) {
        if address >= 0x4100 && address <= 0x4243 {
            self.write_internal_reg(address, data);
            return;
        }

        if address >= 0x4800 && address < 0x5000 {
            let offset = 0x1800 + (address - 0x4800) as usize;
            self.mapper_ram[offset & 0x1FFF] = data;
            return;
        }

        if address >= 0x5000 && address < 0x6000 {
            let offset = (self.fpga_ram_bank as usize * 0x1000) + (address - 0x5000) as usize;
            self.mapper_ram[offset & 0x1FFF] = data;
            return;
        }

        if address >= 0x6000 && address < 0x8000 {
            let (bank_reg, bank_size, offset_in_bank) = if self.low_mode != 0 {
                if address < 0x7000 {
                    (self.low_banks[0], 0x1000, (address - 0x6000) as usize)
                } else {
                    (self.low_banks[1], 0x1000, (address - 0x7000) as usize)
                }
            } else {
                (self.low_banks[0], 0x2000, (address - 0x6000) as usize)
            };

            let source = (bank_reg & 0xC000) >> 14;
            match source {
                0 | 1 => {
                    let abs_addr = ((bank_reg & 0x7FFF) as usize * bank_size) + offset_in_bank;
                    self.prg_flash.write(&mut cart.prg_rom, abs_addr, data);
                }
                2 => {
                    if !cart.prg_ram.is_empty() {
                        let abs_addr = ((bank_reg & 0x3FFF) as usize * bank_size) + offset_in_bank;
                        let len = cart.prg_ram.len();
                        cart.prg_ram[abs_addr % len] = data;
                    }
                }
                3 => {
                    let abs_addr = ((bank_reg & 0x3FFF) as usize * bank_size) + offset_in_bank;
                    self.mapper_ram[abs_addr & 0x1FFF] = data;
                }
                _ => {}
            }
            return;
        }

        if address >= 0x8000 {
            let (reg, size, offset) = self.resolve_high_bank(address);
            if (reg & 0x8000) != 0 {
                if !cart.prg_ram.is_empty() {
                    let abs_addr = ((reg & 0x7FFF) as usize * size) + offset;
                    let len = cart.prg_ram.len();
                    cart.prg_ram[abs_addr % len] = data;
                }
            } else {
                let abs_addr = ((reg & 0x7FFF) as usize * size) + offset;
                self.prg_flash.write(&mut cart.prg_rom, abs_addr, data);
            }
        }
    }

    fn handle_cpu_write(&mut self, address: u16, data: u8) {
        match address {
            0x2000 => {
                self.large_sprites = (data & 0x20) != 0;
            }
            0x2003 => {
                self.oam_addr = data;
            }
            0x2004 => {
                if (self.oam_addr & 0x03) == 0 {
                    self.oam_pos_y[(self.oam_addr >> 2) as usize] = data;
                }
                self.oam_addr = self.oam_addr.wrapping_add(1);
            }
            _ => {}
        }
    }

    fn handle_cpu_read(&mut self, address: u16) {
        if address == 0x4011 && self.cpu_irq_ack_on_4011 {
            self.ack_cpu_irq();
        }
    }

    fn mirror_nametable(&self, _cart: &Cartridge, address: u16) -> u16 {
        mirror_h_or_v(false, address)
    }

    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        chr_rom: &[u8],
        _prg_ram: &[u8],
        chr_ram: &[u8],
        _prg_vram: &[u8],
        using_chr_ram: bool,
        _nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        vram: &[u8],
    ) -> (u8, u16) {
        let addr = (ppu_address_bus & 0x3F00) | (ppu_octal_latch as u16);
        let mut new_addr_bus = ppu_address_bus & 0xFF00;

        if self.chr_flash.is_software_id_mode() && self.chr_source == 0 {
            if let Some(id) = self.chr_flash.read(addr as u32) {
                new_addr_bus |= id as u16;
                return (id, new_addr_bus);
            }
        }

        self.ppu_read_counter = self.ppu_read_counter.wrapping_add(1);
        self.detect_scanline_start(addr);

        if self.sl_irq_scanline as i16 == self.scanline_counter && self.sl_irq_offset == self.ppu_read_counter {
            self.sl_irq_pending = true;
            self.update_irq_status();
        }

        self.ppu_idle_counter = 3;
        self.last_ppu_read_addr = addr;

        if !self.in_frame {
            let data = self.internal_read_vram(addr, chr_rom, chr_ram, vram, using_chr_ram);
            new_addr_bus |= data as u16;
            return (data, new_addr_bus);
        }

        let is_nt_fetch = addr >= 0x2000 && addr <= 0x2FFF;
        let data = if is_nt_fetch {
            let is_attribute_fetch = (addr & 0x3FF) >= 0x3C0;
            if !is_attribute_fetch {
                self.nt_fetch_counter += 1;
                if self.nt_fetch_counter == 33 {
                    self.in_hblank = true;
                }
                self.in_window = false;
                if self.window_enabled {
                    self.update_in_window_flag();
                }
            }

            let ctrl = if self.in_window {
                self.window_control
            } else {
                self.nt_control[((addr >> 10) & 0x03) as usize]
            };

            let mut fetch_addr = addr;
            let mut shift = 0;
            if self.in_window {
                let scanline = if self.nt_fetch_counter >= 41 {
                    (self.scanline_counter + 1) as u8
                } else {
                    self.scanline_counter as u8
                };
                let window_scanline = (scanline as usize + self.window_scroll_y as usize) % 240;
                let column = ((self.nt_fetch_counter + 1) % 42) as usize;
                let nt_addr = ((window_scanline / 8) * 32) + ((column + self.window_scroll_x as usize) & 0x1F);
                if !is_attribute_fetch {
                    fetch_addr = nt_addr as u16;
                } else {
                    fetch_addr = 0x3C0 + ((((window_scanline >> 2) & 0xF8) | (((column + self.window_scroll_x as usize) & 0x1F) >> 2)) as u16);
                    shift = ((nt_addr >> 4) & 0x04) | (nt_addr & 0x02);
                }
            }

            if !is_attribute_fetch {
                self.override_tile_fetch = ctrl.bg_ext_mode;
                let has_ext_mode = ctrl.attr_ext_mode || ctrl.bg_ext_mode;
                if has_ext_mode {
                    let ram_offset = (ctrl.fpga_ram_src as usize * 0x400) + (fetch_addr as usize & 0x3FF);
                    self.ext_data = self.mapper_ram[ram_offset & 0x1FFF];
                }
                if ctrl.fill_mode {
                    self.fill_mode_tile_index
                } else if self.in_window {
                    let ram_offset = (self.window_bank as usize * 0x400) + (fetch_addr as usize & 0x3FF);
                    self.mapper_ram[ram_offset & 0x1FFF]
                } else {
                    self.read_nt_data(
                        ctrl.source,
                        self.nt_banks[((addr >> 10) & 0x03) as usize],
                        fetch_addr,
                        chr_rom,
                        chr_ram,
                        vram,
                        using_chr_ram,
                    )
                }
            } else if ctrl.attr_ext_mode {
                let attr = (self.ext_data & 0xC0) >> 6;
                attr * 0x55
            } else if ctrl.fill_mode {
                self.fill_mode_attr_index * 0x55
            } else if self.in_window {
                let ram_offset = (self.window_bank as usize * 0x400) + (fetch_addr as usize & 0x3FF);
                let palette = (self.mapper_ram[ram_offset & 0x1FFF] >> shift) & 0x03;
                palette * 0x55
            } else {
                self.read_nt_data(
                    ctrl.source,
                    self.nt_banks[((addr >> 10) & 0x03) as usize],
                    fetch_addr,
                    chr_rom,
                    chr_ram,
                    vram,
                    using_chr_ram,
                )
            }
        } else {
            let is_bg_fetch = self.nt_fetch_counter < 33 || self.nt_fetch_counter >= 41;
            if self.in_window && is_bg_fetch {
                let scanline = if self.nt_fetch_counter >= 41 {
                    (self.scanline_counter + 1) as u8
                } else {
                    self.scanline_counter as u8
                };
                let window_scanline = (scanline as usize + self.window_scroll_y as usize) % 240;
                if self.override_tile_fetch {
                    let chr_addr = ((addr & 0xFF8) as usize)
                        | (window_scanline & 0x07)
                        | ((self.ext_data as usize & 0x3F) << 12)
                        | ((self.bg_ext_mode_offset as usize) << 18);
                    self.read_chr_direct(chr_addr, chr_rom, chr_ram)
                } else {
                    let chr_addr = ((addr & 0x1FF8) as usize) | (window_scanline & 0x07);
                    self.internal_read_vram(chr_addr as u16, chr_rom, chr_ram, vram, using_chr_ram)
                }
            } else if self.override_tile_fetch && is_bg_fetch {
                let chr_addr = (addr as usize & 0xFFF)
                    | ((self.ext_data as usize & 0x3F) << 12)
                    | ((self.bg_ext_mode_offset as usize) << 18);
                self.read_chr_direct(chr_addr, chr_rom, chr_ram)
            } else if self.sprite_ext_mode && !is_bg_fetch {
                let fetch_idx = (self.nt_fetch_counter.saturating_sub(33) as usize).min(7);
                let sprite_index = self.oam_mappings[fetch_idx] as usize;
                let chr_addr = if self.large_sprites {
                    ((self.sprite_ext_bank as usize) << 21)
                        | ((self.sprite_ext_data[sprite_index] as usize) << 13)
                        | (addr as usize & 0x1FFF)
                } else {
                    ((self.sprite_ext_bank as usize) << 20)
                        | ((self.sprite_ext_data[sprite_index] as usize) << 12)
                        | (addr as usize & 0xFFF)
                };
                self.read_chr_direct(chr_addr, chr_rom, chr_ram)
            } else {
                self.internal_read_vram(addr, chr_rom, chr_ram, vram, using_chr_ram)
            }
        };

        new_addr_bus |= data as u16;
        (data, new_addr_bus)
    }

    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        let addr = address & 0x3FFF;
        if addr < 0x2000 {
            if self.chr_source == 2 {
                self.mapper_ram[addr as usize & 0x0FFF] = data;
            } else if self.chr_source == 3 {
                vram[(addr as usize) & 0x07FF] = data;
            } else if self.chr_source == 1 {
                if !cart.chr_ram.is_empty() {
                    let chr_mode = (self.chr_mode as usize).min(7);
                    let chr_bank_size = (0x2000 >> chr_mode) as usize;
                    let chr_bank_count = 1 << chr_mode;
                    let slot = (addr as usize / chr_bank_size) % chr_bank_count;
                    let offset = addr as usize % chr_bank_size;
                    let abs_addr = (self.chr_banks[slot] as usize * chr_bank_size) + offset;
                    let len = cart.chr_ram.len();
                    cart.chr_ram[abs_addr & (len - 1)] = data;
                }
            } else {
                let chr_mode = (self.chr_mode as usize).min(7);
                let chr_bank_size = (0x2000 >> chr_mode) as usize;
                let chr_bank_count = 1 << chr_mode;
                let slot = (addr as usize / chr_bank_size) % chr_bank_count;
                let offset = addr as usize % chr_bank_size;
                let abs_addr = (self.chr_banks[slot] as usize * chr_bank_size) + offset;
                self.chr_flash
                    .write(&mut self.chr_flash_data, abs_addr, data);
            }
        } else if addr >= 0x2000 && addr < 0x3F00 {
            let quadrant = ((addr >> 10) & 3) as usize;
            let ctrl = self.nt_control[quadrant];
            let offset = (addr & 0x3FF) as usize;
            let bank = self.nt_banks[quadrant];
            match ctrl.source {
                0 => {
                    let vram_offset = ((bank as usize * 0x400) + offset) & 0x7FF;
                    vram[vram_offset] = data;
                }
                1 => {
                    if !cart.chr_ram.is_empty() {
                        let chr_offset = ((bank as usize * 0x400) + offset) & (cart.chr_ram.len() - 1);
                        cart.chr_ram[chr_offset] = data;
                    }
                }
                2 => {
                    let fpga_offset = (((bank as usize & 0x03) * 0x400) + offset) & 0x1FFF;
                    self.mapper_ram[fpga_offset] = data;
                }
                _ => {}
            }
        }
    }

    fn cpu_clock(&mut self, _cycles: u8) -> bool {
        self.cpu_parity = !self.cpu_parity;
        self.jitter_counter = self.jitter_counter.wrapping_add(1);
        self.audio.clock();

        if self.cpu_irq_enabled && self.cpu_irq_counter > 0 {
            self.cpu_irq_counter -= 1;
            if self.cpu_irq_counter == 0 {
                self.cpu_irq_counter = self.cpu_irq_reload_value;
                self.cpu_irq_pending = true;
                self.update_irq_status();
            }
        }

        if self.ppu_idle_counter > 0 {
            self.ppu_idle_counter -= 1;
            if self.ppu_idle_counter == 0 {
                self.in_frame = false;
                self.in_hblank = false;
                self.scanline_counter = -1;
                self.nt_fetch_counter = 0;
                self.nt_read_counter = 0;
                self.oam_addr = 0;
            }
        }

        self.irq_active
    }

    fn cpu_clock_irq_level(&self) -> bool {
        true
    }

    fn audio_sample(&self) -> f32 {
        self.audio.sample()
    }

    fn battery_save_data(&self, cart: &Cartridge) -> Option<Vec<u8>> {
        let mut save = Vec::new();
        save.extend_from_slice(&self.mapper_ram);
        save.extend_from_slice(&(cart.prg_ram.len() as u32).to_le_bytes());
        save.extend_from_slice(&cart.prg_ram);
        save.extend_from_slice(&(cart.prg_rom.len() as u32).to_le_bytes());
        save.extend_from_slice(&cart.prg_rom);
        save.extend_from_slice(&(cart.chr_rom.len() as u32).to_le_bytes());
        save.extend_from_slice(&cart.chr_rom);
        save.extend_from_slice(&(self.chr_flash_data.len() as u32).to_le_bytes());
        save.extend_from_slice(&self.chr_flash_data);
        Some(save)
    }

    fn load_battery_save(&mut self, cart: &mut Cartridge, data: &[u8]) {
        let mut offset = 0;
        if data.len() < 0x2000 {
            return;
        }
        self.mapper_ram.copy_from_slice(&data[0..0x2000]);
        offset += 0x2000;

        if offset + 4 <= data.len() {
            let prg_ram_len = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;
            if offset + prg_ram_len <= data.len() {
                if cart.prg_ram.len() == prg_ram_len {
                    cart.prg_ram.copy_from_slice(&data[offset..offset + prg_ram_len]);
                }
                offset += prg_ram_len;
            }
        }

        if offset + 4 <= data.len() {
            let prg_rom_len = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;
            if offset + prg_rom_len <= data.len() {
                if cart.prg_rom.len() == prg_rom_len {
                    cart.prg_rom.copy_from_slice(&data[offset..offset + prg_rom_len]);
                }
                offset += prg_rom_len;
            }
        }

        if offset + 4 <= data.len() {
            let chr_rom_len = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;
            if offset + chr_rom_len <= data.len() {
                if cart.chr_rom.len() == chr_rom_len {
                    cart.chr_rom.copy_from_slice(&data[offset..offset + chr_rom_len]);
                }
                offset += chr_rom_len;
            }
        }

        if offset + 4 <= data.len() {
            let chr_flash_len = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;
            if offset + chr_flash_len <= data.len() {
                if self.chr_flash_data.len() == chr_flash_len {
                    self.chr_flash_data.copy_from_slice(&data[offset..offset + chr_flash_len]);
                }
            }
        }
    }

    fn save_mapper_registers(&self, _cart: &Cartridge) -> Vec<u8> {
        let mut s = Vec::new();
        for b in &self.high_banks {
            s.extend_from_slice(&b.to_le_bytes());
        }
        for b in &self.low_banks {
            s.extend_from_slice(&b.to_le_bytes());
        }
        for b in &self.chr_banks {
            s.extend_from_slice(&b.to_le_bytes());
        }
        s.push(self.fpga_ram_bank);
        s.push(self.high_mode);
        s.push(self.low_mode);
        s.push(self.chr_mode);
        s.push(self.chr_source);
        s.push(if self.window_enabled { 1 } else { 0 });
        s.push(if self.sprite_ext_mode { 1 } else { 0 });
        s.push(self.bg_ext_mode_offset);
        s.extend_from_slice(&self.nt_banks);
        for ctrl in &self.nt_control {
            s.push(ctrl.to_byte());
        }
        s.push(self.fill_mode_tile_index);
        s.push(self.fill_mode_attr_index);
        s.push(self.window_control.to_byte());
        s.push(self.window_bank);
        s.push(self.window_x1);
        s.push(self.window_x2);
        s.push(self.window_y1);
        s.push(self.window_y2);
        s.push(self.window_scroll_x);
        s.push(self.window_scroll_y);
        s.push(if self.in_window { 1 } else { 0 });
        s.push(if self.sl_irq_enabled { 1 } else { 0 });
        s.push(if self.sl_irq_pending { 1 } else { 0 });
        s.push(self.sl_irq_scanline);
        s.push(self.sl_irq_offset);
        s.push(if self.irq_active { 1 } else { 0 });
        s.extend_from_slice(&self.last_ppu_read_addr.to_le_bytes());
        s.extend_from_slice(&self.scanline_counter.to_le_bytes());
        s.push(self.ppu_idle_counter);
        s.push(self.nt_read_counter);
        s.push(self.ppu_read_counter);
        s.push(if self.in_frame { 1 } else { 0 });
        s.push(if self.in_hblank { 1 } else { 0 });
        s.push(self.jitter_counter);
        s.extend_from_slice(&self.cpu_irq_counter.to_le_bytes());
        s.extend_from_slice(&self.cpu_irq_reload_value.to_le_bytes());
        s.push(if self.cpu_irq_enabled { 1 } else { 0 });
        s.push(if self.cpu_irq_pending { 1 } else { 0 });
        s.push(if self.cpu_irq_enable_after_ack { 1 } else { 0 });
        s.push(if self.cpu_irq_ack_on_4011 { 1 } else { 0 });
        s.push(if self.cpu_parity { 1 } else { 0 });
        s.extend_from_slice(&self.fpga_ram_addr.to_le_bytes());
        s.push(self.fpga_ram_inc);
        s.push(if self.nmi_vector_enabled { 1 } else { 0 });
        s.push(if self.irq_vector_enabled { 1 } else { 0 });
        s.extend_from_slice(&self.nmi_vector_addr.to_le_bytes());
        s.extend_from_slice(&self.irq_vector_addr.to_le_bytes());
        s.push(if self.override_tile_fetch { 1 } else { 0 });
        s.push(self.ext_data);
        s.push(self.nt_fetch_counter);
        s.extend_from_slice(&self.sprite_ext_data);
        s.extend_from_slice(&self.oam_pos_y);
        s.extend_from_slice(&self.oam_mappings);
        s.push(self.sprite_ext_bank);
        s.push(if self.large_sprites { 1 } else { 0 });
        s.push(self.oam_addr);
        s.push(self.oam_ext_update_page);
        s.push(self.oam_slow_update_page);
        s.push(self.oam_sprite_limit);
        s.extend_from_slice(&self.oam_code);
        s.push(if self.oam_code_locked { 1 } else { 0 });
        s.push(if self.esp_enabled { 1 } else { 0 });
        s.push(if self.wifi_irq_enabled { 1 } else { 0 });
        s.push(if self.wifi_irq_pending { 1 } else { 0 });
        s.push(if self.data_sent { 1 } else { 0 });
        s.push(if self.data_received { 1 } else { 0 });
        s.push(if self.data_ready { 1 } else { 0 });
        s.push(self.send_src_addr);
        s.push(self.recv_dst_addr);
        s.extend_from_slice(&self.mapper_ram);
        s.extend_from_slice(&(self.chr_flash_data.len() as u32).to_le_bytes());
        s.extend_from_slice(&self.chr_flash_data);
        s.extend_from_slice(&self.prg_flash.serialize());
        s.extend_from_slice(&self.chr_flash.serialize());
        s.extend_from_slice(&self.audio.serialize());
        s
    }

    fn load_mapper_registers(&mut self, _cart: &mut Cartridge, state: &[u8], start: usize) -> usize {
        let mut offset = start;
        for b in &mut self.high_banks {
            if offset + 2 > state.len() { return offset; }
            *b = u16::from_le_bytes([state[offset], state[offset + 1]]);
            offset += 2;
        }
        for b in &mut self.low_banks {
            if offset + 2 > state.len() { return offset; }
            *b = u16::from_le_bytes([state[offset], state[offset + 1]]);
            offset += 2;
        }
        for b in &mut self.chr_banks {
            if offset + 2 > state.len() { return offset; }
            *b = u16::from_le_bytes([state[offset], state[offset + 1]]);
            offset += 2;
        }
        if offset + 11 > state.len() { return offset; }
        self.fpga_ram_bank = state[offset]; offset += 1;
        self.high_mode = state[offset]; offset += 1;
        self.low_mode = state[offset]; offset += 1;
        self.chr_mode = state[offset]; offset += 1;
        self.chr_source = state[offset]; offset += 1;
        self.window_enabled = state[offset] != 0; offset += 1;
        self.sprite_ext_mode = state[offset] != 0; offset += 1;
        self.bg_ext_mode_offset = state[offset]; offset += 1;
        self.nt_banks.copy_from_slice(&state[offset..offset + 4]); offset += 4;
        for ctrl in &mut self.nt_control {
            if offset >= state.len() { return offset; }
            ctrl.from_byte(state[offset]); offset += 1;
        }
        if offset + 16 > state.len() { return offset; }
        self.fill_mode_tile_index = state[offset]; offset += 1;
        self.fill_mode_attr_index = state[offset]; offset += 1;
        self.window_control.from_byte(state[offset]); offset += 1;
        self.window_bank = state[offset]; offset += 1;
        self.window_x1 = state[offset]; offset += 1;
        self.window_x2 = state[offset]; offset += 1;
        self.window_y1 = state[offset]; offset += 1;
        self.window_y2 = state[offset]; offset += 1;
        self.window_scroll_x = state[offset]; offset += 1;
        self.window_scroll_y = state[offset]; offset += 1;
        self.in_window = state[offset] != 0; offset += 1;
        self.sl_irq_enabled = state[offset] != 0; offset += 1;
        self.sl_irq_pending = state[offset] != 0; offset += 1;
        self.sl_irq_scanline = state[offset]; offset += 1;
        self.sl_irq_offset = state[offset]; offset += 1;
        self.irq_active = state[offset] != 0; offset += 1;

        if offset + 20 > state.len() { return offset; }
        self.last_ppu_read_addr = u16::from_le_bytes([state[offset], state[offset + 1]]); offset += 2;
        self.scanline_counter = i16::from_le_bytes([state[offset], state[offset + 1]]); offset += 2;
        self.ppu_idle_counter = state[offset]; offset += 1;
        self.nt_read_counter = state[offset]; offset += 1;
        self.ppu_read_counter = state[offset]; offset += 1;
        self.in_frame = state[offset] != 0; offset += 1;
        self.in_hblank = state[offset] != 0; offset += 1;
        self.jitter_counter = state[offset]; offset += 1;
        self.cpu_irq_counter = u16::from_le_bytes([state[offset], state[offset + 1]]); offset += 2;
        self.cpu_irq_reload_value = u16::from_le_bytes([state[offset], state[offset + 1]]); offset += 2;
        self.cpu_irq_enabled = state[offset] != 0; offset += 1;
        self.cpu_irq_pending = state[offset] != 0; offset += 1;
        self.cpu_irq_enable_after_ack = state[offset] != 0; offset += 1;
        self.cpu_irq_ack_on_4011 = state[offset] != 0; offset += 1;
        self.cpu_parity = state[offset] != 0; offset += 1;

        if offset + 15 > state.len() { return offset; }
        self.fpga_ram_addr = u16::from_le_bytes([state[offset], state[offset + 1]]); offset += 2;
        self.fpga_ram_inc = state[offset]; offset += 1;
        self.nmi_vector_enabled = state[offset] != 0; offset += 1;
        self.irq_vector_enabled = state[offset] != 0; offset += 1;
        self.nmi_vector_addr = u16::from_le_bytes([state[offset], state[offset + 1]]); offset += 2;
        self.irq_vector_addr = u16::from_le_bytes([state[offset], state[offset + 1]]); offset += 2;
        self.override_tile_fetch = state[offset] != 0; offset += 1;
        self.ext_data = state[offset]; offset += 1;
        self.nt_fetch_counter = state[offset]; offset += 1;

        if offset + 64 + 64 + 8 > state.len() { return offset; }
        self.sprite_ext_data.copy_from_slice(&state[offset..offset + 64]); offset += 64;
        self.oam_pos_y.copy_from_slice(&state[offset..offset + 64]); offset += 64;
        self.oam_mappings.copy_from_slice(&state[offset..offset + 8]); offset += 8;

        if offset + 7 > state.len() { return offset; }
        self.sprite_ext_bank = state[offset]; offset += 1;
        self.large_sprites = state[offset] != 0; offset += 1;
        self.oam_addr = state[offset]; offset += 1;
        self.oam_ext_update_page = state[offset]; offset += 1;
        self.oam_slow_update_page = state[offset]; offset += 1;
        self.oam_sprite_limit = state[offset]; offset += 1;

        if offset + 0x506 > state.len() { return offset; }
        self.oam_code.copy_from_slice(&state[offset..offset + 0x506]); offset += 0x506;

        if offset + 9 > state.len() { return offset; }
        self.oam_code_locked = state[offset] != 0; offset += 1;
        self.esp_enabled = state[offset] != 0; offset += 1;
        self.wifi_irq_enabled = state[offset] != 0; offset += 1;
        self.wifi_irq_pending = state[offset] != 0; offset += 1;
        self.data_sent = state[offset] != 0; offset += 1;
        self.data_received = state[offset] != 0; offset += 1;
        self.data_ready = state[offset] != 0; offset += 1;
        self.send_src_addr = state[offset]; offset += 1;
        self.recv_dst_addr = state[offset]; offset += 1;

        if offset + 0x2000 > state.len() { return offset; }
        self.mapper_ram.copy_from_slice(&state[offset..offset + 0x2000]); offset += 0x2000;

        if offset + 4 > state.len() { return offset; }
        let chr_flash_len = u32::from_le_bytes([state[offset], state[offset + 1], state[offset + 2], state[offset + 3]]) as usize;
        offset += 4;
        if offset + chr_flash_len <= state.len() && self.chr_flash_data.len() == chr_flash_len {
            self.chr_flash_data.copy_from_slice(&state[offset..offset + chr_flash_len]);
        }
        offset = offset.saturating_add(chr_flash_len);

        let _ = self.prg_flash.deserialize(state, &mut offset);
        let _ = self.chr_flash.deserialize(state, &mut offset);
        let _ = self.audio.deserialize(state, &mut offset);
        offset
    }
}