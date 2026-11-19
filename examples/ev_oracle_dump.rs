use stardust_blackjack::{
    cards::{Card, Hand, UNKNOWN_POSITION},
    engine::Action,
    ev::{Decision, Evaluator, Peek, Rules, Status},
};
fn card(rank: u8) -> Card {
    Card::from_id(if rank == 1 { 12 } else { rank - 2 }).unwrap()
}
fn main() {
    let mut evaluator = Evaluator::default();
    for h17 in [false, true] {
        for up in 1..=10 {
            let rules = Rules {
                hit_soft_17: h17,
                ..Rules::default()
            };
            for peek in [Peek::Uncleared, Peek::NoBlackjack] {
                let d = evaluator.dealer(card(up), rules, peek);
                print!("D,{},{},{}", u8::from(h17), up, peek as u8);
                for v in d.outcomes {
                    print!(",{v:.17e}")
                }
                println!(",{:.17e}", d.blackjack);
            }
            for soft in [false, true] {
                for total in if soft { 12..=20 } else { 4..=20 } {
                    let a = if soft {
                        1
                    } else {
                        2.max(total - total.min(10))
                    };
                    let b = if soft { total - 11 } else { total - a };
                    let mut hand = Hand::default();
                    hand.add(card(a), UNKNOWN_POSITION);
                    hand.add(card(b), UNKNOWN_POSITION);
                    let result = evaluator.evaluate(Decision {
                        hand,
                        upcard: card(up),
                        legal_mask: 31 & !Action::Split.bit(),
                        rules,
                        peek: Peek::NoBlackjack,
                    });
                    assert_eq!(result.status, Status::Ok);
                    assert!(result.complete);
                    print!("A,{},{},{},{}", u8::from(h17), up, a, b);
                    for action in [
                        Action::Stand,
                        Action::Hit,
                        Action::Double,
                        Action::Surrender,
                    ] {
                        print!(",{:.17e}", result.value(action).unwrap())
                    }
                    println!();
                }
            }
        }
    }
}
