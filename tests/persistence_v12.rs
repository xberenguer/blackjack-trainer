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
fn old(mut packet: [u8; SIZE], version: u16) -> [u8; SIZE] {
    packet[4..6].copy_from_slice(&version.to_le_bytes());
    packet[10] &= if version >= 11 { 0x1f } else { 7 };
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
    panic!("settings missing")
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
fn v12_three_card_backs_and_all_background_flags_roundtrip_exactly() {
    assert_eq!((VERSION, SIZE), (12, 2636));
    assert_eq!(Save::default().card_back, 0);
    for background in 0..4 {
        for card_back in 0..3 {
            for flags in 0u8..8 {
                let save = Save {
                    background,
                    card_back,
                    sound: flags & 1 != 0,
                    flipped: flags & 2 != 0,
                    pause_on_mistake: flags & 4 != 0,
                    ..Save::default()
                };
                let packet = save.encode().unwrap();
                assert_eq!(packet[10], flags | background << 3 | card_back << 5);
                assert_eq!(Save::decode(&packet), Some(save));
            }
        }
    }
    assert!(
        Save {
            card_back: 3,
            ..Save::default()
        }
        .encode()
        .is_none()
    );
    for invalid in [0x60, 0x80, 0xff] {
        let mut packet = Save::default().encode().unwrap();
        packet[10] = invalid;
        resign(&mut packet);
        assert!(Save::decode(&packet).is_none());
    }
    let mut packet = Save::default().encode().unwrap();
    packet[4..6].copy_from_slice(&13u16.to_le_bytes());
    resign(&mut packet);
    assert!(Save::decode(&packet).is_none());
}
#[test]
fn v11_backgrounds_and_v10_preferences_migrate_without_losing_a_single_score() {
    let mut save = Save::decode(include_bytes!("data/save-v8.bin")).unwrap();
    save.audio_volume = 63;
    save.audio_mask = 3;
    save.fast_mode = false;
    save.resume_enabled = true;
    save.background = 2;
    save.card_back = 0;
    assert!(save.drill_stats.first.all.decisions > (1u64 << 53));
    let v11 = old(save.encode().unwrap(), 11);
    assert_eq!(Save::decode(&v11), Some(save));
    let current = Save::decode(&v11).unwrap().encode().unwrap();
    assert_eq!(&current[8..SIZE - 4], &v11[8..SIZE - 4]);
    for bit in [0x20, 0x40, 0x80] {
        let mut bad = v11;
        bad[10] |= bit;
        resign(&mut bad);
        assert!(Save::decode(&bad).is_none());
    }
    save.background = 0;
    let v10 = old(save.encode().unwrap(), 10);
    assert_eq!(Save::decode(&v10), Some(save));
    let mut app = Box::new(App::new(7));
    app.round_display = true;
    assert!(app.load(Save::decode(&v10).unwrap()));
    assert_eq!(app.card_back, 0);
    assert_eq!(app.background, 0);
    assert_eq!(app.training.stats(), save.stats);
    assert_eq!(app.count_practice.stats(), save.count_stats);
    assert_eq!(app.wager_practice.stats(), save.wager_stats);
    assert_eq!(app.deviation_practice.stats(), save.drill_stats);
}
#[test]
fn round_display_uses_stable_return_id_and_visual_card_back_order() {
    let mut app = Box::new(App::new(7));
    app.round_display = true;
    display(&mut app);
    for (id, label) in [
        (0, "Brightness"),
        (1, "Flip screen"),
        (2, "Background"),
        (4, "Card back"),
        (3, "Return"),
        (0, "Brightness"),
    ] {
        assert_eq!((app.focus(), app.menu_item()), (id, label));
        app.event(GREY);
    }
    while app.focus() != 4 {
        app.event(GREY);
    }
    let before = (app.game.position, app.game.rounds, app.training.stats());
    for back in [1, 2, 0] {
        app.event(BLUE);
        assert_eq!(app.card_back, back);
        assert_eq!(app.background, 0);
        assert!(app.dirty);
    }
    assert_eq!(
        before,
        (app.game.position, app.game.rounds, app.training.stats())
    );
    app.event(GREY);
    assert_eq!((app.focus(), app.menu_item()), (3, "Return"));
    app.event(BLUE);
    assert_eq!(app.view_name(), "Settings");
    let mut stick = Box::new(App::new(7));
    display(&mut stick);
    stick.event(GREY);
    stick.event(GREY);
    assert_eq!((stick.focus(), stick.menu_item()), (2, "Return"));
    stick.event(BLUE);
    assert_eq!(stick.view_name(), "Settings");
    assert_eq!(stick.card_back, 0);
}
#[test]
fn all_card_back_choices_survive_private_copy_and_round_card_back_cursor_restore() {
    for card_back in 0..3 {
        let mut app = Box::new(App::new(7));
        app.round_display = true;
        app.background = 3;
        app.card_back = card_back;
        display(&mut app);
        while app.focus() != 4 {
            app.event(GREY);
        }
        app.render();
        let mut frozen = Box::new(App::EMPTY);
        app.capture_clone_into(&mut frozen).unwrap();
        assert_eq!(frozen.card_back, card_back);
        assert_eq!(frozen.background, 3);
        let mut private = vec![0; snapshot::MAX_BYTES];
        let n = snapshot::capture_private(&app, &mut private, 41, true).unwrap();
        let mut candidate = Box::new(App::EMPTY);
        snapshot::restore_candidate(&mut candidate, &private[..n], 1000, true).unwrap();
        assert_eq!(
            (candidate.card_back, candidate.background, candidate.focus()),
            (card_back, 3, 4)
        );
        assert!(!candidate.round_display);
        candidate.round_display = true;
        assert_eq!(candidate.menu_item(), "Card back");
        assert_eq!(candidate.save(), app.save());
    }
}
#[test]
fn old_v11_private_return_cursor3_stays_return_and_requires_a_new_v12_binding() {
    let mut app = Box::new(App::new(7));
    app.round_display = true;
    app.background = 2;
    display(&mut app);
    while app.focus() != 3 {
        app.event(GREY);
    }
    assert_eq!(app.menu_item(), "Return");
    app.render();
    let old_public = old(app.save().encode().unwrap(), 11);
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
    assert_eq!(
        (candidate.card_back, candidate.background, candidate.focus()),
        (0, 2, 3)
    );
    assert_eq!(candidate.save(), app.save());
    let canonical = candidate.save().encode().unwrap();
    assert_eq!(canonical[4], 12);
    assert_ne!(canonical, old_public);
    // Keep the old epoch fixed once: rejection must depend on the exact changed
    // public packet, rather than merely on a different checkpoint number.
    let mut same_epoch = vec![0; snapshot::MAX_BYTES];
    let n = snapshot::capture_private(&candidate, &mut same_epoch, 41, true).unwrap();
    assert!(resume_binding::bind_private(&mut bound, old, &same_epoch[..n]).is_err());
    let mut fresh = vec![0; snapshot::MAX_BYTES];
    let n = snapshot::capture_private(&candidate, &mut fresh, 42, true).unwrap();
    assert!(resume_binding::bind_private(&mut bound, old, &fresh[..n]).is_err());
    let current = PublicCommit {
        generation: 7,
        checkpoint_epoch: 42,
        transaction_session: [7; 16],
        packet: &canonical,
    };
    assert!(resume_binding::bind_private(&mut bound, current, &fresh[..n]).is_ok());
    candidate.round_display = true;
    assert_eq!(candidate.menu_item(), "Return");
    candidate.event(BLUE);
    assert_eq!(candidate.view_name(), "Settings");
}
