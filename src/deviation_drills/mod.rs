//! Isolated no_std targeted-deviation practice, without engine/RNG/hidden-card input.
//! Original attempts, help and replay scores remain separate and immutable.
#![forbid(unsafe_code)]
mod catalogue;
use crate::{
    cards::{Card, Hand, Outcome, UNKNOWN_POSITION},
    deviations::Count,
    engine::Action,
    rules::{Money, Profile, Rules},
    strategy,
    training::{COUNTER_LIMIT, Choice, Counters},
};
use catalogue::{ENTRIES, Shape};

pub const HISTORY_CAPACITY: usize = 128;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum IndexId {
    Tens4,
    Tens5,
    Tens6,
    Soft19v4,
    Soft19v5,
    Soft19v6H17,
    Soft19v6S17,
    Soft17v2,
    Hard16v9,
    Hard16v10,
    Hard16vAceH17,
    Hard15v10,
    Hard15vAceH17,
    Hard13v2,
    Hard12v2,
    Hard12v3,
    Hard12v4,
    Hard11vAceS17,
    Hard10v10,
    Hard10vAceH17,
    Hard10vAceS17,
    Hard9v2,
    Hard9v7,
    Hard8v6,
    Surrender16v8,
    Surrender16v9,
    Surrender15v9,
    Surrender15v10,
    Surrender15vAceH17,
    Surrender15vAceS17,
    Insurance,
}
impl IndexId {
    pub fn from_id(id: u8) -> Option<Self> {
        Self::ALL.get(id as usize).copied()
    }
    pub fn name(self) -> &'static str {
        [
            "Tens vs4",
            "Tens vs5",
            "Tens vs6",
            "Soft19 vs4",
            "Soft19 vs5",
            "Soft19 vs6 H17",
            "Soft19 vs6 S17",
            "Soft17 vs2",
            "Hard16 vs9",
            "Hard16 vs10",
            "Hard16 vsA H17",
            "Hard15 vs10",
            "Hard15 vsA H17",
            "Hard13 vs2",
            "Hard12 vs2",
            "Hard12 vs3",
            "Hard12 vs4",
            "Hard11 vsA S17",
            "Hard10 vs10",
            "Hard10 vsA H17",
            "Hard10 vsA S17",
            "Hard9 vs2",
            "Hard9 vs7",
            "Hard8 vs6",
            "Surrender16 vs8",
            "Surrender16 vs9",
            "Surrender15 vs9",
            "Surrender15 vs10",
            "Surrender15 vsA H17",
            "Surrender15 vsA S17",
            "Insurance / even money",
        ][self as usize]
    }
    pub const COUNT: usize = 31;
    pub const ALL: [Self; Self::COUNT] = [
        Self::Tens4,
        Self::Tens5,
        Self::Tens6,
        Self::Soft19v4,
        Self::Soft19v5,
        Self::Soft19v6H17,
        Self::Soft19v6S17,
        Self::Soft17v2,
        Self::Hard16v9,
        Self::Hard16v10,
        Self::Hard16vAceH17,
        Self::Hard15v10,
        Self::Hard15vAceH17,
        Self::Hard13v2,
        Self::Hard12v2,
        Self::Hard12v3,
        Self::Hard12v4,
        Self::Hard11vAceS17,
        Self::Hard10v10,
        Self::Hard10vAceH17,
        Self::Hard10vAceS17,
        Self::Hard9v2,
        Self::Hard9v7,
        Self::Hard8v6,
        Self::Surrender16v8,
        Self::Surrender16v9,
        Self::Surrender15v9,
        Self::Surrender15v10,
        Self::Surrender15vAceH17,
        Self::Surrender15vAceS17,
        Self::Insurance,
    ];
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Policy {
    Bja2018HiLoTruncatedV1,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Threshold {
    AtLeast(i16),
    AtMost(i16),
    PositiveRunning,
    NegativeRunning,
}
impl Threshold {
    pub const fn reached(self, rc: i16, tc: i16) -> bool {
        match self {
            Self::AtLeast(n) => tc >= n,
            Self::AtMost(n) => tc <= n,
            Self::PositiveRunning => rc > 0,
            Self::NegativeRunning => rc < 0,
        }
    }
    fn boundary_rc(self) -> i16 {
        match self {
            Self::AtLeast(n) if n > 0 => n * 3,
            Self::AtLeast(n) => (n - 1) * 3 + 1,
            Self::AtMost(n) if n < 0 => n * 3,
            Self::AtMost(n) => (n + 1) * 3 - 1,
            Self::PositiveRunning => 1,
            Self::NegativeRunning => -1,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Funding {
    pub bankroll: Money,
    pub original_wager: Money,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublicContext {
    pub hand: Hand,
    pub dealer_up: Card,
    pub legal_mask: u8,
    pub rules: Rules,
    pub count: Count,
    pub funding: Funding,
    pub session: u64,
    pub policy: Policy,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Origin {
    First,
    Replay { original_token: u64 },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Question {
    pub token: u64,
    pub index: IndexId,
    pub context: PublicContext,
    origin: Origin,
}
impl Question {
    pub fn from_public(token: u64, index: IndexId, context: PublicContext) -> Option<Self> {
        let q = Self {
            token,
            index,
            context,
            origin: Origin::First,
        };
        reference(q).map(|_| q)
    }
    pub const fn origin(self) -> Origin {
        self.origin
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Reference {
    pub best: Choice,
    pub basic: Choice,
    pub threshold: Threshold,
    pub threshold_reached: bool,
    pub integer_true_count: i16,
    pub policy: Policy,
    pub normalized_legal_mask: u8,
    pub insurance_affordable: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Attempt {
    pub question: Question,
    pub answer: Choice,
    pub correct: bool,
    pub assisted: bool,
    pub reference: Reference,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Recorded,
    Duplicate,
    Unavailable,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Grade {
    pub status: Status,
    pub attempt: Option<Attempt>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Totals {
    pub all: Counters,
    pub indices: [Counters; IndexId::COUNT],
}
impl Default for Totals {
    fn default() -> Self {
        Self::EMPTY
    }
}
impl Totals {
    pub const EMPTY: Self = Self {
        all: Counters::EMPTY,
        indices: [Counters::EMPTY; IndexId::COUNT],
    };
    pub fn valid(self) -> bool {
        if !self.all.valid() {
            return false;
        }
        let mut sum = Counters::default();
        for c in self.indices {
            if !c.valid() {
                return false;
            }
            let Some(decisions) = sum.decisions.checked_add(c.decisions) else {
                return false;
            };
            let Some(correct) = sum.correct.checked_add(c.correct) else {
                return false;
            };
            let Some(errors) = sum.errors.checked_add(c.errors) else {
                return false;
            };
            let Some(assisted) = sum.assisted.checked_add(c.assisted) else {
                return false;
            };
            sum = Counters {
                decisions,
                correct,
                errors,
                assisted,
            };
        }
        sum == self.all
    }
    fn record(&mut self, index: IndexId, correct: bool, assisted: bool) -> bool {
        if self.all.decisions >= COUNTER_LIMIT {
            return false;
        }
        for c in [&mut self.all, &mut self.indices[index as usize]] {
            c.decisions += 1;
            if assisted {
                c.assisted += 1
            } else if correct {
                c.correct += 1
            } else {
                c.errors += 1
            }
        }
        true
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct Stats {
    pub first: Totals,
    pub replay: Totals,
}
impl Stats {
    pub const EMPTY: Self = Self {
        first: Totals::EMPTY,
        replay: Totals::EMPTY,
    };
    pub fn valid(self) -> bool {
        self.first.valid() && self.replay.valid()
    }
}
#[derive(Clone, Copy)]
struct Pending {
    question: Question,
    reference: Reference,
    helped: bool,
}
pub struct Model {
    pending: Option<Pending>,
    last_token: u64,
    stats: Stats,
    history: [Option<Attempt>; HISTORY_CAPACITY],
    start: usize,
    len: usize,
    restored: bool,
}
impl Default for Model {
    fn default() -> Self {
        Self::EMPTY
    }
}
impl Model {
    pub const EMPTY: Self = Self {
        pending: None,
        last_token: 0,
        stats: Stats::EMPTY,
        history: [None; HISTORY_CAPACITY],
        start: 0,
        len: 0,
        restored: false,
    };
    pub fn stats(&self) -> Stats {
        self.stats
    }
    pub fn question(&self) -> Option<Question> {
        self.pending.map(|p| p.question)
    }
    pub fn history_count(&self) -> usize {
        self.len
    }
    pub fn history(&self, index: usize) -> Option<&Attempt> {
        if index >= self.len {
            None
        } else {
            self.history[(self.start + index) % HISTORY_CAPACITY].as_ref()
        }
    }
    pub fn newest(&self, offset: usize) -> Option<&Attempt> {
        offset
            .checked_add(1)
            .and_then(|n| self.len.checked_sub(n))
            .and_then(|i| self.history(i))
    }
    fn recorded(&self, token: u64) -> Option<Attempt> {
        (0..self.len)
            .find_map(|i| self.history(i).filter(|a| a.question.token == token))
            .copied()
    }
    pub fn begin(&mut self, question: Question) -> bool {
        if self.pending.is_some() || question.token <= self.last_token {
            return false;
        }
        let Some(r) = reference(question) else {
            return false;
        };
        self.last_token = question.token;
        self.pending = Some(Pending {
            question,
            reference: r,
            helped: false,
        });
        true
    }
    pub fn cancel(&mut self, token: u64) -> bool {
        if self.pending.is_some_and(|p| p.question.token == token) {
            self.pending = None;
            true
        } else {
            false
        }
    }
    pub fn reference(&self, token: u64) -> Option<Reference> {
        self.recorded(token).map(|a| a.reference).or_else(|| {
            self.pending
                .filter(|p| p.question.token == token && p.helped)
                .map(|p| p.reference)
        })
    }
    pub fn request_help(&mut self, token: u64) -> Option<Reference> {
        if let Some(p) = self.pending.as_mut().filter(|p| p.question.token == token) {
            p.helped = true;
            Some(p.reference)
        } else {
            self.reference(token)
        }
    }
    /// Clear learning scores/history only, retaining consumed-token protection.
    pub fn reset_stats(&mut self) -> bool {
        if self.pending.is_some() {
            return false;
        }
        self.stats = Stats::EMPTY;
        self.history.fill(None);
        self.start = 0;
        self.len = 0;
        true
    }
    pub fn restore(&mut self, stats: Stats) -> bool {
        if self.restored
            || self.last_token != 0
            || self.len != 0
            || self.pending.is_some()
            || !stats.valid()
        {
            return false;
        }
        self.stats = stats;
        self.restored = true;
        true
    }
    pub fn submit(&mut self, token: u64, answer: Choice, assisted: bool) -> Grade {
        if let Some(a) = self.recorded(token) {
            return Grade {
                status: Status::Duplicate,
                attempt: Some(a),
            };
        }
        let Some(mut p) = self.pending.filter(|p| p.question.token == token) else {
            return Grade {
                status: if token <= self.last_token {
                    Status::Duplicate
                } else {
                    Status::Unavailable
                },
                attempt: None,
            };
        };
        p.helped |= assisted;
        self.pending = Some(p);
        let legal = match answer {
            Choice::Play(a) => {
                p.question.index != IndexId::Insurance
                    && p.reference.normalized_legal_mask & a.bit() != 0
            }
            Choice::Insurance(yes) => {
                p.question.index == IndexId::Insurance && (!yes || p.reference.insurance_affordable)
            }
        };
        if !legal {
            return Grade {
                status: Status::Unavailable,
                attempt: None,
            };
        }
        let a = Attempt {
            question: p.question,
            answer,
            correct: answer == p.reference.best,
            assisted: p.helped,
            reference: p.reference,
        };
        let totals = match a.question.origin {
            Origin::First => &mut self.stats.first,
            Origin::Replay { .. } => &mut self.stats.replay,
        };
        if !totals.record(a.question.index, a.correct, a.assisted) {
            return Grade {
                status: Status::Unavailable,
                attempt: None,
            };
        }
        self.pending = None;
        if self.len == HISTORY_CAPACITY {
            self.history[self.start] = Some(a);
            self.start = (self.start + 1) % HISTORY_CAPACITY
        } else {
            self.history[(self.start + self.len) % HISTORY_CAPACITY] = Some(a);
            self.len += 1
        }
        Grade {
            status: Status::Recorded,
            attempt: Some(a),
        }
    }
    /// A replay retains the original public rule/count/legal/funding evidence;
    /// its outcome updates only replay totals, never first-attempt accuracy.
    pub fn replay_missed(&self, token: u64, new_token: u64) -> Option<Question> {
        let a = self.recorded(token)?;
        if a.correct || a.assisted || new_token <= self.last_token {
            return None;
        }
        Some(Question {
            token: new_token,
            origin: Origin::Replay {
                original_token: token,
            },
            ..a.question
        })
    }
}
fn normalized(c: PublicContext) -> u8 {
    let mut mask = c.legal_mask & 31;
    let h = c.hand;
    let pair = h.count() == 2 && h.cards()[0].value() == h.cards()[1].value();
    if h.count() != 2 {
        mask &= !(Action::Double.bit() | Action::Split.bit() | Action::Surrender.bit())
    }
    if !pair {
        mask &= !Action::Split.bit()
    }
    if h.split || !c.rules.late_surrender {
        mask &= !Action::Surrender.bit()
    }
    if h.split && !c.rules.double_after_split {
        mask &= !Action::Double.bit()
    }
    if c.funding.bankroll < c.funding.original_wager {
        mask &= !(Action::Double.bit() | Action::Split.bit())
    }
    if h.split_aces || h.total() >= 21 {
        mask &= Action::Stand.bit()
    }
    mask
}
fn hi_lo(c: Card) -> i16 {
    match c.value() {
        2..=6 => 1,
        10 | 11 => -1,
        _ => 0,
    }
}
/// Existence check for a possible exposed set containing the stated visible
/// cards. It does not infer or consult any actual hidden composition/order.
fn plausible_exposed(c: PublicContext) -> bool {
    let visible = c.hand.count() + 1;
    let observed = c.count.observed as usize;
    if observed < visible {
        return false;
    }
    let mut copies = [0u8; 52];
    let mut low = 20 * c.rules.decks as i16;
    let mut high = low;
    let mut neutral = 12 * c.rules.decks as i16;
    let mut shown = 0;
    for card in c
        .hand
        .cards()
        .iter()
        .copied()
        .chain(core::iter::once(c.dealer_up))
    {
        let i = card.id() as usize;
        copies[i] += 1;
        if copies[i] > c.rules.decks {
            return false;
        }
        let tag = hi_lo(card);
        shown += tag;
        match tag {
            1 => low -= 1,
            -1 => high -= 1,
            _ => neutral -= 1,
        }
    }
    let n = (observed - visible) as i16;
    let target = c.count.running - shown;
    for l in 0..=low.min(n) {
        let h = l - target;
        let z = n - l - h;
        if h >= 0 && h <= high && z >= 0 && z <= neutral {
            return true;
        }
    }
    false
}
fn reference(q: Question) -> Option<Reference> {
    let c = q.context;
    let e = ENTRIES[q.index as usize];
    let h = c.hand;
    if q.token == 0
        || c.session == 0
        || !c.rules.valid()
        || !c.count.valid(c.rules)
        // The named US-peek decision context retains an unseen burn and hole.
        || c.count.observed as usize + 2 > c.rules.total_cards() - c.count.undealt as usize
        || !e.enabled(c.rules)
        || h.count() < 2
        || h.done
        || h.outcome != Outcome::Pending
        || h.total() > 21
        || c.dealer_up.value() != e.up
        || c.funding.bankroll < 0
        || c.funding.bankroll > 1_000_000_000
        || c.funding.original_wager < c.rules.minimum
        || c.funding.original_wager > c.rules.maximum
        || c.funding.original_wager % 500 != 0
        || h.bet != c.funding.original_wager
        || !plausible_exposed(c)
    {
        return None;
    }
    let tc = c.count.integer_true_count(c.rules)?;
    let reached = e.threshold.reached(c.count.running, tc);
    let affordable = c.funding.bankroll >= c.funding.original_wager / 2;
    if matches!(e.shape, Shape::Insurance) {
        return (c.legal_mask == 0 && h.count() == 2 && !h.split).then_some(Reference {
            best: Choice::Insurance(affordable && reached),
            basic: Choice::Insurance(false),
            threshold: e.threshold,
            threshold_reached: reached,
            integer_true_count: tc,
            policy: c.policy,
            normalized_legal_mask: 0,
            insurance_affordable: affordable,
        });
    }
    let mask = normalized(c);
    if mask == 0 || mask != c.legal_mask {
        return None;
    }
    let pair = h.count() == 2 && h.cards()[0].value() == h.cards()[1].value();
    match e.shape {
        Shape::PairTens if !pair || h.cards()[0].value() != 10 => return None,
        Shape::Soft(t) if !h.soft() || h.total() != t => return None,
        Shape::Hard(t) if h.soft() || h.total() != t => return None,
        Shape::Surrender(t)
            if h.soft() || h.total() != t || mask & Action::Surrender.bit() == 0 =>
        {
            return None;
        }
        _ => {}
    }
    let basic = strategy::recommend(&h, c.dealer_up, mask, c.rules)?;
    if basic.best == Action::Split && !matches!(e.shape, Shape::PairTens) {
        return None;
    }
    // Do not generate a hard-index question where a surrender decision has priority.
    if matches!(e.shape, Shape::Hard(_))
        && mask & Action::Surrender.bit() != 0
        && (basic.best == Action::Surrender
            || matches!((h.total(), c.dealer_up.value()), (16, 8 | 9) | (15, 9..=11)))
    {
        return None;
    }
    let chosen = if reached { Some(e.above) } else { e.otherwise };
    let best = chosen.filter(|a| mask & a.bit() != 0).unwrap_or(basic.best);
    Some(Reference {
        best: Choice::Play(best),
        basic: Choice::Play(basic.best),
        threshold: e.threshold,
        threshold_reached: reached,
        integer_true_count: tc,
        policy: c.policy,
        normalized_legal_mask: mask,
        insurance_affordable: affordable,
    })
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Boundary {
    Below,
    At,
    Above,
}
fn card(rank: u8) -> Card {
    Card::from_id(rank - 2).unwrap()
}
/// Generate a synthetic, physically possible public context at a one-RC boundary.
/// A separate three-deck divisor is explicitly stated in the question.
pub fn generate(
    token: u64,
    index: IndexId,
    profile: Profile,
    boundary: Boundary,
) -> Option<Question> {
    let rules = Rules::for_profile(profile);
    let e = ENTRIES[index as usize];
    if !e.enabled(rules) {
        return None;
    }
    let mut h = Hand::default();
    let ranks = match e.shape {
        Shape::PairTens => (10, 10),
        Shape::Soft(t) => (14, (t - 11) as u8),
        Shape::Hard(t) | Shape::Surrender(t) => {
            if t >= 12 {
                (10, (t - 10) as u8)
            } else {
                match t {
                    8 => (3, 5),
                    9 => (4, 5),
                    10 => (4, 6),
                    11 => (5, 6),
                    _ => return None,
                }
            }
        }
        Shape::Insurance => (10, 9),
    };
    h.add(card(ranks.0), UNKNOWN_POSITION);
    h.add(card(ranks.1), UNKNOWN_POSITION);
    // A hard15/16 index must be a real post-hit context when this table
    // offers late surrender. Do not merely hide a legal surrender choice.
    if rules.late_surrender && matches!(e.shape, Shape::Hard(15 | 16)) {
        let total = match e.shape {
            Shape::Hard(t) => t,
            _ => 0,
        };
        h = Hand::default();
        for rank in [if total == 15 { 4 } else { 5 }, 5, 6] {
            h.add(card(rank), UNKNOWN_POSITION);
        }
    }
    h.bet = rules.minimum;
    let delta = match boundary {
        Boundary::Below => -1,
        Boundary::At => 0,
        Boundary::Above => 1,
    };
    let rc = e.threshold.boundary_rc() + delta;
    let mut c = PublicContext {
        hand: h,
        dealer_up: card(if e.up == 11 { 14 } else { e.up }),
        legal_mask: 31,
        rules,
        count: Count {
            running: rc,
            undealt: 156,
            decks: rules.decks,
            observed: rules.total_cards() as u16 - 156 - 2,
            shoe: 1,
        },
        funding: Funding {
            bankroll: 100_000,
            original_wager: rules.minimum,
        },
        session: 1,
        policy: Policy::Bja2018HiLoTruncatedV1,
    };
    if matches!(e.shape, Shape::Insurance) {
        c.legal_mask = 0
    } else {
        c.legal_mask = normalized(c);
    }
    Question::from_public(token, index, c)
}
/// Local practice RNG; this type never accepts an Engine or advances its RNG.
pub struct Generator {
    state: u64,
}
impl Generator {
    pub const fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x4452494c4c534545 } else { seed },
        }
    }
    pub(crate) fn bounded(&mut self, n: u64) -> u64 {
        loop {
            let mut x = self.state;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.state = x;
            if x >= n.wrapping_neg() % n {
                return x % n;
            }
        }
    }
    pub fn next(&mut self, token: u64, profile: Profile) -> Option<Question> {
        let start = self.bounded(IndexId::COUNT as u64) as usize;
        let boundary = match self.bounded(3) {
            0 => Boundary::Below,
            1 => Boundary::At,
            _ => Boundary::Above,
        };
        for offset in 0..IndexId::COUNT {
            if let Some(q) = generate(
                token,
                IndexId::ALL[(start + offset) % IndexId::COUNT],
                profile,
                boundary,
            ) {
                return Some(q);
            }
        }
        None
    }
}

// RESUME10 Funding
impl crate::resume_private::Wire for Funding {
    fn initial() -> Self {
        Self {
            bankroll: crate::resume_private::Wire::initial(),
            original_wager: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            bankroll,
            original_wager,
        } = self;
        bankroll.put(w)?;
        original_wager.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            bankroll,
            original_wager,
        } = self;
        bankroll.read_into(r)?;
        original_wager.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            bankroll: crate::resume_private::Wire::get(r)?,
            original_wager: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 PublicContext
impl crate::resume_private::Wire for PublicContext {
    fn initial() -> Self {
        Self {
            hand: crate::resume_private::Wire::initial(),
            dealer_up: crate::resume_private::Wire::initial(),
            legal_mask: crate::resume_private::Wire::initial(),
            rules: crate::resume_private::Wire::initial(),
            count: crate::resume_private::Wire::initial(),
            funding: crate::resume_private::Wire::initial(),
            session: crate::resume_private::Wire::initial(),
            policy: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            hand,
            dealer_up,
            legal_mask,
            rules,
            count,
            funding,
            session,
            policy,
        } = self;
        hand.put(w)?;
        dealer_up.put(w)?;
        legal_mask.put(w)?;
        rules.put(w)?;
        count.put(w)?;
        funding.put(w)?;
        session.put(w)?;
        policy.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            hand,
            dealer_up,
            legal_mask,
            rules,
            count,
            funding,
            session,
            policy,
        } = self;
        hand.read_into(r)?;
        dealer_up.read_into(r)?;
        legal_mask.read_into(r)?;
        rules.read_into(r)?;
        count.read_into(r)?;
        funding.read_into(r)?;
        session.read_into(r)?;
        policy.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            hand: crate::resume_private::Wire::get(r)?,
            dealer_up: crate::resume_private::Wire::get(r)?,
            legal_mask: crate::resume_private::Wire::get(r)?,
            rules: crate::resume_private::Wire::get(r)?,
            count: crate::resume_private::Wire::get(r)?,
            funding: crate::resume_private::Wire::get(r)?,
            session: crate::resume_private::Wire::get(r)?,
            policy: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Question
impl crate::resume_private::Wire for Question {
    fn initial() -> Self {
        Self {
            token: crate::resume_private::Wire::initial(),
            index: crate::resume_private::Wire::initial(),
            context: crate::resume_private::Wire::initial(),
            origin: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            token,
            index,
            context,
            origin,
        } = self;
        token.put(w)?;
        index.put(w)?;
        context.put(w)?;
        origin.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            token,
            index,
            context,
            origin,
        } = self;
        token.read_into(r)?;
        index.read_into(r)?;
        context.read_into(r)?;
        origin.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            token: crate::resume_private::Wire::get(r)?,
            index: crate::resume_private::Wire::get(r)?,
            context: crate::resume_private::Wire::get(r)?,
            origin: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Reference
impl crate::resume_private::Wire for Reference {
    fn initial() -> Self {
        Self {
            best: crate::resume_private::Wire::initial(),
            basic: crate::resume_private::Wire::initial(),
            threshold: crate::resume_private::Wire::initial(),
            threshold_reached: crate::resume_private::Wire::initial(),
            integer_true_count: crate::resume_private::Wire::initial(),
            policy: crate::resume_private::Wire::initial(),
            normalized_legal_mask: crate::resume_private::Wire::initial(),
            insurance_affordable: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            best,
            basic,
            threshold,
            threshold_reached,
            integer_true_count,
            policy,
            normalized_legal_mask,
            insurance_affordable,
        } = self;
        best.put(w)?;
        basic.put(w)?;
        threshold.put(w)?;
        threshold_reached.put(w)?;
        integer_true_count.put(w)?;
        policy.put(w)?;
        normalized_legal_mask.put(w)?;
        insurance_affordable.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            best,
            basic,
            threshold,
            threshold_reached,
            integer_true_count,
            policy,
            normalized_legal_mask,
            insurance_affordable,
        } = self;
        best.read_into(r)?;
        basic.read_into(r)?;
        threshold.read_into(r)?;
        threshold_reached.read_into(r)?;
        integer_true_count.read_into(r)?;
        policy.read_into(r)?;
        normalized_legal_mask.read_into(r)?;
        insurance_affordable.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            best: crate::resume_private::Wire::get(r)?,
            basic: crate::resume_private::Wire::get(r)?,
            threshold: crate::resume_private::Wire::get(r)?,
            threshold_reached: crate::resume_private::Wire::get(r)?,
            integer_true_count: crate::resume_private::Wire::get(r)?,
            policy: crate::resume_private::Wire::get(r)?,
            normalized_legal_mask: crate::resume_private::Wire::get(r)?,
            insurance_affordable: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Attempt
impl crate::resume_private::Wire for Attempt {
    fn initial() -> Self {
        Self {
            question: crate::resume_private::Wire::initial(),
            answer: crate::resume_private::Wire::initial(),
            correct: crate::resume_private::Wire::initial(),
            assisted: crate::resume_private::Wire::initial(),
            reference: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            question,
            answer,
            correct,
            assisted,
            reference,
        } = self;
        question.put(w)?;
        answer.put(w)?;
        correct.put(w)?;
        assisted.put(w)?;
        reference.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            question,
            answer,
            correct,
            assisted,
            reference,
        } = self;
        question.read_into(r)?;
        answer.read_into(r)?;
        correct.read_into(r)?;
        assisted.read_into(r)?;
        reference.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            question: crate::resume_private::Wire::get(r)?,
            answer: crate::resume_private::Wire::get(r)?,
            correct: crate::resume_private::Wire::get(r)?,
            assisted: crate::resume_private::Wire::get(r)?,
            reference: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Totals
impl crate::resume_private::Wire for Totals {
    fn initial() -> Self {
        Self {
            all: crate::resume_private::Wire::initial(),
            indices: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self { all, indices } = self;
        all.put(w)?;
        indices.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self { all, indices } = self;
        all.read_into(r)?;
        indices.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            all: crate::resume_private::Wire::get(r)?,
            indices: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Stats
impl crate::resume_private::Wire for Stats {
    fn initial() -> Self {
        Self {
            first: crate::resume_private::Wire::initial(),
            replay: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self { first, replay } = self;
        first.put(w)?;
        replay.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self { first, replay } = self;
        first.read_into(r)?;
        replay.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            first: crate::resume_private::Wire::get(r)?,
            replay: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Pending
impl crate::resume_private::Wire for Pending {
    fn initial() -> Self {
        Self {
            question: crate::resume_private::Wire::initial(),
            reference: crate::resume_private::Wire::initial(),
            helped: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            question,
            reference,
            helped,
        } = self;
        question.put(w)?;
        reference.put(w)?;
        helped.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            question,
            reference,
            helped,
        } = self;
        question.read_into(r)?;
        reference.read_into(r)?;
        helped.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            question: crate::resume_private::Wire::get(r)?,
            reference: crate::resume_private::Wire::get(r)?,
            helped: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Model
impl crate::resume_private::Wire for Model {
    fn initial() -> Self {
        Self {
            pending: crate::resume_private::Wire::initial(),
            last_token: crate::resume_private::Wire::initial(),
            stats: crate::resume_private::Wire::initial(),
            history: crate::resume_private::Wire::initial(),
            start: crate::resume_private::Wire::initial(),
            len: crate::resume_private::Wire::initial(),
            restored: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            pending,
            last_token,
            stats,
            history,
            start,
            len,
            restored,
        } = self;
        pending.put(w)?;
        last_token.put(w)?;
        stats.put(w)?;
        history.put(w)?;
        start.put(w)?;
        len.put(w)?;
        restored.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            pending,
            last_token,
            stats,
            history,
            start,
            len,
            restored,
        } = self;
        pending.read_into(r)?;
        last_token.read_into(r)?;
        stats.read_into(r)?;
        history.read_into(r)?;
        start.read_into(r)?;
        len.read_into(r)?;
        restored.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            pending: crate::resume_private::Wire::get(r)?,
            last_token: crate::resume_private::Wire::get(r)?,
            stats: crate::resume_private::Wire::get(r)?,
            history: crate::resume_private::Wire::get(r)?,
            start: crate::resume_private::Wire::get(r)?,
            len: crate::resume_private::Wire::get(r)?,
            restored: crate::resume_private::Wire::get(r)?,
        })
    }
}
impl crate::resume_private::SnapshotCopy for Model {
    fn snapshot_copy_to(&self, out: &mut Self) {
        let Self {
            pending,
            last_token,
            stats,
            history,
            start,
            len,
            restored,
        } = self;
        crate::resume_private::SnapshotCopy::snapshot_copy_to(pending, &mut out.pending);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(last_token, &mut out.last_token);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(stats, &mut out.stats);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(history, &mut out.history);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(start, &mut out.start);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(len, &mut out.len);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(restored, &mut out.restored);
    }
}

// RESUME10 Generator
impl crate::resume_private::Wire for Generator {
    fn initial() -> Self {
        Self {
            state: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self { state } = self;
        state.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self { state } = self;
        state.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            state: crate::resume_private::Wire::get(r)?,
        })
    }
}
impl crate::resume_private::SnapshotCopy for Generator {
    fn snapshot_copy_to(&self, out: &mut Self) {
        let Self { state } = self;
        crate::resume_private::SnapshotCopy::snapshot_copy_to(state, &mut out.state);
    }
}

// RESUME10 enum IndexId
impl crate::resume_private::Wire for IndexId {
    fn initial() -> Self {
        Self::Tens4
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::Tens4),
            1 => Ok(Self::Tens5),
            2 => Ok(Self::Tens6),
            3 => Ok(Self::Soft19v4),
            4 => Ok(Self::Soft19v5),
            5 => Ok(Self::Soft19v6H17),
            6 => Ok(Self::Soft19v6S17),
            7 => Ok(Self::Soft17v2),
            8 => Ok(Self::Hard16v9),
            9 => Ok(Self::Hard16v10),
            10 => Ok(Self::Hard16vAceH17),
            11 => Ok(Self::Hard15v10),
            12 => Ok(Self::Hard15vAceH17),
            13 => Ok(Self::Hard13v2),
            14 => Ok(Self::Hard12v2),
            15 => Ok(Self::Hard12v3),
            16 => Ok(Self::Hard12v4),
            17 => Ok(Self::Hard11vAceS17),
            18 => Ok(Self::Hard10v10),
            19 => Ok(Self::Hard10vAceH17),
            20 => Ok(Self::Hard10vAceS17),
            21 => Ok(Self::Hard9v2),
            22 => Ok(Self::Hard9v7),
            23 => Ok(Self::Hard8v6),
            24 => Ok(Self::Surrender16v8),
            25 => Ok(Self::Surrender16v9),
            26 => Ok(Self::Surrender15v9),
            27 => Ok(Self::Surrender15v10),
            28 => Ok(Self::Surrender15vAceH17),
            29 => Ok(Self::Surrender15vAceS17),
            30 => Ok(Self::Insurance),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}

// RESUME10 enum Policy
impl crate::resume_private::Wire for Policy {
    fn initial() -> Self {
        Self::Bja2018HiLoTruncatedV1
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::Bja2018HiLoTruncatedV1),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}

// RESUME10 enum Boundary
impl crate::resume_private::Wire for Boundary {
    fn initial() -> Self {
        Self::Below
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::Below),
            1 => Ok(Self::At),
            2 => Ok(Self::Above),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}
impl crate::resume_private::Wire for Threshold {
    fn initial() -> Self {
        Self::AtLeast(0)
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        match self {
            Self::AtLeast(n) => {
                w.u8(0)?;
                crate::resume_private::Wire::put(n, w)
            }
            Self::AtMost(n) => {
                w.u8(1)?;
                crate::resume_private::Wire::put(n, w)
            }
            Self::PositiveRunning => w.u8(2),
            Self::NegativeRunning => w.u8(3),
        }
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::AtLeast(crate::resume_private::Wire::get(r)?)),
            1 => Ok(Self::AtMost(crate::resume_private::Wire::get(r)?)),
            2 => Ok(Self::PositiveRunning),
            3 => Ok(Self::NegativeRunning),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}
impl crate::resume_private::Wire for Origin {
    fn initial() -> Self {
        Self::First
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        match self {
            Self::First => w.u8(0),
            Self::Replay { original_token } => {
                w.u8(1)?;
                crate::resume_private::Wire::put(original_token, w)
            }
        }
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::First),
            1 => Ok(Self::Replay {
                original_token: crate::resume_private::Wire::get(r)?,
            }),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}
impl Model {
    pub(crate) fn resume_valid(&self) -> bool {
        if !self.stats.valid() || self.start >= HISTORY_CAPACITY || self.len > HISTORY_CAPACITY {
            return false;
        }
        if let Some(p) = self.pending
            && (!p.question.context.hand.resume_valid()
                || reference(p.question) != Some(p.reference)
                || p.question.token != self.last_token)
        {
            return false;
        }
        let mut token = 0;
        for i in 0..self.len {
            let Some(a) = self.history(i) else {
                return false;
            };
            if !a.question.context.hand.resume_valid()
                || a.question.token <= token
                || reference(a.question) != Some(a.reference)
            {
                return false;
            }
            token = a.question.token;
        }
        self.last_token >= token
    }
    pub(crate) fn resume_last_token(&self) -> u64 {
        self.last_token
    }
}
impl Generator {
    pub(crate) fn resume_valid(&self) -> bool {
        self.state != 0
    }
}
