//! Optional arithmetic practice against an explicitly named example ramp.
//! No engine, cards, RNG, hidden shoe or actuation API enters this model.
use crate::{
    deviations::Count,
    rules::{Money, Rules},
    training::{COUNTER_LIMIT, Counters},
};
pub const HISTORY_CAPACITY: usize = 128;
pub const MAX_UNITS: u32 = 1_000_000;
pub const MAX_BANKROLL: Money = i64::MAX / 4;
pub const POLICY_LABEL: &str = "EXAMPLE PRACTICE POLICY";
pub const POLICY_WARNING: &str =
    "Arithmetic example; not an optimal ramp or a prediction of advantage.";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PolicyVersion {
    ExampleRampV1,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Policy {
    pub version: PolicyVersion,
    pub units: [u32; 5],
}
impl Default for Policy {
    fn default() -> Self {
        Self::EXAMPLE
    }
}
impl Policy {
    pub const EXAMPLE: Self = Self {
        version: PolicyVersion::ExampleRampV1,
        units: [1, 2, 4, 8, 12],
    };
    pub fn valid(self) -> bool {
        self.units.into_iter().all(|n| n > 0 && n <= MAX_UNITS)
    }
    pub fn is_standard(self) -> bool {
        self == Self::default()
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Band {
    NonPositive,
    One,
    Two,
    Three,
    FourPlus,
}
impl Band {
    pub const fn from_index(n: i16) -> Self {
        match n {
            ..=0 => Self::NonPositive,
            1 => Self::One,
            2 => Self::Two,
            3 => Self::Three,
            _ => Self::FourPlus,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Category {
    NonPositive,
    One,
    Two,
    Three,
    FourPlus,
    Refill,
}
impl From<Band> for Category {
    fn from(b: Band) -> Self {
        match b {
            Band::NonPositive => Self::NonPositive,
            Band::One => Self::One,
            Band::Two => Self::Two,
            Band::Three => Self::Three,
            Band::FourPlus => Self::FourPlus,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Table {
    pub minimum: Money,
    pub maximum: Money,
    pub step: Money,
}
impl Table {
    pub fn from_rules(r: Rules) -> Option<Self> {
        if !r.valid() {
            return None;
        }
        Some(Self {
            minimum: r.minimum,
            maximum: r.maximum,
            step: 500,
        })
    }
    pub fn valid(self) -> bool {
        self.minimum >= 500
            && self.minimum <= self.maximum
            && self.maximum <= 1_000_000
            && self.step == 500
            && self.minimum % self.step == 0
            && self.maximum % self.step == 0
    }
}
/// Caller-owned monotonic live-context identity. A new session may restart its
/// shoe/checkpoint IDs. Within a session, checkpoint IDs cannot move backwards.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Context {
    pub session: u64,
    pub checkpoint: u64,
    pub shoe: u32,
}
impl Context {
    pub fn valid(self) -> bool {
        self.session != 0 && self.checkpoint != 0 && self.shoe != 0
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Snapshot {
    pub context: Context,
    pub rules: Rules,
    pub count: Count,
    pub bankroll: Money,
    pub predeal: bool,
    pub estimated_raw_tc: Option<f64>,
}
impl Snapshot {
    pub fn valid(self) -> bool {
        self.context.valid()
            && self.predeal
            && self.count.shoe == self.context.shoe
            && self.count.valid(self.rules)
            && (0..=MAX_BANKROLL).contains(&self.bankroll)
            && Table::from_rules(self.rules).is_some_and(Table::valid)
            && self
                .estimated_raw_tc
                .is_none_or(|n| n.is_finite() && n.abs() <= 21632.0)
    }
    fn follows(self, previous: Self) -> bool {
        if self.context.session < previous.context.session {
            return false;
        }
        if self.context.session > previous.context.session {
            return true;
        }
        if self.context.checkpoint < previous.context.checkpoint
            || self.context.shoe < previous.context.shoe
        {
            return false;
        }
        if self.context.checkpoint == previous.context.checkpoint {
            return self == previous;
        }
        true
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Choice {
    Wager(Money),
    NoBetRefill,
}
impl Choice {
    pub fn valid(self, s: Snapshot) -> bool {
        match self {
            Self::NoBetRefill => true,
            Self::Wager(n) => {
                n >= s.rules.minimum && n <= s.rules.maximum && n <= s.bankroll && n % 500 == 0
            }
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Adjustments {
    pub table_cap: bool,
    pub bankroll_cap: bool,
    pub step_rounding: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Target {
    pub index: i16,
    pub band: Band,
    pub units: u32,
    pub unit_size: Money,
    pub raw_wager: Money,
    pub choice: Choice,
    pub adjustments: Adjustments,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reference {
    pub label: &'static str,
    pub warning: &'static str,
    pub snapshot: Snapshot,
    pub policy: Policy,
    pub table: Table,
    pub raw_true_count: f64,
    pub canonical: Target,
    pub estimated: Option<Target>,
}
fn target(index: i16, s: Snapshot, p: Policy, t: Table) -> Option<Target> {
    let band = Band::from_index(index);
    let units = p.units[band as usize];
    let raw = t.minimum.checked_mul(units as Money)?;
    let table_cap = raw.min(t.maximum);
    let bankroll_cap = table_cap.min(s.bankroll);
    let rounded = bankroll_cap / t.step * t.step;
    Some(Target {
        index,
        band,
        units,
        unit_size: t.minimum,
        raw_wager: raw,
        choice: if s.bankroll < t.minimum {
            Choice::NoBetRefill
        } else {
            Choice::Wager(rounded)
        },
        adjustments: Adjustments {
            table_cap: raw > t.maximum,
            bankroll_cap: table_cap > s.bankroll,
            step_rounding: bankroll_cap != rounded,
        },
    })
}
fn evaluate(s: Snapshot, p: Policy) -> Option<Reference> {
    if !s.valid() || !p.valid() {
        return None;
    }
    let table = Table::from_rules(s.rules)?;
    let numerator = (s.count.running as i32).checked_mul(52)?;
    let index = (numerator / s.count.undealt as i32) as i16;
    Some(Reference {
        label: POLICY_LABEL,
        warning: POLICY_WARNING,
        snapshot: s,
        policy: p,
        table,
        raw_true_count: numerator as f64 / s.count.undealt as f64,
        canonical: target(index, s, p, table)?,
        estimated: s
            .estimated_raw_tc
            .and_then(|n| target(n as i16, s, p, table)),
    })
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Question {
    pub token: u64,
    pub context: Context,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Attempt {
    pub question: Question,
    pub choice: Choice,
    pub category: Category,
    pub correct: bool,
    pub assisted: bool,
    pub reference: Reference,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct Stats {
    pub all: Counters,
    pub categories: [Counters; 6],
}
impl Stats {
    pub const EMPTY: Self = Self {
        all: Counters::EMPTY,
        categories: [Counters::EMPTY; 6],
    };
    pub fn valid(self) -> bool {
        if !self.all.valid() {
            return false;
        }
        let mut sum = Counters::default();
        for c in self.categories {
            if !c.valid() {
                return false;
            }
            let Some(d) = sum.decisions.checked_add(c.decisions) else {
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
                decisions: d,
                correct,
                errors,
                assisted,
            };
        }
        sum == self.all
    }
    /// Defined only for unassisted attempts. No percentage for an empty sample.
    pub fn accuracy(self) -> Option<f64> {
        if !self.valid() {
            return None;
        }
        let n = self.all.correct + self.all.errors;
        if n == 0 {
            None
        } else {
            Some(self.all.correct as f64 / n as f64)
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Recorded,
    Duplicate,
    Unavailable,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grade {
    pub status: Status,
    pub attempt: Option<Attempt>,
}
#[derive(Clone, Copy)]
struct Pending {
    question: Question,
    reference: Reference,
    assisted: bool,
    revealed: bool,
}
pub struct Model {
    pending: Option<Pending>,
    last_token: Option<u64>,
    last_snapshot: Option<Snapshot>,
    aided: Option<Context>,
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
    /// Enables future App static-image initialization without a runtime Model copy.
    pub const EMPTY: Self = Self {
        pending: None,
        last_token: None,
        last_snapshot: None,
        aided: None,
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
    pub fn awaiting_answer(&self) -> bool {
        self.pending.is_some()
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
    fn find(&self, token: u64) -> Option<Attempt> {
        (0..self.len)
            .filter_map(|i| self.history(i))
            .find(|a| a.question.token == token)
            .copied()
    }
    pub fn begin(&mut self, token: u64, s: Snapshot, policy: Policy) -> bool {
        if token == 0
            || self.pending.is_some()
            || self.last_token.is_some_and(|n| token <= n)
            || self.stats.all.decisions >= COUNTER_LIMIT
            || self
                .last_snapshot
                .is_some_and(|previous| !s.follows(previous))
        {
            return false;
        }
        let Some(reference) = evaluate(s, policy) else {
            return false;
        };
        self.pending = Some(Pending {
            question: Question {
                token,
                context: s.context,
            },
            reference,
            assisted: self.aided == Some(s.context),
            revealed: false,
        });
        self.last_token = Some(token);
        self.last_snapshot = Some(s);
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
    /// Cancel an unanswered live prompt when its live context changes. The App
    /// must also call cancel before dealing, even if it reuses context IDs.
    pub fn synchronize(&mut self, context: Context) -> bool {
        if self.pending.is_some_and(|p| p.question.context != context) {
            self.pending = None;
            true
        } else {
            false
        }
    }
    pub fn reference(&self, token: u64) -> Option<Reference> {
        self.find(token).map(|a| a.reference).or_else(|| {
            self.pending
                .filter(|p| p.question.token == token && p.revealed)
                .map(|p| p.reference)
        })
    }
    pub fn mark_assisted(&mut self, token: u64) -> bool {
        if let Some(p) = self.pending.as_mut().filter(|p| p.question.token == token) {
            p.assisted = true;
            self.aided = Some(p.question.context);
            true
        } else {
            false
        }
    }
    /// Explicit help reveals the frozen reference. History help stamps its
    /// checkpoint too; a new question at that checkpoint remains assisted.
    pub fn request_help(&mut self, token: u64) -> Option<Reference> {
        if let Some(p) = self.pending.as_mut().filter(|p| p.question.token == token) {
            p.revealed = true;
            p.assisted = true;
            self.aided = Some(p.question.context);
            return Some(p.reference);
        }
        let a = self.find(token)?;
        self.aid_context(a.question.context);
        Some(a.reference)
    }
    fn aid_context(&mut self, context: Context) {
        // Looking at an older checkpoint cannot erase knowledge of the live
        // checkpoint's previously revealed target.
        if self.last_snapshot.is_some_and(|s| s.context == context) {
            self.aided = Some(context);
        }
        if let Some(p) = self
            .pending
            .as_mut()
            .filter(|p| p.question.context == context)
        {
            p.assisted = true;
        }
    }
    pub fn submit(&mut self, token: u64, choice: Choice, assisted: bool) -> Grade {
        if let Some(a) = self.find(token) {
            self.aid_context(a.question.context);
            return Grade {
                status: Status::Duplicate,
                attempt: Some(a),
            };
        }
        let Some(p) = self.pending.as_mut().filter(|p| p.question.token == token) else {
            return Grade {
                status: if self.last_token.is_some_and(|n| token <= n) {
                    Status::Duplicate
                } else {
                    Status::Unavailable
                },
                attempt: None,
            };
        };
        p.assisted |= assisted;
        if assisted {
            self.aided = Some(p.question.context)
        }
        if !choice.valid(p.reference.snapshot) || self.stats.all.decisions >= COUNTER_LIMIT {
            return Grade {
                status: Status::Unavailable,
                attempt: None,
            };
        }
        let t = p.reference.canonical;
        let a = Attempt {
            question: p.question,
            choice,
            category: if t.choice == Choice::NoBetRefill {
                Category::Refill
            } else {
                t.band.into()
            },
            correct: choice == t.choice,
            assisted: p.assisted,
            reference: p.reference,
        };
        self.pending = None;
        self.aided = Some(a.question.context);
        if self.len == HISTORY_CAPACITY {
            self.start = (self.start + 1) % HISTORY_CAPACITY;
            self.len -= 1
        }
        self.history[(self.start + self.len) % HISTORY_CAPACITY] = Some(a);
        self.len += 1;
        for c in [
            &mut self.stats.all,
            &mut self.stats.categories[a.category as usize],
        ] {
            c.decisions += 1;
            if a.assisted {
                c.assisted += 1
            } else if a.correct {
                c.correct += 1
            } else {
                c.errors += 1
            }
        }
        Grade {
            status: Status::Recorded,
            attempt: Some(a),
        }
    }
    pub fn reset_stats(&mut self) -> bool {
        if self.pending.is_some() {
            return false;
        }
        self.stats = Stats::default();
        self.history = [None; HISTORY_CAPACITY];
        self.start = 0;
        self.len = 0;
        true
    }
    /// Import saved scores once before the first question. Pending contexts,
    /// first attempts and old shoe identities are deliberately never resumed.
    pub fn restore(&mut self, stats: Stats) -> bool {
        if self.restored || self.last_token.is_some() || self.pending.is_some() || !stats.valid() {
            return false;
        }
        self.stats = stats;
        self.restored = true;
        true
    }
}

pub mod saved {
    use super::{Counters, Stats};
    pub const SIZE: usize = 236;
    /// Standalone prototype packet, separate from the shipping SBTR save schema.
    pub fn encode(stats: Stats) -> Option<[u8; SIZE]> {
        if !stats.valid() {
            return None;
        }
        let mut out = [0; SIZE];
        out[..4].copy_from_slice(b"WPS1");
        out[4..6].copy_from_slice(&1u16.to_le_bytes());
        out[6..8].copy_from_slice(&(SIZE as u16).to_le_bytes());
        let mut p = 8;
        for c in core::iter::once(stats.all).chain(stats.categories) {
            for n in [c.decisions, c.correct, c.errors, c.assisted] {
                out[p..p + 8].copy_from_slice(&n.to_le_bytes());
                p += 8
            }
        }
        let crc = crate::persistence::crc32(&out[..SIZE - 4]);
        out[SIZE - 4..].copy_from_slice(&crc.to_le_bytes());
        Some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Stats> {
        if bytes.len() != SIZE
            || bytes[..4] != *b"WPS1"
            || u16::from_le_bytes(bytes[4..6].try_into().ok()?) != 1
            || u16::from_le_bytes(bytes[6..8].try_into().ok()?) as usize != SIZE
            || u32::from_le_bytes(bytes[SIZE - 4..].try_into().ok()?)
                != crate::persistence::crc32(&bytes[..SIZE - 4])
        {
            return None;
        }
        let mut p = 8;
        let mut all = [Counters::default(); 7];
        for c in &mut all {
            let mut n = [0; 4];
            for v in &mut n {
                *v = u64::from_le_bytes(bytes[p..p + 8].try_into().ok()?);
                p += 8
            }
            *c = Counters {
                decisions: n[0],
                correct: n[1],
                errors: n[2],
                assisted: n[3],
            }
        }
        let mut categories = [Counters::default(); 6];
        categories.copy_from_slice(&all[1..]);
        let stats = Stats {
            all: all[0],
            categories,
        };
        if stats.valid() { Some(stats) } else { None }
    }
}

// RESUME10 Policy
impl crate::resume_private::Wire for Policy {
    fn initial() -> Self {
        Self {
            version: crate::resume_private::Wire::initial(),
            units: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self { version, units } = self;
        version.put(w)?;
        units.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self { version, units } = self;
        version.read_into(r)?;
        units.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            version: crate::resume_private::Wire::get(r)?,
            units: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Table
impl crate::resume_private::Wire for Table {
    fn initial() -> Self {
        Self {
            minimum: crate::resume_private::Wire::initial(),
            maximum: crate::resume_private::Wire::initial(),
            step: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            minimum,
            maximum,
            step,
        } = self;
        minimum.put(w)?;
        maximum.put(w)?;
        step.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            minimum,
            maximum,
            step,
        } = self;
        minimum.read_into(r)?;
        maximum.read_into(r)?;
        step.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            minimum: crate::resume_private::Wire::get(r)?,
            maximum: crate::resume_private::Wire::get(r)?,
            step: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Context
impl crate::resume_private::Wire for Context {
    fn initial() -> Self {
        Self {
            session: crate::resume_private::Wire::initial(),
            checkpoint: crate::resume_private::Wire::initial(),
            shoe: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            session,
            checkpoint,
            shoe,
        } = self;
        session.put(w)?;
        checkpoint.put(w)?;
        shoe.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            session,
            checkpoint,
            shoe,
        } = self;
        session.read_into(r)?;
        checkpoint.read_into(r)?;
        shoe.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            session: crate::resume_private::Wire::get(r)?,
            checkpoint: crate::resume_private::Wire::get(r)?,
            shoe: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Snapshot
impl crate::resume_private::Wire for Snapshot {
    fn initial() -> Self {
        Self {
            context: crate::resume_private::Wire::initial(),
            rules: crate::resume_private::Wire::initial(),
            count: crate::resume_private::Wire::initial(),
            bankroll: crate::resume_private::Wire::initial(),
            predeal: crate::resume_private::Wire::initial(),
            estimated_raw_tc: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            context,
            rules,
            count,
            bankroll,
            predeal,
            estimated_raw_tc,
        } = self;
        context.put(w)?;
        rules.put(w)?;
        count.put(w)?;
        bankroll.put(w)?;
        predeal.put(w)?;
        estimated_raw_tc.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            context,
            rules,
            count,
            bankroll,
            predeal,
            estimated_raw_tc,
        } = self;
        context.read_into(r)?;
        rules.read_into(r)?;
        count.read_into(r)?;
        bankroll.read_into(r)?;
        predeal.read_into(r)?;
        estimated_raw_tc.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            context: crate::resume_private::Wire::get(r)?,
            rules: crate::resume_private::Wire::get(r)?,
            count: crate::resume_private::Wire::get(r)?,
            bankroll: crate::resume_private::Wire::get(r)?,
            predeal: crate::resume_private::Wire::get(r)?,
            estimated_raw_tc: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Adjustments
impl crate::resume_private::Wire for Adjustments {
    fn initial() -> Self {
        Self {
            table_cap: crate::resume_private::Wire::initial(),
            bankroll_cap: crate::resume_private::Wire::initial(),
            step_rounding: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            table_cap,
            bankroll_cap,
            step_rounding,
        } = self;
        table_cap.put(w)?;
        bankroll_cap.put(w)?;
        step_rounding.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            table_cap,
            bankroll_cap,
            step_rounding,
        } = self;
        table_cap.read_into(r)?;
        bankroll_cap.read_into(r)?;
        step_rounding.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            table_cap: crate::resume_private::Wire::get(r)?,
            bankroll_cap: crate::resume_private::Wire::get(r)?,
            step_rounding: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Target
impl crate::resume_private::Wire for Target {
    fn initial() -> Self {
        Self {
            index: crate::resume_private::Wire::initial(),
            band: crate::resume_private::Wire::initial(),
            units: crate::resume_private::Wire::initial(),
            unit_size: crate::resume_private::Wire::initial(),
            raw_wager: crate::resume_private::Wire::initial(),
            choice: crate::resume_private::Wire::initial(),
            adjustments: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            index,
            band,
            units,
            unit_size,
            raw_wager,
            choice,
            adjustments,
        } = self;
        index.put(w)?;
        band.put(w)?;
        units.put(w)?;
        unit_size.put(w)?;
        raw_wager.put(w)?;
        choice.put(w)?;
        adjustments.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            index,
            band,
            units,
            unit_size,
            raw_wager,
            choice,
            adjustments,
        } = self;
        index.read_into(r)?;
        band.read_into(r)?;
        units.read_into(r)?;
        unit_size.read_into(r)?;
        raw_wager.read_into(r)?;
        choice.read_into(r)?;
        adjustments.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            index: crate::resume_private::Wire::get(r)?,
            band: crate::resume_private::Wire::get(r)?,
            units: crate::resume_private::Wire::get(r)?,
            unit_size: crate::resume_private::Wire::get(r)?,
            raw_wager: crate::resume_private::Wire::get(r)?,
            choice: crate::resume_private::Wire::get(r)?,
            adjustments: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Reference
impl crate::resume_private::Wire for Reference {
    fn initial() -> Self {
        Self {
            label: crate::resume_private::Wire::initial(),
            warning: crate::resume_private::Wire::initial(),
            snapshot: crate::resume_private::Wire::initial(),
            policy: crate::resume_private::Wire::initial(),
            table: crate::resume_private::Wire::initial(),
            raw_true_count: crate::resume_private::Wire::initial(),
            canonical: crate::resume_private::Wire::initial(),
            estimated: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            label,
            warning,
            snapshot,
            policy,
            table,
            raw_true_count,
            canonical,
            estimated,
        } = self;
        label.put(w)?;
        warning.put(w)?;
        snapshot.put(w)?;
        policy.put(w)?;
        table.put(w)?;
        raw_true_count.put(w)?;
        canonical.put(w)?;
        estimated.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            label,
            warning,
            snapshot,
            policy,
            table,
            raw_true_count,
            canonical,
            estimated,
        } = self;
        label.read_into(r)?;
        warning.read_into(r)?;
        snapshot.read_into(r)?;
        policy.read_into(r)?;
        table.read_into(r)?;
        raw_true_count.read_into(r)?;
        canonical.read_into(r)?;
        estimated.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            label: crate::resume_private::Wire::get(r)?,
            warning: crate::resume_private::Wire::get(r)?,
            snapshot: crate::resume_private::Wire::get(r)?,
            policy: crate::resume_private::Wire::get(r)?,
            table: crate::resume_private::Wire::get(r)?,
            raw_true_count: crate::resume_private::Wire::get(r)?,
            canonical: crate::resume_private::Wire::get(r)?,
            estimated: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Question
impl crate::resume_private::Wire for Question {
    fn initial() -> Self {
        Self {
            token: crate::resume_private::Wire::initial(),
            context: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self { token, context } = self;
        token.put(w)?;
        context.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self { token, context } = self;
        token.read_into(r)?;
        context.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            token: crate::resume_private::Wire::get(r)?,
            context: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Attempt
impl crate::resume_private::Wire for Attempt {
    fn initial() -> Self {
        Self {
            question: crate::resume_private::Wire::initial(),
            choice: crate::resume_private::Wire::initial(),
            category: crate::resume_private::Wire::initial(),
            correct: crate::resume_private::Wire::initial(),
            assisted: crate::resume_private::Wire::initial(),
            reference: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            question,
            choice,
            category,
            correct,
            assisted,
            reference,
        } = self;
        question.put(w)?;
        choice.put(w)?;
        category.put(w)?;
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
            choice,
            category,
            correct,
            assisted,
            reference,
        } = self;
        question.read_into(r)?;
        choice.read_into(r)?;
        category.read_into(r)?;
        correct.read_into(r)?;
        assisted.read_into(r)?;
        reference.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            question: crate::resume_private::Wire::get(r)?,
            choice: crate::resume_private::Wire::get(r)?,
            category: crate::resume_private::Wire::get(r)?,
            correct: crate::resume_private::Wire::get(r)?,
            assisted: crate::resume_private::Wire::get(r)?,
            reference: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Stats
impl crate::resume_private::Wire for Stats {
    fn initial() -> Self {
        Self {
            all: crate::resume_private::Wire::initial(),
            categories: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self { all, categories } = self;
        all.put(w)?;
        categories.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self { all, categories } = self;
        all.read_into(r)?;
        categories.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            all: crate::resume_private::Wire::get(r)?,
            categories: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Pending
impl crate::resume_private::Wire for Pending {
    fn initial() -> Self {
        Self {
            question: crate::resume_private::Wire::initial(),
            reference: crate::resume_private::Wire::initial(),
            assisted: crate::resume_private::Wire::initial(),
            revealed: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            question,
            reference,
            assisted,
            revealed,
        } = self;
        question.put(w)?;
        reference.put(w)?;
        assisted.put(w)?;
        revealed.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            question,
            reference,
            assisted,
            revealed,
        } = self;
        question.read_into(r)?;
        reference.read_into(r)?;
        assisted.read_into(r)?;
        revealed.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            question: crate::resume_private::Wire::get(r)?,
            reference: crate::resume_private::Wire::get(r)?,
            assisted: crate::resume_private::Wire::get(r)?,
            revealed: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Model
impl crate::resume_private::Wire for Model {
    fn initial() -> Self {
        Self {
            pending: crate::resume_private::Wire::initial(),
            last_token: crate::resume_private::Wire::initial(),
            last_snapshot: crate::resume_private::Wire::initial(),
            aided: crate::resume_private::Wire::initial(),
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
            last_snapshot,
            aided,
            stats,
            history,
            start,
            len,
            restored,
        } = self;
        pending.put(w)?;
        last_token.put(w)?;
        last_snapshot.put(w)?;
        aided.put(w)?;
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
            last_snapshot,
            aided,
            stats,
            history,
            start,
            len,
            restored,
        } = self;
        pending.read_into(r)?;
        last_token.read_into(r)?;
        last_snapshot.read_into(r)?;
        aided.read_into(r)?;
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
            last_snapshot: crate::resume_private::Wire::get(r)?,
            aided: crate::resume_private::Wire::get(r)?,
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
            last_snapshot,
            aided,
            stats,
            history,
            start,
            len,
            restored,
        } = self;
        crate::resume_private::SnapshotCopy::snapshot_copy_to(pending, &mut out.pending);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(last_token, &mut out.last_token);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(
            last_snapshot,
            &mut out.last_snapshot,
        );
        crate::resume_private::SnapshotCopy::snapshot_copy_to(aided, &mut out.aided);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(stats, &mut out.stats);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(history, &mut out.history);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(start, &mut out.start);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(len, &mut out.len);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(restored, &mut out.restored);
    }
}

// RESUME10 enum PolicyVersion
impl crate::resume_private::Wire for PolicyVersion {
    fn initial() -> Self {
        Self::ExampleRampV1
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::ExampleRampV1),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}

// RESUME10 enum Band
impl crate::resume_private::Wire for Band {
    fn initial() -> Self {
        Self::NonPositive
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::NonPositive),
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            3 => Ok(Self::Three),
            4 => Ok(Self::FourPlus),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}

// RESUME10 enum Category
impl crate::resume_private::Wire for Category {
    fn initial() -> Self {
        Self::NonPositive
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::NonPositive),
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            3 => Ok(Self::Three),
            4 => Ok(Self::FourPlus),
            5 => Ok(Self::Refill),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}
impl crate::resume_private::Wire for Choice {
    fn initial() -> Self {
        Self::NoBetRefill
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        match self {
            Self::Wager(n) => {
                w.u8(0)?;
                crate::resume_private::Wire::put(n, w)
            }
            Self::NoBetRefill => w.u8(1),
        }
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::Wager(crate::resume_private::Wire::get(r)?)),
            1 => Ok(Self::NoBetRefill),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}
impl Model {
    pub(crate) fn resume_valid(&self) -> bool {
        if !self.stats.valid()
            || self.start >= HISTORY_CAPACITY
            || self.len > HISTORY_CAPACITY
            || self.last_snapshot.is_some_and(|s| !s.valid())
        {
            return false;
        }
        if let Some(p) = self.pending {
            let Some(expected) = evaluate(p.reference.snapshot, p.reference.policy) else {
                return false;
            };
            if expected != p.reference
                || p.question.context != p.reference.snapshot.context
                || self.last_token != Some(p.question.token)
            {
                return false;
            }
        }
        let mut token = None;
        for i in 0..self.len {
            let Some(a) = self.history(i) else {
                return false;
            };
            if token.is_some_and(|t| a.question.token <= t)
                || !a.choice.valid(a.reference.snapshot)
                || evaluate(a.reference.snapshot, a.reference.policy) != Some(a.reference)
            {
                return false;
            }
            let mut m = Self::EMPTY;
            if !m.begin(a.question.token, a.reference.snapshot, a.reference.policy) {
                return false;
            }
            let Some(g) = m.submit(a.question.token, a.choice, a.assisted).attempt else {
                return false;
            };
            if g.correct != a.correct || g.category != a.category || g.assisted != a.assisted {
                return false;
            }
            token = Some(a.question.token);
        }
        self.last_token >= token
    }
    pub(crate) fn resume_last_token(&self) -> Option<u64> {
        self.last_token
    }
}
