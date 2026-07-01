// the mapper factory and some mapper trait defs are all included in this file.

use crate::cartridge::Cartridge;
pub use crate::mappers::axrom::MapperAxROM;
pub use crate::mappers::bandai::{BandaiKind, MapperBandai};
pub use crate::mappers::cnrom::{CnromConfig, MapperCNROM};
pub use crate::mappers::cprom::MapperCpROM;
pub use crate::mappers::fds::Mapper20;
pub use crate::mappers::ffe::{FfeConfig, MapperFfe};
pub use crate::mappers::fme7::MapperFME7;
pub use crate::mappers::gxrom::Mapper66;
pub use crate::mappers::mmc1::{MapperMMC1, Mmc1Config};
pub use crate::mappers::mmc2::MapperMMC2;
pub use crate::mappers::mmc3::{MapperMMC3, Mmc3Config};
pub use crate::mappers::mmc4::MapperMMC4;
pub use crate::mappers::mmc5::{MapperMMC5, Mmc5Config};
pub use crate::mappers::n106::Mapper19;
pub use crate::mappers::nrom::{MapperNROM, NromConfig};
pub use crate::mappers::sl12::MapperSL12;
pub use crate::mappers::sl1632::MapperSL1632;
pub use crate::mappers::uxrom::{MapperUxROM, UxromConfig};
pub use crate::mappers::vrc2_4::{Vrc2And4, VrcVariant};
pub use crate::mappers::vrc6::{Vrc6, Vrc6Variant};
pub use crate::mappers::vrc7::Vrc7;
pub use crate::mappers::mapper8::Mapper8;
pub use crate::mappers::mapper11::Mapper11;
pub use crate::mappers::mapper12::Mapper12;
pub use crate::mappers::mapper15::Mapper15;
pub use crate::mappers::mapper18::Mapper18;
pub use crate::mappers::mapper27::Mapper27;
pub use crate::mappers::mapper28::Mapper28;
pub use crate::mappers::mapper29::Mapper29;
pub use crate::mappers::mapper30::Mapper30;
pub use crate::mappers::mapper31::Mapper31;
pub use crate::mappers::mapper32::Mapper32;
pub use crate::mappers::mapper33::Mapper33;
pub use crate::mappers::mapper34::Mapper34;
pub use crate::mappers::mapper36::Mapper36;
pub use crate::mappers::mapper37::Mapper37;
pub use crate::mappers::mapper38::Mapper38;
pub use crate::mappers::mapper39::Mapper39;
pub use crate::mappers::mapper40::Mapper40;
pub use crate::mappers::mapper41::Mapper41;
pub use crate::mappers::mapper42::Mapper42;
pub use crate::mappers::mapper43::Mapper43;
pub use crate::mappers::mapper44::Mapper44;
pub use crate::mappers::mapper45::Mapper45;
pub use crate::mappers::mapper46::Mapper46;
pub use crate::mappers::mapper47::Mapper47;
pub use crate::mappers::mapper48::Mapper48;
pub use crate::mappers::mapper49::Mapper49;
pub use crate::mappers::mapper50::Mapper50;
pub use crate::mappers::mapper51::Mapper51;
pub use crate::mappers::mapper52::Mapper52;
pub use crate::mappers::mapper53::Mapper53;
pub use crate::mappers::mapper54::Mapper54;
pub use crate::mappers::mapper55::Mapper55;
pub use crate::mappers::mapper56::Mapper56;
pub use crate::mappers::mapper57::Mapper57;
pub use crate::mappers::mapper58::Mapper58;
pub use crate::mappers::mapper59::Mapper59;
pub use crate::mappers::mapper60::Mapper60;
pub use crate::mappers::mapper61::Mapper61;
pub use crate::mappers::mapper62::Mapper62;
pub use crate::mappers::mapper63::Mapper63;
pub use crate::mappers::mapper64::Mapper64;
pub use crate::mappers::mapper65::Mapper65;
pub use crate::mappers::mapper67::Mapper67;
pub use crate::mappers::mapper68::Mapper68;
pub use crate::mappers::mapper70::Mapper70;
pub use crate::mappers::mapper71::Mapper71;
pub use crate::mappers::mapper72::{Mapper72, Mapper72Variant};
pub use crate::mappers::mapper73::Mapper73;
pub use crate::mappers::mapper74::Mapper74;
pub use crate::mappers::mapper75::Mapper75;
pub use crate::mappers::mapper76::Mapper76;
pub use crate::mappers::mapper77::Mapper77;
pub use crate::mappers::mapper78::Mapper78;
pub use crate::mappers::mapper79::Mapper79;
pub use crate::mappers::mapper80::Mapper80;
pub use crate::mappers::mapper81::Mapper81;
pub use crate::mappers::mapper82::Mapper82;
pub use crate::mappers::mapper83::Mapper83;
pub use crate::mappers::mapper86::Mapper86;
pub use crate::mappers::mapper87::Mapper87;
pub use crate::mappers::mapper88::Mapper88;
pub use crate::mappers::mapper89::Mapper89;
pub use crate::mappers::mapper90::{Mapper90, Mapper90Variant};
pub use crate::mappers::mapper91::Mapper91;
pub use crate::mappers::mapper93::Mapper93;
pub use crate::mappers::mapper94::Mapper94;
pub use crate::mappers::mapper95::Mapper95;
pub use crate::mappers::mapper96::Mapper96;
pub use crate::mappers::mapper97::Mapper97;
pub use crate::mappers::mapper99::Mapper99;
pub use crate::mappers::mapper100::Mapper100;
pub use crate::mappers::mapper101::Mapper101;
pub use crate::mappers::mapper103::Mapper103;
pub use crate::mappers::mapper104::Mapper104;
pub use crate::mappers::mapper106::Mapper106;
pub use crate::mappers::mapper107::Mapper107;
pub use crate::mappers::mapper108::Mapper108;
pub use crate::mappers::mapper111::Mapper111;
pub use crate::mappers::mapper112::Mapper112;
pub use crate::mappers::mapper113::Mapper113;
pub use crate::mappers::mapper114::Mapper114;
pub use crate::mappers::mapper115::Mapper115;
pub use crate::mappers::mapper117::Mapper117;
pub use crate::mappers::mapper118::Mapper118;
pub use crate::mappers::mapper119::Mapper119;
pub use crate::mappers::mapper120::Mapper120;
pub use crate::mappers::mapper121::Mapper121;
pub use crate::mappers::mapper122::Mapper122;
pub use crate::mappers::mapper123::Mapper123;
pub use crate::mappers::mapper124::Mapper124;
pub use crate::mappers::mapper125::Mapper125;
pub use crate::mappers::mapper127::Mapper127;
pub use crate::mappers::mapper128::Mapper128;
pub use crate::mappers::mapper130::Mapper130;
pub use crate::mappers::mapper131::Mapper131;
pub use crate::mappers::mapper132::Mapper132;
pub use crate::mappers::mapper133::Mapper133;
pub use crate::mappers::mapper134::Mapper134;
pub use crate::mappers::mapper136::Mapper136;
pub use crate::mappers::mapper137::Mapper137;
pub use crate::mappers::mapper138::Mapper138;
pub use crate::mappers::mapper139::Mapper139;
pub use crate::mappers::mapper140::Mapper140;
pub use crate::mappers::mapper141::Mapper141;
pub use crate::mappers::mapper142::Mapper142;
pub use crate::mappers::mapper143::Mapper143;
pub use crate::mappers::mapper144::Mapper144;
pub use crate::mappers::mapper145::Mapper145;
pub use crate::mappers::mapper147::Mapper147;
pub use crate::mappers::mapper148::Mapper148;
pub use crate::mappers::mapper149::Mapper149;
pub use crate::mappers::mapper150::Mapper150;
pub use crate::mappers::mapper151::Mapper151;
pub use crate::mappers::mapper154::Mapper154;
pub use crate::mappers::mapper156::Mapper156;
pub use crate::mappers::mapper162::Mapper162;
pub use crate::mappers::mapper163::Mapper163;
pub use crate::mappers::mapper164::Mapper164;
pub use crate::mappers::mapper165::Mapper165;
pub use crate::mappers::mapper166::Mapper166;
pub use crate::mappers::mapper167::Mapper167;
pub use crate::mappers::mapper168::Mapper168;
pub use crate::mappers::mapper169::Mapper169;
pub use crate::mappers::mapper170::Mapper170;
pub use crate::mappers::mapper172::Mapper172;
pub use crate::mappers::mapper173::Mapper173;
pub use crate::mappers::mapper174::Mapper174;
pub use crate::mappers::mapper175::Mapper175;
pub use crate::mappers::mapper176::Mapper176;
pub use crate::mappers::mapper177::Mapper177;
pub use crate::mappers::mapper178::Mapper178;
pub use crate::mappers::mapper179::Mapper179;
pub use crate::mappers::mapper180::Mapper180;
pub use crate::mappers::mapper183::Mapper183;
pub use crate::mappers::mapper184::Mapper184;
pub use crate::mappers::mapper185::{Mapper185, Mapper185Config};
pub use crate::mappers::mapper186::Mapper186;
pub use crate::mappers::mapper187::Mapper187;
pub use crate::mappers::mapper188::Mapper188;
pub use crate::mappers::mapper189::Mapper189;
pub use crate::mappers::mapper190::Mapper190;
pub use crate::mappers::mapper191::Mapper191;
pub use crate::mappers::mapper192::Mapper192;
pub use crate::mappers::mapper193::Mapper193;
pub use crate::mappers::mapper194::Mapper194;
pub use crate::mappers::mapper195::Mapper195;
pub use crate::mappers::mapper196::Mapper196;
pub use crate::mappers::mapper197::Mapper197;
pub use crate::mappers::mapper198::Mapper198;
pub use crate::mappers::mapper199::Mapper199;
pub use crate::mappers::mapper200::Mapper200;
pub use crate::mappers::mapper201::Mapper201;
pub use crate::mappers::mapper202::Mapper202;
pub use crate::mappers::mapper203::Mapper203;
pub use crate::mappers::mapper204::Mapper204;
pub use crate::mappers::mapper205::Mapper205;
pub use crate::mappers::mapper206::Mapper206;
pub use crate::mappers::mapper207::Mapper207;
pub use crate::mappers::mapper208::Mapper208;
pub use crate::mappers::mapper210::Mapper210;
pub use crate::mappers::mapper212::Mapper212;
pub use crate::mappers::mapper214::Mapper214;
pub use crate::mappers::mapper215::Mapper215;
pub use crate::mappers::mapper216::Mapper216;
pub use crate::mappers::mapper217::Mapper217;
pub use crate::mappers::mapper218::Mapper218;
pub use crate::mappers::mapper219::Mapper219;
pub use crate::mappers::mapper221::Mapper221;
pub use crate::mappers::mapper222::Mapper222;
pub use crate::mappers::mapper224::Mapper224;
pub use crate::mappers::mapper225::Mapper225;
pub use crate::mappers::mapper226::Mapper226;
pub use crate::mappers::mapper227::Mapper227;
pub use crate::mappers::mapper228::Mapper228;
pub use crate::mappers::mapper229::Mapper229;
pub use crate::mappers::mapper230::Mapper230;
pub use crate::mappers::mapper231::Mapper231;
pub use crate::mappers::mapper232::Mapper232;
pub use crate::mappers::mapper233::Mapper233;
pub use crate::mappers::mapper234::Mapper234;
pub use crate::mappers::mapper235::Mapper235;
pub use crate::mappers::mapper236::Mapper236;
pub use crate::mappers::mapper237::Mapper237;
pub use crate::mappers::mapper238::Mapper238;
pub use crate::mappers::mapper239::Mapper239;
pub use crate::mappers::mapper240::Mapper240;
pub use crate::mappers::mapper241::Mapper241;
pub use crate::mappers::mapper242::Mapper242;
pub use crate::mappers::mapper243::Mapper243;
pub use crate::mappers::mapper244::Mapper244;
pub use crate::mappers::mapper245::Mapper245;
pub use crate::mappers::mapper246::Mapper246;
pub use crate::mappers::mapper248::Mapper248;
pub use crate::mappers::mapper249::Mapper249;
pub use crate::mappers::mapper250::Mapper250;
pub use crate::mappers::mapper252::Mapper252;
pub use crate::mappers::mapper253::Mapper253;
pub use crate::mappers::mapper254::Mapper254;
pub use crate::mappers::mapper255::Mapper255;
pub use crate::mappers::mapper256::Mapper256;
pub use crate::mappers::mapper257::Mapper257;
pub use crate::mappers::mapper259::Mapper259;
pub use crate::mappers::mapper260::Mapper260;
pub use crate::mappers::mapper265::Mapper265;
pub use crate::mappers::mapper284::Mapper284;
pub use crate::mappers::mapper290::Mapper290;
pub use crate::mappers::mapper298::Mapper298;
pub use crate::mappers::mapper326::Mapper326;
pub use crate::mappers::mapper328::Mapper328;
pub use crate::mappers::mapper329::Mapper329;
pub use crate::mappers::mapper331::Mapper331;
pub use crate::mappers::mapper365::Mapper365;
pub use crate::mappers::mapper385::Mapper385;
pub use crate::mappers::mapper389::Mapper389;
pub use crate::mappers::mapper409::Mapper409;
pub use crate::mappers::mapper418::Mapper418;
pub use crate::mappers::mapper437::Mapper437;
pub use crate::mappers::mapper455::Mapper455;
pub use crate::mappers::mapper471::Mapper471;
pub use crate::mappers::mapper476::Mapper476;
pub use crate::mappers::mapper486::Mapper486;
pub use crate::mappers::mapper495::Mapper495;
pub use crate::mappers::mapper497::Mapper497;
pub use crate::mappers::mapper514::Mapper514;
pub use crate::mappers::mapper521::Mapper521;
pub use crate::mappers::mapper525::Mapper525;
pub use crate::mappers::mapper531::Mapper531;
pub use crate::mappers::mapper533::Mapper533;
pub use crate::mappers::mapper534::{MapperAx5202p, Ax5202pVariant};
pub use crate::mappers::mapper552::Mapper552;
pub use crate::mappers::mapper553::Mapper553;
pub use crate::mappers::mapper582::Mapper582;

