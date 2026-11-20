//! Optional exposed-count checkpoints. Correct references are gated until an
//! answer or an explicit help request; cancelled tokens cannot be reused.
use crate::{
    counting::Tracker,
    training::{COUNTER_LIMIT, Counters},
};
pub const HISTORY_CAPACITY: usize = 128;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Kind {
    Running,
    Decks,
    TrueCount,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub running: i16,
    pub undealt: u16,
    pub decks: u8,
    pub observed: u16,
    pub shoe: u32,
    pub session: u64,
}
impl Snapshot {
    pub fn from_tracker(t: &Tracker, undealt: u16, session: u64) -> Option<Self> {
        let s = Self {
            running: t.running_count(),
            undealt,
            decks: t.decks(),
            observed: t.observed_count(),
            shoe: t.shoe_number(),
            session,
        };
        if s.valid() { Some(s) } else { None }
    }
    pub fn valid(self) -> bool {
        matches!(self.decks, 6 | 8)
            && self.shoe != 0
            && self.undealt <= self.decks as u16 * 52
            && self.observed <= self.decks as u16 * 52 - self.undealt
            && (self.running as i32).abs() <= self.observed as i32
            && (self.running as i32).abs() <= self.decks as i32 * 20
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Answer {
    Running(i16),
    Decks(u8),
    TrueCount(i16),
} // Decks is a count of half-decks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Question {
    pub token: u64,
    pub kind: Kind,
    pub given_half_decks: Option<u8>,
}
#[derive(Clone, Copy, Debug)]
pub struct Reference {
    pub running: i16,
    pub exact_decks: f64,
    pub rounded_half_decks: u8,
    pub physical_true_count: Option<f64>,
    pub physical_index: Option<i16>,
    pub given_index: Option<i16>,
}
fn reference(s: Snapshot, q: Question) -> Reference {
    Reference {
        running: s.running,
        exact_decks: s.undealt as f64 / 52.0,
        rounded_half_decks: ((s.undealt + 13) / 26) as u8,
        physical_true_count: if s.undealt == 0 {
            None
        } else {
            Some(s.running as f64 / (s.undealt as f64 / 52.0))
        },
        physical_index: if s.undealt == 0 {
            None
        } else {
            Some((s.running as i32 * 52 / s.undealt as i32) as i16)
        },
        given_index: q
            .given_half_decks
            .map(|h| (s.running as i32 * 2 / h as i32) as i16),
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Attempt {
    pub question: Question,
    pub snapshot: Snapshot,
    pub answer: Answer,
    pub correct: bool,
    pub assisted: bool,
    pub reference: Reference,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct Stats {
    pub all: Counters,
    pub kinds: [Counters; 3],
}
impl Stats {
    pub const EMPTY: Self = Self {
        all: Counters::EMPTY,
        kinds: [Counters::EMPTY; 3],
    };
    pub fn valid(self) -> bool {
        if !self.all.valid() {
            return false;
        }
        let mut sum = Counters::default();
        for c in self.kinds {
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
#[derive(Clone, Copy)]
struct Pending {
    question: Question,
    snapshot: Snapshot,
    helped: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Unavailable,
    Duplicate,
    Recorded,
}
#[derive(Clone, Copy)]
pub struct Grade {
    pub status: Status,
    pub attempt: Option<Attempt>,
}
pub struct Model {
    pending: Option<Pending>,
    last_token: Option<u64>,
    stats: Stats,
    history: [Option<Attempt>; HISTORY_CAPACITY],
    start: usize,
    len: usize,
}
impl Model {
    pub const EMPTY: Self = Self {
        pending: None,
        last_token: None,
        stats: Stats::EMPTY,
        history: [None; HISTORY_CAPACITY],
        start: 0,
        len: 0,
    };
}
impl Default for Model {
    fn default() -> Self {
        Self::EMPTY
    }
}
impl Model {
    pub fn question(&self) -> Option<Question> {
        self.pending.map(|p| p.question)
    }
    pub fn awaiting_answer(&self) -> bool {
        self.pending.is_some()
    }
    pub fn stats(&self) -> Stats {
        self.stats
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
    pub fn begin(&mut self, question: Question, snapshot: Snapshot) -> bool {
        if self.pending.is_some()
            || self.last_token.is_some_and(|t| question.token <= t)
            || !snapshot.valid()
            || (question.kind == Kind::TrueCount && snapshot.undealt == 0)
            || question.given_half_decks.is_some_and(|h| {
                question.kind != Kind::TrueCount || h == 0 || h > snapshot.decks * 2
            })
        {
            return false;
        }
        self.pending = Some(Pending {
            question,
            snapshot,
            helped: false,
        });
        self.last_token = Some(question.token);
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
        if let Some(a) =
            (0..self.len).find_map(|i| self.history(i).filter(|a| a.question.token == token))
        {
            return Some(a.reference);
        }
        self.pending
            .filter(|p| p.question.token == token && p.helped)
            .map(|p| reference(p.snapshot, p.question))
    }
    pub fn request_help(&mut self, token: u64) -> Option<Reference> {
        if let Some(p) = self.pending.as_mut().filter(|p| p.question.token == token) {
            p.helped = true;
            return Some(reference(p.snapshot, p.question));
        }
        self.reference(token)
    }
    /// A deck image discloses only the same bounded visual cue as a real stack.
    pub fn stack_height(&self, token: u64) -> Option<u16> {
        self.pending
            .filter(|p| p.question.token == token && p.question.kind == Kind::Decks)
            .map(|p| ((p.snapshot.undealt as u32 * 9 + 26) / 52).max(1) as u16)
    }
    pub fn submit(&mut self, token: u64, answer: Answer, assisted: bool) -> Grade {
        if let Some(a) = (0..self.len)
            .find_map(|i| self.history(i).filter(|a| a.question.token == token))
            .copied()
        {
            return Grade {
                status: Status::Duplicate,
                attempt: Some(a),
            };
        }
        let Some(p) = self.pending.as_mut().filter(|p| p.question.token == token) else {
            return Grade {
                status: if self.last_token.is_some_and(|t| token <= t) {
                    Status::Duplicate
                } else {
                    Status::Unavailable
                },
                attempt: None,
            };
        };
        p.helped |= assisted;
        let valid = match (p.question.kind, answer) {
            (Kind::Running, Answer::Running(n)) => (n as i32).abs() <= p.snapshot.decks as i32 * 20,
            (Kind::Decks, Answer::Decks(h)) => h <= p.snapshot.decks * 2,
            (Kind::TrueCount, Answer::TrueCount(n)) => (n as i32).abs() <= 21632,
            _ => false,
        };
        if !valid || self.stats.all.decisions >= COUNTER_LIMIT {
            return Grade {
                status: Status::Unavailable,
                attempt: None,
            };
        }
        let r = reference(p.snapshot, p.question);
        let correct = match answer {
            Answer::Running(n) => n == r.running,
            Answer::Decks(h) => (h as i32 * 26 - p.snapshot.undealt as i32).abs() <= 26,
            Answer::TrueCount(n) => Some(n) == r.given_index.or(r.physical_index),
        };
        let a = Attempt {
            question: p.question,
            snapshot: p.snapshot,
            answer,
            correct,
            assisted: p.helped,
            reference: r,
        };
        self.pending = None;
        if self.len == HISTORY_CAPACITY {
            self.start = (self.start + 1) % HISTORY_CAPACITY;
            self.len -= 1
        }
        self.history[(self.start + self.len) % HISTORY_CAPACITY] = Some(a);
        self.len += 1;
        for c in [
            &mut self.stats.all,
            &mut self.stats.kinds[a.question.kind as usize],
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
    pub fn restore(&mut self, stats: Stats) -> bool {
        if self.last_token.is_some() || self.pending.is_some() || !stats.valid() {
            return false;
        }
        self.stats = stats;
        true
    }
    pub fn reset_stats(&mut self) -> bool {
        if self.pending.is_some() {
            return false;
        }
        self.stats = Stats::default();
        self.history.fill(None);
        self.start = 0;
        self.len = 0;
        true
    }
}

// RESUME10 Snapshot
impl crate::resume_private::Wire for Snapshot {
    fn initial() -> Self {
        Self {
            running: crate::resume_private::Wire::initial(),
            undealt: crate::resume_private::Wire::initial(),
            decks: crate::resume_private::Wire::initial(),
            observed: crate::resume_private::Wire::initial(),
            shoe: crate::resume_private::Wire::initial(),
            session: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            running,
            undealt,
            decks,
            observed,
            shoe,
            session,
        } = self;
        running.put(w)?;
        undealt.put(w)?;
        decks.put(w)?;
        observed.put(w)?;
        shoe.put(w)?;
        session.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            running,
            undealt,
            decks,
            observed,
            shoe,
            session,
        } = self;
        running.read_into(r)?;
        undealt.read_into(r)?;
        decks.read_into(r)?;
        observed.read_into(r)?;
        shoe.read_into(r)?;
        session.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            running: crate::resume_private::Wire::get(r)?,
            undealt: crate::resume_private::Wire::get(r)?,
            decks: crate::resume_private::Wire::get(r)?,
            observed: crate::resume_private::Wire::get(r)?,
            shoe: crate::resume_private::Wire::get(r)?,
            session: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Question
impl crate::resume_private::Wire for Question {
    fn initial() -> Self {
        Self {
            token: crate::resume_private::Wire::initial(),
            kind: crate::resume_private::Wire::initial(),
            given_half_decks: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            token,
            kind,
            given_half_decks,
        } = self;
        token.put(w)?;
        kind.put(w)?;
        given_half_decks.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            token,
            kind,
            given_half_decks,
        } = self;
        token.read_into(r)?;
        kind.read_into(r)?;
        given_half_decks.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            token: crate::resume_private::Wire::get(r)?,
            kind: crate::resume_private::Wire::get(r)?,
            given_half_decks: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Reference
impl crate::resume_private::Wire for Reference {
    fn initial() -> Self {
        Self {
            running: crate::resume_private::Wire::initial(),
            exact_decks: crate::resume_private::Wire::initial(),
            rounded_half_decks: crate::resume_private::Wire::initial(),
            physical_true_count: crate::resume_private::Wire::initial(),
            physical_index: crate::resume_private::Wire::initial(),
            given_index: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            running,
            exact_decks,
            rounded_half_decks,
            physical_true_count,
            physical_index,
            given_index,
        } = self;
        running.put(w)?;
        exact_decks.put(w)?;
        rounded_half_decks.put(w)?;
        physical_true_count.put(w)?;
        physical_index.put(w)?;
        given_index.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            running,
            exact_decks,
            rounded_half_decks,
            physical_true_count,
            physical_index,
            given_index,
        } = self;
        running.read_into(r)?;
        exact_decks.read_into(r)?;
        rounded_half_decks.read_into(r)?;
        physical_true_count.read_into(r)?;
        physical_index.read_into(r)?;
        given_index.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            running: crate::resume_private::Wire::get(r)?,
            exact_decks: crate::resume_private::Wire::get(r)?,
            rounded_half_decks: crate::resume_private::Wire::get(r)?,
            physical_true_count: crate::resume_private::Wire::get(r)?,
            physical_index: crate::resume_private::Wire::get(r)?,
            given_index: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Attempt
impl crate::resume_private::Wire for Attempt {
    fn initial() -> Self {
        Self {
            question: crate::resume_private::Wire::initial(),
            snapshot: crate::resume_private::Wire::initial(),
            answer: crate::resume_private::Wire::initial(),
            correct: crate::resume_private::Wire::initial(),
            assisted: crate::resume_private::Wire::initial(),
            reference: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            question,
            snapshot,
            answer,
            correct,
            assisted,
            reference,
        } = self;
        question.put(w)?;
        snapshot.put(w)?;
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
            snapshot,
            answer,
            correct,
            assisted,
            reference,
        } = self;
        question.read_into(r)?;
        snapshot.read_into(r)?;
        answer.read_into(r)?;
        correct.read_into(r)?;
        assisted.read_into(r)?;
        reference.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            question: crate::resume_private::Wire::get(r)?,
            snapshot: crate::resume_private::Wire::get(r)?,
            answer: crate::resume_private::Wire::get(r)?,
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
            kinds: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self { all, kinds } = self;
        all.put(w)?;
        kinds.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self { all, kinds } = self;
        all.read_into(r)?;
        kinds.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            all: crate::resume_private::Wire::get(r)?,
            kinds: crate::resume_private::Wire::get(r)?,
        })
    }
}

// RESUME10 Pending
impl crate::resume_private::Wire for Pending {
    fn initial() -> Self {
        Self {
            question: crate::resume_private::Wire::initial(),
            snapshot: crate::resume_private::Wire::initial(),
            helped: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            question,
            snapshot,
            helped,
        } = self;
        question.put(w)?;
        snapshot.put(w)?;
        helped.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            question,
            snapshot,
            helped,
        } = self;
        question.read_into(r)?;
        snapshot.read_into(r)?;
        helped.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            question: crate::resume_private::Wire::get(r)?,
            snapshot: crate::resume_private::Wire::get(r)?,
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
        } = self;
        pending.put(w)?;
        last_token.put(w)?;
        stats.put(w)?;
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
            pending,
            last_token,
            stats,
            history,
            start,
            len,
        } = self;
        pending.read_into(r)?;
        last_token.read_into(r)?;
        stats.read_into(r)?;
        history.read_into(r)?;
        start.read_into(r)?;
        len.read_into(r)?;
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
        } = self;
        crate::resume_private::SnapshotCopy::snapshot_copy_to(pending, &mut out.pending);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(last_token, &mut out.last_token);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(stats, &mut out.stats);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(history, &mut out.history);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(start, &mut out.start);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(len, &mut out.len);
    }
}

// RESUME10 enum Kind
impl crate::resume_private::Wire for Kind {
    fn initial() -> Self {
        Self::Running
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::Running),
            1 => Ok(Self::Decks),
            2 => Ok(Self::TrueCount),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}

impl crate::resume_private::Wire for Answer {
    fn initial() -> Self {
        Self::Running(0)
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        match self {
            Self::Running(n) => {
                w.u8(0)?;
                n.put(w)
            }
            Self::Decks(n) => {
                w.u8(1)?;
                n.put(w)
            }
            Self::TrueCount(n) => {
                w.u8(2)?;
                n.put(w)
            }
        }
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        use crate::resume_private::Wire;
        match r.u8()? {
            0 => Ok(Self::Running(Wire::get(r)?)),
            1 => Ok(Self::Decks(Wire::get(r)?)),
            2 => Ok(Self::TrueCount(Wire::get(r)?)),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}

impl Model {
    pub(crate) fn resume_valid(&self) -> bool {
        if !self.stats.valid() || self.start >= HISTORY_CAPACITY || self.len > HISTORY_CAPACITY {
            return false;
        }
        if let Some(p) = self.pending {
            let mut m = Self::EMPTY;
            if !m.begin(p.question, p.snapshot) || self.last_token != Some(p.question.token) {
                return false;
            }
        }
        let mut token = None;
        for i in 0..self.len {
            let Some(a) = self.history(i) else {
                return false;
            };
            if token.is_some_and(|t| a.question.token <= t) {
                return false;
            }
            let mut m = Self::EMPTY;
            if !m.begin(a.question, a.snapshot) {
                return false;
            }
            let Some(expected) = m.submit(a.question.token, a.answer, a.assisted).attempt else {
                return false;
            };
            if expected.correct != a.correct
                || expected.assisted != a.assisted
                || expected.reference.running != a.reference.running
                || expected.reference.exact_decks.to_bits() != a.reference.exact_decks.to_bits()
                || expected.reference.rounded_half_decks != a.reference.rounded_half_decks
                || expected.reference.physical_index != a.reference.physical_index
                || expected.reference.given_index != a.reference.given_index
                || expected.reference.physical_true_count.map(f64::to_bits)
                    != a.reference.physical_true_count.map(f64::to_bits)
            {
                return false;
            }
            token = Some(a.question.token);
        }
        self.last_token >= token
    }
}

impl Model {
    pub(crate) fn resume_last_token(&self) -> Option<u64> {
        self.last_token
    }
}
