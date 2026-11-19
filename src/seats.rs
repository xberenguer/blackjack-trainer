//! Optional simulated tablemates. They use rule-bound basic strategy and their
//! own virtual-chip ledgers; they never inspect a hidden card or use the count.
use crate::{
    cards::Hand,
    rules::{MAX_HANDS, Money},
};
pub const MAX_EXTRA_SEATS: usize = 6;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OtherSeat {
    pub hands: [Hand; MAX_HANDS],
    pub hand_count: usize,
    pub active: usize,
    pub bankroll: Money,
    pub deposits: Money,
    pub round_stake: Money,
    pub round_return: Money,
    pub round_net: Money,
    pub rounds: u32,
    pub(crate) held: [bool; MAX_HANDS],
}
impl OtherSeat {
    pub const EMPTY: Self = Self {
        hands: [Hand::EMPTY; MAX_HANDS],
        hand_count: 0,
        active: 0,
        bankroll: 100_000,
        deposits: 100_000,
        round_stake: 0,
        round_return: 0,
        round_net: 0,
        rounds: 0,
        held: [false; MAX_HANDS],
    };
    pub(crate) fn clear_round(&mut self) {
        self.hands = [Hand::EMPTY; MAX_HANDS];
        self.hand_count = 0;
        self.active = 0;
        self.round_stake = 0;
        self.round_return = 0;
        self.round_net = 0;
        self.held = [false; MAX_HANDS]
    }
}
impl Default for OtherSeat {
    fn default() -> Self {
        Self::EMPTY
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct Inventory {
    pub unseen: usize,
    pub held: usize,
    pub discarded: usize,
    pub total: usize,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Zone {
    Unseen,
    Held,
    Discard,
}

// RESUME10 OtherSeat
impl crate::resume_private::Wire for OtherSeat {
    fn initial() -> Self {
        Self {
            hands: crate::resume_private::Wire::initial(),
            hand_count: crate::resume_private::Wire::initial(),
            active: crate::resume_private::Wire::initial(),
            bankroll: crate::resume_private::Wire::initial(),
            deposits: crate::resume_private::Wire::initial(),
            round_stake: crate::resume_private::Wire::initial(),
            round_return: crate::resume_private::Wire::initial(),
            round_net: crate::resume_private::Wire::initial(),
            rounds: crate::resume_private::Wire::initial(),
            held: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            hands,
            hand_count,
            active,
            bankroll,
            deposits,
            round_stake,
            round_return,
            round_net,
            rounds,
            held,
        } = self;
        hands.put(w)?;
        hand_count.put(w)?;
        active.put(w)?;
        bankroll.put(w)?;
        deposits.put(w)?;
        round_stake.put(w)?;
        round_return.put(w)?;
        round_net.put(w)?;
        rounds.put(w)?;
        held.put(w)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            hands: crate::resume_private::Wire::get(r)?,
            hand_count: crate::resume_private::Wire::get(r)?,
            active: crate::resume_private::Wire::get(r)?,
            bankroll: crate::resume_private::Wire::get(r)?,
            deposits: crate::resume_private::Wire::get(r)?,
            round_stake: crate::resume_private::Wire::get(r)?,
            round_return: crate::resume_private::Wire::get(r)?,
            round_net: crate::resume_private::Wire::get(r)?,
            rounds: crate::resume_private::Wire::get(r)?,
            held: crate::resume_private::Wire::get(r)?,
        })
    }

    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            hands,
            hand_count,
            active,
            bankroll,
            deposits,
            round_stake,
            round_return,
            round_net,
            rounds,
            held,
        } = self;
        hands.read_into(r)?;
        hand_count.read_into(r)?;
        active.read_into(r)?;
        bankroll.read_into(r)?;
        deposits.read_into(r)?;
        round_stake.read_into(r)?;
        round_return.read_into(r)?;
        round_net.read_into(r)?;
        rounds.read_into(r)?;
        held.read_into(r)?;
        Ok(())
    }
}

// RESUME10 enum Zone
impl crate::resume_private::Wire for Zone {
    fn initial() -> Self {
        Self::Unseen
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        w.u8(*self as u8)
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        match r.u8()? {
            0 => Ok(Self::Unseen),
            1 => Ok(Self::Held),
            2 => Ok(Self::Discard),
            _ => Err(crate::resume_private::Error::Invalid),
        }
    }
}
