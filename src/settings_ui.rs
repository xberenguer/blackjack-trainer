//! Settings controls and screens. Audio previews never supply play/count advice.
use super::*;
use crate::{
    canvas::{BACKGROUND_NAMES, rgb},
    render::{Align, CARD_BACK_NAMES, TextStyle},
    text::Text,
    widgets::{GOLD, MUTED, PANEL, Widgets},
};
use core::fmt::Write;
const SETTINGS: [&str; 7] = [
    "Display",
    "Sound",
    "Speed",
    "Add $1000 chips",
    "Count display",
    "Resume session",
    "Return",
];
const SOUND: [&str; 9] = [
    "Master sound",
    "Volume",
    "Menus",
    "Cards",
    "Results",
    "Test deal",
    "Test win",
    "Test menu",
    "Return",
];
const DISPLAY: [&str; 5] = [
    "Brightness",
    "Flip screen",
    "Return",
    "Background",
    "Card back",
];
const DISPLAY_ORDER: [usize; 5] = [0, 1, 3, 4, 2];
// Semantic IDs are stable across private saves: old Return remains ID3.
const ROUND_DISPLAY: [&str; 5] = [
    "Brightness",
    "Flip screen",
    "Background",
    "Return",
    "Card back",
];
const ROUND_DISPLAY_ORDER: [usize; 5] = [0, 1, 2, 4, 3];
pub(crate) fn item(view: View, selected: usize, round_display: bool) -> &'static str {
    match view {
        View::Settings => SETTINGS.get(selected),
        View::Sound => SOUND.get(selected),
        View::Display if round_display => ROUND_DISPLAY.get(selected),
        View::Display => DISPLAY.get(selected),
        _ => None,
    }
    .copied()
    .unwrap_or("")
}
impl App {
    pub(crate) fn paced_delay(&self, normal: u32, minimum: u32) -> u32 {
        if self.fast_mode {
            (normal / 2).max(minimum)
        } else {
            normal
        }
    }
    fn audition(&mut self, cue: Cue) {
        if self.sound_enabled && self.audio_volume != 0 {
            self.sounds.push_audition(cue)
        }
    }
    fn adjust_volume(&mut self, louder: bool) {
        let current = self.audio_volume.min(100);
        self.audio_volume = if louder {
            if current < 10 {
                current + 1
            } else {
                ((current / 5 + 1) * 5).min(100)
            }
        } else if current <= 10 {
            current.saturating_sub(1)
        } else {
            ((current - 1) / 5) * 5
        };
        if self.audio_volume == 0 {
            self.sounds.clear()
        }
    }
    pub fn brightness_percent(&self) -> u8 {
        ((u16::from(self.brightness) * 100 + 127) / 255) as u8
    }
    fn adjust_brightness(&mut self, lighter: bool) {
        let current = self.brightness_percent();
        let next = if lighter {
            current.saturating_add(5).min(100)
        } else {
            current.saturating_sub(5).max(8)
        };
        self.brightness = ((u16::from(next) * 255 + 50) / 100).clamp(20, 255) as u8;
    }
    pub(crate) fn handle_settings_event(&mut self, event: Event) -> bool {
        if event == Event::Hold(Key::Grey) {
            let parent = match self.view {
                View::Sound => Some((View::Settings, 1)),
                View::Display => Some((View::Settings, 0)),
                View::AudioVolume => Some((View::Sound, 1)),
                View::Brightness => Some((View::Display, 0)),
                View::CardBackPreview => Some((View::Display, 4)),
                _ => None,
            };
            if let Some((view, row)) = parent {
                self.emit(Cue::MenuSelect);
                self.set(view);
                self.selected = row;
                return true;
            }
        }
        if self.view == View::AudioVolume && event == Event::Hold(Key::Blue) {
            self.audition(Cue::CardDeal);
            return true;
        }
        let grey = event == Event::Tap(Key::Grey);
        if !grey && event != Event::Tap(Key::Blue) {
            return false;
        }
        match self.view {
            View::AudioVolume => {
                self.adjust_volume(!grey);
                self.emit(Cue::MenuMove);
            }
            View::Brightness => {
                self.adjust_brightness(!grey);
                self.emit(Cue::MenuMove);
            }
            View::Sound => {
                self.emit(Cue::MenuMove);
                if grey {
                    self.selected = (self.selected + 1) % SOUND.len()
                } else {
                    match self.selected {
                        0 => {
                            self.sound_enabled = !self.sound_enabled;
                            if self.sound_enabled {
                                self.emit(Cue::MenuSelect)
                            } else {
                                self.sounds.clear()
                            }
                        }
                        1 => self.set(View::AudioVolume),
                        2..=4 => self.audio_mask ^= 1 << (self.selected - 2),
                        5 => self.audition(Cue::CardDeal),
                        6 => self.audition(Cue::Win),
                        7 => self.audition(Cue::MenuSelect),
                        _ => {
                            self.set(View::Settings);
                            self.selected = 1;
                        }
                    }
                }
            }
            View::CardBackPreview => {
                self.emit(Cue::MenuMove);
                if grey {
                    self.card_back = (self.card_back + 1) % CARD_BACK_NAMES.len() as u8;
                } else {
                    self.set(View::Display);
                    self.selected = 4;
                }
            }
            View::Display => {
                self.emit(Cue::MenuMove);
                if grey {
                    let order = if self.round_display {
                        &ROUND_DISPLAY_ORDER
                    } else {
                        &DISPLAY_ORDER
                    };
                    self.selected = order
                        .iter()
                        .position(|id| *id == self.selected)
                        .map_or(0, |index| order[(index + 1) % order.len()]);
                } else {
                    match self.selected {
                        0 => self.set(View::Brightness),
                        1 => self.flipped = !self.flipped,
                        2 if self.round_display => {
                            self.background = (self.background + 1) % BACKGROUND_NAMES.len() as u8;
                        }
                        3 if !self.round_display => {
                            self.compact_background =
                                (self.compact_background + 1) % BACKGROUND_NAMES.len() as u8;
                        }
                        4 => self.set(View::CardBackPreview),
                        _ => self.set(View::Settings),
                    }
                }
            }
            _ => return false,
        }
        true
    }
    /// New preferences live in the mirrored public TLV, keeping the old private field order.
    pub(crate) fn apply_mirrored_preferences(&mut self, prefs: Save) {
        self.audio_volume = prefs.audio_volume;
        self.audio_mask = prefs.audio_mask;
        self.fast_mode = prefs.fast_mode;
        self.background = prefs.background;
        self.card_back = prefs.card_back;
        self.compact_background = prefs.compact_background;
    }
}
fn editor(w: &mut Widgets<'_>, title: &str, value: u8, volume: bool) {
    w.title(
        title,
        Some(if volume {
            "Fine steps below 10%"
        } else {
            "Display level"
        }),
        false,
    );
    let mut text = Text::<16>::new();
    let _ = write!(text, "{value}%");
    w.label(
        (120, 48),
        text.as_str(),
        TextStyle {
            size: 24,
            bold: true,
            color: GOLD,
            align: Align::Center,
        },
    );
    w.frame.rect(24, 84, 192, 6, PANEL);
    w.frame.rect(
        24,
        84,
        (192 * u16::from(value) / 100) as i32,
        6,
        rgb(160, 160, 160),
    );
    w.label(
        (120, 101),
        if volume {
            "HOLD BLUE: TEST   HOLD GREY: BACK"
        } else {
            "HOLD GREY: BACK"
        },
        TextStyle {
            size: 9,
            bold: false,
            color: MUTED,
            align: Align::Center,
        },
    );
    w.footer("GREY: LOWER   BLUE: HIGHER");
}
pub(crate) fn draw(a: &mut App) -> bool {
    if !matches!(
        a.view,
        View::Settings | View::Sound | View::AudioVolume | View::Display | View::Brightness
    ) {
        return false;
    }
    let percent = a.brightness_percent();
    let mut w = Widgets {
        frame: &mut a.frame,
        renderer: &mut a.renderer,
    };
    match a.view {
        View::Settings => {
            w.title(
                concat!("SETTINGS v", env!("CARGO_PKG_VERSION")),
                None,
                false,
            );
            let mut count = Text::<40>::new();
            let setting = if a.training.mode == Mode::Guide {
                a.guide_count_display
            } else {
                a.count_display
            };
            let _ = write!(
                count,
                "{} count: {}",
                if a.training.mode == Mode::Guide {
                    "Guide"
                } else {
                    "Normal"
                },
                ["OFF", "RC", "RC + TC"][setting as usize]
            );
            w.list(
                &[
                    "Display",
                    "Sound",
                    if a.fast_mode {
                        "Speed: Fast"
                    } else {
                        "Speed: Normal"
                    },
                    "Add $1000 chips",
                    count.as_str(),
                    if a.resume_enabled {
                        "Resume session: ON"
                    } else {
                        "Resume session: OFF"
                    },
                    "Return",
                ],
                a.selected,
                None,
            );
        }
        View::Sound => {
            w.title("SOUND", None, false);
            let mut volume = Text::<24>::new();
            let _ = write!(volume, "Volume: {}%", a.audio_volume);
            w.list(
                &[
                    if a.sound_enabled {
                        "Master sound: ON"
                    } else {
                        "Master sound: OFF"
                    },
                    volume.as_str(),
                    if a.audio_mask & 1 != 0 {
                        "Menus: ON"
                    } else {
                        "Menus: OFF"
                    },
                    if a.audio_mask & 2 != 0 {
                        "Cards: ON"
                    } else {
                        "Cards: OFF"
                    },
                    if a.audio_mask & 4 != 0 {
                        "Results: ON"
                    } else {
                        "Results: OFF"
                    },
                    "Test deal",
                    "Test win",
                    "Test menu",
                    "Return",
                ],
                a.selected,
                None,
            );
        }
        View::Display => {
            w.title("DISPLAY", None, false);
            let mut brightness = Text::<32>::new();
            let _ = write!(brightness, "Brightness: {percent}%");
            let screen = if a.flipped {
                "Screen: FLIPPED"
            } else {
                "Screen: NORMAL"
            };
            if a.round_display {
                let mut background = Text::<24>::new();
                let _ = write!(
                    background,
                    "Background: {}",
                    BACKGROUND_NAMES[a.background.min(3) as usize]
                );
                let mut card_back = Text::<24>::new();
                let _ = write!(
                    card_back,
                    "Card back: {}",
                    CARD_BACK_NAMES[a.card_back.min(3) as usize]
                );
                let selected = ROUND_DISPLAY_ORDER
                    .iter()
                    .position(|id| *id == a.selected)
                    .unwrap_or(0);
                w.list(
                    &[
                        brightness.as_str(),
                        screen,
                        background.as_str(),
                        card_back.as_str(),
                        "Return",
                    ],
                    selected,
                    None,
                );
            } else {
                w.list(&[brightness.as_str(), screen, "Return"], a.selected, None);
            }
        }
        View::AudioVolume => editor(&mut w, "VOLUME", a.audio_volume, true),
        View::Brightness => editor(&mut w, "BRIGHTNESS", percent, false),
        _ => unreachable!(),
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::boxed::Box;
    #[test]
    fn fast_card_stages_keep_minimum_visible_dwell_and_hide_count_until_complete() {
        let mut a = App::new(7);
        a.fast_mode = true;
        assert!(a.game.configure_seats(2, 0));
        a.event(Event::Tap(Key::Blue));
        a.event(Event::Tap(Key::Blue));
        let observed = a.tracker.observed_count();
        let mut now = 0;
        while let Some(reveal) = a.reveal {
            let delay = if reveal.progress >= 4 { 400 } else { 120 };
            a.tick(now + delay - 1);
            a.render();
            assert_eq!(a.reveal.unwrap().progress, reveal.progress);
            assert_eq!(a.tracker.observed_count(), observed);
            assert!(a.is_busy());
            a.tick(now + delay);
            now += delay;
        }
        assert!(a.tracker.observed_count() > observed);
        assert!(
            !a.is_busy() || a.game.phase == Phase::OtherPlayers || a.game.phase == Phase::Dealer
        );
        let mut b = App::new(7);
        b.fast_mode = false;
        b.event(Event::Tap(Key::Blue));
        b.event(Event::Tap(Key::Blue));
        b.tick(179);
        assert_eq!(b.reveal.unwrap().progress, 0);
        b.tick(180);
        assert_eq!(b.reveal.unwrap().progress, 1);
    }
    #[test]
    fn fast_queued_faces_keep_full_dwell_after_private_restore() {
        let mut found = false;
        for seed in 1..40 {
            let mut a = Box::new(App::new(seed));
            a.fast_mode = true;
            a.game.configure_seats(2, 0);
            a.event(Event::Tap(Key::Blue));
            a.event(Event::Tap(Key::Blue));
            let mut now = 0;
            for _ in 0..100 {
                if a.other_reveal_len != 0 {
                    let at = a.other_reveal_at;
                    let seen = a.tracker.observed_count();
                    a.render();
                    let mut packet = std::vec![0;crate::resume_private::MAX_BYTES];
                    let n =
                        crate::resume_private::capture_private(&a, &mut packet, 41, true).unwrap();
                    let mut b = Box::new(App::EMPTY);
                    crate::resume_private::restore_candidate(&mut b, &packet[..n], 20000, true)
                        .unwrap();
                    assert!(b.fast_mode);
                    b.tick(20399);
                    b.render();
                    assert_eq!(b.other_reveal_at, at);
                    assert_eq!(b.tracker.observed_count(), seen);
                    assert!(b.is_busy());
                    b.tick(20400);
                    assert!(b.other_reveal_len == 0 || b.other_reveal_at != at);
                    found = true;
                    break;
                }
                now += 400;
                a.tick(now);
                if a.reveal.is_none()
                    && a.other_reveal_len == 0
                    && matches!(a.game.phase, Phase::Player | Phase::Insurance)
                {
                    a.event(Event::Tap(Key::Blue));
                }
                a.render();
            }
            if found {
                break;
            }
        }
        assert!(found, "seeded fixture must exercise a queued visible draw");
    }
    #[test]
    fn fast_mode_does_not_time_out_reviews_or_quizzes_or_shorten_result_reveal() {
        let mut a = App::new(7);
        a.fast_mode = true;
        a.training.mode = Mode::Test;
        a.view = View::Table;
        a.game.phase = Phase::Result;
        a.test_round = true;
        a.round_id = 1;
        a.last_review = 0;
        a.review_due = Some(0);
        let mut hand = crate::cards::Hand::EMPTY;
        hand.bet = 500;
        hand.add(crate::cards::Card::from_id(3).unwrap(), 10);
        hand.add(crate::cards::Card::from_id(5).unwrap(), 11);
        a.training.record(training::Attempt {
            token: 1,
            round: 1,
            hand_index: 0,
            profile: Profile::SixH17,
            hand,
            dealer_up: crate::cards::Card::from_id(4).unwrap(),
            legal_mask: 3,
            choice: Choice::Play(Action::Hit),
            policy: Policy::Basic,
            count: None,
            assisted: false,
            values: None,
        });
        a.tick(899);
        assert_eq!(a.view, View::Table);
        a.tick(900);
        assert_eq!(a.view, View::RoundReview);
        a.tick(1000000);
        assert_eq!(a.view, View::RoundReview);
        let mut b = App::new(7);
        b.fast_mode = true;
        b.open_count(Kind::Running);
        let q = b.count_practice.question().unwrap();
        b.tick(1000000);
        assert_eq!(b.view, View::CountAnswer);
        assert_eq!(b.count_practice.question(), Some(q));
        assert_eq!(b.count_practice.stats().all.decisions, 0);
    }
}
