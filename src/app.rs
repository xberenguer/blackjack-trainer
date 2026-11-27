//! Shared allocation-free controller for the physical display and WASM frontend.
//! It receives only button events. All advice snapshots contain exposed faces.
use crate::{
    audio::{Cue, SoundQueue},
    canvas::Canvas,
    controls::{Controls, Event, Key},
    count_practice::{self, Answer, Kind, Question, Snapshot},
    counting::Tracker,
    deviations,
    engine::{Action, Engine, Phase},
    ev,
    persistence::{self, Save},
    render::Renderer,
    rules::{Profile, Rules},
    split_ev, table,
    training::{self, Choice, Decision, ExpectedValues, Mode, Policy},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum View {
    AnalysisMenu,
    AnalysisSummary,
    AnalysisGraph,
    AnalysisRounds,
    AnalysisAccuracy,
    AnalysisCosts,
    AnalysisDecisions,
    AnalysisAssumptions,
    DeviationPractice,
    DeviationTarget,
    DeviationQuestion,
    DeviationContext,
    DeviationReference,
    DeviationResult,
    DeviationStats,
    DeviationHistory,
    DeviationPolicy,
    TableSeats,
    SeatCards,
    WagerPractice,
    WagerAnswer,
    WagerHelp,
    WagerResult,
    WagerStats,
    WagerHistory,
    WagerPolicy,
    Cut,
    Burn,
    Table,
    Menu,
    Wager,
    Amount,
    Training,
    TrainingStats,
    TrainingReview,
    RoundReview,
    Hint,
    CoachCheck,
    ActionEv,
    EvReview,
    CountPractice,
    CountAnswer,
    CountResult,
    CountStats,
    CountHistory,
    CountInfo,
    Settings,
    Profile,
    ProfilePreset,
    ProfileAmount,
    ProfileDepth,
    ShoeInfo,
    Rules,
    Session,
    Bankroll,
    Cards,
    Controls,
    Leave,
    Summary,
    Refill,
    Notice,
    Sound,
    AudioVolume,
    Display,
    Brightness,
    TableMenu,
    SessionMenu,
    SessionOverview,
    BankrollMenu,
    NewSessionConfirm,
    ResetStatsConfirm,
    HelpMenu,
    TrainingModes,
    LiveCounts,
    PracticeMenu,
    ProgressMenu,
    AdviceMenu,
    SoundCategories,
    SoundTests,
    HandProgress,
    CountProgress,
    WagerProgress,
    DeviationProgress,
    CardBackPreview,
}
impl View {
    pub fn name(self) -> &'static str {
        match self {
            Self::AnalysisMenu => "AnalysisMenu",
            Self::AnalysisSummary => "AnalysisSummary",
            Self::AnalysisGraph => "AnalysisGraph",
            Self::AnalysisRounds => "AnalysisRounds",
            Self::AnalysisAccuracy => "AnalysisAccuracy",
            Self::AnalysisCosts => "AnalysisCosts",
            Self::AnalysisDecisions => "AnalysisDecisions",
            Self::AnalysisAssumptions => "AnalysisAssumptions",
            Self::DeviationPractice => "DeviationPractice",
            Self::DeviationTarget => "DeviationTarget",
            Self::DeviationQuestion => "DeviationQuestion",
            Self::DeviationContext => "DeviationContext",
            Self::DeviationReference => "DeviationReference",
            Self::DeviationResult => "DeviationResult",
            Self::DeviationStats => "DeviationStats",
            Self::DeviationHistory => "DeviationHistory",
            Self::DeviationPolicy => "DeviationPolicy",
            Self::TableSeats => "TableSeats",
            Self::SeatCards => "SeatCards",
            Self::WagerPractice => "WagerPractice",
            Self::WagerAnswer => "WagerAnswer",
            Self::WagerHelp => "WagerHelp",
            Self::WagerResult => "WagerResult",
            Self::WagerStats => "WagerStats",
            Self::WagerHistory => "WagerHistory",
            Self::WagerPolicy => "WagerPolicy",
            Self::Cut => "Cut",
            Self::Burn => "Burn",
            Self::Table => "Table",
            Self::Menu => "Menu",
            Self::Wager => "Wager",
            Self::Amount => "Amount",
            Self::Training => "Training",
            Self::TrainingStats => "TrainingStats",
            Self::TrainingReview => "TrainingReview",
            Self::RoundReview => "RoundReview",
            Self::Hint => "TrainingHint",
            Self::CoachCheck => "CoachCheck",
            Self::ActionEv => "ActionEV",
            Self::EvReview => "EVReview",
            Self::CountPractice => "CountPractice",
            Self::CountAnswer => "CountAnswer",
            Self::CountResult => "CountResult",
            Self::CountStats => "CountStats",
            Self::CountHistory => "CountHistory",
            Self::CountInfo => "CountInfo",
            Self::Settings => "Settings",
            Self::Profile => "TableProfile",
            Self::ProfilePreset => "ProfilePreset",
            Self::ProfileAmount => "ProfileAmount",
            Self::ProfileDepth => "ProfileDepth",
            Self::ShoeInfo => "ShoeInfo",
            Self::Rules => "Rules",
            Self::Session => "Session",
            Self::Bankroll => "Bankroll",
            Self::Cards => "Cards",
            Self::Controls => "Controls",
            Self::Leave => "Leave",
            Self::Summary => "Summary",
            Self::Refill => "Refill",
            Self::Notice => "Notice",
            Self::Sound => "Sound",
            Self::AudioVolume => "AudioVolume",
            Self::Display => "Display",
            Self::Brightness => "Brightness",
            Self::TableMenu => "TableMenu",
            Self::SessionMenu => "SessionMenu",
            Self::SessionOverview => "SessionOverview",
            Self::BankrollMenu => "BankrollMenu",
            Self::NewSessionConfirm => "NewSessionConfirm",
            Self::ResetStatsConfirm => "ResetStatsConfirm",
            Self::HelpMenu => "HelpMenu",
            Self::TrainingModes => "TrainingModes",
            Self::LiveCounts => "LiveCounts",
            Self::PracticeMenu => "PracticeMenu",
            Self::ProgressMenu => "ProgressMenu",
            Self::AdviceMenu => "AdviceMenu",
            Self::SoundCategories => "SoundCategories",
            Self::SoundTests => "SoundTests",
            Self::HandProgress => "HandProgress",
            Self::CountProgress => "CountProgress",
            Self::WagerProgress => "WagerProgress",
            Self::DeviationProgress => "DeviationProgress",
            Self::CardBackPreview => "CardBackPreview",
        }
    }
}
pub const MENU: [&str; 14] = [
    "Return to game",
    "Select wager",
    "Training",
    "Settings",
    "Table profile",
    "Shoe information",
    "Table rules",
    "Session totals",
    "Review cards",
    "Controls",
    "End session",
    "Table",
    "Session",
    "Help",
];
pub const PROFILES: [&str; 6] = [
    "6 decks H17 / DAS / LS",
    "6 decks S17 / DAS / LS",
    "8 decks H17 / DAS / LS",
    "6 decks H17 / no surrender",
    "6 decks H17 / no DAS",
    "6 decks H17 / pays 6:5",
];
#[derive(Clone, Copy)]
pub(crate) struct SeatReveal {
    pub seat: u8,
    pub hand_index: u8,
    pub hand: crate::cards::Hand,
}
pub struct App {
    pub session_analysis: crate::session_analysis::Model,
    pub session_metrics: crate::session_metrics::Metrics,
    analysis_last_round: u32,
    analysis_base_wager: i64,
    analysis_valid: bool,
    pub deviation_practice: crate::deviation_drills::Model,
    pub drill_target: u8,
    pub drill_mix: u8,
    pub drill_profile: Profile,
    drill_generator: crate::deviation_drills::Generator,
    drill_token: u64,
    drill_choice: usize,

