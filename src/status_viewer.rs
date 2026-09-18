// status window!!!
use crate::emulator::Emulator;
use crate::{draw_rect, draw_text, point_in_rect, UiColors, MenuState};
use winit::event::{MouseButton, VirtualKeyCode};

pub fn get_mapper_name(mapper: u16) -> &'static str {
    match mapper {
        0 => "Nintendo NROM",
        1 => "Nintendo SxROM (MMC1B)",
        2 => "Nintendo UxROM",
        3 => "Nintendo CNROM",
        4 => "Nintendo TxROM/HKROM",
        5 => "Nintendo ExROM",
        6 => "Front Fareast Magic Card 1M/2M",
        7 => "Nintendo AxROM",
        9 => "Nintendo PNROM",
        10 => "Nintendo FJROM/FKROM",
        11 => "Color Dreams",
        12 => "SL-5020B/Front Fareast Magic Card 4M",
        13 => "Nintendo CPROM",
        14 => "SL-1632",
        15 => "K-1029",
        16 => "Bandai FCG",
        17 => "Front Fareast Super Magic Card",
        18 => "Jaleco SS8806",
        19 => "Namco N129/N163",
        21 => "Konami VRC4a/VRC4c",
        22 => "Konami VRC2 A1/A0+CHR shift",
        23 => "Konami VRC2b/VRC4e/VRC4f",
        24 => "Konami 351951",
        25 => "Konami VRC2c/VRC4b/VRC4d",
        26 => "Konami 351949A",
        27 => "CC-21",
        28 => "Action 53",
        29 => "RET-CUFROM",
        30 => "UNROM-512",
        31 => "2A03 Puritans Album",
        32 => "Irem G-101",
        33 => "Taito TC0190/TC0390",
        34 => "AVE NINA-001/Nintendo BNROM",
        35 => "EL870914C",
        36 => "TXC 01-22000-200/400, 01-22110-200",
        37 => "Nintendo ZZ",
        38 => "PCI556",
        40 => "NTDEC 2722",
        41 => "NTDEC 2399",
        42 => "Kaiser KS-7050",
        43 => "TONY-I",
        44 => "Super HiK 7-in-1 (MMC3)",
        45 => "TC3294",
        46 => "GameStation/RumbleStation",
        47 => "Nintendo NES-QJ",
        48 => "Taito TC0190+PAL16R4/TC0690",
        49 => "820401/T-217",
        50 => "N-32 (761214)",
        51 => "820718C",
        52 => "ING005",
        53 => "Supervision 16-in-1",
        55 => "NCN-35A",
        56 => "Kaiser SMB3",
        57 => "GK 6-in-1",
        58 => "GK-192",
        59 => "BS-01/VT1512A",
        60 => "Reset-based NROM-128",
        61 => "GS-2017/BS-N032",
        62 => "K-1016/N-190B",
        63 => "NTDEC 2291",
        64 => "Tengen 800032",
        65 => "Irem H-3001",
        66 => "Nintendo GNROM/MHROM",
        67 => "Sunsoft-3 ASIC",
        68 => "Sunsoft-4 ASIC",
        69 => "Sunsoft-5 ASIC",
        70 => "Bandai UOROM",
        71 => "BIC BF9093/BF9097",
        72 => "Jaleco JF-17",
        73 => "Konami VRC3",
        74 => "43-393/860908C",
        75 => "Konami VRC1",
        76 => "Namco 3446",
        77 => "Irem LROG017",
        78 => "Jaleco JF-16/Irem IF-12",
        79 => "AVE NINA-003",
        80 => "Taito P3-33/34/36",
        81 => "NTDEC N715021",
        82 => "Taito P3-044 (wrong PRG order)",
        83 => "Cony",
        85 => "Konami VRC7",
        86 => "Jaleco JF-13",
        87 => "Jaleco/Konami CNROM",
        88 => "Namco 3433",
        89 => "Sunsoft-2",
        90 => "EL861226C",
        91 => "EJ-006-1/ YY830624C/JY830848C",
        92 => "Jaleco JF-19",
        93 => "Sunsoft-2 ASIC",
        94 => "Nintendo UN1ROM",
        95 => "Namco 3425",
        96 => "Oeka Kids",
        97 => "Irem TAM-S1",
        99 => "Nintendo Vs. System",
        100 => "Nesticle MMC3",
        101 => "Jaleco/Konami CNROM with wrong bit order",
        103 => "Whirlwind Manu LH30",
        104 => "Pegasus 5-in-1",
        105 => "NES-EVENT",
        106 => "890418",
        107 => "Magic Dragon",
        108 => "DH-08",
        111 => "GTROM",
        112 => "NTDEC MMC3",
        113 => "HES NTD-8",
        114 => "6122",
        115 => "SFC-02B/-03/-004",
        116 => "SOMARI-P",
        117 => "Future Media",
        118 => "Nintendo TKSROM/TLSROM",
        119 => "Nintendo TQROM",
        120 => "FDS Tobidase Daisakusen",
        121 => "A9711/A9713",
        122 => "JY043",
        123 => "H2288",
        124 => "Super Game Mega Type 3",
        125 => "Whirlwind Manu LH32",
        126 => "TEC9719 with swapped CHR",
        127 => "Double Dragon pirate",
        128 => "4-in-1",
        132 => "TXC 01-22003-400/01-22111-100/01-22270-000",
        133 => "3009/72008",
        134 => "WX-KB4K/T4A54A/BS-5652/A9716",
        135 => "TC-021A",
        136 => "3011/SA-002",
        137 => "SA8259D",
        138 => "SA8259B",
        139 => "SA8259C",
        140 => "Jaleco GNROM",
        141 => "2M-RAM-COB",
        142 => "Kaiser KS-7032",
        143 => "(TC-A001-72P/SA-014)",
        144 => "AGCI-50282",
        145 => "(SA-72007)",
        146 => "3015/SA-016",
        147 => "3018",
        148 => "SA-008-A",
        149 => "(SA-0036)",
        150 => "SA-015/SA-630",
        152 => "Bandai UOROM 1SM",
        153 => "Bandai FCG with 8 KiB PRG-RAM",
        154 => "Namco 3453",
        155 => "Nintendo SxROM (MMC1A)",
        156 => "ROM Controller DIS23C01  245",
        157 => "Bandai Datach Joint ROM System",
        158 => "Tengen 800037",
        159 => "Bandai FCG with 24C01 EEPROM",
        162 => "FS304",
        163 => "FC-001",
        164 => "cy2000-3",
        165 => "Fire Emblem",
        168 => "Racermate Challenge 2",
        169 => "Educational Computers",
        170 => "NROM",
        171 => "BBK",
        172 => "Super Mega SMCYII-900",
        173 => "Idea-Tek ET.xx",
        174 => "NTDEC 5-in-1",
        175 => "Kaiser KS-122",
        176 => "FS005/FS006/FK23C(A)",
        177 => "Hengge Dianzi",
        178 => "FS305/ NJ0430/PB030703-1x1",
        180 => "Inverse UNROM",
        182 => "YH-001",
        183 => "09035",
        184 => "Sunsoft-1 ASIC",
        185 => "Nintendo CNROM+Security",
        186 => "Family Study Box by Fukutake Shoten",
        187 => "A98402",
        188 => "Bandai Karaoke Studio",
        189 => "TXC 01-22017-000/01-22018-400/FC-001EM(C)/K-1069A",
        190 => "Zemina",
        191 => "4-in-1",
        192 => "FS308",
        193 => "NTDEC 2394",
        195 => "FS303",
        196 => "MRCM UT1374",
        197 => "TLROM-512",
        198 => "BJ-0026",
        199 => "FS309",
        200 => "36-in-1",
        201 => "21-in-1",
        202 => "SP60 150-in-1",
        203 => "35-in-1",
        204 => "204",
        205 => "JC-016-2",
        206 => "Namco N118",
        207 => "Taito Ashura",
        208 => "SL-37017",
        209 => "YY850629C",
        210 => "Namco N175/N340",
        211 => "EL860339C",
        212 => "CS669",
        213 => "EJ-3003/820428-C",
        214 => "Super Gun 20-in-1",
        215 => "Realtec 823x(A)",
        216 => "Bonza",
        217 => "GI 9549, ET-450",
        218 => "Magic Floor",
        219 => "A9746",
        221 => "NTDEC N625092",
        222 => "810343-C",
        224 => "KT-008",
        225 => "ET-4310/K-1010",
        226 => "0380/910307",
        227 => "FW01",
        228 => "Action 52",
        229 => "SC 0892",
        230 => "CTC-43A",
        231 => "20-in-1",
        232 => "BIC BF9096",
        233 => "Reset-based Tsang Hai 4+4 Mib",
        234 => "Maxi 15",
        235 => "Golden Game modular multicart",
        236 => "Realtec 8024/8031/8099/8106/8155",
        237 => "Teletubbies 420-in-1",
        238 => "Sakano MMC3",
        239 => "OK-043",
        240 => "/",
        241 => "BNROM with WRAM",
        242 => "43-272/ET-113",
        243 => "SA-020A",
        244 => "C&E Decathlon",
        245 => "FS003",
        246 => "G0151-1",
        248 => "SFC-02B/-03/-004",
        249 => "43-319",
        250 => "L4015",
        252 => "FS???",
        253 => "F009S",
        254 => "Pikachu Y2K",
        255 => "128-in-1",
        256 => "OneBus",
        257 => "PEC-586",
        258 => "Shanghai Paradise 158B",
        259 => "F-15",
        260 => "HP10xx-HP20xx",
        261 => "810544-C-A1/NTDEC 2746",
        262 => "Street Heroes",
        263 => "S.M.I. NSM-xxx",
        264 => "Yoko",
        265 => "T-262",
        266 => "City Fighter IV",
        267 => "EL861121C",
        268 => "KP6022/AA6023 ASIC",
        269 => "Games Xplosion 121-in-1",
        270 => "VT42xx",
        271 => "MGC-026",
        272 => "J-2012-II",
        273 => "J-3?-C",
        274 => "80013-B",
        277 => "09-078",
        280 => "K-3017",
        281 => "YY860417C",
        282 => "860224C",
        283 => "GS-2004/GS-2013",
        284 => "Drip",
        285 => "A65AS",
        286 => "BS-5",
        287 => "811120-C/810849-C",
        288 => "GKCXIN1",
        289 => "60311C/N76A-1",
        290 => "Asder 20-in-1",
        291 => "Super 2-in-1",
        292 => "BMW8544",
        293 => "BMC NEWSTAR 12-IN-1/76-IN-1",
        294 => "63-1601",
        295 => "YY860216C",
        296 => "V.R. Technology VT32",
        297 => "TXC 01-22110-000",
        298 => "NTDEC 1201",
        299 => "TXC 6-in-1/HS-011",
        300 => "190-in-1",
        301 => "K-3003",
        302 => "Kaiser KS-7057",
        303 => "Kaiser KS-7017",
        304 => "09-034A",
        305 => "Kaiser KS-7031",
        306 => "Kaiser KS-7016",
        307 => "Kaiser KS-7037",
        308 => "NTDEC 2131",
        309 => "Whirlwind Manu LH51",
        310 => "K-1053",
        311 => "SMB2JX",
        312 => "Kaiser KS-7013B",
        313 => "Reset-based TKROM multicart",
        314 => "64-in-1 No Repeat",
        315 => "830134C",
        319 => "HP-898F",
        320 => "830425C-4391T/T-259",
        321 => "820310",
        322 => "6-15-C/K-3033",
        323 => "FARID SLROM",
        324 => "FARID UNROM",
        325 => "Mali Splash Bomb",
        326 => "Gryzor Bootleg",
        327 => "10-24-C-A1",
        328 => "RT-01",
        329 => "EDU2000",
        330 => "Sangokushi II bootleg",
        331 => "12-in-1",
        332 => "WS-1001",
        333 => "New Star 8-in-1",
        334 => "821202C",
        335 => "CTC-09",
        336 => "K-3046",
        337 => "CTC-12IN1",
        338 => "SA005-A",
        339 => "K-3006/TL 8058",
        340 => "K-3008/K-3032/K-3036/K-3055",
        341 => "TJ-03",
        342 => "COOLGIRL",
        343 => "I030/0365",
        344 => "GN-26",
        345 => "L6IN1",
        346 => "Kaiser KS-7012",
        347 => "Kaiser KS-7030",
        348 => "830118C",
        349 => "G-146",
        350 => "891227",
        351 => "Techline XB",
        352 => "Reset-based NROM-256 (Mirroring dependent on game)",
        353 => "81-03-05-C",
        354 => "FAM250/81-01-39-C/SCHI-24",
        355 => "3D-BLOCK",
        356 => "JY-208",
        357 => "P3117",
        358 => "YY860606C",
        359 => "GCL8050/SB-5013/841242C",
        360 => "P3150",
        361 => "YY841101C",
        362 => "830506C",
        363 => "5069",
        364 => "JY830832C",
        365 => "Asder PC-95",
        366 => "GN-45",
        367 => "JC-016-2 variant",
        368 => "YUNG-08",
        369 => "N49C-300",
        370 => "F600",
        371 => "PEC-586 (Spanish)",
        372 => "SFX-12",
        373 => "SFX-13",
        374 => "Reset-based SLROM multicart",
        375 => "135-in-1",
        376 => "YY841155C, Realtec 9056",
        377 => "EL860947C",
        378 => "8-in-1 AOROM+UNROM",
        379 => "35BH-1",
        380 => "970630C/KN-35A",
        381 => "KN-42",
        382 => "830928C",
        383 => "YY840708C",
        384 => "L1A16",
        385 => "NTDEC 2779",
        386 => "YY860729C",
        387 => "YY850735C",
        388 => "YY850835C",
        389 => "Caltron 9-in-1",
        390 => "Realtec 8031",
        391 => "BS-110",
        392 => "00202650",
        393 => "820720C",
        394 => "HSK007",
        395 => "Realtec 8210",
        396 => "YY850437C",
        397 => "YY850439C",
        398 => "YY840820C",
        399 => "BATMAP-000",
        400 => "8BIT-XMAS",
        401 => "KC885",
        402 => "J-2282",
        403 => "89433",
        404 => "JY012005",
        405 => "UMC UM6578",
        406 => "Impact Soft",
        407 => "Win, Lose & Draw",
        408 => "VT4FFx",
        409 => "retroUSB DPCMcart",
        410 => "JY-302",
        411 => "A88S-1",
        412 => "FK-206 JG",
        413 => "BATMAP-SRR-X",
        414 => "9999999-in-1",
        415 => "0353",
        416 => "N-32 4-in-1",
        417 => "Fine Studio Happy New Years 1990",
        418 => "820106-C/821007C",
        419 => "Taikee TK-8007 MCU",
        420 => "A971210",
        421 => "SC871115C",
        422 => "TEC9719",
        423 => "Lexibook Compact Cyber Arcade",
        424 => "Lexibook Retro TV Game Console",
        425 => "Cube Tech VT369",
        426 => "VT369 with serial ROM",
        427 => "VT369 with inverter/EEPROM",
        428 => "BB-002A/TF2740",
        429 => "LIKO BBG-235-8-1B/Milowork FCFC1",
        430 => "831031C/T-308",
        431 => "Realtec GN-91B",
        432 => "Realtec 8023/8043/8086/8090/8286, GN-30C",
        433 => "Realtec NC-20MB",
        434 => "S-009",
        435 => "F-1002",
        436 => "ZLX-08",
        437 => "TH2348",
        438 => "K-3071/K-3014",
        439 => "YS2309",
        440 => "Sonic REC-9388",
        441 => "850335C",
        442 => "Golden Key",
        443 => "NC3000M",
        444 => "NC7000M",
        445 => "DG574B",
        446 => "SMD172B_FPGA",
        447 => "KL-06/GC007/KL-07",
        448 => "830768C",
        449 => "22-in-1 King Series",
        450 => "YY841157C",
        451 => "Impact Soft C-IM2-BASE",
        452 => "DS-9-27",
        453 => "Realtec 8042",
        454 => "ET-89",
        455 => "N625836",
        456 => "Realtec K6C3001A",
        457 => "810431C",
        458 => "GN-23",
        459 => "8-in-1",
        460 => "FC-29-40",
        461 => "CM-9309",
        462 => "BMC-971107-00G",
        463 => "YH810X1",
        464 => "NTDEC 9012",
        465 => "ET-120",
        466 => "Keybyte Computer",
        467 => "47-2",
        468 => "BlazePro FPGA",
        469 => "BlazePro FDS",
        470 => "INX_007T_V01",
        471 => "Impact Soft IM1",
        472 => "830947",
        473 => "KJ01A-18",
        474 => "NTDEC N625231",
        475 => "820215-C-A2",
        476 => "Croaky Karaoke",
        477 => "15-in-1",
        478 => "WE7HGX",
        479 => "CoolX Lite",
        480 => "480",
        481 => "045N",
        482 => "K-1079",
        483 => "3927",
        484 => "MMC3 2x256 KiB",
        485 => "0359",
        486 => "Kaiser KS-7009",
        487 => "AVE NINA-08",
        488 => "HC001",
        489 => "N-80",
        490 => "K-3101",
        491 => "Sane Ting 5-in-1",
        492 => "K-3069/12-28",
        493 => "AVE-NTDEC 30-in-1",
        494 => "CH512K/OK-103",
        495 => "N-46",
        496 => "VT369 with inverteradder",
        497 => "Subor LOGO",
        498 => "K-3011",
        499 => "FC-41",
        500 => "Yhc UNROM",
        501 => "Yhc AXROM",
        502 => "Yhc-A/B/UXROM",
        503 => "ET-170",
        504 => "K-3054",
        505 => "5426757A-Y2-230630",
        506 => "GA-009",
        507 => "A018",
        508 => "JY-014",
        509 => "K-3022",
        510 => "FC-53A",
        511 => "1n4148",
        513 => "SA-9602B",
        514 => "OK",
        515 => "Family Noraebang",
        516 => "COCOMA",
        517 => "Kkachi-wa Nolae Chingu",
        518 => "SB97",
        519 => "840348C/43-163/EH8813A",
        520 => "+Datach Dragon Ball Z multicart",
        521 => "Dreamtech 01",
        522 => "Whirlwind Manu LH10",
        524 => "900218",
        525 => "KS-7021A",
        526 => "BJ-56",
        527 => "AX-40G",
        528 => "831128C",
        529 => "YY0807/J-2148/T-230",
        530 => "AX5705",
        531 => "LittleCom PC-95",
        532 => "CHINA_ER_SAN2",
        533 => "3014",
        534 => "ING003C",
        535 => "Whirlwind Manu LH53",
        536 => "N42S-2",
        537 => "JY4M4",
        538 => "60-1064-16L",
        539 => "Parthena (bootleg)",
        540 => "82112C",
        541 => "LittleCom 160",
        542 => "JYV610 830626C",
        543 => "5-in-1",
        544 => "FS306",
        545 => "ST-80",
        546 => "03-101",
        547 => "Konami Q",
        548 => "CTC-15",
        549 => "Kaiser KS-7016B",
        550 => "JY820845C",
        551 => "KT-xxx",
        552 => "Taito P3-044",
        553 => "3013",
        554 => "Kaiser KS-7010",
        555 => "NES-EVENT2",
        556 => "C88DIP",
        557 => "NTDEC 2718",
        558 => "FSxxx",
        559 => "Subor Sango II",
        560 => "C/E BASIC",
        561 => "Bung Super Game Doctor",
        562 => "Venus Turbo Game Doctor",
        563 => "J-2020",
        564 => "bd23.pcb",
        565 => "J-33-C",
        566 => "ET-149",
        567 => "Top Ten Variety (Super Fighter III)",
        568 => "T-227",
        569 => "820315-C",
        570 => "9052",
        571 => "JC-011",
        572 => "F-648",
        573 => "5068",
        574 => "FC-40",
        575 => "W-03",
        576 => "J-2096",
        577 => "KN-20",
        578 => "910610",
        579 => "T-215",
        580 => "ET-156",
        581 => "ET-82",
        582 => "A9778",
        583 => "8203",
        584 => "ST-32",
        585 => "FE-01-1",
        586 => "HN-02/ET-147",
        587 => "3355",
        588 => "ET-81",
        589 => "810706",
        590 => "810430",
        591 => "07027/810543",
        592 => "J-2054/J-2035",
        593 => "PMMC3",
        594 => "Rinco FSG2",
        595 => "4MROM-512",
        596 => "FC-49",
        597 => "GN-27",
        598 => "K-3021, 3936",
        599 => "ET-133A",
        600 => "ABL UM6578",
        601 => "Jungletac UM6578",
        602 => "Subor SB-2000",
        603 => "J-2061",
        604 => "Dancing Expert",
        605 => "New Star TX5/8IN1",
        606 => "New Star T4IN1",
        607 => "4705",
        608 => "A-23",
        609 => "63-100",
        610 => "J-2042",
        611 => "T-124/43-117/831049",
        612 => "K-3004",
        613 => "S5668 3366",
        614 => "New Star 9135",
        615 => "LB12in1",
        616 => "K-3044",
        617 => "3-in-1",
        618 => "FC 4-in-1 (NS32)",
        619 => "68-in-1",
        620 => "4782/820226",
        621 => "Predator bootleg",
        622 => "3945",
        623 => "J-2083",
        624 => "KL-08/KL-09B",
        625 => "ET-20",
        626 => "90-in-1",
        627 => "821134",
        682 => "Rainbow Mapper",
        761 => "Bung MFC",
        _ => "Unknown Mapper",
    }
}

