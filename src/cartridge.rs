// the ines based cartridge loader

use std::fs;
use crate::crc::crc32;
use crate::mapper::{Mapper, create_mapper};
use crate::region::TvSystem;

pub struct Cartridge {
    pub name: String,

    pub prg_rom: Vec<u8>,     
    pub chr_rom: Vec<u8>,     

    pub memory_mapper: u16, 
    pub sub_mapper: u8,     
    #[allow(dead_code)]
    pub prg_size: u8,         
    #[allow(dead_code)]
    pub chr_size: u8,          
    pub prg_size_minus_1: u8, 

    pub chr_ram: Vec<u8>,      
    pub using_chr_ram: bool,   

    pub prg_ram: Vec<u8>,      
    pub has_battery: bool,     
    pub alternative_nametable_arrangement: bool,
    pub prg_vram: Vec<u8>,     

    pub nametable_horizontal_mirroring: bool,

    pub fds_disks: Vec<Vec<u8>>, 
    pub trainer: Vec<u8>,        
    #[allow(dead_code)]
    pub misc_rom: Vec<u8>,    

    pub mapper_chip: Box<dyn Mapper + Send>,

    pub mapper_cpu_cycle: i64,

    #[allow(dead_code)]
    pub prg_rom_crc32: u32,
    #[allow(dead_code)]
    pub chr_rom_crc32: u32,
    #[allow(dead_code)]
    pub overall_crc32: u32,

    pub is_vs_system: bool,

    pub tv_system: TvSystem,
}
fn convert_mfc_to_ines(rom: &[u8]) -> Result<Vec<u8>, String> {
    if rom.len() < 16 || &rom[0..4] != b"mfc\0" {
        return Err("Not a valid MFC ROM file".to_string());
    }
    let prg_16k = rom[4] as usize;
    let chr_8k = rom[5] as usize;
    let mfc_mapper = ((rom[6] & 0xF0) >> 4) | (rom[7] & 0xF0);
    let expected = 16 + prg_16k * 0x4000 + chr_8k * 0x2000;
    if rom.len() < expected {
        return Err(format!(
            "MFC ROM file too small: expected {} bytes, got {}",
            expected,
            rom.len()
        ));
    }

    let (mapper, submapper, prgram, chrram, battery) = match mfc_mapper {
        1 => (256u16, 1u8, 0x00u8, 0x00u8, false),
        3 => (4u16, 0u8, 0x00u8, 0x00u8, false),
        30 => (176u16, 2u8, 0x90u8, 0x00u8, true),
        61 => (164u16, 0u8, 0x70u8, 0x07u8, true),
        _ => return Err(format!("Unknown MFC mapper number: {}", mfc_mapper)),
    };

    let mut out = vec![0u8; expected];
    out[0..4].copy_from_slice(b"NES\x1a");
    out[4] = prg_16k as u8;
    out[5] = chr_8k as u8;
    out[6] = ((mapper as u8 & 0x0F) << 4) | 0x01 | if battery { 0x02 } else { 0x00 };
    out[7] = 0x08 | (((mapper >> 4) as u8 & 0x0F) << 4);
    out[8] = ((submapper & 0x0F) << 4) | ((mapper >> 8) as u8 & 0x0F);
    out[9] = 0x00;
    out[10] = prgram;
    out[11] = chrram;
    out[12] = 0x03;
    out[13] = if mapper == 256 { 8 } else { 0 };
    out[16..].copy_from_slice(&rom[16..expected]);
    Ok(out)
}

impl Cartridge {