    pub wager_practice: crate::wager_practice::Model,
    pending_extra_seats: u8,
    pending_player_seat: u8,
    pub wager_enabled: bool,
    pub wager_policy: crate::wager_practice::Policy,
    other_reveals: [Option<SeatReveal>; 4],
    other_reveal_len: u8,
    other_reveal_at: u8,
    other_reveal_clock: u32,
    other_reveal_previous: Phase,
    observed_emergency_shuffles: u32,
    wager_token: u64,
    wager_checkpoint: u64,
    wager_evidence: Option<crate::wager_practice::Snapshot>,
    wager_skip_once: bool,
    wager_amount: i64,
    pub game: Engine,
    pub frame: Canvas,
    pub training: training::Model,
    pub count_practice: count_practice::Model,
    /// Runtime display capability; never part of a saved game or private wire identity.
    pub round_display: bool,
    /// Round table background: Black0, Blue1, Green2, Red3.
    pub background: u8,
    /// Shared card back: Gold0, Red1, Blue2, Charcoal3.
    pub card_back: u8,
    /// Independent Stick/browser background. Defaults to Blue.
    pub compact_background: u8,
    pub brightness: u8,
    pub flipped: bool,
    pub sound_enabled: bool,
    pub audio_volume: u8,
    pub audio_mask: u8,
    pub fast_mode: bool,
    pub resume_enabled: bool,
    checkpoint_revision: u64,
    pub dirty: bool,
    pub policy: Policy,
    pub count_display: u8,
    pub guide_count_display: u8,
    pub(crate) view: View,
    pub(crate) selected: usize,
    pub(crate) tracker: Tracker,
    pub(crate) renderer: Renderer,
    controls: Controls,
    sounds: SoundQueue,
    initial_seed: u64,
    load_allowed: bool,
    now: u32,
    clock: u32,
    pub(crate) cut_index: u8,
    pub(crate) result_hand: usize,
    return_view: View,
    notice_return: View,
    pub(crate) notice: &'static str,
    pub(crate) pending_rules: Rules,
    pub(crate) minimum_edit: bool,
    pub(crate) amount: i64,
    pub(crate) page: usize,
    pub(crate) card_group: usize,
    pub(crate) card_page: usize,
    pub(crate) pending: Option<Decision>,
    pub(crate) hint: Option<(Choice, &'static str)>,
    pub(crate) live_ev: Option<ExpectedValues>,
    pub(crate) round_id: u32,
    last_review: u32,
    test_round: bool,
    review_due: Option<u32>,
    decision_counter: u64,
    fingerprint: u64,
    current_token: u64,
    assisted_token: u64,
    pub(crate) reveal: Option<table::Reveal>,
    reveal_clock: u32,
    pub(crate) toast: Option<Mode>,
    toast_clock: u32,
    session: u64,
    count_token: u64,
    pub(crate) count_kind: Kind,
    pub(crate) count_answer: i16,
    count_assisted: bool,
    aid_key: (u32, usize, u16),
    aid_mask: u8,
}
impl App {
    /// Static storage image for direct flash-to-PSRAM copying. This is a valid
    /// Rust value without any runtime whole-App construction. Call boot once
    /// before gameplay; an unbooted image has no shuffled shoe or funded session.
    pub const EMPTY: Self = Self {
        session_analysis: crate::session_analysis::Model::EMPTY,
        session_metrics: crate::session_metrics::Metrics::EMPTY,
        analysis_last_round: 0,
        analysis_base_wager: 500,
        analysis_valid: true,
        deviation_practice: crate::deviation_drills::Model::EMPTY,
        drill_target: 31,
        drill_mix: 0,
        drill_profile: Profile::SixH17,
        drill_generator: crate::deviation_drills::Generator::new(1),
        drill_token: 0,
        drill_choice: 0,

        wager_practice: crate::wager_practice::Model::EMPTY,
        pending_extra_seats: 0,
        pending_player_seat: 0,
        wager_enabled: false,
        wager_policy: crate::wager_practice::Policy::EXAMPLE,
        other_reveals: [None; 4],
        other_reveal_len: 0,
        other_reveal_at: 0,
        other_reveal_clock: 0,
        other_reveal_previous: Phase::OtherPlayers,
        observed_emergency_shuffles: 0,
        wager_token: 0,
        wager_checkpoint: 0,
        wager_evidence: None,
        wager_skip_once: false,
        wager_amount: 500,
        pending_rules: Rules::for_profile(Profile::SixH17),
        game: Engine::EMPTY,
        frame: Canvas {
            pixels: [0; crate::canvas::WIDTH * crate::canvas::HEIGHT],
        },
        training: training::Model::EMPTY,
        count_practice: count_practice::Model::EMPTY,
        round_display: false,
        background: 0,
        card_back: 0,
        compact_background: 1,
        brightness: 255,
        flipped: false,
        sound_enabled: true,
        audio_volume: 20,
        audio_mask: 7,
        fast_mode: true,
        resume_enabled: false,
        checkpoint_revision: 0,
        dirty: true,
        policy: Policy::HiLo,
        count_display: 0,
        guide_count_display: 2,
        view: View::Cut,
        selected: 0,
        tracker: Tracker::EMPTY,
        renderer: Renderer::EMPTY,
        controls: Controls::EMPTY,
        sounds: SoundQueue::EMPTY,
        initial_seed: 0,
        load_allowed: true,
        now: 0,
        clock: 0,
        cut_index: 0,
        result_hand: 0,
        return_view: View::Cut,
        notice_return: View::Menu,
        notice: "",
        minimum_edit: true,
        amount: 500,
        page: 0,
        card_group: 0,
        card_page: 0,
        pending: None,
        hint: None,
        live_ev: None,
        round_id: 0,
        last_review: 0,
        test_round: false,
        review_due: None,
        decision_counter: 0,
        fingerprint: 0,
        current_token: 0,
        assisted_token: 0,
        reveal: None,
        reveal_clock: 0,
        toast: None,
        toast_clock: 0,
        session: 1,
        count_token: 0,
        count_kind: Kind::Running,
        count_answer: 0,
        count_assisted: false,
        aid_key: (0, 0, 0),
        aid_mask: 0,
    };
    pub fn new(seed: u64) -> Self {
        let mut app = Self::EMPTY;
        app.boot(seed);
        app
    }
    /// Initialize a fresh EMPTY image through small field assignments. Firmware
    /// must copy a static EMPTY image into PSRAM first; do not write App::new's
    /// large returned value onto a constrained embedded stack.
    pub fn boot(&mut self, seed: u64) -> bool {
        if self.game.shoe_number != 0 {
            return false;
        }
        self.game = Engine::new(seed, Rules::default()).unwrap();
        self.game.buy_in(100_000);
        self.game.set_bet(self.game.rules.minimum);
        self.pending_rules = self.game.rules;
        self.initial_seed = seed;
        self.drill_generator = crate::deviation_drills::Generator::new(seed ^ 0x444556494154494f);
        self.cut_index = self.game.bounded(17).unwrap() as u8;
        self.observe();
        self.render();
        true
    }
    pub fn view_name(&self) -> &str {
        self.view.name()
    }
    pub fn focus(&self) -> usize {
        self.selected
    }
    pub fn menu_item(&self) -> &str {
        if let Some(label) = menu_ui::focused_label(self) {
            return label;
        }
        let (actions, n) = self.extras();
        if self.view == View::Menu {
            if self.selected < n {
                table::action_name(actions[self.selected])
            } else {
                MENU.get(self.selected - n).copied().unwrap_or("")
            }
        } else {
            settings_ui::item(self.view, self.selected)
        }
    }
    /// Publicly displayed tablemate animation only; never exposes a pending shoe.
    pub(crate) fn round_face_reveal(&self) -> Option<SeatReveal> {
        if self.other_reveal_len == 0 {
            None
        } else {
            self.other_reveals
                .get(self.other_reveal_at as usize)
                .copied()
                .flatten()
        }
    }
    pub fn is_busy(&self) -> bool {
        self.reveal.is_some()
            || self.other_reveal_len != 0
            || matches!(self.game.phase, Phase::Dealer | Phase::OtherPlayers)
    }
    pub fn buttons(&mut self, down: [bool; 2], now: u32) {
        self.tick(now);
        let events = self.controls.update(
            down,
            now,
            self.view == View::CountAnswer && self.count_practice.awaiting_answer(),
        );
        for e in events.into_iter().flatten() {
            self.event(e)
        }
    }
    pub fn pop_sound(&mut self) -> Option<Cue> {
        while let Some((cue, audition)) = self.sounds.pop_entry() {
            if self.sound_enabled
                && self.audio_volume != 0
                && (audition || self.audio_mask & cue.category_bit() != 0)
            {
                return Some(cue);
            }
        }
        None
    }
    pub fn cancel_buttons(&mut self) {
        self.controls = Controls::default();
    }
    fn emit(&mut self, cue: Cue) {
        if self.sound_enabled && self.audio_volume != 0 && self.audio_mask & cue.category_bit() != 0
        {
            self.sounds.push(cue)
        }
    }
    fn set(&mut self, v: View) {
        self.checkpoint_changed();
        if v == View::Table && self.view != View::Table {
            self.reveal_clock = self.now;
            self.other_reveal_clock = self.now;
        }
        self.view = v;
        self.selected = 0;
        self.page = 0;
        self.clock = self.now;
        self.dirty = true;
    }
    fn notice(&mut self, text: &'static str, back: View) {
        self.notice = text;
        self.notice_return = back;
        self.set(View::Notice)
    }
    fn open_table(&mut self) {
        self.set(if self.game.phase == Phase::Cut {
            View::Cut
        } else if self.game.phase == Phase::Closed {
            View::Summary
        } else {
            View::Table
        })
    }
    pub(crate) fn extras(&self) -> ([Action; 3], usize) {
        let mut a = [Action::Double, Action::Split, Action::Surrender];
        let mut n = 0;
        if !self.is_busy() {
            for x in [Action::Double, Action::Split, Action::Surrender] {
                if self.game.legal(x) {
                    a[n] = x;
                    n += 1
                }
            }
        }
        (a, n)
    }
    fn menu(&mut self) {
        self.cancel_drill();
        self.cancel_wager();
        if self.view == View::Menu {
            let v = self.return_view;
            self.set(v);
            return;
        }
        if let Some(q) = self.count_practice.question() {
            self.count_practice.cancel(q.token);
        }
        if matches!(
            self.view,
            View::Table | View::Cut | View::Burn | View::Summary | View::CoachCheck
        ) {
            self.return_view = self.view
        } else if self.view == View::RoundReview {
            self.return_view = View::Table
        }
        self.set(View::Menu)
    }
    fn cycle_mode(&mut self) {
        self.training.mode = self.training.mode.next();
        if self.training.mode == Mode::Coach {
            self.training.pause_on_mistake = true
        }
        if self.training.mode == Mode::Test
            && (matches!(self.game.phase, Phase::Player | Phase::Insurance)
                || self.round_decisions() > 0)
        {
            self.test_round = true
        }
        if matches!(self.view, View::CoachCheck | View::RoundReview) {
            self.set(View::Table)
        }
        self.toast = Some(self.training.mode);
        self.toast_clock = self.now;
        self.emit(Cue::MenuSelect);
        self.dirty = true;
    }
    pub(crate) fn observe(&mut self) {
        self.tracker
            .ensure_shoe(self.game.rules.decks, self.game.shoe_number);
        if self.reveal.is_some() || self.other_reveal_len != 0 {
            return;
        }
        if self.game.uses_extended_shoe() {
            self.game.observe_exposed(&mut self.tracker);
            return;
        }
        for h in &self.game.hands[..self.game.hand_count] {
            for (&p, &c) in h.positions().iter().zip(h.cards()) {
                self.tracker.observe(p, c);
            }
        }
        for (i, (&p, &c)) in self
            .game
            .dealer
            .positions()
            .iter()
            .zip(self.game.dealer.cards())
            .enumerate()
        {
            if i == 0 || self.game.hole_visible {
                self.tracker.observe(p, c);
            }
        }
    }
    pub(crate) fn count_context(&self) -> deviations::Count {
        deviations::Count {
            running: self.tracker.running_count(),
            undealt: self.game.total_cards().saturating_sub(self.game.position) as u16,
            decks: self.tracker.decks(),
            observed: self.tracker.observed_count(),
            shoe: self.tracker.shoe_number(),
        }
    }
    fn token(&mut self) -> u64 {
        let f = (self.game.shoe_number as u64) << 32
            | (self.game.position as u64) << 16
            | (self.game.active as u64) << 8
            | self.game.phase as u64;
        if f != self.fingerprint {
            self.fingerprint = f;
            self.decision_counter = self.decision_counter.saturating_add(1);
            self.current_token = self.decision_counter
        }
        self.current_token
    }
    pub(crate) fn mark_assisted(&mut self) {
        if matches!(self.game.phase, Phase::Player | Phase::Insurance) {
            let token = self.token();
            if self.assisted_token != token {
                self.assisted_token = token;
                self.checkpoint_changed();
            }
        }
    }
    fn aid_key(&self) -> (u32, usize, u16) {
        (
            self.game.shoe_number,
            self.game.position,
            self.tracker.observed_count(),
        )
    }
    pub(crate) fn mark_aid(&mut self, mask: u8) {
        let key = self.aid_key();
        if key != self.aid_key || self.aid_mask & mask != mask {
            self.checkpoint_changed();
        }
        if key != self.aid_key {
            self.aid_key = key;
            self.aid_mask = 0
        }
        self.aid_mask |= mask;
    }
    pub(crate) fn active_count_display(&self) -> u8 {
        if self.training.mode == Mode::Guide {
            self.guide_count_display
        } else {
            self.count_display
        }
    }
    pub(crate) fn best(&self) -> Option<(Choice, &'static str)> {
        if self.game.phase == Phase::Insurance {
            let yes = self.policy == Policy::HiLo
                && deviations::insurance(
                    self.game.bankroll >= self.game.hands[0].bet / 2,
                    self.game.rules,
                    self.count_context(),
                )
                .unwrap_or(false);
            return Some((
                Choice::Insurance(yes),
                if self.policy == Policy::HiLo {
                    "Hi-Lo insurance: take at TC+3 or above when affordable."
                } else {
                    "Basic strategy declines insurance and even money."
                },
            ));
        }
        if self.game.phase != Phase::Player {
            return None;
        }
        let h = &self.game.hands[self.game.active];
        let up = self.game.dealer.cards()[0];
        let r = if self.policy == Policy::HiLo {
            deviations::recommend(
                h,
                up,
                self.game.legal_mask(),
                self.game.rules,
                self.count_context(),
            )
            .play
        } else {
            crate::strategy::recommend(h, up, self.game.legal_mask(), self.game.rules)
        }?;
        Some((Choice::Play(r.best), r.explanation))
    }
    pub(crate) fn capture_ev(&mut self) -> Option<ExpectedValues> {
        if !matches!(self.game.phase, Phase::Player | Phase::Insurance)
            || self.reveal.is_some()
            || self.other_reveal_len != 0
        {
            return None;
        }
        let h = self.game.hands[self.game.active];
        let up = self.game.dealer.cards()[0];
        if self.game.phase == Phase::Insurance {
            let mask = if self.game.bankroll >= h.bet / 2 {
                3
            } else {
                1
            };
            let mut v = [f64::NAN; 5];
            v[0] = 0.0;
            if mask == 3 {
                v[1] = ev::Evaluator::insurance(up)?
            }
            return Some(ExpectedValues {
                model: training::EvModel::InfiniteReplacementV1,
                original_wager: h.bet,
                legal_mask: mask,
                evaluated_mask: mask,
                best_mask: 1,
                initial_pair_funded: false,
                values: v,
            });
        }
        let d = ev::Decision {
            hand: h,
            upcard: up,
            legal_mask: self.game.legal_mask(),
            rules: ev::Rules::from_table(self.game.rules)?,
            peek: ev::Peek::NoBlackjack,
        };
        let mut result = ev::Evaluator::default().evaluate(d);
        let mut funded = false;
        if result.status == ev::Status::SplitUnsupported {
            let r = split_ev::SplitEvaluator::default().evaluate(split_ev::Request {
                decision: d,
                hands_in_round: self.game.hand_count as u8,
                active_hand: self.game.active as u8,
                previous_splits: self.game.hand_count.saturating_sub(1) as u8,
                first_decision: h.count() == 2
                    && !h.split
                    && !h.doubled
                    && self.game.active == 0
                    && self.game.hand_count == 1,
                original_wager: h.bet,
                uncommitted: self.game.bankroll,
                max_hands: 4,
                resplit_aces: false,
                one_card_aces: true,
                sequential: true,
            });
            if let Some(v) = r.value {
                result.values[Action::Split as usize] = v;
                result.evaluated_mask |= Action::Split.bit();
                result.omitted_mask = 0;
                result.complete = true;
                funded = true;
                let mut best = f64::NEG_INFINITY;
                for a in Action::ALL {
                    if let Some(v) = result.value(a) {
                        best = best.max(v)
                    }
                }
                result.best_mask = 0;
                for a in Action::ALL {
                    if result.value(a).is_some_and(|v| best - v <= 2e-8) {
                        result.best_mask |= a.bit();
                    }
                }
            }
        }
        if !matches!(result.status, ev::Status::Ok | ev::Status::SplitUnsupported) {
            return None;
        }
        Some(ExpectedValues {
            model: training::EvModel::InfiniteReplacementV1,
            original_wager: h.bet,
            legal_mask: d.legal_mask,
            evaluated_mask: result.evaluated_mask,
            best_mask: result.best_mask,
            initial_pair_funded: funded,
            values: result.values,
        })
    }
    fn perform(&mut self, choice: Choice) {
        if self.is_busy() {
            return;
        }
        let legal = match choice {
            Choice::Play(a) => self.game.legal(a),
            Choice::Insurance(yes) => {
                self.game.phase == Phase::Insurance
                    && (!yes || self.game.bankroll >= self.game.hands[0].bet / 2)
            }
        };
        if !legal {
            return;
        }
        self.observe();
        let token = self.token();
        if self.training.mode != Mode::Off {
            let values = self.capture_ev();
            let a = training::Attempt {
                token,
                round: self.round_id,
                hand_index: self.game.active as u8,
                profile: self.game.rules.profile,
                hand: self.game.hands[self.game.active],
                dealer_up: self.game.dealer.cards()[0],
                legal_mask: match choice {
                    Choice::Play(_) => self.game.legal_mask(),
                    Choice::Insurance(_) => {
                        if self.game.bankroll >= self.game.hands[0].bet / 2 {
                            3
                        } else {
                            1
                        }
                    }
                },
                choice,
                policy: self.policy,
                count: if self.policy == Policy::HiLo {
                    Some(self.count_context())
                } else {
                    None
                },
                assisted: self.assisted_token == token || self.active_count_display() != 0,
                values,
            };
            let grade = self.training.record(a);
            if grade.status == training::Status::Recorded
                && let Some(decision) = grade.decision
            {
                self.session_metrics.record_card(decision);
            }
            if grade.pause {
                self.pending = grade.decision;
                self.set(View::CoachCheck);
                return;
            }
        }
        self.perform_core(choice)
    }
    fn perform_core(&mut self, c: Choice) {
        let p = self.game.phase;
        let pos = self.game.position;
        let before_hands = self.game.hands;
        let before_count = self.game.hand_count;
        let before_shoe = self.game.shoe_number;
        let ok = match c {
            Choice::Play(a) => self.game.act(a),
            Choice::Insurance(y) => self.game.insure(y),
        };
        if ok {
            for _ in pos..self.game.position {
                self.emit(Cue::CardDeal)
            }
            self.queue_hidden_player_faces(before_hands, before_count, before_shoe, p);
            if self.other_reveal_len == 0 {
                self.after_action(p)
            } else {
                self.dirty = true;
            }
        }
    }
    fn after_action(&mut self, previous: Phase) {
        self.observe();
        self.record_settled_round();
        if self.game.phase == Phase::Result && previous != Phase::Result {
            self.emit(
                if self.game.hands[..self.game.hand_count]
                    .iter()
                    .any(|h| h.outcome == crate::cards::Outcome::Blackjack)
                {
                    Cue::Blackjack
                } else if self.game.round_net > 0 {
                    Cue::Win
                } else if self.game.round_net < 0 {
                    Cue::Loss
                } else {
                    Cue::Push
                },
            );
            self.result_hand = 0;
            self.review_due = Some(self.now)
        }
        if matches!(self.game.phase, Phase::Dealer | Phase::OtherPlayers)
            && self.game.phase != previous
        {
            self.clock = self.now;
            if self.game.phase == Phase::Dealer {
                self.emit(Cue::CardFlip)
            }
        }
        if self.game.emergency_shuffles > self.observed_emergency_shuffles {
            self.observed_emergency_shuffles = self.game.emergency_shuffles;
            self.notice(
                "Discard rack shuffled. Held cards excluded. Unseen burn; new count epoch.",
                View::Table,
            );
        }
        self.dirty = true;
    }
    fn finish_training_round(&mut self) {
        if self.game.phase == Phase::Result
            && self.training.mode == Mode::Test
            && self.test_round
            && self.last_review != self.round_id
            && self.reveal.is_none()
            && self.other_reveal_len == 0
        {
            self.last_review = self.round_id;
            self.review_due = None;
            self.set(View::RoundReview)
        }
    }
    fn next_deal(&mut self) {
        if self.is_busy() {
            return;
        }
        if self.game.phase == Phase::Result && self.review_due.is_some() {
            self.finish_training_round();
            self.review_due = None;
            if self.view == View::RoundReview {
                return;
            }
        }
        if self.wager_enabled
            && !self.wager_skip_once
            && matches!(self.game.phase, Phase::Ready | Phase::Result)
            && !self.game.needs_shuffle()
        {
            self.open_wager_question();
            return;
        }
        if self.game.bankroll < self.game.rules.minimum {
            self.set(View::Refill);
            return;
        }
        if self.game.bankroll < self.game.selected_bet {
            self.notice("Not enough chips for this wager", View::Wager);
            return;
        }
        if self.game.needs_shuffle() {
            if self.game.phase != Phase::Cut {
                self.game.shuffle();
                self.cut_index = self.game.bounded(17).unwrap() as u8;
            }
            self.open_table();
            self.emit(Cue::Shuffle);
            return;
        }
        self.wager_skip_once = false;
        self.cancel_wager();
        let bankroll = self.game.bankroll - self.game.selected_bet;
        if self.game.deal() {
            self.analysis_base_wager = self.game.selected_bet;
            self.load_allowed = false;
            self.round_id = self.round_id.wrapping_add(1);
            self.test_round = self.training.mode == Mode::Test;
            self.fingerprint = 0;
            self.assisted_token = 0;
            self.result_hand = 0;
            self.set(View::Table);
            self.reveal = Some(table::Reveal {
                progress: 0,
                bankroll,
            });
            self.reveal_clock = self.now;
            self.dirty = true;
        }
    }
    fn open_count(&mut self, kind: Kind) {
        if self.is_busy() {
            self.notice("Finish dealing before a count drill", View::CountPractice);
            return;
        }
        self.observe();
        if let Some(q) = self.count_practice.question() {
            self.count_practice.cancel(q.token);
        }
        let Some(s) =
            Snapshot::from_tracker(&self.tracker, self.count_context().undealt, self.session)
        else {
            self.notice("Count drill unavailable here", View::CountPractice);
            return;
        };
        self.count_token = self.count_token.saturating_add(1);
        let q = Question {
            token: self.count_token,
            kind,
            given_half_decks: if kind == Kind::TrueCount {
                Some(((s.undealt + 13) / 26).max(1) as u8)
            } else {
                None
            },
        };
        if !self.count_practice.begin(q, s) {
            self.notice("Count drill unavailable here", View::CountPractice);
            return;
        }
        self.count_assisted = self.aid_key == self.aid_key()
            && self.aid_mask
                & match kind {
                    Kind::Running => 1,
                    Kind::Decks => 2,
                    Kind::TrueCount => 5,
                }
                != 0;
        self.count_kind = kind;
        self.count_answer = 0;
        self.set(View::CountAnswer)
    }
    fn submit_count(&mut self) {
        if self.view != View::CountAnswer {
            return;
        }
        let Some(q) = self.count_practice.question() else {
            return;
        };
        let answer = match q.kind {
            Kind::Running => Answer::Running(self.count_answer),
            Kind::Decks => Answer::Decks(self.count_answer as u8),
            Kind::TrueCount => Answer::TrueCount(self.count_answer),
        };
        let g = self
            .count_practice
            .submit(q.token, answer, self.count_assisted);
        if let Some(a) = g.attempt {
            self.mark_aid(match a.question.kind {
                Kind::Running => 1,
                Kind::Decks => 2,
                Kind::TrueCount => 4,
            });
            self.mark_assisted();
            self.emit(if a.correct {
                Cue::MenuSelect
            } else {
                Cue::Loss
            });
            self.set(View::CountResult)
        }
    }
    fn reset_session(&mut self) {
        self.session_analysis = crate::session_analysis::Model::EMPTY;
        self.session_metrics = crate::session_metrics::Metrics::EMPTY;
        self.analysis_last_round = 0;
        self.analysis_valid = true;
        self.cancel_drill();
        let seed = self.game.random64();
        let rules = self.game.rules;
        let seats = (self.game.extra_seats, self.game.player_seat);
        self.game = Engine::new(seed, rules).unwrap();
        self.analysis_base_wager = rules.minimum;
        self.game.configure_seats(seats.0, seats.1);
        self.game.buy_in(100_000);
        self.game.set_bet(rules.minimum);
        self.session = self.session.saturating_add(1);
        self.cancel_wager();
        self.observed_emergency_shuffles = 0;
        self.wager_evidence = None;
        self.wager_checkpoint = 0;
        self.wager_skip_once = false;
        self.last_review = 0;
        self.pending = None;
        self.live_ev = None;
        self.hint = None;
        self.fingerprint = 0;
        self.assisted_token = 0;
        self.reveal = None;
        self.other_reveal_len = 0;
        self.other_reveal_at = 0;
        self.other_reveals = [None; 4];
        self.review_due = None;
        self.aid_mask = 0;
        self.aid_key = (0, 0, 0);
        self.cut_index = self.game.bounded(17).unwrap() as u8;
        self.tracker = Tracker::default();
        self.count_assisted = false;
        self.observe();
        self.set(View::Cut)
    }
    pub fn checkpoint_revision(&self) -> u64 {
        self.checkpoint_revision
    }
    fn checkpoint_changed(&mut self) {
        self.checkpoint_revision = self.checkpoint_revision.wrapping_add(1);
    }
    fn checkpoint_key(&self) -> [u64; 19] {
        [
            self.view as u64,
            self.selected as u64,
            self.page as u64,
            self.game.phase as u64,
            self.game.position as u64,
            self.game.shoe_number as u64,
            self.game.rounds as u64,
            self.game.active as u64,
            self.game.active_seat as u64,
            self.game.rng,
            self.reveal.map_or(256, |r| r.progress as u64),
            self.other_reveal_len as u64,
            self.other_reveal_at as u64,
            self.review_due.map_or(1u64 << 32, |n| n as u64),
            self.tracker.observed_count() as u64,
            self.count_practice.question().map_or(0, |q| q.token),
            self.wager_practice.question().map_or(0, |q| q.token),
            self.deviation_practice.question().map_or(0, |q| q.token),
            self.game.bankroll as u64,
        ]
    }
    pub fn tick(&mut self, now: u32) {
        let before = self.checkpoint_key();
        self.tick_core(now);
        if before != self.checkpoint_key() {
            self.checkpoint_changed();
        }
    }
    fn tick_core(&mut self, now: u32) {
        self.reconcile_wager();
        self.now = now;
        if self.view == View::Table
            && self
                .review_due
                .is_some_and(|at| now.wrapping_sub(at) >= 900)
            && self.reveal.is_none()
            && self.other_reveal_len == 0
        {
            self.finish_training_round();
            self.review_due = None;
        }
        if self.toast.is_some() && now.wrapping_sub(self.toast_clock) >= 1000 {
            self.toast = None;
            self.dirty = true
        }
        if self.view == View::Burn && now.wrapping_sub(self.clock) >= self.paced_delay(1100, 550) {
            self.set(View::Table)
        }
        if self.view == View::Table {
            if self.other_reveal_len != 0 {
                if now.wrapping_sub(self.other_reveal_clock) >= self.paced_delay(750, 400) {
                    self.other_reveal_clock = now;
                    self.other_reveal_at += 1;
                    if self.other_reveal_at >= self.other_reveal_len {
                        self.other_reveal_len = 0;
                        self.other_reveal_at = 0;
                        self.other_reveals = [None; 4];
                        self.after_action(self.other_reveal_previous);
                        self.clock = now;
                    }
                    self.dirty = true;
                }
                return;
            }
            if let Some(mut r) = self.reveal {
                // Every other seat's initial faces must actually be shown
                // before the authoritative count or player decision is available.
                let last = 4 + self.game.extra_seats;
                let delay = if self.game.extra_seats > 0 && r.progress >= 4 {
                    self.paced_delay(750, 400)
                } else {
                    self.paced_delay(180, 120)
                };
                if now.wrapping_sub(self.reveal_clock) >= delay {
                    self.reveal_clock = now;
                    if r.progress < last {
                        r.progress += 1;
                        self.reveal = Some(r);
                        self.emit(Cue::CardDeal)
                    } else {
                        self.reveal = None;
                        self.observe();
                        self.after_action(Phase::Ready)
                    }
                    self.dirty = true
                }
            } else if self.game.phase == Phase::OtherPlayers
                && now.wrapping_sub(self.clock) >= self.paced_delay(650, 325)
            {
                self.clock = now;
                let previous = self.game.phase;
                let position = self.game.position;
                let place = self.game.active_seat;
                let index = if place < self.game.player_seat {
                    place as usize
                } else {
                    place as usize - 1
                };
                let before = self.game.others[index];
                let before_shoe = self.game.shoe_number;
                if self.game.other_step() {
                    self.queue_other_faces(place, before, self.game.others[index], before_shoe);
                    if self.game.position > position || self.other_reveal_len != 0 {
                        self.emit(Cue::CardDeal)
                    }
                    if self.other_reveal_len == 0 {
                        self.after_action(previous)
                    } else {
                        self.dirty = true;
                    }
                };
            } else if self.game.phase == Phase::Dealer
                && now.wrapping_sub(self.clock) >= self.paced_delay(650, 325)
            {
                self.clock = now;
                let p = self.game.phase;
                let n = self.game.position;
                self.game.dealer_step();
                if self.game.position > n {
                    self.emit(Cue::CardDeal)
                }
                self.after_action(p)
            }
        }
    }
    pub fn event(&mut self, event: Event) {
        self.checkpoint_changed();
        self.load_allowed = false;
        let menu_audition =
            self.view == View::SoundTests && self.selected < 3 && event == Event::Tap(Key::Blue);
        if menu_ui::handle(self, event) {
            if !menu_audition {
                self.emit(if event == Event::Hold(Key::Grey) {
                    Cue::MenuSelect
                } else {
                    Cue::MenuMove
                });
            }
            self.dirty = true;
            return;
        }
        if self.handle_settings_event(event) {
            self.dirty = true;
            return;
        }
        match event {
            Event::Hold(Key::Blue) => {
                self.cycle_mode();
                return;
            }
            Event::Hold(Key::Grey) => {
                self.emit(Cue::MenuSelect);
                self.menu();
                return;
            }
            Event::SubmitCount => {
                self.submit_count();
                return;
            }
            _ => {}
        }
        self.emit(Cue::MenuMove);
        let grey = event == Event::Tap(Key::Grey);
        if self.handle_drill_tap(grey) {
            self.dirty = true;
            return;
        }
        if self.handle_analysis_tap(grey) {
            self.dirty = true;
            return;
        }
        if self.handle_seat_tap(grey) {
            self.dirty = true;
            return;
        }
        if self.handle_wager_tap(grey) {
            self.dirty = true;
            return;
        }
        if self.view == View::CountAnswer {
            let limit = match self.count_kind {
                Kind::Running => self.game.rules.decks as i16 * 20,
                Kind::Decks => self.game.rules.decks as i16 * 2,
                Kind::TrueCount => 21632,
            };
            let low = if self.count_kind == Kind::Decks {
                0
            } else {
                -limit
            };
            self.count_answer = (self.count_answer + if grey { -1 } else { 1 }).clamp(low, limit);
            self.dirty = true;
            return;
        }
        if grey {
            self.grey()
        } else {
            self.blue()
        }
        self.dirty = true;
    }
    pub fn request_count_help(&mut self) {
        if let Some(q) = self.count_practice.question() {
            self.count_practice.request_help(q.token);
            self.count_assisted = true;
            self.mark_assisted();
            self.dirty = true
        }
    }
    fn grey(&mut self) {
        match self.view {
            View::Cut => self.cut_index = (self.cut_index + 1) % 17,
            View::Table => {
                if self.is_busy() {
                    return;
                }
                match self.game.phase {
                    Phase::Player => self.perform(Choice::Play(Action::Hit)),
                    Phase::Insurance => self.selected = (self.selected + 1) % 2,
                    Phase::Ready | Phase::Result => self.set(View::Wager),
                    Phase::Cut => self.set(View::Cut),
                    _ => {}
                }
            }
            View::Menu => self.selected = (self.selected + 1) % (MENU.len() + self.extras().1),
            View::Wager => self.selected = (self.selected + 1) % 6,
            View::Amount | View::ProfileAmount => self.selected = (self.selected + 1) % 5,
            View::Training => self.selected = (self.selected + 1) % 11,
            View::Settings => self.selected = (self.selected + 1) % 7,
            View::Profile => self.selected = (self.selected + 1) % 8,
            View::ProfilePreset => self.selected = (self.selected + 1) % 7,
            View::ProfileDepth => self.selected = (self.selected + 1) % 8,
            View::CountPractice => self.selected = (self.selected + 1) % 7,
            View::CoachCheck | View::Leave | View::Summary => {
                self.selected = (self.selected + 1) % 2
            }
            View::Session => self.selected = (self.selected + 1) % 5,
            View::TrainingStats => self.page = (self.page + 1) % 5,
            View::CountStats => self.page = (self.page + 1) % 4,
            View::TrainingReview | View::EvReview => {
                self.page = (self.page + 1) % self.training.history_count().max(1)
            }
            View::RoundReview => self.page = (self.page + 1) % (self.round_decisions() + 1),
            View::CountHistory => {
                self.page = (self.page + 1) % self.count_practice.history_count().max(1);
                self.mark_count_history()
            }
            View::CountResult => self.open_count(self.count_kind),
            View::ActionEv => {
                let n = self
                    .live_ev
                    .map_or(1, |v| v.legal_mask.count_ones() as usize);
                self.page = (self.page + 1) % n.max(1)
            }
            View::Rules => self.page = (self.page + 1) % 7,
            View::Controls => self.page = (self.page + 1) % 4,
            View::Bankroll => self.page = (self.page + 1) % 2,
            View::Cards => self.next_cards(),
            _ => {}
        }
    }
    fn blue(&mut self) {
        match self.view {
            View::Cut => {
                let total = self.game.total_cards();
                let at = 52 + (total - 104) * self.cut_index as usize / 16;
                if self.game.cut(at) {
                    self.emit(Cue::Shuffle);
                    self.set(View::Burn)
                }
            }
            View::Burn => {
                self.set(View::Table);
                self.next_deal()
            }
            View::Table => {
                if self.is_busy() {
                    return;
                }
                match self.game.phase {
                    Phase::Ready | Phase::Result => self.next_deal(),
                    Phase::Cut => self.set(View::Cut),
                    Phase::Player => self.perform(Choice::Play(Action::Stand)),
                    Phase::Insurance => self.perform(Choice::Insurance(self.selected == 1)),
                    _ => {}
                }
            }
            View::Menu => self.select_menu(),
            View::Wager => {
                if self.selected < 4 {
                    let v = [500, 1000, 2500, 10000][self.selected];
                    if self.game.set_bet(v) {
                        self.emit(Cue::Chip);
                        self.open_table()
                    } else {
                        self.notice("Wager is outside table limits", View::Wager)
                    }
                } else if self.selected == 4 {
                    self.amount = self.game.selected_bet;
                    self.set(View::Amount)
                } else {
                    self.open_table()
                }
            }
            View::Amount | View::ProfileAmount => {
                let profile = self.view == View::ProfileAmount;
                let (low, high) = if profile {
                    if self.minimum_edit {
                        (500, self.pending_rules.maximum)
                    } else {
                        (self.pending_rules.minimum, 1_000_000)
                    }
                } else {
                    (self.game.rules.minimum, self.game.rules.maximum)
                };
                if self.selected < 4 {
                    self.amount =
                        (self.amount + [500, -500, 2500, -2500][self.selected]).clamp(low, high)
                } else if profile {
                    if self.minimum_edit {
                        self.pending_rules.minimum = self.amount
                    } else {
                        self.pending_rules.maximum = self.amount
                    }
                    self.set(View::Profile)
                } else if self.game.set_bet(self.amount) {
                    self.emit(Cue::Chip);
                    self.open_table()
                }
            }
            View::Training => match self.selected {
                0 => self.cycle_mode(),
                1 => {
                    if self.game.between_rounds() && !self.is_busy() {
                        self.policy = if self.policy == Policy::Basic {
                            Policy::HiLo
                        } else {
                            Policy::Basic
                        }
                    } else {
                        self.notice("Change strategy between rounds", View::Training)
                    }
                }
                2 => self.set(View::CountPractice),
                3 => self.set(View::TrainingStats),
                4 => {
                    self.mark_assisted();
                    self.set(View::TrainingReview)
                }
                5 => {
                    self.live_ev = self.capture_ev();
                    if self.live_ev.is_some() {
                        self.mark_assisted();
                        self.set(View::ActionEv)
                    } else {
                        self.notice("Action EV needs a fully revealed turn", View::Training)
                    }
                }
                6 => {
                    self.mark_assisted();
                    self.set(View::EvReview)
                }
                7 => {
                    if self.training.mode == Mode::Off {
                        self.notice("Select a training mode first", View::Training)
                    } else if !self.is_busy() {
                        self.hint = self.best();
                        if self.hint.is_some() {
                            self.mark_assisted();
                            self.set(View::Hint)
                        } else {
                            self.notice("Hints are available during your turn", View::Training)
                        }
                    } else {
                        self.notice("Finish dealing before viewing a hint", View::Training)
                    }
                }
                8 => self.set(View::WagerPractice),
                9 => self.set(View::DeviationPractice),
                _ => self.set(View::Menu),
            },
            View::CountPractice => match self.selected {
                0 => self.open_count(Kind::Running),
                1 => self.open_count(Kind::Decks),
                2 => self.open_count(Kind::TrueCount),
                3 => {
                    if self.is_busy() {
                        self.notice("Finish dealing before count info", View::CountPractice)
                    } else {
                        self.observe();
                        self.mark_aid(7);
                        self.mark_assisted();
                        self.set(View::CountInfo)
                    }
                }
                4 => self.set(View::CountStats),
                5 => {
                    self.set(View::CountHistory);
                    self.mark_count_history()
                }
                _ => self.set(View::Training),
            },
            View::CountResult | View::CountStats | View::CountHistory | View::CountInfo => {
                self.set(View::CountPractice)
            }
            View::TrainingStats
            | View::TrainingReview
            | View::Hint
            | View::ActionEv
            | View::EvReview => self.set(View::Training),
            View::RoundReview => self.open_table(),
            View::CoachCheck => {
                if let Some(d) = self.pending.take() {
                    let c = if self.selected == 1 {
                        d.best
                    } else {
                        d.attempt.choice
                    };
                    self.set(View::Table);
                    self.perform_core(c)
                }
            }
            View::Settings => match self.selected {
                0 => self.set(View::Display),
                1 => self.set(View::Sound),
                2 => self.fast_mode = !self.fast_mode,
                3 => {
                    if !self.game.between_rounds() || self.is_busy() {
                        self.notice("Finish the round before adding chips", View::Settings)
                    } else if self.game.buy_in(100_000) {
                        self.emit(Cue::Chip)
                    } else {
                        self.notice("Credit limit reached", View::Settings)
                    }
                }
                4 => {
                    if self.training.mode == Mode::Guide {
                        self.guide_count_display = (self.guide_count_display + 1) % 3
                    } else {
                        self.count_display = (self.count_display + 1) % 3
                    }
                }
                5 => self.resume_enabled = !self.resume_enabled,
                _ => self.set(View::Menu),
            },
            View::Profile => match self.selected {
                0 => {
                    self.set(View::ProfilePreset);
                    self.selected = self.pending_rules.profile as usize
                }
                1 | 2 => {
                    self.minimum_edit = self.selected == 1;
                    self.amount = if self.minimum_edit {
                        self.pending_rules.minimum
                    } else {
                        self.pending_rules.maximum
                    };
                    self.set(View::ProfileAmount)
                }
                3 => self.set(View::ProfileDepth),
                4 => {
                    if self.game.configure(self.pending_rules) {
                        self.cut_index = self.game.bounded(17).unwrap() as u8;
                        self.aid_mask = 0;
                        self.observe();
                        self.open_table()
                    } else {
                        self.notice("Profile unavailable; finish the round", View::Profile)
                    }
                }
                _ => self.set(View::Menu),
            },
            View::ProfilePreset => {
                if self.selected < 6 {
                    let mut r = Rules::for_profile(Profile::ALL[self.selected]);
                    r.minimum = self.pending_rules.minimum;
                    r.maximum = self.pending_rules.maximum;
                    r.penetration = self.pending_rules.penetration;
                    self.pending_rules = r
                }
                self.set(View::Profile)
            }
            View::ProfileDepth => {
                if self.selected < 7 {
                    self.pending_rules.penetration = [50, 60, 70, 75, 80, 85, 90][self.selected]
                }
                self.set(View::Profile)
            }
            View::Session => match self.selected {
                0 => self.set(View::Menu),
                1 => self.set(View::Bankroll),
                2 => {
                    if self.game.between_rounds() {
                        self.set(View::Refill)
                    } else {
                        self.notice("Finish the round before adding chips", View::Session)
                    }
                }
                _ => self.set(View::Leave),
            },
            View::Bankroll => self.set(View::Session),
            View::ShoeInfo | View::Rules | View::Controls | View::Cards => self.set(View::Menu),
            View::Refill => {
                if self.game.buy_in(100_000) {
                    self.emit(Cue::Chip);
                    self.open_table()
                } else {
                    self.notice("Refill unavailable during this round", View::Table)
                }
            }
            View::Leave => {
                if self.selected == 0 {
                    self.set(View::Menu)
                } else if self.game.leave() {
                    self.emit(Cue::Chip);
                    self.set(View::Summary)
                } else {
                    self.notice("Finish the round before leaving", View::Menu)
                }
            }
            View::Summary => {
                if self.selected == 0 {
                    self.reset_session()
                } else {
                    self.set(View::Rules)
                }
            }
            View::Notice => {
                let v = self.notice_return;
                self.set(v);
                if v == View::Table && self.game.phase == Phase::Result {
                    self.review_due = Some(self.now)
                }
            }
            View::AnalysisMenu
            | View::AnalysisSummary
            | View::AnalysisGraph
            | View::AnalysisRounds
            | View::AnalysisAccuracy
            | View::AnalysisCosts
            | View::AnalysisDecisions
            | View::AnalysisAssumptions
            | View::DeviationPractice
            | View::DeviationTarget
            | View::DeviationQuestion
            | View::DeviationContext
            | View::DeviationReference
            | View::DeviationResult
            | View::DeviationStats
            | View::DeviationHistory
            | View::DeviationPolicy
            | View::TableSeats
            | View::SeatCards
            | View::WagerPractice
            | View::WagerAnswer
            | View::WagerHelp
            | View::WagerResult
            | View::WagerStats
            | View::WagerHistory
            | View::WagerPolicy
            | View::CountAnswer
            | View::Sound
            | View::AudioVolume
            | View::Display
            | View::Brightness
            | View::TableMenu
            | View::SessionMenu
            | View::SessionOverview
            | View::BankrollMenu
            | View::NewSessionConfirm
            | View::ResetStatsConfirm
            | View::HelpMenu
            | View::TrainingModes
            | View::LiveCounts
            | View::PracticeMenu
            | View::ProgressMenu
            | View::AdviceMenu
            | View::SoundCategories
            | View::SoundTests
            | View::HandProgress
            | View::CountProgress
            | View::WagerProgress
            | View::DeviationProgress
            | View::CardBackPreview => {}
        }
    }
    fn select_menu(&mut self) {
        let (a, n) = self.extras();
        if self.selected < n {
            let action = a[self.selected];
            self.set(View::Table);
            self.perform(Choice::Play(action));
            return;
        }
        match self.selected - n {
            0 => {
                let v = self.return_view;
                self.set(v)
            }
            1 => {
                if self.game.between_rounds() && !self.is_busy() {
                    self.set(View::Wager)
                } else {
                    self.notice("Finish the round to change wagers", View::Menu)
                }
            }
            2 => self.set(View::Training),
            3 => self.set(View::Settings),
            4 => {
                if self.game.between_rounds() && !self.is_busy() {
                    self.pending_rules = self.game.rules;
                    self.set(View::Profile)
                } else {
                    self.notice("Change profiles between rounds", View::Menu)
                }
            }
            5 => {
                self.mark_aid(6);
                self.mark_assisted();
                self.set(View::ShoeInfo)
            }
            6 => self.set(View::Rules),
            7 => self.set(View::Session),
            8 => {
                if self.game.hand_count > 0 && !self.is_busy() {
                    self.card_group = if self.game.phase == Phase::Player {
                        self.game.active
                    } else {
                        self.result_hand
                    };
                    self.card_page = 0;
                    self.set(View::Cards)
                } else {
                    self.notice("No revealed cards to review", View::Menu)
                }
            }
            9 => self.set(View::Controls),
            11 => self.set(View::TableMenu),
            12 => self.set(View::SessionMenu),
            13 => self.set(View::HelpMenu),
            _ => {
                if self.game.between_rounds() && !self.is_busy() {
                    self.set(View::Leave)
                } else {
                    self.notice("Finish the round before leaving", View::Menu)
                }
            }
        }
    }
    fn next_cards(&mut self) {
        let dealer = self.card_group == self.game.hand_count;
        let visible = if dealer && !self.game.hole_visible {
            1
        } else if dealer {
            self.game.dealer.count()
        } else {
            self.game.hands[self.card_group].count()
        };
        self.card_page += 1;
        if self.card_page * 4 >= visible {
            self.card_page = 0;
            self.card_group = (self.card_group + 1) % (self.game.hand_count + 1)
        }
    }
    pub(crate) fn round_decisions(&self) -> usize {
        (0..self.training.history_count())
            .filter(|&i| {
                self.training
                    .history(i)
                    .is_some_and(|d| d.attempt.round == self.round_id)
            })
            .count()
    }
    pub(crate) fn review_decision(&self) -> Option<Decision> {
        if self.view == View::RoundReview {
            (0..self.training.history_count())
                .filter_map(|i| self.training.history(i))
                .filter(|d| d.attempt.round == self.round_id)
                .nth(self.page.saturating_sub(1))
                .copied()
        } else {
            self.training.newest(self.page).copied()
        }
    }
    fn mark_count_history(&mut self) {
        if let Some(a) = self.count_practice.newest(self.page).copied()
            && a.snapshot.session == self.session
            && a.snapshot.shoe == self.game.shoe_number
            && a.snapshot.undealt == self.count_context().undealt
            && a.snapshot.observed == self.tracker.observed_count()
        {
            self.mark_aid(match a.question.kind {
                Kind::Running => 1,
                Kind::Decks => 2,
                Kind::TrueCount => 4,
            });
            self.mark_assisted()
        }
    }
    pub fn render(&mut self) {
        self.observe();
        crate::screens::draw(self);
        self.dirty = false;
    }
    pub fn save(&self) -> persistence::Save {
        Save {
            resume_enabled: self.resume_enabled,
            drill_target: self.drill_target,
            drill_mix: self.drill_mix,
            drill_profile: self.drill_profile,
            drill_stats: self.deviation_practice.stats(),
            extra_seats: self.game.extra_seats,
            player_seat: self.game.player_seat,
            wager_enabled: self.wager_enabled,
            wager_policy: self.wager_policy,
            wager_stats: self.wager_practice.stats(),
            mode: self.training.mode,
            brightness: self.brightness,
            background: self.background,
            card_back: self.card_back,
            compact_background: self.compact_background,
            sound: self.sound_enabled,
            audio_volume: self.audio_volume,
            audio_mask: self.audio_mask,
            fast_mode: self.fast_mode,
            flipped: self.flipped,
            pause_on_mistake: self.training.pause_on_mistake,
            profile: self.game.rules.profile,
            penetration: self.game.rules.penetration,
            count_display: self.count_display,
            guide_count_display: self.guide_count_display,
            minimum_bet: self.game.rules.minimum as u32,
            maximum_bet: self.game.rules.maximum as u32,
            stats: self.training.stats(),
            policy: self.policy,
            count_stats: self.count_practice.stats(),
        }
    }
    /// Restore preferences/scores once at boot. Live session and shuffled cards are absent.
    pub fn load(&mut self, s: Save) -> bool {
        if !self.load_allowed
            || self.game.hand_count != 0
            || self.game.rounds != 0
            || !s.valid()
            || self.training.history_count() != 0
            || self.count_practice.history_count() != 0
            || self.wager_practice.history_count() != 0
            || self.wager_practice.awaiting_answer()
            || self.deviation_practice.history_count() != 0
            || self.deviation_practice.question().is_some()
        {
            return false;
        }
        if !self.deviation_practice.restore(s.drill_stats) {
            return false;
        }
        if !self.wager_practice.restore(s.wager_stats) {
            return false;
        }
        let mut training = training::Model::default();
        let mut count = count_practice::Model::default();
        if !training.restore(s.stats, s.pause_on_mistake) || !count.restore(s.count_stats) {
            return false;
        }
        self.game = Engine::new(self.initial_seed, s.rules()).unwrap();
        self.game.configure_seats(s.extra_seats, s.player_seat);
        self.game.buy_in(100_000);
        self.game.set_bet(s.rules().minimum);
        self.training = training;
        self.count_practice = count;
        self.drill_target = s.drill_target;
        self.drill_mix = s.drill_mix;
        self.drill_profile = s.drill_profile;
        self.wager_enabled = s.wager_enabled;
        self.wager_policy = s.wager_policy;
        self.policy = s.policy;
        self.count_display = s.count_display;
        self.guide_count_display = s.guide_count_display;
        self.brightness = s.brightness;
        self.background = s.background;
        self.card_back = s.card_back;
        self.compact_background = s.compact_background;
        self.flipped = s.flipped;
        self.sound_enabled = s.sound;
        self.audio_volume = s.audio_volume;
        self.audio_mask = s.audio_mask;
        self.fast_mode = s.fast_mode;
        self.resume_enabled = s.resume_enabled;
        self.pending_rules = s.rules();
        self.cut_index = self.game.bounded(17).unwrap() as u8;
        self.tracker = Tracker::default();
        self.observe();
        self.session_analysis = crate::session_analysis::Model::EMPTY;
        self.session_metrics = crate::session_metrics::Metrics::EMPTY;
        self.analysis_last_round = 0;
        self.analysis_base_wager = s.rules().minimum;
        self.analysis_valid = true;
        self.load_allowed = false;
        self.set(View::Cut);
        true
    }
}

#[path = "settings_ui.rs"]
pub(crate) mod settings_ui;

#[path = "wager_ui.rs"]
pub(crate) mod wager_ui;

#[path = "table_setup.rs"]
pub(crate) mod table_setup;

#[path = "deviation_ui.rs"]
pub(crate) mod deviation_ui;

#[cfg(test)]
mod npc_visibility_tests {
    use super::*;
    #[test]
    fn terminal_queue_defers_dealer_count_exposure_and_review() {
        let mut found = false;
        for seed in 1..200 {
            let mut app = App::new(seed);
            app.game.configure_seats(2, 0);
            app.event(Event::Tap(Key::Blue));
            app.event(Event::Tap(Key::Blue));
            for now in [200, 400, 600, 800, 1550, 2300, 3050] {
                app.tick(now)
            }
            if app.game.phase == Phase::Insurance {
                app.event(Event::Tap(Key::Blue));
            }
            if app.game.phase != Phase::Player {
                continue;
            }
            app.event(Event::Tap(Key::Blue));
            let mut now = 3050;
            for _ in 0..80 {
                if app.game.phase != Phase::OtherPlayers && app.other_reveal_len == 0 {
                    break;
                }
                let observed = app.tracker.observed_count();
                let running = app.tracker.running_count();
                now += 650;
                app.tick(now);
                if app.other_reveal_len != 0 {
                    assert_eq!(app.tracker.observed_count(), observed);
                    assert_eq!(app.tracker.running_count(), running);
                    app.render();
                    assert_eq!(app.tracker.observed_count(), observed);
                    if matches!(app.game.phase, Phase::Dealer | Phase::Result) {
                        found = true;
                        assert!(app.is_busy());
                        assert!(app.review_due.is_none());
                    }
                    while app.other_reveal_len != 0 {
                        now += 750;
                        app.tick(now)
                    }
                }
            }
            if found {
                return;
            }
        }
        panic!("no terminal NPC draw reaching dealer/result");
    }
    #[test]
    fn default_split_double_final_faces_are_queued_only_when_hidden() {
        for seed in 1..3000 {
            let mut app = App::new(seed);
            app.event(Event::Tap(Key::Blue));
            app.event(Event::Tap(Key::Blue));
            for n in 1..=5 {
                app.tick(n * 200)
            }
            if !app.game.legal(Action::Split) || app.game.hands[0].cards()[0].value() == 11 {
                continue;
            }
            app.perform(Choice::Play(Action::Split));
            app.render();
            if app.game.phase != Phase::Player
                || app.game.active != 0
                || app.other_reveal_len != 0
                || !app.game.legal(Action::Double)
            {
                continue;
            }
            assert_eq!(app.game.hands[1].count(), 1);
            app.perform(Choice::Play(Action::Double));
            if app.game.phase != Phase::Player || app.game.active != 1 {
                continue;
            }
            assert_eq!(app.other_reveal_len, 2);
            assert_eq!(app.other_reveals[0].unwrap().hand_index, 0);
            assert_eq!(app.other_reveals[1].unwrap().hand_index, 1);
            let position = app.game.position;
            app.event(Event::Tap(Key::Blue));
            assert_eq!(app.game.position, position);
            assert_eq!(app.game.phase, Phase::Player);
            app.tick(1750);
            app.render();
            app.tick(2500);
            assert_eq!(app.other_reveal_len, 0);
            if !app.game.legal(Action::Double) {
                continue;
            }
            app.perform(Choice::Play(Action::Double));
            assert!(matches!(app.game.phase, Phase::Dealer | Phase::Result));
            assert_eq!(app.other_reveal_len, 1);
            assert_eq!(app.other_reveals[0].unwrap().hand_index, 1);
            assert_eq!(app.other_reveals[0].unwrap().seat, app.game.player_seat);
            return;
        }
        panic!("no seeded split-double path");
    }
    #[test]
    fn default_ordinary_visible_single_hand_draw_needs_no_queue() {
        for seed in 1..50 {
            let mut app = App::new(seed);
            app.event(Event::Tap(Key::Blue));
            app.event(Event::Tap(Key::Blue));
            for n in 1..=5 {
                app.tick(n * 200)
            }
            if !app.game.legal(Action::Double) {
                continue;
            }
            app.perform(Choice::Play(Action::Double));
            assert_eq!(app.other_reveal_len, 0);
            return;
        }
        panic!("no ordinary double");
    }
}

#[path = "session_ui.rs"]
pub(crate) mod session_ui;

#[path = "app_resume.rs"]
mod resume_bridge;

#[path = "menu_ui.rs"]
pub(crate) mod menu_ui;
