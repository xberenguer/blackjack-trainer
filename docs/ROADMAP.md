# Release roadmap

The accepted 0.1.3 appearance and controls remain the design contract. The game was rewritten throughout in Rust; the old implementation is a validation/reference artifact, not a runtime dependency. Version numbers continue forward.

| Version | Delivered behavior |
|---|---|
| 0.1.0–0.1.3 | Historical playable C++ table, approved blue minimal UI, readable cards/balance/bet and quick controls. |
| 0.2 | Historical optional Test/Coach, automatic after-hand review and saved accuracy. |
| 0.3.0–0.3.1 | Historical Guide, unified Grey menu, hold-Blue mode cycle, adjustable random player cut and independent Guide RC/TC default. |
| 0.4 | Explicit infinite-replacement action EV, bounded funded split EV and immutable review evidence. |
| 0.5.0 | Fresh Rust catch-up, verified device play, visible version, Hi-Lo index guidance/grading, count checkpoints, exact legacy migration. |
| 0.6 | Up to six tablemates, complete exposed-card sequences, deeper dealer markers and discard-only shoe-exhaustion recovery. Delivered with 0.8. |
| 0.7 | Optional count-based wager exercise, example ramp, frozen first choices and legal limits/bankroll caps. Delivered with 0.8. |
| 0.8.0 | 31 targeted deviation/insurance drills, public contexts, explicit help, missed replay and distinct immutable progress. Device installed and main play paths checked. |
| 0.9 | Menu-only settled session/history, dispersion/drawdown and first-answer/model reports. Qualified natively and in the browser; combined with the next device update. |
| 0.10.0 | Opt-in exact private resume, save acknowledgment gates, corruption/retry recovery, audio gain correction, memory/performance hardening and complete player documentation. Qualified and installed; actual play, timing and private save/reboot/restore passed. |

| 0.10.2 | Selected B menu: four full rows with a faded next-row peek, small grey version and LED-off readback. |
| 0.11.0 | Amplifier GPIO setup fix; percentage volume, separate sound categories and auditions; display editor and Normal/Fast speed. |

| 0.11.1 | Default volume20% and Fast speed. Existing explicit V10 preferences remain saved; connected device updated to the requested choices. |

| 0.11.2 | Hi-Lo strategy defaults on new installations; connected device switched to Hi-Lo. Existing saved strategy selections and historical Basic-only saves are preserved. |

| 0.11.3 | Guide insurance choices remain visible; illustrated main guide, hosted web game and signed flasher release contract. |
| 0.12.0 | Rust Waveshare 1.75C port: native round layout, top physical buttons, touch disabled, centered card fans, fixed card size/height, OLED backgrounds and Stardust back colors, six-row menus with a next-row preview. |

| 0.12.1 | Grouped menus, consistent Back, clear mode help, explicit session/stat confirmations, safe learning reset and fresh-shoe count reset. |
| 0.12.3 | Green and Red backgrounds on StickS3 and a Charcoal card back on both devices. |
| 0.12.2 | StickS3 Black background and card-back colors, live card-back preview on both displays and side-by-side game screenshots. |

## How feedback shaped the result

- **Cards first.** Large approved faces and text remain; the main logo, chip stacks, redundant “Chips” label, automatic totals, shoe details and action-button row were removed from normal play.
- **Two buttons, quick entry.** Two Blue taps reach a hand. Grey hits, Blue stands; hold Grey opens the unified menu and hold Blue changes the help mode. Legal special actions come first. Scrolling is visible.
- **Optional help.** Off remains a plain simulator. Guide explains the selected policy and defaults to RC+TC; other modes have independent count preferences. Test reviews automatically after a hand.
- **Realistic dealing.** Player cut, dealer marker, burn, finite shoe, rule-bound actions, exact payouts, tablemates and visible-card counting are explicit. Complete supported rules are documented rather than implied by a casino name.
- **Trustworthy practice.** First answers and original assistance/policy/model evidence cannot be rewritten by help, correction, replay, cancellation or resume.
- **Readable details on demand.** Rules, counts, EV, history and session analysis have dedicated menu pages rather than squeezing more text onto the cards.

## Completion criteria

Rust 0.10 completes this defined trainer roadmap. The source, browser, firmware, actual device qualification and recovery artifacts are recorded in the final requirement audit. Native/math/browser success is reported separately from actual USB/device observations and subjective physical acceptance.

The release does not claim exact composition-dependent EV, a universal optimal betting ramp, a casino-profit forecast, unsupported table variants, live named-casino rule verification or guaranteed per-tap resume. Those require separately defined work. No additional numbered version is needed merely to fill a ten-version list.
