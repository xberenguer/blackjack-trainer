# 0.12.0 — Blackjack on the round Waveshare

The Waveshare **ESP32-S3-Touch-AMOLED-1.75C** now runs the same Rust game and trainers as StickS3, with its own native **466 × 466** circular layout. Hold both buttons at the top: large left selects/stands; small right advances/hits. Hold right for the unified menu and hold left to cycle Off, Test, Coach and Guide. **Touch is disabled**, including startup and menus; enclosure orientation is 270°.

The whole player and dealer hand re-centers horizontally as cards arrive. Cards keep the same size and vertical position through draws, dealer play and results. Long hands overlap evenly, with an explicit earlier-card count if a fan exceeds its display limit. Your hand sits centrally, dealer cards above; Guide advice, RC/TC, bankroll and wager sit below. The ready-screen wager hint has its own space and disappears during play.

OLED defaults to **Black**. Settings → Display offers saved Black/Blue/Green/Red backgrounds and Gold/Red/Blue versions of the Stardust card back. Faces retain their original design. Round menus show six full rows plus a faded next choice; advancing the selection scrolls one row without a side scrollbar. The small grey version remains in the menu.

[Illustrated round-device guide](WAVESHARE.md) · [Install/update](INSTALL.md) · [Complete shared feature guide](../README.md)

The round build also uses a larger audio DMA buffer to cover native rendering and settings-save work. The repeated appearance/settings stress check passes without audio underruns; Stick audio buffering is unchanged.

## Saves and other versions

Public Save V12 remains **2,636 bytes**. Old preferences and training scores migrate exactly; V11 backgrounds are retained, and older saves receive Gold backs. The private body schema is unchanged, preserving exact old public bindings until an explicitly resumed checkpoint commits the new format. Old Display Return keeps its semantic ID. [Format and compatibility details](RESUME.md#round-display-preferences-and-older-saves).

StickS3 and the hosted web game retain the approved 240×135 blue interface and original cards. Browser checks compare fresh Cut, Burn, initial hand, Test, Coach and Guide pixels to the pre-round build. Rules, count-aware Hi-Lo guidance, grading, EV models and reports are shared rather than independently reimplemented.

## Installation and verification scope

The release contains separate, source-bound StickS3 and Waveshare applications. The signed flasher consumes both device entries from this release. Waveshare writes **only the application at `0x200000`**, below the reserved save tail; its factory partition table, bootloader, NVS/calibration and model/storage filesystems are retained. The supplied direct installer backs up overwritten sectors and verifies the write. Public release assets never contain device backups or player saves.

[Scoped validation evidence](../validation/release-0.12.0.json) records native, browser, firmware and actual-device checks separately. The round guide's 48 examples are real Rust renderer exports of synthetic demonstration states. They are not hardware photographs or player results. Browser-flasher physical installation remains a separate qualification from the verified direct serial installation; digital audio/DMA checks do not establish acoustic quality.
