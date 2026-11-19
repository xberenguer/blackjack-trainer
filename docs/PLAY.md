# Start playing

This is the quickstart for **Stardust Blackjack Rust 0.11.2**. It assumes you know regular blackjack and uses virtual chips. The [complete feature guide](FEATURE_GUIDE.md) explains every mode, exercise, report and setting; the [screen guide](SCREEN_GUIDE.md) shows the interface.

## Deal your first hand

1. A normal boot starts at the **player cut**, with **$1,000**, the table minimum selected and training **Off**.
2. Press **Blue** to accept the randomly positioned cut. Grey moves it if you want.
3. Press **Blue** again at **SHOE IS READY** to skip the burn notice and deal. If that notice has already cleared, Blue at the ready table deals.
4. On your turn, tap **Grey to Hit** or **Blue to Stand**. To Double, Split or Surrender, hold Grey and choose a legal action at the top of the menu.
5. After the result, Blue deals another hand. Grey opens wager selection instead.

The automatic insurance prompt against a dealer ace is an ordinary offer, not coaching. Grey changes decline/take and Blue confirms; a ten-up peek has no offer. [Why the prompt appears](FEATURE_GUIDE.md#why-insurance-appears-automatically).

## Learn the two buttons

| Context | Grey tap | Blue tap |
|---|---|---|
| Your turn | Hit | Stand |
| Between hands | Select wager | Deal |
| Menu | Next row | Select/toggle |
| Count quiz | Decrease answer | Increase answer |
| Volume/brightness editor | Lower | Raise |

**Hold Grey** for about 0.85 seconds to open the unified menu. Legal Double/Split/Surrender choices appear first, so the number of rows before Training can change. Grey advances through rows and wraps; Blue selects. A faded next-row peek indicates more choices below. Hold Grey again at the main menu to return to the previous context. Nested sound/display editors use hold Grey to go back to their parent.

**Hold Blue** cycles **Off → Test → Coach → Guide → Off**. Release after a hold does not also play a card. The Volume editor is the exception: hold Blue auditions a deal sound there.

**Both buttons together submit only count-answer quizzes.** Wager and deviation exercises use Blue on a selected answer. Release both buttons after boot/resume and after a chord. In the browser, Left Arrow is Grey and Right Arrow is Blue; hold them for the same shortcuts.

Allow dealing/tablemate card displays to finish before playing or starting a count question. To inspect exposed cards later, use **Review cards**. Hidden dealer/burn faces remain hidden.

## Pick a learning mode

| Mode | What happens |
|---|---|
| **Off** | Ordinary play, without automatic card-strategy grading/coaching. Normal boot starts here. |
| **Test** | Your first decisions are recorded; after the settled hand, Round review shows recommendations and mistakes. |
| **Coach** | A mistaken first choice pauses **before acting**. Choose Play my choice or Use best action; the original grade stays unchanged. |
| **Guide** | Recommended move/reason is visible on the table. Count aids default to RC+TC here; assisted learning stays separate from unassisted accuracy. |

The in-game **Controls** pages explain the mode shortcut, but do not include these full mode descriptions. See [mode examples](FEATURE_GUIDE.md#off-ordinary-play).

**Training → Strategy** selects **Basic** or **Hi-Lo**, independently of the mode. Hi-Lo is the new-install default; existing saved Basic selections and V1–V4 Basic saves are retained. Strategy changes are made between rounds. Hi-Lo adds the named published count-index deviations to the rule-specific Basic policy.

**Settings → Count display** is separate again: OFF, RC or RC+TC. Guide and the other modes have independent preferences. A visible answer/count, Hint or EV can make an attempt assisted. The first submitted answer is immutable; help is useful, but does not count as an independent correct answer. [Policies, count conventions and scoring](FEATURE_GUIDE.md#choose-teaching-policy-and-aids-separately).

Try Guide while learning, then Test with normal count display OFF for an independent check. If you want to study Basic specifically, select BASIC before starting.

## Find the feature you want

Hold Grey for the menu, then follow these paths:

| Goal | Path and reference |
|---|---|
| Change a bet | **Select wager**: presets or Custom wager. [Manual money controls](FEATURE_GUIDE.md#manual-wagers-and-virtual-funding). |
| Get a hint or review strategy | **Training → Hint / Review decisions / Accuracy**. [First-answer scoring](FEATURE_GUIDE.md#accuracy-and-first-answer-rules). |
| Practice keeping RC, estimating decks or converting TC | **Training → Count practice**. Grey−/Blue+, both submit. [Count practice](FEATURE_GUIDE.md#count-practice-three-separate-skills). |
| Practice a count-based example bet | **Training → Wager practice**. Opt-in predeal prompts; manual/no-score betting remains available. [Wager practice](FEATURE_GUIDE.md#wager-practice-example-ramp-arithmetic). |
| Practice individual count thresholds | **Training → Deviation drills**. Synthetic questions, given context, help, replay and separate totals. [Deviation drills](FEATURE_GUIDE.md#deviation-drills-focused-synthetic-decisions). |
| Compare mathematical action values | **Training → Action EV / EV review**. Fixed replacement-model expected net; viewing is assistance. [EV scope](FEATURE_GUIDE.md#action-ev-and-ev-review). |
| Change rules, limits or dealer cut | **Table profile**, between rounds; apply the pending profile. [Table setup](FEATURE_GUIDE.md#table-profile-and-rules). |
| Add other players or choose your seat | **Table profile → Other seats / your place**; apply seating/fresh shoe. [Tablemates](FEATURE_GUIDE.md#optional-tablemates-and-card-reviews). |
| Check shoe/dealt information | **Shoe information**; it is a count aid. [Cut and shoe](FEATURE_GUIDE.md#player-cut-dealer-marker-and-shoe-information). |
| Inspect current money and reports | **Session totals → Bankroll details / Session analysis**. [Analysis pages](FEATURE_GUIDE.md#session-totals-and-analysis). |
| Add virtual chips | **Settings → Add $1000 chips**, after the round; recorded as funding, not profit. |
| Finish and start another session | **Color up & leave → Cash out & leave → New $1000 session**. Lifetime trainer counters remain. |
| Adjust audio, brightness, flip or pacing | **Settings → Sound / Display / Speed**. [Settings](SETTINGS.md). |

The example wager ramp is 1/2/4/8/12 minimum-bet units, with table/bankroll caps. It is not an optimal betting system. Action EV uses an infinite-replacement model, not the live hidden shoe. Session reports describe observed play, not future advantage. The [trainer glossary](FEATURE_GUIDE.md#trainer-glossary) explains policy, RC/TC, index, assistance, checkpoints and EV terms.

## Sound, display and speed

New installs start with sound on at **20%**, all categories enabled and **Fast** speed. Saved preferences take precedence.

- **Sound:** master mute, Volume, independent Menus/Cards/Results categories, and Test deal/win/menu. In Volume, Grey lowers/Blue raises; 0–10% uses one-point steps, then five-point steps. Hold Blue auditions and hold Grey returns. Master mute/volume 0 silences tests; tests never turn sound on.
- **Display:** Brightness and Flip screen. Brightness uses about five-percentage-point steps, with a readable minimum; hold Grey returns.
- **Speed:** exactly Normal or Fast. Fast preserves visible counting faces; quizzes and reviews stay untimed. Choose Normal for slower automatic stages.

## Save and resume

Preferences and lifetime trainer scores are saved independently from a live session. **Resume session is OFF by default**. An ordinary reboot starts fresh at $1,000/minimum/Off while retaining saved preferences and scores.

To opt in, use **Settings → Resume session: ON**. After a matching private checkpoint is saved, the next boot offers **Resume saved session** or **Start a new session**. Blue selects Resume; Grey changes the choice and Blue confirms. Only explicit resume restores live mode, bankroll, shoe, questions/help, first answers, histories and reports. Release both buttons afterward.

Checkpoints happen cooperatively in idle time; every tap is not instantly resumable. Newer progress can invalidate an older checkpoint, which is refused rather than rolling a question or grade back. Browser storage is local to that profile/origin; clearing site data removes it, and device/browser saves do not synchronize automatically.

**SAVE RETRY** or a read-error choice needs Retry or a deliberate **Start fresh without saving**. That second choice discards the live pending session, uses acknowledged totals/preferences when known, starts Off/$1,000/minimum, displays **UNSAVED** and disables writes/resume until reboot. If the initial read failed, prior totals are unavailable rather than guessed. This RAM-only recovery leaves saved journals untouched, so reboot may still offer the last durable checkpoint. [Full save/recovery guide](RESUME.md).

For pictures of each context, open the [screen guide](SCREEN_GUIDE.md). For all features and examples, open the [complete feature guide](FEATURE_GUIDE.md).