pub fn md5_hex(data: &[u8]) -> String {
    let s: [u32; 64] = [
        7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,
        5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,
        4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,
        6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,
    ];

    let k: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee,
        0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
        0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
        0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
        0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa,
        0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
        0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
        0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
        0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
        0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
        0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
        0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039,
        0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
        0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
    ];

    let mut a0: u32 = 0x67452301;
    let mut b0: u32 = 0xefcdab89;
    let mut c0: u32 = 0x98badcfe;
    let mut d0: u32 = 0x10325476;

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = data.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_le_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut m = [0u32; 16];
        for i in 0..16 {
            m[i] = u32::from_le_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }

        let mut a = a0;
        let mut b = b0;
        let mut c = c0;
        let mut d = d0;

        for i in 0..64 {
            let (f, g) = if i < 16 {
                ((b & c) | ((!b) & d), i)
            } else if i < 32 {
                ((d & b) | ((!d) & c), (5 * i + 1) % 16)
            } else if i < 48 {
                (b ^ c ^ d, (3 * i + 5) % 16)
            } else {
                (c ^ (b | (!d)), (7 * i) % 16)
            };

            let d_temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(a.wrapping_add(f).wrapping_add(k[i]).wrapping_add(m[g]).rotate_left(s[i]));
            a = d_temp;
        }

        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    let mut result = [0u8; 16];
    result[0..4].copy_from_slice(&a0.to_le_bytes());
    result[4..8].copy_from_slice(&b0.to_le_bytes());
    result[8..12].copy_from_slice(&c0.to_le_bytes());
    result[12..16].copy_from_slice(&d0.to_le_bytes());

    let mut s_out = String::with_capacity(32);
    for b in result {
        s_out.push_str(&format!("{:02x}", b));
    }
    s_out
}

