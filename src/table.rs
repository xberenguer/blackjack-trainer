//! Approved compact table layout. Advice receives visible hand/upcard snapshots;
//! exposed-card accounting and assistance are returned explicitly to the UI.
use crate::{
    canvas::{Canvas, FELT, rgb},
    cards::Outcome,
    counting::Tracker,
    deviations::{self, Count, Threshold},
    engine::{Action, Engine, Phase},
    guide,
    render::{Align, Renderer, TextStyle, text_height, text_width},
    strategy,
    text::{Cash, Text},
};
use core::fmt::Write;
const GOLD: u16 = rgb(255, 255, 0);
const WHITE: u16 = rgb(255, 255, 255);
const MUTED: u16 = rgb(168, 155, 193);
const DARK: u16 = rgb(11, 8, 36);
const LINE: u16 = rgb(61, 49, 98);
const GREEN: u16 = rgb(71, 238, 151);
const RED: u16 = rgb(255, 91, 108);
pub use crate::training::{Mode, Policy};
#[derive(Clone, Copy)]
pub struct Reveal {
    pub progress: u8,
    pub bankroll: i64,
}
#[derive(Clone, Copy)]
pub struct Options {
    pub mode: Mode,
    pub policy: Policy,
    pub normal_counter: u8,
    pub guide_counter: u8,
    pub reveal: Option<Reveal>,
    pub result_hand: usize,
    pub insurance_yes: bool,
    pub toast: Option<Mode>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            mode: Mode::Off,
            policy: Policy::Basic,
            normal_counter: 0,
            guide_counter: 2,
            reveal: None,
            result_hand: 0,
            insurance_yes: false,
            toast: None,
        }
    }
}
#[derive(Default, Debug, Clone, Copy)]
pub struct Exposure {
    pub play_assisted: bool,
    pub count_aids: u8,
}
struct Painter<'a> {
    frame: &'a mut Canvas,
    render: &'a mut Renderer,
}
impl Painter<'_> {
    fn toast(&mut self, mode: Option<Mode>) {
        if let Some(mode) = mode {
            self.frame.rect(164, 76, 76, 59, FELT);
            self.label((202, 83), "MODE", 11, MUTED, true, Align::Center);
            self.label((202, 103), mode.name(), 17, GOLD, true, Align::Center);
        }
    }
    fn insurance_toast(&mut self, mode: Option<Mode>) {
        if let Some(mode) = mode {
            // Insurance always has two player cards, ending at x116. Keep the
            // transient mode label in the spare strip, never over the choices.
            self.frame.rect(120, 76, 42, 59, FELT);
            self.label((141, 83), "MODE", 8, MUTED, true, Align::Center);
            self.label((141, 103), mode.name(), 11, GOLD, true, Align::Center);
        }
    }
    fn label(&mut self, p: (i32, i32), s: &str, size: u8, color: u16, bold: bool, align: Align) {
        self.render.text(
            self.frame,
            p,
            s,
            TextStyle {
                size,
                color,
                bold,
                align,
            },
        );
    }
    fn actions(&mut self, names: &[&str], selected: usize) {
        let count = names.len() as i32;
        if count == 0 {
            return;
        }
        let h = if count <= 2 {
            24
        } else if count == 3 {
            20
        } else {
            14
        };
        let start = 135 - count * h;
        for (i, name) in names.iter().enumerate() {
            let y = start + i as i32 * h;
            let active = i == selected;
            self.frame
                .rect(164, y, 76, h, if active { GOLD } else { DARK });
            if i + 1 < names.len() {
                self.frame.rect(164, y + h - 1, 76, 1, LINE)
            }
            let mut size = if count <= 2 { 18 } else { 14 };
            while size > 9
                && (text_width(name, size, true) > 72 || text_height(name, size, true) > h - 4)
            {
                size -= 1
            }
            self.label(
                (202, y + (h - text_height(name, size, true)) / 2),
                name,
                size,
                if active { rgb(0, 0, 0) } else { WHITE },
                true,
                Align::Center,
            );
        }
    }
    fn count(&mut self, g: &Engine, t: &Tracker, o: Options, exposure: &mut Exposure) {
        let display = if o.mode == Mode::Guide {
            o.guide_counter
        } else {
            o.normal_counter
        };
        if display == 0 {
            return;
        }
        exposure.count_aids |= if display == 2 { 7 } else { 1 };
        let mut text = Text::<48>::new();
        if display == 2 {
            if let Some(tc) = t.true_count((g.total_cards() - g.position) as u16) {
                let _ = write!(text, "RC{:+} TC{:+.1}", t.running_count(), tc);
            } else {
                let _ = write!(text, "RC{:+} TC --", t.running_count());
            }
        } else {
            let _ = write!(text, "RC{:+}", t.running_count());
        }
        let mut size = 10;
        let minimum_size = if g.phase == Phase::Insurance { 7 } else { 8 };
        while size > minimum_size && text_width(text.as_str(), size, true) > 77 {
            size -= 1
        }
        self.label((237, 22), text.as_str(), size, MUTED, true, Align::Right);
        if o.reveal.is_none() && matches!(g.phase, Phase::Player | Phase::Insurance) {
            exposure.play_assisted = true
        }
    }
    fn guide(
        &mut self,
        g: &Engine,
        t: &Tracker,
        o: Options,
        exposure: &mut Exposure,
        insurance: bool,
    ) {
        let Some(up) = g.dealer.cards().first().copied() else {
            return;
        };
        let Some(h) = g.hands.get(g.active) else {
            return;
        };
        let count = Count {
            running: t.running_count(),
            undealt: (g.total_cards() - g.position) as u16,
            decks: t.decks(),
            observed: t.observed_count(),
            shoe: if t.shoe_number() == g.shoe_number {
                t.shoe_number()
            } else {
                0
            },
        };
        let advice = if o.policy == Policy::HiLo {
            deviations::recommend(h, up, g.legal_mask(), g.rules, count)
        } else {
            deviations::Advice {
                play: strategy::recommend(h, up, g.legal_mask(), g.rules),
                integer_true_count: None,
                change: None,
            }
        };
        if !insurance && advice.play.is_none() {
            return;
        }
        exposure.play_assisted = true;
        let affordable = g.bankroll >= g.hands[0].bet / 2;
        let side = if o.policy == Policy::HiLo {
            deviations::insurance(affordable, g.rules, count)
        } else {
            None
        };
        let counted = if insurance {
            side.is_some()
        } else {
            advice.integer_true_count.is_some()
        };
        let (left, head, action, reason) = if insurance {
            // Separate advice from the unchanged right-column insurance UI.
            // Dealer upcard/hole finish at x75; all text ends before x154.
            (82, 1, 13, 29)
        } else {
            (164, 76, 87, 105)
        };
        self.label(
            (left, head),
            if insurance {
                if counted {
                    "HI-LO ADVICE"
                } else {
                    "BASIC ADVICE"
                }
            } else if counted {
                "HI-LO MOVE"
            } else {
                "BASIC MOVE"
            },
            9,
            MUTED,
            true,
            Align::Left,
        );
        let best = if insurance {
            if side == Some(true) {
                "INSURE"
            } else {
                "DECLINE"
            }
        } else {
            action_name(advice.play.unwrap().best)
        };
        let mut size = 14;
        while size > 10 && text_width(best, size, true) > if insurance { 71 } else { 75 } {
            size -= 1
        }
        self.label((left, action), best, size, GOLD, true, Align::Left);
        let mut text = Text::<128>::new();
        let why = if counted && insurance {
            exposure.count_aids |= 4;
            let tc = count.integer_true_count(g.rules).unwrap();
            if tc >= 3 && !affordable {
                let _ = write!(text, "Not funded\nNeed {}", Cash(g.hands[0].bet / 2, false));
            } else {
                let _ = write!(
                    text,
                    "TC{:+} {} +3\n{}",
                    tc,
                    if side == Some(true) { ">=" } else { "<" },
                    if side == Some(true) {
                        "Index reached"
                    } else {
                        "Index not met"
                    }
                );
            }
            text.as_str()
        } else if let Some(change) = advice.change {
            match change.threshold {
                Threshold::PositiveRunning | Threshold::NegativeRunning => {
                    exposure.count_aids |= 1;
                    let _ = write!(
                        text,
                        "RC{:+}\nRC {} 0\n{}",
                        t.running_count(),
                        if change.threshold == Threshold::PositiveRunning {
                            ">"
                        } else {
                            "<"
                        },
                        if change.reached {
                            "Index reached"
                        } else {
                            "Index not met"
                        }
                    );
                }
                Threshold::AtLeast(n) | Threshold::AtMost(n) => {
                    exposure.count_aids |= 4;
                    let _ = write!(
                        text,
                        "TC{:+} trunc\nTC {} {:+}\n{}",
                        advice.integer_true_count.unwrap(),
                        if matches!(change.threshold, Threshold::AtLeast(_)) {
                            ">="
                        } else {
                            "<="
                        },
                        n,
                        if change.reached {
                            "Index reached"
                        } else {
                            "Index not met"
                        }
                    );
                }
            }
            text.as_str()
        } else if insurance {
            "Basic policy:\nno side bet"
        } else {
            guide::short_reason(h, up, advice.play.unwrap())
        };
        for (i, line) in why
            .split('\n')
            .take(if insurance { 2 } else { 3 })
            .enumerate()
        {
            let mut size = if insurance { 9 } else { 10 };
            if insurance {
                while size > 7 && text_width(line, size, false) > 71 {
                    size -= 1;
                }
            }
            self.label(
                (left, reason + i as i32 * 10),
                line,
                size,
                WHITE,
                false,
                Align::Left,
            );
        }
    }
}
pub fn action_name(a: Action) -> &'static str {
    match a {
        Action::Hit => "HIT",
        Action::Stand => "STAND",
        Action::Double => "DOUBLE",
        Action::Split => "SPLIT",
        Action::Surrender => "SURRENDER",
    }
}
fn outcome_name(o: Outcome) -> &'static str {
    match o {
        Outcome::Pending => "PLAYING",
        Outcome::Win => "WIN",
        Outcome::Loss => "LOSE",
        Outcome::Push => "PUSH",
        Outcome::Blackjack => "BLACKJACK",
        Outcome::Bust => "BUST",
        Outcome::Surrender => "SURRENDER",
    }
}
pub fn draw(
    frame: &mut Canvas,
    render: &mut Renderer,
    g: &Engine,
    tracker: &mut Tracker,
    o: Options,
) -> Exposure {
    // Initial extra-seat cards are presented by the App carousel. Until all
    // have appeared, neither a full count nor a stale count may be shown.
    let mut o = o;
    if g.extra_seats > 0 && o.reveal.is_some() {
        o.normal_counter = 0;
        o.guide_counter = 0;
    }
    tracker.ensure_shoe(g.rules.decks, g.shoe_number);
    if g.uses_extended_shoe() && o.reveal.is_none() {
        g.observe_exposed(tracker);
    }
    let mut exposure = Exposure::default();
    let mut p = Painter { frame, render };
    p.frame.clear(FELT);
    let mut text = Text::<64>::new();
    let _ = write!(
        text,
        "{}",
        Cash(o.reveal.map_or(g.bankroll, |r| r.bankroll), false)
    );
    let mut size = 24;
    let minimum_size = if g.phase == Phase::Insurance { 7 } else { 10 };
    while size > minimum_size && text_width(text.as_str(), size, true) > 77 {
        size -= 1
    }
    p.label((237, 1), text.as_str(), size, GOLD, true, Align::Right);
    let show = if g.phase == Phase::Player {
        g.active
    } else {
        o.result_hand
    }
    .min(g.hand_count.saturating_sub(1));
    let bet = if g.between_rounds() {
        g.selected_bet
    } else if g.hand_count > 0 {
        g.hands[show].bet
    } else {
        g.selected_bet
    };
    text.clear();
    let _ = write!(text, "BET {}", Cash(bet, false));
    size = 20;
    while size > 10 && text_width(text.as_str(), size, true) > 77 {
        size -= 1
    }
    let counter = if o.mode == Mode::Guide {
        o.guide_counter
    } else {
        o.normal_counter
    };
    p.label(
        (237, if counter != 0 { 35 } else { 31 }),
        text.as_str(),
        size,
        WHITE,
        true,
        Align::Right,
    );
    if g.hand_count == 0 {
        p.count(g, tracker, o, &mut exposure);
        p.label(
            (80, 55),
            if g.bankroll < g.rules.minimum {
                "OUT OF CHIPS"
            } else {
                "READY TO PLAY"
            },
            12,
            GOLD,
            true,
            Align::Center,
        );
        p.label(
            (80, 77),
            if g.bankroll < g.rules.minimum {
                "BLUE: REFILL $1000"
            } else {
                "GREY: CHANGE BET"
            },
            10,
            WHITE,
            true,
            Align::Center,
        );
        p.actions(
            &[if g.bankroll < g.rules.minimum {
                "REFILL $1000"
            } else if g.phase == Phase::Cut {
                "CUT SHOE"
            } else {
                "DEAL"
            }],
            0,
        );
        p.toast(o.toast);
        return exposure;
    }
    let show_hole = g.hole_visible && o.reveal.is_none();
    let dealer_visible = if let Some(r) = o.reveal {
        if r.progress >= 4 {
            2
        } else if r.progress >= 2 {
            1
        } else {
            0
        }
    } else if show_hole {
        g.dealer.count()
    } else {
        g.dealer.count().min(2)
    };
    let ds = dealer_visible.saturating_sub(6);
    let dc = dealer_visible - ds;
    let step = match dc {
        0..=3 => 37,
        4 => 30,
        5 => 26,
        _ => 23,
    };
    for i in ds..dealer_visible {
        let hidden = i == 1 && !show_hole;
        p.render.card(
            p.frame,
            (3 + (i - ds) as i32 * step, 0),
            if hidden {
                None
            } else {
                Some(g.dealer.cards()[i])
            },
            true,
        );
        if !hidden && !g.uses_extended_shoe() {
            tracker.observe(g.dealer.positions()[i], g.dealer.cards()[i]);
        }
    }
    let h = &g.hands[show];
    let visible = if let Some(r) = o.reveal {
        if r.progress >= 3 {
            2
        } else if r.progress >= 1 {
            1
        } else {
            0
        }
    } else {
        h.count()
    };
    let start = visible.saturating_sub(6);
    let count = visible - start;
    let step = match count {
        0..=2 => 58,
        3 => 44,
        4 => 33,
        5 => 25,
        _ => 20,
    };
    for i in start..visible {
        p.render.card(
            p.frame,
            (3 + (i - start) as i32 * step, 53),
            Some(h.cards()[i]),
            false,
        );
        if !g.uses_extended_shoe() {
            tracker.observe(h.positions()[i], h.cards()[i]);
        }
    }
    p.count(g, tracker, o, &mut exposure);
    if g.hand_count > 1 {
        text.clear();
        let _ = write!(text, "HAND {}/{}", show + 1, g.hand_count);
        p.label((164, 58), text.as_str(), 13, GOLD, true, Align::Left);
    } else if start > 0 {
        text.clear();
        let _ = write!(text, "+{} CARDS", start);
        p.label((164, 58), text.as_str(), 13, MUTED, false, Align::Left);
    }
    if o.reveal.is_none() && g.phase == Phase::Result {
        text.clear();
        let _ = write!(text, "{}", Cash(g.round_net, true));
        size = 22;
        while size > 10 && text_width(text.as_str(), size, true) > 77 {
            size -= 1
        }
        p.label(
            (237, 82),
            text.as_str(),
            size,
            if g.round_net >= 0 { GREEN } else { RED },
            true,
            Align::Right,
        );
        let outcome = if g.insured && g.dealer.natural() {
            "INSURED"
        } else if h.outcome == Outcome::Blackjack {
            if g.rules.blackjack_numerator == 6 {
                "BJ 6:5"
            } else {
                "BJ 3:2"
            }
        } else if h.outcome == Outcome::Surrender {
            "SURR"
        } else {
            outcome_name(h.outcome)
        };
        size = 18;
        while size > 10 && text_width(outcome, size, true) > 77 {
            size -= 1
        }
        p.label((237, 111), outcome, size, GOLD, true, Align::Right);
    }
    if o.reveal.is_some() {
        p.label((202, 100), "DEALING", 14, GOLD, true, Align::Center);
    } else {
        match g.phase {
            Phase::Player => {
                if o.toast.is_none() && o.mode == Mode::Guide {
                    p.guide(g, tracker, o, &mut exposure, false)
                }
            }
            Phase::Insurance => {
                text.clear();
                let _ = write!(text, "YES {}", Cash(g.hands[0].bet / 2, false));
                let names = ["DECLINE", text.as_str()];
                if o.mode == Mode::Guide {
                    p.guide(g, tracker, o, &mut exposure, true);
                }
                p.label((202, 65), "INSURE?", 14, GOLD, true, Align::Center);
                p.actions(&names, usize::from(o.insurance_yes));
            }
            Phase::Dealer => {
                p.label((202, 82), "DEALER", 14, GOLD, true, Align::Center);
                p.label((202, 105), "PLAYS", 14, GOLD, true, Align::Center);
            }
            Phase::Result => {}
            _ => p.actions(&["DEAL"], 0),
        }
    }
    if g.phase == Phase::Insurance && o.reveal.is_none() {
        p.insurance_toast(o.toast);
    } else {
        p.toast(o.toast);
    }
    exposure
}

// RESUME10 Reveal
impl crate::resume_private::Wire for Reveal {
    fn initial() -> Self {
        Self {
            progress: crate::resume_private::Wire::initial(),
            bankroll: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self { progress, bankroll } = self;
        progress.put(w)?;
        bankroll.put(w)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            progress: crate::resume_private::Wire::get(r)?,
            bankroll: crate::resume_private::Wire::get(r)?,
        })
    }

    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self { progress, bankroll } = self;
        progress.read_into(r)?;
        bankroll.read_into(r)?;
        Ok(())
    }
}
