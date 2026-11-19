use stardust_blackjack::{
    app::App,
    controls::{Event, Key},
    engine::Phase,
    resume_private::{self as r, Error},
};
fn packet(a: &App) -> Vec<u8> {
    let mut b = vec![0; r::MAX_BYTES];
    let n=r::capture_private(a,&mut b,41,true).unwrap_or_else(|e|panic!("capture error={e:?} phase={:?} view={} pos={} marker={} cut={} hands={} active={} bankroll={} stats={:?}",a.game.phase,a.view_name(),a.game.position,a.game.marker,a.game.cut_at,a.game.hand_count,a.game.active,a.game.bankroll,a.training.stats().all));
    b.truncate(n);
    b
}
fn advance(a: &mut App, now: &mut u32) {
    *now = now.wrapping_add(1201);
    a.tick(*now);
    let e = if a.game.phase == Phase::Player && a.game.hands[a.game.active].total() < 16 {
        Event::Tap(Key::Grey)
    } else {
        Event::Tap(Key::Blue)
    };
    a.event(e);
    a.render();
}
#[test]
fn every_phase_snapshot_exactly_round_trips_and_continues() {
    let mut largest = 0;
    let mut phases = 0u32;
    for seed in [0, 1, 7, 99, u64::MAX] {
        for mode in 0..4 {
            let mut a = Box::new(App::new(seed));
            for _ in 0..mode {
                a.event(Event::Hold(Key::Blue))
            }
            let mut now = 0;
            for _ in 0..200 {
                a.render();
                let b = packet(&a);
                largest = largest.max(b.len());
                phases |= 1 << a.game.phase as u8;
                let mut c = Box::new(App::EMPTY);
                let restored = r::restore_candidate(&mut c, &b, now, true).unwrap();
                assert_eq!(restored.sequence, 41);
                assert_eq!(a.frame.pixels, c.frame.pixels);
                assert_eq!(packet(&a), packet(&c));
                let mut after = now;
                advance(&mut c, &mut after);
                advance(&mut a, &mut now);
                assert_eq!(packet(&a), packet(&c));
            }
        }
    }
    println!("snapshots=4000 max_bytes={largest} phases_mask={phases:#x}");
}
#[test]
fn clock_rebase_neutral_gate_and_opt_in() {
    let mut a = App::new(7);
    a.tick(u32::MAX - 100);
    a.event(Event::Tap(Key::Blue));
    a.render();
    let b = packet(&a);
    let mut c = App::EMPTY;
    let mut restored = r::restore_candidate(&mut c, &b, 50, true).unwrap();
    assert_eq!(a.frame.pixels, c.frame.pixels);
    assert!(!restored.neutral.accept([true, false]));
    assert!(!restored.neutral.accept([true, true]));
    assert!(!restored.neutral.accept([false, false]));
    assert!(restored.neutral.accept([false, false]));
    assert_eq!(
        r::restore_candidate(&mut c, &b, 0, false),
        Err(Error::Disabled)
    );
    assert_eq!(
        r::capture_private(&a, &mut [0; 100], 1, false),
        Err(Error::Disabled)
    );
}
#[test]
fn truncated_and_every_byte_corrupted_packets_fail() {
    let a = App::new(99);
    let b = packet(&a);
    for n in 0..b.len() {
        let mut c = App::EMPTY;
        assert!(
            r::restore_candidate(&mut c, &b[..n], 0, true).is_err(),
            "length{n}"
        );
    }
    for i in 0..b.len() {
        let mut bad = b.clone();
        bad[i] ^= 1;
        let mut c = App::EMPTY;
        assert!(
            r::restore_candidate(&mut c, &bad, 0, true).is_err(),
            "byte{i}"
        );
    }
    println!("corrupt bytes/lengths={} each", b.len());
}
#[test]
fn correctly_checksummed_mutations_never_panic_or_change_live_app() {
    let a = App::new(7);
    let before = packet(&a);
    let mut accepted = 0;
    for i in r::HEADER..before.len() {
        let mut b = before.clone();
        b[i] ^= 0xa5;
        let c = stardust_blackjack::persistence::crc32(&b[r::HEADER..]);
        b[40..44].copy_from_slice(&c.to_le_bytes());
        let c = stardust_blackjack::persistence::crc32(&b[..44]);
        b[44..48].copy_from_slice(&c.to_le_bytes());
        let result = std::panic::catch_unwind(|| {
            let mut candidate = Box::new(App::EMPTY);
            if r::restore_candidate(&mut candidate, &b, 0, true).is_ok() {
                let _ = packet(&candidate);
                true
            } else {
                false
            }
        });
        assert!(result.is_ok(), "panic on semantic mutation byte{i}");
        accepted += usize::from(result.unwrap());
        assert_eq!(packet(&a), before);
    }
    println!(
        "checksummed semantic mutations={} accepted_valid={accepted}",
        before.len() - r::HEADER
    );
}
#[test]
fn live_app_cannot_be_used_as_decode_candidate() {
    let mut live = App::new(99);
    let before = packet(&live);
    assert_eq!(
        r::restore_candidate(&mut live, &before, 0, true),
        Err(Error::Inconsistent)
    );
    assert_eq!(packet(&live), before);
}
#[test]
fn historical_scores_above_f64_integer_precision_are_exact() {
    let old =
        stardust_blackjack::persistence::Save::decode(include_bytes!("data/save-v5.bin")).unwrap();
    assert!(old.count_stats.all.decisions > (1u64 << 53));
    let mut a = App::new(7);
    assert!(a.load(old));
    assert_eq!(a.training.mode, stardust_blackjack::training::Mode::Off);
    a.training.mode = old.mode; // Explicit live training state; normal preference import intentionally boots Off.
    a.render();
    let b = packet(&a);
    let mut c = App::EMPTY;
    r::restore_candidate(&mut c, &b, 0, true).unwrap();
    assert_eq!(c.save(), old);
    assert_eq!(packet(&c), b);
}
#[test]
fn cooperative_capture_matches_single_pass_at_all_primitive_boundaries() {
    let mut a = App::new(99);
    let mut now = 0;
    for _ in 0..50 {
        advance(&mut a, &mut now)
    }
    let expected = packet(&a);
    for budget in [1, 2, 3, 4, 7, 16, 64, 512, 4096] {
        let mut out = vec![0; r::MAX_BYTES];
        let mut task = r::CaptureTask::new(&a, &mut out, 41, true).unwrap();
        let mut calls = 0;
        let n = loop {
            calls += 1;
            match task.step(budget).unwrap() {
                r::CaptureProgress::More => {}
                r::CaptureProgress::Done { bytes } => break bytes,
            }
        };
        assert_eq!(&out[..n], &expected);
        assert!(calls >= n / budget.min(512));
    }
}
