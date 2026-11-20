# Menus — 0.12.1

The menu is organized around the task you want to do. Hold **Grey** on Stick, or the **small right button** on Waveshare, to open it. Legal Double/Split/Surrender actions still come first during a hand.

**Next:** tap Grey/right. **Select:** tap Blue/large left. **Back one level:** hold Grey/right. The parent selection is remembered. Hold the same button from the main menu to return to play. Hold Blue/left at the table to change teaching modes without opening a menu.

A `>` means another screen opens. A row showing a value, such as **Speed: Fast**, changes that value. Lists retain the faded next-row preview: four complete rows on Stick and six on Waveshare.

![Actual menu walkthrough](media/menus/menu-walk-round.gif)

*Actual Rust frames from a synthetic button-input walkthrough: Main → Settings → Sound → Categories, then back through the same parents. This is not a hardware recording or a player save.*

## Main menu

| Entry | Purpose |
|---|---|
| Back to game | Return to the table, cut or review you left. |
| Bet | Choose a preset or custom bet between rounds. |
| Training | Teaching mode, policy, count aids, practice, learning progress and advice. |
| Table | Configure the table or inspect its rules, shoe and revealed cards. |
| Session | Current financial results, reports, bankroll and starting a new session. |
| Settings | Device presentation, sound, speed and save/resume. |
| Help | Controls and the four teaching-mode explanations. |

## Training

| Path | What it does |
|---|---|
| Mode | Off → Test → Coach → Guide. Hold Blue/left is the table shortcut. |
| Strategy | Basic or count-aware Hi-Lo; change between rounds. |
| Live counts | Separate Normal play and Guide mode HUD preferences: Off, RC, RC + TC. Count information is also available here. |
| Practice → Counting | Running count, remaining-deck estimate or true-count exercise. |
| Practice → Bet sizing | Optional automatic checkpoints, manual practice and the example bet ramp. |
| Practice → Deviations | Target, mix, rules and contextual Hi-Lo deviation practice. |
| Progress → Hand play / Counting / Bet sizing / Deviations | Accuracy and history for that learning activity. Scores are reached here rather than repeated throughout practice menus. |
| Progress → Reset training stats | Separate confirmation before clearing saved learning scores and histories. |
| Advice & EV | Hint, action EV and EV review. These retain their assistance rules and stated model limits. |

## Table, session and settings

| Path | What it does |
|---|---|
| Table → Table setup | Presets, limits, dealer cut and other-seat settings. Apply changes between rounds. |
| Table → Current rules | Read the active table rules. |
| Table → Shoe info | Inspect depth, remaining cards and shuffle state; this is a count aid. |
| Table → Review cards | Inspect already revealed cards. |
| Session → Overview | Current bankroll, rounds, net and win/loss/push totals. Grey/right changes page. |
| Session → Reports | Existing session analysis and retained round/decision evidence. |
| Session → Bankroll | Balance details and Add $1000. Deposits are separate from winnings. |
| Session → New session | Confirm a fresh $1000 session and shoe. |
| Settings → Display | Brightness and orientation; Waveshare also has background and Stardust-back colors. |
| Settings → Sound | Master sound and volume. Categories and Test sounds each have their own submenu. |
| Settings → Speed | Fast or Normal; Fast remains the default. |
| Settings → Save/resume | Opt-in exact private resume; default Off. Ordinary preferences and scores still save independently. |
| Help → Controls / Training modes | App controls and concise Off/Test/Coach/Guide explanations. Page counters show how many pages remain. |

## Two different reset actions

| Action | Clears | Keeps |
|---|---|---|
| Session → New session | Current bankroll/session results and shoe; starts at $1000 with a fresh shoe. | Table/seating, presentation preferences and lifetime learning scores. |
| Training → Progress → Reset training stats | Hand/count/bet/deviation scores and learning histories. | Current bankroll, table, preferences and financial session reports. |

Both are available between hands and open with **Cancel selected**. Merely opening a confirmation erases nothing. New-session counting starts from the fresh shoe. There is no cash-out or “Color up & leave” action in normal navigation.

## Every menu, on both displays

The following images come from the actual shared controller and renderers, using synthetic sessions. Stick previews enlarge exact 240×135 pixels by 3×; round examples are native 466×466. They do not contain personal saved scores. [Image/source provenance](menu-screens.json).

### Main menu

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Main menu on Stick](media/menus/stick/menu-main.png)](media/menus/stick/native/menu-main.png) | [![Main menu on Waveshare](media/menus/round/menu-main.png)](media/menus/round/menu-main.png) |

### Training

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Training on Stick](media/menus/stick/training.png)](media/menus/stick/native/training.png) | [![Training on Waveshare](media/menus/round/training.png)](media/menus/round/training.png) |

### Live counts

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Live counts on Stick](media/menus/stick/counts.png)](media/menus/stick/native/counts.png) | [![Live counts on Waveshare](media/menus/round/counts.png)](media/menus/round/counts.png) |

### Practice

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Practice on Stick](media/menus/stick/practice.png)](media/menus/stick/native/practice.png) | [![Practice on Waveshare](media/menus/round/practice.png)](media/menus/round/practice.png) |

### Counting practice

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Counting practice on Stick](media/menus/stick/count-practice.png)](media/menus/stick/native/count-practice.png) | [![Counting practice on Waveshare](media/menus/round/count-practice.png)](media/menus/round/count-practice.png) |

