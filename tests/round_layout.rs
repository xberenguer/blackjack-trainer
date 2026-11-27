//! Physical composition and information-boundary regressions, using real UI inputs.
use stardust_blackjack::{
    app::App,
    cards::{Card, Hand},
    controls::{Event, Key},
    engine::Phase,
    round::{HEIGHT, RoundCanvas, RoundRenderer, WIDTH, inside_circle},
};
fn run(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
}
fn tap(a: &mut App, key: Key) {
    a.event(Event::Tap(key));
    a.render();
}
fn menu(a: &mut App, label: &str) {
    a.event(Event::Hold(Key::Grey));
    for _ in 0..30 {
        if a.view_name() == "Menu" && a.menu_item() == label {
            tap(a, Key::Blue);
            return;
        }
        tap(a, Key::Grey);
    }
    panic!("Missing menu item {label}");
}
fn select(a: &mut App, label: &str) {
    for _ in 0..30 {
        if a.menu_item() == label {
            tap(a, Key::Blue);
            return;
        }
        tap(a, Key::Grey);
    }
    panic!("Missing item {label} in {}", a.view_name());
}
fn gold(f: &RoundCanvas, y: usize, h: usize) -> usize {
    f.pixels[y * WIDTH..(y + h) * WIDTH]
        .iter()
        .filter(|p| **p == 0xffe0)
        .count()
}
fn round_safe(f: &RoundCanvas) {
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if !inside_circle(x as i32, y as i32) {
                assert_eq!(f.pixels[y * WIDTH + x], 0);
            }
        }
    }
    assert_eq!(f.pixels[233 * WIDTH], f.background());
    assert_eq!(f.pixels[233 * WIDTH + 465], f.background());
}
#[test]
fn native_titles_and_menu_rows_have_visible_glyphs() {
    run(|| {
        let mut a = Box::new(App::new(334));
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        r.render(&a, &mut f);
        assert!(
            gold(&f, 55, 48) > 1000,
            "Cut heading must contain real font pixels"
        );
        round_safe(&f);
        a.event(Event::Hold(Key::Grey));
        a.render();
        r.render(&a, &mut f);
        assert!(gold(&f, 60, 42) > 500, "Menu title vanished");
        assert!(
            gold(&f, 130, 35) > 400,
            "Selected row must include readable words, not just a selection bar"
        );
        assert_eq!(r.info.visible_rows, 6);
        round_safe(&f);
        for _ in 0..10 {
            tap(&mut a, Key::Grey);
        }
        r.render(&a, &mut f);
        assert_eq!(r.info.visible_rows, 6);
        assert!(gold(&f, 60, 42) > 500);
        round_safe(&f);
    });
}
#[test]
fn bankroll_and_hidden_hole_survive_native_composition() {
    run(|| {
        let mut a = Box::new(App::new(334));
        tap(&mut a, Key::Blue);
        tap(&mut a, Key::Blue);
        for t in 1..=30 {
            a.tick(t * 400);
        }
        a.game.phase = Phase::Player;
        a.game.hole_visible = false;
        let mut dealer = Hand::EMPTY;
        dealer.add(Card::from_id(0).unwrap(), 0);
        dealer.add(Card::from_id(12).unwrap(), 1);
        a.game.dealer = dealer;
        a.render();
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        r.render(&a, &mut f);
        assert!(r.info.native_table && r.info.hidden_hole);
        assert!(r.info.player_cards >= 2);
        assert!(gold(&f, 370, 45) > 500, "Bankroll must be legible");
        round_safe(&f);
        let original = f.pixels.to_vec();
        let mut other = Hand::EMPTY;
        other.add(Card::from_id(0).unwrap(), 0);
        other.add(Card::from_id(25).unwrap(), 1);
        a.game.dealer = other;
        r.render(&a, &mut f);
        assert_eq!(
            original.as_slice(),
            f.pixels.as_slice(),
            "An unseen hole card must not affect the round renderer"
        );
        a.game.hole_visible = true;
        r.render(&a, &mut f);
        assert_ne!(
            original.as_slice(),
            f.pixels.as_slice(),
            "Revealed hole must change pixels"
        );
        assert!(!r.info.hidden_hole);
    });
}
#[test]
fn volume_has_large_numeric_content_and_physical_button_hints() {
    run(|| {
        let mut a = Box::new(App::new(334));
        menu(&mut a, "Settings");
        tap(&mut a, Key::Grey);
        tap(&mut a, Key::Blue);
        tap(&mut a, Key::Grey);
        tap(&mut a, Key::Blue);
        assert_eq!(a.view_name(), "AudioVolume");
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        r.render(&a, &mut f);
        assert!(
            gold(&f, 150, 100) > 1500,
            "Volume percentage must use the native large type"
        );
        assert_eq!(r.info.content_bands, 0);
        assert!(
            f.pixels[327 * WIDTH..350 * WIDTH]
                .iter()
                .filter(|p| **p == 0xffff)
                .count()
                > 200,
            "Physical LEFT/RIGHT hints vanished"
        );
        round_safe(&f);
    });
}
#[test]
fn legacy_resume_composition_preserves_circle_and_visible_contents() {
    run(|| {
        let mut a = Box::new(App::new(37));
        menu(&mut a, "Table");
        select(&mut a, "Current rules");
        a.render();
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        r.render_legacy(&a.frame, &mut f);
        assert!(r.info.content_bands > 0);
        assert!(gold(&f, 50, 65) > 100);
        round_safe(&f);
    });
}
#[test]
fn guide_insurance_keeps_count_advice_and_explicit_choices() {
    run(|| {
        use stardust_blackjack::training::{Mode, Policy};
        let mut a = Box::new(App::new(334));
        tap(&mut a, Key::Blue);
        tap(&mut a, Key::Blue);
        for t in 1..=30 {
            a.tick(t * 400);
        }
        a.game.phase = Phase::Insurance;
        a.game.hole_visible = false;
        let mut dealer = Hand::EMPTY;
        dealer.add(Card::from_id(12).unwrap(), 0);
        dealer.add(Card::from_id(0).unwrap(), 1);
        a.game.dealer = dealer;
        a.training.mode = Mode::Guide;
        a.policy = Policy::HiLo;
        a.render();
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        r.render(&a, &mut f);
        assert!(r.info.guide && r.info.count_visible && r.info.hidden_hole);
        assert_eq!(r.info.player_cards, 2);
        assert!(
            f.pixels[323 * WIDTH + 113..323 * WIDTH + 225]
                .iter()
                .all(|p| *p == 0xffe0),
            "Decline choice must be present and visibly selected"
        );
        assert!(
            (347..365)
                .flat_map(|y| f.pixels[y * WIDTH + 50..y * WIDTH + 416].iter())
                .filter(|p| **p == 0xffe0)
                .count()
                > 100,
            "Guide insurance advice must remain visible below the centered cards"
        );
        round_safe(&f);
        a.training.mode = Mode::Off;
        a.render();
        r.render(&a, &mut f);
        assert!(
            !r.info.guide && !r.info.count_visible,
            "Guide-default count must not leak into normal mode"
        );
    });
}
#[test]
fn long_hit_hands_and_split_keep_large_visible_card_fans() {
    run(|| {
        let mut a = Box::new(App::new(334));
        tap(&mut a, Key::Blue);
        tap(&mut a, Key::Blue);
        for t in 1..=30 {
            a.tick(t * 400);
        }
        a.game.phase = Phase::Player;
        a.game.hole_visible = false;
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        for n in 2..=9 {
            let mut h = Hand::EMPTY;
            h.bet = 500;
            for i in 0..n {
                h.add(Card::from_id(12).unwrap(), i as u16);
            }
            a.game.hands[0] = h;
            a.render();
            r.render(&a, &mut f);
            assert_eq!(r.info.player_cards, n as u8);
            assert!(
                f.pixels[260 * WIDTH..375 * WIDTH]
                    .iter()
                    .filter(|p| **p == 0xffff)
                    .count()
                    > 1800,
                "Large hand fan lost its card faces"
            );
            round_safe(&f);
        }
    });
}
fn player_pixels(f: &RoundCanvas) -> Vec<u16> {
    f.pixels[153 * WIDTH..313 * WIDTH].to_vec()
}
fn first_dealer_pixels(f: &RoundCanvas) -> Vec<u16> {
    (55..143)
        .flat_map(|y| f.pixels[y * WIDTH + 169..y * WIDTH + 229].iter().copied())
        .collect()
}
#[test]
fn stand_dealer_and_result_keep_exact_player_pixels_and_fixed_geometry() {
    run(|| {
        let mut a = Box::new(App::new(334));
        tap(&mut a, Key::Blue);
        tap(&mut a, Key::Blue);
        for t in 1..=30 {
            a.tick(t * 400);
        }
        assert_eq!(a.game.phase, Phase::Player);
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        a.render();
        r.render(&a, &mut f);
        let player = player_pixels(&f);
        let dealer = first_dealer_pixels(&f);
        let geometry = (r.info.player_rect, r.info.dealer_rect);
        assert_eq!(geometry, ((119, 153, 110, 160), (169, 55, 60, 88)));
        tap(&mut a, Key::Blue);
        assert!(matches!(a.game.phase, Phase::Dealer | Phase::Result));
        r.render(&a, &mut f);
        assert!(
            r.info.native_table,
            "Stand must not select a landscape fallback"
        );
        assert_eq!(r.info.content_bands, 0);
        assert_eq!(player, player_pixels(&f));
        assert_eq!(dealer, first_dealer_pixels(&f));
        assert_eq!(geometry, (r.info.player_rect, r.info.dealer_rect));
        for t in 31..=60 {
            a.tick(t * 400);
            a.render();
            r.render(&a, &mut f);
            assert!(r.info.native_table);
            assert_eq!(r.info.content_bands, 0);
            assert_eq!(
                player,
                player_pixels(&f),
                "Dealer phase must leave player cards at identical pixels"
            );
            assert_eq!(geometry.0, r.info.player_rect);
            assert_eq!(
                (
                    r.info.dealer_rect.1,
                    r.info.dealer_rect.2,
                    r.info.dealer_rect.3
                ),
                (55, 60, 88)
            );
            let total = r.info.dealer_rect.2 as i32
                + r.info.dealer_step as i32 * (r.info.dealer_cards.saturating_sub(1)) as i32;
            assert!(
                (r.info.dealer_rect.0 as i32
                    - (WIDTH as i32 - r.info.dealer_rect.0 as i32 - total))
                    .abs()
                    <= 1
            );
            if a.game.phase == Phase::Result {
                break;
            }
        }
        assert_eq!(a.game.phase, Phase::Result);
        round_safe(&f);
    });
}
#[test]
fn player_fan_recenters_horizontally_on_deal_and_hit() {
    run(|| {
        let mut a = Box::new(App::new(334));
        tap(&mut a, Key::Blue);
        tap(&mut a, Key::Blue);
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        for (now, player, dealer) in [
            (0, 0, 0),
            (120, 1, 0),
            (240, 1, 1),
            (360, 2, 1),
            (480, 2, 2),
            (600, 2, 2),
        ] {
            a.tick(now);
            a.render();
            r.render(&a, &mut f);
            assert!(r.info.native_table);
            assert_eq!((r.info.player_cards, r.info.dealer_cards), (player, dealer));
            let left = if player < 2 { 178 } else { 119 };
            assert_eq!(r.info.player_rect, (left, 153, 110, 160));
            assert_eq!(
                r.info.dealer_rect,
                (if dealer < 2 { 203 } else { 169 }, 55, 60, 88)
            );
        }
        assert_eq!(a.game.phase, Phase::Player);
        tap(&mut a, Key::Grey);
        for t in 2..=8 {
            a.tick(t * 400);
            r.render(&a, &mut f);
            if r.info.player_cards == 3 {
                break;
            }
        }
        assert_eq!(r.info.player_cards, 3);
        assert_eq!(r.info.player_rect, (60, 153, 110, 160));
        let row = &f.pixels[233 * WIDTH..234 * WIDTH];
        for gap in [170..178, 288..296] {
            assert!(row[gap].iter().all(|&p| p == f.background()));
        }
        a.game.phase = Phase::Player;
        for n in 2..=9 {
            let mut h = Hand::EMPTY;
            h.bet = 500;
            for i in 0..n {
                h.add(Card::from_id(12).unwrap(), i as u16);
            }
            a.game.hands[0] = h;
            a.render();
            r.render(&a, &mut f);
            assert_eq!(r.info.player_cards, n as u8);
            let (left, y, width, height) = r.info.player_rect;
            assert_eq!((y, width, height), (153, 110, 160));
            let row = &f.pixels[233 * WIDTH..234 * WIDTH];
            let occupied = |p: &u16| *p != f.background();
            let first = row.iter().position(occupied).unwrap();
            let last = row.iter().rposition(occupied).unwrap();
            assert!(first.abs_diff(WIDTH - 1 - last) <= 1);
            for x in [i32::from(left), last as i32] {
                assert!(inside_circle(x, 153) && inside_circle(x, 312));
            }
            round_safe(&f);
        }
    });
}
#[test]
fn guide_toggle_and_other_players_phase_use_the_same_native_anchors() {
    run(|| {
        use stardust_blackjack::training::Mode;
        let mut a = Box::new(App::new(334));
        tap(&mut a, Key::Blue);
        tap(&mut a, Key::Blue);
        for t in 1..=30 {
            a.tick(t * 400);
        }
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        a.render();
        r.render(&a, &mut f);
        let player = player_pixels(&f);
        let dealer = first_dealer_pixels(&f);
        a.training.mode = Mode::Guide;
        a.render();
        r.render(&a, &mut f);
        assert_eq!(player, player_pixels(&f));
        assert_eq!(dealer, first_dealer_pixels(&f));
        a.game.extra_seats = 1;
        a.game.player_seat = 0;
        a.game.active_seat = 1;
        a.game.phase = Phase::OtherPlayers;
        a.render();
        r.render(&a, &mut f);
        assert!(r.info.native_table);
        assert_eq!(r.info.content_bands, 0);
        assert_eq!(player, player_pixels(&f));
        assert_eq!(dealer, first_dealer_pixels(&f));
    });
}
#[test]
fn queued_tablemate_faces_stay_native_and_delay_dealer_hole_and_counts() {
    run(|| {
        use stardust_blackjack::training::Mode;
        for seed in 1..=40 {
            let mut a = Box::new(App::new(seed));
            assert!(a.game.configure_seats(2, 0));
            tap(&mut a, Key::Blue);
            tap(&mut a, Key::Blue);
            for t in 1..=20 {
                a.tick(t * 400);
            }
            if a.game.phase == Phase::Insurance {
                tap(&mut a, Key::Blue);
            }
            if a.game.phase != Phase::Player {
                continue;
            }
            a.training.mode = Mode::Guide;
            a.render();
            let mut f = Box::new(RoundCanvas::EMPTY);
            let mut r = RoundRenderer::new();
            r.render(&a, &mut f);
            let player = player_pixels(&f);
            tap(&mut a, Key::Blue);
            for t in 21..=70 {
                a.tick(t * 400);
                a.render();
                r.render(&a, &mut f);
                assert!(r.info.native_table);
                assert_eq!(r.info.content_bands, 0);
                assert_eq!(
                    player,
                    player_pixels(&f),
                    "Other seats must not replace or move your hand"
                );
                if matches!(a.game.phase, Phase::Dealer | Phase::Result) && r.info.seat_cards > 0 {
                    assert!(
                        r.info.hidden_hole,
                        "A queued face precedes the dealer's hole reveal"
                    );
                    assert!(
                        !r.info.count_visible && !r.info.guide,
                        "Count/advice must not include faces still waiting for public presentation"
                    );
                    round_safe(&f);
                    return;
                }
                if a.game.phase == Phase::Result && !a.is_busy() {
                    break;
                }
            }
        }
        panic!("No bounded seeded terminal tablemate reveal reached Dealer/Result");
    });
}
#[test]
fn oled_background_is_black_by_default_and_four_choices_reach_native_and_legacy_frames() {
    run(|| {
        use stardust_blackjack::round::background_color;
        assert_eq!(RoundCanvas::EMPTY.background(), 0);
        let mut a = Box::new(App::new(334));
        let original = a.frame.pixels.to_vec();
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        for index in 0..4 {
            a.background = index;
            a.render();
            assert_eq!(
                original.as_slice(),
                a.frame.pixels.as_slice(),
                "Round background must not recolor the approved Stick core frame"
            );
            r.render(&a, &mut f);
            assert_eq!(f.background(), background_color(index));
            assert_eq!(f.pixels[5 * WIDTH + 233], background_color(index));
            round_safe(&f);
            f.set_background(index);
            r.render_legacy(&a.frame, &mut f);
            assert_eq!(f.pixels[233 * WIDTH], background_color(index));
            round_safe(&f);
        }
        assert_eq!(
            background_color(1),
            stardust_blackjack::canvas::rgb(11, 0, 107)
        );
        assert_eq!(
            background_color(2),
            stardust_blackjack::canvas::rgb(0, 107, 11)
        );
        assert_eq!(
            background_color(3),
            stardust_blackjack::canvas::rgb(107, 11, 0)
        );
    });
}
#[test]
fn menu_six_full_rows_peek_then_advance_one_row_without_a_side_scrollbar() {
    run(|| {
        let mut a = Box::new(App::new(334));
        a.round_display = true;
        a.event(Event::Hold(Key::Grey));
        a.render();
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        r.render(&a, &mut f);
        assert_eq!(
            (r.info.visible_rows, r.info.menu_top, r.info.peek_row),
            (6, 0, true)
        );
        for _ in 0..5 {
            tap(&mut a, Key::Grey);
        }
        r.render(&a, &mut f);
        assert_eq!(r.info.selected_row, 5);
        assert_eq!(
            r.info.menu_top, 0,
            "Last full row stays in the initial viewport"
        );
        let ink = f.pixels[359 * WIDTH..377 * WIDTH]
            .iter()
            .filter(|p| **p != f.background())
            .count();
        assert!(
            ink > 100,
            "Partial next row must advertise the seventh item"
        );
        tap(&mut a, Key::Grey);
        r.render(&a, &mut f);
        assert_eq!(
            (r.info.selected_row, r.info.menu_top, r.info.peek_row),
            (6, 1, false)
        );
        for y in 135..347 {
            if inside_circle(428, y as i32) {
                assert_eq!(
                    f.pixels[y * WIDTH + 428],
                    f.background(),
                    "The removed edge scrollbar must not return"
                );
            }
        }
        round_safe(&f);
    });
}
#[test]
fn round_display_background_row_cycles_the_same_four_persisted_choices() {
    run(|| {
        use stardust_blackjack::round::background_color;
        let mut a = Box::new(App::new(334));
        a.round_display = true;
        menu(&mut a, "Settings");
        tap(&mut a, Key::Blue);
        assert_eq!(a.view_name(), "Display");
        tap(&mut a, Key::Grey);
        tap(&mut a, Key::Grey);
        assert_eq!(a.menu_item(), "Colors");
        tap(&mut a, Key::Blue);
        assert_eq!(a.view_name(), "CardBackPreview");
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        for expected in [1, 2, 3, 0] {
            tap(&mut a, Key::Blue);
            assert_eq!(a.menu_item(), "Background");
            assert_eq!(a.background, expected);
            r.render(&a, &mut f);
            assert_eq!(f.background(), background_color(expected));
            round_safe(&f);
        }
        tap(&mut a, Key::Grey);
        assert_eq!(a.menu_item(), "Card back");
        for back in [1, 2, 3, 0] {
            tap(&mut a, Key::Blue);
            assert_eq!(a.card_back, back);
        }
        tap(&mut a, Key::Grey);
        tap(&mut a, Key::Blue);
        assert_eq!((a.view_name(), a.focus()), ("Display", 4));
        tap(&mut a, Key::Grey);
        assert_eq!(a.focus(), 3);
        assert_eq!(a.menu_item(), "Back");
    });
}
#[test]
fn long_guide_surrender_word_fits_its_reserved_lower_slot() {
    run(|| {
        use stardust_blackjack::{
            render::{text_height, text_width},
            training::{Mode, Policy},
        };
        let mut a = Box::new(App::new(334));
        tap(&mut a, Key::Blue);
        tap(&mut a, Key::Blue);
        for t in 1..=30 {
            a.tick(t * 400);
        }
        a.game.phase = Phase::Player;
        a.game.hole_visible = false;
        a.training.mode = Mode::Guide;
        a.policy = Policy::Basic;
        let mut hand = Hand::EMPTY;
        hand.bet = 500;
        hand.add(Card::from_id(8).unwrap(), 0);
        hand.add(Card::from_id(4).unwrap(), 1);
        a.game.hands[0] = hand;
        let mut dealer = Hand::EMPTY;
        dealer.add(Card::from_id(8).unwrap(), 2);
        dealer.add(Card::from_id(0).unwrap(), 3);
        a.game.dealer = dealer;
        a.render();
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        r.render(&a, &mut f);
        assert!(r.info.guide);
        assert!(
            r.info.guide_action_px >= 20,
            "SURRENDER must remain large enough to read"
        );
        let px = r.info.guide_action_px as i32;
        let base = px.min(24) as u8;
        let w = (text_width("SURRENDER", base, true) * px + base as i32 - 1) / base as i32;
        let h = (text_height("SURRENDER", base, true) * px + base as i32 - 1) / base as i32;
        for (x, y) in [
            (233 - w / 2, 323),
            (233 + (w + 1) / 2, 323),
            (233 - w / 2, 323 + h),
            (233 + (w + 1) / 2, 323 + h),
        ] {
            assert!(
                inside_circle(x, y),
                "Long off-centre advice must not be cropped by the round OLED"
            );
        }
        let ink = (323..347)
            .flat_map(|y| f.pixels[y * WIDTH + 50..y * WIDTH + 416].iter())
            .filter(|p| **p == 0xffe0)
            .count();
        assert!(
            ink > 200,
            "Surrender advice must contain visible complete lettering"
        );
        round_safe(&f);
    });
}
fn overlap(a: (i16, i16, u16, u16), b: (i16, i16, u16, u16)) -> bool {
    (a.0 as i32) < b.0 as i32 + b.2 as i32
        && (b.0 as i32) < a.0 as i32 + a.2 as i32
        && (a.1 as i32) < b.1 as i32 + b.3 as i32
        && (b.1 as i32) < a.1 as i32 + a.3 as i32
}

