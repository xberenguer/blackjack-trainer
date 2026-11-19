//! UI contract checks compare the ordinary actionable insurance UI with Guide,
//! independently of how advice is drawn. Fixtures contain no player save data.
#[path = "support/insurance_fixture.rs"]
mod insurance_fixture;
use stardust_blackjack::{
    canvas::{Canvas, WIDTH, rgb},
    cards::{Card, Hand},
    counting::Tracker,
    engine::Engine,
    render::Renderer,
    table::{self, Mode, Options, Policy},
};
fn frame(g: &Engine, t: &Tracker, o: Options) -> (Canvas, Tracker, table::Exposure) {
    let mut c = Canvas::default();
    let mut r = Renderer::default();
    let mut t = t.clone();
    let exposure = table::draw(&mut c, &mut r, g, &mut t, o);
    (c, t, exposure)
}
fn region(c: &Canvas, x: usize, y: usize, w: usize, h: usize) -> Vec<u16> {
    (y..y + h)
        .flat_map(|row| {
            c.pixels[row * WIDTH + x..row * WIDTH + x + w]
                .iter()
                .copied()
        })
        .collect()
}
#[test]
fn guide_retains_identical_prompt_choices_amount_and_selection_as_ordinary_insurance() {
    for funded in [false, true] {
        for policy in [Policy::Basic, Policy::HiLo] {
            for high in [false, true] {
                for yes in [false, true] {
                    let (g, t) = insurance_fixture::fixture(funded, high);
                    let options = Options {
                        policy,
                        insurance_yes: yes,
                        ..Options::default()
                    };
                    let (off, _, _) = frame(&g, &t, options);
                    for toast in [None, Some(Mode::Guide)] {
                        let (guide, tracked, e) = frame(
                            &g,
                            &t,
                            Options {
                                mode: Mode::Guide,
                                toast,
                                ..options
                            },
                        );
                        assert_eq!(
                            region(&guide, 164, 53, 76, 82),
                            region(&off, 164, 53, 76, 82),
                            "Guide/its mode toast must never replace the insurance prompt, dollar amount or either full-size selectable row"
                        );
                        assert_eq!(
                            guide.pixels[(if yes { 112 } else { 88 }) * WIDTH + 165],
                            rgb(255, 255, 0),
                            "selected row has the ordinary gold highlight"
                        );
                        assert_eq!(
                            region(&guide, 0, 0, 76, 50),
                            region(&off, 0, 0, 76, 50),
                            "upcard and hidden-hole back remain untouched"
                        );
                        assert_eq!(
                            region(&guide, 0, 53, 117, 82),
                            region(&off, 0, 53, 117, 82),
                            "both full-size player cards remain untouched"
                        );
                        assert_eq!(tracked.observed_count(), if high { 21 } else { 3 });
                        assert_eq!(tracked.running_count(), if high { 19 } else { 1 });
                        assert!(e.play_assisted);
                        assert_eq!(e.count_aids, 7);
                    }
                }
            }
        }
    }
}
#[test]
fn policy_threshold_and_funding_change_advice_without_changing_the_choices() {
    let (g, t) = insurance_fixture::fixture(true, true);
    let o = Options {
        mode: Mode::Guide,
        policy: Policy::HiLo,
        ..Options::default()
    };
    let (hilo, _, _) = frame(&g, &t, o);
    let (basic, _, _) = frame(
        &g,
        &t,
        Options {
            policy: Policy::Basic,
            ..o
        },
    );
    let (poor, t2) = insurance_fixture::fixture(false, true);
    let (unfunded, _, _) = frame(&poor, &t2, o);
    let (low, t3) = insurance_fixture::fixture(true, false);
    let (below, _, _) = frame(&low, &t3, o);
    assert_ne!(
        region(&hilo, 82, 1, 72, 48),
        region(&basic, 82, 1, 72, 48),
        "high-count Hi-Lo insurance recommendation must differ from Basic decline"
    );
    assert_ne!(
        region(&hilo, 82, 1, 72, 48),
        region(&unfunded, 82, 1, 72, 48),
        "unfunded half-bet must get its decline/funding explanation"
    );
    assert_ne!(
        region(&hilo, 82, 1, 72, 48),
        region(&below, 82, 1, 72, 48),
        "below-index Hi-Lo advice must differ from insure"
    );
    for c in [&basic, &unfunded, &below] {
        assert_eq!(region(c, 164, 53, 76, 82), region(&hilo, 164, 53, 76, 82));
    }
}
#[test]
fn insurance_advice_never_changes_header_count_or_hidden_hole_pixels() {
    for money in [99_500, stardust_blackjack::rules::MAX_BANK, i64::MAX / 4] {
        let (mut g, t) = insurance_fixture::fixture(true, true);
        g.bankroll = money;
        let off_options = Options {
            normal_counter: 2,
            policy: Policy::HiLo,
            ..Options::default()
        };
        let (off, _, _) = frame(&g, &t, off_options);
        let options = Options {
            mode: Mode::Guide,
            ..off_options
        };
        let (original, counted, _) = frame(&g, &t, options);
        assert_eq!(
            region(&original, 154, 0, 86, 53),
            region(&off, 154, 0, 86, 53),
            "bankroll, RC/TC and BET must remain independent of advice at large balances"
        );
        for id in 0..52 {
            let up = g.dealer.cards()[0];
            g.dealer = Hand::EMPTY;
            g.dealer.add(up, 153);
            g.dealer.add(Card::from_id(id).unwrap(), 155);
            let (other, observed, _) = frame(&g, &t, options);
            assert!(
                other == original,
                "insurance guide must never inspect the hole card"
            );
            assert_eq!(observed.running_count(), counted.running_count());
            assert_eq!(observed.observed_count(), counted.observed_count());
        }
    }
}

