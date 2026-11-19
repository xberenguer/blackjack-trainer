# Third-party material and publication status

Reviewed 2026-11-18. This notice records evidence; it does not grant new rights.
The current local game remains unchanged. See [publication preparation](PUBLISHING.md)
for the distinction between a private review copy and a public source draft.

## Fresh Rust source and predecessor project

`Cargo.toml` declares `MIT` for this Rust package. At this audit's start, no root
`LICENSE` file accompanied that declaration. The maintainer must supply the
matching license text and state its scope before a public release. A source
license must expressly leave third-party assets and dependencies under their
own terms; the metadata does not relicense them.

The earlier [xberenguer/videopoker](https://github.com/xberenguer/videopoker)
project labels its original program GPL-2.0-or-later. Its actual
[LICENSE](https://github.com/xberenguer/videopoker/blob/main/LICENSE) and
[NOTICE](https://github.com/xberenguer/videopoker/blob/main/NOTICE), inspected
read-only, distinguish program code from vendor images, marks and Arial-derived
bitmaps. That predecessor notice is not a permission grant for these assets.
The Rust asset importer copies generated data arrays, not that program's runtime.
Any future import of predecessor program code requires its own license review.

## Asset banks

| Local material | Recorded source | Publication evidence/status |
|---|---|---|
| `assets/sprites.bin`: 52 card faces and button bases | VideoPoker.com public image service; the preceding local asset manifests record original URLs and hashes | No redistribution grant located in this project. Obtain permission or use a separately reviewed replacement in a future release. |
| `assets/sprites.bin`: Venetian chip faces | ChipGuide image reposted to PokerChipForum, another PokerChipForum photo, Spinetti's product photo, and an eBay photo | Source photographs and chip designs retain their owners' rights. No redistribution permission recorded. |
| `assets/sprites.bin`: Stardust back/wordmark | Locally approved generated artwork based on a user-supplied vintage reference | Approval of the local result does not establish rights in the reference or marks. Record those rights and avoid implying affiliation. |
| `assets/fonts.bin` | ASCII glyph atlases rasterized from installed macOS Arial regular/bold, sizes7–24 | The original manifest expressly describes this as a private local build. No app/device redistribution license located. A full per-glyph font bank is distinct from an image of a finished text passage. |
| `assets/audio.bin` | 13 unique Ultimate X OGG files, mapped to14 cue entries; public VideoPoker.com media URLs | Downloads and PCM conversion establish provenance, not permission to redistribute. No grant located. |

[The source index](asset-source-index.json) preserves public source URLs, source
hashes and exact bank hashes without copying credentials or font files. The
current `assets/manifest.json` pins the generated C++ header hashes used by the
one-time Rust data importer. The larger predecessor source masters/manifests
are not present in this standalone Rust directory; earlier documentation that
linked them as if they were included was inaccurate.

The [VideoPoker.com terms](https://www.videopoker.com/terms/) are a service
agreement, not an open-source asset license. Public download access and an
upstream project's license badge do not establish this project's right to
republish the media. Microsoft's [font redistribution FAQ](https://learn.microsoft.com/en-us/typography/fonts/font-faq)
distinguishes game glyph banks from finished text images, but addresses
Windows-supplied fonts; it does not resolve the actual macOS Arial license here.
Confirm the source font's applicable license with its licensor.

These qualifications apply equally to raw banks, embedded firmware/WASM,
full source archives containing the banks, and screenshots/video containing the
artwork. No approved local asset is removed or replaced by this audit.

## esp-hal and linked dependencies

The local `firmware/hal-patch/` is esp-hal1.2.2, from upstream commit
`915465df59aee91b627fe538f0b442e56c7f4fc9`, with the two documented changes in
`firmware/hal-patch/STARDUST-PATCH.md`. Its upstream package declares
`MIT OR Apache-2.0`. The registry crate did not include the workspace license
files referred to by its README. Exact texts from that pinned upstream commit
are now retained here:

- [esp-hal MIT notice](licenses/esp-hal-LICENSE-MIT)
- [esp-hal Apache-2.0 license](licenses/esp-hal-LICENSE-APACHE)

Those texts apply to esp-hal and retain the upstream copyright, not to every
file in this repository. Cargo dependency versions/checksums are pinned in the
relevant `Cargo.lock` files. Before distributing binaries, inventory the actual
linked dependencies and retain their applicable notices; this page is not a
complete transitive binary-notice bundle. Do not publish the generated Cargo
cache/vendor symlinks as if they were portable source.

This is an independent project. M5Stack, Espressif, VideoPoker.com, Action Gaming,
Ultimate X, Arial, Venetian and Stardust references identify compatibility or
provenance and do not assert sponsorship or endorsement.


## Waveshare hardware adaptation

The round CO5300 driver follows the Rust hardware implementation in [xberenguer/stopwatch](https://github.com/xberenguer/stopwatch). Panel wiring, pixel orientation and physical button mapping match [xberenguer/Seeder-Fork](https://github.com/xberenguer/Seeder-Fork). The app-only installer is adapted from its MIT-licensed `scripts/flash_waveshare175c.py`. Copyright (c) 2023 Bitmaker and Lunaticoin. Its MIT permission notice is retained below. The ES8311 register/clock configuration follows Waveshare’s published board example; no microphone or touchscreen functionality is used.

```text
MIT License

Copyright (c) 2023 Bitmaker
Copyright (c) 2023 Lunaticoin

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

Libraries:

uBitcoin - MIT License
Copyright (c) 2019 Stepan Snigirev
```
