//! Synthetic public-context index practice. No live shoe or RNG enters a drill.
use super::*;
use crate::{
    deviation_drills as drill,
    render::{Align, TextStyle, text_width},
    text::{Cash, Text},
    training::Counters,
    widgets::{GOLD, MUTED, WHITE, Widgets},
};
use core::fmt::Write;
impl View {
    fn is_deviation(self) -> bool {
        matches!(
            self,
            Self::DeviationPractice
                | Self::DeviationTarget
                | Self::DeviationQuestion
                | Self::DeviationContext
                | Self::DeviationReference
                | Self::DeviationResult
                | Self::DeviationStats
                | Self::DeviationHistory
                | Self::DeviationPolicy
        )
    }
}
fn choices(q: drill::Question) -> ([Choice; 5], usize) {
    let mut out = [Choice::Play(Action::Hit); 5];
    let mut n = 0;
    if q.index == drill::IndexId::Insurance {
        out[0] = Choice::Insurance(false);
        n = 1;
        if q.context.funding.bankroll >= q.context.funding.original_wager / 2 {
            out[1] = Choice::Insurance(true);
            n = 2;
        }
    } else {
        for a in Action::ALL {
            if q.context.legal_mask & a.bit() != 0 {
                out[n] = Choice::Play(a);
                n += 1;
            }
        }
    }
    (out, n)
}
fn choice(c: Choice) -> &'static str {
    match c {
        Choice::Play(a) => table::action_name(a),
        Choice::Insurance(true) => "TAKE INSURANCE",
        Choice::Insurance(false) => "DECLINE INSURANCE",
    }
}
fn target(id: u8) -> &'static str {
    drill::IndexId::from_id(id).map_or("Mixed targets", drill::IndexId::name)
}
fn mix(id: u8) -> &'static str {
    ["MIXED", "BELOW", "AT", "ABOVE"][id.min(3) as usize]
}
impl App {
    pub(super) fn cancel_drill(&mut self) {
        if let Some(q) = self.deviation_practice.question() {
            self.deviation_practice.cancel(q.token);
        }
    }
    fn new_drill(&mut self, replay: bool) {
        if self.is_busy() {
            self.notice("Finish dealing before a drill", View::DeviationPractice);
            return;
        }
        self.cancel_drill();
        let Some(token) = self.drill_token.checked_add(1) else {
            self.notice("Drill counter limit reached", View::DeviationPractice);
            return;
        };
        self.drill_token = token;
        let question = if replay {
            let missed = (0..self.deviation_practice.history_count()).find_map(|i| {
                self.deviation_practice
                    .newest(i)
                    .filter(|a| !a.correct && !a.assisted)
                    .map(|a| a.question.token)
            });
            missed.and_then(|t| self.deviation_practice.replay_missed(t, token))
        } else {
            let boundary = match self.drill_mix {
                1 => drill::Boundary::Below,
                2 => drill::Boundary::At,
                3 => drill::Boundary::Above,
                _ => match self.drill_generator.bounded(3) {
                    0 => drill::Boundary::Below,
                    1 => drill::Boundary::At,
                    _ => drill::Boundary::Above,
                },
            };
            if let Some(index) = drill::IndexId::from_id(self.drill_target) {
                drill::generate(token, index, self.drill_profile, boundary)
            } else {
                let start = self.drill_generator.bounded(drill::IndexId::COUNT as u64) as usize;
                (0..drill::IndexId::COUNT).find_map(|i| {
                    drill::generate(
                        token,
                        drill::IndexId::ALL[(start + i) % drill::IndexId::COUNT],
                        self.drill_profile,
                        boundary,
                    )
                })
            }
        };
        if let Some(q) = question
            && self.deviation_practice.begin(q)
        {
            self.drill_choice = 0;
            self.set(View::DeviationQuestion)
        } else {
            self.notice(
                if replay {
                    "No unassisted mistakes in history"
                } else {
                    "Target is unavailable under these rules"
                },
                View::DeviationPractice,
            )
        }
    }
    pub(super) fn handle_drill_tap(&mut self, grey: bool) -> bool {
        if !self.view.is_deviation() {
            return false;
        }
        if grey {
            match self.view {
                View::DeviationPractice => self.selected = (self.selected + 1) % 9,
                View::DeviationTarget => self.selected = (self.selected + 1) % 32,
                View::DeviationQuestion => {
                    if let Some(q) = self.deviation_practice.question() {
                        self.selected = (self.selected + 1) % (choices(q).1 + 3)
                    }
                }
                View::DeviationContext => self.page = (self.page + 1) % 3,
                View::DeviationReference => self.page = (self.page + 1) % 2,
                View::DeviationResult => self.new_drill(false),
                View::DeviationStats => self.page = (self.page + 1) % 33,
                View::DeviationHistory => {
                    self.page =
                        (self.page + 1) % (self.deviation_practice.history_count() * 6).max(1)
                }
                View::DeviationPolicy => self.page = (self.page + 1) % 4,
                _ => {}
            }
        } else {
            match self.view {
                View::DeviationPractice => match self.selected {
                    0 => self.new_drill(false),
                    1 => {
                        self.set(View::DeviationTarget);
                        self.selected = self.drill_target as usize
                    }
                    2 => self.drill_mix = (self.drill_mix + 1) % 4,
                    3 => {
                        self.drill_profile =
                            Profile::ALL[(self.drill_profile as usize + 1) % Profile::ALL.len()]
                    }
                    4 => self.set(View::DeviationStats),
                    5 => self.new_drill(true),
                    6 => self.set(View::DeviationHistory),
                    7 => self.set(View::DeviationPolicy),
                    _ => self.set(View::Training),
                },
                View::DeviationTarget => {
                    self.drill_target = self.selected as u8;
                    self.set(View::DeviationPractice)
                }
                View::DeviationQuestion => {
                    if let Some(q) = self.deviation_practice.question() {
                        let (answer, n) = choices(q);
                        if self.selected < n {
                            let grade = self.deviation_practice.submit(
                                q.token,
                                answer[self.selected],
                                false,
                            );
                            if grade.status == drill::Status::Recorded {
                                self.emit(if grade.attempt.is_some_and(|a| a.correct) {
                                    Cue::MenuSelect
                                } else {
                                    Cue::Loss
                                });
                                self.set(View::DeviationResult)
                            }
                        } else if self.selected == n {
                            self.drill_choice = self.selected;
                            self.set(View::DeviationContext)
                        } else if self.selected == n + 1 {
                            self.drill_choice = self.selected;
                            self.deviation_practice.request_help(q.token);
                            self.set(View::DeviationReference)
                        } else {
                            self.cancel_drill();
                            self.set(View::DeviationPractice)
                        }
                    }
                }
                View::DeviationContext | View::DeviationReference => {
                    self.set(View::DeviationQuestion);
                    self.selected = self.drill_choice
                }
                View::DeviationResult
                | View::DeviationStats
                | View::DeviationHistory
                | View::DeviationPolicy => self.set(View::DeviationPractice),
                _ => {}
            }
        }
        true
    }
}
fn label(w: &mut Widgets<'_>, y: i32, s: &str, size: u8, color: u16) {
    let mut size = size;
    while size > 7 && text_width(s, size, false) > 224 {
        size -= 1
    }
    w.label(
        (8, y),
        s,
        TextStyle {
            size,
            bold: false,
            color,
            align: Align::Left,
        },
    );
}
fn at(w: &mut Widgets<'_>, x: i32, y: i32, s: &str, size: u8, color: u16) {
    w.label(
        (x, y),
        s,
        TextStyle {
            size,
            bold: false,
            color,
            align: Align::Left,
        },
    );
}
fn text<'a>(b: &'a mut Text<160>, args: core::fmt::Arguments<'_>) -> &'a str {
    b.clear();
    let _ = b.write_fmt(args);
    b.as_str()
}
fn rules(c: drill::PublicContext) -> Text<160> {
    let mut t = Text::new();
    let _ = write!(
        t,
        "{}D {} {} {} / BJ{}:{}",
        c.rules.decks,
        if c.rules.hit_soft_17 { "H17" } else { "S17" },
        if c.rules.double_after_split {
            "DAS"
        } else {
            "NO DAS"
        },
        if c.rules.late_surrender {
            "LS"
        } else {
            "NO LS"
        },
        c.rules.blackjack_numerator,
        c.rules.blackjack_denominator
    );
    t
}
fn threshold(t: drill::Threshold) -> Text<160> {
    let mut b = Text::new();
    match t {
        drill::Threshold::AtLeast(n) => {
            let _ = write!(b, "TC >= {n:+} (inclusive)");
        }
        drill::Threshold::AtMost(n) => {
            let _ = write!(b, "TC <= {n:+} (inclusive)");
        }
        drill::Threshold::PositiveRunning => {
            let _ = write!(b, "RC >0 (strictly positive)");
        }
        drill::Threshold::NegativeRunning => {
            let _ = write!(b, "RC <0 (strictly negative)");
        }
    }
    b
}
fn context(w: &mut Widgets<'_>, q: drill::Question, page: usize) {
    let c = q.context;
    let mut b = Text::<160>::new();
    w.title(
        "ORIGINAL DRILL CONTEXT",
        Some("Synthetic public snapshot / no live shoe"),
        false,
    );
    match page % 3 {
        0 => {
            label(w, 34, rules(c).as_str(), 11, WHITE);
            label(w, 53, "US hole card / peek; double any2", 10, MUTED);
            label(w, 72, "Max4 hands; no resplit aces", 10, MUTED);
            label(w, 91, "Split aces:1 card; split21 pays1:1", 10, MUTED);
            label(w, 108, "Generic rules; no named-casino claim", 8, MUTED);
        }
        1 => {
            label(
                w,
                34,
                text(
                    &mut b,
                    format_args!("Given running count {:+}", c.count.running),
                ),
                12,
                WHITE,
            );
            label(
                w,
                55,
                text(
                    &mut b,
                    format_args!(
                        "Given {} cards /52 = {:.1} decks",
                        c.count.undealt,
                        c.count.undealt as f64 / 52.0
                    ),
                ),
                10,
                WHITE,
            );
            label(
                w,
                76,
                text(
                    &mut b,
                    format_args!(
                        "Signed truncated TC {:+}",
                        c.count.integer_true_count(c.rules).unwrap_or(0)
                    ),
                ),
                12,
                GOLD,
            );
            label(w, 98, "BJA2018 Hi-Lo / drop fractional part", 9, MUTED);
            label(w, 110, "No hidden/future card sequence", 8, MUTED);
        }
        _ => {
            label(
                w,
                34,
                text(
                    &mut b,
                    format_args!("Original wager {}", Cash(c.funding.original_wager, false)),
                ),
                12,
                WHITE,
            );
            label(
                w,
                55,
                text(
                    &mut b,
                    format_args!("Available funding {}", Cash(c.funding.bankroll, false)),
                ),
                12,
                WHITE,
            );
            label(
                w,
                76,
                text(
                    &mut b,
                    format_args!("Question #{} / {:?}", q.token, q.origin()),
                ),
                9,
                WHITE,
            );
            label(w, 95, "Legal actions were fixed with context", 9, MUTED);
            label(w, 109, "Help and replay keep separate scores", 8, MUTED);
        }
    }
}
fn reference(w: &mut Widgets<'_>, q: drill::Question, r: drill::Reference, page: usize) {
    let mut b = Text::<160>::new();
    w.title(
        "DEVIATION REFERENCE",
        Some("BJA2018 Hi-Lo / signed truncation"),
        false,
    );
    if page.is_multiple_of(2) {
        label(w, 34, choice(r.best), 16, GOLD);
        label(w, 57, threshold(r.threshold).as_str(), 11, WHITE);
        label(
            w,
            77,
            text(
                &mut b,
                format_args!(
                    "RC{:+} /{:.1}d -> TC{:+}",
                    q.context.count.running,
                    q.context.count.undealt as f64 / 52.0,
                    r.integer_true_count
                ),
            ),
            11,
            WHITE,
        );
        label(
            w,
            97,
            if r.threshold_reached {
                "Threshold reached; respect legality"
            } else {
                "Threshold not reached; respect legality"
            },
            9,
            MUTED,
        );
        label(w, 110, "Reference before answer = assisted", 8, MUTED);
    } else {
        label(
            w,
            34,
            text(&mut b, format_args!("Basic: {}", choice(r.basic))),
            12,
            WHITE,
        );
        label(
            w,
            55,
            text(&mut b, format_args!("Policy: {}", choice(r.best))),
            12,
            GOLD,
        );
        label(w, 77, "Surrender/pair priority and funding", 10, MUTED);
        label(w, 95, "Unavailable actions use legal fallback", 9, MUTED);
        label(w, 110, "Published index policy; not exact EV", 8, MUTED);
    }
}
fn result(w: &mut Widgets<'_>, a: drill::Attempt) {
    let mut b = Text::<160>::new();
    w.title(
        if a.assisted {
            "ASSISTED DRILL"
        } else if a.correct {
            "CORRECT DRILL"
        } else {
            "DEVIATION MISTAKE"
        },
        Some(a.question.index.name()),
        false,
    );
    label(
        w,
        34,
        text(&mut b, format_args!("You: {}", choice(a.answer))),
        12,
        WHITE,
    );
    label(
        w,
        55,
        text(
            &mut b,
            format_args!("Reference: {}", choice(a.reference.best)),
        ),
        13,
        GOLD,
    );
    label(w, 77, threshold(a.reference.threshold).as_str(), 10, WHITE);
    label(
        w,
        95,
        text(
            &mut b,
            format_args!(
                "RC{:+} /{:.1}d / TC{:+}",
                a.question.context.count.running,
                a.question.context.count.undealt as f64 / 52.0,
                a.reference.integer_true_count
            ),
        ),
        10,
        MUTED,
    );
    label(
        w,
        110,
        if matches!(a.question.origin(), drill::Origin::Replay { .. }) {
            "Replay score; original attempt unchanged"
        } else {
            "First submitted answer retained"
        },
        8,
        MUTED,
    );
}
fn totals(w: &mut Widgets<'_>, c: Counters, subtitle: &str) {
    let mut b = Text::<160>::new();
    w.title("DEVIATION ACCURACY", Some(subtitle), false);
    label(
        w,
        34,
        text(
            &mut b,
            format_args!("Correct {} /{}", c.correct, c.correct + c.errors),
        ),
        13,
        GOLD,
    );
    label(
        w,
        57,
        text(&mut b, format_args!("Errors {}", c.errors)),
        12,
        WHITE,
    );
    label(
        w,
        79,
        text(&mut b, format_args!("Assisted {}", c.assisted)),
        12,
        WHITE,
    );
    label(
        w,
        99,
        text(&mut b, format_args!("Submitted {}", c.decisions)),
        10,
        MUTED,
    );
    label(w, 112, "Assisted and replay stay separate", 8, MUTED);
}
pub(crate) fn draw(a: &mut App) -> bool {
    if !a.view.is_deviation() {
        return false;
    }
    let pending = a.deviation_practice.question();
    let pending_ref = pending.and_then(|q| a.deviation_practice.reference(q.token));
    let attempt = a
        .deviation_practice
        .newest(if a.view == View::DeviationHistory {
            a.page / 6
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
        View::DeviationPractice => {
            w.title(
                "DEVIATION DRILLS",
                Some("Synthetic contexts / independent of live game"),
                false,
            );
            let mut target_text = Text::<80>::new();
            let _ = write!(target_text, "Target: {}", target(a.drill_target));
            let mut mix_text = Text::<48>::new();
            let _ = write!(mix_text, "Boundary mix: {}", mix(a.drill_mix));
            let mut rule_text = Text::<80>::new();
            let r = Rules::for_profile(a.drill_profile);
            let _ = write!(
                rule_text,
                "Rules: {}D {} {}",
                r.decks,
                if r.hit_soft_17 { "H17" } else { "S17" },
                if r.late_surrender { "LS" } else { "NO LS" }
            );
            w.list(
                &[
                    "New question",
                    target_text.as_str(),
                    mix_text.as_str(),
                    rule_text.as_str(),
                    "Drill accuracy",
                    "Replay missed",
                    "Review attempts",
                    "Policy / original rules",
                    "Return",
                ],
                a.selected,
                None,
            )
        }
        View::DeviationTarget => {
            w.title(
                "CHOOSE DRILL TARGET",
                Some("Rule-specific targets require matching rules"),
                false,
            );
            let mut names = [""; 32];
            for (i, n) in names[..31].iter_mut().enumerate() {
                *n = drill::IndexId::ALL[i].name()
            }
            names[31] = "Mixed targets";
            w.list(&names, a.selected, None)
        }
        View::DeviationQuestion => {
            if let Some(q) = pending {
                let c = q.context;
                w.title("", None, false);
                let mut header = Text::<80>::new();
                let _ = write!(
                    header,
                    "{}D {} {} {}",
                    c.rules.decks,
                    if c.rules.hit_soft_17 { "H17" } else { "S17" },
                    if c.rules.double_after_split {
                        "DAS"
                    } else {
                        "NO DAS"
                    },
                    if c.rules.late_surrender {
                        "LS"
                    } else {
                        "NO LS"
                    }
                );
                w.label(
                    (8, 1),
                    header.as_str(),
                    TextStyle {
                        size: 13,
                        bold: true,
                        color: GOLD,
                        align: Align::Left,
                    },
                );
                at(&mut w, 190, 3, "DEALER", 11, MUTED);
                for (i, &card) in c.hand.cards().iter().take(3).enumerate() {
                    w.renderer
                        .card(w.frame, (8 + i as i32 * 57, 20), Some(card), false);
                }
                w.renderer.card(w.frame, (196, 20), Some(c.dealer_up), true);
                at(
                    &mut w,
                    190,
                    74,
                    text(&mut b, format_args!("RC{:+}", c.count.running)),
                    11,
                    WHITE,
                );
                at(
                    &mut w,
                    190,
                    89,
                    text(
                        &mut b,
                        format_args!("TC{:+}", c.count.integer_true_count(c.rules).unwrap_or(0)),
                    ),
                    11,
                    WHITE,
                );
                let (options, n) = choices(q);
                let selected = if a.selected < n {
                    choice(options[a.selected])
                } else if a.selected == n {
                    "Original rules / count"
                } else if a.selected == n + 1 {
                    "Show reference (assisted)"
                } else {
                    "Cancel question"
                };
                w.row(103, selected, true, true);
                w.footer("GREY: NEXT   BLUE: SELECT")
            } else {
                w.title("NO ACTIVE DRILL", None, false);
                w.footer("BLUE: DRILLS")
            }
        }
        View::DeviationContext => {
            if let Some(q) = pending {
                context(&mut w, q, a.page)
            } else {
                w.title("NO ACTIVE DRILL", None, false)
            }
            w.footer("GREY: PAGE   BLUE: QUESTION")
        }
        View::DeviationReference => {
            if let (Some(q), Some(r)) = (pending, pending_ref) {
                reference(&mut w, q, r, a.page)
            } else {
                w.title("REFERENCE IS GATED", None, false);
                label(&mut w, 50, "Submit or explicitly request help", 11, MUTED)
            }
            w.footer("GREY: PAGE   BLUE: QUESTION")
        }
        View::DeviationResult => {
            if let Some(record) = attempt {
                result(&mut w, record)
            } else {
                w.title("NO DRILL ATTEMPTS", None, false)
            }
            w.footer("GREY: AGAIN   BLUE: DRILLS")
        }
        View::DeviationHistory => {
            if let Some(record) = attempt {
                match a.page % 6 {
                    0 => result(&mut w, record),
                    1 => reference(&mut w, record.question, record.reference, 0),
                    2 => reference(&mut w, record.question, record.reference, 1),
                    n => context(&mut w, record.question, n - 3),
                }
            } else {
                w.title("NO DRILL ATTEMPTS", None, false)
            }
            w.footer("GREY: NEXT   BLUE: DRILLS")
        }
        View::DeviationStats => {
            let s = a.deviation_practice.stats();
            if a.page == 0 {
                totals(&mut w, s.first.all, "Original first attempts / all targets")
            } else if a.page == 1 {
                totals(&mut w, s.replay.all, "Replays only / originals unchanged")
            } else {
                let i = (a.page - 2).min(30);
                totals(&mut w, s.first.indices[i], drill::IndexId::ALL[i].name())
            }
            w.footer("GREY: CATEGORY   BLUE: DRILLS")
        }
        View::DeviationPolicy => {
            w.title(
                "DRILL POLICY / LIMITS",
                Some("BJA2018 Hi-Lo / signed truncated TC"),
                false,
            );
            let lines = match a.page % 4 {
                0 => [
                    "Published H17/S17 index policy",
                    "Count / explicit remaining-deck divisor",
                    "Discard fractional part toward zero",
                    "Strict positive/negative RC at zero",
                    "Synthetic questions; no live shoe input",
                ],
                1 => [
                    "Initial legal pair/surrender priority",
                    "Never invent unavailable actions",
                    "Double/split require funding",
                    "Insurance at TC+3 if affordable",
                    "Not exact composition-dependent EV",
                ],
                2 => [
                    "Question reference stays gated",
                    "Show reference marks assistance",
                    "First submitted answer is immutable",
                    "Missed replays use original evidence",
                    "Replay scores cannot fix originals",
                ],
                _ => [
                    "Generic casino-style rule profiles",
                    "Max4 hands / no resplit aces",
                    "One card to split aces; 21 pays1:1",
                    "No claim of a named casino match",
                    "Actual game uses its own saved rules",
                ],
            };
            for (i, line) in lines.into_iter().enumerate() {
                label(
                    &mut w,
                    35 + i as i32 * 17,
                    line,
                    9,
                    if i == 0 { WHITE } else { MUTED },
                )
            }
            w.footer("GREY: PAGE   BLUE: DRILLS")
        }
        _ => {}
    }
    if let Some(mode) = a.toast {
        w.mode_footer(mode)
    }
    true
}
