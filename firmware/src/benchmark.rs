//! Optional USB `p` validation; all App state belongs to fixed public fixtures in spare PSRAM.
//! Never reads, advances, saves or renders the live user's game. Reports aggregates only.
use crate::speaker::Speaker;
use stardust_blackjack::{app::App, audio::Cue, training::Mode};
/// SAFETY: exclusive spare region is initialized/reset between cases; active/frozen Apps are distinct.
pub unsafe fn run(ptr: *mut App, template: *const App, speaker: &mut Speaker) {
    let before = speaker.underruns;
    let mut maximum = 0u64;
    for mode in [Mode::Guide, Mode::Test, Mode::Coach] {
        for case in 0..6u8 {
            // SAFETY: exclusive inactive region and valid immutable no-Drop App template; no large stack copy.
            unsafe {
                core::ptr::copy_nonoverlapping(template, ptr, 1);
            }
            let app = unsafe { &mut *ptr };
            assert!(app.prepare_fixed_benchmark(case, mode));
            speaker.cancel();
            speaker.cue(Cue::Shuffle);
            speaker.pump(true);
            esp_hal::delay::Delay::new().delay_millis(30);
            speaker.pump(true);
            let underruns = speaker.underruns;
            let started = super::us();
            app.fixed_benchmark_event();
            let elapsed = super::us() - started;
            maximum = maximum.max(elapsed);
            speaker.pump(true);
            esp_println::println!(
                "STARDUST BENCH case={} mode={:?} event_us={} audio_underruns={} graded={}",
                case,
                mode,
                elapsed,
                speaker.underruns - underruns,
                app.training.stats().all.decisions
            );
            speaker.cancel();
        }
    }
    esp_println::println!(
        "STARDUST BENCH COMPLETE cases=18 max_event_us={} audio_underruns={} live_state_unchanged=true fixed_cases=true",
        maximum,
        speaker.underruns - before
    );
}
