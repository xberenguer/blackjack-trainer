//! Shared compact menu vocabulary and geometry for the 240×135 display.
use crate::{
    canvas::{Canvas, rgb},
    render::{Align, Renderer, TextStyle, text_height, text_width},
    text::Text,
    training::Mode,
};
use core::fmt::Write;
pub const BG: u16 = rgb(7, 5, 30);
pub const PANEL: u16 = rgb(24, 19, 59);
pub const SELECT: u16 = rgb(44, 38, 83);
pub const LINE: u16 = rgb(61, 49, 98);
pub const GOLD: u16 = rgb(255, 255, 0);
pub const WHITE: u16 = rgb(255, 255, 255);
pub const MUTED: u16 = rgb(168, 155, 193);
pub const DARK: u16 = rgb(11, 8, 36);
pub struct Widgets<'a> {
    pub frame: &'a mut Canvas,
    pub renderer: &'a mut Renderer,
}
impl Widgets<'_> {
    pub fn label(&mut self, point: (i32, i32), text: &str, style: TextStyle) {
        self.renderer.text(self.frame, point, text, style);
    }
    pub fn title(&mut self, text: &str, subtitle: Option<&str>, felt: bool) {
        self.frame
            .clear(if felt { self.renderer.felt() } else { BG });
        let mut size = if subtitle.is_some() { 14 } else { 18 };
        while size > 12 && text_width(text, size, true) > 202 {
            size -= 1
        }
        self.label(
            (8, 1),
            text,
            TextStyle {
                size,
                bold: true,
                color: GOLD,
                align: Align::Left,
            },
        );
        if let Some(subtitle) = subtitle {
            self.label(
                (8, 19),
                subtitle,
                TextStyle {
                    size: 9,
                    bold: false,
                    color: MUTED,
                    align: Align::Left,
                },
            );
        }
    }
    pub fn footer(&mut self, text: &str) {
        self.frame.rect(0, 121, 240, 14, DARK);
        self.label(
            (120, 124),
            text,
            TextStyle {
                size: 10,
                bold: false,
                color: MUTED,
                align: Align::Center,
            },
        );
    }
    pub fn row(&mut self, y: i32, text: &str, selected: bool, enabled: bool) {
        self.row_at((8, y, 224), text, selected, enabled);
    }

    pub fn row_at(
        &mut self,
        (x, y, width): (i32, i32, i32),
        text: &str,
        selected: bool,
        enabled: bool,
    ) {
        self.frame.round_rect(
            (x, y, width, 17),
            if selected { SELECT } else { PANEL },
            if selected { GOLD } else { LINE },
            if selected { 2 } else { 1 },
        );
        let mut size = 14;
        while size > 10
            && (text_width(text, size, true) > width - 12 || text_height(text, size, true) > 14)
        {
            size -= 1
        }
        self.label(
            (x + width / 2, y + (17 - text_height(text, size, true)) / 2),
            text,
            TextStyle {
                size,
                bold: true,
                color: if !enabled {
                    MUTED
                } else if selected {
                    GOLD
                } else {
                    WHITE
                },
                align: Align::Center,
            },
        );
    }
    pub fn list(&mut self, items: &[&str], focus: usize, note: Option<&str>) {
        if items.is_empty() {
            self.footer("BLUE: BACK");
            return;
        }
        let visible = 4;
        let pitch = if note.is_some() { 19 } else { 20 };
        let focus = focus.min(items.len() - 1);
        let top = focus.saturating_sub(3).min(items.len().saturating_sub(4));
        for (i, item) in items.iter().enumerate().skip(top).take(visible) {
            self.row(30 + (i - top) as i32 * pitch, item, focus == i, true);
        }
        if note.is_none()
            && let Some(next) = items.get(top + visible)
        {
            self.peek(next);
        }
        self.scroll(items.len(), top, note);
        self.footer("GREY: NEXT   BLUE: SELECT");
    }
    /// A clipped, fading next row indicates continuation without extra chrome.
    pub fn peek(&mut self, next: &str) {
        self.row(110, next, false, true);
        for y in 110..121 {
            let alpha = (55 + (y - 110) * 20).min(255) as u8;
            for x in 8..232 {
                self.frame.blend(x, y, BG, alpha);
            }
        }
    }
    /// Preserve explanatory notes in custom four-row viewports.
    pub fn scroll(&mut self, _count: usize, _top: usize, note: Option<&str>) {
        if let Some(note) = note {
            self.label(
                (120, 109),
                note,
                TextStyle {
                    size: 9,
                    bold: false,
                    color: MUTED,
                    align: Align::Center,
                },
            );
        }
    }
    pub fn cut(&mut self, decks: u8, index: u8) {
        self.title("CUT THE SHOE", None, true);
        let mut text = Text::<48>::new();
        let _ = write!(text, "{} DECKS - PLAYER CUT", decks);
        self.label(
            (120, 20),
            text.as_str(),
            TextStyle {
                size: 11,
                bold: true,
                color: WHITE,
                align: Align::Center,
            },
        );
        for i in 0..22 {
            self.renderer.card(self.frame, (20 + i * 8, 36), None, true);
        }
        let x = 20 + index.min(16) as i32 * 10;
        self.frame.rect(x, 34, 3, 54, GOLD);
        self.frame.rect(x - 3, 32, 9, 4, GOLD);
        self.label(
            (120, 99),
            "GREY: MOVE   BLUE: CUT",
            TextStyle {
                size: 12,
                bold: true,
                color: GOLD,
                align: Align::Center,
            },
        );
    }
    pub fn mode_footer(&mut self, mode: Mode) {
        self.frame.rect(0, 121, 240, 14, DARK);
        let mut text = Text::<24>::new();
        let _ = write!(text, "MODE: {}", mode.name());
        self.label(
            (120, 123),
            text.as_str(),
            TextStyle {
                size: 11,
                bold: true,
                color: GOLD,
                align: Align::Center,
            },
        );
    }
}