    pub fn from_file(filepath: &str) -> Result<Cartridge, String> {
        let mut rom = fs::read(filepath).map_err(|e| format!("Failed to read file: {}", e))?;

        if rom.len() < 16 {
            return Err("File too small to contain iNES header".to_string());
        }

        let is_fds = filepath.to_lowercase().ends_with(".fds") || &rom[0..4] == b"FDS\x1a";
        let is_qd = filepath.to_lowercase().ends_with(".qd") || (rom.len() >= 65536 && rom.len() % 65536 == 0 && &rom[0..15] == b"\x01*NINTENDO-HVC*");

        if is_fds || is_qd {
            let bios_path = crate::config::load_fds_bios_path();
            let disksys = fs::read(&bios_path).map_err(|_| format!("Failed to find FDS BIOS file at: {}", bios_path))?;
            if disksys.len() != 0x2000 {
                return Err(format!("FDS BIOS file '{}' is not exactly 8KB", bios_path));
            }

            let mut fds_disks = Vec::new();
            let mut offset = 0;
            let (num_sides, side_capacity) = if &rom[0..4] == b"FDS\x1a" {
                offset = 16;
                (rom[4] as usize, 65500)
            } else if is_qd {
                (rom.len() / 65536, 65536)
            } else {
                (rom.len() / 65500, 65500)
            };

            for i in 0..num_sides {
                if offset + side_capacity <= rom.len() {
                    let mut side = vec![0u8; side_capacity];
                    side.copy_from_slice(&rom[offset..offset + side_capacity]);
                    let side_crc = crc32(&side);
                    let fixed_side = if is_qd {
                        fix_qd_disk_side(&side, i, side_crc)?
                    } else {
                        fix_fds_disk_side(&side, i, side_crc)?
                    };
                    fds_disks.push(fixed_side);
                    offset += side_capacity;
                } else {
                    break;
                }
            }

            if fds_disks.is_empty() {
                return Err("FDS file has no disk sides".to_string());
            }

            let mut prg_ram = vec![0u8; 0x8000];
            let sav_path = crate::config::save_file_path(filepath);
            if let Ok(sav_data) = fs::read(&sav_path) {
                if sav_data.len() >= 4 && &sav_data[0..4] == b"ACFD" {
                    let mut pos = 4;
                    if pos + 4 <= sav_data.len() {
                        let prg_len = u32::from_le_bytes([sav_data[pos], sav_data[pos + 1], sav_data[pos + 2], sav_data[pos + 3]]) as usize;
                        pos += 4;
                        let copy_len = prg_len.min(prg_ram.len());
                        if pos + copy_len <= sav_data.len() {
                            prg_ram[..copy_len].copy_from_slice(&sav_data[pos..pos + copy_len]);
                        }
                        pos += prg_len.max(copy_len);
                    }
                    let mut loaded = Vec::new();
                    if pos + 4 <= sav_data.len() {
                        let count = u32::from_le_bytes([sav_data[pos], sav_data[pos + 1], sav_data[pos + 2], sav_data[pos + 3]]) as usize;
                        pos += 4;
                        for _ in 0..count {
                            if pos + 4 > sav_data.len() {
                                break;
                            }
                            let len = u32::from_le_bytes([sav_data[pos], sav_data[pos + 1], sav_data[pos + 2], sav_data[pos + 3]]) as usize;
                            pos += 4;
                            if pos + len > sav_data.len() {
                                break;
                            }
                            loaded.push(sav_data[pos..pos + len].to_vec());
                            pos += len;
                        }
                    }
                    if loaded.len() == fds_disks.len() {
                        fds_disks = loaded;
                        println!("Loaded FDS disk save from {:?}", sav_path);
                    }
                } else if sav_data.len() <= prg_ram.len() {
                    prg_ram[..sav_data.len()].copy_from_slice(&sav_data);
                    println!("Loaded FDS save RAM from {:?}", sav_path);
                }
            }

            let fds_overall_crc = crc32(&rom);
            let fds_prg_rom_crc = crc32(&disksys);
            let cartridge = Cartridge {
                name: filepath.to_string(),
                prg_rom: disksys,
                chr_rom: Vec::new(),
                memory_mapper: 20,
                sub_mapper: 0,
                prg_size: 0,
                chr_size: 0,
                prg_size_minus_1: 0,
                chr_ram: vec![0u8; 0x2000],
                using_chr_ram: true,
                prg_ram,
                has_battery: true,
                alternative_nametable_arrangement: false,
                prg_vram: Vec::new(),
                nametable_horizontal_mirroring: true,
                fds_disks,
                trainer: Vec::new(),
                misc_rom: Vec::new(),
                mapper_chip: Box::new(crate::mapper::Mapper20::new(vec![])),
                mapper_cpu_cycle: 0,
                prg_rom_crc32: fds_prg_rom_crc,
                chr_rom_crc32: 0,
                overall_crc32: fds_overall_crc,
                is_vs_system: false,
                tv_system: TvSystem::Unknown,
            };
            
            let mut cartridge = cartridge;
            cartridge.mapper_chip = Box::new(crate::mapper::Mapper20::new(cartridge.fds_disks.clone()));

            println!("Loaded FDS ROM: {} ({} sides)", cartridge.name, cartridge.fds_disks.len());
            return Ok(cartridge);
        }

        let is_unf = filepath.to_lowercase().ends_with(".unf") || filepath.to_lowercase().ends_with(".unif") || &rom[0..4] == b"UNIF";

        if is_unf {
            if rom.len() < 32 || &rom[0..4] != b"UNIF" {
                return Err("Not a valid UNIF ROM file".to_string());
            }

            let mut mapper_name = String::new();
            let mut prg_chunks: Vec<Vec<u8>> = vec![Vec::new(); 16];
            let mut chr_chunks: Vec<Vec<u8>> = vec![Vec::new(); 16];
            let mut has_battery = false;
            let mut mirroring = 0;

            let mut offset = 32;
            while offset + 8 <= rom.len() {
                let chunk_type = std::str::from_utf8(&rom[offset..offset + 4]).unwrap_or("").to_string();
                let length = u32::from_le_bytes([
                    rom[offset + 4],
                    rom[offset + 5],
                    rom[offset + 6],
                    rom[offset + 7],
                ]) as usize;
                
                offset += 8;
                if offset + length > rom.len() {
                    break;
                }

                let chunk_data = &rom[offset..offset + length];

                if chunk_type == "MAPR" {
                    let mut s = String::new();
                    for &b in chunk_data {
                        if b == 0 {
                            break;
                        }
                        if b != b' ' {
                            s.push(b as char);
                        }
                    }
                    mapper_name = s;
                } else if chunk_type.starts_with("PRG") && chunk_type.len() == 4 {
                    if let Some(digit) = chunk_type.chars().nth(3).and_then(|c| c.to_digit(16)) {
                        let idx = digit as usize;
                        if idx < 16 {
                            prg_chunks[idx] = chunk_data.to_vec();
                        }
                    }
                } else if chunk_type.starts_with("CHR") && chunk_type.len() == 4 {
                    if let Some(digit) = chunk_type.chars().nth(3).and_then(|c| c.to_digit(16)) {
                        let idx = digit as usize;
                        if idx < 16 {
                            chr_chunks[idx] = chunk_data.to_vec();
                        }
                    }
                } else if chunk_type == "BATR" {
                    if !chunk_data.is_empty() {
                        has_battery = chunk_data[0] > 0;
                    }
                } else if chunk_type == "MIRR" {
                    if !chunk_data.is_empty() {
                        mirroring = chunk_data[0];
                    }
                }

                offset += length;
                while offset < rom.len() && rom[offset] == 0 {
                    offset += 1;
                }
            }

            let mut prg_rom = Vec::new();
            for chunk in prg_chunks {
                prg_rom.extend_from_slice(&chunk);
            }
            let mut chr_rom = Vec::new();
            for chunk in chr_chunks {
                chr_rom.extend_from_slice(&chunk);
            }

            if prg_rom.is_empty() || mapper_name.is_empty() {
                return Err("Invalid or empty UNIF ROM".to_string());
            }

            let normalized_name = if mapper_name.starts_with("NES-") 
                || mapper_name.starts_with("UNL-") 
                || mapper_name.starts_with("HVC-") 
                || mapper_name.starts_with("BTL-") 
                || mapper_name.starts_with("BMC-") {
                mapper_name[4..].to_string()
            } else {
                mapper_name.clone()
            };

            let memory_mapper = match normalized_name.as_str() {
                "11160" => 299,
                "12-IN-1" => 331,
                "190in1" => 300,
                "22211" => 132,
                "411120-C" => 287,
                "42in1ResetSwitch" => 226,
                "43272" => 227,
                "603-5052" => 238,
                "64in1NoRepeat" => 314,
                "70in1" | "70in1B" => 236,
                "810544-C-A1" => 261,
                "830425C-4391T" => 320,
                "8157" => 242,
                "8237" => 215,
                "830118C" => 348,
                "A65AS" | "JY-066" => 285,
                "ANROM" => 7,
                "AX5705" => 530,
                "BB" => 108,
                "BS-5" => 286,
                "CITYFIGHT" => 266,
                "COOLBOY" => 268,
                "CNROM" => 3,
                "CPROM" => 13,
                "D1038" => 59,
                "DANCE2000" => 518,
                "Dreamtech" | "DREAMTECH01" => 521,
                "EDU2000" => 329,
                "EKROM" | "ELROM" | "ETROM" | "EWROM" => 5,
                "FARID_SLROM_8-IN-1" => 323,
                "FARID_UNROM_8-IN-1" => 324,
                "FK23C" | "FK23CA" => 176,
                "FS304" => 162,
                "G-146" => 349,
                "GK-192" => 58,
                "GS-2004" => 283,
                "H2288" => 123,
                "HKROM" => 4,
                "KOF97" => 263,
                "KONAMI-QTAI" => 190,
                "K-3046" => 336,
                "KS7012" => 346,
                "KS7013B" => 312,
                "KS7016" => 306,
                "KS7017" => 303,
                "KS7031" => 305,
                "KS7032" => 142,
                "KS7037" => 307,
                "KS7057" => 302,
                "LH10" => 522,
                "LH32" => 125,
                "LH51" => 309,
                "MALISB" => 325,
                "MARIO1-MALEE2" | "Malee" => 55,
                "MHROM" => 66,
                "N625092" => 221,
                "NROM" | "NROM-128" | "NROM-256" | "RROM" | "RROM-128" => 0,
                "NTBROM" => 68,
                "NTD-03" => 290,
                "NovelDiamond" | "NovelDiamond9999999in1" => 54,
                "OneBus" => 256,
                "RESET-TXROM" => 313,
                "RET-CUFROM" => 29,
                "SA-002" => 136,
                "SA-0036" => 149,
                "SA-0037" => 148,
                "SA-009" => 160,
                "SA-016-1M" => 146,
                "SA-72007" => 145,
                "SA-72008" => 133,
                "SA-9602B" => 513,
                "SA-NROM" => 143,
                "SAROM" | "SBROM" | "SCROM" | "SEROM" | "SGROM" | "SKROM" | "SL1ROM" | "SLROM" | "SNROM" | "SOROM" => 1,
                "SC-127" => 35,
                "SL12" => 116,
                "SL1632" => 14,
                "SMB2J" => 304,
                "SUNSOFT_UNROM" => 93,
                "Sachen-74LS374N" => 150,
                "Sachen-74LS374NA" => 243,
                "Sachen-8259A" => 141,
                "Sachen-8259B" => 138,
                "Sachen-8259C" => 139,
                "Sachen-8259D" => 137,
                "Super24in1SC03" => 176,
                "SuperHIK8in1" => 45,
                "Supervision16in1" => 53,
                "T-230" => 529,
                "T-262" => 265,
                "TBROM" | "TEROM" | "TFROM" | "TGROM" | "TKROM" | "TKSROM" | "TLROM" | "TLSROM" | "TQROM" | "TR1ROM" | "TSROM" | "TVROM" => 4,
                "TC-U01-1.5M" => 147,
                "TEK90" => 90,
                "TF1201" => 298,
                "UNROM" | "UOROM" => 2,
                "UNROM-512-8" | "UNROM-512-16" | "UNROM-512-32" => 30,
                "VRC7" => 85,
                "YOKO" => 264,
                "158B" => 258,
                "DRAGONFIGHTER" => 292,
                "EH8813A" => 519,
                "HP898F" => 319,
                "F-15" => 259,
                "RT-01" => 328,
                "8-IN-1" => 333,
                "WS" => 332,
                "80013-B" => 274,
                "WAIXING-FW01" => 227,
                "HPxx" | "HP2018A" => 260,
                "DRIPGAME" => 284,
                "60311C" => 289,
                _ => return Err(format!("UNIF Board not supported: {}", mapper_name)),
            };

            let unif_submapper = match normalized_name.as_str() {
                "JY-066" => 1,
                _ if memory_mapper == 285 && normalized_name == "A65AS" && filepath.to_lowercase().contains("jy-066") => 1,
                _ => 0,
            };

            let using_chr_ram = chr_rom.is_empty() || matches!(memory_mapper, 268 | 286 | 522);
            let chr_ram = if using_chr_ram || matches!(memory_mapper, 268 | 286 | 522) {
                if memory_mapper == 268 {
                    let vram_shift = match rom.get(11) {
                        Some(&b) => b & 0x0F,
                        None => 0,
                    };
                    let battery_shift = match rom.get(11) {
                        Some(&b) => (b >> 4) & 0x0F,
                        None => 0,
                    };
                    let vram_kb = if vram_shift == 0 { 0 } else { (64usize << vram_shift) / 1024 };
                    let battery_kb = if battery_shift == 0 { 0 } else { (64usize << battery_shift) / 1024 };
                    let size = (vram_kb + battery_kb) * 1024;
                    vec![0u8; if size > 0 { size } else { 0x40000 }]
                } else if memory_mapper == 286 {
                    let mut ram = vec![0u8; chr_rom.len().max(0x2000)];
                    if !chr_rom.is_empty() {
                        let len = chr_rom.len().min(ram.len());
                        ram[..len].copy_from_slice(&chr_rom[..len]);
                    }
                    ram
                } else {
                    vec![0u8; 0x2000]
                }
            } else {
                Vec::new()
            };

            let nametable_horizontal_mirroring = match mirroring {
                0 => true,  // horizontal
                1 => false, // vertical
                _ => true,
            };

            let prg_size = (prg_rom.len() / 0x4000) as u8;
            let chr_size = (chr_rom.len() / 0x2000) as u8;
            let prg_size_minus_1 = if prg_size > 0 { prg_size - 1 } else { 0 };

            let prg_ram = vec![0u8; 0x2000];

            let header_placeholder = vec![0u8; 16];
            let raw_rom_placeholder = Vec::new();
            
            let mapper_chip = create_mapper(
                memory_mapper,
                unif_submapper,
                &header_placeholder,
                &raw_rom_placeholder,
                prg_size,
                using_chr_ram,
                has_battery,
                filepath,
            ).map_err(|e| format!("Error: {}", e))?;

            let unif_overall_crc = crc32(&rom);
            let unif_prg_crc = if prg_rom.is_empty() { 0 } else { crc32(&prg_rom) };
            let unif_chr_crc = if chr_rom.is_empty() { 0 } else { crc32(&chr_rom) };
            let cartridge = Cartridge {
                name: filepath.to_string(),
                prg_rom,
                chr_rom,
                memory_mapper,
                sub_mapper: unif_submapper,
                prg_size,
                chr_size,
                prg_size_minus_1,
                chr_ram,
                using_chr_ram,
                prg_ram,
                has_battery,
                alternative_nametable_arrangement: false,
                prg_vram: Vec::new(),
                nametable_horizontal_mirroring,
                fds_disks: Vec::new(),
                trainer: Vec::new(),
                misc_rom: Vec::new(),
                mapper_chip,
                mapper_cpu_cycle: 0,
                prg_rom_crc32: unif_prg_crc,
                chr_rom_crc32: unif_chr_crc,
                overall_crc32: unif_overall_crc,
                is_vs_system: false,
                tv_system: TvSystem::Unknown,
            };

            println!("Loaded UNIF ROM: {} (Board: {}, Mapper: {})", cartridge.name, mapper_name, memory_mapper);
            return Ok(cartridge);
        }

        let is_mfc = &rom[0..4] == b"mfc\0";
        let is_mfc_encrypted = is_mfc && rom.len() > 11 && rom[11] != 0;
        if is_mfc {
            let mfc_mapper = ((rom[6] & 0xF0) >> 4) | (rom[7] & 0xF0);
            rom = convert_mfc_to_ines(&rom)?;
            println!("MFC ROM detected (MFC mapper {}), converted to iNES 2.0", mfc_mapper);
        }

        if &rom[0..4] != b"NES\x1a" {
            return Err("Not a valid iNES ROM file".to_string());
        }

        let is_nes20 = (rom[7] & 0x0C) == 0x08;
        let is_wxn = !is_nes20 && rom.len() > 11 && rom[11] == 1;
        let mut memory_mapper = ((rom[6] >> 4) as u16) | ((rom[7] & 0xF0) as u16)
            | if is_nes20 { ((rom[8] & 0x0F) as u16) << 8 } else { 0 };
        if is_wxn && memory_mapper == 4 {
            memory_mapper = 256;
        }
        let is_vs_system = (rom[7] & 0x03) == 1 || memory_mapper == 99;
        let sub_mapper = if is_nes20 {
            (rom[8] >> 4) & 0x0F
        } else {
            0
        };

        let mut chr_size = rom[5];
        let has_trainer = (rom[6] & 4) != 0;
        let trainer_len = if has_trainer { 512 } else { 0 };

        let mut prg_size = rom[4];

        let chr_size_val = if is_nes20 {
            (rom[5] as usize) | ((rom[9] & 0xF0) as usize) << 4
        } else {
            rom[5] as usize
        };
        let mut using_chr_ram = chr_size_val == 0;

        let mut prg_rom_len = if prg_size == 0 {
            let after_header = 0x10 + trainer_len;
            let chr_bytes = chr_size_val * 0x2000;
            let remaining = rom.len().saturating_sub(after_header + chr_bytes);
            let detected_16kb = remaining / 0x4000;
            if detected_16kb > 0 {
                prg_size = detected_16kb.min(0xFF) as u8;
            } else {
                prg_size = 1;
            }
            detected_16kb * 0x4000
        } else if is_nes20 && (rom[9] & 0x0F) == 0x0F {
            let lo = rom[4] as usize;
            let prg_bytes = ((2 * (lo & 3) + 1) << (lo >> 2)) as usize;
            prg_size = (prg_bytes / 0x4000).min(0xFF) as u8;
            prg_bytes
        } else if is_nes20 {
            let prg_size_12bit = (rom[4] as usize) | (((rom[9] & 0x0F) as usize) << 8);
            prg_size = prg_size_12bit.min(0xFF) as u8;
            prg_size_12bit * 0x4000
        } else {
            prg_size as usize * 0x4000
        };

        let chr_rom_len = chr_size_val * 0x2000;
        let after_header = 0x10 + trainer_len;
        let file_prg_avail = rom.len().saturating_sub(after_header + chr_rom_len);
        if file_prg_avail > prg_rom_len {
            prg_rom_len = file_prg_avail;
            prg_size = (prg_rom_len / 0x4000).min(0xFF) as u8;
        }

        let prg_size_minus_1 = prg_size.wrapping_sub(1);

        if rom.len() < 0x10 + trainer_len + prg_rom_len + chr_rom_len {
            return Err(format!(
                "ROM file too small: expected {} bytes, got {}",
                0x10 + trainer_len + prg_rom_len + chr_rom_len,
                rom.len()
            ));
        }

        let mut trainer = Vec::new();
        if has_trainer {
            trainer = rom[0x10..0x10 + 512].to_vec();
        }

        let mut prg_rom = vec![0u8; prg_rom_len];
        prg_rom.copy_from_slice(&rom[0x10 + trainer_len..0x10 + trainer_len + prg_rom_len]);

        let mut chr_rom = vec![0u8; chr_rom_len];
        if chr_rom_len > 0 {
            chr_rom.copy_from_slice(&rom[0x10 + trainer_len + prg_rom_len..0x10 + trainer_len + prg_rom_len + chr_rom_len]);
        }

        if is_wxn || is_mfc_encrypted {
            crate::wxn::decrypt_wxn(&mut prg_rom, &mut chr_rom);
            if is_wxn {
                println!("WXN encrypted ROM detected, decrypting PRG/CHR data");
            } else {
                println!("MFC encrypted ROM detected, decrypting PRG/CHR data");
            }
        }

        if is_mfc && memory_mapper == 164 {
            let scan_len = prg_rom.len().min(0x8000);
            let mut found = false;
            for i in 0..scan_len.saturating_sub(2) {
                if prg_rom[i] == 0x8D && prg_rom[i + 1] == 0x00 && prg_rom[i + 2] == 0x50 {
                    found = true;
                    break;
                }
            }
            if !found {
                memory_mapper = 227;
                println!("MFC mapper 61: no STA $5000 found in first 32K, using Waixing FW01 (mapper 227)");
            }
        }

        println!("ROM SIZES: prg_size={} prg_rom_len=${:X} chr_size={} chr_rom_len=${:X} prg_rom_file_len=${:X}",
            prg_size, prg_rom_len, chr_size_val, chr_rom_len, prg_rom.len());

        let game_data_end = 0x10 + trainer_len + prg_rom_len + chr_rom_len;
        let i_nes_game_crc32 = if is_wxn || is_mfc_encrypted {
            let mut game_data = prg_rom.clone();
            game_data.extend_from_slice(&chr_rom);
            crate::crc::crc32(&game_data)
        } else {
            crate::crc::crc32(&rom[0x10 + trainer_len..game_data_end])
        };
        if crate::crc::lookup_crc_override(i_nes_game_crc32).is_some() {
            let mut mapper_override = memory_mapper;
            let mut has_chr_rom = chr_rom_len > 0;
            crate::crc::apply_crc_override(i_nes_game_crc32, &mut mapper_override, &mut has_chr_rom);
            if mapper_override != memory_mapper {
                memory_mapper = mapper_override;
            }
            if !has_chr_rom && chr_rom_len > 0 {
                chr_rom.clear();
                using_chr_ram = true;
                chr_size = 0;
            }
        }

        let misc_rom = if rom.len() > 0x10 + trainer_len + prg_rom_len + chr_rom_len {
            rom[0x10 + trainer_len + prg_rom_len + chr_rom_len..].to_vec()
        } else {
            Vec::new()
        };

        let nrom_cfg = if memory_mapper == 0 {
            Some(crate::mappers::nrom::NromConfig::for_ines(&rom[0..16], chr_size))
        } else {
            None
        };

        let uxrom_cfg = if memory_mapper == 2 || memory_mapper == 71 {
            Some(crate::mappers::uxrom::UxromConfig::for_ines(
                &rom[0..16],
                sub_mapper,
                chr_size,
            ))
        } else {
            None
        };

        let cnrom_cfg = if memory_mapper == 3 {
            Some(crate::mappers::cnrom::CnromConfig::for_ines(&rom[0..16], sub_mapper))
        } else {
            None
        };

        let mmc3_cfg = if memory_mapper == 4 || memory_mapper == 12 {
            Some(crate::mappers::mmc3::Mmc3Config::for_ines(
                &rom[0..16],
                sub_mapper,
                chr_size,
                &rom,
                filepath,
            ))
        } else {
            None
        };

        let chr_ram = if memory_mapper == 13 {
            vec![0u8; 0x4000]
        } else if let Some(ref cfg) = nrom_cfg {
            vec![0u8; cfg.chr_ram_size]
        } else if let Some(ref cfg) = uxrom_cfg {
            vec![0u8; cfg.chr_ram_size]
        } else if let Some(ref cfg) = mmc3_cfg {
            vec![0u8; cfg.chr_ram_size]
        } else if memory_mapper == 6 || memory_mapper == 17 {
            vec![0u8; 32 * 1024]
        } else if memory_mapper == 77 {
            vec![0u8; 6 * 1024]
        } else if memory_mapper == 34 {
            vec![0u8; 8 * 1024]
        } else if memory_mapper == 74 || memory_mapper == 191 || memory_mapper == 194 || memory_mapper == 252 || memory_mapper == 253 {
            vec![0u8; 2 * 1024]
        } else if memory_mapper == 192 || memory_mapper == 195 {
            vec![0u8; 4 * 1024]
        } else if memory_mapper == 119 {
            vec![0u8; 8 * 1024]
        } else if matches!(memory_mapper, 306 | 307 | 309 | 310 | 312) {
            vec![0u8; 8 * 1024]
        } else if memory_mapper == 314 && chr_size == 0 {
            vec![0u8; 8 * 1024]
        } else if memory_mapper == 111 {
            vec![0u8; 32 * 1024]
        } else if memory_mapper == 286 {
            let mut ram = vec![0u8; chr_rom.len().max(0x2000)];
            if !chr_rom.is_empty() {
                let len = chr_rom.len().min(ram.len());
                ram[..len].copy_from_slice(&chr_rom[..len]);
            }
            ram
        } else if memory_mapper == 403 {
            vec![0u8; 32 * 1024]
        } else if memory_mapper == 442 {
            vec![0u8; 8 * 1024]
        } else if is_nes20 && rom.len() > 11 && (rom[11] & 0xFF) != 0 {
            let vram_shift = rom[11] & 0x0F;
            let battery_shift = (rom[11] >> 4) & 0x0F;
            let vram_bytes = if vram_shift == 0 { 0 } else { 64usize << vram_shift };
            let battery_bytes = if battery_shift == 0 { 0 } else { 64usize << battery_shift };
            let total_ram = vram_bytes + battery_bytes;
            vec![0u8; total_ram]
        } else if matches!(memory_mapper, 233 | 235 | 237 | 241 | 242 | 245 | 247 | 262 | 268 | 372 | 375 | 381 | 382 | 393 | 396 | 399 | 400 | 402 | 448 | 452 | 453 | 454 | 460 | 522) || crate::mappers::one_bus::is_onebus_mapper(memory_mapper) {
            vec![0u8; 0x2000]
        } else if using_chr_ram {
            vec![0u8; 0x2000]
        } else {
            Vec::new()
        };
        let mut using_chr_ram = using_chr_ram || !chr_ram.is_empty();
        if memory_mapper == 13 {
            using_chr_ram = !chr_ram.is_empty();
        }
        if memory_mapper == 77 {
            using_chr_ram = true;
        }
        if memory_mapper == 286 || matches!(memory_mapper, 74 | 119 | 111 | 191 | 192 | 194 | 195 | 233 | 235 | 237 | 241 | 242 | 245 | 247 | 252 | 253 | 262 | 306 | 307 | 309 | 310 | 312 | 372 | 375 | 381 | 382 | 393 | 396 | 399 | 400 | 402 | 403 | 442 | 448 | 452 | 453 | 454 | 460 | 522) || crate::mappers::one_bus::is_onebus_mapper(memory_mapper) {
            using_chr_ram = true;
        }
        if memory_mapper == 314 && chr_size == 0 {
            using_chr_ram = true;
        }

        let mut nametable_horizontal_mirroring = (rom[6] & 1) == 0;
        let mut alternative_nametable_arrangement = (rom[6] & 8) != 0;
        crate::crc::apply_crc_mirror_override(i_nes_game_crc32, &mut nametable_horizontal_mirroring, &mut alternative_nametable_arrangement);

        if memory_mapper == 262 {
            alternative_nametable_arrangement = true;
        }

        let prg_vram = if alternative_nametable_arrangement {
            vec![0u8; 0x800]
        } else {
            Vec::new()
        };

        let mut has_battery = (rom[6] & 2) != 0;
        if memory_mapper == 16 || memory_mapper == 159 || memory_mapper == 157 {
            has_battery = true;
        }

        let mmc5_cfg = if memory_mapper == 5 {
            Some(crate::mappers::mmc5::Mmc5Config::for_ines(
                &rom[0..16],
                &rom,
                has_battery,
            ))
        } else {
            None
        };

        let ffe_cfg = if memory_mapper == 6 || memory_mapper == 17 {
            Some(if memory_mapper == 17 {
                crate::mappers::ffe::FfeConfig::mapper17(&rom[0..16], has_battery)
            } else {
                crate::mappers::ffe::FfeConfig::mapper6(&rom[0..16], sub_mapper, has_battery)
            })
        } else {
            None
        };

        let mut prg_ram = if let Some(ref cfg) = nrom_cfg {
            vec![0u8; cfg.prg_ram_size]
        } else if let Some(ref cfg) = uxrom_cfg {
            vec![0u8; cfg.prg_ram_size]
        } else if let Some(ref cfg) = cnrom_cfg {
            vec![0u8; cfg.prg_ram_size]
        } else if let Some(ref cfg) = mmc3_cfg {
            vec![0u8; cfg.prg_ram_size]
        } else if let Some(ref cfg) = mmc5_cfg {
            vec![0u8; cfg.wram_size]
        } else if let Some(ref cfg) = ffe_cfg {
            vec![0u8; cfg.wram_size]
        } else if memory_mapper == 10 {
            vec![0u8; 0x2000]
        } else if memory_mapper == 15 {
            vec![0u8; 8 * 1024]
        } else if memory_mapper == 32 {
            vec![0u8; 8 * 1024]
        } else if memory_mapper == 34 {
            vec![0u8; 8 * 1024]
        } else if memory_mapper == 69 {
            vec![0u8; crate::mappers::fme7::wram_size(&rom[0..16])]
        } else if memory_mapper == 153 {
            vec![0u8; crate::mappers::bandai::prg_ram_size(153)]
        } else if memory_mapper == 16 || memory_mapper == 159 || memory_mapper == 157 {
            Vec::new()
        } else if memory_mapper == 543 {
            vec![0u8; 0x10000]
        } else if memory_mapper == 1
            || memory_mapper == 105
            || memory_mapper == 155
            || memory_mapper == 171
        {
            let cfg = crate::mappers::mmc1::Mmc1Config::for_ines(
                &rom[0..16],
                &rom,
                memory_mapper,
                sub_mapper,
                prg_size,
                using_chr_ram,
                has_battery,
            );
            vec![0u8; cfg.wram_size]
        } else {
            vec![0u8; 0x2000]
        };
        let bandai_sav = if has_battery
            && matches!(memory_mapper, 16 | 153 | 157 | 159)
        {
            let sav_path = crate::config::save_file_path(filepath);
            fs::read(&sav_path).ok()
        } else {
            None
        };

        if has_battery && bandai_sav.is_none() {
            let sav_path = crate::config::save_file_path(filepath);
            if let Ok(sav_data) = fs::read(&sav_path) {
                let save_len = if let Some(ref cfg) = mmc5_cfg {
                    cfg.battery_save_size.min(prg_ram.len())
                } else if let Some(ref cfg) = ffe_cfg {
                    cfg.battery_save_size.min(prg_ram.len())
                } else {
                    prg_ram.len()
                };
                let copy_len = sav_data.len().min(save_len);
                if copy_len > 0 {
                    prg_ram[..copy_len].copy_from_slice(&sav_data[..copy_len]);
                    println!("Loaded save RAM from {:?}", sav_path);
                }
            }
        }

        let mapper_chip = create_mapper(
            memory_mapper,
            sub_mapper,
            &rom[0..16],
            &rom,
            prg_size,
            using_chr_ram,
            has_battery,
            filepath,
        ).map_err(|e| format!("Error: {}", e))?;

        if (memory_mapper == 6 || memory_mapper == 17) && !trainer.is_empty() {
            crate::mappers::ffe::install_trainer(&trainer, &mut prg_ram);
        }

        let ines_overall_crc = crc32(&rom);
        let ines_prg_crc = if prg_rom.is_empty() { 0 } else { crc32(&prg_rom) };
        let ines_chr_crc = if chr_rom.is_empty() { 0 } else { crc32(&chr_rom) };
        let tv_system = TvSystem::from_ines_header(&rom, is_nes20);
        let mut cartridge = Cartridge {
            name: filepath.to_string(),
            prg_rom,
            chr_rom,
            memory_mapper,
            sub_mapper,
            prg_size,
            chr_size,
            prg_size_minus_1,
            chr_ram,
            using_chr_ram,
            prg_ram,
            has_battery,
            alternative_nametable_arrangement,
            prg_vram,
            nametable_horizontal_mirroring,
            fds_disks: Vec::new(),
            trainer,
            misc_rom,
            mapper_chip,
            mapper_cpu_cycle: 0,
            prg_rom_crc32: ines_prg_crc,
            chr_rom_crc32: ines_chr_crc,
            overall_crc32: ines_overall_crc,
            is_vs_system,
            tv_system,
        };

        if memory_mapper == 100 {
            crate::mappers::mapper100::install_mapper100_trainer(&mut cartridge);
        }

        if let Some(sav_data) = bandai_sav {
            let sav_path = std::path::Path::new(filepath).with_extension("sav");
            let mut mapper = std::mem::replace(
                &mut cartridge.mapper_chip,
                Box::new(crate::mapper::MapperNROM::new(
                    crate::mapper::NromConfig::default(),
                )),
            );
            mapper.load_battery_save(&mut cartridge, &sav_data);
            cartridge.mapper_chip = mapper;
            println!("Loaded Bandai save from {:?}", sav_path);
        }

        println!("Loaded ROM: {} (Mapper {}, Sub-mapper {})", cartridge.name, cartridge.memory_mapper, cartridge.sub_mapper);
        if cartridge.alternative_nametable_arrangement {
            println!("Alternative nametable arrangement detected (PRG VRAM enabled)");
        }

        Ok(cartridge)
    }
}
fn ccitt(mut crc: u16, bit: i32) -> u16 {
    let bitc = crc & 1;
    crc >>= 1;
    if (bitc ^ (bit as u16)) != 0 {
        crc ^= 0x8408;
    }
    crc
}

