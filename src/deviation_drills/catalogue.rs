//! Independent transcription of the named 2018 BJA H17/S17 red-cell indices.
//! Expected answers are not part of the public Question API.
use crate::deviation_drills::{IndexId, Threshold};
use crate::{engine::Action, rules::Rules};
#[derive(Clone, Copy)]
pub(crate) enum Shape {
    PairTens,
    Soft(u16),
    Hard(u16),
    Surrender(u16),
    Insurance,
}
#[derive(Clone, Copy)]
pub(crate) struct Entry {
    pub shape: Shape,
    pub up: u8,
    pub h17: bool,
    pub s17: bool,
    pub threshold: Threshold,
    pub above: Action,
    pub otherwise: Option<Action>,
}
impl Entry {
    pub fn enabled(self, r: Rules) -> bool {
        if r.hit_soft_17 { self.h17 } else { self.s17 }
    }
}
use Action::{Double, Hit, Split, Stand, Surrender};
use Shape::{Hard, Insurance, PairTens, Soft};
use Threshold::{AtLeast, AtMost, NegativeRunning, PositiveRunning};
const fn e(
    shape: Shape,
    up: u8,
    gate: u8,
    threshold: Threshold,
    above: Action,
    otherwise: Option<Action>,
) -> Entry {
    Entry {
        shape,
        up,
        h17: gate & 1 != 0,
        s17: gate & 2 != 0,
        threshold,
        above,
        otherwise,
    }
}
pub(crate) const ENTRIES: [Entry; IndexId::COUNT] = [
    e(PairTens, 4, 3, AtLeast(6), Split, None),
    e(PairTens, 5, 3, AtLeast(5), Split, None),
    e(PairTens, 6, 3, AtLeast(4), Split, None),
    e(Soft(19), 4, 3, AtLeast(3), Double, None),
    e(Soft(19), 5, 3, AtLeast(1), Double, None),
    e(Soft(19), 6, 1, NegativeRunning, Stand, None),
    e(Soft(19), 6, 2, AtLeast(1), Double, None),
    e(Soft(17), 2, 3, AtLeast(1), Double, None),
    e(Hard(16), 9, 3, AtLeast(4), Stand, None),
    e(Hard(16), 10, 3, PositiveRunning, Stand, None),
    e(Hard(16), 11, 1, AtLeast(3), Stand, None),
    e(Hard(15), 10, 3, AtLeast(4), Stand, None),
    e(Hard(15), 11, 1, AtLeast(5), Stand, None),
    e(Hard(13), 2, 3, AtMost(-1), Hit, None),
    e(Hard(12), 2, 3, AtLeast(3), Stand, None),
    e(Hard(12), 3, 3, AtLeast(2), Stand, None),
    e(Hard(12), 4, 3, NegativeRunning, Hit, None),
    e(Hard(11), 11, 2, AtLeast(1), Double, None),
    e(Hard(10), 10, 3, AtLeast(4), Double, None),
    e(Hard(10), 11, 1, AtLeast(3), Double, None),
    e(Hard(10), 11, 2, AtLeast(4), Double, None),
    e(Hard(9), 2, 3, AtLeast(1), Double, None),
    e(Hard(9), 7, 3, AtLeast(3), Double, None),
    e(Hard(8), 6, 3, AtLeast(2), Double, None),
    e(Shape::Surrender(16), 8, 3, AtLeast(4), Surrender, Some(Hit)),
    e(Shape::Surrender(16), 9, 3, AtMost(-1), Hit, Some(Surrender)),
    e(Shape::Surrender(15), 9, 3, AtLeast(2), Surrender, Some(Hit)),
    e(
        Shape::Surrender(15),
        10,
        3,
        NegativeRunning,
        Hit,
        Some(Surrender),
    ),
    e(
        Shape::Surrender(15),
        11,
        1,
        AtLeast(-1),
        Surrender,
        Some(Hit),
    ),
    e(
        Shape::Surrender(15),
        11,
        2,
        AtLeast(2),
        Surrender,
        Some(Hit),
    ),
    e(Insurance, 11, 3, AtLeast(3), Stand, None),
];
