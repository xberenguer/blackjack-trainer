# Artwork, font and audio provenance

The Rust runtime reads three compiled data banks. It does not link the former
C++ renderer or audio implementation. `tools/import_asset_data.py` copied
approved RGB565/alpha sprites, bitmap glyph masks/metrics and signed PCM from
three generated headers. [assets/manifest.json](../assets/manifest.json) records
those header hashes, bank sizes/hashes and record order.

The original card faces, button bases and Arial glyph resources came from the
user's earlier Video Poker project. Cards retain the earlier source-resize/native
resize pipeline; text retains per-glyph bearings, advances and kerning, sizes7–24.
The Stardust back/wordmark came from approved local generated art based on the
user's vintage reference. Venetian chip sprites use cropped and normalized
photographic references. These are provenance descriptions, not rights grants.

Audio uses13 original Ultimate X recordings in14 cue entries, including the
shared button clip. The earlier pipeline decoded/resampled these to mono signed
16-bit PCM at22050Hz. The Rust migration preserves that PCM; the separately
verified Rust playback helper applies the user's percentage volume. Matching
PCM and digital gain do not alone prove speaker acoustics.

The original public media URLs, available source hashes, bank hashes and
publication status are now in [the source index](asset-source-index.json).
Predecessor source images, font atlases, OGG originals, generation prompts and
its fuller conversion manifests are retained in the local predecessor project;
they are **not included** under this Rust directory's `assets/` paths.

[Third-party notices and unresolved redistribution rights](THIRD_PARTY.md)
separate the fresh Rust program from inherited data and dependencies. The older
reference to “the project's GPL” applied to the predecessor program; this Rust
package currently declares MIT metadata, whose scope must not extend to those
third-party materials.