pub(crate) fn ccitt_8(mut crc: u16, b: u8) -> u16 {
    for i in 0..8 {
        let bit = ((b >> i) & 1) as i32;
        crc = ccitt(crc, bit);
    }
    crc
}

fn write_block(dest: &mut Vec<u8>, data: &[u8], pregap: usize) {
    for _ in 0..(pregap.saturating_sub(1)) {
        dest.push(0);
    }
    let mut crc = 0u16;
    dest.push(0x80);
    crc = ccitt_8(crc, 0x80);
    for &b in data {
        dest.push(b);
        crc = ccitt_8(crc, b);
    }
    dest.push((crc & 0xFF) as u8);
    dest.push((crc >> 8) as u8);
}

fn fix_fds_disk_side(disk: &[u8], _side_index: usize, disk_crc32: u32) -> Result<Vec<u8>, String> {
    let mut offset = 0;
    let mut ret = Vec::new();
    let mut current_file_size = 0;
    let mut current_file_name = [0u8; 8];
    let mut pending_block3: Option<[u8; 16]> = None;

    let is_hokusai = disk_crc32 == 0x1D234703;
    let is_jingorou = disk_crc32 == 0x72A0BD81;

    while offset < disk.len() {
        let block_type = disk[offset];
        if block_type == 0 && ret.is_empty() {
            break;
        }

        match block_type {
            0x01 => {
                if offset + 56 > disk.len() { break; }
                write_block(&mut ret, &disk[offset..offset + 56], 3500);
                offset += 56;
            }
            0x02 => {
                if offset + 2 > disk.len() { break; }
                write_block(&mut ret, &disk[offset..offset + 2], 120);
                offset += 2;
            }
            0x03 => {
                if offset + 16 > disk.len() { break; }
                current_file_size = (disk[offset + 13] as usize) | ((disk[offset + 14] as usize) << 8);
                current_file_name.copy_from_slice(&disk[offset + 3..offset + 11]);
                if is_hokusai {
                    let mut hdr = [0u8; 16];
                    hdr.copy_from_slice(&disk[offset..offset + 16]);
                    pending_block3 = Some(hdr);
                } else {
                    write_block(&mut ret, &disk[offset..offset + 16], 120);
                }
                offset += 16;
            }
            0x04 => {
                if is_hokusai && offset >= 0x470 && current_file_size == 1
                    && &current_file_name == b"KYODAKLL"
                    && offset + 0xC001 <= disk.len()
                {
                    if let Some(mut hdr) = pending_block3.take() {
                        hdr[13] = 0x00;
                        hdr[14] = 0xC0; 
                        write_block(&mut ret, &hdr, 120);
                    }
                    write_block(&mut ret, &disk[offset..offset + 0xC001], 120);
                    break;
                }
                if is_jingorou && offset >= 0x570 && current_file_size == 1
                    && &current_file_name == b"KYODAKLL"
                    && offset + 0xE001 <= disk.len()
                {
                    if let Some(mut hdr) = pending_block3.take() {
                        hdr[13] = 0x00;
                        hdr[14] = 0xE0;
                        write_block(&mut ret, &hdr, 120);
                    }
                    write_block(&mut ret, &disk[offset..offset + 0xE001], 120);
                    break;
                }
                if let Some(hdr) = pending_block3.take() {
                    write_block(&mut ret, &hdr, 120);
                }
                if offset + current_file_size + 1 > disk.len() { break; }
                write_block(&mut ret, &disk[offset..offset + current_file_size + 1], 120);
                offset += current_file_size + 1;
            }
            _ => {
                if let Some(hdr) = pending_block3.take() {
                    write_block(&mut ret, &hdr, 120);
                }
                let mut remaining = Vec::new();
                if offset == 0x5F91 && block_type == 0x34 {
                    remaining.push(0x00);
                    remaining.extend_from_slice(&disk[offset..]);
                } else {
                    remaining.extend_from_slice(&disk[offset..]);
                }
                let end_pos = remaining.iter().rposition(|&b| b != 0).map(|i| i + 1).unwrap_or(0);
                if end_pos > 0 {
                    write_block(&mut ret, &remaining[..end_pos], 120);
                    offset += if offset == 0x5F91 && block_type == 0x34 { end_pos.saturating_sub(1) } else { end_pos };
                } else {
                    break;
                }
            }
        }
    }

    while ret.len() < 65500 {
        ret.push(0);
    }
    
    Ok(ret)
}

