# AccuNES Movie Format (`.acm` / `.acmb`)

This document specifies the .acm and .acmb file formats for the AccuNES emulator.
Movie files are stored in one of two encodings, chosen by extension:

| Extension | Encoding | Notes |
|-----------|----------|-------|
| `.acm`    | text     | default, more readable |
| `.acmb`   | binary   | compact, faster loading |

Both encodings are 100% interchangeable.

## Text format (`.acm`)

UTF-8 text. Lines are `\n`-terminated. Blank lines and lines whose first non-whitespace character
is `#` are ignored. Section/field keys are case-sensitive.

### Magic and headers

The first line must be the magic string `ACMN`. After it, an unordered set of single-line header
fields (`key value`, separated by whitespace; the value is the rest of the line, trimmed):

| Field | Example | Meaning |
|-------|---------|---------|
| `[format]` | `1` | Format version. Must be ≥ 1 and no greater than the build's supported version. |
| `[emulator]` | `AccuNES 1.7.5` | Emulator + version that created the movie. |
| `[romTitle]` | `Super Mario Bros.` | Internal cartridge name. |
| `[romHash]` | `crc32:2C9B0B02` | ROM identity hash (see below). |
| `[region]` | `NTSC` | `AUTO`, `NTSC`, `PAL` or `DENDY`. |
| `[rerecords]` | `3` | Number of re-records made while editing the movie. |
| `[author]` | `feos` | Author name. |
| `[comment]` | `RTA-ish TAS` | Free-form comment. |
| `[powerOn]` | `1` | `1` = power-cycle the console when the movie starts; `0` = don't. |
| `[guid]` | `4E347A83-149B-4D88-927F-E2090FDFE214` | 128-bit UUID identifying this movie session. |
| `[setting]` | `autofire=3` | Free-form `key=value` setting; repeatable. |

`[romHash]` uses the `crc32:XXXXXXXX` form (uppercase hex). The same hash is what the Import/Export
feature cross-checks against the current ROM before starting playback.

### `[devices]` section

Introduced by the line `[devices]`. One device per line, positional order matters (frame fields are
aligned to device order):

```
slot kind B1 B2 ...
```

`slot` and `kind` are non-empty whitespace-free tokens; each `Bn` is a single ASCII alphabetic
character (case-sensitive; duplicates rejected). Example:

```
[devices]
port1 NES_PAD R L D U S T B A
port2 NES_PAD R L D U S T B A
```

### `[frames]` section

Introduced by `[frames]`. Only frames with at least one device pressing at least one button are
listed; **frame numbers are implicit** (gaps = empty input). The line form is:

```
|FRAMEHEX|FIELD0|FIELD1|...
```

- `FRAMEHEX` is the frame number written in **hexadecimal** with no prefix (so `|10|` is frame 16).
- Exactly one field per device. A field is a string of length `#buttons` of that device: each
  position shows the button char when pressed, or `.` when not pressed.

Example (SMB, 2 gamepads: on-frame 5 press `A` on pad 1, frame 0x20 press `Right`, frame 0x40 press
`Right` and `A`):

```
[frames]
|5|.......A|........
|20|R.......|........
|40|R......A|........
```

### `[events]` section

Introduced by `[events]`. Each line is a frame tag followed by an event name:

```
F<FRAMEHEX> <NAME>
```

| Name | Meaning |
|------|---------|
| `SOFT_RESET` | Warm / software reset |
| `HARD_RESET` | Cold / hardware reset |
| `POWER` | Power cycle (off/on) |
| `FDS_INSERT` | FDS disk inserted |
| `FDS_EJECT` | FDS disk ejected |
| `FDS_SWAP` | FDS disk side swapped |
| `VS_COIN1` | VS System Coin 1 inserted |
| `VS_COIN2` | VS System Coin 2 inserted |
| `VS_SERVICE` | VS System Service button pressed |

Example:

```
[events]
F0 POWER
F40 SOFT_RESET
```

### `[chapters]` / `[subtitles]` (optional)

Written only when non-empty.

```
[chapters]
F<FRAMEHEX> <marker name>

[subtitles]
F<STARTHEX>-F<ENDHEX> <subtitle text>
```

### String escaping

In any free-text value (`[author]`, `[comment]`, `[setting]`, chapter names, subtitle text)
`\` becomes `\x5c` and newline becomes `\n`. Escaping is not applied to `[emulator]`/`[romTitle]`/
`[romHash]`/`[region]`/`[format]`/`[rerecords]`/`[powerOn]` in the text writer, so those values must
not contain newlines.

### Minimal example

```
ACMN
[format] 1
[emulator] AccuNES 1.7.5
[romTitle] Super Mario Bros.
[romHash] crc32:2C9B0B02
[region] NTSC
[rerecords] 2
[author] an author
[comment] hello, movie
[powerOn] 1
[devices]
port1 NES_PAD R L D U S T B A
[frames]
|0|.......A
|5|R.......
|a|U.......
[events]
F0 POWER
[chapters]
F0 start
F20 1-1 clear
[subtitles]
F0-F20 RUN!
```

---

## Binary format (`.acmb`)

Little-endian scalars. Strings are `u32 length` + raw UTF-8 bytes. Each array is prefixed by a
`u32` element count (devices, frames, events, chapters, subtitles; the settings map likewise).

### Layout

```
offset  type      field
------  ----      -----
0       [4]	magic "ACMB"
4       u32       format version (1)
8       u8        region index (0=AUTO, 1=NTSC, 2=PAL, 3=DENDY)
9       u32       rerecords
13      u8        powerOn (0 or 1)
14      str       emulator
        str       rom_title            (crc32:XXXXXXXX)
        str       rom_hash
        str       author
        str       comment
        u32       settings.len()
        repeat: str key, str value
        u32       devices.len()
        repeat:   str slot, str kind, u32 nbuttons, nbuttons x u8 (button chars)
        u32       frames.len()
        RLE frame stream (see below)
        u32       events.len()
        repeat:   u64 frame, u8 kind index
        u32       chapters.len()
        repeat:   u64 frame, str name
        u32       subtitles.len()
        repeat:   u64 start, u64 end, str text
