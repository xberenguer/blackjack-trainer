use stardust_blackjack::{
    cards::{Card, Hand, UNKNOWN_POSITION},
    deviations::{self, Count, Threshold},
    engine::Action,
    rules::{Profile, Rules},
};
fn card(rank: u8) -> Card {
    Card::from_id(rank - 2).unwrap()
}
fn hand(a: u8, b: u8) -> Hand {
    let mut h = Hand::default();
    h.add(card(a), UNKNOWN_POSITION);
    h.add(card(b), UNKNOWN_POSITION);
    h
}
fn count(rc: i16, r: Rules) -> Count {
    Count {
        running: rc,
        undealt: 156,
        decks: r.decks,
        observed: r.total_cards() as u16 - 156,
        shoe: 1,
    }
}
fn best(a: u8, b: u8, up: u8, profile: Profile, rc: i16, mask: u8) -> Action {
    let r = Rules::for_profile(profile);
    deviations::recommend(&hand(a, b), card(up), mask, r, count(rc, r))
        .play
        .unwrap()
        .best
}
#[test]
fn strict_rc_zero_exceptions_are_not_rounded_tc_zero() {
    let r = Rules::for_profile(Profile::SixNoSurrender);
    let pos = deviations::recommend(&hand(10, 6), card(10), 3, r, count(1, r));
    assert_eq!(pos.integer_true_count, Some(0));
    assert_eq!(pos.play.unwrap().best, Action::Stand);
    assert_eq!(pos.change.unwrap().threshold, Threshold::PositiveRunning);
    for rc in [0, -1] {
        assert_eq!(best(10, 6, 10, Profile::SixNoSurrender, rc, 3), Action::Hit)
    }
    assert_eq!(best(10, 2, 4, Profile::SixH17, -1, 31), Action::Hit);
    assert_eq!(best(10, 2, 4, Profile::SixH17, 0, 31), Action::Stand);
}
#[test]
fn insurance_has_a_real_plus_three_threshold_and_affordability() {
    let r = Rules::default();
    assert_eq!(deviations::insurance(true, r, count(8, r)), Some(false));
    assert_eq!(deviations::insurance(true, r, count(9, r)), Some(true));
    assert_eq!(deviations::insurance(false, r, count(9, r)), Some(false));
    let mut bad = count(9, r);
    bad.undealt = 0;
    assert_eq!(deviations::insurance(true, r, bad), None);
}
#[test]
fn profile_specific_indices_and_surrender_take_priority() {
    use Action::*;
    use Profile::*;
    assert_eq!(best(10, 6, 9, SixNoSurrender, 11, 3), Hit);
    assert_eq!(best(10, 6, 9, SixNoSurrender, 12, 3), Stand);
    assert_eq!(best(10, 6, 14, SixS17, 18, 3), Hit); // no S17 16vA red index in the named chart
    assert_eq!(best(10, 6, 14, SixH17, 9, 3), Stand);
    assert_eq!(best(4, 6, 14, SixH17, 8, 31), Hit);
    assert_eq!(best(4, 6, 14, SixH17, 9, 31), Double);
    assert_eq!(best(4, 6, 14, SixS17, 9, 31), Hit);
    assert_eq!(best(4, 6, 14, SixS17, 12, 31), Double);
    assert_eq!(best(14, 8, 6, SixH17, -1, 31), Stand);
    assert_eq!(best(14, 8, 6, SixH17, 0, 31), Double);
    assert_eq!(best(14, 8, 6, SixS17, 2, 31), Stand);
    assert_eq!(best(14, 8, 6, SixS17, 3, 31), Double);
    assert_eq!(best(10, 5, 10, SixH17, -1, 31), Hit);
    assert_eq!(best(10, 5, 10, SixH17, 0, 31), Surrender);
    assert_eq!(best(10, 5, 14, SixH17, -5, 31), Surrender);
    assert_eq!(best(10, 5, 14, SixH17, -6, 31), Hit);
    assert_eq!(best(10, 5, 14, SixS17, 5, 31), Hit);
    assert_eq!(best(10, 5, 14, SixS17, 6, 31), Surrender);
    assert_eq!(best(8, 8, 10, SixH17, 18, 31), Split);
    assert_eq!(best(8, 8, 14, SixH17, 18, 31), Surrender);
    assert_eq!(best(10, 10, 4, SixH17, 17, 31), Stand);
    assert_eq!(best(10, 10, 4, SixH17, 18, 31), Split);
    assert_eq!(best(10, 10, 5, SixH17, 15, 31), Split);
    assert_eq!(best(10, 10, 6, SixH17, 12, 31), Split);
}
#[test]
fn indices_never_authorize_an_illegal_action_and_invalid_count_falls_back() {
    let mut checked = 0;
    for profile in Profile::ALL {
        let r = Rules::for_profile(profile);
        for a in 0..13 {
            for b in 0..13 {
                let mut h = Hand::default();
                h.add(Card::from_id(a).unwrap(), 0);
                h.add(Card::from_id(b).unwrap(), 1);
                for up in [0, 1, 2, 3, 4, 5, 6, 7, 8, 12] {
                    for mask in 0..32 {
                        for rc in [-18, -6, -1, 0, 1, 6, 18] {
                            let advice = deviations::recommend(
                                &h,
                                Card::from_id(up).unwrap(),
                                mask,
                                r,
                                count(rc, r),
                            );
                            if let Some(play) = advice.play {
                                assert_ne!(play.acceptable_mask, 0);
                                assert_eq!(play.acceptable_mask & mask, play.acceptable_mask);
                            }
                            checked += 1;
                        }
                    }
                }
            }
        }
    }
    let r = Rules::default();
    let mut bad = count(0, r);
    bad.shoe = 0;
    let h = hand(10, 6);
    let a = deviations::recommend(&h, card(10), 31, r, bad);
    assert!(a.integer_true_count.is_none() && a.change.is_none());
    assert_eq!(a.play.unwrap().best, Action::Surrender);
    println!(
        "PASS Rust Hi-Lo: {checked} legal-mask/count contexts plus primary H17/S17/RC-zero/insurance/pair/surrender boundaries"
    );
}
