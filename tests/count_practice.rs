use stardust_blackjack::{
    count_practice::{Answer, Kind, Model, Question, Snapshot, Status},
    training::COUNTER_LIMIT,
};
fn snapshot(running: i16, undealt: u16) -> Snapshot {
    Snapshot {
        running,
        undealt,
        decks: 6,
        observed: 312 - undealt,
        shoe: 1,
        session: 17,
    }
}
fn question(token: u64, kind: Kind, given: Option<u8>) -> Question {
    Question {
        token,
        kind,
        given_half_decks: given,
    }
}
#[test]
fn questions_do_not_reveal_answers_and_first_attempts_remain_immutable() {
    let mut m = Model::default();
    assert!(m.begin(question(1, Kind::Running, None), snapshot(5, 156)));
    assert!(m.reference(1).is_none());
    let first = m.submit(1, Answer::Running(4), false).attempt.unwrap();
    assert!(!first.correct && !first.assisted);
    assert_eq!(first.snapshot.session, 17);
    assert_eq!(m.reference(1).unwrap().running, 5);
    let retry = m.submit(1, Answer::Running(5), true);
    assert_eq!(retry.status, Status::Duplicate);
    assert_eq!(retry.attempt.unwrap().answer, Answer::Running(4));
    assert_eq!(m.stats().all.errors, 1);
    assert_eq!(m.stats().all.assisted, 0);
}
#[test]
fn explicit_help_and_invalid_assisted_answers_stay_assisted() {
    let mut m = Model::default();
    m.begin(question(1, Kind::Running, None), snapshot(5, 156));
    assert_eq!(m.request_help(1).unwrap().running, 5);
    assert!(
        m.submit(1, Answer::Running(5), false)
            .attempt
            .unwrap()
            .assisted
    );
    m.begin(question(2, Kind::Decks, None), snapshot(0, 104));
    assert_eq!(
        m.submit(2, Answer::TrueCount(0), true).status,
        Status::Unavailable
    );
    assert!(m.reference(2).is_some());
    assert!(
        m.submit(2, Answer::Decks(4), false)
            .attempt
            .unwrap()
            .assisted
    );
    assert_eq!(m.stats().all.correct, 0);
    assert_eq!(m.stats().all.assisted, 2);
}
#[test]
fn true_count_truncates_toward_zero_and_given_divisors_are_explicit() {
    for (rc, undealt, wanted) in [(-7, 104, -3), (-1, 104, 0), (7, 104, 3), (-6, 156, -2)] {
        let mut m = Model::default();
        assert!(m.begin(question(1, Kind::TrueCount, None), snapshot(rc, undealt)));
        assert!(
            m.submit(1, Answer::TrueCount(wanted), false)
                .attempt
                .unwrap()
                .correct
        );
    }
    let mut m = Model::default();
    assert!(m.begin(question(1, Kind::TrueCount, Some(6)), snapshot(5, 117)));
    let a = m.submit(1, Answer::TrueCount(1), false).attempt.unwrap();
    assert!(a.correct);
    assert_eq!(a.reference.physical_index, Some(2));
    assert_eq!(a.reference.given_index, Some(1));
    assert!(!m.begin(question(2, Kind::TrueCount, Some(0)), snapshot(5, 117)));
}
#[test]
fn deck_grading_has_exact_half_deck_tolerance_and_ties_round_up() {
    for (cards, answer, correct, rounded) in [
        (104, 3, true, 4),
        (104, 5, true, 4),
        (105, 3, false, 4),
        (105, 5, true, 4),
        (103, 5, false, 4),
        (117, 4, true, 5),
        (117, 5, true, 5),
        (117, 6, false, 5),
        (13, 0, true, 1),
    ] {
        let mut m = Model::default();
        assert!(m.begin(question(1, Kind::Decks, None), snapshot(0, cards)));
        let a = m.submit(1, Answer::Decks(answer), false).attempt.unwrap();
        assert_eq!(a.correct, correct);
        assert_eq!(a.reference.rounded_half_decks, rounded);
    }
}
#[test]
fn cancellations_history_rollover_and_saved_metrics_keep_tokens_consumed() {
    let mut m = Model::default();
    assert!(m.begin(question(1, Kind::Running, None), snapshot(0, 156)));
    assert!(m.cancel(1));
    assert!(!m.begin(question(1, Kind::Running, None), snapshot(0, 156)));
    assert!(m.reference(1).is_none());
    for token in 2..150 {
        assert!(m.begin(question(token, Kind::Running, None), snapshot(0, 156)));
        assert_eq!(
            m.submit(token, Answer::Running(0), false).status,
            Status::Recorded
        );
    }
    assert_eq!(m.history_count(), 128);
    assert_eq!(m.stats().all.correct, 148);
    assert!(m.stats().valid());
    assert!(m.newest(usize::MAX).is_none());
    let stats = m.stats();
    let mut restored = Model::default();
    assert!(restored.restore(stats));
    assert_eq!(restored.stats(), stats);
    assert_eq!(restored.history_count(), 0);
    assert!(!m.restore(stats));
    assert!(m.reset_stats());
    assert!(!m.begin(question(149, Kind::Running, None), snapshot(0, 156)));
}
#[test]
fn per_skill_accuracy_and_counter_ceiling_are_independent() {
    let mut m = Model::default();
    for (token, kind, answer) in [
        (1, Kind::Running, Answer::Running(2)),
        (2, Kind::Decks, Answer::Decks(4)),
        (3, Kind::TrueCount, Answer::TrueCount(1)),
    ] {
        assert!(m.begin(question(token, kind, None), snapshot(2, 104)));
        assert!(m.submit(token, answer, false).attempt.unwrap().correct);
    }
    let s = m.stats();
    assert!(s.valid());
    assert_eq!(s.all.correct, 3);
    for c in s.kinds {
        assert_eq!(c.correct, 1);
    }
    let mut max = s;
    max.all.decisions = COUNTER_LIMIT;
    max.all.correct = COUNTER_LIMIT;
    max.kinds = [Default::default(); 3];
    max.kinds[0] = max.all;
    let mut ceiling = Model::default();
    assert!(ceiling.restore(max));
    assert!(ceiling.begin(question(1, Kind::Running, None), snapshot(0, 104)));
    assert_eq!(
        ceiling.submit(1, Answer::Running(0), false).status,
        Status::Unavailable
    );
    assert_eq!(ceiling.stats(), max);
}
