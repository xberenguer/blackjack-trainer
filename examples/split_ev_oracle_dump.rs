use stardust_blackjack::{
    cards::{Card, Hand, UNKNOWN_POSITION},
    ev::{Decision, Peek, Rules},
    split_ev::{Request, SplitEvaluator, SplitStatus},
};
fn card(rank: u8) -> Card {
    Card::from_id(if rank == 1 { 12 } else { rank - 2 }).unwrap()
}
fn main() {
    let mut evaluator = SplitEvaluator::default();
    for h17 in [false, true] {
        for das in [false, true] {
            for pair in 1..=10 {
                for up in 1..=10 {
                    let mut hand = Hand::default();
                    hand.add(card(pair), UNKNOWN_POSITION);
                    hand.add(card(pair), UNKNOWN_POSITION);
                    let result = evaluator.evaluate(Request {
                        decision: Decision {
                            hand,
                            upcard: card(up),
                            legal_mask: 31,
                            rules: Rules {
                                hit_soft_17: h17,
                                double_after_split: das,
                                ..Rules::default()
                            },
                            peek: Peek::NoBlackjack,
                        },
                        hands_in_round: 1,
                        active_hand: 0,
                        previous_splits: 0,
                        first_decision: true,
                        original_wager: 500,
                        uncommitted: 3500,
                        max_hands: 4,
                        resplit_aces: false,
                        one_card_aces: true,
                        sequential: true,
                    });
                    assert_eq!(result.status, SplitStatus::Ok);
                    print!(
                        "{},{},{},{},{:.17e}",
                        u8::from(h17),
                        u8::from(das),
                        pair,
                        up,
                        result.value.unwrap()
                    );
                    for s in 0..=2 {
                        for k in 1..=4 - s {
                            print!(",{}", result.matching[k][s] as u8)
                        }
                    }
                    println!();
                }
            }
        }
    }
}
