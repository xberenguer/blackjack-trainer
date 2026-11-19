//! Reproducible publication-only demo frames from the actual native renderer.
//! Run with --features validation. No device, save file, private checkpoint or I/O
//! adapter is used. Fixed benchmark cases and all scores are synthetic examples.
#[cfg(feature = "validation")]
mod gallery {
    use stardust_blackjack::{
        app::App,
        cards::Outcome,
        controls::{Event, Key},
        deviation_drills::IndexId,
        engine::Phase,
        resume_ui::{BootChooser, Preference},
        rules::Profile,
        training::{Mode, Policy},
    };
    use std::{
        fs,
        io::{BufWriter, Write},
        path::{Path, PathBuf},
    };
    const B: Event = Event::Tap(Key::Blue);
    const G: Event = Event::Tap(Key::Grey);
    const HG: Event = Event::Hold(Key::Grey);
    struct Demo {
        a: Box<App>,
        now: u32,
        seed: u64,
    }
    impl Demo {
        fn new(seed: u64) -> Self {
            Self {
                a: Box::new(App::new(seed)),
                now: 0,
                seed,
            }
        }
        fn fixed(case: u8, mode: Mode) -> Self {
            let mut a = Box::new(App::EMPTY);
            assert!(a.prepare_fixed_benchmark(case, mode));
            Self {
                a,
                now: 0,
                seed: [334, 205, 37, 19, 86, 165][case as usize],
            }
        }
        fn event(&mut self, e: Event) {
            self.a.event(e);
            self.a.render();
        }
        fn tick(&mut self, ms: u32) {
            self.now = self.now.wrapping_add(ms);
            self.a.tick(self.now);
            self.a.render();
        }
        fn focus(&mut self, n: usize) {
            let view = self.a.view_name().to_owned();
            for _ in 0..150 {
                if self.a.focus() == n {
                    return;
                }
                self.event(G);
                assert_eq!(self.a.view_name(), view)
            }
            panic!("focus{n} {}", self.a.view_name())
        }
        fn select(&mut self, n: usize) {
            self.focus(n);
            self.event(B);
        }
        fn menu(&mut self) {
            for _ in 0..5 {
                if self.a.view_name() == "Menu" {
                    return;
                }
                self.event(HG)
            }
            panic!("menu {}", self.a.view_name())
        }
        fn menu_item(&mut self, label: &str) {
            self.menu();
            for _ in 0..30 {
                if self.a.menu_item().eq_ignore_ascii_case(label) {
                    self.event(B);
                    return;
                }
                self.event(G)
            }
            panic!("menu item{label}")
        }
        fn training(&mut self, n: usize) {
            self.menu_item("Training");
            self.select(n);
        }
        fn quiet(&mut self) {
            for _ in 0..80 {
                if !self.a.is_busy() {
                    return;
                }
                self.tick(400)
            }
            panic!("busy {:?}", self.a.game.phase)
        }
        fn first_hand(&mut self) {
            assert_eq!(self.a.view_name(), "Cut");
            self.event(B);
            self.event(B);
            self.quiet();
        }
        fn settle(&mut self) {
            for _ in 0..100 {
                if self.a.view_name() == "CoachCheck" {
                    self.select(0);
                } else if self.a.is_busy() {
                    self.tick(400);
                } else if self.a.game.phase == Phase::Result {
                    if self.a.training.mode == Mode::Test {
                        self.tick(1000);
                    }
                    return;
                } else if self.a.game.phase == Phase::Player
                    || self.a.game.phase == Phase::Insurance
                {
                    self.event(B)
                } else {
                    self.tick(400)
                }
            }
            panic!("settle {:?}", self.a.game.phase)
        }
    }
    struct Gallery {
        dir: PathBuf,
        rows: Vec<String>,
    }
    impl Gallery {
        fn new(dir: PathBuf) -> Self {
            fs::create_dir_all(&dir).unwrap();
            Self {
                dir,
                rows: vec!["slug\tview\tseed\tsynthetic".into()],
            }
        }
        fn pixels(&mut self, d: &Demo, slug: &str, view: &str) {
            let mut file =
                BufWriter::new(fs::File::create(self.dir.join(format!("{slug}.ppm"))).unwrap());
            file.write_all(b"P6\n240 135\n255\n").unwrap();
            for &c in &d.a.frame.pixels {
                file.write_all(&[
                    (((c >> 11) & 31) * 255 / 31) as u8,
                    (((c >> 5) & 63) * 255 / 63) as u8,
                    ((c & 31) * 255 / 31) as u8,
                ])
                .unwrap();
            }
            self.rows.push(format!("{slug}\t{view}\t{}\ttrue", d.seed));
        }
        fn shot(&mut self, d: &mut Demo, slug: &str, expected: &str) {
            assert_eq!(d.a.view_name(), expected, "{slug}");
            d.a.render();
            self.pixels(d, slug, expected);
        }
        fn pages(&mut self, d: &mut Demo, prefix: &str, expected: &str, n: usize) {
            for i in 0..n {
                self.shot(d, &format!("{prefix}-{i:02}"), expected);
                d.event(G);
            }
        }
        fn finish(&self) {
            fs::write(self.dir.join("frames.tsv"), self.rows.join("\n") + "\n").unwrap();
            println!(
                "Rendered{} actual native frames; all synthetic; no device/save input",
                self.rows.len() - 1
            );
        }
    }
    pub fn run() {
        let dir = std::env::args()
            .nth(1)
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new("validation/publication").to_path_buf());
        let mut g = Gallery::new(dir);
        // Fresh defaults, cut/burn/deal and a genuine timed card sequence.
        let mut d = Demo::new(2);
        g.shot(&mut d, "cut", "Cut");
        d.event(G);
        g.shot(&mut d, "cut-moved", "Cut");
        d.event(B);
        g.shot(&mut d, "burn", "Burn");
        d.event(B);
        for i in 0..8 {
            g.shot(&mut d, &format!("anim-deal-{i:02}"), "Table");
            d.tick(130);
        }
        d.quiet();
        g.shot(&mut d, "table-player", "Table");
        d.menu();
        g.shot(&mut d, "menu", "Menu");
        d.menu_item("Return to game");
        d.settle();
        g.shot(&mut d, "table-result", "Table");
        // Fixed publicly exposed diagnostic cases are existing validation APIs.
        let mut b = Demo::fixed(3, Mode::Off);
        g.shot(&mut b, "table-insurance", "Table");
        b.event(G);
        g.shot(&mut b, "table-insurance-yes", "Table");
        b.event(B);
        b.settle();
        let mut b = Demo::fixed(0, Mode::Off);
        b.menu_item("Split");
        b.quiet();
        g.shot(&mut b, "table-split", "Table");
        b.settle();
        let mut b = Demo::fixed(2, Mode::Off);
        b.menu_item("Double");
        g.shot(&mut b, "table-double", "Table");
        b.settle();
        let mut b = Demo::fixed(0, Mode::Off);
        b.menu_item("Surrender");
        b.quiet();
        g.shot(&mut b, "table-surrender", "Table");
        let mut b = Demo::fixed(2, Mode::Guide);
        g.shot(&mut b, "table-guide", "Table");
        b.training(7);
        g.shot(&mut b, "hint", "TrainingHint");
        let mut b = Demo::fixed(0, Mode::Coach);
        b.a.training.pause_on_mistake = true;
        b.event(B);
        assert_eq!(b.a.view_name(), "CoachCheck");
        g.shot(&mut b, "coach-check", "CoachCheck");
        b.select(1);
        b.quiet();
        g.shot(&mut b, "table-coach-continued", "Table");
        // Distinct publicly visible results found from deterministic synthetic deals.
        for (label, outcome) in [
            ("blackjack", Outcome::Blackjack),
            ("push", Outcome::Push),
            ("win", Outcome::Win),
            ("loss", Outcome::Loss),
            ("bust", Outcome::Bust),
        ] {
            let mut found = false;
            for seed in 1..300 {
                let mut b = Demo::new(seed);
                b.first_hand();
                if label == "bust" {
                    for _ in 0..20 {
                        if b.a.game.phase != Phase::Player {
                            break;
                        }
                        b.event(G);
                        b.quiet();
                    }
                }
                b.settle();
                if b.a.game.hands[0].outcome == outcome {
                    g.shot(&mut b, &format!("table-{label}"), "Table");
                    found = true;
                    break;
                }
            }
            assert!(found, "result{label}");
        }
        // Between-round menus, custom amount, notices and exit flow.
        let mut b = Demo::new(11);
        b.menu_item("Select wager");
        g.shot(&mut b, "wager", "Wager");
        b.select(4);
        g.shot(&mut b, "amount", "Amount");
        b.event(B);
        g.shot(&mut b, "amount-edited", "Amount");
        let mut b = Demo::new(11);
        b.menu_item("Review cards");
        g.shot(&mut b, "notice", "Notice");
        let mut b = Demo::new(11);
        b.menu_item("Session totals");
        g.shot(&mut b, "session", "Session");
        b.select(1);
        g.pages(&mut b, "bankroll", "Bankroll", 2);
        b.event(B);
        b.select(2);
        g.shot(&mut b, "refill", "Refill");
        let mut b = Demo::new(11);
        b.menu_item("Color up & leave");
        g.shot(&mut b, "leave", "Leave");
        b.select(1);
        g.shot(&mut b, "summary", "Summary");
        // Profile and seating editor, then genuine initial and later tablemate reveals.
        let mut b = Demo::new(11);
        b.menu_item("Table profile");
        g.shot(&mut b, "profile", "TableProfile");
        b.select(0);
        g.shot(&mut b, "profile-preset", "ProfilePreset");
        b.event(B);
        b.select(1);
        g.shot(&mut b, "profile-minimum", "ProfileAmount");
        b.select(4);
        b.select(2);
        g.shot(&mut b, "profile-maximum", "ProfileAmount");
        b.select(4);
        b.select(3);
        g.shot(&mut b, "profile-depth", "ProfileDepth");
        b.select(6);
        b.select(4);
        b.event(B);
        b.event(B);
        b.select(1);
        b.event(B);
        g.shot(&mut b, "table-seats", "TableSeats");
        b.select(2);
        b.first_hand();
        g.shot(&mut b, "tablemates-player", "Table");
        b.menu_item("Review cards");
        g.shot(&mut b, "seat-cards", "SeatCards");
        b.event(B);
        b.menu_item("Return to game");
        // A separate player-first branch makes post-player tablemate play visible.
        let mut mates = (1..30)
            .find_map(|seed| {
                let mut demo = Demo::new(seed);
                assert!(demo.a.game.configure_seats(2, 0));
                demo.first_hand();
                (demo.a.game.phase == Phase::Player
                    && demo.a.game.others[..2]
                        .iter()
                        .any(|seat| seat.hands[0].total() < 17))
                .then_some(demo)
            })
            .expect("synthetic tablemate-draw case");
        mates.event(B);
        for i in 0..10 {
            g.shot(&mut mates, &format!("anim-tablemates-{i:02}"), "Table");
            mates.tick(400);
        }
        // General information and actual exposed-card review.
        let mut b = Demo::fixed(2, Mode::Off);
        b.menu_item("Review cards");
        g.shot(&mut b, "cards", "Cards");
        b.menu_item("Shoe information");
        g.shot(&mut b, "shoe-info", "ShoeInfo");
        b.menu_item("Table rules");
        g.pages(&mut b, "rules", "Rules", 7);
        b.menu_item("Controls");
        g.pages(&mut b, "controls", "Controls", 4);
        // Full nested settings, keeping actual fresh 20%/Fast defaults in primary shots.
        let mut b = Demo::new(7);
        b.menu_item("Settings");
        g.shot(&mut b, "settings", "Settings");
        b.focus(5);
        g.shot(&mut b, "settings-resume", "Settings");
        b.select(1);
        g.shot(&mut b, "sound", "Sound");
        b.focus(8);
        g.shot(&mut b, "sound-auditions", "Sound");
        b.select(1);
        g.shot(&mut b, "audio-volume", "AudioVolume");
        for i in 0..5 {
            g.shot(&mut b, &format!("anim-volume-{i:02}"), "AudioVolume");
            b.event(G);
        }
        b.event(HG);
        b.event(HG);
        b.select(0);
        g.shot(&mut b, "display", "Display");
        b.select(0);
        g.shot(&mut b, "brightness", "Brightness");
        b.event(G);
        g.shot(&mut b, "brightness-edited", "Brightness");
        b.event(HG);
        b.event(HG);
        b.select(2);
        g.shot(&mut b, "settings-normal-speed", "Settings");
        // Training scores/reviews/EV are generated by real synthetic first decisions.
        let mut s = Demo::new(21);
        s.a.policy = Policy::Basic;
        s.a.training.mode = Mode::Test;
        let mut captured_review = false;
        for _ in 0..32 {
            if s.a.view_name() == "RoundReview" {
                s.event(B)
            }
            if s.a.view_name() == "Cut" {
                s.event(B)
            }
            if s.a.view_name() == "Burn" || matches!(s.a.game.phase, Phase::Ready | Phase::Result) {
                s.event(B)
            }
            s.quiet();
            s.settle();
            if s.a.view_name() == "RoundReview" && !captured_review {
                g.shot(&mut s, "round-review", "RoundReview");
                s.event(G);
                g.shot(&mut s, "round-review-decision", "RoundReview");
                captured_review = true;
            }
            if s.a.view_name() == "Refill" {
                s.event(B)
            }
        }
        assert!(captured_review);
        if s.a.view_name() == "RoundReview" {
            s.event(B)
        }
        s.training(3);
        g.pages(&mut s, "training-stats", "TrainingStats", 5);
        s.event(B);
        s.select(4);
        g.shot(&mut s, "training-review", "TrainingReview");
        s.event(B);
        s.select(6);
        g.shot(&mut s, "ev-review", "EVReview");
        let mut b = Demo::new(7);
        b.menu_item("Training");
        g.shot(&mut b, "training", "Training");
        let mut b = Demo::fixed(2, Mode::Off);
        b.a.policy = Policy::Basic;
        b.training(5);
        g.pages(&mut b, "action-ev", "ActionEV", 3);
        // Count exercises use current public context; references are never forged.
        let mut c = Demo::new(23);
        c.first_hand();
        c.settle();
        c.training(2);
        g.shot(&mut c, "count-practice", "CountPractice");
        for (row, label) in [(0, "running"), (1, "decks"), (2, "true-count")] {
            c.select(row);
            g.shot(&mut c, &format!("count-answer-{label}"), "CountAnswer");
            c.event(Event::SubmitCount);
            g.shot(&mut c, &format!("count-result-{label}"), "CountResult");
            c.event(B);
        }
        c.select(4);
        g.pages(&mut c, "count-stats", "CountStats", 4);
        c.event(B);
        c.select(5);
        g.shot(&mut c, "count-history", "CountHistory");
        c.event(B);
        c.select(3);
        g.shot(&mut c, "count-info", "CountInfo");
        // Example-ramp wager training is independent from normal betting.
        let mut w = Demo::new(29);
        w.event(B);
        w.training(8);
        g.shot(&mut w, "wager-practice", "WagerPractice");
        w.select(4);
        g.shot(&mut w, "wager-policy", "WagerPolicy");
        w.select(5);
        w.select(1);
        g.shot(&mut w, "wager-answer", "WagerAnswer");
        w.select(6);
        g.shot(&mut w, "wager-help", "WagerHelp");
        w.event(B);
        w.select(4);
        g.shot(&mut w, "wager-result", "WagerResult");
        w.event(G);
        g.shot(&mut w, "wager-result-options", "WagerResult");
        w.select(3);
        w.select(2);
        g.pages(&mut w, "wager-stats", "WagerStats", 7);
        w.event(B);
        w.select(3);
        g.shot(&mut w, "wager-history", "WagerHistory");
        let mut wrong = Demo::new(29);
        wrong.event(B);
        wrong.training(8);
        wrong.select(1);
        wrong.select(0);
        wrong.select(4);
        g.shot(&mut wrong, "wager-result-mistake", "WagerResult");
        assert!(!wrong.a.wager_practice.newest(0).unwrap().correct);
        wrong.event(B);
        wrong.quiet();
        g.shot(&mut wrong, "wager-played-choice", "Table");
        // Synthetic index practice: explicit target/boundary/rules, helped and unassisted.
        let mut x = Demo::new(7);
        x.a.drill_target = IndexId::Hard16v10 as u8;
        x.a.drill_mix = 2;
        x.a.drill_profile = Profile::SixNoSurrender;
        x.training(9);
        g.shot(&mut x, "deviation-practice", "DeviationPractice");
        x.select(1);
        g.shot(&mut x, "deviation-target", "DeviationTarget");
        x.event(B);
        x.select(0);
        g.shot(&mut x, "deviation-question", "DeviationQuestion");
        x.select(3);
        g.pages(&mut x, "deviation-context", "DeviationContext", 3);
        x.event(B);
        x.select(4);
        g.pages(&mut x, "deviation-reference", "DeviationReference", 2);
        x.event(B);
        x.select(1);
        g.shot(&mut x, "deviation-result", "DeviationResult");
        x.event(B);
        x.select(4);
        g.pages(&mut x, "deviation-stats", "DeviationStats", 33);
        x.event(B);
        x.select(6);
        g.pages(&mut x, "deviation-history", "DeviationHistory", 6);
        x.event(B);
        x.select(7);
        g.pages(&mut x, "deviation-policy", "DeviationPolicy", 4);
        x.event(B);
        x.a.drill_target = IndexId::Insurance as u8;
        x.select(0);
        g.shot(&mut x, "deviation-insurance", "DeviationQuestion");
        x.select(4);
        x.a.drill_target = IndexId::Hard16v10 as u8;
        x.a.drill_profile = Profile::SixH17;
        x.select(0);
        g.shot(&mut x, "deviation-three-cards", "DeviationQuestion");
        let mut replay = Demo::new(41);
        replay.a.drill_target = IndexId::Hard16v10 as u8;
        replay.a.drill_mix = 2;
        replay.a.drill_profile = Profile::SixNoSurrender;
        replay.training(9);
        replay.select(0);
        replay.select(0);
        assert_eq!(replay.a.deviation_practice.stats().first.all.errors, 1);
        replay.event(B);
        replay.select(5);
        g.shot(&mut replay, "deviation-replay", "DeviationQuestion");
        replay.select(1);
        g.shot(&mut replay, "deviation-replay-result", "DeviationResult");
        assert_eq!(replay.a.deviation_practice.stats().replay.all.correct, 1);
        replay.event(B);
        replay.select(4);
        for _ in 0..32 {
            replay.event(G);
        }
        g.shot(&mut replay, "deviation-replay-stats", "DeviationStats");
        // Session analysis: authentic synthetic settled ledgers and first-answer history.
        s.menu_item("Session totals");
        s.select(3);
        g.shot(&mut s, "analysis-menu", "AnalysisMenu");
        for (row, label, view, n) in [
            (0, "analysis-summary", "AnalysisSummary", 4),
            (1, "analysis-graph", "AnalysisGraph", 1),
            (2, "analysis-rounds", "AnalysisRounds", 2),
            (3, "analysis-accuracy", "AnalysisAccuracy", 12),
            (4, "analysis-costs", "AnalysisCosts", 3),
            (5, "analysis-decisions", "AnalysisDecisions", 2),
            (6, "analysis-assumptions", "AnalysisAssumptions", 3),
        ] {
            s.select(row);
            g.pages(&mut s, label, view, n);
            s.event(B);
        }
        let mut hilo = Demo::new(2);
        hilo.a.training.mode = Mode::Test;
        hilo.first_hand();
        hilo.settle();
        hilo.menu_item("Session totals");
        hilo.select(3);
        hilo.select(4);
        hilo.event(G);
        hilo.event(G);
        g.shot(
            &mut hilo,
            "analysis-costs-hilo-unavailable",
            "AnalysisCosts",
        );
        // Recovery layouts are shared draw-only APIs, not real saved or failed sessions.
        let mut r = Demo::new(31);
        let chooser = BootChooser::new(Preference::from_enabled(true), true, false);
        r.a.render_resume_chooser(&chooser);
        g.pixels(&r, "resume-chooser", "RecoveryChooser");
        r.a.render_resume_read_error_choice(0);
        g.pixels(&r, "read-retry", "RecoveryReadRetry");
        r.a.render_resume_storage_error_choice(0);
        g.pixels(&r, "save-retry", "RecoverySaveRetry");
        r.a.render_resume_storage_error_choice(1);
        g.pixels(&r, "save-retry-fresh", "RecoverySaveRetry");
        r.a.render();
        r.a.render_unsaved_banner();
        g.pixels(&r, "unsaved", "RecoveryUnsaved");
        g.finish();
    }
}

#[cfg(feature = "validation")]
fn main() {
    gallery::run();
}

#[cfg(not(feature = "validation"))]
fn main() {
    eprintln!("Run this synthetic gallery with --features validation");
}