pub(crate) struct StatusLayout {
    pub win_x: usize,
    pub win_y: usize,
    pub win_w: usize,
    pub win_h: usize,
    pub close_x: usize,
    pub close_y: usize,
    pub close_w: usize,
    pub close_h: usize,
    pub content_x: usize,
    pub content_y: usize,
    pub content_w: usize,
    pub content_h: usize,
    pub line_h: usize,
    pub vis_lines: usize,
    pub copy_btn: (usize, usize, usize, usize),
    pub clear_btn: (usize, usize, usize, usize),
    pub close_btn: (usize, usize, usize, usize),
    pub scroll_up_btn: (usize, usize, usize, usize),
    pub scroll_down_btn: (usize, usize, usize, usize),
    pub scrollbar_x: usize,
    pub scrollbar_y: usize,
    pub scrollbar_w: usize,
    pub scrollbar_h: usize,
}

pub(crate) fn compute_status_layout(width: usize, height: usize, scale: f32) -> StatusLayout {
    let sc = scale;
    let pad_x = (10.0 * sc).round() as usize;
    let title_h = (24.0 * sc).round() as usize;
    let close_w = (20.0 * sc).round() as usize;
    let close_h = (20.0 * sc).round() as usize;
    let btn_h = (22.0 * sc).round() as usize;
    let btn_gap = (6.0 * sc).round() as usize;
    let line_h = (12.0 * sc).round().max(10.0) as usize;

    let target_w = (560.0 * sc).round() as usize;
    let target_h = (440.0 * sc).round() as usize;

    let win_w = target_w.min(width.saturating_sub(16)).max(340);
    let win_h = target_h.min(height.saturating_sub(24)).max(260);

    let win_x = (width.saturating_sub(win_w)) / 2;
    let win_y = (height.saturating_sub(win_h)) / 2;

    let close_x = win_x + win_w - close_w - (6.0 * sc).round() as usize;
    let close_y = win_y + (2.0 * sc).round() as usize;

    let bottom_bar_h = btn_h + (10.0 * sc).round() as usize;
    let content_x = win_x + pad_x;
    let content_y = win_y + title_h + (4.0 * sc).round() as usize;
    let content_w = win_w.saturating_sub(pad_x * 2);
    let content_h = win_h.saturating_sub(title_h + bottom_bar_h + (10.0 * sc).round() as usize);
    let vis_lines = if line_h > 0 { content_h / line_h } else { 1 };

    let copy_w = (140.0 * sc).round() as usize;
    let clear_w = (96.0 * sc).round() as usize;
    let close_btn_w = (76.0 * sc).round() as usize;
    let arrow_w = (26.0 * sc).round() as usize;

    let by = win_y + win_h - btn_h - (6.0 * sc).round() as usize;
    let copy_btn = (content_x, by, copy_w, btn_h);
    let clear_btn = (content_x + copy_w + btn_gap, by, clear_w, btn_h);

    let right_end = content_x + content_w;
    let close_btn = (right_end.saturating_sub(close_btn_w), by, close_btn_w, btn_h);
    let scroll_down_btn = (close_btn.0.saturating_sub(arrow_w + btn_gap), by, arrow_w, btn_h);
    let scroll_up_btn = (scroll_down_btn.0.saturating_sub(arrow_w + (4.0 * sc).round() as usize), by, arrow_w, btn_h);

    let scrollbar_w = (8.0 * sc).round() as usize;
    let scrollbar_x = content_x + content_w.saturating_sub(scrollbar_w);
    let scrollbar_y = content_y;
    let scrollbar_h = content_h;

    StatusLayout {
        win_x,
        win_y,
        win_w,
        win_h,
        close_x,
        close_y,
        close_w,
        close_h,
        content_x,
        content_y,
        content_w,
        content_h,
        line_h,
        vis_lines,
        copy_btn,
        clear_btn,
        close_btn,
        scroll_up_btn,
        scroll_down_btn,
        scrollbar_x,
        scrollbar_y,
        scrollbar_w,
        scrollbar_h,
    }
}

