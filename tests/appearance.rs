//! Shared back artwork and compact theme regressions; no device I/O.
use stardust_blackjack::{
    app::App,
    assets::Sprite,
    canvas::{Canvas, FELT},
    controls::{Event, Key},
    render::{Renderer, card_back_color},
    round::{RoundCanvas, RoundRenderer},
};
fn run(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}
fn tap(a: &mut App, key: Key) {
    a.event(Event::Tap(key));
    a.render();
}
fn choose(a: &mut App, label: &str) {
    for _ in 0..32 {
        if a.menu_item() == label {
            tap(a, Key::Blue);
            return;
        }
        tap(a, Key::Grey);
    }
    panic!("Missing {label}");
}
fn play() -> Box<App> {
    let mut a = Box::new(App::new(334));
    tap(&mut a, Key::Blue);
    tap(&mut a, Key::Blue);
    for t in 1..=30 {
        a.tick(t * 400);
    }
    a.render();
    a
}
#[test]
fn blue_default_is_unchanged_and_black_preserves_opaque_art() {
    run(|| {
        let mut a = play();
        assert!(!a.compact_black);
        assert_eq!(a.card_back, 0);
        let blue = a.frame.pixels.to_vec();
        a.compact_black = true;
        a.render();
        assert!(blue.iter().filter(|p| **p == FELT).count() > 3000);
        for (&before, &after) in blue.iter().zip(a.frame.pixels.iter()) {
            if before == FELT {
                assert_eq!(after, 0, "Bare felt must be true black");
            }
            if [0xffff, 0xffe0, 0].contains(&before) {
                assert_eq!(after, before, "Opaque text and ink must keep their colours");
            }
        }
        // Antialias edges naturally blend into the new felt. Opaque card art
        // must remain exact, rather than being recoloured with the background.
        let face = Sprite::at(a.game.hands[0].cards()[0].id() as usize).unwrap();
        for y in 0..face.height {
            for x in 0..face.width {
                if let Some((_, alpha)) = face.rgba565(y * face.width + x)
                    && alpha == 255
                {
                    let i = (53 + y) * 240 + 3 + x;
                    assert_eq!(blue[i], a.frame.pixels[i], "Opaque face artwork changed");
                }
            }
        }
        a.compact_black = false;
        a.render();
        assert_eq!(
            blue.as_slice(),
            a.frame.pixels.as_slice(),
            "Blue selection must exactly restore approved pixels"
        );
        let mut r = RoundRenderer::new();
        let mut f = Box::new(RoundCanvas::EMPTY);
        r.render(&a, &mut f);
        let native = f.pixels.to_vec();
        a.compact_black = true;
        a.render();
        r.render(&a, &mut f);
        assert_eq!(
            native.as_slice(),
            f.pixels.as_slice(),
            "Compact appearance cannot override native round theme"
        );
    });
}
#[test]
fn shared_back_colours_preserve_faces_and_gold_is_exact_original_art() {
    let mut r = Renderer::default();
    let mut gold = Canvas::default();
    gold.clear(FELT);
    r.card(&mut gold, (0, 0), None, false);
    let mut original = Canvas::default();
    original.clear(FELT);
    r.sprite(&mut original, (0, 0), Sprite::at(104).unwrap(), None);
    assert_eq!(gold.pixels, original.pixels);
    let mut faces = Vec::new();
    let mut backs = Vec::new();
    for color in 0..3 {
        r.set_appearance(false, color);
        let mut c = Canvas::default();
        c.clear(FELT);
        r.card(
            &mut c,
            (0, 0),
            Some(stardust_blackjack::cards::Card::from_id(12).unwrap()),
            false,
        );
        faces.push(c.pixels.to_vec());
        r.card(&mut c, (80, 0), None, false);
        backs.push(c.pixels.to_vec());
    }
    assert_eq!(faces[0], faces[1]);
    assert_eq!(faces[1], faces[2]);
    assert_ne!(backs[0], backs[1]);
    assert_ne!(backs[1], backs[2]);
}
#[test]
fn live_preview_both_targets_cycles_shared_ink_and_returns_to_same_display_row() {
    run(|| {
        let source = Sprite::at(104).unwrap();
        for round in [false, true] {
            let mut a = play();
            a.round_display = round;
            a.event(Event::Hold(Key::Grey));
            choose(&mut a, "Settings");
            choose(&mut a, "Display");
            choose(&mut a, "Card back");
            assert_eq!(a.view_name(), "CardBackPreview");
            let mut r = RoundRenderer::new();
            let mut f = Box::new(RoundCanvas::EMPTY);
            for color in 0..3 {
                assert_eq!(a.card_back, color);
                a.render();
                r.render(&a, &mut f);
                let (x, y, w, h) = if round {
                    (163, 154, 140, 204)
                } else {
                    (24, 26, 60, 88)
                };
                for dy in 0..h {
                    for dx in 0..w {
                        let sy = dy * source.height / h;
                        let sx = dx * source.width / w;
                        if let Some((c, alpha)) = source.rgba565(sy * source.width + sx)
                            && alpha == 255
                        {
                            let actual = if round {
                                f.pixels[(y + dy) * 466 + x + dx]
                            } else {
                                a.frame.pixels[(y + dy) * 240 + x + dx]
                            };
                            assert_eq!(
                                actual,
                                card_back_color(c, color),
                                "Same Stardust pixels must be used in preview and table"
                            );
                        }
                    }
                }
                tap(&mut a, Key::Grey);
            }
            tap(&mut a, Key::Blue);
            assert_eq!((a.view_name(), a.focus()), ("Display", 4));
        }
    });
}
