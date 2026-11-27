use stardust_blackjack::{
    app::App,
    controls::{Event, Key},
    persistence::{SIZE, Save, VERSION, crc32},
    resume_binding::{self, PublicCommit},
    resume_private as snapshot,
};
const BLUE: Event = Event::Tap(Key::Blue);
const GREY: Event = Event::Tap(Key::Grey);
fn resign(packet: &mut [u8]) {
    let end = packet.len() - 4;
    let crc = crc32(&packet[..end]);
    packet[end..].copy_from_slice(&crc.to_le_bytes());
}
fn v10(mut packet: [u8; SIZE]) -> [u8; SIZE] {
    packet[4..6].copy_from_slice(&10u16.to_le_bytes());
    packet[10] &= 7;
    resign(&mut packet);
    packet
}
fn display(a: &mut App) {
    a.event(Event::Hold(Key::Grey));
    for _ in 0..30 {
        if a.menu_item() == "Settings" {
            a.event(BLUE);
            a.event(BLUE);
            assert_eq!(a.view_name(), "Display");
            return;
        }
        a.event(GREY);
    }
    panic!("settings missing");
}
fn substitute_public(private: &mut [u8], public: &[u8]) {
    let mut at = snapshot::HEADER;
    let mut found = false;
    while at < private.len() {
        let tag = u16::from_le_bytes(private[at..at + 2].try_into().unwrap());
        let len = u32::from_le_bytes(private[at + 4..at + 8].try_into().unwrap()) as usize;
        at += 8;
        if tag == 2 {
            assert_eq!(len, public.len());
            private[at..at + len].copy_from_slice(public);
            found = true;
        }
        at += len;
    }
    assert!(found);
    let crc = crc32(&private[snapshot::HEADER..]);
    private[40..44].copy_from_slice(&crc.to_le_bytes());
    let crc = crc32(&private[..44]);
    private[44..48].copy_from_slice(&crc.to_le_bytes());
}
#[test]
fn v11_four_backgrounds_roundtrip_without_consuming_more_packet_space() {
    assert_eq!((VERSION, SIZE), (14, 2636));
    assert_eq!(Save::default().background, 0);
    for background in 0..4 {
        let save = Save {
            background,
            sound: true,
            flipped: true,
            pause_on_mistake: true,
            ..Save::default()
        };
        let packet = save.encode().unwrap();
        assert_eq!(packet[10], 7 | (background << 3));
        assert_eq!(Save::decode(&packet), Some(save));
    }
    assert!(
        Save {
            background: 4,
            ..Save::default()
        }
        .encode()
        .is_none()
    );
    for bit in [0x20, 0x40, 0x80] {
        let mut packet = Save::default().encode().unwrap();
        packet[4..6].copy_from_slice(&11u16.to_le_bytes());
        packet[10] |= bit;
        resign(&mut packet);
        assert!(Save::decode(&packet).is_none());
    }
}
#[test]
fn v10_migration_preserves_all_scores_rules_audio_and_preferences_exactly() {
    let mut save = Save::decode(include_bytes!("data/save-v8.bin")).unwrap();
    save.resume_enabled = true;
    save.audio_volume = 61;
    save.audio_mask = 5;
    save.fast_mode = false;
    save.background = 0;
    save.brightness = 147;
    save.flipped = true;
    let old = v10(save.encode().unwrap());
    let migrated = Save::decode(&old).unwrap();
    assert_eq!(migrated, save);
    assert!(migrated.drill_stats.first.all.decisions > (1u64 << 53));
    let current = migrated.encode().unwrap();
    assert_eq!(&current[8..SIZE - 4], &old[8..SIZE - 4]);
    assert_eq!(current[4], 14);
    assert_eq!(old[4], 10);
    let mut app = Box::new(App::new(7));
    app.round_display = true;
    assert!(app.load(migrated));
    assert!(app.round_display);
    assert_eq!(app.background, 0);
    assert_eq!(app.training.stats(), save.stats);
    assert_eq!(app.count_practice.stats(), save.count_stats);
    assert_eq!(app.deviation_practice.stats(), save.drill_stats);
    assert_eq!(app.wager_practice.stats(), save.wager_stats);
}
#[test]
fn every_old_version_keeps_background_bits_reserved() {
    for version in 1..=8 {
        let old = std::fs::read(format!(
            "{}/tests/data/save-v{version}.bin",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        assert_eq!(Save::decode(&old).unwrap().background, 0);
        for bit in [8, 16] {
            let mut changed = old.clone();
            changed[10] |= bit;
            resign(&mut changed);
            assert!(Save::decode(&changed).is_none());
        }
    }
    for version in [9u16, 10] {
        let mut old = Save::default().encode().unwrap();
        old[4..6].copy_from_slice(&version.to_le_bytes());
        if version == 9 {
            old[202..204].fill(0);
        }
        resign(&mut old);
        assert_eq!(Save::decode(&old).unwrap().background, 0);
        for bit in [8, 16] {
            let mut changed = old;
            changed[10] |= bit;
            resign(&mut changed);
            assert!(Save::decode(&changed).is_none());
        }
    }
}
#[test]
fn both_devices_preview_all_four_independent_backgrounds_in_colors() {
    for round in [false, true] {
        let mut app = Box::new(App::new(7));
        app.round_display = round;
        display(&mut app);
        assert_eq!(app.menu_item(), "Brightness");
        app.event(GREY);
        assert_eq!(app.menu_item(), "Flip screen");
        app.event(GREY);
        assert_eq!(app.menu_item(), "Colors");
        app.event(BLUE);
        assert_eq!(app.menu_item(), "Background");
        let before = (app.game.position, app.game.rounds, app.training.stats());
        let cycle = if round { [1, 2, 3, 0] } else { [2, 3, 0, 1] };
        for expected in cycle {
            app.event(BLUE);
            let selected = if round {
                app.background
            } else {
                app.compact_background
            };
            assert_eq!(selected, expected);
            assert!(app.dirty);
        }
        assert_eq!(
            before,
            (app.game.position, app.game.rounds, app.training.stats())
        );
        app.event(GREY);
        assert_eq!(app.menu_item(), "Card back");
        app.event(GREY);
        assert_eq!(app.menu_item(), "Back");
        app.event(BLUE);
        assert_eq!((app.view_name(), app.menu_item()), ("Display", "Colors"));
        let save = app.save();
        app.round_display = !round;
        assert_eq!(
            app.save(),
            save,
            "hardware capability changes no saved option"
        );
    }
}
#[test]
fn background_private_copy_restore_and_round_return_cursor_remain_valid() {
    for background in 0..4 {
        let mut app = Box::new(App::new(7));
        app.round_display = true;
        app.background = background;
        display(&mut app);
        app.event(GREY);
        app.event(GREY);
        app.event(GREY);
        assert_eq!(app.focus(), 3);
        app.render();
        let mut frozen = Box::new(App::EMPTY);
        app.capture_clone_into(&mut frozen).unwrap();
        assert!(frozen.round_display);
        assert_eq!(frozen.background, background);
        let mut private = vec![0; snapshot::MAX_BYTES];
        let n = snapshot::capture_private(&app, &mut private, 41, true).unwrap();
        let mut candidate = Box::new(App::EMPTY);
        snapshot::restore_candidate(&mut candidate, &private[..n], 1000, true).unwrap();
        assert_eq!(candidate.background, background);
        assert_eq!(candidate.focus(), 3);
        assert!(
            !candidate.round_display,
            "runtime capability is not imported from a private packet"
        );
        candidate.round_display = true;
        assert_eq!(candidate.menu_item(), "Back");
        assert_eq!(candidate.save(), app.save());
    }
}
#[test]
fn raw_v10_private_binding_restores_and_requires_a_new_v11_commit() {
    let mut app = Box::new(App::new(7));
    app.audio_volume = 61;
    app.audio_mask = 5;
    app.fast_mode = false;
    app.render();
    let old_public = v10(app.save().encode().unwrap());
    let mut private = vec![0; snapshot::MAX_BYTES];
    let n = snapshot::capture_private(&app, &mut private, 41, true).unwrap();
    private.truncate(n);
    substitute_public(&mut private, &old_public);
    let old = PublicCommit {
        generation: 6,
        checkpoint_epoch: 41,
        transaction_session: [7; 16],
        packet: &old_public,
    };
    let mut bound = vec![0; snapshot::MAX_BYTES];
    let n = resume_binding::bind_private(&mut bound, old, &private).unwrap();
    let mut candidate = Box::new(App::EMPTY);
    resume_binding::restore_bound(&mut candidate, &bound[..n], old, 1000, true).unwrap();
    assert_eq!(candidate.background, 0);
    assert_eq!(
        (
            candidate.audio_volume,
            candidate.audio_mask,
            candidate.fast_mode
        ),
        (61, 5, false)
    );
    assert_eq!(candidate.save(), app.save());
    let current = candidate.save().encode().unwrap();
    assert_ne!(current, old_public);
    assert_eq!(current[4], 14);
    let mut fresh = vec![0; snapshot::MAX_BYTES];
    let n = snapshot::capture_private(&candidate, &mut fresh, 42, true).unwrap();
    assert!(resume_binding::bind_private(&mut bound, old, &fresh[..n]).is_err());
    let new = PublicCommit {
        generation: 7,
        checkpoint_epoch: 42,
        transaction_session: [7; 16],
        packet: &current,
    };
    assert!(resume_binding::bind_private(&mut bound, new, &fresh[..n]).is_ok());
}
