use crate::{cards::Card, rules::MAX_CARDS};
// No shoe/engine reference: the caller must explicitly supply each exposed face.
#[derive(Clone)]
pub struct Tracker {
    seen: [bool; MAX_CARDS],
    decks: u8,
    shoe: u32,
    observed: u16,
    running: i16,
}
impl Tracker {
    pub const EMPTY: Self = Self {
        seen: [false; MAX_CARDS],
        decks: 0,
        shoe: 0,
        observed: 0,
        running: 0,
    };
}
impl Default for Tracker {
    fn default() -> Self {
        Self::EMPTY
    }
}
impl Tracker {
    pub fn reset(&mut self, decks: u8, shoe: u32) -> bool {
        *self = Self::default();
        if !matches!(decks, 6 | 8) || shoe == 0 {
            return false;
        }
        self.decks = decks;
        self.shoe = shoe;
        true
    }
    pub fn ensure_shoe(&mut self, decks: u8, shoe: u32) {
        if self.decks != decks || self.shoe != shoe {
            self.reset(decks, shoe);
        }
    }
    pub fn observe(&mut self, position: u16, card: Card) -> bool {
        let i = position as usize;
        if self.decks == 0 || i >= self.decks as usize * 52 || self.seen[i] {
            return false;
        }
        self.seen[i] = true;
        self.observed += 1;
        self.running += match card.rank() {
            2..=6 => 1,
            10..=14 => -1,
            _ => 0,
        };
        true
    }
    pub fn running_count(&self) -> i16 {
        self.running
    }
    pub fn observed_count(&self) -> u16 {
        self.observed
    }
    pub fn decks(&self) -> u8 {
        self.decks
    }
    pub fn shoe_number(&self) -> u32 {
        self.shoe
    }
    pub fn true_count(&self, undealt: u16) -> Option<f64> {
        if self.decks == 0 || undealt == 0 || undealt > self.decks as u16 * 52 {
            None
        } else {
            Some(self.running as f64 / (undealt as f64 / 52.0))
        }
    }
    pub fn truncated_index(&self, undealt: u16) -> Option<i16> {
        self.true_count(undealt)
            .map(|_| (self.running as i32 * 52 / undealt as i32) as i16)
    }
}

// RESUME10 Tracker
impl crate::resume_private::Wire for Tracker {
    fn initial() -> Self {
        Self {
            seen: crate::resume_private::Wire::initial(),
            decks: crate::resume_private::Wire::initial(),
            shoe: crate::resume_private::Wire::initial(),
            observed: crate::resume_private::Wire::initial(),
            running: crate::resume_private::Wire::initial(),
        }
    }
    fn put(&self, w: &mut crate::resume_private::Writer) -> crate::resume_private::Result<()> {
        let Self {
            seen,
            decks,
            shoe,
            observed,
            running,
        } = self;
        seen.put(w)?;
        decks.put(w)?;
        shoe.put(w)?;
        observed.put(w)?;
        running.put(w)?;
        Ok(())
    }
    fn read_into(
        &mut self,
        r: &mut crate::resume_private::Reader,
    ) -> crate::resume_private::Result<()> {
        let Self {
            seen,
            decks,
            shoe,
            observed,
            running,
        } = self;
        seen.read_into(r)?;
        decks.read_into(r)?;
        shoe.read_into(r)?;
        observed.read_into(r)?;
        running.read_into(r)?;
        Ok(())
    }
    fn get(r: &mut crate::resume_private::Reader) -> crate::resume_private::Result<Self> {
        Ok(Self {
            seen: crate::resume_private::Wire::get(r)?,
            decks: crate::resume_private::Wire::get(r)?,
            shoe: crate::resume_private::Wire::get(r)?,
            observed: crate::resume_private::Wire::get(r)?,
            running: crate::resume_private::Wire::get(r)?,
        })
    }
}
impl crate::resume_private::SnapshotCopy for Tracker {
    fn snapshot_copy_to(&self, out: &mut Self) {
        let Self {
            seen,
            decks,
            shoe,
            observed,
            running,
        } = self;
        crate::resume_private::SnapshotCopy::snapshot_copy_to(seen, &mut out.seen);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(decks, &mut out.decks);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(shoe, &mut out.shoe);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(observed, &mut out.observed);
        crate::resume_private::SnapshotCopy::snapshot_copy_to(running, &mut out.running);
    }
}
impl Tracker {
    pub(crate) fn resume_valid(&self, e: &crate::engine::Engine) -> bool {
        if self.decks != e.rules.decks || self.shoe != e.shoe_number {
            return false;
        }
        let mut observed = 0;
        let mut running = 0;
        for (p, &seen) in self.seen.iter().enumerate() {
            if seen {
                let Some(c) = e.resume_exposed_card(p) else {
                    return false;
                };
                observed += 1;
                running += match c.rank() {
                    2..=6 => 1,
                    10..=14 => -1,
                    _ => 0,
                };
            }
        }
        observed == self.observed && running == self.running
    }
    pub(crate) fn resume_seen(&self, p: u16) -> bool {
        self.seen.get(p as usize).copied().unwrap_or(false)
    }
}
