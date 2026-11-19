//! Shared opt-in/settings and boot chooser prototype; no storage or hidden payload access.
use crate::controls::{Event, Key};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Preference {
    enabled: bool,
}
impl Preference {
    pub const OFF: Self = Self { enabled: false };
    pub const fn from_enabled(enabled: bool) -> Self {
        Self { enabled }
    }
    pub fn enabled(self) -> bool {
        self.enabled
    }
    pub fn label(self) -> &'static str {
        if self.enabled {
            "Resume session: ON"
        } else {
            "Resume session: OFF"
        }
    }
    pub fn proposed_toggle(self) -> Self {
        Self {
            enabled: !self.enabled,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootState {
    Fresh,
    ChooseSaved { row: u8 },
    ReadFailure { row: u8 },
    Restoring,
    StartingNew,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    None,
    RetryRead,
    RestoreCandidate,
    CommitNewSessionTombstone,
}
pub struct BootChooser {
    pub state: BootState,
    neutral: bool,
}
impl BootChooser {
    pub fn new(pref: Preference, valid_saved: bool, read_failed: bool) -> Self {
        Self {
            state: if !pref.enabled() {
                BootState::Fresh
            } else if read_failed {
                BootState::ReadFailure { row: 0 }
            } else if valid_saved {
                BootState::ChooseSaved { row: 0 }
            } else {
                BootState::Fresh
            },
            neutral: false,
        }
    }
    pub fn neutral(&mut self, down: [bool; 2]) -> bool {
        if !self.neutral {
            if down == [false; 2] {
                self.neutral = true
            }
            false
        } else {
            true
        }
    }
    pub fn rows(&self) -> [&'static str; 2] {
        match self.state {
            BootState::ReadFailure { .. } => ["Retry saved session", "Start a new session"],
            _ => ["Resume saved session", "Start a new session"],
        }
    }
    pub fn event(&mut self, event: Event) -> Command {
        if !self.neutral {
            return Command::None;
        }
        match (&mut self.state, event) {
            (
                BootState::ChooseSaved { row } | BootState::ReadFailure { row },
                Event::Tap(Key::Grey),
            ) => {
                *row = 1 - *row;
                Command::None
            }
            (BootState::ChooseSaved { row: 0 }, Event::Tap(Key::Blue)) => {
                self.state = BootState::Restoring;
                Command::RestoreCandidate
            }
            (BootState::ReadFailure { row: 0 }, Event::Tap(Key::Blue)) => Command::RetryRead,
            (
                BootState::ChooseSaved { row: 1 } | BootState::ReadFailure { row: 1 },
                Event::Tap(Key::Blue),
            ) => {
                self.state = BootState::StartingNew;
                Command::CommitNewSessionTombstone
            }
            _ => Command::None,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_off_and_explicit_menu_toggle() {
        let p = Preference::default();
        assert!(!p.enabled());
        assert_eq!(p.label(), "Resume session: OFF");
        let proposal = p.proposed_toggle();
        assert!(proposal.enabled());
        assert!(
            !p.enabled(),
            "proposal alone must not mutate durable preference"
        );
    }
    #[test]
    fn boot_actions_wait_for_neutral_and_preserve_saved_default() {
        let mut c = BootChooser::new(Preference::OFF.proposed_toggle(), true, false);
        assert_eq!(c.event(Event::Tap(Key::Blue)), Command::None);
        assert!(!c.neutral([true, false]));
        assert!(!c.neutral([false, false]));
        assert_eq!(c.event(Event::Tap(Key::Blue)), Command::RestoreCandidate);
        let mut c = BootChooser::new(Preference::OFF.proposed_toggle(), true, true);
        c.neutral([false, false]);
        assert_eq!(c.event(Event::Tap(Key::Blue)), Command::RetryRead);
        c.event(Event::Tap(Key::Grey));
        assert_eq!(
            c.event(Event::Tap(Key::Blue)),
            Command::CommitNewSessionTombstone
        );
    }
}
