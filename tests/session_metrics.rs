use stardust_blackjack::session_metrics::{Metrics, Skip};
use stardust_blackjack::{
    cards::{Card, Hand},
    deviations::Count,
    engine::Action,
    rules::Profile,
    training::{self, Attempt, Choice, EvModel, ExpectedValues, Mode, Policy},
    wager_practice as wager,
};
fn hand(pair: bool, bet: i64) -> Hand {
    let mut h = Hand::default();
    h.bet = bet;
    h.add(Card::from_id(if pair { 6 } else { 3 }).unwrap(), 10);
    h.add(Card::from_id(if pair { 19 } else { 5 }).unwrap(), 11);
    h
}
fn decision(token: u64, assisted: bool) -> training::Decision {
    let h = hand(false, 500);
    let ev = ExpectedValues {
        model: EvModel::InfiniteReplacementV1,
        original_wager: 500,
        legal_mask: 3,
        evaluated_mask: 3,
        best_mask: 2,
        initial_pair_funded: false,
        values: [-0.2, 0.1, f64::NAN, f64::NAN, f64::NAN],
    };
    let a = Attempt {
        token,
        round: 1,
        hand_index: 0,
        profile: Profile::SixH17,
        hand: h,
        dealer_up: Card::from_id(4).unwrap(),
        legal_mask: 3,
        choice: Choice::Play(Action::Hit),
        policy: Policy::Basic,
        count: None,
        assisted,
        values: Some(ev),
    };
    let mut m = training::Model::default();
    m.mode = Mode::Test;
    m.record(a).decision.unwrap()
}
#[test]
fn freezes_original_choice_model_wager_and_assistance_with_duplicate_guard() {
    let mut m = Metrics::default();
    let d = decision(1, false);
    assert!(!d.correct);
    assert!(m.record_card(d));
    let record = m.newest(0).unwrap();
    assert_eq!(record.choice, Choice::Play(Action::Hit));
    let comparison = record.comparison.unwrap();
    assert_eq!(comparison.original_wager, 500);
    assert!((comparison.difference_cents - 150.0).abs() < 1e-10);
    let mut changed = d;
    changed.attempt.choice = Choice::Play(Action::Stand);
    changed.assisted = true;
    assert!(!m.record_card(changed));
    assert_eq!(m.newest(0), Some(record));
    assert_eq!(m.card_stats().all.errors, 1);
    assert_eq!(m.card_stats().all.assisted, 0);
    assert_eq!(m.costs().unassisted.decisions, 1);
    assert_eq!(m.costs().assisted.decisions, 0);
    assert_eq!(record.hand, d.attempt.hand);
    assert_eq!(record.dealer_up, d.attempt.dealer_up);
}
#[test]
fn keeps_assisted_model_difference_out_of_unassisted_bucket() {
    let mut m = Metrics::default();
    assert!(m.record_card(decision(1, false)));
    assert!(m.record_card(decision(2, true)));
    let c = m.costs();
    assert_eq!(c.unassisted.decisions, 1);
    assert_eq!(c.assisted.decisions, 1);
    assert_eq!(c.unassisted.difference_cents, c.assisted.difference_cents);
    assert_eq!(m.card_stats().all.errors, 1);
    assert_eq!(m.card_stats().all.assisted, 1);
    assert!(m.card_stats().valid());
}
#[test]
fn correct_hilo_insurance_is_explicitly_skipped_and_never_regraded_as_model_error() {
    let mut a = decision(1, false).attempt;
    a.hand = hand(true, 500);
    a.dealer_up = Card::from_id(12).unwrap();
    a.policy = Policy::HiLo;
    a.count = Some(Count {
        running: 6,
        undealt: 104,
        decks: 6,
        observed: 208,
        shoe: 1,
    });
    a.choice = Choice::Insurance(true);
    a.values = Some(ExpectedValues {
        model: EvModel::InfiniteReplacementV1,
        original_wager: 500,
        legal_mask: 3,
        evaluated_mask: 3,
        best_mask: 1,
        initial_pair_funded: false,
        values: [0.0, -1.0 / 26.0, f64::NAN, f64::NAN, f64::NAN],
    });
    let mut training = training::Model::default();
    training.mode = Mode::Test;
    let d = training.record(a).decision.unwrap();
    assert!(d.correct);
    let mut m = Metrics::default();
    assert!(m.record_card(d));
    assert_eq!(m.card_stats().all.correct, 1);
    assert_eq!(m.card_stats().all.errors, 0);
    assert_eq!(m.costs().skipped_hilo, 1);
    assert_eq!(m.costs().unassisted.decisions, 0);
    assert_eq!(m.costs().unassisted.difference_cents, 0.0);
    assert_eq!(m.newest(0).unwrap().skipped, Some(Skip::HiLoPolicy));
    assert!(m.newest(0).unwrap().comparison.is_none());
}
#[test]
fn unavailable_partial_or_invalid_values_never_become_zero_loss_samples() {
    let mut m = Metrics::default();
    let mut d = decision(1, false);
    d.attempt.values = None;
    assert!(m.record_card(d));
    assert_eq!(m.newest(0).unwrap().skipped, Some(Skip::NoSnapshot));
    let mut d = decision(2, false);
    d.attempt.hand = hand(true, 500);
    d.attempt.legal_mask = 11;
    let v = d.attempt.values.as_mut().unwrap();
    v.legal_mask = 11;
    v.best_mask = 0;
    assert!(m.record_card(d));
    assert_eq!(m.newest(0).unwrap().skipped, Some(Skip::PartialSnapshot));
    let mut d = decision(3, false);
    d.attempt.values.as_mut().unwrap().values[0] = f64::INFINITY;
    assert!(m.record_card(d));
    assert_eq!(m.newest(0).unwrap().skipped, Some(Skip::InvalidSnapshot));
    assert_eq!(m.costs().unavailable, 3);
    assert_eq!(m.costs().unassisted.decisions, 0);
    assert_eq!(m.costs().unassisted.mean_difference_cents, None);
}
#[test]
fn aggregate_evidence_survives_history_rollover() {
    let mut m = Metrics::EMPTY;
    for token in 1..=145 {
        assert!(m.record_card(decision(token, false)));
    }
    assert_eq!(m.history_count(), 128);
    assert_eq!(m.newest(127).unwrap().token, 18);
    assert_eq!(m.newest(0).unwrap().token, 145);
    assert!(m.newest(128).is_none());
    assert!(m.newest(usize::MAX).is_none());
    assert_eq!(m.card_stats().all.decisions, 145);
    assert!((m.costs().unassisted.difference_cents - 21750.0).abs() < 1e-8);
    assert!(!m.record_card(decision(1, false)));
    assert!(m.card_stats().valid());
}
#[test]
fn compensated_aggregation_keeps_small_model_differences_after_large_values() {
    let mut m = Metrics::EMPTY;
    let mut d = decision(1, false);
    d.attempt.hand.bet = 1_000_000;
    let v = d.attempt.values.as_mut().unwrap();
    v.original_wager = 1_000_000;
    v.values[0] = -1.0;
    v.values[1] = 1.0;
    assert!(m.record_card(d));
    for token in 2..=10001 {
        let mut d = decision(token, false);
        let v = d.attempt.values.as_mut().unwrap();
        v.values[0] = -1e-10;
        v.values[1] = 0.0;
        assert!(m.record_card(d));
    }
    assert!((m.costs().unassisted.difference_cents - (2_000_000.0 + 0.0005)).abs() < 1e-9);
}
#[test]
fn wager_accuracy_remains_separate_and_uses_original_assistance() {
    let mut m = Metrics::EMPTY;
    let mut w = wager::Model::EMPTY;
    for token in 1..=3 {
        let s = wager::Snapshot {
            context: wager::Context {
                session: 1,
                checkpoint: token,
                shoe: 1,
            },
            rules: stardust_blackjack::rules::Rules::default(),
            count: Count {
                running: 0,
                undealt: 104,
                decks: 6,
                observed: 208,
                shoe: 1,
            },
            bankroll: 100000,
            predeal: true,
            estimated_raw_tc: None,
        };
        assert!(w.begin(token, s, wager::Policy::EXAMPLE));
        let a = w
            .submit(
                token,
                wager::Choice::Wager(if token == 1 { 500 } else { 1000 }),
                token == 3,
            )
            .attempt
            .unwrap();
        assert!(m.record_wager(a));
        assert!(!m.record_wager(a));
    }
    assert_eq!(
        m.wager_stats().all,
        training::Counters {
            decisions: 3,
            correct: 1,
            errors: 1,
            assisted: 1
        }
    );
    assert_eq!(m.card_stats().all.decisions, 0);
    assert!(m.wager_stats().valid());
}
#[test]
fn session_reset_has_no_leak_from_previous_session_or_assistance() {
    let mut m = Metrics::EMPTY;
    m.record_card(decision(1, true));
    m = Metrics::EMPTY;
    assert_eq!(m.card_stats().all.decisions, 0);
    assert_eq!(m.history_count(), 0);
    assert_eq!(m.costs().assisted.decisions, 0);
    assert!(m.record_card(decision(2, false)));
    assert_eq!(m.card_stats().all.errors, 1);
}
