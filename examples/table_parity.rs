use stardust_blackjack::{
    canvas::{Canvas, HEIGHT, WIDTH},
    cards::{Card, Hand, Outcome},
    counting::Tracker,
    engine::{Engine, Phase},
    render::Renderer,
    rules::{Profile, Rules},
    table::{self, Mode, Options, Policy, Reveal},
};
use std::{
    io::{self, BufRead, Write},
    path::Path,
};
fn next<T: core::str::FromStr>(s: &mut core::str::SplitWhitespace<'_>) -> T
where
    T::Err: core::fmt::Debug,
{
    s.next().unwrap().parse().unwrap()
}
fn hand(s: &mut core::str::SplitWhitespace<'_>) -> Hand {
    let count: usize = next(s);
    let mut h = Hand::default();
    h.bet = next(s);
    h.split = next::<u8>(s) != 0;
    h.split_aces = next::<u8>(s) != 0;
    h.done = next::<u8>(s) != 0;
    h.doubled = next::<u8>(s) != 0;
    h.outcome = [
        Outcome::Pending,
        Outcome::Win,
        Outcome::Loss,
        Outcome::Push,
        Outcome::Blackjack,
        Outcome::Bust,
        Outcome::Surrender,
    ][next::<usize>(s)];
    for _ in 0..count {
        assert!(h.add(Card::from_id(next(s)).unwrap(), next(s)));
    }
    h
}
fn hash(c: &Canvas) -> u32 {
    c.pixels
        .iter()
        .flat_map(|p| p.to_le_bytes())
        .fold(2166136261, |h, b| (h ^ b as u32).wrapping_mul(16777619))
}
fn shot(c: &Canvas, name: &str) {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("validation/rust-table");
    std::fs::create_dir_all(&directory).unwrap();
    let mut f = std::fs::File::create(directory.join(name)).unwrap();
    write!(f, "P6\n{WIDTH} {HEIGHT}\n255\n").unwrap();
    for p in c.pixels {
        f.write_all(&[
            (((p >> 11) & 31) as u32 * 255 / 31) as u8,
            (((p >> 5) & 63) as u32 * 255 / 63) as u8,
            ((p & 31) as u32 * 255 / 31) as u8,
        ])
        .unwrap();
    }
}
fn main() {
    let mut tracker = Tracker::default();
    let mut renderer = Renderer::default();
    let mut canvas = Canvas::default();
    let mut checked = 0;
    let mut insurance_updates = 0;
    let mut shots = [false; 4];
    let modes = [Mode::Off, Mode::Test, Mode::Coach, Mode::Guide];
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        if line == "R" {
            tracker = Tracker::default();
            continue;
        }
        let mut s = line.split_whitespace();
        assert_eq!(s.next(), Some("F"));
        let phase: usize = next(&mut s);
        let profile: u8 = next(&mut s);
        let mut g = Engine::new(1, Rules::for_profile(Profile::from_id(profile).unwrap())).unwrap();
        g.phase = [
            Phase::Cut,
            Phase::Ready,
            Phase::Insurance,
            Phase::Player,
            Phase::Dealer,
            Phase::Result,
            Phase::Closed,
        ][phase];
        g.position = next(&mut s);
        g.shoe_number = next(&mut s);
        g.hand_count = next(&mut s);
        g.active = next(&mut s);
        g.bankroll = next(&mut s);
        g.selected_bet = next(&mut s);
        g.round_net = next(&mut s);
        g.insured = next::<u8>(&mut s) != 0;
        g.hole_visible = next::<u8>(&mut s) != 0;
        let revealing = next::<u8>(&mut s) != 0;
        let progress = next(&mut s);
        let bankroll = next(&mut s);
        let mode = modes[next::<usize>(&mut s)];
        let policy = if next::<u8>(&mut s) == 0 {
            Policy::Basic
        } else {
            Policy::HiLo
        };
        let normal_counter = next(&mut s);
        let guide_counter = next(&mut s);
        let result_hand = next(&mut s);
        let insurance_yes = next::<usize>(&mut s) == 1;
        let toast = if next::<u8>(&mut s) != 0 {
            Some(mode)
        } else {
            None
        };
        let expected: u32 = next(&mut s);
        g.dealer = hand(&mut s);
        for h in &mut g.hands {
            *h = hand(&mut s)
        }
        assert!(s.next().is_none());
        let options = Options {
            mode,
            policy,
            normal_counter,
            guide_counter,
            result_hand,
            insurance_yes,
            toast,
            reveal: if revealing {
                Some(Reveal { progress, bankroll })
            } else {
                None
            },
        };
        table::draw(&mut canvas, &mut renderer, &g, &mut tracker, options);
        let actual = hash(&canvas);
        // Preserve the historical oracle verbatim. 0.11.3 intentionally changes
        // Guide insurance and keeps insurance choices visible through mode toasts;
        // dedicated guide_insurance tests cover these new actionable UI contracts.
        let insurance_updated =
            g.phase == Phase::Insurance && !revealing && (mode == Mode::Guide || toast.is_some());
        if insurance_updated {
            insurance_updates += 1;
        }
        if actual != expected && !insurance_updated {
            shot(&canvas, "mismatch.ppm");
            panic!("table frame {checked}: expected {expected}, got {actual}; {line}")
        }
        let scene = if g.phase == Phase::Player && !revealing && toast.is_none() {
            Some(if mode == Mode::Guide && guide_counter != 2 {
                4
            } else if mode == Mode::Guide {
                if policy == Policy::HiLo { 2 } else { 1 }
            } else {
                0
            })
        } else if g.phase == Phase::Result && !revealing {
            Some(3)
        } else {
            None
        };
        if let Some(scene) = scene
            && scene < 4
            && !shots[scene]
        {
            shot(
                &canvas,
                [
                    "normal.ppm",
                    "basic-guide.ppm",
                    "hi-lo-guide.ppm",
                    "result.ppm",
                ][scene],
            );
            shots[scene] = true;
        }
        checked += 1;
    }
    println!(
        "PASS Rust table: {checked} historical frames; unaffected layouts pixel-identical; {insurance_updates} intentional insurance UI updates covered by guide_insurance regressions"
    );
}
