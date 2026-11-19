//! Portable little-endian bitmap/font/PCM data; no foreign runtime or pointers.
pub(crate) static SPRITES: &[u8] = include_bytes!("../assets/sprites.bin");
pub(crate) static FONTS: &[u8] = include_bytes!("../assets/fonts.bin");
pub(crate) static AUDIO: &[u8] = include_bytes!("../assets/audio.bin");
fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    let s = data.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes([s[0], s[1]]))
}
fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    let s = data.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}
pub(crate) fn record(data: &'static [u8], magic: &[u8; 4], index: usize) -> Option<&'static [u8]> {
    if data.get(..4)? != magic || u16_at(data, 4)? != 1 || index >= u16_at(data, 6)? as usize {
        return None;
    }
    let offset = u32_at(data, 8 + index * 4)? as usize;
    data.get(offset..)
}
#[derive(Clone, Copy)]
pub struct Sprite {
    pub width: usize,
    pub height: usize,
    pixels: &'static [u8],
    alpha: &'static [u8],
}
impl Sprite {
    pub fn at(index: usize) -> Option<Self> {
        let data = record(SPRITES, b"SART", index)?;
        let width = u16_at(data, 0)? as usize;
        let height = u16_at(data, 2)? as usize;
        let size = width.checked_mul(height)?;
        if size == 0 {
            return None;
        }
        let end = 4usize.checked_add(size.checked_mul(2)?)?;
        Some(Self {
            width,
            height,
            pixels: data.get(4..end)?,
            alpha: data.get(end..end.checked_add(size)?)?,
        })
    }
    pub fn rgba565(self, index: usize) -> Option<(u16, u8)> {
        Some((
            u16_at(self.pixels, index.checked_mul(2)?)?,
            *self.alpha.get(index)?,
        ))
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Glyph {
    pub offset: usize,
    pub width: usize,
    pub height: usize,
    pub bx: i32,
    pub by: i32,
    pub advance: i32,
}
#[derive(Clone, Copy)]
pub struct Font {
    pub size: u8,
    pub bold: bool,
    glyphs: &'static [u8],
    kerning: &'static [u8],
    alpha: &'static [u8],
}
impl Font {
    pub fn get(size: u8, bold: bool) -> Option<Self> {
        let size = size.clamp(7, 24);
        Self::at((size - 7) as usize + if bold { 18 } else { 0 })
    }
    pub fn at(index: usize) -> Option<Self> {
        let data = record(FONTS, b"SFNT", index)?;
        let size = *data.first()?;
        let bold = *data.get(1)? != 0;
        let kern_count = u16_at(data, 2)? as usize;
        let alpha_size = u32_at(data, 4)? as usize;
        let glyph_end = 8 + 95 * 16;
        let kern_end = glyph_end + kern_count * 6;
        Some(Self {
            size,
            bold,
            glyphs: data.get(8..glyph_end)?,
            kerning: data.get(glyph_end..kern_end)?,
            alpha: data.get(kern_end..kern_end.checked_add(alpha_size)?)?,
        })
    }
    pub(crate) fn glyph(self, ch: u8) -> Glyph {
        let ch = if (32..=126).contains(&ch) { ch } else { b'?' };
        let i = (ch - 32) as usize * 16;
        Glyph {
            offset: u32_at(self.glyphs, i).unwrap() as usize,
            width: u16_at(self.glyphs, i + 4).unwrap() as usize,
            height: u16_at(self.glyphs, i + 6).unwrap() as usize,
            bx: u16_at(self.glyphs, i + 8).unwrap() as i16 as i32,
            by: u16_at(self.glyphs, i + 10).unwrap() as i16 as i32,
            advance: u32_at(self.glyphs, i + 12).unwrap() as i32,
        }
    }
    pub(crate) fn kern(self, previous: u8, ch: u8) -> i32 {
        if previous == 0 {
            return 0;
        }
        let pair = previous as u16 * 128 + ch as u16;
        for entry in self.kerning.chunks_exact(6) {
            if u16_at(entry, 0) == Some(pair) {
                return u32_at(entry, 2).unwrap() as i32;
            }
        }
        0
    }
    pub(crate) fn alpha(self, index: usize) -> u8 {
        self.alpha.get(index).copied().unwrap_or(0)
    }
}
#[derive(Clone, Copy)]
pub struct Clip {
    pub rate: u32,
    pub samples: usize,
    pcm: &'static [u8],
}
impl Clip {
    pub fn at(index: usize) -> Option<Self> {
        let data = record(AUDIO, b"SPCM", index)?;
        let rate = u32_at(data, 0)?;
        let samples = u32_at(data, 4)? as usize;
        let end = 8usize.checked_add(samples.checked_mul(2)?)?;
        Some(Self {
            rate,
            samples,
            pcm: data.get(8..end)?,
        })
    }
    pub fn sample(self, index: usize) -> Option<i16> {
        Some(u16_at(self.pcm, index.checked_mul(2)?)? as i16)
    }
    pub fn bytes(self) -> &'static [u8] {
        self.pcm
    }
}
pub fn validate_banks() -> bool {
    if u16_at(SPRITES, 6) != Some(141)
        || u16_at(FONTS, 6) != Some(36)
        || u16_at(AUDIO, 6) != Some(14)
    {
        return false;
    }
    for i in 0..141 {
        if Sprite::at(i).is_none() {
            return false;
        }
    }
    for i in 0..36 {
        let Some(f) = Font::at(i) else { return false };
        if f.size != 7 + (i % 18) as u8 || f.bold != (i >= 18) {
            return false;
        }
        for ch in 32..=126 {
            let g = f.glyph(ch);
            if g.offset + g.width * g.height > f.alpha.len() {
                return false;
            }
        }
    }
    for i in 0..14 {
        let Some(c) = Clip::at(i) else { return false };
        if c.rate != 22050 || c.samples == 0 {
            return false;
        }
    }
    true
}
