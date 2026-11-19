> Historical integration notes from the pre-release 0.7 work copy. For current behavior and installation status, see the project README and player guide. References to future work below describe that earlier stage.

# Stardust Rust 0.7 integration prototype

This isolated, playable controller/library copy combines the accepted fresh Rust 0.5 core, the independently checked 0.6 extra-seat/deeper-shoe engine, and complete optional 0.7 example wager practice. It has not been installed on the physical Stick and is not a device-qualified release. The current release under `outputs/` is untouched by this prototype.

All gameplay, graphics, grading, wager math, controls and save migration are Rust. Approved bitmap/font/audio files are reused as data. There is no C++ runtime or hardware adapter.

## Optional wager practice

Training → **Wager practice** opens the feature. It defaults off; normal Grey Hit/Blue Stand, Grey between-round Wager, Blue Deal, hold-Grey unified menu and hold-Blue Off/Test/Coach/Guide cycle remain intact. Both-button submission remains exclusive to count quizzes.

Enable predeal prompts or start one checkpoint manually. The amount editor always shows the current chosen wager, bankroll and table limits. It supports $5/$25 changes, submitting a legal amount, declaring that the minimum cannot be funded, explicit help, manual play without scoring, and cancellation. Invalid wagers do not consume the first answer.

The versioned **EXAMPLE PRACTICE POLICY** uses table-minimum units of 1/2/4/8/12 for truncated physical TC bands ≤0/1/2/3/≥4. It is an arithmetic example, not an optimal ramp or prediction of advantage. Optional custom positive units are bounded and retained in each attempt. The standalone backend's separately labeled estimated-TC comparison never replaces physical-TC grading.

The target applies table, bankroll and $5-step caps; an unfunded minimum is explicit **No Bet / Refill**, never a $0 wager. Results/history show frozen exposed RC, truncated TC, original first choice, target, individual cap reasons and assistance. The submitted legal choice is preserved: Blue **Play choice** applies that choice, including a legal wrong wager, after checking that the live predeal context is unchanged. It does not silently apply the example answer. A refill adds a deposit and does not automatically deal.

Aggregate plus six category pages keep unassisted correct/errors separate from assisted attempts. A 128-attempt ring retains complete original rules, policy units, count, bankroll, choice and cap evidence. Help/history exposure at the same checkpoint stays assisted through retries, cancellation and score resets. Advancing the live context cancels an unanswered case; old references cannot authorize a wager against changed bankroll, rules or shoe. The normal Wager menu and manual/no-score bypass remain available.

## Inherited 0.6 controls

Table profile now includes seating, visible-table-card review and dealer markers through 90%. Up to six simulated tablemates use basic strategy and independent virtual-chip ledgers. The controlled seat may occupy any clockwise place. Zero extra seats preserves the accepted normal table pixels and default two-Blue fast deal.

NPC turns advance on the same nonblocking 650 ms cadence as dealer play. Their visible active cards have their own screen; all exposed table hands can be paged through the menu. Authoritative engine exposure tracking includes NPCs, excludes dealer holes, and prevents previously exposed mixed discards from contaminating a new count epoch. Initial-deal count observation waits for the progressive reveal to finish on the extended-shoe path.

If a round exceptionally exhausts its undealt pool, only discards are shuffled, held cards remain excluded, one unseen card burns, the count epoch resets, and the current round completes before a full shuffle. An explicit App notice requires acknowledgement, and Shoe Information/Rules describe the procedure. The engine tests include a legal ordered-shoe trajectory that reaches recovery, conservation checks, held-card exclusion, forgotten mixed exposures and hidden-hole gating. Cross-compilation does not establish physical acceptance of this procedure.

## Persistence and embedded construction

Public SBTR **V7 is 584 bytes**. It migrates V1–V6 exact integer scores and preferences; V1–V6 default wager practice off with 1/2/4/8/12 units and zero wager scores. V6's seat fields remain at 200/201. V7 adds enable at 332, policy ID at 333, reserved zero bytes 334–335, five little-endian `u32` units at 336–355, seven groups of `u64` counters at 356–579, and CRC32 at 580–583. Invalid dimensions, enum/flag values, units, counter totals, reserved bytes or CRC are rejected. Scores above 2^53 remain exact.

Only preferences and aggregate scores are saved. Shoe order, hidden faces, RNG, live questions and histories are not exported. Load remains boot-only and boots the card-training mode Off. Seat preferences and wager opt-in/units/scores are restored. A separate later private resume format must explicitly handle the additional controller/model fields.

`App::EMPTY` and each new model's const empty image preserve direct flash-to-PSRAM construction; firmware should copy the static image and call `boot(seed)` rather than put a large returned App on its stack. Firmware save buffers and browser ABI buffers must use `persistence::SIZE`, not the former hardcoded 336 bytes, when this prototype is merged.

## Validation and next integration

`validation/final-native/` records the reproducible native test, strict all-feature Clippy, example, reference pixel/state and exact EV oracle checks. `validation/wasm-library.log` records the allocation-free WASM library build. `validation/screens/` contains real RGB565 renderer captures for the wager screens, seating, active NPC cards and table-card review; the contact sheet was visually inspected for overlap/legibility. `tests/app_wager.rs` covers actual controller paths, stale-context guards, help, manual bypass, immutable wrong-choice play, saved-seat application, all seven controlled seat positions, exact migration and exposed-count binding. `tests/wager_model.rs` includes 667,392 independent rational-interval/legal-candidate oracle cases.

For the current checkout, reproduce the native checks from the repository root:

```sh
python3 tools/check.py --logs validation/latest-native
```

This copy is the base for isolated 0.8 drill UI/save integration. Root owns release merging, final browser/device validation, audio/display/button checks, recovery packaging and installation. Native tests and renderer captures do not claim that a 0.7 firmware is installed or physically accepted.
