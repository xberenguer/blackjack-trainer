use stardust_blackjack::{
    app::App,
    controls::{Event, Key},
};
const BLUE: Event = Event::Tap(Key::Blue);
const GREY: Event = Event::Tap(Key::Grey);
pub fn root(a: &mut App) {
    for _ in 0..16 {
        if a.view_name() == "Menu" {
            return;
        }
        a.event(Event::Hold(Key::Grey));
    }
    panic!("cannot reach root from {}", a.view_name());
}
pub fn focus(a: &mut App, label: &str) {
    for _ in 0..32 {
        if a.menu_item() == label {
            return;
        }
        a.event(GREY);
    }
    panic!("{label:?} not visible in {}", a.view_name());
}
pub fn choose(a: &mut App, label: &str) {
    focus(a, label);
    a.event(BLUE);
}
pub fn path(a: &mut App, labels: &[&str]) {
    root(a);
    for label in labels {
        choose(a, label);
    }
}
