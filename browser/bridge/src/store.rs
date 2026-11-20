//! Internal opaque storage transport. Not part of the page's public game/debug API.
//! All slices have fixed bounds. Tasks exclusively own disjoint static buffers;
//! JS cannot re-enter an export and no exported function invokes foreign code.
use super::app;
use journal::{Identity, Kind, Record, Slot};
use stardust_blackjack::{
    app::App,
    controls::Controls,
    persistence::{self, Save},
    resume_binding::{self, BindProgress, BindTask, PublicCommit},
    resume_private::{self, CaptureProgress, CaptureTask, NeutralGate},
    resume_ui::{BootChooser, BootState, Command, Preference},
};
use stardust_browser_journal as journal;
const PUBLIC_CAP: usize = persistence::SIZE + journal::HEADER + journal::FOOTER;
const CAP: usize = resume_private::MAX_BYTES + journal::HEADER + journal::FOOTER;
const ZERO: Identity = Identity {
    generation: 0,
    epoch: 0,
    nonce: [0; 16],
};
struct Slots {
    data: [[u8; CAP]; 2],
    len: [usize; 2],
    state: [u8; 2],
}
impl Slots {
    const EMPTY: Self = Self {
        data: [[0; CAP]; 2],
        len: [0; 2],
        state: [0; 2],
    };
    fn slot(&self, i: usize) -> Slot<'_> {
        match self.state[i] {
            0 => Slot::Missing,
            1 => Slot::Bytes(&self.data[i][..self.len[i]]),
            2 => Slot::Io,
            _ => Slot::Bytes(&[]),
        }
    }
    fn latest(&self, public: bool) -> journal::Result<Option<(usize, Record<'_>)>> {
        journal::latest(self.slot(0), self.slot(1), public)
    }
    fn clear(&mut self) {
        self.len = [0; 2];
        self.state = [0; 2];
    }
    fn load(&mut self, i: usize, status: u32, b: &[u8]) {
        if i >= 2 {
            return;
        }
        self.len[i] = 0;
        self.state[i] = status as u8;
        if status == 1 && b.len() <= CAP {
            self.data[i][..b.len()].copy_from_slice(b);
            self.len[i] = b.len()
        } else if status == 1 {
            self.state[i] = 3
        }
    }
}
struct State {
    public: Slots,
    private: Slots,
    verify: Slots,
    verify_private: Slots,
    raw: [u8; persistence::SIZE],
    rawlen: usize,
    id: Identity,
    base: bool,
    pubslot: usize,
    privslot: usize,
    legacy: [u8; persistence::SIZE],
    legacy_len: usize,
    pending_raw: [u8; persistence::SIZE],
    pending_id: Identity,
    pending_slot: usize,
    token: u32,
    prepared: bool,
    uncertain_write: bool,
    revision: u64,
    session: u64,
    frozen_revision: u64,
    frozen_session: u64,
    force: bool,
    chooser: Option<BootChooser>,
    controls: Controls,
    neutral: Option<NeutralGate>,
    command: u32,
    capture_ready: bool,
    private_len: usize,
    blocked: bool,
    seed: u64,
    game_clock: u32,
    output_blocked: bool,
    wait_neutral: bool,
    ram_only: bool,
    error_row: u8,
    error_neutral: bool,
    error_visible: bool,
}
impl State {
    const EMPTY: Self = Self {
        public: Slots::EMPTY,
        private: Slots::EMPTY,
        verify: Slots::EMPTY,
        verify_private: Slots::EMPTY,
        raw: [0; persistence::SIZE],
        rawlen: 0,
        id: ZERO,
        base: false,
        pubslot: 1,
        privslot: 1,
        legacy: [0; persistence::SIZE],
        legacy_len: 0,
        pending_raw: [0; persistence::SIZE],
        pending_id: ZERO,
        pending_slot: 0,
        token: 0,
        prepared: false,
        uncertain_write: false,
        revision: 0,
        session: 0,
        frozen_revision: 0,
        frozen_session: 0,
        force: false,
        chooser: None,
        controls: Controls::EMPTY,
        neutral: None,
        command: 0,
        capture_ready: false,
        private_len: 0,
        blocked: false,
        seed: 1,
        game_clock: 0,
        output_blocked: false,
        wait_neutral: false,
        ram_only: false,
        error_row: 0,
        error_neutral: false,
        error_visible: false,
    };
}
#[derive(Clone, Copy)]
pub struct PublicStatus {
    pub phase: u32,
    pub mode: u32,
    pub rounds: u32,
    pub chips: i64,
    pub wager: i64,
    pub brightness: u32,
    pub flipped: u32,
    pub sound: u32,
    pub audio_volume: u32,
    pub audio_mask: u32,
    pub fast_mode: u32,
}
static mut ACK_FRAME: [u16; 32400] = [0; 32400];
static mut ACK: PublicStatus = PublicStatus {
    phase: 0,
    mode: 0,
    rounds: 0,
    chips: 100000,
    wager: 500,
    brightness: 255,
    flipped: 0,
    sound: 1,
    audio_volume: 20,
    audio_mask: 7,
    fast_mode: 1,
};
fn acknowledge() {
    let a = app();
    let snapshot = PublicStatus {
        phase: a.game.phase as u32,
        mode: a.training.mode as u32,
        rounds: a.game.rounds,
        chips: a.game.bankroll,
        wager: a.game.selected_bet,
        brightness: a.brightness as u32,
        flipped: u32::from(a.flipped),
        sound: u32::from(a.sound_enabled),
        audio_volume: a.audio_volume as u32,
        audio_mask: a.audio_mask as u32,
        fast_mode: u32::from(a.fast_mode),
    };
    unsafe {
        *core::ptr::addr_of_mut!(ACK) = snapshot;
        (&mut *core::ptr::addr_of_mut!(ACK_FRAME)).copy_from_slice(&a.frame.pixels);
    }
}
pub fn acknowledged() -> Option<PublicStatus> {
    if output_pending() {
        Some(unsafe { *core::ptr::addr_of!(ACK) })
    } else {
        None
    }
}
static mut STATE: State = State::EMPTY;
static mut IO: [u8; CAP] = [0; CAP];
static mut FROZEN: App = App::EMPTY;
static mut CANDIDATE: App = App::EMPTY;
static mut CAPTURE: [u8; resume_private::MAX_BYTES] = [0; resume_private::MAX_BYTES];
static mut BIND_PUBLIC: [u8; persistence::SIZE] = [0; persistence::SIZE];
static mut BOUND: [u8; resume_private::MAX_BYTES] = [0; resume_private::MAX_BYTES];
static mut CAPTURE_TASK: Option<CaptureTask<'static>> = None;
static mut BIND_TASK: Option<BindTask<'static>> = None;
fn state() -> &'static mut State {
    unsafe { &mut *core::ptr::addr_of_mut!(STATE) }
}
fn io() -> &'static mut [u8; CAP] {
    unsafe { &mut *core::ptr::addr_of_mut!(IO) }
}
fn frozen() -> &'static mut App {
    unsafe { &mut *core::ptr::addr_of_mut!(FROZEN) }
}
fn candidate() -> &'static mut App {
    unsafe { &mut *core::ptr::addr_of_mut!(CANDIDATE) }
}
fn cancel() {
    unsafe {
        *core::ptr::addr_of_mut!(CAPTURE_TASK) = None;
        *core::ptr::addr_of_mut!(BIND_TASK) = None;
    }
    let s = state();
    s.capture_ready = false;
    s.private_len = 0;
}
fn commit<'a>(id: Identity, packet: &'a [u8]) -> PublicCommit<'a> {
    PublicCommit {
        generation: id.generation,
        checkpoint_epoch: id.epoch,
        transaction_session: id.nonce,
        packet,
    }
}
fn reset_candidate() {
    unsafe { core::ptr::addr_of_mut!(CANDIDATE).write(App::EMPTY) }
}
pub fn boot(seed: u64) {
    cancel();
    let s = state();
    s.seed = seed;
    s.public.clear();
    s.private.clear();
    s.verify.clear();
    s.verify_private.clear();
    s.rawlen = 0;
    s.legacy_len = 0;
    s.id = ZERO;
    s.base = false;
    s.pubslot = 1;
    s.privslot = 1;
    s.prepared = false;
    s.uncertain_write = false;
    s.token = 0;
    s.game_clock = 0;
    s.chooser = None;
    s.neutral = None;
    s.controls = Controls::EMPTY;
    s.command = 0;
    s.blocked = false;
    s.output_blocked = false;
    s.wait_neutral = false;
    s.ram_only = false;
    s.error_visible = false;
    s.force = false;
    s.revision = app().checkpoint_revision();
    s.session = app().resume_session_id();
    acknowledge();
}
pub fn buttons(mask: u32, now: u32) -> bool {
    let down = [mask & 1 != 0, mask & 2 != 0];
    let s = state();
    if s.output_blocked {
        if !s.error_neutral {
            if down == [false; 2] {
                s.error_neutral = true;
            }
            return false;
        }
        for event in s.controls.update(down, now, false).into_iter().flatten() {
            match event {
                stardust_blackjack::controls::Event::Tap(
                    stardust_blackjack::controls::Key::Grey,
                ) => s.error_row = 1 - s.error_row,
                stardust_blackjack::controls::Event::Tap(
                    stardust_blackjack::controls::Key::Blue,
                ) => s.command = if s.error_row == 0 { 4 } else { 5 },
                _ => {}
            }
        }
        return false;
    }
    if s.wait_neutral {
        if down == [false; 2] {
            s.wait_neutral = false;
        }
        return false;
    }
    if let Some(c) = s.chooser.as_mut() {
        if c.neutral(down) {
            for e in s.controls.update(down, now, false).into_iter().flatten() {
                s.command = match c.event(e) {
                    Command::None => s.command,
                    Command::RetryRead => 1,
                    Command::RestoreCandidate => 2,
                    Command::CommitNewSessionTombstone => 3,
                };
            }
        }
        return false;
    }
    if let Some(g) = s.neutral.as_mut() {
        if !g.accept(down) {
            return false;
        }
        s.neutral = None;
        app().cancel_buttons();
    }
    let game_now = clamp_clock(s, now);
    app().tick(game_now);
    let submit = app().view_name() == "CountAnswer" && app().count_practice.awaiting_answer();
    for event in s.controls.update(down, now, submit).into_iter().flatten() {
        app().event(event);
        if s.ram_only {
            app().resume_enabled = false;
        }
    }
    false
}
fn clamp_clock(s: &mut State, now: u32) -> u32 {
    if now.wrapping_sub(s.game_clock) < (1u32 << 31) {
        s.game_clock = now;
    }
    s.game_clock
}
pub fn tick(now: u32) {
    if can_tick() {
        let time = clamp_clock(state(), now);
        app().tick(time);
    }
}
pub fn can_tick() -> bool {
    let s = state();
    s.chooser.is_none() && s.neutral.is_none() && !s.output_blocked && !s.wait_neutral
}
pub fn render() -> bool {
    if state().ram_only {
        app().resume_enabled = false;
    }
    if state().output_blocked {
        app().render_resume_storage_error_choice(state().error_row);
        return true;
    }
    if let Some(c) = state().chooser.as_ref() {
        app().render_resume_chooser(c);
        true
    } else {
        false
    }
}
pub fn cancel_buttons() {
    state().controls = Controls::EMPTY;
}
pub fn view() -> &'static str {
    if state().output_blocked {
        return "StorageRetry";
    }
    if output_pending() {
        return "Saving";
    }
    match state().chooser.as_ref().map(|c| c.state) {
        Some(BootState::ReadFailure { .. }) => "ResumeReadFailure",
        Some(_) => "ResumeChooser",
        None => app().view_name(),
    }
}
pub fn item() -> &'static str {
    if state().output_blocked {
        return if state().error_row == 0 {
            "Retry saving session"
        } else {
            "Start fresh without saving"
        };
    }
    if output_pending() {
        return "";
    }
    if let Some(c) = state().chooser.as_ref() {
        c.rows()[match c.state {
            BootState::ChooseSaved { row } | BootState::ReadFailure { row } => row as usize,
            _ => 0,
        }]
    } else {
        app().menu_item()
    }
}
pub fn focus() -> u32 {
    if state().output_blocked {
        return state().error_row as u32;
    }
    if output_pending() {
        return 0;
    }
    if let Some(c) = state().chooser.as_ref() {
        match c.state {
            BootState::ChooseSaved { row } | BootState::ReadFailure { row } => row as u32,
            _ => 0,
        }
    } else {
        app().focus() as u32
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_buffer() -> *mut u8 {
    core::ptr::addr_of_mut!(IO).cast::<u8>()
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_capacity(kind: u32) -> u32 {
    if kind == 0 {
        PUBLIC_CAP as u32
    } else {
        CAP as u32
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_begin() {
    cancel();
    let s = state();
    s.public.clear();
    s.private.clear();
    s.legacy_len = 0;
    s.chooser = None;
    s.command = 0;
    s.controls = Controls::EMPTY;
    s.blocked = false;
    s.output_blocked = false;
    s.ram_only = false;
}
// status: 0 missing,1 bytes,2 read I/O,3 corrupt. kind: public/private/verify-public/verify-private.
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_load(kind: u32, slot: u32, status: u32, len: u32) {
    let b = if len as usize <= CAP {
        &io()[..len as usize]
    } else {
        &[]
    };
    let s = state();
    let target = match kind {
        0 => &mut s.public,
        1 => &mut s.private,
        2 => &mut s.verify,
        3 => &mut s.verify_private,
        _ => return,
    };
    target.load(
        slot as usize,
        if len as usize > CAP { 3 } else { status },
        b,
    );
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_legacy(len: u32) -> u32 {
    let s = state();
    if len as usize > persistence::SIZE || Save::decode(&io()[..len as usize]).is_none() {
        return 0;
    }
    s.legacy[..len as usize].copy_from_slice(&io()[..len as usize]);
    s.legacy_len = len as usize;
    1
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_finish_public() -> u32 {
    let s = state();
    let result = journal::recover_public(s.public.slot(0), s.public.slot(1));
    let Ok((record, tainted)) = result else {
        return 0;
    };
    let mut raw = [0; persistence::SIZE];
    let (id, len, slot, base) = if let Some((slot, r)) = record {
        let Some(save) = Save::decode(r.payload) else {
            return 0;
        };
        if save.resume_enabled != r.resume_enabled || r.payload.len() > raw.len() {
            return 0;
        }
        raw[..r.payload.len()].copy_from_slice(r.payload);
        (r.identity, r.payload.len(), slot, true)
    } else {
        raw[..s.legacy_len].copy_from_slice(&s.legacy[..s.legacy_len]);
        (ZERO, s.legacy_len, 1, false)
    };
    unsafe {
        super::replace_app(App::new(s.seed));
    }
    if len > 0 {
        let Some(save) = Save::decode(&raw[..len]) else {
            return 0;
        };
        if !app().load(save) {
            return 0;
        }
    }
    app().render();
    s.raw = raw;
    s.rawlen = len;
    s.id = id;
    s.pubslot = slot;
    s.base = base;
    s.revision = app().checkpoint_revision();
    s.session = app().resume_session_id();
    s.force = !base && len > 0;
    acknowledge();
    if tainted {
        s.blocked = true;
        0
    } else {
        1
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_wants_private() -> u32 {
    u32::from(app().resume_enabled)
}
fn valid_private(now: u32) -> bool {
    let s = state();
    if !s.base || !app().resume_enabled {
        return false;
    }
    let Ok(Some((slot, r))) = s.private.latest(false) else {
        return false;
    };
    if r.kind != Kind::Private || r.identity != s.id {
        return false;
    }
    reset_candidate();
    let okay = resume_binding::restore_bound(
        candidate(),
        r.payload,
        commit(s.id, &s.raw[..s.rawlen]),
        now,
        true,
    )
    .is_ok();
    if okay {
        s.privslot = slot
    }
    okay
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_finish_boot(now: u32) -> u32 {
    let valid = valid_private(now);
    let s = state();
    s.chooser = if valid {
        Some(BootChooser::new(
            Preference::from_enabled(true),
            true,
            false,
        ))
    } else {
        None
    };
    s.controls = Controls::EMPTY;
    if valid { 1 } else { 0 }
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_read_failure() {
    cancel();
    let s = state();
    s.blocked = true;
    s.chooser = Some(BootChooser::new(
        Preference::from_enabled(true),
        false,
        true,
    ));
    s.controls = Controls::EMPTY;
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_verify_begin() {
    let s = state();
    s.verify.clear();
    s.verify_private.clear();
}
fn base_matches(s: &State) -> bool {
    match s.verify.latest(true) {
        Ok(Some((_, r))) => {
            s.base
                && Save::decode(r.payload).is_some()
                && journal::same_public(r, s.id, &s.raw[..s.rawlen])
        }
        Ok(None) => !s.base,
        Err(_) => false,
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_base_matches() -> u32 {
    u32::from(base_matches(state()))
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_needs() -> u32 {
    let s = state();
    if s.blocked || s.chooser.is_some() {
        return 0;
    }
    let Some(raw) = app().save().encode() else {
        return 0;
    };
    u32::from(
        s.force
            || s.rawlen != raw.len()
            || s.raw != raw
            || app().resume_enabled && s.revision != app().checkpoint_revision(),
    )
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_prepare(new_session: u32) -> u32 {
    cancel();
    let s = state();
    if s.blocked || s.chooser.is_some() {
        return 0;
    }
    let mut nonce = [0; 16];
    nonce.copy_from_slice(&io()[..16]);
    let rotate = new_session != 0 || s.session != app().resume_session_id();
    let Ok(id) = s.id.next(nonce, rotate) else {
        return 0;
    };
    let Some(raw) = app().save().encode() else {
        return 0;
    };
    if app().capture_clone_into(frozen()).is_err() {
        return 0;
    }
    s.token = s.token.wrapping_add(1).max(1);
    s.pending_raw = raw;
    s.pending_id = id;
    s.pending_slot = 1 - s.pubslot;
    s.prepared = true;
    s.frozen_revision = app().checkpoint_revision();
    s.frozen_session = app().resume_session_id();
    journal::encode(
        io(),
        Record {
            identity: id,
            kind: Kind::Public,
            payload: &s.pending_raw,
            resume_enabled: app().resume_enabled,
        },
    )
    .map_or(0, |n| n as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_token() -> u32 {
    state().token
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_plan_slot() -> u32 {
    state().pending_slot as u32
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_public_committed(token: u32, len: u32) -> u32 {
    let s = state();
    if token != s.token || !s.prepared || len as usize > PUBLIC_CAP {
        return 0;
    }
    let Ok(r) = journal::decode(&io()[..len as usize], true) else {
        return 0;
    };
    if !journal::same_public(r, s.pending_id, &s.pending_raw)
        || r.resume_enabled != frozen().resume_enabled
    {
        return 0;
    }
    s.raw = s.pending_raw;
    s.rawlen = persistence::SIZE;
    s.id = s.pending_id;
    s.base = true;
    s.pubslot = s.pending_slot;
    s.revision = s.frozen_revision;
    s.session = s.frozen_session;
    s.force = false;
    s.prepared = false;
    s.uncertain_write = false;
    s.error_visible = false;
    acknowledge();
    if frozen().resume_enabled {
        let output = unsafe { &mut *core::ptr::addr_of_mut!(CAPTURE) };
        let source = unsafe { &*core::ptr::addr_of!(FROZEN) };
        if let Ok(t) = CaptureTask::new(source, output, s.id.epoch, true) {
            unsafe { *core::ptr::addr_of_mut!(CAPTURE_TASK) = Some(t) }
        }
    }
    1
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_public_failed() {
    cancel();
    let s = state();
    s.uncertain_write = s.prepared;
    s.prepared = false;
    s.blocked = true;
    let bound = app().resume_enabled
        || Save::decode(&s.raw[..s.rawlen]).is_some_and(|save| save.resume_enabled);
    s.output_blocked = bound;
    s.wait_neutral = bound;
    // A background retry is the same visible failure, not a new dialog. Keep
    // the user's choice and button neutral gate while they read or confirm it.
    if !s.error_visible {
        s.controls = Controls::EMPTY;
        s.error_row = 0;
        s.error_neutral = false;
    }
    s.error_visible = bound;
    if bound {
        app().render_resume_storage_error_choice(state().error_row);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_capture_pending() -> u32 {
    unsafe {
        u32::from(
            (*core::ptr::addr_of!(CAPTURE_TASK)).is_some()
                || (*core::ptr::addr_of!(BIND_TASK)).is_some()
                || state().capture_ready,
        )
    }
}
// 0 idle,1 more,2 ready,3 obsolete/failure. Every core task step processes at most512 bytes.
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_step(budget: u32) -> u32 {
    let s = state();
    if s.blocked
        || s.revision != app().checkpoint_revision()
        || s.raw != app().save().encode().unwrap_or([0; persistence::SIZE])
    {
        cancel();
        return 3;
    }
    unsafe {
        if let Some(mut t) = (*core::ptr::addr_of_mut!(CAPTURE_TASK)).take() {
            match t.step(budget as usize) {
                Ok(CaptureProgress::More) => {
                    *core::ptr::addr_of_mut!(CAPTURE_TASK) = Some(t);
                    return 1;
                }
                Ok(CaptureProgress::Done { bytes }) => {
                    let out = &mut *core::ptr::addr_of_mut!(BOUND);
                    let private = &(&*core::ptr::addr_of!(CAPTURE))[..bytes];
                    core::ptr::addr_of_mut!(BIND_PUBLIC).write(s.raw);
                    let packet = &*core::ptr::addr_of!(BIND_PUBLIC);
                    match BindTask::new(out, commit(s.id, packet), private) {
                        Ok(task) => *core::ptr::addr_of_mut!(BIND_TASK) = Some(task),
                        Err(_) => return 3,
                    }
                }
                Err(_) => return 3,
            }
        }
        if let Some(mut t) = (*core::ptr::addr_of_mut!(BIND_TASK)).take() {
            match t.step(budget as usize) {
                Ok(BindProgress::More) => {
                    *core::ptr::addr_of_mut!(BIND_TASK) = Some(t);
                    return 1;
                }
                Ok(BindProgress::Done { bytes }) => {
                    s.private_len = bytes;
                    s.capture_ready = true;
                    return 2;
                }
                Err(_) => return 3,
            }
        }
    }
    if s.capture_ready { 2 } else { 0 }
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_private_plan() -> u32 {
    let s = state();
    if !s.capture_ready {
        return 0;
    }
    let b = unsafe { &(&*core::ptr::addr_of!(BOUND))[..s.private_len] };
    journal::encode(
        io(),
        Record {
            identity: s.id,
            kind: Kind::Private,
            payload: b,
            resume_enabled: true,
        },
    )
    .map_or(0, |n| n as u32)
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_private_slot() -> u32 {
    (1 - state().privslot) as u32
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_private_current(token: u32) -> u32 {
    let s = state();
    u32::from(
        token == s.token
            && s.capture_ready
            && base_matches(s)
            && s.revision == app().checkpoint_revision()
            && s.verify_private.latest(false).is_ok(),
    )
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_private_committed(token: u32, len: u32) -> u32 {
    let s = state();
    if token != s.token || !s.capture_ready || len as usize > CAP {
        return 0;
    }
    let Ok(r) = journal::decode(&io()[..len as usize], false) else {
        return 0;
    };
    let bound = unsafe { &(&*core::ptr::addr_of!(BOUND))[..s.private_len] };
    if r.identity != s.id || r.kind != Kind::Private || r.payload != bound {
        return 0;
    }
    s.privslot = 1 - s.privslot;
    cancel();
    1
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_private_failed() {
    cancel();
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_command() -> u32 {
    let s = state();
    let c = s.command;
    s.command = 0;
    c
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_start_new() {
    cancel();
    let s = state();
    let raw = s.raw;
    let len = s.rawlen;
    unsafe {
        super::replace_app(App::new(s.seed));
    }
    if len > 0
        && let Some(save) = Save::decode(&raw[..len])
    {
        app().load(save);
    }
    app().render();
    s.chooser = None;
    s.neutral = None;
    s.controls = Controls::EMPTY;
    s.force = true;
    s.command = 0;
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_start_unsaved() {
    sb_store_start_new();
    app().resume_enabled = false;
    let s = state();
    s.blocked = true;
    s.ram_only = true;
    s.error_visible = false;
    s.output_blocked = false;
    s.wait_neutral = true;
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_restore(now: u32) -> u32 {
    let s = state();
    if !base_matches(s) {
        let Ok(Some((slot, record))) = s.verify.latest(true) else {
            return 0;
        };
        let Some(save) = Save::decode(record.payload) else {
            return 0;
        };
        if record.payload.len() > persistence::SIZE || record.resume_enabled != save.resume_enabled
        {
            return 0;
        }
        let mut raw = [0; persistence::SIZE];
        raw[..record.payload.len()].copy_from_slice(record.payload);
        let id = record.identity;
        let len = record.payload.len();
        unsafe {
            super::replace_app(App::new(s.seed));
        }
        if !app().load(save) {
            return 0;
        }
        app().render();
        s.raw = raw;
        s.rawlen = len;
        s.id = id;
        s.pubslot = slot;
        s.base = true;
        s.revision = app().checkpoint_revision();
        s.session = app().resume_session_id();
        acknowledge();
    }
    if !app().resume_enabled {
        return 0;
    }
    let Ok(Some((slot, r))) = s.verify_private.latest(false) else {
        return 0;
    };
    if r.identity != s.id || r.kind != Kind::Private {
        return 0;
    }
    reset_candidate();
    let Ok(restored) = resume_binding::restore_bound(
        candidate(),
        r.payload,
        commit(s.id, &s.raw[..s.rawlen]),
        now,
        true,
    ) else {
        return 0;
    };
    if candidate().capture_clone_into(app()).is_err() {
        return 0;
    }
    app().render();
    s.chooser = None;
    s.controls = Controls::EMPTY;
    s.game_clock = now;
    s.neutral = Some(restored.neutral);
    s.privslot = slot;
    s.revision = app().checkpoint_revision();
    s.session = app().resume_session_id();
    s.command = 0;
    acknowledge();
    1
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_refuse_resume() {
    let s = state();
    s.blocked = true;
    s.chooser = Some(BootChooser::new(
        Preference::from_enabled(true),
        false,
        true,
    ));
    s.controls = Controls::EMPTY;
}

// Called only after public readback and before pixels/audio can be presented.
// Storage wait time is excluded from a never-presented face/review dwell.
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_commit_clock(now: u32) {
    let s = state();
    let clock = clamp_clock(s, now);
    app().checkpoint_output_clock(clock);
}

#[unsafe(no_mangle)]
pub extern "C" fn sb_store_output_blocked() -> u32 {
    u32::from(state().output_blocked)
}
// 1 unchanged verified base (retry current RAM transition);2 uncertain write is now verified.
// Foreign public progress/corruption is refused, preserving both journals unchanged.
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_recover() -> u32 {
    let s = state();
    if !s.output_blocked {
        return 0;
    }
    let Ok(latest) = s.verify.latest(true) else {
        return 0;
    };
    if let Some((_, r)) = latest
        && s.uncertain_write
        && journal::same_public(r, s.pending_id, &s.pending_raw)
        && Save::decode(r.payload).is_some()
    {
        let Ok(n) = journal::encode(io(), r) else {
            return 0;
        };
        s.prepared = true;
        if sb_store_public_committed(s.token, n as u32) == 0 {
            return 0;
        }
        state().blocked = false;
        state().output_blocked = false;
        return 2;
    }
    if base_matches(s) {
        s.blocked = false;
        s.output_blocked = false;
        s.force = true;
        1
    } else {
        0
    }
}

pub fn acknowledged_packet() -> Option<[u8; persistence::SIZE]> {
    let s = state();
    let save = Save::decode(&s.raw[..s.rawlen])?;
    // Preserve same-sized legacy bytes under their acknowledged identity until
    // an explicit canonical migration has actually committed.
    if s.rawlen == persistence::SIZE { Some(s.raw) } else { save.encode() }
}

// Computed before the asynchronous origin-lock wait: never expose a new protected
// grade through status, PCM, Save, or RGB565 transport while ACK is pending.
pub fn output_pending() -> bool {
    let s = state();
    if s.ram_only {
        return false;
    }
    if s.output_blocked {
        return true;
    }
    if s.chooser.is_some() {
        return false;
    }
    let protected = app().resume_enabled
        || Save::decode(&s.raw[..s.rawlen]).is_some_and(|save| save.resume_enabled);
    protected
        && (s.force
            || s.revision != app().checkpoint_revision()
            || app()
                .save()
                .encode()
                .is_none_or(|raw| s.rawlen != raw.len() || s.raw != raw))
}
pub fn frame() -> *const u16 {
    if output_pending() && !state().output_blocked {
        core::ptr::addr_of!(ACK_FRAME).cast::<u16>()
    } else {
        app().frame.pixels.as_ptr()
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn sb_store_output_pending() -> u32 {
    u32::from(output_pending())
}

pub fn rendered_frame() {
    if state().ram_only {
        app().render_unsaved_banner();
    }
    if !output_pending() {
        unsafe {
            (&mut *core::ptr::addr_of_mut!(ACK_FRAME)).copy_from_slice(&app().frame.pixels);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sb_store_fresh_seed() {
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&io()[..8]);
    state().seed = u64::from_le_bytes(bytes);
}

#[unsafe(no_mangle)]
pub extern "C" fn sb_store_public_known() -> u32 {
    let s = state();
    u32::from(Save::decode(&s.raw[..s.rawlen]).is_some())
}
