//! Synthetic public renderer fixture: A-heart dealer upcard, hidden hole,
//! five-clubs/two-clubs player hand. Never reads a device or saved session.
use stardust_blackjack::{
    cards::{Card, Hand},
    counting::Tracker,
    engine::{Engine, Phase},
    rules::Rules,
};
pub fn fixture(affordable: bool, high_count: bool) -> (Engine, Tracker) {
    let mut g = Engine::new(17, Rules::default()).unwrap();
    g.phase = Phase::Insurance;
    g.shoe_number = 1;
    g.position = 156;
    g.bankroll = if affordable { 99_500 } else { 249 };
    g.hand_count = 1;
    g.active = 0;
    g.hands[0] = Hand::EMPTY;
    g.hands[0].bet = 500;
    g.hands[0].add(Card::from_id(16).unwrap(), 152);
    g.hands[0].add(Card::from_id(13).unwrap(), 154);
    g.dealer = Hand::EMPTY;
    g.dealer.add(Card::from_id(38).unwrap(), 153);
    g.dealer.add(Card::from_id(45).unwrap(), 155);
    g.hole_visible = false;
    let mut t = Tracker::default();
    assert!(t.reset(6, g.shoe_number));
    if high_count {
        for p in 0..18 {
            assert!(t.observe(p, Card::from_id((p % 5) as u8).unwrap()));
        }
    }
    (g, t)
}