/// prg fetch result
pub struct FetchResult {
    pub data: u8,
    pub driven: bool,
}

/// the mapper trait with all its handling templates
pub trait Mapper: Send {
    // fetching and storing prg by bus through mapper
    fn fetch_prg(&mut self, cart: &Cartridge, address: u16) -> FetchResult;
    fn store_prg(&mut self, cart: &mut Cartridge, address: u16, data: u8);

    // ppu nametable mirroring handling
    fn mirror_nametable(&self, cart: &Cartridge, address: u16) -> u16;

    // ppu data fetching by bus through mapper
    fn fetch_ppu(
        &mut self,
        _prg_rom: &[u8],
        _chr_rom: &[u8],
        _prg_ram: &[u8],
        _chr_ram: &[u8],
        _prg_vram: &[u8],
        _using_chr_ram: bool,
        nametable_horizontal_mirroring: bool,
        _alternative_nametable_arrangement: bool,
        ppu_address_bus: u16,
        ppu_octal_latch: u8,
        _vram: &[u8],
    ) -> (u8, u16);

    // ppu data storing by bus through mapper
    fn store_ppu(&mut self, cart: &mut Cartridge, address: u16, data: u8, vram: &mut [u8]) {
        if address < 0x2000 && cart.using_chr_ram {
            cart.chr_ram[address as usize & 0x1FFF] = data;
        } else if address >= 0x2000 && address < 0x3F00 {
            let mirrored = self.mirror_nametable(cart, address);
            vram[(mirrored & 0x7FF) as usize] = data;
        }
    }

