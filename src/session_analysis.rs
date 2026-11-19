//! Descriptive session evidence. No shoe input, RNG, forecasts or action advice.

pub const HISTORY: usize = 128;
pub const MAX_ROUNDS: u32 = 1_000_000;
pub const MAX_ROUND_MONEY: i64 = 1_000_000_000_000;
pub const UNCERTAINTY: &str = "Observed rounds; variable wagers and changing shoe counts are dependent. The normal reference band is descriptive, not a confidence interval for advantage or a profit forecast.";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Round {
    pub id: u64,
    pub base_wager: i64,
    pub stake: i64,
    pub returned: i64,
    pub bankroll: i64,
    pub deposits: i64,
    pub cashouts: i64,
}
impl Round {
    pub fn valid(self) -> bool {
        self.id != 0
            && self.base_wager >= 500
            && self.base_wager <= MAX_ROUND_MONEY
            && self.base_wager % 500 == 0
            && (self.base_wager..=MAX_ROUND_MONEY).contains(&self.stake)
            && (0..=MAX_ROUND_MONEY).contains(&self.returned)
            && self.bankroll >= 0
            && self.bankroll >= self.returned
            && self.deposits >= 0
            && self.cashouts >= 0
    }
    pub const fn net(self) -> i64 {
        self.returned - self.stake
    }
    pub const fn ledger_net(self) -> i128 {
        self.bankroll as i128 + self.cashouts as i128 - self.deposits as i128
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Summary {
    pub rounds: u32,
    pub wins: u32,
    pub losses: u32,
    pub pushes: u32,
    pub total_staked: i128,
    pub net: i128,
    pub largest_wager: i64,
    pub best_round: i64,
    pub worst_round: i64,
    pub peak_net: i128,
    pub max_drawdown: i128,
    pub current_loss_streak: u32,
    pub longest_loss_streak: u32,
    pub mean_net_cents: f64,
    pub sample_stddev_cents: Option<f64>,
    /// From 30 rounds: 1.96 * s/sqrt(n), a labeled IID-normal reference only.
    /// Live blackjack rounds do not satisfy the independence assumptions.
    pub normal_reference_halfwidth_cents: Option<f64>,
}

/// Fixed, immutable round evidence and exact integer moment accumulators.
/// MAX_ROUNDS and MAX_ROUND_MONEY bound all i128 products below 10^36.
pub struct Model {
    initial_ledger: i128,
    last_id: u64,
    n: u32,
    sum: i128,
    squares: i128,
    stake: i128,
    wins: u32,
    losses: u32,
    pushes: u32,
    largest: i64,
    best: i64,
    worst: i64,
    peak: i128,
    drawdown: i128,
    loss_streak: u32,
    longest_loss_streak: u32,
    history: [Round; HISTORY],
    len: usize,
    next: usize,
}
impl Model {
    pub const EMPTY: Self = Self {
        initial_ledger: 0,
        last_id: 0,
        n: 0,
        sum: 0,
        squares: 0,
        stake: 0,
        wins: 0,
        losses: 0,
        pushes: 0,
        largest: 0,
        best: 0,
        worst: 0,
        peak: 0,
        drawdown: 0,
        loss_streak: 0,
        longest_loss_streak: 0,
        history: [Round {
            id: 0,
            base_wager: 0,
            stake: 0,
            returned: 0,
            bankroll: 0,
            deposits: 0,
            cashouts: 0,
        }; HISTORY],
        len: 0,
        next: 0,
    };
    pub fn new(initial_ledger: i128) -> Self {
        Self {
            initial_ledger,
            ..Self::EMPTY
        }
    }
    pub fn record(&mut self, r: Round) -> bool {
        if !r.valid()
            || r.id <= self.last_id
            || self.n == MAX_ROUNDS
            || self
                .initial_ledger
                .checked_add(self.sum)
                .and_then(|v| v.checked_add(r.net() as i128))
                != Some(r.ledger_net())
        {
            return false;
        }
        let net = r.net();
        self.last_id = r.id;
        self.n += 1;
        self.sum += net as i128;
        self.squares += net as i128 * net as i128;
        self.stake += r.stake as i128;
        self.largest = self.largest.max(r.base_wager);
        if self.n == 1 {
            self.best = net;
            self.worst = net;
        } else {
            self.best = self.best.max(net);
            self.worst = self.worst.min(net);
        }
        if net > 0 {
            self.wins += 1;
            self.loss_streak = 0;
        } else if net < 0 {
            self.losses += 1;
            self.loss_streak += 1;
            self.longest_loss_streak = self.longest_loss_streak.max(self.loss_streak);
        } else {
            self.pushes += 1;
            self.loss_streak = 0;
        }
        self.peak = self.peak.max(self.sum);
        self.drawdown = self.drawdown.max(self.peak - self.sum);
        self.history[self.next] = r;
        self.next = (self.next + 1) % HISTORY;
        self.len = (self.len + 1).min(HISTORY);
        true
    }
    pub fn newest(&self, offset: usize) -> Option<Round> {
        if offset >= self.len {
            return None;
        }
        Some(self.history[(self.next + HISTORY - 1 - offset) % HISTORY])
    }
    pub fn summary(&self) -> Summary {
        let n = self.n as i128;
        let stddev = if self.n < 2 {
            None
        } else {
            // Exact subtraction avoids catastrophic cancellation for near-equal
            // large results. Convert to f64 only after the exact numerator.
            Some(sqrt(
                (n * self.squares - self.sum * self.sum) as f64 / (n * (n - 1)) as f64,
            ))
        };
        Summary {
            rounds: self.n,
            wins: self.wins,
            losses: self.losses,
            pushes: self.pushes,
            total_staked: self.stake,
            net: self.sum,
            largest_wager: self.largest,
            best_round: self.best,
            worst_round: self.worst,
            peak_net: self.peak,
            max_drawdown: self.drawdown,
            current_loss_streak: self.loss_streak,
            longest_loss_streak: self.longest_loss_streak,
            mean_net_cents: if self.n == 0 {
                0.0
            } else {
                self.sum as f64 / self.n as f64
            },
            sample_stddev_cents: stddev,
            normal_reference_halfwidth_cents: if self.n < 30 {
                None
            } else {
                stddev.map(|s| 1.96 * s / sqrt(self.n as f64))
            },
        }
    }
}
impl Default for Model {
    fn default() -> Self {
        Self::EMPTY
    }
}

fn sqrt(v: f64) -> f64 {
    if v == 0.0 {
        return 0.0;
    }
    let mut x = if v >= 1.0 { v } else { 1.0 };
    for _ in 0..128 {
        let next = 0.5 * (x + v / x);
        if next == x {
            break;
        }
        x = next;
    }
    x
}

/// A frozen EV comparison, expressed per original wager under the stated
/// replacement model. This is an expected-value difference, not realized loss.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecisionCost {
    pub cents: f64,
    pub assisted: bool,
}
pub fn decision_cost(
    best_ev: f64,
    chosen_ev: f64,
    wager: i64,
    assisted: bool,
) -> Option<DecisionCost> {
    if !best_ev.is_finite()
        || !chosen_ev.is_finite()
        || wager <= 0
        || wager > MAX_ROUND_MONEY
        || best_ev < chosen_ev - 1e-10
    {
        return None;
    }
    let cents = (best_ev - chosen_ev).max(0.0) * wager as f64;
    if !cents.is_finite() {
        return None;
    }
    Some(DecisionCost { cents, assisted })
}

// RESUME10 Round
impl crate::resume_private::Wire for Round {
    fn initial() -> Self {
        Self {
            id: crate::resume_private::Wire::initial(),
            base_wager: crate::resume_private::Wire::initial(),
            stake: crate::resume_private::Wire::initial(),
            returned: crate::resume_private::Wire::initial(),
            bankroll: crate::resume_private::Wire::initial(),
            deposits: crate::resume_private::Wire::initial(),
            cashouts: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            id,
            base_wager,
            stake,
            returned,
            bankroll,
            deposits,
            cashouts,
        } = self;
        id.put(w)?;
        base_wager.put(w)?;
        stake.put(w)?;
        returned.put(w)?;
        bankroll.put(w)?;
        deposits.put(w)?;
        cashouts.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            id,
            base_wager,
            stake,
            returned,
            bankroll,
            deposits,
            cashouts,
        } = self;
        id.read_into(r)?;
        base_wager.read_into(r)?;
        stake.read_into(r)?;
        returned.read_into(r)?;
        bankroll.read_into(r)?;
        deposits.read_into(r)?;
        cashouts.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            id: crate::resume_private::Wire::get(r)?,
            base_wager: crate::resume_private::Wire::get(r)?,
            stake: crate::resume_private::Wire::get(r)?,
            returned: crate::resume_private::Wire::get(r)?,
            bankroll: crate::resume_private::Wire::get(r)?,
            deposits: crate::resume_private::Wire::get(r)?,
            cashouts: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Model
impl crate::resume_private::Wire for Model {
    fn initial() -> Self {
        Self {
            initial_ledger: crate::resume_private::Wire::initial(),
            last_id: crate::resume_private::Wire::initial(),
            n: crate::resume_private::Wire::initial(),
            sum: crate::resume_private::Wire::initial(),
            squares: crate::resume_private::Wire::initial(),
            stake: crate::resume_private::Wire::initial(),
            wins: crate::resume_private::Wire::initial(),
            losses: crate::resume_private::Wire::initial(),
            pushes: crate::resume_private::Wire::initial(),
            largest: crate::resume_private::Wire::initial(),
            best: crate::resume_private::Wire::initial(),
            worst: crate::resume_private::Wire::initial(),
            peak: crate::resume_private::Wire::initial(),
            drawdown: crate::resume_private::Wire::initial(),
            loss_streak: crate::resume_private::Wire::initial(),
            longest_loss_streak: crate::resume_private::Wire::initial(),
            history: crate::resume_private::Wire::initial(),
            len: crate::resume_private::Wire::initial(),
            next: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            initial_ledger,
            last_id,
            n,
            sum,
            squares,
            stake,
            wins,
            losses,
            pushes,
            largest,
            best,
            worst,
            peak,
            drawdown,
            loss_streak,
            longest_loss_streak,
            history,
            len,
            next,
        } = self;
        initial_ledger.put(w)?;
        last_id.put(w)?;
        n.put(w)?;
        sum.put(w)?;
        squares.put(w)?;
        stake.put(w)?;
        wins.put(w)?;
        losses.put(w)?;
        pushes.put(w)?;
        largest.put(w)?;
        best.put(w)?;
        worst.put(w)?;
        peak.put(w)?;
        drawdown.put(w)?;
        loss_streak.put(w)?;
        longest_loss_streak.put(w)?;
        history.put(w)?;
        len.put(w)?;
        next.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            initial_ledger,
            last_id,
            n,
            sum,
            squares,
            stake,
            wins,
            losses,
            pushes,
            largest,
            best,
            worst,
            peak,
            drawdown,
            loss_streak,
            longest_loss_streak,
            history,
            len,
            next,
        } = self;
        initial_ledger.read_into(r)?;
        last_id.read_into(r)?;
        n.read_into(r)?;
        sum.read_into(r)?;
        squares.read_into(r)?;
        stake.read_into(r)?;
        wins.read_into(r)?;
        losses.read_into(r)?;
        pushes.read_into(r)?;
        largest.read_into(r)?;
        best.read_into(r)?;
        worst.read_into(r)?;
        peak.read_into(r)?;
        drawdown.read_into(r)?;
        loss_streak.read_into(r)?;
        longest_loss_streak.read_into(r)?;
        history.read_into(r)?;
        len.read_into(r)?;
        next.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            initial_ledger: crate::resume_private::Wire::get(r)?,
            last_id: crate::resume_private::Wire::get(r)?,
            n: crate::resume_private::Wire::get(r)?,
            sum: crate::resume_private::Wire::get(r)?,
            squares: crate::resume_private::Wire::get(r)?,
            stake: crate::resume_private::Wire::get(r)?,
            wins: crate::resume_private::Wire::get(r)?,
            losses: crate::resume_private::Wire::get(r)?,
            pushes: crate::resume_private::Wire::get(r)?,
            largest: crate::resume_private::Wire::get(r)?,
            best: crate::resume_private::Wire::get(r)?,
            worst: crate::resume_private::Wire::get(r)?,
            peak: crate::resume_private::Wire::get(r)?,
            drawdown: crate::resume_private::Wire::get(r)?,
            loss_streak: crate::resume_private::Wire::get(r)?,
            longest_loss_streak: crate::resume_private::Wire::get(r)?,
            history: crate::resume_private::Wire::get(r)?,
            len: crate::resume_private::Wire::get(r)?,
            next: crate::resume_private::Wire::get(r)?,
        })
    }
}
impl crate::resume_private::SnapshotCopy for Model {
    fn snapshot_copy_to(&self, out: &mut Self) {
        let Self {
            initial_ledger,
            last_id,
            n,
            sum,
            squares,
            stake,
            wins,
            losses,
            pushes,
            largest,
            best,
            worst,
            peak,
            drawdown,
            loss_streak,
            longest_loss_streak,
            history,
            len,
            next,
        } = self;
        crate::resume_private::SnapshotCopy::snapshot_copy_to(
            initial_ledger,
            &mut out.initial_ledger,
        );
        crate::resume_private::SnapshotCopy::snapshot_copy_to(last_id, &mut out.last_id);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(n, &mut out.n);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(sum, &mut out.sum);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(squares, &mut out.squares);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(stake, &mut out.stake);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(wins, &mut out.wins);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(losses, &mut out.losses);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(pushes, &mut out.pushes);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(largest, &mut out.largest);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(best, &mut out.best);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(worst, &mut out.worst);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(peak, &mut out.peak);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(drawdown, &mut out.drawdown);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(loss_streak, &mut out.loss_streak);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(
            longest_loss_streak,
            &mut out.longest_loss_streak,
        );
        crate::resume_private::SnapshotCopy::snapshot_copy_to(history, &mut out.history);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(len, &mut out.len);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(next, &mut out.next);
    }
}

