# 0.11.3 — Clear Guide insurance and web play

Guide keeps the dealer insurance prompt, large Decline/Yes choices, side-bet amount and selection highlight visible. Advice occupies a separate space; mode notifications also stay clear of insurance controls. Grey changes the choice; Blue confirms. Strategy, count handling and SaveV10 remain unchanged.

The GitHub front page is now the complete illustrated feature guide. Installation and the hosted [web version](https://xberenguer.github.io/blackjack-trainer/) are linked first. The redundant Quickstart is removed. Existing feature-guide links still lead to the main guide.

The [firmware flasher](https://xn--sg8hhb.ws/flasher/) consumes a versioned firmware contract from stable GitHub releases. Its signed catalogue checks source CI, artifact hashes and partition maps. Matching installs update only the application; migrations protect existing matching journals and require a recovery backup. StickS3 browser migration is **Preview**, pending physical end-to-end qualification.

## Verification scope

- Five new insurance tests plus four existing table checks pass: Basic/Hi-Lo advice, affordability, large count/bankroll values, Grey/Blue choices and hidden-hole invariance.
- Strict all-target/all-feature Clippy passes. Historical table checks retain 5,326 matching frames, with 35 intentional insurance/notification differences covered separately; golden fixtures are unchanged.
- Firmware builds with 305,068 bytes of main stack, a 20,720-byte main frame, no unknown stack mutations and no IRAM/DRAM overlap. Application SHA256: `50d47e4702087dad5fe4ca9510dbaca04a4825e8ea5b06d29f678c2468509335`.
- The hosted module is rebuilt from 0.11.3 and recorded with current shared sources/assets/web files by `browser/bundle-manifest.json`. CI checks this reviewed integrity record, then compiles and tests a fresh WASM module from the deployment revision before Pages publication. Different compiler versions can produce different module hashes.
- No physical 0.11.3 device observation is claimed here. Previous [0.11.2 validation](../validation/release-0.11.2.json) records device checks separately. Complete browser-flasher installation/migration remains Preview.

The updated insurance illustrations in the [main guide](../README.md#why-insurance-appears-automatically) are native renderer exports of synthetic sessions, not player saves.
