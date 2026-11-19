//! Logical buttons-up canvas to the CO5300's native orientation.
pub const W: usize = 466;
pub const PIXELS: usize = W * W;
pub const fn logical_index(native_x: usize, native_y: usize, flipped: bool) -> usize {
    if flipped {
        native_x * W + (W - 1 - native_y)
    } else {
        (W - 1 - native_x) * W + native_y
    }
}
pub fn wire_pixel(pixels: &[u16], native_x: usize, native_y: usize, flipped: bool) -> [u8; 2] {
    pixels[logical_index(native_x, native_y, flipped)].to_be_bytes()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotation_is_bijective_and_matches_seeder() {
        let mut seen = std::vec![false;PIXELS];
        for y in 0..W {
            for x in 0..W {
                let at = logical_index(W - 1 - y, x, false);
                assert_eq!(at, y * W + x);
                assert!(!seen[at]);
                seen[at] = true;
                assert_eq!(
                    logical_index(W - 1 - y, x, true),
                    (W - 1 - y) * W + W - 1 - x
                );
            }
        }
        assert!(seen.into_iter().all(|v| v));
    }
    #[test]
    fn rgb565_is_converted_exactly_once() {
        let mut fb = std::vec![0;PIXELS];
        let at = logical_index(0, 0, false);
        for (color, bytes) in [
            (0xf800, [0xf8, 0]),
            (0x07e0, [7, 0xe0]),
            (0x001f, [0, 0x1f]),
            (0x081a, [8, 0x1a]),
        ] {
            fb[at] = color;
            assert_eq!(wire_pixel(&fb, 0, 0, false), bytes);
        }
    }
}
