#![allow(dead_code)]
#[path = "../../../src/resume.rs"]
mod resume;
#[path = "../../../src/storage_format.rs"]
mod storage_format;
mod storage {
    use crate::storage_format::{self as fmt, Meta, PACKET};
    use private_resume_journal_prototype::{Error, Flash};
    use stardust_blackjack::{persistence::Save, resume_binding::PublicCommit};
    pub const BASE: usize = 0x770000;
    #[derive(Clone)]
    pub struct Memory {
        pub bytes: Vec<u8>,
        pub io_failed: bool,
        pub operations: Vec<(u8, usize, usize)>,
    }
    impl Memory {
        pub fn new() -> Self {
            Self {
                bytes: vec![255; 0x80000],
                io_failed: false,
                operations: vec![],
            }
        }
    }
    impl Flash for Memory {
        fn read(&mut self, p: usize, out: &mut [u8]) -> Result<(), Error> {
            if self.io_failed {
                return Err(Error::Io);
            }
            let p = p.checked_sub(BASE).ok_or(Error::Bounds)?;
            out.copy_from_slice(self.bytes.get(p..p + out.len()).ok_or(Error::Bounds)?);
            self.operations.push((0, p, out.len()));
            Ok(())
        }
        fn erase(&mut self, p: usize, n: usize) -> Result<(), Error> {
            if self.io_failed {
                return Err(Error::Io);
            }
            let p = p.checked_sub(BASE).ok_or(Error::Bounds)?;
            if p % 4096 != 0 || n % 4096 != 0 {
                return Err(Error::Bounds);
            }
            self.bytes.get_mut(p..p + n).ok_or(Error::Bounds)?.fill(255);
            self.operations.push((1, p, n));
            Ok(())
        }
        fn program(&mut self, p: usize, b: &[u8]) -> Result<(), Error> {
            if self.io_failed {
                return Err(Error::Io);
            }
            let p = p.checked_sub(BASE).ok_or(Error::Bounds)?;
            let out = self.bytes.get_mut(p..p + b.len()).ok_or(Error::Bounds)?;
            for (o, &v) in out.iter_mut().zip(b) {
                if *o & v != v {
                    return Err(Error::Io);
                }
                *o &= v;
            }
            self.operations.push((2, p, b.len()));
            Ok(())
        }
    }
    impl Flash for &mut Memory {
        fn read(&mut self, p: usize, b: &mut [u8]) -> Result<(), Error> {
            (**self).read(p, b)
        }
        fn erase(&mut self, p: usize, n: usize) -> Result<(), Error> {
            (**self).erase(p, n)
        }
        fn program(&mut self, p: usize, b: &[u8]) -> Result<(), Error> {
            (**self).program(p, b)
        }
    }
    pub struct Storage {
        pub packet: [u8; PACKET],
        pub meta: Meta,
        pub nonce: [u8; 16],
        pub memory: Memory,
        pub enabled: bool,
        pub safe: bool,
        pub fail_public: bool,
        pub writes: u32,
    }
    impl Storage {
        pub fn new(save: Save) -> Self {
            Self {
                packet: save.encode().unwrap(),
                meta: Meta {
                    generation: 8,
                    epoch: 11,
                    nonce: [7; 16],
                },
                nonce: [7; 16],
                memory: Memory::new(),
                enabled: true,
                safe: true,
                fail_public: false,
                writes: 0,
            }
        }
        pub fn resume_safe(&self) -> bool {
            self.safe
        }
        pub fn read_failed(&self)->bool{!self.enabled}
        pub fn enabled(&self) -> bool {
            self.enabled
        }
        pub fn load(&mut self) -> Option<Save> {
            self.enabled = !self.fail_public;
            Save::decode(&self.packet)
        }
        pub fn checkpoint(&mut self, save: Save) -> bool {
            if self.fail_public {
                self.enabled = false;
                return false;
            }
            let Some(packet) = save.encode() else {
                return false;
            };
            let meta = Meta {
                generation: self.meta.generation.wrapping_add(1),
                epoch: self.meta.epoch.wrapping_add(1),
                nonce: self.nonce,
            };
            let record = fmt::record_v2(&packet, meta).unwrap();
            assert_eq!(fmt::decode_v2(&record.bytes).unwrap(), (meta, &packet[..]));
            self.packet = packet;
            self.meta = meta;
            self.safe = true;
            self.writes += 1;
            true
        }
        pub fn rotate_nonce(&mut self, n: [u8; 16]) {
            self.nonce = n
        }
        pub fn committed(&self) -> Option<PublicCommit<'_>> {
            if !self.safe {
                return None;
            }
            Some(PublicCommit {
                generation: self.meta.generation,
                checkpoint_epoch: self.meta.epoch,
                transaction_session: self.meta.nonce,
                packet: &self.packet,
            })
        }
        pub fn resume_partition(&self) -> Option<usize> {
            Some(BASE)
        }
        pub fn private_flash(&mut self) -> Option<&mut Memory> {
            Some(&mut self.memory)
        }
    }
}
fn main() {}
#[cfg(test)]
mod tests {
    use super::*;
    use private_resume_journal_prototype::{CommitTask, Journal, Progress};
    use stardust_blackjack::{
        app::App,
        controls::{Event, Key},
        resume_binding as binding, resume_private as snap,
        resume_ui::BootState,
        training::Mode,
    };
    static TEMPLATE: App = App::EMPTY;
    fn app(seed: u64) -> &'static mut App {
        Box::leak(Box::new(App::new(seed)))
    }
    pub(super) fn runtime() -> resume::Resume {
        let frozen = Box::leak(Box::new(App::EMPTY));
        let candidate = Box::leak(Box::new(App::EMPTY));
        let private = vec![0; snap::MAX_BYTES].leak();
        let bound = vec![0; snap::MAX_BYTES].leak();
        let public = vec![0; stardust_blackjack::persistence::SIZE].leak();
        unsafe {
            resume::Resume::new(
                resume::Regions {
                    frozen,
                    candidate,
                    private: private.as_mut_ptr(),
                    bound: bound.as_mut_ptr(),
                    public: public.as_mut_ptr(),
                    template: &TEMPLATE,
                },
                [9; 16],
            )
        }
    }
    pub(super) fn saved() -> (&'static mut App, storage::Storage) {
        let source = app(7);
        source.resume_enabled = true;
        source.training.mode = Mode::Guide;
        source.render();
        let mut store = storage::Storage::new(source.save());
        let mut private = vec![0; snap::MAX_BYTES];
        let n = snap::capture_private(source, &mut private, store.meta.epoch, true).unwrap();
        let mut bound = vec![0; snap::MAX_BYTES];
        let n =
            binding::bind_private(&mut bound, store.committed().unwrap(), &private[..n]).unwrap();
        let j = Journal::<262144>::new(storage::BASE);
        let mut task =
            CommitTask::new(&j, None, source.resume_session_id(), &bound[..n], true).unwrap();
        loop {
            if let Progress::Committed(_) = task.step(&mut store.memory).unwrap() {
                break;
            }
        }
        store.memory.operations.clear();
        (source, store)
    }
    pub(super) fn fresh(store: &storage::Storage) -> &'static mut App {
        let a = app(99);
        assert!(a.load(stardust_blackjack::persistence::Save::decode(&store.packet).unwrap()));
        assert_eq!(a.training.mode, Mode::Off);
        a
    }
    #[test]
    fn long_chooser_wait_preserves_raw_guide_public_identity_and_requires_neutral() {
        let (source, mut store) = saved();
        let mut a = fresh(&store);
        let raw = store.packet;
        let meta = store.meta;
        let mut r = runtime();
        r.boot(a, &mut store, 100);
        assert!(matches!(r.chooser.state, BootState::ChooseSaved { .. }));
        for now in [101, 1000, 10000, 120000] {
            r.chooser_events(
                &mut a,
                &mut store,
                [true, false],
                &[Event::Tap(Key::Blue)],
                now,
            );
            r.step(a, &mut store, now);
            assert_eq!(store.packet, raw);
            assert_eq!(store.meta, meta);
            assert_eq!(store.writes, 0);
            assert!(r.choosing());
        }
        r.chooser_events(&mut a, &mut store, [false, false], &[], 120001);
        r.chooser_events(
            &mut a,
            &mut store,
            [false, false],
            &[Event::Tap(Key::Blue)],
            120002,
        );
        assert!(!r.choosing());
        assert_eq!(a.training.mode, Mode::Guide);
        assert_eq!(a.save(), source.save());
        assert_eq!(store.packet, raw);
        assert!(!r.accept_input([true, false]));
        assert!(!r.accept_input([false, false]));
        assert!(r.accept_input([false, false]));
        assert!(r.acknowledge(a, &mut store, 120003));
        assert_eq!(store.writes, 0);
    }
    #[test]
    fn power_loss_during_chooser_changes_no_storage_and_offers_again() {
        let (_, mut store) = saved();
        let original = store.memory.bytes.clone();
        let raw = store.packet;
        for now in [100, 10000, 120000] {
            let a = fresh(&store);
            let mut r = runtime();
            r.boot(a, &mut store, now);
            r.step(a, &mut store, now + 100000);
            assert!(r.choosing());
            assert_eq!(store.packet, raw);
            assert_eq!(store.memory.bytes, original);
            assert_eq!(store.writes, 0);
        }
    }
    #[test]
    fn explicit_fresh_commits_off_before_invalidating_old_private() {
        let (_, mut store) = saved();
        let mut a = fresh(&store);
        let old = store.memory.bytes.clone();
        let old_meta = store.meta;
        let mut r = runtime();
        r.boot(a, &mut store, 1);
        r.chooser_events(&mut a, &mut store, [false, false], &[], 2);
        r.chooser_events(
            &mut a,
            &mut store,
            [false, false],
            &[Event::Tap(Key::Grey), Event::Tap(Key::Blue)],
            3,
        );
        assert!(!r.choosing());
        assert_eq!(a.training.mode, Mode::Off);
        assert_eq!(store.writes, 1);
        assert_ne!(store.meta, old_meta);
        assert_eq!(
            store.memory.bytes, old,
            "private erase follows public commit cooperatively"
        );
        let mut scratch = vec![0; snap::MAX_BYTES];
        let j = Journal::<262144>::new(storage::BASE);
        let record = j.latest(&mut store.memory, &mut scratch).unwrap();
        let mut c = App::EMPTY;
        assert_eq!(
            binding::restore_bound(
                &mut c,
                &scratch[..record.length],
                store.committed().unwrap(),
                3,
                true
            ),
            Err(binding::Error::Stale)
        );
    }
    #[test]
    fn default_off_never_writes_private_even_after_events() {
        let a = app(7);
        let mut store = storage::Storage::new(a.save());
        let mut r = runtime();
        r.boot(a, &mut store, 1);
        for now in [2, 2000, 10000] {
            a.event(Event::Tap(Key::Blue));
            a.render();
            assert!(r.acknowledge(a, &mut store, now));
            r.step(a, &mut store, now + 2000);
        }
        assert!(store.memory.operations.is_empty());
        assert_eq!(r.writes, 0);
    }
    #[test]
    fn public_failure_blocks_ack_and_private_writes_until_read_retry() {
        let (_, mut store) = saved();
        let mut a = fresh(&store);
        let mut r = runtime();
        r.boot(a, &mut store, 1);
        r.chooser_events(&mut a, &mut store, [false, false], &[], 2);
        r.chooser_events(
            &mut a,
            &mut store,
            [false, false],
            &[Event::Tap(Key::Blue)],
            3,
        );
        store.memory.operations.clear();
        store.fail_public = true;
        a.event(Event::Tap(Key::Blue));
        a.render();
        assert!(!r.outputs_allowed(a));
        assert!(!r.acknowledge(a, &mut store, 4));
        assert!(r.blocked);
        assert!(!r.outputs_allowed(a));
        r.step(a, &mut store, 10000);
        assert!(store.memory.operations.is_empty());
        store.fail_public = false;
        r.chooser_events(&mut a,&mut store,[false,false],&[],10000);
        r.chooser_events(&mut a,&mut store,[false,false],&[Event::Tap(Key::Blue)],10001);
        assert!(!r.blocked);
        assert!(r.outputs_allowed(a));
        assert_eq!(store.writes, 1);
    }
    #[test]
    fn private_read_failure_is_retryable_and_does_not_overwrite() {
        let (_, mut store) = saved();
        store.memory.io_failed = true;
        let mut a = fresh(&store);
        let mut r = runtime();
        let raw = store.packet;
        r.boot(a, &mut store, 1);
        assert!(matches!(r.chooser.state, BootState::ReadFailure { .. }));
        assert_eq!(store.packet, raw);
        assert_eq!(store.writes, 0);
        store.memory.io_failed = false;
        r.chooser_events(&mut a, &mut store, [false, false], &[], 2);
        r.chooser_events(
            &mut a,
            &mut store,
            [false, false],
            &[Event::Tap(Key::Blue)],
            3,
        );
        assert!(matches!(r.chooser.state, BootState::ChooseSaved { .. }));
        assert_eq!(store.writes, 0);
    }
    #[test]
    fn idle_capture_is_cooperative_and_new_input_invalidates_prior_checkpoint() {
        let (_, mut store) = saved();
        let mut a = fresh(&store);
        let mut r = runtime();
        r.boot(a, &mut store, 1);
        r.chooser_events(&mut a, &mut store, [false, false], &[], 2);
        r.chooser_events(
            &mut a,
            &mut store,
            [false, false],
            &[Event::Tap(Key::Blue)],
            3,
        );
        a.event(Event::Tap(Key::Blue));
        a.render();
        assert!(r.acknowledge(a, &mut store, 4));
        let before = store.meta;
        store.memory.operations.clear();
        for n in 0..3000 {
            let count = store.memory.operations.len();
            r.step(a, &mut store, 2000 + n);
            assert!(store.memory.operations.len() - count <= 1);
            if r.writes > 0 {
                break;
            }
        }
        assert_eq!(r.writes, 1);
        for &(kind, _, n) in &store.memory.operations {
            if kind == 1 {
                assert_eq!(n, 4096)
            } else {
                assert!(n <= 512)
            }
        }
        let mut scratch = vec![0; snap::MAX_BYTES];
        let j = Journal::<262144>::new(storage::BASE);
        let record = j.latest(&mut store.memory, &mut scratch).unwrap();
        let mut c = App::EMPTY;
        assert!(
            binding::restore_bound(
                &mut c,
                &scratch[..record.length],
                store.committed().unwrap(),
                1,
                true
            )
            .is_ok()
        );
        a.event(Event::Tap(Key::Blue));
        a.render();
        assert!(r.acknowledge(a, &mut store, 6000));
        assert_ne!(store.meta, before);
        let mut c = App::EMPTY;
        assert_eq!(
            binding::restore_bound(
                &mut c,
                &scratch[..record.length],
                store.committed().unwrap(),
                1,
                true
            ),
            Err(binding::Error::Stale)
        );
    }
}