```

The file must be consumed exactly: any trailing bytes after the last subtitle block are an error.

### Frame bit-packing and RLE

For a given device, its button state is packed LSB-first: button *i* occupies bit `i % 8` of byte
`i / 8`; each device therefore takes `(nbuttons + 7) / 8` bytes. Scripts applying deltas need the
device list to know widths, but the format itself interleaves deltas device by device.

The frame stream is a list of runs, each starting with a `u8` tag:

| Tag | Payload | Meaning |
|-----|---------|---------|
| `0x00` | `u32 count` | The next `count` frames are byte-identical to the previous frame (previous = all-off for the first frame). `count` must be ≥ 1 and must not overrun the declared frame count. |
| `0x01` | delta bytes | One new frame whose packed bit-state is `previous XOR delta`, where `delta` is `Σ ceil(nbuttons/8)` bytes in device order, and `previous` is the last emitted frame (all-off before the first). |

### Kind/chapter/subtitle arrays

Events: `u64 frame` then `u8 kind index`:

| Index | Event |
|-------|-------|
| 0 | SoftReset |
| 1 | HardReset |
| 2 | Power |
| 3 | FdsInsert |
| 4 | FdsEject |
| 5 | FdsSwapSide |
| 6 | VsInsertCoin1 |
| 7 | VsInsertCoin2 |
| 8 | VsService |

Chapters: `u64 frame`, then name string. Subtitles: `u64 start`, `u64 end`, then text string
(`end` must be ≥ `start`).

---

## Movie savestates

Every AccuNES savestate ends with a fixed 37-byte **movie trailer** (or legacy 21-byte trailer), ensuring that a savestate can only be loaded into a matching movie context while enabling seeking and re-recording:

| Offset | Size | Field |
|--------|------|-------|
| 0 | 4 | magic `"ACM!"` |
| 4 | 1 | active flag: 1 = a movie was active when the state was saved |
| 5 | 8 | `u64` movie cursor (input-frame index) |
| 13 | 4 | `u32` recorded frames count |
| 17 | 4 | `u32` recorded events count |
| 21 | 16 | `[u8; 16]` movie GUID (128-bit session identifier) |

Because the trailer is located at a fixed offset from the *end* of the state, readers do not need to
know the internal state layout to validate it. Load behavior:

- **No trailer** (any file/state older than this feature): the state may only be loaded while no
  movie is actively providing input; otherwise loading is refused to prevent silent desync.
- **Trailer, `active = 0`**: the state carries no movie context; loaded normally when not in movie playback.
- **Trailer, `active = 1`**:
  - **Identity verification**: if a 16-byte GUID is present in the trailer, it must match the currently loaded movie's GUID. For legacy 21-byte trailers without GUID, the frame and event counts are checked against the current movie. Loading a state saved for a different movie is rejected.
  - **Playback seeking**: when playing a movie, loading any savestate saved during that movie session restores `cursor` to the saved position as long as `cursor <= movie.frames.len()`.
  - **Re-recording**: when recording, loading an earlier savestate truncates all recorded frames and events beyond that cursor, moves `cursor` back to the saved point, increments the `rerecords` counter, and continues recording.
  - **No movie loaded**: loading is refused until the matching movie is loaded.

---

## FM2 compatibility

AccuNES can **import** and **export** the standard FCEUX native FM2 text format:

- **Headers written/read:** `version`, `emuVersion`, `rerecordCount`, `palFlag`, `romFilename` /
  `romName`, `romChecksum` (FCEUX decimal CRC32), `guid` (UUID), `fourscore` (0 or 1), `port0`–`port3`,
  `comment`, `subtitles`, `author`, `startsFromSavestate` (0 for power-on, 1 for savestate), `savestateBinaryInput`.
  On import, `romName` takes precedence over `romFilename`.
- **Frames:** standard FCEUX pipe-and-mnemonic format:
  ```
  |<commands>|<port0>|<port1>|...|
  ```
  - **Commands column (bitmask):**
    - Bit 0 (`1`): Soft reset
    - Bit 1 (`2`): Power cycle / hard reset
    - Bit 2 (`4`): FDS insert / eject disk
    - Bit 3 (`8`): FDS swap disk side
    - Bit 4 (`16`): VS System Coin 1
    - Bit 5 (`32`): VS System Coin 2
    - Bit 6 (`64`): VS System Service button
  - **Gamepad columns:** uses standard FCEUX `RLDUTSBA` / `.` mnemonics (`R`ight, `L`eft, `D`own, `U`p, S`t`art, `S`elect, `B`, `A`), mapping directly to AccuNES controller inputs. AccuNES also supports importing legacy comma-separated columns and raw binary strings.
- **Regions:** `palFlag` maps to `PAL`/`NTSC`; the legacy region is otherwise `AUTO`.