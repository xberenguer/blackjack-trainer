//! Native 466 × 466 composition for the button-only Waveshare round display.
//!
//! The game controller, exposed-card accounting and decisions remain identical
//! to StickS3. Render `App` first, then compose this separate PSRAM framebuffer.
//! No touchscreen or device I/O exists in this module.
use crate::{
    app::{App, View},
    assets::{Font, Sprite},
    canvas::{FELT, rgb},
    cards::{Card, Hand, Outcome},
    engine::Phase,
    render::{text_height, text_width},
    text::{Cash, Text},
    training::{Choice, Mode, Policy},
    widgets,
};
use core::fmt::Write;

pub const WIDTH: usize = 466;
pub const HEIGHT: usize = 466;
pub const PIXELS: usize = WIDTH * HEIGHT;
pub const ORIENTATION_DEGREES: u16 = 270;
/// OLED table/menu backgrounds. StickS3 keeps its independently approved blue.
pub const BACKGROUND_NAMES: [&str; 4] = ["Black", "Blue", "Green", "Red"];
pub const CARD_BACK_NAMES: [&str; 3] = ["Gold", "Red", "Blue"];
/// Re-hue only coloured ink. The cream paper, white border, lettering shapes
/// and lattice geometry stay exactly the same; face sprites never use this.
pub fn card_back_color(c: u16, choice: u8) -> u16 {
    if choice == 0 || choice > 2 {
        return c;
    }
    let r = ((c >> 11) & 31) as u32 * 255 / 31;
    let g = ((c >> 5) & 63) as u32 * 255 / 63;
    let b = (c & 31) as u32 * 255 / 31;
    let hi = r.max(g).max(b);
    let lo = r.min(g).min(b);
    let chroma = hi - lo;
    if chroma * 100 < hi * 16 {
        return c;
    }
    if choice == 1 {
        rgb(hi as u8, lo as u8, lo as u8)
    } else {
        rgb(lo as u8, (lo + chroma / 4) as u8, hi as u8)
    }
}
pub const fn background_color(index: u8) -> u16 {
    match index {
        1 => FELT,
        2 => rgb(0, 107, 11),
        3 => rgb(107, 11, 0),
        _ => 0,
    }
}
const WHITE: u16 = rgb(255, 255, 255);
const GOLD: u16 = rgb(255, 255, 0);
const MUTED: u16 = rgb(168, 155, 193);
const GREEN: u16 = rgb(71, 238, 151);
const RED: u16 = rgb(255, 91, 108);
const PANEL: u16 = rgb(24, 19, 59);
const SELECT: u16 = rgb(44, 38, 83);
const PLAYER_CARD_Y: i32 = 153;
const PLAYER_CARD_WIDTH: i32 = 110;
const PLAYER_CARD_HEIGHT: i32 = 160;