pub(crate) enum StatusLineKind {
    Header,
    LabelValue,
    Info,
    LogMsg,
    Empty,
}

pub(crate) struct FormattedStatusLine {
    pub text: String,
    pub kind: StatusLineKind,
}

pub(crate) fn build_status_lines(emu: &Emulator, ms: &MenuState) -> Vec<FormattedStatusLine> {
    let mut lines = Vec::new();

    lines.push(FormattedStatusLine {
        text: "--- ROM INFORMATION ---".to_string(),
        kind: StatusLineKind::Header,
    });

    if let Some(cart) = emu.cart.as_ref() {
        let file_title = std::path::Path::new(&cart.name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&cart.name);
        lines.push(FormattedStatusLine {
            text: format!("File:         {}", file_title),
            kind: StatusLineKind::LabelValue,
        });

        let rom_format = emu.debug_get_rom_format();
        lines.push(FormattedStatusLine {
            text: format!("Format:       {}", rom_format),
            kind: StatusLineKind::LabelValue,
        });

        let mapper_name = get_mapper_name(cart.memory_mapper);
        lines.push(FormattedStatusLine {
            text: format!("Mapper:       {} ({})", cart.memory_mapper, mapper_name),
            kind: StatusLineKind::LabelValue,
        });

        if cart.sub_mapper > 0 || cart.is_nes20 {
            lines.push(FormattedStatusLine {
                text: format!("Submapper:    {}", cart.sub_mapper),
                kind: StatusLineKind::LabelValue,
            });
        }

        let prg_kb = cart.prg_rom.len() / 1024;
        let prg_banks = cart.prg_rom.len() / 16384;
        lines.push(FormattedStatusLine {
            text: format!("PRG-ROM:      {} KB ({} x 16 KB)", prg_kb, prg_banks),
            kind: StatusLineKind::LabelValue,
        });

        if cart.using_chr_ram || cart.chr_rom.is_empty() {
            let chr_ram_kb = cart.chr_ram.len() / 1024;
            lines.push(FormattedStatusLine {
                text: format!("CHR:          CHR-RAM {} KB", chr_ram_kb),
                kind: StatusLineKind::LabelValue,
            });
        } else {
            let chr_kb = cart.chr_rom.len() / 1024;
            let chr_banks = cart.chr_rom.len() / 8192;
            lines.push(FormattedStatusLine {
                text: format!("CHR-ROM:      {} KB ({} x 8 KB)", chr_kb, chr_banks),
                kind: StatusLineKind::LabelValue,
            });
        }

        let prg_ram_kb = cart.prg_ram.len() / 1024;
        lines.push(FormattedStatusLine {
            text: format!("PRG-RAM:      {} KB", prg_ram_kb),
            kind: StatusLineKind::LabelValue,
        });

        let mirroring = emu.debug_get_mirroring_desc();
        lines.push(FormattedStatusLine {
            text: format!("Mirroring:    {}", mirroring),
            kind: StatusLineKind::LabelValue,
        });

        lines.push(FormattedStatusLine {
            text: format!("Battery:      {}", if cart.has_battery { "Yes" } else { "No" }),
            kind: StatusLineKind::LabelValue,
        });

        lines.push(FormattedStatusLine {
            text: format!("Trainer:      {}", if !cart.trainer.is_empty() { "Present (512 bytes)" } else { "None" }),
            kind: StatusLineKind::LabelValue,
        });

        let tv_desc = match cart.tv_system {
            crate::region::TvSystem::Ntsc => "NTSC",
            crate::region::TvSystem::Pal => "PAL",
            crate::region::TvSystem::Dual => "Dual (NTSC/PAL)",
            crate::region::TvSystem::Dendy => "Dendy",
            crate::region::TvSystem::Unknown => "Unknown",
        };
        lines.push(FormattedStatusLine {
            text: format!("TV System:    {}", tv_desc),
            kind: StatusLineKind::LabelValue,
        });

        lines.push(FormattedStatusLine {
            text: format!("PRG CRC32:    0x{:08X}", cart.prg_rom_crc32),
            kind: StatusLineKind::LabelValue,
        });

        if !cart.chr_rom.is_empty() {
            lines.push(FormattedStatusLine {
                text: format!("CHR CRC32:    0x{:08X}", cart.chr_rom_crc32),
                kind: StatusLineKind::LabelValue,
            });
        }

        lines.push(FormattedStatusLine {
            text: format!("PRG+CHR CRC:  0x{:08X}", cart.prg_chr_crc32),
            kind: StatusLineKind::LabelValue,
        });

        lines.push(FormattedStatusLine {
            text: format!("Overall CRC:  0x{:08X}", cart.overall_crc32),
            kind: StatusLineKind::LabelValue,
        });

        if !cart.prg_rom.is_empty() {
            let prg_md5 = md5_hex(&cart.prg_rom);
            lines.push(FormattedStatusLine {
                text: format!("PRG MD5:      {}", prg_md5),
                kind: StatusLineKind::LabelValue,
            });
        }
    } else {
        lines.push(FormattedStatusLine {
            text: "No cartridge loaded.".to_string(),
            kind: StatusLineKind::Info,
        });
    }

    lines.push(FormattedStatusLine {
        text: String::new(),
        kind: StatusLineKind::Empty,
    });

    lines.push(FormattedStatusLine {
        text: "--- EMULATION STATUS ---".to_string(),
        kind: StatusLineKind::Header,
    });

    let region_hz = emu.debug_get_region_hz();
    lines.push(FormattedStatusLine {
        text: format!("Region:       {}", region_hz),
        kind: StatusLineKind::LabelValue,
    });

    lines.push(FormattedStatusLine {
        text: format!("Total Cycles: {}", emu.total_cycles),
        kind: StatusLineKind::LabelValue,
    });

    lines.push(FormattedStatusLine {
        text: format!("Lag Frames:   {}", emu.lag_frames),
        kind: StatusLineKind::LabelValue,
    });

    let c1_str = format!("{:?}", emu.controller1_type);
    let c2_str = format!("{:?}", emu.controller2_type);
    lines.push(FormattedStatusLine {
        text: format!("Port 1:       {}", c1_str),
        kind: StatusLineKind::LabelValue,
    });
    lines.push(FormattedStatusLine {
        text: format!("Port 2:       {}", c2_str),
        kind: StatusLineKind::LabelValue,
    });

    lines.push(FormattedStatusLine {
        text: String::new(),
        kind: StatusLineKind::Empty,
    });

    lines.push(FormattedStatusLine {
        text: "--- MESSAGE LOG ---".to_string(),
        kind: StatusLineKind::Header,
    });

    if ms.status_log_messages.is_empty() {
        lines.push(FormattedStatusLine {
            text: "(No session messages logged yet)".to_string(),
            kind: StatusLineKind::Info,
        });
    } else {
        for msg in &ms.status_log_messages {
            lines.push(FormattedStatusLine {
                text: msg.clone(),
                kind: StatusLineKind::LogMsg,
            });
        }
    }

    lines
}

