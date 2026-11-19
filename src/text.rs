use core::fmt::{self, Write};
/// Bounded UTF-8 formatting without a heap. A failed append leaves prior text intact.
pub struct Text<const N: usize = 128> {
    bytes: [u8; N],
    len: usize,
}
impl<const N: usize> Default for Text<N> {
    fn default() -> Self {
        Self::new()
    }
}
impl<const N: usize> Text<N> {
    pub const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
    pub fn clear(&mut self) {
        self.len = 0
    }
}
impl<const N: usize> Write for Text<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len.checked_add(s.len()).ok_or(fmt::Error)?;
        if end > N {
            return Err(fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}
pub struct Cash(pub i64, pub bool);
impl fmt::Display for Cash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let amount = self.0.unsigned_abs();
        let prefix = if self.0 < 0 {
            "-"
        } else if self.1 && self.0 > 0 {
            "+"
        } else {
            ""
        };
        write!(f, "{}${}", prefix, amount / 100)?;
        if !amount.is_multiple_of(100) {
            write!(f, ".{:02}", amount % 100)?
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn currency_and_utf8_buffers_are_bounded() {
        for (value, sign, want) in [
            (100000, false, "$1000"),
            (13750, false, "$137.50"),
            (-250, true, "-$2.50"),
            (500, true, "+$5"),
            (0, true, "$0"),
        ] {
            let mut text = Text::<32>::new();
            write!(text, "{}", Cash(value, sign)).unwrap();
            assert_eq!(text.as_str(), want)
        }
        let mut text = Text::<3>::new();
        assert!(text.write_str("é").is_ok());
        assert!(text.write_str("ab").is_err());
        assert_eq!(text.as_str(), "é");
    }
}
