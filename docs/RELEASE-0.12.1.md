# 0.12.1 — Clearer menus and explicit resets

The main menu is reduced from **11 entries to 7**: Back to game, Bet, Training, Table, Session, Settings and Help. Legal Double/Split/Surrender actions still appear first during a hand. Table controls, financial session tools, learning features and device preferences now have separate homes.

- **Settings: 7 → 5 rows.** Display, Sound, Speed, Save/resume and Back. Funding moves to Session → Bankroll; count preferences move to Training → Live counts.
- **Sound: 9 → 5 rows.** Master sound, Volume, Categories, Test sounds and Back. The three category switches and three auditions each have their own submenu.
- **Training** groups live counts, practice, progress and advice/EV. Progress groups accuracy and history by hand play, counting, bet sizing and deviations. Normal-play and Guide count preferences are directly editable as separate rows.
- Submenus show **>**; toggles show their value. Holding Grey/right goes back one level and restores the parent selection. Teaching-mode help and session overview show page counters. The small grey version remains visible.

There is no casino cash-out/“Color up & leave” step in normal navigation. **Session → New session** starts at $1000 with a fresh shoe and clears financial session reports, retaining learning scores and preferences. **Training → Progress → Reset training stats** clears learning scores/histories while retaining the live bankroll, table, preferences and financial reports. Each opens a separate confirmation with **Cancel selected** and is restricted to between-hand use.

New session also clears the previous shoe's exposed-card tracker. Starting a new first shoe with the same number as the old first shoe can no longer retain the old running count or seen-card positions. Resetting learning scores retains consumed-token protection, so a previous answer cannot be scored again or recovered from an obsolete private checkpoint.

The shared menu definition drives both Stick and Waveshare renderers. Quick table controls, the centered round card fans, pitch-black OLED default, saved appearance choices, default Hi-Lo/Fast/20% volume and separate Guide count default remain intact. Menu navigation sounds respect mute, zero volume and category choices; sound auditions do not receive a second generic click.

[Every menu, with 68 screenshots and a walkthrough](MENUS.md) · [Complete feature guide](../README.md) · [Waveshare guide](WAVESHARE.md) · [Install/update](INSTALL.md)

## Compatibility and verification

Public Save **V12 remains 2,636 bytes**, with existing preferences and lifetime totals preserved on upgrade. New private view tags are appended; old view/row IDs retain their original meanings. Legacy cash-out/setting rows are kept only for safe interpretation of an older saved private screen, not ordinary menu navigation. The private body schema and game-state field order are unchanged.

[Scoped release evidence](../validation/release-0.12.1.json) separates native, browser, firmware and actual-device checks. Native tests exercise navigation, default Cancel, in-hand restrictions, score reset, old-token rejection, private round-trips and the new-session count reset. Browser acceptance includes normal/seated play, continuation faults, settings/audio and the menu hierarchy; a real Chrome sweep visits all eight session-report views and 54 detail pages.

Menu illustrations are actual Rust renderer frames from synthetic button-input sessions, not device photographs or player saves. Physical browser-flasher installation is a separate qualification from direct serial installation. No personal scores, device backups or private checkpoints are published.
