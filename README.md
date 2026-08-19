# <img src="accunesicon.ico" width="48" height="48" alt=""> AccuNES

a cycle-accurate NES/Famicom emulator for windows, written in rust and focused on hardware-accurate behavior!

## Features

- **cycle accurate 6502 cpu** (all legal and illegal opcodes , addressing modes, interrupts, dma and open bus edge cases are handled!!!)
- **scanline accurate ppu at half cycle level accuracy**!!! (sprite/bg rendering, accurate vblank and nmi timing, sprite eval, open bus and oam dma edge cases all handled!!!)
- **accurate apu at cycle at half cycle level accuracy**!!! (all channels implemented, irqs, dmc, controller strobing, clocking and dmc dma edge cases are handled!!!)
- passes **ALL ACCURACYCOIN tests!** (141/141 as of today!!) passes **all blargg tests** too!!
- also as of today, a whopping **~512 mappers** are supported!!! (this was and still is very challenging to implement, some mappers may not be fully working yet but i think i'm making good progress on matching something like nintendulator's insane mapper counts!!!)
- **saving/loading** save states, **per game slots** system + **quick save/quick load** systems!
- **battery backed ram** saving for games that need it!
- **nes gamepads, zappers, power pads, snes gamepads, snes mouse, subor mouse and four score** controller types supported!
- supports **windows x64, x32 and ARM** devices!


## Building

there are two ways as of release v1.0.8:
**reccomended method** using the custom package builder:

```sh
./package-release.ps1 
```
currently supported args:
-Profile (profile)

currently supported profiles:
"release" (default): win64 release version!
"debug": win64 debug version!
"x32": win32 release version!
"x32debug": win32 debug version!
"arm64": windows arm release version!
"arm64debug": windows arm debug version!

** manual method ** using rust cargo builder:

```sh
cargo build --release
```
this outputs the win64 release version!

you can also use:

```sh
cargo build
```

for the win64 debug version!

or:

```sh
cargo build --release --target i686-pc-windows-msvc
```

for the win32 release version!

or:
```sh
cargo build --target i686-pc-windows-msvc
```

for the win32 debug version!

or:
```sh
cargo build --release --target aarch64-pc-windows-msvc
```
for the windows arm release version!

or:
```sh
cargo build --target aarch64-pc-windows-msvc
```
for the windows arm debug version!

output folders/archives for both methods are usually inside the target folder.


## Usage

launch AccuNES and use the menu to open a valid nes rom file (`.nes`, `.fds`, `.unf`, etc.)!

## Future

possible future additions:

- improve onebus mapper accuracies!
- go back and implement some complex mappers i opted to skip for now!
- more mappers!
- famicom expansion port!
- cheats!
- tas record/playback!

if you have more suggestions feel free to send them through github!

## Credits
- [Oussema Ammar](https://github.com/ammaroussema): hello! this is me! i made the emulator :D
- [FCEUX](https://fceux.com): very helpful in understanding vs system, pal and dendy! also some obscure mappers and audio config!
- [Mesen](https://www.mesen.ca/): very helpful for obscure mappers and video config!
- [Nestopia](http://0ldsk00l.ca/nestopia/): very helpful for obscure mappers!
- [NintendulatorNRS](https://www.qmtpro.com/~nes/nintendulator/): very useful for ALOT of obscure mappers and controller configs!
- [TriCNES](https://github.com/100thCoin/TriCNES/tree/main): helped me discover a lot of odd hardware accurate behavior for all main components!!
- [NesDev](https://www.nesdev.org/): can't forget the classics! if you're ever making a nes emulator, there's nothing more perfect than this site!!!

