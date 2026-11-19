use stardust_blackjack::{
    cards::{Card, Hand, UNKNOWN_POSITION},
    engine::Action,
    ev::{self, Decision, Evaluator, Payout, Peek, Rules, Status},
    split_ev::{Request, SplitEvaluator, SplitStatus},
};
fn card(r: u8) -> Card {
    Card::from_id(if r == 1 { 12 } else { r - 2 }).unwrap()
}
fn hand(a: u8, b: u8) -> Hand {
    let mut h = Hand::default();
    h.bet = 500;
    h.add(card(a), UNKNOWN_POSITION);
    h.add(card(b), UNKNOWN_POSITION);
    h
}
fn decision(a: u8, b: u8, up: u8, mask: u8) -> Decision {
    Decision {
        hand: hand(a, b),
        upcard: card(up),
        legal_mask: mask,
        rules: Rules::default(),
        peek: Peek::NoBlackjack,
    }
}
fn request(pair: u8) -> Request {
    Request {
        decision: decision(pair, pair, 6, 31),
        hands_in_round: 1,
        active_hand: 0,
        previous_splits: 0,
        first_decision: true,
        original_wager: 500,
        uncommitted: 3500,
        max_hands: 4,
        resplit_aces: false,
        one_card_aces: true,
        sequential: true,
    }
}
#[test]
fn unavailable_split_is_nan_and_never_produces_a_false_best_move() {
    let mut e = Evaluator::default();
    let partial = e.evaluate(decision(8, 8, 6, 31));
    assert_eq!(partial.status, Status::SplitUnsupported);
    assert!(!partial.complete);
    assert_eq!(partial.omitted_mask, Action::Split.bit());
    assert_eq!(partial.best_mask, 0);
    assert!(partial.values[Action::Split as usize].is_nan());
    assert!(partial.value(Action::Split).is_none());
    for a in [
        Action::Hit,
        Action::Stand,
        Action::Double,
        Action::Surrender,
    ] {
        assert!(partial.value(a).unwrap().is_finite());
    }
    let complete = e.evaluate(decision(8, 8, 6, 31 & !Action::Split.bit()));
    assert!(complete.complete);
    assert_ne!(complete.best_mask, 0);
}
#[test]
fn peek_naturals_and_insurance_are_separate_from_action_decisions() {
    let mut e = Evaluator::default();
    let mut d = decision(10, 6, 1, 3);
    d.peek = Peek::Uncleared;
    assert_eq!(e.evaluate(d).status, Status::PeekRequired);
    d.upcard = card(10);
    assert_eq!(e.evaluate(d).status, Status::PeekRequired);
    d.upcard = card(6);
    assert_eq!(e.evaluate(d).status, Status::Ok);
    assert_eq!(e.evaluate(decision(1, 10, 6, 3)).status, Status::NoDecision);
    assert!((Evaluator::insurance(card(1)).unwrap() + 1.0 / 26.0).abs() < 1e-15);
    assert!(Evaluator::insurance(card(6)).is_none());
    for (payout, profit) in [(Payout::ThreeToTwo, 1.5), (Payout::SixToFive, 1.2)] {
        let r = Rules {
            payout,
            ..Rules::default()
        };
        assert_eq!(
            Evaluator::natural_settlement(&hand(1, 10), card(1), r, Peek::NoBlackjack),
            Some(profit)
        );
        let pre = Evaluator::natural_settlement(&hand(1, 10), card(1), r, Peek::Uncleared).unwrap();
        assert!((pre - profit * 9.0 / 13.0).abs() < 1e-15);
    }
}
#[test]
fn illegal_capabilities_and_resolved_hands_are_rejected() {
    let mut e = Evaluator::default();
    for defect in 0..8 {
        let mut d = decision(8, 3, 6, 23);
        match defect {
            0 => d.legal_mask &= !Action::Stand.bit(),
            1 => {
                d.hand.add(card(2), UNKNOWN_POSITION);
            }
            2 => d.hand.split = true,
            3 => d.rules.late_surrender = false,
            4 => d.legal_mask |= Action::Split.bit(),
            5 => d.legal_mask |= 128,
            6 => {
                d.hand.split = true;
                d.rules.double_after_split = false;
                d.legal_mask = 7;
            }
            _ => d.hand.split_aces = true,
        }
        assert_eq!(e.evaluate(d).status, Status::InvalidInput);
    }
}
#[test]
fn split_funding_is_bound_to_the_entire_supported_tree() {
    let mut e = SplitEvaluator::default();
    for (pair, das, units) in [(1, true, 1), (8, false, 3), (8, true, 7)] {
        let mut r = request(pair);
        r.decision.rules.double_after_split = das;
        r.uncommitted = units * 500 - 1;
        assert_eq!(e.evaluate(r).status, SplitStatus::InsufficientFunds);
        r.uncommitted += 1;
        let v = e.evaluate(r);
        assert_eq!(v.status, SplitStatus::Ok);
        assert!(v.value.unwrap().is_finite());
    }
    let mut r = request(8);
    r.original_wager = i64::MAX;
    r.uncommitted = i64::MAX;
    assert_eq!(e.evaluate(r).status, SplitStatus::InsufficientFunds);
}
#[test]
fn unsupported_split_context_is_explicit() {
    let mut e = SplitEvaluator::default();
    for defect in 0..11 {
        let mut r = request(8);
        let expected = match defect {
            0 => {
                r.max_hands = 5;
                SplitStatus::UnsupportedRules
            }
            1 => {
                r.resplit_aces = true;
                SplitStatus::UnsupportedRules
            }
            2 => {
                r.one_card_aces = false;
                SplitStatus::UnsupportedRules
            }
            3 => {
                r.sequential = false;
                SplitStatus::UnsupportedRules
            }
            4 => {
                r.first_decision = false;
                SplitStatus::UnsupportedContext
            }
            5 => {
                r.hands_in_round = 2;
                SplitStatus::UnsupportedContext
            }
            6 => {
                r.active_hand = 1;
                SplitStatus::UnsupportedContext
            }
            7 => {
                r.previous_splits = 1;
                SplitStatus::UnsupportedContext
            }
            8 => {
                r.decision.hand.split = true;
                SplitStatus::UnsupportedContext
            }
            9 => {
                r.original_wager = 0;
                SplitStatus::InvalidInput
            }
            _ => {
                r.decision = decision(8, 8, 10, 31);
                r.decision.peek = Peek::Uncleared;
                SplitStatus::PeekRequired
            }
        };
        let result = e.evaluate(r);
        assert_eq!(result.status, expected);
        assert!(result.value.is_none());
    }
}
#[test]
fn ev_workspaces_are_small_and_allocation_free() {
    assert!(core::mem::size_of::<Evaluator>() <= 2800);
    assert!(core::mem::size_of::<SplitEvaluator>() <= 2800);
    assert_eq!(core::mem::size_of::<f64>(), 8);
    assert_eq!(f64::MANTISSA_DIGITS, 53);
    assert_eq!(ev::ABSOLUTE_ERROR_BOUND, 1e-10);
}