#[test]
fn extreme_count_text_and_large_money_stay_outside_the_advice_panel() {
    let (mut g, mut tracker) = insurance_fixture::fixture(true, false);
    g.position = 311;
    g.bankroll = i64::MAX / 4;
    tracker.reset(6, g.shoe_number);
    for position in 0..119 {
        tracker.observe(position, Card::from_id((position % 5) as u8).unwrap());
    }
    let off = Options {
        normal_counter: 2,
        policy: Policy::HiLo,
        ..Options::default()
    };
    let (a, _, _) = frame(&g, &tracker, off);
    let (b, t, _) = frame(
        &g,
        &tracker,
        Options {
            mode: Mode::Guide,
            ..off
        },
    );
    assert_eq!(t.running_count(), 120);
    assert_eq!(t.true_count(1), Some(6240.0));
    for y in 0..135 {
        for x in 0..240 {
            if a.pixels[y * WIDTH + x] != b.pixels[y * WIDTH + x] {
                assert!(
                    (82..154).contains(&x) && (1..49).contains(&y),
                    "advice may not overwrite bankroll, long RC/TC, bet, cards or insurance controls at {x},{y}"
                );
            }
        }
    }
}
#[test]
fn grey_cycles_and_blue_confirms_insurance_in_guide() {
    use stardust_blackjack::{
        app::App,
        controls::{Event, Key},
        engine::Phase,
    };
    let b = Event::Tap(Key::Blue);
    let grey = Event::Tap(Key::Grey);
    for funded in [false, true] {
        for policy in [Policy::Basic, Policy::HiLo] {
            let mut app = App::new(7);
            app.event(b);
            app.event(b);
            for now in (200..5000).step_by(200) {
                app.tick(now);
                app.render();
                if !app.is_busy() {
                    break;
                }
            }
            assert_eq!(app.view_name(), "Table");
            app.game = insurance_fixture::fixture(funded, false).0;
            app.training.mode = Mode::Guide;
            app.policy = policy;
            let bankroll = app.game.bankroll;
            app.render();
            assert_eq!(app.focus(), 0);
            app.event(grey);
            assert_eq!(app.focus(), 1);
            assert_eq!(app.game.phase, Phase::Insurance);
            assert_eq!(app.game.bankroll, bankroll);
            app.event(b);
            if funded {
                assert!(app.game.insured);
                assert_eq!(app.game.bankroll, bankroll - 250);
            } else {
                assert!(!app.game.insured);
                assert_eq!(app.game.phase, Phase::Insurance);
                assert_eq!(app.game.bankroll, bankroll);
                app.event(grey);
                app.event(b);
                assert_eq!(app.game.phase, Phase::Player);
            }
        }
    }
}
