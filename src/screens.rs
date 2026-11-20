//! Readable 240×135 screens built with the accepted renderer/menu geometry.
use crate::{
    app::{App, MENU, PROFILES, View},
    count_practice::{self, Answer, Kind},
    engine::{Action, Phase},
    render::{Align, TextStyle, text_width},
    table,
    text::{Cash, Text},
    training::{Choice, Counters, Decision, ExpectedValues, Policy},
    widgets::{GOLD, MUTED, WHITE, Widgets},
};
use core::fmt::Write;
fn label(w: &mut Widgets<'_>, x: i32, y: i32, s: &str, size: u8, color: u16, bold: bool) {
    let mut size = size;
    while size > 8 && text_width(s, size, bold) > 224 {
        size -= 1
    }
    w.label(
        (x, y),
        s,
        TextStyle {
            size,
            bold,
            color,
            align: Align::Left,
        },
    );
}
fn line(w: &mut Widgets<'_>, y: i32, s: &str) {
    label(w, 8, y, s, 13, WHITE, false)
}
fn paragraph(w: &mut Widgets<'_>, y: i32, s: &str, size: u8, max: usize) {
    let mut rest = s;
    for row in 0..max {
        if rest.is_empty() {
            break;
        }
        let mut end = rest.len();
        for (i, c) in rest.char_indices() {
            if c == '\n' {
                end = i;
                break;
            }
            if text_width(&rest[..i + c.len_utf8()], size, false) > 220 {
                end = rest[..i].rfind(' ').unwrap_or(i);
                break;
            }
        }
        if end == 0 {
            break;
        }
        label(
            w,
            8,
            y + row as i32 * (size as i32 + 3),
            &rest[..end],
            size,
            MUTED,
            false,
        );
        rest = rest[end..].trim_start();
    }
}
fn choice(c: Choice) -> &'static str {
    match c {
        Choice::Play(a) => table::action_name(a),
        Choice::Insurance(true) => "INSURE",
        Choice::Insurance(false) => "DECLINE INSURANCE",
    }
}
fn kind(k: Kind) -> &'static str {
    match k {
        Kind::Running => "RUNNING COUNT",
        Kind::Decks => "DECKS LEFT",
        Kind::TrueCount => "TRUE COUNT",
    }
}
fn text<'a>(b: &'a mut Text<128>, args: core::fmt::Arguments<'_>) -> &'a str {
    b.clear();
    let _ = b.write_fmt(args);
    b.as_str()
}
fn counters(w: &mut Widgets<'_>, c: Counters) {
    let mut b = Text::new();
    label(
        w,
        8,
        33,
        text(
            &mut b,
            format_args!("Correct {} / {}", c.correct, c.correct + c.errors),
        ),
        16,
        GOLD,
        true,
    );
    line(w, 59, text(&mut b, format_args!("Errors {}", c.errors)));
    line(w, 80, text(&mut b, format_args!("Assisted {}", c.assisted)));
    label(
        w,
        8,
        104,
        "Assisted attempts stay separate",
        10,
        MUTED,
        false,
    );
}
fn context(d: Decision) -> Text<128> {
    let mut b = Text::new();
    if matches!(d.attempt.choice, Choice::Insurance(_)) {
        let _ = write!(b, "Insurance vs ace");
    } else {
        let h = d.attempt.hand;
        let up = d.attempt.dealer_up.value();
        let _ = write!(
            b,
            "{} {} vs {}",
            if h.soft() { "Soft" } else { "Hard" },
            h.total(),
            if up == 11 {
                "A"
            } else {
                match up {
                    2 => "2",
                    3 => "3",
                    4 => "4",
                    5 => "5",
                    6 => "6",
                    7 => "7",
                    8 => "8",
                    9 => "9",
                    _ => "10",
                }
            }
        );
    }
    b
}
fn review(w: &mut Widgets<'_>, d: Option<Decision>, page: usize, count: usize, coach: bool) {
    let Some(d) = d else {
        line(w, 48, "No decisions recorded");
        line(w, 77, "History stays in this session");
        w.footer("BLUE: BACK");
        return;
    };
    let mut b = Text::new();
    let c = context(d);
    label(w, 8, 24, c.as_str(), 14, GOLD, true);
    line(
        w,
        44,
        text(&mut b, format_args!("You: {}", choice(d.attempt.choice))),
    );
    line(
        w,
        62,
        text(&mut b, format_args!("Best: {}", choice(d.best))),
    );
    paragraph(w, 82, d.explanation, 10, 2);
    label(
        w,
        8,
        109,
        if d.assisted {
            "ASSISTED / first answer retained"
        } else if d.correct {
            "CORRECT / first answer retained"
        } else {
            "MISTAKE / first answer retained"
        },
        9,
        MUTED,
        false,
    );
    if coach {
        w.row(83, "Play my choice", false, true);
        w.row(102, "Use best action", false, true);
    } else {
        label(
            w,
            188,
            5,
            text(&mut b, format_args!("{}/{}", page + 1, count.max(1))),
            9,
            MUTED,
            false,
        );
        w.footer("GREY: HISTORY   BLUE: BACK")
    }
}
fn ev(
    w: &mut Widgets<'_>,
    v: Option<ExpectedValues>,
    insurance: bool,
    page: usize,
    history: Option<Decision>,
) {
    let mut b = Text::new();
    let Some(v) = v else {
        line(w, 50, "EV snapshot unavailable");
        line(w, 77, "This attempt has no model snapshot");
        w.footer("BLUE: BACK");
        return;
    };
    label(w, 8, 22, "Infinite-deck replacement", 11, GOLD, true);
    let index = if let Some(d) = history {
        match d.attempt.choice {
            Choice::Play(a) => a as usize,
            Choice::Insurance(y) => usize::from(y),
        }
    } else {
        (0..5)
            .filter(|i| v.legal_mask & (1 << i) != 0)
            .nth(page)
            .unwrap_or(0)
    };
    let name = if insurance {
        if index == 0 {
            "DECLINE INSURANCE"
        } else {
            "INSURE (half wager)"
        }
    } else {
        table::action_name(Action::ALL[index])
    };
    label(w, 8, 37, name, 15, WHITE, true);
    let available = v.evaluated_mask & (1 << index) != 0;
    if available {
        label(
            w,
            8,
            56,
            text(&mut b, format_args!("Net {:+.4} units", v.values[index])),
            17,
            WHITE,
            true,
        );
        let cents = {
            let cents = v.values[index] * v.original_wager as f64;
            (cents + if cents < 0.0 { -0.5 } else { 0.5 }) as i64
        };
        label(
            w,
            8,
            77,
            text(&mut b, format_args!("Expected net {}", Cash(cents, true))),
            16,
            GOLD,
            true,
        );
    } else {
        label(w, 8, 57, "N/A - split EV unavailable", 16, WHITE, true)
    }
    label(
        w,
        8,
        97,
        text(
            &mut b,
            format_args!("1u = {} original wager", Cash(v.original_wager, false)),
        ),
        10,
        MUTED,
        false,
    );
    if history.is_some() && v.complete() && available {
        let mut best = f64::NEG_INFINITY;
        for i in 0..5 {
            if v.evaluated_mask & (1 << i) != 0 {
                best = best.max(v.values[i])
            }
        }
        label(
            w,
            8,
            110,
            text(
                &mut b,
                format_args!("Model loss {:.4}u", (best - v.values[index]).max(0.0)),
            ),
            10,
            MUTED,
            false,
        )
    } else {
        label(
            w,
            8,
            110,
            if !v.complete() {
                "Incomplete: split EV unavailable"
            } else if v.initial_pair_funded {
                "Initial pair / full funding"
            } else {
                "Fixed odds; no count adjustment"
            },
            10,
            MUTED,
            false,
        )
    }
    w.footer(if history.is_some() {
        "GREY: HISTORY   BLUE: BACK"
    } else {
        "GREY: ACTION   BLUE: BACK"
    });
}
fn count_result(w: &mut Widgets<'_>, a: Option<count_practice::Attempt>, history: bool) {
    let Some(a) = a else {
        w.title("COUNT HISTORY", None, false);
        line(w, 51, "No count attempts");
        w.footer("BLUE: BACK");
        return;
    };
    w.title(
        if a.assisted {
            "COUNT ASSISTED"
        } else if a.correct {
            "COUNT CORRECT"
        } else {
            "COUNT MISTAKE"
        },
        Some(kind(a.question.kind)),
        false,
    );
    let mut b = Text::new();
    match a.answer {
        Answer::Decks(h) => line(
            w,
            35,
            text(&mut b, format_args!("Your answer {}.{}", h / 2, h % 2 * 5)),
        ),
        Answer::Running(n) | Answer::TrueCount(n) => {
            line(w, 35, text(&mut b, format_args!("Your answer {:+}", n)))
        }
    }
    let r = a.reference;
    match a.question.kind {
        Kind::Running => label(
            w,
            8,
            58,
            text(&mut b, format_args!("Reference {:+}", r.running)),
            18,
            GOLD,
            true,
        ),
        Kind::Decks => {
            label(
                w,
                8,
                58,
                text(
                    &mut b,
                    format_args!("Reference {:.1}", r.rounded_half_decks as f64 / 2.0),
                ),
                18,
                GOLD,
                true,
            );
            label(
                w,
                8,
                83,
                text(
                    &mut b,
                    format_args!("Actual {:.2} decks; tolerance 0.5", r.exact_decks),
                ),
                11,
                MUTED,
                false,
            )
        }
        Kind::TrueCount => {
            label(
                w,
                8,
                58,
                text(
                    &mut b,
                    format_args!(
                        "Reference {:+}",
                        r.given_index.or(r.physical_index).unwrap_or(0)
                    ),
                ),
                18,
                GOLD,
                true,
            );
            label(
                w,
                8,
                83,
                text(
                    &mut b,
                    format_args!(
                        "RC {:+} / {:.1}; truncate",
                        r.running,
                        a.question
                            .given_half_decks
                            .map_or(r.exact_decks, |h| h as f64 / 2.0)
                    ),
                ),
                11,
                MUTED,
                false,
            )
        }
    }
    label(
        w,
        8,
        104,
        if a.assisted {
            "Assisted; excluded from accuracy"
        } else {
            "First answer retained"
        },
        10,
        MUTED,
        false,
    );
    w.footer(if history {
        "GREY: HISTORY   BLUE: BACK"
    } else {
        "GREY: AGAIN   BLUE: BACK"
    });
}
/// Drawing table advice/counts returns explicit aid bits; once viewed, the
/// decision/checkpoint remains assisted even after that aid is switched off.
pub fn draw(a: &mut App) {
    a.renderer.set_appearance(a.compact_black, a.card_back);
    if a.view == View::CardBackPreview {
        let mut w = Widgets {
            frame: &mut a.frame,
            renderer: &mut a.renderer,
        };
        w.title("CARD BACK", None, true);
        w.renderer.card_back(w.frame, (24, 26), (60, 88));
        w.label(
            (157, 59),
            crate::round::CARD_BACK_NAMES[a.card_back.min(2) as usize],
            TextStyle {
                size: 24,
                bold: true,
                color: GOLD,
                align: Align::Center,
            },
        );
        w.footer("GREY: NEXT   BLUE: DONE");
        return;
    }
    if crate::app::menu_ui::draw(a) {
        return;
    }
    if crate::app::settings_ui::draw(a) {
        return;
    }
    if crate::app::session_ui::draw(a) {
        return;
    }
    if crate::app::deviation_ui::draw(a) {
        return;
    }
    if crate::app::table_setup::draw(a) {
        return;
    }
    if crate::app::wager_ui::draw(a) {
        return;
    }
    if a.view == View::Table {
        let exposure = table::draw(
            &mut a.frame,
            &mut a.renderer,
            &a.game,
            &mut a.tracker,
            table::Options {
                mode: a.training.mode,
                policy: a.policy,
                normal_counter: a.count_display,
                guide_counter: a.guide_count_display,
                reveal: a.reveal,
                result_hand: a.result_hand,
                insurance_yes: a.selected == 1,
                toast: a.toast,
            },
        );
        if exposure.play_assisted || exposure.count_aids != 0 {
            a.mark_assisted()
        }
        if exposure.count_aids != 0 {
            a.mark_aid(exposure.count_aids)
        }
        return;
    }
    if a.view == View::Cut {
        let mut w = Widgets {
            frame: &mut a.frame,
            renderer: &mut a.renderer,
        };
        w.cut(a.game.rules.decks, a.cut_index);
        if let Some(m) = a.toast {
            w.mode_footer(m)
        }
        return;
    }
    let round_count = a.round_decisions();
    let review_d = if matches!(a.view, View::TrainingReview | View::RoundReview) {
        a.review_decision()
    } else {
        None
    };
    let mut w = Widgets {
        frame: &mut a.frame,
        renderer: &mut a.renderer,
    };
    let mut b = Text::<128>::new();
    match a.view {
        View::Burn => {
            w.title("SHOE IS READY", None, true);
            line(&mut w, 39, "Dealer burns one unseen card");
            line(&mut w, 65, "Your cut is in place");
            w.footer("BLUE: DEAL   HOLD GREY: MENU")
        }
        View::Menu => {
            w.title("MENU", None, false);
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
            let (actions, n) = {
                let mut actions = [Action::Double; 3];
                let mut n = 0;
                if a.reveal.is_none() && a.game.phase != Phase::Dealer {
                    for action in [Action::Double, Action::Split, Action::Surrender] {
                        if a.game.legal(action) {
                            actions[n] = action;
                            n += 1
                        }
                    }
                }
                (actions, n)
            };
            let mut items = [""; 14];
            for i in 0..n {
                items[i] = table::action_name(actions[i])
            }
            items[n..n + 11].copy_from_slice(&MENU[..11]);
            w.list(&items[..n + 11], a.selected, None)
        }
        View::Wager => {
            w.title(
                "SELECT WAGER",
                Some("Virtual chips / $5 table steps"),
                false,
            );
            let items = [
                "Wager $5",
                "Wager $10",
                "Wager $25",
                "Wager $100",
                "Custom wager",
                "Return",
            ];
            let top = a.selected.saturating_sub(3).min(2);
            for i in top..(top + 4).min(6) {
                let enabled = i >= 4
                    || ([500, 1000, 2500, 10000][i] >= a.game.rules.minimum
                        && [500, 1000, 2500, 10000][i] <= a.game.rules.maximum);
                w.row(
                    30 + (i - top) as i32 * 19,
                    items[i],
                    a.selected == i,
                    enabled,
                )
            }
            if let Some(next) = items.get(top + 4) {
                w.peek(next);
            }
            w.footer("GREY: NEXT   BLUE: SELECT")
        }
        View::Amount | View::ProfileAmount => {
            w.title(
                if a.view == View::Amount {
                    "CUSTOM WAGER"
                } else if a.minimum_edit {
                    "MINIMUM BET"
                } else {
                    "MAXIMUM BET"
                },
                Some(text(
                    &mut b,
                    format_args!("Amount {}", Cash(a.amount, false)),
                )),
                false,
            );
            w.list(
                &[
                    "Add $5",
                    "Subtract $5",
                    "Add $25",
                    "Subtract $25",
                    "Accept amount",
                ],
                a.selected,
                None,
            )
        }
        View::Training => {
            w.title(
                "TRAINING",
                Some(if a.policy == Policy::HiLo {
                    "BJA Hi-Lo index policy / truncated TC"
                } else {
                    "Rule-bound basic strategy"
                }),
                false,
            );
            let mut mode = Text::<48>::new();
            let _ = write!(mode, "Mode: {}", a.training.mode.name());
            let mut policy = Text::<48>::new();
            let _ = write!(
                policy,
                "Strategy: {}",
                if a.policy == Policy::Basic {
                    "BASIC"
                } else {
                    "HI-LO"
                }
            );
            w.list(
                &[
                    mode.as_str(),
                    policy.as_str(),
                    "Count practice",
                    "Accuracy",
                    "Review decisions",
                    "Action EV",
                    "EV review",
                    "Hint",
                    "Wager practice",
                    "Deviation drills",
                    "Return",
                ],
                a.selected,
                None,
            )
        }
        View::TrainingStats => {
            let s = a.training.stats();
            w.title(
                [
                    "TRAINING ACCURACY",
                    "HARD TOTALS",
                    "SOFT TOTALS",
                    "PAIRS",
                    "INSURANCE",
                ][a.page.min(4)],
                None,
                false,
            );
            counters(
                &mut w,
                if a.page == 0 {
                    s.all
                } else {
                    s.categories[a.page - 1]
                },
            );
            w.footer("GREY: CATEGORY   BLUE: BACK")
        }
        View::TrainingReview => {
            w.title("DECISION REVIEW", None, false);
            review(&mut w, review_d, a.page, a.training.history_count(), false)
        }
        View::RoundReview => {
            w.title("ROUND REVIEW", None, false);
            if a.page == 0 {
                let s = a.training.round_stats(a.round_id);
                line(
                    &mut w,
                    29,
                    text(
                        &mut b,
                        format_args!("Correct {} / {}", s.correct, s.correct + s.errors),
                    ),
                );
                line(
                    &mut w,
                    49,
                    text(
                        &mut b,
                        format_args!("Errors {} / assisted {}", s.errors, s.assisted),
                    ),
                );
                line(
                    &mut w,
                    73,
                    text(
                        &mut b,
                        format_args!("Round {}", Cash(a.game.round_net, true)),
                    ),
                );
                label(
                    &mut w,
                    8,
                    101,
                    "GREY reviews each first decision",
                    11,
                    MUTED,
                    false,
                );
            } else {
                review(&mut w, review_d, a.page - 1, round_count, false)
            }
            w.footer("GREY: REVIEW   BLUE: TABLE")
        }
        View::Hint => {
            w.title("TRAINING HINT", None, false);
            if let Some((best, why)) = a.hint {
                label(&mut w, 8, 28, choice(best), 20, GOLD, true);
                paragraph(&mut w, 57, why, 12, 4);
            }
            label(
                &mut w,
                8,
                109,
                "Current decision is now assisted",
                10,
                MUTED,
                false,
            );
            w.footer("BLUE: BACK")
        }
        View::CoachCheck => {
            w.title(
                "COACH CHECK",
                Some("First answer recorded before acting"),
                false,
            );
            if let Some(d) = a.pending {
                line(
                    &mut w,
                    31,
                    text(&mut b, format_args!("You: {}", choice(d.attempt.choice))),
                );
                line(
                    &mut w,
                    49,
                    text(&mut b, format_args!("Best: {}", choice(d.best))),
                );
                paragraph(&mut w, 66, d.explanation, 9, 1);
            }
            w.row(82, "Play my choice", a.selected == 0, true);
            w.row(102, "Use best action", a.selected == 1, true);
            w.footer("GREY: NEXT   BLUE: PLAY")
        }
        View::ActionEv => {
            w.title("ACTION EV", None, false);
            ev(
                &mut w,
                a.live_ev,
                a.game.phase == Phase::Insurance,
                a.page,
                None,
            )
        }
        View::EvReview => {
            let d = a.training.newest(a.page).copied();
            w.title("EV REVIEW", None, false);
            ev(
                &mut w,
                d.and_then(|d| d.attempt.values),
                d.is_some_and(|d| matches!(d.attempt.choice, Choice::Insurance(_))),
                a.page,
                d,
            )
        }
        View::CountPractice => {
            w.title(
                "COUNT PRACTICE",
                Some("Checkpoints; each skill scores separately"),
                false,
            );
            w.list(
                &[
                    "Running count",
                    "Decks left",
                    "True count",
                    "Show count info",
                    "Count accuracy",
                    "Count history",
                    "Return",
                ],
                a.selected,
                None,
            )
        }
        View::CountAnswer => {
            w.title(kind(a.count_kind), None, false);
            if let Some(q) = a.count_practice.question() {
                if a.count_kind == Kind::Decks {
                    line(&mut w, 23, "Estimate the stack in decks");
                    let h = a.count_practice.stack_height(q.token).unwrap_or(1) as i32;
                    for (x, h) in [(10, 9), (70, h)] {
                        w.frame
                            .rect(x, 110 - h, 47, h, crate::canvas::rgb(191, 185, 161));
                        for y in (112 - h..110).step_by(3) {
                            w.frame.rect(x, y, 47, 1, crate::canvas::rgb(87, 82, 67))
                        }
                        w.frame.rect(x - 1, 107 - h, 49, 4, GOLD)
                    }
                    label(&mut w, 10, 83, "1 deck", 11, WHITE, false);
                    label(&mut w, 145, 53, "ANSWER", 11, MUTED, true);
                    label(
                        &mut w,
                        157,
                        76,
                        text(
                            &mut b,
                            format_args!("{}.{}", a.count_answer / 2, a.count_answer % 2 * 5),
                        ),
                        24,
                        GOLD,
                        true,
                    );
                } else {
                    if let Some(h) = q.given_half_decks {
                        line(
                            &mut w,
                            28,
                            text(
                                &mut b,
                                format_args!("Your RC / {:.1} decks", h as f64 / 2.0),
                            ),
                        )
                    } else {
                        line(&mut w, 28, "Enter the count you kept")
                    }
                    label(
                        &mut w,
                        90,
                        55,
                        text(&mut b, format_args!("{:+}", a.count_answer)),
                        26,
                        GOLD,
                        true,
                    );
                    label(
                        &mut w,
                        8,
                        92,
                        if a.count_kind == Kind::TrueCount {
                            "Drop decimals toward zero"
                        } else {
                            "Both buttons submit once"
                        },
                        11,
                        MUTED,
                        false,
                    )
                }
                if let Some(r) = a.count_practice.reference(q.token) {
                    label(
                        &mut w,
                        8,
                        108,
                        text(
                            &mut b,
                            format_args!(
                                "Help: RC {:+} / TC {:+}",
                                r.running,
                                r.given_index.or(r.physical_index).unwrap_or(0)
                            ),
                        ),
                        10,
                        MUTED,
                        false,
                    )
                }
            }
            w.footer("GREY: -   BLUE: +   BOTH: SUBMIT")
        }
        View::CountResult | View::CountHistory => count_result(
            &mut w,
            a.count_practice
                .newest(if a.view == View::CountHistory {
                    a.page
                } else {
                    0
                })
                .copied(),
            a.view == View::CountHistory,
        ),
        View::CountStats => {
            w.title(
                [
                    "COUNT ACCURACY",
                    "RUNNING COUNT",
                    "DECK ESTIMATE",
                    "TRUE COUNT",
                ][a.page.min(3)],
                None,
                false,
            );
            let s = a.count_practice.stats();
            counters(
                &mut w,
                if a.page == 0 {
                    s.all
                } else {
                    s.kinds[a.page - 1]
                },
            );
            w.footer("GREY: SKILL   BLUE: BACK")
        }
        View::CountInfo => {
            w.title(
                "COUNT INFO",
                Some("Exposed faces / physical undealt cards"),
                false,
            );
            let left = a.game.total_cards().saturating_sub(a.game.position) as u16;
            label(
                &mut w,
                8,
                33,
                text(&mut b, format_args!("RC {:+}", a.tracker.running_count())),
                23,
                GOLD,
                true,
            );
            if let Some(tc) = a.tracker.true_count(left) {
                label(
                    &mut w,
                    8,
                    62,
                    text(&mut b, format_args!("TC {:+.2}", tc)),
                    20,
                    WHITE,
                    true,
                )
            } else {
                line(&mut w, 62, "TC unavailable")
            }
            line(
                &mut w,
                92,
                text(&mut b, format_args!("Left {:.2} decks", left as f64 / 52.0)),
            );
            w.footer("BLUE: BACK")
        }
        View::Settings
        | View::Sound
        | View::AudioVolume
        | View::Display
        | View::Brightness
        | View::TableMenu
        | View::SessionMenu
        | View::SessionOverview
        | View::BankrollMenu
        | View::NewSessionConfirm
        | View::ResetStatsConfirm
        | View::HelpMenu
        | View::TrainingModes
        | View::LiveCounts
        | View::PracticeMenu
        | View::ProgressMenu
        | View::AdviceMenu
        | View::SoundCategories
        | View::SoundTests
        | View::HandProgress
        | View::CountProgress
        | View::WagerProgress
        | View::DeviationProgress => {}
        View::Profile => {
            w.title(
                "TABLE PROFILE",
                Some(PROFILES[a.pending_rules.profile as usize]),
                false,
            );
            let mut min = Text::<40>::new();
            let _ = write!(min, "Minimum: {}", Cash(a.pending_rules.minimum, false));
            let mut max = Text::<40>::new();
            let _ = write!(max, "Maximum: {}", Cash(a.pending_rules.maximum, false));
            let mut depth = Text::<40>::new();
            let _ = write!(depth, "Dealer cut: {}%", a.pending_rules.penetration);
            w.list(
                &[
                    "Choose preset",
                    min.as_str(),
                    max.as_str(),
                    depth.as_str(),
                    "Other seats / your place",
                    "Review table cards",
                    "Apply profile",
                    "Return",
                ],
                a.selected,
                None,
            )
        }
        View::ProfilePreset => {
            w.title("CHOOSE PRESET", None, false);
            w.list(
                &[
                    PROFILES[0],
                    PROFILES[1],
                    PROFILES[2],
                    PROFILES[3],
                    PROFILES[4],
                    PROFILES[5],
                    "Return",
                ],
                a.selected,
                None,
            )
        }
        View::ProfileDepth => {
            w.title(
                "DEALER CUT DEPTH",
                Some("Marker controls when the next shoe starts"),
                false,
            );
            w.list(
                &["50%", "60%", "70%", "75%", "80%", "85%", "90%", "Return"],
                a.selected,
                None,
            )
        }
        View::ShoeInfo => {
            w.title("SHOE INFORMATION", None, false);
            line(
                &mut w,
                28,
                text(
                    &mut b,
                    format_args!("Shoe {} / {} decks", a.game.shoe_number, a.game.rules.decks),
                ),
            );
            line(
                &mut w,
                48,
                text(
                    &mut b,
                    format_args!("Dealt {} / {}", a.game.position, a.game.total_cards()),
                ),
            );
            line(
                &mut w,
                68,
                text(
                    &mut b,
                    format_args!("Dealer marker {}%", a.game.rules.penetration),
                ),
            );
            line(
                &mut w,
                88,
                if a.game.needs_shuffle() {
                    "Shuffle before next round"
                } else {
                    "Finish ordinary rounds before shuffle"
                },
            );
            w.footer("BLUE: BACK")
        }
        View::Rules => {
            w.title(
                "TABLE RULES",
                Some(PROFILES[a.game.rules.profile as usize]),
                false,
            );
            let r = a.game.rules;
            match a.page {
                0 => {
                    line(
                        &mut w,
                        35,
                        text(
                            &mut b,
                            format_args!(
                                "{} decks / dealer {}",
                                r.decks,
                                if r.hit_soft_17 {
                                    "hits soft 17"
                                } else {
                                    "stands soft 17"
                                }
                            ),
                        ),
                    );
                    line(
                        &mut w,
                        63,
                        text(
                            &mut b,
                            format_args!(
                                "Blackjack pays {}:{}",
                                r.blackjack_numerator, r.blackjack_denominator
                            ),
                        ),
                    );
                    line(
                        &mut w,
                        91,
                        text(
                            &mut b,
                            format_args!(
                                "Limits {} - {}",
                                Cash(r.minimum, false),
                                Cash(r.maximum, false)
                            ),
                        ),
                    )
                }
                1 => paragraph(
                    &mut w,
                    33,
                    "Double any initial two cards. Double after split follows the selected profile. Split to four hands; no resplitting aces. Split aces receive one card.",
                    13,
                    5,
                ),
                2 => paragraph(
                    &mut w,
                    33,
                    if r.late_surrender {
                        "Late surrender returns half the wager after dealer checks for blackjack. Unsplit initial hands only."
                    } else {
                        "Surrender is not available under this table profile."
                    },
                    13,
                    5,
                ),
                3 => paragraph(
                    &mut w,
                    33,
                    "US hole card and ace/ten peek. Insurance costs half the original wager and pays 2:1. Basic strategy declines this side bet.",
                    13,
                    5,
                ),
                _ => paragraph(
                    &mut w,
                    33,
                    "Virtual chips. Fresh session starts at $1000. Saves retain preferences and accuracy; shoe order, active hands and history stay in RAM.",
                    13,
                    5,
                ),
            }
            w.footer("GREY: PAGE   BLUE: BACK")
        }
        View::Session => {
            w.title(
                "SESSION TOTALS",
                Some(text(
                    &mut b,
                    format_args!(
                        "{} rounds / net {}",
                        a.game.rounds,
                        Cash(a.game.net(), true)
                    ),
                )),
                false,
            );
            w.list(
                &[
                    "Return",
                    "Bankroll details",
                    "Add $1000 chips",
                    "Session analysis",
                    "End session",
                ],
                a.selected,
                None,
            )
        }
        View::Bankroll => {
            w.title("BANKROLL", None, false);
            if a.page == 0 {
                line(
                    &mut w,
                    31,
                    text(
                        &mut b,
                        format_args!("Chips {}", Cash(a.game.bankroll, false)),
                    ),
                );
                line(
                    &mut w,
                    53,
                    text(
                        &mut b,
                        format_args!("Deposited {}", Cash(a.game.deposits, false)),
                    ),
                );
                line(
                    &mut w,
                    75,
                    text(
                        &mut b,
                        format_args!("Cashed out {}", Cash(a.game.cashouts, false)),
                    ),
                );
                line(
                    &mut w,
                    97,
                    text(&mut b, format_args!("Net {}", Cash(a.game.net(), true))),
                )
            } else {
                line(
                    &mut w,
                    32,
                    text(
                        &mut b,
                        format_args!("Wins {} / losses {}", a.game.wins, a.game.losses),
                    ),
                );
                line(
                    &mut w,
                    57,
                    text(
                        &mut b,
                        format_args!("Pushes {} / rounds {}", a.game.pushes, a.game.rounds),
                    ),
                );
                line(
                    &mut w,
                    82,
                    text(
                        &mut b,
                        format_args!("Committed {}", Cash(a.game.committed(), false)),
                    ),
                )
            }
            w.footer("GREY: PAGE   BLUE: BACK")
        }
        View::Cards => {
            let dealer = a.card_group == a.game.hand_count;
            w.title(
                if dealer {
                    "DEALER CARDS"
                } else {
                    "PLAYER CARDS"
                },
                Some("Only exposed faces are available"),
                false,
            );
            let h = if dealer {
                &a.game.dealer
            } else {
                &a.game.hands[a.card_group]
            };
            let visible = if dealer && !a.game.hole_visible {
                1
            } else {
                h.count()
            };
            for (i, c) in h
                .cards()
                .iter()
                .take(visible)
                .skip(a.card_page * 4)
                .take(4)
                .enumerate()
            {
                w.renderer
                    .card(w.frame, (13 + i as i32 * 55, 37), Some(*c), true)
            }
            label(
                &mut w,
                8,
                108,
                text(
                    &mut b,
                    format_args!("Group {} / {}", a.card_group + 1, a.game.hand_count + 1),
                ),
                10,
                MUTED,
                false,
            );
            w.footer("GREY: CARDS   BLUE: BACK")
        }
        View::Controls => {
            w.title("TWO BUTTONS", None, false);
            paragraph(
                &mut w,
                32,
                [
                    "GREY tap: hit during your turn. BLUE tap: stand. Between rounds, GREY selects wager and BLUE deals.",
                    "Hold GREY: open the menu; inside menus, go back one level. GREY moves; BLUE selects. Double, split and surrender come first.",
                    "Hold BLUE: cycle Off, Test, Coach, Guide. Holds consume their release; releasing does not play a card.",
                    "Count quizzes: GREY minus, BLUE plus; both submit once. Hold GREY cancels the question and goes back.",
                ][a.page.min(3)],
                13,
                5,
            );
            w.footer("GREY: PAGE   BLUE: BACK")
        }
        View::Leave => {
            w.title(
                "END SESSION",
                Some("End this session and close the table"),
                false,
            );
            w.list(&["Keep playing", "End session"], a.selected, None)
        }
        View::Summary => {
            w.title(
                "SESSION COMPLETE",
                Some(text(
                    &mut b,
                    format_args!(
                        "Net {} / {} rounds",
                        Cash(a.game.net(), true),
                        a.game.rounds
                    ),
                )),
                false,
            );
            w.list(&["New $1000 session", "Table rules"], a.selected, None)
        }
        View::Refill => {
            w.title("ADD VIRTUAL CHIPS", None, false);
            label(&mut w, 8, 37, "REFILL $1000", 23, GOLD, true);
            line(&mut w, 75, "Session totals retain deposits");
            w.footer("BLUE: REFILL   HOLD GREY: MENU")
        }
        View::Notice => {
            w.title("TABLE NOTICE", None, false);
            paragraph(&mut w, 42, a.notice, 15, 4);
            w.footer("BLUE: RETURN")
        }
        View::AnalysisMenu
        | View::AnalysisSummary
        | View::AnalysisGraph
        | View::AnalysisRounds
        | View::AnalysisAccuracy
        | View::AnalysisCosts
        | View::AnalysisDecisions
        | View::AnalysisAssumptions
        | View::DeviationPractice
        | View::DeviationTarget
        | View::DeviationQuestion
        | View::DeviationContext
        | View::DeviationReference
        | View::DeviationResult
        | View::DeviationStats
        | View::DeviationHistory
        | View::DeviationPolicy
        | View::TableSeats
        | View::SeatCards
        | View::WagerPractice
        | View::WagerAnswer
        | View::WagerHelp
        | View::WagerResult
        | View::WagerStats
        | View::WagerHistory
        | View::WagerPolicy
        | View::Cut
        | View::CardBackPreview
        | View::Table => {}
    }
    if let Some(m) = a.toast {
        w.mode_footer(m)
    }
}
