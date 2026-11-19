use crate::{
    cards::{Card, Hand, Outcome},
    counting::Tracker,
    rules::{MAX_BANK, MAX_BUY, MAX_CARDS, MAX_HANDS, MIN_BET, Money, Rules},
    seats::{Inventory, MAX_EXTRA_SEATS, OtherSeat, Zone},
    strategy,
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Phase {
    Cut,
    Ready,
    Insurance,
    Player,
    Dealer,
    Result,
    Closed,
    OtherPlayers,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Action {
    Hit,
    Stand,
    Double,
    Split,
    Surrender,
}
impl Action {
    pub const ALL: [Self; 5] = [
        Self::Hit,
        Self::Stand,
        Self::Double,
        Self::Split,
        Self::Surrender,
    ];
    pub const fn bit(self) -> u8 {
        1 << (self as u8)
    }
}
#[derive(Clone)]
pub struct Engine {
    pub phase: Phase,
    pub rules: Rules,
    shoe: [Card; MAX_CARDS],
    physical: [u16; MAX_CARDS],
    tracking: [u16; MAX_CARDS],
    by_position: [u16; MAX_CARDS],
    zones: [Zone; MAX_CARDS],
    exposed: [bool; MAX_CARDS],
    draw_cursor: usize,
    draw_len: usize,
    force_shuffle: bool,
    player_held: [bool; MAX_HANDS],
    dealer_held: bool,
    pub extra_seats: u8,
    pub player_seat: u8,
    pub active_seat: u8,
    pub others: [OtherSeat; MAX_EXTRA_SEATS],
    pub emergency_shuffles: u32,
    pub hands: [Hand; MAX_HANDS],
    pub dealer: Hand,
    pub position: usize,
    pub marker: usize,
    pub hand_count: usize,
    pub active: usize,
    pub shoe_number: u32,
    pub cut_at: usize,
    pub bankroll: Money,
    pub deposits: Money,
    pub cashouts: Money,
    pub selected_bet: Money,
    pub insurance: Money,
    pub round_stake: Money,
    pub round_return: Money,
    pub round_net: Money,
    pub rng: u64,
    pub rounds: u32,
    pub wins: u32,
    pub losses: u32,
    pub pushes: u32,
    pub hole_visible: bool,
    pub insurance_resolved: bool,
    pub insured: bool,
    pub cut_pending: bool,
    pub message: &'static str,
}
impl Engine {
    /// Valid static storage image; call App::boot before using its unshuffled shoe.
    pub(crate) const EMPTY: Self = Self {
        phase: Phase::Cut,
        rules: Rules::for_profile(crate::rules::Profile::SixH17),
        shoe: [Card::TWO; MAX_CARDS],
        physical: [0; MAX_CARDS],
        tracking: [0; MAX_CARDS],
        by_position: [0; MAX_CARDS],
        zones: [Zone::Unseen; MAX_CARDS],
        exposed: [false; MAX_CARDS],
        draw_cursor: 0,
        draw_len: 0,
        force_shuffle: false,
        player_held: [false; MAX_HANDS],
        dealer_held: false,
        extra_seats: 0,
        player_seat: 0,
        active_seat: 0,
        others: [OtherSeat::EMPTY; MAX_EXTRA_SEATS],
        emergency_shuffles: 0,
        hands: [Hand::EMPTY; MAX_HANDS],
        dealer: Hand::EMPTY,
        position: 0,
        marker: Rules::for_profile(crate::rules::Profile::SixH17).cut_marker(),
        hand_count: 0,
        active: 0,
        shoe_number: 0,
        cut_at: Rules::for_profile(crate::rules::Profile::SixH17).total_cards() / 2,
        bankroll: 0,
        deposits: 0,
        cashouts: 0,
        selected_bet: 500,
        insurance: 0,
        round_stake: 0,
        round_return: 0,
        round_net: 0,
        rng: 0,
        rounds: 0,
        wins: 0,
        losses: 0,
        pushes: 0,
        hole_visible: false,
        insurance_resolved: false,
        insured: false,
        cut_pending: true,
        message: "Buy chips, then cut the shoe",
    };

    pub fn new(seed: u64, rules: Rules) -> Option<Self> {
        if !rules.valid() {
            return None;
        }
        let mut e = Self {
            phase: Phase::Cut,
            rules,
            shoe: [Card::TWO; MAX_CARDS],
            physical: [0; MAX_CARDS],
            tracking: [0; MAX_CARDS],
            by_position: [0; MAX_CARDS],
            zones: [Zone::Unseen; MAX_CARDS],
            exposed: [false; MAX_CARDS],
            draw_cursor: 0,
            draw_len: 0,
            force_shuffle: false,
            player_held: [false; MAX_HANDS],
            dealer_held: false,
            extra_seats: 0,
            player_seat: 0,
            active_seat: 0,
            others: [OtherSeat::EMPTY; MAX_EXTRA_SEATS],
            emergency_shuffles: 0,
            hands: [Hand::default(); MAX_HANDS],
            dealer: Hand::default(),
            position: 0,
            marker: rules.cut_marker(),
            hand_count: 0,
            active: 0,
            shoe_number: 0,
            cut_at: rules.total_cards() / 2,
            bankroll: 0,
            deposits: 0,
            cashouts: 0,
            selected_bet: 2500i64.clamp(rules.minimum, rules.maximum),
            insurance: 0,
            round_stake: 0,
            round_return: 0,
            round_net: 0,
            rng: seed,
            rounds: 0,
            wins: 0,
            losses: 0,
            pushes: 0,
            hole_visible: false,
            insurance_resolved: false,
            insured: false,
            cut_pending: true,
            message: "Buy chips, then cut the shoe",
        };
        e.shuffle();
        Some(e)
    }
    pub fn total_cards(&self) -> usize {
        self.rules.total_cards()
    }
    pub fn between_rounds(&self) -> bool {
        matches!(self.phase, Phase::Cut | Phase::Ready | Phase::Result)
    }
    pub fn configure(&mut self, rules: Rules) -> bool {
        if !self.between_rounds() || !rules.valid() {
            return false;
        }
        self.rules = rules;
        self.selected_bet = (self.selected_bet / 500 * 500).clamp(rules.minimum, rules.maximum);
        self.hands = [Hand::default(); MAX_HANDS];
        self.dealer = Hand::default();
        self.hand_count = 0;
        self.active = 0;
        self.insurance = 0;
        self.round_stake = 0;
        self.round_return = 0;
        self.round_net = 0;
        self.insurance_resolved = false;
        self.insured = false;
        self.hole_visible = false;
        self.shuffle()
    }
    pub fn random64(&mut self) -> u64 {
        self.rng = self.rng.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    pub fn bounded(&mut self, n: u32) -> Option<u32> {
        if n == 0 {
            return None;
        }
        let threshold = 0u64.wrapping_sub(n as u64) % n as u64;
        loop {
            let r = self.random64();
            if r >= threshold {
                return Some((r % n as u64) as u32);
            }
        }
    }
    pub fn buy_in(&mut self, amount: Money) -> bool {
        if !self.between_rounds()
            || !(MIN_BET..=MAX_BUY).contains(&amount)
            || amount % 100 != 0
            || self.bankroll > MAX_BANK - amount
            || self.deposits > MAX_BANK - amount
        {
            return false;
        }
        self.bankroll += amount;
        self.deposits += amount;
        self.message = "Cash exchanged for chips";
        true
    }
    pub fn set_bet(&mut self, bet: Money) -> bool {
        if !self.between_rounds()
            || bet < self.rules.minimum
            || bet > self.rules.maximum
            || bet % 500 != 0
        {
            return false;
        }
        self.selected_bet = bet;
        true
    }
    pub fn needs_shuffle(&self) -> bool {
        self.cut_pending
            || self.position >= self.marker
            || self.force_shuffle
            || (self.draw_len - self.draw_cursor) < 2 * (self.extra_seats as usize + 2)
    }
    pub fn shuffle(&mut self) -> bool {
        if !self.between_rounds() || !self.rules.valid() {
            return false;
        }
        let len = self.total_cards();
        for i in 0..len {
            self.shoe[i] = Card::from_id((i % 52) as u8).unwrap();
            self.physical[i] = i as u16;
        }
        for i in (1..len).rev() {
            let j = self.bounded((i + 1) as u32).unwrap() as usize;
            self.shoe.swap(i, j);
            self.physical.swap(i, j);
        }
        self.draw_cursor = 0;
        self.draw_len = len;
        self.zones = [Zone::Unseen; MAX_CARDS];
        self.exposed = [false; MAX_CARDS];
        self.player_held = [false; MAX_HANDS];
        self.dealer_held = false;
        self.force_shuffle = false;
        for seat in &mut self.others {
            seat.clear_round()
        }
        self.map_tracking();
        self.position = 0;
        self.marker = self.rules.cut_marker();
        self.cut_at = len / 2;
        self.cut_pending = true;
        self.phase = Phase::Cut;
        self.hands = [Hand::default(); MAX_HANDS];
        self.hand_count = 0;
        self.active = 0;
        self.dealer = Hand::default();
        self.hole_visible = false;
        self.shoe_number += 1;
        self.message = "Place the cut card in the stack";
        true
    }
    pub fn cut(&mut self, at: usize) -> bool {
        if self.phase != Phase::Cut
            || !self.rules.valid()
            || at < 52
            || at > self.total_cards() - 52
        {
            return false;
        }
        let len = self.total_cards();
        self.shoe[..len].rotate_left(at);
        self.physical[..len].rotate_left(at);
        self.map_tracking();
        self.draw_cursor = 1;
        self.zones[self.physical[0] as usize] = Zone::Discard;
        self.cut_at = at;
        self.position = 1;
        self.marker = self.rules.cut_marker();
        self.cut_pending = false;
        self.phase = Phase::Ready;
        self.message = "Dealer burns one card. Place your bet";
        true
    }
    pub fn can_draw(&self, n: usize) -> bool {
        self.draw_cursor <= self.draw_len && n <= self.draw_len - self.draw_cursor
    }
    fn pop(&mut self) -> Option<(Card, u16)> {
        if !self.can_draw(1) && !self.recover_discards() {
            self.message = "No safe cards. Round paused";
            return None;
        }
        let at = self.draw_cursor;
        let physical = self.physical[at] as usize;
        if self.zones[physical] != Zone::Unseen {
            return None;
        }
        self.draw_cursor += 1;
        self.position = self.total_cards() - (self.draw_len - self.draw_cursor);
        self.zones[physical] = Zone::Held;
        Some((self.shoe[at], self.tracking[physical]))
    }
    fn draw_player(&mut self, index: usize) -> bool {
        if index >= self.hand_count || self.hands[index].count() == 22 {
            return false;
        }
        if let Some((c, p)) = self.pop() {
            self.exposed[self.by_position[p as usize] as usize] = true;
            self.hands[index].add(c, p)
        } else {
            false
        }
    }
    fn draw_dealer(&mut self) -> bool {
        if self.dealer.count() == 22 {
            return false;
        }
        if let Some((c, p)) = self.pop() {
            if self.dealer.count() == 0 || self.hole_visible {
                self.exposed[self.by_position[p as usize] as usize] = true;
            }
            self.dealer.add(c, p)
        } else {
            false
        }
    }
    pub fn deal(&mut self) -> bool {
        if !matches!(self.phase, Phase::Ready | Phase::Result)
            || self.needs_shuffle()
            || !self.rules.valid()
            || self.bankroll < self.selected_bet
            || self.selected_bet < self.rules.minimum
            || self.selected_bet > self.rules.maximum
            || self.selected_bet % 500 != 0
            || !self.can_draw(if self.extra_seats == 0 && self.rules.penetration <= 75 {
                self.rules.maximum_round_cards()
            } else {
                2 * (self.extra_seats as usize + 2)
            })
        {
            return false;
        }
        self.hands = [Hand::default(); MAX_HANDS];
        self.dealer = Hand::default();
        self.hand_count = 1;
        self.active = 0;
        self.hole_visible = false;
        self.insurance = 0;
        self.insurance_resolved = false;
        self.insured = false;
        self.round_stake = self.selected_bet;
        self.round_return = 0;
        self.round_net = 0;
        self.player_held = [false; MAX_HANDS];
        self.player_held[0] = true;
        self.dealer_held = true;
        self.hands[0].bet = self.selected_bet;
        self.bankroll -= self.selected_bet;
        if self.extra_seats == 0 {
            self.draw_player(0);
            self.draw_dealer();
            self.draw_player(0);
            self.draw_dealer();
        } else {
            for i in 0..self.extra_seats as usize {
                let seat = &mut self.others[i];
                seat.clear_round();
                if seat.bankroll < self.rules.minimum {
                    seat.bankroll += 100_000;
                    seat.deposits += 100_000
                }
                seat.hand_count = 1;
                seat.hands[0].bet = self.rules.minimum;
                seat.round_stake = self.rules.minimum;
                seat.bankroll -= self.rules.minimum;
                seat.held[0] = true;
            }
            for _ in 0..2 {
                for place in 0..=self.extra_seats {
                    if place == self.player_seat {
                        self.draw_player(0);
                    } else {
                        let i = self.other_index(place);
                        let mut seat = self.others[i];
                        self.draw_other(&mut seat, 0);
                        self.others[i] = seat;
                    }
                }
                self.draw_dealer();
            }
        }
        if self.dealer.cards()[0].rank() == 14 {
            self.phase = Phase::Insurance;
            self.message = "Dealer shows ace. Insurance?"
        } else {
            self.peek()
        }
        true
    }
    pub fn insure(&mut self, yes: bool) -> bool {
        if self.phase != Phase::Insurance {
            return false;
        }
        let stake = self.hands[0].bet / 2;
        if yes && self.bankroll < stake {
            return false;
        }
        if yes {
            self.insured = true;
            self.insurance = stake;
            self.bankroll -= stake;
            self.round_stake += stake
        }
        self.peek();
        true
    }
    fn peek(&mut self) {
        if self.extra_seats > 0 {
            self.peek_table();
            return;
        }
        let blackjack = self.dealer.natural();
        if !self.insurance_resolved {
            if self.insured && blackjack {
                let returned = self.insurance * 3;
                self.bankroll += returned;
                self.round_return += returned
            }
            self.insurance_resolved = true
        }
        if blackjack {
            self.hole_visible = true;
            self.expose_dealer();
            for h in &mut self.hands[..self.hand_count] {
                h.done = true;
                h.outcome = if h.natural() {
                    Outcome::Push
                } else {
                    Outcome::Loss
                };
                h.returned = if h.natural() { h.bet } else { 0 }
            }
            self.finish_round();
            self.message = "Dealer blackjack";
            return;
        }
        if self.hands[0].natural() {
            let h = &mut self.hands[0];
            h.done = true;
            h.outcome = Outcome::Blackjack;
            h.returned = h.bet
                * (self.rules.blackjack_denominator + self.rules.blackjack_numerator) as Money
                / self.rules.blackjack_denominator as Money;
            self.finish_round();
            self.message = if self.rules.blackjack_numerator == 6 {
                "Blackjack pays 6:5"
            } else {
                "Blackjack pays 3:2"
            };
            return;
        }
        self.phase = Phase::Player;
        self.message = "Choose your action";
    }
    pub fn legal(&self, action: Action) -> bool {
        if self.phase != Phase::Player || self.active >= self.hand_count {
            return false;
        }
        let h = &self.hands[self.active];
        if h.done {
            return false;
        }
        match action {
            Action::Hit => h.total() < 21 && !h.split_aces,
            Action::Stand => true,
            Action::Double => {
                h.count() == 2
                    && !h.split_aces
                    && (!h.split || self.rules.double_after_split)
                    && self.bankroll >= h.bet
            }
            Action::Split => {
                h.count() == 2
                    && !h.split_aces
                    && self.hand_count < MAX_HANDS
                    && h.cards()[0].value() == h.cards()[1].value()
                    && self.bankroll >= h.bet
            }
            Action::Surrender => self.rules.late_surrender && !h.split && h.count() == 2,
        }
    }
    pub fn legal_mask(&self) -> u8 {
        Action::ALL
            .iter()
            .fold(0, |m, &a| if self.legal(a) { m | a.bit() } else { m })
    }
    pub fn act(&mut self, action: Action) -> bool {
        if !self.legal(action) {
            return false;
        }
        let pending = self.hands[self.active + 1..self.hand_count]
            .iter()
            .filter(|h| h.count() == 1)
            .count();
        let required = pending
            + match action {
                Action::Split => 2,
                Action::Hit | Action::Double => 1,
                _ => 0,
            };
        if !self.can_draw_or_recover(required) {
            return false;
        }
        let at = self.active;
        match action {
            Action::Hit => {
                self.draw_player(at);
                if self.hands[at].total() >= 21 {
                    self.hands[at].done = true;
                    if self.hands[at].total() > 21 {
                        self.hands[at].outcome = Outcome::Bust;
                        self.release_player(at);
                    }
                    self.advance()
                }
            }
            Action::Stand => {
                self.hands[at].done = true;
                self.advance()
            }
            Action::Double => {
                let bet = self.hands[at].bet;
                self.bankroll -= bet;
                self.round_stake += bet;
                self.hands[at].bet *= 2;
                self.hands[at].doubled = true;
                self.draw_player(at);
                self.hands[at].done = true;
                if self.hands[at].total() > 21 {
                    self.hands[at].outcome = Outcome::Bust;
                    self.release_player(at);
                }
                self.advance()
            }
            Action::Surrender => {
                let h = &mut self.hands[at];
                h.done = true;
                h.outcome = Outcome::Surrender;
                h.returned = h.bet / 2;
                self.release_player(at);
                self.advance()
            }
            Action::Split => {
                let h = self.hands[at];
                let second = h.cards()[1];
                let p = h.positions()[1];
                let aces = second.rank() == 14;
                for i in (at + 2..=self.hand_count).rev() {
                    self.hands[i] = self.hands[i - 1];
                    self.player_held[i] = self.player_held[i - 1];
                }
                self.hand_count += 1;
                let mut next = Hand::default();
                next.bet = h.bet;
                next.split = true;
                next.split_aces = aces;
                next.add(second, p);
                self.hands[at + 1] = next;
                self.player_held[at + 1] = true;
                self.bankroll -= h.bet;
                self.round_stake += h.bet;
                self.hands[at].keep_first();
                self.hands[at].split = true;
                self.hands[at].split_aces = aces;
                self.draw_player(at);
                if aces || self.hands[at].total() == 21 {
                    self.hands[at].done = true;
                    self.advance()
                }
            }
        }
        true
    }
    fn advance(&mut self) {
        while self.active < self.hand_count {
            if !self.hands[self.active].done {
                break;
            }
            self.active += 1;
            if self.active < self.hand_count && self.hands[self.active].count() == 1 {
                if !self.draw_player(self.active) {
                    return;
                }
                if self.hands[self.active].split_aces || self.hands[self.active].total() == 21 {
                    self.hands[self.active].done = true
                }
            }
        }
        if self.active < self.hand_count {
            self.message = "Next hand";
            return;
        }
        if self.extra_seats > 0 {
            self.advance_table();
            return;
        }
        if self.hands[..self.hand_count]
            .iter()
            .all(|h| h.outcome != Outcome::Pending)
        {
            self.finish_round();
            return;
        }
        self.hole_visible = true;
        self.expose_dealer();
        self.phase = Phase::Dealer;
        self.message = "Dealer plays";
    }
    pub fn dealer_step(&mut self) -> bool {
        if self.phase != Phase::Dealer {
            return false;
        }
        let total = self.dealer.total();
        if total < 17 || (total == 17 && self.dealer.soft() && self.rules.hit_soft_17) {
            return self.draw_dealer();
        }
        for h in &mut self.hands[..self.hand_count] {
            if h.outcome != Outcome::Pending {
                continue;
            }
            let player = h.total();
            if player > 21 {
                h.outcome = Outcome::Bust
            } else if total > 21 || player > total {
                h.outcome = Outcome::Win;
                h.returned = h.bet * 2
            } else if player == total {
                h.outcome = Outcome::Push;
                h.returned = h.bet
            } else {
                h.outcome = Outcome::Loss
            }
        }
        if self.extra_seats > 0 {
            self.settle_others(total);
        }
        self.finish_round();
        true
    }
    fn finish_round(&mut self) {
        for i in 0..self.extra_seats as usize {
            let mut seat = self.others[i];
            for h in &seat.hands[..seat.hand_count] {
                seat.bankroll += h.returned;
                seat.round_return += h.returned;
            }
            seat.round_net = seat.round_return - seat.round_stake;
            seat.rounds += 1;
            for j in 0..seat.hand_count {
                self.release_other(&mut seat, j)
            }
            self.others[i] = seat;
        }
        for i in 0..self.hand_count {
            self.release_player(i)
        }
        if self.dealer_held {
            let h = self.dealer;
            self.discard_hand(&h);
            self.dealer_held = false;
        }

        for h in &self.hands[..self.hand_count] {
            self.bankroll += h.returned;
            self.round_return += h.returned
        }
        self.round_net = self.round_return - self.round_stake;
        self.rounds += 1;
        if self.round_net > 0 {
            self.wins += 1
        } else if self.round_net < 0 {
            self.losses += 1
        } else {
            self.pushes += 1
        }
        self.phase = Phase::Result;
        self.message = if self.round_net > 0 {
            "Winning round"
        } else if self.round_net < 0 {
            "Losing round"
        } else {
            "Round pushes"
        };
    }
    pub fn leave(&mut self) -> bool {
        if !self.between_rounds() {
            return false;
        }
        self.cashouts += self.bankroll;
        self.bankroll = 0;
        self.phase = Phase::Closed;
        self.message = "Colored up. Session complete";
        true
    }
    pub fn committed(&self) -> Money {
        if matches!(
            self.phase,
            Phase::Player | Phase::Dealer | Phase::Insurance | Phase::OtherPlayers
        ) {
            self.round_stake - self.round_return
        } else {
            0
        }
    }
    pub fn net(&self) -> Money {
        self.bankroll + self.cashouts + self.committed() - self.deposits
    }
    pub fn invariant(&self) -> bool {
        self.rules.valid()
            && self.pool_invariant()
            && self.extra_seats as usize <= MAX_EXTRA_SEATS
            && self.player_seat <= self.extra_seats
            && self.position <= self.total_cards()
            && self.hand_count <= MAX_HANDS
            && self.bankroll >= 0
            && self.bankroll <= i64::MAX / 4
            && self.deposits >= 0
            && self.cashouts >= 0
            && (!matches!(self.phase, Phase::Player | Phase::Insurance)
                || self.active < self.hand_count)
            && self.hands[..self.hand_count]
                .iter()
                .all(|h| h.bet >= 0 && h.count() <= 22)
    }
    /// Deterministic diagnostics hash; contains no card identity accessor for UI/advice.
    #[cfg(feature = "validation")]
    pub fn diagnostic_shoe_hash(&self) -> u64 {
        self.shoe[..self.total_cards()]
            .iter()
            .fold(0xcbf29ce484222325, |h, c| {
                (h ^ c.id() as u64).wrapping_mul(0x100000001b3)
            })
    }
}
impl Engine {
    /// Configure optional simulated seats between rounds. Seat0 is dealer-left;
    /// the controlled seat can occupy any place. Defaults remain one spot.
    pub fn configure_seats(&mut self, extra: u8, player: u8) -> bool {
        if !self.between_rounds() || extra as usize > MAX_EXTRA_SEATS || player > extra {
            return false;
        }
        if self.extra_seats == extra && self.player_seat == player {
            return true;
        }
        self.extra_seats = extra;
        self.player_seat = player;
        self.active_seat = player;
        self.others = [OtherSeat::EMPTY; MAX_EXTRA_SEATS];
        self.shuffle()
    }
    pub fn seat_count(&self) -> usize {
        self.extra_seats as usize + 1
    }
    fn other_index(&self, place: u8) -> usize {
        if place < self.player_seat {
            place as usize
        } else {
            place as usize - 1
        }
    }
    fn map_tracking(&mut self) {
        for at in 0..self.total_cards() {
            let id = self.physical[at] as usize;
            self.tracking[id] = at as u16;
            self.by_position[at] = id as u16
        }
    }
    pub fn inventory(&self) -> Inventory {
        let mut n = Inventory {
            total: self.total_cards(),
            ..Inventory::default()
        };
        for zone in &self.zones[..n.total] {
            match zone {
                Zone::Unseen => n.unseen += 1,
                Zone::Held => n.held += 1,
                Zone::Discard => n.discarded += 1,
            }
        }
        n
    }
    /// Exposed-face source for the count tracker, including all tablemates.
    /// Old discards lose their exposure flags when mixed back into the shoe;
    /// exposed held cards remain known and the dealer hole remains gated.
    pub fn observe_exposed(&self, tracker: &mut Tracker) {
        tracker.ensure_shoe(self.rules.decks, self.shoe_number);
        for id in 0..self.total_cards() {
            if self.exposed[id] {
                tracker.observe(self.tracking[id], Card::from_id((id % 52) as u8).unwrap());
            }
        }
    }
    pub fn uses_extended_shoe(&self) -> bool {
        self.extra_seats > 0 || self.emergency_shuffles > 0
    }
    fn expose_dealer(&mut self) {
        for &p in self.dealer.positions() {
            self.exposed[self.by_position[p as usize] as usize] = true
        }
    }
    fn discard_hand(&mut self, h: &Hand) {
        for &p in h.positions() {
            let id = self.by_position[p as usize] as usize;
            debug_assert_eq!(self.zones[id], Zone::Held);
            self.zones[id] = Zone::Discard;
        }
    }
    fn release_player(&mut self, i: usize) {
        if self.player_held[i] {
            let h = self.hands[i];
            self.discard_hand(&h);
            self.player_held[i] = false;
        }
    }
    fn release_other(&mut self, seat: &mut OtherSeat, i: usize) {
        if seat.held[i] {
            self.discard_hand(&seat.hands[i]);
            seat.held[i] = false;
        }
    }
    fn can_draw_or_recover(&self, n: usize) -> bool {
        self.can_draw(n)
            || (!self.between_rounds()
                && self.inventory().discarded > n.saturating_sub(self.draw_len - self.draw_cursor))
    }
    /// Named NJ-style simulation choice: shuffle only the discard rack, burn
    /// one unseen card and finish the current round. Force a full shuffle next.
    /// Held physical cards never enter this recovery pool.
    fn recover_discards(&mut self) -> bool {
        if self.between_rounds() || self.can_draw(1) {
            return false;
        }
        let available = self.inventory().discarded;
        if available < 2 {
            return false;
        }
        let mut n = 0;
        for id in 0..self.total_cards() {
            self.exposed[id] &= self.zones[id] == Zone::Held;
            if self.zones[id] == Zone::Discard {
                self.physical[n] = id as u16;
                self.shoe[n] = Card::from_id((id % 52) as u8).unwrap();
                self.zones[id] = Zone::Unseen;
                n += 1;
            }
        }
        for i in (1..n).rev() {
            let j = self.bounded((i + 1) as u32).unwrap() as usize;
            self.physical.swap(i, j);
            self.shoe.swap(i, j);
        }
        self.draw_len = n;
        self.draw_cursor = 1;
        self.zones[self.physical[0] as usize] = Zone::Discard;
        self.position = self.total_cards() - (n - 1);
        self.shoe_number += 1;
        self.emergency_shuffles += 1;
        self.force_shuffle = true;
        self.message = "Discard rack shuffled; unseen burn; round continues";
        true
    }
    fn draw_other(&mut self, seat: &mut OtherSeat, index: usize) -> bool {
        if index >= seat.hand_count || seat.hands[index].count() >= 22 {
            return false;
        }
        let Some((c, p)) = self.pop() else {
            return false;
        };
        self.exposed[self.by_position[p as usize] as usize] = true;
        seat.hands[index].add(c, p)
    }
    fn peek_table(&mut self) {
        let blackjack = self.dealer.natural();
        if !self.insurance_resolved {
            if self.insured && blackjack {
                let returned = self.insurance * 3;
                self.bankroll += returned;
                self.round_return += returned;
            }
            self.insurance_resolved = true;
        }
        if blackjack {
            self.hole_visible = true;
            self.expose_dealer();
            for h in &mut self.hands[..self.hand_count] {
                h.done = true;
                h.outcome = if h.natural() {
                    Outcome::Push
                } else {
                    Outcome::Loss
                };
                h.returned = if h.natural() { h.bet } else { 0 };
            }
            for seat in &mut self.others[..self.extra_seats as usize] {
                for h in &mut seat.hands[..seat.hand_count] {
                    h.done = true;
                    h.outcome = if h.natural() {
                        Outcome::Push
                    } else {
                        Outcome::Loss
                    };
                    h.returned = if h.natural() { h.bet } else { 0 };
                }
            }
            self.finish_round();
            self.message = "Dealer blackjack";
            return;
        }
        if self.hands[0].natural() {
            let h = &mut self.hands[0];
            h.done = true;
            h.outcome = Outcome::Blackjack;
            h.returned = h.bet
                * (self.rules.blackjack_denominator + self.rules.blackjack_numerator) as Money
                / self.rules.blackjack_denominator as Money;
            self.release_player(0);
        }
        for i in 0..self.extra_seats as usize {
            let mut seat = self.others[i];
            if seat.hands[0].natural() {
                let h = &mut seat.hands[0];
                h.done = true;
                h.outcome = Outcome::Blackjack;
                h.returned = h.bet
                    * (self.rules.blackjack_denominator + self.rules.blackjack_numerator) as Money
                    / self.rules.blackjack_denominator as Money;
                self.release_other(&mut seat, 0);
            }
            self.others[i] = seat;
        }
        self.advance_table();
    }
    fn advance_table(&mut self) {
        for place in 0..=self.extra_seats {
            if place == self.player_seat {
                if let Some(active) = self.hands[..self.hand_count].iter().position(|h| !h.done) {
                    self.active = active;
                    self.active_seat = place;
                    self.phase = Phase::Player;
                    self.message = "Choose your action";
                    return;
                }
            } else {
                let i = self.other_index(place);
                if self.others[i].hands[..self.others[i].hand_count]
                    .iter()
                    .any(|h| !h.done)
                {
                    self.active_seat = place;
                    self.phase = Phase::OtherPlayers;
                    self.message = "Tablemate plays basic strategy";
                    return;
                }
            }
        }
        let pending = self.hands[..self.hand_count]
            .iter()
            .any(|h| h.outcome == Outcome::Pending)
            || self.others[..self.extra_seats as usize].iter().any(|s| {
                s.hands[..s.hand_count]
                    .iter()
                    .any(|h| h.outcome == Outcome::Pending)
            });
        if pending {
            self.hole_visible = true;
            self.expose_dealer();
            self.phase = Phase::Dealer;
            self.message = "Dealer plays"
        } else {
            self.finish_round()
        }
    }
    fn other_legal(&self, seat: &OtherSeat) -> u8 {
        if seat.active >= seat.hand_count {
            return 0;
        }
        let h = &seat.hands[seat.active];
        if h.done {
            return 0;
        }
        let mut mask = Action::Stand.bit();
        if h.total() < 21 && !h.split_aces {
            mask |= Action::Hit.bit()
        }
        if h.count() == 2
            && !h.split_aces
            && (!h.split || self.rules.double_after_split)
            && seat.bankroll >= h.bet
        {
            mask |= Action::Double.bit()
        }
        if h.count() == 2
            && !h.split_aces
            && seat.hand_count < MAX_HANDS
            && h.cards()[0].value() == h.cards()[1].value()
            && seat.bankroll >= h.bet
        {
            mask |= Action::Split.bit()
        }
        if self.rules.late_surrender && !h.split && h.count() == 2 {
            mask |= Action::Surrender.bit()
        }
        mask
    }
    fn advance_other(&mut self, seat: &mut OtherSeat) {
        while seat.active < seat.hand_count {
            if !seat.hands[seat.active].done {
                break;
            }
            seat.active += 1;
            if seat.active < seat.hand_count && seat.hands[seat.active].count() == 1 {
                self.draw_other(seat, seat.active);
                if seat.hands[seat.active].split_aces || seat.hands[seat.active].total() == 21 {
                    seat.hands[seat.active].done = true;
                }
            }
        }
    }
    /// Advance exactly one optional tablemate decision. The controller may
    /// animate/display each exposed card before calling this again.
    pub fn other_step(&mut self) -> bool {
        if self.phase != Phase::OtherPlayers || self.active_seat == self.player_seat {
            return false;
        }
        let i = self.other_index(self.active_seat);
        let mut seat = self.others[i];
        let at = seat.active;
        let Some(play) = strategy::recommend(
            &seat.hands[at],
            self.dealer.cards()[0],
            self.other_legal(&seat),
            self.rules,
        ) else {
            return false;
        };
        let action = play.best;
        let pending = seat.hands[at + 1..seat.hand_count]
            .iter()
            .filter(|h| h.count() == 1)
            .count();
        let required = pending
            + match action {
                Action::Split => 2,
                Action::Hit | Action::Double => 1,
                _ => 0,
            };
        if !self.can_draw_or_recover(required) {
            return false;
        }
        match action {
            Action::Hit => {
                self.draw_other(&mut seat, at);
                if seat.hands[at].total() >= 21 {
                    seat.hands[at].done = true;
                    if seat.hands[at].total() > 21 {
                        seat.hands[at].outcome = Outcome::Bust;
                        self.release_other(&mut seat, at);
                    }
                    self.advance_other(&mut seat);
                }
            }
            Action::Stand => {
                seat.hands[at].done = true;
                self.advance_other(&mut seat)
            }
            Action::Double => {
                let bet = seat.hands[at].bet;
                seat.bankroll -= bet;
                seat.round_stake += bet;
                seat.hands[at].bet *= 2;
                seat.hands[at].doubled = true;
                self.draw_other(&mut seat, at);
                seat.hands[at].done = true;
                if seat.hands[at].total() > 21 {
                    seat.hands[at].outcome = Outcome::Bust;
                    self.release_other(&mut seat, at);
                }
                self.advance_other(&mut seat)
            }
            Action::Surrender => {
                let h = &mut seat.hands[at];
                h.done = true;
                h.outcome = Outcome::Surrender;
                h.returned = h.bet / 2;
                self.release_other(&mut seat, at);
                self.advance_other(&mut seat)
            }
            Action::Split => {
                let h = seat.hands[at];
                let second = h.cards()[1];
                let position = h.positions()[1];
                let aces = second.rank() == 14;
                for j in (at + 2..=seat.hand_count).rev() {
                    seat.hands[j] = seat.hands[j - 1];
                    seat.held[j] = seat.held[j - 1];
                }
                seat.hand_count += 1;
                let mut next = Hand::EMPTY;
                next.bet = h.bet;
                next.split = true;
                next.split_aces = aces;
                next.add(second, position);
                seat.hands[at + 1] = next;
                seat.held[at + 1] = true;
                seat.bankroll -= h.bet;
                seat.round_stake += h.bet;
                seat.hands[at].keep_first();
                seat.hands[at].split = true;
                seat.hands[at].split_aces = aces;
                self.draw_other(&mut seat, at);
                if aces || seat.hands[at].total() == 21 {
                    seat.hands[at].done = true;
                    self.advance_other(&mut seat);
                }
            }
        }
        self.others[i] = seat;
        self.advance_table();
        true
    }
    fn settle_others(&mut self, total: u16) {
        for seat in &mut self.others[..self.extra_seats as usize] {
            for h in &mut seat.hands[..seat.hand_count] {
                if h.outcome != Outcome::Pending {
                    continue;
                }
                if h.total() > 21 {
                    h.outcome = Outcome::Bust
                } else if total > 21 || h.total() > total {
                    h.outcome = Outcome::Win;
                    h.returned = h.bet * 2
                } else if h.total() == total {
                    h.outcome = Outcome::Push;
                    h.returned = h.bet
                } else {
                    h.outcome = Outcome::Loss
                }
            }
        }
    }
    fn pool_invariant(&self) -> bool {
        let n = self.total_cards();
        if self.draw_cursor > self.draw_len
            || self.draw_len > n
            || self.position != n - (self.draw_len - self.draw_cursor)
        {
            return false;
        }
        let mut queued = [false; MAX_CARDS];
        for at in self.draw_cursor..self.draw_len {
            let id = self.physical[at] as usize;
            if id >= n
                || queued[id]
                || self.zones[id] != Zone::Unseen
                || self.shoe[at].id() as usize != id % 52
            {
                return false;
            }
            queued[id] = true;
        }
        let mut tracking = [false; MAX_CARDS];
        for (id, &in_queue) in queued[..n].iter().enumerate() {
            if (self.zones[id] == Zone::Unseen) != in_queue {
                return false;
            }
            let p = self.tracking[id] as usize;
            if p >= n || tracking[p] || self.by_position[p] as usize != id {
                return false;
            }
            tracking[p] = true;
            if self.exposed[id] && self.zones[id] == Zone::Unseen {
                return false;
            }
        }
        let mut owned = [false; MAX_CARDS];
        let mut valid = true;
        let mut claim = |h: &Hand| {
            for &p in h.positions() {
                if p as usize >= n {
                    valid = false;
                    continue;
                }
                let id = self.by_position[p as usize] as usize;
                if owned[id] || self.zones[id] != Zone::Held {
                    valid = false
                }
                owned[id] = true;
            }
        };
        for (i, h) in self.hands[..self.hand_count].iter().enumerate() {
            if self.player_held[i] {
                claim(h)
            }
        }
        for seat in &self.others[..self.extra_seats as usize] {
            if seat.hand_count > MAX_HANDS || seat.active > seat.hand_count || seat.bankroll < 0 {
                return false;
            }
            for (i, h) in seat.hands[..seat.hand_count].iter().enumerate() {
                if seat.held[i] {
                    claim(h)
                }
            }
        }
        if self.dealer_held {
            claim(&self.dealer)
        }
        valid
            && owned[..n]
                .iter()
                .enumerate()
                .all(|(id, &held)| held == (self.zones[id] == Zone::Held))
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    fn running(hand: &Hand) -> i16 {
        hand.cards()
            .iter()
            .map(|c| match c.rank() {
                2..=6 => 1,
                10..=14 => -1,
                _ => 0,
            })
            .sum()
    }
    fn exhaust_unseen(e: &mut Engine) {
        for at in e.draw_cursor..e.draw_len {
            e.zones[e.physical[at] as usize] = Zone::Discard;
        }
        e.draw_cursor = e.draw_len;
        e.position = e.total_cards();
    }
    #[test]
    fn recovery_excludes_held_cards_forgets_mixed_exposures_and_keeps_hole_hidden() {
        let mut e = (1..1000)
            .find_map(|seed| {
                let r = Rules {
                    penetration: 90,
                    ..Rules::default()
                };
                let mut e = Engine::new(seed, r).unwrap();
                e.buy_in(100000);
                e.cut(156);
                e.deal();
                if e.phase == Phase::Insurance {
                    e.insure(false);
                }
                (e.phase == Phase::Player && e.hands[0].total() <= 9).then_some(e)
            })
            .unwrap();
        let mut held = [false; MAX_CARDS];
        for &p in e.hands[0].positions().iter().chain(e.dealer.positions()) {
            held[p as usize] = true;
        }
        let old_held = e.inventory().held;
        exhaust_unseen(&mut e);
        let old_discard = (0..e.total_cards())
            .find(|&id| e.zones[id] == Zone::Discard)
            .unwrap();
        e.exposed[old_discard] = true;
        let mut tracker = Tracker::default();
        e.observe_exposed(&mut tracker);
        let epoch = e.shoe_number;
        let expected_old = running(&e.hands[0])
            + match e.dealer.cards()[0].rank() {
                2..=6 => 1,
                10..=14 => -1,
                _ => 0,
            };
        assert!(e.act(Action::Hit));
        assert_eq!(e.shoe_number, epoch + 1);
        assert_eq!(e.emergency_shuffles, 1);
        assert!(!e.hole_visible);
        assert_eq!(e.hands[0].count(), 3);
        assert!(!held[e.hands[0].positions()[2] as usize]);
        assert_eq!(e.inventory().held, old_held + 1);
        assert_eq!(e.inventory().discarded, 1);
        assert_eq!(e.position, old_held + 2);
        assert!(e.invariant());
        e.observe_exposed(&mut tracker);
        assert_eq!(tracker.observed_count(), 4);
        let new = e.hands[0].cards()[2];
        let new_tag = match new.rank() {
            2..=6 => 1,
            10..=14 => -1,
            _ => 0,
        };
        assert_eq!(tracker.running_count(), expected_old + new_tag);
        e.observe_exposed(&mut tracker);
        assert_eq!(tracker.observed_count(), 4);
        assert!(e.act(Action::Stand));
        while e.phase == Phase::Dealer {
            assert!(e.dealer_step());
        }
        assert_eq!(e.phase, Phase::Result);
        assert!(e.needs_shuffle());
        assert_eq!(e.inventory().held, 0);
        assert!(e.shuffle());
        assert!(e.cut(156));
        e.observe_exposed(&mut tracker);
        assert_eq!(tracker.observed_count(), 0);
        assert_eq!(tracker.running_count(), 0);
        assert!(e.invariant());
    }
    #[test]
    fn a_real_deep_seven_seat_shoe_reaches_and_recovers_from_exhaustion() {
        let mut recovered = false;
        for seed in 1..=100 {
            let rules = Rules {
                penetration: 90,
                ..Rules::default()
            };
            let mut e = Engine::new(seed, rules).unwrap();
            e.buy_in(1_000_000);
            e.configure_seats(6, 0);
            e.cut(156);
            // Valid complete six-deck permutation with all48 twos/threes at
            // the tail. Earlier rounds must actually consume the prefix.
            let n = e.total_cards();
            let mut order = [0u16; MAX_CARDS];
            let mut front = 0;
            let mut tail = n - 48;
            for &id in &e.physical[..n] {
                let rank = Card::from_id((id as usize % 52) as u8).unwrap().rank();
                if matches!(rank, 2 | 3) {
                    order[tail] = id;
                    tail += 1
                } else {
                    order[front] = id;
                    front += 1;
                }
            }
            assert_eq!(front, n - 48);
            assert_eq!(tail, n);
            e.physical = order;
            for (i, &id) in order[..n].iter().enumerate() {
                e.shoe[i] = Card::from_id((id as usize % 52) as u8).unwrap();
            }
            e.zones = [Zone::Unseen; MAX_CARDS];
            e.zones[order[0] as usize] = Zone::Discard;
            e.map_tracking();
            while !e.needs_shuffle() && e.rounds < 30 {
                assert!(e.deal());
                let mut steps = 0;
                while e.phase != Phase::Result {
                    steps += 1;
                    assert!(steps < 500);
                    match e.phase {
                        Phase::Insurance => {
                            assert!(e.insure(false));
                        }
                        Phase::Player => {
                            assert!(e.act(Action::Stand));
                        }
                        Phase::OtherPlayers => assert!(e.other_step()),
                        Phase::Dealer => assert!(e.dealer_step()),
                        _ => panic!("invalid phase"),
                    };
                    assert!(e.invariant());
                }
                if e.emergency_shuffles > 0 {
                    assert!(e.needs_shuffle());
                    assert_eq!(e.inventory().held, 0);
                    recovered = true;
                    break;
                }
            }
            if recovered {
                break;
            }
        }
        assert!(
            recovered,
            "ordered complete shoe must exercise legally reached exhaustion"
        );
    }
    #[test]
    fn recovery_without_a_safe_discard_and_burn_pool_changes_nothing() {
        let mut e = Engine::new(1, Rules::default()).unwrap();
        e.cut(156);
        e.phase = Phase::Player;
        e.zones = [Zone::Held; MAX_CARDS];
        e.zones[0] = Zone::Discard;
        e.draw_cursor = e.draw_len;
        e.position = e.total_cards();
        let zones = e.zones;
        let epoch = e.shoe_number;
        let rng = e.rng;
        assert!(!e.recover_discards());
        assert_eq!(e.zones, zones);
        assert_eq!(e.shoe_number, epoch);
        assert_eq!(e.rng, rng);
    }
}

#[cfg(test)]
mod tablemate_oracle_tests {
    extern crate std;
    use super::*;
    fn scripted(player: u8, ranks: &[u8]) -> Engine {
        let mut e = Engine::new(1, Rules::default()).unwrap();
        e.buy_in(100_000);
        e.configure_seats(1, player);
        e.cut(156);
        let n = e.total_cards();
        let mut used = [false; MAX_CARDS];
        let mut order = [0u16; MAX_CARDS];
        let mut at = 0;
        for &rank in ranks {
            let id = (0..n)
                .find(|&id| !used[id] && Card::from_id((id % 52) as u8).unwrap().rank() == rank)
                .unwrap();
            used[id] = true;
            order[at] = id as u16;
            at += 1;
        }
        for (id, &taken) in used[..n].iter().enumerate() {
            if !taken {
                order[at] = id as u16;
                at += 1;
            }
        }
        e.physical = order;
        for (i, &physical) in order[..n].iter().enumerate() {
            e.shoe[i] = Card::from_id((physical as usize % 52) as u8).unwrap();
        }
        e.zones = [Zone::Unseen; MAX_CARDS];
        e.zones[order[0] as usize] = Zone::Discard;
        e.map_tracking();
        e.exposed = [false; MAX_CARDS];
        assert!(e.invariant());
        assert!(e.deal());
        e
    }
    fn dealer(e: &mut Engine) {
        while e.phase == Phase::Dealer {
            assert!(e.dealer_step());
        }
        assert_eq!(e.phase, Phase::Result);
        assert!(e.invariant());
    }
    #[test]
    fn tablemate_double_is_funded_and_settled_independently() {
        let mut e = scripted(0, &[7, 10, 5, 6, 8, 6, 10, 10, 5]);
        assert_eq!(e.phase, Phase::Player);
        assert!(e.act(Action::Stand));
        assert_eq!(e.phase, Phase::OtherPlayers);
        assert!(e.other_step());
        let npc = e.others[0];
        assert_eq!(
            npc.hands[0]
                .cards()
                .iter()
                .map(|c| c.rank())
                .collect::<std::vec::Vec<_>>(),
            [5, 6, 10]
        );
        assert!(npc.hands[0].doubled);
        assert_eq!(npc.hands[0].bet, 1000);
        assert_eq!(npc.bankroll, 99000);
        assert_eq!(e.phase, Phase::Dealer);
        dealer(&mut e);
        assert_eq!(e.others[0].hands[0].outcome, Outcome::Push);
        assert_eq!(e.others[0].round_net, 0);
        assert_eq!(e.others[0].bankroll, 100000);
        assert_eq!(e.hands[0].outcome, Outcome::Loss);
        assert_eq!(e.bankroll, 97500);
    }
    #[test]
    fn tablemate_split_aces_get_one_card_and_twenty_one_pays_one_to_one() {
        let mut e = scripted(0, &[7, 10, 14, 6, 8, 14, 10, 3, 10, 10]);
        assert!(e.act(Action::Stand));
        assert!(e.other_step());
        assert_eq!(e.others[0].hand_count, 2);
        assert_eq!(e.phase, Phase::Dealer);
        for h in &e.others[0].hands[..2] {
            assert!(h.split && h.split_aces && h.done);
            assert_eq!(h.count(), 2);
            assert!(!h.natural());
        }
        dealer(&mut e);
        assert_eq!(e.others[0].round_return, 2000);
        assert_eq!(e.others[0].round_stake, 1000);
        assert_eq!(e.others[0].bankroll, 101000);
        assert_eq!(e.bankroll, 102500);
    }
    #[test]
    fn tablemate_surrender_discards_before_dealer_and_is_not_settled_twice() {
        let mut e = scripted(0, &[7, 10, 10, 10, 8, 6, 7]);
        assert!(e.act(Action::Stand));
        assert!(e.other_step());
        assert_eq!(e.others[0].hands[0].outcome, Outcome::Surrender);
        assert_eq!(e.inventory().discarded, 3);
        assert_eq!(e.inventory().held, 4);
        dealer(&mut e);
        assert_eq!(e.others[0].round_return, 250);
        assert_eq!(e.others[0].bankroll, 99750);
        assert_eq!(e.bankroll, 102500);
    }
    #[test]
    fn dealer_peek_resolves_all_seats_and_naturals_before_any_play() {
        let mut e = scripted(0, &[7, 14, 10, 14, 10, 9, 10]);
        assert_eq!(e.phase, Phase::Insurance);
        assert!(e.insure(false));
        assert_eq!(e.phase, Phase::Result);
        assert_eq!(e.hands[0].outcome, Outcome::Push);
        assert_eq!(e.others[0].hands[0].outcome, Outcome::Loss);
        assert_eq!(e.bankroll, 100000);
        assert_eq!(e.others[0].bankroll, 99500);
        assert_eq!(e.inventory().held, 0);
        assert!(e.invariant());
    }
    #[test]
    fn earlier_seat_finishes_before_controlled_player_is_offered_an_action() {
        let mut e = scripted(1, &[7, 5, 10, 6, 6, 8, 10, 10, 5]);
        assert_eq!(e.phase, Phase::OtherPlayers);
        assert!(!e.act(Action::Stand));
        assert!(e.other_step());
        assert_eq!(e.phase, Phase::Player);
        assert_eq!(e.active_seat, 1);
        assert!(e.act(Action::Stand));
        assert_eq!(e.phase, Phase::Dealer);
        dealer(&mut e);
        assert_eq!(e.others[0].round_net, 0);
    }
}

#[cfg(test)]
mod stable_identity_tests {
    use super::*;
    #[test]
    fn repeated_refills_preserve_identity_maps_and_release_exact_held_cards() {
        let mut e = (1..100)
            .find_map(|seed| {
                let mut e = Engine::new(seed, Rules::default()).unwrap();
                e.buy_in(100000);
                e.cut(156);
                e.deal();
                if e.phase == Phase::Insurance {
                    e.insure(false);
                }
                (e.phase == Phase::Player).then_some(e)
            })
            .unwrap();
        let tracking = e.tracking;
        let reverse = e.by_position;
        let user = e.hands[0];
        let dealer = e.dealer;
        for expected in 1..=3 {
            // Deplete only the unheld pool; the held hands and their identities
            // are preserved throughout this focused identity-map regression.
            for at in e.draw_cursor..e.draw_len {
                e.zones[e.physical[at] as usize] = Zone::Discard;
            }
            e.draw_cursor = e.draw_len;
            e.position = e.total_cards();
            assert!(e.recover_discards());
            assert_eq!(e.emergency_shuffles, expected);
            assert_eq!(e.tracking, tracking);
            assert_eq!(e.by_position, reverse);
            let (face, p) = e.pop().unwrap();
            assert!(!user.positions().contains(&p));
            assert!(!dealer.positions().contains(&p));
            let physical = reverse[p as usize] as usize;
            assert_eq!(tracking[physical], p);
            assert_eq!(face.id() as usize, physical % 52);
            let mut transit = Hand::EMPTY;
            transit.add(face, p);
            e.discard_hand(&transit);
            for &held in user.positions().iter().chain(dealer.positions()) {
                assert_eq!(e.zones[reverse[held as usize] as usize], Zone::Held);
            }
            assert_eq!(e.inventory().held, user.count() + dealer.count());
            assert!(e.invariant());
        }
        e.release_player(0);
        for &p in user.positions() {
            assert_eq!(e.zones[reverse[p as usize] as usize], Zone::Discard);
        }
        for &p in dealer.positions() {
            assert_eq!(e.zones[reverse[p as usize] as usize], Zone::Held);
        }
        assert_eq!(e.inventory().held, dealer.count());
        assert!(e.invariant());
    }
}

// RESUME10 Engine
impl crate::resume_private::Wire for Engine {
    fn initial() -> Self {
        Self {
            phase: crate::resume_private::Wire::initial(),
            rules: crate::resume_private::Wire::initial(),
            shoe: crate::resume_private::Wire::initial(),
            physical: crate::resume_private::Wire::initial(),
            tracking: crate::resume_private::Wire::initial(),
            by_position: crate::resume_private::Wire::initial(),
            zones: crate::resume_private::Wire::initial(),
            exposed: crate::resume_private::Wire::initial(),
            draw_cursor: crate::resume_private::Wire::initial(),
            draw_len: crate::resume_private::Wire::initial(),
            force_shuffle: crate::resume_private::Wire::initial(),
            player_held: crate::resume_private::Wire::initial(),
            dealer_held: crate::resume_private::Wire::initial(),
            extra_seats: crate::resume_private::Wire::initial(),
            player_seat: crate::resume_private::Wire::initial(),
            active_seat: crate::resume_private::Wire::initial(),
            others: crate::resume_private::Wire::initial(),
            emergency_shuffles: crate::resume_private::Wire::initial(),
            hands: crate::resume_private::Wire::initial(),
            dealer: crate::resume_private::Wire::initial(),
            position: crate::resume_private::Wire::initial(),
            marker: crate::resume_private::Wire::initial(),
            hand_count: crate::resume_private::Wire::initial(),
            active: crate::resume_private::Wire::initial(),
            shoe_number: crate::resume_private::Wire::initial(),
            cut_at: crate::resume_private::Wire::initial(),
            bankroll: crate::resume_private::Wire::initial(),
            deposits: crate::resume_private::Wire::initial(),
            cashouts: crate::resume_private::Wire::initial(),
            selected_bet: crate::resume_private::Wire::initial(),
            insurance: crate::resume_private::Wire::initial(),
            round_stake: crate::resume_private::Wire::initial(),
            round_return: crate::resume_private::Wire::initial(),
            round_net: crate::resume_private::Wire::initial(),
            rng: crate::resume_private::Wire::initial(),
            rounds: crate::resume_private::Wire::initial(),
            wins: crate::resume_private::Wire::initial(),
            losses: crate::resume_private::Wire::initial(),
            pushes: crate::resume_private::Wire::initial(),
            hole_visible: crate::resume_private::Wire::initial(),
            insurance_resolved: crate::resume_private::Wire::initial(),
            insured: crate::resume_private::Wire::initial(),
            cut_pending: crate::resume_private::Wire::initial(),
            message: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            phase,
            rules,
            shoe,
            physical,
            tracking,
            by_position,
            zones,
            exposed,
            draw_cursor,
            draw_len,
            force_shuffle,
            player_held,
            dealer_held,
            extra_seats,
            player_seat,
            active_seat,
            others,
            emergency_shuffles,
            hands,
            dealer,
            position,
            marker,
            hand_count,
            active,
            shoe_number,
            cut_at,
            bankroll,
            deposits,
            cashouts,
            selected_bet,
            insurance,
            round_stake,
            round_return,
            round_net,
            rng,
            rounds,
            wins,
            losses,
            pushes,
            hole_visible,
            insurance_resolved,
            insured,
            cut_pending,
            message,
        } = self;
        phase.put(w)?;
        rules.put(w)?;
        shoe.put(w)?;
        physical.put(w)?;
        tracking.put(w)?;
        by_position.put(w)?;
        zones.put(w)?;
        exposed.put(w)?;
        draw_cursor.put(w)?;
        draw_len.put(w)?;
        force_shuffle.put(w)?;
        player_held.put(w)?;
        dealer_held.put(w)?;
        extra_seats.put(w)?;
        player_seat.put(w)?;
        active_seat.put(w)?;
        others.put(w)?;
        emergency_shuffles.put(w)?;
        hands.put(w)?;
        dealer.put(w)?;
        position.put(w)?;
        marker.put(w)?;
        hand_count.put(w)?;
        active.put(w)?;
        shoe_number.put(w)?;
        cut_at.put(w)?;
        bankroll.put(w)?;
        deposits.put(w)?;
        cashouts.put(w)?;
        selected_bet.put(w)?;
        insurance.put(w)?;
        round_stake.put(w)?;
        round_return.put(w)?;
        round_net.put(w)?;
        rng.put(w)?;
        rounds.put(w)?;
        wins.put(w)?;
        losses.put(w)?;
        pushes.put(w)?;
        hole_visible.put(w)?;
        insurance_resolved.put(w)?;
        insured.put(w)?;
        cut_pending.put(w)?;
        message.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            phase,
            rules,
            shoe,
            physical,
            tracking,
            by_position,
            zones,
            exposed,
            draw_cursor,
            draw_len,
            force_shuffle,
            player_held,
            dealer_held,
            extra_seats,
            player_seat,
            active_seat,
            others,
            emergency_shuffles,
            hands,
            dealer,
            position,
            marker,
            hand_count,
            active,
            shoe_number,
            cut_at,
            bankroll,
            deposits,
            cashouts,
            selected_bet,
            insurance,
            round_stake,
            round_return,
            round_net,
            rng,
            rounds,
            wins,
            losses,
            pushes,
            hole_visible,
            insurance_resolved,
            insured,
            cut_pending,
            message,
        } = self;
        phase.read_into(r)?;
        rules.read_into(r)?;
        shoe.read_into(r)?;
        physical.read_into(r)?;
        tracking.read_into(r)?;
        by_position.read_into(r)?;
        zones.read_into(r)?;
        exposed.read_into(r)?;
        draw_cursor.read_into(r)?;
        draw_len.read_into(r)?;
        force_shuffle.read_into(r)?;
        player_held.read_into(r)?;
        dealer_held.read_into(r)?;
        extra_seats.read_into(r)?;
        player_seat.read_into(r)?;
        active_seat.read_into(r)?;
        others.read_into(r)?;
        emergency_shuffles.read_into(r)?;
        hands.read_into(r)?;
        dealer.read_into(r)?;
        position.read_into(r)?;
        marker.read_into(r)?;
        hand_count.read_into(r)?;
        active.read_into(r)?;
        shoe_number.read_into(r)?;
        cut_at.read_into(r)?;
        bankroll.read_into(r)?;
        deposits.read_into(r)?;
        cashouts.read_into(r)?;
        selected_bet.read_into(r)?;
        insurance.read_into(r)?;
        round_stake.read_into(r)?;
        round_return.read_into(r)?;
        round_net.read_into(r)?;
        rng.read_into(r)?;
        rounds.read_into(r)?;
        wins.read_into(r)?;
        losses.read_into(r)?;
        pushes.read_into(r)?;
        hole_visible.read_into(r)?;
        insurance_resolved.read_into(r)?;
        insured.read_into(r)?;
        cut_pending.read_into(r)?;
        message.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            phase: crate::resume_private::Wire::get(r)?,
            rules: crate::resume_private::Wire::get(r)?,
            shoe: crate::resume_private::Wire::get(r)?,
            physical: crate::resume_private::Wire::get(r)?,
            tracking: crate::resume_private::Wire::get(r)?,
            by_position: crate::resume_private::Wire::get(r)?,
            zones: crate::resume_private::Wire::get(r)?,
            exposed: crate::resume_private::Wire::get(r)?,
            draw_cursor: crate::resume_private::Wire::get(r)?,
            draw_len: crate::resume_private::Wire::get(r)?,
            force_shuffle: crate::resume_private::Wire::get(r)?,
            player_held: crate::resume_private::Wire::get(r)?,
            dealer_held: crate::resume_private::Wire::get(r)?,
            extra_seats: crate::resume_private::Wire::get(r)?,
            player_seat: crate::resume_private::Wire::get(r)?,
            active_seat: crate::resume_private::Wire::get(r)?,
            others: crate::resume_private::Wire::get(r)?,
            emergency_shuffles: crate::resume_private::Wire::get(r)?,
            hands: crate::resume_private::Wire::get(r)?,
            dealer: crate::resume_private::Wire::get(r)?,
            position: crate::resume_private::Wire::get(r)?,
            marker: crate::resume_private::Wire::get(r)?,
            hand_count: crate::resume_private::Wire::get(r)?,
            active: crate::resume_private::Wire::get(r)?,
            shoe_number: crate::resume_private::Wire::get(r)?,
            cut_at: crate::resume_private::Wire::get(r)?,
            bankroll: crate::resume_private::Wire::get(r)?,
            deposits: crate::resume_private::Wire::get(r)?,
            cashouts: crate::resume_private::Wire::get(r)?,
            selected_bet: crate::resume_private::Wire::get(r)?,
            insurance: crate::resume_private::Wire::get(r)?,
            round_stake: crate::resume_private::Wire::get(r)?,
            round_return: crate::resume_private::Wire::get(r)?,
            round_net: crate::resume_private::Wire::get(r)?,
            rng: crate::resume_private::Wire::get(r)?,
            rounds: crate::resume_private::Wire::get(r)?,
            wins: crate::resume_private::Wire::get(r)?,
            losses: crate::resume_private::Wire::get(r)?,
            pushes: crate::resume_private::Wire::get(r)?,
            hole_visible: crate::resume_private::Wire::get(r)?,
            insurance_resolved: crate::resume_private::Wire::get(r)?,
            insured: crate::resume_private::Wire::get(r)?,
            cut_pending: crate::resume_private::Wire::get(r)?,
            message: crate::resume_private::Wire::get(r)?,
        })
    }
}
impl crate::resume_private::SnapshotCopy for Engine {
    fn snapshot_copy_to(&self, out: &mut Self) {
        let Self {
            phase,
            rules,
            shoe,
            physical,
            tracking,
            by_position,
            zones,
            exposed,
            draw_cursor,
            draw_len,
            force_shuffle,
            player_held,
            dealer_held,
            extra_seats,
            player_seat,
            active_seat,
            others,
            emergency_shuffles,
            hands,
            dealer,
            position,
            marker,
            hand_count,
            active,
            shoe_number,
            cut_at,
            bankroll,
            deposits,
            cashouts,
            selected_bet,
            insurance,
            round_stake,
            round_return,
            round_net,
            rng,
            rounds,
            wins,
            losses,
            pushes,
            hole_visible,
            insurance_resolved,
            insured,
            cut_pending,
            message,
        } = self;
        crate::resume_private::SnapshotCopy::snapshot_copy_to(phase, &mut out.phase);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(rules, &mut out.rules);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(shoe, &mut out.shoe);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(physical, &mut out.physical);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(tracking, &mut out.tracking);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(by_position, &mut out.by_position);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(zones, &mut out.zones);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(exposed, &mut out.exposed);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(draw_cursor, &mut out.draw_cursor);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(draw_len, &mut out.draw_len);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(
            force_shuffle,
            &mut out.force_shuffle,
        );
        crate::resume_private::SnapshotCopy::snapshot_copy_to(player_held, &mut out.player_held);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(dealer_held, &mut out.dealer_held);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(extra_seats, &mut out.extra_seats);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(player_seat, &mut out.player_seat);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(active_seat, &mut out.active_seat);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(others, &mut out.others);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(
            emergency_shuffles,
            &mut out.emergency_shuffles,
        );
        crate::resume_private::SnapshotCopy::snapshot_copy_to(hands, &mut out.hands);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(dealer, &mut out.dealer);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(position, &mut out.position);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(marker, &mut out.marker);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(hand_count, &mut out.hand_count);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(active, &mut out.active);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(shoe_number, &mut out.shoe_number);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(cut_at, &mut out.cut_at);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(bankroll, &mut out.bankroll);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(deposits, &mut out.deposits);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(cashouts, &mut out.cashouts);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(selected_bet, &mut out.selected_bet);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(insurance, &mut out.insurance);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(round_stake, &mut out.round_stake);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(round_return, &mut out.round_return);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(round_net, &mut out.round_net);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(rng, &mut out.rng);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(rounds, &mut out.rounds);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(wins, &mut out.wins);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(losses, &mut out.losses);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(pushes, &mut out.pushes);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(hole_visible, &mut out.hole_visible);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(
            insurance_resolved,
            &mut out.insurance_resolved,
        );
        crate::resume_private::SnapshotCopy::snapshot_copy_to(insured, &mut out.insured);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(cut_pending, &mut out.cut_pending);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(message, &mut out.message);
    }
}

// RESUME10 enum Phase
impl crate::resume_private::Wire for Phase {
    fn initial() -> Self {
        Self::Cut
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::Cut),
            1 => Ok(Self::Ready),
            2 => Ok(Self::Insurance),
            3 => Ok(Self::Player),
            4 => Ok(Self::Dealer),
            5 => Ok(Self::Result),
            6 => Ok(Self::Closed),
            7 => Ok(Self::OtherPlayers),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}

// RESUME10 enum Action
impl crate::resume_private::Wire for Action {
    fn initial() -> Self {
        Self::Hit
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::Hit),
            1 => Ok(Self::Stand),
            2 => Ok(Self::Double),
            3 => Ok(Self::Split),
            4 => Ok(Self::Surrender),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}
impl Engine {
    pub(crate) fn resume_valid(&self) -> bool {
        let n = self.rules.total_cards();
        if !self.rules.valid()
            || n > MAX_CARDS
            || self.shoe_number == 0
            || self.extra_seats as usize > MAX_EXTRA_SEATS
            || self.player_seat > self.extra_seats
            || self.active_seat > self.extra_seats.saturating_add(1)
            || self.hand_count > MAX_HANDS
            || self.active > MAX_HANDS
            || self.draw_cursor > self.draw_len
            || self.draw_len > n
            || self.position > n
        {
            return false;
        }
        if self
            .hands
            .iter()
            .chain(core::iter::once(&self.dealer))
            .any(|h| !h.resume_valid())
            || self.others.iter().any(|s| {
                s.hand_count > MAX_HANDS
                    || s.active > MAX_HANDS
                    || s.hands.iter().any(|h| !h.resume_valid())
                    || s.bankroll < 0
                    || s.bankroll > i64::MAX / 4
                    || s.deposits < 0
                    || s.deposits > i64::MAX / 4
            })
        {
            return false;
        }
        if self.marker > n
            || self.cut_at > n
            || self.selected_bet < self.rules.minimum
            || self.selected_bet > self.rules.maximum
            || self.selected_bet % 500 != 0
            || self.insurance < 0
            || self.insurance > self.rules.maximum / 2
            || self.round_stake < 0
            || self.round_stake > 100_000_000
            || self.round_return < 0
            || self.round_return > 100_000_000
            || self.round_net.unsigned_abs() > 100_000_000
        {
            return false;
        }
        if self
            .hands
            .iter()
            .chain(self.others.iter().flat_map(|s| s.hands.iter()))
            .any(|h| h.bet < 0 || h.bet > self.rules.maximum * 2)
        {
            return false;
        }
        if matches!(
            self.phase,
            Phase::Player | Phase::Insurance | Phase::Dealer | Phase::OtherPlayers
        ) && self.dealer.count() < 2
        {
            return false;
        }
        if self.phase == Phase::Player && self.active_seat != self.player_seat {
            return false;
        }
        if self.phase == Phase::OtherPlayers {
            if self.active_seat > self.extra_seats || self.active_seat == self.player_seat {
                return false;
            }
            let i = if self.active_seat < self.player_seat {
                self.active_seat as usize
            } else {
                self.active_seat as usize - 1
            };
            if self.others[i].hand_count == 0 || self.others[i].active >= self.others[i].hand_count
            {
                return false;
            }
        }
        if self.bankroll < 0
            || self.bankroll > i64::MAX / 4
            || self.deposits < 0
            || self.deposits > i64::MAX / 4
            || self.cashouts < 0
            || self.cashouts > i64::MAX / 4
        {
            return false;
        }
        self.invariant()
    }
    pub(crate) fn resume_exposed_card(&self, p: usize) -> Option<Card> {
        if p >= self.total_cards() {
            return None;
        }
        let id = self.by_position[p] as usize;
        if id >= self.total_cards() || !self.exposed[id] {
            return None;
        }
        Card::from_id((id % 52) as u8)
    }
}
