# Session analysis

Rust 0.10 includes menu-only descriptive session reports and immutable first-answer metrics. Normal play keeps the approved minimal table.

## Using analysis

Hold Grey → Session totals → **Session analysis**. The menu offers session totals/dispersion, a net-history graph, settled round records, first-answer accuracy/weak areas, Basic-policy model EV differences, first-answer history and assumptions. Grey changes pages/categories/history; Blue returns. The normal table and two-button play controls are unchanged.

Every settled controlled-player round is captured once, after pending card visibility finishes. The original base wager is frozen at deal; total stake and return include actual double/split/insurance effects. Deposits and cashouts are ledger movements, not gambling wins or losses. Optional tablemates retain their independent chips and do not contaminate the controlled player's money report. Failed ledger validation stops additional round analysis and visibly labels prior retained reports partial; gameplay continues.

The 128-round history retains immutable settlement snapshots. Exact `i128` first/second-moment totals, net, stake, drawdown and streaks survive history rollover through one million rounds. Reports include observed mean, sample standard deviation, wins/losses/pushes, maximum net drawdown, largest **base** wager, best/worst round and losing streaks. The graph shows cumulative settled net, with funding excluded; it does not advance live RNG or simulate future outcomes.

Card first-answer accuracy records only live choices graded in Test/Coach/Guide, under the selected original Basic or Hi-Lo policy. Synthetic targeted drills retain their separate 0.8 scores. Wager-practice answers have separate session totals/categories; manual/no-score wagers do not invent exercise answers. New sessions clear these RAM metrics and money history without clearing saved lifetime trainer progress.

## Model differences are not realized losses

The EV aggregate includes **only Basic-policy first answers** with complete, validated `InfiniteReplacementV1` evidence. It converts the frozen action difference using the original wager and uses compensated summation. Assisted and unassisted samples remain separate. Hi-Lo decisions are explicitly skipped: a correct +3 insurance index must not become a supposed error because a different fixed-replacement model still values insurance negatively. Partial, invalid and missing snapshots are reported as unavailable, never as zero-difference observations.

A separate 128-entry first-answer history retains original visible hand/upcard, selected policy/count, category, original choice, selected-policy best choice, correctness, assistance, original wager and available model comparison. Coach corrections do not replace the original wrong choice or improve its original unassisted score. Policy grades and replacement-model values are distinct measures; a model difference is not the amount actually lost or a prediction of profit.

## Uncertainty scope

From 30 settled observations, a page can display a specifically labeled **IID-normal reference**, mean ± 1.96 × sample SD / √n. It is descriptive, not a confidence interval for blackjack advantage, a risk-of-ruin calculation or a forecast. Rounds within a finite shoe and adaptive wager sizes are dependent; reaching 30 observations does not establish independence. Small samples show no reference band.

The conventional estimated-variance mean interval uses a Student-t critical value, as shown in the [NIST mean-confidence reference](https://www.itl.nist.gov/div898/handbook/eda/section3/eda352.htm). The app deliberately does not present its fixed 1.96 reference as that interval, or claim that the underlying blackjack observations meet the required assumptions.

## Persistence and implementation

Public preferences and lifetime progress use **SBTR V10, 2636 bytes**, with exact V1–V9 migration. Normal boot starts fresh at $1,000/minimum/Off. Session analysis stays in RAM unless optional private resume is enabled and a complete matching checkpoint has been committed. Explicit resume restores its exact accumulators, rings and settlement-deduplication state. A new session clears session analysis without clearing lifetime trainer progress. See the [main guide](../README.md#save-and-resume) for checkpoint availability and recovery behavior.

`session_analysis` consumes only completed money ledgers. `session_metrics` consumes already-recorded immutable decisions and wager attempts. Neither backend receives an engine, shoe, hidden face, future order or RNG. All models provide const `EMPTY` images; the App retains safe static initialization for direct flash-to-PSRAM construction. Source changes are in `app.rs`, `screens.rs`, `wager_ui.rs`, `session_ui.rs`, `session_analysis.rs`, `session_metrics.rs` and module registration; the 0.8 math, engine, controls and public save codec are preserved.

## Validation

New coverage includes the eight independent root ledger/variance tests (1,000 pairwise variance oracles), eight first-answer evidence tests (including exact +3 Hi-Lo insurance exclusion, partial/unavailable cases, immutability, compensated aggregation and ring rollover), and eleven actual App tests for settlement/naturals, original wagers, funding, new-session reset, Coach correction, live Hi-Lo skip, tablemates, manual wager scoring, stopped-ledger behavior and menu/RNG purity.

`validation/session-screens/` contains real 240×135 RGB565 renderer captures. Summary, outcomes, mean, dispersion, graph and bankroll agree for the seeded 35-round losing session: 12 wins, 21 losses, 2 pushes, net −$45, ending bankroll $955. Negative realized net/mean use the approved red; positive values use green and zero uses gold. The minus sign is present in the approved font. Model EV differences retain their separate descriptive color/scope.

Run the native/pixel/math audit from the project directory:

```sh
python3 tools/check.py
```

Actual browser checks are under `browser/tests/`. Release evidence distinguishes mathematical/controller tests, browser interactions, linked firmware and actual device checks. Renderer captures do not establish physical button feel or speaker quality.
