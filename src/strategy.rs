//! Rule-bound total-dependent basic strategy. This module receives visible
//! faces and legal actions only; it has no engine, hole-card or shoe access.
use crate::{
    cards::{Card, Hand, Outcome},
    engine::Action,
    rules::Rules,
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Recommendation {
    pub best: Action,
    pub acceptable_mask: u8,
    pub explanation: &'static str,
}
impl Recommendation {
    pub fn accepts(self, action: Action) -> bool {
        self.acceptable_mask & action.bit() != 0
    }
}
fn choose(action: Action, explanation: &'static str) -> Recommendation {
    Recommendation {
        best: action,
        acceptable_mask: action.bit(),
        explanation,
    }
}
fn constrained(preferred: Action, mask: u8, why: &'static str) -> Option<Recommendation> {
    if mask & preferred.bit() != 0 {
        return Some(choose(preferred, why));
    }
    [
        Action::Stand,
        Action::Hit,
        Action::Double,
        Action::Split,
        Action::Surrender,
    ]
    .into_iter()
    .find(|a| mask & a.bit() != 0)
    .map(|a| choose(a, "Only this permitted fallback is available."))
}
fn double_or(mask: u8, fallback: Action, why: &'static str) -> Option<Recommendation> {
    if mask & Action::Double.bit() != 0 {
        Some(choose(Action::Double, why))
    } else {
        constrained(
            fallback,
            mask,
            if fallback == Action::Stand {
                "Doubling is unavailable; keep this strong soft hand."
            } else {
                "Doubling is unavailable; take another card."
            },
        )
    }
}
pub(crate) fn normalized_mask(hand: &Hand, mut mask: u8, rules: Rules) -> u8 {
    mask &= 31;
    let pair = hand.count() == 2 && hand.cards()[0].value() == hand.cards()[1].value();
    if hand.count() != 2 {
        mask &= !(Action::Double.bit() | Action::Split.bit() | Action::Surrender.bit())
    }
    if !pair {
        mask &= !Action::Split.bit()
    }
    if hand.split || !rules.late_surrender {
        mask &= !Action::Surrender.bit()
    }
    if hand.split && !rules.double_after_split {
        mask &= !Action::Double.bit()
    }
    if hand.split_aces || hand.total() >= 21 {
        mask &= Action::Stand.bit()
    }
    mask
}
fn split_pair(pair: u8, up: u8, das: bool) -> bool {
    match pair {
        11 | 8 => true,
        2 | 3 => (if das { 2 } else { 4 }..=7).contains(&up),
        7 => (2..=7).contains(&up),
        4 => das && matches!(up, 5 | 6),
        6 => (if das { 2 } else { 3 }..=6).contains(&up),
        9 => (2..=6).contains(&up) || matches!(up, 8 | 9),
        _ => false,
    }
}
pub fn recommend(hand: &Hand, upcard: Card, legal: u8, rules: Rules) -> Option<Recommendation> {
    if !rules.valid()
        || hand.count() < 2
        || hand.done
        || hand.outcome != Outcome::Pending
        || hand.total() > 21
        || hand.natural()
    {
        return None;
    }
    let mask = normalized_mask(hand, legal, rules);
    if mask == 0 {
        return None;
    }
    let total = hand.total();
    let up = upcard.value();
    let soft = hand.soft();
    let pair = if hand.count() == 2 && hand.cards()[0].value() == hand.cards()[1].value() {
        hand.cards()[0].value()
    } else {
        0
    };
    let surrender = !soft
        && ((total == 15 && (up == 10 || (rules.hit_soft_17 && up == 11)))
            || (total == 16 && matches!(up, 9..=11))
            || (rules.hit_soft_17 && total == 17 && up == 11));
    let split_eights = pair == 8
        && (matches!(up, 9 | 10) || (!rules.hit_soft_17 && up == 11))
        && mask & Action::Split.bit() != 0;
    if surrender && !split_eights && mask & Action::Surrender.bit() != 0 {
        return Some(choose(
            Action::Surrender,
            if pair == 8 && up == 11 {
                "H17: surrender eights against an ace."
            } else if total == 17 {
                "H17: surrender hard 17 against an ace."
            } else {
                "Late surrender limits this weak hand's loss."
            },
        ));
    }
    if pair != 0
        && split_pair(pair, up, rules.double_after_split)
        && mask & Action::Split.bit() != 0
    {
        return Some(choose(
            Action::Split,
            match pair {
                11 => "Split aces into two stronger starting hands.",
                8 => "Split eights; avoid playing a hard 16.",
                4 => "DAS makes splitting fours best against 5 or 6.",
                9 => "Split nines against 2-6, 8 or 9.",
                _ => "Split this pair against the dealer's upcard.",
            },
        ));
    }
    if soft {
        return match (total, up) {
            (12, 6) => double_or(mask, Action::Hit, "If aces cannot split, double against 6."),
            (13 | 14, 5 | 6) => {
                double_or(mask, Action::Hit, "Double soft 13 or 14 against 5 or 6.")
            }
            (15 | 16, 4..=6) => double_or(mask, Action::Hit, "Double soft 15 or 16 against 4-6."),
            (17, 3..=6) => double_or(mask, Action::Hit, "Double soft 17 against 3-6."),
            (18, 2..=6) if rules.hit_soft_17 || up >= 3 => double_or(
                mask,
                Action::Stand,
                "Double soft 18 against this dealer upcard.",
            ),
            (18, _) => constrained(
                if up <= 8 { Action::Stand } else { Action::Hit },
                mask,
                if up <= 8 {
                    "Keep soft 18 against this dealer upcard."
                } else {
                    "Hit soft 18 against 9, 10 or ace."
                },
            ),
            (19, 6) if rules.hit_soft_17 => {
                double_or(mask, Action::Stand, "H17: double soft 19 against 6.")
            }
            (19.., _) => constrained(Action::Stand, mask, "Keep this strong soft total."),
            _ => constrained(
                Action::Hit,
                mask,
                "A soft hand can improve without an immediate bust.",
            ),
        };
    }
    match (total, up) {
        (9, 3..=6) => double_or(mask, Action::Hit, "Double 9 against 3-6."),
        (10, 2..=9) => double_or(mask, Action::Hit, "Double 10 against 2-9."),
        (11, _) if up != 11 || rules.hit_soft_17 => {
            double_or(mask, Action::Hit, "Double 11 against this dealer upcard.")
        }
        (17.., _) => constrained(Action::Stand, mask, "Keep a hard total of 17 or more."),
        (12, 4..=6) => constrained(Action::Stand, mask, "Stand on 12 against 4-6."),
        (12, _) => constrained(Action::Hit, mask, "Hit 12 outside the dealer's 4-6 range."),
        (13..=16, 2..=6) => constrained(
            Action::Stand,
            mask,
            "Stand on 13-16 against the dealer's 2-6.",
        ),
        (13..=16, _) => constrained(
            Action::Hit,
            mask,
            "Without surrender or splitting, hit this weak total.",
        ),
        _ => constrained(
            Action::Hit,
            mask,
            "Take another card on this low hard total.",
        ),
    }
}
pub const fn recommend_insurance() -> bool {
    false
}
