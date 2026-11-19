use crate::assets::Clip;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Cue {
    MenuMove,
    MenuSelect,
    CardDeal,
    CardFlip,
    Chip,
    Win,
    Blackjack,
    Push,
    Loss,
    Shuffle,
    CreditRollup,
    BetMax,
    Category,
    LastAward,
}
impl Cue {
    pub const ALL: [Self; 14] = [
        Self::MenuMove,
        Self::MenuSelect,
        Self::CardDeal,
        Self::CardFlip,
        Self::Chip,
        Self::Win,
        Self::Blackjack,
        Self::Push,
        Self::Loss,
        Self::Shuffle,
        Self::CreditRollup,
        Self::BetMax,
        Self::Category,
        Self::LastAward,
    ];
    /// Menu feedback includes manual wager/chip controls; rollups and awards are results.
    pub const fn category_bit(self) -> u8 {
        match self {
            Self::MenuMove | Self::MenuSelect | Self::Chip | Self::BetMax => 1,
            Self::CardDeal | Self::CardFlip | Self::Shuffle => 2,
            _ => 4,
        }
    }
    pub fn clip(self) -> Option<Clip> {
        Clip::at(self as usize)
    }
    pub const fn event_id(self) -> u8 {
        match self {
            Self::MenuMove => 1,
            Self::MenuSelect => 2,
            Self::CardDeal => 3,
            Self::Chip => 4,
            Self::Win => 5,
            Self::Loss => 6,
            Self::Push => 7,
            Self::Shuffle => 8,
            Self::CardFlip => 9,
            Self::Blackjack => 10,
            Self::CreditRollup => 11,
            Self::BetMax => 12,
            Self::Category => 13,
            Self::LastAward => 14,
        }
    }
    pub const fn interrupts_on_input(self) -> bool {
        matches!(self, Self::Win | Self::Blackjack)
    }
}
pub struct SoundQueue {
    cues: [Cue; 16],
    auditions: [bool; 16],
    read: usize,
    write: usize,
}
impl SoundQueue {
    pub const EMPTY: Self = Self {
        cues: [Cue::MenuMove; 16],
        auditions: [false; 16],
        read: 0,
        write: 0,
    };
}
impl Default for SoundQueue {
    fn default() -> Self {
        Self::EMPTY
    }
}
impl SoundQueue {
    pub fn push(&mut self, cue: Cue) {
        self.push_kind(cue, false);
    }
    pub fn push_audition(&mut self, cue: Cue) {
        self.push_kind(cue, true);
    }
    fn push_kind(&mut self, cue: Cue, audition: bool) {
        let next = (self.write + 1) % self.cues.len();
        if next == self.read {
            self.read = (self.read + 1) % self.cues.len()
        }
        self.cues[self.write] = cue;
        self.auditions[self.write] = audition;
        self.write = next;
    }
    pub fn pop(&mut self) -> Option<Cue> {
        self.pop_entry().map(|entry| entry.0)
    }
    pub fn pop_entry(&mut self) -> Option<(Cue, bool)> {
        if self.read == self.write {
            return None;
        }
        let cue = (self.cues[self.read], self.auditions[self.read]);
        self.read = (self.read + 1) % self.cues.len();
        Some(cue)
    }
    pub fn clear(&mut self) {
        self.read = self.write;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sound_queue_retains_recent_cues_and_clear_is_immediate() {
        let mut q = SoundQueue::default();
        for n in 0..30 {
            q.push(Cue::ALL[n % 14])
        }
        for n in 15..30 {
            assert_eq!(q.pop(), Some(Cue::ALL[n % 14]))
        }
        assert_eq!(q.pop(), None);
        q.push(Cue::CardDeal);
        q.clear();
        assert_eq!(q.pop(), None);
        assert!(Cue::Blackjack.interrupts_on_input());
        assert!(!Cue::CardDeal.interrupts_on_input());
    }
}
