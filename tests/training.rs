use stardust_blackjack::{
    cards::{Card, Hand, UNKNOWN_POSITION},
    deviations::Count,
    engine::Action,
    rules::Profile,
    training::{
        self, Attempt, COUNTER_LIMIT, Choice, ExpectedValues, Mode, Model, Policy, Stats, Status,
    },
};
fn hand(a: u8, b: u8) -> Hand {
    let mut h = Hand::default();
    h.bet = 500;
    h.add(Card::from_id(a - 2).unwrap(), UNKNOWN_POSITION);
    h.add(Card::from_id(b - 2).unwrap(), UNKNOWN_POSITION);
    h
}
fn input(token: u64, choice: Action) -> Attempt {
    Attempt {
        token,
        round: 1,
        hand_index: 0,
        profile: Profile::SixH17,
        hand: hand(8, 3),
        dealer_up: Card::from_id(4).unwrap(),
        legal_mask: 23,
        choice: Choice::Play(choice),
        policy: Policy::Basic,
        count: None,
        assisted: false,
        values: None,
    }
}
#[test]
fn coach_first_attempt_is_immutable_across_corrections_and_mode_changes() {
    let mut m = Model::default();
    assert_eq!(m.record(input(1, Action::Stand)).status, Status::Off);
    m.mode = Mode::Coach;
    m.pause_on_mistake = true;
    let first = m.record(input(1, Action::Stand));
    assert_eq!(first.status, Status::Recorded);
    assert!(first.pause && !first.decision.unwrap().correct);
    assert_eq!(first.decision.unwrap().best, Choice::Play(Action::Double));
    let mut correction = input(1, Action::Double);
    correction.assisted = true;
    m.mode = Mode::Guide;
    let retry = m.record(correction);
    assert_eq!(retry.status, Status::Duplicate);
    assert!(!retry.pause);
    assert_eq!(
        retry.decision.unwrap().attempt.choice,
        Choice::Play(Action::Stand)
    );
    assert!(!retry.decision.unwrap().assisted);
    assert_eq!(m.stats().all.errors, 1);
    assert_eq!(m.stats().all.assisted, 0);
}
#[test]
fn guide_and_explicit_assistance_are_separate_from_accuracy() {
    let mut m = Model::default();
    m.mode = Mode::Test;
    assert!(m.record(input(1, Action::Double)).decision.unwrap().correct);
    assert_eq!(m.stats().streak, 1);
    m.mode = Mode::Guide;
    let wrong = m.record(input(2, Action::Stand)).decision.unwrap();
    assert!(!wrong.correct && wrong.assisted);
    let right = m.record(input(3, Action::Double)).decision.unwrap();
    assert!(right.correct && right.assisted);
    m.mode = Mode::Test;
    let mut hint = input(4, Action::Double);
    hint.assisted = true;
    m.record(hint);
    let s = m.stats();
    assert_eq!(
        (s.all.decisions, s.all.correct, s.all.errors, s.all.assisted),
        (4, 1, 0, 3)
    );
    assert_eq!((s.streak, s.best_streak), (0, 1));
    assert!(s.valid());
}
#[test]
fn hilo_grades_against_frozen_public_count_and_profile() {
    let mut m = Model::default();
    m.mode = Mode::Test;
    let mut p = input(1, Action::Stand);
    p.profile = Profile::SixNoSurrender;
    p.hand = hand(10, 6);
    p.dealer_up = Card::from_id(8).unwrap();
    p.legal_mask = 3;
    p.policy = Policy::HiLo;
    p.count = Some(Count {
        running: 1,
        undealt: 300,
        decks: 6,
        observed: 10,
        shoe: 2,
    });
    let d = m.record(p).decision.unwrap();
    assert!(d.correct && !d.assisted);
    assert_eq!(d.best, Choice::Play(Action::Stand));
    assert_eq!(
        d.attempt
            .count
            .unwrap()
            .integer_true_count(stardust_blackjack::rules::Rules::for_profile(
                d.attempt.profile
            )),
        Some(0)
    );
    p.count.as_mut().unwrap().running = -1;
    p.profile = Profile::SixH17;
    assert_eq!(
        m.newest(0).unwrap().attempt.profile,
        Profile::SixNoSurrender
    );
    assert_eq!(m.newest(0).unwrap().attempt.count.unwrap().running, 1);
    assert_eq!(m.record(p).status, Status::Duplicate);
}
#[test]
fn insurance_uses_selected_policy_and_affordability() {
    for policy in [Policy::Basic, Policy::HiLo] {
        let mut m = Model::default();
        m.mode = Mode::Test;
        let mut p = input(1, Action::Stand);
        p.hand = hand(10, 9);
        p.dealer_up = Card::from_id(12).unwrap();
        p.choice = Choice::Insurance(true);
        p.legal_mask = 3;
        p.policy = policy;
        if policy == Policy::HiLo {
            p.count = Some(Count {
                running: 9,
                undealt: 156,
                decks: 6,
                observed: 156,
                shoe: 1,
            });
        }
        let d = m.record(p).decision.unwrap();
        assert_eq!(d.correct, policy == Policy::HiLo);
        assert_eq!(d.best, Choice::Insurance(policy == Policy::HiLo));
        p.token = 2;
        p.legal_mask = 1;
        assert_eq!(m.record(p).status, Status::Unavailable);
        p.choice = Choice::Insurance(false);
        assert!(m.record(p).decision.unwrap().correct);
    }
}
#[test]
fn ev_capture_is_not_assistance_and_values_are_frozen() {
    let mut m = Model::default();
    m.mode = Mode::Test;
    let mut p = input(1, Action::Stand);
    p.legal_mask = 7;
    p.values = Some(ExpectedValues {
        model: training::EvModel::InfiniteReplacementV1,
        original_wager: 500,
        legal_mask: 7,
        evaluated_mask: 7,
        best_mask: 4,
        initial_pair_funded: false,
        values: [-0.1, -0.2, 0.3, f64::NAN, f64::NAN],
    });
    let d = m.record(p).decision.unwrap();
    assert!(!d.correct && !d.assisted);
    p.values.as_mut().unwrap().values[1] = 0.5;
    assert_eq!(m.newest(0).unwrap().attempt.values.unwrap().values[1], -0.2);
    assert_eq!(m.record(p).status, Status::Duplicate);
    p.token = 2;
    assert_eq!(m.record(p).status, Status::Unavailable);
    assert_eq!(m.stats().all.decisions, 1);
}
#[test]
fn cumulative_round_totals_survive_history_eviction_and_reset_tokens_stay_consumed() {
    let mut m = Model::default();
    m.mode = Mode::Test;
    for token in 1..=160 {
        m.record(input(
            token,
            if token == 1 {
                Action::Stand
            } else {
                Action::Double
            },
        ));
    }
    assert_eq!(m.history_count(), 128);
    assert_eq!(m.history(0).unwrap().attempt.token, 33);
    assert_eq!(m.round_stats(1).decisions, 160);
    assert_eq!(m.round_stats(1).errors, 1);
    assert_eq!(m.record(input(1, Action::Double)).status, Status::Duplicate);
    assert!(!m.reset_stats(false));
    assert!(m.reset_stats(true));
    assert_eq!(m.stats(), Stats::default());
    assert_eq!(
        m.record(input(160, Action::Double)).status,
        Status::Duplicate
    );
    assert_eq!(
        m.record(input(161, Action::Double)).status,
        Status::Recorded
    );
}
#[test]
fn invalid_public_context_and_counter_overflow_never_change_progress() {
    let mut m = Model::default();
    m.mode = Mode::Test;
    for defect in 0..5 {
        let mut p = input(1, Action::Double);
        p.policy = Policy::HiLo;
        p.count = Some(Count {
            running: 2,
            undealt: 156,
            decks: 6,
            observed: 156,
            shoe: 1,
        });
        match defect {
            0 => p.count = None,
            1 => p.count.as_mut().unwrap().shoe = 0,
            2 => p.count.as_mut().unwrap().undealt = 0,
            3 => p.count.as_mut().unwrap().decks = 8,
            _ => p.count.as_mut().unwrap().running = 200,
        }
        assert_eq!(m.record(p).status, Status::Unavailable);
        assert_eq!(m.stats(), Stats::default());
    }
    let mut s = Stats {
        all: training::Counters {
            decisions: COUNTER_LIMIT,
            correct: COUNTER_LIMIT,
            errors: 0,
            assisted: 0,
        },
        ..Stats::default()
    };
    s.categories[0] = s.all;
    assert!(s.valid());
    let mut ceiling = Model::default();
    assert!(ceiling.restore(s, false));
    assert_eq!(ceiling.mode, Mode::Off);
    ceiling.mode = Mode::Test;
    assert_eq!(
        ceiling.record(input(1, Action::Double)).status,
        Status::Unavailable
    );
    assert_eq!(ceiling.stats(), s);
}
