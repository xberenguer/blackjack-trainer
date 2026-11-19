use stardust_blackjack::{
    canvas::Canvas,
    cards::{Card, Hand},
    counting::Tracker,
    engine::{Engine, Phase},
    render::Renderer,
    rules::{Profile, Rules},
    table::{self, Mode, Options, Policy, Reveal},
};
fn playing() -> Engine {
    for seed in 1..100 {
        let mut g = Engine::new(seed, Rules::default()).unwrap();
        g.buy_in(100_000);
        g.set_bet(500);
        g.cut(156);
        g.deal();
        if g.phase == Phase::Insurance {
            g.insure(false);
        }
        if g.phase == Phase::Player {
            return g;
        }
    }
    panic!("no ordinary fixture")
}
#[test]
fn hidden_dealer_faces_do_not_change_guide_pixels_or_running_count() {
    let mut g = playing();
    let mut t = Tracker::default();
    let mut r = Renderer::default();
    let mut canvas = Canvas::default();
    let o = Options {
        mode: Mode::Guide,
        ..Options::default()
    };
    table::draw(&mut canvas, &mut r, &g, &mut t, o);
    assert_eq!(t.observed_count(), 3);
    let pixels = canvas.clone();
    let up = g.dealer.cards()[0];
    let positions = [g.dealer.positions()[0], g.dealer.positions()[1]];
    let running = t.running_count();
    for id in 0..52 {
        g.dealer = Hand::default();
        g.dealer.add(up, positions[0]);
        g.dealer.add(Card::from_id(id).unwrap(), positions[1]);
        table::draw(&mut canvas, &mut r, &g, &mut t, o);
        assert!(canvas == pixels);
        assert_eq!(t.running_count(), running);
        assert_eq!(t.observed_count(), 3);
    }
    g.hole_visible = true;
    g.phase = Phase::Dealer;
    table::draw(&mut canvas, &mut r, &g, &mut t, o);
    assert_eq!(t.observed_count(), 4);
    table::draw(&mut canvas, &mut r, &g, &mut t, o);
    assert_eq!(t.observed_count(), 4);
}
#[test]
fn initial_reveal_counts_only_faces_the_player_has_seen() {
    let g = playing();
    let mut t = Tracker::default();
    let mut r = Renderer::default();
    let mut canvas = Canvas::default();
    for (progress, expected) in [(0, 0), (1, 1), (2, 2), (3, 3), (4, 3)] {
        table::draw(
            &mut canvas,
            &mut r,
            &g,
            &mut t,
            Options {
                reveal: Some(Reveal {
                    progress,
                    bankroll: 99500,
                }),
                ..Options::default()
            },
        );
        assert_eq!(t.observed_count(), expected);
    }
}
#[test]
fn normal_test_and_coach_tables_remain_minimal_and_guide_counter_defaults_are_separate() {
    let g = playing();
    let mut t = Tracker::default();
    let mut r = Renderer::default();
    let mut canvas = Canvas::default();
    let normal = table::draw(&mut canvas, &mut r, &g, &mut t, Options::default());
    assert!(!normal.play_assisted);
    assert_eq!(normal.count_aids, 0);
    let pixels = canvas.clone();
    for mode in [Mode::Test, Mode::Coach] {
        let e = table::draw(
            &mut canvas,
            &mut r,
            &g,
            &mut t,
            Options {
                mode,
                ..Options::default()
            },
        );
        assert!(canvas == pixels);
        assert_eq!(e.count_aids, 0);
        assert!(!e.play_assisted);
    }
    let e = table::draw(
        &mut canvas,
        &mut r,
        &g,
        &mut t,
        Options {
            mode: Mode::Guide,
            ..Options::default()
        },
    );
    assert!(e.play_assisted);
    assert_eq!(e.count_aids, 7);
    let e = table::draw(
        &mut canvas,
        &mut r,
        &g,
        &mut t,
        Options {
            mode: Mode::Guide,
            guide_counter: 0,
            ..Options::default()
        },
    );
    assert!(e.play_assisted);
    assert_eq!(e.count_aids, 0);
}
#[test]
fn count_revealed_in_a_hilo_explanation_is_assistance_even_with_hud_hidden() {
    let mut g = playing();
    g.rules = Rules::for_profile(Profile::SixNoSurrender);
    g.phase = Phase::Player;
    g.position = 30;
    g.hand_count = 1;
    g.active = 0;
    g.hands[0] = Hand::default();
    g.hands[0].bet = 500;
    g.hands[0].add(Card::from_id(8).unwrap(), 20);
    g.hands[0].add(Card::from_id(4).unwrap(), 22);
    g.dealer = Hand::default();
    g.dealer.add(Card::from_id(8).unwrap(), 21);
    g.dealer.add(Card::from_id(5).unwrap(), 23);
    let mut t = Tracker::default();
    t.reset(6, g.shoe_number);
    for p in [2, 3, 7] {
        t.observe(p, Card::from_id(0).unwrap());
    }
    let mut r = Renderer::default();
    let mut canvas = Canvas::default();
    let e = table::draw(
        &mut canvas,
        &mut r,
        &g,
        &mut t,
        Options {
            mode: Mode::Guide,
            policy: Policy::HiLo,
            guide_counter: 0,
            ..Options::default()
        },
    );
    assert_eq!(t.running_count(), 2);
    assert!(e.play_assisted);
    assert_eq!(e.count_aids, 1);
}