fn write_qd_block(dest: &mut Vec<u8>, data: &[u8], pregap: usize) {
    for _ in 0..(pregap.saturating_sub(1)) {
        dest.push(0);
    }
    dest.push(0x80);
    dest.extend_from_slice(data);
}

fn fix_qd_disk_side(disk: &[u8], _side_index: usize, _disk_crc32: u32) -> Result<Vec<u8>, String> {
    let mut offset = 0;
    let mut ret = Vec::new();
    let mut current_file_size = 0;

    while offset < disk.len() {
        let block_type = disk[offset];
        if block_type == 0 && !ret.is_empty() {
            break;
        }

        match block_type {
            0x01 => {
                if offset + 58 > disk.len() { break; }
                write_qd_block(&mut ret, &disk[offset..offset + 58], 3500);
                offset += 58;
            }
            0x02 => {
                if offset + 4 > disk.len() { break; }
                write_qd_block(&mut ret, &disk[offset..offset + 4], 120);
                offset += 4;
            }
            0x03 => {
                if offset + 18 > disk.len() { break; }
                current_file_size = (disk[offset + 13] as usize) | ((disk[offset + 14] as usize) << 8);
                write_qd_block(&mut ret, &disk[offset..offset + 18], 120);
                offset += 18;
            }
            0x04 => {
                let block_len = current_file_size + 3;
                if offset + block_len > disk.len() { break; }
                write_qd_block(&mut ret, &disk[offset..offset + block_len], 120);
                offset += block_len;
            }
            _ => {
                let remaining = &disk[offset..];
                let end_pos = remaining.iter().rposition(|&b| b != 0).map(|i| i + 1).unwrap_or(0);
                if end_pos > 0 {
                    write_qd_block(&mut ret, &remaining[..end_pos], 120);
                    offset += end_pos;
                } else {
                    break;
                }
            }
        }
    }

    while ret.len() < 65536 {
        ret.push(0);
    }

    Ok(ret)
}
