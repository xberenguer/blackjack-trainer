use stardust_blackjack::{
    cards::{Card, Hand, Outcome, UNKNOWN_POSITION},
    engine::Action,
    rules::{Profile, Rules},
    strategy,
};
struct Chart {
    hard: [[u8; 10]; 22],
    soft: [[u8; 10]; 22],
    pair: [[u8; 10]; 12],
}
const CSV: [&str; 6] = [
    include_str!("data/strategy-6dh17-das-ls.csv"),
    include_str!("data/strategy-6ds17-das-ls.csv"),
    include_str!("data/strategy-8dh17-das-ls.csv"),
    include_str!("data/strategy-6dh17-das-nols.csv"),
    include_str!("data/strategy-6dh17-nodas-ls.csv"),
    include_str!("data/strategy-6dh17-das-ls-65.csv"),
];
fn chart(text: &str) -> Chart {
    let mut c = Chart {
        hard: [[0; 10]; 22],
        soft: [[0; 10]; 22],
        pair: [[0; 10]; 12],
    };
    let mut rows = 0;
    for line in text
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("type,"))
    {
        let fields: Vec<_> = line.split(',').collect();
        assert_eq!(fields.len(), 12);
        let n: usize = fields[1].parse().unwrap();
        let row = match fields[0] {
            "hard" => &mut c.hard[n],
            "soft" => &mut c.soft[n],
            "pair" => &mut c.pair[n],
            _ => panic!("bad chart row"),
        };
        for i in 0..10 {
            row[i] = fields[i + 2].as_bytes()[0]
        }
        rows += 1
    }
    assert_eq!(rows, 37);
    c
}
fn amount(c: Card) -> u16 {
    match c.id() % 13 + 2 {
        14 => 11,
        10..=13 => 10,
        n => n as u16,
    }
}
fn inspect(h: &Hand) -> (u16, bool, u16) {
    let mut low = 0;
    let mut ace = false;
    for &c in h.cards() {
        let v = amount(c);
        ace |= v == 11;
        low += if v == 11 { 1 } else { v }
    }
    let soft = ace && low + 10 <= 21;
    let pair = if h.count() == 2 && amount(h.cards()[0]) == amount(h.cards()[1]) {
        amount(h.cards()[0])
    } else {
        0
    };
    (low + if soft { 10 } else { 0 }, soft, pair)
}
fn oracle(c: &Chart, h: &Hand, up: Card, mut mask: u8, r: Rules) -> u8 {
    if h.count() < 2 || h.done || h.outcome != Outcome::Pending {
        return 0;
    }
    let (total, soft, pair) = inspect(h);
    if total > 21 || (!h.split && h.count() == 2 && total == 21) {
        return 0;
    }
    mask &= 31;
    if h.count() != 2 {
        mask &= !(4 | 8 | 16)
    }
    if pair == 0 {
        mask &= !8
    }
    if h.split || !r.late_surrender {
        mask &= !16
    }
    if h.split && !r.double_after_split {
        mask &= !4
    }
    if h.split_aces || total >= 21 {
        mask &= 2
    }
    if mask == 0 {
        return 0;
    }
    let col = amount(up) as usize - 2;
    let mut preferred = 0;
    if pair != 0 && mask & 8 != 0 {
        match c.pair[pair as usize][col] {
            b'P' => preferred = b'P',
            b'R' => preferred = if mask & 16 != 0 { b'R' } else { b'P' },
            _ => {}
        }
    }
    if preferred == 0 {
        preferred = if total < 5 {
            b'H'
        } else if soft {
            c.soft[total as usize][col]
        } else {
            c.hard[total as usize][col]
        };
        if preferred == b'R' && mask & 16 == 0 {
            preferred = if total == 17 { b'S' } else { b'H' }
        }
        if preferred == b'D' && mask & 4 == 0 {
            preferred = if soft && total >= 18 { b'S' } else { b'H' }
        }
    }
    let wanted = match preferred {
        b'H' => 1,
        b'S' => 2,
        b'D' => 4,
        b'P' => 8,
        b'R' => 16,
        _ => panic!("Missing independent chart cell {total} {soft} {pair} {col}"),
    };
    if mask & wanted != 0 {
        return wanted;
    }
    for bit in [2, 1, 4, 8, 16] {
        if mask & bit != 0 {
            return bit;
        }
    }
    unreachable!()
}
fn check(c: &Chart, h: &Hand, up: Card, mask: u8, r: Rules, checked: &mut u64) {
    let expected = oracle(c, h, up, mask, r);
    let got = strategy::recommend(h, up, mask, r);
    assert_eq!(
        got.map_or(0, |a| a.acceptable_mask),
        expected,
        "profile={:?} hand={:?} up={:?} mask={}",
        r.profile,
        h,
        up,
        mask
    );
    if let Some(a) = got {
        assert!(a.accepts(a.best));
        assert_eq!(a.acceptable_mask & mask, a.acceptable_mask);
        assert!(!a.explanation.is_empty())
    }
    *checked += 1;
}
fn hand(a: u8, b: u8) -> Hand {
    let mut h = Hand::default();
    h.add(Card::from_id(a).unwrap(), UNKNOWN_POSITION);
    h.add(Card::from_id(b).unwrap(), UNKNOWN_POSITION);
    h
}
#[test]
fn six_profiles_match_independent_published_charts() {
    let mut checked = 0;
    for (id, text) in CSV.iter().enumerate() {
        let c = chart(text);
        let r = Rules::for_profile(Profile::from_id(id as u8).unwrap());
        for a in 0..52 {
            for b in 0..52 {
                let h = hand(a, b);
                for up in [0, 1, 2, 3, 4, 5, 6, 7, 8, 12] {
                    for mask in 0..32 {
                        check(&c, &h, Card::from_id(up).unwrap(), mask, r, &mut checked)
                    }
                }
            }
        }
        for a in 0..13 {
            for b in 0..13 {
                for third in 0..13 {
                    let mut h = hand(a, b);
                    h.add(Card::from_id(third).unwrap(), UNKNOWN_POSITION);
                    for up in [0, 1, 2, 3, 4, 5, 6, 7, 8, 12] {
                        for mask in 0..32 {
                            check(&c, &h, Card::from_id(up).unwrap(), mask, r, &mut checked)
                        }
                    }
                }
            }
        }
        for a in 0..13 {
            for b in 0..13 {
                for aces in [false, true] {
                    let mut h = hand(a, b);
                    h.split = true;
                    h.split_aces = aces;
                    for up in [0, 1, 2, 3, 4, 5, 6, 7, 8, 12] {
                        for mask in 0..32 {
                            check(&c, &h, Card::from_id(up).unwrap(), mask, r, &mut checked)
                        }
                    }
                }
            }
        }
    }
    assert!(!strategy::recommend_insurance());
    assert_eq!(Action::Surrender.bit(), 16);
    println!(
        "PASS Rust basic strategy: {checked} independent chart/mask/hand comparisons across all six profiles"
    );
}
