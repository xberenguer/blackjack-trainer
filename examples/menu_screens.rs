//! Current menu hierarchy captured through public button inputs on both targets.
use stardust_blackjack::{
    app::App,
    controls::{Event, Key},
    round::{HEIGHT, RoundCanvas, RoundRenderer, WIDTH},
};
use std::{
    fs,
    io::{BufWriter, Write},
    path::Path,
};

fn press(a: &mut App, event: Event) {
    a.event(event);
    a.render();
}
fn choose(a: &mut App, label: &str) {
    for _ in 0..32 {
        if a.menu_item() == label {
            press(a, Event::Tap(Key::Blue));
            return;
        }
        press(a, Event::Tap(Key::Grey));
    }
    panic!("Cannot reach {label} from {}", a.view_name());
}
fn ppm(path: &Path, pixels: &[u16], w: usize, h: usize) {
    let mut out = BufWriter::new(fs::File::create(path).unwrap());
    write!(out, "P6\n{w} {h}\n255\n").unwrap();
    for &p in pixels {
        out.write_all(&[
            (((p >> 11) & 31) * 255 / 31) as u8,
            (((p >> 5) & 63) * 255 / 63) as u8,
            ((p & 31) * 255 / 31) as u8,
        ])
        .unwrap();
    }
}
fn save(out: &Path, name: &str, a: &mut App, r: &mut RoundRenderer, f: &mut RoundCanvas) {
    a.render();
    if a.round_display {
        r.render(a, f);
        ppm(&out.join(format!("{name}.ppm")), &f.pixels, WIDTH, HEIGHT);
        println!("round/{name}: {} {:?}", a.view_name(), r.info);
    } else {
        ppm(&out.join(format!("{name}.ppm")), &a.frame.pixels, 240, 135);
        println!("stick/{name}: {}", a.view_name());
    }
}
fn main() {
    let root = std::env::args()
        .nth(1)
        .unwrap_or("/tmp/blackjack-menu-screens".into());
    let paths: &[(&str, &[&str])] = &[
        ("menu-main", &[]),
        ("training", &["Training"]),
        ("practice", &["Training", "Practice"]),
        ("count-practice", &["Training", "Practice", "Counting"]),
        ("bet-practice", &["Training", "Practice", "Bet sizing"]),
        (
            "deviation-practice",
            &["Training", "Practice", "Deviations"],
        ),
        ("progress", &["Training", "Progress"]),
        ("hand-progress", &["Training", "Progress", "Hand play"]),
        ("advice", &["Training", "Advice & EV"]),
        ("counts", &["Training", "Live counts"]),
        ("table", &["Table"]),
        ("session", &["Session"]),
        ("session-overview", &["Session", "Overview"]),
        ("bankroll", &["Session", "Bankroll"]),
        ("settings", &["Settings"]),
        ("sound", &["Settings", "Sound"]),
        ("categories", &["Settings", "Sound", "Categories"]),
        ("test-sounds", &["Settings", "Sound", "Test sounds"]),
        ("display", &["Settings", "Display"]),
        ("help", &["Help"]),
        ("modes", &["Help", "Training modes"]),
        ("new-session-confirm", &["Session", "New session"]),
        (
            "reset-training-confirm",
            &["Training", "Progress", "Reset training stats"],
        ),
    ];
    let mut renderer = RoundRenderer::new();
    let mut frame = Box::new(RoundCanvas::EMPTY);
    for round in [false, true] {
        let out = Path::new(&root).join(if round { "round" } else { "stick" });
        fs::create_dir_all(&out).unwrap();
        for (name, path) in paths {
            let mut a = Box::new(App::new(334));
            a.round_display = round;
            press(&mut a, Event::Hold(Key::Grey));
            for label in *path {
                choose(&mut a, label);
            }
            save(&out, name, &mut a, &mut renderer, &mut frame);
            if *name == "session-overview" {
                press(&mut a, Event::Tap(Key::Grey));
                save(
                    &out,
                    "session-overview-1",
                    &mut a,
                    &mut renderer,
                    &mut frame,
                );
            }
            if *name == "modes" {
                for page in 1..4 {
                    press(&mut a, Event::Tap(Key::Grey));
                    save(
                        &out,
                        &format!("modes-{page}"),
                        &mut a,
                        &mut renderer,
                        &mut frame,
                    );
                }
            }
        }
        let mut walk = Box::new(App::new(334));
        walk.round_display = round;
        press(&mut walk, Event::Hold(Key::Grey));
        save(&out, "walk-0-main", &mut walk, &mut renderer, &mut frame);
        for (i, label) in ["Settings", "Sound", "Categories"].iter().enumerate() {
            choose(&mut walk, label);
            save(
                &out,
                &format!("walk-{}", i + 1),
                &mut walk,
                &mut renderer,
                &mut frame,
            );
        }
        for i in 4..7 {
            press(&mut walk, Event::Hold(Key::Grey));
            save(
                &out,
                &format!("walk-{i}"),
                &mut walk,
                &mut renderer,
                &mut frame,
            );
        }
    }
}