pub(crate) fn render_status_window(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    ms: &MenuState,
    colors: &UiColors,
    emu: &mut Emulator,
    scale: f32,
) {
    let l = compute_status_layout(width, height, scale);
    let sc = scale;

    draw_rect(buffer, l.win_x, l.win_y, l.win_w, l.win_h, width, colors.window_bg);
    draw_rect(buffer, l.win_x, l.win_y, l.win_w, 1, width, colors.box_border);
    draw_rect(buffer, l.win_x, l.win_y + l.win_h.saturating_sub(1), l.win_w, 1, width, colors.box_border);
    draw_rect(buffer, l.win_x, l.win_y, 1, l.win_h, width, colors.box_border);
    draw_rect(buffer, l.win_x + l.win_w.saturating_sub(1), l.win_y, 1, l.win_h, width, colors.box_border);

    let title_h = (24.0 * sc).round() as usize;
    draw_rect(buffer, l.win_x, l.win_y, l.win_w, title_h, width, colors.dropdown_bg);
    draw_rect(buffer, l.win_x, l.win_y + title_h.saturating_sub(1), l.win_w, 1, width, colors.box_border);

    let font_h = (8.0 * sc).round() as usize;
    let title_ty = l.win_y + (title_h.saturating_sub(font_h)) / 2;
    draw_text(buffer, l.win_x + (10.0 * sc).round() as usize, title_ty, width, "Status", colors.menu_text, scale);

    let (mx, my) = ms.mouse_pos;
    let close_hovered = point_in_rect(mx, my, l.close_x, l.close_y, l.close_w, l.close_h);
    let close_bg = if close_hovered { colors.box_bg_hover } else { colors.box_bg_default };
    draw_rect(buffer, l.close_x, l.close_y, l.close_w, l.close_h, width, close_bg);
    draw_rect(buffer, l.close_x, l.close_y, l.close_w, 1, width, colors.box_border);
    draw_rect(buffer, l.close_x, l.close_y + l.close_h.saturating_sub(1), l.close_w, 1, width, colors.box_border);
    draw_rect(buffer, l.close_x, l.close_y, 1, l.close_h, width, colors.box_border);
    draw_rect(buffer, l.close_x + l.close_w.saturating_sub(1), l.close_y, 1, l.close_h, width, colors.box_border);
    let cx = l.close_x + (l.close_w.saturating_sub((8.0 * sc).round() as usize)) / 2;
    let cy = l.close_y + (l.close_h.saturating_sub(font_h)) / 2;
    draw_text(buffer, cx, cy, width, "X", colors.menu_text, scale);

    draw_rect(buffer, l.content_x, l.content_y, l.content_w, l.content_h, width, 0xFF121212);
    draw_rect(buffer, l.content_x, l.content_y, l.content_w, 1, width, colors.box_border);
    draw_rect(buffer, l.content_x, l.content_y + l.content_h.saturating_sub(1), l.content_w, 1, width, colors.box_border);
    draw_rect(buffer, l.content_x, l.content_y, 1, l.content_h, width, colors.box_border);
    draw_rect(buffer, l.content_x + l.content_w.saturating_sub(1), l.content_y, 1, l.content_h, width, colors.box_border);

    let lines = build_status_lines(emu, ms);
    let max_scroll = lines.len().saturating_sub(l.vis_lines);
    let scroll = ms.status_scroll.min(max_scroll);

    let text_area_w = l.content_w.saturating_sub(l.scrollbar_w + (6.0 * sc).round() as usize);
    let text_pad_x = l.content_x + (8.0 * sc).round() as usize;

    for (row_idx, line) in lines.iter().skip(scroll).take(l.vis_lines).enumerate() {
        let ly = l.content_y + (4.0 * sc).round() as usize + row_idx * l.line_h;
        if ly + font_h > l.content_y + l.content_h {
            break;
        }

        let (text_color, bg_color) = match line.kind {
            StatusLineKind::Header => (colors.menu_highlight, Some(0xFF1E2632)),
            StatusLineKind::LabelValue => (colors.menu_text, None),
            StatusLineKind::Info => (colors.disabled_text, None),
            StatusLineKind::LogMsg => (0xFFD0D0D0, None),
            StatusLineKind::Empty => (colors.menu_text, None),
        };

        if let Some(bg) = bg_color {
            let bh = l.line_h.saturating_sub(1);
            draw_rect(buffer, l.content_x + 1, ly, text_area_w, bh, width, bg);
        }

        let max_chars = if (8.0 * sc) > 0.0 {
            (text_area_w as f32 / (8.0 * sc)) as usize
        } else {
            line.text.len()
        };

        let display_text = if line.text.len() > max_chars && max_chars > 3 {
            format!("{}...", &line.text[..max_chars.saturating_sub(3)])
        } else {
            line.text.clone()
        };

        draw_text(buffer, text_pad_x, ly, width, &display_text, text_color, scale);
    }

    if lines.len() > l.vis_lines && l.scrollbar_h > 10 {
        draw_rect(buffer, l.scrollbar_x, l.scrollbar_y, l.scrollbar_w, l.scrollbar_h, width, 0xFF202020);
        let thumb_ratio = (l.vis_lines as f32 / lines.len() as f32).clamp(0.1, 1.0);
        let thumb_h = ((l.scrollbar_h as f32 * thumb_ratio).round() as usize).max(12);
        let avail_track = l.scrollbar_h.saturating_sub(thumb_h);
        let thumb_y = if max_scroll > 0 {
            l.scrollbar_y + (scroll * avail_track) / max_scroll
        } else {
            l.scrollbar_y
        };
        draw_rect(buffer, l.scrollbar_x, thumb_y, l.scrollbar_w, thumb_h, width, colors.box_border);
    }

    let is_copy_hover = point_in_rect(mx, my, l.copy_btn.0, l.copy_btn.1, l.copy_btn.2, l.copy_btn.3);
    let copy_bg = if is_copy_hover { colors.box_bg_hover } else { colors.box_bg_default };
    draw_button(buffer, l.copy_btn.0, l.copy_btn.1, l.copy_btn.2, l.copy_btn.3, width, "Copy to Clipboard", copy_bg, colors.box_border, colors.menu_text, scale);

    let is_clear_hover = point_in_rect(mx, my, l.clear_btn.0, l.clear_btn.1, l.clear_btn.2, l.clear_btn.3);
    let clear_bg = if is_clear_hover { colors.box_bg_hover } else { colors.box_bg_default };
    draw_button(buffer, l.clear_btn.0, l.clear_btn.1, l.clear_btn.2, l.clear_btn.3, width, "Clear Log", clear_bg, colors.box_border, colors.menu_text, scale);

    let is_up_hover = point_in_rect(mx, my, l.scroll_up_btn.0, l.scroll_up_btn.1, l.scroll_up_btn.2, l.scroll_up_btn.3);
    let up_bg = if is_up_hover { colors.box_bg_hover } else { colors.box_bg_default };
    draw_button(buffer, l.scroll_up_btn.0, l.scroll_up_btn.1, l.scroll_up_btn.2, l.scroll_up_btn.3, width, "^", up_bg, colors.box_border, colors.menu_text, scale);

    let is_down_hover = point_in_rect(mx, my, l.scroll_down_btn.0, l.scroll_down_btn.1, l.scroll_down_btn.2, l.scroll_down_btn.3);
    let down_bg = if is_down_hover { colors.box_bg_hover } else { colors.box_bg_default };
    draw_button(buffer, l.scroll_down_btn.0, l.scroll_down_btn.1, l.scroll_down_btn.2, l.scroll_down_btn.3, width, "v", down_bg, colors.box_border, colors.menu_text, scale);

    let is_close_hover = point_in_rect(mx, my, l.close_btn.0, l.close_btn.1, l.close_btn.2, l.close_btn.3);
    let close_btn_bg = if is_close_hover { colors.box_bg_hover } else { colors.box_bg_default };
    draw_button(buffer, l.close_btn.0, l.close_btn.1, l.close_btn.2, l.close_btn.3, width, "Close", close_btn_bg, colors.box_border, colors.menu_text, scale);

    if let Some((instant, ref msg)) = ms.status_toast {
        if instant.elapsed() < std::time::Duration::from_millis(2500) {
            let toast_w = (msg.len() as f32 * 8.0 * sc).round() as usize + (20.0 * sc).round() as usize;
            let toast_h = (22.0 * sc).round() as usize;
            let toast_x = l.win_x + (l.win_w.saturating_sub(toast_w)) / 2;
            let toast_y = l.win_y + l.win_h.saturating_sub(toast_h + (35.0 * sc).round() as usize);
            draw_rect(buffer, toast_x, toast_y, toast_w, toast_h, width, 0xEE1E4028);
            draw_rect(buffer, toast_x, toast_y, toast_w, 1, width, 0xFF40A060);
            draw_rect(buffer, toast_x, toast_y + toast_h.saturating_sub(1), toast_w, 1, width, 0xFF40A060);
            draw_rect(buffer, toast_x, toast_y, 1, toast_h, width, 0xFF40A060);
            draw_rect(buffer, toast_x + toast_w.saturating_sub(1), toast_y, 1, toast_h, width, 0xFF40A060);
            let tx = toast_x + (10.0 * sc).round() as usize;
            let ty = toast_y + (toast_h.saturating_sub(font_h)) / 2;
            draw_text(buffer, tx, ty, width, msg, 0xFFFFFFFF, scale);
        }
    }
}

