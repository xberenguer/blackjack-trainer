use crate::{
    assets::{Font, Sprite},
    canvas::{Canvas, HEIGHT, WIDTH},
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Align {
    Left,
    Center,
    Right,
}
#[derive(Clone, Copy)]
pub struct TextStyle {
    pub size: u8,
    pub bold: bool,
    pub color: u16,
    pub align: Align,
}
#[derive(Clone, Copy)]
struct Bounds {
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
}
fn printable(c: u8) -> u8 {
    if (32..=126).contains(&c) { c } else { b'?' }
}
// Signed exact 26.6 nearest-even rounding, matching approved font placement.
fn nearest_even(fixed: i32) -> i32 {
    let q = fixed.div_euclid(64);
    let r = fixed.rem_euclid(64);
    q + i32::from(r > 32 || (r == 32 && q & 1 != 0))
}
fn bounds(text: &str, font: Font) -> Bounds {
    let mut b = Bounds {
        min_x: 0,
        min_y: 0,
        max_x: 1,
        max_y: 1,
    };
    let mut cursor = 0i32;
    let mut previous = 0;
    for ch in text.bytes().take_while(|c| *c != 0).map(printable) {
        cursor = cursor.saturating_add(font.kern(previous, ch));
        let g = font.glyph(ch);
        let x = nearest_even(cursor.saturating_add(g.bx * 64));
        b.min_x = b.min_x.min(x);
        b.min_y = b.min_y.min(g.by);
        b.max_x = b.max_x.max(x + g.width as i32);
        b.max_y = b.max_y.max(g.by + g.height as i32);
        cursor = cursor.saturating_add(g.advance);
        previous = ch;
    }
    b
}
pub fn text_width(text: &str, size: u8, bold: bool) -> i32 {
    Font::get(size, bold).map_or(1, |f| {
        let b = bounds(text, f);
        b.max_x - b.min_x
    })
}
pub fn text_height(text: &str, size: u8, bold: bool) -> i32 {
    Font::get(size, bold).map_or(1, |f| {
        let b = bounds(text, f);
        b.max_y - b.min_y
    })
}
pub struct Renderer {
    mask: [u8; WIDTH * 32],
}
impl Renderer {
    pub const EMPTY: Self = Self {
        mask: [0; WIDTH * 32],
    };
}
impl Default for Renderer {
    fn default() -> Self {
        Self::EMPTY
    }
}
impl Renderer {
    pub fn text(
        &mut self,
        canvas: &mut Canvas,
        point: (i32, i32),
        text: &str,
        style: TextStyle,
    ) -> bool {
        let Some(font) = Font::get(style.size, style.bold) else {
            return false;
        };
        let b = bounds(text, font);
        let (mut x, y) = point;
        let full_w = b.max_x - b.min_x;
        let full_h = b.max_y - b.min_y;
        x = x.saturating_sub(match style.align {
            Align::Left => 0,
            Align::Center => full_w / 2,
            Align::Right => full_w,
        });
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = x.saturating_add(full_w).min(WIDTH as i32);
        let y1 = y.saturating_add(full_h).min(HEIGHT as i32);
        let w = x1 - x0;
        let h = y1 - y0;
        if w <= 0 || h <= 0 || h > 32 {
            return true;
        }
        let (w, h) = (w as usize, h as usize);
        self.mask[..w * h].fill(0);
        let mut cursor = 0i32;
        let mut previous = 0;
        for ch in text.bytes().take_while(|c| *c != 0).map(printable) {
            cursor = cursor.saturating_add(font.kern(previous, ch));
            let g = font.glyph(ch);
            let gx = x + nearest_even(cursor.saturating_add(g.bx * 64)) - b.min_x;
            let gy = y + g.by - b.min_y;
            for j in 0..g.height {
                for i in 0..g.width {
                    let dx = gx + i as i32;
                    let dy = gy + j as i32;
                    if dx >= x0 && dx < x1 && dy >= y0 && dy < y1 {
                        let target = (dy - y0) as usize * w + (dx - x0) as usize;
                        self.mask[target] =
                            self.mask[target].max(font.alpha(g.offset + j * g.width + i));
                    }
                }
            }
            cursor = cursor.saturating_add(g.advance);
            previous = ch;
        }
        for j in 0..h {
            for i in 0..w {
                canvas.blend(
                    x0 + i as i32,
                    y0 + j as i32,
                    style.color,
                    self.mask[j * w + i],
                );
            }
        }
        true
    }
    pub fn sprite(
        &mut self,
        canvas: &mut Canvas,
        point: (i32, i32),
        sprite: Sprite,
        size: Option<(i32, i32)>,
    ) {
        let (x, y) = point;
        let (w, h) = size.unwrap_or((sprite.width as i32, sprite.height as i32));
        if w < 1 || h < 1 {
            return;
        }
        let i0 = 0.max(x.saturating_neg());
        let j0 = 0.max(y.saturating_neg());
        let i1 = w.min((WIDTH as i32).saturating_sub(x));
        let j1 = h.min((HEIGHT as i32).saturating_sub(y));
        for j in j0..j1 {
            let sy = (j as i64 * sprite.height as i64 / h as i64) as usize;
            for i in i0..i1 {
                let sx = (i as i64 * sprite.width as i64 / w as i64) as usize;
                if let Some((color, alpha)) = sprite.rgba565(sy * sprite.width + sx) {
                    canvas.blend(x + i, y + j, color, alpha);
                }
            }
        }
    }
    pub fn card(
        &mut self,
        canvas: &mut Canvas,
        point: (i32, i32),
        card: Option<crate::cards::Card>,
        small: bool,
    ) {
        let index = match card {
            Some(c) => c.id() as usize + if small { 52 } else { 0 },
            None => {
                if small {
                    105
                } else {
                    104
                }
            }
        };
        if let Some(sprite) = Sprite::at(index) {
            self.sprite(canvas, point, sprite, None)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::nearest_even;
    #[test]
    fn signed_half_pixel_ties_are_even() {
        for (q, expected) in [
            (-160, -2),
            (-96, -2),
            (-32, 0),
            (32, 0),
            (96, 2),
            (160, 2),
            (-33, -1),
            (-31, 0),
            (31, 0),
            (33, 1),
        ] {
            assert_eq!(nearest_even(q), expected)
        }
    }
}
