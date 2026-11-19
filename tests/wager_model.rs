const _: stardust_blackjack::wager_practice::Model =
    stardust_blackjack::wager_practice::Model::EMPTY;
use stardust_blackjack::wager_practice::*;
use stardust_blackjack::{
    deviations::Count,
    rules::{Profile, Rules},
    training::{COUNTER_LIMIT, Counters},
};
fn snap(rc: i16, undealt: u16, bankroll: i64, checkpoint: u64) -> Snapshot {
    let r = Rules::default();
    Snapshot {
        context: Context {
            session: 1,
            checkpoint,
            shoe: 1,
        },
        rules: r,
        count: Count {
            running: rc,
            undealt,
            decks: r.decks,
            observed: r.total_cards() as u16 - undealt,
            shoe: 1,
        },
        bankroll,
        predeal: true,
        estimated_raw_tc: None,
    }
}
fn answer(s: Snapshot, p: Policy, choice: Choice) -> Attempt {
    let mut m = Model::default();
    assert!(m.begin(1, s, p));
    assert!(m.reference(1).is_none());
    let g = m.submit(1, choice, false);
    assert_eq!(g.status, Status::Recorded);
    g.attempt.unwrap()
}
#[test]
fn published_example_boundaries_and_signed_truncation() {
    for (rc, left, index, units, target) in [
        (-4, 104, -2, 1, 500),
        (-1, 104, 0, 1, 500),
        (0, 104, 0, 1, 500),
        (1, 104, 0, 1, 500),
        (3, 104, 1, 2, 1000),
        (4, 104, 2, 4, 2000),
        (6, 104, 3, 8, 4000),
        (8, 104, 4, 12, 6000),
        (10, 104, 5, 12, 6000),
        (1, 52, 1, 2, 1000),
        (1, 53, 0, 1, 500),
    ] {
        let a = answer(
            snap(rc, left, 100000, 1),
            Policy::default(),
            Choice::Wager(target),
        );
        let r = a.reference;
        assert!(a.correct && !a.assisted);
        assert_eq!(r.label, POLICY_LABEL);
        assert_eq!(r.canonical.index, index);
        assert_eq!(r.canonical.units, units);
        assert_eq!(r.canonical.unit_size, 500);
        assert_eq!(r.canonical.raw_wager, 500 * units as i64);
        assert_eq!(r.table.step, 500);
        assert_eq!(r.canonical.band, Band::from_index(index));
    }
}
#[test]
fn independent_exhaustive_rational_band_and_cap_oracle() {
    let mut cases = 0;
    for left in 1..=312u16 {
        let maxrc = (312 - left).min(120) as i16;
        for rc in -maxrc..=maxrc {
            for bank in [
                0,
                1,
                499,
                500,
                501,
                999,
                1250,
                2250,
                6000,
                100000,
                i64::MAX / 4,
            ] {
                let mut s = snap(rc, left, bank, 1);
                s.rules.maximum = 2500;
                let mut m = Model::default();
                assert!(m.begin(1, s, Policy::default()));
                let r = m.request_help(1).unwrap();
                // Independent rational interval search: no TC division, and
                // no floating rounding at exact negative integer boundaries.
                let magnitude = rc.unsigned_abs() as u32 * 52;
                let (mut lo, mut hi) = (0u32, 6241u32);
                while lo + 1 < hi {
                    let mid = (lo + hi) / 2;
                    if mid * left as u32 <= magnitude {
                        lo = mid
                    } else {
                        hi = mid
                    }
                }
                let index = if rc < 0 { -(lo as i16) } else { lo as i16 };
                assert_eq!(r.canonical.index, index, "RC{rc}/left{left}");
                let unit = [1, 2, 4, 8, 12][match index {
                    ..=0 => 0,
                    1 => 1,
                    2 => 2,
                    3 => 3,
                    _ => 4,
                }];
                let expected = if bank < 500 {
                    Choice::NoBetRefill
                } else {
                    let raw = 500 * unit;
                    let allowed: Vec<i64> = (500..=2500)
                        .step_by(500)
                        .filter(|&n| n <= bank && n <= raw)
                        .collect();
                    Choice::Wager(*allowed.last().unwrap())
                };
                assert_eq!(r.canonical.choice, expected);
                assert!(m.submit(1, expected, false).attempt.unwrap().correct);
                cases += 1;
            }
        }
    }
    assert!(cases > 600000);
}
#[test]
fn caps_steps_and_refill_are_distinct() {
    for (bank, max, target, table_cap, bank_cap, step) in [
        (100000, 50000, 6000, false, false, false),
        (100000, 2500, 2500, true, false, false),
        (2250, 2500, 2000, true, true, true),
        (2250, 50000, 2000, false, true, true),
        (6000, 6000, 6000, false, false, false),
        (5999, 6000, 5500, false, true, true),
    ] {
        let mut s = snap(8, 104, bank, 1);
        s.rules.maximum = max;
        let a = answer(s, Policy::default(), Choice::Wager(target));
        assert!(a.correct);
        assert_eq!(
            a.reference.canonical.adjustments,
            Adjustments {
                table_cap,
                bankroll_cap: bank_cap,
                step_rounding: step
            }
        );
    }
    for bank in [0, 1, 499] {
        let a = answer(
            snap(8, 104, bank, 1),
            Policy::default(),
            Choice::NoBetRefill,
        );
        assert!(a.correct);
        assert_eq!(a.category, Category::Refill);
        assert_eq!(a.reference.canonical.choice, Choice::NoBetRefill)
    }
    let a = answer(snap(0, 104, 500, 1), Policy::default(), Choice::NoBetRefill);
    assert!(!a.correct);
    assert_eq!(a.category, Category::NonPositive);
}
#[test]
fn initial_reference_is_gated_help_is_sticky_and_first_attempt_immutable() {
    let mut m = Model::default();
    assert!(m.begin(1, snap(4, 104, 100000, 1), Policy::default()));
    assert!(m.reference(1).is_none());
    assert!(m.request_help(2).is_none());
    let a = m.submit(1, Choice::Wager(500), false).attempt.unwrap();
    assert!(!a.correct && !a.assisted);
    let retry = m.submit(1, Choice::Wager(2000), true);
    assert_eq!(retry.status, Status::Duplicate);
    assert_eq!(retry.attempt, Some(a));
    assert_eq!(
        m.stats().all,
        Counters {
            decisions: 1,
            correct: 0,
            errors: 1,
            assisted: 0
        }
    );
    assert!(m.begin(2, snap(4, 104, 100000, 1), Policy::default()));
    assert!(m.reference(2).is_none());
    let helped = m.submit(2, Choice::Wager(2000), false).attempt.unwrap();
    assert!(helped.correct && helped.assisted);
    assert!(m.begin(3, snap(4, 104, 100000, 2), Policy::default()));
    assert!(m.request_help(3).is_some());
    assert!(m.reference(3).is_some());
    let a = m.submit(3, Choice::Wager(2000), false).attempt.unwrap();
    assert!(a.correct && a.assisted);
    assert_eq!(
        m.stats().all,
        Counters {
            decisions: 3,
            correct: 0,
            errors: 1,
            assisted: 2
        }
    );
}
#[test]
fn invalid_wager_retry_never_consumes_first_answer_or_clears_assistance() {
    let mut m = Model::default();
    assert!(m.begin(1, snap(0, 104, 1250, 1), Policy::default()));
    for n in [i64::MIN, -500, 0, 499, 501, 750, 1500, 50000, i64::MAX] {
        let g = m.submit(1, Choice::Wager(n), n == 750);
        assert_eq!(g.status, Status::Unavailable);
        assert!(g.attempt.is_none());
        assert!(m.awaiting_answer());
        assert!(m.reference(1).is_none());
        assert_eq!(m.history_count(), 0)
    }
    let a = m.submit(1, Choice::Wager(500), false).attempt.unwrap();
    assert!(a.assisted && a.correct);
    assert_eq!(m.stats().all.assisted, 1);
}
#[test]
fn frozen_rules_count_bankroll_custom_policy_and_estimate_are_evidence() {
    let mut s = snap(3, 104, 100000, 1);
    s.estimated_raw_tc = Some(2.9);
    let mut p = Policy {
        units: [3, 5, 7, 9, 11],
        ..Policy::default()
    };
    let mut m = Model::default();
    assert!(m.begin(1, s, p));
    s.rules.minimum = 1000;
    s.count.running = 0;
    s.bankroll = 0;
    p.units = [1; 5];
    let a = m.submit(1, Choice::Wager(2500), false).attempt.unwrap();
    assert!(a.correct);
    let r = a.reference;
    assert_eq!(r.snapshot.rules.minimum, 500);
    assert_eq!(r.snapshot.count.running, 3);
    assert_eq!(r.snapshot.bankroll, 100000);
    assert_eq!(r.policy.units, [3, 5, 7, 9, 11]);
    assert_eq!(r.canonical.index, 1);
    assert_eq!(r.canonical.choice, Choice::Wager(2500));
    assert_eq!(r.estimated.unwrap().index, 2);
    assert_eq!(r.estimated.unwrap().choice, Choice::Wager(3500));
    assert!(!r.policy.is_standard());
    assert_eq!(s.bankroll, 0);
    assert_eq!(p.units, [1; 5]);
}
#[test]
fn physical_target_always_grades_even_if_estimate_differs() {
    let mut s = snap(3, 104, 100000, 1);
    s.estimated_raw_tc = Some(2.9);
    let a = answer(s, Policy::default(), Choice::Wager(2000));
    assert!(!a.correct);
    assert_eq!(a.reference.canonical.choice, Choice::Wager(1000));
    assert_eq!(a.reference.estimated.unwrap().choice, Choice::Wager(2000));
    s.estimated_raw_tc = Some(-0.5);
    let a = answer(s, Policy::default(), Choice::Wager(1000));
    assert!(a.correct);
    assert_eq!(a.reference.estimated.unwrap().index, 0);
}
#[test]
fn context_identity_rule_binding_and_new_sessions_are_checked() {
    let mut m = Model::default();
    let s = snap(0, 104, 100000, 1);
    assert!(m.begin(1, s, Policy::default()));
    assert!(!m.begin(2, s, Policy::default()));
    assert!(m.cancel(1));
    for mut wrong in [s; 1] {
        wrong.context.session = 0;
        assert!(!m.begin(2, wrong, Policy::default()));
    }
    let mut changed = s;
    changed.bankroll += 500;
    assert!(!m.begin(2, changed, Policy::default()));
    changed = s;
    changed.count.running = 1;
    assert!(!m.begin(2, changed, Policy::default()));
    changed = s;
    changed.rules = Rules::for_profile(Profile::EightH17);
    assert!(!changed.valid());
    assert!(!m.begin(2, changed, Policy::default()));
    changed = s;
    changed.rules.hit_soft_17 = false;
    assert!(!changed.valid());
    changed = s;
    changed.context.checkpoint = 2;
    assert!(m.begin(2, changed, Policy::default()));
    assert!(m.synchronize(Context {
        checkpoint: 3,
        ..changed.context
    }));
    assert!(!m.begin(2, changed, Policy::default()));
    changed.context.session = 2;
    changed.context.checkpoint = 1;
    assert!(m.begin(3, changed, Policy::default()));
    assert!(m.cancel(3));
    assert!(!m.begin(4, s, Policy::default()));
}
#[test]
fn malformed_snapshots_policies_tables_are_rejected_without_token_consumption() {
    for which in 0..17 {
        let mut s = snap(0, 104, 100000, 1);
        match which {
            0 => s.predeal = false,
            1 => s.count.undealt = 0,
            2 => s.count.undealt = 313,
            3 => s.count.decks = 7,
            4 => s.count.observed = 209,
            5 => s.count.running = 121,
            6 => s.count.running = -121,
            7 => s.count.shoe = 0,
            8 => s.context.shoe = 2,
            9 => s.context.session = 0,
            10 => s.context.checkpoint = 0,
            11 => s.bankroll = -1,
            12 => s.bankroll = MAX_BANKROLL + 1,
            13 => s.estimated_raw_tc = Some(f64::NAN),
            14 => s.estimated_raw_tc = Some(f64::INFINITY),
            15 => s.estimated_raw_tc = Some(-21633.0),
            _ => s.rules.minimum = 501,
        };
        let mut m = Model::default();
        assert!(!m.begin(1, s, Policy::default()), "case {which}");
        assert!(m.begin(1, snap(0, 104, 100000, 1), Policy::default()));
    }
    for units in [[0; 5], [MAX_UNITS + 1; 5]] {
        let mut m = Model::default();
        assert!(!m.begin(
            1,
            snap(0, 104, 100000, 1),
            Policy {
                units,
                ..Policy::default()
            }
        ));
    }
    assert!(
        !Table {
            minimum: 500,
            maximum: 1000000,
            step: 100
        }
        .valid()
    );
    assert!(
        !Table {
            minimum: 1000,
            maximum: 500,
            step: 500
        }
        .valid()
    );
}
#[test]
fn large_bankroll_and_maximum_product_do_not_overflow() {
    let mut s = snap(120, 1, MAX_BANKROLL, 1);
    s.rules.minimum = 1_000_000;
    s.rules.maximum = 1_000_000;
    let p = Policy {
        units: [MAX_UNITS; 5],
        ..Policy::default()
    };
    let a = answer(s, p, Choice::Wager(1_000_000));
    assert!(a.correct);
    assert_eq!(a.reference.canonical.raw_wager, 1_000_000_000_000);
    assert!(a.reference.canonical.adjustments.table_cap);
    assert!(!a.reference.canonical.adjustments.bankroll_cap);
    let a = answer(
        snap(8, 104, 1_000_000_001, 1),
        Policy::default(),
        Choice::Wager(6000),
    );
    assert!(a.correct);
}
#[test]
fn metrics_categories_help_and_late_history_view() {
    let mut m = Model::default();
    let mut token = 0;
    for (i, rc) in [0, 2, 4, 6, 8].into_iter().enumerate() {
        for style in 0..3 {
            token += 1;
            let s = snap(rc, 104, 100000, token);
            assert!(m.begin(token, s, Policy::default()));
            let target = [500, 1000, 2000, 4000, 6000][i];
            if style == 2 {
                m.request_help(token);
            }
            let wrong = if target == 500 { 1000 } else { 500 };
            m.submit(
                token,
                Choice::Wager(if style == 0 { target } else { wrong }),
                false,
            );
        }
        assert_eq!(
            m.stats().categories[i],
            Counters {
                decisions: 3,
                correct: 1,
                errors: 1,
                assisted: 1
            }
        );
    }
    for helped in [false, true] {
        token += 1;
        assert!(m.begin(token, snap(0, 104, 0, token), Policy::default()));
        m.submit(token, Choice::NoBetRefill, helped);
    }
    assert_eq!(
        m.stats().all,
        Counters {
            decisions: 17,
            correct: 6,
            errors: 5,
            assisted: 6
        }
    );
    assert_eq!(m.stats().accuracy(), Some(6.0 / 11.0));
    let s = snap(0, 104, 0, token);
    assert!(m.begin(token + 1, s, Policy::default()));
    m.request_help(token);
    let a = m
        .submit(token + 1, Choice::NoBetRefill, false)
        .attempt
        .unwrap();
    assert!(a.assisted);
    assert!(m.stats().valid());
}
#[test]
fn ring_eviction_cancellation_and_reset_preserve_consumed_tokens() {
    let mut m = Model::default();
    assert!(m.begin(1, snap(0, 104, 100000, 1), Policy::default()));
    assert!(m.cancel(1));
    assert_eq!(
        m.submit(1, Choice::Wager(500), false).status,
        Status::Duplicate
    );
    assert!(!m.begin(1, snap(0, 104, 100000, 1), Policy::default()));
    for token in 2..=146 {
        assert!(m.begin(token, snap(0, 104, 100000, token), Policy::default()));
        m.submit(
            token,
            Choice::Wager(if token == 2 { 1000 } else { 500 }),
            false,
        );
    }
    assert_eq!(m.history_count(), 128);
    assert_eq!(m.history(0).unwrap().question.token, 19);
    assert_eq!(m.newest(0).unwrap().question.token, 146);
    assert!(m.history(128).is_none());
    assert!(m.newest(usize::MAX).is_none());
    assert_eq!(
        m.stats().all,
        Counters {
            decisions: 145,
            correct: 144,
            errors: 1,
            assisted: 0
        }
    );
    assert_eq!(m.submit(2, Choice::Wager(500), true).attempt, None);
    assert!(m.request_help(2).is_none());
    assert!(m.reset_stats());
    assert!(!m.begin(146, snap(0, 104, 100000, 146), Policy::default()));
    assert!(m.begin(u64::MAX, snap(0, 104, 100000, 147), Policy::default()));
    assert!(!m.reset_stats());
    m.submit(u64::MAX, Choice::Wager(500), false);
    assert!(m.reset_stats());
    assert!(!m.begin(u64::MAX, snap(0, 104, 100000, 148), Policy::default()));
}
#[test]
fn saved_stats_exact_u64_import_is_validated_and_boot_only() {
    let n = (1u64 << 53) + 123;
    let c = Counters {
        decisions: n,
        correct: n - 2,
        errors: 1,
        assisted: 1,
    };
    let mut stats = Stats {
        all: c,
        ..Stats::default()
    };
    stats.categories[0] = c;
    assert!(stats.valid());
    let packet = saved::encode(stats).unwrap();
    assert_eq!(saved::decode(&packet), Some(stats));
    let mut m = Model::default();
    assert!(m.restore(stats));
    assert_eq!(m.stats(), stats);
    assert!(!m.restore(stats));
    let mut active = Model::default();
    assert!(active.begin(1, snap(0, 104, 100000, 1), Policy::default()));
    assert!(!active.restore(stats));
    assert!(active.cancel(1));
    assert!(!active.restore(stats));
    let mut bad = stats;
    bad.categories[1] = c;
    assert!(!bad.valid());
    assert!(saved::encode(bad).is_none());
    let mut fresh = Model::default();
    assert!(!fresh.restore(bad));
    assert!(fresh.restore(stats));
    for i in 0..saved::SIZE {
        let mut damaged = packet;
        damaged[i] ^= 1;
        assert!(saved::decode(&damaged).is_none(), "byte {i}");
    }
    assert!(saved::decode(&packet[..saved::SIZE - 1]).is_none());
}
#[test]
fn malformed_stats_and_counter_limit_never_wrap() {
    let mut s = Stats::default();
    let c = Counters {
        decisions: COUNTER_LIMIT - 1,
        correct: COUNTER_LIMIT - 1,
        errors: 0,
        assisted: 0,
    };
    s.all = c;
    s.categories[0] = c;
    let mut m = Model::default();
    assert!(m.restore(s));
    assert!(m.begin(1, snap(0, 104, 100000, 1), Policy::default()));
    assert!(
        m.submit(1, Choice::Wager(500), false)
            .attempt
            .unwrap()
            .correct
    );
    assert_eq!(m.stats().all.decisions, COUNTER_LIMIT);
    assert!(m.stats().valid());
    assert!(!m.begin(2, snap(0, 104, 100000, 2), Policy::default()));
    for c in [
        Counters {
            decisions: u64::MAX,
            correct: u64::MAX,
            errors: 0,
            assisted: 0,
        },
        Counters {
            decisions: 1,
            correct: 1,
            errors: 1,
            assisted: 0,
        },
        Counters {
            decisions: 0,
            correct: 0,
            errors: 0,
            assisted: 1,
        },
    ] {
        let s = Stats {
            all: c,
            categories: [c; 6],
        };
        assert!(!s.valid());
        assert!(!Model::default().restore(s));
    }
}