fn draw_button(
    buffer: &mut [u32],
    bx: usize,
    by: usize,
    bw: usize,
    bh: usize,
    width: usize,
    text: &str,
    bg: u32,
    border: u32,
    fg: u32,
    scale: f32,
) {
    let sc = scale;
    draw_rect(buffer, bx, by, bw, bh, width, bg);
    draw_rect(buffer, bx, by, bw, 1, width, border);
    draw_rect(buffer, bx, by + bh.saturating_sub(1), bw, 1, width, border);
    draw_rect(buffer, bx, by, 1, bh, width, border);
    draw_rect(buffer, bx + bw.saturating_sub(1), by, 1, bh, width, border);

    let text_w = (text.len() as f32 * 8.0 * sc).round() as usize;
    let font_h = (8.0 * sc).round() as usize;
    let tx = bx + (bw.saturating_sub(text_w)) / 2;
    let ty = by + (bh.saturating_sub(font_h)) / 2;
    draw_text(buffer, tx, ty, width, text, fg, scale);
}

pub(crate) fn handle_status_viewer_click(
    ms: &mut MenuState,
    emu: &Emulator,
    button: MouseButton,
    mx: usize,
    my: usize,
    width: usize,
    height: usize,
    scale: f32,
) {
    if button != MouseButton::Left {
        return;
    }

    let l = compute_status_layout(width, height, scale);

    if point_in_rect(mx, my, l.close_x, l.close_y, l.close_w, l.close_h)
        || point_in_rect(mx, my, l.close_btn.0, l.close_btn.1, l.close_btn.2, l.close_btn.3)
    {
        ms.show_status_window = false;
        return;
    }

    if point_in_rect(mx, my, l.copy_btn.0, l.copy_btn.1, l.copy_btn.2, l.copy_btn.3) {
        let lines = build_status_lines(emu, ms);
        let mut full_text = String::new();
        for line in lines {
            full_text.push_str(&line.text);
            full_text.push_str("\r\n");
        }
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(full_text);
            ms.status_toast = Some((std::time::Instant::now(), "Copied to clipboard!".to_string()));
        }
        return;
    }

    if point_in_rect(mx, my, l.clear_btn.0, l.clear_btn.1, l.clear_btn.2, l.clear_btn.3) {
        ms.status_log_messages.clear();
        ms.status_scroll = 0;
        return;
    }

    if point_in_rect(mx, my, l.scroll_up_btn.0, l.scroll_up_btn.1, l.scroll_up_btn.2, l.scroll_up_btn.3) {
        ms.status_scroll = ms.status_scroll.saturating_sub(3);
        return;
    }

    if point_in_rect(mx, my, l.scroll_down_btn.0, l.scroll_down_btn.1, l.scroll_down_btn.2, l.scroll_down_btn.3) {
        let lines = build_status_lines(emu, ms);
        let max_scroll = lines.len().saturating_sub(l.vis_lines);
        ms.status_scroll = (ms.status_scroll + 3).min(max_scroll);
        return;
    }
}

