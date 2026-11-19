//! StickS3 signed-16-bit mono gain, copied equally to both I2S output channels.
//!
//! Reproduces the scalar gain and Q8 quantization of the approved M5Unified
//! Speaker_Class path: board magnification 1, stereo output, master volume 120,
//! default channel volume 255, no DAC or buzzer. PCM assets remain unmodified.
//!
//! This is gain, not a resampler. The former mixer's interpolation, prior-sample
//! state, and clock drift are deliberately outside this stateless operation.

/// Original application's `M5.Speaker.setVolume(120)` setting.
pub const DEFAULT_MASTER_VOLUME: u8 = 120;
/// M5Unified's unchanged `channel_info_t` default.
pub const DEFAULT_CHANNEL_VOLUME: u8 = 255;
/// Original ESP32-S3 raw-clock divider result (22049.875 Hz, in Q8).
/// The new output transport may run nominal 22050 Hz; this constant only preserves
/// the original scalar coefficient's floating-point rounding.
pub const LEGACY_RATE_X256: u32 = 5_644_768;

const RATE: f32 = LEGACY_RATE_X256 as f32;
// Source evaluates these divisions in this order; do not replace with a rounded
// decimal gain or a single rational multiplication.
const MAGNIFICATION: f32 = (2.0 / RATE) / ((1_u32 << 28) as f32);

/// Apply the original app's fixed master/default-channel gain to one raw sample.
/// Duplicate the result for left and right; do not sum the mono input twice.
#[inline]
pub fn scaled_sample(sample: i16) -> i16 {
    scaled_sample_with_channel(sample, DEFAULT_MASTER_VOLUME, DEFAULT_CHANNEL_VOLUME)
}

/// Same scalar path for another master volume; no volume UI is implied.
#[inline]
pub fn scaled_sample_at_volume(sample: i16, master: u8) -> i16 {
    scaled_sample_with_channel(sample, master, DEFAULT_CHANNEL_VOLUME)
}

/// Complete scalar path for the original 8-bit master/channel volume domains.
///
/// Each f32 operation follows the old mixer. Conversion to Q8 truncates toward
/// zero, then arithmetic shift rounds negative nonmultiples of 256 downward.
/// That distinction matters near zero: default gain maps +1 to 0 and -1 to -1.
/// The original final signed-16-bit clamp is retained, although this board's
/// single-channel gain cannot clip any valid input at any master/channel volume.
#[inline]
pub fn scaled_sample_with_channel(sample: i16, master: u8, channel: u8) -> i16 {
    let master_squared = u32::from(master) * u32::from(master);
    let volume = MAGNIFICATION * master_squared as f32;
    let channel_squared = u32::from(channel) * u32::from(channel);
    let channel_gain = volume * channel_squared as f32;
    let base = f32::from(sample) * channel_gain;
    let q8 = (base * RATE) as i32;
    (q8 >> 8).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

/// User-facing percentage, mapped to the existing 8-bit squared-volume path.
/// This leaves the legacy reference helpers and raw PCM unchanged.
#[inline]
pub fn playback_sample(sample: i16, percent: u8) -> i16 {
    let master = (u16::from(percent.min(100)) * 255 + 50) / 100;
    scaled_sample_at_volume(sample, master as u8)
}
