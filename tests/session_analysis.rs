use stardust_blackjack::session_analysis::{Model, Round, decision_cost};
fn add(m: &mut Model, id: u64, net: i64, cumulative: i64) -> bool {
    m.record(Round {
        id,
        base_wager: 500,
        stake: 500,
        returned: 500 + net,
        bankroll: 100_000 + cumulative,
        deposits: 100_000,
        cashouts: 0,
    })
}
#[test]
fn independent_known_sample_and_drawdown() {
    let mut m = Model::default();
    let mut sum = 0;
    for (i, net) in [500, -500, -500, 0, 750].into_iter().enumerate() {
        sum += net;
        assert!(add(&mut m, i as u64 + 1, net, sum));
    }
    let s = m.summary();
    assert_eq!((s.rounds, s.wins, s.losses, s.pushes), (5, 2, 2, 1));
    assert_eq!(
        (s.net, s.total_staked, s.max_drawdown, s.peak_net),
        (250, 2500, 1000, 500)
    );
    assert_eq!(
        (s.longest_loss_streak, s.best_round, s.worst_round),
        (2, 750, -500)
    );
    assert_eq!(s.mean_net_cents, 50.0);
    // Independent hand calculation: deviations450,-550,-550,-50,700;
    // sum squared=1,300,000; sample variance=325,000.
    assert!((s.sample_stddev_cents.unwrap().powi(2) - 325_000.0).abs() < 1e-7);
    assert!(s.normal_reference_halfwidth_cents.is_none());
}
#[test]
fn rejects_duplicate_changed_retry_and_bad_ledger_without_mutation() {
    let mut m = Model::default();
    assert!(add(&mut m, 1, 500, 500));
    let s = m.summary();
    assert!(!add(&mut m, 1, -500, 0));
    assert!(!add(&mut m, 2, -500, 999));
    assert_eq!(m.summary(), s);
    assert!(add(&mut m, 2, -500, 0));
}
#[test]
fn rollover_retains_full_totals_and_bound_checks() {
    let mut m = Model::default();
    for id in 1..=1000 {
        assert!(add(&mut m, id, 0, 0));
    }
    assert_eq!(m.summary().rounds, 1000);
    assert_eq!(m.newest(127).unwrap().id, 873);
    assert!(m.newest(128).is_none());
    assert!(m.newest(usize::MAX).is_none());
    assert_eq!(m.summary().sample_stddev_cents, Some(0.0));
}
#[test]
fn large_near_equal_results_do_not_cancel_variance() {
    let mut m = Model::default();
    let mut sum = 0;
    for (i, n) in [999_999_999_499, 999_999_999_500].into_iter().enumerate() {
        sum += n;
        assert!(m.record(Round {
            id: i as u64 + 1,
            base_wager: 500,
            stake: 500,
            returned: n + 500,
            bankroll: sum + 500,
            deposits: 500,
            cashouts: 0
        }));
    }
    assert!((m.summary().sample_stddev_cents.unwrap().powi(2) - 0.5).abs() < 1e-15);
}
#[test]
fn observed_equity_ignores_refills_and_colorups() {
    let mut m = Model::new(-200);
    assert!(m.record(Round {
        id: 1,
        base_wager: 500,
        stake: 500,
        returned: 1000,
        bankroll: 100_300,
        deposits: 100_000,
        cashouts: 0
    }));
    assert!(m.record(Round {
        id: 2,
        base_wager: 500,
        stake: 500,
        returned: 0,
        bankroll: 0,
        deposits: 200_000,
        cashouts: 199_800
    }));
    assert_eq!(m.summary().net, 0);
    assert_eq!(m.summary().max_drawdown, 500);
}
#[test]
fn empty_and_single_round_have_no_fake_uncertainty() {
    let mut m = Model::default();
    assert!(m.summary().sample_stddev_cents.is_none());
    assert!(add(&mut m, 1, 0, 0));
    assert!(m.summary().normal_reference_halfwidth_cents.is_none());
}
#[test]
fn frozen_decision_cost_is_model_value_not_result() {
    assert_eq!(
        decision_cost(0.1, -0.2, 500, false).unwrap().cents,
        150.00000000000003
    );
    assert!(decision_cost(-0.2, 0.1, 500, false).is_none());
    assert!(decision_cost(f64::NAN, 0.0, 500, false).is_none());
    assert!(decision_cost(1e308, 0.0, 500, false).is_none());
    assert!(decision_cost(0.0, 0.0, 500, true).unwrap().assisted);
}

#[test]
fn independent_pairwise_variance_oracle() {
    // Sample variance also equals sum over unordered squared differences / n(n-1).
    // This independent O(n²) calculation does not duplicate moment subtraction.
    let mut rng = 90210_u64;
    for sample in 0..1000 {
        let mut m = Model::default();
        let mut values = [0_i64; 32];
        let mut cumulative = 0;
        let len = 2 + sample % 31;
        for (i, value) in values[..len].iter_mut().enumerate() {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            *value = ((rng >> 32) % 1501) as i64 - 500;
            cumulative += *value;
            assert!(add(&mut m, i as u64 + 1, *value, cumulative));
        }
        let mut pairs = 0_i128;
        for i in 0..len {
            for j in i + 1..len {
                let d = values[i] as i128 - values[j] as i128;
                pairs += d * d;
            }
        }
        let expected = pairs as f64 / (len * (len - 1)) as f64;
        assert!((m.summary().sample_stddev_cents.unwrap().powi(2) - expected).abs() < 1e-7);
        if len >= 30 {
            let band = m.summary().normal_reference_halfwidth_cents.unwrap();
            assert!((band.powi(2) - 3.8416 * expected / len as f64).abs() < 1e-7);
        } else {
            assert!(m.summary().normal_reference_halfwidth_cents.is_none());
        }
    }
}
