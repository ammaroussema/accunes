# <img src="accunesicon.ico" width="48" height="48" alt=""> AccuNES

a cycle-accurate NES/Famicom emulator for windows, written in rust and focused on hardware-accurate behavior!

HEAVILY based on pre-existing emulators like fceux, mesen, nestopia, nintendulatornrs and tricnes! make sure
to check them out too!

## Features

- **cycle-accurate CPU/PPU/APU** — emulates the NES at the ppu half cycle level!
- **extensive mapper support** — plane 0 mappers (mappers 0-255) should be fully supported, the other planes have some working mappers
    but currently wip!
- **hardware-accurate PPU** — sprite evaluation, oam corruption, pal/dendy timing, odd-frame skip, palette corruption edge cases!
- **save states** — quick save/load states, plus 8 save slots per game!
- **controller support** — keyboard bindings and gamepad input via gilrs!
- **audio** — cycle-accurate apu with dmc, sweep, length counter and frame counters!

## Building

requires [rust](https://rustup.rs/) 2021 edition!

```sh
cargo build --release
```

the executable is written to `target/release/accunes.exe`.

### Dependencies

- `winit` — windowing and events
- `softbuffer` — directx 11/12 surface rendering
- `cpal` — audio output
- `gilrs` — gamepad input
- `image` — icon loading
- `rfd` — file dialogs
- `font8x8` — debug overlay font

## Usage

launch AccuNES and use the menu to open a valid nes rom file (`.nes`, `.fds`, `.unf`, etc.)!

## Future
possible future additions:
- more mappers!
- famicom expansion port!
- cheats!
- tas record/playback!
- auto update check!
if you have more suggestions feel free to send them through github!

## Credits
- [Oussema Ammar](https://github.com/ammaroussema): hello! this is me! i made the emulator :D
- [FCEUX](https://fceux.com): very helpful in understanding vs system, pal and dendy! also some obscure mappers and audio config!
- [Mesen](https://www.mesen.ca/): very helpful for obscure mappers and video config!
- [Nestopia](http://0ldsk00l.ca/nestopia/): very helpful for obscure mappers!
- [NintendulatorNRS](https://www.qmtpro.com/~nes/nintendulator/): very useful for ALOT of obscure mappers and controller configs!
- [TriCNES](https://github.com/100thCoin/TriCNES/tree/main): helped me discover a lot of odd hardware accurate behavior for all main    
  components!!
- [NesDev](https://www.nesdev.org/): can't forget the classics! if you're ever making a nes emulator, there's nothing more perfect than
  this site!!!

