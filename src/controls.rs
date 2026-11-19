//! The same deterministic recognizer will serve physical keys and browser keys.
//! Long presses fire once; a chord consumes both release tails until neutral.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Key {
    Blue,
    Grey,
}
impl Key {
    fn at(index: usize) -> Self {
        if index == 0 { Self::Blue } else { Self::Grey }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Tap(Key),
    Hold(Key),
    SubmitCount,
}
pub struct Controls {
    pressed: [bool; 2],
    used: [bool; 2],
    started: [u32; 2],
    chord: bool,
}
impl Default for Controls {
    fn default() -> Self {
        Self::EMPTY
    }
}
impl Controls {
    pub const EMPTY: Self = Self {
        pressed: [false; 2],
        used: [false; 2],
        started: [0; 2],
        chord: false,
    };
    pub const HOLD_MS: u32 = 850;
    pub fn update(
        &mut self,
        down: [bool; 2],
        now: u32,
        can_submit_count: bool,
    ) -> [Option<Event>; 2] {
        let mut out = [None; 2];
        let mut count = 0;
        if down[0] && down[1] && !self.chord {
            if can_submit_count
                && (!self.pressed[0] || !self.used[0])
                && (!self.pressed[1] || !self.used[1])
            {
                out[count] = Some(Event::SubmitCount);
                count += 1;
            }
            self.chord = true;
            self.used = [true; 2];
        }
        for (i, &is_down) in down.iter().enumerate() {
            if is_down && !self.pressed[i] {
                self.started[i] = now;
                self.used[i] = self.chord;
            }
            if is_down
                && !self.used[i]
                && !self.chord
                && now.wrapping_sub(self.started[i]) >= Self::HOLD_MS
            {
                out[count] = Some(Event::Hold(Key::at(i)));
                count += 1;
                self.used[i] = true;
            }
            if !is_down && self.pressed[i] && !self.used[i] && !self.chord {
                // Sampling may skip the exact hold threshold, especially in a
                // background browser tab. Classify by elapsed time on release.
                out[count] = Some(if now.wrapping_sub(self.started[i]) >= Self::HOLD_MS {
                    Event::Hold(Key::at(i))
                } else {
                    Event::Tap(Key::at(i))
                });
                count += 1;
            }
            self.pressed[i] = is_down;
        }
        if !down[0] && !down[1] {
            self.chord = false;
        }
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_classifies_duration_without_a_threshold_sample() {
        for (key, down) in [(Key::Blue, [true, false]), (Key::Grey, [false, true])] {
            for duration in [849, 850, 851, 5000] {
                let mut c = Controls::default();
                let start = u32::MAX - 100;
                c.update(down, start, false);
                let event = if duration < 850 {
                    Event::Tap(key)
                } else {
                    Event::Hold(key)
                };
                assert_eq!(
                    c.update([false; 2], start.wrapping_add(duration), false),
                    [Some(event), None]
                );
                assert_eq!(
                    c.update([false; 2], start.wrapping_add(duration + 1), false),
                    [None; 2]
                );
            }
        }
    }
    #[test]
    fn taps_and_holds_are_exclusive() {
        for (key, down) in [(Key::Blue, [true, false]), (Key::Grey, [false, true])] {
            let mut c = Controls::default();
            assert_eq!(c.update(down, 10, false), [None; 2]);
            assert_eq!(c.update(down, 859, false), [None; 2]);
            assert_eq!(c.update(down, 860, false), [Some(Event::Hold(key)), None]);
            assert_eq!(c.update(down, 5000, false), [None; 2]);
            assert_eq!(c.update([false; 2], 5001, false), [None; 2]);
            c.update(down, 6000, false);
            assert_eq!(
                c.update([false; 2], 6050, false),
                [Some(Event::Tap(key)), None]
            );
        }
    }
    #[test]
    fn chords_submit_only_in_quizzes_and_consume_both_tails() {
        for quiz in [false, true] {
            for first in [[true, false], [false, true], [true, true]] {
                let mut c = Controls::default();
                let initial = c.update(first, 100, quiz);
                let both = c.update([true, true], 150, quiz);
                let submits = initial
                    .iter()
                    .chain(both.iter())
                    .filter(|e| **e == Some(Event::SubmitCount))
                    .count();
                assert_eq!(submits, usize::from(quiz));
                assert_eq!(c.update([true, true], 1100, quiz), [None; 2]);
                assert_eq!(c.update([false, true], 1200, quiz), [None; 2]);
                assert_eq!(c.update([false, true], 2200, quiz), [None; 2]);
                assert_eq!(c.update([false, false], 2300, quiz), [None; 2]);
            }
        }
    }
    #[test]
    fn completed_hold_cannot_also_submit() {
        let mut c = Controls::default();
        c.update([true, false], 0, true);
        assert_eq!(
            c.update([true, false], 850, true),
            [Some(Event::Hold(Key::Blue)), None]
        );
        assert_eq!(c.update([true, true], 900, true), [None; 2]);
        c.update([false, false], 1000, true);
        assert_eq!(
            c.update([true, true], 1100, true),
            [Some(Event::SubmitCount), None]
        );
    }
    #[test]
    fn millisecond_clock_wrap_does_not_break_holds() {
        let mut c = Controls::default();
        let start = u32::MAX - 50;
        c.update([false, true], start, false);
        assert_eq!(
            c.update([false, true], start.wrapping_add(849), false),
            [None; 2]
        );
        assert_eq!(
            c.update([false, true], start.wrapping_add(850), false),
            [Some(Event::Hold(Key::Grey)), None]
        );
    }
}
