use stardust_blackjack::{
    app::App,
    controls::{Event, Key},
    deviation_drills::IndexId,
    rules::Profile,
};
use std::{fs, path::PathBuf};
const B: Event = Event::Tap(Key::Blue);
const G: Event = Event::Tap(Key::Grey);
fn focus(a: &mut App, n: usize) {
    let mut guard = 0;
    while a.focus() != n {
        a.event(G);
        guard += 1;
        assert!(guard < 40)
    }
}
fn save(a: &mut App, path: &std::path::Path, name: &str) {
    a.render();
    let mut out = b"P6\n240 135\n255\n".to_vec();
    for &c in &a.frame.pixels {
        out.extend_from_slice(&[
            (((c >> 11) & 31) * 255 / 31) as u8,
            (((c >> 5) & 63) * 255 / 63) as u8,
            ((c & 31) * 255 / 31) as u8,
        ]);
    }
    fs::write(path.join(format!("{name}.ppm")), out).unwrap();
}
fn main() {
    let path = std::env::args().nth(1).map_or_else(
        || PathBuf::from("validation/deviation-screens"),
        PathBuf::from,
    );
    fs::create_dir_all(&path).unwrap();
    let mut a = App::new(7);
    a.drill_target = IndexId::Hard16v10 as u8;
    a.drill_mix = 2;
    a.drill_profile = Profile::SixNoSurrender;
    a.event(Event::Hold(Key::Grey));
    while a.menu_item() != "Training" {
        a.event(G)
    }
    a.event(B);
    focus(&mut a, 9);
    a.event(B);
    save(&mut a, &path, "00-practice");
    focus(&mut a, 1);
    a.event(B);
    save(&mut a, &path, "01-target");
    a.event(B);
    focus(&mut a, 0);
    a.event(B);
    save(&mut a, &path, "02-question");
    focus(&mut a, 3);
    a.event(B);
    for i in 0..3 {
        save(&mut a, &path, &format!("03-context-{i}"));
        a.event(G)
    }
    a.event(B);
    focus(&mut a, 4);
    a.event(B);
    for i in 0..2 {
        save(&mut a, &path, &format!("04-reference-{i}"));
        a.event(G)
    }
    a.event(B);
    focus(&mut a, 1);
    a.event(B);
    save(&mut a, &path, "05-assisted-result");
    a.event(B);
    focus(&mut a, 4);
    a.event(B);
    for i in 0..3 {
        save(&mut a, &path, &format!("06-stats-{i}"));
        a.event(G)
    }
    a.event(B);
    focus(&mut a, 6);
    a.event(B);
    for i in 0..6 {
        save(&mut a, &path, &format!("07-history-{i}"));
        a.event(G)
    }
    a.event(B);
    focus(&mut a, 7);
    a.event(B);
    for i in 0..4 {
        save(&mut a, &path, &format!("08-policy-{i}"));
        a.event(G)
    }
    a.event(B);
    a.drill_target = IndexId::Insurance as u8;
    focus(&mut a, 0);
    a.event(B);
    save(&mut a, &path, "09-insurance");
    focus(&mut a, 4);
    a.event(B);
    a.drill_target = IndexId::Hard16v10 as u8;
    a.drill_profile = Profile::SixH17;
    focus(&mut a, 0);
    a.event(B);
    assert_eq!(
        a.deviation_practice
            .question()
            .unwrap()
            .context
            .hand
            .count(),
        3
    );
    save(&mut a, &path, "10-ls-three-card");
    println!("Rendered synthetic drill screens in {}", path.display());
}
