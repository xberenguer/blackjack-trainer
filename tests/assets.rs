use stardust_blackjack::{
    assets::{Clip, Font, Sprite, validate_banks},
    audio::Cue,
};
include!("data/audio_reference.rs");
#[test]
fn all_pcm_cues_match_the_accepted_source_sample_for_sample() {
    assert!(validate_banks());
    for (index, cue) in Cue::ALL.into_iter().enumerate() {
        let c = cue.clip().unwrap();
        let hash = c.bytes().iter().fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ *b as u64).wrapping_mul(0x100000001b3)
        });
        assert_eq!((c.rate, c.samples, hash), AUDIO_REFERENCE[index]);
        assert!(c.sample(c.samples).is_none());
        assert!(
            c.bytes()
                .chunks_exact(2)
                .enumerate()
                .all(|(i, b)| c.sample(i) == Some(i16::from_le_bytes([b[0], b[1]])))
        );
    }
    assert!(Cue::MenuMove.clip().unwrap().bytes() == Cue::Push.clip().unwrap().bytes());
}
#[test]
fn asset_indices_and_boundaries_are_checked() {
    assert!(Sprite::at(141).is_none());
    assert!(Font::at(36).is_none());
    assert!(Clip::at(14).is_none());
    for (index, w, h) in [
        (0, 55, 80),
        (51, 55, 80),
        (52, 35, 50),
        (103, 35, 50),
        (104, 55, 80),
        (105, 35, 50),
    ] {
        let s = Sprite::at(index).unwrap();
        assert_eq!((s.width, s.height), (w, h));
        assert!(s.rgba565(w * h - 1).is_some());
        assert!(s.rgba565(w * h).is_none());
    }
}
