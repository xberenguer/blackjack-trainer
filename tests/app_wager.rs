#[path = "support/navigation.rs"]
mod nav;
use stardust_blackjack::{
    app::App,
    controls::{Event, Key},
    engine::Phase,
    persistence::{SIZE, Save, VERSION, crc32},
    training::{Counters, Mode},
    wager_practice::{self, Choice},
};
const B: Event = Event::Tap(Key::Blue);
const G: Event = Event::Tap(Key::Grey);
fn select(a: &mut App, n: usize) {
    for _ in 0..n {
        a.event(G);
    }
    a.event(B);
}
fn wager_menu(a: &mut App) {
    nav::path(a, &["Training", "Practice", "Bet sizing"]);
    assert_eq!(a.view_name(), "WagerPractice");
}
fn start_auto(a: &mut App) {
    a.wager_enabled = true;
    a.event(B);
    a.event(B);
    assert_eq!(a.view_name(), "WagerAnswer");
    assert_eq!(a.game.hand_count, 0);
    assert!(a.wager_practice.awaiting_answer())
}
#[test]
fn default_off_keeps_two_blue_deal_and_optin_freezes_before_cards() {
    let mut a = App::new(7);
    assert!(!a.wager_enabled);
    a.event(B);
    a.event(B);
    assert!(a.game.hand_count > 0);
    let mut b = App::new(7);
    start_auto(&mut b);
    assert_eq!(b.game.bankroll, 100000);
    assert_eq!(b.wager_practice.stats().all.decisions, 0);
    assert!(
        b.wager_practice
            .reference(b.wager_practice.question().unwrap().token)
            .is_none()
    );
}
#[test]
fn wrong_legal_first_wager_is_recorded_and_played_unchanged() {
    let mut a = App::new(7);
    start_auto(&mut a);
    a.event(B);
    select(&mut a, 4);
    assert_eq!(a.view_name(), "WagerResult");
    let first = *a.wager_practice.newest(0).unwrap();
    assert_eq!(first.choice, Choice::Wager(1000));
    assert!(!first.correct && !first.assisted);
    assert_eq!(first.reference.canonical.choice, Choice::Wager(500));
    assert_eq!(a.game.hand_count, 0);
    a.render();
    a.event(B);
    assert!(a.game.hand_count > 0);
    assert_eq!(a.game.hands[0].bet, 1000);
    assert_eq!(a.wager_practice.newest(0), Some(&first));
    assert_eq!(a.wager_practice.stats().all.errors, 1);
}
#[test]
fn manual_bypass_is_available_and_does_not_create_attempt() {
    let mut a = App::new(7);
    start_auto(&mut a);
    select(&mut a, 7);
    assert_eq!(a.view_name(), "Table");
    assert!(a.game.hand_count > 0);
    assert_eq!(a.wager_practice.history_count(), 0);
    assert!(!a.wager_practice.awaiting_answer());
    assert!(a.wager_enabled);
}
#[test]
fn only_count_quizzes_use_simultaneous_submit_and_holdblue_keeps_modecycle() {
    let mut a = App::new(7);
    start_auto(&mut a);
    a.buttons([true, true], 100);
    a.buttons([false, false], 200);
    assert_eq!(a.view_name(), "WagerAnswer");
    assert_eq!(a.wager_practice.history_count(), 0);
    a.event(Event::Hold(Key::Blue));
    assert_eq!(a.training.mode, Mode::Test);
    assert_eq!(a.view_name(), "WagerAnswer");
}
#[test]
fn explicit_help_unlocks_reference_and_marks_first_wager_assisted() {
    let mut a = App::new(7);
    start_auto(&mut a);
    select(&mut a, 6);
    assert_eq!(a.view_name(), "WagerHelp");
    let q = a.wager_practice.question().unwrap();
    assert!(a.wager_practice.reference(q.token).is_some());
    a.render();
    a.event(B);
    assert_eq!(a.view_name(), "WagerAnswer");
    select(&mut a, 4);
    let first = a.wager_practice.newest(0).unwrap();
    assert!(first.assisted && first.correct);
    assert_eq!(
        a.wager_practice.stats().all,
        Counters {
            decisions: 1,
            correct: 0,
            errors: 0,
            assisted: 1
        }
    );
}
#[test]
fn checkpoint_changes_cancel_unanswered_cases_and_block_stale_application() {
    let mut a = App::new(7);
    start_auto(&mut a);
    let token = a.wager_practice.question().unwrap().token;
    a.game.buy_in(500);
    a.tick(10);
    assert!(!a.wager_practice.awaiting_answer());
    assert_eq!(a.view_name(), "Notice");
    assert_eq!(
        a.wager_practice
            .submit(token, Choice::Wager(500), false)
            .status,
        wager_practice::Status::Duplicate
    );
    assert_eq!(a.wager_practice.history_count(), 0);
    let mut b = App::new(7);
    start_auto(&mut b);
    select(&mut b, 4);
    assert_eq!(b.view_name(), "WagerResult");
    b.game.buy_in(500);
    b.event(B);
    assert_eq!(b.view_name(), "Notice");
    assert_eq!(b.game.hand_count, 0);
    assert_eq!(b.wager_practice.history_count(), 1);
}
#[test]
fn returning_to_same_checkpoint_after_feedback_remains_assisted() {
    let mut a = App::new(7);
    start_auto(&mut a);
    select(&mut a, 4);
    assert!(!a.wager_practice.newest(0).unwrap().assisted);
    a.event(G);
    select(&mut a, 2);
    assert_eq!(a.view_name(), "WagerAnswer");
    select(&mut a, 4);
    assert!(a.wager_practice.newest(0).unwrap().assisted);
    assert_eq!(a.wager_practice.stats().all.decisions, 2);
}
#[test]
fn refill_is_not_zero_wager_and_deposit_is_separate() {
    let mut a = App::new(7);
    a.game.bankroll = 100;
    a.game.deposits = 100;
    a.wager_enabled = true;
    wager_menu(&mut a);
    nav::choose(&mut a, "Practice now");
    assert_eq!(a.view_name(), "WagerAnswer");
    select(&mut a, 5);
    assert_eq!(a.view_name(), "WagerResult");
    assert_eq!(
        a.wager_practice.newest(0).unwrap().choice,
        Choice::NoBetRefill
    );
    assert_eq!(
        a.wager_practice.newest(0).unwrap().category,
        wager_practice::Category::Refill
    );
    a.event(B);
    assert_eq!(a.game.bankroll, 100100);
    assert_eq!(a.game.deposits, 100100);
    assert_eq!(a.game.rounds, 0);
    assert_eq!(a.game.hand_count, 0);
    assert_eq!(a.game.net(), 0);
}
#[test]
fn optin_policy_stats_restore_once_and_roundtrip_exact_u64() {
    let n = (1u64 << 54) + 7;
    let counters = Counters {
        decisions: n,
        correct: n - 2,
        errors: 1,
        assisted: 1,
    };
    let mut stats = wager_practice::Stats {
        all: counters,
        ..wager_practice::Stats::default()
    };
    stats.categories[3] = counters;
    let s = Save {
        wager_enabled: true,
        wager_policy: wager_practice::Policy {
            units: [3, 5, 7, 9, 11],
            ..wager_practice::Policy::default()
        },
        wager_stats: stats,
        ..Save::default()
    };
    let packet = s.encode().unwrap();
    assert_eq!(VERSION, 14);
    assert_eq!(SIZE, 2636);
    assert_eq!(Save::decode(&packet), Some(s));
    let mut a = App::new(1);
    assert!(a.load(s));
    assert!(a.wager_enabled);
    assert_eq!(a.wager_policy.units, [3, 5, 7, 9, 11]);
    assert_eq!(a.wager_practice.stats(), stats);
    assert!(!a.load(s));
    assert_eq!(a.training.mode, Mode::Off);
}
#[test]
fn v6_seat_settings_migrate_without_creating_wager_progress() {
    let mut v6 = include_bytes!("data/save-v5.bin").to_vec();
    v6[4..6].copy_from_slice(&6u16.to_le_bytes());
    v6[200] = 6;
    v6[201] = 3;
    let crc = crc32(&v6[..332]);
    v6[332..].copy_from_slice(&crc.to_le_bytes());
    let s = Save::decode(&v6).unwrap();
    assert_eq!((s.extra_seats, s.player_seat), (6, 3));
    assert!(!s.wager_enabled);
    assert_eq!(s.wager_stats, wager_practice::Stats::default());
    assert_eq!(s.wager_policy, wager_practice::Policy::default());
    assert_eq!(Save::decode(&s.encode().unwrap()), Some(s));
}
#[test]
fn every_wager_screen_renders_without_mutating_first_attempt() {
    let mut a = App::new(7);
    start_auto(&mut a);
    select(&mut a, 4);
    let first = *a.wager_practice.newest(0).unwrap();
    a.render();
    a.event(G);
    a.render();
    select(&mut a, 3);
    assert_eq!(a.view_name(), "WagerPractice");
    nav::path(&mut a, &["Training", "Progress", "Bet sizing", "Accuracy"]);
    assert_eq!(a.view_name(), "WagerStats");
    for _ in 0..7 {
        a.render();
        a.event(G)
    }
    a.event(B);
    nav::choose(&mut a, "History");
    assert_eq!(a.view_name(), "WagerHistory");
    a.render();
    a.event(B);
    nav::path(&mut a, &["Training", "Practice", "Bet sizing", "Bet ramp"]);
    assert_eq!(a.view_name(), "WagerPolicy");
    for _ in 0..6 {
        a.render();
        a.event(G)
    }
    assert_eq!(a.wager_practice.newest(0), Some(&first));
    assert!(a.save().valid());
    assert!(matches!(a.game.phase, Phase::Ready | Phase::Result));
}