    // ppu clocking via mapper for irqs etc
    fn ppu_clock(
        &mut self,
        _ppu_address_bus: u16,
        _ppu_a12_prev: bool,
        _scanline: u16,
        _dot: u16,
        _ppu_sprite_x16: bool,
        _rendering_on: bool,
    ) -> bool {
        false
    }
    fn cpu_clock(&mut self, _cycles: u8) -> bool { false }
    fn cpu_clock_rise(&mut self, _ppu_address_bus: u16) -> bool { false }

    // save state and load state
    #[allow(dead_code)]
    fn save_mapper_registers(&self, cart: &Cartridge) -> Vec<u8>;
    #[allow(dead_code)]
    fn load_mapper_registers(&mut self, cart: &mut Cartridge, state: &[u8], start: usize) -> usize;

    // fds disk swapping
    fn change_disk(&mut self) {}

    // coin insertion for vs system and dip switch mappers
    fn insert_coin(&mut self, _coin: u8) {}
    fn service_button(&mut self) {}

    // dip switch support for vs system and multicart mappers
    fn get_dip_switches(&self) -> u8 { 0 }
    fn set_dip_switches(&mut self, _value: u8) {}

    // controller read adjustment for vs system mappers
    fn adjust_controller_read(&self, _address: u16, value: u8) -> u8 { value }

