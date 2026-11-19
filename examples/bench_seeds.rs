use stardust_blackjack::{
    engine::{Engine, Phase},
    rules::Rules,
};
fn main() {
    let mut found = [false; 6];
    for seed in 0..100000u64 {
        let mut e = Engine::new(seed, Rules::default()).unwrap();
        e.buy_in(100000);
        e.cut(156);
        e.deal();
        let i = if e.phase == Phase::Insurance {
            3
        } else if e.phase == Phase::Player
            && e.hands[0].count() == 2
            && e.hands[0].cards()[0].value() == e.hands[0].cards()[1].value()
        {
            if e.hands[0].cards()[0].value() == 2 {
                4
            } else if e.hands[0].cards()[0].value() == 3 {
                5
            } else if e.hands[0].cards()[0].value() == 8 {
                0
            } else if e.hands[0].cards()[0].value() == 11 {
                1
            } else {
                continue;
            }
        } else if e.phase == Phase::Player && e.hands[0].soft() {
            2
        } else {
            continue;
        };
        if !found[i] {
            println!("case={i} seed={seed}");
            found[i] = true
        }
        if found.iter().all(|&b| b) {
            break;
        }
    }
}
