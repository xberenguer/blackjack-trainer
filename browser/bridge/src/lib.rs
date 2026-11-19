//! The only unsafe Rust is this single-threaded, non-reentrant browser ABI.
//! No imports/callbacks/threads or shared memory exist. Exported pointers address
//! fixed buffers: callers may only write the bounded public-save and internal
//! journal transport buffers. The page exposes neither private payload nor RNG.
//! Every game operation and pixel is produced by the safe Rust App.
#![no_std]
use core::mem::MaybeUninit;
use stardust_blackjack::{
    app::App,
    audio::Cue,
    persistence::{self, Save},
};

mod store;

static mut APP: MaybeUninit<App> = MaybeUninit::uninit();
static mut READY: bool = false;
static mut SAVE: [u8; persistence::SIZE] = [0; persistence::SIZE];

// SAFETY: JS calls are sequential and these exports never call foreign code.
fn app() -> &'static mut App {
    unsafe {
        if !READY {
            core::ptr::addr_of_mut!(APP).write(MaybeUninit::new(App::new(1)));
            READY = true;
        }
        &mut *core::ptr::addr_of_mut!(APP).cast::<App>()
    }
}
// SAFETY: single-threaded ABI, no callbacks; existing task borrows are canceled first.
unsafe fn replace_app(value: App) {
    unsafe {
        core::ptr::addr_of_mut!(APP).write(MaybeUninit::new(value));
        READY = true;
    }
}
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_boot(low: u32, high: u32) {
    unsafe {
        core::ptr::addr_of_mut!(APP)
            .write(MaybeUninit::new(App::new((high as u64) << 32 | low as u64)));
        READY = true;
    }
    store::boot((high as u64) << 32 | low as u64);
    app().render();
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_buttons(mask: u32, now: u32) {
    if store::buttons(mask, now) {
        app().buttons([mask & 1 != 0, mask & 2 != 0], now);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_tick(now: u32) {
    store::tick(now);
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_render() {
    if !store::render() {
        app().render();
    }
    store::rendered_frame();
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_frame() -> *const u16 {
    store::frame()
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_frame_len() -> u32 {
    240 * 135
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_version() -> *const u8 {
    stardust_blackjack::VERSION.as_ptr()
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_version_len() -> u32 {
    stardust_blackjack::VERSION.len() as u32
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_save_size() -> u32 {
    persistence::SIZE as u32
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_save_buffer() -> *mut u8 {
    core::ptr::addr_of_mut!(SAVE).cast::<u8>()
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_save_export() -> u32 {
    let packet = if store::output_pending() {
        store::acknowledged_packet()
    } else {
        app().save().encode()
    };
    let Some(bytes) = packet else {
        return 0;
    };
    unsafe {
        core::ptr::addr_of_mut!(SAVE).write(bytes);
    }
    bytes.len() as u32
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_save_import(len: u32) -> u32 {
    if len as usize > persistence::SIZE {
        return 0;
    }
    let bytes = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(SAVE).cast::<u8>(), len as usize)
    };
    let Some(save) = Save::decode(bytes) else {
        return 0;
    };
    let loaded = app().load(save);
    app().render();
    u32::from(loaded)
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_sound() -> u32 {
    if store::output_pending() {
        0
    } else {
        app().pop_sound().map_or(0, |c| c as u32 + 1)
    }
}
fn cue(id: u32) -> Option<Cue> {
    id.checked_sub(1)
        .and_then(|n| Cue::ALL.get(n as usize).copied())
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_audio_data(id: u32) -> *const u8 {
    cue(id)
        .and_then(Cue::clip)
        .map_or(core::ptr::null(), |c| c.bytes().as_ptr())
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_audio_samples(id: u32) -> u32 {
    cue(id).and_then(Cue::clip).map_or(0, |c| c.samples as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_audio_rate(id: u32) -> u32 {
    cue(id).and_then(Cue::clip).map_or(0, |c| c.rate)
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_audio_interruptible(id: u32) -> u32 {
    u32::from(cue(id).is_some_and(Cue::interrupts_on_input))
}
// Deliberate public status allowlist. No shoe, RNG, hole face or raw App export.
#[unsafe(no_mangle)]
pub extern "C" fn sb_view() -> *const u8 {
    store::view().as_ptr()
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_view_len() -> u32 {
    store::view().len() as u32
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_focus() -> u32 {
    store::focus()
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_phase() -> u32 {
    store::acknowledged().map_or_else(|| app().game.phase as u32, |s| s.phase)
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_mode() -> u32 {
    store::acknowledged().map_or_else(|| app().training.mode as u32, |s| s.mode)
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_rounds() -> u32 {
    store::acknowledged().map_or_else(|| app().game.rounds, |s| s.rounds)
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_bankroll() -> i64 {
    store::acknowledged().map_or_else(|| app().game.bankroll, |s| s.chips)
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_wager() -> i64 {
    store::acknowledged().map_or_else(|| app().game.selected_bet, |s| s.wager)
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_brightness() -> u32 {
    store::acknowledged().map_or_else(|| app().brightness as u32, |s| s.brightness)
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_flipped() -> u32 {
    store::acknowledged().map_or_else(|| u32::from(app().flipped), |s| s.flipped)
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_sound_enabled() -> u32 {
    store::acknowledged().map_or_else(
        || u32::from(app().sound_enabled),
        |s| s.sound.min(u32::from(app().sound_enabled)),
    )
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_cancel_buttons() {
    app().cancel_buttons();
    store::cancel_buttons();
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_menu_item() -> *const u8 {
    store::item().as_ptr()
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_menu_item_len() -> u32 {
    store::item().len() as u32
}

/// Gain-only parity with the original master120/channel255 mixer. Raw PCM ABI remains unchanged.
#[unsafe(no_mangle)]
pub extern "C" fn sb_audio_scaled_sample(id: u32, index: u32) -> i32 {
    cue(id)
        .and_then(Cue::clip)
        .and_then(|clip| clip.sample(index as usize))
        .map_or(0, |raw| {
            stardust_blackjack::audio_gain::scaled_sample(raw) as i32
        })
}

/// Current acknowledged preference; immediate reductions are permitted like mute.
#[unsafe(no_mangle)]
pub extern "C" fn sb_audio_volume() -> u32 {
    store::acknowledged().map_or_else(|| app().audio_volume as u32, |s| s.audio_volume.min(app().audio_volume as u32))
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_audio_mask() -> u32 {
    store::acknowledged().map_or_else(|| app().audio_mask as u32, |s| s.audio_mask)
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_fast_mode() -> u32 {
    store::acknowledged().map_or_else(|| u32::from(app().fast_mode), |s| s.fast_mode)
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_audio_playback_sample(id: u32, index: u32) -> i32 {
    cue(id).and_then(Cue::clip).and_then(|clip| clip.sample(index as usize))
        .map_or(0, |raw| stardust_blackjack::audio_gain::playback_sample(raw, sb_audio_volume() as u8) as i32)
}
