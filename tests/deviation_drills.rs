use stardust_blackjack::deviation_drills::{
    Boundary, Funding, Generator, IndexId as I, Model, Origin, Policy, Question, Stats, Status,
    Threshold, generate,
};
use stardust_blackjack::{
    cards::{Card, Hand, UNKNOWN_POSITION},
    deviations,
    engine::Action::{self, *},
    rules::Profile::{self, *},
    training::{Choice, Counters},
};
fn play(a: Action) -> Choice {
    Choice::Play(a)
}
fn reference(q: Question) -> stardust_blackjack::deviation_drills::Reference {
    let mut m = Model::default();
    assert!(m.begin(q));
    assert!(m.reference(q.token).is_none());
    m.request_help(q.token).unwrap()
}
#[test]
fn independently_transcribed_primary_chart_boundaries_cover_every_index() {
    // Independent fixtures, not computed from the production index table.
    let rows: [(I, Profile, [i16; 3], [Action; 3]); 30] = [
        (I::Tens4, SixH17, [17, 18, 19], [Stand, Split, Split]),
        (I::Tens5, SixH17, [14, 15, 16], [Stand, Split, Split]),
        (I::Tens6, SixH17, [11, 12, 13], [Stand, Split, Split]),
        (I::Soft19v4, SixH17, [8, 9, 10], [Stand, Double, Double]),
        (I::Soft19v5, SixH17, [2, 3, 4], [Stand, Double, Double]),
        (I::Soft19v6H17, SixH17, [-2, -1, 0], [Stand, Stand, Double]),
        (I::Soft19v6S17, SixS17, [2, 3, 4], [Stand, Double, Double]),
        (I::Soft17v2, SixH17, [2, 3, 4], [Hit, Double, Double]),
        (I::Hard16v9, SixH17, [11, 12, 13], [Hit, Stand, Stand]),
        (I::Hard16v10, SixH17, [0, 1, 2], [Hit, Stand, Stand]),
        (I::Hard16vAceH17, SixH17, [8, 9, 10], [Hit, Stand, Stand]),
        (I::Hard15v10, SixH17, [11, 12, 13], [Hit, Stand, Stand]),
        (I::Hard15vAceH17, SixH17, [14, 15, 16], [Hit, Stand, Stand]),
        (I::Hard13v2, SixH17, [-4, -3, -2], [Hit, Hit, Stand]),
        (I::Hard12v2, SixH17, [8, 9, 10], [Hit, Stand, Stand]),
        (I::Hard12v3, SixH17, [5, 6, 7], [Hit, Stand, Stand]),
        (I::Hard12v4, SixH17, [-2, -1, 0], [Hit, Hit, Stand]),
        (I::Hard11vAceS17, SixS17, [2, 3, 4], [Hit, Double, Double]),
        (I::Hard10v10, SixH17, [11, 12, 13], [Hit, Double, Double]),
        (I::Hard10vAceH17, SixH17, [8, 9, 10], [Hit, Double, Double]),
        (
            I::Hard10vAceS17,
            SixS17,
            [11, 12, 13],
            [Hit, Double, Double],
        ),
        (I::Hard9v2, SixH17, [2, 3, 4], [Hit, Double, Double]),
        (I::Hard9v7, SixH17, [8, 9, 10], [Hit, Double, Double]),
        (I::Hard8v6, SixH17, [5, 6, 7], [Hit, Double, Double]),
        (
            I::Surrender16v8,
            SixH17,
            [11, 12, 13],
            [Hit, Surrender, Surrender],
        ),
        (
            I::Surrender16v9,
            SixH17,
            [-4, -3, -2],
            [Hit, Hit, Surrender],
        ),
        (
            I::Surrender15v9,
            SixH17,
            [5, 6, 7],
            [Hit, Surrender, Surrender],
        ),
        (
            I::Surrender15v10,
            SixH17,
            [-2, -1, 0],
            [Hit, Hit, Surrender],
        ),
        (
            I::Surrender15vAceH17,
            SixH17,
            [-6, -5, -4],
            [Hit, Surrender, Surrender],
        ),
        (
            I::Surrender15vAceS17,
            SixS17,
            [5, 6, 7],
            [Hit, Surrender, Surrender],
        ),
    ];
    let sides = [Boundary::Below, Boundary::At, Boundary::Above];
    let mut covered = [false; I::COUNT];
    for (index, profile, rc, actions) in rows {
        for side in 0..3 {
            let q = generate(1, index, profile, sides[side]).expect("valid generated context");
            assert_eq!(q.context.count.running, rc[side], "{index:?}");
            let r = reference(q);
            assert_eq!(r.best, play(actions[side]), "{index:?} RC {}", rc[side]);
            assert_eq!(r.integer_true_count, rc[side] / 3);
            assert_eq!(r.policy, Policy::Bja2018HiLoTruncatedV1);
        }
        covered[index as usize] = true;
    }
    for (side, expected) in sides.into_iter().zip([false, true, true]) {
        let q = generate(1, I::Insurance, SixH17, side).unwrap();
        assert_eq!(reference(q).best, Choice::Insurance(expected));
    }
    covered[I::Insurance as usize] = true;
    assert!(covered.into_iter().all(|b| b));
}
#[test]
fn strict_running_count_zero_and_negative_truncation_are_preserved() {
    let positive = generate(1, I::Hard16v10, SixNoSurrender, Boundary::At).unwrap();
    let r = reference(positive);
    assert_eq!(r.integer_true_count, 0);
    assert_eq!(r.threshold, Threshold::PositiveRunning);
    assert_eq!(r.best, play(Stand));
    let negative = generate(1, I::Hard12v4, SixH17, Boundary::At).unwrap();
    let r = reference(negative);
    assert_eq!(r.integer_true_count, 0);
    assert_eq!(r.threshold, Threshold::NegativeRunning);
    assert_eq!(r.best, play(Hit));
    let q = generate(1, I::Surrender15vAceH17, SixH17, Boundary::At).unwrap();
    assert_eq!(q.context.count.running, -5);
    assert_eq!(reference(q).integer_true_count, -1);
}
#[test]
fn references_are_gated_help_is_sticky_and_first_answer_is_immutable() {
    let mut m = Model::default();
    let q = generate(1, I::Hard16v10, SixNoSurrender, Boundary::At).unwrap();
    assert!(m.begin(q));
    assert!(m.reference(1).is_none());
    assert_eq!(m.submit(1, play(Hit), false).status, Status::Recorded);
    let first = *m.newest(0).unwrap();
    assert!(!first.correct && !first.assisted);
    assert_eq!(m.request_help(1), Some(first.reference));
    let duplicate = m.submit(1, play(Stand), false);
    assert_eq!(duplicate.status, Status::Duplicate);
    assert_eq!(duplicate.attempt, Some(first));
    assert_eq!(m.stats().first.all.errors, 1);
    assert_eq!(m.stats().first.all.correct, 0);
    let q = generate(2, I::Hard12v4, SixH17, Boundary::At).unwrap();
    assert!(m.begin(q));
    let best = m.request_help(2).unwrap().best;
    let a = m.submit(2, best, false).attempt.unwrap();
    assert!(a.correct && a.assisted);
    assert_eq!(m.stats().first.all.assisted, 1);
    assert_eq!(m.stats().first.all.correct, 0);
    let q = generate(3, I::Hard16v10, SixNoSurrender, Boundary::At).unwrap();
    assert!(m.begin(q));
    assert_eq!(
        m.submit(3, Choice::Insurance(true), true).status,
        Status::Unavailable
    );
    let a = m.submit(3, play(Stand), false).attempt.unwrap();
    assert!(a.assisted);
    assert!(m.stats().valid());
}
#[test]
fn retries_preserve_original_snapshot_and_have_separate_scores() {
    let mut m = Model::default();
    let q = generate(1, I::Hard16v10, SixNoSurrender, Boundary::At).unwrap();
    assert!(m.begin(q));
    let a = m.submit(1, play(Hit), false).attempt.unwrap();
    let retry = m.replay_missed(1, 2).unwrap();
    assert_eq!(retry.context, a.question.context);
    assert_eq!(retry.origin(), Origin::Replay { original_token: 1 });
    assert!(m.begin(retry));
    assert_eq!(m.submit(2, play(Stand), false).status, Status::Recorded);
    assert_eq!(m.stats().first.all.errors, 1);
    assert_eq!(m.stats().first.all.correct, 0);
    assert_eq!(m.stats().replay.all.correct, 1);
    assert_eq!(m.history(0), Some(&a));
    assert!(m.replay_missed(2, 3).is_none());
}
#[test]
fn cancellation_consumes_tokens_and_bad_answers_do_not_score() {
    let mut m = Model::default();
    let q = generate(1, I::Insurance, SixH17, Boundary::At).unwrap();
    assert!(m.begin(q));
    assert!(!m.begin(q));
    assert_eq!(m.submit(1, play(Hit), false).status, Status::Unavailable);
    assert_eq!(m.stats(), Stats::default());
    assert!(m.cancel(1));
    assert!(!m.begin(q));
    assert!(m.reference(1).is_none());
    assert_eq!(
        m.submit(1, Choice::Insurance(true), false).status,
        Status::Duplicate
    );
    assert_eq!(
        m.submit(2, Choice::Insurance(true), false).status,
        Status::Unavailable
    );
    assert_eq!(m.stats(), Stats::default());
}
#[test]
fn funding_legality_and_surrender_priority_cannot_be_invented() {
    let q = generate(1, I::Hard10v10, SixH17, Boundary::At).unwrap();
    let mut c = q.context;
    c.legal_mask &= !Double.bit();
    c.funding.bankroll = 0;
    let restricted = Question::from_public(1, q.index, c).unwrap();
    assert_eq!(reference(restricted).best, play(Hit));
    c.legal_mask |= Double.bit();
    assert!(Question::from_public(1, q.index, c).is_none());
    let q = generate(1, I::Insurance, SixH17, Boundary::At).unwrap();
    let mut c = q.context;
    c.funding.bankroll = 249;
    let q = Question::from_public(1, q.index, c).unwrap();
    let mut m = Model::default();
    assert!(m.begin(q));
    assert_eq!(m.request_help(1).unwrap().best, Choice::Insurance(false));
    assert_eq!(
        m.submit(1, Choice::Insurance(true), false).status,
        Status::Unavailable
    );
    assert_eq!(
        m.submit(1, Choice::Insurance(false), false).status,
        Status::Recorded
    );
    let q = generate(1, I::Hard16v9, SixH17, Boundary::At).unwrap();
    let mut c = q.context;
    c.legal_mask |= Surrender.bit();
    assert!(Question::from_public(1, q.index, c).is_none());
    assert!(generate(1, I::Surrender16v9, SixNoSurrender, Boundary::At).is_none());
    assert!(generate(1, I::Soft19v6H17, SixS17, Boundary::At).is_none());
}
#[test]
fn invalid_or_physically_impossible_public_contexts_are_rejected() {
    let q = generate(1, I::Hard16v10, SixNoSurrender, Boundary::At).unwrap();
    let mut c = q.context;
    c.count.observed = 3;
    c.count.running = 3;
    assert!(c.count.valid(c.rules));
    assert!(Question::from_public(1, q.index, c).is_none());
    c = q.context;
    c.count.observed = 1;
    c.count.running = 0;
    assert!(Question::from_public(1, q.index, c).is_none());
    c = q.context;
    c.count.undealt = 0;
    assert!(Question::from_public(1, q.index, c).is_none());
    c = q.context;
    c.dealer_up = Card::from_id(0).unwrap();
    assert!(Question::from_public(1, q.index, c).is_none());
    c = q.context;
    c.funding = Funding {
        bankroll: -1,
        original_wager: 500,
    };
    assert!(Question::from_public(1, q.index, c).is_none());
    c = q.context;
    c.session = 0;
    assert!(Question::from_public(1, q.index, c).is_none());
    assert!(Question::from_public(0, q.index, q.context).is_none());
    // Pair splitting takes priority over an ordinary hard16 index.
    c = q.context;
    let mut h = Hand::default();
    h.add(Card::from_id(6).unwrap(), UNKNOWN_POSITION);
    h.add(Card::from_id(19).unwrap(), UNKNOWN_POSITION);
    h.bet = 500;
    c.hand = h;
    c.legal_mask |= Split.bit();
    assert!(Question::from_public(1, q.index, c).is_none());
}
#[test]
fn rollover_and_exact_64bit_restoration_do_not_regrade_old_tokens() {
    let mut m = Model::default();
    for token in 1..=300 {
        let q = generate(token, I::Insurance, SixH17, Boundary::Below).unwrap();
        assert!(m.begin(q));
        assert_eq!(
            m.submit(token, Choice::Insurance(false), false).status,
            Status::Recorded
        )
    }
    assert_eq!(m.history_count(), 128);
    assert_eq!(m.history(0).unwrap().question.token, 173);
    assert_eq!(m.stats().first.all.correct, 300);
    assert_eq!(
        m.submit(1, Choice::Insurance(true), false).status,
        Status::Duplicate
    );
    assert!(m.reference(1).is_none());
    let n = (1u64 << 53) + 123;
    let c = Counters {
        decisions: n,
        correct: n,
        errors: 0,
        assisted: 0,
    };
    let mut stats = Stats::default();
    stats.first.all = c;
    stats.first.indices[I::Insurance as usize] = c;
    let mut restored = Model::default();
    assert!(restored.restore(stats));
    assert!(!restored.restore(stats));
    let q = generate(1, I::Insurance, SixH17, Boundary::Below).unwrap();
    assert!(restored.begin(q));
    restored.submit(1, Choice::Insurance(false), false);
    assert_eq!(restored.stats().first.all.correct, n + 1);
    assert!(restored.stats().valid());
}
#[test]
fn independently_resolved_answers_match_existing_policy_across_public_masks_and_counts() {
    let mut m = Model::default();
    let mut token = 0;
    let mut checked = 0;
    for profile in Profile::ALL {
        for index in I::ALL {
            let Some(seed) = generate(1, index, profile, Boundary::At) else {
                continue;
            };
            for rc in -24..=24 {
                for mask in [1, 2, 3, 7, 11, 15, 19, 31] {
                    let mut c = seed.context;
                    c.count.running = rc;
                    if index != I::Insurance {
                        c.legal_mask &= mask
                    }
                    token += 1;
                    let Some(q) = Question::from_public(token, index, c) else {
                        continue;
                    };
                    assert!(m.begin(q));
                    let r = m.request_help(token).unwrap();
                    let expected = if index == I::Insurance {
                        Choice::Insurance(
                            deviations::insurance(
                                c.funding.bankroll >= c.funding.original_wager / 2,
                                c.rules,
                                c.count,
                            )
                            .unwrap(),
                        )
                    } else {
                        play(
                            deviations::recommend(
                                &c.hand,
                                c.dealer_up,
                                c.legal_mask,
                                c.rules,
                                c.count,
                            )
                            .play
                            .unwrap()
                            .best,
                        )
                    };
                    assert_eq!(
                        r.best, expected,
                        "{index:?} {profile:?} RC{rc} mask{}",
                        c.legal_mask
                    );
                    assert_eq!(m.submit(token, r.best, false).status, Status::Recorded);
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 40000);
    assert!(m.stats().valid());
    println!("PASS {checked} independent targeted references against core policy contexts");
}
#[test]
fn isolated_generator_is_reproducible_and_covers_all_supported_targets() {
    let mut a = Generator::new(7);
    let mut b = Generator::new(7);
    let mut seen = [false; I::COUNT];
    for token in 1..=5000 {
        let profile = if token % 2 == 0 { SixH17 } else { SixS17 };
        let q = a.next(token, profile).unwrap();
        assert_eq!(Some(q), b.next(token, profile));
        seen[q.index as usize] = true;
        assert!(reference(q).normalized_legal_mask == q.context.legal_mask);
    }
    assert!(seen.into_iter().all(|b| b));
}
#[test]
fn counter_capacity_and_corrupt_restore_are_bounded() {
    use stardust_blackjack::training::COUNTER_LIMIT;
    let counter = Counters {
        decisions: COUNTER_LIMIT,
        correct: COUNTER_LIMIT,
        errors: 0,
        assisted: 0,
    };
    let mut stats = Stats::default();
    stats.first.all = counter;
    stats.first.indices[I::Insurance as usize] = counter;
    let mut m = Model::default();
    assert!(m.restore(stats));
    let q = generate(1, I::Insurance, SixH17, Boundary::Below).unwrap();
    assert!(m.begin(q));
    assert_eq!(
        m.submit(1, Choice::Insurance(false), false).status,
        Status::Unavailable
    );
    assert_eq!(m.stats(), stats);
    assert!(m.cancel(1));
    let mut corrupt = stats;
    corrupt.first.indices.fill(counter);
    assert!(!corrupt.valid());
    assert!(!Model::default().restore(corrupt));
}
#[test]
fn every_generated_play_context_has_a_complete_physically_legal_action_menu() {
    for profile in stardust_blackjack::rules::Profile::ALL {
        for index in I::ALL {
            let Some(q) = generate(1, index, profile, Boundary::At) else {
                continue;
            };
            if index == I::Insurance {
                assert_eq!(q.context.legal_mask, 0);
                continue;
            }
            let c = q.context;
            let h = c.hand;
            let mut legal = Stand.bit();
            if h.total() < 21 && !h.split_aces {
                legal |= Hit.bit()
            }
            if h.count() == 2
                && !h.split_aces
                && (!h.split || c.rules.double_after_split)
                && c.funding.bankroll >= h.bet
            {
                legal |= Double.bit()
            }
            if h.count() == 2
                && !h.split_aces
                && h.cards()[0].value() == h.cards()[1].value()
                && c.funding.bankroll >= h.bet
            {
                legal |= Split.bit()
            }
            if h.count() == 2 && !h.split && c.rules.late_surrender {
                legal |= Surrender.bit()
            }
            assert_eq!(c.legal_mask, legal, "{profile:?} {index:?}");
            if matches!(
                index,
                I::Hard16v9 | I::Hard16v10 | I::Hard16vAceH17 | I::Hard15v10 | I::Hard15vAceH17
            ) && c.rules.late_surrender
            {
                assert_eq!(h.count(), 3);
            }
        }
    }
}

#[test]
fn synthetic_decisions_reserve_unknown_burn_and_hole_and_keep_visible_faces_feasible() {
    for profile in stardust_blackjack::rules::Profile::ALL {
        for index in I::ALL {
            for side in [Boundary::Below, Boundary::At, Boundary::Above] {
                let Some(q) = generate(1, index, profile, side) else {
                    continue;
                };
                let c = q.context;
                let dealt = c.rules.total_cards() - c.count.undealt as usize;
                assert_eq!(c.count.observed as usize, dealt - 2);
                assert!(c.count.observed as usize > c.hand.count());
                assert!(Question::from_public(1, index, c).is_some());
                for unknown in [0, 1] {
                    let mut bad = c;
                    bad.count.observed = (dealt - unknown) as u16;
                    assert!(bad.count.valid(bad.rules));
                    assert!(
                        Question::from_public(1, index, bad).is_none(),
                        "{profile:?} {index:?} unknown{unknown}"
                    );
                }
            }
        }
    }
    // These three visible faces are10+6 versus10: their exact tag sum is-1.
    // Declaring only those faces observed but RC+3 is impossible even though
    // generic Count bounds and the two-unknown-card budget both permit it.
    let q = generate(1, I::Hard16v10, SixNoSurrender, Boundary::At).unwrap();
    let mut c = q.context;
    c.count.observed = 3;
    c.count.running = 3;
    assert!(c.count.valid(c.rules));
    assert!(Question::from_public(1, q.index, c).is_none());
    c.count.running = -1;
    assert!(Question::from_public(1, q.index, c).is_some());
}
