# Rust 0.10.0

Qualified and installed. Actual boot, ordinary play, Guide, fixed training/EV timing and private save/reboot/restore pass. Saved training totals are preserved.

## What changed from 0.8

**Session analysis** is available under **Session totals → Session analysis**. It records settled-round net, outcomes, observed dispersion, drawdown and history. Separate first-answer pages show accuracy and weak areas. Only complete Basic-policy replacement-model evidence contributes to the model-difference aggregate; correct Hi-Lo deviations are not penalized by a different model. Deposits and other seats' chips do not become gambling profit.

**Optional full resume** is under **Settings → Resume session**. It is off by default. Enable it to allow private idle checkpoints, then choose Resume on a later boot when a complete matching checkpoint is available. It restores the live mode, shoe/hands, pending question/help, immutable grades and session analysis. A normal fresh start remains Off/$1,000/minimum and two Blue taps into a hand.

**Save/recovery hardening** keeps a new result, answer or sound behind the public-save acknowledgment when protecting a resumable session. Private checkpoints cannot roll back newer scores. Interrupted writes, corruption, unavailable storage and multiple browser tabs have explicit handling. A deliberate fresh RAM-only escape discards the pending session rather than silently continuing an unacknowledged question.

**Audio gain** now shares the independently checked original fixed master/channel scalar and quantization path. Raw sound data is unchanged. This is a digital-gain equivalence claim, not a claim about analog speaker loudness or the former resampler.

## Review and corrections

- An older public-record fallback could resurrect an unanswered private count question after a newer graded record became corrupt. Resume now treats that public identity as unsafe, retaining surviving valid public progress without restoring the old question.
- A pending browser storage lock could briefly expose the new frame/status/feedback before the save finished. The gate now covers the whole pending acknowledgment, including the raw ABI, public Save and sound queue.
- A held/released button during a save failure could become an unintended long press after recovery. Failure/resume now cancel held controls and require neutral input.
- A long save/retry could consume a card/review's reading time. Visible output receives a fresh dwell after acknowledgment.
- A permanent storage failure needed an explicit playable escape, rather than endless retry. Fresh RAM-only recovery starts a new session with acknowledged progress and visible UNSAVED status.

- Actual resume reboot exposed a target stack overflow missed by the old main-only check. Recursive in-place decoding reduces the App decoder from 231,056 to 432 bytes. The linked audit checks 600 functions, including literal-loaded stack allocations; the final physical save/reboot/restore now passes.

## Deliberate limits

Checkpoints are cooperative idle saves, not guaranteed per-tap saves. A stale checkpoint can be refused while newer valid public scores are kept. Session statistics are descriptive; the IID-normal reference is not a forecast or a confidence interval for blackjack advantage. The table presets are complete generic supported combinations, not verified live named-casino offerings. No unsupported exact-composition EV or universal optimal betting ramp is implied.

The approved main table, card art, text size and quick controls are retained. Physical button feel, subjective colors and audible quality require physical observation; automated USB, native frame and DMA checks have a narrower scope.
