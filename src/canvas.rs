pub const WIDTH: usize = 240;
pub const HEIGHT: usize = 135;
pub const fn rgb(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 >> 3) << 11) | ((g as u16 >> 2) << 5) | (b as u16 >> 3)
}
pub const FELT: u16 = rgb(11, 0, 107);
pub const BACKGROUND_NAMES: [&str; 4] = ["Black", "Blue", "Green", "Red"];
pub const fn background_color(index: u8) -> u16 {
    match index {
        1 => FELT,
        2 => rgb(0, 107, 11),
        3 => rgb(107, 11, 0),
        _ => 0,
    }
}
#[derive(Clone, Eq, PartialEq)]
pub struct Canvas {
    pub pixels: [u16; WIDTH * HEIGHT],
}
impl Default for Canvas {
    fn default() -> Self {
        Self {
            pixels: [0; WIDTH * HEIGHT],
        }
    }
}
impl Canvas {
    pub fn clear(&mut self, color: u16) {
        self.pixels.fill(color)
    }
    pub fn pixel(&mut self, x: i32, y: i32, color: u16) {
        if x >= 0 && y >= 0 && x < WIDTH as i32 && y < HEIGHT as i32 {
            self.pixels[y as usize * WIDTH + x as usize] = color
        }
    }
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u16) {
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = x.saturating_add(w).min(WIDTH as i32);
        let y1 = y.saturating_add(h).min(HEIGHT as i32);
        for row in y0..y1 {
            for col in x0..x1 {
                self.pixels[row as usize * WIDTH + col as usize] = color
            }
        }
    }
    pub fn outline(&mut self, rect: (i32, i32, i32, i32), color: u16, thickness: i32) {
        let (x, y, w, h) = rect;
        self.rect(x, y, w, thickness, color);
        self.rect(x, y + h - thickness, w, thickness, color);
        self.rect(x, y, thickness, h, color);
        self.rect(x + w - thickness, y, thickness, h, color)
    }
    pub fn round_rect(
        &mut self,
        rect: (i32, i32, i32, i32),
        fill: u16,
        stroke: u16,
        thickness: i32,
    ) {
        let (x, y, w, h) = rect;
        self.rect(x + 2, y, w - 4, h, fill);
        self.rect(x, y + 2, w, h - 4, fill);
        self.rect(x + 2, y, w - 4, thickness, stroke);
        self.rect(x + 2, y + h - thickness, w - 4, thickness, stroke);
        self.rect(x, y + 2, thickness, h - 4, stroke);
        self.rect(x + w - thickness, y + 2, thickness, h - 4, stroke);
        for (dx, dy) in [(1, 1), (w - 2, 1), (1, h - 2), (w - 2, h - 2)] {
            self.pixel(x + dx, y + dy, stroke)
        }
    }
    pub fn blend(&mut self, x: i32, y: i32, color: u16, alpha: u8) {
        if alpha == 0 || x < 0 || y < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 {
            return;
        }
        if alpha == 255 {
            self.pixel(x, y, color);
            return;
        }
        let dest = self.pixels[y as usize * WIDTH + x as usize];
        let a = alpha as u32;
        let b = 255 - a;
        let r = (((color as u32 >> 11) & 31) * a + ((dest as u32 >> 11) & 31) * b + 127) / 255;
        let g = (((color as u32 >> 5) & 63) * a + ((dest as u32 >> 5) & 63) * b + 127) / 255;
        let blue = ((color as u32 & 31) * a + (dest as u32 & 31) * b + 127) / 255;
        self.pixels[y as usize * WIDTH + x as usize] = ((r << 11) | (g << 5) | blue) as u16;
    }
}
