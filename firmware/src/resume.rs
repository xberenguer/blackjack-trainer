//! Single-owner private persistence coordinator. Opaque buffers never reach USB diagnostics.
use crate::{storage::Storage, storage_format::Meta};
use private_resume_journal_prototype::{CommitTask, Error, Journal, Progress, Record};
use stardust_blackjack::{
    app::App,
    controls::{Controls, Event},
    persistence::{SIZE, Save},
    resume_binding::{self, BindProgress, BindTask, PublicCommit},
    resume_private::{self, CaptureProgress, CaptureTask, NeutralGate},
    resume_ui::{BootChooser, BootState, Command, Preference},
};
const SLOT: usize = 256 * 1024;
enum Job {
    Capture(CaptureTask<'static>),
    Bind(BindTask<'static>),
    Write(CommitTask<'static, SLOT>),
}
/// All regions are distinct PSRAM ranges allocated once by main; only this owner creates references.
pub struct Regions {
    pub frozen: *mut App,
    pub candidate: *mut App,
    pub private: *mut u8,
    pub bound: *mut u8,
    pub public: *mut u8,
    pub template: *const App,
}
pub struct Resume {
    acknowledged: Save,
    pub ram_only: bool,
    boot_read_failed: bool,
    regions: Regions,
    journal: Option<Journal<SLOT>>,
    previous: Option<Record>,
    job: Option<Job>,
    meta: Meta,
    private_len: usize,
    bound_len: usize,
    captured_revision: u64,
    done_revision: Option<u64>,
    ack_revision: u64,
    migration_pending: bool,
    last_enabled: bool,
    changed: u32,
    private_io_failed: bool,
    candidate_ready: bool,
    pub chooser: BootChooser,
    controls: Controls,
    neutral: Option<NeutralGate>,
    nonce: [u8; 16],
    pub blocked: bool,
    pub writes: u32,
    pub errors: u32,
}
impl Resume {
    /// SAFETY: caller owns initialized, aligned, nonoverlapping regions for the entire program;
    /// none are accessed elsewhere while this coordinator exists. Active App is in a fourth region.
    pub unsafe fn new(regions: Regions, nonce: [u8; 16]) -> Self {
        Self {
            acknowledged: Save::default(),
            ram_only: false,
            boot_read_failed: false,
            regions,
            journal: None,
            previous: None,
            job: None,
            meta: Meta {
                generation: 0,
                epoch: 0,
                nonce,
            },
            private_len: 0,
            bound_len: 0,
            captured_revision: 0,
            done_revision: None,
            ack_revision: 0,
            migration_pending: false,
            last_enabled: false,
            changed: 0,
            private_io_failed: false,
            candidate_ready: false,
            chooser: BootChooser::new(Preference::OFF, false, false),
            controls: Controls::EMPTY,
            neutral: None,
            nonce,
            blocked: false,
            writes: 0,
            errors: 0,
        }
    }
    pub fn boot(&mut self, app: &mut App, store: &mut Storage, now: u32) {
        self.acknowledged = app.save();
        if store.read_failed() {
            self.boot_read_failed = true;
            self.fail_public(app);
            return;
        }
        self.journal = store.resume_partition().map(Journal::new);
        self.last_enabled = app.resume_enabled;
        self.ack_revision = app.checkpoint_revision();
        self.changed = now;
        if app.resume_enabled {
            self.scan(store, now);
            self.chooser = BootChooser::new(
                Preference::from_enabled(true),
                self.candidate_ready,
                self.private_io_failed,
            );
            if matches!(self.chooser.state, BootState::Fresh) {
                self.fresh(app, store, now);
            }
        }
    }
    fn scan(&mut self, store: &mut Storage, now: u32) {
        self.candidate_ready = false;
        self.private_io_failed = false;
        if !store.resume_safe() {
            self.private_io_failed = true;
            return;
        }
        let Some(j) = &self.journal else { return };
        let Some(mut flash) = store.private_flash() else {
            self.private_io_failed = true;
            return;
        };
        // SAFETY: scan is called only with no capture/write job. Bound buffer is exclusively writable.
        let buffer = unsafe {
            core::slice::from_raw_parts_mut(self.regions.bound, resume_private::MAX_BYTES)
        };
        match j.latest(&mut flash, buffer) {
            Ok(record) => {
                self.previous = Some(record);
                if record.tombstone {
                    return;
                }
                self.bound_len = record.length;
                let _ = flash;
                // SAFETY: candidate is separate from active/frozen; copying the immutable App::EMPTY image
                // creates a valid App without a large constructor temporary. No references to candidate escape.
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        self.regions.template,
                        self.regions.candidate,
                        1,
                    );
                }
                if let Some(public) = store.committed() {
                    let candidate = unsafe { &mut *self.regions.candidate };
                    match resume_binding::restore_bound(
                        candidate,
                        &buffer[..record.length],
                        public,
                        now,
                        true,
                    ) {
                        Ok(restored) => {
                            self.candidate_ready = true;
                            self.neutral = Some(restored.neutral)
                        }
                        // Public-first invalidation deliberately makes earlier snapshots stale.
                        Err(resume_binding::Error::Stale) => {}
                        Err(_) => {
                            self.errors += 1;
                        }
                    }
                }
            }
            Err(Error::Io) => {
                self.private_io_failed = true;
                self.errors += 1
            }
            Err(_) => {
                self.previous = None;
            }
        }
    }
    fn abort(&mut self) {
        self.job = None;
        self.done_revision = None;
    }
    fn fresh(&mut self, app: &mut App, store: &mut Storage, now: u32) {
        self.abort();
        store.rotate_nonce(self.nonce);
        if !store.checkpoint(app.save()) {
            self.fail_public(app);
            return;
        }
        self.acknowledged = app.save();
        self.blocked = false;
        self.migration_pending = false;
        self.ack_revision = app.checkpoint_revision();
        self.last_enabled = app.resume_enabled;
        self.changed = now;
        self.chooser.state = BootState::Fresh;
        self.scan(store, now);
        self.candidate_ready = false;
        self.neutral = Some(NeutralGate::new());
        app.cancel_buttons();
        app.dirty = true;
        if !self.private_io_failed {
            if let Some(j) = &self.journal {
                self.job = CommitTask::tombstone(j, self.previous, app.resume_session_id())
                    .ok()
                    .map(Job::Write);
            }
        }
    }
    pub fn choosing(&self) -> bool {
        !matches!(self.chooser.state, BootState::Fresh)
    }
    pub fn chooser_events(
        &mut self,
        app: &mut &'static mut App,
        store: &mut Storage,
        down: [bool; 2],
        external: &[Event],
        now: u32,
    ) {
        let ready = self.chooser.neutral(down);
        let mut command = Command::None;
        if ready {
            for e in self
                .controls
                .update(down, now, false)
                .into_iter()
                .flatten()
                .chain(external.iter().copied())
            {
                let c = self.chooser.event(e);
                if c != Command::None {
                    command = c;
                    break;
                }
            }
        } else {
            self.controls = Controls::EMPTY;
        }
        if self.blocked {
            match command {
                Command::RetryRead => self.retry_public(app, store, now),
                Command::CommitNewSessionTombstone => self.ram_fresh(app, now),
                _ => {}
            }
            return;
        }
        match command {
            Command::RestoreCandidate if self.candidate_ready => {
                // SAFETY: the validated candidate occupies a distinct region and has no outstanding references.
                // Move only the exclusive reference, never the large App value. The old active region becomes
                // the next candidate; this owner will reset it before decoding any future private packet.
                let candidate = unsafe { &mut *self.regions.candidate };
                let old = core::mem::replace(app, candidate);
                self.regions.candidate = old as *mut App;
                self.chooser.state = BootState::Fresh;
                self.acknowledged = app.save();
                self.ack_revision = app.checkpoint_revision();
                self.last_enabled = app.resume_enabled;
                self.changed = now;
                // Keep the original bound packet unchanged throughout the chooser.
                // Explicit restore may introduce newer preference defaults. Their
                // canonical packet needs a new public identity before any output
                // or private capture is authorized.
                self.migration_pending = store.committed().is_none_or(|public| {
                    app.save()
                        .encode()
                        .as_ref()
                        .is_none_or(|packet| packet.as_slice() != public.packet)
                });
                self.done_revision = if self.migration_pending {
                    None
                } else {
                    Some(self.ack_revision)
                };
                app.resume_commit_clock(now);
                self.ack_revision = app.checkpoint_revision();
                app.cancel_buttons();
                app.dirty = true;
            }
            Command::CommitNewSessionTombstone => self.fresh(app, store, now),
            Command::RetryRead => {
                self.scan(store, now);
                self.chooser = BootChooser::new(
                    Preference::from_enabled(true),
                    self.candidate_ready,
                    self.private_io_failed,
                );
            }
            _ => {}
        }
    }
    fn fail_public(&mut self, app: &mut App) {
        self.abort();
        self.blocked = true;
        self.chooser = BootChooser::new(Preference::from_enabled(true), false, true);
        self.controls = Controls::EMPTY;
        self.neutral = Some(NeutralGate::new());
        app.cancel_buttons();
    }
    fn retry_public(&mut self, app: &mut &'static mut App, store: &mut Storage, now: u32) {
        if self.boot_read_failed {
            let save = store.load();
            if store.read_failed() {
                return;
            }
            if let Some(save) = save {
                if !app.load(save) {
                    return;
                }
            }
            self.boot_read_failed = false;
            self.blocked = false;
            self.chooser.state = BootState::Fresh;
            self.boot(app, store, now);
            app.dirty = true;
            return;
        }
        // Retry exactly the current unacknowledged transition; never restore an older private question.
        self.chooser.state = BootState::Fresh;
        if self.acknowledge(app, store, now) {
            self.neutral = Some(NeutralGate::new());
            self.controls = Controls::EMPTY;
            app.cancel_buttons();
            app.checkpoint_output_clock(now);
            app.dirty = true;
        }
    }
    fn ram_fresh(&mut self, app: &mut &'static mut App, now: u32) {
        self.abort();
        let mut prefs = self.acknowledged;
        prefs.resume_enabled = false;
        let seed = u64::from_le_bytes(self.nonce[..8].try_into().expect("nonce bytes"))
            ^ 0x72616d5f66726573
            ^ now as u64;
        // SAFETY: all jobs were dropped; inactive candidate is exclusive and initialized from EMPTY.
        unsafe {
            core::ptr::copy_nonoverlapping(self.regions.template, self.regions.candidate, 1);
        }
        let candidate = unsafe { &mut *self.regions.candidate };
        assert!(candidate.boot(seed));
        assert!(candidate.load(prefs));
        candidate.resume_enabled = false;
        candidate.resume_commit_clock(now);
        let old = core::mem::replace(app, candidate);
        self.regions.candidate = old as *mut App;
        self.ram_only = true;
        self.migration_pending = false;
        self.boot_read_failed = false;
        self.blocked = false;
        self.last_enabled = false;
        self.chooser.state = BootState::Fresh;
        self.candidate_ready = false;
        self.neutral = Some(NeutralGate::new());
        self.controls = Controls::EMPTY;
        self.ack_revision = app.checkpoint_revision();
        app.cancel_buttons();
        app.dirty = true;
    }
    pub fn record_public_saved(&mut self, save: Save) {
        if !self.ram_only {
            self.acknowledged = save;
        }
    }
    pub fn checkpoint_current(&self, app: &App) -> bool {
        !self.ram_only
            && !self.blocked
            && !self.choosing()
            && app.resume_enabled
            && self.job.is_none()
            && self.done_revision == Some(app.checkpoint_revision())
    }
    pub fn saved_scores_unavailable(&self) -> bool {
        self.boot_read_failed
    }
    pub fn benchmark_region(&mut self) -> Option<*mut App> {
        if self.choosing() || self.blocked {
            None
        } else {
            self.candidate_ready = false;
            Some(self.regions.candidate)
        }
    }
    pub fn outputs_allowed(&self, app: &App) -> bool {
        !self.choosing()
            && !self.blocked
            && !self.migration_pending
            && (!app.resume_enabled && !self.last_enabled
                || app.checkpoint_revision() == self.ack_revision)
    }
    pub fn accept_input(&mut self, down: [bool; 2]) -> bool {
        if let Some(g) = &mut self.neutral {
            if !g.accept(down) {
                return false;
            }
            self.neutral = None;
        }
        !self.blocked && !self.choosing() && !self.migration_pending
    }
    /// Called after render has latched aid/help, before showing that frame or sending it over USB.
    pub fn acknowledge(&mut self, app: &mut App, store: &mut Storage, now: u32) -> bool {
        if self.ram_only {
            self.abort();
            app.resume_enabled = false;
            self.ack_revision = app.checkpoint_revision();
            self.last_enabled = false;
            self.migration_pending = false;
            return true;
        }
        let revision = app.checkpoint_revision();
        if revision == self.ack_revision && !self.blocked && !self.migration_pending {
            return true;
        }
        self.abort();
        self.changed = now;
        if app.resume_enabled || self.last_enabled {
            if !store.enabled() {
                let _ = store.load();
            }
            store.rotate_nonce(self.nonce);
            if !store.checkpoint(app.save()) {
                self.fail_public(app);
                self.errors += 1;
                return false;
            }
        }
        if app.resume_enabled || self.last_enabled {
            self.acknowledged = app.save();
        }
        let was_enabled = self.last_enabled;
        if app.resume_enabled && !was_enabled {
            self.scan(store, now);
            self.candidate_ready = false;
        }
        self.blocked = false;
        self.migration_pending = false;
        self.ack_revision = revision;
        self.last_enabled = app.resume_enabled;
        if was_enabled && !app.resume_enabled && !self.private_io_failed {
            if let Some(j) = &self.journal {
                self.job = CommitTask::tombstone(j, self.previous, app.resume_session_id())
                    .ok()
                    .map(Job::Write);
            }
        }
        true
    }
    fn start(&mut self, app: &App, store: &Storage) -> bool {
        let Some(public) = store.committed() else {
            return false;
        };
        self.meta = Meta {
            generation: public.generation,
            epoch: public.checkpoint_epoch,
            nonce: public.transaction_session,
        };
        // SAFETY: no job exists. Frozen App and byte buffers are exclusively owned distinct regions;
        // a job borrows them immutably/mutably until dropped, and abort() drops it before any reuse.
        unsafe {
            let frozen = &mut *self.regions.frozen;
            if app.capture_clone_into(frozen).is_err() {
                return false;
            }
            core::slice::from_raw_parts_mut(self.regions.public, SIZE)
                .copy_from_slice(public.packet);
            let out =
                core::slice::from_raw_parts_mut(self.regions.private, resume_private::MAX_BYTES);
            match CaptureTask::new(&*self.regions.frozen, out, self.meta.epoch, true) {
                Ok(task) => self.job = Some(Job::Capture(task)),
                Err(_) => return false,
            }
        }
        self.captured_revision = app.checkpoint_revision();
        true
    }
    pub fn step(&mut self, app: &App, store: &mut Storage, now: u32) {
        if self.ram_only || self.blocked || self.choosing() || self.migration_pending {
            return;
        }
        if self.job.is_none() {
            if !app.resume_enabled
                || self.journal.is_none()
                || self.private_io_failed
                || self.done_revision == Some(app.checkpoint_revision())
                || now.wrapping_sub(self.changed) < 1200
            {
                return;
            }
            if !self.start(app, store) {
                self.errors += 1;
                return;
            }
        }
        let Some(job) = self.job.take() else { return };
        match job {
            Job::Capture(mut task) => match task.step(512) {
                Ok(CaptureProgress::More) => self.job = Some(Job::Capture(task)),
                Ok(CaptureProgress::Done { bytes }) => {
                    drop(task);
                    self.private_len = bytes;
                    // SAFETY: capture ended; private bytes are immutable and bound output is a separate region.
                    let result = unsafe {
                        let public = PublicCommit {
                            generation: self.meta.generation,
                            checkpoint_epoch: self.meta.epoch,
                            transaction_session: self.meta.nonce,
                            packet: core::slice::from_raw_parts(self.regions.public, SIZE),
                        };
                        BindTask::new(
                            core::slice::from_raw_parts_mut(
                                self.regions.bound,
                                resume_private::MAX_BYTES,
                            ),
                            public,
                            core::slice::from_raw_parts(self.regions.private, bytes),
                        )
                    };
                    match result {
                        Ok(task) => self.job = Some(Job::Bind(task)),
                        Err(_) => self.errors += 1,
                    }
                }
                Err(_) => self.errors += 1,
            },
            Job::Bind(mut task) => match task.step(512) {
                Ok(BindProgress::More) => self.job = Some(Job::Bind(task)),
                Ok(BindProgress::Done { bytes }) => {
                    drop(task);
                    self.bound_len = bytes;
                    if let Some(j) = &self.journal {
                        // SAFETY: binding ended; the bound packet remains immutable until the write job ends.
                        let payload =
                            unsafe { core::slice::from_raw_parts(self.regions.bound, bytes) };
                        match CommitTask::new(
                            j,
                            self.previous,
                            app.resume_session_id(),
                            payload,
                            true,
                        ) {
                            Ok(task) => self.job = Some(Job::Write(task)),
                            Err(_) => self.errors += 1,
                        }
                    }
                }
                Err(_) => self.errors += 1,
            },
            Job::Write(mut task) => {
                let Some(mut flash) = store.private_flash() else {
                    self.private_io_failed = true;
                    self.errors += 1;
                    return;
                };
                match task.step(&mut flash) {
                    Ok(Progress::More) => self.job = Some(Job::Write(task)),
                    Ok(Progress::Committed(record)) => {
                        self.previous = Some(record);
                        self.done_revision = if record.tombstone {
                            None
                        } else {
                            Some(self.captured_revision)
                        };
                        self.writes += 1
                    }
                    Err(_) => {
                        self.private_io_failed = true;
                        self.errors += 1
                    }
                }
            }
        }
    }
}