#[test]
fn enabled_unfunded_predeal_prompts_the_no_bet_case_before_refill() {
    let mut a = App::new(7);
    a.game.bankroll = 100;
    a.game.deposits = 100;
    start_auto(&mut a);
    select(&mut a, 5);
    assert!(a.wager_practice.newest(0).unwrap().correct);
    assert_eq!(
        a.wager_practice.newest(0).unwrap().category,
        wager_practice::Category::Refill
    );
}

#[test]
fn export_screens_when_requested() {
    let Ok(dir) = std::env::var("STARDUST_WAGER_SCREEN_DIR") else {
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
    let mut a = App::new(7);
    wager_menu(&mut a);
    dump(&mut a, &dir, "practice");
    nav::choose(&mut a, "Practice now");
    dump(&mut a, &dir, "answer");
    select(&mut a, 6);
    dump(&mut a, &dir, "help");
    a.event(B);
    select(&mut a, 4);
    dump(&mut a, &dir, "result");
    a.event(G);
    dump(&mut a, &dir, "result-options");
    select(&mut a, 3);
    nav::path(&mut a, &["Training", "Progress", "Bet sizing", "Accuracy"]);
    dump(&mut a, &dir, "stats");
    a.event(B);
    nav::choose(&mut a, "History");
    dump(&mut a, &dir, "history");
    a.event(B);
    nav::path(&mut a, &["Training", "Practice", "Bet sizing", "Bet ramp"]);
    dump(&mut a, &dir, "policy");
    let mut setup = App::new(7);
    nav::path(&mut setup, &["Table", "Table setup"]);
    dump(&mut setup, &dir, "profile");
    select(&mut setup, 4);
    dump(&mut setup, &dir, "seating");
    for seed in 1..100 {
        let mut terminal = App::new(seed);
        terminal.game.configure_seats(1, 1);
        terminal.event(B);
        terminal.event(B);
        for now in [200, 400, 600, 800, 1550, 2300] {
            terminal.tick(now)
        }
        let mut captured = false;
        for step in 1..100 {
            terminal.tick(2300 + step * 200);
            if terminal.game.phase == Phase::Player && terminal.is_busy() {
                dump(&mut terminal, &dir, "npc-terminal");
                captured = true;
                break;
            }
            if terminal.game.phase == Phase::Player && !terminal.is_busy() {
                break;
            }
        }
        if captured {
            break;
        }
    }
    for seed in 1..30 {
        let mut table = App::new(seed);
        table.game.configure_seats(6, 6);
        table.training.mode = Mode::Guide;
        table.event(B);
        table.event(B);
        for now in [200, 400, 600, 800, 1550] {
            table.tick(now)
        }
        dump(&mut table, &dir, "initial-seat");
        for now in [2300, 3050, 3800, 4550, 5300, 6050] {
            table.tick(now)
        }
        if table.game.phase == Phase::OtherPlayers {
            dump(&mut table, &dir, "npc-turn");
            nav::path(&mut table, &["Table", "Review cards"]);
            dump(&mut table, &dir, "seat-cards");
            break;
        }
    }
}

#[test]
fn saved_seats_are_applied_and_preserved_by_new_session() {
    let s = Save {
        extra_seats: 6,
        player_seat: 4,
        penetration: 90,
        ..Save::default()
    };
    let mut a = App::new(17);
    assert!(a.load(s));
    assert_eq!((a.game.extra_seats, a.game.player_seat), (6, 4));
    assert_eq!(a.game.rules.penetration, 90);
    assert_eq!(a.save(), s);
    nav::path(&mut a, &["Session", "New session"]);
    nav::choose(&mut a, "Start new session");
    assert_eq!((a.game.extra_seats, a.game.player_seat), (6, 4));
    assert_eq!(a.game.rules.penetration, 90);
    assert_eq!(a.game.bankroll, 100000);
}
#[test]
fn npc_turns_tick_without_consuming_user_actions_and_wager_counts_include_all_visible_seats() {
    for place in 0..=6 {
        let mut a = App::new(200 + place as u64);
        assert!(a.game.configure_seats(6, place));
        a.event(B);
        a.event(B);
        let mut now = 0;
        for _ in 0..200 {
            now += 700;
            a.tick(now);
            a.render();
            if a.game.phase == Phase::OtherPlayers {
                let pos = a.game.position;
                a.event(B);
                a.event(G);
                assert_eq!(a.game.position, pos)
            }
            if !a.is_busy() {
                match a.game.phase {
                    Phase::Insurance | Phase::Player => a.event(B),
                    Phase::Result => break,
                    _ => {}
                }
            }
        }
        assert_eq!(a.game.phase, Phase::Result, "seat {place}");
        assert!(a.game.invariant());
        let mut tracker = stardust_blackjack::counting::Tracker::default();
        a.game.observe_exposed(&mut tracker);
        wager_menu(&mut a);
        nav::choose(&mut a, "Practice now");
        select(&mut a, 6);
        let q = a.wager_practice.question().unwrap();
        let reference = a.wager_practice.reference(q.token).unwrap();
        assert_eq!(reference.snapshot.count.running, tracker.running_count());
        assert_eq!(reference.snapshot.count.observed, tracker.observed_count());
        assert_eq!(reference.snapshot.count.shoe, tracker.shoe_number());
        assert_eq!(
            reference.snapshot.count.undealt,
            (a.game.total_cards() - a.game.position) as u16
        );
    }
}

#[test]
fn viewed_count_hud_assists_wager_without_unlocking_example_answer() {
    let mut a = App::new(7);
    a.count_display = 1;
    a.event(B);
    a.tick(1200);
    assert_eq!(a.view_name(), "Table");
    a.render();
    a.wager_enabled = true;
    a.event(B);
    assert_eq!(a.view_name(), "WagerAnswer");
    let q = a.wager_practice.question().unwrap();
    assert!(a.wager_practice.reference(q.token).is_none());
    select(&mut a, 4);
    assert!(a.wager_practice.newest(0).unwrap().assisted);
}

#[test]
fn seating_and_deeper_cut_are_configurable_through_the_profile_menu() {
    let mut a = App::new(7);
    nav::path(&mut a, &["Table", "Table setup"]);
    select(&mut a, 4);
    assert_eq!(a.view_name(), "TableSeats");
    for _ in 0..6 {
        a.event(B)
    }
    a.event(G);
    for _ in 0..4 {
        a.event(B)
    }
    a.event(G);
    a.event(B);
    assert_eq!((a.game.extra_seats, a.game.player_seat), (6, 4));
    assert_eq!(a.view_name(), "Cut");
    nav::path(&mut a, &["Table", "Table setup"]);
    select(&mut a, 3);
    assert_eq!(a.view_name(), "ProfileDepth");
    select(&mut a, 3);
    assert_eq!(a.view_name(), "TableProfile");
    select(&mut a, 6);
    assert_eq!(a.game.rules.penetration, 90);
    assert_eq!((a.game.extra_seats, a.game.player_seat), (6, 4));
    assert!(a.save().valid());
}
#[test]
fn exceptional_discard_refill_has_an_explicit_acknowledged_notice() {
    let mut a = App::new(7);
    a.event(B);
    a.event(B);
    for n in 1..=5 {
        a.tick(n * 200)
    }
    assert_eq!(a.game.phase, Phase::Player);
    a.game.emergency_shuffles = 1;
    a.event(B);
    assert_eq!(a.view_name(), "Notice");
    a.render();
    a.event(B);
    assert_eq!(a.view_name(), "Table");
}

#[test]
fn every_initial_other_seat_is_shown_big_before_actions_or_count_aids() {
    use stardust_blackjack::{
        canvas::{Canvas, FELT},
        render::Renderer,
        table,
    };
    for player in [0, 3, 6] {
        let mut a = App::new(71 + player as u64);
        a.fast_mode = false;
        a.game.configure_seats(6, player);
        a.training.mode = Mode::Guide;
        a.event(B);
        a.event(B);
        for now in [200, 400, 600, 800] {
            a.tick(now);
            a.render()
        }
        assert!(a.is_busy());
        let mut tracker = stardust_blackjack::counting::Tracker::default();
        let mut frame = Canvas::default();
        let mut renderer = Renderer::default();
        let exposure = table::draw(
            &mut frame,
            &mut renderer,
            &a.game,
            &mut tracker,
            table::Options {
                mode: Mode::Guide,
                reveal: Some(table::Reveal {
                    progress: 4,
                    bankroll: a.game.bankroll,
                }),
                ..table::Options::default()
            },
        );
        assert_eq!(exposure.count_aids, 0);
        assert!(!exposure.play_assisted);
        let fixed_position = a.game.position;
        let mut now = 800;
        for index in 0..6 {
            now += 750;
            a.tick(now);
            a.render();
            assert!(a.is_busy());
            let h = a.game.others[index].hands[0];
            let mut expected = Canvas::default();
            expected.clear(FELT);
            for (i, c) in h.cards().iter().take(2).enumerate() {
                renderer.card(&mut expected, (8 + i as i32 * 56, 36), Some(*c), false)
            }
            for y in 36..116 {
                for x in 8..119 {
                    assert_eq!(
                        a.frame.pixels[y * 240 + x],
                        expected.pixels[y * 240 + x],
                        "seat{player}, stage{index}, x{x},y{y}"
                    );
                }
            }
            a.event(B);
            a.event(G);
            a.event(Event::SubmitCount);
            assert_eq!(a.game.position, fixed_position);
            assert_eq!(a.count_practice.history_count(), 0);
        }
        a.tick(now + 749);
        assert!(a.is_busy());
        a.tick(now + 750);
        a.render();
        assert_eq!(a.game.position, fixed_position);
        assert_eq!(
            a.game
                .others
                .iter()
                .take(6)
                .map(|s| s.hands[0].count())
                .sum::<usize>(),
            12
        );
    }
}
#[test]
fn menu_pause_and_mode_cycle_cannot_skip_initial_seat_faces() {
    let mut a = App::new(81);
    a.fast_mode = false;
    a.game.configure_seats(2, 0);
    a.event(B);
    a.event(B);
    for now in [200, 400, 600, 800, 1550] {
        a.tick(now)
    }
    a.render();
    let before = a.frame.clone();
    a.event(Event::Hold(Key::Grey));
    assert_eq!(a.view_name(), "Menu");
    a.tick(10000);
    a.event(B);
    assert_eq!(a.view_name(), "Table");
    a.render();
    assert!(a.frame == before);
    a.tick(10749);
    a.render();
    assert!(a.frame == before);
    a.event(Event::Hold(Key::Blue));
    assert_eq!(a.training.mode, Mode::Test);
    assert!(a.is_busy());
    a.tick(10750);
    a.render();
    assert!(a.frame != before);
    assert!(a.is_busy());
    a.tick(11500);
    assert!(!a.is_busy());
}

#[test]
fn terminal_npc_draw_stays_visible_and_blocks_player_decision_after_phase_advance() {
    use stardust_blackjack::{
        canvas::{Canvas, FELT},
        render::Renderer,
    };
    for seed in 1..300 {
        let mut a = App::new(seed);
        a.fast_mode = false;
        a.game.configure_seats(1, 1);
        a.training.mode = Mode::Test;
        a.event(B);
        a.event(B);
        for now in [200, 400, 600, 800, 1550, 2300] {
            a.tick(now)
        }
        let mut now = 2300;
        for _ in 0..30 {
            if a.game.phase != Phase::OtherPlayers {
                break;
            }
            let before = a.game.others[0];
            now += 650;
            a.tick(now);
            let after = a.game.others[0];
            let changed: Vec<_> = (0..after.hand_count)
                .filter(|&i| {
                    i >= before.hand_count
                        || before.hands[i].cards() != after.hands[i].cards()
                        || before.hands[i].positions() != after.hands[i].positions()
                })
                .collect();
            if !changed.is_empty() && a.game.phase == Phase::Player {
                assert!(a.is_busy());
                a.render();
                let hand = after.hands[changed[0]];
                let mut expected = Canvas::default();
                expected.clear(FELT);
                let mut renderer = Renderer::default();
                for (i, c) in hand.cards().iter().rev().take(4).rev().enumerate() {
                    renderer.card(&mut expected, (8 + i as i32 * 56, 36), Some(*c), false)
                }
                for y in 36..116 {
                    for x in 8..231 {
                        assert_eq!(
                            a.frame.pixels[y * 240 + x],
                            expected.pixels[y * 240 + x],
                            "terminal NPC card x{x}, y{y}"
                        )
                    }
                }
                let position = a.game.position;
                a.event(B);
                a.event(G);
                assert_eq!(a.game.phase, Phase::Player);
                assert_eq!(a.game.position, position);
                assert_eq!(a.training.stats().all.decisions, 0);
                let frozen = a.frame.clone();
                a.event(Event::Hold(Key::Grey));
                a.tick(now + 5000);
                a.event(B);
                a.render();
                assert!(a.frame == frozen);
                a.tick(now + 5749);
                assert!(a.is_busy());
                now += 5750;
                a.tick(now);
                for _ in 1..changed.len() {
                    now += 750;
                    a.tick(now)
                }
                assert!(!a.is_busy());
                a.event(B);
                assert_eq!(a.training.stats().all.decisions, 1);
                return;
            }
            for _ in changed {
                now += 750;
                a.tick(now)
            }
        }
    }
    panic!("no terminal draw before player among seeded cases");
}
