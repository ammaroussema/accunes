# <img src="accunesicon.ico" width="48" height="48" alt=""> AccuNES

a cycle-accurate NES/Famicom emulator for windows, written in rust and focused on hardware-accurate behavior!

## Features

- **cycle accurate 6502 cpu** (all legal and illegal opcodes , addressing modes, interrupts, dma and open bus edge cases are handled!!!)
- **scanline accurate ppu at half cycle level accuracy**!!! (sprite/bg rendering, accurate vblank and nmi timing, sprite eval, open bus and oam dma edge cases all handled!!!)
- **accurate apu at cycle at half cycle level accuracy**!!! (all channels implemented, irqs, dmc, controller strobing, clocking and dmc dma edge cases are handled!!!)
- passes **ALL ACCURACYCOIN tests!** (144/144 as of today!!) passes **all blargg tests** too!!
- also as of today, a whopping **~646 mappers** are supported!!! (let me know if i missed any/if any are slightly broken!!!)
- **saving/loading** save states, **per game slots** system + **quick save/quick load** systems!
- **battery backed ram** saving for games that need it!
- **nes gamepads, famicom gamepads, famicom microphone, zapper, paddle, power pads, snes gamepad, snes mouse, subor mouse, virtual boy gamepad and four score** controller types supported!
- **famicom zapper, paddle, family trainer, famicom 2/4-player adapters, hori 4-player adapter, quiz king buzzers, top rider, famicom network controller, city patrolman lightgun, pokkun moguraa mat, sharp c1 cassette interface, majesco golden nugget casino, abl pinball, tv pump, triface mahjong, mahjong gekitou densetsu controller, oeka kid tablet, konami hyper shot, family basic keyboard, pec586 keyboard, bit-79 keyboard, keda keyboard, kingwon keyboard, ze cheng keyboard, party tap and pachinko controller** expansion port types supported!
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

