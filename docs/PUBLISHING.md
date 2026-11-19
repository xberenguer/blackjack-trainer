# Preparing blackjack-trainer for publication

This is a local preparation guide, reviewed2026-11-18 against0.11.2. No repository
or release URL is assigned here. No remote has been created or pushed by this
audit. The proposed GitHub repository name is `blackjack-trainer`.

The immediate deliverable is a **complete local review candidate** with the
approved game assets intact, private device data excluded, and the asset-rights
questions clearly attached. It is not a statement that the current archive is
cleared for public upload. No stripped or unbuildable public export is created
as part of this preparation.

## What needs review before an external upload

- Resolve the source-code license text and its scope. The Rust Cargo metadata
  declares MIT; the predecessor Video Poker program is GPL-2.0-or-later. They
  must not be described as a single blanket license.
- Review redistribution rights for the inherited cards/buttons, photographic
  Venetian chip sprites, Ultimate X sounds, reusable Arial glyph bank, and the
  source reference/marks behind the generated Stardust art. Exact evidence and
  outstanding questions are in [THIRD_PARTY.md](THIRD_PARTY.md) and
  [asset-source-index.json](asset-source-index.json). Retain the approved local
  assets during this review; this preparation does not remove or replace them.
- Include applicable dependency license texts/notices for the actual source
  and binaries being distributed. Pinned esp-hal license texts are retained in
  `docs/licenses/`; its local changes are documented in the HAL patch directory.
- Test the intended source layout on a clean machine or clean directory before
  claiming a portable build. See the current portability findings below.
- Review the staged file list and every release attachment independently. A
  `.gitignore` file does not filter an uploaded ZIP, erase tracked history, or
  confer asset permissions.

## Curated file selection

Use an explicit manifest, not a recursive copy of this working directory.

| Material | Local review candidate | Git source candidate / eventual release |
|---|---|---|
| `src/`, root Cargo files, `examples/`, `resume-journal/` | Include | Include, retaining the declared license scope |
| `tests/`, including `tests/data/` and `tests/reference/*.gz` | Include unchanged | Include unchanged: these are synthetic fixtures/oracle state and hash rows; do not rewrite them to conceal history |
| `assets/manifest.json`, importer and provenance documents | Include | Include with exact hashes and honest permission status |
| Three `assets/*.bin` banks | Include approved originals, flag rights review | Decide after rights review; all are required by the current build and none is automatically cleared by the source license |
| Browser HTML/JS/CSS, `bridge/`, `journal/`, tests/build scripts | Include source | Include source after portable-path fixes |
| `browser/stardust-rust.wasm` | May include the exact verified build for local review/play | Generated release attachment after rights review; contains embedded asset data |
| Firmware Rust source, Cargo files, build/linker files, partition CSV and complete `hal-patch/` | Include source and license notices | Include after clean-layout build verification |
| Verified firmware BIN/ELF/partition and sanitized build proof | Select deliberately for local review | Select as versioned release attachments after rights review; these are not flash backups |
| Root/player/install/settings/analysis/rules/resume guides | Include | Include with working relative links and clone-root commands |
| Curated synthetic screenshots/video | Include for local media review | Review visible content and rights before upload; captions must distinguish simulated images from actual device evidence |
| Raw validation directories, compiler logs and historical archives | Exclude by default | Publish only a separately curated aggregate proof, without author paths or player data |
| Recovery, flash/NVS/journal snapshots and private checkpoint bytes | Exclude entirely | Never add to the public source or release attachments |

The ignore rules exclude `recovery/`, `releases/`, raw validation/device captures,
Cargo targets/caches, generated firmware outputs/WASM, local environments and
credentials. Only six individually reviewed aggregate validation JSON files are
allowlisted: releases0.10.0,0.10.2,0.11.0,0.11.1,0.11.2 and
`rust-core-validation.json`. Older personal-score records and all nested
validation directories remain excluded.

The three rights-uncleared asset banks are also ignored for Git staging by
default. They remain unchanged on disk and are deliberately included by the
separate full **local review** exporter; that exporter is not a Git publish
action. Resolve the rights review before removing their exclusions or staging
them explicitly. There is no blanket `.bin` or `.gz` rule, so native fixtures
remain intact. Do not force-add an ignored path just to make a ZIP match an
earlier archive.

Keep actual test readbacks outside the review bundle, including the optional
`STARDUST_OLD_RESUME_READBACK` input to `firmware/tools/storage-check`. The harness
itself is source; an actual player's checkpoint is not a fixture to publish.

A complete source archive from `tools/package_release.py` currently differs
from this public-file policy: it excludes recovery/device directories but still
includes approved third-party banks and some general validation logs containing
local paths. Its successful privacy checks must not be relabeled as permission
clearance or a complete public-upload audit. Reinspect each actual archive.

## Player-facing delivery

Write for experienced blackjack players; explain this tool's controls, training,
saving and settings rather than teaching ordinary blackjack. Installation can
still be step-by-step for someone who does not develop firmware.

For a reviewed ready-to-play browser bundle, include its matching WASM and all
browser runtime files. From the extracted project directory:

```sh
python3 -m http.server 8000 --bind 127.0.0.1 --directory browser
```

Open `http://localhost:8000`, then click or press a control to allow browser
audio. Opening the HTML through `file://` is not the supported launch path.
Saves belong to this browser profile and origin; changing the port/origin or
clearing site data changes which saved session is available. Resume remains an
explicit opt-in. A source-only checkout needs a browser build before this step.

