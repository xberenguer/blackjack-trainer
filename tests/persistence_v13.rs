#[path = "support/navigation.rs"]
mod nav;
use stardust_blackjack::{
    app::App,
    controls::{Event, Key},
    persistence::{SIZE, Save, VERSION, crc32},
    resume_binding::{self, PublicCommit},
    resume_private as snapshot,
};
const BLUE: Event = Event::Tap(Key::Blue);
const GREY: Event = Event::Tap(Key::Grey);
fn resign(bytes: &mut [u8]) {
    let end = bytes.len() - 4;
    let crc = crc32(&bytes[..end]);
    bytes[end..].copy_from_slice(&crc.to_le_bytes());
}
fn v12(mut bytes: [u8; SIZE]) -> [u8; SIZE] {
    bytes[4..6].copy_from_slice(&12u16.to_le_bytes());
    bytes[10] &= 0x7f;
    resign(&mut bytes);
    bytes
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
fn v14_independent_backgrounds_and_four_card_backs_roundtrip() {
    assert_eq!((VERSION, SIZE), (14, 2636));
    assert_eq!(Save::default().compact_background, 1);
    for compact_background in 0..4 {
        for background in 0..4 {
            for card_back in 0..4 {
                for flags in 0..8 {
                    let save = Save {
                        compact_background,
                        background,
                        card_back,
                        sound: flags & 1 != 0,
                        flipped: flags & 2 != 0,
                        pause_on_mistake: flags & 4 != 0,
                        ..Save::default()
                    };
                    let packet = save.encode().unwrap();
                    assert_eq!(packet[10], flags | background << 3 | card_back << 5);
                    assert_eq!(packet[203] >> 4, compact_background ^ 1);
                    assert_eq!(Save::decode(&packet), Some(save));
                }
            }
        }
    }
    let mut invalid = Save::default().encode().unwrap();
    invalid[203] |= 0x40;
    resign(&mut invalid);
    assert!(Save::decode(&invalid).is_none());

    let original = Save::decode(include_bytes!("data/save-v8.bin")).unwrap();
    for compact_background in [0, 1] {
        let mut old = original.encode().unwrap();
        old[4..6].copy_from_slice(&13u16.to_le_bytes());
        old[10] |= u8::from(compact_background == 0) << 7;
        resign(&mut old);
        assert_eq!(
            Save::decode(&old),
            Some(Save {
                compact_background,
                ..original
            })
        );
        old[10] |= 0x60;
        resign(&mut old);
        assert!(Save::decode(&old).is_none(), "V13 reserved card-back ID");
    }
}
#[test]
fn v12_migrates_every_score_and_preference_and_keeps_bit7_reserved() {
    let mut save = Save::decode(include_bytes!("data/save-v8.bin")).unwrap();
    save.background = 3;
    save.card_back = 2;
    save.audio_volume = 63;
    save.audio_mask = 5;
    save.fast_mode = false;
    save.resume_enabled = true;
    save.flipped = true;
    assert!(save.drill_stats.first.all.decisions > (1u64 << 53));
    let old = v12(save.encode().unwrap());
    assert_eq!(Save::decode(&old), Some(save));
    let mut bad = old;
    bad[10] |= 0x80;
    resign(&mut bad);
    assert!(Save::decode(&bad).is_none());
    let canonical = Save::decode(&old).unwrap().encode().unwrap();
    assert_eq!(&canonical[8..SIZE - 4], &old[8..SIZE - 4]);
    let mut app = Box::new(App::new(7));
    assert!(app.load(Save::decode(&old).unwrap()));
    assert_eq!(app.training.mode, stardust_blackjack::training::Mode::Off);
    // Existing boot policy starts regular play; stored scores and all preferences remain exact.
    save.mode = stardust_blackjack::training::Mode::Off;
    assert_eq!(app.save(), save);
    assert_eq!(app.compact_background, 1);
    app.compact_background = 0;
    let mut changed = save;
    changed.compact_background = 0;
    assert_eq!(app.save(), changed);
}
#[test]
fn both_display_menus_have_live_preview_and_preserve_historical_back_ids() {
    for round in [false, true] {
        let mut app = Box::new(App::new(7));
        app.round_display = round;
        nav::path(&mut app, &["Settings", "Display"]);
        for (id, label) in [
            (0, "Brightness"),
            (1, "Flip screen"),
            (4, "Colors"),
            (if round { 3 } else { 2 }, "Back"),
            (0, "Brightness"),
        ] {
            assert_eq!((app.focus(), app.menu_item()), (id, label));
            app.event(GREY);
        }
        nav::choose(&mut app, "Colors");
        assert_eq!(app.menu_item(), "Background");
        app.event(BLUE);
        assert_eq!(app.background, if round { 1 } else { 0 });
        assert_eq!(app.compact_background, if round { 1 } else { 2 });
        app.event(GREY);
        assert_eq!(app.menu_item(), "Card back");
        let before = (
            app.game.rng,
            app.game.position,
            app.game.bankroll,
            app.training.stats(),
        );
        assert_eq!((app.view_name(), app.focus()), ("CardBackPreview", 1));
        for colour in [1, 2, 3, 0] {
            app.dirty = false;
            app.event(BLUE);
            assert_eq!(app.card_back, colour);
            assert_eq!(app.save().card_back, colour);
            assert!(app.dirty);
        }
        app.event(GREY);
        assert_eq!(app.menu_item(), "Back");
        app.event(BLUE);
        assert_eq!(
            (app.view_name(), app.menu_item(), app.focus()),
            ("Display", "Colors", 4)
        );
        app.event(BLUE);
        app.event(GREY);
        app.event(Event::Hold(Key::Grey));
        assert_eq!(
            (app.view_name(), app.menu_item(), app.focus()),
            ("Display", "Colors", 4)
        );
        assert_eq!(
            before,
            (
                app.game.rng,
                app.game.position,
                app.game.bankroll,
                app.training.stats()
            )
        );
    }
}
#[test]
fn preview_and_compact_choice_survive_private_copy_restore_and_done_routing() {
    for round in [false, true] {
        let mut app = Box::new(App::new(7));
        app.round_display = round;
        app.compact_background = 0;
        app.background = 2;
        nav::path(&mut app, &["Settings", "Display", "Colors"]);
        app.event(GREY);
        app.event(BLUE);
        app.render();
        let mut copy = Box::new(App::EMPTY);
        app.capture_clone_into(&mut copy).unwrap();
        assert_eq!(copy.compact_background, 0);
        assert_eq!(copy.round_display, round);
        let mut private = vec![0; snapshot::MAX_BYTES];
        let n = snapshot::capture_private(&app, &mut private, 41, true).unwrap();
        let mut candidate = Box::new(App::EMPTY);
        snapshot::restore_candidate(&mut candidate, &private[..n], 1000, true).unwrap();
        assert_eq!(candidate.view_name(), "CardBackPreview");
        assert!(!candidate.round_display);
        candidate.round_display = round;
        assert_eq!(candidate.compact_background, 0);
        assert_eq!(candidate.card_back, 1);
        assert_eq!(candidate.menu_item(), "Card back");
        assert_eq!(candidate.background, 2);
        assert_eq!(candidate.save(), app.save());
        candidate.event(GREY);
        candidate.event(BLUE);
        assert_eq!(
            (
                candidate.view_name(),
                candidate.focus(),
                candidate.menu_item()
            ),
            ("Display", 4, "Colors")
        );
    }
}
#[test]
fn raw_v12_private_compact_back2_and_round_back3_keep_old_meaning_and_binding() {
    for round in [false, true] {
        let mut app = Box::new(App::new(7));
        app.round_display = round;
        app.background = 2;
        app.card_back = 1;
        nav::path(&mut app, &["Settings", "Display"]);
        nav::focus(&mut app, "Back");
        assert_eq!(app.focus(), if round { 3 } else { 2 });
        app.render();
        let old_public = v12(app.save().encode().unwrap());
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
        candidate.round_display = round;
        assert_eq!(candidate.compact_background, 1);
        assert_eq!(
            (candidate.focus(), candidate.menu_item()),
            (if round { 3 } else { 2 }, "Back")
        );
        assert_eq!(candidate.save(), app.save());
        let mut fresh = vec![0; snapshot::MAX_BYTES];
        let n = snapshot::capture_private(&candidate, &mut fresh, 41, true).unwrap();
        assert!(resume_binding::bind_private(&mut bound, old, &fresh[..n]).is_err());
        candidate.event(BLUE);
        assert_eq!(candidate.view_name(), "Settings");
    }
}
