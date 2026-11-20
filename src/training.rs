//! Immutable first-attempt grading, with no engine/hole/future-shoe access.
use crate::{
    cards::{Card, Hand},
    deviations::{self, Count},
    engine::Action,
    rules::{Profile, Rules},
    strategy,
};
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
#[repr(u8)]
pub enum Mode {
    #[default]
    Off,
    Test,
    Coach,
    Guide,
}
impl Mode {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::Test => "TEST",
            Self::Coach => "COACH",
            Self::Guide => "GUIDE",
        }
    }
    pub const fn next(self) -> Self {
        match self {
            Self::Off => Self::Test,
            Self::Test => Self::Coach,
            Self::Coach => Self::Guide,
            Self::Guide => Self::Off,
        }
    }
    pub const fn from_id(n: u8) -> Option<Self> {
        match n {
            0 => Some(Self::Off),
            1 => Some(Self::Test),
            2 => Some(Self::Coach),
            3 => Some(Self::Guide),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
#[repr(u8)]
pub enum Policy {
    #[default]
    Basic,
    HiLo,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Category {
    Hard,
    Soft,
    Pair,
    Insurance,
}
pub const HISTORY_CAPACITY: usize = 128;
pub const COUNTER_LIMIT: u64 = u64::MAX / 8;
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct Counters {
    pub decisions: u64,
    pub correct: u64,
    pub errors: u64,
    pub assisted: u64,
}
impl Counters {
    pub const EMPTY: Self = Self {
        decisions: 0,
        correct: 0,
        errors: 0,
        assisted: 0,
    };
    pub fn valid(self) -> bool {
        self.decisions <= COUNTER_LIMIT
            && self.correct <= self.decisions
            && self.errors <= self.decisions
            && self.assisted <= self.decisions
            && self.correct + self.errors + self.assisted == self.decisions
    }
    fn record(&mut self, correct: bool, assisted: bool) {
        self.decisions += 1;
        if assisted {
            self.assisted += 1
        } else if correct {
            self.correct += 1
        } else {
            self.errors += 1
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct Stats {
    pub all: Counters,
    pub streak: u64,
    pub best_streak: u64,
    pub categories: [Counters; 4],
}
impl Stats {
    pub const EMPTY: Self = Self {
        all: Counters::EMPTY,
        streak: 0,
        best_streak: 0,
        categories: [Counters::EMPTY; 4],
    };
    pub fn valid(self) -> bool {
        if !self.all.valid()
            || self.streak > self.best_streak
            || self.best_streak > self.all.correct
        {
            return false;
        }
        let mut sum = Counters::default();
        for c in self.categories {
            if !c.valid() {
                return false;
            }
            sum.decisions += c.decisions;
            sum.correct += c.correct;
            sum.errors += c.errors;
            sum.assisted += c.assisted;
        }
        sum == self.all
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Choice {
    Play(Action),
    Insurance(bool),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvModel {
    InfiniteReplacementV1,
}
#[derive(Clone, Copy)]
pub struct ExpectedValues {
    pub model: EvModel,
    pub original_wager: i64,
    pub legal_mask: u8,
    pub evaluated_mask: u8,
    pub best_mask: u8,
    pub initial_pair_funded: bool,
    pub values: [f64; 5],
}
impl ExpectedValues {
    pub fn complete(self) -> bool {
        self.legal_mask == self.evaluated_mask
    }
    pub fn valid(self, insurance: bool, legal_mask: u8, hand: &Hand, hand_index: u8) -> bool {
        if self.original_wager != hand.bet
            || self.original_wager < 500
            || self.original_wager > 1_000_000
            || self.original_wager % 500 != 0
            || self.legal_mask != legal_mask
            || self.legal_mask == 0
            || self.legal_mask & !31 != 0
            || self.evaluated_mask & !self.legal_mask != 0
            || self.best_mask & !self.evaluated_mask != 0
        {
            return false;
        }
        if insurance {
            if self.legal_mask & !3 != 0
                || self.legal_mask & 1 == 0
                || self.initial_pair_funded
                || !self.complete()
            {
                return false;
            }
        } else {
            if self.legal_mask & Action::Stand.bit() == 0 {
                return false;
            }
            if (self.evaluated_mask & Action::Split.bit() != 0) != self.initial_pair_funded {
                return false;
            }
            if self.initial_pair_funded
                && (hand.split
                    || hand.split_aces
                    || hand.count() != 2
                    || hand_index != 0
                    || hand.cards()[0].value() != hand.cards()[1].value())
            {
                return false;
            }
        }
        if !self.complete()
            && ((self.legal_mask & !self.evaluated_mask) != Action::Split.bit()
                || self.best_mask != 0)
        {
            return false;
        }
        let mut best = f64::NEG_INFINITY;
        for (i, v) in self.values.into_iter().enumerate() {
            if self.evaluated_mask & (1 << i) == 0 {
                if !v.is_nan() {
                    return false;
                }
                continue;
            }
            let (lower, upper) = if insurance {
                (-0.5, 1.0)
            } else if i == Action::Split as usize {
                (-8.0, 8.0)
            } else {
                (-2.0, 2.0)
            };
            if !v.is_finite() || v < lower - 1e-8 || v > upper + 1e-8 {
                return false;
            }
            if insurance && i == 0 && v.abs() > 1e-8 {
                return false;
            }
            if !insurance && i == Action::Surrender as usize && (v + 0.5).abs() > 1e-8 {
                return false;
            }
            best = best.max(v);
        }
        if self.complete() {
            if self.best_mask == 0 {
                return false;
            }
            for (i, v) in self.values.into_iter().enumerate() {
                if self.best_mask & (1 << i) != 0 && best - v > 2e-8 {
                    return false;
                }
            }
        }
        true
    }
}
#[derive(Clone, Copy)]
pub struct Attempt {
    pub token: u64,
    pub round: u32,
    pub hand_index: u8,
    pub profile: Profile,
    pub hand: Hand,
    pub dealer_up: Card,
    pub legal_mask: u8,
    pub choice: Choice,
    pub policy: Policy,
    pub count: Option<Count>,
    pub assisted: bool,
    pub values: Option<ExpectedValues>,
}
#[derive(Clone, Copy)]
pub struct Decision {
    pub attempt: Attempt,
    pub category: Category,
    pub best: Choice,
    pub acceptable_mask: u8,
    pub correct: bool,
    pub assisted: bool,
    pub explanation: &'static str,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Off,
    Unavailable,
    Duplicate,
    Recorded,
}
#[derive(Clone, Copy)]
pub struct Grade {
    pub status: Status,
    pub decision: Option<Decision>,
    pub pause: bool,
}
impl Grade {
    fn empty(status: Status) -> Self {
        Self {
            status,
            decision: None,
            pause: false,
        }
    }
}
pub struct Model {
    pub mode: Mode,
    pub pause_on_mistake: bool,
    stats: Stats,
    history: [Option<Decision>; HISTORY_CAPACITY],
    start: usize,
    len: usize,
    last_token: Option<u64>,
    latest_round: Option<u32>,
    latest: Counters,
}
impl Model {
    pub const EMPTY: Self = Self {
        mode: Mode::Off,
        pause_on_mistake: false,
        stats: Stats::EMPTY,
        history: [None; HISTORY_CAPACITY],
        start: 0,
        len: 0,
        last_token: None,
        latest_round: None,
        latest: Counters::EMPTY,
    };
}
impl Default for Model {
    fn default() -> Self {
        Self::EMPTY
    }
}
impl Model {
    pub fn stats(&self) -> Stats {
        self.stats
    }
    pub fn history_count(&self) -> usize {
        self.len
    }
    pub fn history(&self, index: usize) -> Option<&Decision> {
        if index >= self.len {
            None
        } else {
            self.history[(self.start + index) % HISTORY_CAPACITY].as_ref()
        }
    }
    pub fn newest(&self, offset: usize) -> Option<&Decision> {
        offset
            .checked_add(1)
            .and_then(|n| self.len.checked_sub(n))
            .and_then(|i| self.history(i))
    }
    pub fn round_stats(&self, round: u32) -> Counters {
        if self.latest_round == Some(round) {
            return self.latest;
        }
        let mut result = Counters::default();
        for i in 0..self.len {
            let d = self.history(i).unwrap();
            if d.attempt.round == round {
                result.record(d.correct, d.assisted)
            }
        }
        result
    }
    pub fn record(&mut self, input: Attempt) -> Grade {
        if self.mode == Mode::Off {
            return Grade::empty(Status::Off);
        }
        if self.last_token.is_some_and(|t| input.token <= t) {
            let decision = (0..self.len).find_map(|i| {
                self.history(i)
                    .filter(|d| d.attempt.token == input.token)
                    .copied()
            });
            return Grade {
                status: Status::Duplicate,
                decision,
                pause: false,
            };
        }
        if input.hand_index >= 4
            || input.hand.count() < 2
            || self.stats.all.decisions >= COUNTER_LIMIT
        {
            return Grade::empty(Status::Unavailable);
        }
        let rules = Rules::for_profile(input.profile);
        if (input.policy == Policy::Basic && input.count.is_some())
            || (input.policy == Policy::HiLo && !input.count.is_some_and(|c| c.valid(rules)))
        {
            return Grade::empty(Status::Unavailable);
        }
        let (best, mask, explanation, category, correct) = match input.choice {
            Choice::Play(choice) => {
                if input.legal_mask & !31 != 0
                    || strategy::normalized_mask(&input.hand, input.legal_mask, rules)
                        & choice.bit()
                        == 0
                {
                    return Grade::empty(Status::Unavailable);
                }
                let advice = if input.policy == Policy::HiLo {
                    deviations::recommend(
                        &input.hand,
                        input.dealer_up,
                        input.legal_mask,
                        rules,
                        input.count.unwrap(),
                    )
                    .play
                } else {
                    strategy::recommend(&input.hand, input.dealer_up, input.legal_mask, rules)
                };
                let Some(advice) = advice else {
                    return Grade::empty(Status::Unavailable);
                };
                let category = if input.hand.count() == 2
                    && input.hand.cards()[0].value() == input.hand.cards()[1].value()
                {
                    Category::Pair
                } else if input.hand.soft() {
                    Category::Soft
                } else {
                    Category::Hard
                };
                (
                    Choice::Play(advice.best),
                    advice.acceptable_mask,
                    advice.explanation,
                    category,
                    advice.accepts(choice),
                )
            }
            Choice::Insurance(yes) => {
                if input.dealer_up.rank() != 14
                    || input.legal_mask & 1 == 0
                    || input.legal_mask & !3 != 0
                    || (yes && input.legal_mask & 2 == 0)
                {
                    return Grade::empty(Status::Unavailable);
                }
                let affordable = input.legal_mask & 2 != 0;
                let best = if input.policy == Policy::HiLo {
                    deviations::insurance(affordable, rules, input.count.unwrap()).unwrap()
                } else {
                    false
                };
                (
                    Choice::Insurance(best),
                    if best { 2 } else { 1 },
                    if input.policy == Policy::HiLo {
                        "Hi-Lo insurance: take at TC+3 or above when affordable."
                    } else {
                        "Basic strategy declines insurance and even money."
                    },
                    Category::Insurance,
                    yes == best,
                )
            }
        };
        if input.values.is_some_and(|v| {
            !v.valid(
                matches!(input.choice, Choice::Insurance(_)),
                input.legal_mask,
                &input.hand,
                input.hand_index,
            )
        }) {
            return Grade::empty(Status::Unavailable);
        }
        let assisted = input.assisted || self.mode == Mode::Guide;
        let decision = Decision {
            attempt: input,
            category,
            best,
            acceptable_mask: mask,
            correct,
            assisted,
            explanation,
        };
        if self.len == HISTORY_CAPACITY {
            self.start = (self.start + 1) % HISTORY_CAPACITY;
            self.len -= 1
        }
        self.history[(self.start + self.len) % HISTORY_CAPACITY] = Some(decision);
        self.len += 1;
        self.last_token = Some(input.token);
        if self.latest_round != Some(input.round) {
            self.latest_round = Some(input.round);
            self.latest = Counters::default()
        }
        self.latest.record(correct, assisted);
        self.stats.all.record(correct, assisted);
        self.stats.categories[category as usize].record(correct, assisted);
        if correct && !assisted {
            self.stats.streak += 1;
            self.stats.best_streak = self.stats.best_streak.max(self.stats.streak)
        } else {
            self.stats.streak = 0
        }
        Grade {
            status: Status::Recorded,
            decision: Some(decision),
            pause: self.mode == Mode::Coach && self.pause_on_mistake && !correct,
        }
    }
    pub fn restore(&mut self, stats: Stats, pause: bool) -> bool {
        if self.last_token.is_some() || !stats.valid() {
            return false;
        }
        *self = Self::default();
        self.stats = stats;
        self.pause_on_mistake = pause;
        true
    }
    pub fn reset_stats(&mut self, between_rounds: bool) -> bool {
        if !between_rounds {
            return false;
        }
        self.stats = Stats::default();
        self.history.fill(None);
        self.start = 0;
        self.len = 0;
        self.latest_round = None;
        self.latest = Counters::default();
        true
    }
}

// RESUME10 Counters
impl crate::resume_private::Wire for Counters {
    fn initial() -> Self {
        Self {
            decisions: crate::resume_private::Wire::initial(),
            correct: crate::resume_private::Wire::initial(),
            errors: crate::resume_private::Wire::initial(),
            assisted: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            decisions,
            correct,
            errors,
            assisted,
        } = self;
        decisions.put(w)?;
        correct.put(w)?;
        errors.put(w)?;
        assisted.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            decisions,
            correct,
            errors,
            assisted,
        } = self;
        decisions.read_into(r)?;
        correct.read_into(r)?;
        errors.read_into(r)?;
        assisted.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            decisions: crate::resume_private::Wire::get(r)?,
            correct: crate::resume_private::Wire::get(r)?,
            errors: crate::resume_private::Wire::get(r)?,
            assisted: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Stats
impl crate::resume_private::Wire for Stats {
    fn initial() -> Self {
        Self {
            all: crate::resume_private::Wire::initial(),
            streak: crate::resume_private::Wire::initial(),
            best_streak: crate::resume_private::Wire::initial(),
            categories: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            all,
            streak,
            best_streak,
            categories,
        } = self;
        all.put(w)?;
        streak.put(w)?;
        best_streak.put(w)?;
        categories.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            all,
            streak,
            best_streak,
            categories,
        } = self;
        all.read_into(r)?;
        streak.read_into(r)?;
        best_streak.read_into(r)?;
        categories.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            all: crate::resume_private::Wire::get(r)?,
            streak: crate::resume_private::Wire::get(r)?,
            best_streak: crate::resume_private::Wire::get(r)?,
            categories: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 ExpectedValues
impl crate::resume_private::Wire for ExpectedValues {
    fn initial() -> Self {
        Self {
            model: crate::resume_private::Wire::initial(),
            original_wager: crate::resume_private::Wire::initial(),
            legal_mask: crate::resume_private::Wire::initial(),
            evaluated_mask: crate::resume_private::Wire::initial(),
            best_mask: crate::resume_private::Wire::initial(),
            initial_pair_funded: crate::resume_private::Wire::initial(),
            values: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            model,
            original_wager,
            legal_mask,
            evaluated_mask,
            best_mask,
            initial_pair_funded,
            values,
        } = self;
        model.put(w)?;
        original_wager.put(w)?;
        legal_mask.put(w)?;
        evaluated_mask.put(w)?;
        best_mask.put(w)?;
        initial_pair_funded.put(w)?;
        values.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            model,
            original_wager,
            legal_mask,
            evaluated_mask,
            best_mask,
            initial_pair_funded,
            values,
        } = self;
        model.read_into(r)?;
        original_wager.read_into(r)?;
        legal_mask.read_into(r)?;
        evaluated_mask.read_into(r)?;
        best_mask.read_into(r)?;
        initial_pair_funded.read_into(r)?;
        values.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            model: crate::resume_private::Wire::get(r)?,
            original_wager: crate::resume_private::Wire::get(r)?,
            legal_mask: crate::resume_private::Wire::get(r)?,
            evaluated_mask: crate::resume_private::Wire::get(r)?,
            best_mask: crate::resume_private::Wire::get(r)?,
            initial_pair_funded: crate::resume_private::Wire::get(r)?,
            values: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Attempt
impl crate::resume_private::Wire for Attempt {
    fn initial() -> Self {
        Self {
            token: crate::resume_private::Wire::initial(),
            round: crate::resume_private::Wire::initial(),
            hand_index: crate::resume_private::Wire::initial(),
            profile: crate::resume_private::Wire::initial(),
            hand: crate::resume_private::Wire::initial(),
            dealer_up: crate::resume_private::Wire::initial(),
            legal_mask: crate::resume_private::Wire::initial(),
            choice: crate::resume_private::Wire::initial(),
            policy: crate::resume_private::Wire::initial(),
            count: crate::resume_private::Wire::initial(),
            assisted: crate::resume_private::Wire::initial(),
            values: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            token,
            round,
            hand_index,
            profile,
            hand,
            dealer_up,
            legal_mask,
            choice,
            policy,
            count,
            assisted,
            values,
        } = self;
        token.put(w)?;
        round.put(w)?;
        hand_index.put(w)?;
        profile.put(w)?;
        hand.put(w)?;
        dealer_up.put(w)?;
        legal_mask.put(w)?;
        choice.put(w)?;
        policy.put(w)?;
        count.put(w)?;
        assisted.put(w)?;
        values.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            token,
            round,
            hand_index,
            profile,
            hand,
            dealer_up,
            legal_mask,
            choice,
            policy,
            count,
            assisted,
            values,
        } = self;
        token.read_into(r)?;
        round.read_into(r)?;
        hand_index.read_into(r)?;
        profile.read_into(r)?;
        hand.read_into(r)?;
        dealer_up.read_into(r)?;
        legal_mask.read_into(r)?;
        choice.read_into(r)?;
        policy.read_into(r)?;
        count.read_into(r)?;
        assisted.read_into(r)?;
        values.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            token: crate::resume_private::Wire::get(r)?,
            round: crate::resume_private::Wire::get(r)?,
            hand_index: crate::resume_private::Wire::get(r)?,
            profile: crate::resume_private::Wire::get(r)?,
            hand: crate::resume_private::Wire::get(r)?,
            dealer_up: crate::resume_private::Wire::get(r)?,
            legal_mask: crate::resume_private::Wire::get(r)?,
            choice: crate::resume_private::Wire::get(r)?,
            policy: crate::resume_private::Wire::get(r)?,
            count: crate::resume_private::Wire::get(r)?,
            assisted: crate::resume_private::Wire::get(r)?,
            values: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Decision
impl crate::resume_private::Wire for Decision {
    fn initial() -> Self {
        Self {
            attempt: crate::resume_private::Wire::initial(),
            category: crate::resume_private::Wire::initial(),
            best: crate::resume_private::Wire::initial(),
            acceptable_mask: crate::resume_private::Wire::initial(),
            correct: crate::resume_private::Wire::initial(),
            assisted: crate::resume_private::Wire::initial(),
            explanation: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            attempt,
            category,
            best,
            acceptable_mask,
            correct,
            assisted,
            explanation,
        } = self;
        attempt.put(w)?;
        category.put(w)?;
        best.put(w)?;
        acceptable_mask.put(w)?;
        correct.put(w)?;
        assisted.put(w)?;
        explanation.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            attempt,
            category,
            best,
            acceptable_mask,
            correct,
            assisted,
            explanation,
        } = self;
        attempt.read_into(r)?;
        category.read_into(r)?;
        best.read_into(r)?;
        acceptable_mask.read_into(r)?;
        correct.read_into(r)?;
        assisted.read_into(r)?;
        explanation.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            attempt: crate::resume_private::Wire::get(r)?,
            category: crate::resume_private::Wire::get(r)?,
            best: crate::resume_private::Wire::get(r)?,
            acceptable_mask: crate::resume_private::Wire::get(r)?,
            correct: crate::resume_private::Wire::get(r)?,
            assisted: crate::resume_private::Wire::get(r)?,
            explanation: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Model
impl crate::resume_private::Wire for Model {
    fn initial() -> Self {
        Self {
            mode: crate::resume_private::Wire::initial(),
            pause_on_mistake: crate::resume_private::Wire::initial(),
            stats: crate::resume_private::Wire::initial(),
            history: crate::resume_private::Wire::initial(),
            start: crate::resume_private::Wire::initial(),
            len: crate::resume_private::Wire::initial(),
            last_token: crate::resume_private::Wire::initial(),
            latest_round: crate::resume_private::Wire::initial(),
            latest: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            mode,
            pause_on_mistake,
            stats,
            history,
            start,
            len,
            last_token,
            latest_round,
            latest,
        } = self;
        mode.put(w)?;
        pause_on_mistake.put(w)?;
        stats.put(w)?;
        history.put(w)?;
        start.put(w)?;
        len.put(w)?;
        last_token.put(w)?;
        latest_round.put(w)?;
        latest.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            mode,
            pause_on_mistake,
            stats,
            history,
            start,
            len,
            last_token,
            latest_round,
            latest,
        } = self;
        mode.read_into(r)?;
        pause_on_mistake.read_into(r)?;
        stats.read_into(r)?;
        history.read_into(r)?;
        start.read_into(r)?;
        len.read_into(r)?;
        last_token.read_into(r)?;
        latest_round.read_into(r)?;
        latest.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            mode: crate::resume_private::Wire::get(r)?,
            pause_on_mistake: crate::resume_private::Wire::get(r)?,
            stats: crate::resume_private::Wire::get(r)?,
            history: crate::resume_private::Wire::get(r)?,
            start: crate::resume_private::Wire::get(r)?,
            len: crate::resume_private::Wire::get(r)?,
            last_token: crate::resume_private::Wire::get(r)?,
            latest_round: crate::resume_private::Wire::get(r)?,
            latest: crate::resume_private::Wire::get(r)?,
        })
    }
}
impl crate::resume_private::SnapshotCopy for Model {
    fn snapshot_copy_to(&self, out: &mut Self) {
        let Self {
            mode,
            pause_on_mistake,
            stats,
            history,
            start,
            len,
            last_token,
            latest_round,
            latest,
        } = self;
        crate::resume_private::SnapshotCopy::snapshot_copy_to(mode, &mut out.mode);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(
            pause_on_mistake,
            &mut out.pause_on_mistake,
        );
        crate::resume_private::SnapshotCopy::snapshot_copy_to(stats, &mut out.stats);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(history, &mut out.history);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(start, &mut out.start);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(len, &mut out.len);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(last_token, &mut out.last_token);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(latest_round, &mut out.latest_round);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(latest, &mut out.latest);
    }
}

// RESUME10 enum Mode
impl crate::resume_private::Wire for Mode {
    fn initial() -> Self {
        Self::Off
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::Off),
            1 => Ok(Self::Test),
            2 => Ok(Self::Coach),
            3 => Ok(Self::Guide),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}

// RESUME10 enum Policy
impl crate::resume_private::Wire for Policy {
    fn initial() -> Self {
        Self::Basic
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::Basic),
            1 => Ok(Self::HiLo),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}

// RESUME10 enum Category
impl crate::resume_private::Wire for Category {
    fn initial() -> Self {
        Self::Hard
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::Hard),
            1 => Ok(Self::Soft),
            2 => Ok(Self::Pair),
            3 => Ok(Self::Insurance),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}

// RESUME10 enum EvModel
impl crate::resume_private::Wire for EvModel {
    fn initial() -> Self {
        Self::InfiniteReplacementV1
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::InfiniteReplacementV1),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}

impl crate::resume_private::Wire for Choice {
    fn initial() -> Self {
        Self::Play(Action::Hit)
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        match self {
            Self::Play(a) => {
                w.u8(0)?;
                a.put(w)
            }
            Self::Insurance(b) => {
                w.u8(1)?;
                b.put(w)
            }
        }
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        use crate::resume_private::Wire;
        match r.u8()? {
            0 => Ok(Self::Play(Wire::get(r)?)),
            1 => Ok(Self::Insurance(Wire::get(r)?)),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}

impl Decision {
    pub(crate) fn resume_valid(&self) -> bool {
        let a = self.attempt;
        if !a.hand.resume_valid() {
            return false;
        }
        let mut m = Model::EMPTY;
        m.mode = if self.assisted && !a.assisted {
            Mode::Guide
        } else {
            Mode::Test
        };
        let Some(d) = m.record(a).decision else {
            return false;
        };
        d.category == self.category
            && d.best == self.best
            && d.acceptable_mask == self.acceptable_mask
            && d.correct == self.correct
            && d.assisted == self.assisted
            && d.explanation == self.explanation
    }
}

impl Model {
    pub(crate) fn resume_valid(&self) -> bool {
        if !self.stats.valid()
            || !self.latest.valid()
            || self.start >= HISTORY_CAPACITY
            || self.len > HISTORY_CAPACITY
        {
            return false;
        }
        let mut token = None;
        for i in 0..self.len {
            let Some(d) = self.history(i) else {
                return false;
            };
            if !d.resume_valid() || token.is_some_and(|t| d.attempt.token <= t) {
                return false;
            }
            token = Some(d.attempt.token);
        }
        self.last_token >= token
    }
}

impl Model {
    pub(crate) fn resume_last_token(&self) -> Option<u64> {
        self.last_token
    }
}

impl ExpectedValues {
    pub(crate) fn resume_valid(&self) -> bool {
        self.original_wager >= 500
            && self.original_wager <= 1_000_000
            && self.original_wager % 500 == 0
            && self.legal_mask != 0
            && self.legal_mask & !31 == 0
            && self.evaluated_mask & !self.legal_mask == 0
            && self.best_mask & !self.evaluated_mask == 0
            && self.values.iter().enumerate().all(|(i, v)| {
                if self.evaluated_mask & (1 << i) == 0 {
                    v.is_nan()
                } else {
                    v.is_finite() && *v >= -8.0000001 && *v <= 8.0000001
                }
            })
    }
}
