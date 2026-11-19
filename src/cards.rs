use crate::rules::Money;
pub const UNKNOWN_POSITION: u16 = u16::MAX;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Card(u8);
impl Card {
    pub const fn from_id(id: u8) -> Option<Self> {
        if id < 52 { Some(Self(id)) } else { None }
    }
    pub const fn id(self) -> u8 {
        self.0
    }
    pub const fn rank(self) -> u8 {
        self.0 % 13 + 2
    }
    pub const fn value(self) -> u8 {
        match self.rank() {
            14 => 11,
            10..=13 => 10,
            n => n,
        }
    }
    pub(crate) const TWO: Self = Self(0);
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Outcome {
    Pending,
    Win,
    Loss,
    Push,
    Blackjack,
    Bust,
    Surrender,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Hand {
    cards: [Card; 22],
    positions: [u16; 22],
    count: u8,
    pub bet: Money,
    pub returned: Money,
    pub split: bool,
    pub split_aces: bool,
    pub done: bool,
    pub doubled: bool,
    pub outcome: Outcome,
}
impl Hand {
    pub const EMPTY: Self = Self {
        cards: [Card::TWO; 22],
        positions: [UNKNOWN_POSITION; 22],
        count: 0,
        bet: 0,
        returned: 0,
        split: false,
        split_aces: false,
        done: false,
        doubled: false,
        outcome: Outcome::Pending,
    };
}
impl Default for Hand {
    fn default() -> Self {
        Self::EMPTY
    }
}
impl Hand {
    pub fn cards(&self) -> &[Card] {
        &self.cards[..self.count as usize]
    }
    pub fn positions(&self) -> &[u16] {
        &self.positions[..self.count as usize]
    }
    pub fn count(&self) -> usize {
        self.count as usize
    }
    pub fn add(&mut self, card: Card, position: u16) -> bool {
        if self.count as usize == self.cards.len() {
            return false;
        }
        let i = self.count as usize;
        self.cards[i] = card;
        self.positions[i] = position;
        self.count += 1;
        true
    }
    pub(crate) fn keep_first(&mut self) {
        self.count = 1;
    }
    fn score(&self) -> (u16, bool) {
        let mut total = 0u16;
        let mut aces = 0;
        for c in self.cards() {
            total += c.value() as u16;
            if c.rank() == 14 {
                aces += 1
            }
        }
        while total > 21 && aces > 0 {
            total -= 10;
            aces -= 1
        }
        (total, aces > 0)
    }
    pub fn total(&self) -> u16 {
        self.score().0
    }
    pub fn soft(&self) -> bool {
        self.score().1
    }
    pub fn natural(&self) -> bool {
        !self.split && self.count == 2 && self.total() == 21
    }
}

// RESUME10 Hand
impl crate::resume_private::Wire for Hand {
    fn initial() -> Self {
        Self {
            cards: crate::resume_private::Wire::initial(),
            positions: crate::resume_private::Wire::initial(),
            count: crate::resume_private::Wire::initial(),
            bet: crate::resume_private::Wire::initial(),
            returned: crate::resume_private::Wire::initial(),
            split: crate::resume_private::Wire::initial(),
            split_aces: crate::resume_private::Wire::initial(),
            done: crate::resume_private::Wire::initial(),
            doubled: crate::resume_private::Wire::initial(),
            outcome: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            cards,
            positions,
            count,
            bet,
            returned,
            split,
            split_aces,
            done,
            doubled,
            outcome,
        } = self;
        cards.put(w)?;
        positions.put(w)?;
        count.put(w)?;
        bet.put(w)?;
        returned.put(w)?;
        split.put(w)?;
        split_aces.put(w)?;
        done.put(w)?;
        doubled.put(w)?;
        outcome.put(w)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            cards: crate::resume_private::Wire::get(r)?,
            positions: crate::resume_private::Wire::get(r)?,
            count: crate::resume_private::Wire::get(r)?,
            bet: crate::resume_private::Wire::get(r)?,
            returned: crate::resume_private::Wire::get(r)?,
            split: crate::resume_private::Wire::get(r)?,
            split_aces: crate::resume_private::Wire::get(r)?,
            done: crate::resume_private::Wire::get(r)?,
            doubled: crate::resume_private::Wire::get(r)?,
            outcome: crate::resume_private::Wire::get(r)?,
        })
    }

    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            cards,
            positions,
            count,
            bet,
            returned,
            split,
            split_aces,
            done,
            doubled,
            outcome,
        } = self;
        cards.read_into(r)?;
        positions.read_into(r)?;
        count.read_into(r)?;
        bet.read_into(r)?;
        returned.read_into(r)?;
        split.read_into(r)?;
        split_aces.read_into(r)?;
        done.read_into(r)?;
        doubled.read_into(r)?;
        outcome.read_into(r)?;
        Ok(())
    }
}

// RESUME10 enum Outcome
impl crate::resume_private::Wire for Outcome {
    fn initial() -> Self {
        Self::Pending
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::Pending),
            1 => Ok(Self::Win),
            2 => Ok(Self::Loss),
            3 => Ok(Self::Push),
            4 => Ok(Self::Blackjack),
            5 => Ok(Self::Bust),
            6 => Ok(Self::Surrender),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}

impl crate::resume_private::Wire for Card {
    fn initial() -> Self {
        Self::TWO
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(self.id())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Self::from_id(r.u8()?).ok_or(crate::resume_private::Error::Invalid)
    }
}

impl Hand {
    pub(crate) fn resume_valid(&self) -> bool {
        self.count <= 22
            && self.bet >= 0
            && self.returned >= 0
            && self.bet <= crate::rules::MAX_BANK
            && self.returned <= crate::rules::MAX_BANK
    }
}