For StickS3 installation, use [INSTALL.md](INSTALL.md) and
[hardware/partition notes](../firmware/README.md). The release must identify the
exact compatible StickS3 model,8MB layout, version/hash, required bootloader,
full-backup procedure, port selection, offset-specific update steps and recovery
path. Never present a whole-chip erase or somebody else's device backup as a
normal installer. Starting from a blank device is a separate, unqualified
bootstrap path until a bootloader is supplied and tested for it.

## Build prerequisites and current portability findings

The core has no external crate dependencies. Native checks need Rust/Cargo with
edition2024 support, Clippy and Python3.11+ (`tomllib` is used by tools). The
target builders require Python3.12+ for safe tar extraction. The
verified local workflow is:

```sh
python3 tools/check.py
```

The browser bridge needs an appropriate nightly toolchain, `rust-src`,
`wasm32v1-none`, `rust-lld` and `-Zbuild-std=core`. Node.js runs its software tests;
actual browser tests additionally need Playwright and an installed Chrome or
Chromium. The tested build fixes WASM memory at16MiB and its stack at2MiB.
Preserve those flags. The project command is:

```sh
python3 browser/build.py
node browser/tests/wasm.cjs
node browser/tests/platform.cjs
node browser/tests/audio.cjs
```

Firmware needs an Xtensa-enabled Espressif Rust toolchain, `rust-src`, the pinned
Cargo dependency sources, an ESP32-S3 Xtensa GCC/binutils toolchain, Python3.11+
and a Python environment containing esptool. The tested local compiler is
Espressif Rust1.97 nightly under the custom name `roulette-esp`; that name is a
local toolchain alias, not something a normal `rustup` install automatically
provides. esp-hal1.2.2 declares Rust1.95 minimum. Record the exact compiler/tool
versions in a release rather than implying an arbitrary stable compiler can
build Xtensa. The [official installation guide](https://docs.espressif.com/projects/rust/book/getting-started/toolchain.html)
explains the Espressif toolchain and `espup`; installing the current toolchain
alone does not establish bit-for-bit reproduction of this release.

```sh
python3 firmware/tools/build.py
```

That builder never opens a device or flashes. `--download` retrieves exact
missing Cargo archives and verifies lockfile hashes; without it, missing sources
cause failure. Offline builds require a prepared toolchain, rust-src/sysroot and
all locked crates. A clean clone is not an offline dependency distribution.
`STARDUST_XTENSA_GCC` and `STARDUST_ESPTOOL_PYTHON` select tools explicitly.

The publication-preparation path fixes now use project-local `.build/` defaults,
explicit Cargo/tool overrides, and sysroot discovery from the selected toolchain.
The browser acceptance runner no longer depends on an ancestor Cargo directory;
its version comes from Cargo metadata. Chrome tests use standard Playwright
resolution or documented environment overrides. See [BUILD.md](BUILD.md) for the
current commands, prerequisites, tool versions and environment-variable table.

Read-only path/override probes and the full browser acceptance passed after
those fixes: WASM build, state/platform/audio tests, actual Chrome, journal tests
and both strict Clippy checks. The runner used an ephemeral localhost server and
the current0.11.2 core stayed unchanged. The resulting WASM is5823661 bytes,
SHA256 `57cfcb3507eaa0c918dc062c19aa2bcc8d52f71befdbcbc20881d47dd6f270e3`.
These checks establish software behavior in the tested environment, not a
universal fresh-toolchain installer, new firmware build or new device flash.

The remaining limits are concrete:

- The exact tested custom Xtensa/Rust sysroot distribution still needs a
  reproducible installer or a separately qualified replacement. Standard
  `espup install` is an upstream starting point, not a tested reproduction claim.
- Offline first builds need prepared pinned dependencies and the required
  vendored rust-src tree. No installer or cache is silently downloaded by the
  default offline build.
- `firmware/.cargo/config.toml` contains generated absolute linker/vendor paths;
  it stays ignored and is regenerated for each build location.
- The one-time `tools/import_asset_data.py` requires predecessor generated
  headers not included here. Building with approved existing banks needs no C++;
  regenerating source art/audio requires its separate inputs and rights review.
- Historic evidence remains local where it contains author paths or player
  details. Only the six reviewed aggregate JSON proofs are allowlisted. The
  numerical native fixtures/oracles remain unchanged.
- A blank-device first installation remains separate from a verified update of
  the existing bootloader/layout; see [INSTALL.md](INSTALL.md).

## Final local review before any publish action

1. Select a single version and freeze its runtime, guides and attachment hashes.
2. Generate a source manifest from the curated candidate. Confirm there are no
   recovery/device/private records, vendor/cache symlinks or unintended local
   environments. Check text files for author paths and credential patterns;
   review findings rather than replacing binary data blindly.
3. Confirm notices, asset review status, and source/build limits travel with the
   candidate. Keep the full approved local review game usable.
4. Test extracted browser launch and developer build commands from the promised
   directory layout. Distinguish a source build from running a prebuilt bundle.
5. Once a Git repository is deliberately initialized, inspect `git status`,
   `git diff --cached --stat`, `git diff --cached` and `git ls-files`. Check
   ignored paths with `git check-ignore -v`. Ignore rules do not remove already
   staged/tracked files or history; resolve that before an external upload.
6. Inspect ZIP members and hashes separately from Git's staged files. Preserve
   all private backups locally. Record unresolved rights/build questions in the
   review report; do not claim that local preparation already published anything.

Repository creation, remote configuration and pushing are separate actions.
This audit performs none of them.
