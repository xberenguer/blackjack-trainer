//! Render synthetic insurance regression frames; no device/save input.
#[path = "../tests/support/insurance_fixture.rs"]
mod insurance_fixture;
use stardust_blackjack::{
    canvas::Canvas,
    render::Renderer,
    table::{self, Mode, Options, Policy},
};
use std::{fs, io::Write, path::PathBuf};
fn main() {
    let dir = PathBuf::from(std::env::args_os().nth(1).expect("output directory"));
    fs::create_dir_all(&dir).unwrap();
    for (name, mode, policy, funded, high, yes) in [
        ("off-decline", Mode::Off, Policy::Basic, true, false, false),
        (
            "guide-basic-decline",
            Mode::Guide,
            Policy::Basic,
            true,
            false,
            false,
        ),
        (
            "guide-basic-yes",
            Mode::Guide,
            Policy::Basic,
            true,
            false,
            true,
        ),
        (
            "guide-hilo-below",
            Mode::Guide,
            Policy::HiLo,
            true,
            false,
            false,
        ),
        (
            "guide-hilo-below-yes",
            Mode::Guide,
            Policy::HiLo,
            true,
            false,
            true,
        ),
        (
            "guide-hilo-insure",
            Mode::Guide,
            Policy::HiLo,
            true,
            true,
            true,
        ),
        (
            "guide-hilo-unfunded",
            Mode::Guide,
            Policy::HiLo,
            false,
            true,
            true,
        ),
    ] {
        let (g, mut t) = insurance_fixture::fixture(funded, high);
        let mut frame = Canvas::default();
        let mut r = Renderer::default();
        table::draw(
            &mut frame,
            &mut r,
            &g,
            &mut t,
            Options {
                mode,
                policy,
                insurance_yes: yes,
                ..Options::default()
            },
        );
        let mut f = fs::File::create(dir.join(format!("{name}.ppm"))).unwrap();
        f.write_all(b"P6\n240 135\n255\n").unwrap();
        for &p in &frame.pixels {
            f.write_all(&[
                (((p >> 11) & 31) * 255 / 31) as u8,
                (((p >> 5) & 63) * 255 / 63) as u8,
                ((p & 31) * 255 / 31) as u8,
            ])
            .unwrap();
        }
    }
}
