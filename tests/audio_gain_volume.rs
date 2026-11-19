use stardust_blackjack::audio_gain::{playback_sample, scaled_sample_at_volume};
#[test]
fn percentage_volume_maps_to_verified_master_domain_and_clamps() {
    for percent in 0..=255u8 {
        let master = (u16::from(percent.min(100)) * 255 + 50) / 100;
        for raw in [i16::MIN, -20000, -1000, -1, 0, 1, 1000, 20000, i16::MAX] {
            assert_eq!(playback_sample(raw, percent), scaled_sample_at_volume(raw, master as u8));
        }
    }
    for raw in i16::MIN..=i16::MAX { assert_eq!(playback_sample(raw, 0), 0); }
}
#[test]
fn increasing_percent_is_monotone_for_both_polarities() {
    let mut positive=0;let mut negative=0;
    for percent in 0..=100 {
        let p=playback_sample(i16::MAX,percent);let n=playback_sample(i16::MIN,percent);
        assert!(p>=positive && n<=negative);positive=p;negative=n;
    }
    assert!(playback_sample(i16::MAX,50)>0);
}