#[test]
fn ready_hint_has_its_own_space_and_disappears_on_dealing() {
    run(|| {
        let mut a = Box::new(App::new(334));
        a.round_display = true;
        tap(&mut a, Key::Blue);
        a.tick(600);
        a.render();
        assert_eq!(a.view_name(), "Table");
        assert_eq!(a.game.phase, Phase::Ready);
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        r.render(&a, &mut f);
        let info = r.info;
        assert!(info.ready_hint_rect.2 > 0);
        assert!(!overlap(info.ready_hint_rect, info.bankroll_rect));
        assert!(!overlap(info.ready_hint_rect, info.wager_rect));
        assert!(!overlap(info.bankroll_rect, info.wager_rect));
        assert!(
            gold(&f, 380, 43) > 500,
            "Ready bankroll must remain visible"
        );
        tap(&mut a, Key::Blue);
        r.render(&a, &mut f);
        assert_eq!(
            r.info.ready_hint_rect,
            (0, 0, 0, 0),
            "Ready-only hint must vanish once a hand starts"
        );
        round_safe(&f);
    });
}
#[test]
fn saved_card_back_colours_preserve_faces_and_the_cream_design() {
    run(|| {
        use stardust_blackjack::round::card_back_color;
        let mut a = Box::new(App::new(334));
        tap(&mut a, Key::Blue);
        tap(&mut a, Key::Blue);
        for t in 1..=30 {
            a.tick(t * 400);
        }
        a.render();
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        r.render(&a, &mut f);
        let faces = player_pixels(&f);
        let up = first_dealer_pixels(&f);
        let mut backs = Vec::new();
        for choice in 0..3 {
            a.card_back = choice;
            r.render(&a, &mut f);
            assert_eq!(
                faces,
                player_pixels(&f),
                "Changing a back must not recolour card faces"
            );
            assert_eq!(up, first_dealer_pixels(&f));
            let hole: Vec<_> = (55..143)
                .flat_map(|y| f.pixels[y * WIDTH + 237..y * WIDTH + 297].iter().copied())
                .collect();
            backs.push(hole);
            round_safe(&f);
        }
        assert_ne!(backs[0], backs[1]);
        assert_ne!(backs[0], backs[2]);
        assert_ne!(backs[1], backs[2]);
        let cream = stardust_blackjack::canvas::rgb(248, 240, 220);
        for choice in 0..3 {
            assert_eq!(card_back_color(cream, choice), cream);
            assert_eq!(card_back_color(0xffff, choice), 0xffff);
        }
        let ink = stardust_blackjack::canvas::rgb(184, 152, 64);
        assert_eq!(card_back_color(ink, 0), ink);
        assert_ne!(card_back_color(ink, 1), ink);
        assert_ne!(card_back_color(ink, 2), ink);
    });
}
#[test]
fn dealer_fans_recentre_with_equal_margins_and_keep_all_visible_cards_inside_circle() {
    run(|| {
        let mut a = Box::new(App::new(334));
        tap(&mut a, Key::Blue);
        tap(&mut a, Key::Blue);
        for t in 1..=30 {
            a.tick(t * 400);
        }
        a.game.phase = Phase::Dealer;
        a.game.hole_visible = true;
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        for n in 2usize..=12 {
            let mut h = Hand::EMPTY;
            for i in 0..n {
                h.add(Card::from_id(12).unwrap(), i as u16);
            }
            a.game.dealer = h;
            a.render();
            r.render(&a, &mut f);
            let (x, y, w, height) = r.info.dealer_rect;
            let count = r.info.dealer_cards as usize;
            let right =
                x as i32 + w as i32 + r.info.dealer_step as i32 * count.saturating_sub(1) as i32;
            assert!((x as i32 - (WIDTH as i32 - right)).abs() <= 1);
            assert_eq!((y, w, height), (55, 60, 88));
            assert_eq!(r.info.dealer_omitted, n.saturating_sub(8) as u8);
            for (xx, yy) in [
                (x as i32, 55),
                (right - 1, 55),
                (x as i32, 142),
                (right - 1, 142),
            ] {
                assert!(inside_circle(xx, yy));
            }
            round_safe(&f);
        }
    });
}