impl Model {
    pub(crate) fn resume_valid(&self) -> bool {
        let n = self.n as i128;
        let money = MAX_ROUND_MONEY as i128;
        if self.n > MAX_ROUNDS
            || self.len > HISTORY
            || self.next >= HISTORY
            || self.len != self.n.min(HISTORY as u32) as usize
            || self.next != self.n as usize % HISTORY
            || self
                .wins
                .checked_add(self.losses)
                .and_then(|v| v.checked_add(self.pushes))
                != Some(self.n)
            || self.sum < -n * money
            || self.sum > n * money
            || self.squares < 0
            || self.squares > n * money * money
            || self.stake < 0
            || self.stake > n * money
            || self.peak < 0
            || self.peak > n * money
            || self.drawdown < 0
            || self.drawdown > 2 * n * money
            || self.loss_streak > self.longest_loss_streak
            || self.longest_loss_streak > self.n
            || self.initial_ledger < -(i64::MAX as i128) * 3
            || self.initial_ledger > (i64::MAX as i128) * 3
            || self.largest < 0
            || self.largest > MAX_ROUND_MONEY
            || self.best.unsigned_abs() > MAX_ROUND_MONEY as u64
            || self.worst.unsigned_abs() > MAX_ROUND_MONEY as u64
            || n * self.squares < self.sum * self.sum
        {
            return false;
        }
        if self.n == 0 {
            return self.last_id == 0 && self.sum == 0 && self.squares == 0 && self.stake == 0;
        }
        if self.last_id == 0 || self.best < self.worst {
            return false;
        }
        let mut next_id = self.last_id.checked_add(1).unwrap_or(self.last_id);
        for i in 0..self.len {
            let Some(r) = self.newest(i) else {
                return false;
            };
            if !r.valid() || r.id >= next_id {
                return false;
            }
            if i == 0 && (r.id != self.last_id || r.ledger_net() != self.initial_ledger + self.sum)
            {
                return false;
            }
            next_id = r.id;
        }
        true
    }
}