#[test]
fn every_rule_profile_is_bound_to_its_own_decks_limits_and_predeal_context() {
    for profile in Profile::ALL {
        for (minimum, maximum) in [(500, 50000), (1500, 100000), (50000, 1000000)] {
            let mut s = snap(4, 104, 1_000_000, 1);
            s.rules = Rules::for_profile(profile);
            s.rules.minimum = minimum;
            s.rules.maximum = maximum;
            s.count.decks = s.rules.decks;
            s.count.observed = s.rules.total_cards() as u16 - s.count.undealt;
            let a = answer(
                s,
                Policy::default(),
                Choice::Wager((minimum * 4).min(maximum)),
            );
            assert!(a.correct);
            assert_eq!(a.reference.snapshot.rules, s.rules);
            assert_eq!(a.reference.snapshot.count.decks, s.rules.decks);
        }
    }
}
#[test]
fn new_session_clears_checkpoint_help_without_changing_first_attempts() {
    let mut m = Model::default();
    let s = snap(4, 104, 100000, 1);
    assert!(m.begin(1, s, Policy::default()));
    m.request_help(1);
    let a = m.submit(1, Choice::Wager(2000), false).attempt.unwrap();
    assert!(a.assisted);
    let mut next = s;
    next.context.session = 2;
    assert!(m.begin(2, next, Policy::default()));
    m.request_help(1);
    assert!(m.reference(2).is_none());
    let b = m.submit(2, Choice::Wager(2000), false).attempt.unwrap();
    assert!(!b.assisted);
    assert_eq!(m.history(0), Some(&a));
}
#[test]
fn schema_validates_counters_after_valid_checksum_and_empty_accuracy_is_explicit() {
    assert_eq!(Stats::default().accuracy(), None);
    let mut packet = saved::encode(Stats::default()).unwrap();
    packet[8..16].copy_from_slice(&1u64.to_le_bytes());
    let crc = stardust_blackjack::persistence::crc32(&packet[..saved::SIZE - 4]);
    packet[saved::SIZE - 4..].copy_from_slice(&crc.to_le_bytes());
    assert!(saved::decode(&packet).is_none());
}

#[test]
fn viewing_old_history_cannot_erase_current_checkpoint_help_after_cancellation() {
    let mut m = Model::default();
    assert!(m.begin(1, snap(0, 104, 100000, 1), Policy::default()));
    m.submit(1, Choice::Wager(500), false);
    let current = snap(4, 104, 100000, 2);
    assert!(m.begin(2, current, Policy::default()));
    m.request_help(2);
    assert!(m.cancel(2));
    m.request_help(1);
    assert!(m.begin(3, current, Policy::default()));
    let a = m.submit(3, Choice::Wager(2000), false).attempt.unwrap();
    assert!(a.assisted);
    let invalid = Stats {
        all: Counters {
            decisions: 0,
            correct: u64::MAX,
            errors: u64::MAX,
            assisted: 0,
        },
        ..Stats::default()
    };
    assert_eq!(invalid.accuracy(), None);
}