### Bet-sizing practice

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Bet-sizing practice on Stick](media/menus/stick/bet-practice.png)](media/menus/stick/native/bet-practice.png) | [![Bet-sizing practice on Waveshare](media/menus/round/bet-practice.png)](media/menus/round/bet-practice.png) |

### Deviation practice

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Deviation practice on Stick](media/menus/stick/deviation-practice.png)](media/menus/stick/native/deviation-practice.png) | [![Deviation practice on Waveshare](media/menus/round/deviation-practice.png)](media/menus/round/deviation-practice.png) |

### Progress

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Progress on Stick](media/menus/stick/progress.png)](media/menus/stick/native/progress.png) | [![Progress on Waveshare](media/menus/round/progress.png)](media/menus/round/progress.png) |

### Hand-play progress

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Hand-play progress on Stick](media/menus/stick/hand-progress.png)](media/menus/stick/native/hand-progress.png) | [![Hand-play progress on Waveshare](media/menus/round/hand-progress.png)](media/menus/round/hand-progress.png) |

### Advice and EV

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Advice and EV on Stick](media/menus/stick/advice.png)](media/menus/stick/native/advice.png) | [![Advice and EV on Waveshare](media/menus/round/advice.png)](media/menus/round/advice.png) |

### Table

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Table on Stick](media/menus/stick/table.png)](media/menus/stick/native/table.png) | [![Table on Waveshare](media/menus/round/table.png)](media/menus/round/table.png) |

### Session

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Session on Stick](media/menus/stick/session.png)](media/menus/stick/native/session.png) | [![Session on Waveshare](media/menus/round/session.png)](media/menus/round/session.png) |

### Session overview

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Session overview on Stick](media/menus/stick/session-overview.png)](media/menus/stick/native/session-overview.png) | [![Session overview on Waveshare](media/menus/round/session-overview.png)](media/menus/round/session-overview.png) |

### Bankroll

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Bankroll on Stick](media/menus/stick/bankroll.png)](media/menus/stick/native/bankroll.png) | [![Bankroll on Waveshare](media/menus/round/bankroll.png)](media/menus/round/bankroll.png) |

### Settings

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Settings on Stick](media/menus/stick/settings.png)](media/menus/stick/native/settings.png) | [![Settings on Waveshare](media/menus/round/settings.png)](media/menus/round/settings.png) |

### Sound

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Sound on Stick](media/menus/stick/sound.png)](media/menus/stick/native/sound.png) | [![Sound on Waveshare](media/menus/round/sound.png)](media/menus/round/sound.png) |

### Sound categories

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Sound categories on Stick](media/menus/stick/categories.png)](media/menus/stick/native/categories.png) | [![Sound categories on Waveshare](media/menus/round/categories.png)](media/menus/round/categories.png) |

### Test sounds

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Test sounds on Stick](media/menus/stick/test-sounds.png)](media/menus/stick/native/test-sounds.png) | [![Test sounds on Waveshare](media/menus/round/test-sounds.png)](media/menus/round/test-sounds.png) |

### Display

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Display on Stick](media/menus/stick/display.png)](media/menus/stick/native/display.png) | [![Display on Waveshare](media/menus/round/display.png)](media/menus/round/display.png) |

### Help

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Help on Stick](media/menus/stick/help.png)](media/menus/stick/native/help.png) | [![Help on Waveshare](media/menus/round/help.png)](media/menus/round/help.png) |

### Training modes: Off

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Training modes: Off on Stick](media/menus/stick/modes.png)](media/menus/stick/native/modes.png) | [![Training modes: Off on Waveshare](media/menus/round/modes.png)](media/menus/round/modes.png) |

### Training modes: Test

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Training modes: Test on Stick](media/menus/stick/modes-1.png)](media/menus/stick/native/modes-1.png) | [![Training modes: Test on Waveshare](media/menus/round/modes-1.png)](media/menus/round/modes-1.png) |

### Training modes: Coach

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Training modes: Coach on Stick](media/menus/stick/modes-2.png)](media/menus/stick/native/modes-2.png) | [![Training modes: Coach on Waveshare](media/menus/round/modes-2.png)](media/menus/round/modes-2.png) |

### Training modes: Guide

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Training modes: Guide on Stick](media/menus/stick/modes-3.png)](media/menus/stick/native/modes-3.png) | [![Training modes: Guide on Waveshare](media/menus/round/modes-3.png)](media/menus/round/modes-3.png) |

### New-session confirmation

| StickS3 | Waveshare 1.75C |
|---|---|
| [![New-session confirmation on Stick](media/menus/stick/new-session-confirm.png)](media/menus/stick/native/new-session-confirm.png) | [![New-session confirmation on Waveshare](media/menus/round/new-session-confirm.png)](media/menus/round/new-session-confirm.png) |

### Clear-training confirmation

| StickS3 | Waveshare 1.75C |
|---|---|
| [![Clear-training confirmation on Stick](media/menus/stick/reset-training-confirm.png)](media/menus/stick/native/reset-training-confirm.png) | [![Clear-training confirmation on Waveshare](media/menus/round/reset-training-confirm.png)](media/menus/round/reset-training-confirm.png) |