/// Allocate this in PSRAM on hardware; constructing it on a small stack is costly.
pub struct RoundCanvas {
    pub pixels: [u16; PIXELS],
    background: u16,
}
impl RoundCanvas {
    pub const EMPTY: Self = Self {
        pixels: [0; PIXELS],
        background: 0,
    };
    pub fn set_background(&mut self, index: u8) {
        self.background = background_color(index);
    }
    pub const fn background(&self) -> u16 {
        self.background
    }
    pub fn clear(&mut self) {
        for y in 0..HEIGHT as i32 {
            for x in 0..WIDTH as i32 {
                self.pixels[y as usize * WIDTH + x as usize] = if inside_circle(x, y) {
                    self.background
                } else {
                    0
                };
            }
        }
    }
    fn pixel(&mut self, x: i32, y: i32, c: u16) {
        if inside_circle(x, y) {
            self.pixels[y as usize * WIDTH + x as usize] = c;
        }
    }
    fn blend(&mut self, x: i32, y: i32, c: u16, a: u8) {
        if a == 0 || !inside_circle(x, y) {
            return;
        }
        if a == 255 {
            self.pixel(x, y, c);
            return;
        }
        let d = self.pixels[y as usize * WIDTH + x as usize];
        let a = a as u32;
        let b = 255 - a;
        let r = (((c as u32 >> 11) & 31) * a + ((d as u32 >> 11) & 31) * b + 127) / 255;
        let g = (((c as u32 >> 5) & 63) * a + ((d as u32 >> 5) & 63) * b + 127) / 255;
        let v = ((c as u32 & 31) * a + (d as u32 & 31) * b + 127) / 255;
        self.pixel(x, y, ((r << 11) | (g << 5) | v) as u16);
    }
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u16) {
        for yy in y.max(0)..(y + h).min(HEIGHT as i32) {
            for xx in x.max(0)..(x + w).min(WIDTH as i32) {
                self.pixel(xx, yy, c);
            }
        }
    }
}
/// Physical pixel centres, including the circular mask used by host previews.
pub const fn inside_circle(x: i32, y: i32) -> bool {
    if x < 0 || y < 0 || x >= 466 || y >= 466 {
        return false;
    }
    let dx = 2 * x + 1 - 466;
    let dy = 2 * y + 1 - 466;
    dx * dx + dy * dy <= 466 * 466
}
fn square_root(n: i32) -> i32 {
    let mut a = 0;
    let mut b = 467;
    while a + 1 < b {
        let m = (a + b) / 2;
        if m * m <= n { a = m } else { b = m }
    }
    a
}
fn chord(y: i32, h: i32) -> i32 {
    let dy = (2 * y + 1 - 466)
        .abs()
        .max((2 * (y + h - 1) + 1 - 466).abs());
    if dy >= 466 {
        0
    } else {
        square_root(466 * 466 - dy * dy) - 12
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LayoutInfo {
    pub native_table: bool,
    pub dealer_cards: u8,
    pub player_cards: u8,
    pub hidden_hole: bool,
    pub guide: bool,
    pub count_visible: bool,
    pub visible_rows: u8,
    pub content_bands: u8,
    /// First displayed card geometry. The player fan is centered as cards arrive.
    pub dealer_rect: (i16, i16, u16, u16),
    pub player_rect: (i16, i16, u16, u16),
    pub seat_cards: u8,
    pub menu_top: u8,
    pub peek_row: bool,
    pub guide_action_px: u8,
    pub player_step: u16,
    pub dealer_step: u16,
    pub player_omitted: u8,
    pub dealer_omitted: u8,
    pub ready_hint_rect: (i16, i16, u16, u16),
    pub bankroll_rect: (i16, i16, u16, u16),
    pub wager_rect: (i16, i16, u16, u16),
    pub shared_menu: bool,
    pub confirmation: bool,
    pub selected_row: u8,
}
/// Layout plus the last publicly displayed player hand. Queued reveals can
/// finish after the Engine advances; this small cache prevents showing a
/// future queued face early while keeping the same hand in place between reveals.
pub struct RoundRenderer {
    pub info: LayoutInfo,
    player_cache: [Hand; 4],
    player_cache_valid: [bool; 4],
    card_back: u8,
}
impl RoundRenderer {
    pub const EMPTY: Self = Self {
        info: LayoutInfo {
            native_table: false,
            dealer_cards: 0,
            player_cards: 0,
            hidden_hole: false,
            guide: false,
            count_visible: false,
            visible_rows: 0,
            content_bands: 0,
            dealer_rect: (0, 0, 0, 0),
            player_rect: (0, 0, 0, 0),
            seat_cards: 0,
            menu_top: 0,
            peek_row: false,
            guide_action_px: 0,
            player_step: 0,
            dealer_step: 0,
            player_omitted: 0,
            dealer_omitted: 0,
            ready_hint_rect: (0, 0, 0, 0),
            bankroll_rect: (0, 0, 0, 0),
            wager_rect: (0, 0, 0, 0),
            shared_menu: false,
            confirmation: false,
            selected_row: 0,
        },
        player_cache: [Hand::EMPTY; 4],
        player_cache_valid: [false; 4],
        card_back: 0,
    };
    pub const fn new() -> Self {
        Self::EMPTY
    }
    /// Resume/error chooser pixels take priority over normal gameplay. They use
    /// the same round-safe composition without consulting live hand state.
    pub fn render_legacy(&mut self, source: &crate::canvas::Canvas, frame: &mut RoundCanvas) {
        self.render_legacy_with_hint(source, frame, "LEFT: SELECT   RIGHT: NEXT");
    }
    /// Let device-owned resume/failure choosers retain their exact confirmation
    /// gesture while using readable physical-button wording.
    pub fn render_legacy_with_hint(
        &mut self,
        source: &crate::canvas::Canvas,
        frame: &mut RoundCanvas,
        hint: &str,
    ) {
        frame.clear();
        self.info = LayoutInfo::default();
        self.compose_canvas(source, frame);
        self.footer(frame, hint);
    }
    pub fn render(&mut self, app: &App, frame: &mut RoundCanvas) {
        frame.set_background(app.background);
        self.card_back = app.card_back;
        frame.clear();
        self.info = LayoutInfo::default();
        // The shared controller owns labels, capability rows and semantic-to-
        // visual selection. The round target only composes that single model.
        if let Some(spec) = app.menu_spec() {
            let mut labels = [""; 16];
            for (label, item) in labels.iter_mut().zip(spec.items.iter()).take(spec.len) {
                *label = item.as_str();
            }
            self.info.shared_menu = true;
            self.info.selected_row = spec.selected as u8;
            let body = if spec.details.as_str().is_empty() {
                spec.body
            } else {
                Some(spec.details.as_str())
            };
            if spec.confirmation || body.is_some() {
                self.confirmation_menu(
                    frame,
                    spec.title,
                    spec.subtitle,
                    body,
                    &labels[..spec.len],
                    spec.selected,
                );
                self.info.confirmation = spec.confirmation;
            } else {
                self.list(
                    frame,
                    spec.title,
                    &labels[..spec.len],
                    spec.selected,
                    spec.subtitle,
                );
            }
            // Translate the controller's exact action hint, including read-only
            // PAGE/BACK and sound TEST hints, to the physical button positions.
            let hint = physical_hint(spec.footer);
            frame.rect(0, 394, WIDTH as i32, 28, frame.background);
            self.footer(frame, hint.as_str());
            if app.view == View::Menu && spec.subtitle.is_none() {
                self.text(
                    frame,
                    310,
                    105,
                    crate::VERSION,
                    12,
                    rgb(160, 160, 160),
                    false,
                    90,
                );
            }
            return;
        }
        match app.view {
            View::Table => self.table(app, frame),
            View::Cut => self.cut(app, frame),
            View::AudioVolume | View::Brightness => self.editor(app, frame),
            View::Controls => self.controls(app, frame),
            View::Burn => {
                self.heading(frame, "SHOE IS READY");
                self.text(
                    frame,
                    233,
                    185,
                    "Your cut is in place",
                    25,
                    WHITE,
                    false,
                    360,
                );
                self.text(
                    frame,
                    233,
                    232,
                    "Dealer burns one unseen card",
                    21,
                    MUTED,
                    false,
                    380,
                );
                self.text(frame, 233, 330, "LEFT: DEAL", 24, GOLD, true, 320);
                self.footer(frame, "HOLD RIGHT: MENU");
            }
            _ => self.compose_screen(app, frame),
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn text(
        &self,
        f: &mut RoundCanvas,
        cx: i32,
        y: i32,
        s: &str,
        mut px: u8,
        c: u16,
        bold: bool,
        max: i32,
    ) -> u8 {
        let mut base = px.clamp(7, 24);
        loop {
            let h = (text_height(s, base, bold) * px as i32 + base as i32 - 1) / base as i32;
            let w = (text_width(s, base, bold) * px as i32 + base as i32 - 1) / base as i32;
            // A centred chord budget alone is wrong for labels beside the
            // dealer: their right edge reaches the circle much sooner.
            let available = chord(y, h) - 2 * (cx - 233).abs();
            if px <= 10 || w <= max.min(available) {
                break;
            }
            px -= 1;
            base = px.clamp(7, 24);
        }
        let Some(font) = Font::get(base, bold) else {
            return 0;
        };
        let mut minx = 0;
        let mut miny = 0;
        let mut maxx = 1;
        let mut cursor = 0i32;
        let mut prev = 0;
        for ch in s.bytes().map(printable) {
            cursor += font.kern(prev, ch);
            let g = font.glyph(ch);
            let x = nearest_even(cursor + g.bx * 64);
            minx = minx.min(x);
            miny = miny.min(g.by);
            maxx = maxx.max(x + g.width as i32);
            cursor += g.advance;
            prev = ch;
        }
        let width = (maxx - minx) * px as i32 / base as i32;
        let x0 = cx - width / 2;
        cursor = 0;
        prev = 0;
        for ch in s.bytes().map(printable) {
            cursor += font.kern(prev, ch);
            let g = font.glyph(ch);
            let gx = nearest_even(cursor + g.bx * 64) - minx;
            let gy = g.by - miny;
            let dx = gx * px as i32 / base as i32;
            let dy = gy * px as i32 / base as i32;
            let w = (g.width as i32 * px as i32 + base as i32 - 1) / base as i32;
            let h = (g.height as i32 * px as i32 + base as i32 - 1) / base as i32;
            for yy in 0..h {
                for xx in 0..w {
                    let sx = (xx * base as i32 / px as i32) as usize;
                    let sy = (yy * base as i32 / px as i32) as usize;
                    if sx < g.width && sy < g.height {
                        f.blend(
                            x0 + dx + xx,
                            y + dy + yy,
                            c,
                            font.alpha(g.offset + sy * g.width + sx),
                        );
                    }
                }
            }
            cursor += g.advance;
            prev = ch;
        }
        px
    }
    fn heading(&self, f: &mut RoundCanvas, s: &str) {
        self.text(f, 233, 65, s, 32, GOLD, true, 330);
    }
    fn footer(&self, f: &mut RoundCanvas, s: &str) {
        self.text(f, 233, 400, s, 16, MUTED, false, 280);
    }
    fn sprite(&self, f: &mut RoundCanvas, x: i32, y: i32, w: i32, h: i32, index: usize) {
        if let Some(s) = Sprite::at(index) {
            for yy in 0..h {
                for xx in 0..w {
                    let sx = xx as usize * s.width / w as usize;
                    let sy = yy as usize * s.height / h as usize;
                    if let Some((c, a)) = s.rgba565(sy * s.width + sx) {
                        let c = if index == 104 || index == 105 {
                            card_back_color(c, self.card_back)
                        } else {
                            c
                        };
                        f.blend(x + xx, y + yy, c, a);
                    }
                }
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn fan(
        &self,
        f: &mut RoundCanvas,
        cards: &[Card],
        shown: usize,
        hidden: bool,
        anchor: Option<i32>,
        y: i32,
        width: i32,
        w: i32,
        h: i32,
        max: usize,
    ) -> (usize, usize, i32, i32) {
        let width = width.min(chord(y, h));
        let start = shown.saturating_sub(max);
        let n = shown - start;
        let intervals = n.saturating_sub(1) as i32;
        let step = if intervals == 0 {
            0
        } else {
            ((width - w) / intervals).min(w + 8)
        };
        let fan_width = w + intervals * step;
        let left = anchor.unwrap_or((WIDTH as i32 - fan_width) / 2);
        for (i, card) in cards.iter().enumerate().take(shown).skip(start) {
            let index = if hidden && i == 1 {
                104
            } else {
                card.id() as usize
            };
            self.sprite(f, left + (i - start) as i32 * step, y, w, h, index);
        }
        (n, start, left, step)
    }
    /// Compact public auxiliary card corners retain readable rank/suit pixels
    /// without covering the player's full-size central hand or the dealer row.
    fn auxiliary_cards(&self, f: &mut RoundCanvas, hand: &Hand, shown: usize) -> usize {
        let start = shown.saturating_sub(4);
        let n = shown - start;
        for (i, card) in hand.cards().iter().enumerate().take(shown).skip(start) {
            if let Some(sprite) = Sprite::at(card.id() as usize) {
                for y in 0..38 {
                    for x in 0..28 {
                        if let Some((c, a)) = sprite.rgba565(y * sprite.width + x) {
                            f.blend(
                                244 + (i - start) as i32 * 34 + x as i32,
                                323 + y as i32,
                                c,
                                a,
                            );
                        }
                    }
                }
            }
        }
        n
    }
    fn table(&mut self, a: &App, f: &mut RoundCanvas) {
        let g = &a.game;
        self.info.native_table = true;
        let queued = a.round_face_reveal();
        if a.reveal.is_some_and(|r| r.progress == 0) {
            self.player_cache_valid.fill(false);
        }
        let mut s = Text::<160>::new();
        let bank = a.reveal.map_or(g.bankroll, |r| r.bankroll);
        let _ = write!(s, "{}", Cash(bank, false));
        let px = self.text(f, 233, 388, s.as_str(), 36, GOLD, true, 270);
        self.info.bankroll_rect = rendered_rect(233, 388, s.as_str(), px, true);
        let hand_index = if g.phase == Phase::Player {
            g.active
        } else {
            a.result_hand
        }
        .min(g.hand_count.saturating_sub(1));
        let bet = if g.between_rounds() || g.hand_count == 0 {
            g.selected_bet
        } else {
            g.hands[hand_index].bet
        };
        s.clear();
        let _ = write!(s, "BET {}", Cash(bet, false));
        let px = self.text(f, 233, 428, s.as_str(), 24, WHITE, true, 240);
        self.info.wager_rect = rendered_rect(233, 428, s.as_str(), px, true);
        let count_setting = a.active_count_display();
        let showing_count =
            count_setting != 0 && queued.is_none() && !(g.extra_seats > 0 && a.reveal.is_some());
        self.info.count_visible = showing_count;
        if let Some(mode) = a.toast {
            self.text(f, 233, 18, mode.name(), 18, MUTED, true, 145);
        }
        if showing_count {
            s.clear();
            let _ = write!(s, "RC {:+}", a.tracker.running_count());
            if count_setting == 2 {
                if let Some(tc) = a.tracker.true_count((g.total_cards() - g.position) as u16) {
                    let _ = write!(s, "   TC {:+.1}", tc);
                } else {
                    let _ = write!(s, "   TC --");
                }
            }
            self.text(f, 233, 368, s.as_str(), 17, MUTED, true, 330);
        }
        if g.hand_count == 0 {
            self.text(
                f,
                233,
                203,
                if bank < g.rules.minimum {
                    "OUT OF CHIPS"
                } else {
                    "READY TO PLAY"
                },
                30,
                GOLD,
                true,
                360,
            );
            self.text(
                f,
                233,
                260,
                if bank < g.rules.minimum {
                    "LEFT: REFILL $1000"
                } else {
                    "LEFT: DEAL"
                },
                24,
                WHITE,
                true,
                360,
            );
            let hint = "RIGHT: CHANGE BET";
            let px = self.text(f, 233, 323, hint, 18, MUTED, false, 330);
            self.info.ready_hint_rect = rendered_rect(233, 323, hint, px, false);
            return;
        }
        self.info.guide = a.training.mode == Mode::Guide
            && a.reveal.is_none()
            && queued.is_none()
            && matches!(g.phase, Phase::Player | Phase::Insurance)
            && (g.phase == Phase::Insurance || a.toast.is_none());
        let show_hole = g.hole_visible && a.reveal.is_none() && queued.is_none();
        let shown = if let Some(r) = a.reveal {
            if r.progress >= 4 {
                2
            } else if r.progress >= 2 {
                1
            } else {
                0
            }
        } else if show_hole {
            g.dealer.count()
        } else {
            g.dealer.count().min(2)
        };
        let (dealer, dealer_skipped, dealer_left, dealer_step) = self.fan(
            f,
            g.dealer.cards(),
            shown,
            !show_hole,
            None,
            55,
            chord(55, 88),
            60,
            88,
            8,
        );
        self.info.dealer_rect = (dealer_left as i16, 55, 60, 88);
        self.info.dealer_step = dealer_step as u16;
        self.info.dealer_omitted = dealer_skipped as u8;
        self.info.dealer_cards = dealer as u8;
        self.info.hidden_hole = shown >= 2 && !show_hole;
        let displayed_hand = if let Some(reveal) = queued {
            if reveal.seat == g.player_seat && reveal.hand_index as usize == hand_index {
                self.player_cache[hand_index] = reveal.hand;
                self.player_cache_valid[hand_index] = true;
                reveal.hand
            } else if let Some(cached) = self
                .player_cache
                .iter()
                .zip(self.player_cache_valid)
                .find_map(|(hand, valid)| {
                    (valid && hand.positions().first() == g.hands[hand_index].positions().first())
                        .then_some(*hand)
                })
            {
                cached
            } else {
                // On restoration without a previously displayed hand, withhold
                // faces until their public reveal rather than showing future
                // queued snapshots from the already advanced Engine.
                Hand::EMPTY
            }
        } else {
            self.player_cache[hand_index] = g.hands[hand_index];
            self.player_cache_valid[hand_index] = true;
            g.hands[hand_index]
        };
        let h = &displayed_hand;
        let visible = if let Some(r) = a.reveal {
            if r.progress >= 3 {
                2
            } else if r.progress >= 1 {
                1
            } else {
                0
            }
        } else {
            h.count()
        };
        let (player, skipped, left, player_step) = self.fan(
            f,
            h.cards(),
            visible,
            false,
            None,
            PLAYER_CARD_Y,
            chord(PLAYER_CARD_Y, PLAYER_CARD_HEIGHT),
            PLAYER_CARD_WIDTH,
            PLAYER_CARD_HEIGHT,
            9,
        );
        self.info.player_rect = (
            left as i16,
            PLAYER_CARD_Y as i16,
            PLAYER_CARD_WIDTH as u16,
            PLAYER_CARD_HEIGHT as u16,
        );
        self.info.player_cards = player as u8;
        self.info.player_step = player_step as u16;
        self.info.player_omitted = skipped as u8;
        let initial_seat = a.reveal.and_then(|r| {
            let index = r.progress.checked_sub(5)? as usize;
            if index >= g.extra_seats as usize {
                return None;
            }
            let seat = if index < g.player_seat as usize {
                index
            } else {
                index + 1
            };
            Some((seat as u8, 0u8, g.others[index].hands[0], true))
        });
        let seat_view = queued
            .map(|r| (r.seat, r.hand_index, r.hand, false))
            .or(initial_seat)
            .or_else(|| {
                if g.phase != Phase::OtherPlayers || a.reveal.is_some() {
                    return None;
                }
                let seat = g.active_seat;
                let index = if seat < g.player_seat {
                    seat as usize
                } else {
                    (seat as usize).saturating_sub(1)
                };
                let other = g.others.get(index)?;
                let hand = other.active.min(other.hand_count.saturating_sub(1));
                Some((seat, hand as u8, other.hands[hand], false))
            });
        if let Some((seat, index, hand, initial)) = seat_view {
            // Already exposed auxiliary faces occupy their own fixed upper
            // strip. Your hand and dealer never move into that strip.
            let n = if initial {
                hand.count().min(2)
            } else {
                hand.count()
            };
            let shown = self.auxiliary_cards(f, &hand, n);
            self.info.seat_cards = shown as u8;
            s.clear();
            if seat == g.player_seat {
                let _ = write!(s, "YOUR HAND {} DRAW", index + 1);
            } else {
                let _ = write!(s, "SEAT {} / HAND {}", seat + 1, index + 1);
            }
            if n > 4 {
                let _ = write!(s, " / +{}", n - 4);
            }
            self.text(f, 150, 333, s.as_str(), 15, GOLD, true, 170);
        }
        let insurance = g.phase == Phase::Insurance && a.reveal.is_none() && queued.is_none();
        if seat_view.is_none() && g.hand_count > 1 && g.phase == Phase::Player && !self.info.guide {
            s.clear();
            let _ = write!(s, "HAND {}/{}", hand_index + 1, g.hand_count);
            self.text(f, 233, 323, s.as_str(), 18, GOLD, true, 330);
        }
        if a.reveal.is_some() {
            if seat_view.is_none() {
                self.text(f, 233, 323, "DEALING", 18, GOLD, true, 330);
            }
            return;
        }
        if queued.is_some() {
            return;
        }
        if skipped > 0 || dealer_skipped > 0 {
            s.clear();
            if skipped > 0 {
                let _ = write!(s, "+{} EARLIER PLAYER CARDS", skipped);
            }
            if dealer_skipped > 0 {
                if skipped > 0 {
                    let _ = write!(s, " / ");
                }
                let _ = write!(s, "+{} DEALER", dealer_skipped);
            }
            self.text(f, 233, 347, s.as_str(), 15, MUTED, false, 350);
        }
        if insurance {
            let yes = a.selected == 1;
            self.choice(f, 113, 323, 112, "DECLINE", !yes);
            s.clear();
            let _ = write!(s, "YES {}", Cash(g.hands[0].bet / 2, false));
            self.choice(f, 241, 323, 112, s.as_str(), yes);
            s.clear();
            if self.info.guide {
                let take = a
                    .best()
                    .is_some_and(|(choice, _)| choice == Choice::Insurance(true));
                if a.policy == Policy::HiLo {
                    if let Some(tc) = a.count_context().integer_true_count(g.rules) {
                        if tc >= 3 && g.bankroll < g.hands[0].bet / 2 {
                            let _ = write!(s, "DECLINE / NEED {}", Cash(g.hands[0].bet / 2, false));
                        } else {
                            let _ = write!(
                                s,
                                "{} / TC {:+} {} +3",
                                if take { "INSURE" } else { "DECLINE" },
                                tc,
                                if take { ">=" } else { "<" }
                            );
                        }
                    } else {
                        let _ = write!(s, "BASIC: DECLINE INSURANCE");
                    }
                } else {
                    let _ = write!(s, "BASIC: DECLINE INSURANCE");
                }
                self.info.guide_action_px = self.text(f, 233, 347, s.as_str(), 16, GOLD, true, 350);
            } else {
                self.text(f, 233, 347, "INSURE?", 18, GOLD, true, 350);
            }
            return;
        }
        if self.info.guide {
            if let Some((Choice::Play(best), _)) = a.best() {
                // The shared policy is count-aware; the round view must never
                // replace its recommendation with an unconditional basic chart.
                let advice = if a.policy == Policy::HiLo {
                    crate::deviations::recommend(
                        h,
                        g.dealer.cards()[0],
                        g.legal_mask(),
                        g.rules,
                        a.count_context(),
                    )
                } else {
                    crate::deviations::Advice {
                        play: crate::strategy::recommend(
                            h,
                            g.dealer.cards()[0],
                            g.legal_mask(),
                            g.rules,
                        ),
                        integer_true_count: None,
                        change: None,
                    }
                };
                let mut action = Text::<64>::new();
                if g.hand_count > 1 {
                    let _ = write!(action, "{}/{}  ", hand_index + 1, g.hand_count);
                }
                let _ = write!(action, "{}", crate::table::action_name(best));
                self.info.guide_action_px =
                    self.text(f, 233, 323, action.as_str(), 24, GOLD, true, 360);
                s.clear();
                if let Some(change) = advice.change {
                    use crate::deviations::Threshold;
                    match change.threshold {
                        Threshold::PositiveRunning => {
                            let _ = write!(
                                s,
                                "RC {:+} {} 0",
                                a.tracker.running_count(),
                                if change.reached { ">" } else { "<=" }
                            );
                        }
                        Threshold::NegativeRunning => {
                            let _ = write!(
                                s,
                                "RC {:+} {} 0",
                                a.tracker.running_count(),
                                if change.reached { "<" } else { ">=" }
                            );
                        }
                        Threshold::AtLeast(n) => {
                            let _ = write!(
                                s,
                                "TC {:+} {} {:+}",
                                advice.integer_true_count.unwrap_or(0),
                                if change.reached { ">=" } else { "<" },
                                n
                            );
                        }
                        Threshold::AtMost(n) => {
                            let _ = write!(
                                s,
                                "TC {:+} {} {:+}",
                                advice.integer_true_count.unwrap_or(0),
                                if change.reached { "<=" } else { ">" },
                                n
                            );
                        }
                    }
                } else if let Some(play) = advice.play {
                    for word in
                        crate::guide::short_reason(h, g.dealer.cards()[0], play).split_whitespace()
                    {
                        if !s.as_str().is_empty() {
                            let _ = write!(s, " ");
                        }
                        let _ = write!(s, "{word}");
                    }
                }
                if skipped == 0 && dealer_skipped == 0 {
                    self.wrap(f, 233, s.as_str(), 347, 15, 1, 350, MUTED);
                }
            }
        } else if g.phase == Phase::Result {
            s.clear();
            let _ = write!(s, "{}  {}", outcome(h.outcome), Cash(g.round_net, true));
            self.text(
                f,
                233,
                323,
                s.as_str(),
                27,
                if g.round_net >= 0 { GREEN } else { RED },
                true,
                350,
            );
        } else if g.phase == Phase::Dealer {
            self.text(f, 233, 323, "DEALER PLAYS", 18, GOLD, true, 330);
        } else if g.phase == Phase::OtherPlayers && seat_view.is_none() {
            self.text(f, 233, 323, "TABLE PLAYS", 18, GOLD, true, 330);
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn wrap(
        &self,
        f: &mut RoundCanvas,
        cx: i32,
        s: &str,
        y: i32,
        size: u8,
        lines: usize,
        max: i32,
        color: u16,
    ) {
        let mut rest = s;
        for row in 0..lines {
            if rest.is_empty() {
                break;
            }
            let mut end = rest.len();
            for (i, c) in rest.char_indices() {
                if c == '\n' {
                    end = i;
                    break;
                }
                if text_width(&rest[..i + c.len_utf8()], size.min(24), false) > max {
                    end = rest[..i].rfind(' ').unwrap_or(i);
                    break;
                }
            }
            if end == 0 {
                break;
            }
            self.text(
                f,
                cx,
                y + row as i32 * (size as i32 + 4),
                &rest[..end],
                size,
                color,
                false,
                max,
            );
            rest = rest[end..].trim_start();
        }
    }
    fn choice(&self, f: &mut RoundCanvas, x: i32, y: i32, w: i32, s: &str, selected: bool) {
        f.rect(x, y, w, 20, if selected { GOLD } else { PANEL });
        self.text(
            f,
            x + w / 2,
            y + 3,
            s,
            15,
            if selected { 0 } else { WHITE },
            true,
            w - 8,
        );
    }
    fn cut(&self, a: &App, f: &mut RoundCanvas) {
        self.heading(f, "CUT THE SHOE");
        let mut s = Text::<64>::new();
        let _ = write!(s, "{} DECKS / PLAYER CUT", a.game.rules.decks);
        self.text(f, 233, 119, s.as_str(), 21, WHITE, true, 360);
        for i in 0..22 {
            self.sprite(f, 61 + i * 13, 175, 70, 102, 104);
        }
        let x = 70 + a.cut_index.min(16) as i32 * 18;
        f.rect(x, 169, 5, 114, GOLD);
        f.rect(x - 4, 165, 13, 7, GOLD);
        self.text(f, 233, 328, "LEFT: CUT", 26, GOLD, true, 320);
        self.footer(f, "RIGHT: MOVE CUT");
        if let Some(mode) = a.toast {
            self.text(f, 233, 366, mode.name(), 18, GOLD, true, 300);
        }
    }
    fn list(
        &mut self,
        f: &mut RoundCanvas,
        title: &str,
        items: &[&str],
        focus: usize,
        subtitle: Option<&str>,
    ) {
        self.heading(f, title);
        if let Some(s) = subtitle {
            self.text(f, 233, 104, s, 15, MUTED, false, 350);
        }
        let focus = focus.min(items.len().saturating_sub(1));
        let top = focus.saturating_sub(5).min(items.len().saturating_sub(6));
        self.info.menu_top = top as u8;
        for (row, i) in (top..items.len()).take(6).enumerate() {
            let y = 131 + row as i32 * 38;
            let active = i == focus;
            let width = 390.min(chord(y, 32));
            let x = 233 - width / 2;
            f.rect(x, y, width, 32, if active { SELECT } else { PANEL });
            if active {
                f.rect(x, y, 3, 32, GOLD);
            }
            self.text(
                f,
                233,
                y + 6,
                items[i],
                23,
                if active { GOLD } else { WHITE },
                true,
                width - 22,
            );
            self.info.visible_rows += 1;
        }
        if let Some(next) = items.get(top + 6) {
            self.info.peek_row = true;
            let width = 390.min(chord(359, 24));
            let x = 233 - width / 2;
            f.rect(x, 359, width, 18, PANEL);
            self.text(f, 233, 365, next, 21, WHITE, true, width - 22);
            // Crop then fade the next row into the selected OLED background.
            // The continuation cue needs neither a side rail nor extra words.
            f.rect(0, 377, 466, 16, f.background);
            for y in 359..377 {
                let alpha = (60 + (y - 359) * 11).min(255) as u8;
                for xx in x..x + width {
                    f.blend(xx, y, f.background, alpha);
                }
            }
        }
        self.footer(f, "RIGHT: NEXT   LEFT: SELECT");
    }
    #[allow(clippy::too_many_arguments)]
    fn confirmation_menu(
        &mut self,
        f: &mut RoundCanvas,
        title: &str,
        subtitle: Option<&str>,
        body: Option<&str>,
        items: &[&str],
        selected: usize,
    ) {
        self.heading(f, title);
        if let Some(subtitle) = subtitle {
            self.text(f, 233, 104, subtitle, 15, MUTED, false, 350);
        }
        if let Some(body) = body {
            let physical = physical_hint(body);
            self.wrap(
                f,
                233,
                physical.as_str(),
                128,
                20,
                if items.is_empty() { 10 } else { 7 },
                350,
                WHITE,
            );
        }
        for (i, item) in items.iter().enumerate().take(2) {
            let y = 306 + i as i32 * 38;
            let width = 360.min(chord(y, 32));
            let x = 233 - width / 2;
            let active = i == selected;
            f.rect(x, y, width, 32, if active { SELECT } else { PANEL });
            if active {
                f.rect(x, y, 3, 32, GOLD);
            }
            self.text(
                f,
                233,
                y + 6,
                item,
                23,
                if active { GOLD } else { WHITE },
                true,
                width - 22,
            );
            self.info.visible_rows += 1;
        }
        self.footer(f, "RIGHT: NEXT   LEFT: SELECT");
    }
    /// Reflow specialist screens into independent header, content bands and
    /// footer. Text/cards retain their aspect ratio; whole screens are never
    /// stretched into a circle or reduced to a landscape letterbox.
    fn editor(&self, a: &App, f: &mut RoundCanvas) {
        let volume = a.view == View::AudioVolume;
        let value = if volume {
            a.audio_volume
        } else {
            a.brightness_percent()
        };
        self.heading(f, if volume { "VOLUME" } else { "BRIGHTNESS" });
        let mut s = Text::<24>::new();
        let _ = write!(s, "{value}%");
        self.text(f, 233, 171, s.as_str(), 64, GOLD, true, 300);
        f.rect(70, 270, 326, 12, PANEL);
        f.rect(70, 270, 326 * value as i32 / 100, 12, MUTED);
        self.text(
            f,
            233,
            327,
            "LEFT: HIGHER   RIGHT: LOWER",
            18,
            WHITE,
            true,
            340,
        );
        if volume {
            self.text(f, 233, 367, "HOLD LEFT: TEST", 18, MUTED, false, 320);
        }
        self.footer(f, "HOLD RIGHT: BACK");
    }
    fn controls(&self, a: &App, f: &mut RoundCanvas) {
        self.heading(f, "TWO BUTTONS");
        let s = match a.page.min(3) {
            0 => {
                "RIGHT: hit during your turn. LEFT: stand. Between rounds, RIGHT selects the wager and LEFT deals."
            }
            1 => {
                "Hold RIGHT for the unified menu. Legal double, split and surrender appear first. RIGHT moves; LEFT selects."
            }
            2 => {
                "Hold LEFT to cycle Off, Test, Coach and Guide. Releasing a held button never plays a card."
            }
            _ => {
                "Count quiz: RIGHT minus, LEFT plus; BOTH submit once. Hold RIGHT to cancel. Hold LEFT cycles training modes."
            }
        };
        self.wrap(f, 233, s, 150, 23, 7, 360, WHITE);
        self.footer(f, "RIGHT: PAGE   LEFT: MENU");
    }
    fn compose_screen(&mut self, a: &App, f: &mut RoundCanvas) {
        self.compose_canvas(&a.frame, f);
        let hint = match a.view {
            View::CountAnswer => "RIGHT: -   LEFT: +   BOTH: SUBMIT",
            View::CountResult => "RIGHT: RETRY   LEFT: BACK",
            View::CoachCheck => "RIGHT: NEXT   LEFT: PLAY",
            View::Refill => "LEFT: REFILL   HOLD RIGHT: MENU",
            View::Notice => "LEFT: RETURN",
            View::Cards => "RIGHT: CARDS   LEFT: MENU",
            View::Rules => "RIGHT: PAGE   LEFT: MENU",
            View::TrainingStats => "RIGHT: CATEGORY   LEFT: BACK",
            View::RoundReview => "RIGHT: REVIEW   LEFT: TABLE",
            View::TrainingReview | View::EvReview | View::CountHistory => {
                "RIGHT: OLDER   LEFT: BACK"
            }
            View::ActionEv => "RIGHT: NEXT   LEFT: BACK",
            _ => "RIGHT: NEXT   LEFT: SELECT",
        };
        self.footer(f, hint);
    }
    fn compose_canvas(&mut self, a: &crate::canvas::Canvas, f: &mut RoundCanvas) {
        self.copy_band(a, f, 0, 28, 66, 65, 334, 39);
        let mut bands = [(0usize, 0usize); 24];
        let mut n = 0;
        let mut start = None;
        let mut last = 29;
        for y in 29..121 {
            let ink = (0..240).any(|x| important(a.pixels[y * 240 + x]));
            if ink {
                if start.is_none() {
                    start = Some(y);
                }
                last = y;
            } else if let Some(first) = start
                && y >= last + 3
            {
                if n < bands.len() {
                    bands[n] = (first, last + 1);
                    n += 1;
                }
                start = None;
            }
        }
        if let Some(first) = start
            && n < bands.len()
        {
            bands[n] = (first, last + 1);
            n += 1;
        }
        if n == 0 {
            bands[0] = (29, 121);
            n = 1;
        }
        let total: i32 = bands[..n]
            .iter()
            .map(|(s, e)| ((e - s) as i32 * 3 + 1) / 2)
            .sum();
        let gap = if n > 1 {
            ((232 - total) / (n as i32 - 1)).clamp(4, 28)
        } else {
            0
        };
        let mut y = (466 - (total + gap * (n as i32 - 1))) / 2 + 17;
        for &(s, e) in &bands[..n] {
            let h = ((e - s) as i32 * 3 + 1) / 2;
            let width = 360.min(chord(y, h));
            let height = (h * width / 360).max(1);
            self.copy_band(a, f, s, e - s, 233 - width / 2, y, width, height);
            y += h + gap;
        }
        self.info.content_bands = n as u8;
    }
    #[allow(clippy::too_many_arguments)]
    fn copy_band(
        &self,
        a: &crate::canvas::Canvas,
        f: &mut RoundCanvas,
        sy: usize,
        sh: usize,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) {
        if w <= 0 || h <= 0 {
            return;
        }
        for yy in 0..h {
            for xx in 0..w {
                let sx = xx as usize * 240 / w as usize;
                let py = sy + yy as usize * sh / h as usize;
                let mut c = a.pixels[py.min(134) * 240 + sx.min(239)];
                if c == widgets::BG || c == widgets::DARK || c == FELT {
                    c = f.background;
                }
                f.pixel(x + xx, y + yy, c);
            }
        }
    }
}
fn important(c: u16) -> bool {
    c != FELT
        && c != 0
        && c != widgets::BG
        && c != widgets::PANEL
        && c != widgets::SELECT
        && c != widgets::DARK
        && c != widgets::LINE
}
fn printable(c: u8) -> u8 {
    if (32..=126).contains(&c) { c } else { b'?' }
}
fn nearest_even(v: i32) -> i32 {
    let q = v.div_euclid(64);
    let r = v.rem_euclid(64);
    q + i32::from(r > 32 || (r == 32 && q & 1 != 0))
}
fn outcome(o: Outcome) -> &'static str {
    match o {
        Outcome::Pending => "PLAYING",
        Outcome::Win => "WIN",
        Outcome::Loss => "LOSE",
        Outcome::Push => "PUSH",
        Outcome::Blackjack => "BLACKJACK",
        Outcome::Bust => "BUST",
        Outcome::Surrender => "SURRENDER",
    }
}

impl Default for RoundRenderer {
    fn default() -> Self {
        Self::EMPTY
    }
}

fn rendered_rect(cx: i32, y: i32, text: &str, px: u8, bold: bool) -> (i16, i16, u16, u16) {
    let base = px.clamp(7, 24);
    let w = (text_width(text, base, bold) * px as i32 + base as i32 - 1) / base as i32;
    let h = (text_height(text, base, bold) * px as i32 + base as i32 - 1) / base as i32;
    ((cx - w / 2) as i16, y as i16, w as u16, h as u16)
}

fn physical_hint(source: &str) -> Text<512> {
    let mut out = Text::new();
    let mut rest = source;
    while !rest.is_empty() {
        if let Some(tail) = rest.strip_prefix("GREY") {
            let _ = out.write_str("RIGHT");
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("BLUE") {
            let _ = out.write_str("LEFT");
            rest = tail;
        } else if let Some(c) = rest.chars().next() {
            let _ = out.write_char(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}
