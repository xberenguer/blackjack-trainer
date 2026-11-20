use stardust_blackjack::{
    persistence::{SIZE, Save, crc32},
    rules::Profile,
    training::{Mode, Policy},
};
const LEGACY: [&[u8]; 5] = [
    include_bytes!("data/save-v1.bin"),
    include_bytes!("data/save-v2.bin"),
    include_bytes!("data/save-v3.bin"),
    include_bytes!("data/save-v4.bin"),
    include_bytes!("data/save-v5.bin"),
];
fn resign(b: &mut [u8]) {
    let end = b.len() - 4;
    let crc = crc32(&b[..end]);
    b[end..].copy_from_slice(&crc.to_le_bytes());
}
#[test]
fn all_previous_schemas_preserve_exact_progress_and_preferences() {
    assert_eq!(crc32(b"123456789"), 0xcbf43926);
    for (version, bytes) in LEGACY.iter().enumerate() {
        let d = Save::decode(bytes).unwrap();
        if version < 2 {
            assert_eq!(d.stats.all.decisions, 0x0102030405060708);
            assert_eq!(d.minimum_bet, 500);
            assert_eq!(d.maximum_bet, 50000);
            assert_eq!(d.count_display, 0);
            assert_eq!(d.guide_count_display, 2);
        }
        if version == 2 {
            assert_eq!(d.profile, Profile::EightH17);
            assert_eq!(d.penetration, 60);
            assert_eq!(d.guide_count_display, 2);
        }
        if version == 3 {
            assert_eq!(
                (
                    d.stats.all.decisions,
                    d.stats.all.correct,
                    d.stats.all.errors,
                    d.stats.all.assisted
                ),
                (107, 26, 8, 73)
            );
            assert_eq!(d.policy, Policy::Basic);
        }
        let encoded = d.encode().unwrap();
        assert_eq!(Save::decode(&encoded), Some(d));
        if version == 4 {
            assert_eq!(&encoded[8..202], &bytes[8..202]);
            assert_eq!(&encoded[204..332], &bytes[204..332]);
            assert!(!d.wager_enabled);
            assert_eq!(d.wager_policy.units, [1, 2, 4, 8, 12]);
            assert_eq!(
                d.wager_stats,
                stardust_blackjack::wager_practice::Stats::default()
            );
            assert_eq!(d.policy, Policy::HiLo);
            assert_eq!(d.mode, Mode::Guide);
            assert_eq!(d.profile, Profile::EightH17);
            assert_eq!(d.count_display, 1);
            assert_eq!(d.guide_count_display, 0);
            assert_eq!(d.count_stats.kinds[2].correct, (1u64 << 54) + 2);
            assert_eq!(d.count_stats.all.decisions, 3 * (1u64 << 54) + 33);
        }
    }
}
#[test]
fn corruption_malformed_sizes_reserved_bytes_and_invalid_fields_are_rejected() {
    for bytes in LEGACY {
        for at in 0..bytes.len() {
            let mut bad = bytes.to_vec();
            bad[at] ^= 0x80;
            assert!(Save::decode(&bad).is_none(), "uncaught byte {at}")
        }
        for size in 0..bytes.len() {
            assert!(Save::decode(&bytes[..size]).is_none());
        }
    }
    let valid = Save::default().encode().unwrap();
    for (at, value) in [
        (4, 6),
        (8, 4),
        (9, 19),
        (10, 96),
        (11, 6),
        (12, 99),
        (13, 3),
        (14, 3),
        (15, 2),
        (200, 7),
        (201, 1),
        (202, 101),
        (203, 16),
        (204, 1),
        (332, 2),
        (333, 1),
        (334, 1),
        (335, 1),
        (336, 0),
        (356, 1),
    ] {
        let mut bad = valid;
        bad[at] = value;
        resign(&mut bad);
        assert!(Save::decode(&bad).is_none(), "invalid field {at}");
    }
    let mut expanded = valid.to_vec();
    expanded.push(0);
    assert!(Save::decode(&expanded).is_none());
    let mut invalid = Save::default();
    invalid.count_stats.all.decisions = 1;
    assert!(invalid.encode().is_none());
    invalid = Save::default();
    invalid.stats.all.assisted = 1;
    assert!(invalid.encode().is_none());
    assert_eq!(valid.len(), SIZE);
}
