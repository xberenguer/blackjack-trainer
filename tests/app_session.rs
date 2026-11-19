use stardust_blackjack::{
    app::App,
    controls::{Event, Key},
    engine::{Action, Phase},
    session_metrics::Skip,
    training::{Mode, Policy},
};
const B: Event = Event::Tap(Key::Blue);
const G: Event = Event::Tap(Key::Grey);
fn select(a: &mut App, n: usize) {
    for _ in 0..n {
        a.event(G)
    }
    a.event(B)
}
fn settle(a: &mut App, now: &mut u32) {
    let old = a.game.rounds;
    for _ in 0..2000 {
        if a.view_name() == "RoundReview" {
            a.event(B)
        }
        if a.view_name() == "Cut"
            || a.view_name() == "Burn"
            || (a.view_name() == "Table"
                && matches!(a.game.phase, Phase::Ready | Phase::Result)
                && !a.is_busy()
                && a.game.rounds == old)
        {
            a.event(B)
        }
        if a.game.rounds == old
            && !a.is_busy()
            && matches!(a.game.phase, Phase::Player | Phase::Insurance)
        {
            a.event(B)
        }
        *now = now.wrapping_add(200);
        a.tick(*now);
        a.render();
        if a.game.rounds == old + 1 && !a.is_busy() {
            assert_eq!(a.session_analysis.summary().rounds, a.game.rounds);
            return;
        }
    }
    panic!("round did not settle: {} {:?}", a.view_name(), a.game.phase)
}
fn analysis_menu(a: &mut App) {
    a.event(Event::Hold(Key::Grey));
    let extras = if a.is_busy() {
        0
    } else {
        [Action::Double, Action::Split, Action::Surrender]
            .iter()
            .filter(|&&x| a.game.legal(x))
            .count()
    };
    select(a, extras + 7);
    assert_eq!(a.view_name(), "Session");
    select(a, 3);
    assert_eq!(a.view_name(), "AnalysisMenu")
}
#[test]
fn captures_each_settled_round_once_and_funding_does_not_become_profit() {
    let mut a = App::new(17);
    assert!(a.game.buy_in(50000));
    let mut now = 0;
    let mut net = 0i64;
    let mut stake = 0i64;
    for _ in 0..40 {
        settle(&mut a, &mut now);
        net += a.game.round_net;
        stake += a.game.round_stake;
        let last = a.session_analysis.newest(0).unwrap();
        assert_eq!(last.net(), a.game.round_net);
        assert_eq!(last.base_wager, 500);
        assert_eq!(last.returned, a.game.round_return);
        assert_eq!(last.ledger_net(), a.game.net() as i128);
        let before = a.session_analysis.summary();
        for _ in 0..3 {
            now += 100;
            a.tick(now);
            a.render()
        }
        assert_eq!(a.session_analysis.summary(), before);
        if a.game.rounds == 20 {
            assert!(a.game.buy_in(100000));
        }
    }
    let summary = a.session_analysis.summary();
    assert_eq!(summary.rounds, 40);
    assert_eq!(summary.net, net as i128);
    assert_eq!(summary.total_staked, stake as i128);
    assert_eq!(summary.net, a.game.net() as i128);
    assert!(summary.normal_reference_halfwidth_cents.is_some());
    assert_eq!(a.session_metrics.card_stats().all.decisions, 0);
}
#[test]
fn initial_natural_is_recorded_after_all_initial_faces_have_appeared() {
    for seed in 1..300 {
        let mut a = App::new(seed);
        a.event(B);
        a.event(B);
        if a.game.phase != Phase::Result {
            continue;
        }
        assert_eq!(a.session_analysis.summary().rounds, 0);
        for now in [200, 400, 600, 800] {
            a.tick(now);
            a.render();
            assert_eq!(a.session_analysis.summary().rounds, 0)
        }
        a.tick(1000);
        assert_eq!(a.session_analysis.summary().rounds, 1);
        let expected = a.game.round_net as i128;
        a.tick(5000);
        a.render();
        assert_eq!(a.session_analysis.summary().net, expected);
        assert_eq!(a.session_analysis.summary().rounds, 1);
        return;
    }
    panic!("no natural outcome");
}
#[test]
fn coach_first_answer_is_retained_before_correction_without_duplicate_cost() {
    for seed in 1..200 {
        let mut a = App::new(seed);
        a.training.mode = Mode::Coach;
        a.training.pause_on_mistake = true;
        a.event(B);
        a.event(B);
        for n in 1..=5 {
            a.tick(n * 200)
        }
        if a.game.phase != Phase::Player {
            continue;
        }
        let position = a.game.position;
        a.event(B);
        if a.view_name() != "CoachCheck" {
            continue;
        }
        assert_eq!(a.game.position, position);
        let first = a.session_metrics.newest(0).unwrap();
        assert!(!first.correct);
        assert_eq!(a.session_metrics.card_stats().all.errors, 1);
        let costs = a.session_metrics.costs();
        a.event(G);
        a.event(B);
        assert_eq!(a.session_metrics.newest(0), Some(first));
        assert_eq!(a.session_metrics.card_stats().all.decisions, 1);
        assert_eq!(a.session_metrics.costs(), costs);
        return;
    }
    panic!("no wrong stand");
}
#[test]
fn hilo_grades_are_kept_and_fixed_odds_costs_explicitly_skip_them() {
    let mut a = App::new(7);
    a.training.mode = Mode::Test;
    a.policy = Policy::HiLo;
    let mut now = 0;
    settle(&mut a, &mut now);
    assert!(a.session_metrics.card_stats().all.decisions > 0);
    assert_eq!(a.session_metrics.card_stats(), a.training.stats());
    assert_eq!(
        a.session_metrics.costs().skipped_hilo,
        a.session_metrics.card_stats().all.decisions
    );
    assert_eq!(a.session_metrics.costs().unassisted.decisions, 0);
    assert_eq!(
        a.session_metrics.newest(0).unwrap().skipped,
        Some(Skip::HiLoPolicy)
    );
}
#[test]
fn completed_rounds_and_metrics_reset_only_on_new_session() {
    let mut a = App::new(7);
    a.training.mode = Mode::Test;
    let mut now = 0;
    settle(&mut a, &mut now);
    assert_eq!(a.session_analysis.summary().rounds, 1);
    let old_grade = a.training.stats();
    assert!(old_grade.all.decisions > 0);
    a.event(Event::Hold(Key::Grey));
    select(&mut a, 10);
    select(&mut a, 1);
    assert_eq!(a.view_name(), "Summary");
    assert_eq!(a.session_analysis.summary().net, a.game.net() as i128);
    a.event(B);
    assert_eq!(a.view_name(), "Cut");
    assert_eq!(a.session_analysis.summary().rounds, 0);
    assert_eq!(a.session_metrics.card_stats().all.decisions, 0);
    assert_eq!(a.training.stats(), old_grade);
    settle(&mut a, &mut now);
    assert_eq!(a.session_analysis.summary().rounds, 1);
    assert_eq!(a.session_analysis.summary().net, a.game.net() as i128);
}
#[test]
fn menu_analysis_never_advances_live_rng_or_changes_frozen_ledger() {
    let mut a = App::new(17);
    let mut now = 0;
    for _ in 0..6 {
        settle(&mut a, &mut now)
    }
    let expected = a.session_analysis.summary();
    let last = a.session_analysis.newest(0);
    let rng = a.game.rng;
    let position = a.game.position;
    analysis_menu(&mut a);
    for page in 0..7 {
        select(&mut a, page);
        for _ in 0..20 {
            a.render();
            a.event(G);
            now += 100;
            a.tick(now)
        }
        a.event(B);
        assert_eq!(a.view_name(), "AnalysisMenu");
        assert_eq!(a.game.rng, rng);
        assert_eq!(a.game.position, position);
        assert_eq!(a.session_analysis.summary(), expected);
        assert_eq!(a.session_analysis.newest(0), last);
    }
    assert!(a.save().valid());
}
#[test]
fn extra_seat_rounds_are_counted_once_with_only_controlled_player_money() {
    let mut a = App::new(99);
    a.game.configure_seats(6, 4);
    let mut now = 0;
    let mut net = 0;
    for _ in 0..12 {
        settle(&mut a, &mut now);
        net += a.game.round_net;
        assert_eq!(a.session_analysis.summary().net, net as i128);
        assert_eq!(a.session_analysis.summary().rounds, a.game.rounds);
        assert!(a.game.invariant());
    }
}
#[test]
fn wager_first_answers_are_separate_from_live_card_accuracy() {
    let mut a = App::new(7);
    a.wager_enabled = true;
    a.event(B);
    a.event(B);
    assert_eq!(a.view_name(), "WagerAnswer");
    a.event(B);
    select(&mut a, 4);
    assert_eq!(a.session_metrics.wager_stats().all.errors, 1);
    assert_eq!(a.session_metrics.card_stats().all.decisions, 0);
    a.event(B);
    let mut now = 0;
    for _ in 0..300 {
        now += 200;
        a.tick(now);
        if !a.is_busy() && matches!(a.game.phase, Phase::Player | Phase::Insurance) {
            a.event(B)
        }
        if a.game.phase == Phase::Result && !a.is_busy() {
            break;
        }
    }
    assert_eq!(a.session_analysis.summary().rounds, 1);
    assert_eq!(a.session_analysis.newest(0).unwrap().base_wager, 1000);
    assert_eq!(a.session_metrics.wager_stats().all.errors, 1);
}
#[test]
fn malformed_live_ledger_is_reported_as_stopped_without_rewriting_old_rounds() {
    let mut a = App::new(7);
    a.game.bankroll += 100;
    let mut now = 0;
    for _ in 0..300 {
        if a.view_name() == "Cut" || a.view_name() == "Burn" {
            a.event(B)
        }
        now += 200;
        a.tick(now);
        if !a.is_busy() && matches!(a.game.phase, Phase::Player | Phase::Insurance) {
            a.event(B)
        }
        if a.game.phase == Phase::Result && !a.is_busy() {
            break;
        }
    }
    assert_eq!(a.session_analysis.summary().rounds, 0);
    analysis_menu(&mut a);
    a.render();
    assert!(a.game.invariant());
    assert_eq!(a.game.rounds, 1);
}
#[test]
fn export_analysis_screens_when_requested() {
    let Ok(dir) = std::env::var("STARDUST_ANALYSIS_SCREEN_DIR") else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    fn dump(a: &mut App, dir: &str, name: &str) {
        a.render();
        let mut bytes = b"P6\n240 135\n255\n".to_vec();
        for &p in &a.frame.pixels {
            let (r, g, b) = (
                ((p >> 11) & 31) as u8,
                ((p >> 5) & 63) as u8,
                (p & 31) as u8,
            );
            bytes.extend_from_slice(&[
                (r << 3) | (r >> 2),
                (g << 2) | (g >> 4),
                (b << 3) | (b >> 2),
            ]);
        }
        std::fs::write(format!("{dir}/{name}.ppm"), bytes).unwrap();
    }
    let mut a = App::new(17);
    a.training.mode = Mode::Test;
    let mut now = 0;
    for _ in 0..35 {
        settle(&mut a, &mut now)
    }
    analysis_menu(&mut a);
    dump(&mut a, &dir, "menu");
    for (index, name) in [
        (0, "summary"),
        (1, "graph"),
        (2, "round"),
        (3, "accuracy"),
        (4, "costs"),
        (5, "first-answer"),
        (6, "assumptions"),
    ] {
        select(&mut a, index);
        dump(&mut a, &dir, name);
        if index == 0 {
            for name in ["outcomes", "dispersion", "reference"] {
                a.event(G);
                dump(&mut a, &dir, name)
            }
        }
        if index == 4 {
            a.event(G);
            dump(&mut a, &dir, "costs-assisted");
            a.event(G);
            dump(&mut a, &dir, "costs-skipped");
        }
        if index == 6 {
            a.event(G);
            dump(&mut a, &dir, "assumptions-reference");
            a.event(G);
            dump(&mut a, &dir, "assumptions-policy");
        }
        a.event(B);
    }
}

#[test]
fn round_base_wager_is_frozen_at_deal_even_if_next_wager_changes_before_animation_finishes() {
    for seed in 1..300 {
        let mut a = App::new(seed);
        a.game.set_bet(1000);
        a.event(B);
        a.event(B);
        if a.game.phase != Phase::Result {
            continue;
        }
        assert!(a.game.set_bet(5000));
        for n in 1..=5 {
            a.tick(n * 200)
        }
        assert_eq!(a.session_analysis.newest(0).unwrap().base_wager, 1000);
        assert_eq!(a.session_analysis.summary().largest_wager, 1000);
        assert_eq!(a.session_analysis.summary().net, a.game.round_net as i128);
        return;
    }
    panic!("no initial natural");
}
