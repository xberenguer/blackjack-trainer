#[path = "support/navigation.rs"]
mod nav;
use stardust_blackjack::{
    app::App,
    audio::Cue,
    controls::{Event, Key},
    training::Mode,
};
const BLUE: Event = Event::Tap(Key::Blue);
const GREY: Event = Event::Tap(Key::Grey);
fn focus(a: &mut App, row: usize) {
    for _ in 0..30 {
        if a.focus() == row {
            return;
        }
        a.event(GREY)
    }
    panic!("missing row{row} in{}", a.view_name())
}
fn settings(a: &mut App) {
    nav::path(a, &["Settings"]);
}
fn sound(a: &mut App) {
    nav::path(a, &["Settings", "Sound"]);
    assert_eq!(a.view_name(), "Sound");
}
fn drain(a: &mut App) {
    while a.pop_sound().is_some() {}
}
#[test]
fn phoenix_volume_fine_range_clamps_and_audition_keeps_mode_mute_and_mask() {
    let mut a = App::new(7);
    sound(&mut a);
    focus(&mut a, 1);
    a.event(BLUE);
    assert_eq!(a.view_name(), "AudioVolume");
    a.audio_mask = 0;
    a.audio_volume = 0;
    drain(&mut a);
    a.event(Event::Hold(Key::Blue));
    assert_eq!(a.pop_sound(), None);
    for expected in (1..=10).chain((15..=100).step_by(5)) {
        a.event(BLUE);
        assert_eq!(a.audio_volume, expected)
    }
    a.event(BLUE);
    assert_eq!(a.audio_volume, 100);
    for expected in (10..=95).step_by(5).rev().chain((0..10).rev()) {
        a.event(GREY);
        assert_eq!(a.audio_volume, expected)
    }
    a.event(GREY);
    assert_eq!(a.audio_volume, 0);
    a.audio_volume = 53;
    a.event(GREY);
    assert_eq!(a.audio_volume, 50);
    a.event(BLUE);
    assert_eq!(a.audio_volume, 55);
    drain(&mut a);
    let before = (
        a.training.mode,
        a.game.position,
        a.game.rounds,
        a.training.stats(),
    );
    a.event(Event::Hold(Key::Blue));
    assert_eq!(a.pop_sound(), Some(Cue::CardDeal));
    assert_eq!(
        (
            a.training.mode,
            a.game.position,
            a.game.rounds,
            a.training.stats()
        ),
        before
    );
    assert_eq!(a.audio_mask, 0);
    a.sound_enabled = false;
    a.event(Event::Hold(Key::Blue));
    assert_eq!(a.pop_sound(), None);
    assert_eq!(a.audio_volume, 55);
    assert!(!a.sound_enabled);
    a.event(Event::Hold(Key::Grey));
    assert_eq!(a.view_name(), "Sound");
    assert_eq!(a.focus(), 1);
    nav::choose(&mut a, "Test sounds");
    nav::choose(&mut a, "Win");
    assert_eq!(a.pop_sound(), None);
    assert!(!a.sound_enabled);
    a.sound_enabled = true;
    a.event(BLUE);
    assert_eq!(a.pop_sound(), Some(Cue::Win));
    assert_eq!(a.training.mode, Mode::Off);
}
#[test]
fn category_filters_cover_all_cues_and_filter_old_queued_feedback() {
    for cue in Cue::ALL {
        let expected = match cue {
            Cue::MenuMove | Cue::MenuSelect | Cue::Chip | Cue::BetMax => 1,
            Cue::CardDeal | Cue::CardFlip | Cue::Shuffle => 2,
            _ => 4,
        };
        assert_eq!(cue.category_bit(), expected);
    }
    let mut a = App::new(1);
    a.event(Event::Hold(Key::Grey));
    a.audio_mask = 2;
    assert_eq!(a.pop_sound(), None);
    a.audio_mask = 1;
    a.event(GREY);
    assert_eq!(a.pop_sound(), Some(Cue::MenuMove));
    a.sound_enabled = false;
    a.event(GREY);
    assert_eq!(a.pop_sound(), None);
    sound(&mut a);
    let volume = a.audio_volume;
    a.event(BLUE);
    assert!(a.sound_enabled);
    a.event(BLUE);
    assert!(!a.sound_enabled);
    assert_eq!(a.audio_volume, volume);
    assert_eq!(a.pop_sound(), None);
}
#[test]
fn settings_parent_order_display_steps_speed_toggle_and_add_chips_are_retained() {
    let mut a = App::new(1);
    settings(&mut a);
    assert_eq!(a.menu_item(), "Display");
    a.event(BLUE);
    assert_eq!(a.view_name(), "Display");
    a.event(BLUE);
    assert_eq!(a.view_name(), "Brightness");
    a.event(GREY);
    assert_eq!(a.brightness_percent(), 95);
    for _ in 0..30 {
        a.event(GREY)
    }
    assert_eq!(a.brightness, 20);
    for _ in 0..30 {
        a.event(BLUE)
    }
    assert_eq!(a.brightness, 255);
    a.event(Event::Hold(Key::Grey));
    assert_eq!(a.view_name(), "Display");
    focus(&mut a, 1);
    a.event(BLUE);
    assert!(a.flipped);
    a.event(Event::Hold(Key::Grey));
    assert_eq!(a.view_name(), "Settings");
    focus(&mut a, 2);
    assert_eq!(a.audio_volume, 20);
    assert!(a.fast_mode);
    a.event(BLUE);
    assert!(!a.fast_mode);
    a.event(BLUE);
    assert!(a.fast_mode);
    nav::path(&mut a, &["Session", "Bankroll"]);
    nav::choose(&mut a, "Add $1000");
    assert_eq!(a.game.bankroll, 200000);
    assert_eq!(a.training.stats().all.decisions, 0);
    settings(&mut a);
    focus(&mut a, 5);
    assert_eq!(a.menu_item(), "Save/resume");
    a.event(BLUE);
    assert!(a.resume_enabled);
    focus(&mut a, 6);
    assert_eq!(a.menu_item(), "Back");
}

