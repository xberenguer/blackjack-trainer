# StickS3 Rust firmware

The [Waveshare 1.75C target](../docs/WAVESHARE.md) is also pure Rust. Build it with `python3 firmware/tools/build.py --board waveshare175c`; it uses a separate round renderer, CO5300 QSPI driver and physical buttons. Touch is held in reset. Its factory-map app-only installation differs from the Stick instructions below.

The application, controller, renderer, codec, flash journal and hardware drivers are Rust `no_std`, with no allocator, C++ adapter, Arduino application runtime or radios. The existing bootloader and chip ROM remain in use. Core1 is not started.

Build/package from the project root:

```sh
python3 firmware/tools/build.py
```

The tested builder uses an Espressif Rust1.97 toolchain (locally named `roulette-esp`), pinned Cargo sources, Xtensa GCC and `-Zbuild-std=core`. See [build setup and toolchain limits](../docs/BUILD.md) for prerequisites, project-local build paths and overrides. `--download` retrieves missing exact archives and verifies Cargo.lock hashes. `STARDUST_XTENSA_GCC` and `STARDUST_ESPTOOL_PYTHON` override installed tool paths. `.build/firmware-vendor/` is generated local state, not portable release source. No build command opens serial or flashes a device.

`build/` contains app/partition binaries, ELF, SHA256 manifest, stack disassembly and an independent IRAM/DRAM physical-alias check. The local licensed `hal-patch` reserves the complete actual IRAM alias including vectors/alignment; it also supplies primitive I2S diagnostics. It never patches Cargo home. Packaging rejects alias overlap, main frames of32KB or greater, and internal stacks below256KB. Stack guards remain enabled. The private decoder recursively fills existing candidate fields/array elements, avoiding whole-model return temporaries. The linked audit also resolves literal-loaded `movsp` allocations beyond the entry instruction; exact frame sizes are recorded in `linked-stack-proof.json`. Ordinary functions are capped at32KB and every in-place decoder at8KB. Only existing `App::load` and wager-model validation have explicit64KB limits; both remain listed. Unresolved stack mutations fail packaging. Resolved call paths and unresolved-call caveats remain visible rather than claiming a complete interrupt/recursion bound.

## Installation and partition boundaries

**Retain a complete current8MB flash backup before changing this table.** Also retain any original recovery backup. Never erase the chip, NVS, OTA data or the whole predecessor filesystem.

| Region | Offset | Bytes | Change |
|---|---:|---:|---|
| NVS | `0x9000` | `0x5000` | Preserved; legacy importer is read-only |
| OTA data | `0xe000` | `0x2000` | Preserved |
| App0 | `0x10000` | `0x600000` | Same offset/capacity |
| Reserved predecessor filesystem | `0x610000` | `0x160000` | Unmounted reserved region now ends at`0x770000` |
| Private `rustresume` | `0x770000` | `0x80000` | New: two256KB slots; may contain old predecessor bytes until explicitly written |
| Public `rustsave` | `0x7f0000` | `0x2000` | Existing two4KB sectors preserved/migrated |

Write the table at`0x8000` and application at`0x10000` only after backup/qualification. Existing bootloader/OTA data remain intact. The trailing flash region is not assigned or erased. Firmware requires exact nonoverlapping, unencrypted partition entries before enabling writes. It never guesses that old bytes are blank. This project does not flash automatically.

## Hardware and memory

| Function | Configuration |
|---|---|
| ST7789V2 display | SPI3 MOSI39/SCK40/DC45/CS41/reset21,40MHz,240×135 landscape; accepted40 and52/53 offsets; MADCTL`0x6c`/`0xb8` includes BGR |
| Backlight | GPIO38,256Hz/10-bit PWM; valid78125 fixed divider at80MHz APB |
| Blue/Grey | GPIO11/12, active-low pull-ups,30ms debounce; shared850ms hold recognition |
| PM1/codec bus | I2C1 SDA47/SCL48,100kHz; PM1`0x6e`, ES8311`0x18`; preserve unrelated PMIC bits, disable watchdog/Grey-reset/LED, configure GPIO3 as push-pull GPIO output before enabling amplifier |
| Speaker | I2S0 MCLK18/BCLK17/WS15/DOUT14; explicit Philips WS16/active-low/MSB-delay, two16-bit slots,22050Hz; mono duplicated to stereo |
| Entropy | SAR-ADC TRNG acquires separate boot seed and transaction nonce; source then stops; no radio startup |

Active, frozen and candidate Apps plus USB/opaque storage buffers occupy disjoint8MB OPI PSRAM regions. An immutable valid `App::EMPTY` image is copied directly into each region; boot/decode work changes individual fields. No runtime whole-App constructor is placed on the main stack. DMA and aligned512-byte flash staging buffers stay in internal RAM. PSRAM is accessed before/after cache-off flash calls, never by the driver during them. Arbitrary final logical chunks are padded with`0xff` to a4-byte physical write; CRC/length exclude padding.

