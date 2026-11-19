use stardust_blackjack::{
    cards::{Hand, Outcome},
    counting::Tracker,
    engine::{Action, Engine, Phase},
    rules::{Profile, Rules},
    seats::MAX_EXTRA_SEATS,
};
fn tag(rank: u8) -> i16 {
    match rank {
        2..=6 => 1,
        10..=14 => -1,
        _ => 0,
    }
}
fn held(e: &Engine) -> Vec<u16> {
    let mut positions = Vec::new();
    if !matches!(
        e.phase,
        Phase::Cut | Phase::Ready | Phase::Result | Phase::Closed
    ) {
        for h in e.hands[..e.hand_count].iter().chain(
            e.others[..e.extra_seats as usize]
                .iter()
                .flat_map(|s| s.hands[..s.hand_count].iter()),
        ) {
            if h.outcome == Outcome::Pending {
                positions.extend_from_slice(h.positions());
            }
        }
        positions.extend_from_slice(e.dealer.positions());
    }
    positions
}
fn check(e: &Engine) {
    assert!(
        e.invariant(),
        "phase {:?} inventory {:?}",
        e.phase,
        e.inventory()
    );
    let n = e.inventory();
    assert_eq!(n.unseen + n.held + n.discarded, n.total);
    assert_eq!(n.unseen, e.total_cards() - e.position);
    let p = held(e);
    let mut unique = p.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), p.len(), "held physical card reused");
    assert_eq!(p.len(), n.held);
}
fn finish(e: &mut Engine, split: bool) {
    let mut steps = 0;
    let mut last_seat = 0;
    while !matches!(e.phase, Phase::Result) {
        steps += 1;
        assert!(steps < 500);
        check(e);
        if matches!(e.phase, Phase::Player | Phase::OtherPlayers) {
            assert!(e.active_seat >= last_seat);
            last_seat = e.active_seat;
        }
        match e.phase {
            Phase::Insurance => {
                assert!(e.insure(false));
            }
            Phase::Player => {
                let h = &e.hands[e.active];
                let a = if split && e.legal(Action::Split) {
                    Action::Split
                } else if h.total() < 17 && e.legal(Action::Hit) {
                    Action::Hit
                } else {
                    Action::Stand
                };
                assert!(e.act(a));
            }
            Phase::OtherPlayers => assert!(e.other_step()),
            Phase::Dealer => assert!(e.dealer_step()),
            p => panic!("unexpected{p:?}"),
        }
    }
    check(e);
}
#[test]
fn initial_cards_are_round_robin_and_player_can_choose_any_position() {
    for extras in 0..=MAX_EXTRA_SEATS as u8 {
        for player in 0..=extras {
            let mut e = Engine::new(91, Rules::default()).unwrap();
            assert!(e.buy_in(100000));
            assert!(e.configure_seats(extras, player));
            assert!(e.cut(156));
            assert!(e.deal());
            let seats = extras as usize + 1;
            assert_eq!(
                e.hands[0].positions(),
                &[1 + player as u16, (seats + 2 + player as usize) as u16]
            );
            assert_eq!(
                e.dealer.positions(),
                &[(seats + 1) as u16, (seats * 2 + 2) as u16]
            );
            for place in 0..=extras {
                if place == player {
                    continue;
                }
                let i = if place < player {
                    place as usize
                } else {
                    place as usize - 1
                };
                assert_eq!(
                    e.others[i].hands[0].positions(),
                    &[1 + place as u16, (seats + 2 + place as usize) as u16]
                );
            }
            let mut t = Tracker::default();
            e.observe_exposed(&mut t);
            assert_eq!(
                t.observed_count() as usize,
                seats * 2 + 1 + usize::from(e.hole_visible)
            );
            let mut rc = 0;
            for h in e.hands[..e.hand_count].iter().chain(
                e.others[..e.extra_seats as usize]
                    .iter()
                    .flat_map(|s| s.hands[..s.hand_count].iter()),
            ) {
                rc += h.cards().iter().map(|c| tag(c.rank())).sum::<i16>();
            }
            rc += tag(e.dealer.cards()[0].rank());
            if e.hole_visible {
                rc += tag(e.dealer.cards()[1].rank())
            }
            assert_eq!(t.running_count(), rc);
            check(&e);
        }
    }
}
#[test]
fn betting_configuration_is_gated_and_other_seats_do_not_spend_player_chips() {
    let mut e = Engine::new(33, Rules::default()).unwrap();
    assert!(e.buy_in(100000));
    assert!(!e.configure_seats(7, 0));
    assert!(!e.configure_seats(2, 3));
    assert!(e.configure_seats(3, 2));
    assert!(e.cut(156));
    let bank = e.bankroll;
    assert!(e.deal());
    assert_eq!(e.bankroll, bank - e.selected_bet);
    if e.phase != Phase::Result {
        assert!(!e.configure_seats(0, 0));
    }
    finish(&mut e, false);
    for s in &e.others[..3] {
        assert_eq!(
            s.round_stake,
            s.hands[..s.hand_count].iter().map(|h| h.bet).sum()
        );
        assert_eq!(
            s.round_return,
            s.hands[..s.hand_count].iter().map(|h| h.returned).sum()
        );
        assert_eq!(s.bankroll, s.deposits + s.round_net);
    }
    assert_eq!(
        e.round_stake,
        e.hands[..e.hand_count].iter().map(|h| h.bet).sum::<i64>() + e.insurance
    );
}
#[test]
fn deep_seven_spot_rounds_conserve_cards_and_each_ledger() {
    let mut rounds = 0u64;
    let mut recoveries = 0u64;
    for profile in Profile::ALL {
        for depth in [75, 80, 85, 90] {
            for seed in 1..=8 {
                let mut r = Rules::for_profile(profile);
                r.penetration = depth;
                assert!(r.valid());
                let mut e = Engine::new(seed, r).unwrap();
                assert!(e.buy_in(1000000));
                assert!(e.configure_seats(6, (seed % 7) as u8));
                let mut player_net = 0;
                let mut npc_net = [0i64; 6];
                for _ in 0..200 {
                    if e.needs_shuffle() {
                        assert!(e.shuffle());
                        assert!(e.cut(156));
                    }
                    if e.phase == Phase::Cut {
                        assert!(e.cut(156));
                    }
                    if e.bankroll < e.selected_bet {
                        assert!(e.buy_in(100000));
                    }
                    assert!(e.deal());
                    finish(&mut e, true);
                    player_net += e.round_net;
                    assert_eq!(e.bankroll, e.deposits + player_net);
                    for (i, s) in e.others[..6].iter().enumerate() {
                        npc_net[i] += s.round_net;
                        assert_eq!(s.bankroll, s.deposits + npc_net[i]);
                    }
                    rounds += 1;
                }
                recoveries += e.emergency_shuffles as u64;
            }
        }
    }
    println!(
        "PASS {rounds} seven-seat rounds at75/80/85/90% acrosssixprofiles;{recoveries} naturally encountered in-round recoveries; forced recovery fixtures run separately"
    );
}
#[test]
fn discard_visibility_does_not_disclose_an_unrevealed_hole() {
    let mut checked = 0;
    for seed in 1..=200 {
        let mut e = Engine::new(seed, Rules::default()).unwrap();
        e.buy_in(100000);
        e.configure_seats(2, 0);
        e.cut(156);
        e.deal();
        if e.phase == Phase::Insurance {
            e.insure(false);
        }
        if !matches!(e.phase, Phase::Player) {
            continue;
        }
        let before = e.shoe_number;
        let mut t = Tracker::default();
        e.observe_exposed(&mut t);
        let n = t.observed_count();
        e.observe_exposed(&mut t);
        assert_eq!(t.observed_count(), n);
        assert_eq!(n, 7);
        assert!(!e.hole_visible);
        finish(&mut e, false);
        e.observe_exposed(&mut t);
        assert_eq!(e.shoe_number, before);
        assert_eq!(
            t.observed_count() as usize,
            e.hands[..e.hand_count]
                .iter()
                .map(Hand::count)
                .sum::<usize>()
                + e.others[..2]
                    .iter()
                    .flat_map(|s| s.hands[..s.hand_count].iter())
                    .map(Hand::count)
                    .sum::<usize>()
                + if e.hole_visible { e.dealer.count() } else { 1 }
        );
        checked += 1;
    }
    assert!(checked > 100);
}
