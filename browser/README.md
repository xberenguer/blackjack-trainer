# Standalone Rust browser

From the repository root, serve the browser directory over HTTP:

```sh
python3 -m http.server 8000 --bind 127.0.0.1 --directory browser
```

The display is native 240×135 RGB565, with smoothing disabled. Grey/Left hits or advances; Blue/Right stands, deals or selects. Hold for 850 ms: Grey opens the menu and Blue cycles Off/Test/Coach/Guide. Both submit a count answer. Tab+Enter/Space activates an accessible button tap. All rules, controls, screens, grading, advice and save migration run in safe Rust. JavaScript transports input, pixels, audio and opaque storage records.

Resume is OFF by default. Enable **Settings → Resume session** to save a private checkpoint on this browser. On the next load choose **Resume saved session** or **Start a new session**. Normal boot and a deliberate new session always start training Off; only explicit resume restores the exact saved live mode. Resume preserves active hands, chip session, ordered card reveals, pending questions/help, first recorded grades, reviews and current session analysis. A restored face/review receives a fresh readable dwell, and held keys are consumed until neutral. No old audio cue replays on resume.

Public preferences and exact training totals use separate journal records from the private session. Each material resumable transition advances the public generation/epoch even if its public totals are unchanged. Public bytes are written and read back before the new frame or feedback cue can be presented. The private checkpoint is then captured and bound cooperatively. A missing/stale/corrupt private checkpoint never replaces newer public totals. Public corruption taints resume: surviving valid public progress may load Off, but the old matching private question cannot return. Read I/O is not treated as an empty save: the boot chooser offers Retry or a deliberate fresh RAM session, labels unknown saved totals as unavailable/defaults, and keeps automatic writes disabled on that failure.

When resume is enabled, a failed public acknowledgment shows **SAVE RETRY**, holds the new answer/feedback behind the gate and retries without overwriting foreign progress. Choose **Start fresh without saving** to discard the pending answer/session in RAM, retain only acknowledged public totals/preferences, and play a fresh Off session with no writes or resume for this boot. Both journals remain unchanged; a later reboot may still offer the last valid durable checkpoint. With resume disabled, unavailable storage permits RAM play with an explicit **UNSAVED** status. A private quota failure after a successful public write keeps the scores and reports that resume is unavailable. Browser storage is local to its origin/profile; private checkpoints are internal persistence, absent from the page's game info, help, downloads and debug API.

## Build and checks

This is a standalone `no_std` `wasm32v1-none` module with zero runtime imports, no C++, Emscripten, allocator or memory growth. See [BUILD.md](../docs/BUILD.md) for tested toolchains, project-local caches and explicit environment overrides. With those prerequisites prepared, run from the repository root:

```sh
python3 browser/build.py
node browser/tests/wasm.cjs
node browser/tests/platform.cjs
node browser/tests/audio.cjs
node browser/tests/settings.cjs
# Full software acceptance, including its own temporary local server and Chrome:
python3 browser/tests/acceptance.py
```

The read-only `--print-paths` option shows resolved paths without creating caches. Override `STARDUST_TOOLCHAIN`, `CARGO_HOME` and `CARGO_TARGET_DIR` for another environment. The build needs nightly `-Zbuild-std=core`, `wasm32v1-none` and `rust-lld`. The bridge depends on the safe core and the auxiliary safe `journal/` crate shipped inside this browser folder.

`wasm.cjs` exercises 7200 normal rounds across 48 seeds, 60 deep seven-seat rounds, controls, pixels, raw PCM, bounded imports and legacy saves. `platform.cjs` runs the real JavaScript journal and actual WASM through pending questions, independent continuation, power loss, quota, read I/O, corruption, nonce and multi-tab conflicts. `audio.cjs` compares every raw/scaled sample against independent original-C++ golden hashes. `chrome.cjs` drives the actual page, reload chooser, failed-write recovery and frame/status/audio acknowledgment order, saving screenshots. These are software checks; device validation is separate.

## ABI and safety

Memory is fixed 16 MiB, unshared, with a 2 MiB stack. `sb_frame()` addresses 32400 RGB565 pixels. `sb_save_buffer()` has exactly `sb_save_size()` (2636 bytes); imports accept 196/208/336/584/2636-byte V1–V10 packets and the safe core validates version, CRC and semantics. Current exports are V10, with percentage volume, sound categories and speed stored in previously reserved bytes. V1–V9 initialize these preferences to20%/all categories/Fast while preserving existing scores. Exact 64-bit totals stay in binary and never pass through JavaScript numbers. Live repeated save imports are rejected.

`sb_store_*` is an internal bounded journal transport, used only by the lexical storage adapter. Its buffer has a 262088-byte ceiling (262000-byte bound checkpoint plus 88-byte record framing). The Rust codec owns identities, slot selection, checksums, strict public taint, nonce/epoch/raw-packet binding and all candidate validation. Web Locks serialize this adapter's writers; read-before-write rejects a changed base rather than overwriting another tab. JavaScript obtains an independent 128-bit nonce from `crypto.getRandomValues`, never from game RNG. The two localStorage slots use atomic `setItem` plus exact readback. No private bytes are logged, returned through `window.stardust`, or offered as an export. Temporary JavaScript private byte copies are cleared after storage transport.

The narrowly scoped unsafe code is at the single-threaded, non-reentrant WASM ABI: singleton Apps, fixed buffers and retained task borrows. Exports never call foreign code; there are no threads/imports. Active, frozen and candidate Apps and task buffers are disjoint. Safe explicit fieldwise cloning creates the frozen/candidate state; private decode never targets the active App. A task is canceled before its borrowed buffers are reused. The core and journal crates forbid unsafe. The host may write only the bounded public-save and internal transport buffers. Normal status exposes allowlisted public facts only. During pending or failed protected acknowledgment it returns cached/safe facts, acknowledged public Save and frame, and no new feedback cue.

PCM assets and sample rates remain unchanged. Browser output calls shared Rust `audio_gain::playback_sample` with the selected percentage volume, then sends `i16/32768` to unity WebAudio gain. The original fixed-master `scaled_sample` remains available for independent legacy gain checks. This verifies digital gain, not perceived loudness through a physical speaker/analog chain or the former stateful resampler. Clips play serially; input interrupts only Win/Blackjack as designated by Rust. Master mute, volume and category changes immediately stop the active buffer. Explicit auditions bypass their category toggle but obey master mute and zero volume. AudioContext unlock/playback remains best effort after a gesture.
