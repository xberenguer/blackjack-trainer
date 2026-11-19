pub type Money = i64;
pub const MAX_HANDS: usize = 4;
pub const MAX_CARDS: usize = 416;
pub const MIN_BET: Money = 500;
pub const MAX_BUY: Money = 10_000_000;
pub const MAX_BANK: Money = 1_000_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Profile {
    SixH17,
    SixS17,
    EightH17,
    SixNoSurrender,
    SixNoDas,
    SixSixFive,
}
impl Profile {
    pub const ALL: [Self; 6] = [
        Self::SixH17,
        Self::SixS17,
        Self::EightH17,
        Self::SixNoSurrender,
        Self::SixNoDas,
        Self::SixSixFive,
    ];
    pub const fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::SixH17),
            1 => Some(Self::SixS17),
            2 => Some(Self::EightH17),
            3 => Some(Self::SixNoSurrender),
            4 => Some(Self::SixNoDas),
            5 => Some(Self::SixSixFive),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rules {
    pub profile: Profile,
    pub decks: u8,
    pub hit_soft_17: bool,
    pub double_after_split: bool,
    pub late_surrender: bool,
    pub blackjack_numerator: u8,
    pub blackjack_denominator: u8,
    pub minimum: Money,
    pub maximum: Money,
    pub penetration: u8,
}
impl Default for Rules {
    fn default() -> Self {
        Self::for_profile(Profile::SixH17)
    }
}
impl Rules {
    pub const fn for_profile(profile: Profile) -> Self {
        Self {
            profile,
            decks: if matches!(profile, Profile::EightH17) {
                8
            } else {
                6
            },
            hit_soft_17: !matches!(profile, Profile::SixS17),
            double_after_split: !matches!(profile, Profile::SixNoDas),
            late_surrender: !matches!(profile, Profile::SixNoSurrender),
            blackjack_numerator: if matches!(profile, Profile::SixSixFive) {
                6
            } else {
                3
            },
            blackjack_denominator: if matches!(profile, Profile::SixSixFive) {
                5
            } else {
                2
            },
            minimum: 500,
            maximum: 50_000,
            penetration: 75,
        }
    }
    pub const fn total_cards(self) -> usize {
        self.decks as usize * 52
    }
    pub const fn cut_marker(self) -> usize {
        self.total_cards() * self.penetration as usize / 100
    }
    pub const fn maximum_round_cards(self) -> usize {
        let mut budget = 4 * 30 + 26;
        let mut count = 0;
        let mut value = 1;
        while value <= 10 {
            let copies = 4 * self.decks as usize * if value == 10 { 4 } else { 1 };
            let take = if budget / value < copies {
                budget / value
            } else {
                copies
            };
            count += take;
            budget -= take * value;
            value += 1;
        }
        count
    }
    pub fn valid(self) -> bool {
        let p = Self::for_profile(self.profile);
        self.decks == p.decks
            && self.hit_soft_17 == p.hit_soft_17
            && self.double_after_split == p.double_after_split
            && self.late_surrender == p.late_surrender
            && self.blackjack_numerator == p.blackjack_numerator
            && self.blackjack_denominator == p.blackjack_denominator
            && self.minimum >= 500
            && self.maximum <= 1_000_000
            && self.minimum <= self.maximum
            && self.minimum % 500 == 0
            && self.maximum % 500 == 0
            && self.penetration >= 1
            && self.penetration <= 90
            && self.total_cards() - self.cut_marker() >= 4
    }
}

// RESUME10 Rules
impl crate::resume_private::Wire for Rules {
    fn initial() -> Self {
        Self {
            profile: crate::resume_private::Wire::initial(),
            decks: crate::resume_private::Wire::initial(),
            hit_soft_17: crate::resume_private::Wire::initial(),
            double_after_split: crate::resume_private::Wire::initial(),
            late_surrender: crate::resume_private::Wire::initial(),
            blackjack_numerator: crate::resume_private::Wire::initial(),
            blackjack_denominator: crate::resume_private::Wire::initial(),
            minimum: crate::resume_private::Wire::initial(),
            maximum: crate::resume_private::Wire::initial(),
            penetration: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            profile,
            decks,
            hit_soft_17,
            double_after_split,
            late_surrender,
            blackjack_numerator,
            blackjack_denominator,
            minimum,
            maximum,
            penetration,
        } = self;
        profile.put(w)?;
        decks.put(w)?;
        hit_soft_17.put(w)?;
        double_after_split.put(w)?;
        late_surrender.put(w)?;
        blackjack_numerator.put(w)?;
        blackjack_denominator.put(w)?;
        minimum.put(w)?;
        maximum.put(w)?;
        penetration.put(w)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            profile: crate::resume_private::Wire::get(r)?,
            decks: crate::resume_private::Wire::get(r)?,
            hit_soft_17: crate::resume_private::Wire::get(r)?,
            double_after_split: crate::resume_private::Wire::get(r)?,
            late_surrender: crate::resume_private::Wire::get(r)?,
            blackjack_numerator: crate::resume_private::Wire::get(r)?,
            blackjack_denominator: crate::resume_private::Wire::get(r)?,
            minimum: crate::resume_private::Wire::get(r)?,
            maximum: crate::resume_private::Wire::get(r)?,
            penetration: crate::resume_private::Wire::get(r)?,
        })
    }

    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            profile,
            decks,
            hit_soft_17,
            double_after_split,
            late_surrender,
            blackjack_numerator,
            blackjack_denominator,
            minimum,
            maximum,
            penetration,
        } = self;
        profile.read_into(r)?;
        decks.read_into(r)?;
        hit_soft_17.read_into(r)?;
        double_after_split.read_into(r)?;
        late_surrender.read_into(r)?;
        blackjack_numerator.read_into(r)?;
        blackjack_denominator.read_into(r)?;
        minimum.read_into(r)?;
        maximum.read_into(r)?;
        penetration.read_into(r)?;
        Ok(())
    }
}

// RESUME10 enum Profile
impl crate::resume_private::Wire for Profile {
    fn initial() -> Self {
        Self::SixH17
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::SixH17),
            1 => Ok(Self::SixS17),
            2 => Ok(Self::EightH17),
            3 => Ok(Self::SixNoSurrender),
            4 => Ok(Self::SixNoDas),
            5 => Ok(Self::SixSixFive),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}
