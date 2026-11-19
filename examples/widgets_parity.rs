use stardust_blackjack::{canvas::Canvas, render::Renderer, widgets::Widgets};
use std::io::{self, BufRead};
const ITEMS: [&str; 14] = [
    "DOUBLE",
    "SPLIT",
    "SURRENDER",
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
    "Color up & leave",
];
fn main() {
    let mut canvas = Canvas::default();
    let mut renderer = Renderer::default();
    let mut count = 0;
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let f: Vec<_> = line.split_whitespace().collect();
        let mut widgets = Widgets {
            frame: &mut canvas,
            renderer: &mut renderer,
        };
        match f[0] {
            "L" => {
                widgets.title("MENU", Some("v0.5.0-dev.1"), false);
                widgets.list(
                    &ITEMS[..f[1].parse::<usize>().unwrap()],
                    f[2].parse().unwrap(),
                    if f[3] == "1" {
                        Some("Optional aid; Guide defaults RC+TC")
                    } else {
                        None
                    },
                );
            }
            "C" => widgets.cut(f[1].parse().unwrap(), f[2].parse().unwrap()),
            _ => unreachable!(),
        }
        let hash = canvas
            .pixels
            .iter()
            .flat_map(|p| p.to_le_bytes())
            .fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619));
        assert_eq!(
            hash,
            f.last().unwrap().parse::<u32>().unwrap(),
            "Widget {line}"
        );
        count += 1;
    }
    println!(
        "PASS Rust legacy cut selectors: {count} pixel-identical six/eight-deck layouts; current 0.10.1 menus visually qualified separately"
    );
}
