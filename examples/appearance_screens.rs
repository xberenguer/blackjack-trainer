//! Same seeded live hand and menu-driven appearance choices on both displays.
use stardust_blackjack::{
    app::App,
    controls::{Event, Key},
    round::{RoundCanvas, RoundRenderer},
};
use std::{
    fs,
    io::{BufWriter, Write},
    path::Path,
};
fn press(a: &mut App, e: Event) {
    a.event(e);
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
    panic!("Missing {label} in {}", a.view_name());
}
fn display(a: &mut App) {
    press(a, Event::Hold(Key::Grey));
    choose(a, "Settings");
    choose(a, "Display");
}
fn table(a: &mut App) {
    press(a, Event::Hold(Key::Grey));
    press(a, Event::Hold(Key::Grey));
    choose(a, "Back to game");
}
fn save(out: &Path, name: &str, a: &mut App, r: &mut RoundRenderer, f: &mut RoundCanvas) {
    a.render();
    let (p, w, h) = if a.round_display {
        r.render(a, f);
        (&f.pixels[..], 466, 466)
    } else {
        (&a.frame.pixels[..], 240, 135)
    };
    let mut out = BufWriter::new(fs::File::create(out.join(format!("{name}.ppm"))).unwrap());
    write!(out, "P6\n{w} {h}\n255\n").unwrap();
    for &c in p {
        out.write_all(&[
            (((c >> 11) & 31) * 255 / 31) as u8,
            (((c >> 5) & 63) * 255 / 63) as u8,
            ((c & 31) * 255 / 31) as u8,
        ])
        .unwrap();
    }
}
fn main() {
    let root = std::env::args()
        .nth(1)
        .unwrap_or("/tmp/blackjack-appearance-v0123".into());
    let mut r = RoundRenderer::new();
    let mut f = Box::new(RoundCanvas::EMPTY);
    for round in [false, true] {
        let out = Path::new(&root).join(if round { "round" } else { "stick" });
        fs::create_dir_all(&out).unwrap();
        let mut a = Box::new(App::new(334));
        a.round_display = round;
        press(&mut a, Event::Tap(Key::Blue));
        press(&mut a, Event::Tap(Key::Blue));
        for t in 1..=30 {
            a.tick(t * 400);
        }
        a.render();
        assert_eq!(a.view_name(), "Table");
        assert_eq!(a.game.phase, stardust_blackjack::engine::Phase::Player);
        assert!(!a.game.hole_visible);
        save(&out, "table-default", &mut a, &mut r, &mut f);
        display(&mut a);
        save(&out, "display", &mut a, &mut r, &mut f);
        choose(&mut a, "Card back");
        assert_eq!(a.view_name(), "CardBackPreview");
        for name in ["gold", "red", "blue", "charcoal"] {
            save(&out, &format!("preview-{name}"), &mut a, &mut r, &mut f);
            press(&mut a, Event::Tap(Key::Grey));
        }
        press(&mut a, Event::Tap(Key::Blue));
        table(&mut a);
        for (color, name) in ["gold", "red", "blue", "charcoal"].iter().enumerate() {
            display(&mut a);
            choose(&mut a, "Card back");
            while a.card_back != color as u8 {
                press(&mut a, Event::Tap(Key::Grey));
            }
            press(&mut a, Event::Tap(Key::Blue));
            table(&mut a);
            save(&out, &format!("table-{name}"), &mut a, &mut r, &mut f);
        }
        for _ in 0..4 {
            display(&mut a);
            choose(&mut a, "Background");
            let color = if round {
                a.background
            } else {
                a.compact_background
            };
            table(&mut a);
            save(&out, &format!("background-{color}"), &mut a, &mut r, &mut f);
        }
    }
}
