//! Named BJA 2018 H17/S17 red-cell index policy, layered over verified basic
//! strategy. This is an index policy, not exact remaining-composition EV.
use crate::{
    cards::{Card, Hand},
    engine::Action,
    rules::Rules,
    strategy::{self, Recommendation},
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Count {
    pub running: i16,
    pub undealt: u16,
    pub decks: u8,
    pub observed: u16,
    pub shoe: u32,
}
impl Count {
    pub fn valid(self, r: Rules) -> bool {
        r.valid()
            && self.decks == r.decks
            && self.shoe != 0
            && self.undealt > 0
            && self.undealt <= self.decks as u16 * 52
            && self.observed <= self.decks as u16 * 52 - self.undealt
            && (self.running as i32).abs() <= self.observed as i32
            && (self.running as i32).abs() <= self.decks as i32 * 20
    }
    pub fn integer_true_count(self, r: Rules) -> Option<i16> {
        if self.valid(r) {
            Some((self.running as i32 * 52 / self.undealt as i32) as i16)
        } else {
            None
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Threshold {
    AtLeast(i16),
    AtMost(i16),
    PositiveRunning,
    NegativeRunning,
}
impl Threshold {
    fn reached(self, rc: i16, tc: i16) -> bool {
        match self {
            Self::AtLeast(n) => tc >= n,
            Self::AtMost(n) => tc <= n,
            Self::PositiveRunning => rc > 0,
            Self::NegativeRunning => rc < 0,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndexDecision {
    pub threshold: Threshold,
    pub reached: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Advice {
    pub play: Option<Recommendation>,
    pub integer_true_count: Option<i16>,
    pub change: Option<IndexDecision>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Pair(u8),
    Soft(u16),
    Hard(u16),
    Surrender(u16),
}
#[derive(Clone, Copy)]
struct Index {
    kind: Kind,
    up: u8,
    gate: u8,
    condition: Threshold,
    play: Action,
    otherwise: Option<Action>,
    why: &'static str,
}
const fn index(
    kind: Kind,
    up: u8,
    gate: u8,
    condition: Threshold,
    play: Action,
    otherwise: Option<Action>,
    why: &'static str,
) -> Index {
    Index {
        kind,
        up,
        gate,
        condition,
        play,
        otherwise,
        why,
    }
}
use Action::{Double, Hit, Split, Stand, Surrender};
use Threshold::{AtLeast, AtMost, NegativeRunning, PositiveRunning};
const INDICES: [Index; 30] = [
    index(
        Kind::Pair(10),
        4,
        3,
        AtLeast(6),
        Split,
        None,
        "Hi-Lo: split tens vs4 at TC+6 or above.",
    ),
    index(
        Kind::Pair(10),
        5,
        3,
        AtLeast(5),
        Split,
        None,
        "Hi-Lo: split tens vs5 at TC+5 or above.",
    ),
    index(
        Kind::Pair(10),
        6,
        3,
        AtLeast(4),
        Split,
        None,
        "Hi-Lo: split tens vs6 at TC+4 or above.",
    ),
    index(
        Kind::Soft(19),
        4,
        3,
        AtLeast(3),
        Double,
        None,
        "Hi-Lo: double soft19 vs4 at TC+3 or above.",
    ),
    index(
        Kind::Soft(19),
        5,
        3,
        AtLeast(1),
        Double,
        None,
        "Hi-Lo: double soft19 vs5 at TC+1 or above.",
    ),
    index(
        Kind::Soft(19),
        6,
        1,
        NegativeRunning,
        Stand,
        None,
        "Hi-Lo H17: stand soft19 vs6 when RC is negative.",
    ),
    index(
        Kind::Soft(19),
        6,
        2,
        AtLeast(1),
        Double,
        None,
        "Hi-Lo S17: double soft19 vs6 at TC+1 or above.",
    ),
    index(
        Kind::Soft(17),
        2,
        3,
        AtLeast(1),
        Double,
        None,
        "Hi-Lo: double soft17 vs2 at TC+1 or above.",
    ),
    index(
        Kind::Hard(16),
        9,
        3,
        AtLeast(4),
        Stand,
        None,
        "Hi-Lo: stand hard16 vs9 at TC+4; apply surrender first.",
    ),
    index(
        Kind::Hard(16),
        10,
        3,
        PositiveRunning,
        Stand,
        None,
        "Hi-Lo: stand hard16 vs10 when RC is strictly positive.",
    ),
    index(
        Kind::Hard(16),
        11,
        1,
        AtLeast(3),
        Stand,
        None,
        "Hi-Lo H17: stand hard16 vs ace at TC+3 without surrender.",
    ),
    index(
        Kind::Hard(15),
        10,
        3,
        AtLeast(4),
        Stand,
        None,
        "Hi-Lo: stand hard15 vs10 at TC+4 without surrender.",
    ),
    index(
        Kind::Hard(15),
        11,
        1,
        AtLeast(5),
        Stand,
        None,
        "Hi-Lo H17: stand hard15 vs ace at TC+5 without surrender.",
    ),
    index(
        Kind::Hard(13),
        2,
        3,
        AtMost(-1),
        Hit,
        None,
        "Hi-Lo: hit hard13 vs2 at TC-1 or below.",
    ),
    index(
        Kind::Hard(12),
        2,
        3,
        AtLeast(3),
        Stand,
        None,
        "Hi-Lo: stand hard12 vs2 at TC+3 or above.",
    ),
    index(
        Kind::Hard(12),
        3,
        3,
        AtLeast(2),
        Stand,
        None,
        "Hi-Lo: stand hard12 vs3 at TC+2 or above.",
    ),
    index(
        Kind::Hard(12),
        4,
        3,
        NegativeRunning,
        Hit,
        None,
        "Hi-Lo: hit hard12 vs4 when RC is strictly negative.",
    ),
    index(
        Kind::Hard(11),
        11,
        2,
        AtLeast(1),
        Double,
        None,
        "Hi-Lo S17: double hard11 vs ace at TC+1 or above.",
    ),
    index(
        Kind::Hard(10),
        10,
        3,
        AtLeast(4),
        Double,
        None,
        "Hi-Lo: double hard10 vs10 at TC+4 or above.",
    ),
    index(
        Kind::Hard(10),
        11,
        1,
        AtLeast(3),
        Double,
        None,
        "Hi-Lo H17: double hard10 vs ace at TC+3 or above.",
    ),
    index(
        Kind::Hard(10),
        11,
        2,
        AtLeast(4),
        Double,
        None,
        "Hi-Lo S17: double hard10 vs ace at TC+4 or above.",
    ),
    index(
        Kind::Hard(9),
        2,
        3,
        AtLeast(1),
        Double,
        None,
        "Hi-Lo: double hard9 vs2 at TC+1 or above.",
    ),
    index(
        Kind::Hard(9),
        7,
        3,
        AtLeast(3),
        Double,
        None,
        "Hi-Lo: double hard9 vs7 at TC+3 or above.",
    ),
    index(
        Kind::Hard(8),
        6,
        3,
        AtLeast(2),
        Double,
        None,
        "Hi-Lo: double hard8 vs6 at TC+2 or above.",
    ),
    index(
        Kind::Surrender(16),
        8,
        3,
        AtLeast(4),
        Surrender,
        Some(Hit),
        "Hi-Lo: surrender hard16 vs8 at TC+4; otherwise hit.",
    ),
    index(
        Kind::Surrender(16),
        9,
        3,
        AtMost(-1),
        Hit,
        Some(Surrender),
        "Hi-Lo: hit hard16 vs9 at TC-1 or below; otherwise surrender.",
    ),
    index(
        Kind::Surrender(15),
        9,
        3,
        AtLeast(2),
        Surrender,
        Some(Hit),
        "Hi-Lo: surrender hard15 vs9 at TC+2; otherwise hit.",
    ),
    index(
        Kind::Surrender(15),
        10,
        3,
        NegativeRunning,
        Hit,
        Some(Surrender),
        "Hi-Lo: hit hard15 vs10 at negative RC; otherwise surrender.",
    ),
    index(
        Kind::Surrender(15),
        11,
        1,
        AtLeast(-1),
        Surrender,
        Some(Hit),
        "Hi-Lo H17: surrender hard15 vs ace at TC-1 or above.",
    ),
    index(
        Kind::Surrender(15),
        11,
        2,
        AtLeast(2),
        Surrender,
        Some(Hit),
        "Hi-Lo S17: surrender hard15 vs ace at TC+2 or above.",
    ),
];
fn apply(mut advice: Advice, i: Index, count: Count, tc: i16, mask: u8) -> Advice {
    let reached = i.condition.reached(count.running, tc);
    let chosen = if reached { Some(i.play) } else { i.otherwise };
    if let Some(chosen) = chosen
        && mask & chosen.bit() != 0
        && advice.play.is_some_and(|r| r.best != chosen)
    {
        advice.play = Some(Recommendation {
            best: chosen,
            acceptable_mask: chosen.bit(),
            explanation: i.why,
        });
        advice.change = Some(IndexDecision {
            threshold: i.condition,
            reached,
        });
    }
    advice
}
pub fn recommend(hand: &Hand, upcard: Card, legal: u8, rules: Rules, count: Count) -> Advice {
    let base = strategy::recommend(hand, upcard, legal, rules);
    let tc = count.integer_true_count(rules);
    let out = Advice {
        play: base,
        integer_true_count: tc,
        change: None,
    };
    let (Some(base), Some(tc)) = (base, tc) else {
        return out;
    };
    let mask = strategy::normalized_mask(hand, legal, rules);
    let gate = if rules.hit_soft_17 { 1 } else { 2 };
    let up = upcard.value();
    let pair = hand.count() == 2 && hand.cards()[0].value() == hand.cards()[1].value();
    let total = hand.total();
    let matching = |kind| {
        INDICES
            .iter()
            .find(|i| i.kind == kind && i.up == up && i.gate & gate != 0)
            .copied()
    };
    if pair && hand.cards()[0].value() == 10 {
        return matching(Kind::Pair(10)).map_or(out, |i| apply(out, i, count, tc, mask));
    }
    if base.best == Split {
        return out;
    }
    if !hand.soft() && mask & Surrender.bit() != 0 {
        if let Some(i) = matching(Kind::Surrender(total)) {
            return apply(out, i, count, tc, mask);
        }
        if base.best == Surrender {
            return out;
        }
    }
    let kind = if hand.soft() {
        Kind::Soft(total)
    } else {
        Kind::Hard(total)
    };
    matching(kind).map_or(out, |i| apply(out, i, count, tc, mask))
}
pub fn insurance(affordable: bool, rules: Rules, count: Count) -> Option<bool> {
    count
        .integer_true_count(rules)
        .map(|tc| affordable && tc >= 3)
}

// RESUME10 Count
impl crate::resume_private::Wire for Count {
    fn initial() -> Self {
        Self {
            running: crate::resume_private::Wire::initial(),
            undealt: crate::resume_private::Wire::initial(),
            decks: crate::resume_private::Wire::initial(),
            observed: crate::resume_private::Wire::initial(),
            shoe: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            running,
            undealt,
            decks,
            observed,
            shoe,
        } = self;
        running.put(w)?;
        undealt.put(w)?;
        decks.put(w)?;
        observed.put(w)?;
        shoe.put(w)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            running: crate::resume_private::Wire::get(r)?,
            undealt: crate::resume_private::Wire::get(r)?,
            decks: crate::resume_private::Wire::get(r)?,
            observed: crate::resume_private::Wire::get(r)?,
            shoe: crate::resume_private::Wire::get(r)?,
        })
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
        } = self;
        running.read_into(r)?;
        undealt.read_into(r)?;
        decks.read_into(r)?;
        observed.read_into(r)?;
        shoe.read_into(r)?;
        Ok(())
    }
}