pub(crate) fn handle_status_viewer_key(ms: &mut MenuState, emu: &Emulator, keycode: VirtualKeyCode) {
    match keycode {
        VirtualKeyCode::Escape => {
            ms.show_status_window = false;
        }
        VirtualKeyCode::Up => {
            ms.status_scroll = ms.status_scroll.saturating_sub(1);
        }
        VirtualKeyCode::Down => {
            let lines = build_status_lines(emu, ms);
            let max_scroll = lines.len().saturating_sub(1);
            ms.status_scroll = (ms.status_scroll + 1).min(max_scroll);
        }
        VirtualKeyCode::PageUp => {
            ms.status_scroll = ms.status_scroll.saturating_sub(10);
        }
        VirtualKeyCode::PageDown => {
            let lines = build_status_lines(emu, ms);
            let max_scroll = lines.len().saturating_sub(1);
            ms.status_scroll = (ms.status_scroll + 10).min(max_scroll);
        }
        VirtualKeyCode::Home => {
            ms.status_scroll = 0;
        }
        VirtualKeyCode::End => {
            let lines = build_status_lines(emu, ms);
            ms.status_scroll = lines.len().saturating_sub(1);
        }
        VirtualKeyCode::C => {
            let lines = build_status_lines(emu, ms);
            let mut full_text = String::new();
            for line in lines {
                full_text.push_str(&line.text);
                full_text.push_str("\r\n");
            }
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_text(full_text);
                ms.status_toast = Some((std::time::Instant::now(), "Copied to clipboard!".to_string()));
            }
        }
        _ => {}
    }
}

pub(crate) fn handle_status_scroll(ms: &mut MenuState, amount: i32) {
    let cur = ms.status_scroll as i32 - amount;
    ms.status_scroll = cur.max(0) as usize;
}

pub fn log_status_message(ms: &mut MenuState, msg: String) {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = since_epoch.as_secs();
    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    let s = secs % 60;
    let entry = format!("[{:02}:{:02}:{:02}] {}", hours, mins, s, msg);
    ms.status_log_messages.push(entry);
    if ms.status_log_messages.len() > 1000 {
        ms.status_log_messages.remove(0);
    }
}
