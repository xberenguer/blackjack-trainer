//! Initial-pair, fully funded, sequential-deal replacement model. Optional
//! resplit decisions optimize the finite queue DAG; arbitrary live splits are N/A.
use crate::{
    cards::{Card, Hand, UNKNOWN_POSITION},
    engine::Action,
    ev::{self, Decision, Evaluator, Status},
};
pub const ABSOLUTE_ERROR_BOUND: f64 = 1e-8;
#[derive(Clone, Copy)]
pub struct Request {
    pub decision: Decision,
    pub hands_in_round: u8,
    pub active_hand: u8,
    pub previous_splits: u8,
    pub first_decision: bool,
    pub original_wager: i64,
    pub uncommitted: i64,
    pub max_hands: u8,
    pub resplit_aces: bool,
    pub one_card_aces: bool,
    pub sequential: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SplitStatus {
    Ok,
    InvalidInput,
    UnsupportedRules,
    UnsupportedContext,
    InsufficientFunds,
    PeekRequired,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
#[repr(u8)]
pub enum ResplitPolicy {
    #[default]
    Unreachable,
    Play,
    Resplit,
    Indifferent,
}
#[derive(Clone, Copy)]
pub struct Result {
    pub status: SplitStatus,
    pub value: Option<f64>,
    pub matching: [[ResplitPolicy; 3]; 5],
}
impl Default for Result {
    fn default() -> Self {
        Self {
            status: SplitStatus::UnsupportedContext,
            value: None,
            matching: [[ResplitPolicy::Unreachable; 3]; 5],
        }
    }
}
#[derive(Default)]
pub struct SplitEvaluator {
    ordinary: Evaluator,
}
fn total(a: usize, b: usize) -> usize {
    let min = a + b;
    min + if (a == 1 || b == 1) && min <= 11 {
        10
    } else {
        0
    }
}
fn card(rank: u8) -> Card {
    Card::from_id(if rank == 1 { 12 } else { rank - 2 }).unwrap()
}
impl SplitEvaluator {
    pub fn evaluate(&mut self, r: Request) -> Result {
        let mut out = Result::default();
        let d = r.decision;
        if r.max_hands != 4 || r.resplit_aces || !r.one_card_aces || !r.sequential {
            out.status = SplitStatus::UnsupportedRules;
            return out;
        }
        if !r.first_decision
            || r.hands_in_round != 1
            || r.active_hand != 0
            || r.previous_splits != 0
            || d.hand.split
            || d.hand.split_aces
            || d.hand.count() != 2
            || d.legal_mask & Action::Split.bit() == 0
        {
            return out;
        }
        if r.original_wager <= 0 || r.uncommitted < 0 {
            out.status = SplitStatus::InvalidInput;
            return out;
        }
        let original = self.ordinary.evaluate(d);
        if original.status != Status::SplitUnsupported {
            out.status = match original.status {
                Status::PeekRequired => SplitStatus::PeekRequired,
                Status::UnsupportedRules => SplitStatus::UnsupportedRules,
                _ => SplitStatus::InvalidInput,
            };
            return out;
        }
        let pair = ev::rank(d.hand.cards()[0]) as usize;
        let required = if pair == 1 {
            1
        } else if d.rules.double_after_split {
            7
        } else {
            3
        };
        if r.uncommitted / r.original_wager < required {
            out.status = SplitStatus::InsufficientFunds;
            return out;
        }
        let dealer = self.ordinary.dealer(d.upcard, d.rules, d.peek);
        let mut play = [0.0; 11];
        for (rank, value) in play.iter_mut().enumerate().skip(1) {
            if pair == 1 || total(pair, rank) == 21 {
                *value = ev::stand(total(pair, rank), dealer);
                continue;
            }
            let mut hand = Hand::default();
            hand.add(card(pair as u8), UNKNOWN_POSITION);
            hand.add(card(rank as u8), UNKNOWN_POSITION);
            hand.split = true;
            let legal_mask = Action::Stand.bit()
                | Action::Hit.bit()
                | if d.rules.double_after_split {
                    Action::Double.bit()
                } else {
                    0
                };
            let ordinary = self.ordinary.evaluate(Decision {
                hand,
                legal_mask,
                ..d
            });
            if ordinary.status != Status::Ok || !ordinary.complete {
                out.status = SplitStatus::InvalidInput;
                return out;
            }
            let mut best = f64::NEG_INFINITY;
            for action in Action::ALL {
                if let Some(v) = ordinary.value(action) {
                    best = best.max(v)
                }
            }
            *value = best;
        }
        let value = if pair == 1 {
            let mut value = 0.0;
            for (rank, v) in play.iter().enumerate().skip(1) {
                value += if rank == 10 { 4.0 } else { 1.0 } * v
            }
            value * 2.0 / 13.0
        } else {
            let mut queue = [[0.0; 3]; 5];
            for remaining in 0..=2 {
                for queued in 1..=4 - remaining {
                    let mut sum = 0.0;
                    for (rank, v) in play.iter().enumerate().skip(1) {
                        let ordinary = v + queue[queued - 1][remaining];
                        let mut next = ordinary;
                        if rank == pair {
                            out.matching[queued][remaining] = ResplitPolicy::Play;
                            if remaining > 0 {
                                let resplit = queue[queued + 1][remaining - 1];
                                next = ordinary.max(resplit);
                                out.matching[queued][remaining] =
                                    if (ordinary - resplit).abs() <= 2.0 * ABSOLUTE_ERROR_BOUND {
                                        ResplitPolicy::Indifferent
                                    } else if resplit > ordinary {
                                        ResplitPolicy::Resplit
                                    } else {
                                        ResplitPolicy::Play
                                    };
                            }
                        }
                        sum += if rank == 10 { 4.0 } else { 1.0 } * next;
                    }
                    queue[queued][remaining] = sum / 13.0;
                }
            }
            queue[2][2]
        };
        out.status = SplitStatus::Ok;
        out.value = Some(value);
        out
    }
}
