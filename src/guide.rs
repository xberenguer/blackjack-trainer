use crate::{
    cards::{Card, Hand},
    engine::Action,
    strategy::Recommendation,
};
pub fn short_reason(hand: &Hand, up: Card, advice: Recommendation) -> &'static str {
    if advice.explanation.contains("Only this permitted") {
        return "Only legal\nfallback";
    }
    if advice.explanation.contains("Doubling is unavailable") {
        return if advice.best == Action::Stand {
            "No double;\nkeep this\nsoft total"
        } else {
            "No double;\ntake a card"
        };
    }
    match advice.best {
        Action::Surrender => "Lose only\nhalf the bet",
        Action::Split => match hand.cards()[0].value() {
            8 => "Avoid a\nhard 16",
            11 => "Two strong\nace starts",
            4 => "Can double\nafter split",
            9 => "Better as\ntwo hands\nthan one 18",
            _ => "Better as\ntwo hands vs\nthis upcard",
        },
        Action::Double => {
            if hand.soft() {
                "Ace keeps\none draw\nflexible"
            } else if hand.total() == 11 {
                "Strong 11:\none card to\nwin more"
            } else {
                "Good total;\nbet more vs\nthis upcard"
            }
        }
        Action::Stand => {
            if hand.soft() {
                "Keep this\nstrong soft\ntotal"
            } else if hand.total() >= 17 {
                "Hard 17+;\nextra cards\nrisk a bust"
            } else {
                "Let dealer\nrisk a bust"
            }
        }
        Action::Hit => {
            if hand.soft() {
                "Ace can\ncount as 1;\nno bust yet"
            } else if hand.total() == 12 {
                "12 is weak\noutside\n4-6 upcards"
            } else if hand.total() >= 13 && up.value() >= 7 {
                "Dealer is\nstrong; try\nto improve"
            } else {
                "Low total;\ntake a card"
            }
        }
    }
}
pub const fn insurance_reason() -> &'static str {
    "Basic chart:\ndecline the\nside bet"
}
