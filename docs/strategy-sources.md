# Rule-bound basic strategy (0.3)

Verified November 18, 2026. This release binds total-dependent advice to the six
named `ProfileId` presets in `rules.h`. The three-argument legacy API retains the
six-deck H17/DAS/late-surrender policy and its 219,650-case baseline. UI callers
must use `recommend(hand, dealerUp, legalMask, game.rules)`.

`validRules` must accept the rules before advice is produced. A preset ID cannot
be used to bless changed core rules; wager limits and safe penetration settings
are customizable without changing this total-dependent policy. Each accepted
binding has an explicit reference grid:

| Profile | Reference CSV suffix | Policy difference from SixH17 |
| --- | --- | --- |
| SixH17 | `6dh17-das-ls` | Baseline |
| SixS17 | `6ds17-das-ls` | S17 surrender and doubling cells |
| EightH17 | `8dh17-das-ls` | Same 4–8 deck chart cells |
| SixNoSurrender | `6dh17-das-nols` | Surrender falls back to ordinary play |
| SixNoDas | `6dh17-nodas-ls` | Pair decisions change; no post-split double |
| SixSixFive | `6dh17-das-ls-65` | Same nonnatural play chart; natural settlement differs |

All bindings use US peek, double any first two cards, four hands maximum, no
resplitting aces, one card to split aces, and no surrender after splitting. These
features are fixed by the engine rather than exposed as unsupported toggles.
Changing a natural payout does not create new decisions for a resolved natural.
Insurance/even money remain declined in these uncounted profiles.

## Primary references and transcription

[Wizard of Odds: four-to-eight-deck strategy](https://wizardofodds.com/games/blackjack/strategy/4-decks/)
is the chart/text source. The S17 grid supplies the baseline decisions. Its H17
amendments add surrender of 15/17 and 8,8 against an ace, double 11 against an ace,
soft 18 against 2, and soft 19 against 6. Its pair rules explicitly bind DAS: without
DAS, split 2/3 against 4–7, never split 4, and split 6 against 3–6. Unavailable splits
use the corresponding total; the special unavailable-ace policy doubles against 6
when possible and otherwise hits. Soft18/19 unavailable doubles stand.

[Wizard of Odds: strategy calculator](https://wizardofodds.com/games/blackjack/strategy/calculator/)
independently confirms 4–8 deck American/peek H17/S17, DAS/no-DAS and surrender
bindings. Its embedded `H17_2` and `S17_2` grids and `CellCode` rules were inspected
on the verification date. In `H17_2`, the 8,8 versus10 cell is `P`, including no-DAS;
versus ace it is `RP` (surrender else split). `RP` is unaffected by the DAS selector.
Therefore SixNoDas still splits8,8 against 10 when splitting is legal. Fetch SHA256:
`01414ed0149fa45a2b9cb6ac648adf961340b0a6d8041884bb55106a49738722`.
The transient HTML was inspected from `/tmp` and is not bundled into firmware.

[Wizard of Odds: surrender](https://wizardofodds.com/games/blackjack/surrender/)
cross-checks the hard-total H17/S17 late-surrender differences and the four/six-deck
8,8 exception: surrender against an H17 ace; preserve splitting against 9/10 and
an S17 ace. These decisions apply after the dealer's blackjack check. Unavailable
surrender uses hit for 15/16 or stand for 17, while an available split still takes
precedence for the pair cells that call for splitting.

The 2,160 main-chart cells across all six CSVs were also checked against the
fetched calculator arrays/selectors (`evidence/profile-strategy-source-audit.txt`).
The CSV files in `tests/data` hold independently checked chart facts; firmware
uses range/rule branches rather than reading them. The eight-deck and 6:5 grids
explicitly repeat their shared decisions instead of silently selecting another
profile at runtime. No EV tie, tolerance, numeric error or counting deviation is
claimed by this module. Each gradable recommendation has one acceptable action.

## Information and legality boundary

Advice receives visible hand, dealer-upcard ID 0–51, capability bits and declared
rules only. It has no access to hole/burn cards, shoe, RNG or future order.
`legalMask` only asks the engine which actions it permits. Capability bits cannot
select house rules. Advice additionally removes stale multi-card double/split/
surrender, nonpair splits, post-split surrender, no-DAS post-split doubles,
forbidden split-ace moves and actions on 21. Empty advice is ungradable.

Tests exhaust all 32 capability masks. For artificial masks omitting ordinary
play, a documented safety order chooses a permitted stand, hit, double, split or
surrender; these impossible-mask guards do not claim chart optimality. Live
bankroll and resplit limits use the engine mask. Hidden-state mutation tests
verify unchanged actions and explanations for each supported profile.

## Executable evidence

`tests/profile_strategy_test.cpp` passes 8,446,470 independent CSV/classification/
capability comparisons across all six profiles, every two-card identity, all
upcard values, three-card rank combinations, post-split hands, all 32 masks,
face/suit aliases and invalid bindings. Test hand arithmetic independently counts
aces as one before promoting a usable ace. Additional checks cover real bankroll
and four-hand restrictions, the no-DAS 8,8 versus 10 case, H17/S17 differences, safe
customized limits/penetration, and hidden information independence.

`tests/strategy_test.cpp` still passes the original 219,650 comparisons. Its only
0.3 compatibility edit changes array `.fill` to iterator `std::fill` for the
engine's dynamically sized six/eight-deck shoe.

```sh
clang++ -std=c++17 -O1 -g -fsanitize=address,undefined \
  -Wall -Wextra -Werror -Isrc tests/strategy_test.cpp src/strategy.cpp \
  -o /tmp/stardust-strategy-test
/tmp/stardust-strategy-test
clang++ -std=c++17 -O1 -g -fsanitize=address,undefined \
  -Wall -Wextra -Werror -Isrc tests/profile_strategy_test.cpp src/strategy.cpp \
  -o /tmp/stardust-profile-strategy-test
/tmp/stardust-profile-strategy-test
```

These checks establish conformance to the declared total-dependent chart policy.
They are software checks, not physical-device validation or an EV proof.