#[cfg(test)]
mod unsafe_public_test {
    use super::*;
    use stardust_blackjack::{
        controls::{Event, Key},
        resume_ui::BootState,
    };
    #[test]
    fn unsafe_public_identity_cannot_restore_private_and_explicit_fresh_recovers() {
        let (a, mut store) = super::tests::saved();
        let active = super::tests::fresh(&store);
        store.safe = false;
        let mut r = super::tests::runtime();
        r.boot(active, &mut store, 1);
        assert!(matches!(r.chooser.state, BootState::ReadFailure { .. }));
        let raw = store.packet;
        assert_eq!(store.writes, 0);
        let mut active = active;
        r.chooser_events(&mut active, &mut store, [false, false], &[], 2);
        r.chooser_events(
            &mut active,
            &mut store,
            [false, false],
            &[Event::Tap(Key::Grey), Event::Tap(Key::Blue)],
            3,
        );
        assert!(!r.choosing());
        assert!(store.safe);
        assert_ne!(store.packet, raw);
        assert_eq!(
            active.training.mode,
            stardust_blackjack::training::Mode::Off
        );
        assert_eq!(active.training.stats(), a.training.stats());
    }
}
#[cfg(test)]mod reenabling_test{use super::*;use stardust_blackjack::{controls::{Event,Key},resume_binding as b,resume_private as s};use private_resume_journal_prototype::{Journal,Progress,CommitTask};
#[test]fn reenable_after_off_boot_scans_old_sequence_before_first_new_checkpoint(){let(_,mut store)=super::tests::saved();let j=Journal::<262144>::new(storage::BASE);let mut scratch=vec![0;s::MAX_BYTES];let old=j.latest(&mut store.memory,&mut scratch).unwrap();let body=scratch[..old.length].to_vec();j.commit(&mut store.memory,1,&body,true,&mut scratch).unwrap();let previous=j.commit(&mut store.memory,1,&body,true,&mut scratch).unwrap();let mut off=stardust_blackjack::persistence::Save::decode(&store.packet).unwrap();off.resume_enabled=false;assert!(store.checkpoint(off));store.memory.operations.clear();let a=super::tests::fresh(&store);let mut r=super::tests::runtime();r.boot(a,&mut store,1);assert!(!r.choosing());assert!(store.memory.operations.is_empty());a.resume_enabled=true;a.event(Event::Tap(Key::Grey));a.render();assert!(r.acknowledge(a,&mut store,2));for n in 0..4000{r.step(a,&mut store,2000+n);if r.writes>0{break}}assert_eq!(r.writes,1);let latest=j.latest(&mut store.memory,&mut scratch).unwrap();assert_eq!(latest.sequence,previous.sequence+1);let mut candidate=stardust_blackjack::app::App::EMPTY;assert!(b::restore_bound(&mut candidate,&scratch[..latest.length],store.committed().unwrap(),0,true).is_ok());}}
