use stardust_blackjack::{
    app::App,
    persistence::{SIZE, Save, VERSION, crc32},
    resume_binding::{self, PublicCommit},
    resume_private as snapshot,
};
fn resign(b: &mut [u8]) {
    let n = b.len() - 4;
    let c = crc32(&b[..n]);
    b[n..].copy_from_slice(&c.to_le_bytes())
}
fn v9(mut b: [u8; SIZE]) -> [u8; SIZE] {
    b[4..6].copy_from_slice(&9u16.to_le_bytes());
    b[202..204].fill(0);
    resign(&mut b);
    b
}
#[test]
fn v10_volume_mask_speed_all_boundaries_and_legacy_defaults() {
    assert_eq!((VERSION, SIZE), (12, 2636));
    let defaults = Save::default();
    assert_eq!(defaults.policy, stardust_blackjack::training::Policy::HiLo);
    assert_eq!(
        (
            defaults.audio_volume,
            defaults.audio_mask,
            defaults.fast_mode
        ),
        (20, 7, true)
    );
    for volume in [0, 1, 9, 10, 11, 50, 99, 100] {
        for mask in 0..=7 {
            for fast in [false, true] {
                let s = Save {
                    audio_volume: volume,
                    audio_mask: mask,
                    fast_mode: fast,
                    ..defaults
                };
                let b = s.encode().unwrap();
                assert_eq!(b[202], volume);
                assert_eq!(b[203], mask | if fast { 8 } else { 0 });
                assert_eq!(Save::decode(&b), Some(s));
            }
        }
    }
    for (at, value) in [(202, 101), (202, 255), (203, 16), (203, 128)] {
        let mut b = defaults.encode().unwrap();
        b[at] = value;
        resign(&mut b);
        assert!(Save::decode(&b).is_none());
    }
    for version in 1..=8 {
        let b = std::fs::read(format!(
            "{}/tests/data/save-v{version}.bin",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let s = Save::decode(&b).unwrap();
        assert_eq!((s.audio_volume, s.audio_mask, s.fast_mode), (20, 7, true));
    }
    let old = v9(defaults.encode().unwrap());
    let s = Save::decode(&old).unwrap();
    assert_eq!((s.audio_volume, s.audio_mask, s.fast_mode), (20, 7, true));
    let mut bad = old;
    bad[202] = 50;
    resign(&mut bad);
    assert!(Save::decode(&bad).is_none());
}
#[test]
fn new_preferences_copy_and_private_restore_from_public_tlv_without_private_body_changes() {
    let mut a = Box::new(App::new(7));
    a.audio_volume = 9;
    a.audio_mask = 2;
    a.fast_mode = true;
    a.render();
    let mut frozen = Box::new(App::EMPTY);
    a.capture_clone_into(&mut frozen).unwrap();
    assert_eq!(
        (frozen.audio_volume, frozen.audio_mask, frozen.fast_mode),
        (9, 2, true)
    );
    let mut bytes = vec![0; snapshot::MAX_BYTES];
    let n = snapshot::capture_private(&a, &mut bytes, 41, true).unwrap();
    let mut restored = Box::new(App::EMPTY);
    snapshot::restore_candidate(&mut restored, &bytes[..n], 1000, true).unwrap();
    assert_eq!(
        (
            restored.audio_volume,
            restored.audio_mask,
            restored.fast_mode
        ),
        (9, 2, true)
    );
    assert_eq!(restored.save(), a.save());
}
#[test]
fn raw_v9_public_and_old_private_public_tlv_restore_then_require_new_v10_binding() {
    let a = Box::new(App::new(7));
    let old_public = v9(a.save().encode().unwrap());
    let mut private = vec![0; snapshot::MAX_BYTES];
    let n = snapshot::capture_private(&a, &mut private, 41, true).unwrap();
    private.truncate(n);
    // Private field order remains unchanged: substitute the independently specified
    // old V9 public packet in its existing mandatory TLV, resigning only envelopes.
    let mut at = snapshot::HEADER;
    let mut found = false;
    while at < private.len() {
        let tag = u16::from_le_bytes(private[at..at + 2].try_into().unwrap());
        let len = u32::from_le_bytes(private[at + 4..at + 8].try_into().unwrap()) as usize;
        at += 8;
        if tag == 2 {
            assert_eq!(len, SIZE);
            private[at..at + len].copy_from_slice(&old_public);
            found = true;
        }
        at += len;
    }
    assert!(found);
    let c = crc32(&private[snapshot::HEADER..]);
    private[40..44].copy_from_slice(&c.to_le_bytes());
    let c = crc32(&private[..44]);
    private[44..48].copy_from_slice(&c.to_le_bytes());
    let public = PublicCommit {
        generation: 6,
        checkpoint_epoch: 41,
        transaction_session: [7; 16],
        packet: &old_public,
    };
    let mut bundle = vec![0; snapshot::MAX_BYTES];
    let n = resume_binding::bind_private(&mut bundle, public, &private).unwrap();
    let mut candidate = Box::new(App::EMPTY);
    resume_binding::restore_bound(&mut candidate, &bundle[..n], public, 1000, true).unwrap();
    assert_eq!(
        (
            candidate.audio_volume,
            candidate.audio_mask,
            candidate.fast_mode
        ),
        (20, 7, true)
    );
    let canonical = candidate.save().encode().unwrap();
    assert_eq!(u16::from_le_bytes(canonical[4..6].try_into().unwrap()), 12);
    assert_ne!(canonical, old_public);
    let mut new_private = vec![0; snapshot::MAX_BYTES];
    let n = snapshot::capture_private(&candidate, &mut new_private, 42, true).unwrap();
    assert!(resume_binding::bind_private(&mut bundle, public, &new_private[..n]).is_err());
    let new_public = PublicCommit {
        generation: 7,
        checkpoint_epoch: 42,
        transaction_session: [7; 16],
        packet: &canonical,
    };
    assert!(resume_binding::bind_private(&mut bundle, new_public, &new_private[..n]).is_ok());
}
