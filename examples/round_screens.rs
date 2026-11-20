//! Synthetic layout examples using the real Rust renderer, not frequency evidence.
#[cfg(feature = "validation")]
use stardust_blackjack::training::Mode;
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
fn save(out: &Path, name: &str, a: &mut App, r: &mut RoundRenderer, f: &mut RoundCanvas) {
    a.round_display = true;
    a.render();
    r.render(a, f);
    let mut w = BufWriter::new(fs::File::create(out.join(format!("{name}.ppm"))).unwrap());
    write!(w, "P6\n{WIDTH} {HEIGHT}\n255\n").unwrap();
    for p in f.pixels.iter().copied() {
        let red = ((p >> 11) & 31) as u8;
        let green = ((p >> 5) & 63) as u8;
        let blue = (p & 31) as u8;
        w.write_all(&[
            ((red as u16) * 255 / 31) as u8,
            ((green as u16) * 255 / 63) as u8,
            ((blue as u16) * 255 / 31) as u8,
        ])
        .unwrap();
    }
    println!("{name} {:?}", r.info);
}
fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or("/tmp/blackjack-round".into());
    let out = Path::new(&out);
    fs::create_dir_all(out).unwrap();
    let mut a = Box::new(App::new(334));
    let mut r = RoundRenderer::new();
    let mut f = Box::new(RoundCanvas::EMPTY);
    save(out, "cut", &mut a, &mut r, &mut f);
    a.event(Event::Hold(Key::Grey));
    save(out, "menu", &mut a, &mut r, &mut f);
    for _ in 0..6 {
        a.event(Event::Tap(Key::Grey));
    }
    save(out, "menu-bottom", &mut a, &mut r, &mut f);
    let mut menus = Box::new(App::new(334));
    menus.round_display = true;
    menus.event(Event::Hold(Key::Grey));
    for _ in 0..5 {
        menus.event(Event::Tap(Key::Grey));
    }
    save(out, "menu-last-full", &mut menus, &mut r, &mut f);
    menus.event(Event::Tap(Key::Grey));
    save(out, "menu-scrolled-one", &mut menus, &mut r, &mut f);
    let mut display = Box::new(App::new(334));
    display.round_display = true;
    display.event(Event::Hold(Key::Grey));
    for _ in 0..30 {
        if display.menu_item() == "Settings" {
            break;
        }
        display.event(Event::Tap(Key::Grey));
    }
    display.event(Event::Tap(Key::Blue));
    display.event(Event::Tap(Key::Blue));
    display.event(Event::Tap(Key::Grey));
    display.event(Event::Tap(Key::Grey));
    save(out, "display-background", &mut display, &mut r, &mut f);
    display.event(Event::Tap(Key::Grey));
    save(out, "display-card-back", &mut display, &mut r, &mut f);
    #[cfg(feature = "validation")]
    for case in 0..6 {
        for mode in [Mode::Off, Mode::Guide] {
            let mut a = Box::new(App::EMPTY);
            assert!(a.prepare_fixed_benchmark(case, mode));
            save(
                out,
                &format!("hand-{case}-{}", mode.name()),
                &mut a,
                &mut r,
                &mut f,
            );
        }
    }
    #[cfg(feature = "validation")]
    {
        use stardust_blackjack::{
            cards::{Card, Hand},
            engine::{Action, Phase},
        };
        for n in 3..=9 {
            let mut a = Box::new(App::EMPTY);
            assert!(a.prepare_fixed_benchmark(0, Mode::Off));
            let mut h = Hand::EMPTY;
            h.bet = a.game.hands[0].bet;
            for i in 0..n {
                assert!(h.add(Card::from_id(12).unwrap(), i as u16));
            }
            a.game.hands[0] = h;
            save(out, &format!("player-{n}-cards"), &mut a, &mut r, &mut f);
        }
        let mut surrender = Box::new(App::EMPTY);
        assert!(surrender.prepare_fixed_benchmark(0, Mode::Guide));
        surrender.policy = stardust_blackjack::training::Policy::Basic;
        let mut hand = Hand::EMPTY;
        hand.bet = 500;
        hand.add(Card::from_id(8).unwrap(), 0);
        hand.add(Card::from_id(4).unwrap(), 1);
        surrender.game.hands[0] = hand;
        let mut dealer = Hand::EMPTY;
        dealer.add(Card::from_id(8).unwrap(), 2);
        dealer.add(Card::from_id(0).unwrap(), 3);
        surrender.game.dealer = dealer;
        save(
            out,
            "guide-surrender-16-vs-10",
            &mut surrender,
            &mut r,
            &mut f,
        );
        for back in 0..3 {
            let mut backed = Box::new(App::EMPTY);
            assert!(backed.prepare_fixed_benchmark(0, Mode::Off));
            backed.card_back = back;
            save(
                out,
                &format!("card-back-{back}"),
                &mut backed,
                &mut r,
                &mut f,
            );
        }
        for background in 0..4 {
            let mut themed = Box::new(App::EMPTY);
            assert!(themed.prepare_fixed_benchmark(0, Mode::Off));
            themed.background = background;
            save(
                out,
                &format!("theme-{background}"),
                &mut themed,
                &mut r,
                &mut f,
            );
        }
        let mut stable = Box::new(App::EMPTY);
        assert!(stable.prepare_fixed_benchmark(0, Mode::Off));
        save(out, "transition-player", &mut stable, &mut r, &mut f);
        assert!(stable.game.act(Action::Stand));
        save(out, "transition-after-stand", &mut stable, &mut r, &mut f);
        let mut draw = 0;
        while stable.game.phase == Phase::Dealer {
            assert!(stable.game.dealer_step());
            save(
                out,
                &format!("transition-dealer-{draw}"),
                &mut stable,
                &mut r,
                &mut f,
            );
            draw += 1;
        }
        save(out, "transition-result", &mut stable, &mut r, &mut f);
        let mut a = Box::new(App::EMPTY);
        assert!(a.prepare_fixed_benchmark(0, Mode::Off));
        assert!(a.game.act(Action::Split));
        save(out, "split-hand", &mut a, &mut r, &mut f);
        let mut a = Box::new(App::EMPTY);
        assert!(a.prepare_fixed_benchmark(0, Mode::Guide));
        a.game.phase = Phase::Insurance;
        let mut dealer = Hand::EMPTY;
        dealer.add(Card::from_id(12).unwrap(), 0);
        dealer.add(Card::from_id(0).unwrap(), 1);
        a.game.dealer = dealer;
        save(out, "guide-insurance", &mut a, &mut r, &mut f);
        a.game.phase = Phase::Result;
        a.game.hole_visible = true;
        a.game.round_net = 500;
        a.game.hands[0].outcome = stardust_blackjack::cards::Outcome::Win;
        save(out, "result", &mut a, &mut r, &mut f);
    }
    let mut a = Box::new(App::new(334));
    a.event(Event::Tap(Key::Blue));
    save(out, "burn", &mut a, &mut r, &mut f);
    a.tick(600);
    save(out, "ready-after-cut", &mut a, &mut r, &mut f);
    a.event(Event::Tap(Key::Blue));
    for i in 1..=10 {
        a.tick(600 + i * 400);
    }
    save(out, "actual-hand", &mut a, &mut r, &mut f);
    a.event(Event::Hold(Key::Grey));
    for _ in 0..30 {
        if a.menu_item() == "Settings" {
            break;
        }
        a.event(Event::Tap(Key::Grey));
    }
    a.event(Event::Tap(Key::Blue));
    save(out, "settings", &mut a, &mut r, &mut f);
    a.event(Event::Tap(Key::Grey));
    a.event(Event::Tap(Key::Blue));
    save(out, "sound", &mut a, &mut r, &mut f);
    a.event(Event::Tap(Key::Grey));
    a.event(Event::Tap(Key::Blue));
    save(out, "volume", &mut a, &mut r, &mut f);
}