    // notify mapper of resolved CPU clock (for audio rate mappers)
    fn set_cpu_clock(&mut self, _clock: f64) {}

    // expanded audio for mappers with extra audio channels
    fn audio_sample(&self) -> f32 { 0.0 }

    // irq clearing handler for mmc3-like mappers
    fn take_irq_ack(&mut self) -> bool {
        false
    }

    // battery-backed save data handling
    fn battery_save_data(&self, _cart: &Cartridge) -> Option<Vec<u8>> {
        None
    }

    fn load_battery_save(&mut self, _cart: &mut Cartridge, _data: &[u8]) {}

    // and finally mapper reset handling
    fn reset(&mut self) {}
}


// the mapper factory
pub fn create_mapper(
    mapper_id: u16,
    submapper_id: u8,
    header: &[u8],
    rom: &[u8],
    prg_size: u8,
    using_chr_ram: bool,
    has_battery: bool,
    rom_name: &str,
) -> Result<Box<dyn Mapper>, String> {
    let mapper: Box<dyn Mapper> = match mapper_id {
        // plane 0
        0 => Box::new(MapperNROM::new(NromConfig::for_ines(
            header,
            if using_chr_ram { 0 } else { header[5] },
        ))),
        1 => Box::new(MapperMMC1::new(Mmc1Config::for_ines(
            header,
            rom,
            mapper_id,
            submapper_id,
            prg_size,
            using_chr_ram,
            has_battery,
        ))),
        2 => Box::new(MapperUxROM::new(UxromConfig::for_ines(
            header,
            submapper_id,
            if using_chr_ram { 0 } else { header[5] },
        ))),
        3 => Box::new(MapperCNROM::new(CnromConfig::for_ines(header, submapper_id))),
        4 => Box::new(MapperMMC3::new(Mmc3Config::for_ines(
            header,
            submapper_id,
            if using_chr_ram { 0 } else { header[5] },
            rom,
            rom_name,
        ))),
        5 => Box::new(MapperMMC5::new(Mmc5Config::for_ines(
            header,
            rom,
            has_battery,
        ))),
        6 => Box::new(MapperFfe::new(FfeConfig::mapper6(
            header,
            submapper_id,
            has_battery,
        ))),
        7 => Box::new(MapperAxROM::new()),
        8 => Box::new(Mapper8::new()),
        9 => Box::new(MapperMMC2::new()),
        10 => Box::new(MapperMMC4::new()),
        11 => Box::new(Mapper11::new()),
        12 => Box::new(Mapper12::new(
            header,
            submapper_id,
            if using_chr_ram { 0 } else { header[5] },
            rom,
            rom_name,
            has_battery,
        )),
        13 => Box::new(MapperCpROM::new()),
        14 => Box::new(MapperSL1632::new()),
        15 => Box::new(Mapper15::new()),
        16 => Box::new(MapperBandai::new(BandaiKind::Mapper16)),
        17 => Box::new(MapperFfe::new(FfeConfig::mapper17(header, has_battery))),
        18 => Box::new(Mapper18::new()),
        19 => Box::new(Mapper19::new()),
//      20 => Box::new(MapperFDS::new()),
        21 => Box::new(Vrc2And4::new(VrcVariant::Mapper21)),
        22 => Box::new(Vrc2And4::new(VrcVariant::Mapper22)),
        23 => Box::new(Vrc2And4::new(VrcVariant::Mapper23)),
        24 => Box::new(Vrc6::new(Vrc6Variant::Mapper24)),
        25 => Box::new(Vrc2And4::new(VrcVariant::Mapper25)),
        26 => Box::new(Vrc6::new(Vrc6Variant::Mapper26)),
        27 => Box::new(Mapper27::new()),
        28 => {
            let prg_size_16k = if header[4] == 0 { 1 } else { header[4] as usize };
            Box::new(Mapper28::new(prg_size_16k - 1))
        }
        29 => Box::new(Mapper29::new()),
        30 => Box::new(Mapper30::new(submapper_id, has_battery, header)),
        31 => Box::new(Mapper31::new()),
        32 => Box::new(Mapper32::new()),
        33 => Box::new(Mapper33::new()),
        34 => Box::new(Mapper34::new()),
        35 => Box::new(Mapper90::new(Mapper90Variant::Mapper35)),
        36 => Box::new(Mapper36::new()),
        37 => Box::new(Mapper37::new(header, rom, rom_name)),
        38 => Box::new(Mapper38::new()),
        39 => Box::new(Mapper39::new()),
        40 => Box::new(Mapper40::new()),
        41 => Box::new(Mapper41::new()),
        42 => Box::new(Mapper42::new()),
        43 => Box::new(Mapper43::new()),
        44 => Box::new(Mapper44::new(header, rom, rom_name)),
        45 => Box::new(Mapper45::new(header, rom, rom_name)),
        46 => Box::new(Mapper46::new()),
        47 => Box::new(Mapper47::new(header, rom, rom_name)),
        48 => Box::new(Mapper48::new(header, rom, rom_name)),
        49 => Box::new(Mapper49::new(header, rom, rom_name)),
        50 => Box::new(Mapper50::new()),
        51 => Box::new(Mapper51::new()),
        52 => Box::new(Mapper52::new(header, rom, rom_name)),
        53 => {
            let has_trainer = (rom[6] & 4) != 0;
            let trainer_len = if has_trainer { 512 } else { 0 };
            let prg_rom_len = prg_size as usize * 0x4000;
            let start = 16 + trainer_len;
            let prg_slice = if start + prg_rom_len <= rom.len() {
                &rom[start..start + prg_rom_len]
            } else {
                rom
            };
            Box::new(Mapper53::new(prg_slice))
        }
        54 => Box::new(Mapper54::new()),
        55 => Box::new(Mapper55::new()),
        56 => Box::new(Mapper56::new()),
        57 => Box::new(Mapper57::new()),
        58 => Box::new(Mapper58::new()),
        59 => Box::new(Mapper59::new()),
        60 => Box::new(Mapper60::new()),
        61 => Box::new(Mapper61::new()),
        62 => Box::new(Mapper62::new()),
        63 => Box::new(Mapper63::new()),
        64 => Box::new(Mapper64::new((header[6] & 1) == 0)),
        65 => Box::new(Mapper65::new((header[6] & 1) == 0)),
        66 => Box::new(Mapper66::new()),
        67 => Box::new(Mapper67::new()),
        68 => Box::new(Mapper68::new()),
        69 => Box::new(MapperFME7::new()),
        70 => Box::new(Mapper70::new((header[6] & 1) == 0, false)),
        71 => Box::new(Mapper71::new(UxromConfig::for_ines(
            header,
            submapper_id,
            if using_chr_ram { 0 } else { header[5] },
        ))),
        72 => {
            let has_trainer = (header[6] & 4) != 0;
            let trainer_len = if has_trainer { 512 } else { 0 };
            let prg_rom_len = prg_size as usize * 0x4000;
            let chr_rom_len = header[5] as usize * 0x2000;
            let misc_rom = if rom.len() > 0x10 + trainer_len + prg_rom_len + chr_rom_len {
                rom[0x10 + trainer_len + prg_rom_len + chr_rom_len..].to_vec()
            } else {
                Vec::new()
            };
            Box::new(Mapper72::new(Mapper72Variant::Mapper72, misc_rom))
        }
        73 => Box::new(Mapper73::new()),
        74 => Box::new(Mapper74::new(header, rom, rom_name)),
        75 => Box::new(Mapper75::new((header[6] & 1) == 0)),
        76 => Box::new(Mapper76::mapper76()),
        77 => Box::new(Mapper77::new()),
        78 => Box::new(Mapper78::new(submapper_id, (header[6] & 8) != 0)),
        79 => Box::new(Mapper79::new()),
        80 => Box::new(Mapper80::mapper80()),
        81 => Box::new(Mapper81::new()),
        82 => Box::new(Mapper82::new()),
        83 => Box::new(Mapper83::new(83, submapper_id)),
//      84 => Box::new(Mapper40::new()),
        85 => Box::new(Vrc7::new(submapper_id)),
        86 => {
            let has_trainer = (header[6] & 4) != 0;
            let trainer_len = if has_trainer { 512 } else { 0 };
            let prg_rom_len = prg_size as usize * 0x4000;
            let chr_rom_len = header[5] as usize * 0x2000;
            let misc_rom = if rom.len() > 0x10 + trainer_len + prg_rom_len + chr_rom_len {
                rom[0x10 + trainer_len + prg_rom_len + chr_rom_len..].to_vec()
            } else {
                Vec::new()
            };
            Box::new(Mapper86::new(misc_rom))
        }
        87 => Box::new(Mapper87::new()),
        88 => Box::new(Mapper88::mapper88()),
        89 => Box::new(Mapper89::new()),
        90 => Box::new(Mapper90::new(Mapper90Variant::Mapper90)),
        91 => Box::new(Mapper91::new(submapper_id)),
        92 => {
            let has_trainer = (header[6] & 4) != 0;
            let trainer_len = if has_trainer { 512 } else { 0 };
            let prg_rom_len = prg_size as usize * 0x4000;
            let chr_rom_len = header[5] as usize * 0x2000;
            let misc_rom = if rom.len() > 0x10 + trainer_len + prg_rom_len + chr_rom_len {
                rom[0x10 + trainer_len + prg_rom_len + chr_rom_len..].to_vec()
            } else {
                Vec::new()
            };
            Box::new(Mapper72::new(Mapper72Variant::Mapper92, misc_rom))
        }
        93 => Box::new(Mapper93::new()),
        94 => Box::new(Mapper94::new(UxromConfig::for_ines(
            header,
            submapper_id,
            if using_chr_ram { 0 } else { header[5] },
        ))),
        95 => Box::new(Mapper95::mapper95()),
        96 => Box::new(Mapper96::new(header)),
        97 => Box::new(Mapper97::new()),
//      98 => Box::new(Mapper40::new()),
        99 => Box::new(Mapper99::new()),
        100 => Box::new(Mapper100::new()),
        101 => Box::new(Mapper101::new()),
        102 => Box::new(Mapper284::new()),
        103 => Box::new(Mapper103::new()),
        104 => Box::new(Mapper104::new()),
        105 => Box::new(MapperMMC1::new(Mmc1Config::for_ines(
            header,
            rom,
            mapper_id,
            submapper_id,
            prg_size,
            using_chr_ram,
            has_battery,
        ))),
        106 => Box::new(Mapper106::new()),
        107 => Box::new(Mapper107::new()),
        108 => Box::new(Mapper108::new()),
        109 => Box::new(Mapper137::new()),
        110 => Box::new(Mapper243::new()),
        111 => Box::new(Mapper111::new(prg_size, header[5] > 0)),
        112 => Box::new(Mapper112::new()),
        113 => Box::new(Mapper113::new()),
        114 => Box::new(Mapper114::new(prg_size, submapper_id)),
        115 => Box::new(Mapper115::new(prg_size)),
        116 => Box::new(MapperSL12::new()),
        117 => Box::new(Mapper117::new()),
        118 => Box::new(Mapper118::new(Mmc3Config::for_ines(
            header,
            submapper_id,
            if using_chr_ram { 0 } else { header[5] },
            rom,
            rom_name,
        ))),
        119 => Box::new(Mapper119::new(Mmc3Config::for_ines(
            header,
            submapper_id,
            if using_chr_ram { 0 } else { header[5] },
            rom,
            rom_name,
        ))),
        120 => Box::new(Mapper120::new()),
        121 => Box::new(Mapper121::new(
            Mmc3Config::for_ines(
                header,
                submapper_id,
                if using_chr_ram { 0 } else { header[5] },
                rom,
                rom_name,
            ),
            prg_size as usize * 0x4000,
            header[5] as usize * 0x2000,
        )),
        122 => Box::new(Mapper122::new()),
        123 => Box::new(Mapper123::new(prg_size)),
        124 => Box::new(Mapper124::new(if header[14] >> 4 != 0 { header[15] } else { 0 })),
        125 => Box::new(Mapper125::new()),
        126 => Box::new(MapperAx5202p::new(Ax5202pVariant::Mapper126)),
        127 => Box::new(Mapper127::new()),
        128 => Box::new(Mapper128::new()),
        129 => Box::new(Mapper58::new()),
        130 => Box::new(Mapper130::new()),
        131 => Box::new(Mapper131::new(header, rom, rom_name)),
        132 => Box::new(Mapper132::new()),
        133 => Box::new(Mapper133::new()),
        134 => Box::new(Mapper134::new(header, rom, rom_name)),
        135 => Box::new(Mapper141::new()),
        136 => Box::new(Mapper136::new()),
        137 => Box::new(Mapper137::new()),
        138 => Box::new(Mapper138::new()),
        139 => Box::new(Mapper139::new()),
        140 => Box::new(Mapper140::new()),
        141 => Box::new(Mapper141::new()),
        142 => Box::new(Mapper142::new()),
        143 => Box::new(Mapper143::new()),
        144 => Box::new(Mapper144::new()),
        145 => Box::new(Mapper145::new()),
        146 => Box::new(Mapper79::new()),
        147 => Box::new(Mapper147::new()),
        148 => Box::new(Mapper148::new()),
        149 => Box::new(Mapper149::new()),
        150 => Box::new(Mapper150::new()),
        151 => Box::new(Mapper151::new()),
        152 => Box::new(Mapper70::new((header[6] & 1) == 0, true)),
        153 => Box::new(MapperBandai::new(BandaiKind::Mapper153)),
        154 => Box::new(Mapper154::mapper154()),
        155 => Box::new(MapperMMC1::new(Mmc1Config::for_ines(
            header,
            rom,
            mapper_id,
            submapper_id,
            prg_size,
            using_chr_ram,
            has_battery,
        ))),
        156 => Box::new(Mapper156::new()),
        157 => Box::new(MapperBandai::new(BandaiKind::Mapper157)),
        158 => Box::new(Mapper64::new_mapper158()),
        159 => Box::new(MapperBandai::new(BandaiKind::Mapper159)),
        160 => Box::new(Mapper90::new(Mapper90Variant::Mapper90)),
        161 => Box::new(MapperMMC1::new(Mmc1Config::for_ines(
            header,
            rom,
            mapper_id,
            submapper_id,
            prg_size,
            using_chr_ram,
            has_battery,
        ))),
        162 => Box::new(Mapper162::new()),
        163 => Box::new(Mapper163::new()),
        164 => {
            let prg_ram_size = header[0x13] as usize * 64;
            Box::new(Mapper164::new(prg_ram_size))
        },
        165 => Box::new(Mapper165::new()),
        166 => Box::new(Mapper166::new()),
        167 => Box::new(Mapper167::new()),
        168 => Box::new(Mapper168::new()),
        169 => Box::new(Mapper169::new()),
        170 => Box::new(Mapper170::new()),
        171 => Box::new(MapperMMC1::new(Mmc1Config::for_ines(
            header,
            rom,
            mapper_id,
            submapper_id,
            prg_size,
            using_chr_ram,
            has_battery,
        ))),
        172 => Box::new(Mapper172::new()),
        173 => Box::new(Mapper173::new()),
        174 => Box::new(Mapper174::new()),
        175 => Box::new(Mapper175::new()),
        176 => Box::new(Mapper176::new()),
        177 => Box::new(Mapper177::new()),
        178 => Box::new(Mapper178::new()),
        179 => Box::new(Mapper179::new()),
        180 => Box::new(Mapper180::new(UxromConfig::for_ines(
            header,
            submapper_id,
            if using_chr_ram { 0 } else { header[5] },
        ))),
        181 => Box::new(Mapper185::new(Mapper185Config::for_ines(header, submapper_id))),
        182 => Box::new(Mapper114::new(prg_size, submapper_id)),
        183 => Box::new(Mapper183::new()),
        184 => Box::new(Mapper184::new()),
        185 => Box::new(Mapper185::new(Mapper185Config::for_ines(header, submapper_id))),
        186 => Box::new(Mapper186::new()),
        187 => Box::new(Mapper187::new()),
        188 => Box::new(Mapper188::new()),
        189 => Box::new(Mapper189::new()),
        190 => Box::new(Mapper190::new()),
        191 => Box::new(Mapper191::new()),
        192 => Box::new(Mapper192::new()),
        193 => Box::new(Mapper193::new()),
        194 => Box::new(Mapper194::new()),
        195 => Box::new(Mapper195::new()),
        196 => Box::new(Mapper196::new(submapper_id)),
        197 => Box::new(Mapper197::new(submapper_id)),
        198 => Box::new(Mapper198::new()),
        199 => Box::new(Mapper199::new()),
        200 => Box::new(Mapper200::new()),
        201 => Box::new(Mapper201::new()),
        202 => Box::new(Mapper202::new()),
        203 => Box::new(Mapper203::new()),
        204 => Box::new(Mapper204::new()),
        205 => Box::new(Mapper205::new(header, rom, rom_name)),
        206 => Box::new(Mapper206::new(submapper_id)),
        207 => Box::new(Mapper207::mapper207()),
        208 => Box::new(Mapper208::new(header, rom, rom_name)),
        209 => Box::new(Mapper90::new(Mapper90Variant::Mapper209)),
        210 => Box::new(Mapper210::new(submapper_id)),
        211 => Box::new(Mapper90::new(Mapper90Variant::Mapper211)),
        212 => Box::new(Mapper212::new()),
        213 => Box::new(Mapper58::new()),
        214 => Box::new(Mapper214::new()),
        215 => Box::new(Mapper215::new(header, rom, rom_name)),
        216 => Box::new(Mapper216::new()),
        217 => Box::new(Mapper217::new()),
        218 => Box::new(Mapper218::new()),
        219 => Box::new(Mapper219::new(header, rom, rom_name)),
//      220 => Box::new(Mapper220::new()),
        221 => Box::new(Mapper221::new()),
        222 => Box::new(Mapper222::new()),
        223 => Box::new(Mapper199::new()),
        224 => Box::new(Mapper224::new(header, rom, rom_name)),
        225 => Box::new(Mapper225::new()),
        226 => Box::new(Mapper226::new()),
        227 => Box::new(Mapper227::new()),
        228 => Box::new(Mapper228::new()),
        229 => Box::new(Mapper229::new()),
        230 => Box::new(Mapper230::new()),
        231 => Box::new(Mapper231::new()),
        232 => Box::new(Mapper232::new()),
        233 => Box::new(Mapper233::new()),
        234 => Box::new(Mapper234::new()),
        235 => Box::new(Mapper235::new(prg_size as usize)),
        236 => Box::new(Mapper236::new(using_chr_ram || header[5] > 0)),
        237 => Box::new(Mapper237::new()),
        238 => Box::new(Mapper238::new()),
        239 => Box::new(Mapper239::new()),
        240 => Box::new(Mapper240::new()),
        241 => Box::new(Mapper241::new()),
        242 => Box::new(Mapper242::new()),
        243 => Box::new(Mapper243::new()),
        244 => Box::new(Mapper244::new()),
        245 => Box::new(Mapper245::new()),
        246 => Box::new(Mapper246::new()),
//      247 => Box::new(Mapper247::new()),
        248 => Box::new(Mapper248::new(prg_size)),
        249 => Box::new(Mapper249::new()),
        250 => Box::new(Mapper250::new()),
        251 => Box::new(Mapper45::new(header, rom, rom_name)),
        252 => Box::new(Mapper252::new()),
        253 => Box::new(Mapper253::new()),
        254 => Box::new(Mapper254::new()),
        255 => Box::new(Mapper255::new()),
        256 => Box::new(Mapper256::new(
             Mmc3Config::for_ines(
                 header,
                 submapper_id,
                 if using_chr_ram { 0 } else { header[5] },
                 rom,
                 rom_name,
             ),
             submapper_id,
         )),
        257 => {
            let is_small = if submapper_id == 1 { true } else if submapper_id == 2 { false } else { prg_size < 32 };
            if is_small { Box::new(Mapper257::new_small()) } else { Box::new(Mapper257::new_large()) }
        },
        258 => Box::new(Mapper215::new(header, rom, rom_name)),
        259 => Box::new(Mapper259::new(header, rom, rom_name)),
        260 => Box::new(Mapper260::new(header, rom, rom_name)),
        264 => Box::new(Mapper83::new(264, submapper_id)),
        265 => Box::new(Mapper265::new()),
        281 => Box::new(Mapper90::new(Mapper90Variant::Mapper281)),
        282 => Box::new(Mapper90::new(Mapper90Variant::Mapper282)),
        284 => Box::new(Mapper284::new()),
        290 => Box::new(Mapper290::new()),
        295 => Box::new(Mapper90::new(Mapper90Variant::Mapper295)),
        298 => Box::new(Mapper298::new()),
        311 => Box::new(Mapper43::new()),
        326 => Box::new(Mapper326::new()),
        328 => Box::new(Mapper328::new()),
        329 => Box::new(Mapper329::new()),
        331 => Box::new(Mapper331::new()),
        358 => Box::new(Mapper90::new(Mapper90Variant::Mapper358)),
        365 => Box::new(Mapper365::new()),
        385 => Box::new(Mapper385::new()),
        386 => Box::new(Mapper90::new(Mapper90Variant::Mapper386)),
        387 => Box::new(Mapper90::new(Mapper90Variant::Mapper387)),
        388 => Box::new(Mapper90::new(Mapper90Variant::Mapper388)),
        389 => Box::new(Mapper389::new()),
        397 => Box::new(Mapper90::new(Mapper90Variant::Mapper397)),
        409 => Box::new(Mapper409::new()),
        418 => Box::new(Mapper418::new()),
        422 => Box::new(MapperAx5202p::new(Ax5202pVariant::Mapper422)),
        437 => Box::new(Mapper437::new()),
        455 => Box::new(Mapper455::new(header, rom, rom_name)),
        471 => Box::new(Mapper471::new()),
        476 => Box::new(Mapper476::new()),
        486 => Box::new(Mapper486::new()),
        495 => Box::new(Mapper495::new()),
        497 => Box::new(Mapper497::new()),
        514 => Box::new(Mapper514::new()),
        521 => Box::new(Mapper521::new(prg_size)),
        525 => Box::new(Mapper525::new()),
        531 => Box::new(Mapper531::new(header, rom, rom_name)),
        532 => Box::new(Mapper19::new()),
        533 => Box::new(Mapper533::new()),
        534 => Box::new(MapperAx5202p::new(Ax5202pVariant::Mapper534)),
        552 => Box::new(Mapper552::new()),
        553 => Box::new(Mapper553::new()),
        582 => Box::new(Mapper582::new()),
        _ => {
            return Err(format!("Mapper {} is currently unsupported", mapper_id));
        }
    };
    Ok(mapper)
}
