//! Optional example-ramp controller, kept separate from normal table controls.
use super::*;
use crate::{
    render::{Align, TextStyle, text_width},
    text::{Cash, Text},
    wager_practice as wager,
    widgets::{GOLD, MUTED, WHITE, Widgets},
};
use core::fmt::Write;
impl View {
    fn is_wager(self) -> bool {
        matches!(
            self,
            Self::WagerPractice
                | Self::WagerAnswer
                | Self::WagerHelp
                | Self::WagerResult
                | Self::WagerStats
                | Self::WagerHistory
                | Self::WagerPolicy
        )
    }
}
impl App {
    fn wager_snapshot(&mut self) -> Option<wager::Snapshot> {
        if !self.game.between_rounds() || self.is_busy() {
            return None;
        }
        self.observe();
        let mut snapshot = wager::Snapshot {
            context: wager::Context {
                session: self.session,
                checkpoint: self.wager_checkpoint.max(1),
                shoe: self.game.shoe_number,
            },
            rules: self.game.rules,
            count: self.count_context(),
            bankroll: self.game.bankroll,
            predeal: true,
            estimated_raw_tc: None,
        };
        if !snapshot.valid() {
            return None;
        }
        let same = self.wager_evidence.is_some_and(|old| {
            old.context.session == snapshot.context.session
                && old.context.shoe == snapshot.context.shoe
                && old.rules == snapshot.rules
                && old.count == snapshot.count
                && old.bankroll == snapshot.bankroll
        });
        if !same {
            self.wager_checkpoint = self.wager_checkpoint.checked_add(1)?;
            snapshot.context.checkpoint = self.wager_checkpoint;
            self.wager_evidence = Some(snapshot)
        } else {
            snapshot.context = self.wager_evidence.unwrap().context
        }
        Some(snapshot)
    }
    pub(super) fn cancel_wager(&mut self) {
        if let Some(q) = self.wager_practice.question() {
            self.wager_practice.cancel(q.token);
        }
    }
    pub(super) fn reconcile_wager(&mut self) {
        if !self.wager_practice.awaiting_answer() {
            return;
        }
        let old = self.wager_practice.question().unwrap();
        let same = self
            .wager_snapshot()
            .is_some_and(|s| s.context == old.context);
        if !same {
            self.cancel_wager();
            if self.view.is_wager()
                || (self.view == View::Notice && self.notice_return == View::WagerAnswer)
            {
                self.notice(
                    "Predeal checkpoint changed; start again",
                    View::WagerPractice,
                )
            }
        }
    }
    pub(super) fn open_wager_question(&mut self) {
        self.cancel_wager();
        let Some(snapshot) = self.wager_snapshot() else {
            self.notice(
                "Wager practice needs a predeal checkpoint",
                View::WagerPractice,
            );
            return;
        };
        let Some(token) = self.wager_token.checked_add(1) else {
            self.notice("Wager token limit reached", View::WagerPractice);
            return;
        };
        if !self
            .wager_practice
            .begin(token, snapshot, self.wager_policy)
        {
            self.notice("Wager checkpoint unavailable", View::WagerPractice);
            return;
        }
        self.wager_token = token;
        self.wager_amount = self
            .game
            .selected_bet
            .clamp(snapshot.rules.minimum, snapshot.rules.maximum);
        if self.aid_key == self.aid_key() && self.aid_mask & 7 != 0 {
            self.wager_practice.mark_assisted(token);
        }
        self.set(View::WagerAnswer)
    }
    fn wager_reference_aid(&mut self, reference: wager::Reference) {
        if self
            .wager_snapshot()
            .is_some_and(|s| s == reference.snapshot)
        {
            self.mark_aid(7);
        }
        self.mark_assisted();
    }
    fn submit_wager(&mut self, choice: wager::Choice) {
        let Some(q) = self.wager_practice.question() else {
            self.notice("Wager checkpoint expired", View::WagerPractice);
            return;
        };
        if self.wager_snapshot().is_none_or(|s| s.context != q.context) {
            self.cancel_wager();
            self.notice(
                "Predeal checkpoint changed; start again",
                View::WagerPractice,
            );
            return;
        }
        let grade = self.wager_practice.submit(q.token, choice, false);
        if let Some(a) = grade.attempt {
            if grade.status == wager::Status::Recorded {
                self.session_metrics.record_wager(a);
            }
            self.wager_reference_aid(a.reference);
            self.emit(if a.correct {
                Cue::MenuSelect
            } else {
                Cue::Loss
            });
            self.set(View::WagerResult)
        } else {
            self.notice("Use a legal $5 wager within bankroll", View::WagerAnswer)
        }
    }
    fn apply_wager_choice(&mut self) {
        let Some(a) = self.wager_practice.newest(0).copied() else {
            self.set(View::WagerPractice);
            return;
        };
        if self.wager_snapshot() != Some(a.reference.snapshot) {
            self.notice(
                "Predeal checkpoint changed; start again",
                View::WagerPractice,
            );
            return;
        }
        match a.choice {
            wager::Choice::Wager(amount) => {
                if !self.game.set_bet(amount) {
                    self.notice("Wager no longer fits the table", View::WagerPractice);
                    return;
                }
                self.wager_skip_once = true;
                self.set(View::Table);
                self.next_deal()
            }
            wager::Choice::NoBetRefill => {
                if self.game.buy_in(100_000) {
                    self.emit(Cue::Chip);
                    self.open_table()
                } else {
                    self.notice("Refill unavailable", View::WagerPractice)
                }
            }
        }
    }
    pub(super) fn view_wager_history(&mut self) {
        if let Some(a) = self.wager_practice.newest(self.page).copied() {
            self.wager_practice.request_help(a.question.token);
            self.wager_reference_aid(a.reference)
        }
    }
    pub(super) fn handle_wager_tap(&mut self, grey: bool) -> bool {
        if !self.view.is_wager() {
            return false;
        }
        if grey {
            match self.view {
                View::WagerPractice => self.selected = (self.selected + 1) % 6,
                View::WagerAnswer => self.selected = (self.selected + 1) % 9,
                View::WagerResult => {
                    if self.page == 0 {
                        self.page = 1;
                        self.selected = 0
                    } else {
                        self.selected = (self.selected + 1) % 5
                    }
                }
                View::WagerStats => self.page = (self.page + 1) % 7,
                View::WagerHistory => {
                    self.page = (self.page + 1) % self.wager_practice.history_count().max(1);
                    self.view_wager_history()
                }
                View::WagerPolicy => self.selected = (self.selected + 1) % 6,
                _ => {}
            }
            return true;
        }
        match self.view {
            View::WagerPractice => match self.selected {
                0 => {
                    if self.game.between_rounds() && !self.is_busy() {
                        self.wager_enabled = !self.wager_enabled
                    } else {
                        self.notice("Change wager training between rounds", View::WagerPractice)
                    }
                }
                1 => self.open_wager_question(),
                2 => self.set(View::WagerStats),
                3 => {
                    self.set(View::WagerHistory);
                    self.view_wager_history()
                }
                4 => self.set(View::WagerPolicy),
                _ => self.set(View::Training),
            },
            View::WagerAnswer => match self.selected {
                0..=3 => {
                    let high = (self.game.bankroll / 500 * 500)
                        .min(self.game.rules.maximum)
                        .max(self.game.rules.minimum);
                    self.wager_amount = (self.wager_amount
                        + [500, -500, 2500, -2500][self.selected])
                        .clamp(self.game.rules.minimum, high)
                }
                4 => self.submit_wager(wager::Choice::Wager(self.wager_amount)),
                5 => self.submit_wager(wager::Choice::NoBetRefill),
                6 => {
                    if let Some(q) = self.wager_practice.question()
                        && let Some(r) = self.wager_practice.request_help(q.token)
                    {
                        self.wager_reference_aid(r);
                        self.set(View::WagerHelp)
                    }
                }
                7 => {
                    self.cancel_wager();
                    self.wager_skip_once = true;
                    self.open_table();
                    self.next_deal()
                }
                _ => {
                    self.cancel_wager();
                    self.set(View::WagerPractice)
                }
            },
            View::WagerHelp => self.set(View::WagerAnswer),
            View::WagerResult if self.page == 0 => self.apply_wager_choice(),
            View::WagerResult => match self.selected {
                0 => self.apply_wager_choice(),
                1 => {
                    self.wager_skip_once = true;
                    self.set(View::Wager)
                }
                2 => self.open_wager_question(),
                3 => self.set(View::WagerPractice),
                _ => {
                    self.page = 0;
                    self.selected = 0;
                }
            },
            View::WagerStats | View::WagerHistory => self.set(View::WagerPractice),
            View::WagerPolicy => {
                if self.selected == 5 {
                    self.set(View::WagerPractice)
                } else if self.game.between_rounds() && !self.is_busy() {
                    let values = [1, 2, 4, 8, 12];
                    let current = self.wager_policy.units[self.selected];
                    let at = values.iter().position(|&n| n == current).unwrap_or(4);
                    self.wager_policy.units[self.selected] = values[(at + 1) % values.len()];
                } else {
                    self.notice("Change example ramp between rounds", View::WagerPractice)
                }
            }
            _ => {}
        }
        true
    }
}
fn label(w: &mut Widgets<'_>, y: i32, s: &str, size: u8, color: u16) {
    let mut size = size;
    while size > 8 && text_width(s, size, true) > 224 {
        size -= 1
    }
    w.label(
        (8, y),
        s,
        TextStyle {
            size,
            bold: true,
            color,
            align: Align::Left,
        },
    );
}
fn formatted<'a>(b: &'a mut Text<160>, args: core::fmt::Arguments<'_>) -> &'a str {
    b.clear();
    let _ = b.write_fmt(args);
    b.as_str()
}
fn target_text(choice: wager::Choice) -> Text<40> {
    let mut t = Text::new();
    match choice {
        wager::Choice::Wager(n) => {
            let _ = write!(t, "{}", Cash(n, false));
        }
        wager::Choice::NoBetRefill => {
            let _ = write!(t, "NO BET / REFILL");
        }
    }
    t
}
fn reference(w: &mut Widgets<'_>, r: wager::Reference, answer: Option<wager::Attempt>) {
    let mut b = Text::new();
    label(
        w,
        24,
        formatted(
            &mut b,
            format_args!(
                "RC {:+} / TC {:+} (trunc)",
                r.snapshot.count.running, r.canonical.index
            ),
        ),
        12,
        WHITE,
    );
    if let Some(a) = answer {
        let chosen = target_text(a.choice);
        label(
            w,
            43,
            formatted(
                &mut b,
                format_args!("Your first choice {}", chosen.as_str()),
            ),
            13,
            WHITE,
        )
    } else {
        label(w, 43, "EXAMPLE PRACTICE POLICY", 12, MUTED)
    }
    let target = target_text(r.canonical.choice);
    label(
        w,
        62,
        formatted(&mut b, format_args!("Example target {}", target.as_str())),
        15,
        GOLD,
    );
    let adj = r.canonical.adjustments;
    label(
        w,
        84,
        formatted(
            &mut b,
            format_args!(
                "Caps: table {} / bank {} / step {}",
                if adj.table_cap { "yes" } else { "no" },
                if adj.bankroll_cap { "yes" } else { "no" },
                if adj.step_rounding { "yes" } else { "no" }
            ),
        ),
        10,
        MUTED,
    );
    label(
        w,
        102,
        if answer.is_some_and(|a| a.assisted) {
            "Assisted; excluded from personal accuracy"
        } else {
            "Example arithmetic; not an optimal ramp"
        },
        9,
        MUTED,
    );
}
pub(crate) fn draw(a: &mut App) -> bool {
    if !a.view.is_wager() {
        return false;
    }
    let pending = a
        .wager_practice
        .question()
        .and_then(|q| a.wager_practice.reference(q.token));
    let attempt = a
        .wager_practice
        .newest(if a.view == View::WagerHistory {
            a.page
        } else {
            0
        })
        .copied();
    let mut w = Widgets {
        frame: &mut a.frame,
        renderer: &mut a.renderer,
    };
    let mut b = Text::<160>::new();
    match a.view {
        View::WagerPractice => {
            w.title(
                "WAGER PRACTICE",
                Some("EXAMPLE POLICY / manual play stays available"),
                false,
            );
            w.list(
                &[
                    if a.wager_enabled {
                        "Predeal prompts: ON"
                    } else {
                        "Predeal prompts: OFF"
                    },
                    "Start checkpoint",
                    "Wager accuracy",
                    "Wager history",
                    "Example unit ramp",
                    "Return",
                ],
                a.selected,
                None,
            )
        }
        View::WagerAnswer => {
            let mut heading = Text::<120>::new();
            let _ = write!(
                heading,
                "Bet {} / bank {} / {}-{}",
                Cash(a.wager_amount, false),
                Cash(a.game.bankroll, false),
                Cash(a.game.rules.minimum, false),
                Cash(a.game.rules.maximum, false)
            );
            w.title("CHOOSE PRACTICE WAGER", Some(heading.as_str()), false);
            let mut submit = Text::<48>::new();
            let _ = write!(submit, "Submit {}", Cash(a.wager_amount, false));
            w.list(
                &[
                    "Add $5",
                    "Subtract $5",
                    "Add $25",
                    "Subtract $25",
                    submit.as_str(),
                    "Cannot fund minimum",
                    "Show example answer",
                    "Manual play / no score",
                    "Cancel checkpoint",
                ],
                a.selected,
                None,
            )
        }
        View::WagerHelp => {
            w.title("EXAMPLE ANSWER", None, false);
            if let Some(r) = pending {
                reference(&mut w, r, None)
            } else {
                label(&mut w, 50, "Checkpoint expired", 16, WHITE)
            }
            w.footer("BLUE: ANSWER (ASSISTED)")
        }
        View::WagerResult => {
            let Some(attempt) = attempt else {
                w.title("WAGER RESULT", None, false);
                label(&mut w, 49, "No first attempt recorded", 16, WHITE);
                w.footer("BLUE: BACK");
                return true;
            };
            w.title(
                if attempt.assisted {
                    "WAGER ASSISTED"
                } else if attempt.correct {
                    "WAGER CORRECT"
                } else {
                    "WAGER MISTAKE"
                },
                None,
                false,
            );
            if a.page == 0 {
                reference(&mut w, attempt.reference, Some(attempt));
                w.footer(if attempt.choice == wager::Choice::NoBetRefill {
                    "GREY: OPTIONS   BLUE: REFILL"
                } else {
                    "GREY: OPTIONS   BLUE: PLAY CHOICE"
                });
            } else {
                w.list(
                    &[
                        if attempt.choice == wager::Choice::NoBetRefill {
                            "Refill $1000 (deposit)"
                        } else {
                            "Play my chosen wager"
                        },
                        "Edit manual wager",
                        "Another checkpoint",
                        "Wager practice",
                        "View reference",
                    ],
                    a.selected,
                    None,
                );
            }
        }
        View::WagerHistory => {
            w.title("WAGER HISTORY", None, false);
            if let Some(attempt) = attempt {
                reference(&mut w, attempt.reference, Some(attempt))
            } else {
                label(&mut w, 48, "No wager attempts recorded", 15, WHITE);
                label(&mut w, 78, "History stays in this session", 11, MUTED)
            }
            w.footer("GREY: HISTORY   BLUE: BACK")
        }
        View::WagerStats => {
            w.title(
                [
                    "WAGER ACCURACY",
                    "TC ZERO OR BELOW",
                    "TC ONE",
                    "TC TWO",
                    "TC THREE",
                    "TC FOUR PLUS",
                    "REFILL CASES",
                ][a.page.min(6)],
                None,
                false,
            );
            let stats = a.wager_practice.stats();
            let c = if a.page == 0 {
                stats.all
            } else {
                stats.categories[a.page - 1]
            };
            label(
                &mut w,
                33,
                formatted(
                    &mut b,
                    format_args!("Correct {} / {}", c.correct, c.correct + c.errors),
                ),
                16,
                GOLD,
            );
            label(
                &mut w,
                59,
                formatted(&mut b, format_args!("Errors {}", c.errors)),
                14,
                WHITE,
            );
            label(
                &mut w,
                81,
                formatted(&mut b, format_args!("Assisted {}", c.assisted)),
                14,
                WHITE,
            );
            label(
                &mut w,
                105,
                "Example-policy accuracy; not profit",
                10,
                MUTED,
            );
            w.footer("GREY: CATEGORY   BLUE: BACK")
        }
        View::WagerPolicy => {
            w.title(
                "EXAMPLE UNIT RAMP",
                Some("Example arithmetic / not an optimal ramp"),
                false,
            );
            let mut labels: [Text<48>; 5] = core::array::from_fn(|_| Text::new());
            for (i, t) in labels.iter_mut().enumerate() {
                let _ = write!(
                    t,
                    "{}: {} units",
                    ["TC <= 0", "TC 1", "TC 2", "TC 3", "TC >= 4"][i],
                    a.wager_policy.units[i]
                );
            }
            w.list(
                &[
                    labels[0].as_str(),
                    labels[1].as_str(),
                    labels[2].as_str(),
                    labels[3].as_str(),
                    labels[4].as_str(),
                    "Return",
                ],
                a.selected,
                Some("Arithmetic example; not an optimal ramp"),
            );
        }
        _ => {}
    }
    if let Some(mode) = a.toast {
        w.mode_footer(mode)
    }
    true
}