#[test]
fn grouped_menus_share_readable_rows_and_restore_the_parent_focus() {
    run(|| {
        let paths: &[(&[&str], u8)] = &[
            (&["Training"], 6),
            (&["Training", "Live counts"], 4),
            (&["Training", "Practice"], 4),
            (&["Training", "Progress"], 6),
            (&["Training", "Advice & EV"], 4),
            (&["Settings"], 5),
            (&["Settings", "Sound"], 5),
            (&["Settings", "Sound", "Categories"], 4),
            (&["Settings", "Sound", "Test sounds"], 4),
            (&["Table"], 5),
            (&["Session"], 5),
            (&["Session", "Bankroll"], 3),
            (&["Help"], 3),
        ];
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        for &(path, rows) in paths {
            let mut a = Box::new(App::new(334));
            a.round_display = true;
            a.event(Event::Hold(Key::Grey));
            for label in path {
                select(&mut a, label);
            }
            r.render(&a, &mut f);
            assert!(
                r.info.shared_menu,
                "{} bypassed shared menu definition",
                a.view_name()
            );
            assert!(!r.info.confirmation);
            assert_eq!(r.info.visible_rows, rows, "{}", a.view_name());
            assert!(gold(&f, 60, 42) > 200, "{} heading missing", a.view_name());
            assert!(
                gold(&f, 130, 35) > 120,
                "{} selected row missing",
                a.view_name()
            );
            round_safe(&f);
        }
        let mut a = Box::new(App::new(334));
        menu(&mut a, "Settings");
        select(&mut a, "Sound");
        select(&mut a, "Categories");
        a.event(Event::Hold(Key::Grey));
        a.render();
        assert_eq!((a.view_name(), a.menu_item()), ("Sound", "Categories"));
        a.event(Event::Hold(Key::Grey));
        a.render();
        assert_eq!((a.view_name(), a.menu_item()), ("Settings", "Sound"));
        a.event(Event::Hold(Key::Grey));
        a.render();
        assert_eq!((a.view_name(), a.menu_item()), ("Menu", "Settings"));
    });
}
#[test]
fn destructive_confirmations_default_to_cancel_and_keep_explanations_above_choices() {
    run(|| {
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        for label in ["New session", "Reset training stats"] {
            let mut a = Box::new(App::new(334));
            let parent = if label == "New session" {
                menu(&mut a, "Session");
                "SessionMenu"
            } else {
                menu(&mut a, "Training");
                select(&mut a, "Progress");
                "ProgressMenu"
            };
            select(&mut a, label);
            assert_eq!(a.menu_item(), "Cancel");
            r.render(&a, &mut f);
            assert!(r.info.shared_menu && r.info.confirmation);
            assert_eq!((r.info.visible_rows, r.info.selected_row), (2, 0));
            assert!(
                f.pixels[128 * WIDTH..290 * WIDTH]
                    .iter()
                    .filter(|p| **p == 0xffff)
                    .count()
                    > 1000,
                "{label}: keep/reset explanation must remain legible"
            );
            assert!(gold(&f, 306, 32) > 100, "Cancel must visibly be selected");
            assert!(
                f.pixels[344 * WIDTH..376 * WIDTH]
                    .iter()
                    .filter(|p| **p == 0xffff)
                    .count()
                    > 100,
                "{label}: separate confirmation choice is missing"
            );
            round_safe(&f);
            tap(&mut a, Key::Blue);
            assert_eq!((a.view_name(), a.menu_item()), (parent, label));
        }
    });
}
#[test]
fn read_only_mode_help_and_session_overview_are_native_paragraph_screens() {
    run(|| {
        let mut f = Box::new(RoundCanvas::EMPTY);
        let mut r = RoundRenderer::new();
        for path in [["Help", "Training modes"], ["Session", "Overview"]] {
            let mut a = Box::new(App::new(334));
            menu(&mut a, path[0]);
            select(&mut a, path[1]);
            for _ in 0..4 {
                r.render(&a, &mut f);
                assert!(r.info.shared_menu && !r.info.confirmation);
                assert_eq!(r.info.visible_rows, 0);
                assert!(
                    f.pixels[128 * WIDTH..290 * WIDTH]
                        .iter()
                        .filter(|p| **p == 0xffff)
                        .count()
                        > 500
                );
                assert!(
                    f.pixels[400 * WIDTH..420 * WIDTH]
                        .iter()
                        .any(|p| *p != f.background()),
                    "Read-only page/back hint must be visible"
                );
                round_safe(&f);
                tap(&mut a, Key::Grey);
            }
        }
    });
}
