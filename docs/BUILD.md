# Building from source

Run these commands from the project root. Building does not open a serial port, flash a device, or erase storage. See the installation instructions before writing firmware. This page describes the supported scripts and the toolchain limitations; it is not a claim that an arbitrary fresh Espressif installation has been qualified.

## Build paths and offline behavior

The scripts default to caches **inside this checkout**:

| Generated state | Default |
|---|---|
| Cargo home | `.build/cargo/` |
| Firmware target | `.build/firmware-target/` |
| Browser target | `.build/browser-target/` |
| Firmware dependency vendor tree | `.build/firmware-vendor/` |
| Packaged native artifacts and audit proofs | `firmware/build/` |
| Packaged browser module | `browser/stardust-rust.wasm` |
| Browser acceptance logs/screenshots | `browser/validation/` |

`STARDUST_BUILD_DIR` changes the `.build` root. `CARGO_HOME` and `CARGO_TARGET_DIR` independently override their respective locations, taking precedence over that root. Relative environment paths resolve from the directory where you invoke the script; `~` is expanded and symlinks are normalized. No script searches ancestor folders for the author's workspace or silently borrows an unrelated Cargo cache. Reusing an existing cache is an explicit choice, for example:

```sh
CARGO_HOME=/absolute/path/to/populated-cargo-home \
CARGO_TARGET_DIR=/absolute/path/to/browser-target \
python3 browser/build.py
```

Use separate target directories for firmware and browser builds. Both builders offer a **read-only** path/tool inspection:

```sh
python3 firmware/tools/build.py --print-paths
python3 browser/build.py --print-paths
```

These inspect installed toolchains/tools but create no configuration, cache or output files. Ordinary builds use `--offline`. Firmware's optional `--download` fetches missing exact `Cargo.lock` crate archives and verifies their pinned SHA256 before extraction; it does not install Rust, GCC, Python packages or other tools. An offline first build needs a populated explicit Cargo cache or previously prepared local vendor tree.

## Required tools and tested versions

| Component | Requirement and qualification scope |
|---|---|
| Python | Python 3.12+ for the builders (`tomllib` and safe tar extraction filter). The publication path checks used 3.14.7. |
| Rust/rustup | Host Rust for native tests; firmware dependencies require Rust 1.95 or newer, plus a selected toolchain with unstable `-Zbuild-std=core`, `rust-lld` and the required target support for firmware/browser builds. |
| Tested target Rust | Locally named `roulette-esp`: `rustc 1.97.0-nightly (8ea53bcd7 2026-07-08) (1.97.0.0)`, with Xtensa support and vendored Rust library sources. The local alias is not a public rustup distribution name. |
| Xtensa GCC/binutils | ESP32-S3 toolchain including `xtensa-esp32s3-elf-gcc`, `nm` and `objdump`. Tested GCC 8.4.0, crosstool-NG `esp-2021r2-patch5`. Required only for firmware. |
| Image packager | A Python environment with `esptool`; tested 4.9.0. Required only for firmware packaging. |
| Partition generator | Espressif `gen_esp32part.py`, available separately from the Rust sources. The tested copy came from the installed Arduino-ESP32/PlatformIO framework tools. |
| Browser tests | Node.js (tested 24.22.3), Playwright and its installed Chromium, or an explicit Chrome executable. These are test dependencies, not runtime dependencies of the game. |

Both builders honor **`STARDUST_TOOLCHAIN`**, without a leading `+`. If unset, they prefer an installed `roulette-esp`, then an installed toolchain named `esp`; otherwise they stop with a setup error. The `esp` fallback is discovery support, **not** evidence that every `espup` release is compatible. Pin and qualify an alternative before distributing its output.

For the selected toolchain, the scripts ask `rustc --print sysroot` through rustup. They require this directory beneath that actual sysroot:

```text
lib/rustlib/src/rust/library/vendor/
```

That vendored `rust-src` tree supplies the offline `build-std` dependencies. Merely installing a target or a standard `rust-src` component does not guarantee this vendor tree exists. The scripts fail clearly if it is absent. This repository does not currently provide a tested installer that recreates the custom target toolchain on a new machine, or a claim of bit-identical output across compiler versions.

## Native core checks

The shared core has no third-party runtime dependencies. With a compatible host Rust installed:

```sh
cargo test --release --offline
cargo clippy --all-targets --all-features --offline -- -D warnings
python3 tools/check.py
```

