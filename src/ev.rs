//! Infinite-deck independent replacement EV, US-peek conditioned. No shoe,
//! hidden card, count or RNG input. A descending minimum-sum DAG has no cutoff.
use crate::{
    cards::{Card, Hand},
    engine::Action,
};
pub const ABSOLUTE_ERROR_BOUND: f64 = 1e-10;
pub const MODEL_NAME: &str = "Infinite-deck replacement";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Payout {
    ThreeToTwo,
    SixToFive,
}
#[derive(Clone, Copy, Debug)]
pub struct Rules {
    pub hit_soft_17: bool,
    pub double_after_split: bool,
    pub late_surrender: bool,
    pub payout: Payout,
}
impl Default for Rules {
    fn default() -> Self {
        Self {
            hit_soft_17: true,
            double_after_split: true,
            late_surrender: true,
            payout: Payout::ThreeToTwo,
        }
    }
}
impl Rules {
    pub fn from_table(r: crate::rules::Rules) -> Option<Self> {
        if !r.valid() {
            return None;
        }
        Some(Self {
            hit_soft_17: r.hit_soft_17,
            double_after_split: r.double_after_split,
            late_surrender: r.late_surrender,
            payout: if r.blackjack_numerator == 6 {
                Payout::SixToFive
            } else {
                Payout::ThreeToTwo
            },
        })
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Peek {
    Uncleared,
    NoBlackjack,
}
#[derive(Clone, Copy)]
pub struct Decision {
    pub hand: Hand,
    pub upcard: Card,
    pub legal_mask: u8,
    pub rules: Rules,
    pub peek: Peek,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Status {
    Ok,
    SplitUnsupported,
    InvalidInput,
    UnsupportedRules,
    PeekRequired,
    NoDecision,
}
#[derive(Clone, Copy, Debug)]
pub struct Result {
    pub status: Status,
    pub complete: bool,
    pub evaluated_mask: u8,
    pub omitted_mask: u8,
    pub best_mask: u8,
    pub values: [f64; 5],
}
impl Default for Result {
    fn default() -> Self {
        Self {
            status: Status::InvalidInput,
            complete: false,
            evaluated_mask: 0,
            omitted_mask: 0,
            best_mask: 0,
            values: [f64::NAN; 5],
        }
    }
}
impl Result {
    pub fn value(self, action: Action) -> Option<f64> {
        if self.evaluated_mask & action.bit() != 0 {
            Some(self.values[action as usize])
        } else {
            None
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct DealerDistribution {
    pub outcomes: [f64; 6],
    pub blackjack: f64,
}
pub fn rank(c: Card) -> u8 {
    if c.rank() == 14 { 1 } else { c.value() }
}
fn total(minimum: usize, ace: bool) -> usize {
    minimum + if ace && minimum <= 11 { 10 } else { 0 }
}
fn summarize(h: &Hand) -> Option<(usize, bool)> {
    if h.count() < 2 || (h.split_aces && !h.split) {
        return None;
    }
    let mut minimum = 0;
    let mut ace = false;
    for &c in h.cards() {
        let r = rank(c);
        minimum += r as usize;
        ace |= r == 1
    }
    Some((minimum, ace))
}
#[derive(Default)]
pub struct Evaluator {
    dealer_states: [[[f64; 6]; 2]; 22],
    continuation: [[f64; 2]; 22],
    stand: [f64; 22],
    distribution: DealerDistribution,
    prepared: Option<(u8, bool, Peek)>,
}
impl Evaluator {
    fn after_card(&self, minimum: usize, ace: bool, double: bool) -> f64 {
        let mut net = 0.0;
        for rank in 1..=10 {
            let next = minimum + rank;
            let child = if next > 21 {
                -1.0
            } else if double {
                self.stand[total(next, ace || rank == 1)]
            } else {
                self.continuation[next][usize::from(ace || rank == 1)]
            };
            net += if rank == 10 { 4.0 } else { 1.0 } * child;
        }
        net / 13.0 * if double { 2.0 } else { 1.0 }
    }
    fn prepare(&mut self, up: u8, h17: bool, peek: Peek) {
        if self.prepared == Some((up, h17, peek)) {
            return;
        }
        for minimum in (0..=21).rev() {
            for ace in 0..2 {
                let mut out = [0.0; 6];
                let t = total(minimum, ace != 0);
                let soft = ace != 0 && minimum <= 11;
                if t >= 17 && !(h17 && soft && t == 17) {
                    out[t - 16] = 1.0;
                } else {
                    for rank in 1..=10 {
                        let next = minimum + rank;
                        let weight = if rank == 10 { 4.0 } else { 1.0 };
                        if next > 21 {
                            out[0] += weight;
                            continue;
                        }
                        let child = self.dealer_states[next][usize::from(ace != 0 || rank == 1)];
                        for i in 0..6 {
                            out[i] += weight * child[i];
                        }
                    }
                    for v in &mut out {
                        *v /= 13.0
                    }
                }
                self.dealer_states[minimum][ace] = out;
            }
        }
        self.distribution = DealerDistribution::default();
        let denominator = 13
            - if peek == Peek::NoBlackjack {
                if up == 1 {
                    4
                } else if up == 10 {
                    1
                } else {
                    0
                }
            } else {
                0
            };
        for hole in 1..=10 {
            let weight = if hole == 10 { 4.0 } else { 1.0 };
            let bj = (up == 1 && hole == 10) || (up == 10 && hole == 1);
            if bj {
                if peek == Peek::Uncleared {
                    self.distribution.blackjack += weight / denominator as f64;
                }
                continue;
            }
            let child = self.dealer_states[(up + hole) as usize][usize::from(up == 1 || hole == 1)];
            for (i, v) in child.into_iter().enumerate() {
                self.distribution.outcomes[i] += weight * v;
            }
        }
        for v in &mut self.distribution.outcomes {
            *v /= denominator as f64;
        }
        for t in 0..=21 {
            self.stand[t] = stand(t, self.distribution);
        }
        for minimum in (0..=21).rev() {
            for ace in 0..2 {
                let t = total(minimum, ace != 0);
                let stand = self.stand[t];
                self.continuation[minimum][ace] = if t == 21 {
                    stand
                } else {
                    stand.max(self.after_card(minimum, ace != 0, false))
                };
            }
        }
        self.prepared = Some((up, h17, peek));
    }
    pub fn evaluate(&mut self, d: Decision) -> Result {
        let mut out = Result::default();
        let Some((minimum, ace)) = summarize(&d.hand) else {
            return out;
        };
        if d.legal_mask & !31 != 0 {
            return out;
        }
        let t = total(minimum, ace);
        if t >= 21 || d.hand.split_aces {
            out.status = Status::NoDecision;
            return out;
        }
        if d.legal_mask & Action::Stand.bit() == 0
            || (d.legal_mask & Action::Double.bit() != 0
                && (d.hand.count() != 2 || (d.hand.split && !d.rules.double_after_split)))
            || (d.legal_mask & Action::Surrender.bit() != 0
                && (d.hand.count() != 2 || d.hand.split || !d.rules.late_surrender))
            || (d.legal_mask & Action::Split.bit() != 0
                && (d.hand.count() != 2 || rank(d.hand.cards()[0]) != rank(d.hand.cards()[1])))
        {
            return out;
        }
        let up = rank(d.upcard);
        if matches!(up, 1 | 10) && d.peek != Peek::NoBlackjack {
            out.status = Status::PeekRequired;
            return out;
        }
        self.prepare(up, d.rules.hit_soft_17, d.peek);
        for (action, value) in [
            (Action::Stand, self.stand[t]),
            (Action::Hit, self.after_card(minimum, ace, false)),
            (Action::Double, self.after_card(minimum, ace, true)),
            (Action::Surrender, -0.5),
        ] {
            if d.legal_mask & action.bit() != 0 {
                out.evaluated_mask |= action.bit();
                out.values[action as usize] = value
            }
        }
        out.omitted_mask = d.legal_mask & Action::Split.bit();
        out.complete = out.omitted_mask == 0;
        out.status = if out.complete {
            Status::Ok
        } else {
            Status::SplitUnsupported
        };
        if out.complete {
            let mut best = f64::NEG_INFINITY;
            for action in Action::ALL {
                if let Some(v) = out.value(action) {
                    best = best.max(v)
                }
            }
            for action in Action::ALL {
                if out
                    .value(action)
                    .is_some_and(|v| best - v <= 2.0 * ABSOLUTE_ERROR_BOUND)
                {
                    out.best_mask |= action.bit();
                }
            }
        }
        out
    }
    pub fn dealer(&mut self, upcard: Card, rules: Rules, peek: Peek) -> DealerDistribution {
        self.prepare(rank(upcard), rules.hit_soft_17, peek);
        self.distribution
    }
    pub fn natural_settlement(hand: &Hand, upcard: Card, rules: Rules, peek: Peek) -> Option<f64> {
        let (minimum, ace) = summarize(hand)?;
        if hand.split || hand.count() != 2 || total(minimum, ace) != 21 {
            return None;
        }
        let q = if peek == Peek::NoBlackjack {
            0.0
        } else {
            match rank(upcard) {
                1 => 4.0 / 13.0,
                10 => 1.0 / 13.0,
                _ => 0.0,
            }
        };
        Some(
            (1.0 - q)
                * match rules.payout {
                    Payout::ThreeToTwo => 1.5,
                    Payout::SixToFive => 6.0 / 5.0,
                },
        )
    }
    pub fn insurance(upcard: Card) -> Option<f64> {
        if rank(upcard) == 1 {
            Some(0.5 * (3.0 * (4.0 / 13.0) - 1.0))
        } else {
            None
        }
    }
}
pub(crate) fn stand(total: usize, d: DealerDistribution) -> f64 {
    let mut net = d.outcomes[0] - d.blackjack;
    for dealer in 17..=21 {
        net += if total > dealer {
            1.0
        } else if total < dealer {
            -1.0
        } else {
            0.0
        } * d.outcomes[dealer - 16];
    }
    net
}
