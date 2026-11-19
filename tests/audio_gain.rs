use stardust_blackjack::assets::Clip;
use stardust_blackjack::audio_gain::{
    DEFAULT_CHANNEL_VOLUME, DEFAULT_MASTER_VOLUME, LEGACY_RATE_X256, scaled_sample,
    scaled_sample_at_volume, scaled_sample_with_channel,
};
#[path = "data/audio_gain_golden.rs"]
mod golden;

const EDGES: [i16; 35] = [
    i16::MIN,
    -32767,
    -30000,
    -20000,
    -10000,
    -1000,
    -512,
    -256,
    -128,
    -64,
    -37,
    -36,
    -35,
    -2,
    -1,
    0,
    1,
    2,
    35,
    36,
    37,
    64,
    128,
    256,
    512,
    1000,
    10000,
    20000,
    30000,
    32766,
    i16::MAX,
    -257,
    -255,
    255,
    257,
];
const BASIS: u64 = 14695981039346656037;
fn hash_sample(mut hash: u64, sample: i16) -> u64 {
    for byte in sample.to_le_bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(1099511628211);
    }
    hash
}

#[test]
fn every_signed_sample_at_every_master_matches_original_cpp() {
    for master in 0..=u8::MAX {
        let mut hash = BASIS;
        for input in i16::MIN..=i16::MAX {
            hash = hash_sample(hash, scaled_sample_at_volume(input, master));
        }
        assert_eq!(
            hash,
            golden::MASTER_HASHES[usize::from(master)],
            "master={master}"
        );
    }
}

#[test]
fn every_signed_sample_at_every_channel_matches_original_cpp() {
    for channel in 0..=u8::MAX {
        let mut hash = BASIS;
        for input in i16::MIN..=i16::MAX {
            hash = hash_sample(hash, scaled_sample_with_channel(input, 120, channel));
        }
        assert_eq!(
            hash,
            golden::CHANNEL_HASHES[usize::from(channel)],
            "channel={channel}"
        );
    }
}

#[test]
fn all_master_channel_combinations_at_boundaries_match_original_cpp() {
    let mut hash = BASIS;
    for master in 0..=u8::MAX {
        for channel in 0..=u8::MAX {
            for input in EDGES {
                hash = hash_sample(hash, scaled_sample_with_channel(input, master, channel));
            }
        }
    }
    assert_eq!(hash, golden::ALL_VOLUME_BOUNDARIES_HASH);
}

#[test]
fn all_fourteen_raw_clips_unchanged_and_post_gain_matches_original_source() {
    let mut total = 0;
    for (cue, &(rate, samples, raw_hash, scaled_hash)) in golden::CLIPS.iter().enumerate() {
        let clip = Clip::at(cue).unwrap();
        assert_eq!((clip.rate, clip.samples), (rate, samples), "cue={cue}");
        let (mut raw, mut scaled) = (BASIS, BASIS);
        for index in 0..samples {
            let input = clip.sample(index).unwrap();
            raw = hash_sample(raw, input);
            scaled = hash_sample(scaled, scaled_sample(input));
        }
        assert_eq!(raw, raw_hash, "raw PCM cue={cue}");
        assert_eq!(scaled, scaled_hash, "scaled PCM cue={cue}");
        assert_eq!(clip.sample(samples), None);
        total += samples;
    }
    assert_eq!(total, 422516);
    assert!(Clip::at(14).is_none());
}

#[test]
fn original_default_constants_and_negative_q8_rounding_are_explicit() {
    assert_eq!((DEFAULT_MASTER_VOLUME, DEFAULT_CHANNEL_VOLUME), (120, 255));
    assert_eq!(LEGACY_RATE_X256, 5_644_768);
    for (sample, expected) in EDGES.into_iter().zip(golden::BOUNDARIES) {
        assert_eq!(scaled_sample(sample), expected, "sample={sample}");
    }
    assert_eq!(scaled_sample(-1), -1);
    assert_eq!(scaled_sample(1), 0);
    assert_eq!(scaled_sample(-32768), -893);
    assert_eq!(scaled_sample(32767), 892);
    // The earlier linear approximation is far louder than the original mixer.
    assert_eq!(i32::from(32767_i16) * 120 / 255, 15419);
    assert!(scaled_sample(32767) < 1000);
}

#[test]
fn whole_volume_domain_is_bounded_monotonic_and_muted_at_zero() {
    for master in 0..=u8::MAX {
        let mut previous = i16::MIN;
        for input in i16::MIN..=i16::MAX {
            let output = scaled_sample_at_volume(input, master);
            assert!(
                output >= previous,
                "input monotonic master={master} input={input}"
            );
            assert!(i32::from(output).abs() <= i32::from(input).abs());
            previous = output;
            if master == 0 {
                assert_eq!(output, 0);
            }
        }
        for input in EDGES {
            assert_eq!(scaled_sample_with_channel(input, master, 0), 0);
        }
    }
    // Stereo transport must duplicate the already scaled sample, with no second
    // multiplication or L+R sum. This exact integer ratio is browser-safe.
    for input in EDGES {
        let output = scaled_sample(input);
        let web_audio_f32 = f32::from(output) / 32768.0;
        assert_eq!(web_audio_f32 * 32768.0, f32::from(output));
    }
}
