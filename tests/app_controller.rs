#[path = "support/navigation.rs"]
mod nav;
use stardust_blackjack::{
    app::App,
    controls::{Event, Key},
    engine::{Action, Phase},
    persistence::Save,
    training::{Mode, Policy},
};
const BLUE: Event = Event::Tap(Key::Blue);
const GREY: Event = Event::Tap(Key::Grey);
fn select(a: &mut App, n: usize) {
    for _ in 0..n {
        a.event(GREY);
    }
    a.event(BLUE);
}
fn reveal(a: &mut App) {
    for n in 1..=5 {
        a.tick(n * 200)
    }
    a.render();
}
fn start(a: &mut App) {
    a.event(BLUE);
    assert_eq!(a.view_name(), "Burn");
    a.event(BLUE);
    assert_ne!(a.game.phase, Phase::Ready);
    reveal(a);
}
#[test]
fn boot_and_two_blue_fast_path() {
    let mut a = App::new(7);
    assert_eq!(a.game.bankroll, 100000);
    assert_eq!(a.game.selected_bet, a.game.rules.minimum);
    assert_eq!(a.training.mode, Mode::Off);
    assert_eq!(a.policy, Policy::HiLo);
    assert_eq!(a.count_display, 0);
    assert_eq!(a.guide_count_display, 2);
    assert_eq!(a.view_name(), "Cut");
    start(&mut a);
    assert!(a.game.invariant());
    assert!(a.frame.pixels.iter().any(|&p| p != 0));
    println!("App size: {}", core::mem::size_of::<App>());
}
#[test]
fn physical_hold_cycle_consumes_release() {
    let mut a = App::new(2);
    a.buttons([true, false], 100);
    a.buttons([true, false], 950);
    assert_eq!(a.training.mode, Mode::Test);
    let p = a.game.position;
    a.buttons([false, false], 960);
    assert_eq!(a.view_name(), "Cut");
    assert_eq!(a.game.position, p);
    a.cancel_buttons();
    a.buttons([false, true], 1000);
    a.cancel_buttons();
    a.buttons([false, false], 2000);
    assert_eq!(a.view_name(), "Cut");
}
#[test]
fn grey_hit_blue_stand_and_menu_extras() {
    for seed in 0..32 {
        let mut a = App::new(seed);
        start(&mut a);
        if a.game.phase == Phase::Player && a.game.legal(Action::Hit) {
            let n = a.game.position;
            a.event(GREY);
            assert!(a.game.position > n);
            if a.game.phase == Phase::Player {
                a.event(BLUE);
                assert_ne!(a.game.phase, Phase::Player)
            }
            return;
        }
    }
    panic!("no player turn");
}
#[test]
fn coach_records_immutable_wrong_attempt_before_action() {
    for seed in 0..100 {
        let mut a = App::new(seed);
        a.training.mode = Mode::Coach;
        a.training.pause_on_mistake = true;
        start(&mut a);
        if a.game.phase != Phase::Player {
            continue;
        }
        let pos = a.game.position;
        a.event(BLUE);
        if a.view_name() == "CoachCheck" {
            assert_eq!(a.game.position, pos);
            assert_eq!(a.game.phase, Phase::Player);
            assert_eq!(a.training.history_count(), 1);
            let d = *a.training.newest(0).unwrap();
            assert!(!d.correct);
            a.event(GREY);
            a.event(BLUE);
            assert_eq!(a.training.history_count(), 1);
            assert_eq!(
                a.training.newest(0).unwrap().attempt.choice,
                d.attempt.choice
            );
            assert_eq!(a.training.stats().all.errors, 1);
            return;
        }
    }
    panic!("no wrong stand found");
}
#[test]
fn silent_ev_capture_does_not_assist_first_attempt() {
    for seed in 0..30 {
        let mut a = App::new(seed);
        a.training.mode = Mode::Test;
        start(&mut a);
        if a.game.phase != Phase::Player {
            continue;
        }
        a.event(BLUE);
        let d = a.training.newest(0).unwrap();
        assert!(!d.assisted);
        assert!(d.attempt.values.is_some());
        assert!(d.attempt.values.unwrap().valid(
            false,
            d.attempt.legal_mask,
            &d.attempt.hand,
            d.attempt.hand_index
        ));
        return;
    }
    panic!();
}
#[test]
fn viewing_ev_then_returning_marks_current_decision() {
    let mut a = App::new(7);
    a.training.mode = Mode::Test;
    start(&mut a);
    if a.game.phase != Phase::Player {
        return;
    }
    nav::path(&mut a, &["Training", "Advice & EV", "Action EV"]);
    assert_eq!(a.view_name(), "ActionEV");
    a.render();
    a.event(BLUE);
    nav::path(&mut a, &["Back to game"]);
    assert_eq!(a.view_name(), "Table");
    a.event(BLUE);
    assert!(a.training.newest(0).unwrap().assisted);
}
#[test]
fn count_submit_is_quiz_only_and_consumes_tails() {
    let mut a = App::new(4);
    let p = a.game.position;
    a.event(Event::SubmitCount);
    assert_eq!(a.game.position, p);
    nav::path(&mut a, &["Training", "Practice", "Counting"]);
    a.event(BLUE);
    assert_eq!(a.view_name(), "CountAnswer");
    assert_eq!(a.count_practice.stats().all.decisions, 0);
    a.buttons([true, true], 100);
    assert_eq!(a.view_name(), "CountResult");
    assert_eq!(a.count_practice.stats().all.decisions, 1);
    a.buttons([false, true], 200);
    a.buttons([false, false], 300);
    assert_eq!(a.view_name(), "CountResult");
    a.render();
    assert!(a.count_practice.newest(0).unwrap().correct);
}
#[test]
fn count_help_and_checkpoint_history_keep_assistance() {
    let mut a = App::new(5);
    nav::path(&mut a, &["Training", "Practice", "Counting"]);
    a.event(BLUE);
    a.request_count_help();
    a.event(Event::SubmitCount);
    assert!(a.count_practice.newest(0).unwrap().assisted);
    a.event(GREY);
    assert_eq!(a.view_name(), "CountAnswer");
    a.event(Event::SubmitCount);
    assert!(a.count_practice.newest(0).unwrap().assisted);
}
#[test]
fn guide_count_display_is_independent_and_aids_are_sticky() {
    let mut a = App::new(7);
    a.training.mode = Mode::Guide;
    start(&mut a);
    a.render();
    if a.game.phase != Phase::Player {
        return;
    }
    a.training.mode = Mode::Test;
    a.guide_count_display = 0;
    assert_eq!(a.count_display, 0);
    a.event(BLUE);
    assert!(a.training.newest(0).unwrap().assisted);
}
#[test]
fn restore_is_boot_only_and_boots_off_at_profile_minimum() {
    let mut a = App::new(99);
    let s = Save {
        mode: Mode::Guide,
        policy: Policy::HiLo,
        minimum_bet: 1500,
        maximum_bet: 100000,
        brightness: 128,
        ..Save::default()
    };
    assert!(a.load(s));
    assert_eq!(a.training.mode, Mode::Off);
    assert_eq!(a.game.selected_bet, 1500);
    assert_eq!(a.game.bankroll, 100000);
    assert_eq!(a.policy, Policy::HiLo);
    assert!(!a.load(s));
    let mut b = App::new(1);
    start(&mut b);
    assert!(!b.load(s));
    assert!(a.save().valid());
}
#[test]
fn test_review_waits_for_final_table_and_is_untimed() {
    for seed in 0..30 {
        let mut a = App::new(seed);
        a.training.mode = Mode::Test;
        start(&mut a);
        if a.game.phase != Phase::Player {
            continue;
        }
        a.event(BLUE);
        for i in 1..20 {
            a.tick(1000 + i * 700);
            if a.game.phase == Phase::Result {
                break;
            }
        }
        assert_eq!(a.game.phase, Phase::Result);
        assert_eq!(a.view_name(), "Table");
        a.tick(20000);
        assert_eq!(a.view_name(), "RoundReview");
        a.tick(100000);
        assert_eq!(a.view_name(), "RoundReview");
        a.event(BLUE);
        assert_eq!(a.view_name(), "Table");
        return;
    }
    panic!();
}
#[test]
fn app_event_fuzz_preserves_engine_invariants_and_screens() {
    let mut a = App::new(1234);
    let mut r = 27u64;
    for i in 0..10000u32 {
        r = r.wrapping_mul(6364136223846793005).wrapping_add(1);
        a.tick(i.wrapping_mul(200));
        a.event(match r >> 61 {
            0 => Event::Hold(Key::Grey),
            1 => Event::Hold(Key::Blue),
            2 => Event::SubmitCount,
            3 | 4 => GREY,
            _ => BLUE,
        });
        if i % 7 == 0 {
            a.render()
        }
        assert!(a.game.invariant(), "event {i}, {}", a.view_name());
        assert!(a.save().valid());
    }
}

