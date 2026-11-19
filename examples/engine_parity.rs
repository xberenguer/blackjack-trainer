use stardust_blackjack::{
    cards::Hand,
    engine::{Action, Engine},
    rules::{Profile, Rules},
};
use std::io::{self, BufRead};
fn word(h: &mut u64, n: u64) {
    for b in n.to_le_bytes() {
        *h = (*h ^ b as u64).wrapping_mul(0x100000001b3)
    }
}
fn hand(h: &mut u64, v: &Hand) {
    for n in [
        v.count() as u64,
        v.bet as u64,
        v.returned as u64,
        v.split as u64,
        v.split_aces as u64,
        v.done as u64,
        v.doubled as u64,
        v.outcome as u64,
    ] {
        word(h, n)
    }
    for (c, p) in v.cards().iter().zip(v.positions()) {
        word(h, c.id() as u64);
        word(h, *p as u64)
    }
}
fn digest(e: &Engine) -> u64 {
    let mut h = 0xcbf29ce484222325;
    for n in [
        e.phase as u64,
        e.rules.profile as u64,
        e.rules.decks as u64,
        e.rules.hit_soft_17 as u64,
        e.rules.double_after_split as u64,
        e.rules.late_surrender as u64,
        e.rules.blackjack_numerator as u64,
        e.rules.blackjack_denominator as u64,
        e.rules.minimum as u64,
        e.rules.maximum as u64,
        e.rules.penetration as u64,
        e.position as u64,
        e.marker as u64,
        e.hand_count as u64,
        e.active as u64,
        e.shoe_number as u64,
        e.cut_at as u64,
        e.bankroll as u64,
        e.deposits as u64,
        e.cashouts as u64,
        e.selected_bet as u64,
        e.insurance as u64,
        e.round_stake as u64,
        e.round_return as u64,
        e.round_net as u64,
        e.rng,
        e.rounds as u64,
        e.wins as u64,
        e.losses as u64,
        e.pushes as u64,
        e.hole_visible as u64,
        e.insurance_resolved as u64,
        e.insured as u64,
        e.cut_pending as u64,
    ] {
        word(&mut h, n)
    }
    for v in &e.hands {
        hand(&mut h, v)
    }
    hand(&mut h, &e.dealer);
    word(&mut h, e.diagnostic_shoe_hash());
    word(&mut h, e.message.len() as u64);
    for b in e.message.bytes() {
        word(&mut h, b as u64)
    }
    h
}
fn main() {
    let mut engine: Option<Engine> = None;
    let mut count = 0;
    for (line_number, line) in io::stdin().lock().lines().enumerate() {
        let line = line.unwrap();
        let f: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(f.len(), 5);
        let a: u64 = f[1].parse().unwrap();
        let b: u8 = f[2].parse().unwrap();
        let ok = if f[0] == "N" {
            engine =
                Some(Engine::new(a, Rules::for_profile(Profile::from_id(b).unwrap())).unwrap());
            true
        } else {
            let e = engine.as_mut().unwrap();
            match f[0] {
                "B" => e.buy_in(a as i64),
                "W" => e.set_bet(a as i64),
                "C" => e.cut(a as usize),
                "D" => e.deal(),
                "I" => e.insure(a != 0),
                "A" => e.act(Action::ALL[a as usize]),
                "T" => e.dealer_step(),
                "S" => e.shuffle(),
                "L" => e.leave(),
                _ => panic!("Unknown command"),
            }
        };
        let e = engine.as_ref().unwrap();
        assert!(e.invariant());
        assert_eq!(
            ok,
            f[3].parse::<u8>().unwrap() != 0,
            "Result at event {}",
            line_number
        );
        assert_eq!(
            digest(e),
            f[4].parse::<u64>().unwrap(),
            "State at event {}: {}",
            line_number,
            line
        );
        count += 1;
    }
    println!(
        "PASS fresh Rust engine: {count} identical C++ reference states across 6000 rounds, six profiles and 60 seeded sessions; complete shoe/RNG/hand/position/rules/ledger/outcome/message comparison"
    );
}