An8KB DMA stream pumps bounded chunks. Muting and interrupted win/blackjack cues clear queued DMA immediately. New grading cues wait for verified public persistence when Resume is enabled. Existing PCM may continue during persistence. Starvation is counted/recovered; submission counts alone do not establish acoustic output.

All 14 raw PCM clips are unchanged. Native and web playback call `audio_gain::playback_sample`, mapping 0–100% to the original master-volume domain with `(percent * 255 + 50) / 100`. The20% default maps to master51. Legacy `scaled_sample` retains master120 for independent PCM/gain oracle checks; percentage playback uses the same source-order f32, Q8 truncation, arithmetic shift and final i16 clamp.

PM1 GPIO3 must be configured as GPIO, push-pull and output before its output latch enables the speaker amplifier. Setup starts low, then enables the amplifier only with master sound on and nonzero volume. A nonblocking 10ms silent lead-in precedes queued playback after enabling. Mute, volume and category changes cancel active DMA immediately. USB exposes safe amplifier register readbacks for electrical diagnosis; these do not establish audible output.

## Persistence and recovery

See[resume protocol](../docs/RESUME.md). Public Save V13 remains 2636 bytes; byte583 is the default-off resume preference. Byte202 stores volume, byte203 stores category bits and Fast speed. V1–V9 receive20% volume, all categories and Fast speed without changing existing scores. Exact V9 raw public binding bytes remain intact while an old private checkpoint is offered. After explicit resume, output and new private capture wait for verified canonical V13 migration. Legacy NVS remains read-only. Existing SBRJ V1 records remain authoritative over old NVS; new public SBRJ V2 records carry exact public bytes plus64-bit generation/epoch and an independent128-bit nonce.

New meaningful state invalidates older private checkpoints through a verified public commit before showing its frame, exposing status or playing its grading cue. Private snapshots capture after1200ms of stability and write cooperatively. Each step hashes/copies at most512 bytes or performs one4KB erase. The target footer sector is erased first, followed only by actual payload sectors; the commit footer is last. A measured142326-byte full bound checkpoint needs36 erased pages rather than all64.

A valid newer public record always wins. A committed-corrupt public record or newer unfinished validated header prevents private restore; known valid public progress remains available. Read I/O is never treated as empty or permission to import stale NVS. Retry/Fresh choices are visible after display initialization, including initial read failures. An explicit unsaved fresh session discards pending live state, uses only last acknowledged preferences/scores if available, starts Off/$1000/minimum, shows UNSAVED and disables all writes/resume for that boot. Unknown saved scores are labeled unavailable. Neither journal is changed by that RAM fork.

## USB and actual-device qualification

Native serial/JTAG is bounded/nonblocking. `s` reports public state/settings/metrics and hardware counters; `f` returns`RGB565 240 135 64800\n`,64800 LE RGB565 bytes, then`\nEND_FRAME\n`. `b/g` are Blue/Grey taps, `h/m` holds, and`x` quiz submission. Frame transfers use a frozen separate buffer. Status/frame access is gated during unacknowledged transitions; chooser frames are explicitly safe. No command exports private snapshots, future shoe order, hidden cards or game RNG.

Safe resume aggregates are`resume_enabled`, `resume_writes`, `resume_errors`, `resume_blocked`, `resume_saved`, and`unsaved`. `resume_saved=true` means the current revision has completed its private commit. Counts reset at boot. After enabling Resume, open an unsubmitted count quiz, wait for a new private write and`resume_saved=true`, restart, choose Resume after neutral release, and compare visible question/frame and unchanged public totals. Disable Resume and restart to verify normal fresh Off boot. The installer owns all USB actions.

Optional`p` runs18 fixed public fixtures in a separate PSRAM App: Guide/Test/Coach × funded8/8, fundedA/A, soft hand, insurance, funded2/2 and funded3/3. It never saves, advances or renders the live game. Each line reports case/mode/event_us/underrun delta/graded count; completion reports maximum latency and total delta. With the device idle at Cut/Off, use the owning tool:

```sh
python3 tools/device_benchmark.py --port YOUR_SERIAL_PORT --out .build/device-benchmark
```

This is a device validation command, not part of building. It requires unchanged public live state,18 cases and zero DMA-underrun deltas. Record actual timing/physical acceptance separately from host/build results.

Host checks (from the Rust project):

```sh
cargo +stable test --release --offline --manifest-path firmware/tools/nvs-check/Cargo.toml
cargo +stable test --release --offline --manifest-path firmware/tools/resume-check/Cargo.toml
cargo +stable test --release --offline --manifest-path resume-journal/Cargo.toml
```

The read-only NVS tool accepts exact20480-byte NVS or8192-byte public journal backups and prints only game preferences/counters. It does not accept private partition images. Keep actual user backup inputs outside releases.

Round background/card-back preferences use the same public packet and mirrored private TLV; [V12 compatibility details](../docs/RESUME.md#round-display-preferences-and-older-saves).
