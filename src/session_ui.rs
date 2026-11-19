//! Menu-only descriptive session reports. Normal table rendering/controls stay
//! unchanged; recording consumes settled ledgers and immutable first answers.
use super::*;
use crate::{
    canvas::{Canvas, rgb},
    render::{Align, TextStyle, text_width},
    session_analysis as analysis, session_metrics as evidence,
    text::{Cash, Text},
    widgets::{GOLD, LINE, MUTED, WHITE, Widgets},
};
use core::fmt::Write;
const RED: u16 = rgb(255, 91, 108);
const GREEN: u16 = rgb(71, 238, 151);
impl View {
    fn is_analysis(self) -> bool {
        matches!(
            self,
            Self::AnalysisMenu
                | Self::AnalysisSummary
                | Self::AnalysisGraph
                | Self::AnalysisRounds
                | Self::AnalysisAccuracy
                | Self::AnalysisCosts
                | Self::AnalysisDecisions
                | Self::AnalysisAssumptions
        )
    }
}
impl App {
    pub(super) fn record_settled_round(&mut self) {
        if self.game.phase != Phase::Result
            || self.reveal.is_some()
            || self.other_reveal_len != 0
            || self.analysis_last_round == self.game.rounds
        {
            return;
        }
        self.analysis_last_round = self.game.rounds;
        if !self.analysis_valid {
            return;
        }
        self.analysis_valid = self.session_analysis.record(analysis::Round {
            id: self.game.rounds as u64,
            base_wager: self.analysis_base_wager,
            stake: self.game.round_stake,
            returned: self.game.round_return,
            bankroll: self.game.bankroll,
            deposits: self.game.deposits,
            cashouts: self.game.cashouts,
        });
    }
    fn analysis_history_aid(&mut self) {
        self.mark_assisted();
        if let Some(first) = self.session_metrics.newest(self.page)
            && first.count.is_some_and(|c| c == self.count_context())
        {
            self.mark_aid(7)
        }
    }
    pub(super) fn handle_analysis_tap(&mut self, grey: bool) -> bool {
        if self.view == View::Session && !grey && self.selected == 3 {
            self.set(View::AnalysisMenu);
            return true;
        }
        if !self.view.is_analysis() {
            return false;
        }
        if grey {
            match self.view {
                View::AnalysisMenu => self.selected = (self.selected + 1) % 8,
                View::AnalysisSummary => self.page = (self.page + 1) % 4,
                View::AnalysisRounds => {
                    self.page = (self.page + 1)
                        % (self.session_analysis.summary().rounds as usize)
                            .clamp(1, analysis::HISTORY)
                }
                View::AnalysisAccuracy => self.page = (self.page + 1) % 12,
                View::AnalysisCosts | View::AnalysisAssumptions => self.page = (self.page + 1) % 3,
                View::AnalysisDecisions => {
                    self.page = (self.page + 1) % self.session_metrics.history_count().max(1);
                    self.analysis_history_aid()
                }
                _ => {}
            }
            return true;
        }
        match self.view {
            View::AnalysisMenu => match self.selected {
                0 => self.set(View::AnalysisSummary),
                1 => self.set(View::AnalysisGraph),
                2 => self.set(View::AnalysisRounds),
                3 => self.set(View::AnalysisAccuracy),
                4 => {
                    self.mark_assisted();
                    self.set(View::AnalysisCosts)
                }
                5 => {
                    self.set(View::AnalysisDecisions);
                    self.analysis_history_aid()
                }
                6 => self.set(View::AnalysisAssumptions),
                _ => self.set(View::Session),
            },
            _ => self.set(View::AnalysisMenu),
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
fn text<'a>(b: &'a mut Text<160>, args: core::fmt::Arguments<'_>) -> &'a str {
    b.clear();
    let _ = b.write_fmt(args);
    b.as_str()
}
fn cash(cents: f64) -> Cash {
    Cash((cents + if cents < 0.0 { -0.5 } else { 0.5 }) as i64, true)
}
fn choice(c: Choice) -> &'static str {
    match c {
        Choice::Play(a) => table::action_name(a),
        Choice::Insurance(true) => "INSURE",
        Choice::Insurance(false) => "DECLINE",
    }
}
fn up(card: crate::cards::Card) -> &'static str {
    match card.value() {
        2 => "2",
        3 => "3",
        4 => "4",
        5 => "5",
        6 => "6",
        7 => "7",
        8 => "8",
        9 => "9",
        10 => "10",
        _ => "A",
    }
}
fn counters(w: &mut Widgets<'_>, c: training::Counters) {
    let mut b = Text::new();
    label(
        w,
        33,
        text(
            &mut b,
            format_args!("Correct {} / {}", c.correct, c.correct + c.errors),
        ),
        17,
        GOLD,
    );
    label(
        w,
        59,
        text(&mut b, format_args!("Errors {}", c.errors)),
        14,
        WHITE,
    );
    label(
        w,
        81,
        text(&mut b, format_args!("Assisted {}", c.assisted)),
        14,
        WHITE,
    );
    label(w, 105, "Assisted answers stay separate", 10, MUTED);
}
fn line(frame: &mut Canvas, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: u16) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        frame.pixel(x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let twice = error * 2;
        if twice >= dy {
            error += dy;
            x0 += sx
        }
        if twice <= dx {
            error += dx;
            y0 += sy
        }
    }
}
fn graph(w: &mut Widgets<'_>, model: &analysis::Model) {
    let summary = model.summary();
    let n = (summary.rounds as usize).min(analysis::HISTORY);
    if n == 0 {
        label(w, 56, "No settled rounds yet", 16, WHITE);
        return;
    }
    let baseline = model.newest(0).unwrap().ledger_net() - summary.net;
    let start = model.newest(n - 1).unwrap();
    let initial = start.ledger_net() - start.net() as i128 - baseline;
    let mut low = initial.min(0);
    let mut high = initial.max(0);
    for offset in 0..n {
        let value = model.newest(offset).unwrap().ledger_net() - baseline;
        low = low.min(value);
        high = high.max(value)
    }
    if high == low {
        low -= 1;
        high += 1
    }
    let span = (high - low).max(1);
    let y = |value: i128| 110 - ((value - low) * 56 / span) as i32;
    w.frame.rect(8, 52, 224, 60, crate::widgets::PANEL);
    let zero = y(0);
    w.frame.rect(8, zero, 224, 1, LINE);
    let mut previous = (8, y(initial));
    for index in 0..n {
        let x = 8 + (index + 1) as i32 * 223 / n as i32;
        let point = (
            x,
            y(model.newest(n - 1 - index).unwrap().ledger_net() - baseline),
        );
        line(w.frame, previous.0, previous.1, point.0, point.1, GOLD);
        previous = point;
    }
}
pub(crate) fn draw(a: &mut App) -> bool {
    if !a.view.is_analysis() {
        return false;
    }
    let summary = a.session_analysis.summary();
    let net_color = if summary.net < 0 {
        RED
    } else if summary.net > 0 {
        GREEN
    } else {
        GOLD
    };
    let costs = a.session_metrics.costs();
    let first = a.session_metrics.newest(a.page);
    let round = a.session_analysis.newest(a.page);
    let mut w = Widgets {
        frame: &mut a.frame,
        renderer: &mut a.renderer,
    };
    let mut b = Text::<160>::new();
    match a.view {
        View::AnalysisMenu => {
            w.title(
                "SESSION ANALYSIS",
                Some(if a.analysis_valid {
                    if a.resume_enabled {
                        "Saved session / descriptive reports"
                    } else {
                        "Current RAM session / descriptive reports"
                    }
                } else {
                    "Round analysis stopped / prior data retained"
                }),
                false,
            );
            w.list(
                &[
                    "Session summary / dispersion",
                    "Net history graph",
                    "Settled round records",
                    "First-answer accuracy",
                    "Basic model EV differences",
                    "First-answer history",
                    "Assumptions / limits",
                    "Return to session",
                ],
                a.selected,
                None,
            )
        }
        View::AnalysisSummary => {
            w.title(
                [
                    "SESSION SUMMARY",
                    "ROUND OUTCOMES",
                    "OBSERVED DISPERSION",
                    "IID-NORMAL REFERENCE",
                ][a.page.min(3)],
                if a.analysis_valid {
                    None
                } else {
                    Some("PARTIAL / round capture stopped")
                },
                false,
            );
            match a.page {
                0 => {
                    label(
                        &mut w,
                        29,
                        text(&mut b, format_args!("{} settled rounds", summary.rounds)),
                        16,
                        WHITE,
                    );
                    label(
                        &mut w,
                        51,
                        text(
                            &mut b,
                            format_args!("Net {}", Cash(summary.net as i64, true)),
                        ),
                        19,
                        net_color,
                    );
                    label(
                        &mut w,
                        77,
                        text(
                            &mut b,
                            format_args!(
                                "Total staked {}",
                                Cash(summary.total_staked as i64, false)
                            ),
                        ),
                        12,
                        WHITE,
                    );
                    label(
                        &mut w,
                        99,
                        if summary.rounds == 0 {
                            "Need a completed round"
                        } else {
                            "Deposits and cashouts are not wins"
                        },
                        10,
                        MUTED,
                    )
                }
                1 => {
                    label(
                        &mut w,
                        29,
                        text(
                            &mut b,
                            format_args!(
                                "W {} / L {} / P {}",
                                summary.wins, summary.losses, summary.pushes
                            ),
                        ),
                        16,
                        GOLD,
                    );
                    label(
                        &mut w,
                        52,
                        text(
                            &mut b,
                            format_args!(
                                "Best {} / worst {}",
                                Cash(summary.best_round, true),
                                Cash(summary.worst_round, true)
                            ),
                        ),
                        12,
                        WHITE,
                    );
                    label(
                        &mut w,
                        75,
                        text(
                            &mut b,
                            format_args!("Max base wager {}", Cash(summary.largest_wager, false)),
                        ),
                        12,
                        WHITE,
                    );
                    label(
                        &mut w,
                        99,
                        text(
                            &mut b,
                            format_args!("Longest loss streak {}", summary.longest_loss_streak),
                        ),
                        11,
                        MUTED,
                    )
                }
                2 => {
                    label(
                        &mut w,
                        29,
                        if summary.rounds == 0 {
                            "Mean unavailable (no rounds)"
                        } else {
                            text(
                                &mut b,
                                format_args!("Mean {} / round", cash(summary.mean_net_cents)),
                            )
                        },
                        15,
                        net_color,
                    );
                    if let Some(sd) = summary.sample_stddev_cents {
                        label(
                            &mut w,
                            51,
                            text(
                                &mut b,
                                format_args!("Sample SD {}", Cash(cash(sd).0, false)),
                            ),
                            14,
                            WHITE,
                        )
                    } else {
                        label(&mut w, 51, "Dispersion needs two rounds", 13, WHITE)
                    }
                    label(
                        &mut w,
                        76,
                        text(
                            &mut b,
                            format_args!(
                                "Max net drawdown {}",
                                Cash(summary.max_drawdown as i64, false)
                            ),
                        ),
                        12,
                        WHITE,
                    );
                    label(
                        &mut w,
                        100,
                        "Observed variable wagers; not a forecast",
                        9,
                        MUTED,
                    )
                }
                _ => {
                    if let Some(width) = summary.normal_reference_halfwidth_cents {
                        label(
                            &mut w,
                            30,
                            text(
                                &mut b,
                                format_args!("Mean {} / round", cash(summary.mean_net_cents)),
                            ),
                            14,
                            net_color,
                        );
                        label(
                            &mut w,
                            54,
                            text(
                                &mut b,
                                format_args!("Reference +/- {}", Cash(cash(width).0, false)),
                            ),
                            14,
                            WHITE,
                        );
                        label(&mut w, 79, "1.96 x sample SD / sqrt(rounds)", 11, MUTED)
                    } else {
                        label(&mut w, 37, "Available from 30 settled rounds", 14, WHITE);
                        label(&mut w, 70, "Small samples show no band", 12, MUTED)
                    }
                    label(
                        &mut w,
                        104,
                        "NOT an advantage CI or a profit forecast",
                        9,
                        GOLD,
                    )
                }
            }
            w.footer("GREY: PAGE   BLUE: ANALYSIS")
        }
        View::AnalysisGraph => {
            w.title(
                "SESSION NET HISTORY",
                Some(if a.analysis_valid {
                    "Last 128 settled rounds / funding excluded"
                } else {
                    "PARTIAL / round capture stopped"
                }),
                false,
            );
            label(
                &mut w,
                32,
                text(
                    &mut b,
                    format_args!(
                        "Net {} / max drawdown {}",
                        Cash(summary.net as i64, true),
                        Cash(summary.max_drawdown as i64, false)
                    ),
                ),
                11,
                WHITE,
            );
            graph(&mut w, &a.session_analysis);
            w.footer("BLUE: ANALYSIS")
        }
        View::AnalysisRounds => {
            w.title(
                "SETTLED ROUND RECORD",
                if a.analysis_valid {
                    None
                } else {
                    Some("PARTIAL / round capture stopped")
                },
                false,
            );
            if let Some(r) = round {
                label(
                    &mut w,
                    31,
                    text(
                        &mut b,
                        format_args!("Round {} / net {}", r.id, Cash(r.net(), true)),
                    ),
                    16,
                    GOLD,
                );
                label(
                    &mut w,
                    54,
                    text(
                        &mut b,
                        format_args!(
                            "Base {} / stake {}",
                            Cash(r.base_wager, false),
                            Cash(r.stake, false)
                        ),
                    ),
                    12,
                    WHITE,
                );
                label(
                    &mut w,
                    77,
                    text(&mut b, format_args!("Returned {}", Cash(r.returned, false))),
                    13,
                    WHITE,
                );
                label(
                    &mut w,
                    101,
                    text(
                        &mut b,
                        format_args!("Chips at settlement {}", Cash(r.bankroll, false)),
                    ),
                    10,
                    MUTED,
                )
            } else {
                label(&mut w, 49, "No settled rounds recorded", 16, WHITE)
            }
            w.footer("GREY: HISTORY   BLUE: ANALYSIS")
        }
        View::AnalysisAccuracy => {
            let page = a.page.min(11);
            let cards = a.session_metrics.card_stats();
            let wagers = a.session_metrics.wager_stats();
            let (name, c) = match page {
                0 => ("LIVE FIRST ANSWERS", cards.all),
                1..=4 => (
                    [
                        "HARD FIRST ANSWERS",
                        "SOFT FIRST ANSWERS",
                        "PAIR FIRST ANSWERS",
                        "INSURANCE FIRST ANSWERS",
                    ][page - 1],
                    cards.categories[page - 1],
                ),
                5 => ("WAGER FIRST ANSWERS", wagers.all),
                _ => {
                    let i = page - 6;
                    (
                        [
                            "WAGER TC <= 0",
                            "WAGER TC ONE",
                            "WAGER TC TWO",
                            "WAGER TC THREE",
                            "WAGER TC FOUR PLUS",
                            "WAGER REFILL CASES",
                        ][i],
                        wagers.categories[i],
                    )
                }
            };
            w.title(
                name,
                Some(if page <= 4 {
                    "Current session / selected-policy grading"
                } else {
                    "Current session / example policy only"
                }),
                false,
            );
            counters(&mut w, c);
            w.footer("GREY: CATEGORY   BLUE: ANALYSIS")
        }
        View::AnalysisCosts => {
            w.title(
                "MODEL EV DIFFERENCE",
                Some("Basic first answers / original wagers"),
                false,
            );
            if a.page < 2 {
                let bucket = if a.page == 0 {
                    costs.unassisted
                } else {
                    costs.assisted
                };
                label(
                    &mut w,
                    29,
                    if a.page == 0 {
                        "UNASSISTED"
                    } else {
                        "ASSISTED (SEPARATE)"
                    },
                    14,
                    GOLD,
                );
                label(
                    &mut w,
                    48,
                    text(
                        &mut b,
                        format_args!("{} complete snapshots", bucket.decisions),
                    ),
                    12,
                    WHITE,
                );
                label(
                    &mut w,
                    68,
                    text(
                        &mut b,
                        format_args!(
                            "Total model difference {}",
                            Cash(cash(bucket.difference_cents).0, false)
                        ),
                    ),
                    12,
                    WHITE,
                );
                label(
                    &mut w,
                    88,
                    if let Some(mean) = bucket.mean_difference_cents {
                        text(
                            &mut b,
                            format_args!("Mean {} / first answer", Cash(cash(mean).0, false)),
                        )
                    } else {
                        "No complete Basic snapshots"
                    },
                    11,
                    MUTED,
                );
                label(
                    &mut w,
                    108,
                    "Fixed replacement odds; NOT actual loss",
                    9,
                    GOLD,
                )
            } else {
                label(
                    &mut w,
                    31,
                    text(
                        &mut b,
                        format_args!("Hi-Lo choices skipped {}", costs.skipped_hilo),
                    ),
                    14,
                    WHITE,
                );
                label(
                    &mut w,
                    56,
                    text(
                        &mut b,
                        format_args!("Unavailable / partial {}", costs.unavailable),
                    ),
                    14,
                    WHITE,
                );
                label(&mut w, 83, "Count-index grades stay separate", 11, MUTED);
                label(
                    &mut w,
                    105,
                    "Coach corrections keep first evidence",
                    10,
                    MUTED,
                )
            }
            w.footer("GREY: SCOPE   BLUE: ANALYSIS")
        }
        View::AnalysisDecisions => {
            w.title("FIRST-ANSWER HISTORY", None, false);
            if let Some(d) = first {
                let mut context = Text::<80>::new();
                match d.category {
                    training::Category::Insurance => {
                        let _ = write!(context, "Insurance vs A");
                    }
                    training::Category::Pair => {
                        let _ = write!(
                            context,
                            "Pair {} vs {}",
                            up(d.hand.cards()[0]),
                            up(d.dealer_up)
                        );
                    }
                    _ => {
                        let _ = write!(
                            context,
                            "{} {} vs {}",
                            if d.hand.soft() { "Soft" } else { "Hard" },
                            d.hand.total(),
                            up(d.dealer_up)
                        );
                    }
                }
                let _ = write!(
                    context,
                    " / {} / {}",
                    if d.policy == Policy::HiLo {
                        "HI-LO"
                    } else {
                        "BASIC"
                    },
                    Cash(d.original_wager, false)
                );
                label(&mut w, 23, context.as_str(), 12, GOLD);
                label(
                    &mut w,
                    43,
                    text(
                        &mut b,
                        format_args!(
                            "First {} / policy {}",
                            choice(d.choice),
                            choice(d.policy_best)
                        ),
                    ),
                    12,
                    WHITE,
                );
                label(
                    &mut w,
                    63,
                    if d.assisted {
                        "ASSISTED / first answer retained"
                    } else if d.correct {
                        "CORRECT / first answer retained"
                    } else {
                        "MISTAKE / first answer retained"
                    },
                    10,
                    WHITE,
                );
                if let Some(value) = d.comparison {
                    label(
                        &mut w,
                        82,
                        text(
                            &mut b,
                            format_args!(
                                "Model difference {}",
                                Cash(cash(value.difference_cents).0, false)
                            ),
                        ),
                        12,
                        GOLD,
                    )
                } else {
                    label(
                        &mut w,
                        82,
                        if d.skipped == Some(evidence::Skip::HiLoPolicy) {
                            "Hi-Lo skipped from fixed-odds aggregate"
                        } else {
                            "Model snapshot unavailable / partial"
                        },
                        10,
                        MUTED,
                    )
                }
                if let Some(c) = d.count {
                    label(
                        &mut w,
                        103,
                        text(
                            &mut b,
                            format_args!(
                                "Original RC {:+} / TC {:+} (trunc)",
                                c.running,
                                c.integer_true_count(Rules::for_profile(d.profile))
                                    .unwrap_or(0)
                            ),
                        ),
                        10,
                        MUTED,
                    )
                } else {
                    label(
                        &mut w,
                        103,
                        "Frozen model evidence; not actual loss",
                        9,
                        MUTED,
                    )
                }
            } else {
                label(&mut w, 49, "No live first answers recorded", 15, WHITE);
                label(
                    &mut w,
                    80,
                    "Test / Coach / Guide score live choices",
                    10,
                    MUTED,
                )
            }
            w.footer("GREY: HISTORY   BLUE: ANALYSIS")
        }
        View::AnalysisAssumptions => {
            w.title("ANALYSIS ASSUMPTIONS", None, false);
            let lines = match a.page {
                0 => [
                    if a.resume_enabled {
                        "Current session can resume when saved."
                    } else {
                        "Current session stays in RAM."
                    },
                    "Only completed rounds count.",
                    "Deposits/cashouts are not wins.",
                    "Variable bets change dispersion.",
                    "No forecast or risk-of-ruin claim.",
                ],
                1 => [
                    "Rounds within a shoe are dependent.",
                    "1.96 s/sqrt(n): normal reference.",
                    "NOT a confidence interval for edge.",
                    "30 rounds do not prove independence.",
                    "Small samples show no reference band.",
                ],
                _ => [
                    "EV: Basic-policy first answers only.",
                    "Replacement odds; no shoe model.",
                    "Assisted answers stay separate.",
                    "Hi-Lo and partial values are skipped.",
                    "Coach keeps the original first choice.",
                ],
            };
            for (i, s) in lines.iter().enumerate() {
                label(&mut w, 29 + i as i32 * 17, s, 11, WHITE)
            }
            w.footer("GREY: PAGE   BLUE: ANALYSIS")
        }
        _ => {}
    }
    if let Some(mode) = a.toast {
        w.mode_footer(mode)
    }
    true
}
