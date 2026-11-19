//! Session-only immutable first-answer metrics. No engine, cards-in-shoe, RNG,
//! forecast or regrading API. The caller supplies already-recorded decisions.
use crate::{
    rules::Profile,
    training::{self, COUNTER_LIMIT, Choice, Counters, Decision, EvModel, Policy},
    wager_practice,
};
pub const HISTORY: usize = 128;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Skip {
    HiLoPolicy,
    NoSnapshot,
    PartialSnapshot,
    InvalidSnapshot,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Comparison {
    pub model: EvModel,
    pub original_wager: i64,
    pub chosen_ev: f64,
    pub best_ev: f64,
    pub difference_cents: f64,
    pub initial_pair_funded: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FirstAnswer {
    pub hand: crate::cards::Hand,
    pub dealer_up: crate::cards::Card,
    pub count: Option<crate::deviations::Count>,
    pub token: u64,
    pub round: u32,
    pub profile: Profile,
    pub policy: Policy,
    pub category: training::Category,
    pub choice: Choice,
    pub policy_best: Choice,
    pub correct: bool,
    pub assisted: bool,
    pub original_wager: i64,
    pub comparison: Option<Comparison>,
    pub skipped: Option<Skip>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bucket {
    pub decisions: u64,
    pub difference_cents: f64,
    pub mean_difference_cents: Option<f64>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Costs {
    pub unassisted: Bucket,
    pub assisted: Bucket,
    pub skipped_hilo: u64,
    pub unavailable: u64,
}
#[derive(Clone, Copy)]
struct Sum {
    count: u64,
    total: f64,
    compensation: f64,
}
impl Sum {
    const EMPTY: Self = Self {
        count: 0,
        total: 0.0,
        compensation: 0.0,
    };
    fn add(&mut self, value: f64) {
        let adjusted = value - self.compensation;
        let next = self.total + adjusted;
        self.compensation = (next - self.total) - adjusted;
        self.total = next;
        self.count += 1
    }
    fn bucket(self) -> Bucket {
        Bucket {
            decisions: self.count,
            difference_cents: self.total,
            mean_difference_cents: if self.count == 0 {
                None
            } else {
                Some(self.total / self.count as f64)
            },
        }
    }
}
pub struct Metrics {
    card: training::Stats,
    wager: wager_practice::Stats,
    last_card: u64,
    last_wager: u64,
    unassisted: Sum,
    assisted: Sum,
    skipped_hilo: u64,
    unavailable: u64,
    history: [Option<FirstAnswer>; HISTORY],
    start: usize,
    len: usize,
}
impl Default for Metrics {
    fn default() -> Self {
        Self::EMPTY
    }
}
fn count(c: &mut Counters, correct: bool, assisted: bool) {
    c.decisions += 1;
    if assisted {
        c.assisted += 1
    } else if correct {
        c.correct += 1
    } else {
        c.errors += 1
    }
}
fn comparison(d: Decision) -> Result<Comparison, Skip> {
    // Hi-Lo policy correctness must never be relabeled by fixed replacement
    // odds (insurance at a high index is the canonical counterexample).
    if d.attempt.policy == Policy::HiLo {
        return Err(Skip::HiLoPolicy);
    }
    let Some(v) = d.attempt.values else {
        return Err(Skip::NoSnapshot);
    };
    if v.model != EvModel::InfiniteReplacementV1
        || !v.valid(
            matches!(d.attempt.choice, Choice::Insurance(_)),
            d.attempt.legal_mask,
            &d.attempt.hand,
            d.attempt.hand_index,
        )
    {
        return Err(Skip::InvalidSnapshot);
    }
    if !v.complete() {
        return Err(Skip::PartialSnapshot);
    }
    let index = match d.attempt.choice {
        Choice::Play(action) => action as usize,
        Choice::Insurance(yes) => usize::from(yes),
    };
    if v.evaluated_mask & (1 << index) == 0 {
        return Err(Skip::InvalidSnapshot);
    }
    let mut best = f64::NEG_INFINITY;
    for (i, value) in v.values.into_iter().enumerate() {
        if v.evaluated_mask & (1 << i) != 0 {
            best = best.max(value)
        }
    }
    let Some(cost) =
        crate::session_analysis::decision_cost(best, v.values[index], v.original_wager, d.assisted)
    else {
        return Err(Skip::InvalidSnapshot);
    };
    Ok(Comparison {
        model: v.model,
        original_wager: v.original_wager,
        chosen_ev: v.values[index],
        best_ev: best,
        difference_cents: cost.cents,
        initial_pair_funded: v.initial_pair_funded,
    })
}
impl Metrics {
    pub const EMPTY: Self = Self {
        card: training::Stats::EMPTY,
        wager: wager_practice::Stats::EMPTY,
        last_card: 0,
        last_wager: 0,
        unassisted: Sum::EMPTY,
        assisted: Sum::EMPTY,
        skipped_hilo: 0,
        unavailable: 0,
        history: [None; HISTORY],
        start: 0,
        len: 0,
    };
    pub fn card_stats(&self) -> training::Stats {
        self.card
    }
    pub fn wager_stats(&self) -> wager_practice::Stats {
        self.wager
    }
    pub fn costs(&self) -> Costs {
        Costs {
            unassisted: self.unassisted.bucket(),
            assisted: self.assisted.bucket(),
            skipped_hilo: self.skipped_hilo,
            unavailable: self.unavailable,
        }
    }
    pub fn history_count(&self) -> usize {
        self.len
    }
    pub fn newest(&self, offset: usize) -> Option<FirstAnswer> {
        if offset >= self.len {
            None
        } else {
            self.history[(self.start + self.len - 1 - offset) % HISTORY]
        }
    }
    pub fn record_card(&mut self, d: Decision) -> bool {
        if d.attempt.token <= self.last_card || self.card.all.decisions >= COUNTER_LIMIT {
            return false;
        }
        let value = comparison(d);
        let (recorded, skipped) = match value {
            Ok(value) => {
                if d.assisted {
                    self.assisted.add(value.difference_cents)
                } else {
                    self.unassisted.add(value.difference_cents)
                }
                (Some(value), None)
            }
            Err(skip) => {
                if skip == Skip::HiLoPolicy {
                    self.skipped_hilo += 1
                } else {
                    self.unavailable += 1
                }
                (None, Some(skip))
            }
        };
        let evidence = FirstAnswer {
            hand: d.attempt.hand,
            dealer_up: d.attempt.dealer_up,
            count: d.attempt.count,
            token: d.attempt.token,
            round: d.attempt.round,
            profile: d.attempt.profile,
            policy: d.attempt.policy,
            category: d.category,
            choice: d.attempt.choice,
            policy_best: d.best,
            correct: d.correct,
            assisted: d.assisted,
            original_wager: d.attempt.hand.bet,
            comparison: recorded,
            skipped,
        };
        self.last_card = d.attempt.token;
        count(&mut self.card.all, d.correct, d.assisted);
        count(
            &mut self.card.categories[d.category as usize],
            d.correct,
            d.assisted,
        );
        if d.correct && !d.assisted {
            self.card.streak += 1;
            self.card.best_streak = self.card.best_streak.max(self.card.streak)
        } else {
            self.card.streak = 0
        }
        if self.len == HISTORY {
            self.start = (self.start + 1) % HISTORY;
            self.len -= 1
        }
        self.history[(self.start + self.len) % HISTORY] = Some(evidence);
        self.len += 1;
        true
    }
    pub fn record_wager(&mut self, a: wager_practice::Attempt) -> bool {
        if a.question.token <= self.last_wager || self.wager.all.decisions >= COUNTER_LIMIT {
            return false;
        }
        self.last_wager = a.question.token;
        count(&mut self.wager.all, a.correct, a.assisted);
        count(
            &mut self.wager.categories[a.category as usize],
            a.correct,
            a.assisted,
        );
        true
    }
}

// RESUME10 Comparison
impl crate::resume_private::Wire for Comparison {
    fn initial() -> Self {
        Self {
            model: crate::resume_private::Wire::initial(),
            original_wager: crate::resume_private::Wire::initial(),
            chosen_ev: crate::resume_private::Wire::initial(),
            best_ev: crate::resume_private::Wire::initial(),
            difference_cents: crate::resume_private::Wire::initial(),
            initial_pair_funded: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            model,
            original_wager,
            chosen_ev,
            best_ev,
            difference_cents,
            initial_pair_funded,
        } = self;
        model.put(w)?;
        original_wager.put(w)?;
        chosen_ev.put(w)?;
        best_ev.put(w)?;
        difference_cents.put(w)?;
        initial_pair_funded.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            model,
            original_wager,
            chosen_ev,
            best_ev,
            difference_cents,
            initial_pair_funded,
        } = self;
        model.read_into(r)?;
        original_wager.read_into(r)?;
        chosen_ev.read_into(r)?;
        best_ev.read_into(r)?;
        difference_cents.read_into(r)?;
        initial_pair_funded.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            model: crate::resume_private::Wire::get(r)?,
            original_wager: crate::resume_private::Wire::get(r)?,
            chosen_ev: crate::resume_private::Wire::get(r)?,
            best_ev: crate::resume_private::Wire::get(r)?,
            difference_cents: crate::resume_private::Wire::get(r)?,
            initial_pair_funded: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 FirstAnswer
impl crate::resume_private::Wire for FirstAnswer {
    fn initial() -> Self {
        Self {
            hand: crate::resume_private::Wire::initial(),
            dealer_up: crate::resume_private::Wire::initial(),
            count: crate::resume_private::Wire::initial(),
            token: crate::resume_private::Wire::initial(),
            round: crate::resume_private::Wire::initial(),
            profile: crate::resume_private::Wire::initial(),
            policy: crate::resume_private::Wire::initial(),
            category: crate::resume_private::Wire::initial(),
            choice: crate::resume_private::Wire::initial(),
            policy_best: crate::resume_private::Wire::initial(),
            correct: crate::resume_private::Wire::initial(),
            assisted: crate::resume_private::Wire::initial(),
            original_wager: crate::resume_private::Wire::initial(),
            comparison: crate::resume_private::Wire::initial(),
            skipped: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            hand,
            dealer_up,
            count,
            token,
            round,
            profile,
            policy,
            category,
            choice,
            policy_best,
            correct,
            assisted,
            original_wager,
            comparison,
            skipped,
        } = self;
        hand.put(w)?;
        dealer_up.put(w)?;
        count.put(w)?;
        token.put(w)?;
        round.put(w)?;
        profile.put(w)?;
        policy.put(w)?;
        category.put(w)?;
        choice.put(w)?;
        policy_best.put(w)?;
        correct.put(w)?;
        assisted.put(w)?;
        original_wager.put(w)?;
        comparison.put(w)?;
        skipped.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            hand,
            dealer_up,
            count,
            token,
            round,
            profile,
            policy,
            category,
            choice,
            policy_best,
            correct,
            assisted,
            original_wager,
            comparison,
            skipped,
        } = self;
        hand.read_into(r)?;
        dealer_up.read_into(r)?;
        count.read_into(r)?;
        token.read_into(r)?;
        round.read_into(r)?;
        profile.read_into(r)?;
        policy.read_into(r)?;
        category.read_into(r)?;
        choice.read_into(r)?;
        policy_best.read_into(r)?;
        correct.read_into(r)?;
        assisted.read_into(r)?;
        original_wager.read_into(r)?;
        comparison.read_into(r)?;
        skipped.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            hand: crate::resume_private::Wire::get(r)?,
            dealer_up: crate::resume_private::Wire::get(r)?,
            count: crate::resume_private::Wire::get(r)?,
            token: crate::resume_private::Wire::get(r)?,
            round: crate::resume_private::Wire::get(r)?,
            profile: crate::resume_private::Wire::get(r)?,
            policy: crate::resume_private::Wire::get(r)?,
            category: crate::resume_private::Wire::get(r)?,
            choice: crate::resume_private::Wire::get(r)?,
            policy_best: crate::resume_private::Wire::get(r)?,
            correct: crate::resume_private::Wire::get(r)?,
            assisted: crate::resume_private::Wire::get(r)?,
            original_wager: crate::resume_private::Wire::get(r)?,
            comparison: crate::resume_private::Wire::get(r)?,
            skipped: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Sum
impl crate::resume_private::Wire for Sum {
    fn initial() -> Self {
        Self {
            count: crate::resume_private::Wire::initial(),
            total: crate::resume_private::Wire::initial(),
            compensation: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            count,
            total,
            compensation,
        } = self;
        count.put(w)?;
        total.put(w)?;
        compensation.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            count,
            total,
            compensation,
        } = self;
        count.read_into(r)?;
        total.read_into(r)?;
        compensation.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            count: crate::resume_private::Wire::get(r)?,
            total: crate::resume_private::Wire::get(r)?,
            compensation: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Metrics
impl crate::resume_private::Wire for Metrics {
    fn initial() -> Self {
        Self {
            card: crate::resume_private::Wire::initial(),
            wager: crate::resume_private::Wire::initial(),
            last_card: crate::resume_private::Wire::initial(),
            last_wager: crate::resume_private::Wire::initial(),
            unassisted: crate::resume_private::Wire::initial(),
            assisted: crate::resume_private::Wire::initial(),
            skipped_hilo: crate::resume_private::Wire::initial(),
            unavailable: crate::resume_private::Wire::initial(),
            history: crate::resume_private::Wire::initial(),
            start: crate::resume_private::Wire::initial(),
            len: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            card,
            wager,
            last_card,
            last_wager,
            unassisted,
            assisted,
            skipped_hilo,
            unavailable,
            history,
            start,
            len,
        } = self;
        card.put(w)?;
        wager.put(w)?;
        last_card.put(w)?;
        last_wager.put(w)?;
        unassisted.put(w)?;
        assisted.put(w)?;
        skipped_hilo.put(w)?;
        unavailable.put(w)?;
        history.put(w)?;
        start.put(w)?;
        len.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            card,
            wager,
            last_card,
            last_wager,
            unassisted,
            assisted,
            skipped_hilo,
            unavailable,
            history,
            start,
            len,
        } = self;
        card.read_into(r)?;
        wager.read_into(r)?;
        last_card.read_into(r)?;
        last_wager.read_into(r)?;
        unassisted.read_into(r)?;
        assisted.read_into(r)?;
        skipped_hilo.read_into(r)?;
        unavailable.read_into(r)?;
        history.read_into(r)?;
        start.read_into(r)?;
        len.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            card: crate::resume_private::Wire::get(r)?,
            wager: crate::resume_private::Wire::get(r)?,
            last_card: crate::resume_private::Wire::get(r)?,
            last_wager: crate::resume_private::Wire::get(r)?,
            unassisted: crate::resume_private::Wire::get(r)?,
            assisted: crate::resume_private::Wire::get(r)?,
            skipped_hilo: crate::resume_private::Wire::get(r)?,
            unavailable: crate::resume_private::Wire::get(r)?,
            history: crate::resume_private::Wire::get(r)?,
            start: crate::resume_private::Wire::get(r)?,
            len: crate::resume_private::Wire::get(r)?,
        })
    }
}
impl crate::resume_private::SnapshotCopy for Metrics {
    fn snapshot_copy_to(&self, out: &mut Self) {
        let Self {
            card,
            wager,
            last_card,
            last_wager,
            unassisted,
            assisted,
            skipped_hilo,
            unavailable,
            history,
            start,
            len,
        } = self;
        crate::resume_private::SnapshotCopy::snapshot_copy_to(card, &mut out.card);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(wager, &mut out.wager);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(last_card, &mut out.last_card);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(last_wager, &mut out.last_wager);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(unassisted, &mut out.unassisted);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(assisted, &mut out.assisted);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(skipped_hilo, &mut out.skipped_hilo);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(unavailable, &mut out.unavailable);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(history, &mut out.history);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(start, &mut out.start);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(len, &mut out.len);
    }
}

// RESUME10 enum Skip
impl crate::resume_private::Wire for Skip {
    fn initial() -> Self {
        Self::HiLoPolicy
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::HiLoPolicy),
            1 => Ok(Self::NoSnapshot),
            2 => Ok(Self::PartialSnapshot),
            3 => Ok(Self::InvalidSnapshot),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}

impl Metrics {
    pub(crate) fn resume_valid(&self) -> bool {
        if !self.card.valid()
            || !self.wager.valid()
            || self.start >= HISTORY
            || self.len > HISTORY
            || self.len != self.card.all.decisions.min(HISTORY as u64) as usize
            || self.unassisted.count > COUNTER_LIMIT
            || self.assisted.count > COUNTER_LIMIT
            || self.skipped_hilo > COUNTER_LIMIT
            || self.unavailable > COUNTER_LIMIT
        {
            return false;
        }
        for s in [self.unassisted, self.assisted] {
            if !s.total.is_finite()
                || !s.compensation.is_finite()
                || s.total < 0.0
                || s.compensation.abs() > 1e36
            {
                return false;
            }
        }
        if self
            .unassisted
            .count
            .checked_add(self.assisted.count)
            .and_then(|v| v.checked_add(self.skipped_hilo))
            .and_then(|v| v.checked_add(self.unavailable))
            != Some(self.card.all.decisions)
        {
            return false;
        }
        let mut token = self.last_card.checked_add(1).unwrap_or(self.last_card);
        for i in 0..self.len {
            let Some(a) = self.newest(i) else {
                return false;
            };
            if a.token == 0
                || a.token >= token
                || !a.hand.resume_valid()
                || a.hand.count() == 0
                || a.original_wager != a.hand.bet
                || a.original_wager < 0
                || a.comparison.is_some() == a.skipped.is_some()
            {
                return false;
            }
            if let Some(c) = a.comparison
                && (c.model != EvModel::InfiniteReplacementV1
                    || c.original_wager != a.original_wager
                    || !c.chosen_ev.is_finite()
                    || !c.best_ev.is_finite()
                    || !c.difference_cents.is_finite()
                    || c.difference_cents < 0.0
                    || c.difference_cents > 1e36)
            {
                return false;
            }
            token = a.token;
        }
        true
    }
}