The last command includes retained independent reference/math checks. Host tests do not replace linked firmware memory proofs or actual-device qualification.

## Firmware build and packaging

Set tool paths when they are not discoverable:

```sh
export STARDUST_TOOLCHAIN=your-qualified-xtensa-toolchain
export STARDUST_XTENSA_GCC=/absolute/path/to/xtensa-esp32s3-elf-gcc
export STARDUST_ESPTOOL_PYTHON=/absolute/path/to/python-with-esptool
export STARDUST_PARTITION_TOOL=/absolute/path/to/gen_esp32part.py
python3 firmware/tools/build.py --print-paths
python3 firmware/tools/build.py --download
```

Omit `--download` once pinned dependencies are prepared for an offline build. `CARGO_HOME` can explicitly select an existing Cargo archive/source cache. For GCC, the script otherwise tries PATH and the conventional PlatformIO ESP32-S3 package path. For the partition tool, it tries PATH and the conventional PlatformIO Arduino-ESP32 package path. The default esptool Python is the Python running the script; use the override for a separate virtual environment.

The builder prepares its own vendor tree and generates ignored `firmware/.cargo/config.toml` with the resolved linker/vendor locations. Those machine-specific paths are local generated state, not portable source. Cached source files may be linked into that generated vendor tree; do not publish it. Cargo home is not patched. The firmware package version is synchronized with the root Cargo package version.

The existing offline build, application partition size check, actual ELF IRAM/DRAM physical-alias proof, stack-size/main-frame proof and linked dynamic-frame/decoder audit all remain mandatory. Packaging uses the checked-in partition CSV. `firmware/build/manifest.json` records the resulting ELF/BIN/table hashes and memory metadata. The builder does not supply or replace a device bootloader, and does not run a flash command. See [firmware details](../firmware/README.md) for memory, partition and hardware qualification scope.

## Browser build and local play

```sh
python3 browser/build.py --print-paths
python3 browser/build.py
python3 -m http.server 8000 --bind 127.0.0.1 --directory browser
```

Open `http://127.0.0.1:8000`. A localhost HTTP server is needed rather than opening `index.html` as a file. The standalone `wasm32v1-none` module has no runtime imports; the static host is ordinary HTML/JS/CSS. There is no C++, Emscripten or Node requirement to **play** it. The build fixes its 16 MiB linear memory and 2 MiB stack ABI rather than accepting caller memory flags. It uses the selected toolchain's vendored sysroot for offline `core` compilation.

## Browser and Chrome acceptance

Install Playwright through your normal Node project/package environment and install its browser, or explicitly point at existing installations. The test helper uses standard `require('playwright')` resolution, including Node's `NODE_PATH`, unless **`STARDUST_PLAYWRIGHT_MODULE`** names a module directory. It never imports an author-specific absolute path.

| Environment override | Purpose |
|---|---|
| `STARDUST_NODE` | Node executable for the Python acceptance runner; otherwise `node` on PATH. |
| `STARDUST_PLAYWRIGHT_MODULE` | Optional absolute module directory; otherwise standard Node resolution. |
| `STARDUST_CHROME_EXECUTABLE` | Optional browser executable; otherwise Playwright's installed Chromium. |
| `STARDUST_CHROME_CHANNEL` | Optional Playwright channel, such as `chrome`, when no executable override is supplied. |
| `STARDUST_BROWSER_URL` | Origin for a directly invoked resume/DOM test; defaults to `http://127.0.0.1:8784`. The full runner supplies its own temporary localhost origin. |

For a normal local Playwright installation:

```sh
python3 browser/tests/acceptance.py
```

This builds WASM, starts and stops an ephemeral localhost server, runs the existing WASM/platform/audio/Chrome checks and journal/Clippy checks, then writes a versioned source-hash acceptance report. It requires a Clippy component in the chosen target toolchain. It does not require an already-running port 8784 server.

Other focused checks can be run independently:

```sh
node browser/tests/settings.cjs
node browser/tests/resume.mjs
node browser/tests/analysis-chrome.cjs
```

The analysis Chrome test starts its own ephemeral server. Direct `resume-chrome.cjs`/`chrome.cjs` invocation expects a server at `STARDUST_BROWSER_URL` or its default 8784 origin. The existing native/browser assertions and PCM gold fixtures are unchanged by these path fixes. Chrome launch can be restricted by the host's process sandbox; dependency/path resolution alone does not establish successful browser acceptance. Report that separately from WASM/native tests and physical-device checks.
