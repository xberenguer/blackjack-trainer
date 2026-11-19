use stardust_blackjack::{
    app::App,
    controls::{Event, Key},
    persistence::Save,
    resume_binding::{self as binding, Error, PublicCommit},
    resume_private as snap,
};
fn private(a: &App, epoch: u64) -> Vec<u8> {
    let mut b = vec![0; snap::MAX_BYTES];
    let n = snap::capture_private(a, &mut b, epoch, true).unwrap();
    b.truncate(n);
    b
}
fn bundle(a: &App, generation: u64, epoch: u64, nonce: [u8; 16]) -> Vec<u8> {
    let public = a.save().encode().unwrap();
    let checkpoint = private(a, epoch);
    let mut b = vec![0; snap::MAX_BYTES];
    let n = binding::bind_private(
        &mut b,
        PublicCommit {
            generation,
            checkpoint_epoch: epoch,
            transaction_session: nonce,
            packet: &public,
        },
        &checkpoint,
    )
    .unwrap();
    b.truncate(n);
    b
}
fn check(
    a: &App,
    bundle: &[u8],
    generation: u64,
    epoch: u64,
    nonce: [u8; 16],
) -> Result<App, Error> {
    let mut candidate = App::EMPTY;
    binding::restore_bound(
        &mut candidate,
        bundle,
        PublicCommit {
            generation,
            checkpoint_epoch: epoch,
            transaction_session: nonce,
            packet: &a.save().encode().unwrap(),
        },
        0,
        true,
    )?;
    Ok(candidate)
}
#[test]
fn powerloss_between_public_and_private_commit_refuses_old_resume() {
    let a = App::new(99);
    let old = bundle(&a, 5, 41, [7; 16]);
    assert!(check(&a, &old, 5, 41, [7; 16]).is_ok());
    let public_new_epoch = 42;
    assert!(matches!(
        check(&a, &old, 6, public_new_epoch, [7; 16]),
        Err(Error::Stale)
    ));
    let new = bundle(&a, 6, 42, [7; 16]);
    assert!(check(&a, &new, 6, 42, [7; 16]).is_ok());
}
#[test]
fn same_mode_off_no_score_gameplay_still_requires_new_epoch() {
    let mut a = App::new(7);
    assert_eq!(a.save().mode, stardust_blackjack::training::Mode::Off);
    let before = a.save();
    let old = bundle(&a, 1, 1, [2; 16]);
    a.event(Event::Tap(Key::Blue));
    assert_eq!(a.save(), before);
    assert!(matches!(check(&a, &old, 2, 2, [2; 16]), Err(Error::Stale)));
    assert!(
        matches!(check(&a, &old, 1, 2, [2; 16]), Err(Error::Stale)),
        "Epoch changes must matter even if an erroneous caller reuses public generation"
    );
    let new = bundle(&a, 2, 2, [2; 16]);
    let resumed = check(&a, &new, 2, 2, [2; 16]).unwrap();
    assert_eq!(resumed.game.position, a.game.position);
}
#[test]
fn reset_stats_mode_and_newer_public_progress_cannot_be_rolled_back() {
    let data = include_bytes!("data/save-v5.bin");
    let settings = Save::decode(data).unwrap();
    let mut a = App::new(7);
    assert!(a.load(settings));
    a.training.mode = settings.mode;
    let old = bundle(&a, 10, 10, [3; 16]);
    let mut reset = App::new(7);
    let mut zero = settings;
    zero.stats = Default::default();
    zero.count_stats = Default::default();
    assert!(reset.load(zero));
    reset.training.mode = settings.mode;
    assert!(
        matches!(check(&reset, &old, 10, 10, [3; 16]), Err(Error::Stale)),
        "Exact packet comparison must catch a reset even without metadata differences"
    );
    assert!(matches!(
        check(&reset, &old, 11, 11, [3; 16]),
        Err(Error::Stale)
    ));
    let latest = reset.save();
    assert_eq!(latest.stats.all.decisions, 0);
    assert_eq!(latest.count_stats.all.decisions, 0);
    a.training.mode = stardust_blackjack::training::Mode::Off;
    assert!(matches!(
        check(&a, &old, 11, 11, [3; 16]),
        Err(Error::Stale)
    ));
}
#[test]
fn private_ahead_public_wrap_and_new_session_are_refused() {
    let a = App::new(7);
    let ahead = bundle(&a, 2, 2, [8; 16]);
    assert!(matches!(
        check(&a, &ahead, 1, 1, [8; 16]),
        Err(Error::Stale)
    ));
    let old = bundle(&a, u64::MAX, u64::MAX, [8; 16]);
    assert!(matches!(check(&a, &old, 0, 0, [8; 16]), Err(Error::Stale)));
    let wrapped = bundle(&a, 0, 0, [9; 16]);
    assert!(check(&a, &wrapped, 0, 0, [9; 16]).is_ok());
    assert!(matches!(
        check(&a, &wrapped, 0, 0, [8; 16]),
        Err(Error::Stale)
    ));
}
#[test]
fn raw_committed_mode_binding_preserves_private_guide_while_normal_import_is_off() {
    let mut a = App::new(99);
    a.training.mode = stardust_blackjack::training::Mode::Guide;
    let raw = a.save();
    let committed = bundle(&a, 7, 7, [4; 16]);
    let mut fresh = App::new(99);
    assert!(fresh.load(raw));
    assert_eq!(fresh.training.mode, stardust_blackjack::training::Mode::Off);
    let restored = check(&a, &committed, 7, 7, [4; 16]).unwrap();
    assert_eq!(
        restored.training.mode,
        stardust_blackjack::training::Mode::Guide
    );
}
#[test]
fn cooperative_binding_matches_reference_with_arbitrary_byte_boundaries() {
    let a = App::new(7);
    let public = a.save().encode().unwrap();
    let private = private(&a, 77);
    let commit = PublicCommit {
        generation: u64::MAX,
        checkpoint_epoch: 77,
        transaction_session: [23; 16],
        packet: &public,
    };
    let mut expected = vec![0; snap::MAX_BYTES];
    let n = binding::bind_private(&mut expected, commit, &private).unwrap();
    for budget in [1, 3, 7, 64, 512, 4096] {
        let mut out = vec![0; snap::MAX_BYTES];
        let mut task = binding::BindTask::new(&mut out, commit, &private).unwrap();
        let m = loop {
            if let binding::BindProgress::Done { bytes } = task.step(budget).unwrap() {
                break bytes;
            }
        };
        assert_eq!(m, n);
        assert_eq!(&out[..m], &expected[..n]);
    }
}