#[test]
fn grouped_menu_feedback_obeys_master_volume_categories_without_doubling_auditions() {
    let mut a = App::new(7);
    nav::path(&mut a, &["Training"]);
    drain(&mut a);
    a.event(GREY);
    assert_eq!(a.pop_sound(), Some(Cue::MenuMove));
    assert_eq!(a.pop_sound(), None);
    nav::focus(&mut a, "Practice");
    drain(&mut a);
    a.event(BLUE);
    assert_eq!(a.view_name(), "PracticeMenu");
    assert_eq!(a.pop_sound(), Some(Cue::MenuMove));
    assert_eq!(a.pop_sound(), None);
    a.event(Event::Hold(Key::Grey));
    assert_eq!(a.view_name(), "Training");
    assert_eq!(a.pop_sound(), Some(Cue::MenuSelect));
    assert_eq!(a.pop_sound(), None);
    for (master, volume, mask) in [(false, 20, 7), (true, 0, 7), (true, 20, 6)] {
        a.sound_enabled = master;
        a.audio_volume = volume;
        a.audio_mask = mask;
        a.event(GREY);
        a.event(BLUE);
        a.event(Event::Hold(Key::Grey));
        assert_eq!(a.pop_sound(), None);
    }
    a.sound_enabled = true;
    a.audio_volume = 20;
    a.audio_mask = 7;
    nav::path(&mut a, &["Settings", "Sound", "Test sounds"]);
    for (label, cue) in [
        ("Deal", Cue::CardDeal),
        ("Win", Cue::Win),
        ("Menu", Cue::MenuSelect),
    ] {
        nav::focus(&mut a, label);
        drain(&mut a);
        a.event(BLUE);
        assert_eq!(a.pop_sound(), Some(cue));
        assert_eq!(
            a.pop_sound(),
            None,
            "audition must not add a second navigation cue"
        );
    }
}