#[test]
fn training_round_ids_do_not_collide_after_new_session() {
    let mut a = App::new(7);
    a.training.mode = Mode::Test;
    start(&mut a);
    if a.game.phase != Phase::Player {
        return;
    }
    a.event(BLUE);
    for i in 1..30 {
        a.tick(1000 + i * 700);
        if a.game.phase == Phase::Result {
            break;
        }
    }
    a.tick(40000);
    if a.view_name() == "RoundReview" {
        a.event(BLUE)
    }
    let first = a.training.newest(0).unwrap().attempt.round;
    nav::path(&mut a, &["Session", "New session"]);
    nav::choose(&mut a, "Start new session");
    assert_eq!(a.view_name(), "Cut");
    a.event(BLUE);
    a.event(BLUE);
    for i in 1..=5 {
        a.tick(40000 + i * 200)
    }
    if a.game.phase == Phase::Player {
        a.event(BLUE);
        assert!(a.training.newest(0).unwrap().attempt.round > first)
    }
}
#[test]
fn restore_closes_after_first_actual_input() {
    let mut a = App::new(8);
    a.tick(500);
    a.render();
    assert!(a.load(Save::default()));
    let mut b = App::new(8);
    b.event(GREY);
    assert!(!b.load(Save::default()));
}
#[test]
fn shoe_information_marks_deck_quiz_assisted_at_same_checkpoint() {
    let mut a = App::new(9);
    nav::path(&mut a, &["Table", "Shoe info"]);
    assert_eq!(a.view_name(), "ShoeInfo");
    a.render();
    a.event(BLUE);
    // Back restores the Table category; the root helper follows visible parents.
    a.event(Event::Hold(Key::Grey));
    nav::path(&mut a, &["Training", "Practice", "Counting"]);
    select(&mut a, 1);
    assert_eq!(a.view_name(), "CountAnswer");
    a.event(Event::SubmitCount);
    assert!(a.count_practice.newest(0).unwrap().assisted);
}

