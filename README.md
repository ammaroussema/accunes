# <img src="accunesicon.ico" width="48" height="48" alt=""> AccuNES

a hardware-accurate NES/Famicom/Famiclone emulator written in rust!

## Features

- **cycle-accurate 6502 cpu** — every legal and illegal opcode, all addressing modes, interrupts, dma and open-bus edge cases
- **scanline-accurate ppu** — sprite/background rendering, accurate vblank and nmi timing, sprite evaluation, oam dma edge cases
- **accurate apu** — all channels, irqs, dmc, controller strobing, clocking and dmc dma edge cases
- passes **all accuracycoin tests (144/144)** and **all blargg tests**!
- supports **~646 mappers**!
- accurate **vs system, pal and dendy** behavior!
- **nsf/nsfe** playback!
- **.nes, .fds, .unif, .nsf, .nsfe, .qd, .wxn (waixing roms) and study box tapes** support!
- **save states**, **per-game save slots**, **quick save / quick load**
- **battery-backed ram** saving for games that need it
- **rewind** — savestate rewinding!
- **cheats** a built-in cheat database! 
- **video filters** scanline, lcd grid, xbrz, hq, scale, sai, super sai, super eagle, prescale, blargg ntsc, bisqwit ntsc and pal video filters! 
- **achievements** integrated retroachievements support!
- **gamepads**: nes, famicom, snes, virtual boy, four score
- **light guns**: zapper, city patrolman, konami hyper shot
- **pointing**: paddle, snes/subor mice, ps/2-style mice (yuxing, macro winners, mega book) and more!
- **other**: power pads, famicom microphone, sudoku controller and more!
- **expansion ports**: 2/4-player adapters (famicom two/four player & hori), arkanoid paddle, famicom zapper, oeka kids tablet, family trainer, quiz king buzzers, party tap, pachinko, exciting boxing (punching bag), jissen/triface/gekitou mahjong, konami & bandai hyper shots, keyboards (family basic, pec586, bit-79, keda, kingwon, ze cheng, subor), barcode battler, turbo file, battle box, top rider, famicom network controller, city patrolman, pokkun moguraa mat, sharp c1 cassette interface, majesco golden nugget casino, abl pinball, tv pump and more!

### Platforms
- **windows** — x64, x86 and ARM
- **linux**
- **macOS**

## Building

### windows
**recommended** — use the custom package builder!!
```sh
./package-release.ps1
```
supported profiles: `release`/`debug` (host-native per platform) · `x32` · `x32debug` · `arm64` · `arm64debug` (windows cross) · `linux64` · `linuxarm64` · `macosx64` · `macosarm64`


**manual** with cargo:
```sh
cargo build --release        # win64 release
cargo build                  # win64 debug
cargo build --release --target i686-pc-windows-msvc      # win32
cargo build --release --target aarch64-pc-windows-msvc   # windows arm
```

### macOS
```sh
cargo build --release
```

### linux
```sh
cargo build --release
```


output folders/archives are usually inside the `target` folder!!

## Dependencies

**windows**: none!!!

**linux**:
- `libasound2-dev`
- `libudev-dev`
- `libxkbcommon-dev`
- `libwayland-dev` 
- `libssl-dev`
- `libgtk-3-dev`

```sh
sudo apt install libasound2-dev libudev-dev libxkbcommon-dev libwayland-dev libssl-dev libgtk-3-dev
```

**macOS**: none!!!

## Usage

launch AccuNES and use the menu to open a valid rom file (`.nes`, `.fds`, `.unf`, etc.)!
if you have feedback or ideas, feel free to send them through github!

## Credits
- [Oussema Ammar](https://github.com/ammaroussema): hello! this is me! i made the emulator :D
- [FCEUX](https://fceux.com): very helpful in understanding vs system, pal and dendy! also some obscure mappers, video filters, controllers and audio config!
- [Mesen](https://www.mesen.ca/): very helpful for obscure mappers, NSF/NSFe mapper, cheats, video filters, audio config, controllers and video config!
- [Nestopia](http://0ldsk00l.ca/nestopia/): very helpful for obscure mappers!
- [NintendulatorNRS](https://www.qmtpro.com/~nes/nintendulator/): very useful for ALOT of obscure mappers, controllers and controller configs!
- [TriCNES](https://github.com/100thCoin/TriCNES/tree/main): helped me discover a lot of odd hardware accurate behavior for all main components!!
- [zenju]: made the original xbrz video filter!
- [Maxim Stepen & Cameron Zemek]: made the original hq video filter!
- [RetroArch, Hans-Kristian Arntzen and Daniel de Matteis](https://www.retroarch.com/): made the original sai video filter!
- [Andrea Mazzoleni]: made the original scale video filter!
- [Bisqwit]: made the bisqwit NTSC video filter!
- [Blargg]: made the blargg NTSC video filter!
- [feos, HardwareMan and r57shell]: made the PAL video filter!
- [RetroAchievements](https://retroachievements.org/): made the achievements api for nes games!
- [NesDev](https://www.nesdev.org/): can't forget the classics! if you're ever making a nes emulator, there's nothing more perfect than this site!!!