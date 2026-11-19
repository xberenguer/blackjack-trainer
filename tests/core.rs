use stardust_blackjack::{
    cards::{Card, Hand, Outcome, UNKNOWN_POSITION},
    counting::Tracker,
    engine::{Action, Engine, Phase},
    rules::{Profile, Rules},
};
fn card(rank: u8) -> Card {
    Card::from_id(rank - 2).unwrap()
}
#[test]
fn card_types_and_ace_scoring() {
    assert!(Card::from_id(52).is_none());
    assert!(Card::from_id(255).is_none());
    let mut h = Hand::default();
    for rank in [14, 14, 9] {
        assert!(h.add(card(rank), UNKNOWN_POSITION))
    }
    assert_eq!(h.total(), 21);
    assert!(h.soft());
    assert!(!h.natural());
    assert!(h.add(card(10), UNKNOWN_POSITION));
    assert_eq!(h.total(), 21);
    assert!(!h.soft());
    let mut h = Hand::default();
    for _ in 0..22 {
        assert!(h.add(card(14), 0))
    }
    assert!(!h.add(card(2), 0));
    assert_eq!(h.total(), 22);
}
#[test]
fn counting_deduplicates_physical_positions_and_has_no_hidden_input() {
    let mut t = Tracker::default();
    assert!(!t.observe(0, card(2)));
    assert!(t.true_count(100).is_none());
    assert!(t.reset(6, 9));
    for (i, rank) in [2, 3, 6, 7, 8, 9, 10, 11, 12, 13, 14].iter().enumerate() {
        assert!(t.observe(i as u16, card(*rank)));
        assert!(!t.observe(i as u16, card(2)))
    }
    assert_eq!(t.observed_count(), 11);
    assert_eq!(t.running_count(), -2);
    assert_eq!(t.true_count(104), Some(-1.0));
    assert_eq!(t.truncated_index(156), Some(0));
    assert!(!t.observe(312, card(2)));
    assert!(t.true_count(313).is_none());
    assert!(t.true_count(0).is_none());
    t.ensure_shoe(6, 9);
    assert_eq!(t.running_count(), -2);
    t.ensure_shoe(8, 10);
    assert_eq!(t.observed_count(), 0);
    assert!(t.observe(415, card(2)));
    assert_eq!(t.running_count(), 1);
}
#[test]
fn all_profiles_reject_unsafe_depth_and_funding() {
    for profile in Profile::ALL {
        let rules = Rules::for_profile(profile);
        assert!(rules.valid());
        let mut bad = rules;
        bad.penetration = 99;
        assert!(!bad.valid());
        bad = rules;
        bad.minimum = 499;
        assert!(!bad.valid());
        let mut e = Engine::new(17, rules).unwrap();
        assert!(e.invariant());
        assert!(!e.buy_in(499));
        assert!(!e.buy_in(550));
        assert!(!e.set_bet(501));
        assert!(e.buy_in(100_000));
        assert!(e.set_bet(500));
        assert!(!e.cut(51));
        assert!(e.cut(52));
        assert_eq!(e.position, 1);
        assert!(e.deal());
        let mut guard = 0;
        while e.phase == Phase::Result {
            guard += 1;
            assert!(guard < 20);
            assert!(e.deal());
        }
        assert!(!e.buy_in(500));
        assert!(!e.shuffle());
        assert!(!e.leave());
        assert!(e.invariant());
    }
}
#[test]
fn blackjack_awards_use_exact_cents() {
    for (profile, returned) in [(Profile::SixH17, 1250), (Profile::SixSixFive, 1100)] {
        let mut found = false;
        for seed in 1..10000 {
            let mut e = Engine::new(seed, Rules::for_profile(profile)).unwrap();
            e.buy_in(100_000);
            e.set_bet(500);
            e.cut(156);
            e.deal();
            if e.phase == Phase::Insurance {
                e.insure(false);
            }
            if e.phase == Phase::Result && e.hands[0].outcome == Outcome::Blackjack {
                assert_eq!(e.hands[0].returned, returned);
                assert_eq!(e.bankroll, 99_500 + returned);
                assert_eq!(e.net(), returned - 500);
                found = true;
                break;
            }
        }
        assert!(found);
    }
}
#[test]
fn mixed_rounds_conserve_the_ledger_and_card_positions() {
    for profile in Profile::ALL {
        let mut e = Engine::new(117, Rules::for_profile(profile)).unwrap();
        e.buy_in(1_000_000);
        e.set_bet(500);
        let mut policy = 91u64;
        for _ in 0..1500 {
            if e.needs_shuffle() {
                if e.phase != Phase::Cut {
                    assert!(e.shuffle())
                }
                assert!(e.cut(100));
            }
            if e.bankroll < 10_000 {
                assert!(e.buy_in(100_000));
            }
            let before = e.net();
            assert!(e.deal());
            let mut guard = 0;
            while e.phase != Phase::Result {
                guard += 1;
                assert!(guard < 150);
                assert!(e.invariant());
                policy = policy.wrapping_mul(6364136223846793005).wrapping_add(1);
                match e.phase {
                    Phase::Insurance => {
                        e.insure(policy & 1 != 0);
                    }
                    Phase::Player => {
                        let mask = e.legal_mask();
                        let n = mask.count_ones();
                        let chosen = Action::ALL
                            .into_iter()
                            .filter(|a| mask & a.bit() != 0)
                            .nth((policy % n as u64) as usize)
                            .unwrap();
                        let pos = e.position;
                        assert!(e.act(chosen));
                        assert!(e.position >= pos);
                    }
                    Phase::Dealer => {
                        assert!(e.dealer_step());
                    }
                    _ => panic!("unexpected phase"),
                }
            }
            assert!(e.invariant());
            assert_eq!(e.net(), before + e.round_net);
            let mut positions = [false; 416];
            for h in e.hands[..e.hand_count]
                .iter()
                .chain(core::iter::once(&e.dealer))
            {
                for &p in h.positions() {
                    assert!(!positions[p as usize]);
                    positions[p as usize] = true
                }
            }
        }
        assert_eq!(e.rounds, 1500);
        assert_eq!(e.rounds, e.wins + e.losses + e.pushes);
        let net = e.net();
        assert!(e.leave());
        assert_eq!(e.bankroll, 0);
        assert_eq!(e.net(), net);
    }
}
