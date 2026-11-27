//! Shared hierarchy and semantic row IDs for both displays. Legacy IDs retain
//! their original actions when a private checkpoint reopens an older menu.
use super::*;
use crate::{
    render::{Align, TextStyle},
    text::{Cash, Text},
    widgets::{WHITE, Widgets},
};
use core::fmt::Write;
pub(crate) struct MenuSpec {
    pub title: &'static str,
    pub subtitle: Option<&'static str>,
    pub body: Option<&'static str>,
    pub details: Text<256>,
    pub items: [Text<64>; 16],
    pub len: usize,
    pub selected: usize,
    pub footer: &'static str,
    pub confirmation: bool,
    ids: [usize; 16],
}
impl MenuSpec {
    fn new(title: &'static str) -> Self {
        Self {
            title,
            subtitle: None,
            body: None,
            details: Text::new(),
            items: [const { Text::new() }; 16],
            len: 0,
            selected: 0,
            footer: "GREY: NEXT   BLUE: SELECT",
            confirmation: false,
            ids: [0; 16],
        }
    }
    fn row(&mut self, id: usize, label: &str, focus: usize) {
        if self.len == 16 {
            return;
        }
        let _ = self.items[self.len].write_str(label);
        self.ids[self.len] = id;
        if id == focus {
            self.selected = self.len;
        }
        self.len += 1;
    }
}
impl App {
    pub(crate) fn menu_spec(&self) -> Option<MenuSpec> {
        spec(self)
    }
}
fn labels(view: View) -> Option<&'static [(usize, &'static str)]> {
    Some(match view {
        View::Training => &[
            (0, "Mode"),
            (1, "Strategy"),
            (11, "Live counts"),
            (12, "Practice"),
            (13, "Progress"),
            (14, "Advice & EV"),
            (10, "Back"),
        ],
        View::Settings => &[
            (0, "Display"),
            (1, "Sound"),
            (2, "Speed"),
            (5, "Save/resume"),
            (6, "Back"),
        ],
        View::Sound => &[
            (0, "Master sound"),
            (1, "Volume"),
            (9, "Categories"),
            (10, "Test sounds"),
            (8, "Back"),
        ],
        View::TableMenu => &[
            (0, "Table setup"),
            (1, "Current rules"),
            (2, "Shoe info"),
            (3, "Review cards"),
            (4, "Back"),
        ],
        View::SessionMenu => &[
            (0, "Overview"),
            (1, "Reports"),
            (2, "Bankroll"),
            (3, "New session"),
            (5, "Back"),
        ],
        View::BankrollMenu => &[(0, "Balance details"), (1, "Add $1000"), (2, "Back")],
        View::HelpMenu => &[(0, "Controls"), (1, "Training modes"), (2, "Back")],
        View::LiveCounts => &[
            (0, "Normal play"),
            (1, "Guide mode"),
            (2, "Count info"),
            (3, "Back"),
        ],
        View::PracticeMenu => &[
            (0, "Counting"),
            (1, "Bet sizing"),
            (2, "Deviations"),
            (3, "Back"),
        ],
        View::ProgressMenu => &[
            (0, "Hand play"),
            (1, "Counting"),
            (2, "Bet sizing"),
            (3, "Deviations"),
            (5, "Reset training stats"),
            (4, "Back"),
        ],
        View::HandProgress
        | View::CountProgress
        | View::WagerProgress
        | View::DeviationProgress => &[(0, "Accuracy"), (1, "History"), (2, "Back")],
        View::AdviceMenu => &[(0, "Hint"), (1, "Action EV"), (2, "EV review"), (3, "Back")],
        View::SoundCategories => &[(0, "Menus"), (1, "Cards"), (2, "Results"), (3, "Back")],
        View::SoundTests => &[(0, "Deal"), (1, "Win"), (2, "Menu"), (3, "Back")],
        View::CountPractice => &[
            (0, "Running count"),
            (1, "Decks remaining"),
            (2, "True count"),
            (6, "Back"),
        ],
        View::WagerPractice => &[
            (0, "Automatic questions"),
            (1, "Practice now"),
            (4, "Bet ramp"),
            (5, "Back"),
        ],
        View::DeviationPractice => &[
            (0, "Practice now"),
            (1, "Target"),
            (2, "Mix"),
            (3, "Rules"),
            (5, "Replay last mistake"),
            (7, "Policy"),
            (8, "Back"),
        ],
        View::Display => return None,
        _ => return None,
    })
}
fn title(view: View) -> &'static str {
    match view {
        View::Menu => "MENU",
        View::Training => "TRAINING",
        View::Settings => "SETTINGS",
        View::Sound => "SOUND",
        View::TableMenu => "TABLE",
        View::SessionMenu => "SESSION",
        View::SessionOverview => "SESSION OVERVIEW",
        View::BankrollMenu => "BANKROLL",
        View::HelpMenu => "HELP",
        View::TrainingModes => "TRAINING MODES",
        View::LiveCounts => "LIVE COUNTS",
        View::PracticeMenu => "PRACTICE",
        View::ProgressMenu => "PROGRESS",
        View::HandProgress => "HAND PLAY",
        View::CountProgress => "COUNTING",
        View::WagerProgress => "BET SIZING",
        View::DeviationProgress => "DEVIATIONS",
        View::AdviceMenu => "ADVICE & EV",
        View::SoundCategories => "SOUND CATEGORIES",
        View::SoundTests => "TEST SOUNDS",
        View::CountPractice => "COUNTING PRACTICE",
        View::WagerPractice => "BET SIZING",
        View::DeviationPractice => "DEVIATION PRACTICE",
        View::Display => "DISPLAY",
        View::NewSessionConfirm => "NEW SESSION?",
        View::ResetStatsConfirm => "RESET TRAINING STATS?",
        _ => "MENU",
    }
}
fn root_label(id: usize) -> &'static str {
    match id {
        0 => "Back to game",
        1 => "Bet",
        2 => "Training",
        3 => "Settings",
        11 => "Table",
        12 => "Session",
        13 => "Help",
        _ => MENU.get(id).copied().unwrap_or(""),
    }
}
fn legacy_label(view: View, id: usize) -> Option<&'static str> {
    match view {
        View::Training => [
            "Mode",
            "Strategy",
            "Count practice",
            "Accuracy",
            "Review decisions",
            "Action EV",
            "EV review",
            "Hint",
            "Wager practice",
            "Deviation drills",
            "Back",
        ]
        .get(id)
        .copied(),
        View::Settings => [
            "Display",
            "Sound",
            "Speed",
            "Add $1000 chips",
            "Count display",
            "Save/resume",
            "Back",
        ]
        .get(id)
        .copied(),
        View::Sound => [
            "Master sound",
            "Volume",
            "Menus",
            "Cards",
            "Results",
            "Test deal",
            "Test win",
            "Test menu",
            "Back",
        ]
        .get(id)
        .copied(),
        View::CountPractice => [
            "Running count",
            "Decks remaining",
            "True count",
            "Count info",
            "Accuracy",
            "History",
            "Back",
        ]
        .get(id)
        .copied(),
        View::WagerPractice => [
            "Automatic questions",
            "Practice now",
            "Accuracy",
            "History",
            "Bet ramp",
            "Back",
        ]
        .get(id)
        .copied(),
        View::DeviationPractice => [
            "Practice now",
            "Target",
            "Mix",
            "Rules",
            "Accuracy",
            "Replay last mistake",
            "History",
            "Policy",
            "Back",
        ]
        .get(id)
        .copied(),
        _ => None,
    }
}
pub(crate) fn focused_label(a: &App) -> Option<&'static str> {
    if a.view == View::Menu {
        let (actions, n) = a.extras();
        return Some(if a.selected < n {
            table::action_name(actions[a.selected])
        } else {
            root_label(a.selected - n)
        });
    }
    if a.view == View::Display {
        return Some(if a.round_display {
            [
                "Brightness",
                "Flip screen",
                "Background",
                "Back",
                "Card back",
            ]
            .get(a.selected)
            .copied()
            .unwrap_or("")
        } else {
            [
                "Brightness",
                "Flip screen",
                "Back",
                "Background",
                "Card back",
            ]
            .get(a.selected)
            .copied()
            .unwrap_or("")
        });
    }
    if let Some(rows) = labels(a.view) {
        return rows
            .iter()
            .find(|(id, _)| *id == a.selected)
            .map(|(_, s)| *s)
            .or_else(|| legacy_label(a.view, a.selected));
    }
    if matches!(a.view, View::NewSessionConfirm | View::ResetStatsConfirm) {
        return Some(if a.selected == 0 {
            "Cancel"
        } else if a.view == View::NewSessionConfirm {
            "Start new session"
        } else {
            "Clear training stats"
        });
    }
    None
}
fn submenu(a: &App, id: usize) -> bool {
    match a.view {
        View::Menu => id
            .checked_sub(a.extras().1)
            .is_some_and(|id| matches!(id, 1 | 2 | 3 | 11 | 12 | 13)),
        View::Training => matches!(id, 11..=14),
        View::Settings => matches!(id, 0 | 1),
        View::Sound => matches!(id, 1 | 9 | 10),
        View::Display => id == 0,
        View::TableMenu => id < 4,
        View::SessionMenu => id < 5,
        View::BankrollMenu => id == 0,
        View::HelpMenu | View::SoundCategories | View::SoundTests => {
            a.view == View::HelpMenu && id < 2
        }
        View::LiveCounts => id == 2,
        View::PracticeMenu => id < 3,
        View::ProgressMenu => id < 4 || id == 5,
        View::HandProgress
        | View::CountProgress
        | View::WagerProgress
        | View::DeviationProgress => id < 2,
        View::AdviceMenu => id < 3,
        View::CountPractice => id < 3,
        View::WagerPractice => matches!(id, 1 | 4),
        View::DeviationPractice => matches!(id, 0 | 1 | 5 | 7),
        _ => false,
    }
}
fn spec(a: &App) -> Option<MenuSpec> {
    let mut out = spec_inner(a)?;
    for row in 0..out.len {
        if submenu(a, out.ids[row]) {
            let _ = out.items[row].write_str(" >");
        }
    }
    Some(out)
}
fn spec_inner(a: &App) -> Option<MenuSpec> {
    let mut out = MenuSpec::new(title(a.view));
    if a.view == View::Menu {
        let (actions, n) = a.extras();
        for (id, action) in actions.iter().take(n).enumerate() {
            out.row(id, table::action_name(*action), a.selected);
        }
        for id in [0, 1, 2, 11, 12, 3, 13] {
            out.row(n + id, root_label(id), a.selected);
        }
        if !out.ids[..out.len].contains(&a.selected)
            && a.selected >= n
            && a.selected < n + MENU.len()
        {
            out.row(a.selected, root_label(a.selected - n), a.selected);
        }
        return Some(out);
    }
    if matches!(a.view, View::NewSessionConfirm | View::ResetStatsConfirm) {
        out.confirmation = true;
        out.body = Some(if a.view == View::NewSessionConfirm {
            "Start with $1000 and a fresh shoe. Clear session reports. Keep training scores and settings."
        } else {
            "Clear hand, count, bet and deviation scores and learning history. Keep bankroll, table, settings and session reports."
        });
        out.row(0, "Cancel", a.selected);
        out.row(
            1,
            if a.view == View::NewSessionConfirm {
                "Start new session"
            } else {
                "Clear training stats"
            },
            a.selected,
        );
        return Some(out);
    }
    if matches!(a.view, View::SessionOverview | View::TrainingModes) {
        out.footer = "GREY: PAGE   BLUE: BACK";
        out.subtitle = Some(if a.view == View::SessionOverview {
            if a.page.is_multiple_of(2) {
                "1 / 2"
            } else {
                "2 / 2"
            }
        } else {
            ["1 / 4", "2 / 4", "3 / 4", "4 / 4"][a.page % 4]
        });
        if a.view == View::SessionOverview {
            if a.page.is_multiple_of(2) {
                let summary = a.session_analysis.summary();
                let _ = write!(
                    out.details,
                    "Bankroll {}. Played {} rounds. Net {}. Staked {}. Returned {}.",
                    Cash(a.game.bankroll, false),
                    a.game.rounds,
                    Cash(a.game.net(), true),
                    Cash(summary.total_staked as i64, false),
                    Cash((summary.total_staked + summary.net) as i64, false)
                );
            } else {
                let _ = write!(
                    out.details,
                    "Wins {}. Losses {}. Pushes {}. Reports include completed rounds only.",
                    a.game.wins, a.game.losses, a.game.pushes
                );
            }
        }
        out.body = if a.view == View::TrainingModes {
            Some(match a.page % 4 {
                0 => {
                    "Off: regular blackjack with no coaching. Hold BLUE at the table to change training mode."
                }
                1 => {
                    "Test: play independently. After each hand, review your mistakes and the recommended actions."
                }
                2 => {
                    "Coach: mistakes pause before the play. Review the recommendation, then keep your choice or use the best move."
                }
                _ => {
                    "Guide: live best-move advice and reasons, with RC and TC shown by default. Advice follows your selected Basic or Hi-Lo strategy."
                }
            })
        } else {
            None
        };
        return Some(out);
    }
    if a.view == View::Display {
        let mut b = Text::<64>::new();
        let _ = write!(b, "Brightness: {}%", a.brightness_percent());
        out.row(0, b.as_str(), a.selected);
        out.row(
            1,
            if a.flipped {
                "Screen: Flipped"
            } else {
                "Screen: Normal"
            },
            a.selected,
        );
        if a.round_display {
            b.clear();
            let _ = write!(
                b,
                "Background: {}",
                crate::canvas::BACKGROUND_NAMES[a.background.min(3) as usize]
            );
            out.row(2, b.as_str(), a.selected);
        } else {
            b.clear();
            let _ = write!(
                b,
                "Background: {}",
                crate::canvas::BACKGROUND_NAMES[a.compact_background.min(3) as usize]
            );
            out.row(3, b.as_str(), a.selected);
        }
        b.clear();
        let _ = write!(
            b,
            "Card back: {} >",
            crate::render::CARD_BACK_NAMES[a.card_back.min(3) as usize]
        );
        out.row(4, b.as_str(), a.selected);
        out.row(if a.round_display { 3 } else { 2 }, "Back", a.selected);
        return Some(out);
    }
    let rows = labels(a.view)?;
    for &(id, label) in rows {
        let mut b = Text::<64>::new();
        match (a.view, id) {
            (View::Training, 0) => {
                let _ = write!(b, "Mode: {}", a.training.mode.name());
            }
            (View::Training, 1) => {
                let _ = write!(
                    b,
                    "Strategy: {}",
                    if a.policy == Policy::HiLo {
                        "Hi-Lo"
                    } else {
                        "Basic"
                    }
                );
            }
            (View::Settings, 2) => {
                let _ = write!(b, "Speed: {}", if a.fast_mode { "Fast" } else { "Normal" });
            }
            (View::Settings, 5) => {
                let _ = write!(
                    b,
                    "Save/resume: {}",
                    if a.resume_enabled { "On" } else { "Off" }
                );
            }
            (View::Sound, 0) => {
                let _ = write!(
                    b,
                    "Master sound: {}",
                    if a.sound_enabled { "On" } else { "Off" }
                );
            }
            (View::Sound, 1) => {
                let _ = write!(b, "Volume: {}%", a.audio_volume);
            }
            (View::SoundCategories, 0..=2) => {
                let _ = write!(
                    b,
                    "{}: {}",
                    label,
                    if a.audio_mask & (1 << id) != 0 {
                        "On"
                    } else {
                        "Off"
                    }
                );
            }
            (View::LiveCounts, 0 | 1) => {
                let setting = if id == 0 {
                    a.count_display
                } else {
                    a.guide_count_display
                };
                let _ = write!(
                    b,
                    "{}: {}",
                    label,
                    ["Off", "RC", "RC + TC"][setting.min(2) as usize]
                );
            }
            (View::DeviationPractice, 1) => {
                let _ = write!(
                    b,
                    "Target: {}",
                    crate::deviation_drills::IndexId::from_id(a.drill_target)
                        .map_or("Mixed", crate::deviation_drills::IndexId::name)
                );
            }
            (View::DeviationPractice, 2) => {
                let _ = write!(
                    b,
                    "Mix: {}",
                    ["Mixed", "Below", "At", "Above"][a.drill_mix.min(3) as usize]
                );
            }
            (View::DeviationPractice, 3) => {
                let _ = write!(b, "Rules: {}", PROFILES[a.drill_profile as usize]);
            }
            (View::WagerPractice, 0) => {
                let _ = write!(
                    b,
                    "Automatic questions: {}",
                    if a.wager_enabled { "On" } else { "Off" }
                );
            }
            _ => {
                let _ = b.write_str(label);
            }
        }
        out.row(id, b.as_str(), a.selected);
    }
    if !out.ids[..out.len].contains(&a.selected)
        && let Some(label) = legacy_label(a.view, a.selected)
    {
        out.row(a.selected, label, a.selected);
    }
    Some(out)
}
fn go(a: &mut App, view: View, selected: usize) {
    a.set(view);
    a.selected = selected;
}
fn root(a: &mut App, id: usize) {
    let n = a.extras().1;
    go(a, View::Menu, n + id);
}
fn note(a: &mut App, text: &'static str, back: View, selected: usize) {
    a.notice(text, back);
    a.page = selected;
}
fn parent(a: &App) -> Option<(View, usize)> {
    Some(match a.view {
        View::Training => (View::Menu, a.extras().1 + 2),
        View::Settings => (View::Menu, a.extras().1 + 3),
        View::TableMenu => (View::Menu, a.extras().1 + 11),
        View::SessionMenu => (View::Menu, a.extras().1 + 12),
        View::HelpMenu => (View::Menu, a.extras().1 + 13),
        View::LiveCounts => (View::Training, 11),
        View::PracticeMenu => (View::Training, 12),
        View::ProgressMenu => (View::Training, 13),
        View::AdviceMenu => (View::Training, 14),
        View::Sound => (View::Settings, 1),
        View::Display => (View::Settings, 0),
        View::AudioVolume => (View::Sound, 1),
        View::Brightness => (View::Display, 0),
        View::CardBackPreview => (View::Display, 4),
        View::SoundCategories => (View::Sound, 9),
        View::SoundTests => (View::Sound, 10),
        View::SessionOverview => (View::SessionMenu, 0),
        View::BankrollMenu => (View::SessionMenu, 2),
        View::Bankroll => (View::BankrollMenu, 0),
        View::NewSessionConfirm => (View::SessionMenu, 3),
        View::ResetStatsConfirm => (View::ProgressMenu, 5),
        View::TrainingModes => (View::HelpMenu, 1),
        View::Controls => (View::HelpMenu, 0),
        View::Profile => (View::TableMenu, 0),
        View::ProfilePreset => (View::Profile, 0),
        View::ProfileAmount => (View::Profile, if a.minimum_edit { 1 } else { 2 }),
        View::ProfileDepth => (View::Profile, 3),
        View::TableSeats => (View::Profile, 4),
        View::Rules => (View::TableMenu, 1),
        View::ShoeInfo => (View::TableMenu, 2),
        View::Cards | View::SeatCards => (View::TableMenu, 3),
        View::CountPractice => (View::PracticeMenu, 0),
        View::CountAnswer | View::CountResult => (View::CountPractice, a.count_kind as usize),
        View::CountInfo => (View::LiveCounts, 2),
        View::WagerPractice => (View::PracticeMenu, 1),
        View::WagerAnswer | View::WagerResult => (View::WagerPractice, 1),
        View::WagerHelp => (View::WagerAnswer, 6),
        View::WagerPolicy => (View::WagerPractice, 4),
        View::DeviationPractice => (View::PracticeMenu, 2),
        View::DeviationQuestion | View::DeviationResult => (View::DeviationPractice, 0),
        View::DeviationTarget => (View::DeviationPractice, 1),
        View::DeviationPolicy => (View::DeviationPractice, 7),
        View::DeviationContext | View::DeviationReference => {
            (View::DeviationQuestion, a.drill_choice)
        }
        View::HandProgress => (View::ProgressMenu, 0),
        View::CountProgress => (View::ProgressMenu, 1),
        View::WagerProgress => (View::ProgressMenu, 2),
        View::DeviationProgress => (View::ProgressMenu, 3),
        View::TrainingStats => (View::HandProgress, 0),
        View::TrainingReview => (View::HandProgress, 1),
        View::CountStats => (View::CountProgress, 0),
        View::CountHistory => (View::CountProgress, 1),
        View::WagerStats => (View::WagerProgress, 0),
        View::WagerHistory => (View::WagerProgress, 1),
        View::DeviationStats => (View::DeviationProgress, 0),
        View::DeviationHistory => (View::DeviationProgress, 1),
        View::Hint => (View::AdviceMenu, 0),
        View::ActionEv => (View::AdviceMenu, 1),
        View::EvReview => (View::AdviceMenu, 2),
        View::AnalysisMenu => (View::SessionMenu, 1),
        View::AnalysisSummary => (View::AnalysisMenu, 0),
        View::AnalysisGraph => (View::AnalysisMenu, 1),
        View::AnalysisRounds => (View::AnalysisMenu, 2),
        View::AnalysisAccuracy => (View::AnalysisMenu, 3),
        View::AnalysisCosts => (View::AnalysisMenu, 4),
        View::AnalysisDecisions => (View::AnalysisMenu, 5),
        View::AnalysisAssumptions => (View::AnalysisMenu, 6),
        View::Wager => (View::Menu, a.extras().1 + 1),
        View::Amount => (View::Wager, 4),
        _ => return None,
    })
}
fn back(a: &mut App) -> bool {
    let Some((view, row)) = parent(a) else {
        return false;
    };
    if matches!(a.view, View::CountAnswer)
        && let Some(q) = a.count_practice.question()
    {
        a.count_practice.cancel(q.token);
    }
    if matches!(a.view, View::WagerAnswer | View::WagerPractice) {
        a.cancel_wager();
    }
    if matches!(a.view, View::DeviationQuestion | View::DeviationPractice) {
        a.cancel_drill();
    }
    go(a, view, row);
    true
}
fn training_action(a: &mut App, id: usize) {
    go(a, View::Training, id);
    a.blue();
    if a.view == View::Notice {
        a.notice_return = View::AdviceMenu;
        a.page = match id {
            7 => 0,
            5 => 1,
            _ => 2,
        };
    }
}
fn legacy_root(a: &mut App, id: usize) {
    let n = a.extras().1;
    go(a, View::Menu, n + id);
    a.select_menu();
}
fn finish_count_info(a: &mut App) {
    if a.is_busy() {
        note(a, "Finish dealing before count info", View::LiveCounts, 2);
    } else {
        a.observe();
        a.mark_aid(7);
        a.mark_assisted();
        a.set(View::CountInfo);
    }
}
fn safe_to_reset(a: &App) -> bool {
    a.game.between_rounds() && !a.is_busy()
}
fn reset_training(a: &mut App) {
    if !safe_to_reset(a) {
        note(
            a,
            "Finish the round before resetting training",
            View::ProgressMenu,
            5,
        );
        return;
    }
    a.cancel_drill();
    a.cancel_wager();
    if let Some(q) = a.count_practice.question() {
        a.count_practice.cancel(q.token);
    }
    // Model reset APIs retain consumed-token/first-answer protections. Clearing
    // ring slots in place avoids an App-sized or history-sized stack temporary.
    let ok = a.training.reset_stats(true)
        && a.count_practice.reset_stats()
        && a.wager_practice.reset_stats()
        && a.deviation_practice.reset_stats();
    a.pending = None;
    a.live_ev = None;
    a.hint = None;
    a.review_due = None;
    a.last_review = a.round_id;
    a.test_round = false;
    if ok {
        go(a, View::ProgressMenu, 5);
    } else {
        note(a, "Training reset unavailable", View::ProgressMenu, 5);
    }
}
/// Grouped menus intercept their visible rows; hidden old row IDs deliberately
/// fall through to their original controller for private-checkpoint compatibility.
pub(crate) fn handle(a: &mut App, event: Event) -> bool {
    if event == Event::Hold(Key::Grey) {
        return back(a);
    }
    let grey = event == Event::Tap(Key::Grey);
    if !grey && event != Event::Tap(Key::Blue) {
        return false;
    }
    if a.view == View::Notice && !grey && spec_for_return(a.notice_return) {
        let v = a.notice_return;
        let row = a.page;
        go(a, v, row);
        return true;
    }
    if matches!(a.view, View::SessionOverview | View::TrainingModes) {
        if grey {
            a.page = (a.page + 1) % if a.view == View::TrainingModes { 4 } else { 2 };
            a.dirty = true;
        } else {
            back(a);
        }
        return true;
    }
    if !grey
        && matches!(
            a.view,
            View::TrainingStats
                | View::TrainingReview
                | View::CountStats
                | View::CountHistory
                | View::CountInfo
                | View::WagerStats
                | View::WagerHistory
                | View::DeviationStats
                | View::DeviationHistory
                | View::Hint
                | View::ActionEv
                | View::EvReview
                | View::Bankroll
                | View::ShoeInfo
                | View::Rules
                | View::Controls
                | View::Cards
                | View::SeatCards
                | View::AnalysisSummary
                | View::AnalysisGraph
                | View::AnalysisRounds
                | View::AnalysisAccuracy
                | View::AnalysisCosts
                | View::AnalysisDecisions
                | View::AnalysisAssumptions
        )
    {
        return back(a);
    }
    if !grey && a.view == View::AnalysisMenu && a.selected == 7 {
        return back(a);
    }
    if !grey && a.view == View::Profile && a.selected == 7 {
        return back(a);
    }
    if !grey && a.view == View::WagerPolicy && a.selected == 5 {
        return back(a);
    }
    if !grey && a.view == View::DeviationPolicy {
        return back(a);
    }
    if matches!(a.view, View::Display) {
        return false;
    }
    let Some(menu) = spec(a) else { return false };
    if grey {
        if menu.len > 0 {
            let pos = menu.ids[..menu.len]
                .iter()
                .position(|id| *id == a.selected)
                .unwrap_or(menu.len - 1);
            a.selected = menu.ids[(pos + 1) % menu.len];
            a.dirty = true;
        }
        return true;
    }
    match a.view {
        View::Menu => match a.selected.checked_sub(a.extras().1) {
            Some(11) => a.set(View::TableMenu),
            Some(12) => a.set(View::SessionMenu),
            Some(13) => a.set(View::HelpMenu),
            _ => return false,
        },
        View::Training => match a.selected {
            0 => a.cycle_mode(),
            1 => {
                if safe_to_reset(a) {
                    a.policy = if a.policy == Policy::Basic {
                        Policy::HiLo
                    } else {
                        Policy::Basic
                    };
                } else {
                    note(a, "Change strategy between rounds", View::Training, 1);
                }
            }
            11 => a.set(View::LiveCounts),
            12 => a.set(View::PracticeMenu),
            13 => a.set(View::ProgressMenu),
            14 => a.set(View::AdviceMenu),
            10 => {
                root(a, 2);
            }
            _ => return false,
        },
        View::Settings => match a.selected {
            6 => {
                root(a, 3);
            }
            _ => return false,
        },
        View::Sound => match a.selected {
            9 => a.set(View::SoundCategories),
            10 => a.set(View::SoundTests),
            8 => {
                go(a, View::Settings, 1);
            }
            _ => return false,
        },
        View::TableMenu => match a.selected {
            0 => {
                legacy_root(a, 4);
                if a.view == View::Notice {
                    a.notice_return = View::TableMenu;
                    a.page = 0;
                }
            }
            1 => a.set(View::Rules),
            2 => {
                if a.is_busy() {
                    note(a, "Finish dealing before shoe info", View::TableMenu, 2);
                } else {
                    a.observe();
                    a.mark_aid(6);
                    a.mark_assisted();
                    a.set(View::ShoeInfo);
                }
            }
            3 => {
                if a.game.extra_seats > 0 && a.game.hand_count > 0 && !a.is_busy() {
                    a.set(View::SeatCards);
                } else {
                    legacy_root(a, 8);
                    if a.view == View::Notice {
                        a.notice_return = View::TableMenu;
                        a.page = 3;
                    }
                }
            }
            _ => {
                root(a, 11);
            }
        },
        View::SessionMenu => match a.selected {
            0 => a.set(View::SessionOverview),
            1 => a.set(View::AnalysisMenu),
            2 => a.set(View::BankrollMenu),
            3 => {
                if safe_to_reset(a) {
                    a.set(View::NewSessionConfirm);
                } else {
                    note(a, "Finish the round first", View::SessionMenu, 3);
                }
            }
            _ => {
                root(a, 12);
            }
        },
        View::BankrollMenu => match a.selected {
            0 => a.set(View::Bankroll),
            1 => {
                if !safe_to_reset(a) {
                    note(
                        a,
                        "Finish the round before adding money",
                        View::BankrollMenu,
                        1,
                    );
                } else if a.game.buy_in(100_000) {
                    a.emit(Cue::Chip);
                } else {
                    note(a, "Bankroll limit reached", View::BankrollMenu, 1);
                }
            }
            _ => {
                go(a, View::SessionMenu, 2);
            }
        },
        View::NewSessionConfirm => {
            if a.selected == 0 {
                go(a, View::SessionMenu, 3);
            } else if safe_to_reset(a) {
                if let Some(q) = a.count_practice.question() {
                    a.count_practice.cancel(q.token);
                }
                a.reset_session();
            } else {
                note(a, "Finish the round first", View::SessionMenu, 3);
            }
        }
        View::ResetStatsConfirm => {
            if a.selected == 0 {
                go(a, View::ProgressMenu, 5);
            } else {
                reset_training(a);
            }
        }
        View::HelpMenu => match a.selected {
            0 => a.set(View::Controls),
            1 => a.set(View::TrainingModes),
            _ => {
                root(a, 13);
            }
        },
        View::LiveCounts => match a.selected {
            0 => a.count_display = (a.count_display + 1) % 3,
            1 => a.guide_count_display = (a.guide_count_display + 1) % 3,
            2 => finish_count_info(a),
            _ => {
                go(a, View::Training, 11);
            }
        },
        View::PracticeMenu => match a.selected {
            0 => a.set(View::CountPractice),
            1 => a.set(View::WagerPractice),
            2 => a.set(View::DeviationPractice),
            _ => {
                go(a, View::Training, 12);
            }
        },
        View::ProgressMenu => match a.selected {
            0 => a.set(View::HandProgress),
            1 => a.set(View::CountProgress),
            2 => a.set(View::WagerProgress),
            3 => a.set(View::DeviationProgress),
            5 => {
                if safe_to_reset(a) {
                    a.set(View::ResetStatsConfirm);
                } else {
                    note(a, "Finish the round first", View::ProgressMenu, 5);
                }
            }
            _ => {
                go(a, View::Training, 13);
            }
        },
        View::HandProgress
        | View::CountProgress
        | View::WagerProgress
        | View::DeviationProgress => {
            let kind = a.view;
            match a.selected {
                0 => a.set(match kind {
                    View::HandProgress => View::TrainingStats,
                    View::CountProgress => View::CountStats,
                    View::WagerProgress => View::WagerStats,
                    _ => View::DeviationStats,
                }),
                1 => {
                    a.set(match kind {
                        View::HandProgress => View::TrainingReview,
                        View::CountProgress => View::CountHistory,
                        View::WagerProgress => View::WagerHistory,
                        _ => View::DeviationHistory,
                    });
                    if kind == View::HandProgress {
                        a.mark_assisted();
                    }
                    if kind == View::CountProgress {
                        a.mark_count_history();
                    }
                    if kind == View::WagerProgress {
                        a.view_wager_history();
                    }
                }
                _ => {
                    go(
                        a,
                        View::ProgressMenu,
                        match kind {
                            View::HandProgress => 0,
                            View::CountProgress => 1,
                            View::WagerProgress => 2,
                            _ => 3,
                        },
                    );
                }
            }
        }
        View::AdviceMenu => match a.selected {
            0 => training_action(a, 7),
            1 => training_action(a, 5),
            2 => training_action(a, 6),
            _ => {
                go(a, View::Training, 14);
            }
        },
        View::SoundCategories => match a.selected {
            0..=2 => a.audio_mask ^= 1 << a.selected,
            _ => {
                go(a, View::Sound, 9);
            }
        },
        View::SoundTests => match a.selected {
            0..=2 => {
                if a.sound_enabled && a.audio_volume > 0 {
                    a.sounds
                        .push_audition([Cue::CardDeal, Cue::Win, Cue::MenuSelect][a.selected]);
                }
            }
            _ => {
                go(a, View::Sound, 10);
            }
        },
        View::CountPractice => {
            if a.selected == 6 {
                go(a, View::PracticeMenu, 0);
            } else {
                return false;
            }
        }
        View::WagerPractice => {
            if a.selected == 5 {
                go(a, View::PracticeMenu, 1);
            } else {
                return false;
            }
        }
        View::DeviationPractice => {
            if a.selected == 8 {
                go(a, View::PracticeMenu, 2);
            } else {
                return false;
            }
        }
        _ => return false,
    }
    a.dirty = true;
    true
}
fn spec_for_return(view: View) -> bool {
    matches!(
        view,
        View::Menu
            | View::Training
            | View::Settings
            | View::Sound
            | View::TableMenu
            | View::SessionMenu
            | View::BankrollMenu
            | View::LiveCounts
            | View::PracticeMenu
            | View::ProgressMenu
            | View::AdviceMenu
    )
}
fn paragraph(w: &mut Widgets<'_>, text: &str, top: i32, size: u8, lines: usize) {
    let mut line = Text::<128>::new();
    let mut row = 0usize;
    for word in text.split_whitespace() {
        let mut test = Text::<128>::new();
        let _ = write!(
            test,
            "{}{}{}",
            line.as_str(),
            if line.as_str().is_empty() { "" } else { " " },
            word
        );
        if crate::render::text_width(test.as_str(), size, false) > 224 && !line.as_str().is_empty()
        {
            w.label(
                (8, top + row as i32 * (size as i32 + 2)),
                line.as_str(),
                TextStyle {
                    size,
                    bold: false,
                    color: WHITE,
                    align: Align::Left,
                },
            );
            row += 1;
            line.clear();
            if row >= lines {
                return;
            }
        }
        if !line.as_str().is_empty() {
            let _ = line.write_str(" ");
        }
        let _ = line.write_str(word);
    }
    if row < lines {
        w.label(
            (8, top + row as i32 * (size as i32 + 2)),
            line.as_str(),
            TextStyle {
                size,
                bold: false,
                color: WHITE,
                align: Align::Left,
            },
        );
    }
}
pub(crate) fn draw(a: &mut App) -> bool {
    let Some(menu) = spec(a) else { return false };
    let mut w = Widgets {
        frame: &mut a.frame,
        renderer: &mut a.renderer,
    };
    w.title(menu.title, menu.subtitle, false);
    if a.view == View::Menu {
        w.label(
            (232, 6),
            concat!("v", env!("CARGO_PKG_VERSION")),
            TextStyle {
                size: 10,
                bold: false,
                color: crate::canvas::rgb(160, 160, 160),
                align: Align::Right,
            },
        );
    }
    if menu.confirmation {
        if let Some(body) = menu.body {
            paragraph(&mut w, body, 27, 10, 4);
        }
        for i in 0..menu.len {
            w.row(
                82 + i as i32 * 19,
                menu.items[i].as_str(),
                menu.selected == i,
                true,
            );
        }
        w.footer(menu.footer);
    } else if menu.len == 0 {
        let body = if menu.details.as_str().is_empty() {
            menu.body.unwrap_or("")
        } else {
            menu.details.as_str()
        };
        paragraph(&mut w, body, 29, 12, 6);
        w.footer(menu.footer);
    } else {
        let mut items = [""; 16];
        for (i, item) in menu.items.iter().take(menu.len).enumerate() {
            items[i] = item.as_str();
        }
        w.list(&items[..menu.len], menu.selected, None);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::boxed::Box;
    const B: Event = Event::Tap(Key::Blue);
    const G: Event = Event::Tap(Key::Grey);
    const BACK: Event = Event::Hold(Key::Grey);
    fn main(a: &mut App) {
        for _ in 0..16 {
            if a.view == View::Menu {
                return;
            }
            a.event(BACK);
        }
        panic!("main menu unreachable from{}", a.view_name());
    }
    fn choose(a: &mut App, label: &str) {
        for _ in 0..32 {
            if a.menu_item() == label {
                a.event(B);
                return;
            }
            a.event(G);
        }
        panic!("missing {label} in{}", a.view_name());
    }
    fn path(a: &mut App, labels: &[&str]) {
        main(a);
        for label in labels {
            choose(a, label);
        }
    }
    fn settle(a: &mut App, now: &mut u32) {
        let before = a.game.rounds;
        for _ in 0..300 {
            match a.view {
                View::Cut | View::Burn | View::RoundReview => a.event(B),
                View::Table if !a.is_busy() => {
                    if matches!(
                        a.game.phase,
                        Phase::Ready | Phase::Result | Phase::Player | Phase::Insurance
                    ) {
                        a.event(B);
                    }
                }
                _ => {}
            }
            *now += 400;
            a.tick(*now);
            a.render();
            if a.game.rounds > before && !a.is_busy() {
                return;
            }
        }
        panic!("round did not finish");
    }
    fn public(a: &App) -> Save {
        a.save()
    }
    #[test]
    fn hierarchy_is_short_uses_submenu_markers_and_omits_legacy_roleplay() {
        let mut a = Box::new(App::new(7));
        main(&mut a);
        let menu = a.menu_spec().unwrap();
        assert_eq!(menu.len, 7);
        let labels: std::vec::Vec<_> = menu.items[..menu.len].iter().map(Text::as_str).collect();
        assert_eq!(
            labels,
            [
                "Back to game",
                "Bet >",
                "Training >",
                "Table >",
                "Session >",
                "Settings >",
                "Help >"
            ]
        );
        path(&mut a, &["Training"]);
        let menu = a.menu_spec().unwrap();
        assert_eq!(menu.len, 7);
        assert!(menu.items[2].as_str().ends_with('>'));
        path(&mut a, &["Settings"]);
        assert_eq!(a.menu_spec().unwrap().len, 5);
        assert!(
            !a.menu_spec()
                .unwrap()
                .items
                .iter()
                .any(|t| t.as_str().contains("chips"))
        );
        path(&mut a, &["Settings", "Sound"]);
        assert_eq!(a.menu_spec().unwrap().len, 5);
        choose(&mut a, "Categories");
        assert_eq!(a.view, View::SoundCategories);
        a.event(BACK);
        assert_eq!((a.view, a.selected), (View::Sound, 9));
        path(&mut a, &["Training", "Live counts"]);
        choose(&mut a, "Guide mode");
        a.event(BACK);
        assert_eq!((a.view, a.selected), (View::Training, 11));
    }
    #[test]
    fn every_progress_page_has_one_level_blue_and_hold_back() {
        for (group, stats, history) in [
            ("Hand play", View::TrainingStats, View::TrainingReview),
            ("Counting", View::CountStats, View::CountHistory),
            ("Bet sizing", View::WagerStats, View::WagerHistory),
            ("Deviations", View::DeviationStats, View::DeviationHistory),
        ] {
            let mut a = Box::new(App::new(7));
            path(&mut a, &["Training", "Progress", group]);
            let parent = a.view;
            choose(&mut a, "Accuracy");
            assert_eq!(a.view, stats);
            a.event(B);
            assert_eq!((a.view, a.selected), (parent, 0));
            choose(&mut a, "History");
            assert_eq!(a.view, history);
            a.event(BACK);
            assert_eq!((a.view, a.selected), (parent, 1));
        }
        for (group, view) in [
            ("Counting", View::CountPractice),
            ("Bet sizing", View::WagerPractice),
            ("Deviations", View::DeviationPractice),
        ] {
            let mut a = Box::new(App::new(7));
            path(&mut a, &["Training", "Practice", group]);
            assert_eq!(a.view, view);
            assert!(
                !a.menu_spec()
                    .unwrap()
                    .items
                    .iter()
                    .any(|t| matches!(t.as_str(), "Accuracy >" | "History >"))
            );
            a.event(BACK);
            assert_eq!(a.view, View::PracticeMenu);
        }
    }
    #[test]
    fn new_session_default_cancel_preserves_game_and_confirmation_clears_tracker() {
        let mut a = Box::new(App::new(7));
        a.training.mode = Mode::Test;
        let mut now = 0;
        settle(&mut a, &mut now);
        for _ in 0..4 {
            if a.tracker.running_count() != 0 {
                break;
            }
            settle(&mut a, &mut now);
        }
        assert_ne!(
            a.tracker.running_count(),
            0,
            "fixture must exercise an accumulated nonzero count"
        );
        assert_eq!(
            a.game.shoe_number, 1,
            "old and new shoe IDs collide in this regression"
        );
        let save = public(&a);
        let before = (
            a.game.rng,
            a.game.position,
            a.game.bankroll,
            a.game.rounds,
            a.tracker.running_count(),
            a.tracker.observed_count(),
        );
        assert!(before.5 > 0);
        path(&mut a, &["Session", "New session"]);
        assert_eq!((a.view, a.selected), (View::NewSessionConfirm, 0));
        a.event(B);
        assert_eq!((a.view, a.selected), (View::SessionMenu, 3));
        assert_eq!(
            before,
            (
                a.game.rng,
                a.game.position,
                a.game.bankroll,
                a.game.rounds,
                a.tracker.running_count(),
                a.tracker.observed_count()
            )
        );
        assert_eq!(a.save(), save);
        choose(&mut a, "New session");
        a.event(G);
        a.event(B);
        assert_eq!(a.view, View::Cut);
        assert_eq!(a.game.bankroll, 100000);
        assert_eq!(a.game.rounds, 0);
        assert_eq!(a.session_analysis.summary().rounds, 0);
        assert_eq!(a.tracker.running_count(), 0);
        assert_eq!(a.tracker.observed_count(), 0);
        assert_eq!(a.save(), save);
        a.event(B);
        a.event(B);
        for _ in 0..8 {
            now += 400;
            a.tick(now);
            a.render();
        }
        assert!(
            a.tracker.observed_count() > 0,
            "fresh positions must not be suppressed by an old seen map"
        );
    }
    #[test]
    fn destructive_actions_are_gated_in_a_live_hand_and_not_reinterpreted_from_legacy_leave() {
        let mut a = Box::new(App::new(7));
        a.event(B);
        a.event(B);
        let before = (a.game.bankroll, a.game.position, a.game.rounds, a.save());
        path(&mut a, &["Training", "Progress", "Reset training stats"]);
        assert_eq!(a.view, View::Notice);
        assert_eq!(
            before,
            (a.game.bankroll, a.game.position, a.game.rounds, a.save())
        );
        a.event(B);
        assert_eq!((a.view, a.selected), (View::ProgressMenu, 5));
        a.game.phase = Phase::Ready;
        a.reveal = None;
        main(&mut a);
        a.selected = a.extras().1 + 10;
        assert_eq!(a.menu_item(), "End session");
        let old = a.menu_spec().unwrap();
        assert!(
            old.items[..old.len]
                .iter()
                .any(|t| t.as_str() == "End session")
        );
        a.event(B);
        assert_eq!(
            a.view,
            View::Leave,
            "old private row10 must retain its old action, not reset scores"
        );
    }
    #[test]
    fn reset_training_is_confirmed_in_place_preserves_bankroll_and_binds_new_private_state() {
        let mut a = Box::new(App::new(7));
        a.training.mode = Mode::Test;
        a.policy = Policy::Basic;
        let mut now = 0;
        settle(&mut a, &mut now);
        assert!(a.training.stats().all.decisions > 0);
        path(
            &mut a,
            &["Training", "Practice", "Counting", "Running count"],
        );
        a.event(Event::SubmitCount);
        assert_eq!(a.view, View::CountResult);
        path(
            &mut a,
            &["Training", "Practice", "Bet sizing", "Practice now"],
        );
        assert_eq!(a.view, View::WagerAnswer);
        a.selected = 4;
        a.event(B);
        assert_eq!(a.view, View::WagerResult);
        path(
            &mut a,
            &["Training", "Practice", "Deviations", "Practice now"],
        );
        assert_eq!(a.view, View::DeviationQuestion);
        a.event(B);
        assert_eq!(a.view, View::DeviationResult);
        let first = a.training.history(0).unwrap().attempt;
        let old_count = *a.count_practice.newest(0).unwrap();
        let old_wager = *a.wager_practice.newest(0).unwrap();
        let old_drill = *a.deviation_practice.newest(0).unwrap();
        let before = a.save();
        let financial = (
            a.game.bankroll,
            a.game.position,
            a.game.rounds,
            a.session_analysis.summary(),
            a.game.rng,
        );
        path(&mut a, &["Training", "Progress", "Reset training stats"]);
        assert_eq!((a.view, a.selected), (View::ResetStatsConfirm, 0));
        a.event(B);
        assert_eq!(a.save(), before);
        assert_eq!((a.view, a.selected), (View::ProgressMenu, 5));
        choose(&mut a, "Reset training stats");
        a.event(G);
        a.event(B);
        assert_eq!((a.view, a.selected), (View::ProgressMenu, 5));
        assert_eq!(a.training.stats().all.decisions, 0);
        assert_eq!(a.count_practice.stats().all.decisions, 0);
        assert_eq!(a.wager_practice.stats().all.decisions, 0);
        assert_eq!(a.deviation_practice.stats().first.all.decisions, 0);
        assert_eq!(a.deviation_practice.stats().replay.all.decisions, 0);
        assert_eq!(
            (
                a.training.history_count(),
                a.count_practice.history_count(),
                a.wager_practice.history_count(),
                a.deviation_practice.history_count()
            ),
            (0, 0, 0, 0)
        );
        assert_eq!(
            financial,
            (
                a.game.bankroll,
                a.game.position,
                a.game.rounds,
                a.session_analysis.summary(),
                a.game.rng
            )
        );
        let mut expected = before;
        expected.stats = training::Stats::default();
        expected.count_stats = count_practice::Stats::default();
        expected.wager_stats = crate::wager_practice::Stats::default();
        expected.drill_stats = crate::deviation_drills::Stats::EMPTY;
        assert_eq!(a.save(), expected);
        assert_eq!(Save::decode(&a.save().encode().unwrap()), Some(expected));
        assert_ne!(a.training.record(first).status, training::Status::Recorded);
        assert_ne!(
            a.count_practice
                .submit(old_count.question.token, old_count.answer, false)
                .status,
            count_practice::Status::Recorded
        );
        assert_ne!(
            a.wager_practice
                .submit(old_wager.question.token, old_wager.choice, false)
                .status,
            crate::wager_practice::Status::Recorded
        );
        assert_ne!(
            a.deviation_practice
                .submit(old_drill.question.token, old_drill.answer, false)
                .status,
            crate::deviation_drills::Status::Recorded
        );
        a.render();
        let mut bytes = std::vec![0;crate::resume_private::MAX_BYTES];
        let n = crate::resume_private::capture_private(&a, &mut bytes, 41, true).unwrap();
        let mut restored = Box::new(App::EMPTY);
        crate::resume_private::restore_candidate(&mut restored, &bytes[..n], 20000, true).unwrap();
        assert_eq!(restored.save(), expected);
        assert_eq!(restored.game.bankroll, financial.0);
        main(&mut a);
        choose(&mut a, "Back to game");
        settle(&mut a, &mut now);
        assert!(
            a.training.stats().all.decisions > 0,
            "a fresh first decision must be graded after reset"
        );
    }
    #[test]
    fn all_new_view_tags_roundtrip_without_changing_old_leave_tag() {
        use crate::resume_private::Wire;
        let views = [
            View::TableMenu,
            View::SessionMenu,
            View::SessionOverview,
            View::BankrollMenu,
            View::NewSessionConfirm,
            View::ResetStatsConfirm,
            View::HelpMenu,
            View::TrainingModes,
            View::LiveCounts,
            View::PracticeMenu,
            View::ProgressMenu,
            View::AdviceMenu,
            View::SoundCategories,
            View::SoundTests,
            View::HandProgress,
            View::CountProgress,
            View::WagerProgress,
            View::DeviationProgress,
        ];
        assert_eq!(View::Leave as u8, 57);
        assert_eq!(View::Brightness as u8, 64);
        for (offset, view) in views.iter().copied().enumerate() {
            let out = [65 + offset as u8];
            assert_eq!(view as u8, out[0]);
            let mut r = crate::resume_private::Reader::from_bytes(&out[..1]);
            assert_eq!(View::get(&mut r).unwrap(), view);
        }
    }
}