#[test]
fn count_hud_during_progressive_deal_stays_assisted_after_hiding() {
    let mut a = App::new(7);
    a.training.mode = Mode::Guide;
    a.event(BLUE);
    a.event(BLUE);
    for now in [200, 400, 600] {
        a.tick(now);
        a.render()
    }
    a.training.mode = Mode::Test;
    a.count_display = 0;
    for now in [800, 1000] {
        a.tick(now);
        a.render()
    }
    if a.game.phase == Phase::Player {
        a.event(BLUE);
        assert!(a.training.newest(0).unwrap().assisted)
    }
}

#[test]
fn static_empty_boot_matches_regular_constructor_without_runtime_app_temporary() {
    for seed in [0, 1, 7, 99, u64::MAX] {
        let regular = App::new(seed);
        let mut in_place = App::EMPTY;
        assert_eq!(in_place.game.shoe_number, 0);
        assert!(in_place.boot(seed));
        assert!(!in_place.boot(seed));
        assert!(in_place.frame == regular.frame);
        assert_eq!(in_place.save(), regular.save());
        assert_eq!(in_place.game.rng, regular.game.rng);
        assert_eq!(in_place.game.bankroll, regular.game.bankroll);
        assert_eq!(in_place.game.selected_bet, regular.game.selected_bet);
        assert_eq!(in_place.game.shoe_number, regular.game.shoe_number);
        assert_eq!(in_place.view_name(), regular.view_name());
        assert!(in_place.game.invariant());
        // Independently assemble the original cut screen and bankroll contract,
        // rather than only comparing two entry points that share boot().
        let rules = stardust_blackjack::rules::Rules::default();
        let mut engine = stardust_blackjack::engine::Engine::new(seed, rules).unwrap();
        assert!(engine.buy_in(100000));
        assert!(engine.set_bet(rules.minimum));
        let cut_index = engine.bounded(17).unwrap() as u8;
        let mut frame = stardust_blackjack::canvas::Canvas::default();
        let mut renderer = stardust_blackjack::render::Renderer::default();
        stardust_blackjack::widgets::Widgets {
            frame: &mut frame,
            renderer: &mut renderer,
        }
        .cut(rules.decks, cut_index);
        assert!(in_place.frame == frame);
        assert_eq!(in_place.game.rng, engine.rng);
        assert_eq!(in_place.save(), Save::default());
        in_place.event(BLUE);
        assert!(engine.cut(52 + (engine.total_cards() - 104) * cut_index as usize / 16));
        in_place.event(BLUE);
        assert!(engine.deal());
        assert!(in_place.game.hands == engine.hands);
        assert_eq!(in_place.game.position, engine.position);
        assert_eq!(in_place.game.phase, engine.phase);
    }
}
