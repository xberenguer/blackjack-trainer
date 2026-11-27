use stardust_blackjack::{
    app::App,
    deviation_drills::{IndexId, Stats},
    persistence::{SIZE, Save, VERSION, crc32},
    rules::Profile,
    training::Mode,
};
#[test]
fn v1_through_v7_migrate_exact_old_scores_and_create_no_drill_progress() {
    for version in 1..=7 {
        let bytes = std::fs::read(format!(
            "{}/tests/data/save-v{version}.bin",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let save = Save::decode(&bytes).unwrap();
        assert_eq!(save.drill_stats, Stats::default());
        assert_eq!(
            (save.drill_target, save.drill_mix, save.drill_profile),
            (31, 0, Profile::SixH17)
        );
        let encoded = save.encode().unwrap();
        let decoded = Save::decode(&encoded).unwrap();
        assert_eq!(decoded, save);
        if version >= 3 {
            assert_eq!(&encoded[16..200], &bytes[16..200]);
        } else {
            assert_eq!(&encoded[24..200], &bytes[16..192]);
        }
        if version >= 5 {
            assert_eq!(&encoded[204..332], &bytes[204..332]);
        }
        if version >= 6 {
            assert_eq!(&encoded[200..202], &bytes[200..202]);
        }
        if version >= 7 {
            assert_eq!(&encoded[332..580], &bytes[332..580]);
        }
    }
}
#[test]
fn independent_v8_packet_roundtrips_exact_original_and_replay_u64_scores() {
    let packet = include_bytes!("data/save-v8.bin");
    let s = Save::decode(packet).unwrap();
    assert_eq!(VERSION, 14);
    assert_eq!(SIZE, 2636);
    assert_eq!(
        (s.drill_target, s.drill_mix, s.drill_profile),
        (IndexId::Hard16v10 as u8, 2, Profile::SixS17)
    );
    assert_eq!(s.drill_stats.first.all.decisions, (1u64 << 54) + 7);
    assert_eq!(s.drill_stats.first.all.correct, (1u64 << 54) + 5);
    assert_eq!(s.drill_stats.replay.all.decisions, 7);
    let encoded = s.encode().unwrap();
    assert_eq!(&encoded[8..202], &packet[8..202]);
    assert_eq!(&encoded[204..SIZE - 4], &packet[204..SIZE - 4]);
    assert_eq!(Save::decode(&encoded), Some(s));
    let mut a = App::new(1);
    assert!(a.load(s));
    assert_eq!(a.training.mode, Mode::Off);
    assert_eq!(a.deviation_practice.stats(), s.drill_stats);
    assert_eq!(a.deviation_practice.history_count(), 0);
    assert_eq!(a.game.rules.profile, Profile::EightH17);
    assert_eq!(a.drill_profile, Profile::SixS17);
    assert_eq!((a.game.extra_seats, a.game.player_seat), (4, 1));
    assert!(!a.load(s));
}
fn resign(b: &mut [u8]) {
    let at = b.len() - 4;
    let crc = crc32(&b[..at]);
    b[at..].copy_from_slice(&crc.to_le_bytes())
}
#[test]
fn v8_crc_reserved_selectors_schema_and_stat_invariants_are_checked() {
    let packet = *include_bytes!("data/save-v8.bin");
    for at in 0..packet.len() {
        let mut bad = packet;
        bad[at] ^= 0x80;
        assert!(Save::decode(&bad).is_none(), "byte{at}");
    }
    for (at, value) in [
        (4, 14),
        (580, 32),
        (581, 4),
        (582, 6),
        (583, 1),
        (200, 7),
        (201, 7),
        (584, 0),
    ] {
        let mut bad = packet;
        bad[at] = value;
        resign(&mut bad);
        assert!(Save::decode(&bad).is_none(), "field{at}");
    }
    let mut bad = packet.to_vec();
    bad.push(0);
    assert!(Save::decode(&bad).is_none());
    assert!(Save::decode(&packet[..584]).is_none());
}
