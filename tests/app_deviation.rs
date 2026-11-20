#[path = "support/navigation.rs"]
mod nav;
use stardust_blackjack::{
    app::App,
    controls::{Event, Key},
    deviation_drills::{IndexId, Origin},
    engine::Phase,
    rules::Profile,
    training::Choice,
};
const B: Event = Event::Tap(Key::Blue);
const G: Event = Event::Tap(Key::Grey);
fn select(a: &mut App, n: usize) {
    for _ in 0..n {
        a.event(G)
    }
    a.event(B)
}
fn focus(a: &mut App, n: usize) {
    for _ in 0..40 {
        if a.focus() == n {
            return;
        }
        a.event(G);
    }
    panic!("missing choice {n} in {}", a.view_name());
}
fn drills(a: &mut App) {
    nav::path(a, &["Training", "Practice", "Deviations"]);
    assert_eq!(a.view_name(), "DeviationPractice");
}
fn configured() -> App {
    let mut a = App::new(7);
    a.drill_target = IndexId::Hard16v10 as u8;
    a.drill_mix = 2;
    a.drill_profile = Profile::SixNoSurrender;
    drills(&mut a);
    a
}
fn live(a: &App) -> (u64, usize, i64, u32, Phase) {
    (
        a.game.rng,
        a.game.position,
        a.game.bankroll,
        a.game.rounds,
        a.game.phase,
    )
}
#[test]
fn synthetic_questions_do_not_advance_or_disclose_live_game_and_first_answer_is_immutable() {
    let mut a = configured();
    let before = live(&a);
    a.event(B);
    assert_eq!(a.view_name(), "DeviationQuestion");
    let q = a.deviation_practice.question().unwrap();
    assert!(a.deviation_practice.reference(q.token).is_none());
    assert_eq!(q.context.count.running, 1);
    a.render();
    assert!(a.deviation_practice.reference(q.token).is_none());
    a.event(B);
    assert_eq!(a.view_name(), "DeviationResult");
    let first = *a.deviation_practice.newest(0).unwrap();
    assert_eq!(
        first.answer,
        Choice::Play(stardust_blackjack::engine::Action::Hit)
    );
    assert!(!first.correct && !first.assisted);
    a.render();
    assert_eq!(a.deviation_practice.newest(0), Some(&first));
    assert_eq!(live(&a), before);
    assert_eq!(a.deviation_practice.stats().first.all.errors, 1);
    assert_eq!(a.training.stats().all.decisions, 0);
}
#[test]
fn missed_replay_keeps_evidence_and_cannot_fix_original_accuracy() {
    let mut a = configured();
    a.event(B);
    a.event(B);
    let first = *a.deviation_practice.newest(0).unwrap();
    a.event(B);
    focus(&mut a, 5);
    a.event(B);
    assert_eq!(a.view_name(), "DeviationQuestion");
    let q = a.deviation_practice.question().unwrap();
    assert_eq!(q.context, first.question.context);
    assert_eq!(
        q.origin(),
        Origin::Replay {
            original_token: first.question.token
        }
    );
    select(&mut a, 1);
    assert_eq!(a.view_name(), "DeviationResult");
    assert_eq!(a.deviation_practice.stats().first.all.errors, 1);
    assert_eq!(a.deviation_practice.stats().first.all.correct, 0);
    assert_eq!(a.deviation_practice.stats().replay.all.correct, 1);
    assert_eq!(a.deviation_practice.history(0), Some(&first));
}
#[test]
fn reference_help_is_gated_sticky_and_rules_alone_do_not_assist() {
    let mut a = configured();
    a.event(B);
    let q = a.deviation_practice.question().unwrap();
    select(&mut a, 3);
    assert_eq!(a.view_name(), "DeviationContext");
    for _ in 0..3 {
        a.render();
        a.event(G)
    }
    assert!(a.deviation_practice.reference(q.token).is_none());
    a.event(B);
    assert_eq!(a.view_name(), "DeviationQuestion");
    focus(&mut a, 1);
    a.event(B);
    assert!(!a.deviation_practice.newest(0).unwrap().assisted);
    a.event(G);
    assert_eq!(a.view_name(), "DeviationQuestion");
    let q = a.deviation_practice.question().unwrap();
    select(&mut a, 4);
    assert_eq!(a.view_name(), "DeviationReference");
    assert!(a.deviation_practice.reference(q.token).is_some());
    a.render();
    a.event(G);
    a.render();
    a.event(B);
    focus(&mut a, 1);
    a.event(B);
    let attempt = a.deviation_practice.newest(0).unwrap();
    assert!(attempt.correct && attempt.assisted);
    assert_eq!(a.deviation_practice.stats().first.all.correct, 1);
    assert_eq!(a.deviation_practice.stats().first.all.assisted, 1);
}
#[test]
fn cancellation_consumes_question_without_scoring_and_modes_do_not_change_its_policy() {
    let mut a = configured();
    a.event(B);
    let q = a.deviation_practice.question().unwrap();
    a.event(Event::Hold(Key::Blue));
    assert_eq!(a.deviation_practice.question(), Some(q));
    a.event(Event::Hold(Key::Grey));
    assert_eq!(a.view_name(), "DeviationPractice");
    assert!(a.deviation_practice.question().is_none());
    assert_eq!(a.deviation_practice.stats().first.all.decisions, 0);
    assert!(!a.deviation_practice.begin(q));
}
#[test]
fn target_and_practice_rules_are_independent_of_the_live_profile() {
    let mut a = App::new(1);
    drills(&mut a);
    select(&mut a, 1);
    assert_eq!(a.view_name(), "DeviationTarget");
    focus(&mut a, IndexId::Soft19v6S17 as usize);
    a.event(B);
    a.event(B);
    assert_eq!(a.view_name(), "Notice");
    assert!(a.deviation_practice.question().is_none());
    a.event(B);
    focus(&mut a, 3);
    a.event(B);
    assert_eq!(a.drill_profile, Profile::SixS17);
    focus(&mut a, 0);
    a.event(B);
    assert_eq!(a.view_name(), "DeviationQuestion");
    assert!(
        !a.deviation_practice
            .question()
            .unwrap()
            .context
            .rules
            .hit_soft_17
    );
    assert_eq!(a.game.rules.profile, Profile::SixH17);
}
#[test]
fn all_drill_views_render_and_history_keeps_original_snapshots() {
    let mut a = configured();
    a.event(B);
    a.event(B);
    let first = *a.deviation_practice.newest(0).unwrap();
    a.event(B);
    for option in [4, 6, 7] {
        if option == 7 {
            drills(&mut a);
            nav::choose(&mut a, "Policy");
        } else {
            nav::path(
                &mut a,
                &[
                    "Training",
                    "Progress",
                    "Deviations",
                    if option == 4 { "Accuracy" } else { "History" },
                ],
            );
        }
        for _ in 0..if option == 4 {
            33
        } else if option == 6 {
            6
        } else {
            4
        } {
            a.render();
            assert!(a.frame.pixels.iter().any(|&p| p != 0));
            a.event(G)
        }
        a.event(B);
        assert_eq!(
            a.view_name(),
            if option == 7 {
                "DeviationPractice"
            } else {
                "DeviationProgress"
            }
        );
    }
    assert_eq!(a.deviation_practice.newest(0), Some(&first));
}
